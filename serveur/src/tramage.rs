//! Tramage 2 bits de l'Epson SC-S80600 (roadmap 2.3) : un plan d'encre 8 bits de la
//! séparation → un `escp2::Plan` (0 rien, 1 petit, 2 moyen, 3 gros point).
//!
//! Méthode publique : diffusion d'erreur multi-niveaux (noyau de Floyd–Steinberg, 1976),
//! parcours serpentin (une ligne sur deux de droite à gauche, noyau en miroir), et seuil
//! légèrement bruité (pseudo-aléatoire déterministe, graine par encre) pour casser les
//! motifs réguliers que la diffusion pure dessine sur certains aplats (damier à 50 %).
//!
//! Chaque taille de point vaut une couverture équivalente (`poids`, gros = 1 en général).
//! En chaque point on vise `entrée/255 + erreur reçue`, on choisit l'un des deux niveaux
//! qui encadrent cette valeur, et l'écart part aux voisins : la couverture moyenne égale
//! la consigne. Courbe de répartition S→M→L qui en découle (poids 0,33 / 0,66 / 1) :
//! de 0 à 33 % seuls des petits points, de plus en plus serrés ; de 33 à 66 % les moyens
//! remplacent peu à peu les petits ; de 66 à 100 % les gros remplacent les moyens. Les
//! gros points n'apparaissent donc que dans les tons foncés, le grain reste fin dans les
//! clairs. Les poids réels (taille de goutte sur le vinyle) sont à mesurer (2.4/2.8).

use crate::escp2::{Encre, Plan};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Tramage {
    /// Couverture équivalente d'un petit, d'un moyen et d'un gros point, croissante, 0..=1.
    pub poids: [f32; 3],
    /// Bruit du seuil, en fraction de l'écart entre deux niveaux (0 = diffusion pure).
    pub bruit: f32,
}

impl Default for Tramage {
    /// Poids : tiers réguliers, faute de mesure des gouttes. Bruit 0,5 : pic 3,8 au
    /// contrôle de motif périodique du test `degrade_sans_motif_periodique` (seuil 6) ;
    /// mesuré aussi : 0,35 → 4,8, 0,25 → 18, 0,1 → 32, diffusion pure → 62.
    fn default() -> Self {
        Tramage { poids: [0.33, 0.66, 1.0], bruit: 0.5 }
    }
}

