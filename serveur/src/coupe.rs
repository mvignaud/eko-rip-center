//! Mise en coupe d'un travail : ce que l'opérateur règle sur le fichier de coupe sans
//! refaire le rendu, comme les outils du fichier de CUT-Server. Appliquée aux tracés de la
//! page (`prod/decoupe-NNN.json`) chaque fois qu'un fichier de coupe est écrit ou envoyé.
//!
//! - Rotation par quart de tour (roadmap 4.8) : l'orientation du travail pour la machine,
//!   pas le fichier source ; l'opérateur tourne le support d'autant.
//! - Découpe seule (roadmap 4.9) : miroir, échelle proportionnelle ou non, copies X/Y avec
//!   écarts, départ depuis l'origine de la machine. Sans impression seulement : sur un
//!   travail imprimé, la coupe ne suivrait plus l'encre.
//! - Décaler la coupe vers l'extérieur (roadmap 4.10) : la pièce grandit d'autant.
//!
//! Et, au rendu (roadmap 4.10), « étendre l'image près des tracés » : l'encre proche d'un
//! contour fermé est prolongée au-delà, sur une largeur donnée. Ni l'un ni l'autre ne
//! remplace l'étalonnage imprimante-découpeuse (décalage de la machine).

use crate::decoupe::Trace;
use serde::{Deserialize, Serialize};

const PT_PAR_MM: f64 = 72.0 / 25.4;

fn cent() -> [f64; 2] {
    [100.0; 2]
}

fn un() -> [u32; 2] {
    [1; 2]
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mise {
    /// Quart de tour à gauche : 0, 90, 180 ou 270 (même sens que l'édition).
    #[serde(default)]
    pub rotation: u16,
    /// Miroir horizontal (gauche-droite), avant la rotation.
    #[serde(default)]
    pub miroir: bool,
    /// Échelle en % (largeur, hauteur) : égales pour une échelle proportionnelle.
    #[serde(default = "cent")]
    pub echelle: [f64; 2],
    /// Copies en X et en Y : les tracés répétés.
    #[serde(default = "un")]
    pub copies: [u32; 2],
    /// Blanc entre deux copies (X, Y), en mm, d'un tracé à l'autre.
    #[serde(default)]
    pub ecart_mm: [f64; 2],
    /// Les tracés commencent à l'origine de la machine (le blanc autour de la page ôté).
    #[serde(default)]
    pub depuis_origine: bool,
    /// Décalage des contours fermés vers l'extérieur de la pièce, en mm (0 à 5) : un contour
    /// extérieur s'élargit, un trou se resserre. Les tracés ouverts ne bougent pas.
    #[serde(default)]
    pub decalage_mm: f64,
}

impl Default for Mise {
    fn default() -> Self {
        Mise { rotation: 0, miroir: false, echelle: cent(), copies: un(), ecart_mm: [0.0; 2], depuis_origine: false, decalage_mm: 0.0 }
    }
}

/// Tracés posés pour la machine, page (largeur, hauteur) et cadre de repérage, en points.
pub struct Posee {
    pub traces: Vec<Trace>,
    pub taille: (f64, f64),
    pub cadre: Option<[f64; 4]>,
}

impl Mise {
    /// Réglages réservés à la découpe seule (sans impression à suivre).
    pub fn reserve_decoupe_seule(&self) -> bool {
        self.miroir || self.echelle != cent() || self.copies != un() || self.depuis_origine
    }

    /// `imprime` : le travail s'imprime avant d'être coupé.
    pub fn verifier(&self, imprime: bool) -> Result<(), &'static str> {
        let bon = [0, 90, 180, 270].contains(&self.rotation)
            && self.echelle.iter().all(|e| (1.0..=1000.0).contains(e))
            && self.copies.iter().all(|c| (1..=100).contains(c))
            && self.copies[0] * self.copies[1] <= 1000
            && self.ecart_mm.iter().all(|e| (0.0..=1000.0).contains(e))
            && (0.0..=5.0).contains(&self.decalage_mm);
        if !bon {
            return Err("mise_coupe_invalide");
        }
        if imprime && self.reserve_decoupe_seule() {
            return Err("mise_decoupe_seule");
        }
        Ok(())
    }

    /// Échelle, miroir puis rotation d'un point de la page `(l, h)`.
    fn point(&self, (x, y): (f64, f64), (l, h): (f64, f64)) -> (f64, f64) {
        let (kx, ky) = (self.echelle[0] / 100.0, self.echelle[1] / 100.0);
        let (l, h) = (l * kx, h * ky);
        let (x, y) = (x * kx, y * ky);
        let x = if self.miroir { l - x } else { x };
        match self.rotation {
            90 => (h - y, x),
            180 => (l - x, h - y),
            270 => (y, l - x),
            _ => (x, y),
        }
    }

    /// Tracés de la page `taille` (et le cadre de repérage) posés pour la machine.
    pub fn appliquer(&self, traces: &[Trace], taille: (f64, f64), cadre: Option<[f64; 4]>) -> Posee {
        let decales;
        let traces = if self.decalage_mm > 0.0 {
            decales = decaler(traces, self.decalage_mm * PT_PAR_MM);
            &decales[..]
        } else {
            traces
        };
        let (kx, ky) = (self.echelle[0] / 100.0, self.echelle[1] / 100.0);
        let (l, h) = (taille.0 * kx, taille.1 * ky);
        let mut taille_posee = if self.rotation % 180 == 90 { (h, l) } else { (l, h) };
        let mut posees: Vec<Trace> = traces.iter().map(|t| Trace { points: t.points.iter().map(|&p| self.point(p, taille)).collect(), ..t.clone() }).collect();
        let cadre = cadre.map(|[x1, y1, x2, y2]| {
            let (a, b) = (self.point((x1, y1), taille), self.point((x2, y2), taille));
            [a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1)]
        });
        let Some(b) = boite(&posees) else { return Posee { traces: posees, taille: taille_posee, cadre } };
        if self.copies != un() {
            let pas = (b[2] - b[0] + self.ecart_mm[0] * PT_PAR_MM, b[3] - b[1] + self.ecart_mm[1] * PT_PAR_MM);
            let une = std::mem::take(&mut posees);
            for j in 0..self.copies[1] {
                for i in 0..self.copies[0] {
                    let (dx, dy) = (i as f64 * pas.0, j as f64 * pas.1);
                    posees.extend(une.iter().map(|t| Trace { points: t.points.iter().map(|&(x, y)| (x + dx, y + dy)).collect(), ..t.clone() }));
                }
            }
            taille_posee = (taille_posee.0 + (self.copies[0] - 1) as f64 * pas.0, taille_posee.1 + (self.copies[1] - 1) as f64 * pas.1);
        }
        if self.depuis_origine {
            let b = boite(&posees).unwrap_or(b);
            for t in &mut posees {
                for p in &mut t.points {
                    *p = (p.0 - b[0], p.1 - b[1]);
                }
            }
            taille_posee = (b[2] - b[0], b[3] - b[1]);
        }
        Posee { traces: posees, taille: taille_posee, cadre }
    }
}

