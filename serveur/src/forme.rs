//! Imbrication sur la forme : chaque élément est sa silhouette (surface couverte par ses
//! contours de découpe fermés, ou son rectangle), tramée sur une grille et élargie de
//! l'espacement ; les éléments, du plus grand au plus petit, vont à la place libre la plus
//! basse puis la plus à gauche du rouleau, à 0, 90, 180 ou 270°. Les recouvrements se
//! testent par masques de bits (64 cases par mot).
// ponytail: placement « plus bas, plus à gauche » sur une grille de quelques mm, sans
// angles libres ni recherche d'ordre ; un algorithme à polygones exacts (NFP) gagnerait
// encore un peu de matière sur les formes très imbriquées.

use crate::imbrication::{Place, Taille};

/// Une silhouette : taille de la page et polygones fermés, en points, origine au coin bas
/// gauche de la page. Sans polygone, la page entière.
pub struct Silhouette {
    pub taille: Taille,
    pub polygones: Vec<Vec<(f64, f64)>>,
}

/// Masque binaire d'une silhouette tournée et élargie : `lignes[y]` en mots de 64 cases.
struct Masque {
    largeur: usize,
    hauteur: usize,
    lignes: Vec<Vec<u64>>,
}

fn dans(p: (f64, f64), poly: &[(f64, f64)]) -> bool {
    let mut dedans = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let ((xi, yi), (xj, yj)) = (poly[i], poly[j]);
        if (yi > p.1) != (yj > p.1) && p.0 < (xj - xi) * (p.1 - yi) / (yj - yi) + xi {
            dedans = !dedans;
        }
        j = i;
    }
    dedans
}

/// Aire d'un polygone fermé (formule du lacet), en unités au carré.
pub fn aire(p: &[(f64, f64)]) -> f64 {
    let n = p.len();
    (0..n).map(|i| { let (a, b) = (p[i], p[(i + 1) % n]); a.0 * b.1 - b.0 * a.1 }).sum::<f64>().abs() / 2.0
}

/// Point (x, y) de la page tourné de `angle` à gauche, ramené au premier quadrant ; même
/// pose que `imbrication::matrice_pose`.
pub fn tourner((x, y): (f64, f64), t: Taille, angle: u16) -> (f64, f64) {
    match angle {
        90 => (t.hauteur - y, x),
        180 => (t.largeur - x, t.hauteur - y),
        270 => (y, t.largeur - x),
        _ => (x, y),
    }
}

impl Masque {
    /// Trame la silhouette tournée (pas `pas` en points), élargie de `marge` cases.
    fn tramer(s: &Silhouette, angle: u16, pas: f64, marge: usize) -> Masque {
        let (l, h) = if angle % 180 == 90 { (s.taille.hauteur, s.taille.largeur) } else { (s.taille.largeur, s.taille.hauteur) };
        let (nl, nh) = (((l / pas).ceil() as usize).max(1), ((h / pas).ceil() as usize).max(1));
        let polys: Vec<Vec<(f64, f64)>> = s.polygones.iter().filter(|p| p.len() >= 3).map(|p| p.iter().map(|&q| tourner(q, s.taille, angle)).collect()).collect();
        // Case pleine si son centre est dans un polygone ; un trou reste plein (on ne pose
        // rien dans une pièce qui sera détachée).
        let mut grille = vec![vec![polys.is_empty(); nl]; nh];
        if !polys.is_empty() {
            for (cy, ligne) in grille.iter_mut().enumerate() {
                for (cx, c) in ligne.iter_mut().enumerate() {
                    let p = ((cx as f64 + 0.5) * pas, (cy as f64 + 0.5) * pas);
                    *c = polys.iter().any(|poly| dans(p, poly));
                }
            }
        }
        // Les bords tombent entre deux cases : toute case touchée par un segment compte aussi,
        // pour ne jamais rogner une forme.
        for poly in &polys {
            for k in 0..poly.len() {
                let (a, b) = (poly[k], poly[(k + 1) % poly.len()]);
                let n = (((b.0 - a.0).abs().max((b.1 - a.1).abs()) / pas).ceil() as usize).max(1);
                for i in 0..=n {
                    let f = i as f64 / n as f64;
                    let (x, y) = (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f);
                    let (cx, cy) = (((x / pas).max(0.0) as usize).min(nl - 1), ((y / pas).max(0.0) as usize).min(nh - 1));
                    grille[cy][cx] = true;
                }
            }
        }
        // Élargissement carré de `marge` cases (espacement).
        let (wl, wh) = (nl + 2 * marge, nh + 2 * marge);
        let mut lignes = vec![vec![0u64; wl.div_ceil(64)]; wh];
        for (cy, ligne) in grille.iter().enumerate() {
            for (cx, &plein) in ligne.iter().enumerate() {
                if plein {
                    for ligne_masque in &mut lignes[cy..=cy + 2 * marge] {
                        for x in cx..=cx + 2 * marge {
                            ligne_masque[x / 64] |= 1 << (x % 64);
                        }
                    }
                }
            }
        }
        Masque { largeur: wl, hauteur: wh, lignes }
    }

