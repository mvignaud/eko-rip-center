//! Annonce du serveur sur le réseau local (DNS-SD / mDNS, `_eko-rip._tcp.local.`) : les clients
//! le trouvent seuls, sans saisir d'adresse. Jamais bloquant ni fatal : en cas d'échec, une ligne
//! dans le journal et le serveur démarre quand même. Désactivable par `EKO_RIP_ANNONCE=0`.

use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::net::SocketAddr;

/// Type de service recherché par les clients.
pub const TYPE_SERVICE: &str = "_eko-rip._tcp.local.";

/// Port à annoncer pour l'adresse d'écoute, ou rien si elle est illisible ou limitée à la boucle locale.
fn port_a_annoncer(ecoute: &str) -> Option<u16> {
    let a: SocketAddr = ecoute.parse().ok()?;
    (!a.ip().is_loopback()).then_some(a.port())
}

/// Nom du serveur : `EKO_RIP_CLOUD_NOM` s'il est réglé, sinon le nom de la machine.
fn nom_serveur() -> String {
    let brut = std::env::var("EKO_RIP_CLOUD_NOM")
        .ok()
        .filter(|n| !n.trim().is_empty())
        .or_else(|| std::fs::read_to_string("/etc/hostname").ok())
        .or_else(|| std::env::var("HOSTNAME").ok())
        .or_else(|| std::env::var("COMPUTERNAME").ok())
        .unwrap_or_default();
    let nom: String = brut.trim().chars().filter(|c| !c.is_control()).take(60).collect();
    if nom.is_empty() { "Serveur E-KO RIP".into() } else { nom }
}

/// Nom d'hôte DNS (`xxx.local.`) tiré du nom du serveur : lettres, chiffres et tirets seulement.
fn hote_mdns(nom: &str) -> String {
    let etiquette: String = nom.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect();
    let etiquette = etiquette.trim_matches('-');
    format!("{}.local.", if etiquette.is_empty() { "eko-rip" } else { etiquette })
}

/// Lance l'annonce dans un fil à part (la création du démon mDNS ouvre des sockets). Le fil reste
/// en vie tant que le serveur tourne : l'annonce disparaît avec lui.
pub fn demarrer(ecoute: &str) {
    if std::env::var("EKO_RIP_ANNONCE").is_ok_and(|v| v.trim() == "0") {
        return;
    }
    let Some(port) = port_a_annoncer(ecoute) else {
        return;
    };
    let lancement = std::thread::Builder::new().name("annonce".into()).spawn(move || {
        let nom = nom_serveur();
        match enregistrer(&nom, port) {
            Ok(_demon) => loop {
                // Le démon (et son fil) vit tant que ce fil le garde.
                std::thread::park();
            },
            Err(e) => eprintln!("Annonce sur le réseau local impossible : {e}"),
        }
    });
    if let Err(e) = lancement {
        eprintln!("Annonce sur le réseau local impossible : {e}");
    }
}

fn enregistrer(nom: &str, port: u16) -> Result<ServiceDaemon, mdns_sd::Error> {
    let demon = ServiceDaemon::new()?;
    let proprietes = [("version", env!("CARGO_PKG_VERSION")), ("nom", nom)];
    // Adresse vide + `enable_addr_auto` : les adresses des interfaces sont suivies automatiquement.
    let service = ServiceInfo::new(TYPE_SERVICE, nom, &hote_mdns(nom), "", port, &proprietes[..])?.enable_addr_auto();
    demon.register(service)?;
    println!("Annonce du serveur sur le réseau local : {nom} (_eko-rip._tcp, port {port})");
    Ok(demon)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_annonce() {
        assert_eq!(port_a_annoncer("0.0.0.0:8640"), Some(8640));
        assert_eq!(port_a_annoncer("192.168.10.109:9000"), Some(9000));
        assert_eq!(port_a_annoncer("[::]:8640"), Some(8640));
        assert_eq!(port_a_annoncer("127.0.0.1:8640"), None);
        assert_eq!(port_a_annoncer("[::1]:8640"), None);
        assert_eq!(port_a_annoncer("localhost:8640"), None);
    }

    #[test]
    fn hote() {
        assert_eq!(hote_mdns("eko-rip"), "eko-rip.local.");
        assert_eq!(hote_mdns("Atelier E-KO (RIP)"), "atelier-e-ko--rip.local.");
        assert_eq!(hote_mdns("  "), "eko-rip.local.");
    }
}
