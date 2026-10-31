//! Rapports de bug vers la plate-forme E-KO RIP (`POST <plate-forme>/api/rapports`).
//!
//! - **Signaler un problème** (Système, clients Mac et Linux) : `POST /api/rapports` ici ; le
//!   serveur y joint son journal si on le demande et le transmet aussitôt.
//! - **Rapports automatiques** (plantage, rendu ou sortie en échec, rendu anormalement long) :
//!   seulement si l'administrateur l'a accepté (question posée à la première ouverture de
//!   l'administration, puis case dans Système ; `rapports_auto` de `telemetrie.json`). Un même
//!   code part au plus une fois par 6 h. Ils passent par `rapports/attente/` (un plantage les
//!   écrit avant que le processus ne s'arrête) et partent dans les minutes qui suivent.
//!
//! Le journal (dernières lignes de `journalctl -u eko-rip-serveur`) est masqué avant l'envoi :
//! chemins, noms de fichiers et adresses e-mail remplacés. La plate-forme le garde en privé ;
//! l'issue GitHub publique n'en porte rien.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const RAPPORT_INVALIDE: &str = "rapport_invalide";
pub const RAPPORTS_SATURE: &str = "rapports_sature";
pub const PLATEFORME_INJOIGNABLE: &str = "plateforme_injoignable";
/// Un même code automatique au plus une fois par 6 h.
const INTERVALLE_S: u64 = 6 * 3_600;
const LIGNES_JOURNAL: usize = 300;

/// Dernier envoi automatique par code (en mémoire : un redémarrage remet à zéro).
static DERNIERS: Mutex<Option<HashMap<String, u64>>> = Mutex::new(None);

pub fn url() -> String {
    std::env::var("EKO_RAPPORTS_URL").ok().filter(|v| !v.is_empty()).unwrap_or_else(|| format!("{}/api/rapports", crate::cloud::url_plateforme()))
}

/// Envois automatiques acceptés par l'administrateur (`None` : pas encore demandé).
pub fn auto(donnees: &Path) -> Option<bool> {
    crate::telemetrie::lire(donnees).rapports_auto
}

pub fn regler_auto(donnees: &Path, accepte: bool) -> std::io::Result<()> {
    let mut e = crate::telemetrie::lire(donnees);
    e.rapports_auto = Some(accepte);
    crate::telemetrie::ecrire(donnees, &e)
}

// ---------------------------------------------------------------------------------------------
// Masquage

const EXTENSIONS: &[&str] = &["pdf", "ai", "eps", "ps", "tif", "tiff", "png", "jpg", "jpeg", "svg", "psd", "plt", "hpgl", "dxf", "json", "icc", "icm", "cgats", "txt", "csv", "zip", "prn"];

fn masquer_mot(mot: &str) -> String {
    let coeur = mot.trim_matches(|c: char| matches!(c, '"' | '\'' | '«' | '»' | '(' | ')' | '[' | ']' | ',' | ';' | ':' | '.'));
    if coeur.is_empty() {
        return mot.to_string();
    }
    let ext = coeur.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
    let remplace = if coeur.contains('@') {
        Some("<e-mail>")
    } else if coeur.contains('/') || coeur.contains('\\') {
        Some("<chemin>")
    } else if ext.is_some_and(|e| EXTENSIONS.contains(&e.as_str())) {
        Some("<fichier>")
    } else {
        None
    };
    match remplace {
        Some(r) => mot.replacen(coeur, r, 1),
        None => mot.to_string(),
    }
}

/// Chemins, noms de fichiers et e-mails d'une ligne de journal remplacés ; les textes entre
/// guillemets (souvent un nom de travail ou de client) aussi.
pub fn masquer(ligne: &str) -> String {
    let mut sortie = String::with_capacity(ligne.len());
    let mut guillemet: Option<char> = None;
    let mut mot = String::new();
    let vider = |mot: &mut String, sortie: &mut String| {
        if !mot.is_empty() {
            sortie.push_str(&masquer_mot(mot));
            mot.clear();
        }
    };
    for c in ligne.chars() {
        match (guillemet, c) {
            (Some(f), c) if c == f => {
                guillemet = None;
                sortie.push_str("<texte>");
                sortie.push(if f == '»' { '»' } else { c });
            }
            (Some(_), _) => {}
            (None, '"' | '«') => {
                vider(&mut mot, &mut sortie);
                sortie.push(c);
                guillemet = Some(if c == '«' { '»' } else { '"' });
            }
            (None, c) if c.is_whitespace() => {
                vider(&mut mot, &mut sortie);
                sortie.push(c);
            }
            (None, c) => mot.push(c),
        }
    }
    if guillemet.is_some() {
        sortie.push_str("<texte>");
    }
    vider(&mut mot, &mut sortie);
    sortie
}

