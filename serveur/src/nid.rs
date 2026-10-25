//! Imbrication dans la file (roadmap 5.1 à 5.3) : les impressions en attente d'une même
//! imprimante, compatibles entre elles, posées ensemble sur une planche ; la planche part
//! quand son déclencheur le dit (ou sur « Imprimer maintenant »). Logique inspirée de la
//! file d'ONYX (stratégies de placement, déclenchement au temps ou à la surface) ; rien n'en
//! est repris. Ici, le calcul seul : la file et les travaux sont dans `main.rs`.

use crate::imbrication::{self, PT_PAR_MM, Place, Taille};
use serde::{Deserialize, Serialize};

/// Comment les impressions d'une machine sont posées sur le rouleau.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Strategie {
    /// Chaque impression sort seule, dès qu'elle est prête (comportement d'avant).
    #[default]
    UnParUn,
    /// Côte à côte dans l'ordre de priorité, sans rotation ni réordonnancement.
    Groupe,
    /// Économie de support : toute la planche est recalculée à chaque arrivée, par hauteur
    /// décroissante, avec le quart de tour quand il raccourcit la planche.
    Economie,
}

/// Réglage d'imbrication d'une imprimante (dans `machines.json`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Reglage {
    #[serde(default)]
    pub strategie: Strategie,
    /// Départ temporisé : minutes depuis l'arrivée du plus ancien travail de la planche.
    #[serde(default)]
    pub delai_min: Option<u32>,
    /// Départ à la surface : m² de travaux (copies comprises) accumulés.
    #[serde(default)]
    pub surface_m2: Option<f64>,
    /// Longueur cible de la planche, en mm : les travaux de moindre priorité qui la feraient
    /// dépasser attendent la planche suivante, et une planche pleine part d'elle-même.
    #[serde(default)]
    pub longueur_cible_mm: Option<f64>,
    /// Départ quand la planche atteint ce pourcentage de la longueur cible.
    #[serde(default)]
    pub remplissage_pct: Option<f64>,
    #[serde(default = "espace_defaut")]
    pub espace_mm: f64,
    /// Blanc sous et au-dessus de la planche pour la table de découpe (roadmap 5.5).
    #[serde(default)]
    pub marge_haut_mm: f64,
    #[serde(default)]
    pub marge_bas_mm: f64,
    /// Contour de toute la planche, couche `NestTrimBox` (roadmap 5.5).
    #[serde(default)]
    pub contour: bool,
}

fn espace_defaut() -> f64 {
    5.0
}

impl Default for Reglage {
    fn default() -> Self {
        Reglage { strategie: Strategie::UnParUn, delai_min: None, surface_m2: None, longueur_cible_mm: None, remplissage_pct: None, espace_mm: espace_defaut(), marge_haut_mm: 0.0, marge_bas_mm: 0.0, contour: false }
    }
}

impl Reglage {
    pub fn valide(&self) -> bool {
        self.delai_min.is_none_or(|m| (1..=1440).contains(&m))
            && self.surface_m2.is_none_or(|s| (0.01..=1000.0).contains(&s))
            && self.longueur_cible_mm.is_none_or(|l| (100.0..=100_000.0).contains(&l))
            && self.remplissage_pct.is_none_or(|p| (1.0..=100.0).contains(&p))
            && (self.remplissage_pct.is_none() || self.longueur_cible_mm.is_some())
            && (0.0..=100.0).contains(&self.espace_mm)
            && (0.0..=200.0).contains(&self.marge_haut_mm)
            && (0.0..=200.0).contains(&self.marge_bas_mm)
    }

    pub fn finition(&self) -> imbrication::Finition {
        imbrication::Finition { marge_bas: self.marge_bas_mm * PT_PAR_MM, marge_haut: self.marge_haut_mm * PT_PAR_MM, contour: self.contour }
    }
}

/// Une impression en attente, dans l'ordre de la file.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Candidat {
    /// Page à poser (points), sans repères : la planche a les siens.
    pub taille: Taille,
    pub copies: usize,
    /// Échéance du travail (secondes Unix) : la plus proche passe d'abord.
    pub echeance: Option<u64>,
    pub arrivee: u64,
}

