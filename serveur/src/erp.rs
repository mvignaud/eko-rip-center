//! Pont ERP ↔ RIP, contrat `eko-rip-erp/1` (lot R1) : accès machine à machine par jeton,
//! signature HMAC-SHA256 des événements sortants, modèle d'états publié.
//!
//! `erp.json` (droits 0600) garde l'empreinte SHA-256 du jeton (jamais le jeton) et le
//! secret HMAC (en clair : il sert à signer). Les deux se créent en ligne de commande
//! (`eko-rip-serveur erp jeton|secret`) et ne s'affichent qu'une fois ; le secret se saisit
//! aussi dans Système (jamais réaffiché). Lot R2 : URL de base de l'ERP, activation du pont
//! et instance (UUID tiré au premier démarrage, repris par la sauvegarde).

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
    /// URL de base de l'ERP pour cet atelier (lot R2), sans `/` final ; le pont n'y ajoute
    /// que `/evenements` et `/en-attente`.
    #[serde(default)]
    pub url: Option<String>,
    /// Pont activé par l'administrateur : événements numérotés et envois.
    #[serde(default)]
    pub actif: bool,
    /// Instance du pont (UUID v4) : l'ERP compte `seq` par instance. Tirée au premier
    /// démarrage, retirée seulement par l'administrateur (installation remplacée).
    #[serde(default)]
    pub instance: Option<uuid::Uuid>,
}

/// Ce que la sauvegarde de la configuration reprend du pont : ni le jeton ni le secret
/// (la sauvegarde se télécharge ; le secret ne quitte jamais le serveur).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Sauve {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub actif: bool,
    #[serde(default)]
    pub instance: Option<uuid::Uuid>,
}

impl Config {
    pub fn sauve(&self) -> Sauve {
        Sauve { url: self.url.clone(), actif: self.actif, instance: self.instance }
    }
}

/// URL de base admise : `https://hôte/chemin`, sans requête ni fragment ; `http://` seulement
/// vers la machine elle-même (`localhost`, `127.0.0.1`, `[::1]` : essais, faux ERP local).
/// Rend l'URL sans `/` final.
pub fn url_valide(texte: &str) -> Option<String> {
    let t = texte.trim().trim_end_matches('/');
    let uri: axum::http::Uri = t.parse().ok()?;
    let hote = uri.host()?;
    let locale = ["localhost", "127.0.0.1", "[::1]", "::1"].contains(&hote);
    let schema_bon = match uri.scheme_str()? {
        "https" => true,
        "http" => locale,
        _ => false,
    };
    (schema_bon && uri.query().is_none() && !t.contains('#') && !t.contains(char::is_whitespace) && t.len() <= 500).then(|| t.to_string())
}

