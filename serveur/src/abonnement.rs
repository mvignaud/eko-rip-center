//! Abonnement du Cloud E-KO RIP : l'offre se paie à la machine, et les offres payantes
//! incluent la sauvegarde en ligne de la configuration. Le serveur RIP
//! - déclare ses machines à la plate-forme (`POST /api/site/inventaire`) : à l'ouverture du
//!   tunnel, après chaque changement du catalogue, puis toutes les 6 h ;
//! - lui envoie sa sauvegarde (`POST /api/site/sauvegardes`, la même archive que
//!   `GET /api/sauvegarde`) : la nuit à une minute tirée au hasard entre 1 h et 5 h (heure
//!   locale), 2 minutes après un changement de configuration, ou à la demande ;
//! - restaure une sauvegarde sur ordre de la plate-forme (`POST /api/cloud/restaurer`,
//!   signé HMAC-SHA256 avec la clé du site).
//!
//! Tout se fait avec les identifiants du tunnel (`Authorization: Bearer <site>.<clé>`) et
//! seulement quand le serveur est rattaché au cloud.

use crate::{cloud, machines, maintenant};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

/// Entête de la commande de restauration : `t=<secondes Unix>,v1=<hex>`.
pub const ENTETE_SERVICE: &str = "x-eko-cloud-service";
/// Écart d'horloge toléré pour une commande signée.
pub const TOLERANCE_S: u64 = 300;
const DELAI_CHANGEMENT: Duration = Duration::from_secs(120);
const PERIODE_INVENTAIRE: Duration = Duration::from_secs(6 * 3600);
const REPRISE_INVENTAIRE: Duration = Duration::from_secs(300);
/// Taille maximale d'une archive téléchargée.
const ARCHIVE_MAX: u64 = 256 << 20;

/// Marque posée par `garder` sur une commande dont la signature a été vérifiée.
#[derive(Clone, Copy, Debug)]
pub struct Signee;

/// Corps de la commande de restauration.
#[derive(Deserialize)]
pub struct Commande {
    pub id: String,
}

