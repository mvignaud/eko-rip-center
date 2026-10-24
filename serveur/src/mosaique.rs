//! Mosaïque : une page plus grande que la laize (ou que la hauteur maximale) est coupée
//! en tuiles — colonnes × rangées — qui se recouvrent pour le collage ou le soudage. Lés
//! égaux par défaut, ou inégaux (largeurs et hauteurs saisies) ; recouvrement global ou
//! propre à une tuile (négatif : espace entre deux tuiles). Chaque tuile imprimée devient une
//! page du PDF, dans l'ordre choisi ; le travail suit ensuite le circuit multipage ordinaire,
//! et la découpe est rognée au bord de chaque tuile.

use crate::imbrication::{PT_PAR_MM, page_en_formulaire};
use crate::reglages::Mosaique;
use lopdf::{Dictionary, Document, Object, Stream, dictionary};
use serde::{Deserialize, Serialize};

/// Une tuile du PDF produit, pour la visionneuse.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tuile {
    /// Numéro, de haut-gauche à bas-droite (rangée par rangée), de 1 à `total`.
    pub numero: usize,
    pub total: usize,
    /// Colonne et rangée (de 1), sur `colonnes` × `rangees`.
    pub colonne: usize,
    pub rangee: usize,
    pub colonnes: usize,
    pub rangees: usize,
    /// Page du PDF d'origine (de 1).
    pub page_source: usize,
    /// Place sur la page d'origine, en mm, origine en bas à gauche : x0, y0, x1, y1.
    pub boite_mm: [f64; 4],
    /// Format de la page d'origine (éditée), en mm.
    pub source_mm: [f64; 2],
    /// Recouvrement avec la voisine de chaque bord (gauche, bas, droite, haut), en mm ;
    /// 0 sans voisine, négatif pour un espace.
    pub recouvrement_mm: [f64; 4],
    /// Bande de repères sous la tuile, en mm (0 sans repères).
    pub bande_mm: f64,
    /// Décalage de la tuile sur la page rendue (repérage), en mm : rempli au rendu.
    #[serde(default)]
    pub decalage_mm: [f64; 2],
}

/// Tuile de chaque page produite (rien pour une page qui tenait).
pub type Tuiles = Vec<Option<Tuile>>;

/// Bornes (début, fin), en mm, des tuiles le long d'un côté de `total` mm : les tailles
/// `saisies` (leur somme vaut `total` plus un recouvrement par jonction), sinon `n` tuiles
/// égales, sinon le moins de tuiles égales d'au plus `max`. Chaque tuile recouvre la
/// suivante de `r`. Erreur : le détail, non traduit.
pub fn axe(total: f64, max: Option<f64>, r: f64, n: Option<usize>, saisies: Option<&[f64]>) -> Result<Vec<(f64, f64)>, String> {
    let tailles = match (saisies, n) {
        (Some(s), _) => {
            let attendu = total + s.len().saturating_sub(1) as f64 * r;
            let somme: f64 = s.iter().sum();
            if s.is_empty() || (somme - attendu).abs() > 0.1 {
                return Err(format!("somme des tuiles {somme:.1} mm, attendu {attendu:.1} mm"));
            }
            s.to_vec()
        }
        (None, Some(n)) if n > 1 => vec![(total + (n - 1) as f64 * r) / n as f64; n],
        _ => match max {
            Some(m) if total > m + 1e-6 && n.is_none() => {
                if r >= m {
                    return Err("le recouvrement doit être plus petit que la laize".into());
                }
                let n = ((total - r) / (m - r)).ceil() as usize;
                vec![(total + (n - 1) as f64 * r) / n as f64; n]
            }
            _ => vec![total],
        },
    };
    if let Some(&t) = tailles.iter().find(|&&t| (tailles.len() > 1 && t <= r) || max.is_some_and(|m| t > m + 0.01)) {
        return Err(format!("tuile de {t:.1} mm : plus grande que le maximum ou pas plus grande que le recouvrement"));
    }
    let mut x = 0.0;
    let mut bornes: Vec<(f64, f64)> = tailles.iter().map(|t| {
        let b = (x, x + t);
        x += t - r;
        b
    }).collect();
    // La dernière finit au bord, au centième près des tailles saisies.
    if let Some(d) = bornes.last_mut() {
        d.1 = total;
    }
    Ok(bornes)
}