/// Contours fermés écartés de `d` vers l'extérieur de la pièce : à profondeur paire (contenu
/// dans un nombre pair d'autres contours) le contour s'élargit, à profondeur impaire (un
/// trou) il se resserre. Joints en onglet, l'onglet borné à 4 fois `d`.
// ponytail: décalage sommet par sommet, sans reprise des boucles qu'un fort retrait crée dans
// un angle rentrant étroit ; suffit aux quelques dixièmes de mm d'une tolérance d'atelier.
pub fn decaler(traces: &[Trace], d: f64) -> Vec<Trace> {
    let fermes: Vec<usize> = (0..traces.len()).filter(|&i| traces[i].ferme && traces[i].points.len() >= 3).collect();
    traces
        .iter()
        .enumerate()
        .map(|(i, t)| {
            if !fermes.contains(&i) {
                return t.clone();
            }
            let p0 = t.points[0];
            let profondeur = fermes.iter().filter(|&&j| j != i && crate::decoupe::dans_polygone(p0, &traces[j].points)).count();
            let vers = if profondeur % 2 == 0 { d } else { -d };
            Trace { points: decaler_polygone(&t.points, vers), ..t.clone() }
        })
        .collect()
}

fn decaler_polygone(points: &[(f64, f64)], d: f64) -> Vec<(f64, f64)> {
    // Points confondus retirés : une arête nulle n'a pas de normale.
    let mut p: Vec<(f64, f64)> = vec![];
    for &q in points {
        if p.last().is_none_or(|l: &(f64, f64)| (l.0 - q.0).hypot(l.1 - q.1) > 1e-9) {
            p.push(q);
        }
    }
    if p.len() > 1 && (p[0].0 - p[p.len() - 1].0).hypot(p[0].1 - p[p.len() - 1].1) <= 1e-9 {
        p.pop();
    }
    let n = p.len();
    if n < 3 {
        return points.to_vec();
    }
    // Aire signée : positive dans le sens trigonométrique, l'extérieur est alors à droite.
    let aire: f64 = (0..n).map(|i| p[i].0 * p[(i + 1) % n].1 - p[(i + 1) % n].0 * p[i].1).sum();
    let s = if aire >= 0.0 { 1.0 } else { -1.0 };
    let normale = |a: (f64, f64), b: (f64, f64)| {
        let l = (b.0 - a.0).hypot(b.1 - a.1);
        (s * (b.1 - a.1) / l, -s * (b.0 - a.0) / l)
    };
    (0..n)
        .map(|i| {
            let (a, b, c) = (p[(i + n - 1) % n], p[i], p[(i + 1) % n]);
            let (n1, n2) = (normale(a, b), normale(b, c));
            let k = 1.0 + n1.0 * n2.0 + n1.1 * n2.1;
            let mut m = if k > 1e-6 { ((n1.0 + n2.0) / k, (n1.1 + n2.1) / k) } else { n1 };
            let lg = m.0.hypot(m.1);
            if lg > 4.0 {
                m = (m.0 * 4.0 / lg, m.1 * 4.0 / lg);
            }
            (b.0 + d * m.0, b.1 + d * m.1)
        })
        .collect()
}

