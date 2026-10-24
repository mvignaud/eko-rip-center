//! Édition rapide d'un PDF avant le rendu : échelle, rotation, miroir, marge blanche,
//! œillets (repères imprimés le long des bords) et contour de découpe rectangulaire.
//! Chaque page devient un formulaire XObject posé par une matrice sur une nouvelle page ;
//! la découpe, lue sur le PDF produit, suit la même transformation.

use crate::imbrication::{PT_PAR_MM, page_en_formulaire};
use lopdf::{Dictionary, Document, Object, Stream, dictionary};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Oeillets {
    /// Écart maximal entre deux œillets, en mm (ils sont répartis régulièrement).
    pub espacement_mm: f64,
    /// Distance du centre de l'œillet au bord, en mm.
    pub retrait_mm: f64,
    pub diametre_mm: f64,
    /// Bords garnis : haut, bas, gauche, droite.
    pub bords: [bool; 4],
}

impl Default for Oeillets {
    fn default() -> Self {
        Oeillets { espacement_mm: 500.0, retrait_mm: 20.0, diametre_mm: 10.0, bords: [true; 4] }
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Edition {
    /// Échelle en %.
    #[serde(default = "cent")]
    pub echelle: f64,
    /// Quart de tour à gauche : 0, 90, 180 ou 270.
    #[serde(default)]
    pub rotation: u16,
    /// Miroir horizontal (vitrophanie, caisson lumineux vu de l'arrière).
    #[serde(default)]
    pub miroir: bool,
    /// Blanc ajouté autour de l'image, en mm.
    #[serde(default)]
    pub marge_mm: f64,
    #[serde(default)]
    pub oeillets: Option<Oeillets>,
    /// Contour `CutContour` rectangulaire, décalé du bord de l'image (mm, négatif = dedans).
    #[serde(default)]
    pub contour_mm: Option<f64>,
}

fn cent() -> f64 {
    100.0
}

impl Default for Edition {
    fn default() -> Self {
        Edition { echelle: 100.0, rotation: 0, miroir: false, marge_mm: 0.0, oeillets: None, contour_mm: None }
    }
}

impl Edition {
    pub fn neutre(&self) -> bool {
        *self == Edition::default()
    }

    pub fn verifier(&self) -> Result<(), &'static str> {
        let o = self.oeillets.unwrap_or_default();
        let bon = (1.0..=1000.0).contains(&self.echelle)
            && [0, 90, 180, 270].contains(&self.rotation)
            && (0.0..=500.0).contains(&self.marge_mm)
            && (50.0..=5000.0).contains(&o.espacement_mm)
            && (3.0..=200.0).contains(&o.retrait_mm)
            && (2.0..=50.0).contains(&o.diametre_mm)
            && self.contour_mm.is_none_or(|c| (-50.0..=50.0).contains(&c));
        if bon { Ok(()) } else { Err("edition_invalide") }
    }
}

/// Centres des œillets d'une page `l × h` (pt) : sur chaque bord garni, à `retrait` du
/// bord, coins compris, écart régulier au plus `espacement`.
pub fn centres_oeillets(l: f64, h: f64, o: &Oeillets) -> Vec<(f64, f64)> {
    let (r, e) = (o.retrait_mm * PT_PAR_MM, o.espacement_mm * PT_PAR_MM);
    let repartir = |debut: f64, fin: f64| -> Vec<f64> {
        let longueur = fin - debut;
        if longueur <= 0.0 {
            return vec![debut];
        }
        let n = (longueur / e).ceil().max(1.0) as usize;
        (0..=n).map(|i| debut + longueur * i as f64 / n as f64).collect()
    };
    let [haut, bas, gauche, droite] = o.bords;
    let mut c = vec![];
    if bas {
        c.extend(repartir(r, l - r).into_iter().map(|x| (x, r)));
    }
    if haut {
        c.extend(repartir(r, l - r).into_iter().map(|x| (x, h - r)));
    }
    if gauche {
        c.extend(repartir(r, h - r).into_iter().map(|y| (r, y)));
    }
    if droite {
        c.extend(repartir(r, h - r).into_iter().map(|y| (l - r, y)));
    }
    // Les coins sont partagés par deux bords : une seule fois.
    let mut uniques: Vec<(f64, f64)> = vec![];
    for p in c {
        if !uniques.iter().any(|q| (q.0 - p.0).abs() < 0.01 && (q.1 - p.1).abs() < 0.01) {
            uniques.push(p);
        }
    }
    uniques
}

/// Cercle approché par quatre Béziers, centre (x, y), rayon `r`.
fn cercle(x: f64, y: f64, r: f64) -> String {
    let k = 0.552_284_75 * r;
    format!(
        "{:.3} {:.3} m {:.3} {:.3} {:.3} {:.3} {:.3} {:.3} c {:.3} {:.3} {:.3} {:.3} {:.3} {:.3} c {:.3} {:.3} {:.3} {:.3} {:.3} {:.3} c {:.3} {:.3} {:.3} {:.3} {:.3} {:.3} c h ",
        x + r,
        y,
        x + r,
        y + k,
        x + k,
        y + r,
        x,
        y + r,
        x - k,
        y + r,
        x - r,
        y + k,
        x - r,
        y,
        x - r,
        y - k,
        x - k,
        y - r,
        x,
        y - r,
        x + k,
        y - r,
        x + r,
        y - k,
        x + r,
        y
    )
}

/// Produit de matrices PDF : d'abord `m`, puis `n`.
fn produit(m: &[f64; 6], n: &[f64; 6]) -> [f64; 6] {
    [m[0] * n[0] + m[1] * n[2], m[0] * n[1] + m[1] * n[3], m[2] * n[0] + m[3] * n[2], m[2] * n[1] + m[3] * n[3], m[4] * n[0] + m[5] * n[2] + n[4], m[4] * n[1] + m[5] * n[3] + n[5]]
}

/// Applique l'édition à chaque page du PDF.
pub fn appliquer(pdf: &[u8], e: &Edition) -> Result<Vec<u8>, String> {
    let nb_pages = Document::load_mem(pdf).map_err(|err| format!("PDF illisible : {err}"))?.get_pages().len();
    let mut cible = Document::with_version("1.6");
    let pages_id = cible.new_object_id();
    let coupe = cible.add_object(Object::Array(vec![
        "Separation".into(),
        "CutContour".into(),
        "DeviceCMYK".into(),
        Object::Dictionary(dictionary! { "FunctionType" => 2, "Domain" => vec![0.into(), 1.into()], "C0" => vec![0.into(); 4], "C1" => vec![0.into(), 1.into(), 0.into(), 0.into()], "N" => 1 }),
    ]));
    let s = e.echelle / 100.0;
    let marge = e.marge_mm * PT_PAR_MM;
    let mut enfants = vec![];
    for n in 1..=nb_pages {
        let (form, t, [llx, lly]) = page_en_formulaire(&mut cible, pdf, n).map_err(|err| format!("page {n} : {err}"))?;
        // Image transformée : taille après échelle, puis après rotation.
        let (w, h) = (t.largeur * s, t.hauteur * s);
        let (lw, lh) = if e.rotation % 180 == 90 { (h, w) } else { (w, h) };
        // (x, y) → origine de la MediaBox à 0, échelle et miroir dans [0, w] × [0, h].
        let sx = if e.miroir { -s } else { s };
        let base = [sx, 0.0, 0.0, s, -sx * llx + if e.miroir { w } else { 0.0 }, -s * lly];
        // Quart de tour à gauche ramené dans le premier quadrant.
        let rot: [f64; 6] = match e.rotation {
            90 => [0.0, 1.0, -1.0, 0.0, h, 0.0],
            180 => [-1.0, 0.0, 0.0, -1.0, w, h],
            270 => [0.0, -1.0, 1.0, 0.0, 0.0, w],
            _ => [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        };
        let m = produit(&produit(&base, &rot), &[1.0, 0.0, 0.0, 1.0, marge, marge]);
        let (pl, ph) = (lw + 2.0 * marge, lh + 2.0 * marge);
        let mut contenu = format!("q {:.6} {:.6} {:.6} {:.6} {:.4} {:.4} cm /F Do Q\n", m[0], m[1], m[2], m[3], m[4], m[5]);
        if let Some(o) = e.oeillets {
            contenu += "q 0 0 0 1 K 0.6 w ";
            for (x, y) in centres_oeillets(pl, ph, &o) {
                contenu += &cercle(x, y, o.diametre_mm * PT_PAR_MM / 2.0);
                // Croix de centre pour le poinçon.
                contenu += &format!("{:.3} {:.3} m {:.3} {:.3} l {:.3} {:.3} m {:.3} {:.3} l ", x - 1.5, y, x + 1.5, y, x, y - 1.5, x, y + 1.5);
            }
            contenu += "S Q\n";
        }
        if let Some(c) = e.contour_mm {
            let d = c * PT_PAR_MM;
            contenu += &format!("q /CC CS 1 SCN 0.5 w {:.4} {:.4} {:.4} {:.4} re S Q\n", marge - d, marge - d, lw + 2.0 * d, lh + 2.0 * d);
        }
        let flux = cible.add_object(Stream::new(dictionary! {}, contenu.into_bytes()));
        let mut xobjets = Dictionary::new();
        xobjets.set("F", form);
        enfants.push(Object::Reference(cible.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), Object::Real(pl as f32), Object::Real(ph as f32)],
            "Contents" => flux,
            "Resources" => dictionary! { "XObject" => xobjets, "ColorSpace" => dictionary! { "CC" => coupe } },
        })));
    }
    let compte = enfants.len() as i64;
    cible.objects.insert(pages_id, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => enfants, "Count" => compte }));
    let catalogue = cible.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    cible.trailer.set("Root", catalogue);
    cible.prune_objects();
    let mut sortie = vec![];
    cible.save_to(&mut sortie).map_err(|err| err.to_string())?;
    Ok(sortie)
}