    /// Mot `k` de la ligne `r`, le masque décalé de `x` cases vers la droite.
    fn mot_decale(&self, r: usize, x: usize, k: usize) -> u64 {
        let (q, s) = (x / 64, x % 64);
        let mot = |i: usize| self.lignes[r].get(i).copied().unwrap_or(0);
        if k < q {
            return 0;
        }
        let i = k - q;
        if s == 0 { mot(i) } else { (mot(i) << s) | if i == 0 { 0 } else { mot(i - 1) >> (64 - s) } }
    }
}

/// Rouleau tramé : une ligne de mots par rangée de cases, allongée à la demande.
struct Rouleau {
    largeur: usize,
    lignes: Vec<Vec<u64>>,
}

impl Rouleau {
    fn libre(&self, m: &Masque, x: usize, y: usize) -> bool {
        if x + m.largeur > self.largeur {
            return false;
        }
        for r in 0..m.hauteur {
            let Some(ligne) = self.lignes.get(y + r) else { return true };
            let (debut, fin) = (x / 64, (x + m.largeur).div_ceil(64));
            if ligne[debut..fin].iter().enumerate().any(|(j, &occupe)| occupe & m.mot_decale(r, x, debut + j) != 0) {
                return false;
            }
        }
        true
    }

    fn poser(&mut self, m: &Masque, x: usize, y: usize) {
        let mots = self.largeur.div_ceil(64);
        while self.lignes.len() < y + m.hauteur {
            self.lignes.push(vec![0; mots]);
        }
        for r in 0..m.hauteur {
            for k in x / 64..(x + m.largeur).div_ceil(64) {
                let mot = m.mot_decale(r, x, k);
                self.lignes[y + r][k] |= mot;
            }
        }
    }
}

/// Place les silhouettes sur la laize (points) avec `espace` (points) entre elles. Rend les
/// places (coin bas gauche de la case de la page posée, angle) et la taille de la planche.
pub fn placer(silhouettes: &[Silhouette], laize: f64, espace: f64, rotation: bool) -> Result<(Vec<Place>, Taille), String> {
    // Grille : au plus ~800 cases sur la laize, et jamais plus fine que 1 mm.
    let pas = (laize / 800.0).max(72.0 / 25.4);
    // Chaque masque est élargi de la moitié de l'espacement : deux voisins en gardent un entier.
    let marge = ((espace / 2.0) / pas).ceil() as usize;
    let largeur = (laize / pas).floor() as usize + 2 * marge;
    let mut rouleau = Rouleau { largeur, lignes: vec![] };
    let angles: &[u16] = if rotation { &[0, 90, 180, 270] } else { &[0] };
    let mut ordre: Vec<usize> = (0..silhouettes.len()).collect();
    let aire = |i: usize| silhouettes[i].taille.largeur * silhouettes[i].taille.hauteur;
    ordre.sort_by(|&a, &b| aire(b).total_cmp(&aire(a)));
    let mut places = vec![];
    let (mut large, mut haut) = (0.0f64, 0.0f64);
    for i in ordre {
        let s = &silhouettes[i];
        let mut meilleur: Option<(usize, usize, u16, Masque)> = None;
        for &angle in angles {
            let m = Masque::tramer(s, angle, pas, marge);
            if m.largeur > largeur {
                continue;
            }
            // Place la plus basse, puis la plus à gauche ; inutile de chercher plus haut que
            // la meilleure déjà trouvée.
            let limite = meilleur.as_ref().map_or(usize::MAX, |b| b.1);
            'recherche: for y in 0..=rouleau.lignes.len().min(limite) {
                for x in 0..=(largeur - m.largeur) {
                    if rouleau.libre(&m, x, y) {
                        if meilleur.as_ref().is_none_or(|b| (y, x) < (b.1, b.0)) {
                            meilleur = Some((x, y, angle, m));
                        }
                        break 'recherche;
                    }
                }
            }
        }
        let Some((x, y, angle, m)) = meilleur else {
            return Err(format!("l'élément {} ({:.0} × {:.0} mm) est plus large que la laize", i + 1, s.taille.largeur * 25.4 / 72.0, s.taille.hauteur * 25.4 / 72.0));
        };
        rouleau.poser(&m, x, y);
        // Le masque commence `marge` cases avant la page ; la grille, `marge` cases avant 0.
        let (px, py) = (x as f64 * pas, y as f64 * pas);
        let (l, h) = if angle % 180 == 90 { (s.taille.hauteur, s.taille.largeur) } else { (s.taille.largeur, s.taille.hauteur) };
        places.push(Place { indice: i, x: px, y: py, angle });
        large = large.max(px + l);
        haut = haut.max(py + h);
    }
    Ok((places, Taille { largeur: large, hauteur: haut }))
}

