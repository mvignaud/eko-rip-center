//! E-KO RIP Cloud : un serveur RIP d'atelier (derrière une box, sans port ouvert) ouvre
//! lui-même une connexion sortante vers la plate-forme ; la plate-forme fait passer par
//! elle les requêtes des postes et de l'app mobile qui sont hors de l'atelier.
//!
//! Le tunnel : une requête HTTP/1.1 `GET /tunnel` avec `Upgrade: eko-tunnel/1`, puis, dans
//! la connexion retournée, de l'HTTP/2 où les rôles sont inversés : la plate-forme est le
//! client, le serveur RIP sert sa propre API. Plusieurs requêtes à la fois, fichiers en
//! flux, aucun format maison.

pub mod appairage;
pub mod base;
pub mod client;
pub mod installations;
pub mod portail;
pub mod relais;
pub mod sites;

/// Valeur de l'en-tête `Upgrade` du tunnel.
pub const PROTOCOLE: &str = "eko-tunnel/1";
/// Chemin du tunnel sur la plate-forme.
pub const CHEMIN_TUNNEL: &str = "/tunnel";
/// Seul cookie que la plate-forme laisse passer vers un serveur RIP (sa session locale).
/// Les autres cookies du domaine (ceux du portail) ne sortent jamais vers un atelier.
pub const COOKIE_RIP: &str = "eko_rip_session";

/// Jeton aléatoire en hexadécimal (`octets` octets).
pub fn aleatoire_hex(octets: usize) -> String {
    let mut b = vec![0u8; octets];
    getrandom::fill(&mut b).expect("générateur aléatoire");
    b.iter().map(|o| format!("{o:02x}")).collect()
}

/// Empreinte SHA-256 en hexadécimal.
pub fn sha256_hex(texte: &str) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(texte.as_bytes()).iter().map(|o| format!("{o:02x}")).collect()
}

/// Comparaison en temps constant (longueurs égales attendues : empreintes).
pub fn egal_constant(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

pub fn maintenant() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}
