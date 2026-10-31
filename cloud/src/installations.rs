//! Comptage anonyme des installations E-KO RIP (`POST /api/ping`).
//!
//! Chaque serveur RIP et chaque client (Mac, Linux, Windows, mobile) envoie, à son
//! installation, à chaque mise à jour, puis une fois par semaine, un petit message : un
//! identifiant tiré au hasard sur le poste, le type de poste, l'OS, la version et, pour un
//! serveur, le nombre de machines et de réglages. Aucun appairage ni compte n'est demandé :
//! les RIP jamais rattachés au cloud sont comptés aussi.
//!
//! Rien de personnel n'est gardé : ni adresse IP, ni nom de poste ou d'atelier. Le registre
//! est `installations.json` (une ligne par identifiant), écrit au plus une fois par minute.
//!
//! Garde-fous (route publique) : champs bornés et filtrés, au plus un ping compté par
//! identifiant et par heure, au plus `MAX_NOUVEAUX_PAR_HEURE` nouveaux identifiants par heure
//! et `MAX_INSTALLATIONS` au total.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const TYPES: &[&str] = &["serveur", "client-mac", "client-linux", "client-windows", "client-mobile"];
pub const EVENEMENTS: &[&str] = &["installation", "mise_a_jour", "hebdo"];
/// Une installation est « active » si son dernier ping a moins de 15 jours (ping hebdomadaire
/// plus une semaine de marge : poste éteint pendant des congés, réseau coupé…).
pub const ACTIVE_S: u64 = 15 * 86_400;
pub const MAX_INSTALLATIONS: usize = 200_000;
pub const MAX_NOUVEAUX_PAR_HEURE: usize = 2_000;
const INTERVALLE_MIN_S: u64 = 3_600;

#[derive(Debug, Deserialize)]
pub struct Ping {
    pub id: String,
    #[serde(rename = "type")]
    pub type_poste: String,
    pub evenement: String,
    pub version: String,
    pub os: String,
    #[serde(default)]
    pub machines: Option<u32>,
    #[serde(default)]
    pub reglages: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Installation {
    #[serde(rename = "type")]
    pub type_poste: String,
    pub os: String,
    pub version: String,
    pub premiere: u64,
    pub derniere: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub machines: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reglages: Option<u32>,
    #[serde(default)]
    pub mises_a_jour: u32,
}

#[derive(Default)]
pub struct Registre {
    pub installations: HashMap<String, Installation>,
    sale: bool,
    /// Début de l'heure en cours et nouveaux identifiants vus depuis.
    heure: (u64, usize),
}

pub fn fichier(donnees: &Path) -> PathBuf {
    donnees.join("installations.json")
}

/// Fichier absent : registre vide. Illisible : erreur (on ne l'écrase pas en silence).
pub fn lire(donnees: &Path) -> Result<Registre, String> {
    match std::fs::read(fichier(donnees)) {
        Ok(o) => Ok(Registre { installations: serde_json::from_slice(&o).map_err(|e| format!("installations.json illisible : {e}"))?, ..Default::default() }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Registre::default()),
        Err(e) => Err(format!("installations.json : {e}")),
    }
}

fn texte_propre(s: &str, max: usize) -> Option<String> {
    let t = s.trim();
    (!t.is_empty() && t.chars().count() <= max && !t.chars().any(char::is_control)).then(|| t.to_string())
}

#[derive(Debug, PartialEq)]
pub enum Refus {
    Invalide,
    Sature,
}

impl Registre {
    /// Enregistre un ping. Ok(true) : compté ; Ok(false) : ignoré (même identifiant revenu
    /// dans l'heure, sans changement de version).
    pub fn noter(&mut self, p: &Ping, maintenant: u64) -> Result<bool, Refus> {
        let id_ok = p.id.len() == 32 && p.id.bytes().all(|o| o.is_ascii_digit() || (b'a'..=b'f').contains(&o));
        if !id_ok || !TYPES.contains(&p.type_poste.as_str()) || !EVENEMENTS.contains(&p.evenement.as_str()) {
            return Err(Refus::Invalide);
        }
        let (Some(version), Some(os)) = (texte_propre(&p.version, 32), texte_propre(&p.os, 64)) else { return Err(Refus::Invalide) };
        let (machines, reglages) = if p.type_poste == "serveur" { (p.machines.map(|n| n.min(10_000)), p.reglages.map(|n| n.min(10_000))) } else { (None, None) };
        if let Some(i) = self.installations.get_mut(&p.id) {
            if maintenant.saturating_sub(i.derniere) < INTERVALLE_MIN_S && i.version == version {
                return Ok(false);
            }
            if i.version != version {
                i.mises_a_jour = i.mises_a_jour.saturating_add(1);
            }
            *i = Installation { type_poste: p.type_poste.clone(), os, version, derniere: maintenant, machines, reglages, ..i.clone() };
        } else {
            if self.heure.0 + 3_600 <= maintenant {
                self.heure = (maintenant, 0);
            }
            if self.heure.1 >= MAX_NOUVEAUX_PAR_HEURE || self.installations.len() >= MAX_INSTALLATIONS {
                return Err(Refus::Sature);
            }
            self.heure.1 += 1;
            self.installations.insert(p.id.clone(), Installation { type_poste: p.type_poste.clone(), os, version, premiere: maintenant, derniere: maintenant, machines, reglages, mises_a_jour: 0 });
        }
        self.sale = true;
        Ok(true)
    }

    /// Contenu à écrire si le registre a changé depuis la dernière écriture.
    pub fn a_ecrire(&mut self) -> Option<Vec<u8>> {
        if !self.sale {
            return None;
        }
        self.sale = false;
        Some(serde_json::to_vec(&self.installations).expect("sérialisation des installations"))
    }

