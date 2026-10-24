//! Contrôle d'accès : utilisateurs (mots de passe hachés en Argon2id), sessions par
//! cookie, blocage après échecs répétés. Sans utilisateur déclaré, l'accès reste libre
//! (installations existantes) et l'écran le signale.

use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const COOKIE: &str = "eko_rip_session";
pub const DUREE_SESSION: Duration = Duration::from_secs(12 * 3600);
const ECHECS_MAX: u32 = 5;
const BLOCAGE: Duration = Duration::from_secs(5 * 60);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Utilisateur {
    pub nom: String,
    /// Chaîne PHC Argon2id (`$argon2id$v=19$…`) ; jamais le mot de passe.
    pub hachage: String,
    /// Absent des comptes d'avant les rôles : administrateur, comme ils l'étaient.
    #[serde(default)]
    pub role: Role,
}

/// L'opérateur imprime et découpe ; l'administrateur règle aussi le serveur (machines,
/// tarifs, profils ICC, réglages enregistrés).
#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    #[default]
    Admin,
    Operateur,
}

/// Écritures réservées à l'administrateur.
pub fn reserve_admin(methode: &axum::http::Method, chemin: &str) -> bool {
    // La sauvegarde porte les prix : même sa lecture est réservée.
    chemin == "/api/sauvegarde" || *methode != axum::http::Method::GET && (["/api/machines", "/api/tarifs", "/api/profils", "/api/reglages"].contains(&chemin) || chemin.starts_with("/api/reglages/"))
}

pub fn fichier(donnees: &Path) -> PathBuf {
    donnees.join("utilisateurs.json")
}

pub fn lire(donnees: &Path) -> Vec<Utilisateur> {
    crate::etat::lire(&fichier(donnees)).ok().flatten().unwrap_or_default()
}

/// Accès libre seulement quand aucun compte n'est déclaré : fichier absent ou liste vide.
/// Un fichier illisible (coupure) ne l'ouvre JAMAIS.
pub fn libre(donnees: &Path) -> bool {
    match crate::etat::lire::<Vec<Utilisateur>>(&fichier(donnees)) {
        Ok(None) => true,
        Ok(Some(v)) => v.is_empty(),
        Err(()) => false,
    }
}

/// Écrit la liste avec des droits 0600 (fichier temporaire puis renommage).
pub fn ecrire(donnees: &Path, utilisateurs: &[Utilisateur]) -> std::io::Result<()> {
    crate::etat::ecrire(&fichier(donnees), utilisateurs, true)
}

