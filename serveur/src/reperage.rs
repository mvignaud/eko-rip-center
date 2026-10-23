//! Repérage pour l'impression puis la découpe sur le Mutoh Kona.
//!
//! La page est agrandie, son contenu décalé, et les repères ajoutés en noir pur (plaque
//! K seule, le capteur EPOS du Kona lit un noir franc). Cotes : manuel Kona AP-75380
//! chap. 6 et Application Guide Kona, chap. 4.4.2.
//! - `Manuel` : quatre croix P1–P4 aux coins, pointées à la main sur le Kona.
//! - `Cadre` : cadre continu de Type 1, mesuré automatiquement, sans code-barres ni
//!   CutServer.

use lopdf::{Document, Object, Stream, dictionary};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reperage {
    #[default]
    Aucun,
    Manuel,
    Cadre,
}

const PT_PAR_MM: f64 = 72.0 / 25.4;

/// Cotes du cadre de Type 1, en mm, choisies au-dessus des minimums du manuel.
pub mod cadre {
    /// Épaisseur : 3 mm minimum, 5 mm conseillés pour une impression jet d'encre.
    pub const EPAISSEUR: f64 = 5.0;
    /// Blanc entre l'image et le cadre : 6 mm minimum.
    pub const MARGE_IMAGE: f64 = 10.0;
    /// Cadre intérieur minimal : 250 × 250 mm.
    pub const INTERIEUR_MIN: f64 = 250.0;
    /// Blanc à gauche et à droite : 20 mm minimum.
    pub const BORD_COTE: f64 = 25.0;
    /// Blanc à l'avant (côté sortie) : 25 mm minimum.
    pub const BORD_AVANT: f64 = 30.0;
    /// Blanc à l'arrière : 75 mm minimum sur une feuille.
    pub const BORD_ARRIERE: f64 = 80.0;
}

/// Croix du repérage manuel : bras de 10 mm, trait de 0,5 mm, à 10 mm de l'image.
const CROIX_BRAS: f64 = 10.0;
const CROIX_TRAIT: f64 = 0.5;
const CROIX_MARGE: f64 = 10.0;
const CROIX_BORD: f64 = 20.0;

/// Mise en page : taille de la nouvelle page, décalage du contenu d'origine, repères
/// en rectangles pleins. Tout en points.
#[derive(Debug)]
pub struct Mise {
    pub page: (f64, f64),
    pub decalage: (f64, f64),
    pub rectangles: Vec<[f64; 4]>,
}

pub fn mise_en_page(reperage: Reperage, largeur: f64, hauteur: f64) -> Mise {
    let mm = |v: f64| v * PT_PAR_MM;
    match reperage {
        Reperage::Aucun => Mise { page: (largeur, hauteur), decalage: (0.0, 0.0), rectangles: vec![] },
        Reperage::Cadre => {
            use cadre::*;
            // Intérieur du cadre : l'image et sa marge, au moins 250 × 250 mm, image centrée.
            let int_l = (largeur + 2.0 * mm(MARGE_IMAGE)).max(mm(INTERIEUR_MIN));
            let int_h = (hauteur + 2.0 * mm(MARGE_IMAGE)).max(mm(INTERIEUR_MIN));
            let e = mm(EPAISSEUR);
            let (x0, y0) = (mm(BORD_COTE), mm(BORD_AVANT));
            let (ext_l, ext_h) = (int_l + 2.0 * e, int_h + 2.0 * e);
            let decalage = (x0 + e + (int_l - largeur) / 2.0, y0 + e + (int_h - hauteur) / 2.0);
            let rectangles = vec![
                [x0, y0, ext_l, e],
                [x0, y0 + ext_h - e, ext_l, e],
                [x0, y0, e, ext_h],
                [x0 + ext_l - e, y0, e, ext_h],
            ];
            Mise { page: (ext_l + 2.0 * mm(BORD_COTE), y0 + ext_h + mm(BORD_ARRIERE)), decalage, rectangles }
        }
        Reperage::Manuel => {
            let bord = mm(CROIX_BORD + CROIX_BRAS / 2.0);
            let m = mm(CROIX_MARGE);
            let decalage = (bord + m, bord + m);
            let (bras, trait_) = (mm(CROIX_BRAS), mm(CROIX_TRAIT));
            let mut rectangles = vec![];
            for (cx, cy) in [(bord, bord), (bord + largeur + 2.0 * m, bord), (bord + largeur + 2.0 * m, bord + hauteur + 2.0 * m), (bord, bord + hauteur + 2.0 * m)] {
                rectangles.push([cx - bras / 2.0, cy - trait_ / 2.0, bras, trait_]);
                rectangles.push([cx - trait_ / 2.0, cy - bras / 2.0, trait_, bras]);
            }
            Mise { page: (largeur + 2.0 * (bord + m), hauteur + 2.0 * (bord + m)), decalage, rectangles }
        }
    }
}

