//! Ping anonyme d'installation (protocole commun E-KO RIP) : un identifiant aléatoire, le type de poste, l'OS et la version.
//! Part dans un fil d'arrière-plan au lancement puis toutes les 6 h ; jamais d'erreur affichée, jamais de blocage de l'interface.
//! État dans `telemetrie.json` à côté des autres préférences.

use serde::{Deserialize, Serialize};
use std::io::Read;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const URL_PAR_DEFAUT: &str = "https://platform.e-ko.rip/api/ping";
const FICHIER: &str = "telemetrie.json";
const INTERVALLE: Duration = Duration::from_secs(6 * 3600);
const SEPT_JOURS: u64 = 7 * 24 * 3600;

/// Sérialise les lectures-modifications-écritures du fichier (fil d'envoi et case à cocher).
static VERROU: Mutex<()> = Mutex::new(());

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Etat {
    pub id: String,
    pub version_signalee: Option<String>,
    pub dernier_ping: u64,
    pub desactive: bool,
}

/// Quel événement envoyer maintenant, s'il y en a un.
pub fn evenement(etat: &Etat, version: &str, maintenant: u64, env_desactive: bool) -> Option<&'static str> {
    if env_desactive || etat.desactive {
        return None;
    }
    match etat.version_signalee.as_deref() {
        None | Some("") => Some("installation"),
        Some(v) if v != version => Some("mise_a_jour"),
        _ if maintenant.saturating_sub(etat.dernier_ping) >= SEPT_JOURS => Some("hebdo"),
        _ => None,
    }
}

fn env_desactive() -> bool {
    std::env::var("EKO_TELEMETRIE").is_ok_and(|v| matches!(v.trim().to_lowercase().as_str(), "0" | "non" | "false" | "off"))
}

fn maintenant() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

fn lire_etat() -> Etat {
    std::fs::read_to_string(crate::config::dossier().join(FICHIER)).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn ecrire_etat(etat: &Etat) {
    let d = crate::config::dossier();
    if std::fs::create_dir_all(&d).is_ok()
        && let Ok(s) = serde_json::to_string(etat)
    {
        let _ = std::fs::write(d.join(FICHIER), s);
    }
}

fn nouvel_id() -> Option<String> {
    let mut octets = [0u8; 16];
    std::fs::File::open("/dev/urandom").ok()?.read_exact(&mut octets).ok()?;
    Some(octets.iter().map(|o| format!("{o:02x}")).collect())
}

/// `PRETTY_NAME` de /etc/os-release (repli « Linux »), 64 caractères au plus.
pub(crate) fn os() -> String {
    let nom = std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|s| s.lines().find_map(|l| l.strip_prefix("PRETTY_NAME=").map(|v| v.trim().trim_matches(|c| c == '"' || c == '\'').trim().to_string())))
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "Linux".into());
    nom.chars().take(64).collect()
}

/// Réglage local : la case à cocher des préférences. `true` = statistiques envoyées.
pub fn active() -> bool {
    let _g = VERROU.lock().unwrap_or_else(|e| e.into_inner());
    !lire_etat().desactive
}

pub fn regler_active(active: bool) {
    let _g = VERROU.lock().unwrap_or_else(|e| e.into_inner());
    let mut etat = lire_etat();
    etat.desactive = !active;
    ecrire_etat(&etat);
}

fn essayer() {
    let version = env!("CARGO_PKG_VERSION");
    let (id, evt) = {
        let _g = VERROU.lock().unwrap_or_else(|e| e.into_inner());
        let mut etat = lire_etat();
        let Some(evt) = evenement(&etat, version, maintenant(), env_desactive()) else { return };
        if etat.id.len() != 32 {
            let Some(id) = nouvel_id() else { return };
            etat.id = id;
            ecrire_etat(&etat);
        }
        (etat.id, evt)
    };
    let url = std::env::var("EKO_TELEMETRIE_URL").ok().filter(|u| !u.trim().is_empty()).unwrap_or_else(|| URL_PAR_DEFAUT.into());
    let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(10)).build();
    let corps = serde_json::json!({ "id": id, "type": "client-linux", "evenement": evt, "version": version, "os": os() });
    if agent.post(&url).send_json(corps).is_ok_and(|r| r.status() == 204) {
        let _g = VERROU.lock().unwrap_or_else(|e| e.into_inner());
        let mut etat = lire_etat();
        etat.version_signalee = Some(version.to_string());
        etat.dernier_ping = maintenant();
        ecrire_etat(&etat);
    }
}

/// Lance le fil d'arrière-plan : un essai au démarrage, puis toutes les 6 h.
pub fn demarrer() {
    let _ = std::thread::Builder::new().name("telemetrie".into()).spawn(|| {
        loop {
            essayer();
            std::thread::sleep(INTERVALLE);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn etat(version: Option<&str>, dernier: u64, desactive: bool) -> Etat {
        Etat { id: "a".repeat(32), version_signalee: version.map(String::from), dernier_ping: dernier, desactive }
    }

    #[test]
    fn decisions() {
        let now = 10_000_000;
        assert_eq!(evenement(&etat(Some("0.29.0"), now, true), "0.29.0", now, false), None, "désactivé localement");
        assert_eq!(evenement(&etat(None, 0, false), "0.29.0", now, true), None, "désactivé par l'environnement");
        assert_eq!(evenement(&Etat::default(), "0.29.0", now, false), Some("installation"));
        assert_eq!(evenement(&etat(Some("0.28.0"), now, false), "0.29.0", now, false), Some("mise_a_jour"));
        assert_eq!(evenement(&etat(Some("0.29.0"), now - SEPT_JOURS, false), "0.29.0", now, false), Some("hebdo"));
        assert_eq!(evenement(&etat(Some("0.29.0"), now - SEPT_JOURS + 1, false), "0.29.0", now, false), None);
    }

    #[test]
    fn identifiant() {
        if let Some(id) = nouvel_id() {
            assert_eq!(id.len(), 32);
            assert!(id.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')));
        }
    }
}
