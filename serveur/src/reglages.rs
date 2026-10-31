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

/// Mosaïque (`mosaique.rs`) : tuiles d'au plus `laize_mm` de large, qui se recouvrent.
#[derive(Clone, Default, PartialEq, Debug, Serialize, Deserialize)]
pub struct Mosaique {
    pub laize_mm: f64,
    pub recouvrement_mm: f64,
    /// Bande de pied avec le numéro du lé et les traits d'assemblage.
    #[serde(default)]
    pub reperes: bool,
    /// Lés inégaux : largeur imprimée de chaque colonne, recouvrements compris ; leur somme
    /// vaut la largeur de la page plus un recouvrement par jonction.
    #[serde(default)]
    pub les_mm: Option<Vec<f64>>,
    /// Nombre de colonnes égales imposé (sinon le moins possible).
    #[serde(default)]
    pub colonnes: Option<usize>,
    /// Hauteur maximale d'une tuile : au-delà, la page est aussi coupée en rangées.
    #[serde(default)]
    pub hauteur_max_mm: Option<f64>,
    #[serde(default)]
    pub rangees: Option<usize>,
    /// Rangées inégales : hauteur de chaque rangée, du haut vers le bas.
    #[serde(default)]
    pub rangees_mm: Option<Vec<f64>>,
    /// Le recouvrement qui passe sous la voisine (à gauche, en haut) reste sans encre.
    #[serde(default)]
    pub soudure: bool,
    /// Trait de coupe au milieu de chaque recouvrement.
    #[serde(default)]
    pub coupe_milieu: bool,
    /// Tuiles décochées (numéros de 1) : ni rendues, ni mises en file.
    #[serde(default)]
    pub exclus: Vec<usize>,
    /// Ordre d'impression : ces tuiles d'abord, puis les autres par numéro.
    #[serde(default)]
    pub ordre: Vec<usize>,
    /// Recouvrement propre à une tuile (indice = numéro − 1), négatif pour un espace.
    #[serde(default)]
    pub recouvrement_tuiles_mm: Vec<Option<f64>>,
}

/// Recouvrement minimal d'une soudure : un demi-pouce.
pub const SOUDURE_MIN_MM: f64 = 12.7;

