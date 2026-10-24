//! Mosaïque : une page plus large que la laize est coupée en lés verticaux de même
//! largeur (au plus la laize), qui se recouvrent pour le collage ou le soudage. Chaque lé
//! devient une page du PDF ; le travail suit ensuite le circuit multipage ordinaire, et
//! la découpe est rognée au bord de chaque lé.

use crate::imbrication::{PT_PAR_MM, page_en_formulaire};
use lopdf::{Dictionary, Document, Object, Stream, dictionary};

/// Bornes en x (points) des lés d'une page de largeur `largeur` : `n` lés de même
/// largeur `(W + (n − 1)·r) / n ≤ laize`, chacun recouvrant le suivant de `r`.
pub fn les(largeur: f64, laize: f64, recouvrement: f64) -> Result<Vec<(f64, f64)>, String> {
    if recouvrement < 0.0 || recouvrement >= laize {
        return Err("le recouvrement doit être positif et plus petit que la laize".into());
    }
    if largeur <= laize + 1e-6 {
        return Ok(vec![(0.0, largeur)]);
    }
    let n = ((largeur - recouvrement) / (laize - recouvrement)).ceil() as usize;
    let l = (largeur + (n - 1) as f64 * recouvrement) / n as f64;
    Ok((0..n).map(|i| {
        let x0 = i as f64 * (l - recouvrement);
        (x0, x0 + l)
    }).collect())
}

