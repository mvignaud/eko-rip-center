//! Découverte des serveurs E-KO RIP du réseau local (DNS-SD / mDNS, `_eko-rip._tcp.local.`).
//! La page de choix du serveur appelle `chercher_serveurs` au chargement.

use mdns_sd::{ServiceDaemon, ServiceEvent};
use serde::Serialize;
use std::net::Ipv4Addr;
use std::time::{Duration, Instant};

const TYPE_SERVICE: &str = "_eko-rip._tcp.local.";
const DUREE: Duration = Duration::from_secs(3);

/// Un serveur trouvé : nom affiché, adresse `http://ip:port`, version annoncée (vide si inconnue).
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Serveur {
    pub nom: String,
    pub adresse: String,
    pub version: String,
}

/// Transforme un service résolu en serveur : adresse IPv4 (les adresses de lien local
/// 169.254.x.x ne passent qu'en dernier recours), nom tiré du TXT `nom` sinon du nom d'instance.
fn serveur_depuis(nom_complet: &str, ipv4: &[Ipv4Addr], port: u16, nom: Option<&str>, version: Option<&str>) -> Option<Serveur> {
    let mut adresses = ipv4.to_vec();
    adresses.sort_by_key(|a| (a.is_link_local(), *a));
    let ip = adresses.first()?;
    let nom = nom.map(str::trim).filter(|n| !n.is_empty()).map(String::from).unwrap_or_else(|| {
        let instance = nom_complet.strip_suffix(TYPE_SERVICE).map_or(nom_complet, |s| s.trim_end_matches('.'));
        instance.to_string()
    });
    Some(Serveur { nom, adresse: format!("http://{ip}:{port}"), version: version.unwrap_or_default().to_string() })
}

/// Écoute le réseau pendant ~3 s (bloquant) et rend les serveurs trouvés, triés par nom.
fn chercher() -> Vec<Serveur> {
    let Ok(demon) = ServiceDaemon::new() else { return Vec::new() };
    let Ok(recepteur) = demon.browse(TYPE_SERVICE) else {
        let _ = demon.shutdown();
        return Vec::new();
    };
    let fin = Instant::now() + DUREE;
    let mut trouves: Vec<Serveur> = Vec::new();
    while let Some(reste) = fin.checked_duration_since(Instant::now()) {
        match recepteur.recv_timeout(reste) {
            Ok(ServiceEvent::ServiceResolved(s)) => {
                let ipv4: Vec<Ipv4Addr> = s.get_addresses_v4().into_iter().collect();
                let trouve = serveur_depuis(s.get_fullname(), &ipv4, s.get_port(), s.get_property_val_str("nom"), s.get_property_val_str("version"));
                if let Some(t) = trouve.filter(|t| !trouves.iter().any(|d| d.adresse == t.adresse)) {
                    trouves.push(t);
                }
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }
    let _ = demon.stop_browse(TYPE_SERVICE);
    let _ = demon.shutdown();
    trouves.sort_by_key(|s| s.nom.to_lowercase());
    trouves
}

/// Commande Tauri : serveurs E-KO RIP du réseau local (liste vide si rien n'est trouvé).
#[tauri::command]
pub async fn chercher_serveurs() -> Vec<Serveur> {
    tauri::async_runtime::spawn_blocking(chercher).await.unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_vers_serveur() {
        let ip = Ipv4Addr::new(192, 168, 10, 109);
        let s = serveur_depuis("eko-rip._eko-rip._tcp.local.", &[ip], 8640, Some("Atelier"), Some("0.31.0")).unwrap();
        assert_eq!(s, Serveur { nom: "Atelier".into(), adresse: "http://192.168.10.109:8640".into(), version: "0.31.0".into() });
        // Sans TXT : nom d'instance, version vide.
        let s = serveur_depuis("eko-rip._eko-rip._tcp.local.", &[ip], 9000, None, None).unwrap();
        assert_eq!((s.nom.as_str(), s.adresse.as_str(), s.version.as_str()), ("eko-rip", "http://192.168.10.109:9000", ""));
        // Lien local en dernier recours ; sans IPv4, pas de serveur.
        let ll = Ipv4Addr::new(169, 254, 3, 4);
        assert_eq!(serveur_depuis("x._eko-rip._tcp.local.", &[ll, ip], 8640, None, None).unwrap().adresse, "http://192.168.10.109:8640");
        assert!(serveur_depuis("x._eko-rip._tcp.local.", &[], 8640, None, None).is_none());
    }
}