impl Mosaique {
    fn verifier(&self) -> Result<(), &'static str> {
        let tailles = |v: &Option<Vec<f64>>| v.as_ref().is_none_or(|v| !v.is_empty() && v.len() <= 50 && v.iter().all(|t| (1.0..=10000.0).contains(t)));
        let compte = |n: Option<usize>| n.is_none_or(|n| (1..=50).contains(&n));
        let ok = (50.0..=5000.0).contains(&self.laize_mm)
            && (0.0..=200.0).contains(&self.recouvrement_mm)
            && self.recouvrement_mm < self.laize_mm
            && self.hauteur_max_mm.is_none_or(|h| (50.0..=10000.0).contains(&h) && h > self.recouvrement_mm)
            && tailles(&self.les_mm)
            && tailles(&self.rangees_mm)
            && compte(self.colonnes)
            && compte(self.rangees)
            && self.exclus.len() <= 2500
            && self.ordre.len() <= 2500
            && self.recouvrement_tuiles_mm.len() <= 2500
            && self.recouvrement_tuiles_mm.iter().flatten().all(|r| (-200.0..=200.0).contains(r));
        if !ok {
            return Err("mosaique_invalide");
        }
        if self.soudure && (self.recouvrement_mm < SOUDURE_MIN_MM || self.recouvrement_tuiles_mm.iter().flatten().any(|r| *r < SOUDURE_MIN_MM)) {
            return Err("soudure_invalide");
        }
        Ok(())
    }
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
    /// Support du catalogue des tarifs (prix de revient) ; déduit de `support_uuid` quand
    /// celui-ci est donné.
    #[serde(default)]
    pub support: Option<String>,
    /// Support et mode d'impression du catalogue (`supports.json`, roadmap 1.2) : le mode
    /// impose la résolution du rendu.
    #[serde(default)]
    pub support_uuid: Option<uuid::Uuid>,
    #[serde(default)]
    pub mode_uuid: Option<uuid::Uuid>,
    /// Découpeuse affectée au travail (identifiant du catalogue) : la découpe automatique y
    /// va, et l'impression attend qu'elle soit utilisable (roadmap 4.16). Le réglage de coupe
    /// se choisit désormais par couche (`Travail::couches`).
    #[serde(default)]
    pub decoupeuse: Option<String>,
    /// Réglage de coupe choisi pour tout le travail avant les couches (avant la 0.20) : lu
    /// pour ne rien perdre, puis porté au rendu sur la couche qu'il désigne sans ambiguïté
    /// (`decoupe::reprendre_ancien_reglage`) ; sinon gardé et montré à l'écran.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parametre_decoupe: Option<String>,
    /// Tourner de 90° une page trop large pour la laize de l'imprimante, si elle y tient
    /// alors (repérage compris). Sans effet avec une rotation choisie ou une mosaïque.
    #[serde(default = "vrai")]
    pub rotation_auto: bool,
    /// Étendre l'image près des tracés de découpe (roadmap 4.10), sur les plaques rendues.
    #[serde(default)]
    pub etendre_coupe: Option<crate::coupe::Etendre>,
    /// Fond sous les repères (roadmap 4.15), pour un support sombre ou transparent.
    #[serde(default)]
    pub fond_reperes: Option<reperage::FondReperes>,
    /// Un PDF de plusieurs pages devient un travail par page (roadmap 9.3).
    #[serde(default)]
    pub page_par_travail: bool,
    /// Refuser un PDF dont une police n'est pas incorporée (hors les 14 polices de base,
    /// que Ghostscript remplace par leurs équivalentes métriques) (roadmap 9.3).
    #[serde(default)]
    pub abandon_polices: bool,
    /// Imprimante de la mise en file automatique (identifiant du catalogue) ; absente : la
    /// première qui imprime. Posée par la balise `{machine}` d'un nom de fichier (roadmap 9.10).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imprimante: Option<String>,
    /// Impressions de chaque page mises en file d'office (balise `{repetition}`, roadmap 9.10).
    #[serde(default = "un", skip_serializing_if = "est_un")]
    pub repetition: u32,
    /// Épreuve TIFF faite d'office après le rendu, une par page (roadmap 9.13).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub epreuve: Option<crate::epreuve::Options>,
    /// Épreuve seule : l'épreuve TIFF est faite, rien ne part à l'impression.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub epreuve_seule: bool,
    /// Rééchantillonnage des images matricielles agrandies ou réduites au rendu (roadmap 9.15).
    #[serde(default, skip_serializing_if = "Reechantillonnage::est_defaut")]
    pub reechantillonnage: Reechantillonnage,
    /// Couches posées sur l'Epson, de la première à la dernière (roadmap 2.10) ; vide :
    /// `couches` décide (blanc ou argent dessous ou dessus).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub empilement: Vec<epson::Couche>,
}

/// Comment Ghostscript passe une image de sa résolution à celle du rendu (roadmap 9.15).
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reechantillonnage {
    /// Plus proche voisin : pixels francs, le choix des codes-barres et des aplats nets.
    #[default]
    ProcheVoisin,
    /// Filtre cubique (Mitchell) de Ghostscript : dégradés et photos agrandies sans escaliers.
    Lisse,
}

impl Reechantillonnage {
    fn est_defaut(&self) -> bool {
        *self == Reechantillonnage::ProcheVoisin
    }
}

fn vrai() -> bool {
    true
}

fn un() -> u32 {
    1
}

fn est_un(n: &u32) -> bool {
    *n == 1
}

/// Répétition d'une impression mise en file d'office : 100 au plus.
pub const REPETITION_MAX: u32 = 100;

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
        Demande { mode: Mode::Imprimer, tous_les_traces: false, reperage: reperage::Reperage::Aucun, couleur: couleur::Couleur::default(), dpi: dpi_defaut(), mosaique: None, blanc: blanc::Blanc::default(), couches: epson::Couches::default(), edition: edition::Edition::default(), auto: Auto::default(), support: None, support_uuid: None, mode_uuid: None, decoupeuse: None, parametre_decoupe: None, rotation_auto: true, etendre_coupe: None, fond_reperes: None, page_par_travail: false, abandon_polices: false, imprimante: None, repetition: 1, epreuve: None, epreuve_seule: false, reechantillonnage: Reechantillonnage::ProcheVoisin, empilement: vec![] }
    }
}

