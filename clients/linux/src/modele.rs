//! Formes JSON de l'API du serveur (`serveur/src/main.rs`, `file.rs`, `machines.rs`).
//! Tout est facultatif ou par défaut : le serveur ajoute des champs à chaque version.

use serde::Deserialize;
use std::collections::BTreeMap;

/// `GET /api/session`.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct Session {
    #[serde(default)]
    pub acces_libre: bool,
    #[serde(default)]
    pub utilisateur: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    /// Présent quand la session vient du cloud.
    #[serde(default)]
    pub cloud: Option<SessionCloud>,
}

/// `cloud` de `GET /api/session` : le compte de la plate-forme et ses droits.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct SessionCloud {
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub nom: String,
    #[serde(default)]
    pub lecture_seule: bool,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct Fiche {
    #[serde(default)]
    pub commande: String,
    #[serde(default)]
    pub client: String,
    #[serde(default)]
    pub societe: String,
    #[serde(default)]
    pub quantite: u32,
    #[serde(default)]
    pub echeance: Option<u64>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub operateur: String,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct Page {
    #[serde(default)]
    pub plaques: Vec<String>,
    #[serde(default)]
    pub plaques_decoupe: Vec<String>,
    #[serde(default)]
    pub traces_decoupe: usize,
    #[serde(default)]
    pub surface_m2: f64,
    #[serde(default)]
    pub largeur_mm: f64,
    #[serde(default)]
    pub hauteur_mm: f64,
    #[serde(default)]
    pub couverture: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct Alerte {
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub page: Option<usize>,
    #[serde(default)]
    pub detail: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct LienErp {
    #[serde(default)]
    pub ref_erp: String,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StatutTravail {
    EnAttente,
    EnCours,
    Termine,
    Echec,
}

impl StatutTravail {
    pub fn libelle(self) -> &'static str {
        match self {
            StatutTravail::EnAttente => "En attente",
            StatutTravail::EnCours => "En cours",
            StatutTravail::Termine => "Terminé",
            StatutTravail::Echec => "Échec",
        }
    }
}

/// Un élément de `GET /api/travaux`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Travail {
    pub id: String,
    #[serde(default)]
    pub nom: String,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    pub statut: StatutTravail,
    #[serde(default)]
    pub code_erreur: Option<String>,
    #[serde(default)]
    pub erreur: Option<String>,
    #[serde(default)]
    pub pages: Vec<Page>,
    #[serde(default)]
    pub alertes: Vec<Alerte>,
    #[serde(default)]
    pub cree_le: u64,
    #[serde(default)]
    pub fiche: Fiche,
    #[serde(default)]
    pub pages_exclues: Vec<usize>,
    #[serde(default)]
    pub erp: Option<LienErp>,
    #[serde(default)]
    pub prix_de_revient: Option<f64>,
    #[serde(default)]
    pub duree_impression_min: Option<f64>,
}

impl Travail {
    /// Dossier affiché : celui de l'ERP s'il existe, sinon la commande de la fiche.
    pub fn dossier(&self) -> &str {
        match &self.erp {
            Some(e) if !e.ref_erp.is_empty() => &e.ref_erp,
            _ => &self.fiche.commande,
        }
    }

    /// Statut en clair, avec le motif d'un échec.
    pub fn statut_libelle(&self) -> String {
        match (self.statut, self.code_erreur.as_deref()) {
            (StatutTravail::Echec, Some(code)) => format!("Échec : {}", crate::libelles::libelle(code).unwrap_or(code)),
            (s, _) => s.libelle().to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EtatSortie {
    EnAttente,
    Suspendue,
    EnCours,
    Terminee,
    Echec,
    Annulee,
    Imbriquee,
}

impl EtatSortie {
    pub fn libelle(self) -> &'static str {
        match self {
            EtatSortie::EnAttente => "En attente",
            EtatSortie::Suspendue => "Suspendue",
            EtatSortie::EnCours => "En cours",
            EtatSortie::Terminee => "Terminée",
            EtatSortie::Echec => "Échec",
            EtatSortie::Annulee => "Annulée",
            EtatSortie::Imbriquee => "Imbriquée",
        }
    }

    /// En attente, suspendue ou en cours.
    pub fn active(self) -> bool {
        matches!(self, EtatSortie::EnAttente | EtatSortie::Suspendue | EtatSortie::EnCours)
    }
}

/// Un élément de `GET /api/file`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Sortie {
    pub id: String,
    #[serde(default)]
    pub machine: String,
    #[serde(default)]
    pub operation: Option<String>,
    pub travail: String,
    #[serde(default)]
    pub nom: String,
    #[serde(default)]
    pub page: usize,
    pub etat: EtatSortie,
    #[serde(default)]
    pub code_erreur: Option<String>,
    #[serde(default)]
    pub erreur: Option<String>,
    #[serde(default)]
    pub cree_le: u64,
    #[serde(default)]
    pub archivee: bool,
    #[serde(default)]
    pub raison_attente: Option<String>,
    #[serde(default)]
    pub couche: Option<String>,
    #[serde(default)]
    pub duree_min: Option<f64>,
}

impl Sortie {
    pub fn etat_libelle(&self) -> String {
        match (self.etat, self.code_erreur.as_deref()) {
            (EtatSortie::Echec, Some(code)) => format!("Échec : {}", crate::libelles::libelle(code).unwrap_or(code)),
            (e, _) => e.libelle().to_string(),
        }
    }
}

/// Un élément de `GET /api/machines` (le catalogue complet est bien plus large).
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Machine {
    pub id: String,
    #[serde(default)]
    pub nom: String,
    #[serde(default)]
    pub modele: String,
    #[serde(default)]
    pub laize_mm: f64,
    #[serde(default)]
    pub pool: Option<String>,
    #[serde(default)]
    pub variante_de: Option<String>,
}

impl Machine {
    pub fn imprime(&self) -> bool {
        matches!(self.modele.as_str(), "epson_sc_s80600" | "roland_versacamm")
    }

    pub fn decoupe(&self) -> bool {
        matches!(self.modele.as_str(), "mutoh_kona" | "graphtec_fc7000" | "roland_versacamm")
    }

    pub fn modele_libelle(&self) -> &str {
        match self.modele.as_str() {
            "epson_sc_s80600" => "Epson SC-S80600",
            "mutoh_kona" => "Mutoh Kona",
            "graphtec_fc7000" => "Graphtec FC7000",
            "roland_versacamm" => "Roland VersaCAMM VP",
            autre => autre,
        }
    }

    /// « impression », « découpe » ou les deux.
    pub fn operations(&self) -> String {
        match (self.imprime(), self.decoupe()) {
            (true, true) => "Impression et découpe".into(),
            (true, false) => "Impression".into(),
            (false, true) => "Découpe".into(),
            _ => "Opérations inconnues".into(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct Rendus {
    #[serde(default)]
    pub simultanes: usize,
    #[serde(default)]
    pub en_cours: usize,
    #[serde(default)]
    pub en_attente: usize,
}

/// Fichier d'état illisible ou dossier recréé (`etat::Alerte` du serveur).
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct AlerteDossier {
    #[serde(default)]
    pub fichier: String,
    #[serde(default)]
    pub reprise: bool,
    #[serde(default)]
    pub dossier: bool,
}

impl AlerteDossier {
    pub fn libelle(&self) -> String {
        let (f, r) = (&self.fichier, self.reprise);
        match (self.dossier, r) {
            (true, true) => format!("dossier {f} absent : recréé"),
            (true, false) => format!("dossier {f} absent et impossible à recréer (droits ?)"),
            (false, true) => format!("{f} illisible (coupure ?) : version précédente reprise"),
            (false, false) => format!("{f} illisible, sans version précédente : à restaurer"),
        }
    }
}

/// `GET /api/systeme`.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct Systeme {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub tampon: u64,
    #[serde(default)]
    pub rendus: Rendus,
    #[serde(default)]
    pub disque_total: Option<u64>,
    #[serde(default)]
    pub disque_libre: Option<u64>,
    #[serde(default)]
    pub alertes: Vec<AlerteDossier>,
}

/// `GET /api/reglages` : seul le nom nous sert (le serveur applique le réglage).
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Reglage {
    pub nom: String,
}

/// Octets en clair (Ko, Mo, Go…).
pub fn taille(octets: u64) -> String {
    const U: [&str; 5] = ["o", "Ko", "Mo", "Go", "To"];
    let (mut v, mut i) = (octets as f64, 0);
    while v >= 1024.0 && i < U.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 { format!("{octets} o") } else { format!("{v:.1} {}", U[i]).replace('.', ",") }
}

/// Date locale (« 30/10/2026 14:05 ») d'un instant en secondes Unix.
pub fn date(secondes: u64) -> String {
    gtk::glib::DateTime::from_unix_local(secondes as i64)
        .ok()
        .and_then(|d| d.format("%d/%m/%Y %H:%M").ok())
        .map_or_else(|| "—".into(), |s| s.to_string())
}

#[cfg(test)]
mod tests;
