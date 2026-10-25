//! Étiquette et QR code imprimés dans la marge du haut ou du bas (roadmap 3.D5, 3.D6).
//! Posés après la mosaïque : chaque lé porte les siens, avec son numéro. Sur une page
//! coupée en tuiles, seules les tuiles de la rangée du côté choisi ont cette marge.

use crate::edition::{Balises, Cote, Edition, NiveauQr};
use crate::imbrication::PT_PAR_MM;
use crate::mosaique::Tuile;
use lopdf::{Document, Object, Stream, dictionary};

/// Ce que le travail sait de lui-même, pour l'étiquette et le code.
pub struct Infos {
    pub nom: String,
    pub date: String,
    pub travail: uuid::Uuid,
    pub machine: String,
    /// Résolution et support.
    pub mode: String,
    /// Numéro de commande de la fiche travail (roadmap 5.6).
    pub commande: Option<String>,
    /// Quantité due de la fiche travail (roadmap 5.6).
    pub copies: Option<u32>,
}

const VIDE: &str = "—";

/// Texte de l'étiquette : nom · date · L × H mm · lé k/n · copies · mode · commande.
pub fn texte_etiquette(i: &Infos, l_mm: f64, h_mm: f64, tuile: (usize, usize)) -> String {
    let copies = i.copies.map_or(VIDE.to_string(), |c| c.to_string());
    let commande = i.commande.as_deref().filter(|c| !c.is_empty()).unwrap_or(VIDE);
    format!("{} · {} · {:.0} × {:.0} mm · lé {}/{} · copies {copies} · {} · cde {commande}", i.nom, i.date, l_mm, h_mm, tuile.0, tuile.1, i.mode)
}

/// Chaîne du QR code : les balises choisies, `clé=valeur` séparées par `;`, dans l'ordre
/// nom, page, tuile, date, travail, coupe (numéro des codes Kona), machine, commande.
pub fn texte_code(b: &Balises, i: &Infos, page: usize, tuile: (usize, usize)) -> String {
    let propre = |s: &str| s.replace([';', '='], ",");
    let mut champs = vec![];
    let mut ajouter = |oui: bool, cle: &str, valeur: String| {
        if oui {
            champs.push(format!("{cle}={}", propre(&valeur)));
        }
    };
    ajouter(b.nom, "nom", i.nom.clone());
    ajouter(b.page, "page", page.to_string());
    ajouter(b.tuile, "tuile", format!("{}/{}", tuile.0, tuile.1));
    ajouter(b.date, "date", i.date.clone());
    ajouter(b.travail, "travail", i.travail.to_string());
    ajouter(b.coupe, "coupe", format!("{:011}", crate::reperage::numero(i.travail, page)));
    ajouter(b.machine, "machine", i.machine.clone());
    ajouter(b.commande, "commande", i.commande.clone().unwrap_or_default());
    champs.join(";")
}

/// Modules du QR code (vrai = foncé), rangée par rangée depuis le haut.
pub fn modules(texte: &str, niveau: NiveauQr) -> Result<Vec<Vec<bool>>, String> {
    use qrcodegen::{QrCode, QrCodeEcc};
    let ecc = match niveau {
        NiveauQr::L => QrCodeEcc::Low,
        NiveauQr::M => QrCodeEcc::Medium,
        NiveauQr::Q => QrCodeEcc::Quartile,
        NiveauQr::H => QrCodeEcc::High,
    };
    let qr = QrCode::encode_text(texte, ecc).map_err(|e| format!("QR code impossible : {e:?}"))?;
    let n = qr.size();
    Ok((0..n).map(|y| (0..n).map(|x| qr.get_module(x, y)).collect()).collect())
}

