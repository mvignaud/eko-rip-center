//! Mise en coupe d'un travail : ce que l'opérateur règle sur le fichier de coupe sans
//! refaire le rendu, comme les outils du fichier de CUT-Server. Appliquée aux tracés de la
//! page (`prod/decoupe-NNN.json`) chaque fois qu'un fichier de coupe est écrit ou envoyé.
//!
//! - Rotation par quart de tour (roadmap 4.8) : l'orientation du travail pour la machine,
//!   pas le fichier source ; l'opérateur tourne le support d'autant.
//! - Découpe seule (roadmap 4.9) : miroir, échelle proportionnelle ou non, copies X/Y avec
//!   écarts, départ depuis l'origine de la machine. Sans impression seulement : sur un
//!   travail imprimé, la coupe ne suivrait plus l'encre.

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
    /// Copies en X et en Y, la page entière répétée.
    #[serde(default = "un")]
    pub copies: [u32; 2],
    /// Blanc entre deux copies (X, Y), en mm, d'un tracé à l'autre.
    #[serde(default)]
    pub ecart_mm: [f64; 2],
    /// Les tracés commencent à l'origine de la machine (le blanc autour de la page ôté).
    #[serde(default)]
    pub depuis_origine: bool,
}

impl Default for Mise {
    fn default() -> Self {
        Mise { rotation: 0, miroir: false, echelle: cent(), copies: un(), ecart_mm: [0.0; 2], depuis_origine: false }
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
            && self.ecart_mm.iter().all(|e| (0.0..=1000.0).contains(e));
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
