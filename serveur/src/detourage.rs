//! Détourage automatique (roadmap 4.14), comme « Make Path » : l'image rendue est seuillée
//! sur le blanc, les petites taches et (au choix) les contours intérieurs sont ôtés, la forme
//! est écartée vers l'extérieur (offset, que l'aide d'ONYX ne documente pas), puis son bord
//! est suivi, simplifié et lissé. Le résultat est montré en aperçu, puis accepté ou non.
//!
//! L'import d'images (TIFF, JPEG, PNG : roadmap 9.2) n'existe pas encore : le détourage se fait
//! sur la page d'un PDF rendue en couleur (`main.rs`), sans les tons de découpe.

use crate::decoupe::Trace;
use serde::{Deserialize, Serialize};

/// Points par pouce du rendu détouré, réduits pour une grande page (`dpi_pour`).
pub const DPI: f64 = 150.0;
/// Pixels au plus du rendu détouré (≈ 120 Mo en RVB).
const PIXELS_MAX: f64 = 40e6;

fn vrai() -> bool {
    true
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Detourage {
    /// Un pixel est du fond (blanc) si ses trois composantes valent au moins ce seuil (1 à 255).
    pub seuil: u8,
    /// Supprimer les contours intérieurs (trous) : un seul contour par pièce.
    #[serde(default = "vrai")]
    pub sans_interieurs: bool,
    /// Taches et trous plus petits que cette aire ôtés, en mm².
    #[serde(default)]
    pub aire_min_mm2: f64,
    /// Passes de lissage (0 à 5).
    #[serde(default)]
    pub lissage: u8,
    /// Écart du contour vers l'extérieur, en mm (0 à 20).
    #[serde(default)]
    pub decalage_mm: f64,
}

impl Detourage {
    pub fn verifier(&self) -> Result<(), &'static str> {
        let bon = self.seuil >= 1 && (0.0..=10_000.0).contains(&self.aire_min_mm2) && self.lissage <= 5 && (0.0..=20.0).contains(&self.decalage_mm);
        if bon { Ok(()) } else { Err("detourage_invalide") }
    }
}

/// Résolution du rendu d'une page `l × h` points : `DPI`, moins si la page est grande.
pub fn dpi_pour(l: f64, h: f64) -> f64 {
    let pouces = (l / 72.0) * (h / 72.0);
    DPI.min((PIXELS_MAX / pouces.max(1e-9)).sqrt()).max(10.0)
}

/// Contours détourés de l'image RVB (`largeur × hauteur`, rangées du haut vers le bas) rendue à
/// `dpi`, en points de la page (origine en bas à gauche), décalés de `origine` (points).
pub fn detourer(rvb: &[u8], largeur: usize, hauteur: usize, dpi: f64, d: &Detourage, origine: (f64, f64)) -> Vec<Trace> {
    let px_mm = dpi / 25.4;
    let r = (d.decalage_mm * px_mm).round() as usize;
    // Masque bordé de `r + 1` pixels : la forme peut grossir au-delà de l'image.
    let b = r + 1;
    let (l, h) = (largeur + 2 * b, hauteur + 2 * b);
    let mut masque = vec![false; l * h];
    for y in 0..hauteur {
        for x in 0..largeur {
            let p = &rvb[(y * largeur + x) * 3..][..3];
            masque[(y + b) * l + x + b] = p.iter().any(|&v| v < d.seuil);
        }
    }
    let aire_min = (d.aire_min_mm2 * px_mm * px_mm).round() as usize;
    // Taches : composantes (4-connexes) de la forme trop petites.
    for (pixels, _) in composantes(&masque, l, h, true, false) {
        if pixels.len() < aire_min {
            pixels.iter().for_each(|&i| masque[i] = false);
        }
    }
    // Trous : composantes du fond (8-connexes) qui ne touchent pas le bord ; tous comblés sans
    // les contours intérieurs, sinon les petits seulement.
    for (pixels, bord) in composantes(&masque, l, h, false, true) {
        if !bord && (d.sans_interieurs || pixels.len() < aire_min) {
            pixels.iter().for_each(|&i| masque[i] = true);
        }
    }
    if r > 0 {
        masque = dilater(&masque, l, h, r as f64);
    }
    let k = 72.0 / dpi;
    let hauteur_pt = hauteur as f64 * k;
    contours(&masque, l, h)
        .into_iter()
        .map(|c| lisser(&simplifier(&c, 0.5), d.lissage))
        .filter(|c| c.len() >= 3)
        .map(|c| Trace {
            ton: "CutContour".into(),
            // Coin de grille (centres de pixels) → bord du pixel, bordure ôtée, puis points.
            points: c.iter().map(|&(x, y)| (origine.0 + (x + 0.5 - b as f64) * k, origine.1 + hauteur_pt - (y + 0.5 - b as f64) * k)).collect(),
            ferme: true,
        })
        .collect()
}