/// Ajoute l'étiquette et le code à chaque page de `pdf` (sortie de l'édition, puis de la
/// mosaïque : `tuiles` dit quelle tuile est chaque page). Erreur : le détail.
pub fn annoter(pdf: &[u8], e: &Edition, infos: &Infos, tuiles: &[Option<Tuile>]) -> Result<Vec<u8>, String> {
    if e.etiquette.is_none() && e.code.is_none() {
        return Ok(pdf.to_vec());
    }
    let mut doc = Document::load_mem(pdf).map_err(|err| format!("PDF illisible : {err}"))?;
    let pages: Vec<_> = doc.get_pages().into_values().collect();
    let mm = PT_PAR_MM;
    let [bg, bb, bd, bh] = e.bords_mm().map(|v| v * mm);
    let marge = e.marge_effective() * mm;
    let police = doc.add_object(dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "Encoding" => "WinAnsiEncoding" });
    let ton = doc.add_object(Object::Array(vec![
        "Separation".into(),
        "CutContour".into(),
        "DeviceCMYK".into(),
        Object::Dictionary(dictionary! { "FunctionType" => 2, "Domain" => vec![0.into(), 1.into()], "C0" => vec![0.into(); 4], "C1" => vec![0.into(), 1.into(), 0.into(), 0.into()], "N" => 1 }),
    ]));
    for (i, page) in pages.into_iter().enumerate() {
        let n = i + 1;
        let t = tuiles.get(i).cloned().flatten();
        let boite: Vec<f64> = doc.get_dictionary(page).ok().and_then(|d| d.get(b"MediaBox").ok()).and_then(|o| o.as_array().ok()).map(|a| a.iter().filter_map(|o| o.as_float().ok()).map(f64::from).collect()).filter(|b: &Vec<f64>| b.len() == 4).ok_or(format!("page {n} sans MediaBox"))?;
        let (pw, ph) = (boite[2] - boite[0], boite[3] - boite[1]);
        // Repère de la page : bande de repères de lés sous la tuile, voisines à gauche et à droite.
        let (bande, premiere, derniere, haut, bas, rec_g, rec_d) = match &t {
            Some(t) => (t.bande_mm * mm, t.colonne == 1, t.colonne == t.colonnes, t.rangee == 1, t.rangee == t.rangees, t.recouvrement_mm[0].max(0.0) * mm, t.recouvrement_mm[2].max(0.0) * mm),
            None => (0.0, true, true, true, true, 0.0, 0.0),
        };
        let (l_mm, h_mm) = match &t {
            Some(t) => (t.source_mm[0] - (bg + bd) / mm, t.source_mm[1] - (bb + bh) / mm),
            None => ((pw - bg - bd) / mm, (ph - bb - bh) / mm),
        };
        let numero = t.as_ref().map_or((1, 1), |t| (t.numero, t.total));
        let x_gauche = if premiere { bg } else { rec_g } + 2.0 * mm;
        let x_droite = pw - if derniere { bd } else { rec_d } - 2.0 * mm;
        // Bas de la bande de marge du côté demandé, s'il est sur cette page.
        let bande_de = |c: Cote| match c {
            Cote::Haut if haut => Some(boite[1] + ph - marge),
            Cote::Bas if bas => Some(boite[1] + bande),
            _ => None,
        };
        let mut s = String::new();
        let mut droite = x_droite;
        if let Some(c) = e.code
            && let Some(y0) = bande_de(c.cote)
        {
            let lignes = modules(&texte_code(&c.balises, infos, n, numero), c.niveau)?;
            let (cote, marge_code) = (c.taille_mm * mm, c.marge_mm * mm);
            let module = cote / lignes.len() as f64;
            let (x, y) = (boite[0] + x_droite - marge_code - cote, y0 + (marge - cote) / 2.0);
            s += "q 0 0 0 1 k ";
            for (r, ligne) in lignes.iter().enumerate() {
                // Une suite de modules foncés : un seul rectangle.
                let mut k = 0;
                while k < ligne.len() {
                    if !ligne[k] {
                        k += 1;
                        continue;
                    }
                    let debut = k;
                    while k < ligne.len() && ligne[k] {
                        k += 1;
                    }
                    s += &format!("{:.4} {:.4} {:.4} {:.4} re ", x + debut as f64 * module, y + cote - (r + 1) as f64 * module, (k - debut) as f64 * module, module);
                }
            }
            s += "f Q\n";
            if c.contour {
                s += &format!("q /EtCC CS 1 SCN 0.5 w {:.4} {:.4} {:.4} {:.4} re S Q\n", x - marge_code, y - marge_code, cote + 2.0 * marge_code, cote + 2.0 * marge_code);
            }
            droite = x_droite - cote - 2.0 * marge_code - 2.0 * mm;
        }
        if let Some(et) = e.etiquette
            && let Some(y0) = bande_de(et.cote)
        {
            let texte = texte_etiquette(infos, l_mm, h_mm, numero);
            // Corps réduit si la ligne ne tient pas (chasse moyenne d'Helvetica ≈ 0,55 corps).
            let place = if e.code.is_some_and(|c| c.cote == et.cote) { droite } else { x_droite } - x_gauche;
            let corps = (et.taille_mm * mm).min(place / (0.55 * texte.chars().count() as f64)).max(1.0);
            let y = y0 + (marge - corps) / 2.0 + 0.2 * corps;
            s += &format!("q 0 0 0 1 k BT /EtH {corps:.3} Tf {:.4} {y:.4} Td {} Tj ET Q\n", boite[0] + x_gauche, crate::bat::winansi(&texte));
        }
        if s.is_empty() {
            continue;
        }
        let flux = doc.add_object(Stream::new(dictionary! {}, s.into_bytes()));
        ajouter_a_la_page(&mut doc, page, flux, police, ton).map_err(|err| format!("page {n} : {err}"))?;
    }
    let mut sortie = vec![];
    doc.save_to(&mut sortie).map_err(|err| err.to_string())?;
    Ok(sortie)
}

