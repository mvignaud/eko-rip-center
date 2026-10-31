//! Rappels de recalibration (roadmap 8.5), par imprimante : le dernier étalonnage (relevé
//! des mires et linéarisation refaits) et ce qui s'est passé depuis. Jamais bloquant :
//! l'écran et l'API montrent un rappel, l'impression continue.
//!
//! - **14 jours** après l'étalonnage : rappel « à refaire » ;
//! - **3 mois** (90 jours) : plafond, l'étalonnage est périmé ;
//! - **déclencheurs**, quel que soit l'âge : un changement de poche vu dans les relevés
//!   SNMP (hausse d'une encre, `releves.rs`), un nouveau lot d'encre ou de support, une
//!   maintenance, saisis par l'opérateur.
//!
//! Gardé dans `etalonnages.json` : un étalonnage efface les déclencheurs d'avant lui.

use crate::releves::Releve;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const RAPPEL_JOURS: u64 = 14;
pub const PLAFOND_JOURS: u64 = 90;
/// Hausse d'une encre (points de %) entre deux relevés lue comme une poche changée :
/// assez haut pour ignorer le % près de l'imprimante et ses arrondis.
pub const HAUSSE_POCHE: u8 = 10;
const JOUR: u64 = 86_400;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "motif")]
pub enum Declencheur {
    /// Poche d'encre changée, vue dans les relevés SNMP.
    Poche { encre: String },
    NouveauLot {
        #[serde(default)]
        note: String,
    },
    Maintenance {
        #[serde(default)]
        note: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Evenement {
    /// Secondes depuis l'époque Unix.
    pub le: u64,
    #[serde(flatten)]
    pub declencheur: Declencheur,
}

/// Ce qui est gardé pour une imprimante.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Fiche {
    /// Dernier étalonnage (secondes Unix).
    #[serde(default)]
    pub etalonne_le: Option<u64>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
    /// Déclencheurs saisis depuis l'étalonnage (nouveau lot, maintenance).
    #[serde(default)]
    pub evenements: Vec<Evenement>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Statut {
    /// Aucun étalonnage enregistré.
    Jamais,
    AJour,
    /// À refaire : 14 jours passés ou un déclencheur depuis.
    Rappel,
    /// Plus de 3 mois : périmé.
    Perime,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Etat {
    pub statut: Statut,
    pub etalonne_le: Option<u64>,
    /// Date du rappel des 14 jours, et du plafond des 3 mois.
    pub rappel_le: Option<u64>,
    pub plafond_le: Option<u64>,
    /// Déclencheurs depuis l'étalonnage, du plus ancien au plus récent (SNMP et saisis).
    pub declencheurs: Vec<Evenement>,
}

/// Poches changées d'après les relevés : chaque hausse d'au moins `HAUSSE_POCHE` points
/// d'une encre entre deux relevés successifs, datée du second, après `depuis`.
pub fn poches_changees(releves: &[Releve], depuis: u64) -> Vec<Evenement> {
    let mut v = vec![];
    for paire in releves.windows(2) {
        let (avant, apres) = (&paire[0], &paire[1]);
        if apres.le <= depuis {
            continue;
        }
        for (encre, niveau) in &apres.encres {
            if avant.encres.iter().any(|(e, n)| e == encre && niveau.saturating_sub(*n) >= HAUSSE_POCHE) {
                v.push(Evenement { le: apres.le, declencheur: Declencheur::Poche { encre: encre.clone() } });
            }
        }
    }
    v
}

pub fn etat(f: &Fiche, releves: &[Releve], maintenant: u64) -> Etat {
    let Some(le) = f.etalonne_le else {
        return Etat { statut: Statut::Jamais, etalonne_le: None, rappel_le: None, plafond_le: None, declencheurs: vec![] };
    };
    let mut declencheurs: Vec<Evenement> = f.evenements.iter().filter(|e| e.le >= le).cloned().chain(poches_changees(releves, le)).collect();
    declencheurs.sort_by_key(|e| e.le);
    let (rappel, plafond) = (le + RAPPEL_JOURS * JOUR, le + PLAFOND_JOURS * JOUR);
    let statut = if maintenant >= plafond {
        Statut::Perime
    } else if maintenant >= rappel || !declencheurs.is_empty() {
        Statut::Rappel
    } else {
        Statut::AJour
    };
    Etat { statut, etalonne_le: Some(le), rappel_le: Some(rappel), plafond_le: Some(plafond), declencheurs }
}

fn fichier(donnees: &Path) -> PathBuf {
    donnees.join("etalonnages.json")
}

/// Fiches par imprimante ; un fichier absent ou illisible donne aucune fiche.
pub fn lire(donnees: &Path) -> BTreeMap<String, Fiche> {
    std::fs::read(fichier(donnees)).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_default()
}

pub fn ecrire(donnees: &Path, fiches: &BTreeMap<String, Fiche>) -> Result<(), String> {
    crate::machines::ecrire_atomique(&fichier(donnees), &serde_json::to_vec_pretty(fiches).map_err(|e| e.to_string())?)
}

/// Étalonnage refait : il remplace le précédent et efface les déclencheurs.
pub fn etalonner(f: &mut Fiche, le: u64, note: String) {
    *f = Fiche { etalonne_le: Some(le), note, evenements: vec![] };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn releve(le: u64, cyan: u8) -> Releve {
        Releve { le, encres: vec![("Cyan".into(), cyan), ("Black".into(), 50)], longueur_m: None }
    }

    #[test]
    fn delais_14_jours_et_3_mois() {
        assert_eq!(etat(&Fiche::default(), &[], 0).statut, Statut::Jamais);
        let mut f = Fiche::default();
        etalonner(&mut f, 1000, "mire HEXIS".into());
        assert_eq!(etat(&f, &[], 1000 + 13 * JOUR).statut, Statut::AJour);
        let e = etat(&f, &[], 1000 + 14 * JOUR);
        assert_eq!((e.statut, e.rappel_le, e.plafond_le), (Statut::Rappel, Some(1000 + 14 * JOUR), Some(1000 + 90 * JOUR)));
        assert_eq!(etat(&f, &[], 1000 + 90 * JOUR).statut, Statut::Perime);
    }

    /// Critère de la roadmap : une hausse SNMP d'une encre (poche changée) → rappel, même
    /// le lendemain de l'étalonnage ; la baisse ordinaire et l'arrondi n'en font pas.
    #[test]
    fn hausse_snmp_d_une_encre_rappel() {
        let mut f = Fiche::default();
        etalonner(&mut f, 1000, String::new());
        let releves = [releve(500, 4), releve(900, 90), releve(2000, 60), releve(3000, 65), releve(4000, 5), releve(5000, 98)];
        let e = etat(&f, &releves, 6000);
        assert_eq!(e.statut, Statut::Rappel);
        assert_eq!(e.declencheurs, [Evenement { le: 5000, declencheur: Declencheur::Poche { encre: "Cyan".into() } }], "la poche changée avant l'étalonnage ne compte plus, +5 points non plus");
        assert_eq!(etat(&f, &releves[..5], 6000).statut, Statut::AJour);
    }

    #[test]
    fn declencheurs_saisis_et_effaces_par_l_etalonnage() {
        let mut f = Fiche::default();
        etalonner(&mut f, 1000, String::new());
        f.evenements.push(Evenement { le: 2000, declencheur: Declencheur::NouveauLot { note: "lot 24-118".into() } });
        assert_eq!(etat(&f, &[], 3000).statut, Statut::Rappel);
        etalonner(&mut f, 4000, String::new());
        assert_eq!(etat(&f, &[], 5000).statut, Statut::AJour);
        f.evenements.push(Evenement { le: 4000, declencheur: Declencheur::Maintenance { note: String::new() } });
        assert_eq!(etat(&f, &[], 5000).statut, Statut::Rappel, "saisi dans la seconde de l'étalonnage, après lui");
        let lu: Evenement = serde_json::from_value(serde_json::json!({ "le": 1, "motif": "maintenance", "note": "têtes nettoyées" })).unwrap();
        assert_eq!(lu.declencheur, Declencheur::Maintenance { note: "têtes nettoyées".into() });
    }
}
