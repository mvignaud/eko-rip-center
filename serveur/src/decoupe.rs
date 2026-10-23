//! Découpe : les tracés peints dans un ton de découpe (`CutContour`, `Thrucut`…) sont
//! extraits du PDF puis écrits en HP-GL pour le plotter.
//!
//! On lit le contenu de la page : état graphique (`q`/`Q`/`cm`), formulaires XObject,
//! chemins (`m l c v y h re`) et couleur courante. Seule une couleur en Séparation dont
//! le nom est un ton de découpe rend un chemin « à découper ».

use lopdf::{Dictionary, Document, Object, content::Content};

/// Tons de découpe reconnus, comparés sans casse, espaces, tirets ni soulignés.
const TONS_DECOUPE: &[&str] = &["cutcontour", "thrucut", "kisscut", "insidecut", "perforation", "creasing", "score", "vcut"];

/// Écart maximal entre une courbe et ses segments : 0,025 mm, en points PDF.
const TOLERANCE_PT: f64 = 0.025 * 72.0 / 25.4;

/// Unités HP-GL par point PDF (40 unités par mm).
const UNITES_PAR_PT: f64 = 40.0 * 25.4 / 72.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Trace {
    /// Nom du ton tel qu'écrit dans le PDF.
    pub ton: String,
    /// Points en points PDF, origine au coin bas gauche de la page.
    pub points: Vec<(f64, f64)>,
    pub ferme: bool,
}

fn cle(nom: &str) -> String {
    nom.chars().filter(|c| !matches!(c, ' ' | '-' | '_')).flat_map(char::to_lowercase).collect()
}

pub fn est_ton_de_decoupe(nom: &str) -> bool {
    TONS_DECOUPE.contains(&cle(nom).as_str())
}

/// Matrice PDF `[a b c d e f]`.
type Matrice = [f64; 6];
const IDENTITE: Matrice = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// `m` puis `n` (convention PDF : nouvelle CTM = `cm` × CTM).
fn produit(m: &Matrice, n: &Matrice) -> Matrice {
    [
        m[0] * n[0] + m[1] * n[2],
        m[0] * n[1] + m[1] * n[3],
        m[2] * n[0] + m[3] * n[2],
        m[2] * n[1] + m[3] * n[3],
        m[4] * n[0] + m[5] * n[2] + n[4],
        m[4] * n[1] + m[5] * n[3] + n[5],
    ]
}

fn transformer(m: &Matrice, (x, y): (f64, f64)) -> (f64, f64) {
    (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
}

#[derive(Clone)]
struct Etat {
    ctm: Matrice,
    /// Ton de découpe de l'espace de couleur courant (trait, fond), qu'il soit encré ou non.
    espace_trait: Option<String>,
    espace_fond: Option<String>,
    /// Ton de découpe effectivement encré (teinte > 0).
    trait_: Option<String>,
    fond: Option<String>,
}

struct SousChemin {
    points: Vec<(f64, f64)>,
    ferme: bool,
}

struct Lecteur<'a> {
    doc: &'a Document,
    traces: Vec<Trace>,
}

fn nombres(operandes: &[Object]) -> Vec<f64> {
    operandes.iter().filter_map(|o| o.as_float().ok()).map(f64::from).collect()
}

/// Courbe de Bézier aplatie : nombre de segments tiré de l'écart au polygone de contrôle.
fn bezier(sortie: &mut Vec<(f64, f64)>, p0: (f64, f64), p1: (f64, f64), p2: (f64, f64), p3: (f64, f64)) {
    let ecart = |a: (f64, f64), b: (f64, f64), c: (f64, f64)| ((a.0 - 2.0 * b.0 + c.0).powi(2) + (a.1 - 2.0 * b.1 + c.1).powi(2)).sqrt();
    let d = ecart(p0, p1, p2).max(ecart(p1, p2, p3));
    let n = ((0.75 * d / TOLERANCE_PT).sqrt().ceil() as usize).clamp(1, 1000);
    for i in 1..=n {
        let t = i as f64 / n as f64;
        let u = 1.0 - t;
        let (a, b, c, e) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
        sortie.push((a * p0.0 + b * p1.0 + c * p2.0 + e * p3.0, a * p0.1 + b * p1.1 + c * p2.1 + e * p3.1));
    }
}