/// Réécrit la première page du PDF : nouvelle MediaBox, contenu décalé, repères en noir.
pub fn envelopper(pdf: &[u8], reperage: Reperage) -> Result<(Vec<u8>, Mise), String> {
    let mut doc = Document::load_mem(pdf).map_err(|e| format!("PDF illisible : {e}"))?;
    let page = *doc.get_pages().values().next().ok_or("PDF sans page")?;
    let dict = doc.get_dictionary(page).map_err(|e| e.to_string())?;
    let boite: Vec<f64> = dict
        .get(b"MediaBox")
        .and_then(Object::as_array)
        .map(|a| a.iter().filter_map(|o| o.as_float().ok()).map(f64::from).collect())
        .map_err(|_| "page sans MediaBox")?;
    if boite.len() != 4 {
        return Err("MediaBox invalide".into());
    }
    // ponytail: page tournée (/Rotate) refusée plutôt que mal placée ; à gérer si besoin.
    if dict.get(b"Rotate").and_then(Object::as_i64).is_ok_and(|r| r % 360 != 0) {
        return Err("page tournée (/Rotate) : repérage non pris en charge".into());
    }
    let mise = mise_en_page(reperage, boite[2] - boite[0], boite[3] - boite[1]);
    let (dx, dy) = (mise.decalage.0 - boite[0], mise.decalage.1 - boite[1]);

    let anciens: Vec<Object> = match dict.get(b"Contents") {
        Ok(Object::Array(a)) => a.clone(),
        Ok(o) => vec![o.clone()],
        Err(_) => vec![],
    };
    let debut = doc.add_object(Stream::new(dictionary! {}, format!("q 1 0 0 1 {dx:.4} {dy:.4} cm\n").into_bytes()));
    let mut fin = String::from("\nQ q 0 0 0 1 k\n");
    for [x, y, l, h] in &mise.rectangles {
        fin += &format!("{x:.4} {y:.4} {l:.4} {h:.4} re f\n");
    }
    fin += "Q\n";
    let fin = doc.add_object(Stream::new(dictionary! {}, fin.into_bytes()));

    let dict = doc.get_dictionary_mut(page).map_err(|e| e.to_string())?;
    let mut contenus = vec![Object::Reference(debut)];
    contenus.extend(anciens);
    contenus.push(Object::Reference(fin));
    dict.set("Contents", Object::Array(contenus));
    dict.set("MediaBox", vec![0.into(), 0.into(), Object::Real(mise.page.0 as f32), Object::Real(mise.page.1 as f32)]);
    for boite in [b"CropBox".as_slice(), b"BleedBox", b"TrimBox", b"ArtBox"] {
        dict.remove(boite);
    }
    let mut sortie = vec![];
    doc.save_to(&mut sortie).map_err(|e| e.to_string())?;
    Ok((sortie, mise))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MM: f64 = PT_PAR_MM;

    #[test]
    fn cadre_aux_cotes_du_manuel() {
        // Image de 100 × 60 mm : plus petite que l'intérieur minimal de 250 × 250 mm.
        let m = mise_en_page(Reperage::Cadre, 100.0 * MM, 60.0 * MM);
        let [bas, haut, gauche, droite] = [m.rectangles[0], m.rectangles[1], m.rectangles[2], m.rectangles[3]];
        let mm = |v: f64| v / MM;
        assert!((mm(bas[3]) - cadre::EPAISSEUR).abs() < 1e-9 && mm(bas[3]) >= 3.0);
        // Intérieur ≥ 250 mm dans les deux sens.
        let int_l = mm(droite[0] - (gauche[0] + gauche[2]));
        let int_h = mm(haut[1] - (bas[1] + bas[3]));
        assert!(int_l >= 250.0 - 1e-9 && int_h >= 250.0 - 1e-9, "{int_l} × {int_h}");
        // Image centrée, à au moins 6 mm du cadre.
        let (ix, iy) = (mm(m.decalage.0), mm(m.decalage.1));
        assert!(ix - mm(gauche[0] + gauche[2]) >= 6.0 && iy - mm(bas[1] + bas[3]) >= 6.0);
        assert!((ix - mm(gauche[0] + gauche[2]) - (mm(droite[0]) - (ix + 100.0))).abs() < 1e-6, "centrée en largeur");
        // Blancs autour du cadre : 20 mm sur les côtés, 25 à l'avant, 75 à l'arrière.
        assert!(mm(gauche[0]) >= 20.0 && mm(m.page.0 - (droite[0] + droite[2])) >= 20.0);
        assert!(mm(bas[1]) >= 25.0 && mm(m.page.1 - (haut[1] + haut[3])) >= 75.0);
    }

    #[test]
    fn grande_image_garde_sa_marge() {
        let m = mise_en_page(Reperage::Cadre, 1000.0 * MM, 600.0 * MM);
        let gauche = m.rectangles[2];
        assert!(((m.decalage.0 - (gauche[0] + gauche[2])) / MM - cadre::MARGE_IMAGE).abs() < 1e-9);
    }

    #[test]
    fn croix_aux_quatre_coins() {
        let m = mise_en_page(Reperage::Manuel, 100.0 * MM, 50.0 * MM);
        assert_eq!(m.rectangles.len(), 8, "deux barres par croix");
        let centres: Vec<(f64, f64)> = m.rectangles.chunks(2).map(|c| (c[0][0] + c[0][2] / 2.0, c[0][1] + c[0][3] / 2.0)).collect();
        // Rectangle des croix = image + 10 mm de chaque côté.
        let l = (centres[1].0 - centres[0].0) / MM;
        let h = (centres[2].1 - centres[1].1) / MM;
        assert!((l - 120.0).abs() < 1e-9 && (h - 70.0).abs() < 1e-9, "{l} × {h}");
    }

    /// Le cadre sort sur la seule plaque noire, et l'image d'origine est décalée d'autant.
    #[test]
    fn rendu_du_cadre_par_ghostscript() {
        let dossier = std::env::temp_dir().join(format!("eko-rip-reperage-{}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        // PDF de 100 × 100 pt avec un carré cyan plein.
        let mut doc = Document::with_version("1.4");
        let contenu = doc.add_object(Stream::new(dictionary! {}, b"1 0 0 0 k 0 0 100 100 re f".to_vec()));
        let pages = doc.new_object_id();
        let page = doc.add_object(dictionary! { "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(), 0.into(), 100.into(), 100.into()], "Contents" => contenu });
        doc.objects.insert(pages, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }));
        let cat = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
        doc.trailer.set("Root", cat);
        let mut octets = vec![];
        doc.save_to(&mut octets).unwrap();

        let (pdf, mise) = envelopper(&octets, Reperage::Cadre).unwrap();
        let chemin = dossier.join("p.pdf");
        std::fs::write(&chemin, &pdf).unwrap();
        let st = std::process::Command::new("gs")
            .args(["-q", "-dSAFER", "-dBATCH", "-dNOPAUSE", "-sDEVICE=tiffsep", "-r18"])
            .arg(format!("-sOutputFile={}", dossier.join("r.tif").display()))
            .arg(&chemin)
            .status()
            .unwrap();
        assert!(st.success());
        let lire = |plaque: &str, x: f64, y: f64| {
            let img = image::open(dossier.join(format!("r({plaque}).tif"))).unwrap().to_luma8();
            let k = 18.0 / 72.0;
            img.get_pixel((x * k) as u32, img.height() - 1 - (y * k) as u32)[0]
        };
        let bas = mise.rectangles[0];
        let (cx, cy) = (bas[0] + bas[2] / 2.0, bas[1] + bas[3] / 2.0);
        assert_eq!(lire("Black", cx, cy), 0, "cadre en noir plein");
        assert_eq!(lire("Cyan", cx, cy), 255, "cadre sans cyan");
        let (ix, iy) = (mise.decalage.0 + 50.0, mise.decalage.1 + 50.0);
        assert_eq!(lire("Cyan", ix, iy), 0, "carré cyan déplacé avec la page");
        assert_eq!(lire("Black", ix, iy), 255);
        std::fs::remove_dir_all(&dossier).unwrap();
    }
}
