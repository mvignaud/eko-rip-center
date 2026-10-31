//! Sites rattachés à la plate-forme (`sites.json`) : un site = un serveur RIP d'atelier.
//! La clé du site n'est jamais gardée, seulement son empreinte SHA-256 (clé de 32 octets
//! aléatoires : une empreinte simple suffit, rien à deviner).

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Site {
    /// Identifiant public, dans l'adresse `/s/<id>`.
    pub id: String,
    pub nom: String,
    pub cle_sha256: String,
    pub cree_le: u64,
    #[serde(default)]
    pub derniere_connexion: Option<u64>,
    /// Qui l'a validé (texte libre au lot 1 ; compte du portail ensuite).
    #[serde(default)]
    pub proprietaire: Option<String>,
}

impl Site {
    pub fn cle_valide(&self, cle: &str) -> bool {
        crate::egal_constant(&crate::sha256_hex(cle), &self.cle_sha256)
    }
}

/// Lectures-modifications-écritures de `sites.json` l'une après l'autre (appairages et
/// connexions simultanés) : aucune ne défait une autre, et un seul `sites.json.tmp` à la fois.
static VERROU: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn verrou() -> std::sync::MutexGuard<'static, ()> {
    VERROU.lock().unwrap_or_else(|p| p.into_inner())
}

pub fn fichier(donnees: &Path) -> PathBuf {
    donnees.join("sites.json")
}

/// Fichier absent : aucun site. Fichier illisible : erreur (on ne repart jamais de zéro en
/// silence, les sites perdraient leur accès).
pub fn lire(donnees: &Path) -> Result<Vec<Site>, String> {
    match std::fs::read(fichier(donnees)) {
        Ok(o) => serde_json::from_slice(&o).map_err(|e| format!("sites.json illisible : {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(vec![]),
        Err(e) => Err(format!("sites.json : {e}")),
    }
}

/// Écriture atomique (temporaire synchronisé puis renommage), droits 0600.
pub fn ecrire(donnees: &Path, sites: &[Site]) -> std::io::Result<()> {
    std::fs::create_dir_all(donnees)?;
    let tmp = donnees.join("sites.json.tmp");
    let mut o = std::fs::OpenOptions::new();
    o.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut o, 0o600);
    let mut f = o.open(&tmp)?;
    f.write_all(&serde_json::to_vec_pretty(sites).expect("sérialisation des sites"))?;
    f.sync_all()?;
    std::fs::rename(&tmp, fichier(donnees))
}

/// Identifiant de site : 10 caractères minuscules et chiffres (sans 0/o, 1/l).
pub fn nouvel_id() -> String {
    const ALPHABET: &[u8] = b"abcdefghijkmnpqrstuvwxyz23456789";
    let mut b = [0u8; 10];
    getrandom::fill(&mut b).expect("générateur aléatoire");
    b.iter().map(|o| ALPHABET[(*o as usize) % ALPHABET.len()] as char).collect()
}

/// Crée un site et rend sa clé, en clair une seule fois.
pub fn creer(donnees: &Path, nom: &str, proprietaire: Option<String>) -> Result<(Site, String), String> {
    let _v = verrou();
    let mut sites = lire(donnees)?;
    let mut id = nouvel_id();
    while sites.iter().any(|s| s.id == id) {
        id = nouvel_id();
    }
    let cle = crate::aleatoire_hex(32);
    let site = Site { id, nom: nom.to_string(), cle_sha256: crate::sha256_hex(&cle), cree_le: crate::maintenant(), derniere_connexion: None, proprietaire };
    sites.push(site.clone());
    ecrire(donnees, &sites).map_err(|e| e.to_string())?;
    Ok((site, cle))
}

/// Retire un site ; rend faux s'il n'existait pas.
pub fn supprimer(donnees: &Path, id: &str) -> Result<bool, String> {
    let _v = verrou();
    let mut sites = lire(donnees)?;
    let avant = sites.len();
    sites.retain(|s| s.id != id);
    if sites.len() == avant {
        return Ok(false);
    }
    ecrire(donnees, &sites).map_err(|e| e.to_string())?;
    Ok(true)
}

pub fn noter_connexion(donnees: &Path, id: &str) {
    let _v = verrou();
    if let Ok(mut sites) = lire(donnees)
        && let Some(s) = sites.iter_mut().find(|s| s.id == id)
    {
        s.derniere_connexion = Some(crate::maintenant());
        let _ = ecrire(donnees, &sites);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creer_verifier_supprimer() {
        let d = std::env::temp_dir().join(format!("eko-cloud-sites-{}", crate::aleatoire_hex(6)));
        assert!(lire(&d).unwrap().is_empty());
        let (s, cle) = creer(&d, "Atelier", None).unwrap();
        assert_eq!(s.id.len(), 10);
        assert_eq!(cle.len(), 64);
        let lu = lire(&d).unwrap();
        assert!(lu[0].cle_valide(&cle));
        assert!(!lu[0].cle_valide("mauvaise"));
        assert!(!lu[0].cle_sha256.contains(&cle));
        assert!(supprimer(&d, &s.id).unwrap());
        assert!(!supprimer(&d, &s.id).unwrap());
        std::fs::write(fichier(&d), b"{").unwrap();
        assert!(lire(&d).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }
}