/// Réécrit le PDF : chaque page plus large que la laize est remplacée par ses lés.
pub fn composer(pdf: &[u8], laize_mm: f64, recouvrement_mm: f64) -> Result<Vec<u8>, String> {
    let nb_pages = Document::load_mem(pdf).map_err(|e| format!("PDF illisible : {e}"))?.get_pages().len();
    let mut cible = Document::with_version("1.6");
    let pages_id = cible.new_object_id();
    let mut enfants = vec![];
    for n in 1..=nb_pages {
        let (form, taille, [llx, lly]) = page_en_formulaire(&mut cible, pdf, n).map_err(|e| format!("page {n} : {e}"))?;
        for (x0, x1) in les(taille.largeur, laize_mm * PT_PAR_MM, recouvrement_mm * PT_PAR_MM)? {
            let contenu = format!("q 1 0 0 1 {:.4} {:.4} cm /F Do Q\n", -x0 - llx, -lly);
            let flux = cible.add_object(Stream::new(dictionary! {}, contenu.into_bytes()));
            let mut xobjets = Dictionary::new();
            xobjets.set("F", form);
            enfants.push(Object::Reference(cible.add_object(dictionary! {
                "Type" => "Page", "Parent" => pages_id,
                "MediaBox" => vec![0.into(), 0.into(), Object::Real((x1 - x0) as f32), Object::Real(taille.hauteur as f32)],
                "Contents" => flux, "Resources" => dictionary! { "XObject" => xobjets },
            })));
        }
    }
    let compte = enfants.len() as i64;
    cible.objects.insert(pages_id, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => enfants, "Count" => compte }));
    let catalogue = cible.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    cible.trailer.set("Root", catalogue);
    cible.prune_objects();
    let mut sortie = vec![];
    cible.save_to(&mut sortie).map_err(|e| e.to_string())?;
    Ok(sortie)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MM: f64 = PT_PAR_MM;

    #[test]
    fn les_de_meme_largeur_avec_recouvrement() {
        // 3000 mm sur 1600 mm avec 20 mm : 2 lés de 1510 mm, qui se recouvrent de 20 mm.
        let l = les(3000.0 * MM, 1600.0 * MM, 20.0 * MM).unwrap();
        assert_eq!(l.len(), 2);
        for (x0, x1) in &l {
            assert!(((x1 - x0) / MM - 1510.0).abs() < 1e-6 && x1 - x0 <= 1600.0 * MM);
        }
        assert!((l[0].1 - l[1].0 - 20.0 * MM).abs() < 1e-6, "recouvrement de 20 mm");
        assert!((l[1].1 - 3000.0 * MM).abs() < 1e-6, "le dernier lé finit au bord");
        assert_eq!(les(1000.0 * MM, 1600.0 * MM, 20.0 * MM).unwrap(), [(0.0, 1000.0 * MM)], "page plus étroite : un seul lé");
        assert!(les(3000.0 * MM, 1600.0 * MM, 1600.0 * MM).is_err());
    }

    /// Page de 3000 × 300 mm : moitié gauche cyan, moitié droite magenta, contour
    /// `CutContour` à 10 mm du bord ; MediaBox décalée comme à l'export d'Illustrator.
    fn banniere() -> Vec<u8> {
        let (l, h, o) = (3000.0 * MM, 300.0 * MM, 40.0);
        let mut doc = Document::with_version("1.4");
        let sep = Object::Array(vec![
            "Separation".into(),
            "CutContour".into(),
            "DeviceCMYK".into(),
            Object::Dictionary(dictionary! { "FunctionType" => 2, "Domain" => vec![0.into(), 1.into()], "C0" => vec![0.into(); 4], "C1" => vec![0.into(), 1.into(), 0.into(), 0.into()], "N" => 1 }),
        ]);
        let m = 10.0 * MM;
        let contenu = format!(
            "1 0 0 0 k {o} {o} {} {h} re f 0 1 0 0 k {} {o} {} {h} re f /CS0 CS 1 SCN {} {} {} {} re S",
            l / 2.0, o + l / 2.0, l / 2.0, o + m, o + m, l - 2.0 * m, h - 2.0 * m
        );
        let c = doc.add_object(Stream::new(dictionary! {}, contenu.into_bytes()));
        let pages = doc.new_object_id();
        let page = doc.add_object(dictionary! { "Type" => "Page", "Parent" => pages, "MediaBox" => vec![Object::Real(o as f32), Object::Real(o as f32), Object::Real((o + l) as f32), Object::Real((o + h) as f32)], "Contents" => c, "Resources" => dictionary! { "ColorSpace" => dictionary! { "CS0" => sep } } });
        doc.objects.insert(pages, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }));
        let cat = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
        doc.trailer.set("Root", cat);
        let mut v = vec![];
        doc.save_to(&mut v).unwrap();
        v
    }

    #[test]
    fn banniere_en_deux_les_rendus_et_decoupes() {
        let pdf = composer(&banniere(), 1600.0, 20.0).unwrap();
        // Découpe : chaque lé garde sa part du contour, rognée à son bord.
        let pages = crate::decoupe::extraire(&pdf, false).unwrap();
        assert_eq!(pages.len(), 2);
        let le = 1510.0 * MM;
        for (i, p) in pages.iter().enumerate() {
            assert!((p.largeur - le).abs() < 1e-3, "largeur du lé {}", i + 1);
            assert_eq!(p.traces.len(), 1, "lé {} : un morceau de contour, raccordé", i + 1);
            let xs: Vec<f64> = p.traces[0].points.iter().map(|q| q.0).collect();
            assert!(xs.iter().all(|x| *x >= -1e-6 && *x <= le + 1e-6), "contour dans le lé");
        }
        // Lé 1 : le côté gauche du contour (à 10 mm) ; lé 2 : le côté droit (10 mm avant la fin).
        assert!(pages[0].traces[0].points.iter().any(|q| (q.0 - 10.0 * MM).abs() < 1e-3));
        assert!(pages[1].traces[0].points.iter().any(|q| (q.0 - (le - 10.0 * MM)).abs() < 1e-3));

        // Rendu : lé 1 cyan à gauche, lé 2 magenta à droite ; le recouvrement (x 1490–1510 mm
        // de la bannière) est imprimé dans les deux lés.
        let dossier = std::env::temp_dir().join(format!("eko-rip-mosaique-{}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        std::fs::write(dossier.join("m.pdf"), &pdf).unwrap();
        let st = std::process::Command::new("gs")
            .args(["-q", "-dSAFER", "-dBATCH", "-dNOPAUSE", "-sDEVICE=tiffsep", "-r9"])
            .arg(format!("-sOutputFile={}", dossier.join("r-%d.tif").display()))
            .arg(dossier.join("m.pdf"))
            .status()
            .unwrap();
        assert!(st.success());
        let lire = |page: usize, plaque: &str, x_mm: f64| {
            let img = image::open(dossier.join(format!("r-{page}({plaque}).tif"))).unwrap().to_luma8();
            img.get_pixel(((x_mm * MM) * 9.0 / 72.0) as u32, img.height() / 2)[0]
        };
        assert_eq!(lire(1, "Cyan", 100.0), 0, "lé 1 : cyan à gauche");
        assert_eq!(lire(2, "Magenta", 1400.0), 0, "lé 2 : magenta à droite");
        // Bannière x = 1500 mm (dans le recouvrement) : magenta, en x = 1500 dans le lé 1 et
        // en x = 1500 − 1490 = 10 mm dans le lé 2.
        assert_eq!(lire(1, "Magenta", 1505.0), 0, "recouvrement présent en fin de lé 1");
        assert_eq!(lire(2, "Magenta", 15.0), 0, "recouvrement présent en début de lé 2");
        std::fs::remove_dir_all(&dossier).unwrap();
    }
}
