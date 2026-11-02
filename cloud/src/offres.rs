//! Offres de la plate-forme (prix HT, utilisateurs illimités) et règles qui en découlent :
//! sites et machines compris, sièges facturés, conservation et quota des sauvegardes.
//!
//! - Gratuit, pour toujours : 1 site, 1 imprimante + 1 découpeur, dernière sauvegarde seulement, 1 Go.
//! - À la machine : 6 €/mois par machine au-delà des machines gratuites, 1 site, 30 versions, 5 Go par machine.
//! - Atelier : 19 €/mois, 1 site, 5 machines, 90 jours d'historique, 25 Go ; machine en plus 4 €/mois.
//! - Pro : 49 €/mois, 3 sites, 15 machines, 1 an d'historique, 100 Go, restauration vers un autre site ;
//!   machine en plus 4 €/mois.
//! - Entreprise : sur devis.
//!
//! L'annuel vaut 10 mois payés pour 12. Fonctions pures : testées ici.

use rusqlite::Connection;

pub const GO: u64 = 1024 * 1024 * 1024;
pub const JOUR: u64 = 86_400;

/// Délai de grâce d'un dépassement d'offre avant la suspension de l'accès à distance.
pub const GRACE_DEPASSEMENT: u64 = 14 * JOUR;
/// Comptes existants au premier déploiement des offres : 30 jours avant toute suspension.
pub const GRACE_DEPLOIEMENT: u64 = 30 * JOUR;
/// Après un retour au Gratuit, les sauvegardes payantes restent 90 jours.
pub const CONSERVATION_APRES_GRATUIT: u64 = 90 * JOUR;
/// Machine au-delà du forfait (Atelier, Pro), HT par mois, en centimes.
pub const PRIX_SUPPLEMENT: i64 = 400;
/// Option stockage +50 Go, HT par mois, en centimes (pas encore branchée à l'ERP).
pub const PRIX_STOCKAGE: i64 = 200;
pub const CONTACT: &str = "contact@e-ko.rip";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Offre {
    Gratuit,
    Machine,
    Atelier,
    Pro,
    Entreprise,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cycle {
    Mensuel,
    Annuel,
}

impl Cycle {
    pub fn code(self) -> &'static str {
        match self {
            Cycle::Mensuel => "mensuel",
            Cycle::Annuel => "annuel",
        }
    }
    pub fn depuis(s: &str) -> Option<Cycle> {
        match s {
            "mensuel" => Some(Cycle::Mensuel),
            "annuel" => Some(Cycle::Annuel),
            _ => None,
        }
    }
    pub fn libelle(self) -> &'static str {
        match self {
            Cycle::Mensuel => "mensuel",
            Cycle::Annuel => "annuel",
        }
    }
}

/// Conservation des sauvegardes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Conservation {
    DerniereSeulement,
    Versions(usize),
    Jours(u64),
}

pub const PAYANTES: [Offre; 3] = [Offre::Machine, Offre::Atelier, Offre::Pro];
pub const TOUTES: [Offre; 5] = [Offre::Gratuit, Offre::Machine, Offre::Atelier, Offre::Pro, Offre::Entreprise];