/// Contenu ajouté à la fin de la page, police et ton de découpe dans ses ressources.
fn ajouter_a_la_page(doc: &mut Document, page: lopdf::ObjectId, flux: lopdf::ObjectId, police: lopdf::ObjectId, ton: lopdf::ObjectId) -> Result<(), String> {
    let dict = doc.get_dictionary(page).map_err(|e| e.to_string())?;
    let mut contenus = match dict.get(b"Contents") {
        Ok(Object::Array(a)) => a.clone(),
        Ok(o) => vec![o.clone()],
        Err(_) => vec![],
    };
    contenus.push(Object::Reference(flux));
    // Ressources propres à la page (l'édition et la mosaïque en écrivent toujours) ; une
    // référence est suivie.
    let ressources = match dict.get(b"Resources") {
        Ok(Object::Reference(id)) => Some(*id),
        _ => None,
    };
    // Sous-dictionnaires en référence : copiés en place pour y ajouter les nôtres.
    let base = match ressources {
        Some(id) => doc.get_dictionary(id).map_err(|e| e.to_string())?.clone(),
        None => match dict.get(b"Resources") {
            Ok(Object::Dictionary(d)) => d.clone(),
            _ => lopdf::Dictionary::new(),
        },
    };
    let mut copies = vec![];
    for cle in [b"Font".as_slice(), b"ColorSpace"] {
        if let Ok(Object::Reference(id)) = base.get(cle) {
            copies.push((cle, doc.get_dictionary(*id).map_err(|e| e.to_string())?.clone()));
        }
    }
    let completer = |r: &mut lopdf::Dictionary| {
        for (cle, d) in &copies {
            r.set(*cle, d.clone());
        }
        for (cle, nom, id) in [(b"Font".as_slice(), "EtH", police), (b"ColorSpace", "EtCC", ton)] {
            match r.get_mut(cle) {
                Ok(Object::Dictionary(d)) => d.set(nom, id),
                _ => r.set(cle, dictionary! { nom => id }),
            }
        }
    };
    match ressources {
        Some(id) => completer(doc.get_dictionary_mut(id).map_err(|e| e.to_string())?),
        None => {
            let d = doc.get_dictionary_mut(page).map_err(|e| e.to_string())?;
            if !matches!(d.get(b"Resources"), Ok(Object::Dictionary(_))) {
                d.set("Resources", dictionary! {});
            }
            if let Ok(Object::Dictionary(r)) = d.get_mut(b"Resources") {
                completer(r);
            }
        }
    }
    doc.get_dictionary_mut(page).map_err(|e| e.to_string())?.set("Contents", Object::Array(contenus));
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::edition::{CodeMarge, Etiquette};

    const MM: f64 = PT_PAR_MM;

    pub(crate) fn infos() -> Infos {
        Infos { nom: "bâche.pdf".into(), date: "2026-10-25 10:00:00 UTC".into(), travail: uuid::Uuid::from_u128(123_456_789_012), machine: "Epson".into(), mode: "720 dpi".into(), commande: None, copies: None }
    }

    /// Textes des `Tj` de la page, décodés comme du latin-1 (WinAnsi sur ces caractères).
    pub(crate) fn textes(pdf: &[u8], page: u32) -> String {
        let doc = Document::load_mem(pdf).unwrap();
        let id = *doc.get_pages().get(&page).unwrap();
        let ops = lopdf::content::Content::decode(&doc.get_page_content(id)).unwrap().operations;
        ops.iter().filter(|o| o.operator == "Tj").filter_map(|o| o.operands[0].as_str().ok()).map(|b| b.iter().map(|&c| c as char).collect::<String>()).collect::<Vec<_>>().join("\n")
    }

    /// Modules lus sur la plaque noire (`lire(x mm, y mm depuis le bas)`), au centre de
    /// chaque module d'un code de `n` modules posé en (`x`, `y`) mm, `cote` mm.
    pub(crate) fn lire_qr(lire: impl Fn(f64, f64) -> u8, n: usize, (x, y, cote): (f64, f64, f64)) -> Vec<Vec<bool>> {
        let m = cote / n as f64;
        (0..n).map(|r| (0..n).map(|c| lire(x + (c as f64 + 0.5) * m, y + cote - (r as f64 + 0.5) * m) < 128).collect()).collect()
    }

    #[test]
    fn chaine_du_code_et_texte_de_l_etiquette() {
        let i = infos();
        let toutes = Balises { nom: true, page: true, tuile: true, date: true, travail: true, coupe: true, machine: true, commande: true };
        assert_eq!(texte_code(&toutes, &i, 2, (2, 3)), "nom=bâche.pdf;page=2;tuile=2/3;date=2026-10-25 10:00:00 UTC;travail=00000000-0000-0000-0000-001cbe991a14;coupe=45678901202;machine=Epson;commande=");
        assert_eq!(texte_code(&Balises::default(), &i, 1, (1, 1)), "nom=bâche.pdf;page=1;tuile=1/1;travail=00000000-0000-0000-0000-001cbe991a14");
        assert_eq!(texte_etiquette(&i, 1000.0, 500.0, (2, 3)), "bâche.pdf · 2026-10-25 10:00:00 UTC · 1000 × 500 mm · lé 2/3 · copies — · 720 dpi · cde —");
    }

    /// Étiquette et QR dans la marge du bas d'une page éditée : texte dans le PDF, modules
    /// rendus = ceux de la chaîne attendue, contour de découpe autour du code.
    #[test]
    fn etiquette_et_qr_dans_la_marge() {
        let source = crate::imbrication::tests::pdf(150.0 * MM, 60.0 * MM, "1 0 0 0", (0.0, 0.0));
        let code = CodeMarge { taille_mm: 20.0, marge_mm: 3.0, contour: true, niveau: NiveauQr::H, ..Default::default() };
        let e = Edition { etiquette: Some(Etiquette::default()), code: Some(code), ..Default::default() };
        assert!(e.verifier().is_ok());
        assert!((e.marge_effective() - 28.0).abs() < 1e-9, "code + 2 marges + 2 mm");
        let pdf = annoter(&crate::edition::appliquer(&source, &e).unwrap(), &e, &infos(), &[]).unwrap();
        let t = textes(&pdf, 1);
        assert!(t.contains("b\u{e2}che.pdf \u{b7} 2026-10-25 10:00:00 UTC \u{b7} 150 \u{d7} 60 mm \u{b7} l\u{e9} 1/1"), "{t}");
        // Contour du code : un tracé de plus que celui de la source.
        let d = &crate::decoupe::extraire(&pdf, false).unwrap()[0];
        assert_eq!(d.traces.len(), 2);
        let attendu = modules(&texte_code(&code.balises, &infos(), 1, (1, 1)), NiveauQr::H).unwrap();
        // Page 206 × 116 mm : code à droite, 2 mm du bord de l'image, 3 mm de marge, centré
        // dans les 28 mm du bas.
        let (pw, m) = (150.0 + 2.0 * 28.0, 28.0);
        let (x, y) = (pw - m - 2.0 - 3.0 - 20.0, (m - 20.0) / 2.0);
        let (xs, ys): (Vec<f64>, Vec<f64>) = d.traces[1].points.iter().map(|p| (p.0 / MM, p.1 / MM)).unzip();
        let min = |v: &[f64]| v.iter().copied().fold(f64::MAX, f64::min);
        assert!((min(&xs) - (x - 3.0)).abs() < 0.01 && (min(&ys) - (y - 3.0)).abs() < 0.01, "contour à 3 mm du code");
        let dpi = 300;
        let rep = std::env::temp_dir().join(format!("eko-rip-etiquette-{}", std::process::id()));
        std::fs::create_dir_all(&rep).unwrap();
        std::fs::write(rep.join("e.pdf"), &pdf).unwrap();
        let st = std::process::Command::new("gs").args(["-q", "-dSAFER", "-dBATCH", "-dNOPAUSE", "-sDEVICE=tiffsep", &format!("-r{dpi}")]).arg(format!("-sOutputFile={}", rep.join("r.tif").display())).arg(rep.join("e.pdf")).status().unwrap();
        assert!(st.success());
        let noir = image::open(rep.join("r(Black).tif")).unwrap().to_luma8();
        std::fs::remove_dir_all(&rep).unwrap();
        let lire = |x: f64, y: f64| noir.get_pixel((x / 25.4 * dpi as f64) as u32, noir.height() - 1 - (y / 25.4 * dpi as f64) as u32)[0];
        assert_eq!(lire_qr(lire, attendu.len(), (x, y, 20.0)), attendu, "QR rendu = QR de la chaîne attendue");
    }
}