#[cfg(test)]
mod tests {
    #[test]
    fn aire_du_lacet() {
        assert_eq!(aire(&[(0.0, 0.0), (10.0, 0.0), (0.0, 10.0)]), 50.0);
        assert_eq!(aire(&[(0.0, 0.0), (0.0, 4.0), (5.0, 4.0), (5.0, 0.0)]), 20.0);
    }

    use super::*;
    const MM: f64 = 72.0 / 25.4;

    fn rond(diametre_mm: f64) -> Silhouette {
        let r = diametre_mm * MM / 2.0;
        let poly = (0..64).map(|k| { let a = k as f64 * std::f64::consts::TAU / 64.0; (r + r * a.cos(), r + r * a.sin()) }).collect();
        Silhouette { taille: Taille { largeur: 2.0 * r, hauteur: 2.0 * r }, polygones: vec![poly] }
    }

    /// Triangle rectangle de 100 × 100 mm dans sa page carrée.
    fn triangle() -> Silhouette {
        let c = 100.0 * MM;
        Silhouette { taille: Taille { largeur: c, hauteur: c }, polygones: vec![vec![(0.0, 0.0), (c, 0.0), (0.0, c)]] }
    }

    #[test]
    fn deux_triangles_tete_beche_tiennent_dans_un_carre() {
        // Laize de 105 mm : un seul carré de 100 mm par rangée ; à la forme, le second
        // triangle tourné de 180° vient dans la moitié vide du premier.
        let (places, planche) = placer(&[triangle(), triangle()], 105.0 * MM, 0.0, true).unwrap();
        assert!(planche.hauteur / MM < 120.0, "longueur {} mm (deux carrés en rangées feraient 200)", planche.hauteur / MM);
        assert!(places.iter().any(|p| p.angle == 180), "{places:?}");
        // Sans rotation : impossible de les emboîter.
        let (_, droit) = placer(&[triangle(), triangle()], 105.0 * MM, 0.0, false).unwrap();
        assert!(droit.hauteur / MM > 190.0, "{}", droit.hauteur / MM);
    }

    #[test]
    fn ronds_sans_chevauchement_dans_la_laize() {
        let ronds: Vec<Silhouette> = (0..30).map(|k| rond(40.0 + (k % 3) as f64 * 15.0)).collect();
        let espace = 3.0 * MM;
        let (places, planche) = placer(&ronds, 300.0 * MM, espace, true).unwrap();
        assert_eq!(places.len(), 30);
        assert!(planche.largeur <= 300.0 * MM + 1e-6, "{}", planche.largeur / MM);
        // Deux ronds gardent au moins l'espacement, moins une case de grille.
        let pas = (300.0 * MM / 800.0).max(MM);
        let centre = |p: &Place| { let r = ronds[p.indice].taille.largeur / 2.0; (p.x + r, p.y + r, r) };
        for (i, a) in places.iter().enumerate() {
            let (ax, ay, ar) = centre(a);
            assert!(ax - ar >= -1e-6 && ax + ar <= 300.0 * MM + 1e-6, "dans la laize");
            for b in &places[i + 1..] {
                let (bx, by, br) = centre(b);
                let d = ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt();
                assert!(d >= ar + br + espace - 2.0 * pas, "chevauchement : {} mm entre bords", (d - ar - br) / MM);
            }
        }
        // Mieux qu'en rangées : surface des ronds sur surface de rouleau utilisée.
        let aire: f64 = ronds.iter().map(|r| std::f64::consts::PI * (r.taille.largeur / 2.0).powi(2)).sum();
        assert!(aire / (300.0 * MM * planche.hauteur) > 0.5, "taux {}", aire / (300.0 * MM * planche.hauteur));
    }

    #[test]
    fn trop_large_refuse() {
        assert!(placer(&[rond(200.0)], 150.0 * MM, 0.0, true).is_err());
    }
}