fn deref<'d>(doc: &'d Document, o: &'d Object) -> &'d Object {
    doc.dereference(o).map(|(_, o)| o).unwrap_or(o)
}

fn sous_dict<'d>(doc: &'d Document, ressources: Option<&'d Dictionary>, cle: &[u8]) -> Option<&'d Dictionary> {
    deref(doc, ressources?.get(cle).ok()?).as_dict().ok()
}

/// Ton de découpe d'un espace de couleur nommé : `[/Separation /CutContour …]`.
fn ton_espace(doc: &Document, ressources: Option<&Dictionary>, nom: &[u8]) -> Option<String> {
    let espaces = sous_dict(doc, ressources, b"ColorSpace")?;
    let tableau = deref(doc, espaces.get(nom).ok()?).as_array().ok()?;
    if deref(doc, tableau.first()?).as_name().ok()? != b"Separation" {
        return None;
    }
    let ton = String::from_utf8_lossy(deref(doc, tableau.get(1)?).as_name().ok()?).into_owned();
    est_ton_de_decoupe(&ton).then_some(ton)
}

impl<'a> Lecteur<'a> {
    fn lire(&mut self, contenu: &[u8], ressources: Option<&'a Dictionary>, etat_initial: Etat, profondeur: u32) -> Result<(), String> {
        let doc = self.doc;
        if profondeur > 20 {
            return Err("formulaires imbriqués trop profondément".into());
        }
        let operations = Content::decode(contenu).map_err(|e| format!("contenu de page illisible : {e}"))?.operations;
        let mut etat = etat_initial;
        let mut pile: Vec<Etat> = vec![];
        let mut chemin: Vec<SousChemin> = vec![];
        let mut courant = (0.0, 0.0);
        let mut debut = (0.0, 0.0);

        for op in &operations {
            let n = nombres(&op.operands);
            let pt = |i: usize| transformer(&etat.ctm, (n[i], n[i + 1]));
            match op.operator.as_str() {
                "q" => pile.push(etat.clone()),
                "Q" => etat = pile.pop().unwrap_or(etat),
                "cm" if n.len() == 6 => etat.ctm = produit(&[n[0], n[1], n[2], n[3], n[4], n[5]], &etat.ctm),
                "CS" | "cs" => {
                    let ton = op.operands.first().and_then(|o| o.as_name().ok()).and_then(|nom| ton_espace(doc, ressources, nom));
                    // Une Séparation commence à la teinte 1.
                    if op.operator == "CS" {
                        (etat.espace_trait, etat.trait_) = (ton.clone(), ton);
                    } else {
                        (etat.espace_fond, etat.fond) = (ton.clone(), ton);
                    }
                }
                "SC" | "SCN" => etat.trait_ = etat.espace_trait.clone().filter(|_| n.last().is_some_and(|t| *t > 0.0)),
                "sc" | "scn" => etat.fond = etat.espace_fond.clone().filter(|_| n.last().is_some_and(|t| *t > 0.0)),
                "G" | "RG" | "K" => (etat.espace_trait, etat.trait_) = (None, None),
                "g" | "rg" | "k" => (etat.espace_fond, etat.fond) = (None, None),
                "m" if n.len() == 2 => {
                    courant = pt(0);
                    debut = courant;
                    chemin.push(SousChemin { points: vec![courant], ferme: false });
                }
                "l" if n.len() == 2 => {
                    courant = pt(0);
                    if let Some(s) = chemin.last_mut() {
                        s.points.push(courant);
                    }
                }
                "c" | "v" | "y" if n.len() >= 4 => {
                    let (p1, p2, p3) = match op.operator.as_str() {
                        "c" if n.len() == 6 => (pt(0), pt(2), pt(4)),
                        "v" => (courant, pt(0), pt(2)),
                        "y" => (pt(0), pt(2), pt(2)),
                        _ => continue,
                    };
                    if let Some(s) = chemin.last_mut() {
                        bezier(&mut s.points, courant, p1, p2, p3);
                    }
                    courant = p3;
                }
                "h" => {
                    if let Some(s) = chemin.last_mut() {
                        s.ferme = true;
                    }
                    courant = debut;
                }
                "re" if n.len() == 4 => {
                    let (x, y, w, h) = (n[0], n[1], n[2], n[3]);
                    let coins = [(x, y), (x + w, y), (x + w, y + h), (x, y + h)].map(|p| transformer(&etat.ctm, p));
                    chemin.push(SousChemin { points: coins.to_vec(), ferme: true });
                    (courant, debut) = (coins[0], coins[0]);
                }
                "S" | "s" | "f" | "F" | "f*" | "B" | "B*" | "b" | "b*" | "n" => {
                    let remplissage = matches!(op.operator.as_str(), "f" | "F" | "f*");
                    let ton = match op.operator.as_str() {
                        "S" | "s" => etat.trait_.clone(),
                        "n" => None,
                        _ if remplissage => etat.fond.clone(),
                        _ => etat.trait_.clone().or_else(|| etat.fond.clone()),
                    };
                    // Un remplissage ou une fermeture (s, b) ferme chaque sous-chemin.
                    let fermer = op.operator != "S" && op.operator != "B" && op.operator != "B*";
                    if let Some(ton) = ton {
                        for s in chemin.drain(..).filter(|s| s.points.len() >= 2) {
                            self.traces.push(Trace { ton: ton.clone(), points: s.points, ferme: s.ferme || fermer });
                        }
                    }
                    chemin.clear();
                }
                "Do" => {
                    let Some(nom) = op.operands.first().and_then(|o| o.as_name().ok()) else { continue };
                    let Some(xobjets) = sous_dict(doc, ressources, b"XObject") else { continue };
                    let Ok(flux) = xobjets.get(nom).map(|o| deref(doc, o)).and_then(Object::as_stream) else { continue };
                    if flux.dict.get(b"Subtype").and_then(Object::as_name).ok() != Some(b"Form".as_slice()) {
                        continue;
                    }
                    let matrice = flux
                        .dict
                        .get(b"Matrix")
                        .and_then(Object::as_array)
                        .map(|a| nombres(a))
                        .ok()
                        .filter(|m| m.len() == 6)
                        .map(|m| [m[0], m[1], m[2], m[3], m[4], m[5]])
                        .unwrap_or(IDENTITE);
                    let contenu = flux.decompressed_content().unwrap_or_else(|_| flux.content.clone());
                    let ressources_form = flux.dict.get(b"Resources").ok().and_then(|o| deref(doc, o).as_dict().ok()).or(ressources);
                    let mut etat_form = etat.clone();
                    etat_form.ctm = produit(&matrice, &etat.ctm);
                    self.lire(&contenu, ressources_form, etat_form, profondeur + 1)?;
                }
                _ => {}
            }
        }
        Ok(())
    }
}

