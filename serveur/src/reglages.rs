//! Réglages d'un travail (`Demande`) et réglages enregistrés sous un nom, avec leur
//! dossier surveillé `<données>/entrees/<nom>/`.

use crate::{blanc, couleur, edition, epson, reperage};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Imprimer (rendu en plaques, plus la découpe s'il y a des tracés) ou découper seulement.
#[derive(Clone, Copy, Default, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Imprimer,
    Decouper,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Mosaique {
    pub laize_mm: f64,
    pub recouvrement_mm: f64,
    /// Bande de pied avec le numéro du lé et les traits d'assemblage.
    #[serde(default)]
    pub reperes: bool,
}

/// Tout ce qui décide du traitement d'un fichier reçu.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Demande {
    #[serde(default)]
    pub mode: Mode,
    #[serde(default)]
    pub tous_les_traces: bool,
    #[serde(default)]
    pub reperage: reperage::Reperage,
    #[serde(default)]
    pub couleur: couleur::Couleur,
    #[serde(default = "dpi_defaut")]
    pub dpi: u32,
    /// Couper en lés si la page dépasse la laize.
    #[serde(default)]
    pub mosaique: Option<Mosaique>,
    /// Blanc généré après le rendu (sous-couche ou aplat).
    #[serde(default)]
    pub blanc: blanc::Blanc,
    /// Blanc ou argent sous la couleur, ou dessus (vitrophanie).
    #[serde(default)]
    pub couches: epson::Couches,
    /// Édition rapide appliquée au PDF avant le rendu.
    #[serde(default)]
    pub edition: edition::Edition,
    /// Mise en file automatique une fois le rendu fini.
    #[serde(default)]
    pub auto: Auto,
    /// Support du catalogue des tarifs (prix de revient).
    #[serde(default)]
    pub support: Option<String>,
    /// Réglage de coupe de la découpeuse (nom), sinon son premier.
    #[serde(default)]
    pub parametre_decoupe: Option<String>,
}

/// Sorties ajoutées d'office à la file quand le rendu est fini.
#[derive(Clone, Copy, Default, PartialEq, Debug, Serialize, Deserialize)]
pub struct Auto {
    /// Chaque page imprimable vers l'Epson.
    #[serde(default)]
    pub imprimer: bool,
    /// Chaque page qui a des tracés vers le Kona.
    #[serde(default)]
    pub decouper: bool,
    /// Ajoutées suspendues, à libérer depuis la file.
    #[serde(default)]
    pub suspendre: bool,
}

pub fn dpi_defaut() -> u32 {
    720
}

impl Default for Demande {
    fn default() -> Self {
        Demande { mode: Mode::Imprimer, tous_les_traces: false, reperage: reperage::Reperage::Aucun, couleur: couleur::Couleur::default(), dpi: dpi_defaut(), mosaique: None, blanc: blanc::Blanc::default(), couches: epson::Couches::default(), edition: edition::Edition::default(), auto: Auto::default(), support: None, parametre_decoupe: None }
    }
}

impl Demande {
    /// Contrôle les bornes ; rend le code d'erreur traduit par l'écran.
    pub fn verifier(&self, profils: &[couleur::Profil]) -> Result<(), &'static str> {
        if !(36..=2880).contains(&self.dpi) {
            return Err("dpi_invalide");
        }
        if let Some(m) = self.mosaique
            && (!(50.0..=5000.0).contains(&m.laize_mm) || !(0.0..=200.0).contains(&m.recouvrement_mm) || m.recouvrement_mm >= m.laize_mm)
        {
            return Err("mosaique_invalide");
        }
        self.blanc.verifier()?;
        self.edition.verifier()?;
        if self.support.as_ref().is_some_and(|n| n.chars().count() > 60) {
            return Err("tarifs_invalides");
        }
        couleur::verifier(&self.couleur, profils)
    }

    /// Rien n'est imprimé en découpe seule : ni repères, ni couleur, ni mosaïque.
    pub fn normaliser(mut self) -> Self {
        if self.mode == Mode::Decouper {
            self.reperage = reperage::Reperage::Aucun;
            self.couleur = couleur::Couleur::default();
            self.mosaique = None;
            self.blanc = blanc::Blanc::default();
            // Rien n'est imprimé : pas d'œillets ni de marge ; échelle, rotation, miroir et
            // contour restent (ils changent la découpe).
            (self.edition.oeillets, self.edition.marge_mm) = (None, 0.0);
        }
        self
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Reglage {
    pub nom: String,
    pub demande: Demande,
}

/// Nom de réglage : sert aussi de nom de dossier surveillé, donc lettres, chiffres,
/// espaces, `-`, `_`, `+` seulement.
pub fn nom_valide(nom: &str) -> bool {
    let n = nom.trim();
    !n.is_empty() && n == nom && n.chars().count() <= 60 && n.chars().all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '+'))
}