pub fn nom_valide(nom: &str) -> bool {
    (1..=40).contains(&nom.len()) && nom.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

pub fn hacher(mot_de_passe: &str) -> Result<String, String> {
    let mut sel = [0u8; 16];
    getrandom::fill(&mut sel).map_err(|e| e.to_string())?;
    Argon2::default().hash_password_with_salt(mot_de_passe.as_bytes(), &sel).map(|h| h.to_string()).map_err(|e| e.to_string())
}

pub fn verifier_mot_de_passe(mot_de_passe: &str, hachage: &str) -> bool {
    Argon2::default().verify_password(mot_de_passe.as_bytes(), hachage).is_ok()
}

/// Jeton de session : 32 octets aléatoires en hexadécimal.
fn jeton() -> String {
    let mut o = [0u8; 32];
    getrandom::fill(&mut o).expect("générateur aléatoire du système");
    o.iter().map(|b| format!("{b:02x}")).collect()
}

#[derive(Debug, PartialEq)]
pub enum Refus {
    /// Nom ou mot de passe faux (sans dire lequel).
    Identifiants,
    /// Trop d'échecs pour ce nom : réessayer plus tard.
    Bloque,
}

/// Sessions ouvertes et échecs de connexion, en mémoire (perdus au redémarrage).
#[derive(Default)]
pub struct Sessions {
    ouvertes: HashMap<String, (String, Instant)>,
    echecs: HashMap<String, (u32, Instant)>,
}

impl Sessions {
    /// Vérifie le couple ; rend un jeton de session. `maintenant` pour les tests.
    pub fn connecter(&mut self, utilisateurs: &[Utilisateur], nom: &str, mot_de_passe: &str, maintenant: Instant) -> Result<String, Refus> {
        if let Some((n, depuis)) = self.echecs.get(nom)
            && *n >= ECHECS_MAX
            && maintenant.duration_since(*depuis) < BLOCAGE
        {
            return Err(Refus::Bloque);
        }
        let connu = utilisateurs.iter().find(|u| u.nom == nom);
        // Un nom inconnu coûte aussi un hachage : pas de différence de temps qui le trahisse.
        let bon = match connu {
            Some(u) => verifier_mot_de_passe(mot_de_passe, &u.hachage),
            None => {
                let _ = hacher(mot_de_passe);
                false
            }
        };
        if !bon {
            let e = self.echecs.entry(nom.to_string()).or_insert((0, maintenant));
            if maintenant.duration_since(e.1) >= BLOCAGE {
                e.0 = 0;
            }
            e.0 += 1;
            e.1 = maintenant;
            return Err(Refus::Identifiants);
        }
        self.echecs.remove(nom);
        self.ouvertes.retain(|_, (_, debut)| maintenant.duration_since(*debut) < DUREE_SESSION);
        let j = jeton();
        self.ouvertes.insert(j.clone(), (nom.to_string(), maintenant));
        Ok(j)
    }

    /// Nom de l'utilisateur de la session, si elle est valide.
    pub fn utilisateur(&self, jeton: &str, maintenant: Instant) -> Option<String> {
        let (nom, debut) = self.ouvertes.get(jeton)?;
        (maintenant.duration_since(*debut) < DUREE_SESSION).then(|| nom.clone())
    }

    pub fn deconnecter(&mut self, jeton: &str) {
        self.ouvertes.remove(jeton);
    }
}

/// Valeur du cookie de session dans un en-tête `Cookie`.
pub fn jeton_du_cookie(entete: &str) -> Option<&str> {
    entete.split(';').map(str::trim).find_map(|c| c.strip_prefix(COOKIE)?.strip_prefix('='))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mathieu() -> Vec<Utilisateur> {
        vec![Utilisateur { role: Role::Admin, nom: "mathieu".into(), hachage: hacher("bonne-phrase-de-passe").unwrap() }]
    }

    #[test]
    fn hachage_argon2() {
        let h = hacher("secret").unwrap();
        assert!(h.starts_with("$argon2id$"), "{h}");
        assert!(!h.contains("secret"));
        assert!(verifier_mot_de_passe("secret", &h));
        assert!(!verifier_mot_de_passe("Secret", &h));
        assert!(!verifier_mot_de_passe("secret", "pas-un-hachage"));
        assert_ne!(hacher("secret").unwrap(), h, "sel aléatoire");
    }

    #[test]
    fn connexion_session_et_expiration() {
        let (u, t0) = (mathieu(), Instant::now());
        let mut s = Sessions::default();
        assert_eq!(s.connecter(&u, "mathieu", "faux", t0), Err(Refus::Identifiants));
        assert_eq!(s.connecter(&u, "inconnu", "bonne-phrase-de-passe", t0), Err(Refus::Identifiants));
        let j = s.connecter(&u, "mathieu", "bonne-phrase-de-passe", t0).unwrap();
        assert_eq!(j.len(), 64);
        assert_eq!(s.utilisateur(&j, t0 + Duration::from_secs(60)), Some("mathieu".into()));
        assert_eq!(s.utilisateur(&j, t0 + DUREE_SESSION + Duration::from_secs(1)), None, "expirée");
        assert_eq!(s.utilisateur("jeton-invente", t0), None);
        s.deconnecter(&j);
        assert_eq!(s.utilisateur(&j, t0), None);
    }

    #[test]
    fn blocage_apres_cinq_echecs() {
        let (u, t0) = (mathieu(), Instant::now());
        let mut s = Sessions::default();
        for _ in 0..5 {
            assert_eq!(s.connecter(&u, "mathieu", "faux", t0), Err(Refus::Identifiants));
        }
        assert_eq!(s.connecter(&u, "mathieu", "bonne-phrase-de-passe", t0), Err(Refus::Bloque), "même le bon mot de passe");
        assert!(s.connecter(&u, "mathieu", "bonne-phrase-de-passe", t0 + BLOCAGE).is_ok(), "débloqué après 5 min");
    }

    #[test]
    fn cookie_et_noms() {
        assert_eq!(jeton_du_cookie("a=1; eko_rip_session=abc; b=2"), Some("abc"));
        assert_eq!(jeton_du_cookie("eko_rip_session_x=abc"), None);
        assert!(nom_valide("mathieu.v") && !nom_valide("ma thieu") && !nom_valide(""));
    }
}