#[cfg(test)]
mod tests {
    use super::*;
    const MM: f64 = PT_PAR_MM;

    #[test]
    fn oeillets_repartis_coins_compris() {
        // 1000 × 500 mm, retrait 20, écart max 500 : bas et haut 960 mm → 2 intervalles
        // (3 œillets chacun) ; côtés 460 mm → 1 intervalle, coins déjà comptés : 6.
        let o = Oeillets { espacement_mm: 500.0, retrait_mm: 20.0, diametre_mm: 10.0, bords: [true; 4] };
        let c = centres_oeillets(1000.0 * MM, 500.0 * MM, &o);
        assert_eq!(c.len(), 6, "{c:?}");
        assert!(c.iter().any(|p| (p.0 - 500.0 * MM).abs() < 1e-6 && (p.1 - 20.0 * MM).abs() < 1e-6), "milieu du bas");
        assert_eq!(centres_oeillets(1000.0 * MM, 500.0 * MM, &Oeillets { bords: [true, false, false, false], ..o }).len(), 3, "haut seul");
        assert!(Edition { echelle: 0.0, ..Default::default() }.verifier().is_err());
        assert!(Edition { rotation: 45, ..Default::default() }.verifier().is_err());
        assert!(Edition::default().neutre() && Edition::default().verifier().is_ok());
    }