impl Offre {
    pub fn code(self) -> &'static str {
        match self {
            Offre::Gratuit => "gratuit",
            Offre::Machine => "machine",
            Offre::Atelier => "atelier",
            Offre::Pro => "pro",
            Offre::Entreprise => "entreprise",
        }
    }

    pub fn depuis(s: &str) -> Option<Offre> {
        TOUTES.into_iter().find(|o| o.code() == s)
    }

    pub fn nom(self) -> &'static str {
        match self {
            Offre::Gratuit => "Gratuit",
            Offre::Machine => "À la machine",
            Offre::Atelier => "Atelier",
            Offre::Pro => "Pro",
            Offre::Entreprise => "Entreprise",
        }
    }

    pub fn payante(self) -> bool {
        PAYANTES.contains(&self)
    }

    /// Sites compris dans l'offre.
    pub fn sites(self) -> usize {
        match self {
            Offre::Pro => 3,
            Offre::Entreprise => 50,
            _ => 1,
        }
    }

    /// Machines comprises au forfait (`None` : facturées une à une ou illimitées).
    pub fn machines_forfait(self) -> Option<usize> {
        match self {
            Offre::Atelier => Some(5),
            Offre::Pro => Some(15),
            _ => None,
        }
    }

    /// Prix mensuel HT en centimes (par machine pour « À la machine »).
    pub fn prix_mensuel(self) -> i64 {
        match self {
            Offre::Machine => 600,
            Offre::Atelier => 1900,
            Offre::Pro => 4900,
            _ => 0,
        }
    }

    /// Prix HT d'une période en centimes : l'annuel vaut 10 mois.
    pub fn prix(self, cycle: Cycle) -> i64 {
        match cycle {
            Cycle::Mensuel => self.prix_mensuel(),
            Cycle::Annuel => self.prix_mensuel() * 10,
        }
    }

    pub fn conservation(self) -> Conservation {
        match self {
            Offre::Gratuit => Conservation::DerniereSeulement,
            Offre::Machine => Conservation::Versions(30),
            Offre::Atelier => Conservation::Jours(90),
            Offre::Pro | Offre::Entreprise => Conservation::Jours(365),
        }
    }

    pub fn libelle_conservation(self) -> &'static str {
        match self.conservation() {
            Conservation::DerniereSeulement => "dernière version",
            Conservation::Versions(_) => "30 versions",
            Conservation::Jours(90) => "90 jours d'historique",
            Conservation::Jours(_) => "1 an d'historique",
        }
    }

    /// Quota de sauvegarde en octets, pour `machines` machines inventoriées.
    pub fn quota(self, machines: usize) -> u64 {
        match self {
            Offre::Gratuit => GO,
            Offre::Machine => 5 * GO * machines.max(1) as u64,
            Offre::Atelier => 25 * GO,
            Offre::Pro => 100 * GO,
            Offre::Entreprise => 1024 * GO,
        }
    }

    /// Restauration vers un autre site du même propriétaire.
    pub fn restauration_croisee(self) -> bool {
        matches!(self, Offre::Pro | Offre::Entreprise)
    }

    /// Une phrase d'argumentaire (tableau des offres).
    pub fn description(self) -> &'static str {
        match self {
            Offre::Gratuit => "1 site, 1 imprimante et 1 découpeur, dernière sauvegarde, 1 Go.",
            Offre::Machine => "1 site, chaque machine au-delà des machines gratuites, 30 versions de sauvegarde, 5 Go par machine.",
            Offre::Atelier => "1 site, jusqu'à 5 machines, 90 jours d'historique, 25 Go. Machine en plus : 4 € HT/mois.",
            Offre::Pro => "Jusqu'à 3 sites, 15 machines, 1 an d'historique, 100 Go, restauration vers un autre site. Machine en plus : 4 € HT/mois.",
            Offre::Entreprise => "Sites, machines et stockage sur mesure, accompagnement dédié.",
        }
    }
}

/// Clé de `reglages` du code de plan de l'ERP pour une offre et un cycle
/// (`plan_atelier_mensuel`…), ou du supplément machine (`plan_supplement_annuel`).
pub fn cle_plan(offre: Option<Offre>, cycle: Cycle) -> String {
    format!("plan_{}_{}", offre.map_or("supplement", Offre::code), cycle.code())
}

pub fn code_plan(c: &Connection, offre: Option<Offre>, cycle: Cycle) -> Option<String> {
    crate::base::reglage(c, &cle_plan(offre, cycle)).ok().flatten()
}

/// Offre et cycle d'un code de plan de l'ERP (d'après les réglages).
pub fn offre_du_plan(c: &Connection, plan: &str) -> Option<(Offre, Cycle)> {
    for o in PAYANTES {
        for cy in [Cycle::Mensuel, Cycle::Annuel] {
            if code_plan(c, Some(o), cy).as_deref() == Some(plan) {
                return Some((o, cy));
            }
        }
    }
    None
}

/// Machines d'un compte (toutes ses usines confondues).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Parc {
    pub sites: usize,
    pub imprimantes: usize,
    pub decoupeurs: usize,
}

impl Parc {
    pub fn machines(&self) -> usize {
        self.imprimantes + self.decoupeurs
    }
    /// Machines au-delà des machines gratuites (1 imprimante + 1 découpeur).
    pub fn au_dela_du_gratuit(&self) -> usize {
        self.imprimantes.saturating_sub(1) + self.decoupeurs.saturating_sub(1)
    }
}

/// Sièges facturés : (sièges du plan principal s'il est par siège, machines en supplément).
pub fn sieges(offre: Offre, parc: &Parc) -> (Option<i64>, i64) {
    match offre {
        Offre::Machine => (Some(parc.au_dela_du_gratuit().max(1) as i64), 0),
        Offre::Atelier | Offre::Pro => (None, parc.machines().saturating_sub(offre.machines_forfait().unwrap_or(0)) as i64),
        _ => (None, 0),
    }
}