pub fn fichier(donnees: &Path) -> PathBuf {
    donnees.join("reglages.json")
}

pub fn dossier_entree(donnees: &Path, nom: &str) -> PathBuf {
    donnees.join("entrees").join(nom)
}

pub fn lire(donnees: &Path) -> Vec<Reglage> {
    std::fs::read(fichier(donnees)).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_default()
}

/// Écrit la liste (fichier temporaire puis renommage) et crée le dossier surveillé de
/// chaque réglage.
pub fn ecrire(donnees: &Path, reglages: &[Reglage]) -> std::io::Result<()> {
    for r in reglages {
        std::fs::create_dir_all(dossier_entree(donnees, &r.nom))?;
    }
    let tmp = fichier(donnees).with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(reglages).unwrap())?;
    std::fs::rename(tmp, fichier(donnees))
}

/// Fichiers prêts : présents avec la même taille qu'au passage précédent (un fichier en
/// cours de copie grossit encore). `avant` est remplacé par l'état courant.
pub fn stables(avant: &mut std::collections::HashMap<PathBuf, u64>, maintenant: Vec<(PathBuf, u64)>) -> Vec<PathBuf> {
    let prets = maintenant.iter().filter(|(p, t)| *t > 0 && avant.get(p) == Some(t)).map(|(p, _)| p.clone()).collect();
    *avant = maintenant.into_iter().collect();
    prets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noms() {
        assert!(nom_valide("Vinyle brillant + cadre Kona"));
        assert!(nom_valide("Bâche_720-dpi"));
        assert!(!nom_valide(""));
        assert!(!nom_valide(" espace devant"));
        assert!(!nom_valide("../evil"));
        assert!(!nom_valide("a/b"));
    }

    #[test]
    fn enregistrement_et_relecture() {
        let d = std::env::temp_dir().join(format!("eko-rip-reglages-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let r = Reglage {
            nom: "Vinyle + cadre".into(),
            demande: Demande { reperage: reperage::Reperage::Cadre, mosaique: Some(Mosaique { laize_mm: 1600.0, recouvrement_mm: 20.0, reperes: false }), ..Default::default() },
        };
        ecrire(&d, std::slice::from_ref(&r)).unwrap();
        assert_eq!(lire(&d), [r]);
        assert!(dossier_entree(&d, "Vinyle + cadre").is_dir(), "dossier surveillé créé");
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn decoupe_seule_sans_impression() {
        let d = Demande { mode: Mode::Decouper, reperage: reperage::Reperage::Cadre, mosaique: Some(Mosaique { laize_mm: 1600.0, recouvrement_mm: 20.0, reperes: false }), ..Default::default() }.normaliser();
        assert_eq!((d.reperage, d.mosaique), (reperage::Reperage::Aucun, None));
        let mauvaise = Demande { mosaique: Some(Mosaique { laize_mm: 100.0, recouvrement_mm: 150.0, reperes: false }), ..Default::default() };
        assert_eq!(mauvaise.verifier(&[]), Err("mosaique_invalide"));
        assert_eq!(Demande { dpi: 10, ..Default::default() }.verifier(&[]), Err("dpi_invalide"));
    }

    #[test]
    fn un_fichier_qui_grossit_attend() {
        let mut vu = std::collections::HashMap::new();
        let p = PathBuf::from("/e/a.pdf");
        assert!(stables(&mut vu, vec![(p.clone(), 100)]).is_empty(), "premier passage : on attend");
        assert!(stables(&mut vu, vec![(p.clone(), 250)]).is_empty(), "il grossit encore");
        assert_eq!(stables(&mut vu, vec![(p.clone(), 250)]), [p], "taille stable : prêt");
        assert!(stables(&mut vu, vec![(PathBuf::from("/e/vide.pdf"), 0), (PathBuf::from("/e/vide.pdf"), 0)]).is_empty(), "fichier vide jamais prêt");
    }
}