    fn boite(p: &crate::decoupe::Decoupe, i: usize) -> (f64, f64, f64, f64) {
        p.traces[i].points.iter().fold((f64::MAX, f64::MAX, f64::MIN, f64::MIN), |(a, b, c, d), &(x, y)| (a.min(x), b.min(y), c.max(x), d.max(y)))
    }

    #[test]
    fn echelle_rotation_miroir_et_marge_suivis_par_la_decoupe() {
        // Page cyan 100 × 50 pt (MediaBox décalée), contour à 5 pt du bord.
        let source = crate::imbrication::tests::pdf(100.0, 50.0, "1 0 0 0", (30.0, 40.0));
        // 200 %, quart de tour, marge de 10 mm : page (100 + 2m) × (200 + 2m) pt.
        let m = 10.0 * MM;
        let e = Edition { echelle: 200.0, rotation: 90, marge_mm: 10.0, ..Default::default() };
        let p = &crate::decoupe::extraire(&appliquer(&source, &e).unwrap(), false).unwrap()[0];
        assert!((p.largeur - (100.0 + 2.0 * m)).abs() < 1e-3 && (p.hauteur - (200.0 + 2.0 * m)).abs() < 1e-3, "{} × {}", p.largeur, p.hauteur);
        // Contour d'origine à 5 pt du bord : 10 pt une fois doublé.
        let (x0, y0, x1, y1) = boite(p, 0);
        assert!((x0 - (m + 10.0)).abs() < 1e-3 && (x1 - (m + 90.0)).abs() < 1e-3 && (y0 - (m + 10.0)).abs() < 1e-3 && (y1 - (m + 190.0)).abs() < 1e-3, "{x0} {y0} {x1} {y1}");

        // Miroir, marge 5 mm, contour rectangulaire à 3 mm autour de l'image.
        let e = Edition { miroir: true, marge_mm: 5.0, contour_mm: Some(3.0), ..Default::default() };
        let p = &crate::decoupe::extraire(&appliquer(&source, &e).unwrap(), false).unwrap()[0];
        assert_eq!(p.traces.len(), 2);
        let (x0, y0, x1, y1) = boite(p, 1);
        let (m5, d3) = (5.0 * MM, 3.0 * MM);
        assert!((x0 - (m5 - d3)).abs() < 1e-3 && (y0 - (m5 - d3)).abs() < 1e-3 && (x1 - (m5 + 100.0 + d3)).abs() < 1e-3 && (y1 - (m5 + 50.0 + d3)).abs() < 1e-3);
        // Miroir : le contour d'origine reste symétrique (5 pt de chaque bord).
        let (x0, _, x1, _) = boite(p, 0);
        assert!((x0 - (m5 + 5.0)).abs() < 1e-3 && (x1 - (m5 + 95.0)).abs() < 1e-3);
    }

