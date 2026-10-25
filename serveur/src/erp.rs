//! Pont ERP ↔ RIP, contrat `eko-rip-erp/1` (lot R1) : accès machine à machine par jeton,
//! signature HMAC-SHA256 des événements sortants, modèle d'états publié.
//!
//! `erp.json` (droits 0600) garde l'empreinte SHA-256 du jeton (jamais le jeton) et le
//! secret HMAC (en clair : il sert à signer). Les deux se créent en ligne de commande
//! (`eko-rip-serveur erp jeton|secret`) et ne s'affichent qu'une fois.

use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Nom et version du contrat ; un changement incompatible passe à `/2`.
pub const CONTRAT: &str = "eko-rip-erp/1";
/// Préfixe des routes du pont : seules routes ouvertes au jeton, fermées à tout le reste.
pub const PREFIXE: &str = "/api/erp/";

#[derive(Default, Serialize, Deserialize)]
pub struct Config {
    /// Empreinte SHA-256 (hex) du jeton de l'ERP.
    #[serde(default)]
    pub jeton_sha256: Option<String>,
    /// Secret HMAC partagé avec l'ERP (64 caractères hex, utilisés tels quels comme clé).
    #[serde(default)]
    pub secret_hmac: Option<String>,
}

fn fichier(donnees: &Path) -> PathBuf {
    donnees.join("erp.json")
}

pub fn lire(donnees: &Path) -> Config {
    crate::etat::lire(&fichier(donnees)).ok().flatten().unwrap_or_default()
}

pub fn ecrire(donnees: &Path, c: &Config) -> std::io::Result<()> {
    crate::etat::ecrire(&fichier(donnees), c, true)
}

fn hex(o: &[u8]) -> String {
    o.iter().map(|b| format!("{b:02x}")).collect()
}

/// 32 octets aléatoires du système, en hex.
pub fn aleatoire() -> String {
    let mut o = [0u8; 32];
    getrandom::fill(&mut o).expect("générateur aléatoire du système");
    hex(&o)
}

pub fn empreinte(jeton: &str) -> String {
    hex(&Sha256::digest(jeton.as_bytes()))
}

/// Comparaison en temps constant (longueurs publiques).
fn egal(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Le jeton présenté (`Authorization: Bearer <jeton>`) est-il celui de l'ERP ?
pub fn jeton_valide(c: &Config, presente: &str) -> bool {
    c.jeton_sha256.as_deref().is_some_and(|attendu| egal(empreinte(presente).as_bytes(), attendu.as_bytes()))
}

// Utilisées par le transport des événements (lot R2).
#[cfg_attr(not(test), allow(dead_code))]
pub fn hmac_sha256(cle: &[u8], message: &[u8]) -> String {
    let mut m = <Hmac<Sha256> as KeyInit>::new_from_slice(cle).expect("toute longueur de clé convient");
    m.update(message);
    hex(&m.finalize().into_bytes())
}

/// En-tête `X-EKO-RIP-Signature` d'un événement : `t=<secondes Unix>,v1=<hex>`, où
/// `v1 = HMAC-SHA256(clé = secret tel quel en ASCII, message = "<t>.<corps>")` sur les
/// octets exacts du corps envoyé. Le destinataire refuse un `t` à plus de 5 min de son
/// horloge (rejeu) et compare en temps constant.
#[cfg_attr(not(test), allow(dead_code))]
pub fn signature(secret: &str, t: u64, corps: &[u8]) -> String {
    let mut message = format!("{t}.").into_bytes();
    message.extend_from_slice(corps);
    format!("t={t},v1={}", hmac_sha256(secret.as_bytes(), &message))
}

/// Enveloppe d'un événement sortant (le transport arrive avec le lot R2).
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Evenement {
    pub contrat: String,
    /// Installation du pont (UUID tiré au premier démarrage) : l'ERP compte `seq` par instance.
    pub instance: uuid::Uuid,
    /// Numéro croissant sans trou, par instance : l'ERP rejoue « depuis N ».
    pub seq: u64,
    pub id: uuid::Uuid,
    /// `catalogue.modifie`, `travail.etat`, `sortie.etat`, `travail.consommation`…
    #[serde(rename = "type")]
    pub genre: String,
    pub le: u64,
    pub donnees: serde_json::Value,
}

/// Modèle d'états publié (lot R1.3) : états, sens, et états après une coupure.
pub fn modele_etats() -> serde_json::Value {
    serde_json::json!({
        "travail": {
            "etats": {
                "en_attente": "reçu, en attente du rendu",
                "en_cours": "rendu (RIP) en cours",
                "termine": "rendu terminé : prêt à sortir",
                "echec": "rendu impossible (code d'erreur stable)"
            },
            "apres_coupure": "un travail en_cours redevient en_attente et son rendu reprend"
        },
        "sortie": {
            "etats": {
                "en_attente": "en file vers une machine",
                "suspendue": "retenue par un opérateur",
                "en_cours": "envoi à la machine en cours",
                "terminee": "envoyée à la machine (impression ou découpe)",
                "echec": "envoi impossible (code d'erreur stable)",
                "annulee": "retirée avant envoi"
            },
            "apres_coupure": "une sortie en_cours passe en echec avec le code interrompu ; elle ne repart pas seule (une découpe ne se reprend pas au milieu)"
        },
        "a_venir": "en_impression et imprimee viendront du pilote direct de l'Epson (lot R5)"
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_vecteur_rfc_4231() {
        // RFC 4231, cas 2.
        assert_eq!(hmac_sha256(b"Jefe", b"what do ya want for nothing?"), "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843");
        // Signature : le message signé est « t.corps ».
        assert_eq!(signature("Jefe", 1, b"{}"), format!("t=1,v1={}", hmac_sha256(b"Jefe", b"1.{}")));
    }

    #[test]
    fn jeton() {
        let j = aleatoire();
        assert_eq!(j.len(), 64);
        let c = Config { jeton_sha256: Some(empreinte(&j)), secret_hmac: None };
        assert!(jeton_valide(&c, &j));
        assert!(!jeton_valide(&c, "autre"));
        assert!(!jeton_valide(&Config::default(), &j), "aucun jeton déclaré : tout est refusé");
    }
}