/// Composantes de valeur `valeur` du masque : pixels et vrai si l'une touche le bord.
fn composantes(masque: &[bool], l: usize, h: usize, valeur: bool, huit: bool) -> Vec<(Vec<usize>, bool)> {
    let mut vu = vec![false; masque.len()];
    let mut sortie = vec![];
    for depart in 0..masque.len() {
        if vu[depart] || masque[depart] != valeur {
            continue;
        }
        let (mut pile, mut pixels, mut bord) = (vec![depart], vec![], false);
        vu[depart] = true;
        while let Some(i) = pile.pop() {
            pixels.push(i);
            let (x, y) = ((i % l) as i64, (i / l) as i64);
            bord |= x == 0 || y == 0 || x as usize == l - 1 || y as usize == h - 1;
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)].into_iter().take(if huit { 8 } else { 4 }) {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx as usize >= l || ny as usize >= h {
                    continue;
                }
                let j = ny as usize * l + nx as usize;
                if !vu[j] && masque[j] == valeur {
                    vu[j] = true;
                    pile.push(j);
                }
            }
        }
        sortie.push((pixels, bord));
    }
    sortie
}

/// Forme grossie de `r` pixels : distance au plus proche pixel de la forme par chanfrein 5-7-11
/// (écart à la distance vraie sous 2 %).
fn dilater(masque: &[bool], l: usize, h: usize, r: f64) -> Vec<bool> {
    const INF: u32 = u32::MAX / 2;
    let mut d: Vec<u32> = masque.iter().map(|&m| if m { 0 } else { INF }).collect();
    let voisins_avant: [(i64, i64, u32); 8] = [(-1, 0, 5), (-1, -1, 7), (0, -1, 5), (1, -1, 7), (-1, -2, 11), (1, -2, 11), (-2, -1, 11), (2, -1, 11)];
    let mut passe = |sens: i64| {
        let (ys, xs): (Vec<usize>, Vec<usize>) = if sens > 0 { ((0..h).collect(), (0..l).collect()) } else { ((0..h).rev().collect(), (0..l).rev().collect()) };
        for &y in &ys {
            for &x in &xs {
                let i = y * l + x;
                for &(dx, dy, c) in &voisins_avant {
                    let (nx, ny) = (x as i64 + sens * dx, y as i64 + sens * dy);
                    if nx >= 0 && ny >= 0 && (nx as usize) < l && (ny as usize) < h {
                        let v = d[ny as usize * l + nx as usize] + c;
                        if v < d[i] {
                            d[i] = v;
                        }
                    }
                }
            }
        }
    };
    passe(1);
    passe(-1);
    let limite = (r * 5.0).round() as u32;
    d.into_iter().map(|v| v <= limite).collect()
}

/// Point de la grille en coordonnées doublées.
type Point = (i64, i64);