/// Secret saisi à la main : 32 à 256 caractères ASCII visibles (le secret du serveur en fait 64).
pub fn secret_valide(texte: &str) -> bool {
    (32..=256).contains(&texte.len()) && texte.bytes().all(|b| b.is_ascii_graphic())
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

/// Tire l'instance du pont au premier démarrage. Un `erp.json` illisible n'est pas réécrit.
pub fn assurer_instance(donnees: &Path) {
    let Ok(lu) = crate::etat::lire::<Config>(&fichier(donnees)) else { return };
    let mut c = lu.unwrap_or_default();
    if c.instance.is_none() {
        c.instance = Some(uuid::Uuid::new_v4());
        if let Err(e) = std::fs::create_dir_all(donnees).and_then(|()| ecrire(donnees, &c)) {
            eprintln!("erp.json : {e}");
        }
    }
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
pub(crate) fn egal(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Le jeton présenté (`Authorization: Bearer <jeton>`) est-il celui de l'ERP ?
pub fn jeton_valide(c: &Config, presente: &str) -> bool {
    c.jeton_sha256.as_deref().is_some_and(|attendu| egal(empreinte(presente).as_bytes(), attendu.as_bytes()))
}

pub fn hmac_sha256(cle: &[u8], message: &[u8]) -> String {
    let mut m = <Hmac<Sha256> as KeyInit>::new_from_slice(cle).expect("toute longueur de clé convient");
    m.update(message);
    hex(&m.finalize().into_bytes())
}

/// En-tête `X-EKO-RIP-Signature` d'un événement : `t=<secondes Unix>,v1=<hex>`, où
/// `v1 = HMAC-SHA256(clé = secret tel quel en ASCII, message = "<t>.<corps>")` sur les
/// octets exacts du corps envoyé. Le destinataire refuse un `t` à plus de 5 min de son
/// horloge (rejeu) et compare en temps constant.
pub fn signature(secret: &str, t: u64, corps: &[u8]) -> String {
    let mut message = format!("{t}.").into_bytes();
    message.extend_from_slice(corps);
    format!("t={t},v1={}", hmac_sha256(secret.as_bytes(), &message))
}

/// Signature d'une requête sans corps (GET), côté ERP : message `"<t>.GET <chemin?requête>"`,
/// le chemin tel qu'envoyé dans la ligne de requête, sans l'hôte.
pub fn signature_get(secret: &str, t: u64, chemin_et_requete: &str) -> String {
    signature(secret, t, format!("GET {chemin_et_requete}").as_bytes())
}

/// Enveloppe d'un événement sortant, une ligne du journal du pont (`pont.rs`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
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
            "apres_coupure": "un travail en_cours passe en echec (code interrompu) ; à relancer"
        },
        "sortie": {
            "etats": {
                "en_attente": "en file vers une machine",
                "suspendue": "retenue par un opérateur",
                "en_cours": "envoi à la machine en cours",
                "terminee": "envoyée à la machine (impression ou découpe)",
                "echec": "envoi impossible (code d'erreur stable)",
                "annulee": "retirée avant envoi",
                "imbriquee": "posée sur une planche d'imbrication : la planche (un autre travail) l'imprime ; terminee quand la planche est imprimée"
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
        let c = Config { jeton_sha256: Some(empreinte(&j)), ..Default::default() };
        assert!(jeton_valide(&c, &j));
        assert!(!jeton_valide(&c, "autre"));
        assert!(!jeton_valide(&Config::default(), &j), "aucun jeton déclaré : tout est refusé");
    }

    #[test]
    fn exemples_signes_du_document_de_l_erp() {
        // § 6 du contrat côté ERP (secret et horodatage d'exemple, aucun secret réel).
        let secret = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let corps = r#"{"contrat":"eko-rip-erp/1","seq":1,"id":"6f1c0e8e-1b8e-4b9a-9d5e-2f6f3c1a7b10","type":"catalogue.modifie","le":1793000000,"donnees":{"serveur":"0.18.0","machines":[{"uuid":"3f1c0e8e-1b8e-4b9a-9d5e-2f6f3c1a7b10","id":"epson","nom":"Epson SC-S80600","type":"rouleau","laize_mm":1626}]}}"#;
        assert_eq!(signature(secret, 1793000000, corps.as_bytes()), "t=1793000000,v1=8daa0d9b09c96fdf0cbcd11689e21f3e5199a35cdc50a0245c8bbf0348e50b04");
        assert_eq!(signature_get(secret, 1793000000, "/api/v1/gestion-imprimerie/rip/goldenprint/en-attente"), "t=1793000000,v1=1d5fcee0fd5ab7829b5cc2407d7833d2766637d3ba93ba425fdf1b9688a909b4");
    }

    #[test]
    fn url_de_base() {
        assert_eq!(url_valide("https://erp.exemple.test/api/v1/rip/atelier/").as_deref(), Some("https://erp.exemple.test/api/v1/rip/atelier"));
        assert_eq!(url_valide("http://127.0.0.1:9000/rip").as_deref(), Some("http://127.0.0.1:9000/rip"));
        for faux in ["http://erp.exemple.test/rip", "ftp://erp.exemple.test", "https://erp.exemple.test/rip?x=1", "https://erp.exemple.test/r ip", "erp.exemple.test", ""] {
            assert_eq!(url_valide(faux), None, "{faux}");
        }
        assert!(secret_valide(&aleatoire()));
        assert!(!secret_valide("court") && !secret_valide(&format!("{} x", aleatoire())));
    }
}
