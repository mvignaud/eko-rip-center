//! Pré-vérification d'un PDF à imprimer : des alertes, jamais un refus (le travail part
//! quand même ; l'opérateur décide). Polices non incorporées (Ghostscript en substitue
//! une autre), images trop peu définies à leur taille imprimée.

use lopdf::{Dictionary, Document, Object, content::Content};
use serde::{Deserialize, Serialize};

/// Sous cette définition effective (pixels par pouce imprimé), une image est signalée.
// ponytail: seuil fixe ; une bâche vue de loin supporte 50 ppi, un autocollant demande
// 150 ppi. À régler par réglage si l'atelier le demande.
pub const PPI_MIN: f64 = 100.0;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Alerte {
    /// Code traduit par l'écran : `police_non_incorporee`, `image_basse_definition`.
    pub code: String,
    /// Page concernée (à partir de 1), si elle est connue.
    #[serde(default)]
    pub page: Option<usize>,
    /// Détail non traduit : nom de police, définition…
    pub detail: String,
}

fn deref<'d>(doc: &'d Document, o: &'d Object) -> &'d Object {
    doc.dereference(o).map(|(_, o)| o).unwrap_or(o)
}

type Matrice = [f64; 6];

fn produit(m: &Matrice, n: &Matrice) -> Matrice {
    [m[0] * n[0] + m[1] * n[2], m[0] * n[1] + m[1] * n[3], m[2] * n[0] + m[3] * n[2], m[2] * n[1] + m[3] * n[3], m[4] * n[0] + m[5] * n[2] + n[4], m[4] * n[1] + m[5] * n[3] + n[5]]
}

fn matrice(o: Option<&Object>) -> Matrice {
    o.and_then(|o| o.as_array().ok())
        .map(|a| a.iter().filter_map(|v| v.as_float().ok()).map(f64::from).collect::<Vec<_>>())
        .filter(|v| v.len() == 6)
        .map_or([1.0, 0.0, 0.0, 1.0, 0.0, 0.0], |v| [v[0], v[1], v[2], v[3], v[4], v[5]])
}

/// Définition effective d'une image de `largeur` × `hauteur` px peinte dans le carré unité
/// transformé par `ctm` (côtés imprimés en pouces).
fn mesurer(nom: &str, largeur: f64, hauteur: f64, ctm: &Matrice, sortie: &mut Vec<(String, f64)>) {
    let (cote_l, cote_h) = ((ctm[0].powi(2) + ctm[1].powi(2)).sqrt() / 72.0, (ctm[2].powi(2) + ctm[3].powi(2)).sqrt() / 72.0);
    if cote_l > 0.0 && cote_h > 0.0 && largeur > 0.0 && hauteur > 0.0 {
        sortie.push((nom.to_string(), (largeur / cote_l).min(hauteur / cote_h)));
    }
}