/// Bords de la forme par carrés marchants, sur la grille des centres de pixels : un polygone
/// fermé par bord (extérieur ou trou). Deux pixels qui ne se touchent que par un coin sont
/// deux pièces.
fn contours(masque: &[bool], l: usize, h: usize) -> Vec<Vec<(f64, f64)>> {
    let v = |x: i64, y: i64| x >= 0 && y >= 0 && (x as usize) < l && (y as usize) < h && masque[y as usize * l + x as usize];
    // Points en coordonnées doublées : milieux d'arêtes de cellule.
    let mut voisins: std::collections::HashMap<(i64, i64), Vec<(i64, i64)>> = std::collections::HashMap::new();
    for y in -1..h as i64 {
        for x in -1..l as i64 {
            let cas = (v(x, y) as u8) << 3 | (v(x + 1, y) as u8) << 2 | (v(x + 1, y + 1) as u8) << 1 | v(x, y + 1) as u8;
            let (haut, droite, bas, gauche) = ((2 * x + 1, 2 * y), (2 * x + 2, 2 * y + 1), (2 * x + 1, 2 * y + 2), (2 * x, 2 * y + 1));
            let segments: &[(Point, Point)] = match cas {
                1 | 14 => &[(gauche, bas)],
                2 | 13 => &[(bas, droite)],
                3 | 12 => &[(gauche, droite)],
                4 | 11 => &[(haut, droite)],
                5 => &[(gauche, bas), (haut, droite)],
                6 | 9 => &[(haut, bas)],
                7 | 8 => &[(gauche, haut)],
                10 => &[(gauche, haut), (bas, droite)],
                _ => &[],
            };
            for &(a, b) in segments {
                voisins.entry(a).or_default().push(b);
                voisins.entry(b).or_default().push(a);
            }
        }
    }
    let mut sortie = vec![];
    let mut cles: Vec<(i64, i64)> = voisins.keys().copied().collect();
    cles.sort_unstable();
    let mut vus = std::collections::HashSet::new();
    for depart in cles {
        if vus.contains(&depart) {
            continue;
        }
        let mut boucle = vec![depart];
        vus.insert(depart);
        let (mut avant, mut ici) = (depart, voisins[&depart][0]);
        while ici != depart {
            vus.insert(ici);
            boucle.push(ici);
            let n = &voisins[&ici];
            let suivant = if n[0] == avant { n[1] } else { n[0] };
            (avant, ici) = (ici, suivant);
        }
        sortie.push(boucle.into_iter().map(|(x, y)| (x as f64 / 2.0, y as f64 / 2.0)).collect());
    }
    sortie
}

/// Polygone fermé simplifié (Douglas-Peucker) à `tolerance` pixels près.
fn simplifier(p: &[(f64, f64)], tolerance: f64) -> Vec<(f64, f64)> {
    fn dp(p: &[(f64, f64)], t: f64, garde: &mut [bool], a: usize, b: usize) {
        let (pa, pb) = (p[a], p[b]);
        let (dx, dy) = (pb.0 - pa.0, pb.1 - pa.1);
        let lg = dx.hypot(dy).max(1e-12);
        let (mut loin, mut i_loin) = (0.0, a);
        for (i, q) in p.iter().enumerate().take(b).skip(a + 1) {
            let d = ((q.0 - pa.0) * dy - (q.1 - pa.1) * dx).abs() / lg;
            if d > loin {
                (loin, i_loin) = (d, i);
            }
        }
        if loin > t {
            garde[i_loin] = true;
            dp(p, t, garde, a, i_loin);
            dp(p, t, garde, i_loin, b);
        }
    }
    if p.len() < 4 {
        return p.to_vec();
    }
    // Coupé en deux au point le plus loin du premier : deux chaînes ouvertes.
    let loin = (1..p.len()).max_by(|&i, &j| (p[i].0 - p[0].0).hypot(p[i].1 - p[0].1).total_cmp(&(p[j].0 - p[0].0).hypot(p[j].1 - p[0].1))).unwrap();
    let mut ferme = p.to_vec();
    ferme.push(p[0]);
    let mut garde = vec![false; ferme.len()];
    (garde[0], garde[loin]) = (true, true);
    dp(&ferme, tolerance, &mut garde, 0, loin);
    dp(&ferme, tolerance, &mut garde, loin, ferme.len() - 1);
    ferme.pop();
    ferme.into_iter().zip(garde).filter(|(_, g)| *g).map(|(q, _)| q).collect()
}