pub fn tramer(encre: Encre, valeurs: &[u8], largeur: usize, p: &Tramage) -> Result<Plan, String> {
    if largeur == 0 || !valeurs.len().is_multiple_of(largeur) {
        return Err(format!("plan {encre:?} : taille incompatible avec la largeur {largeur}"));
    }
    let w = [0.0, p.poids[0], p.poids[1], p.poids[2]];
    if !(w.windows(2).all(|n| n[0] < n[1]) && w[3] <= 1.0) {
        return Err(format!("poids de points {:?} : croissants dans 0..=1 attendus", p.poids));
    }
    let hauteur = valeurs.len() / largeur;
    // Erreur de la ligne courante et de la suivante, une case de marge de chaque côté.
    let (mut ligne, mut suivante) = (vec![0f32; largeur + 2], vec![0f32; largeur + 2]);
    let mut points = vec![0u8; valeurs.len()];
    let mut graine = 0x9E37_79B9u32 ^ (encre.code() as u32 + 1).wrapping_mul(0x85EB_CA6B);
    for y in 0..hauteur {
        let vers_droite = y % 2 == 0;
        for j in 0..largeur {
            let x = if vers_droite { j } else { largeur - 1 - j };
            let i = y * largeur + x;
            let voulu = valeurs[i] as f32 / 255.0 + ligne[x + 1];
            let haut = (1..3).find(|&n| voulu < w[n]).unwrap_or(3);
            graine ^= graine << 13;
            graine ^= graine >> 17;
            graine ^= graine << 5;
            let alea = (graine >> 8) as f32 / (1u32 << 24) as f32 - 0.5;
            let seuil = (w[haut - 1] + w[haut]) / 2.0 + p.bruit * alea * (w[haut] - w[haut - 1]);
            let niveau = if voulu >= seuil { haut } else { haut - 1 };
            points[i] = niveau as u8;
            let e = voulu - w[niveau];
            let (devant, derriere) = if vers_droite { (x + 2, x) } else { (x, x + 2) };
            ligne[devant] += e * 7.0 / 16.0;
            suivante[derriere] += e * 3.0 / 16.0;
            suivante[x + 1] += e * 5.0 / 16.0;
            suivante[devant] += e / 16.0;
        }
        std::mem::swap(&mut ligne, &mut suivante);
        suivante.fill(0.0);
    }
    Ok(Plan { encre, points })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::escp2::{self, Travail};
    use crate::separation::{self, Cmjn, Separation};

    fn couverture(plan: &Plan, p: &Tramage) -> f32 {
        let w = [0.0, p.poids[0], p.poids[1], p.poids[2]];
        plan.points.iter().map(|&n| w[n as usize]).sum::<f32>() / plan.points.len() as f32
    }

    #[test]
    fn aplats_a_la_consigne() {
        let p = Tramage::default();
        for (consigne, entree) in [(0.25, 64u8), (0.5, 128), (0.75, 191)] {
            let plan = tramer(Encre::C, &vec![entree; 256 * 256], 256, &p).unwrap();
            let mesure = couverture(&plan, &p);
            println!("aplat {consigne} : couverture {mesure:.4}");
            assert!((mesure - consigne).abs() <= 0.02, "aplat {consigne} : {mesure}");
            assert!((mesure - entree as f32 / 255.0).abs() <= 0.005, "écart à l'entrée exacte");
        }
        // Répartition : un ton clair n'a que des petits points, le plein que des gros.
        let clair = tramer(Encre::M, &vec![51; 128 * 128], 128, &p).unwrap();
        assert!(clair.points.iter().all(|&n| n <= 1));
        assert!(tramer(Encre::M, &[255; 64], 8, &p).unwrap().points.iter().all(|&n| n == 3));
        assert!(tramer(Encre::M, &[0; 64], 8, &p).unwrap().points.iter().all(|&n| n == 0));
    }

    /// Pic du spectre de puissance moyen des lignes (DFT naïve sur 256 échantillons),
    /// rapporté à la moyenne du spectre hors fréquence nulle, pour chaque bande de lignes.
    fn pics(plan: &Plan, p: &Tramage, lignes_par_bande: usize) -> Vec<f32> {
        let w = [0.0, p.poids[0], p.poids[1], p.poids[2]];
        let (n, bandes) = (256, plan.points.len() / 256 / lignes_par_bande);
        (0..bandes)
            .filter_map(|b| {
                let mut puissance = vec![0f32; n / 2 + 1];
                for l in 0..lignes_par_bande {
                    let debut = (b * lignes_par_bande + l) * n;
                    let s: Vec<f32> = plan.points[debut..debut + n].iter().map(|&v| w[v as usize]).collect();
                    let moyenne = s.iter().sum::<f32>() / n as f32;
                    for (k, pk) in puissance.iter_mut().enumerate().skip(1) {
                        let (mut re, mut im) = (0f32, 0f32);
                        for (x, v) in s.iter().enumerate() {
                            let a = std::f32::consts::TAU * (k * x) as f32 / n as f32;
                            re += (v - moyenne) * a.cos();
                            im -= (v - moyenne) * a.sin();
                        }
                        *pk += re * re + im * im;
                    }
                }
                let moyenne = puissance[1..].iter().sum::<f32>() / (n / 2) as f32;
                (moyenne > 1e-3).then(|| puissance[1..].iter().fold(0f32, |a, &b| a.max(b)) / moyenne)
            })
            .collect()
    }

    /// Dégradé vertical 0→255 (4 lignes par ton), spectres moyens par bande de 32 lignes
    /// (8 tons). Moyenner 32 lignes lisse le spectre d'un tramage apériodique (bruit bleu :
    /// pic ≤ 4 fois la moyenne, mesuré) ; un motif régulier concentre l'énergie sur une ou
    /// deux fréquences (diffusion pure : 62). Seuil 6 entre les deux ; la diffusion sans
    /// bruit doit le dépasser, sinon c'est l'outil de mesure qui est cassé.
    #[test]
    fn degrade_sans_motif_periodique() {
        let degrade: Vec<u8> = (0..1024 * 256).map(|i| (i / 256 / 4) as u8).collect();
        let p = Tramage::default();
        let avec = pics(&tramer(Encre::Y, &degrade, 256, &p).unwrap(), &p, 32);
        let pur = Tramage { bruit: 0.0, ..p };
        let sans = pics(&tramer(Encre::Y, &degrade, 256, &pur).unwrap(), &pur, 32);
        let max = |v: &[f32]| v.iter().fold(0f32, |a, &b| a.max(b));
        println!("pic max : bruité {:.2}, diffusion pure {:.2}", max(&avec), max(&sans));
        println!("bandes bruitées : {avec:.1?}");
        assert!(max(&avec) < 6.0, "motif périodique : {avec:?}");
        assert!(max(&sans) > 6.0, "le contrôle ne voit plus le motif de la diffusion pure");
    }

    #[test]
    fn reglages_refuses() {
        let p = Tramage { poids: [0.5, 0.4, 1.0], ..Tramage::default() };
        assert!(tramer(Encre::K, &[0; 16], 4, &p).is_err());
        assert!(tramer(Encre::K, &[0; 15], 4, &Tramage::default()).is_err());
    }

    #[test]
    fn bout_en_bout_cmjn_vers_flux_escp2() {
        let (largeur, hauteur) = (96, 40);
        let canal = |f: fn(usize, usize) -> u8| -> Vec<u8> { (0..largeur * hauteur).map(|i| f(i % largeur, i / largeur)).collect() };
        let c = canal(|x, _| (x * 255 / 95) as u8);
        let m = canal(|_, y| (y * 255 / 39) as u8);
        let y = canal(|x, y| ((x + y) % 256) as u8);
        let k = canal(|x, y| if x > 60 && y > 20 { 255 } else { 0 });
        let s = separation::separer(&Cmjn { c: &c, m: &m, y: &y, k: &k, or: None, rd: None }, &Separation::default()).unwrap();
        let p = Tramage::default();
        let plans = s.iter().map(|(e, v)| tramer(*e, v, largeur, &p).unwrap()).collect();
        let t = Travail { nom: "rampe".into(), poste: "atelier".into(), utilisateur: "essai".into(), largeur, hauteur, dpi: 720, x: 0, y: 0, page_largeur: 200, page_hauteur: 100, plans };
        let f = escp2::flux(&t).unwrap();
        assert!(f.len() > 16384);
    }
}
