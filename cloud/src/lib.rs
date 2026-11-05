//! E-KO RIP Cloud : un serveur RIP d'atelier (derrière une box, sans port ouvert) ouvre
//! lui-même une connexion sortante vers la plate-forme ; la plate-forme fait passer par
//! elle les requêtes des postes et de l'app mobile qui sont hors de l'atelier.
//!
//! Le tunnel : une requête HTTP/1.1 `GET /tunnel` avec `Upgrade: websocket` (sous-protocole
//! `eko-tunnel/1`), puis, dans les messages binaires du WebSocket, de l'HTTP/2 où les rôles
//! sont inversés : la plate-forme est le client, le serveur RIP sert sa propre API.
//! Plusieurs requêtes à la fois, fichiers en flux, aucun format maison. Le WebSocket ne sert
//! qu'à traverser les frontaux (LiteSpeed, Passenger…) qui refusent tout autre `Upgrade`.
//! La plate-forme accepte encore `Upgrade: eko-tunnel/1` seul (serveurs RIP 0.30).

pub mod abonnements;
pub mod appairage;
pub mod applis;
pub mod base;
pub mod client;
pub mod courriels;
pub mod demandes;
pub mod erp;
pub mod evenements;
pub mod installations;
pub mod langues;
pub mod montant;
pub mod notifications;
pub mod offres;
pub mod portail;
pub mod rapports;
pub mod s3;
pub mod sauvegardes;
pub mod relais;
pub mod sites;
pub mod support;
pub mod temps;
pub mod webpush;
pub mod ws;

/// Sous-protocole WebSocket du tunnel (et ancienne valeur de l'en-tête `Upgrade`).
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

/// Hexadécimal minuscule.
pub fn hex(octets: &[u8]) -> String {
    octets.iter().map(|o| format!("{o:02x}")).collect()
}

/// Empreinte SHA-256 d'octets, en hexadécimal.
pub fn sha256_octets_hex(octets: &[u8]) -> String {
    use sha2::Digest;
    hex(&sha2::Sha256::digest(octets))
}

/// HMAC-SHA256 (webhooks de l'ERP, commandes de restauration, signature AWS).
pub fn hmac_sha256(cle: &[u8], message: &[u8]) -> Vec<u8> {
    use hmac::{KeyInit, Mac};
    let mut m = hmac::Hmac::<sha2::Sha256>::new_from_slice(cle).expect("HMAC accepte toute clé");
    m.update(message);
    m.finalize().into_bytes().to_vec()
}

/// Base64 « URL » sans remplissage (RFC 4648 § 5), pour PKCE et l'en-tête d'identité.
pub fn base64url(octets: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut s = String::with_capacity(octets.len().div_ceil(3) * 4);
    for bloc in octets.chunks(3) {
        let n = (bloc[0] as u32) << 16 | (*bloc.get(1).unwrap_or(&0) as u32) << 8 | *bloc.get(2).unwrap_or(&0) as u32;
        for i in 0..=bloc.len() {
            s.push(A[(n >> (18 - 6 * i) & 63) as usize] as char);
        }
    }
    s
}

/// Inverse de `base64url` ; None si le texte n'en est pas.
pub fn base64url_decoder(texte: &str) -> Option<Vec<u8>> {
    let val = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            _ => return None,
        } as u32)
    };
    if texte.len() % 4 == 1 {
        return None;
    }
    let mut v = Vec::with_capacity(texte.len() * 3 / 4);
    for bloc in texte.as_bytes().chunks(4) {
        let mut n = 0u32;
        for (i, &c) in bloc.iter().enumerate() {
            n |= val(c)? << (18 - 6 * i);
        }
        for i in 0..bloc.len() - 1 {
            v.push((n >> (16 - 8 * i)) as u8);
        }
    }
    Some(v)
}

/// Comparaison en temps constant (longueurs égales attendues : empreintes).
pub fn egal_constant(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

pub fn maintenant() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}
