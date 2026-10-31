//! Appairage d'un serveur RIP, sans clé à recopier :
//!
//! 1. le serveur RIP demande un appairage (`POST /api/appairage`) et reçoit un numéro de
//!    demande (secret, gardé par lui) et un code court à montrer (`KXRT-4821`) ;
//! 2. quelqu'un valide ce code (lot 1 : `eko-rip-cloud valider <code>` sur la plate-forme ;
//!    ensuite : le portail, une fois connecté à son compte) ;
//! 3. le serveur RIP, qui interroge `GET /api/appairage/<demande>`, reçoit alors son site et
//!    sa clé, une seule fois.
//!
//! Une validation est un fichier `validations/<CODE>` : la commande en ligne et le service
//! n'ont rien d'autre à partager.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const DUREE: Duration = Duration::from_secs(600);
/// Demandes en attente au plus (toutes confondues).
pub const MAX_EN_ATTENTE: usize = 500;

#[derive(Clone, Debug)]
pub struct Demande {
    pub code: String,
    pub nom: String,
    pub creee: Instant,
}

#[derive(Default)]
pub struct Appairages {
    demandes: HashMap<String, Demande>,
}

/// `kxrt 4821`, `KXRT-4821` → `KXRT4821` ; None si ce n'est pas un code.
pub fn normaliser(code: &str) -> Option<String> {
    let c: String = code.chars().filter(|c| !matches!(c, '-' | ' ')).collect::<String>().to_ascii_uppercase();
    (c.len() == 8 && c.is_ascii() && c[..4].chars().all(|x| LETTRES.contains(x)) && c[4..].chars().all(|x| x.is_ascii_digit())).then_some(c)
}

/// Forme affichée : `KXRT-4821`.
pub fn afficher(code: &str) -> String {
    format!("{}-{}", &code[..4], &code[4..])
}

/// Lettres sans I ni O (lues 1 et 0).
const LETTRES: &str = "ABCDEFGHJKLMNPQRSTUVWXYZ";

fn nouveau_code() -> String {
    let mut b = [0u8; 8];
    getrandom::fill(&mut b).expect("générateur aléatoire");
    let l: Vec<char> = LETTRES.chars().collect();
    let lettres: String = b[..4].iter().map(|o| l[(*o as usize) % l.len()]).collect();
    let chiffres: String = b[4..].iter().map(|o| char::from(b'0' + o % 10)).collect();
    lettres + &chiffres
}

pub fn dossier_validations(donnees: &Path) -> PathBuf {
    donnees.join("validations")
}

/// Ce que fait `eko-rip-cloud valider <code> [propriétaire]`.
pub fn valider(donnees: &Path, code: &str, proprietaire: &str) -> Result<String, String> {
    let c = normaliser(code).ok_or("code invalide (forme attendue : ABCD-1234)")?;
    let d = dossier_validations(donnees);
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    std::fs::write(d.join(&c), proprietaire.as_bytes()).map_err(|e| e.to_string())?;
    Ok(c)
}

pub enum Suivi {
    Inconnue,
    EnAttente(String),
    /// Le code a été validé : le propriétaire noté à la validation (vide si aucun).
    Validee { nom: String, proprietaire: Option<String> },
}

impl Appairages {
    /// Nouvelle demande : (numéro de demande, code). None si trop de demandes en attente.
    pub fn demander(&mut self, nom: &str) -> Option<(String, String)> {
        self.purger();
        if self.demandes.len() >= MAX_EN_ATTENTE {
            return None;
        }
        let mut code = nouveau_code();
        while self.demandes.values().any(|d| d.code == code) {
            code = nouveau_code();
        }
        let id = crate::aleatoire_hex(24);
        self.demandes.insert(id.clone(), Demande { code: code.clone(), nom: nom.to_string(), creee: Instant::now() });
        Some((id, code))
    }

    /// Où en est la demande ; une demande validée est retirée (la clé ne sort qu'une fois).
    pub fn suivre(&mut self, donnees: &Path, id: &str) -> Suivi {
        self.purger();
        let Some(d) = self.demandes.get(id) else { return Suivi::Inconnue };
        let f = dossier_validations(donnees).join(&d.code);
        match std::fs::read_to_string(&f) {
            Ok(p) => {
                let _ = std::fs::remove_file(&f);
                let d = self.demandes.remove(id).expect("demande présente");
                let p = p.trim().to_string();
                Suivi::Validee { nom: d.nom, proprietaire: (!p.is_empty()).then_some(p) }
            }
            Err(_) => Suivi::EnAttente(d.code.clone()),
        }
    }

    /// Code (normalisé) d'une demande encore en attente.
    pub fn code_en_attente(&mut self, code: &str) -> bool {
        self.purger();
        self.demandes.values().any(|d| d.code == code)
    }

    /// Retire les demandes expirées.
    pub fn purger(&mut self) {
        self.demandes.retain(|_, d| d.creee.elapsed() < DUREE);
    }

    pub fn en_attente(&self) -> usize {
        self.demandes.len()
    }
}

/// Efface les validations restées sans demande (plus vieilles que la durée d'une demande).
pub fn purger_validations(donnees: &Path) {
    let Ok(lecture) = std::fs::read_dir(dossier_validations(donnees)) else { return };
    for e in lecture.flatten() {
        let vieux = e.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_some_and(|age| age > DUREE);
        if vieux {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes() {
        for _ in 0..200 {
            let c = nouveau_code();
            assert_eq!(normaliser(&c).as_deref(), Some(c.as_str()));
            assert_eq!(normaliser(&afficher(&c).to_lowercase()).as_deref(), Some(c.as_str()));
        }
        assert_eq!(normaliser("KXRT-4821").as_deref(), Some("KXRT4821"));
        assert_eq!(normaliser("kxrt 4821").as_deref(), Some("KXRT4821"));
        assert!(normaliser("KXRO-4821").is_none());
        assert!(normaliser("KXRT-482").is_none());
        assert!(normaliser("../../etc").is_none());
        assert!(normaliser("ABDé123").is_none(), "pas de panique sur un caractère multi-octets");
    }

    #[test]
    fn demande_validee_une_seule_fois() {
        let d = std::env::temp_dir().join(format!("eko-cloud-app-{}", crate::aleatoire_hex(6)));
        let mut a = Appairages::default();
        let (id, code) = a.demander("Atelier").unwrap();
        assert!(matches!(a.suivre(&d, &id), Suivi::EnAttente(c) if c == code));
        valider(&d, &afficher(&code), "mathieu").unwrap();
        assert!(matches!(a.suivre(&d, &id), Suivi::Validee { ref nom, ref proprietaire } if nom == "Atelier" && proprietaire.as_deref() == Some("mathieu")));
        assert!(matches!(a.suivre(&d, &id), Suivi::Inconnue));
        assert!(matches!(a.suivre(&d, "autre"), Suivi::Inconnue));
        let _ = std::fs::remove_dir_all(&d);
    }
}