/// Planche calculée : les candidats retenus (indices, par priorité), les copies déroulées
/// (`elements[k]` : candidat de la copie k) et leurs places, la boîte des éléments.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub retenus: Vec<usize>,
    pub elements: Vec<usize>,
    pub places: Vec<Place>,
    pub planche: Taille,
    /// Candidats laissés pour la planche suivante (longueur cible atteinte).
    pub reportes: Vec<usize>,
}

impl Plan {
    /// Surface des éléments posés, en m².
    pub fn surface_m2(&self, c: &[Candidat]) -> f64 {
        self.elements.iter().map(|&i| c[i].taille.largeur * c[i].taille.hauteur).sum::<f64>() / (PT_PAR_MM * PT_PAR_MM * 1e6)
    }

    /// Part du rouleau consommé (laize × longueur, marges haute et basse comprises) couverte
    /// par les éléments, de 0 à 1.
    pub fn remplissage(&self, c: &[Candidat], laize_mm: f64, marges_mm: f64) -> f64 {
        let rouleau = laize_mm * (self.planche.hauteur / PT_PAR_MM + marges_mm) / 1e6;
        if rouleau > 0.0 { self.surface_m2(c) / rouleau } else { 0.0 }
    }
}

/// Ordre de priorité (roadmap 5.6) : échéance la plus proche d'abord, sans échéance ensuite,
/// à égalité l'ordre de la file.
pub fn priorite(c: &[Candidat]) -> Vec<usize> {
    let mut ordre: Vec<usize> = (0..c.len()).collect();
    ordre.sort_by_key(|&i| (c[i].echeance.is_none(), c[i].echeance, i));
    ordre
}

fn poser(c: &[Candidat], retenus: &[usize], laize: f64, r: &Reglage) -> Result<(Vec<usize>, Vec<Place>, Taille), String> {
    let elements: Vec<usize> = retenus.iter().flat_map(|&i| std::iter::repeat_n(i, c[i].copies.max(1))).collect();
    let tailles: Vec<Taille> = elements.iter().map(|&i| c[i].taille).collect();
    let espace = r.espace_mm * PT_PAR_MM;
    let (places, planche) = match r.strategie {
        Strategie::Economie => imbrication::placer(&tailles, laize, espace, true)?,
        Strategie::Groupe | Strategie::UnParUn => imbrication::placer_en_ordre(&tailles, laize, espace)?,
    };
    Ok((elements, places, planche))
}

/// Planche des candidats sur `laize_mm`, par priorité, jusqu'à la longueur cible (au moins
/// un travail). Un candidat qui ne tient pas seul sur la laize est laissé de côté : sa
/// sortie le dit (format trop grand). `None` : rien à poser.
// ponytail: la planche est recalculée en entier à chaque candidat ajouté (O(n²) placements) ;
// quelques dizaines de travaux en file, c'est instantané.
pub fn planifier(c: &[Candidat], laize_mm: f64, r: &Reglage) -> Option<Plan> {
    let laize = laize_mm * PT_PAR_MM;
    let mut plan: Option<Plan> = None;
    let mut retenus: Vec<usize> = vec![];
    let mut reportes = vec![];
    for i in priorite(c) {
        if poser(c, &[i], laize, r).is_err() {
            continue;
        }
        if !reportes.is_empty() {
            // Planche déjà pleine : les suivants attendent aussi, dans l'ordre.
            reportes.push(i);
            continue;
        }
        retenus.push(i);
        let Ok((elements, places, planche)) = poser(c, &retenus, laize, r) else {
            retenus.pop();
            continue;
        };
        let trop_long = r.longueur_cible_mm.is_some_and(|l| planche.hauteur / PT_PAR_MM > l + 1e-6);
        if trop_long && retenus.len() > 1 {
            retenus.pop();
            reportes.push(i);
            continue;
        }
        plan = Some(Plan { retenus: retenus.clone(), elements, places, planche, reportes: vec![] });
    }
    plan.map(|p| Plan { reportes, ..p })
}

