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

/// Place du groupe d'éléments sur la laize (roadmap 5.7).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Justification {
    #[default]
    Gauche,
    Centre,
    Droite,
}

/// Bord du rouleau qui porte la bande de purge (roadmap 5.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cote {
    Gauche,
    Droite,
    LesDeux,
}

/// Chargement (roadmap 5.11) : libre, la planche prend la laize du rouleau chargé quel que
/// soit le format des travaux ; strict, une planche par format (laize du support au
/// catalogue, sinon de la mosaïque), posée sur ce format.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Chargement {
    #[default]
    Libre,
    Strict,
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
    /// Départ à l'inactivité : minutes sans nouveau travail dans la planche (roadmap 5.11),
    /// compté depuis le dernier arrivé (le délai, lui, depuis le premier).
    #[serde(default)]
    pub inactivite_min: Option<u32>,
    /// Départ quand la planche atteint cette longueur (roadmap 5.11).
    #[serde(default)]
    pub longueur_max_mm: Option<f64>,
    /// Une rangée pleine sort seule (roadmap 5.11) : le travail qui ouvrirait une deuxième
    /// rangée attend la planche suivante, et la rangée part.
    #[serde(default)]
    pub rangee_pleine: bool,
    #[serde(default)]
    pub chargement: Chargement,
    /// Espace entre copies côte à côte (et entre rangées, sauf `espace_vertical_mm`).
    #[serde(default = "espace_defaut")]
    pub espace_mm: f64,
    /// Espace entre rangées ; rien : celui entre copies (roadmap 5.7).
    #[serde(default)]
    pub espace_vertical_mm: Option<f64>,
    #[serde(default)]
    pub justification: Justification,
    /// Décalage d'origine : blanc à gauche de la planche, avant la purge (roadmap 5.7).
    #[serde(default)]
    pub decalage_x_mm: f64,
    /// Bande de purge (une bande par encre CMJN) le long du rouleau ; rien : sans bande.
    #[serde(default)]
    pub purge_cote: Option<Cote>,
    #[serde(default = "purge_defaut")]
    pub purge_mm: f64,
    /// Les lés pairs d'une mosaïque tournés d'un demi-tour (roadmap 5.7).
    #[serde(default)]
    pub retourner_les: bool,
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

fn purge_defaut() -> f64 {
    10.0
}

impl Default for Reglage {
    fn default() -> Self {
        Reglage {
            strategie: Strategie::UnParUn,
            delai_min: None,
            surface_m2: None,
            longueur_cible_mm: None,
            remplissage_pct: None,
            inactivite_min: None,
            longueur_max_mm: None,
            rangee_pleine: false,
            chargement: Chargement::Libre,
            espace_mm: espace_defaut(),
            espace_vertical_mm: None,
            justification: Justification::Gauche,
            decalage_x_mm: 0.0,
            purge_cote: None,
            purge_mm: purge_defaut(),
            retourner_les: false,
            marge_haut_mm: 0.0,
            marge_bas_mm: 0.0,
            contour: false,
        }
    }
}

impl Reglage {
    pub fn valide(&self) -> bool {
        self.delai_min.is_none_or(|m| (1..=1440).contains(&m))
            && self.surface_m2.is_none_or(|s| (0.01..=1000.0).contains(&s))
            && self.longueur_cible_mm.is_none_or(|l| (100.0..=100_000.0).contains(&l))
            && self.remplissage_pct.is_none_or(|p| (1.0..=100.0).contains(&p))
            && (self.remplissage_pct.is_none() || self.longueur_cible_mm.is_some())
            && self.inactivite_min.is_none_or(|m| (1..=1440).contains(&m))
            && self.longueur_max_mm.is_none_or(|l| (100.0..=100_000.0).contains(&l))
            && (0.0..=100.0).contains(&self.espace_mm)
            && self.espace_vertical_mm.is_none_or(|e| (0.0..=100.0).contains(&e))
            && (0.0..=1000.0).contains(&self.decalage_x_mm)
            && (1.0..=100.0).contains(&self.purge_mm)
            && (0.0..=200.0).contains(&self.marge_haut_mm)
            && (0.0..=200.0).contains(&self.marge_bas_mm)
    }

    /// Habillage de la planche ; les bandes de purge, qui dépendent de la laize, viennent du
    /// plan (`Plan::purges`).
    pub fn finition(&self) -> imbrication::Finition {
        imbrication::Finition { marge_bas: self.marge_bas_mm * PT_PAR_MM, marge_haut: self.marge_haut_mm * PT_PAR_MM, contour: self.contour, ..Default::default() }
    }