/// Dernières lignes du journal du service, masquées (vide si `journalctl` n'est pas lisible).
pub fn journal() -> String {
    let sortie = std::process::Command::new("journalctl").args(["-u", "eko-rip-serveur", "-n", &LIGNES_JOURNAL.to_string(), "--no-pager", "-o", "short-iso"]).output();
    match sortie {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).lines().map(masquer).collect::<Vec<_>>().join("\n"),
        _ => String::new(),
    }
}

// ---------------------------------------------------------------------------------------------
// Envoi

/// Corps envoyé à la plate-forme (sans le journal, ajouté à l'envoi). `poste` : type, version,
/// OS, identifiant d'installation.
fn corps([type_poste, version, os, id]: [&str; 4], nature: &str, code: &str, titre: &str, description: &str, email: &str) -> serde_json::Value {
    let serveur = if type_poste == "serveur" { "" } else { VERSION };
    serde_json::json!({ "id": id, "type": type_poste, "version": version, "os": os, "nature": nature, "code": code, "titre": titre, "description": description, "email": email, "serveur": serveur })
}

/// Envoie à la plate-forme ; rend la référence, ou le code d'erreur à montrer.
fn poster(mut corps: serde_json::Value, joindre_journal: bool) -> Result<String, &'static str> {
    if joindre_journal {
        let mut j = journal();
        while j.len() > 500 * 1024 {
            j = j.split_once('\n').map_or(String::new(), |(_, reste)| reste.to_string());
        }
        corps["journal"] = j.into();
    }
    let agent: ureq::Agent = ureq::Agent::config_builder().http_status_as_error(false).timeout_global(Some(Duration::from_secs(30))).user_agent(format!("eko-rip-serveur/{VERSION}")).build().into();
    let mut r = agent.post(url()).send_json(corps).map_err(|_| PLATEFORME_INJOIGNABLE)?;
    match r.status().as_u16() {
        201 => r.body_mut().read_json::<serde_json::Value>().ok().and_then(|v| v["reference"].as_str().map(String::from)).ok_or(PLATEFORME_INJOIGNABLE),
        400 | 413 | 422 => Err(RAPPORT_INVALIDE),
        429 => Err(RAPPORTS_SATURE),
        _ => Err(PLATEFORME_INJOIGNABLE),
    }
}

/// Champs d'un « Signaler un problème » reçu d'un client ou de l'écran Système.
#[derive(Debug, Deserialize)]
pub struct Signalement {
    pub titre: String,
    pub description: String,
    #[serde(default)]
    pub email: String,
    #[serde(default = "vrai")]
    pub joindre_journal: bool,
    #[serde(default)]
    pub client: Option<Client>,
}

fn vrai() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct Client {
    #[serde(rename = "type")]
    pub type_poste: String,
    pub version: String,
    pub os: String,
}

/// Vérifie un signalement ; rend le corps à envoyer.
pub fn preparer(donnees: &Path, s: &Signalement) -> Result<serde_json::Value, &'static str> {
    let (titre, description, email) = (s.titre.trim(), s.description.trim(), s.email.trim());
    let longueur = |t: &str, min: usize, max: usize| (min..=max).contains(&t.chars().count());
    if !longueur(titre, 1, 120) || !longueur(description, 1, 5000) || !longueur(email, 0, 120) {
        return Err(RAPPORT_INVALIDE);
    }
    let (type_poste, version, os) = match &s.client {
        Some(c) if matches!(c.type_poste.as_str(), "client-mac" | "client-linux" | "client-windows" | "client-mobile") => (c.type_poste.clone(), c.version.chars().take(32).collect(), c.os.chars().take(64).collect()),
        Some(_) => return Err(RAPPORT_INVALIDE),
        None => ("serveur".to_string(), VERSION.to_string(), crate::telemetrie::os()),
    };
    let id = if type_poste == "serveur" { crate::telemetrie::lire(donnees).id } else { String::new() };
    Ok(corps([&type_poste, &version, &os, &id], "manuel", "", titre, description, email))
}