impl Demande {
    /// Options de Ghostscript du rendu : couleur, puis rééchantillonnage des images.
    pub fn options_ghostscript(&self, profils: &std::path::Path) -> Vec<String> {
        let mut o = couleur::options_ghostscript(&self.couleur, profils);
        if self.reechantillonnage == Reechantillonnage::Lisse {
            o.push("-dDOINTERPOLATE".into());
        }
        o
    }

    /// Contrôle les bornes ; rend le code d'erreur traduit par l'écran.
    pub fn verifier(&self, profils: &[couleur::Profil]) -> Result<(), &'static str> {
        if !(36..=2880).contains(&self.dpi) {
            return Err("dpi_invalide");
        }
        if let Some(m) = &self.mosaique {
            m.verifier()?;
        }
        self.blanc.verifier()?;
        self.epreuve.as_ref().map_or(Ok(()), crate::epreuve::Options::verifier)?;
        self.edition.verifier()?;
        self.etendre_coupe.as_ref().map_or(Ok(()), crate::coupe::Etendre::verifier)?;
        self.fond_reperes.as_ref().map_or(Ok(()), reperage::FondReperes::verifier)?;
        if self.support.as_ref().is_some_and(|n| n.chars().count() > 60) {
            return Err("tarifs_invalides");
        }
        if self.empilement.len() > epson::COUCHES_MAX {
            return Err("empilement_invalide");
        }
        if self.imprimante.as_ref().is_some_and(|n| n.is_empty() || n.len() > 30) || !(1..=REPETITION_MAX).contains(&self.repetition) {
            return Err("demande_invalide");
        }
        if self.decoupeuse.as_ref().is_some_and(|n| n.is_empty() || n.len() > 30) || self.parametre_decoupe.as_ref().is_some_and(|n| n.chars().count() > 40) {
            return Err("decoupeuse_invalide");
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
            (self.epreuve, self.epreuve_seule) = (None, false);
            // Rien n'est imprimé : pas d'œillets ni de marge ; échelle, rotation, miroir et
            // contour restent (ils changent la découpe).
            (self.edition.oeillets, self.edition.marge_mm) = (None, 0.0);
            // Ni marques, ni fond perdu, ni étiquette, ni code.
            (self.edition.marques, self.edition.fond_perdu, self.edition.etiquette, self.edition.code, self.edition.filtre) = (None, None, None, None, None);
        }
        self
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Reglage {
    /// Identifiant immuable pour les échanges (ERP) ; un réglage renommé est un autre réglage
    /// (son nom est aussi son dossier surveillé).
    #[serde(default)]
    pub uuid: Option<uuid::Uuid>,
    pub nom: String,
    pub demande: Demande,
}

/// Nom de réglage : sert aussi de nom de dossier surveillé, donc lettres, chiffres,
/// espaces, `-`, `_`, `+` seulement.
pub fn nom_valide(nom: &str) -> bool {
    let n = nom.trim();
    !n.is_empty() && n == nom && n.chars().count() <= 60 && n.chars().all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '+'))
}

/// Premier nom libre « nom 2 », « nom 3 »… (60 caractères au plus), pour un import qui
/// renomme au lieu d'écraser (roadmap 1.6).
pub fn nom_libre(nom: &str, pris: &[String]) -> String {
    (2..)
        .map(|i| {
            let suffixe = format!(" {i}");
            let base: String = nom.chars().take(60 - suffixe.chars().count()).collect();
            format!("{}{suffixe}", base.trim_end())
        })
        .find(|n| !pris.contains(n))
        .unwrap()
}

pub fn fichier(donnees: &Path) -> PathBuf {
    donnees.join("reglages.json")
}

pub fn dossier_entree(donnees: &Path, nom: &str) -> PathBuf {
    donnees.join("entrees").join(nom)
}

pub fn lire(donnees: &Path) -> Vec<Reglage> {
    crate::etat::lire(&fichier(donnees)).ok().flatten().unwrap_or_default()
}

/// Écrit la liste (fichier temporaire puis renommage) et crée le dossier surveillé de
/// chaque réglage.
/// Identifiants des réglages : repris du réglage connu de même nom, sinon nouveaux.
pub fn identifier(reglages: &mut [Reglage], connus: &[Reglage]) {
    for r in reglages.iter_mut().filter(|r| r.uuid.is_none()) {
        r.uuid = Some(connus.iter().find(|c| c.nom == r.nom).and_then(|c| c.uuid).unwrap_or_else(uuid::Uuid::new_v4));
    }
}