    /// Blanc réservé à gauche et à droite des éléments (points) : décalage d'origine, bandes
    /// de purge et leur espace ; et les bandes (x, largeur) sur une laize de `laize` points.
    fn bords(&self, laize: f64) -> (f64, f64, [[f64; 2]; 2]) {
        let (largeur, espace, x) = (self.purge_mm * PT_PAR_MM, self.espace_mm * PT_PAR_MM, self.decalage_x_mm * PT_PAR_MM);
        let (g, d) = match self.purge_cote {
            None => (false, false),
            Some(Cote::Gauche) => (true, false),
            Some(Cote::Droite) => (false, true),
            Some(Cote::LesDeux) => (true, true),
        };
        let bande = |oui: bool, a: f64| if oui { [a, largeur] } else { [0.0, 0.0] };
        let gauche = x + if g { largeur + espace } else { 0.0 };
        let droite = if d { largeur + espace } else { 0.0 };
        (gauche, droite, [bande(g, x), bande(d, laize - largeur)])
    }
}

/// Une impression en attente, dans l'ordre de la file.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Candidat {
    /// Page à poser (points), sans repères : la planche a les siens.
    pub taille: Taille,
    pub copies: usize,
    /// N° de lé (tuile de mosaïque, de 1) : les pairs se retournent avec `retourner_les`.
    pub le: Option<usize>,
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
    /// Candidats laissés pour la planche suivante (longueur cible atteinte, rangée pleine).
    pub reportes: Vec<usize>,
    /// Bandes de purge : x et largeur (points) ; largeur nulle, pas de bande.
    pub purges: [[f64; 2]; 2],
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
    let (gauche, droite, _) = r.bords(laize);
    let utile = laize - gauche - droite;
    if utile <= 0.0 {
        return Err("purge et décalage plus larges que la laize".into());
    }
    let (mut places, mut planche) = match r.strategie {
        Strategie::Economie => imbrication::placer(&tailles, utile, espace, true)?,
        Strategie::Groupe | Strategie::UnParUn => imbrication::placer_en_ordre(&tailles, utile, espace)?,
    };
    // Habillage (roadmap 5.7). Espace entre rangées : chaque rangée (même y) décalée d'autant.
    if let Some(v) = r.espace_vertical_mm {
        let mut ys: Vec<f64> = places.iter().map(|p| p.y).collect();
        ys.sort_by(f64::total_cmp);
        ys.dedup();
        let ecart = v * PT_PAR_MM - espace;
        for p in places.iter_mut() {
            p.y += ecart * ys.iter().position(|y| *y == p.y).unwrap_or(0) as f64;
        }
        planche.hauteur += ecart * ys.len().saturating_sub(1) as f64;
    }
    // Justification du groupe dans la laize utile, après le décalage et la purge de gauche.
    let dx = gauche
        + match r.justification {
            Justification::Gauche => 0.0,
            Justification::Centre => (utile - planche.largeur) / 2.0,
            Justification::Droite => utile - planche.largeur,
        };
    for p in places.iter_mut() {
        p.x += dx;
        // Un lé sur deux d'une mosaïque retourné : même case, demi-tour.
        if r.retourner_les && c[elements[p.indice]].le.is_some_and(|n| n % 2 == 0) {
            p.angle = (p.angle + 180) % 360;
        }
    }
    planche.largeur = if r.justification == Justification::Gauche && droite == 0.0 { dx + planche.largeur } else { laize };
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
        // Rangée pleine (roadmap 5.11) : le travail qui ouvre une deuxième rangée attend.
        // ponytail: en économie, un plus petit venu après aurait peut-être tenu dans la rangée ;
        // il attend aussi (les suivants d'une planche pleine attendent tous).
        let deuxieme_rangee = r.rangee_pleine && places.iter().any(|p| p.y > places[0].y + 1e-6);
        if (trop_long || deuxieme_rangee) && retenus.len() > 1 {
            retenus.pop();
            reportes.push(i);
            continue;
        }
        plan = Some(Plan { retenus: retenus.clone(), elements, places, planche, reportes: vec![], purges: r.bords(laize).2 });
    }
    plan.map(|p| Plan { reportes, ..p })
}

