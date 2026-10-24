//! Imbrication : plusieurs pages (ou copies d'une page) placées côte à côte sur la laize
//! du rouleau, en un seul PDF. Chaque page source devient un formulaire XObject placé
//! par sa matrice ; le résultat suit ensuite le circuit ordinaire (rendu, repérage,
//! découpe — l'extracteur de découpe lit les formulaires).

use lopdf::{Dictionary, Document, Object, ObjectId, Stream, dictionary};

pub const PT_PAR_MM: f64 = 72.0 / 25.4;

/// Un élément à placer : largeur et hauteur en points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Taille {
    pub largeur: f64,
    pub hauteur: f64,
}

/// Position (coin bas gauche) de l'élément `indice`, en points ; `tourne` : posé à 90°
/// (sa largeur devient sa hauteur).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    pub indice: usize,
    pub x: f64,
    pub y: f64,
    pub tourne: bool,
}

/// Placement en rangées : éléments par hauteur décroissante, de gauche à droite, nouvelle
/// rangée quand la laize est pleine. Avec `rotation`, quatre orientations sont essayées —
/// tel quel, tout tourné, tout couché, tout debout — et la plus courte en longueur de
/// rouleau est gardée ; un élément trop large n'y passe que tourné. Rend les places et la
/// taille de la planche.
// ponytail: rangées simples ; un placement en « skyline » ou sur la forme du contour
// gagnerait du support sur des formats très mélangés.
pub fn placer(tailles: &[Taille], laize: f64, espace: f64, rotation: bool) -> Result<(Vec<Place>, Taille), String> {
    let tient = |l: f64| l <= laize + 1e-6;
    if let Some(i) = tailles.iter().position(|t| !tient(t.largeur) && !(rotation && tient(t.hauteur))) {
        let t = tailles[i];
        return Err(format!("l'élément {} ({:.0} × {:.0} mm) est plus large que la laize ({:.0} mm)", i + 1, t.largeur / PT_PAR_MM, t.hauteur / PT_PAR_MM, laize / PT_PAR_MM));
    }
    // Orientation de chaque élément selon une règle ; un élément trop large est toujours tourné.
    let selon = |regle: &dyn Fn(&Taille) -> bool| -> Vec<bool> { tailles.iter().map(|t| !tient(t.largeur) || (tient(t.hauteur) && regle(t))).collect() };
    let tel_quel = selon(&|_| false);
    let mut meilleur = rangees(tailles, &tel_quel, laize, espace);
    if !rotation {
        return Ok(meilleur);
    }
    // Tout tourné, côté long en travers (couché), côté long dans le sens du rouleau (debout).
    for tourne in [selon(&|_| true), selon(&|t| t.hauteur > t.largeur), selon(&|t| t.largeur > t.hauteur)] {
        let essai = rangees(tailles, &tourne, laize, espace);
        if essai.1.hauteur < meilleur.1.hauteur - 1e-6 {
            meilleur = essai;
        }
    }
    Ok(meilleur)
}

fn rangees(tailles: &[Taille], tourne: &[bool], laize: f64, espace: f64) -> (Vec<Place>, Taille) {
    let pose = |i: usize| if tourne[i] { Taille { largeur: tailles[i].hauteur, hauteur: tailles[i].largeur } } else { tailles[i] };
    let mut ordre: Vec<usize> = (0..tailles.len()).collect();
    ordre.sort_by(|&a, &b| pose(b).hauteur.total_cmp(&pose(a).hauteur));
    let (mut places, mut x, mut y, mut haut_rangee, mut large_max) = (vec![], 0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for i in ordre {
        let t = pose(i);
        if x > 0.0 && x + t.largeur > laize + 1e-6 {
            y += haut_rangee + espace;
            (x, haut_rangee) = (0.0, 0.0);
        }
        places.push(Place { indice: i, x, y, tourne: tourne[i] });
        large_max = large_max.max(x + t.largeur);
        haut_rangee = haut_rangee.max(t.hauteur);
        x += t.largeur + espace;
    }
    (places, Taille { largeur: large_max, hauteur: y + haut_rangee })
}

/// Une page à imbriquer : PDF source, numéro de page (à partir de 1), nombre de copies.
pub struct Source<'a> {
    pub pdf: &'a [u8],
    pub page: usize,
    pub copies: usize,
}