/// Tuiles d'une page de `l` × `h` mm, dans l'ordre des numéros. Rien à couper : une seule.
pub fn tuiles(l: f64, h: f64, m: &Mosaique) -> Result<Vec<Tuile>, String> {
    let r = m.recouvrement_mm;
    let colonnes = axe(l, Some(m.laize_mm), r, m.colonnes, m.les_mm.as_deref())?;
    // Les rangées se comptent du haut ; y du PDF part du bas.
    let rangees = axe(h, m.hauteur_max_mm, r, m.rangees, m.rangees_mm.as_deref())?;
    let (nc, nr) = (colonnes.len(), rangees.len());
    let mut v = vec![];
    for (j, &(t0, t1)) in rangees.iter().enumerate() {
        for (i, &(x0, x1)) in colonnes.iter().enumerate() {
            let numero = j * nc + i + 1;
            let mut b = [x0, h - t1, x1, h - t0];
            // Recouvrement propre : la tuile s'étend (ou recule) de la moitié de l'écart avec
            // le recouvrement global, sur ses bords intérieurs seulement.
            if let Some(Some(propre)) = m.recouvrement_tuiles_mm.get(numero - 1) {
                let d = (propre - r) / 2.0;
                if i > 0 { b[0] -= d; }
                if i + 1 < nc { b[2] += d; }
                if j + 1 < nr { b[1] -= d; }
                if j > 0 { b[3] += d; }
            }
            let (bl, bh) = (b[2] - b[0], b[3] - b[1]);
            if bl <= 0.0 || bh <= 0.0 || bl > m.laize_mm + 0.01 || m.hauteur_max_mm.is_some_and(|hm| bh > hm + 0.01) {
                return Err(format!("tuile {numero} de {bl:.1} × {bh:.1} mm : vide ou plus grande que le maximum"));
            }
            v.push(Tuile { numero, total: nc * nr, colonne: i + 1, rangee: j + 1, colonnes: nc, rangees: nr, page_source: 1, boite_mm: b, source_mm: [l, h], recouvrement_mm: [0.0; 4], bande_mm: 0.0, decalage_mm: [0.0; 2] });
        }
    }
    // Recouvrement de chaque bord avec la voisine (gauche, bas, droite, haut).
    for k in 0..v.len() {
        let (i, j) = (k % nc, k / nc);
        let b = v[k].boite_mm;
        let mut rec = [0.0; 4];
        if i > 0 { rec[0] = v[k - 1].boite_mm[2] - b[0]; }
        if j + 1 < nr { rec[1] = v[k + nc].boite_mm[3] - b[1]; }
        if i + 1 < nc { rec[2] = b[2] - v[k + 1].boite_mm[0]; }
        if j > 0 { rec[3] = b[3] - v[k - nc].boite_mm[1]; }
        v[k].recouvrement_mm = rec;
    }
    Ok(v)
}

/// Numéros des tuiles à imprimer, dans l'ordre : ceux de `ordre` d'abord, puis les autres
/// par numéro ; les décochées (`exclus`) en sont retirées.
fn a_imprimer(total: usize, m: &Mosaique) -> Result<Vec<usize>, String> {
    if let Some(&k) = m.ordre.iter().chain(&m.exclus).find(|&&k| k == 0 || k > total) {
        return Err(format!("tuile {k} : il n'y en a que {total}"));
    }
    if let Some(k) = m.ordre.iter().enumerate().find(|(i, k)| m.ordre[..*i].contains(k)).map(|(_, k)| k) {
        return Err(format!("tuile {k} deux fois dans l'ordre"));
    }
    let v: Vec<usize> = m.ordre.iter().copied().chain((1..=total).filter(|k| !m.ordre.contains(k))).filter(|k| !m.exclus.contains(k)).collect();
    if v.is_empty() {
        return Err("aucune tuile à imprimer".into());
    }
    Ok(v)
}

/// Hauteur de la bande de repères, en mm.
pub const BANDE_MM: f64 = 10.0;