/// Le déclencheur est-il atteint ? Délai depuis le plus ancien, inactivité depuis le plus
/// récent (tous les travaux du groupe, reportés compris), surface accumulée, remplissage de
/// la longueur cible, longueur maxi, ou planche pleine (des travaux reportés, ou une rangée
/// pleine). Sans aucun déclencheur : seulement sur « Imprimer maintenant ».
pub fn pret(plan: &Plan, c: &[Candidat], r: &Reglage, maintenant: u64) -> bool {
    let plus_ancien = plan.retenus.iter().map(|&i| c[i].arrivee).min().unwrap_or(maintenant);
    let plus_recent = c.iter().map(|x| x.arrivee).max().unwrap_or(maintenant);
    let depuis = |t: u64, m: Option<u32>| m.is_some_and(|m| maintenant.saturating_sub(t) >= u64::from(m) * 60);
    let surface = r.surface_m2.is_some_and(|s| plan.surface_m2(c) >= s - 1e-9);
    let longueur = plan.planche.hauteur / PT_PAR_MM;
    let rempli = r.longueur_cible_mm.zip(r.remplissage_pct).is_some_and(|(l, p)| longueur >= l * p / 100.0 - 1e-6);
    let maxi = r.longueur_max_mm.is_some_and(|l| longueur >= l - 1e-6);
    let rangees = r.rangee_pleine && plan.places.iter().any(|p| p.y > plan.places[0].y + 1e-6);
    depuis(plus_ancien, r.delai_min) || depuis(plus_recent, r.inactivite_min) || surface || rempli || maxi || rangees || !plan.reportes.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MM: f64 = PT_PAR_MM;

    fn cand(l: f64, h: f64) -> Candidat {
        Candidat { taille: Taille { largeur: l * MM, hauteur: h * MM }, copies: 1, le: None, echeance: None, arrivee: 0 }
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

    /// 5.7 : purge des deux côtés, décalage d'origine, espaces horizontal et vertical,
    /// justification au centre puis à droite et à gauche ; positions mesurées sur le plan.
    #[test]
    fn purge_justification_decalage_et_espaces() {
        let c = [cand(300.0, 100.0); 3];
        let base = Reglage { espace_mm: 10.0, espace_vertical_mm: Some(30.0), purge_cote: Some(Cote::LesDeux), purge_mm: 20.0, decalage_x_mm: 50.0, justification: Justification::Centre, ..reglage(Strategie::Groupe) };
        // Laize utile : 1000 − (50 + 20 + 10) − (20 + 10) = 890 ; deux par rangée (610 mm).
        let xy = |p: &Plan| p.places.iter().map(|x| ((x.x / MM).round(), (x.y / MM).round())).collect::<Vec<_>>();
        let p = planifier(&c, 1000.0, &base).unwrap();
        assert_eq!(xy(&p), [(220.0, 0.0), (530.0, 0.0), (220.0, 130.0)], "centré : 80 + (890 − 610) / 2");
        assert!((p.planche.hauteur / MM - 230.0).abs() < 1e-6 && (p.planche.largeur / MM - 1000.0).abs() < 1e-6, "{:?}", p.planche);
        let bandes: Vec<[f64; 2]> = p.purges.iter().map(|b| b.map(|v| (v / MM * 1e6).round() / 1e6)).collect();
        assert_eq!(bandes, [[50.0, 20.0], [980.0, 20.0]], "purges : après le décalage, et au bord droit");
        let p = planifier(&c, 1000.0, &Reglage { justification: Justification::Droite, ..base.clone() }).unwrap();
        assert_eq!(xy(&p)[0], (360.0, 0.0), "à droite : 80 + 280");
        // À gauche sans purge à droite : la planche s'arrête au dernier élément.
        let g = Reglage { justification: Justification::Gauche, purge_cote: Some(Cote::Gauche), espace_vertical_mm: None, ..base.clone() };
        let p = planifier(&c, 1000.0, &g).unwrap();
        assert_eq!(xy(&p), [(80.0, 0.0), (390.0, 0.0), (700.0, 0.0)], "laize utile 920 : trois par rangée");
        assert!((p.planche.largeur / MM - 1000.0).abs() < 1e-6 && (p.planche.hauteur / MM - 100.0).abs() < 1e-6);
        assert_eq!(p.purges[1], [0.0, 0.0]);
        // Sans réglage 5.7 : la planche d'avant, collée à gauche, de la largeur des éléments.
        let p = planifier(&c, 1000.0, &Reglage { espace_mm: 10.0, ..reglage(Strategie::Groupe) }).unwrap();
        assert_eq!((xy(&p)[0], (p.planche.largeur / MM).round()), ((0.0, 0.0), 920.0));
        assert_eq!(p.purges, [[0.0; 2]; 2]);
        // Purge et décalage plus larges que la laize : rien à poser.
        assert!(planifier(&c, 1000.0, &Reglage { decalage_x_mm: 1000.0, ..base }).is_none());
    }

    /// 5.7 : un lé sur deux d'une mosaïque retourné (demi-tour, même case) ; les autres non.
    #[test]
    fn retournement_d_un_le_sur_deux() {
        let c: Vec<Candidat> = (1..=4).map(|n| Candidat { le: Some(n), ..cand(500.0, 1000.0) }).chain([cand(100.0, 100.0)]).collect();
        let r = Reglage { retourner_les: true, ..reglage(Strategie::Groupe) };
        let p = planifier(&c, 2000.0, &r).unwrap();
        let angles: Vec<u16> = p.places.iter().map(|x| x.angle).collect();
        assert_eq!(angles, [0, 180, 0, 180, 0], "lés 2 et 4 retournés, le travail hors mosaïque non");
        let droit = planifier(&c, 2000.0, &reglage(Strategie::Groupe)).unwrap();
        assert!(droit.places.iter().all(|x| x.angle == 0));
        assert_eq!(p.places.iter().map(|x| (x.x, x.y)).collect::<Vec<_>>(), droit.places.iter().map(|x| (x.x, x.y)).collect::<Vec<_>>(), "mêmes cases");
    }

    /// 5.11 : inactivité comptée depuis le dernier arrivé (le délai, depuis le premier).
    #[test]
    fn depart_a_l_inactivite_apres_le_dernier() {
        let mut c: Vec<Candidat> = (0..3).map(|_| cand(100.0, 100.0)).collect();
        (c[0].arrivee, c[1].arrivee, c[2].arrivee) = (1000, 1030, 1050);
        let r = Reglage { inactivite_min: Some(1), ..reglage(Strategie::Groupe) };
        let p = planifier(&c, 1000.0, &r).unwrap();
        assert!(!pret(&p, &c, &r, 1050 + 59), "59 s après le dernier");
        assert!(pret(&p, &c, &r, 1050 + 60), "1 min après le dernier");
        let delai = Reglage { delai_min: Some(1), ..reglage(Strategie::Groupe) };
        assert!(pret(&p, &c, &delai, 1060), "le délai, lui, part 1 min après le premier");
        // Tout travail du groupe compte comme reçu, même hors de la planche calculée.
        let p = planifier(&c[..2], 1000.0, &r).unwrap();
        assert!(!pret(&p, &c, &r, 1050 + 59));
    }

    /// 5.11 : longueur maxi de 2 m : la planche part au travail qui la franchit.
    #[test]
    fn depart_a_la_longueur_maxi() {
        let c: Vec<Candidat> = (0..3).map(|_| cand(1000.0, 800.0)).collect();
        let r = Reglage { longueur_max_mm: Some(2000.0), ..reglage(Strategie::Groupe) };
        for n in 1..=3 {
            let p = planifier(&c[..n], 1000.0, &r).unwrap();
            assert_eq!(pret(&p, &c[..n], &r, 0), n == 3, "{n} travaux : {} mm", p.planche.hauteur / MM);
            assert_eq!(p.retenus.len(), n, "tous posés : la longueur maxi ne reporte rien");
        }
    }

    /// 5.11 : rangée pleine sortie seule : le travail qui ouvrirait la 2e rangée attend.
    #[test]
    fn rangee_pleine_sortie_seule() {
        let c: Vec<Candidat> = (0..3).map(|_| cand(400.0, 100.0)).collect();
        let r = Reglage { rangee_pleine: true, ..reglage(Strategie::Groupe) };
        let p = planifier(&c[..2], 1000.0, &r).unwrap();
        assert!(!pret(&p, &c[..2], &r, 0), "800 mm sur 1000 : la rangée attend");
        let p = planifier(&c, 1000.0, &r).unwrap();
        assert_eq!((p.retenus.as_slice(), p.reportes.as_slice()), (&[0usize, 1][..], &[2usize][..]));
        assert!(p.places.iter().all(|x| x.y == 0.0) && pret(&p, &c, &r, 0), "une rangée, qui part");
        // Un seul travail dont les copies font plusieurs rangées : il part, entier.
        let mut seul = [cand(400.0, 100.0)];
        seul[0].copies = 3;
        let p = planifier(&seul, 1000.0, &r).unwrap();
        assert_eq!(p.elements.len(), 3);
        assert!(pret(&p, &seul, &r, 0));
        assert!(!pret(&p, &seul, &reglage(Strategie::Groupe), 0));
    }

    #[test]
    fn reglage_d_avant_et_bornes() {
        let r: Reglage = serde_json::from_str("{}").unwrap();
        assert_eq!(r, Reglage::default());
        assert!(r.valide());
        assert!(!Reglage { remplissage_pct: Some(80.0), ..r.clone() }.valide(), "pourcentage sans longueur cible");
        assert!(!Reglage { delai_min: Some(0), ..r.clone() }.valide());
        assert!(!Reglage { inactivite_min: Some(0), ..r.clone() }.valide());
        assert!(!Reglage { purge_mm: 0.0, ..r.clone() }.valide());
        assert!(!Reglage { longueur_max_mm: Some(10.0), ..r }.valide());
    }
}
