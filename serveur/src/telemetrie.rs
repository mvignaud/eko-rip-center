//! Ping anonyme d'installation vers la plate-forme E-KO RIP (`POST <plate-forme>/api/ping`),
//! pour compter les installations, y compris les serveurs jamais rattachés au cloud.
//!
//! Part à l'installation, à chaque mise à jour (nouvelle version vue au démarrage), puis une
//! fois par semaine. Contenu : un identifiant tiré au hasard sur ce serveur, le type
//! (`serveur`), l'OS, la version, et le nombre de machines et de réglages. Ni adresse, ni nom
//! d'atelier ou de machine, ni fichier.
//!
//! Désactivation : `EKO_TELEMETRIE=0` (dans `/etc/default/eko-rip-serveur`) ou la case de
//! Système (`telemetrie.json`, `desactive`). Un échec n'a aucun effet visible : nouvel essai
//! à l'heure suivante.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const SEMAINE_S: u64 = 7 * 86_400;
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Etat {
    pub id: String,
    #[serde(default)]
    pub version_signalee: Option<String>,
    #[serde(default)]
    pub dernier_ping: Option<u64>,
    #[serde(default)]
    pub desactive: bool,
}

pub fn fichier(donnees: &Path) -> PathBuf {
    donnees.join("telemetrie.json")
}

/// État en place, ou un nouvel identifiant (écrit tout de suite, pour rester le même).
pub fn lire(donnees: &Path) -> Etat {
    if let Ok(Some(e)) = crate::etat::lire::<Etat>(&fichier(donnees))
        && e.id.len() == 32
    {
        return e;
    }
    let mut o = [0u8; 16];
    getrandom::fill(&mut o).expect("générateur aléatoire");
    let e = Etat { id: o.iter().map(|b| format!("{b:02x}")).collect(), ..Default::default() };
    let _ = ecrire(donnees, &e);
    e
}

pub fn ecrire(donnees: &Path, e: &Etat) -> std::io::Result<()> {
    crate::etat::ecrire(&fichier(donnees), e, false)
}

/// `EKO_TELEMETRIE=0|non|false|off` : aucun ping.
pub fn desactive_par_env() -> bool {
    std::env::var("EKO_TELEMETRIE").is_ok_and(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "0" | "non" | "false" | "off" | "no"))
}

pub fn url() -> String {
    std::env::var("EKO_TELEMETRIE_URL").ok().filter(|v| !v.is_empty()).unwrap_or_else(|| format!("{}/api/ping", crate::cloud::url_plateforme()))
}

/// Ce qu'il faut envoyer maintenant, s'il faut envoyer quelque chose.
pub fn evenement(e: &Etat, version: &str, maintenant: u64, par_env: bool) -> Option<&'static str> {
    if par_env || e.desactive {
        return None;
    }
    match (&e.version_signalee, e.dernier_ping) {
        (None, _) => Some("installation"),
        (Some(v), _) if v != version => Some("mise_a_jour"),
        (_, Some(t)) if maintenant.saturating_sub(t) < SEMAINE_S => None,
        _ => Some("hebdo"),
    }
}

/// `PRETTY_NAME` de `/etc/os-release` (« Ubuntu 26.04 LTS »), sinon le nom du système.
pub fn os() -> String {
    let nom = std::fs::read_to_string("/etc/os-release").ok().and_then(|t| t.lines().find_map(|l| l.strip_prefix("PRETTY_NAME=").map(|v| v.trim_matches('"').trim().to_string()))).filter(|v| !v.is_empty());
    nom.unwrap_or_else(|| std::env::consts::OS.to_string()).chars().filter(|c| !c.is_control()).take(64).collect()
}

/// Un essai (bloquant). `compter` rend (machines, réglages).
pub fn essayer(donnees: &Path, compter: impl Fn() -> (usize, usize)) {
    let mut e = lire(donnees);
    let maintenant = crate::maintenant();
    let Some(evenement) = evenement(&e, VERSION, maintenant, desactive_par_env()) else { return };
    let (machines, reglages) = compter();
    let corps = serde_json::json!({ "id": e.id, "type": "serveur", "evenement": evenement, "version": VERSION, "os": os(), "machines": machines, "reglages": reglages });
    let agent: ureq::Agent = ureq::Agent::config_builder().http_status_as_error(false).timeout_global(Some(Duration::from_secs(10))).user_agent(format!("eko-rip-serveur/{VERSION}")).build().into();
    if agent.post(url()).send_json(corps).is_ok_and(|r| r.status().as_u16() == 204) {
        e.version_signalee = Some(VERSION.into());
        e.dernier_ping = Some(maintenant);
        let _ = ecrire(donnees, &e);
    }
}

/// Premier essai une minute après le démarrage (réseau prêt), puis toutes les heures.
pub async fn boucle(donnees: PathBuf, compter: impl Fn() -> (usize, usize) + Clone + Send + 'static) {
    tokio::time::sleep(Duration::from_secs(60)).await;
    loop {
        let (d, c) = (donnees.clone(), compter.clone());
        let _ = tokio::task::spawn_blocking(move || essayer(&d, c)).await;
        tokio::time::sleep(Duration::from_secs(3_600)).await;
    }
}

/// Pour l'écran Système.
pub fn etat_public(donnees: &Path) -> serde_json::Value {
    let e = lire(donnees);
    serde_json::json!({ "actif": !e.desactive && !desactive_par_env(), "desactive_par_env": desactive_par_env(), "id": e.id, "dernier_ping": e.dernier_ping })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quand_envoyer() {
        let neuf = Etat { id: "a".repeat(32), ..Default::default() };
        assert_eq!(evenement(&neuf, "0.30.0", 1_000, false), Some("installation"));
        assert_eq!(evenement(&neuf, "0.30.0", 1_000, true), None);
        assert_eq!(evenement(&Etat { desactive: true, ..neuf.clone() }, "0.30.0", 1_000, false), None);
        let signale = Etat { version_signalee: Some("0.30.0".into()), dernier_ping: Some(1_000), ..neuf };
        assert_eq!(evenement(&signale, "0.31.0", 1_100, false), Some("mise_a_jour"));
        assert_eq!(evenement(&signale, "0.30.0", 1_000 + SEMAINE_S - 1, false), None);
        assert_eq!(evenement(&signale, "0.30.0", 1_000 + SEMAINE_S, false), Some("hebdo"));
    }

    #[test]
    fn identifiant_stable() {
        let d = std::env::temp_dir().join(format!("eko-telemetrie-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        let a = lire(&d);
        assert_eq!(a.id.len(), 32);
        assert!(a.id.bytes().all(|o| o.is_ascii_hexdigit() && !o.is_ascii_uppercase()));
        assert_eq!(lire(&d).id, a.id);
        let _ = std::fs::remove_dir_all(&d);
    }
}