/// Plus petite définition effective de chaque image peinte dans un contenu (formulaires
/// compris) : l'image occupe le carré unité transformé par la matrice courante.
fn images(doc: &Document, contenu: &[u8], ressources: Option<&Dictionary>, ctm: Matrice, profondeur: u32, sortie: &mut Vec<(String, f64)>) {
    if profondeur > 20 {
        return;
    }
    let Ok(ops) = Content::decode(contenu) else { return };
    let mut ctm = ctm;
    let mut pile = vec![];
    for op in ops.operations {
        match op.operator.as_str() {
            "q" => pile.push(ctm),
            "Q" => ctm = pile.pop().unwrap_or(ctm),
            "cm" => {
                let n: Vec<f64> = op.operands.iter().filter_map(|o| o.as_float().ok()).map(f64::from).collect();
                if n.len() == 6 {
                    ctm = produit(&[n[0], n[1], n[2], n[3], n[4], n[5]], &ctm);
                }
            }
            // Image en ligne (BI … ID … EI) : clés abrégées W et H.
            "BI" => {
                if let Some(Object::Stream(im)) = op.operands.first() {
                    let cle = |court: &[u8], long: &[u8]| im.dict.get(court).or_else(|_| im.dict.get(long)).and_then(Object::as_i64).unwrap_or(0) as f64;
                    mesurer("image en ligne", cle(b"W", b"Width"), cle(b"H", b"Height"), &ctm, sortie);
                }
            }
            "Do" => {
                let Some(nom) = op.operands.first().and_then(|o| o.as_name().ok()) else { continue };
                let Some(xobjets) = ressources.and_then(|r| r.get(b"XObject").ok()).and_then(|x| deref(doc, x).as_dict().ok()) else { continue };
                let Ok(flux) = xobjets.get(nom).map(|o| deref(doc, o)).and_then(Object::as_stream) else { continue };
                match flux.dict.get(b"Subtype").and_then(Object::as_name).ok() {
                    Some(b"Image") => {
                        let dim = |cle: &[u8]| flux.dict.get(cle).and_then(Object::as_i64).unwrap_or(0) as f64;
                        mesurer(&String::from_utf8_lossy(nom), dim(b"Width"), dim(b"Height"), &ctm, sortie);
                    }
                    Some(b"Form") => {
                        let contenu = flux.decompressed_content().unwrap_or_else(|_| flux.content.clone());
                        let res = flux.dict.get(b"Resources").ok().and_then(|o| deref(doc, o).as_dict().ok()).or(ressources);
                        images(doc, &contenu, res, produit(&matrice(flux.dict.get(b"Matrix").ok()), &ctm), profondeur + 1, sortie);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

/// Les 14 polices de base du PDF : Ghostscript les remplace par des équivalentes de mêmes
/// métriques, leur absence ne change pas la mise en page.
const POLICES_DE_BASE: [&str; 14] = [
    "Times-Roman", "Times-Bold", "Times-Italic", "Times-BoldItalic", "Helvetica", "Helvetica-Bold", "Helvetica-Oblique", "Helvetica-BoldOblique",
    "Courier", "Courier-Bold", "Courier-Oblique", "Courier-BoldOblique", "Symbol", "ZapfDingbats",
];

/// Polices manquantes pour l'abandon d'un travail (roadmap 9.3) : non incorporées, hors
/// polices de base.
pub fn polices_manquantes(alertes: &[Alerte]) -> Vec<String> {
    alertes.iter().filter(|a| a.code == "police_non_incorporee" && !POLICES_DE_BASE.contains(&a.detail.as_str())).map(|a| a.detail.clone()).collect()
}

pub fn verifier(pdf: &[u8]) -> Vec<Alerte> {
    let Ok(doc) = Document::load_mem(pdf) else { return vec![] };
    let mut alertes = vec![];

    // Polices : un descripteur sans fichier de police est une police non incorporée.
    let mut polices: Vec<String> = doc
        .objects
        .values()
        .filter_map(|o| o.as_dict().ok())
        .filter(|d| d.get(b"Type").and_then(Object::as_name).ok() == Some(b"FontDescriptor".as_slice()))
        .filter(|d| !d.has(b"FontFile") && !d.has(b"FontFile2") && !d.has(b"FontFile3"))
        .map(|d| d.get(b"FontName").and_then(Object::as_name).map(|n| String::from_utf8_lossy(n).into_owned()).unwrap_or_default())
        .collect();
    // Les 14 polices de base n'ont souvent pas de descripteur du tout.
    polices.extend(doc.objects.values().filter_map(|o| o.as_dict().ok()).filter(|d| {
        d.get(b"Type").and_then(Object::as_name).ok() == Some(b"Font".as_slice())
            && d.get(b"Subtype").and_then(Object::as_name).ok() == Some(b"Type1".as_slice())
            && !d.has(b"FontDescriptor")
    }).map(|d| d.get(b"BaseFont").and_then(Object::as_name).map(|n| String::from_utf8_lossy(n).into_owned()).unwrap_or_default()));
    polices.sort();
    polices.dedup();
    // Préfixe de sous-ensemble « ABCDEF+ » retiré du nom affiché.
    let sans_prefixe = |n: String| match n.split_once('+') {
        Some((p, reste)) if p.len() == 6 && p.chars().all(|c| c.is_ascii_uppercase()) => reste.to_string(),
        _ => n,
    };
    alertes.extend(polices.into_iter().map(|nom| Alerte { code: "police_non_incorporee".into(), page: None, detail: sans_prefixe(nom) }));

    for (i, (_, id)) in doc.get_pages().into_iter().enumerate() {
        let ressources = match doc.get_page_resources(id) {
            Ok((Some(d), _)) => Some(d),
            Ok((None, ids)) => ids.first().and_then(|r| doc.get_dictionary(*r).ok()),
            Err(_) => None,
        };
        let mut trouvees = vec![];
        images(&doc, &doc.get_page_content(id), ressources, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0], 0, &mut trouvees);
        if let Some((nom, ppi)) = trouvees.into_iter().filter(|(_, p)| *p < PPI_MIN).min_by(|a, b| a.1.total_cmp(&b.1)) {
            alertes.push(Alerte { code: "image_basse_definition".into(), page: Some(i + 1), detail: format!("{nom} : {ppi:.0} ppi") });
        }
    }
    alertes
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{Stream, dictionary};

    /// Page de 144 × 144 pt : image de 10 × 10 px peinte sur 72 × 72 pt (10 ppi) dans un
    /// formulaire réduit de moitié (donc 36 pt : 20 ppi), plus du texte en Helvetica non
    /// incorporée.
    fn pdf_a_risques() -> Vec<u8> {
        let mut doc = Document::with_version("1.4");
        let image = doc.add_object(Stream::new(dictionary! { "Type" => "XObject", "Subtype" => "Image", "Width" => 10, "Height" => 10, "ColorSpace" => "DeviceGray", "BitsPerComponent" => 8 }, vec![128; 100]));
        let form = doc.add_object(Stream::new(
            dictionary! { "Type" => "XObject", "Subtype" => "Form", "BBox" => vec![0.into(), 0.into(), 144.into(), 144.into()], "Matrix" => vec![0.5.into(), 0.into(), 0.into(), 0.5.into(), 0.into(), 0.into()], "Resources" => dictionary! { "XObject" => dictionary! { "Im" => image } } },
            b"q 72 0 0 72 0 0 cm /Im Do Q".to_vec(),
        ));
        let police = doc.add_object(dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica" });
        let c = doc.add_object(Stream::new(dictionary! {}, b"/Fo Do BT /F1 12 Tf 10 100 Td (essai) Tj ET".to_vec()));
        let pages = doc.new_object_id();
        let page = doc.add_object(dictionary! { "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(), 0.into(), 144.into(), 144.into()], "Contents" => c, "Resources" => dictionary! { "XObject" => dictionary! { "Fo" => form }, "Font" => dictionary! { "F1" => police } } });
        doc.objects.insert(pages, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }));
        let cat = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
        doc.trailer.set("Root", cat);
        let mut o = vec![];
        doc.save_to(&mut o).unwrap();
        o
    }

    #[test]
    fn police_et_image_signalees() {
        let a = verifier(&pdf_a_risques());
        assert_eq!(a.len(), 2, "{a:?}");
        assert_eq!((a[0].code.as_str(), a[0].detail.as_str()), ("police_non_incorporee", "Helvetica"));
        assert_eq!((a[1].code.as_str(), a[1].page, a[1].detail.as_str()), ("image_basse_definition", Some(1), "Im : 20 ppi"));
        // Un PDF sans image ni texte (celui de l'imbrication) : rien à signaler.
        assert!(verifier(&crate::imbrication::tests::pdf(100.0, 100.0, "1 0 0 0", (0.0, 0.0))).is_empty());
        assert!(verifier(b"pas un pdf").is_empty());
    }

    /// Image en ligne (BI … EI) de 4 × 4 px étirée sur 200 × 100 pt, comme l'écrit Ghostscript.
    #[test]
    fn image_en_ligne() {
        let mut doc = Document::with_version("1.4");
        let c = doc.add_object(Stream::new(dictionary! {}, b"q 200 0 0 100 0 0 cm BI /W 4 /H 4 /CS /G /BPC 8 ID \x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\nEI Q".to_vec()));
        let pages = doc.new_object_id();
        let page = doc.add_object(dictionary! { "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(), 0.into(), 200.into(), 100.into()], "Contents" => c });
        doc.objects.insert(pages, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }));
        let cat = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
        doc.trailer.set("Root", cat);
        let mut o = vec![];
        doc.save_to(&mut o).unwrap();
        let a = verifier(&o);
        assert_eq!(a.len(), 1, "{a:?}");
        // Côté le plus faible : 4 px sur 200/72 pouce = 1,44 ppi.
        assert_eq!(a[0].detail, "image en ligne : 1 ppi");
    }

    #[test]
    fn polices_manquantes_hors_polices_de_base() {
        let a = |detail: &str| Alerte { code: "police_non_incorporee".into(), page: None, detail: detail.into() };
        let alertes = [a("Helvetica"), a("ArialMT"), a("Times-Bold"), Alerte { code: "image_basse_definition".into(), page: Some(1), detail: "Im : 20 ppi".into() }, a("Futura")];
        assert_eq!(polices_manquantes(&alertes), ["ArialMT", "Futura"]);
        assert!(polices_manquantes(&verifier(&pdf_a_risques())).is_empty(), "Helvetica seule : rien ne manque");
    }
}