fn nombres(o: &Object) -> Option<Vec<f64>> {
    Some(o.as_array().ok()?.iter().filter_map(|v| v.as_float().ok()).map(f64::from).collect())
}

/// Copie la page `page` (à partir de 1) de `pdf` dans `cible` sous forme de formulaire
/// XObject : rend son identifiant, sa taille et le coin bas gauche de sa MediaBox.
/// Tous les objets de la source passent dans la cible (numéros décalés) ; les inutiles
/// sont retirés par `prune_objects` à la fin de la composition.
pub fn page_en_formulaire(cible: &mut Document, pdf: &[u8], page: usize) -> Result<(ObjectId, Taille, [f64; 2]), String> {
    let mut doc = Document::load_mem(pdf).map_err(|e| format!("PDF illisible : {e}"))?;
    doc.renumber_objects_with(cible.max_id + 1);
    let id_page = *doc.get_pages().get(&(page as u32)).ok_or(format!("pas de page {page}"))?;
    let dict = doc.get_dictionary(id_page).map_err(|e| e.to_string())?;
    let boite = dict.get(b"MediaBox").ok().and_then(nombres).filter(|b| b.len() == 4).ok_or("page sans MediaBox")?;
    if dict.get(b"Rotate").and_then(Object::as_i64).is_ok_and(|r| r % 360 != 0) {
        return Err("page tournée (/Rotate) non prise en charge".into());
    }
    let ressources: Dictionary = match doc.get_page_resources(id_page).map_err(|e| e.to_string())? {
        (Some(d), _) => d.clone(),
        (None, ids) => ids.first().and_then(|id| doc.get_dictionary(*id).ok()).cloned().unwrap_or_default(),
    };
    let contenu = doc.get_page_content(id_page);
    cible.max_id = doc.max_id;
    cible.objects.extend(doc.objects);
    let form = Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Form",
            "BBox" => boite.iter().map(|v| Object::Real(*v as f32)).collect::<Vec<_>>(),
            "Resources" => ressources,
        },
        contenu,
    );
    let id = cible.add_object(form);
    Ok((id, Taille { largeur: boite[2] - boite[0], hauteur: boite[3] - boite[1] }, [boite[0], boite[1]]))
}