/// Bande de pied d'une tuile (en noir) : son numéro et un trait vertical à chaque bord de
/// recouvrement gauche ou droit, là où la voisine s'arrête ou commence, pour aligner à la pose.
fn reperes(t: &Tuile, largeur: f64) -> String {
    let b = BANDE_MM * PT_PAR_MM;
    let mut s = String::from("q 0 0 0 1 K 0 0 0 1 k 0.5 w ");
    let (g, d) = (t.recouvrement_mm[0] * PT_PAR_MM, t.recouvrement_mm[2] * PT_PAR_MM);
    for x in [(g > 0.0).then_some(g), (d > 0.0).then_some(largeur - d)].into_iter().flatten() {
        s += &format!("{x:.4} 0 m {x:.4} {b:.4} l S ");
    }
    // « Lé » en WinAnsi : é = \351.
    s += &format!("BT /H 8 Tf {:.4} {:.4} Td (L\\351 {}/{}) Tj ET Q\n", (g.max(0.0) + 4.0).min(largeur / 2.0), b / 2.0 - 3.0, t.numero, t.total);
    s
}

/// Réécrit le PDF : chaque page à couper est remplacée par ses tuiles à imprimer, dans
/// l'ordre choisi. Rend aussi, pour chaque page produite, sa tuile (rien pour une page qui
/// tenait). Erreur : (code, détail).
pub fn composer(pdf: &[u8], m: &Mosaique) -> Result<(Vec<u8>, Tuiles), (&'static str, String)> {
    let illisible = |e: String| ("mosaique_invalide", e);
    let nb_pages = Document::load_mem(pdf).map_err(|e| illisible(format!("PDF illisible : {e}")))?.get_pages().len();
    let mut cible = Document::with_version("1.6");
    let pages_id = cible.new_object_id();
    let police = cible.add_object(dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "Encoding" => "WinAnsiEncoding" });
    // Ton de découpe des coupes au milieu des recouvrements.
    let ton = cible.add_object(Object::Array(vec![
        "Separation".into(),
        "CutContour".into(),
        "DeviceCMYK".into(),
        Object::Dictionary(dictionary! { "FunctionType" => 2, "Domain" => vec![0.into(), 1.into()], "C0" => vec![0.into(); 4], "C1" => vec![0.into(), 1.into(), 0.into(), 0.into()], "N" => 1 }),
    ]));
    let (mut enfants, mut produites) = (vec![], vec![]);
    let mm = PT_PAR_MM;
    for n in 1..=nb_pages {
        let (form, taille, [llx, lly]) = page_en_formulaire(&mut cible, pdf, n).map_err(|e| illisible(format!("page {n} : {e}")))?;
        let mut decoupe = tuiles(taille.largeur / mm, taille.hauteur / mm, m).map_err(|e| ("les_incoherents", format!("page {n} : {e}")))?;
        // Une page qui tient n'est pas une tuile : ni bande, ni choix, ni ordre.
        let seule = decoupe.len() == 1;
        let ordre = if seule { vec![1] } else { a_imprimer(decoupe.len(), m).map_err(|e| ("les_incoherents", format!("page {n} : {e}")))? };
        let bande = if m.reperes && !seule { BANDE_MM } else { 0.0 };
        for k in ordre {
            let t = &mut decoupe[k - 1];
            (t.page_source, t.bande_mm) = (n, bande);
            let [x0, y0, x1, y1] = t.boite_mm.map(|v| v * mm);
            let (l, h, b) = (x1 - x0, y1 - y0, bande * mm);
            let [g, bas, d, haut] = t.recouvrement_mm.map(|v| v * mm);
            // Soudure : le recouvrement de gauche et du haut (celui qui passe sous la voisine)
            // reste sans encre ; l'image est rognée à côté.
            let (cg, ch) = if m.soudure { (g.max(0.0), haut.max(0.0)) } else { (0.0, 0.0) };
            let mut contenu = format!("q {cg:.4} {b:.4} {:.4} {:.4} re W n 1 0 0 1 {:.4} {:.4} cm /F Do Q\n", l - cg, h - ch, -x0 - llx, b - y0 - lly);
            if m.coupe_milieu {
                contenu += "q /CC CS 1 SCN 0.5 w ";
                for x in [(g > 0.0).then_some(g / 2.0), (d > 0.0).then_some(l - d / 2.0)].into_iter().flatten() {
                    contenu += &format!("{x:.4} {b:.4} m {x:.4} {:.4} l S ", b + h);
                }
                for y in [(bas > 0.0).then_some(bas / 2.0), (haut > 0.0).then_some(h - haut / 2.0)].into_iter().flatten() {
                    contenu += &format!("0 {:.4} m {l:.4} {:.4} l S ", b + y, b + y);
                }
                contenu += "Q\n";
            }
            if bande > 0.0 {
                contenu += &reperes(t, l);
            }
            let flux = cible.add_object(Stream::new(dictionary! {}, contenu.into_bytes()));
            let mut xobjets = Dictionary::new();
            xobjets.set("F", form);
            enfants.push(Object::Reference(cible.add_object(dictionary! {
                "Type" => "Page", "Parent" => pages_id,
                "MediaBox" => vec![0.into(), 0.into(), Object::Real(l as f32), Object::Real((h + b) as f32)],
                "Contents" => flux,
                "Resources" => dictionary! { "XObject" => xobjets, "Font" => dictionary! { "H" => police }, "ColorSpace" => dictionary! { "CC" => ton } },
            })));
            produites.push((!seule).then(|| t.clone()));
        }
    }
    let compte = enfants.len() as i64;
    cible.objects.insert(pages_id, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => enfants, "Count" => compte }));
    let catalogue = cible.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    cible.trailer.set("Root", catalogue);
    cible.prune_objects();
    let mut sortie = vec![];
    cible.save_to(&mut sortie).map_err(|e| illisible(e.to_string()))?;
    Ok((sortie, produites))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MM: f64 = PT_PAR_MM;

    fn m(laize_mm: f64, recouvrement_mm: f64, reperes: bool) -> Mosaique {
        Mosaique { laize_mm, recouvrement_mm, reperes, ..Default::default() }
    }

    #[test]
    fn les_de_meme_largeur_avec_recouvrement() {
        // 3000 mm sur 1600 mm avec 20 mm : 2 lés de 1510 mm, qui se recouvrent de 20 mm.
        let l = axe(3000.0, Some(1600.0), 20.0, None, None).unwrap();
        assert_eq!(l.len(), 2);
        for (x0, x1) in &l {
            assert!((x1 - x0 - 1510.0).abs() < 1e-6);
        }
        assert!((l[0].1 - l[1].0 - 20.0).abs() < 1e-6, "recouvrement de 20 mm");
        assert!((l[1].1 - 3000.0).abs() < 1e-6, "le dernier lé finit au bord");
        assert_eq!(axe(1000.0, Some(1600.0), 20.0, None, None).unwrap(), [(0.0, 1000.0)], "page plus étroite : un seul lé");
        assert!(axe(3000.0, Some(1600.0), 1600.0, None, None).is_err());
    }

    #[test]
    fn les_inegaux_ordre_et_choix() {
        // Largeurs saisies : leur somme vaut 3000 + 2 × 20.
        let l = axe(3000.0, Some(1600.0), 20.0, None, Some(&[1000.0, 1500.0, 540.0])).unwrap();
        assert_eq!(l, [(0.0, 1000.0), (980.0, 2480.0), (2460.0, 3000.0)]);
        assert!(axe(3000.0, Some(1600.0), 20.0, None, Some(&[1000.0, 1500.0, 500.0])).is_err(), "somme fausse");
        assert!(axe(3000.0, Some(1600.0), 20.0, None, Some(&[1000.0, 1700.0, 340.0])).is_err(), "plus large que la laize");
        assert!(axe(3000.0, Some(1600.0), 20.0, None, Some(&[1520.0, 1520.0, 0.0])).is_err(), "lé vide");
        // Ordre partiel puis le reste ; un décoché disparaît ; doublon ou hors plage refusés.
        let o = |ordre: Vec<usize>, exclus: Vec<usize>| a_imprimer(4, &Mosaique { ordre, exclus, ..Default::default() });
        assert_eq!(o(vec![3, 1], vec![]).unwrap(), [3, 1, 2, 4]);
        assert_eq!(o(vec![3, 1], vec![1]).unwrap(), [3, 2, 4]);
        assert!(o(vec![3, 3], vec![]).is_err() && o(vec![5], vec![]).is_err() && o(vec![], vec![1, 2, 3, 4]).is_err());
    }

    #[test]
    fn tuiles_en_grille_et_recouvrement_propre() {
        // 3000 × 2000 mm, 3 colonnes × 2 rangées, 20 mm : 1013,33 × 1010 mm.
        let g = Mosaique { colonnes: Some(3), rangees: Some(2), ..m(1600.0, 20.0, false) };
        let t = tuiles(3000.0, 2000.0, &g).unwrap();
        assert_eq!(t.len(), 6);
        // Tuile 1 : en haut à gauche ; tuile 4 : en bas à gauche.
        assert_eq!((t[0].colonne, t[0].rangee, t[3].colonne, t[3].rangee), (1, 1, 1, 2));
        assert!((t[0].boite_mm[1] - 990.0).abs() < 1e-6 && (t[0].boite_mm[3] - 2000.0).abs() < 1e-6);
        assert_eq!(t[4].recouvrement_mm.map(|v| (v * 100.0).round() / 100.0), [20.0, 0.0, 20.0, 20.0]);
        // Recouvrement propre de 40 mm sur la tuile 2 : 10 mm de plus de chaque côté intérieur.
        let p = Mosaique { recouvrement_tuiles_mm: vec![None, Some(40.0)], ..g };
        let t2 = tuiles(3000.0, 2000.0, &p).unwrap();
        assert!((t2[1].boite_mm[0] - (t[1].boite_mm[0] - 10.0)).abs() < 1e-6 && (t2[1].boite_mm[2] - (t[1].boite_mm[2] + 10.0)).abs() < 1e-6);
        assert!((t2[1].boite_mm[1] - (t[1].boite_mm[1] - 10.0)).abs() < 1e-6 && (t2[1].boite_mm[3] - t[1].boite_mm[3]).abs() < 1e-6, "pas de voisine en haut");
        assert!((t2[0].recouvrement_mm[2] - 30.0).abs() < 1e-6, "jonction 1-2 : (20 + 40) / 2");
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
        let pdf = composer(&banniere(), &m(1600.0, 20.0, false)).unwrap().0;
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

    /// Repères : page de 310 mm (bande de 10 mm), contenu et découpe remontés de 10 mm,
    /// trait noir dans la bande au bord du recouvrement (lé 1 : 1490 mm ; lé 2 : 20 mm).
    #[test]
    fn bande_de_reperes_des_les() {
        let pdf = composer(&banniere(), &m(1600.0, 20.0, true)).unwrap().0;
        let pages = crate::decoupe::extraire(&pdf, false).unwrap();
        assert!((pages[0].hauteur / MM - 310.0).abs() < 1e-3);
        let y_min = pages[0].traces[0].points.iter().map(|p| p.1).fold(f64::MAX, f64::min);
        assert!((y_min / MM - 20.0).abs() < 1e-3, "contour à 10 mm du bas de l'image, plus la bande : {} mm", y_min / MM);
        assert!(composer(&banniere(), &m(4000.0, 20.0, true)).map(|p| crate::decoupe::extraire(&p.0, false).unwrap()[0].hauteur / MM).unwrap() < 300.1, "pas de lé : pas de bande");

        let dossier = std::env::temp_dir().join(format!("eko-rip-reperes-{}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        std::fs::write(dossier.join("m.pdf"), &pdf).unwrap();
        let dpi = 18.0;
        let st = std::process::Command::new("gs")
            .args(["-q", "-dSAFER", "-dBATCH", "-dNOPAUSE", "-sDEVICE=tiffsep", "-r18"])
            .arg(format!("-sOutputFile={}", dossier.join("r-%d.tif").display()))
            .arg(dossier.join("m.pdf"))
            .status()
            .unwrap();
        assert!(st.success());
        let px = |mm: f64| (mm * MM * dpi / 72.0) as i64;
        for (page, x_mm) in [(1, 1490.0), (2, 20.0)] {
            let noir = image::open(dossier.join(format!("r-{page}(Black).tif"))).unwrap().to_luma8();
            let ligne = noir.height() as i64 - px(5.0) - 1;
            let present = (px(x_mm) - 2..=px(x_mm) + 2).any(|x| noir.get_pixel(x as u32, ligne as u32)[0] < 128);
            assert!(present, "lé {page} : trait d'assemblage à {x_mm} mm");
            let cyan = image::open(dossier.join(format!("r-{page}(Cyan).tif"))).unwrap().to_luma8();
            assert!((0..cyan.width()).all(|x| cyan.get_pixel(x, ligne as u32)[0] == 255), "lé {page} : pas d'image dans la bande");
        }
        std::fs::remove_dir_all(&dossier).unwrap();
    }
}