/// Raison d'un dépassement d'offre, s'il y en a un. `supplement` : un plan « machines
/// supplémentaires » est réglé (sinon, les machines au-delà du forfait comptent en dépassement).
pub fn depassement(offre: Offre, parc: &Parc, supplement: bool) -> Option<String> {
    if parc.sites > offre.sites() {
        return Some(format!("{} sites rattachés pour {} compris dans l'offre {}", parc.sites, offre.sites(), offre.nom()));
    }
    match offre {
        Offre::Gratuit if parc.imprimantes > 1 || parc.decoupeurs > 1 => Some(format!("{} imprimante(s) et {} découpeur(s) pour 1 imprimante et 1 découpeur compris dans l'offre Gratuit", parc.imprimantes, parc.decoupeurs)),
        Offre::Atelier | Offre::Pro if !supplement && parc.machines() > offre.machines_forfait().unwrap_or(usize::MAX) => Some(format!("{} machines pour {} comprises dans l'offre {}", parc.machines(), offre.machines_forfait().unwrap_or(0), offre.nom())),
        _ => None,
    }
}

/// Date de suspension d'un dépassement commencé à `depuis` (grâce de 14 jours, et jamais
/// avant 30 jours après le premier déploiement des offres).
pub fn date_suspension(depuis: u64, deploye_le: u64) -> u64 {
    (depuis + GRACE_DEPASSEMENT).max(deploye_le + GRACE_DEPLOIEMENT)
}

/// Versions à supprimer d'un site : `versions` = (identifiant, date), à date égale de la plus récente à la plus ancienne.
/// La plus récente est toujours gardée. `gratuit_depuis` : date du retour au Gratuit (les
/// versions d'avant restent 90 jours).
pub fn a_supprimer(versions: &[(String, u64)], offre: Offre, gratuit_depuis: Option<u64>, t: u64) -> Vec<String> {
    let mut v: Vec<&(String, u64)> = versions.iter().collect();
    // Tri stable : à date égale, l'ordre reçu (de la plus récente à la plus ancienne) est gardé.
    v.sort_by_key(|x| std::cmp::Reverse(x.1));
    let mut sortie = Vec::new();
    for (i, (id, date)) in v.iter().enumerate() {
        if i == 0 {
            continue;
        }
        let garder = match offre.conservation() {
            Conservation::DerniereSeulement => gratuit_depuis.is_some_and(|g| *date < g && t < g + CONSERVATION_APRES_GRATUIT),
            Conservation::Versions(n) => i < n,
            Conservation::Jours(j) => t.saturating_sub(*date) <= j * JOUR,
        };
        if !garder {
            sortie.push(id.clone());
        }
    }
    sortie
}

/// « 19,00 € » à partir de centimes.
pub fn euros(centimes: i64) -> String {
    let signe = if centimes < 0 { "-" } else { "" };
    let c = centimes.abs();
    format!("{signe}{},{:02} €", c / 100, c % 100)
}

/// Montant de l'ERP (nombre ou texte décimal) en centimes.
pub fn centimes(v: &serde_json::Value) -> Option<i64> {
    match v {
        serde_json::Value::Number(n) => n.as_f64().map(|f| (f * 100.0).round() as i64),
        serde_json::Value::String(s) => s.trim().replace(',', ".").parse::<f64>().ok().map(|f| (f * 100.0).round() as i64),
        _ => None,
    }
}

/// « 2,5 Go », « 340 Mo ».
pub fn taille(octets: u64) -> String {
    if octets >= GO {
        let dixiemes = octets * 10 / GO;
        if dixiemes.is_multiple_of(10) { format!("{} Go", dixiemes / 10) } else { format!("{},{} Go", dixiemes / 10, dixiemes % 10) }
    } else if octets >= 1024 * 1024 {
        format!("{} Mo", octets / (1024 * 1024))
    } else {
        format!("{} ko", octets.div_ceil(1024))
    }
}

/// Date ISO de l'ERP (`2026-11-30`, `2026-11-30T12:00:00Z`, `2026-11-30 12:00:00`) en secondes UTC.
pub fn date_iso(s: &str) -> Option<u64> {
    let s = s.trim();
    let (a, m, j) = (s.get(0..4)?.parse::<i64>().ok()?, s.get(5..7)?.parse::<u32>().ok()?, s.get(8..10)?.parse::<u32>().ok()?);
    if s.as_bytes().get(4) != Some(&b'-') || !(1..=12).contains(&m) || !(1..=31).contains(&j) {
        return None;
    }
    let mut t = crate::temps::jours_depuis_civil(a, m, j) * 86_400;
    if let (Some(h), Some(mi)) = (s.get(11..13).and_then(|h| h.parse::<i64>().ok()), s.get(14..16).and_then(|h| h.parse::<i64>().ok())) {
        t += h * 3600 + mi * 60 + s.get(17..19).and_then(|x| x.parse::<i64>().ok()).unwrap_or(0);
    }
    u64::try_from(t).ok()
}