    /// Rendu réel : page cyan à gauche, magenta à droite ; miroir → magenta à gauche.
    /// Marge blanche, et anneau d'œillet noir dans le coin.
    #[test]
    fn oeillets_et_miroir_rendus() {
        let mut doc = Document::with_version("1.4");
        let (l, h) = (600.0 * MM, 300.0 * MM);
        let c = doc.add_object(Stream::new(dictionary! {}, format!("1 0 0 0 k 0 0 {} {h} re f 0 1 0 0 k {} 0 {} {h} re f", l / 2.0, l / 2.0, l / 2.0).into_bytes()));
        let pages = doc.new_object_id();
        let page = doc.add_object(dictionary! { "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(), 0.into(), Object::Real(l as f32), Object::Real(h as f32)], "Contents" => c });
        doc.objects.insert(pages, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }));
        let cat = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
        doc.trailer.set("Root", cat);
        let mut pdf = vec![];
        doc.save_to(&mut pdf).unwrap();

        let o = Oeillets { espacement_mm: 300.0, retrait_mm: 15.0, diametre_mm: 10.0, bords: [true; 4] };
        let e = Edition { miroir: true, marge_mm: 30.0, oeillets: Some(o), ..Default::default() };
        let d = std::env::temp_dir().join(format!("eko-rip-edition-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("e.pdf"), appliquer(&pdf, &e).unwrap()).unwrap();
        let st = std::process::Command::new("gs")
            .args(["-q", "-dSAFER", "-dBATCH", "-dNOPAUSE", "-sDEVICE=tiffsep", "-r20"])
            .arg(format!("-sOutputFile={}", d.join("r.tif").display()))
            .arg(d.join("e.pdf"))
            .status()
            .unwrap();
        assert!(st.success());
        let px = |mm: f64| (mm / 25.4 * 20.0).round() as u32;
        let plaque = |n: &str| image::open(d.join(format!("r({n}).tif"))).unwrap().to_luma8();
        let (noir, cyan, magenta) = (plaque("Black"), plaque("Cyan"), plaque("Magenta"));
        assert_eq!(noir.dimensions(), (px(660.0), px(360.0)));
        let bas = |mm: f64| noir.height() - 1 - px(mm);
        assert_eq!(cyan.get_pixel(px(10.0), bas(180.0))[0], 255, "marge blanche");
        assert_eq!(magenta.get_pixel(px(100.0), bas(180.0))[0], 0, "miroir : magenta à gauche");
        assert_eq!(cyan.get_pixel(px(560.0), bas(180.0))[0], 0, "miroir : cyan à droite");
        // Anneau de 5 mm de rayon autour de (15, 15) mm : encre sur le cercle, pas au milieu
        // entre le cercle et la croix.
        assert!((px(18.0)..=px(22.0)).any(|x| noir.get_pixel(x, bas(15.0))[0] < 128), "anneau d'œillet");
        std::fs::remove_dir_all(&d).unwrap();
    }
}