/// Étendre l'image près des tracés (roadmap 4.10) : sur une plaque rendue, l'encre qui
/// s'arrête à moins de `seuil_mm` d'un contour fermé est prolongée jusqu'au contour, puis
/// au-delà sur `largeur_mm`. Une encre plus loin du contour n'est pas touchée.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Etendre {
    pub largeur_mm: f64,
    pub seuil_mm: f64,
}

impl Etendre {
    pub fn verifier(&self) -> Result<(), &'static str> {
        if (0.1..=10.0).contains(&self.largeur_mm) && (0.0..=10.0).contains(&self.seuil_mm) { Ok(()) } else { Err("etendre_invalide") }
    }
}

/// Segments des contours fermés en pixels, rangés dans une grille pour trouver vite le plus
/// proche d'un pixel.
struct Bords {
    segments: Vec<[f64; 4]>,
    cellule: f64,
    grille: std::collections::HashMap<(i64, i64), Vec<usize>>,
    /// Segments de chaque rangée de cellules qu'ils traversent (test dedans/dehors).
    rangees: std::collections::HashMap<i64, Vec<usize>>,
}

impl Bords {
    fn new(traces: &[Trace], (kx, ky): (f64, f64), hauteur_pt: f64, portee: f64) -> Bords {
        let px = |(x, y): (f64, f64)| (x * kx, (hauteur_pt - y) * ky);
        let mut segments = vec![];
        for t in traces.iter().filter(|t| t.ferme && t.points.len() >= 3) {
            for i in 0..t.points.len() {
                let (a, b) = (px(t.points[i]), px(t.points[(i + 1) % t.points.len()]));
                segments.push([a.0, a.1, b.0, b.1]);
            }
        }
        let cellule = portee.max(4.0);
        let (mut grille, mut rangees) = (std::collections::HashMap::<(i64, i64), Vec<usize>>::new(), std::collections::HashMap::<i64, Vec<usize>>::new());
        let c = |v: f64| (v / cellule).floor() as i64;
        for (i, s) in segments.iter().enumerate() {
            let (x0, x1, y0, y1) = (s[0].min(s[2]), s[0].max(s[2]), s[1].min(s[3]), s[1].max(s[3]));
            for cy in c(y0 - portee)..=c(y1 + portee) {
                for cx in c(x0 - portee)..=c(x1 + portee) {
                    grille.entry((cx, cy)).or_default().push(i);
                }
            }
            for cy in c(y0)..=c(y1) {
                rangees.entry(cy).or_default().push(i);
            }
        }
        Bords { segments, cellule, grille, rangees }
    }
}