pub fn ecrire(donnees: &Path, reglages: &[Reglage]) -> std::io::Result<()> {
    for r in reglages {
        std::fs::create_dir_all(dossier_entree(donnees, &r.nom))?;
    }
    crate::etat::ecrire(&fichier(donnees), reglages, false)?;
    crate::pont::reveiller();
    Ok(())
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

    /// Roadmap 2.10 : l'empilement se lit en JSON, s'omet vide, et compte 5 couches au plus.
    #[test]
    fn empilement_lu_et_borne() {
        use epson::Couche::*;
        let d: Demande = serde_json::from_value(serde_json::json!({ "empilement": ["blanc", "couleur", "blanc"] })).unwrap();
        assert_eq!(d.empilement, [Blanc, Couleur, Blanc]);
        assert!(d.verifier(&[]).is_ok());
        assert!(serde_json::to_value(Demande::default()).unwrap().get("empilement").is_none(), "vide : omis");
        let trop = Demande { empilement: vec![Couleur; epson::COUCHES_MAX + 1], ..Default::default() };
        assert_eq!(trop.verifier(&[]), Err("empilement_invalide"));
    }

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
            uuid: None,
            nom: "Vinyle + cadre".into(),
            demande: Demande { reperage: reperage::Reperage::Cadre, mosaique: Some(Mosaique { laize_mm: 1600.0, recouvrement_mm: 20.0, ..Default::default() }), ..Default::default() },
        };
        ecrire(&d, std::slice::from_ref(&r)).unwrap();
        assert_eq!(lire(&d), [r]);
        assert!(dossier_entree(&d, "Vinyle + cadre").is_dir(), "dossier surveillé créé");
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn decoupe_seule_sans_impression() {
        let d = Demande { mode: Mode::Decouper, reperage: reperage::Reperage::Cadre, mosaique: Some(Mosaique { laize_mm: 1600.0, recouvrement_mm: 20.0, ..Default::default() }), ..Default::default() }.normaliser();
        assert_eq!((d.reperage, d.mosaique), (reperage::Reperage::Aucun, None));
        let mauvaise = Demande { mosaique: Some(Mosaique { laize_mm: 100.0, recouvrement_mm: 150.0, ..Default::default() }), ..Default::default() };
        assert_eq!(mauvaise.verifier(&[]), Err("mosaique_invalide"));
        let soudure = |r: f64, propre: Option<f64>| Demande { mosaique: Some(Mosaique { laize_mm: 1600.0, recouvrement_mm: r, soudure: true, recouvrement_tuiles_mm: vec![propre], ..Default::default() }), ..Default::default() }.verifier(&[]);
        assert_eq!(soudure(10.0, None), Err("soudure_invalide"), "moins d'un demi-pouce");
        assert_eq!(soudure(20.0, Some(10.0)), Err("soudure_invalide"), "une tuile sous le demi-pouce");
        assert_eq!(soudure(12.7, None), Ok(()));
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

    /// `reglages.json` d'avant les couches : `parametre_decoupe` se lit et se garde (il sera
    /// porté sur une couche au rendu, ou montré à l'écran), rien ne tombe en silence.
    #[test]
    fn ancien_reglages_json_avec_parametre_decoupe() {
        let d = std::env::temp_dir().join(format!("eko-rip-reglages-anciens-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(fichier(&d), r#"[{ "nom": "Vinyle découpe", "demande": { "mode": "imprimer", "dpi": 720, "parametre_decoupe": "Mi-chair V3000" } }]"#).unwrap();
        let lus = lire(&d);
        assert_eq!(lus.len(), 1, "l'ancien fichier se lit");
        assert_eq!(lus[0].demande.parametre_decoupe.as_deref(), Some("Mi-chair V3000"));
        assert!(lus[0].demande.verifier(&[]).is_ok());
        // Réécrit puis relu : toujours là.
        crate::etat::ecrire(&fichier(&d), &lus, false).unwrap();
        assert_eq!(lire(&d)[0].demande.parametre_decoupe.as_deref(), Some("Mi-chair V3000"));
        // Un réglage récent ne l'écrit pas.
        assert!(!serde_json::to_string(&Demande::default()).unwrap().contains("parametre_decoupe"));
        std::fs::remove_dir_all(&d).unwrap();
    }
}