/// Compose la planche : rend le PDF et les places (dans l'ordre des copies déroulées).
pub fn composer(sources: &[Source], laize_mm: f64, espace_mm: f64, rotation: bool) -> Result<(Vec<u8>, Vec<Place>), String> {
    let mut cible = Document::with_version("1.6");
    // Formulaires créés, un par source, et leur taille.
    let mut formulaires: Vec<(ObjectId, Taille, [f64; 2])> = vec![];
    for (n, s) in sources.iter().enumerate() {
        formulaires.push(page_en_formulaire(&mut cible, s.pdf, s.page).map_err(|e| format!("source {} : {e}", n + 1))?);
    }
    // Copies déroulées : élément k → formulaire de sa source.
    let elements: Vec<usize> = sources.iter().enumerate().flat_map(|(n, s)| std::iter::repeat_n(n, s.copies)).collect();
    if elements.is_empty() {
        return Err("rien à imbriquer".into());
    }
    let tailles: Vec<Taille> = elements.iter().map(|&n| formulaires[n].1).collect();
    let (places, planche) = placer(&tailles, laize_mm * PT_PAR_MM, espace_mm * PT_PAR_MM, rotation)?;

    let mut xobjets = Dictionary::new();
    let mut contenu = String::new();
    for (n, (id, _, _)) in formulaires.iter().enumerate() {
        xobjets.set(format!("F{n}"), *id);
    }
    for p in &places {
        let n = elements[p.indice];
        let ([llx, lly], h) = (formulaires[n].2, formulaires[n].1.hauteur);
        contenu += &if p.tourne {
            // Quart de tour à gauche, (x, y) → (−y, x) : le bas de la page se pose à droite.
            format!("q 0 1 -1 0 {:.4} {:.4} cm /F{n} Do Q\n", p.x + lly + h, p.y - llx)
        } else {
            format!("q 1 0 0 1 {:.4} {:.4} cm /F{n} Do Q\n", p.x - llx, p.y - lly)
        };
    }
    let flux = cible.add_object(Stream::new(dictionary! {}, contenu.into_bytes()));
    let pages = cible.new_object_id();
    let page = cible.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages,
        "MediaBox" => vec![0.into(), 0.into(), Object::Real(planche.largeur as f32), Object::Real(planche.hauteur as f32)],
        "Contents" => flux, "Resources" => dictionary! { "XObject" => xobjets },
    });
    cible.objects.insert(pages, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }));
    let catalogue = cible.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    cible.trailer.set("Root", catalogue);
    cible.prune_objects();
    let mut sortie = vec![];
    cible.save_to(&mut sortie).map_err(|e| e.to_string())?;
    Ok((sortie, places))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    const MM: f64 = PT_PAR_MM;

    fn t(l: f64, h: f64) -> Taille {
        Taille { largeur: l * MM, hauteur: h * MM }
    }

    #[test]
    fn placement_dans_la_laize_sans_chevauchement() {
        let tailles = [t(300.0, 200.0), t(500.0, 100.0), t(400.0, 300.0), t(300.0, 200.0), t(600.0, 150.0)];
        let (places, planche) = placer(&tailles, 1000.0 * MM, 5.0 * MM, false).unwrap();
        assert_eq!(places.len(), tailles.len());
        assert!(planche.largeur <= 1000.0 * MM + 1e-6);
        for (i, a) in places.iter().enumerate() {
            let ta = tailles[a.indice];
            assert!(a.x >= -1e-9 && a.x + ta.largeur <= 1000.0 * MM + 1e-6, "dans la laize");
            for b in &places[i + 1..] {
                let tb = tailles[b.indice];
                let separes = a.x + ta.largeur <= b.x + 1e-6 || b.x + tb.largeur <= a.x + 1e-6 || a.y + ta.hauteur <= b.y + 1e-6 || b.y + tb.hauteur <= a.y + 1e-6;
                assert!(separes, "chevauchement entre {} et {}", a.indice, b.indice);
            }
        }
        // Les deux plus hauts (300 et 200 mm) partagent la première rangée avec un 300 mm de large.
        assert_eq!(places[0].indice, 2);
        assert!(planche.hauteur < (300.0 + 200.0 + 200.0 + 150.0 + 100.0) * MM, "rangées partagées : {} mm", planche.hauteur / MM);
    }

    #[test]
    fn rotation_quand_elle_raccourcit_la_planche() {
        // 3 copies de 200 × 100 mm sur 320 mm : tel quel une par rangée (310 mm de long),
        // couchées (100 × 200) trois côte à côte (200 mm).
        let tailles = [t(200.0, 100.0); 3];
        let (_, sans) = placer(&tailles, 320.0 * MM, 5.0 * MM, false).unwrap();
        let (places, avec) = placer(&tailles, 320.0 * MM, 5.0 * MM, true).unwrap();
        assert!((sans.hauteur / MM - 310.0).abs() < 1e-6 && (avec.hauteur / MM - 200.0).abs() < 1e-6);
        assert!(places.iter().all(|p| p.tourne));
        // Un format plus large que la laize n'y passe que tourné ; sans rotation, refus.
        let (places, _) = placer(&[t(400.0, 100.0)], 320.0 * MM, 5.0 * MM, true).unwrap();
        assert!(places[0].tourne);
        assert!(placer(&[t(400.0, 100.0)], 320.0 * MM, 5.0 * MM, false).is_err());
        // Rien à gagner : pas de rotation.
        let (places, _) = placer(&[t(100.0, 50.0); 2], 320.0 * MM, 5.0 * MM, true).unwrap();
        assert!(places.iter().all(|p| !p.tourne));
    }

    /// Un élément tourné est rendu et découpé dans sa case : une page magenta de 60 × 40 pt
    /// (MediaBox décalée) couchée en 40 × 60 sur une laize de 50 pt.
    #[test]
    fn element_tourne_rendu_et_decoupe_en_place() {
        let magenta = pdf(60.0, 40.0, "0 1 0 0", (50.0, 30.0));
        let sources = [Source { pdf: &magenta, page: 1, copies: 1 }];
        let (planche, places) = composer(&sources, 50.0 / PT_PAR_MM, 0.0, true).unwrap();
        assert!(places[0].tourne);
        let d = crate::decoupe::extraire(&planche, false).unwrap();
        assert!((d[0].largeur - 40.0).abs() < 1e-3 && (d[0].hauteur - 60.0).abs() < 1e-3, "{} × {}", d[0].largeur, d[0].hauteur);
        let (xs, ys): (Vec<f64>, Vec<f64>) = d[0].traces[0].points.iter().copied().unzip();
        let borne = |v: &[f64]| (v.iter().copied().fold(f64::MAX, f64::min), v.iter().copied().fold(f64::MIN, f64::max));
        // Contour à 5 pt du bord de la page source : 5..35 en x, 5..55 en y une fois couché.
        let ((x0, x1), (y0, y1)) = (borne(&xs), borne(&ys));
        assert!((x0 - 5.0).abs() < 1e-3 && (x1 - 35.0).abs() < 1e-3 && (y0 - 5.0).abs() < 1e-3 && (y1 - 55.0).abs() < 1e-3, "{x0} {x1} {y0} {y1}");
        let dossier = std::env::temp_dir().join(format!("eko-rip-rotation-{}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        std::fs::write(dossier.join("p.pdf"), &planche).unwrap();
        let st = std::process::Command::new("gs")
            .args(["-q", "-dSAFER", "-dBATCH", "-dNOPAUSE", "-sDEVICE=tiffsep", "-r72"])
            .arg(format!("-sOutputFile={}", dossier.join("r.tif").display()))
            .arg(dossier.join("p.pdf"))
            .status()
            .unwrap();
        assert!(st.success());
        let m = image::open(dossier.join("r(Magenta).tif")).unwrap().to_luma8();
        assert_eq!(m.dimensions(), (40, 60));
        assert_eq!((m.get_pixel(20, 30)[0], m.get_pixel(2, 2)[0], m.get_pixel(37, 57)[0]), (0, 0, 0), "toute la case est magenta");
        std::fs::remove_dir_all(&dossier).unwrap();
    }

    #[test]
    fn trop_large_refuse() {
        let e = placer(&[t(1200.0, 100.0)], 1000.0 * MM, 5.0 * MM, false).unwrap_err();
        assert!(e.contains("plus large que la laize"), "{e}");
    }

    /// PDF d'une page `l × h` pt dont la MediaBox commence en (`ox`, `oy`), remplie d'une
    /// couleur CMJN, avec un contour `CutContour` à 5 pt du bord.
    pub(crate) fn pdf(l: f64, h: f64, cmjn: &str, (ox, oy): (f64, f64)) -> Vec<u8> {
        let mut doc = Document::with_version("1.4");
        let sep = Object::Array(vec![
            "Separation".into(),
            "CutContour".into(),
            "DeviceCMYK".into(),
            Object::Dictionary(dictionary! { "FunctionType" => 2, "Domain" => vec![0.into(), 1.into()], "C0" => vec![0.into(); 4], "C1" => vec![0.into(), 1.into(), 0.into(), 0.into()], "N" => 1 }),
        ]);
        let contenu = format!("{cmjn} k {ox} {oy} {l} {h} re f /CS0 CS 1 SCN {} {} {} {} re S", ox + 5.0, oy + 5.0, l - 10.0, h - 10.0);
        let c = doc.add_object(Stream::new(dictionary! {}, contenu.into_bytes()));
        let pages = doc.new_object_id();
        let page = doc.add_object(dictionary! { "Type" => "Page", "Parent" => pages, "MediaBox" => vec![Object::Real(ox as f32), Object::Real(oy as f32), Object::Real((ox + l) as f32), Object::Real((oy + h) as f32)], "Contents" => c, "Resources" => dictionary! { "ColorSpace" => dictionary! { "CS0" => sep } } });
        doc.objects.insert(pages, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }));
        let cat = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
        doc.trailer.set("Root", cat);
        let mut o = vec![];
        doc.save_to(&mut o).unwrap();
        o
    }

    #[test]
    fn planche_rendue_et_decoupee_aux_bonnes_places() {
        let cyan = pdf(100.0, 100.0, "1 0 0 0", (0.0, 0.0));
        // MediaBox décalée, comme souvent à l'export d'Illustrator.
        let magenta = pdf(60.0, 40.0, "0 1 0 0", (50.0, 30.0));
        let sources = [Source { pdf: &cyan, page: 1, copies: 1 }, Source { pdf: &magenta, page: 1, copies: 2 }];
        // Laize 80 mm ≈ 226,8 pt : cyan (100) + magenta (60) + espace tiennent en rangée 1 ; le 2e magenta passe en rangée 2.
        let (planche, places) = composer(&sources, 80.0, 5.0, false).unwrap();
        assert_eq!(places.len(), 3);

        // Découpe : trois contours, chacun dans son élément placé.
        let d = crate::decoupe::extraire(&planche, false).unwrap().remove(0);
        assert_eq!(d.traces.len(), 3);
        let elements = [0usize, 1, 1];
        for p in &places {
            let (l, h) = if elements[p.indice] == 0 { (100.0, 100.0) } else { (60.0, 40.0) };
            let attendu = (p.x + 5.0, p.y + 5.0);
            let trouve = d.traces.iter().any(|t| (t.points[0].0 - attendu.0).abs() < 1e-3 && (t.points[0].1 - attendu.1).abs() < 1e-3 && (t.points[2].0 - (p.x + l - 5.0)).abs() < 1e-3 && (t.points[2].1 - (p.y + h - 5.0)).abs() < 1e-3);
            assert!(trouve, "contour de l'élément {} absent ou mal placé", p.indice);
        }

        // Rendu : la couleur de chaque élément est à sa place.
        let dossier = std::env::temp_dir().join(format!("eko-rip-planche-{}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        std::fs::write(dossier.join("p.pdf"), &planche).unwrap();
        let st = std::process::Command::new("gs")
            .args(["-q", "-dSAFER", "-dBATCH", "-dNOPAUSE", "-sDEVICE=tiffsep", "-r36"])
            .arg(format!("-sOutputFile={}", dossier.join("r.tif").display()))
            .arg(dossier.join("p.pdf"))
            .status()
            .unwrap();
        assert!(st.success());
        let lire = |plaque: &str, x: f64, y: f64| {
            let img = image::open(dossier.join(format!("r({plaque}).tif"))).unwrap().to_luma8();
            img.get_pixel((x / 2.0) as u32, img.height() - 1 - (y / 2.0) as u32)[0]
        };
        for p in &places {
            let (cx, cy, plaque) = if elements[p.indice] == 0 { (p.x + 50.0, p.y + 50.0, "Cyan") } else { (p.x + 30.0, p.y + 20.0, "Magenta") };
            assert_eq!(lire(plaque, cx, cy), 0, "{plaque} absent au centre de l'élément {}", p.indice);
        }
        std::fs::remove_dir_all(&dossier).unwrap();
    }
}