/// Envoi d'un signalement (bloquant).
pub fn envoyer_signalement(corps: serde_json::Value, joindre_journal: bool) -> Result<String, &'static str> {
    poster(corps, joindre_journal)
}

pub fn dossier_attente(donnees: &Path) -> PathBuf {
    donnees.join("rapports").join("attente")
}

/// Rapport automatique : noté s'il est accepté et que ce code n'est pas parti depuis 6 h.
/// N'attend rien : l'envoi se fait au prochain passage de [`boucle`] ou de [`envoyer_attente`].
pub fn signaler(donnees: &Path, nature: &str, code: &str, detail: &str) -> bool {
    if auto(donnees) != Some(true) {
        return false;
    }
    let maintenant = crate::maintenant();
    {
        let mut d = DERNIERS.lock().unwrap_or_else(|p| p.into_inner());
        let derniers = d.get_or_insert_with(HashMap::new);
        let cle = format!("{nature}:{code}");
        if derniers.get(&cle).is_some_and(|t| maintenant.saturating_sub(*t) < INTERVALLE_S) {
            return false;
        }
        derniers.insert(cle, maintenant);
    }
    ecrire_attente(donnees, nature, code, detail)
}

/// Écrit un rapport en attente (aussi depuis le crochet de plantage : rien d'asynchrone).
pub fn ecrire_attente(donnees: &Path, nature: &str, code: &str, detail: &str) -> bool {
    let titre = match nature {
        "plantage" => "Plantage du serveur",
        "lenteur" => "Traitement anormalement long",
        _ => "Erreur du serveur",
    };
    let description: String = detail.lines().map(masquer).collect::<Vec<_>>().join("\n").chars().take(5000).collect();
    let id = crate::telemetrie::lire(donnees).id;
    let c = corps(["serveur", VERSION, &crate::telemetrie::os(), &id], nature, code, titre, &description, "");
    let d = dossier_attente(donnees);
    let _ = std::fs::create_dir_all(&d);
    std::fs::write(d.join(format!("{}-{}.json", crate::maintenant(), uuid::Uuid::new_v4())), serde_json::to_vec(&c).unwrap_or_default()).is_ok()
}

/// Envoie les rapports en attente (bloquant). Gardés tant que la plate-forme est injoignable ;
/// effacés s'ils sont refusés ou si l'accord a été retiré entre-temps.
pub fn envoyer_attente(donnees: &Path) {
    let Ok(lus) = std::fs::read_dir(dossier_attente(donnees)) else { return };
    let mut fichiers: Vec<PathBuf> = lus.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "json")).collect();
    fichiers.sort();
    let accepte = auto(donnees) == Some(true);
    for f in fichiers {
        let corps = std::fs::read(&f).ok().and_then(|o| serde_json::from_slice::<serde_json::Value>(&o).ok());
        match corps {
            Some(c) if accepte => match poster(c, true) {
                Err(PLATEFORME_INJOIGNABLE) => return,
                Ok(reference) => {
                    eprintln!("rapport automatique envoyé : {reference}");
                    let _ = std::fs::remove_file(&f);
                }
                Err(_) => {
                    let _ = std::fs::remove_file(&f);
                }
            },
            _ => {
                let _ = std::fs::remove_file(&f);
            }
        }
    }
}

/// Envoi des rapports en attente : au démarrage (après un plantage), puis toutes les 5 min.
pub async fn boucle(donnees: PathBuf) {
    tokio::time::sleep(Duration::from_secs(90)).await;
    loop {
        let d = donnees.clone();
        let _ = tokio::task::spawn_blocking(move || envoyer_attente(&d)).await;
        tokio::time::sleep(Duration::from_secs(300)).await;
    }
}