/// Plaque (lignes de `largeur` pixels, 0 = pleine encre) étendue près des tracés : lue par
/// `lire`, écrite par `ecrire` ligne à ligne, en ne gardant en mémoire que les lignes à
/// portée. `page_pt` : format de la page des tracés.
pub fn etendre(largeur: usize, hauteur: usize, mut lire: impl FnMut() -> Result<Vec<u8>, String>, mut ecrire: impl FnMut(&[u8]) -> Result<(), String>, traces: &[Trace], page_pt: (f64, f64), e: &Etendre) -> Result<(), String> {
    let (kx, ky) = (largeur as f64 / page_pt.0, hauteur as f64 / page_pt.1);
    let k = (kx + ky) / 2.0;
    let (portee, seuil) = (e.largeur_mm * PT_PAR_MM * k, e.seuil_mm * PT_PAR_MM * k);
    let bords = Bords::new(traces, (kx, ky), page_pt.1, portee);
    let rayon = (portee + seuil).ceil() as usize + 2;
    let mut tampon: std::collections::VecDeque<Vec<u8>> = std::collections::VecDeque::new();
    let mut premier = 0usize;
    let c = |v: f64| (v / bords.cellule).floor() as i64;
    for y in 0..hauteur {
        while premier + tampon.len() < hauteur.min(y + rayon + 1) {
            tampon.push_back(lire()?);
        }
        while premier + rayon < y {
            tampon.pop_front();
            premier += 1;
        }
        let mut sortie = tampon[y - premier].clone();
        let py = y as f64 + 0.5;
        let cy = c(py);
        // Croisements de la ligne avec les contours : un pixel est dans la pièce s'il en a
        // un nombre impair à sa gauche.
        let mut croisements: Vec<f64> = bords.rangees.get(&cy).into_iter().flatten().filter_map(|&i| {
            let [x0, y0, x1, y1] = bords.segments[i];
            ((y0 <= py) != (y1 <= py)).then(|| x0 + (py - y0) * (x1 - x0) / (y1 - y0))
        }).collect();
        croisements.sort_by(f64::total_cmp);
        let mut colonnes: Vec<i64> = bords.grille.keys().filter(|(_, r)| *r == cy).map(|(cx, _)| *cx).collect();
        colonnes.sort_unstable();
        for cx in colonnes {
            let liste = &bords.grille[&(cx, cy)];
            let debut = ((cx as f64 * bords.cellule).floor().max(0.0) as usize).min(largeur);
            let fin = ((((cx + 1) as f64) * bords.cellule).floor().max(0.0) as usize).min(largeur);
            for (x, valeur) in sortie.iter_mut().enumerate().take(fin).skip(debut) {
                let p = (x as f64 + 0.5, py);
                let dedans = croisements.partition_point(|&v| v < p.0) % 2 == 1;
                if dedans && *valeur < 255 {
                    continue;
                }
                // Point le plus proche sur les contours.
                let mut proche = (f64::MAX, p);
                for &i in liste {
                    let [x0, y0, x1, y1] = bords.segments[i];
                    let (dx, dy) = (x1 - x0, y1 - y0);
                    let l2 = dx * dx + dy * dy;
                    let t = if l2 > 0.0 { (((p.0 - x0) * dx + (p.1 - y0) * dy) / l2).clamp(0.0, 1.0) } else { 0.0 };
                    let q = (x0 + t * dx, y0 + t * dy);
                    let d = (q.0 - p.0).hypot(q.1 - p.1);
                    if d < proche.0 {
                        proche = (d, q);
                    }
                }
                let (d, q) = proche;
                if d > if dedans { seuil } else { portee } || d < 1e-9 {
                    continue;
                }
                // On entre dans la pièce, pixel après pixel, jusqu'au seuil du contour : un
                // pixel blanc de la pièce près du contour se remplit aussi.
                let (u, depart, pas) = if dedans { (((p.0 - q.0) / d, (p.1 - q.1) / d), 1.0, (seuil - d).ceil() as usize) } else { (((q.0 - p.0) / d, (q.1 - p.1) / d), d + 0.5, seuil.ceil() as usize) };
                for j in 0..=pas {
                    let t = depart + j as f64;
                    let (sx, sy) = ((p.0 + u.0 * t).floor(), (p.1 + u.1 * t).floor());
                    if sx < 0.0 || sy < 0.0 || sx as usize >= largeur || (sy as usize) < premier || sy as usize >= premier + tampon.len() {
                        break;
                    }
                    let v = tampon[sy as usize - premier][sx as usize];
                    if v < 255 {
                        *valeur = (*valeur).min(v);
                        break;
                    }
                }
            }
        }
        ecrire(&sortie)?;
    }
    Ok(())
}