    /// Écriture ratée : le registre reste à écrire (nouvel essai au prochain passage).
    pub fn remettre_a_ecrire(&mut self) {
        self.sale = true;
    }

    pub fn statistiques(&self, maintenant: u64) -> Statistiques {
        let mut s = Statistiques { total: self.installations.len(), ..Default::default() };
        let semaine = |t: u64| maintenant.saturating_sub(t) / (7 * 86_400);
        for i in self.installations.values() {
            if (semaine(i.premiere) as usize) < 8 {
                s.nouvelles_par_semaine[semaine(i.premiere) as usize] += 1;
            }
            if maintenant.saturating_sub(i.derniere) > ACTIVE_S {
                continue;
            }
            s.actives += 1;
            *s.par_type.entry(i.type_poste.clone()).or_default() += 1;
            *s.par_os.entry(i.os.clone()).or_default() += 1;
            *s.par_version.entry(format!("{} {}", i.type_poste, i.version)).or_default() += 1;
            s.machines += u64::from(i.machines.unwrap_or(0));
        }
        s
    }
}

#[derive(Debug, Default, Serialize)]
pub struct Statistiques {
    /// Tous les identifiants jamais vus.
    pub total: usize,
    /// Vus depuis moins de 15 jours.
    pub actives: usize,
    pub par_type: BTreeMap<String, usize>,
    pub par_os: BTreeMap<String, usize>,
    pub par_version: BTreeMap<String, usize>,
    /// Machines pilotées par les serveurs actifs.
    pub machines: u64,
    /// Nouvelles installations : [cette semaine, la précédente, …] sur 8 semaines.
    pub nouvelles_par_semaine: [usize; 8],
}

/// Écriture atomique, droits 0600.
pub fn ecrire(donnees: &Path, contenu: &[u8]) -> std::io::Result<()> {
    std::fs::create_dir_all(donnees)?;
    let tmp = donnees.join("installations.json.tmp");
    let mut o = std::fs::OpenOptions::new();
    o.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut o, 0o600);
    let mut f = o.open(&tmp)?;
    f.write_all(contenu)?;
    f.sync_all()?;
    std::fs::rename(&tmp, fichier(donnees))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ping(id: &str, version: &str) -> Ping {
        Ping { id: id.into(), type_poste: "serveur".into(), evenement: "installation".into(), version: version.into(), os: "Ubuntu 26.04 LTS".into(), machines: Some(3), reglages: Some(5) }
    }

    const A: &str = "0123456789abcdef0123456789abcdef";
    const B: &str = "fedcba9876543210fedcba9876543210";

    #[test]
    fn compte_et_filtre() {
        let mut r = Registre::default();
        let t = 1_000_000;
        assert_eq!(r.noter(&ping(A, "0.29.0"), t), Ok(true));
        // Revenu dans l'heure, même version : ignoré.
        assert_eq!(r.noter(&ping(A, "0.29.0"), t + 60), Ok(false));
        // Mise à jour : comptée tout de suite.
        assert_eq!(r.noter(&ping(A, "0.30.0"), t + 120), Ok(true));
        assert_eq!(r.installations[A].mises_a_jour, 1);
        assert_eq!(r.installations[A].premiere, t);
        // Client : pas de nombre de machines gardé.
        let mut c = ping(B, "0.30.0");
        c.type_poste = "client-mac".into();
        c.os = "macOS 26.1.0".into();
        assert_eq!(r.noter(&c, t), Ok(true));
        assert_eq!(r.installations[B].machines, None);
        // Champs refusés.
        for p in [
            Ping { id: "court".into(), ..ping(A, "1") },
            Ping { id: A.to_uppercase(), ..ping(A, "1") },
            Ping { type_poste: "autre".into(), ..ping(A, "1") },
            Ping { evenement: "x".into(), ..ping(A, "1") },
            Ping { os: "a\nb".into(), ..ping(A, "1") },
            Ping { os: "x".repeat(65), ..ping(A, "1") },
            ping(A, ""),
        ] {
            assert_eq!(r.noter(&p, t), Err(Refus::Invalide));
        }
        let s = r.statistiques(t + 200);
        assert_eq!((s.total, s.actives, s.machines), (2, 2, 3));
        assert_eq!(s.par_type["serveur"], 1);
        assert_eq!(s.par_version["client-mac 0.30.0"], 1);
        assert_eq!(s.nouvelles_par_semaine[0], 2);
        // Trois semaines plus tard, plus active.
        let s = r.statistiques(t + 21 * 86_400);
        assert_eq!((s.total, s.actives, s.machines), (2, 0, 0));
        assert_eq!(s.nouvelles_par_semaine[3], 2);
    }

    #[test]
    fn sature_et_ecrit() {
        let mut r = Registre::default();
        for n in 0..MAX_NOUVEAUX_PAR_HEURE {
            assert_eq!(r.noter(&ping(&format!("{n:032x}"), "1"), 10), Ok(true));
        }
        assert_eq!(r.noter(&ping(&format!("{:032x}", u64::MAX), "1"), 20), Err(Refus::Sature));
        assert_eq!(r.noter(&ping(&format!("{:032x}", u64::MAX), "1"), 3_700), Ok(true));
        let d = std::env::temp_dir().join(format!("eko-cloud-inst-{}", crate::aleatoire_hex(6)));
        assert_eq!(lire(&d).unwrap().installations.len(), 0);
        ecrire(&d, &r.a_ecrire().unwrap()).unwrap();
        assert!(r.a_ecrire().is_none());
        assert_eq!(lire(&d).unwrap().installations.len(), MAX_NOUVEAUX_PAR_HEURE + 1);
        std::fs::write(fichier(&d), b"[").unwrap();
        assert!(lire(&d).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }
}