/// Crochet de plantage : le rapport est écrit sur le disque avant l'arrêt et part au
/// redémarrage (systemd relance le service). Le message et l'emplacement dans le code, pas
/// de données de travail.
pub fn installer_crochet(donnees: PathBuf) {
    let precedent = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if auto(&donnees) == Some(true) {
            let lieu = info.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_default();
            let message = info.payload().downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| info.payload().downcast_ref::<String>().cloned()).unwrap_or_default();
            let fil = std::thread::current().name().unwrap_or("?").to_string();
            let trace = std::backtrace::Backtrace::force_capture().to_string();
            let trace: String = trace.lines().take(60).collect::<Vec<_>>().join("\n");
            ecrire_attente(&donnees, "plantage", "panique", &format!("{message}\nà {lieu} (fil {fil})\n\n{trace}"));
        }
        precedent(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masque_ce_qui_peut_nommer_un_client() {
        assert_eq!(masquer("travail 3f2 : gs a échoué sur /var/lib/eko-rip/travaux/x/Dupont_bache.pdf"), "travail 3f2 : gs a échoué sur <chemin>");
        assert_eq!(masquer("fichier Dupont_bache.PDF, illisible"), "fichier <fichier>, illisible");
        assert_eq!(masquer("envoi de \"Bâche Dupont 3x2\" refusé"), "envoi de \"<texte>\" refusé");
        assert_eq!(masquer("réglage «Vinyle Martin» absent"), "réglage «<texte>» absent");
        assert_eq!(masquer("contact jean@dupont.fr ok"), "contact <e-mail> ok");
        assert_eq!(masquer("rendu en 12.5 s, 300 dpi"), "rendu en 12.5 s, 300 dpi");
        assert_eq!(masquer("ouvert \"jamais fermé"), "ouvert \"<texte>");
    }

    #[test]
    fn signalement_valide() {
        let d = std::env::temp_dir().join(format!("eko-rapports-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        let s = |titre: &str, client: Option<Client>| Signalement { titre: titre.into(), description: "rien ne sort".into(), email: String::new(), joindre_journal: true, client };
        let c = preparer(&d, &s("Aperçu blanc", None)).unwrap();
        assert_eq!((c["type"].as_str(), c["nature"].as_str(), c["serveur"].as_str()), (Some("serveur"), Some("manuel"), Some("")));
        assert_eq!(c["id"].as_str().unwrap().len(), 32);
        let c = preparer(&d, &s("x", Some(Client { type_poste: "client-mac".into(), version: "0.30.0".into(), os: "macOS 26".into() }))).unwrap();
        assert_eq!((c["type"].as_str(), c["serveur"].as_str(), c["id"].as_str()), (Some("client-mac"), Some(VERSION), Some("")));
        assert_eq!(preparer(&d, &s("", None)), Err(RAPPORT_INVALIDE));
        assert_eq!(preparer(&d, &s("x", Some(Client { type_poste: "serveur".into(), version: "1".into(), os: "x".into() }))), Err(RAPPORT_INVALIDE));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn automatique_seulement_avec_accord() {
        let d = std::env::temp_dir().join(format!("eko-rapports-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        let attente = || std::fs::read_dir(dossier_attente(&d)).map(|l| l.count()).unwrap_or(0);
        assert_eq!(auto(&d), None);
        assert!(!signaler(&d, "erreur", "essai_sans_accord", "x"));
        regler_auto(&d, true).unwrap();
        assert!(signaler(&d, "erreur", "essai_accord", "sur /tmp/Dupont.pdf"));
        assert!(!signaler(&d, "erreur", "essai_accord", "x"), "une fois par 6 h");
        assert_eq!(attente(), 1);
        let ecrit = std::fs::read_dir(dossier_attente(&d)).unwrap().next().unwrap().unwrap().path();
        let v: serde_json::Value = serde_json::from_slice(&std::fs::read(ecrit).unwrap()).unwrap();
        assert_eq!((v["nature"].as_str(), v["code"].as_str(), v["description"].as_str()), (Some("erreur"), Some("essai_accord"), Some("sur <chemin>")));
        // Accord retiré : l'attente est vidée sans rien envoyer.
        regler_auto(&d, false).unwrap();
        envoyer_attente(&d);
        assert_eq!(attente(), 0);
        let _ = std::fs::remove_dir_all(&d);
    }
}