/// Tracés de découpe de la première page.
// ponytail: première page seulement, comme le rendu ; le multipage viendra avec la file.
pub fn extraire(pdf: &[u8]) -> Result<Vec<Trace>, String> {
    let doc = Document::load_mem(pdf).map_err(|e| format!("PDF illisible : {e}"))?;
    let page = *doc.get_pages().values().next().ok_or("PDF sans page")?;
    let dict_page = doc.get_dictionary(page).map_err(|e| e.to_string())?;
    let ressources = match doc.get_page_resources(page).map_err(|e| e.to_string())? {
        (Some(d), _) => Some(d),
        (None, ids) => ids.first().and_then(|id| doc.get_dictionary(*id).ok()),
    };
    // Origine au coin bas gauche de la MediaBox.
    let origine = dict_page
        .get(b"MediaBox")
        .and_then(Object::as_array)
        .map(|a| nombres(a))
        .ok()
        .filter(|m| m.len() == 4)
        .map(|m| (m[0], m[1]))
        .unwrap_or((0.0, 0.0));
    let ctm = [1.0, 0.0, 0.0, 1.0, -origine.0, -origine.1];
    let etat = Etat { ctm, espace_trait: None, espace_fond: None, trait_: None, fond: None };

    let mut lecteur = Lecteur { doc: &doc, traces: vec![] };
    lecteur.lire(&doc.get_page_content(page), ressources, etat, 0)?;
    Ok(lecteur.traces)
}