/// Libellé français d'un statut d'abonnement de l'ERP.
pub fn libelle_statut(statut: &str) -> &'static str {
    match statut {
        "trial" => "Essai en cours",
        "active" => "Actif",
        "past_due" => "Paiement en retard",
        "paused" => "En pause",
        "cancelled" => "Résilié",
        "" => "—",
        _ => "Inconnu",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prix_et_limites() {
        assert_eq!(Offre::Atelier.prix(Cycle::Annuel), 19_000, "10 mois payés pour 12");
        assert_eq!(euros(Offre::Pro.prix(Cycle::Mensuel)), "49,00 €");
        assert_eq!(euros(-5), "-0,05 €");
        assert_eq!(Offre::depuis("pro"), Some(Offre::Pro));
        assert_eq!(cle_plan(None, Cycle::Annuel), "plan_supplement_annuel");
        assert_eq!(Offre::Machine.quota(3), 15 * GO);
        assert_eq!(taille(GO * 5 / 2), "2,5 Go");
        assert_eq!(centimes(&serde_json::json!("23.40")), Some(2340));
        assert_eq!(centimes(&serde_json::json!(19.9)), Some(1990));
        assert_eq!(date_iso("1970-01-02"), Some(86_400));
        assert_eq!(date_iso("1970-01-01T01:00:30Z"), Some(3630));
        assert_eq!(date_iso("pas une date"), None);
    }

    #[test]
    fn sieges_et_depassements() {
        let p = Parc { sites: 1, imprimantes: 3, decoupeurs: 2 };
        assert_eq!(sieges(Offre::Machine, &p), (Some(3), 0));
        assert_eq!(sieges(Offre::Machine, &Parc { sites: 1, imprimantes: 1, decoupeurs: 0 }), (Some(1), 0), "au moins un siège");
        assert_eq!(sieges(Offre::Atelier, &Parc { sites: 1, imprimantes: 5, decoupeurs: 2 }), (None, 2));
        assert_eq!(sieges(Offre::Pro, &p), (None, 0));
        assert!(depassement(Offre::Gratuit, &p, true).is_some());
        assert!(depassement(Offre::Gratuit, &Parc { sites: 1, imprimantes: 1, decoupeurs: 1 }, false).is_none());
        assert!(depassement(Offre::Gratuit, &Parc { sites: 2, imprimantes: 0, decoupeurs: 0 }, false).is_some());
        assert!(depassement(Offre::Machine, &Parc { sites: 1, imprimantes: 30, decoupeurs: 0 }, false).is_none());
        assert!(depassement(Offre::Atelier, &Parc { sites: 1, imprimantes: 6, decoupeurs: 0 }, false).is_some(), "sans plan supplément");
        assert!(depassement(Offre::Atelier, &Parc { sites: 1, imprimantes: 6, decoupeurs: 0 }, true).is_none());
        assert!(depassement(Offre::Pro, &Parc { sites: 3, imprimantes: 1, decoupeurs: 0 }, true).is_none());
        assert_eq!(date_suspension(100 * JOUR, 0), 114 * JOUR);
        assert_eq!(date_suspension(100 * JOUR, 95 * JOUR), 125 * JOUR, "grâce du déploiement");
    }

    #[test]
    fn conservation_des_versions() {
        let t = 1000 * JOUR;
        let v: Vec<(String, u64)> = (0..40).map(|i| (format!("v{i:02}"), t - i * 10 * JOUR)).collect();
        assert_eq!(a_supprimer(&v, Offre::Gratuit, None, t).len(), 39, "dernière seulement");
        assert_eq!(a_supprimer(&v, Offre::Machine, None, t).len(), 10, "30 versions");
        assert_eq!(a_supprimer(&v, Offre::Atelier, None, t).len(), 40 - 10, "90 jours");
        assert_eq!(a_supprimer(&v, Offre::Pro, None, t).len(), 40 - 37, "1 an");
        assert!(!a_supprimer(&v, Offre::Gratuit, None, t).contains(&"v00".to_string()), "la plus récente reste");
        // Retour au Gratuit il y a 30 jours : les versions payantes restent ; après 90 jours, non.
        let g = t - 30 * JOUR;
        let restent = a_supprimer(&v, Offre::Gratuit, Some(g), t);
        assert_eq!(restent, vec!["v01".to_string(), "v02".to_string(), "v03".to_string()], "versions faites au Gratuit (sauf la dernière) supprimées");
        assert_eq!(a_supprimer(&v, Offre::Gratuit, Some(g), g + CONSERVATION_APRES_GRATUIT).len(), 39);
    }
}