/// Un identifiant de sauvegarde de la plate-forme : jamais de `/` ni de `..` dans l'adresse.
pub fn id_valide(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

static INVENTAIRE: tokio::sync::Notify = tokio::sync::Notify::const_new();
static CONFIG: tokio::sync::Notify = tokio::sync::Notify::const_new();
/// Une seule sauvegarde à la fois (nuit, changement, demande).
static EN_COURS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// À appeler à l'ouverture du tunnel.
pub fn reveiller_inventaire() {
    INVENTAIRE.notify_one();
}

/// À appeler après une écriture réussie de l'API ; `machines` : le catalogue a pu changer.
pub fn config_modifiee(machines: bool) {
    CONFIG.notify_one();
    if machines {
        INVENTAIRE.notify_one();
    }
}

// --- Suivi enregistré (derniere sauvegarde, réponse de l'inventaire) ---

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Suivi {
    /// Dernière sauvegarde acceptée par la plate-forme (secondes Unix).
    pub derniere: Option<u64>,
    /// `ok`, `offre` (offre sans sauvegarde), `quota`, `erreur`.
    pub etat: String,
    pub message: Option<String>,
    /// Empreinte du contenu (sans la date de création) de la dernière sauvegarde envoyée.
    pub empreinte: Option<String>,
    /// L'offre a refusé la sauvegarde (402) ou l'inventaire annonce `sauvegardes: false` :
    /// plus d'essai automatique jusqu'au prochain inventaire favorable.
    pub sans_offre: bool,
    pub offre: Option<String>,
    pub depassement: bool,
}

impl Default for Suivi {
    fn default() -> Self {
        Suivi { derniere: None, etat: "ok".into(), message: None, empreinte: None, sans_offre: false, offre: None, depassement: false }
    }
}

fn fichier(donnees: &Path) -> PathBuf {
    donnees.join("cloud-abonnement.json")
}

static VERROU_SUIVI: Mutex<()> = Mutex::new(());

pub fn suivi(donnees: &Path) -> Suivi {
    crate::etat::lire(&fichier(donnees)).ok().flatten().unwrap_or_default()
}

fn modifier(donnees: &Path, f: impl FnOnce(&mut Suivi)) {
    let _v = VERROU_SUIVI.lock().unwrap_or_else(|p| p.into_inner());
    let mut s = suivi(donnees);
    f(&mut s);
    if let Err(e) = crate::etat::ecrire(&fichier(donnees), &s, false) {
        eprintln!("cloud : suivi de l'abonnement non écrit : {e}");
    }
}

/// Partie `sauvegarde` de `GET /api/cloud`, et l'abonnement tel que la plate-forme l'a annoncé.
pub fn etat_public(donnees: &Path) -> (serde_json::Value, serde_json::Value) {
    let s = suivi(donnees);
    (
        serde_json::json!({ "derniere": s.derniere, "etat": s.etat, "message": s.message }),
        serde_json::json!({ "offre": s.offre, "sauvegardes": !s.sans_offre, "depassement": s.depassement }),
    )
}

// --- Inventaire ---

/// Inventaire des machines : imprimantes d'un côté, découpeurs (traceurs et tables de
/// découpe) de l'autre. Une machine est comptée une fois : celles qui impriment (Epson,
/// Roland VersaCAMM, qui découpe elle-même) sont des imprimantes.
pub fn inventaire(version: &str, liste: &[machines::Machine]) -> serde_json::Value {
    let fiche = |m: &machines::Machine| serde_json::json!({ "id": m.id, "nom": m.nom, "modele": m.modele });
    serde_json::json!({
        "version": version,
        "imprimantes": liste.iter().filter(|m| m.modele.imprime()).map(fiche).collect::<Vec<_>>(),
        "decoupeurs": liste.iter().filter(|m| !m.modele.imprime() && m.modele.decoupe()).map(fiche).collect::<Vec<_>>(),
    })
}

fn agent(delai: u64) -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(delai)))
        .user_agent(format!("eko-rip-serveur/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

fn autorisation(c: &cloud::Config) -> String {
    format!("Bearer {}.{}", c.site, c.cle)
}

/// Déclare les machines à la plate-forme et note sa réponse. Rend false si elle est
/// injoignable (à refaire bientôt).
pub async fn envoyer_inventaire(donnees: &Path, liste: Vec<machines::Machine>) -> bool {
    let Some(c) = cloud::lire(donnees) else { return true };
    let corps = inventaire(env!("CARGO_PKG_VERSION"), &liste);
    let reponse = tokio::task::spawn_blocking(move || -> Option<(u16, serde_json::Value)> {
        let mut r = agent(20).post(format!("{}/api/site/inventaire", c.url)).header("Authorization", autorisation(&c)).send_json(&corps).ok()?;
        Some((r.status().as_u16(), r.body_mut().read_json().unwrap_or_default()))
    })
    .await
    .ok()
    .flatten();
    match reponse {
        Some((200, v)) => {
            let sauvegardes = v["sauvegardes"].as_bool().unwrap_or(false);
            modifier(donnees, |s| {
                s.offre = v["offre"].as_str().map(str::to_string);
                s.depassement = v["depassement"].as_bool().unwrap_or(false);
                s.sans_offre = !sauvegardes;
                if !sauvegardes {
                    s.etat = "offre".into();
                } else if s.etat == "offre" {
                    s.etat = "ok".into();
                    s.message = None;
                }
            });
            true
        }
        Some((statut, _)) => {
            eprintln!("cloud : inventaire refusé ({statut})");
            statut < 500
        }
        None => false,
    }
}

// --- Sauvegardes ---

/// Empreinte SHA-256 en hexadécimal.
pub fn sha256_octets(octets: &[u8]) -> String {
    use sha2::Digest;
    crate::sauvegarde::hex(&sha2::Sha256::digest(octets))
}

/// Empreinte du contenu d'une sauvegarde sans sa date de création : deux nuits sans
/// changement donnent la même.
fn empreinte_contenu(s: &crate::sauvegarde::Sauvegarde) -> String {
    let mut v = serde_json::to_value(s).unwrap_or_default();
    v["cree_le"] = 0.into();
    sha256_octets(v.to_string().as_bytes())
}

/// Motifs de sauvegarde : `nuit`, `changement`, `demande`.
pub async fn sauvegarder(etat: &crate::Partage, motif: &'static str) -> Suivi {
    let donnees = etat.config.donnees.clone();
    let _un_seul = EN_COURS.lock().await;
    let Some(c) = cloud::lire(&donnees) else { return suivi(&donnees) };
    let avant = suivi(&donnees);
    if avant.sans_offre && motif != "demande" {
        return avant;
    }
    let s = match crate::construire_sauvegarde(etat).await {
        Ok(s) => s,
        Err(e) => {
            modifier(&donnees, |x| (x.etat, x.message) = ("erreur".into(), Some(e)));
            return suivi(&donnees);
        }
    };
    let contenu = empreinte_contenu(&s);
    // Rien n'a changé : un « changement » n'est pas renvoyé. La sauvegarde de nuit part quand
    // même : la plate-forme répond 409 (identique) mais note la nuit, sinon elle croirait les
    // sauvegardes en échec.
    if motif == "changement" && avant.empreinte.as_deref() == Some(contenu.as_str()) && avant.etat == "ok" {
        return avant;
    }
    let corps = serde_json::to_vec_pretty(&s).unwrap_or_default();
    let empreinte = sha256_octets(&corps);
    let reponse = tokio::task::spawn_blocking(move || -> Result<(u16, serde_json::Value), String> {
        let mut r = agent(120)
            .post(format!("{}/api/site/sauvegardes", c.url))
            .header("Authorization", autorisation(&c))
            .header("X-EKO-Motif", motif)
            .header("X-EKO-Empreinte", empreinte)
            .content_type("application/octet-stream")
            .send(&corps[..])
            .map_err(|e| e.to_string())?;
        Ok((r.status().as_u16(), r.body_mut().read_json().unwrap_or_default()))
    })
    .await
    .unwrap_or_else(|e| Err(e.to_string()));
    let maintenant = maintenant();
    match reponse {
        // 409 : même empreinte que la précédente sauvegarde, rien à refaire.
        Ok((201 | 409, _)) => modifier(&donnees, |x| (x.derniere, x.etat, x.message, x.empreinte, x.sans_offre) = (Some(maintenant), "ok".into(), None, Some(contenu), false)),
        Ok((402, _)) => modifier(&donnees, |x| (x.etat, x.message, x.sans_offre) = ("offre".into(), None, true)),
        Ok((413, _)) => modifier(&donnees, |x| (x.etat, x.message) = ("quota".into(), None)),
        Ok((statut, v)) => modifier(&donnees, |x| (x.etat, x.message) = ("erreur".into(), Some(format!("http {statut} {}", v["erreur"].as_str().or(v["code"].as_str()).unwrap_or(""))))),
        Err(e) => modifier(&donnees, |x| (x.etat, x.message) = ("erreur".into(), Some(e))),
    }
    suivi(&donnees)
}

/// Télécharge une archive (bloquant).
pub fn telecharger(c: &cloud::Config, id: &str) -> Result<Vec<u8>, u16> {
    let mut r = agent(120).get(format!("{}/api/site/sauvegardes/{id}", c.url)).header("Authorization", autorisation(c)).call().map_err(|_| 0u16)?;
    let statut = r.status().as_u16();
    if statut != 200 {
        return Err(statut);
    }
    r.body_mut().with_config().limit(ARCHIVE_MAX).read_to_vec().map_err(|_| 0)
}

// --- Commande signée de la plate-forme ---

/// Signature attendue : `HMAC-SHA256(clé = sha256 hex de la clé du site, "<t>.<corps>")`.
pub fn signature(cle_site: &str, t: u64, corps: &[u8]) -> String {
    let mut message = format!("{t}.").into_bytes();
    message.extend_from_slice(corps);
    format!("t={t},v1={}", crate::erp::hmac_sha256(eko_rip_cloud::sha256_hex(cle_site).as_bytes(), &message))
}

static VUES: Mutex<Vec<(String, u64)>> = Mutex::new(Vec::new());

/// La commande est-elle signée avec la clé du site, datée de moins de 5 minutes et jamais
/// vue ? Une signature acceptée est retenue le temps de la tolérance (rejeu).
pub fn signature_valide(cle_site: &str, entete: Option<&str>, corps: &[u8], maintenant: u64) -> bool {
    let Some(entete) = entete else { return false };
    let (mut t, mut v1) = (None, None);
    for morceau in entete.split(',') {
        match morceau.trim().split_once('=') {
            Some(("t", x)) => t = x.parse::<u64>().ok(),
            Some(("v1", x)) => v1 = Some(x),
            _ => {}
        }
    }
    let (Some(t), Some(v1)) = (t, v1) else { return false };
    if t.abs_diff(maintenant) > TOLERANCE_S {
        return false;
    }
    let attendue = signature(cle_site, t, corps);
    let attendu_v1 = attendue.rsplit_once("v1=").map_or("", |(_, h)| h);
    if !crate::erp::egal(attendu_v1.as_bytes(), v1.to_ascii_lowercase().as_bytes()) {
        return false;
    }
    let mut vues = VUES.lock().unwrap_or_else(|p| p.into_inner());
    vues.retain(|(_, quand)| quand.abs_diff(maintenant) <= TOLERANCE_S);
    if vues.iter().any(|(h, _)| h == v1) {
        return false;
    }
    vues.push((v1.to_string(), maintenant));
    true
}

// --- Planification ---

/// Décalage de l'heure locale sur UTC, en secondes, à l'instant `t`.
fn decalage_local(t: u64) -> i64 {
    #[cfg(unix)]
    {
        let tt = t as libc::time_t;
        // SAFETY : `tm` est une structure de C sans pointeur à valider, remplie par localtime_r.
        unsafe {
            let mut tm: libc::tm = std::mem::zeroed();
            if libc::localtime_r(&tt, &mut tm).is_null() { 0 } else { tm.tm_gmtoff as i64 }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = t;
        0
    }
}

/// Prochain passage de nuit : la minute `minute` (depuis minuit, heure locale) la plus
/// proche dans le futur. `decalage` : heure locale − UTC, en secondes.
pub fn prochaine_nuit(maintenant: u64, decalage: i64, minute: u64) -> u64 {
    let locale = maintenant as i64 + decalage;
    let mut cible = locale - locale.rem_euclid(86_400) + minute as i64 * 60;
    if cible <= locale {
        cible += 86_400;
    }
    (cible - decalage) as u64
}

/// Minute tirée au hasard entre 1 h et 5 h.
fn minute_de_nuit() -> u64 {
    let mut o = [0u8; 4];
    let _ = getrandom::fill(&mut o);
    60 + u64::from(u32::from_le_bytes(o)) % 240
}

fn dans(secondes: u64) -> tokio::time::Instant {
    tokio::time::Instant::now() + Duration::from_secs(secondes)
}

/// Boucle de fond : inventaire, sauvegardes de nuit, après changement et à la demande.
pub async fn boucle(etat: crate::Partage) {
    let donnees = etat.config.donnees.clone();
    let mut prochain_inventaire = tokio::time::Instant::now() + PERIODE_INVENTAIRE;
    let mut changement: Option<tokio::time::Instant> = None;
    let mut minute = minute_de_nuit();
    let mut nuit = prochaine_nuit(maintenant(), decalage_local(maintenant()), minute);
    let loin = || tokio::time::Instant::now() + Duration::from_secs(86_400 * 365);
    loop {
        let attente_nuit = tokio::time::Instant::now() + Duration::from_secs(nuit.saturating_sub(maintenant()));
        tokio::select! {
            _ = INVENTAIRE.notified() => {
                let liste = etat.machines.read().unwrap().clone();
                prochain_inventaire = if envoyer_inventaire(&donnees, liste).await { dans(PERIODE_INVENTAIRE.as_secs()) } else { dans(REPRISE_INVENTAIRE.as_secs()) };
            }
            _ = tokio::time::sleep_until(prochain_inventaire) => {
                let liste = etat.machines.read().unwrap().clone();
                prochain_inventaire = if envoyer_inventaire(&donnees, liste).await { dans(PERIODE_INVENTAIRE.as_secs()) } else { dans(REPRISE_INVENTAIRE.as_secs()) };
            }
            _ = CONFIG.notified() => {
                if changement.is_none() && cloud::lire(&donnees).is_some() {
                    changement = Some(tokio::time::Instant::now() + DELAI_CHANGEMENT);
                }
            }
            _ = tokio::time::sleep_until(changement.unwrap_or_else(loin)) => {
                changement = None;
                sauvegarder(&etat, "changement").await;
            }
            _ = tokio::time::sleep_until(attente_nuit) => {
                sauvegarder(&etat, "nuit").await;
                minute = minute_de_nuit();
                // Le prochain passage est le lendemain au plus tôt (une seule sauvegarde par nuit).
                let (t, d) = (maintenant(), decalage_local(maintenant()));
                nuit = prochaine_nuit(t + 86_400 - (t as i64 + d).rem_euclid(86_400) as u64, d, minute);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn machine(id: &str, modele: machines::Modele) -> machines::Machine {
        serde_json::from_value(serde_json::json!({ "id": id, "nom": format!("Nom {id}"), "modele": modele, "laize_mm": 1000.0, "connexion": { "type": "dossier" } })).unwrap()
    }

    #[test]
    fn inventaire_classe_imprimantes_et_decoupeurs() {
        use machines::Modele::*;
        let liste = vec![machine("epson", EpsonScS80600), machine("kona", MutohKona), machine("fc7000", GraphtecFc7000), machine("roland", RolandVersacamm)];
        let v = inventaire("1.2.3", &liste);
        assert_eq!(v["version"], "1.2.3");
        let ids = |cle: &str| v[cle].as_array().unwrap().iter().map(|m| m["id"].as_str().unwrap().to_string()).collect::<Vec<_>>();
        assert_eq!(ids("imprimantes"), ["epson", "roland"]);
        assert_eq!(ids("decoupeurs"), ["kona", "fc7000"]);
        assert_eq!(v["decoupeurs"][0], serde_json::json!({ "id": "kona", "nom": "Nom kona", "modele": "mutoh_kona" }));
    }

    #[test]
    fn nuit_entre_1h_et_5h_locale() {
        // 2026-11-01 12:00 UTC ; Paris = UTC+1 (3600 s) : 03:00 locale = 02:00 UTC le lendemain.
        let midi = 1_793_534_400u64;
        let t = prochaine_nuit(midi, 3600, 180);
        assert_eq!(t, midi - 12 * 3600 + 86_400 + 2 * 3600);
        // Avant l'heure du jour : le jour même.
        let t2 = prochaine_nuit(midi - 12 * 3600, 3600, 180);
        assert_eq!(t2, midi - 12 * 3600 + 2 * 3600);
        assert!(t > midi && t - midi <= 86_400);
        for _ in 0..50 {
            assert!((60..300).contains(&minute_de_nuit()));
        }
    }

    #[test]
    fn signature_acceptee_refusee_et_rejeu() {
        let cle = "cle-du-site-test";
        let corps = br#"{"id":"abc"}"#;
        let t = 1_800_000_000u64;
        let bonne = signature(cle, t, corps);
        // Vecteur calculé indépendamment : HMAC(sha256_hex(cle), "t.corps").
        assert_eq!(bonne, format!("t={t},v1={}", crate::erp::hmac_sha256(eko_rip_cloud::sha256_hex(cle).as_bytes(), format!("{t}.{{\"id\":\"abc\"}}").as_bytes())));
        assert!(!signature_valide(cle, None, corps, t));
        assert!(!signature_valide(cle, Some("n'importe quoi"), corps, t));
        assert!(!signature_valide("autre-cle", Some(&bonne), corps, t));
        assert!(!signature_valide(cle, Some(&bonne), br#"{"id":"xyz"}"#, t), "corps modifié");
        // Hors fenêtre de 300 s (dans les deux sens).
        assert!(!signature_valide(cle, Some(&bonne), corps, t + 301));
        assert!(!signature_valide(cle, Some(&bonne), corps, t - 301));
        // Acceptée une fois, refusée au rejeu.
        assert!(signature_valide(cle, Some(&bonne), corps, t + 299));
        assert!(!signature_valide(cle, Some(&bonne), corps, t + 299));
    }

    #[test]
    fn identifiants_de_sauvegarde() {
        assert!(id_valide("a1-B_2"));
        assert!(!id_valide("") && !id_valide("../x") && !id_valide("a/b") && !id_valide(&"a".repeat(65)));
    }
}