/// Le déclencheur est-il atteint ? Délai depuis le plus ancien, surface accumulée,
/// remplissage de la longueur cible, ou planche pleine (des travaux reportés). Sans aucun
/// déclencheur : seulement sur « Imprimer maintenant ».
pub fn pret(plan: &Plan, c: &[Candidat], r: &Reglage, maintenant: u64) -> bool {
    let plus_ancien = plan.retenus.iter().map(|&i| c[i].arrivee).min().unwrap_or(maintenant);
    let delai = r.delai_min.is_some_and(|m| maintenant.saturating_sub(plus_ancien) >= u64::from(m) * 60);
    let surface = r.surface_m2.is_some_and(|s| plan.surface_m2(c) >= s - 1e-9);
    let longueur = plan.planche.hauteur / PT_PAR_MM;
    let rempli = r.longueur_cible_mm.zip(r.remplissage_pct).is_some_and(|(l, p)| longueur >= l * p / 100.0 - 1e-6);
    delai || surface || rempli || !plan.reportes.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MM: f64 = PT_PAR_MM;

    fn cand(l: f64, h: f64) -> Candidat {
        Candidat { taille: Taille { largeur: l * MM, hauteur: h * MM }, copies: 1, echeance: None, arrivee: 0 }
    }

    fn reglage(strategie: Strategie) -> Reglage {
        Reglage { strategie, espace_mm: 0.0, ..Default::default() }
    }

    /// 5.1 : cinq travaux de formats mélangés sur 1000 mm : l'économie (tri par hauteur et
    /// quart de tour) donne une planche plus courte que le groupé, qui garde l'ordre de la file.
    #[test]
    fn economie_plus_courte_que_groupe_qui_garde_l_ordre() {
        let c = [cand(600.0, 200.0), cand(300.0, 500.0), cand(600.0, 200.0), cand(400.0, 500.0), cand(300.0, 200.0)];
        let groupe = planifier(&c, 1000.0, &reglage(Strategie::Groupe)).unwrap();
        let eco = planifier(&c, 1000.0, &reglage(Strategie::Economie)).unwrap();
        let (lg, le) = (groupe.planche.hauteur / MM, eco.planche.hauteur / MM);
        assert!(le < lg - 1.0, "économie {le} mm, groupé {lg} mm");
        // Groupé : chaque élément est posé après le précédent (même rangée plus à droite, ou
        // rangée suivante), jamais avant.
        let ordre: Vec<usize> = groupe.places.iter().map(|p| groupe.elements[p.indice]).collect();
        assert_eq!(ordre, [0, 1, 2, 3, 4], "placé dans l'ordre de la file");
        for w in groupe.places.windows(2) {
            assert!(w[1].y > w[0].y + 1e-6 || (w[1].y == w[0].y && w[1].x > w[0].x), "{:?} puis {:?}", w[0], w[1]);
        }
        assert!(groupe.places.iter().all(|p| p.angle == 0), "groupé : jamais tourné");
        // Groupé : 0 + 1 (rangée de 500), 2 + 3 (600 + 400 = la laize, 500), 4 (200) : 1200 mm.
        assert!((lg - 1200.0).abs() < 1e-6, "{lg}");
        // Remplissage mesuré : surface des pièces / (laize × longueur).
        let surface = (2.0 * 600.0 * 200.0 + 300.0 * 500.0 + 400.0 * 500.0 + 300.0 * 200.0) / 1e6;
        assert!((groupe.surface_m2(&c) - surface).abs() < 1e-9);
        assert!((groupe.remplissage(&c, 1000.0, 0.0) - surface / 1.2).abs() < 1e-9);
        assert!(eco.remplissage(&c, 1000.0, 0.0) > groupe.remplissage(&c, 1000.0, 0.0));
    }

    /// 5.6 : l'échéance la plus proche passe d'abord, sans échéance ensuite, puis la file.
    #[test]
    fn tri_par_echeance() {
        let mut c = [cand(100.0, 100.0); 4];
        c[1].echeance = Some(500);
        c[2].echeance = Some(100);
        assert_eq!(priorite(&c), [2, 1, 0, 3]);
        let p = planifier(&c, 1000.0, &reglage(Strategie::Groupe)).unwrap();
        let ordre: Vec<usize> = p.places.iter().map(|x| p.elements[x.indice]).collect();
        assert_eq!(ordre, [2, 1, 0, 3], "groupé : posés par priorité");
    }

    /// 5.2 : seuil de 1 m² ; des travaux de 0,3 m² : la planche part au 4e (1,2 m²), pas avant.
    #[test]
    fn depart_a_la_surface_au_travail_qui_franchit_le_seuil() {
        let r = Reglage { surface_m2: Some(1.0), ..reglage(Strategie::Economie) };
        let c: Vec<Candidat> = (0..4).map(|_| cand(600.0, 500.0)).collect();
        for n in 1..=4 {
            let p = planifier(&c[..n], 1600.0, &r).unwrap();
            assert_eq!(pret(&p, &c[..n], &r, 0), n == 4, "{n} travaux : {} m²", p.surface_m2(&c[..n]));
        }
        // Aucun déclencheur : jamais d'elle-même.
        let manuel = reglage(Strategie::Economie);
        assert!(!pret(&planifier(&c, 1600.0, &manuel).unwrap(), &c, &manuel, u64::MAX));
    }

    /// 5.2 : temporisé (minutes depuis le plus ancien) et pourcentage d'une longueur cible ;
    /// au-delà de la cible, les travaux de moindre priorité attendent, la planche pleine part.
    #[test]
    fn depart_temporise_au_remplissage_et_planche_pleine() {
        let mut c: Vec<Candidat> = (0..3).map(|_| cand(1000.0, 400.0)).collect();
        c[0].arrivee = 1000;
        c[1].arrivee = 900;
        c[2].arrivee = 950;
        let r = Reglage { delai_min: Some(10), ..reglage(Strategie::Groupe) };
        let p = planifier(&c, 1000.0, &r).unwrap();
        assert!(!pret(&p, &c, &r, 900 + 599));
        assert!(pret(&p, &c, &r, 900 + 600), "10 min après le plus ancien");
        // Cible 1 m à 80 % : deux travaux (800 mm) suffisent ; le 3e est reporté.
        let r = Reglage { longueur_cible_mm: Some(1000.0), remplissage_pct: Some(80.0), ..reglage(Strategie::Groupe) };
        let p = planifier(&c[..1], 1000.0, &r).unwrap();
        assert!(!pret(&p, &c[..1], &r, 0), "400 mm < 800");
        let p = planifier(&c, 1000.0, &r).unwrap();
        assert_eq!((p.retenus.as_slice(), p.reportes.as_slice()), (&[0usize, 1][..], &[2usize][..]));
        assert!((p.planche.hauteur / MM - 800.0).abs() < 1e-6);
        assert!(pret(&p, &c, &r, 0));
        // Planche pleine sans pourcentage : elle part aussi.
        let r = Reglage { remplissage_pct: None, ..r };
        assert!(pret(&planifier(&c, 1000.0, &r).unwrap(), &c, &r, 0));
        // Un travail seul plus long que la cible part quand même seul.
        let long = [cand(500.0, 3000.0)];
        assert_eq!(planifier(&long, 1000.0, &r).unwrap().retenus, [0]);
    }

    /// Copies (quantité due) déroulées ; un format plus large que la laize est écarté.
    #[test]
    fn copies_et_format_trop_large() {
        let mut c = [cand(300.0, 100.0), cand(2000.0, 100.0)];
        c[0].copies = 3;
        let p = planifier(&c, 1000.0, &reglage(Strategie::Groupe)).unwrap();
        assert_eq!((p.retenus.as_slice(), p.elements.as_slice()), (&[0usize][..], &[0usize, 0, 0][..]));
        assert!(planifier(&c[1..], 1000.0, &reglage(Strategie::Groupe)).is_none());
    }

    #[test]
    fn reglage_d_avant_et_bornes() {
        let r: Reglage = serde_json::from_str("{}").unwrap();
        assert_eq!(r, Reglage::default());
        assert!(r.valide());
        assert!(!Reglage { remplissage_pct: Some(80.0), ..r.clone() }.valide(), "pourcentage sans longueur cible");
        assert!(!Reglage { delai_min: Some(0), ..r }.valide());
    }
}