/// Outil du plotter selon le ton : 1 lame, 2 rainage, 3 perforation, 4 coupe en V.
fn outil(ton: &str) -> u8 {
    match cle(ton).as_str() {
        "creasing" | "score" => 2,
        "perforation" => 3,
        "vcut" => 4,
        _ => 1,
    }
}

/// Fichier HP-GL : un `SP` par outil, chaque tracé en `PU` puis `PD`, unités 1/40 mm.
pub fn hpgl(traces: &[Trace]) -> String {
    let u = |(x, y): (f64, f64)| ((x * UNITES_PAR_PT).round() as i64, (y * UNITES_PAR_PT).round() as i64);
    let mut sortie = String::from("IN;");
    let mut outils: Vec<u8> = traces.iter().map(|t| outil(&t.ton)).collect();
    outils.sort();
    outils.dedup();
    for o in outils {
        sortie += &format!("SP{o};");
        for t in traces.iter().filter(|t| outil(&t.ton) == o) {
            let (x0, y0) = u(t.points[0]);
            sortie += &format!("PU{x0},{y0};PD");
            let mut suite: Vec<String> = t.points[1..].iter().map(|p| u(*p)).map(|(x, y)| format!("{x},{y}")).collect();
            if t.ferme {
                suite.push(format!("{x0},{y0}"));
            }
            sortie += &suite.join(",");
            sortie += ";";
        }
    }
    sortie += "PU;SP0;";
    sortie
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{Stream, dictionary};

    fn separation(ton: &str) -> Object {
        Object::Array(vec![
            "Separation".into(),
            Object::Name(ton.as_bytes().to_vec()),
            "DeviceCMYK".into(),
            Object::Dictionary(dictionary! { "FunctionType" => 2, "Domain" => vec![0.into(), 1.into()], "C0" => vec![0.into(); 4], "C1" => vec![0.into(), 1.into(), 0.into(), 0.into()], "N" => 1 }),
        ])
    }

    /// Page : un formulaire tourné de 90° puis décalé, qui découpe un carré de 72 pt, un trait
    /// `Thrucut`, un fond `Silver` et un trait noir (ignorés), un cercle `cut contour`.
    fn pdf_de_test() -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let espaces = dictionary! { "CS0" => separation("CutContour"), "CS1" => separation("Silver"), "CS2" => separation("Thrucut"), "CS3" => separation("cut contour") };
        let espaces_id = doc.add_object(espaces);
        let form = Stream::new(
            dictionary! { "Type" => "XObject", "Subtype" => "Form", "BBox" => vec![0.into(), 0.into(), 72.into(), 72.into()], "Matrix" => vec![0.into(), 1.into(), (-1).into(), 0.into(), 100.into(), 0.into()], "Resources" => dictionary! { "ColorSpace" => espaces_id } },
            b"/CS0 CS 1 SCN 0 0 72 72 re S".to_vec(),
        );
        let form_id = doc.add_object(form);
        // Cercle de rayon 50 centré en (400, 300), en quatre arcs de Bézier.
        let k = 0.5523 * 50.0;
        let cercle = format!(
            "/CS3 CS 1 SCN 450 300 m 450 {a} {b} 350 400 350 c {c} 350 350 {a} 350 300 c 350 {d} {c} 250 400 250 c {b} 250 450 {d} 450 300 c S",
            a = 300.0 + k, b = 400.0 + k, c = 400.0 - k, d = 300.0 - k
        );
        let contenu = format!(
            "q 1 0 0 1 0 50 cm /Fm0 Do Q /CS1 cs 1 scn 0 0 10 10 re f /CS2 CS 1 SCN 200 0 m 200 72 l S /CS2 CS 0 SCN 210 0 m 210 72 l S 0 0 0 1 K 300 0 m 300 72 l S {cercle}"
        );
        let contenu_id = doc.add_object(Stream::new(dictionary! {}, contenu.into_bytes()));
        let pages_id = doc.new_object_id();
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages_id, "MediaBox" => vec![0.into(), 0.into(), 600.into(), 600.into()],
            "Contents" => contenu_id, "Resources" => dictionary! { "ColorSpace" => espaces_id, "XObject" => dictionary! { "Fm0" => form_id } },
        });
        doc.objects.insert(pages_id, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page_id.into()], "Count" => 1 }));
        let catalogue = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        doc.trailer.set("Root", catalogue);
        let mut octets = vec![];
        doc.save_to(&mut octets).unwrap();
        octets
    }

    #[test]
    fn noms_de_tons() {
        assert!(est_ton_de_decoupe("CutContour"));
        assert!(est_ton_de_decoupe("cut contour"));
        assert!(est_ton_de_decoupe("Kiss-Cut"));
        assert!(!est_ton_de_decoupe("White"));
        assert!(!est_ton_de_decoupe("Silver"));
    }

    #[test]
    fn seuls_les_tons_de_decoupe_encres_sont_extraits() {
        let traces = extraire(&pdf_de_test()).unwrap();
        let tons: Vec<&str> = traces.iter().map(|t| t.ton.as_str()).collect();
        assert_eq!(tons, ["CutContour", "Thrucut", "cut contour"], "Silver, trait noir et Thrucut à 0 % ignorés");

        // Formulaire : Matrix (rotation de 90°, puis +100 en x) PUIS cm (+50 en y) ;
        // l'ordre compte, une rotation ne commute pas avec une translation.
        let carre = &traces[0];
        assert!(carre.ferme);
        assert_eq!(carre.points, [(100.0, 50.0), (100.0, 122.0), (28.0, 122.0), (28.0, 50.0)]);

        let trait_ = &traces[1];
        assert!(!trait_.ferme);
        assert_eq!(trait_.points, [(200.0, 0.0), (200.0, 72.0)]);

        let cercle = &traces[2];
        assert!(cercle.points.len() > 20, "courbes aplaties en {} points", cercle.points.len());
        for (x, y) in &cercle.points {
            let r = ((x - 400.0).powi(2) + (y - 300.0).powi(2)).sqrt();
            assert!((r - 50.0).abs() < 0.1, "point hors du cercle : rayon {r}");
        }
    }

    #[test]
    fn hpgl_en_quarantiemes_de_millimetre() {
        let traces = vec![
            Trace { ton: "CutContour".into(), points: vec![(0.0, 0.0), (72.0, 0.0), (72.0, 72.0)], ferme: true },
            Trace { ton: "Creasing".into(), points: vec![(0.0, 0.0), (0.0, 72.0)], ferme: false },
        ];
        // 72 pt = 25,4 mm = 1016 unités.
        assert_eq!(hpgl(&traces), "IN;SP1;PU0,0;PD1016,0,1016,1016,0,0;SP2;PU0,0;PD0,1016;PU;SP0;");
    }
}