/// Boîte [x0, y0, x1, y1] de tous les points ; `None` sans point.
pub fn boite(traces: &[Trace]) -> Option<[f64; 4]> {
    let mut points = traces.iter().flat_map(|t| t.points.iter());
    let &(x, y) = points.next()?;
    Some(points.fold([x, y, x, y], |b, &(x, y)| [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn carre(x: f64, y: f64, c: f64) -> Trace {
        Trace { ton: "CutContour".into(), points: vec![(x, y), (x + c, y), (x + c, y + c), (x, y + c)], ferme: true }
    }

    /// Page 600 × 300 pt, carré de 100 pt en (50, 50) : rotation 90 → page 300 × 600, le
    /// carré de (150, 50) à (250, 150), départ du tracé en (250, 50) ; HP-GL attendu.
    #[test]
    fn rotation_90_coordonnees_attendues() {
        let m = Mise { rotation: 90, ..Default::default() };
        let p = m.appliquer(&[carre(50.0, 50.0, 100.0)], (600.0, 300.0), None);
        assert_eq!(p.taille, (300.0, 600.0));
        assert_eq!(p.traces[0].points, [(250.0, 50.0), (250.0, 150.0), (150.0, 150.0), (150.0, 50.0)]);
        // 1 pt = 35,28 unités de 0,01 mm : 250 pt → 8819, 150 → 5292, 50 → 1764.
        let h = crate::decoupe::hpgl(&p.traces, &[]);
        assert_eq!(h, "IN;SP1;PU8819,1764;PD8819,5292,5292,5292,5292,1764,8819,1764;PU;SP0;");
        for (r, attendu) in [(0, (50.0, 50.0)), (180, (450.0, 150.0)), (270, (50.0, 450.0))] {
            let p = Mise { rotation: r, ..Default::default() }.appliquer(&[carre(50.0, 50.0, 100.0)], (600.0, 300.0), None);
            let b = boite(&p.traces).unwrap();
            assert_eq!((b[0], b[1]), attendu, "rotation {r}");
        }
        // Le cadre de repérage tourne avec les tracés : il les entoure toujours.
        let p = m.appliquer(&[carre(50.0, 50.0, 100.0)], (600.0, 300.0), Some([10.0, 10.0, 590.0, 290.0]));
        assert_eq!(p.cadre, Some([10.0, 10.0, 290.0, 590.0]));
    }

    #[test]
    fn miroir_echelle_origine() {
        let p = Mise { miroir: true, ..Default::default() }.appliquer(&[carre(50.0, 50.0, 100.0)], (600.0, 300.0), None);
        assert_eq!(boite(&p.traces), Some([450.0, 50.0, 550.0, 150.0]));
        let p = Mise { echelle: [200.0, 50.0], ..Default::default() }.appliquer(&[carre(50.0, 50.0, 100.0)], (600.0, 300.0), None);
        assert_eq!((boite(&p.traces), p.taille), (Some([100.0, 25.0, 300.0, 75.0]), (1200.0, 150.0)), "échelle non proportionnelle");
        let p = Mise { depuis_origine: true, ..Default::default() }.appliquer(&[carre(50.0, 50.0, 100.0)], (600.0, 300.0), None);
        assert_eq!((boite(&p.traces), p.taille), (Some([0.0, 0.0, 100.0, 100.0]), (100.0, 100.0)));
    }

    /// 4.10 : décalage de 1 mm → le contour extérieur grandit de 1 mm de chaque côté, le
    /// trou se resserre d'autant, le cercle prend 1 mm de rayon ; un tracé ouvert ne bouge pas.
    #[test]
    fn decaler_la_coupe_vers_l_exterieur() {
        let mm = PT_PAR_MM;
        let mut trou = carre(30.0, 30.0, 40.0);
        trou.points.reverse(); // sens inverse : l'orientation ne doit pas compter
        let cercle = Trace { ton: "CutContour".into(), points: (0..64).map(|i| { let a = i as f64 / 64.0 * std::f64::consts::TAU; (300.0 + 50.0 * a.cos(), 50.0 + 50.0 * a.sin()) }).collect(), ferme: true };
        let ouvert = Trace { ton: "CutContour".into(), points: vec![(0.0, 200.0), (100.0, 200.0)], ferme: false };
        let p = Mise { decalage_mm: 1.0, ..Default::default() }.appliquer(&[carre(0.0, 0.0, 100.0), trou, cercle, ouvert.clone()], (600.0, 300.0), None);
        let b = |i: usize| boite(std::slice::from_ref(&p.traces[i])).unwrap();
        let proche = |a: [f64; 4], e: [f64; 4]| a.iter().zip(e).all(|(x, y)| (x - y).abs() < 1e-6);
        assert!(proche(b(0), [-mm, -mm, 100.0 + mm, 100.0 + mm]), "{:?}", b(0));
        assert!(proche(b(1), [30.0 + mm, 30.0 + mm, 70.0 - mm, 70.0 - mm]), "trou resserré : {:?}", b(1));
        // Polygone à 64 côtés : ses sommets s'écartent de d / cos(π/64).
        for &(x, y) in &p.traces[2].points {
            assert!(((x - 300.0).hypot(y - 50.0) - (50.0 + mm / (std::f64::consts::PI / 64.0).cos())).abs() < 1e-6);
        }
        assert_eq!(p.traces[3], ouvert);
        assert_eq!(Mise { decalage_mm: 6.0, ..Default::default() }.verifier(true), Err("mise_coupe_invalide"));
        assert_eq!(Mise { decalage_mm: 0.5, ..Default::default() }.verifier(true), Ok(()), "permis en impression et coupe");
    }

    /// Plaque de 40 × 40 mm à 254 dpi (10 px/mm), contour carré de 10 à 30 mm. Encre jusqu'au
    /// contour, largeur 2 mm → encre de 8 à 32 mm ; encre arrêtée à 1,5 mm du contour : seuil
    /// 1 mm, rien ; seuil 2 mm, prolongée jusqu'à 32 mm.
    #[test]
    fn etendre_l_image_pres_des_traces() {
        let mm = PT_PAR_MM;
        let contour = [Trace { ton: "CutContour".into(), points: vec![(10.0 * mm, 10.0 * mm), (30.0 * mm, 10.0 * mm), (30.0 * mm, 30.0 * mm), (10.0 * mm, 30.0 * mm)], ferme: true }];
        let page = (40.0 * mm, 40.0 * mm);
        let essai = |encre: std::ops::Range<usize>, seuil_mm: f64| {
            let lignes: Vec<Vec<u8>> = (0..400).map(|y| (0..400).map(|x| if encre.contains(&x) && encre.contains(&y) { 0 } else { 255 }).collect()).collect();
            let mut lues = lignes.into_iter();
            let mut sortie: Vec<Vec<u8>> = vec![];
            etendre(400, 400, || Ok(lues.next().unwrap()), |l| {
                sortie.push(l.to_vec());
                Ok(())
            }, &contour, page, &Etendre { largeur_mm: 2.0, seuil_mm }).unwrap();
            // Boîte de l'encre en mm (colonnes et lignes encrées au milieu de la plaque).
            let colonnes: Vec<usize> = (0..400).filter(|&x| sortie[200][x] == 0).collect();
            let rangs: Vec<usize> = (0..400).filter(|&y| sortie[y][200] == 0).collect();
            assert_eq!(colonnes.len(), colonnes.last().unwrap() - colonnes[0] + 1, "encre d'un seul tenant");
            [colonnes[0], *colonnes.last().unwrap() + 1, rangs[0], *rangs.last().unwrap() + 1].map(|v| v as f64 / 10.0)
        };
        assert_eq!(essai(100..300, 0.5), [8.0, 32.0, 8.0, 32.0], "étendue de 2 mm");
        assert_eq!(essai(115..285, 1.0), [11.5, 28.5, 11.5, 28.5], "encre au-delà du seuil : rien");
        assert_eq!(essai(115..285, 2.0), [8.0, 32.0, 8.0, 32.0], "seuil de 2 mm : prolongée");
        assert_eq!(Etendre { largeur_mm: 0.0, seuil_mm: 1.0 }.verifier(), Err("etendre_invalide"));
    }

    #[test]
    fn reserve_a_la_decoupe_seule() {
        assert_eq!(Mise::default().verifier(true), Ok(()));
        assert_eq!(Mise { rotation: 270, ..Default::default() }.verifier(true), Ok(()), "rotation : aussi en impression et coupe");
        for m in [Mise { miroir: true, ..Default::default() }, Mise { copies: [2, 1], ..Default::default() }, Mise { echelle: [100.0, 90.0], ..Default::default() }, Mise { depuis_origine: true, ..Default::default() }] {
            assert_eq!(m.verifier(true), Err("mise_decoupe_seule"));
            assert_eq!(m.verifier(false), Ok(()));
        }
        assert_eq!(Mise { rotation: 45, ..Default::default() }.verifier(false), Err("mise_coupe_invalide"));
        assert_eq!(Mise { copies: [100, 11], ..Default::default() }.verifier(false), Err("mise_coupe_invalide"));
        // Ancien job.json : pas de champ, mise neutre.
        assert_eq!(serde_json::from_str::<Mise>("{}").unwrap(), Mise::default());
    }
}