/// Lissage de Chaikin (coupe des coins) : `passes` fois.
fn lisser(p: &[(f64, f64)], passes: u8) -> Vec<(f64, f64)> {
    let mut p = p.to_vec();
    for _ in 0..passes {
        let n = p.len();
        p = (0..n).flat_map(|i| {
            let (a, b) = (p[i], p[(i + 1) % n]);
            [(0.75 * a.0 + 0.25 * b.0, 0.75 * a.1 + 0.25 * b.1), (0.25 * a.0 + 0.75 * b.0, 0.25 * a.1 + 0.75 * b.1)]
        }).collect();
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Image connue, 400 × 400 pixels à 254 dpi (10 px/mm) : disque cyan de 10 mm de rayon
    /// centré en (20, 20) mm, percé d'un trou de 3 mm ; tache de 0,5 × 0,5 mm en (35, 35) mm.
    fn image() -> Vec<u8> {
        let mut o = vec![255u8; 400 * 400 * 3];
        for y in 0..400 {
            for x in 0..400 {
                let (cx, cy) = (x as f64 + 0.5 - 200.0, y as f64 + 0.5 - 200.0);
                let r = cx.hypot(cy);
                let tache = (350..355).contains(&x) && (350..355).contains(&y);
                if (r <= 100.0 && r > 30.0) || tache {
                    o[(y * 400 + x) * 3..][..3].copy_from_slice(&[0, 160, 230]);
                }
            }
        }
        o
    }

    /// Rayons (min, max) en mm d'un contour autour du centre (20, 20) mm de la page.
    fn rayons(t: &Trace) -> (f64, f64) {
        let mm = 25.4 / 72.0;
        let r: Vec<f64> = t.points.iter().map(|&(x, y)| (x * mm - 20.0).hypot(y * mm - 20.0)).collect();
        (r.iter().copied().fold(f64::MAX, f64::min), r.iter().copied().fold(f64::MIN, f64::max))
    }

    #[test]
    fn detourage_d_une_image_connue() {
        let img = image();
        let d = |sans_interieurs, aire_min_mm2, lissage, decalage_mm| Detourage { seuil: 250, sans_interieurs, aire_min_mm2, lissage, decalage_mm };
        // Sans les intérieurs, sans la tache : un seul contour, le bord du disque (± 0,1 mm).
        let c = detourer(&img, 400, 400, 254.0, &d(true, 1.0, 0, 0.0), (0.0, 0.0));
        assert_eq!(c.len(), 1, "un seul contour");
        let (rmin, rmax) = rayons(&c[0]);
        assert!(rmin > 9.9 && rmax < 10.1, "rayon {rmin}–{rmax}");
        assert!(c[0].ferme && c[0].ton == "CutContour");
        // Avec les intérieurs : le trou aussi (rayon 3 mm).
        let c = detourer(&img, 400, 400, 254.0, &d(false, 1.0, 0, 0.0), (0.0, 0.0));
        assert_eq!(c.len(), 2, "contour et trou");
        let trou = c.iter().map(rayons).find(|r| r.1 < 5.0).expect("le trou");
        assert!(trou.0 > 2.9 && trou.1 < 3.1, "trou {trou:?}");
        // Sans aire minimale : la tache a son contour.
        assert_eq!(detourer(&img, 400, 400, 254.0, &d(true, 0.0, 0, 0.0), (0.0, 0.0)).len(), 2, "la tache reste");
        // Offset de 2 mm vers l'extérieur, lissé : rayon 12 mm (± 0,15).
        let c = detourer(&img, 400, 400, 254.0, &d(true, 1.0, 3, 2.0), (0.0, 0.0));
        assert_eq!(c.len(), 1);
        let (rmin, rmax) = rayons(&c[0]);
        assert!(rmin > 11.85 && rmax < 12.15, "offset : rayon {rmin}–{rmax}");
        // Origine de la page : les tracés suivent.
        let c = detourer(&img, 400, 400, 254.0, &d(true, 1.0, 0, 0.0), (72.0, 0.0));
        assert!((crate::coupe::boite(&c).unwrap()[0] - (72.0 + 10.0 * 72.0 / 25.4)).abs() < 0.3);
    }

    #[test]
    fn reglages_bornes() {
        let d = Detourage { seuil: 250, sans_interieurs: true, aire_min_mm2: 0.0, lissage: 0, decalage_mm: 0.0 };
        assert_eq!(d.verifier(), Ok(()));
        assert_eq!(Detourage { lissage: 6, ..d }.verifier(), Err("detourage_invalide"));
        assert_eq!(Detourage { decalage_mm: 25.0, ..d }.verifier(), Err("detourage_invalide"));
        assert_eq!(Detourage { seuil: 0, ..d }.verifier(), Err("detourage_invalide"));
        assert!((dpi_pour(72.0, 72.0) - DPI).abs() < 1e-9 && dpi_pour(5000.0, 10000.0) < DPI, "grande page : moins de points");
    }
}
