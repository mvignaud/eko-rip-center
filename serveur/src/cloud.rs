//! Cloud E-KO RIP (lot 1) : le serveur se rattache à la plate-forme (`platform.e-ko.rip`)
//! et lui ouvre un tunnel sortant ; les postes et l'app mobile y accèdent alors depuis
//! l'extérieur, à `https://<site>.platform.e-ko.rip`, sans VPN ni port ouvert.
//!
//! - `cloud.json` (0600) : adresse de la plate-forme, site et clé, reçus à l'appairage.
//! - Appairage : le serveur demande un code court, l'affiche (Système › Cloud), quelqu'un le
//!   valide sur la plate-forme ; le serveur, qui interroge toutes les 3 s, reçoit sa clé.
//! - Tunnel : ouvert tant que `cloud.json` existe, rouvert après une coupure (1 s, 2 s… 60 s).
//! - Jamais sans compte : sans utilisateur déclaré, l'accès est libre sur le réseau local ;
//!   le tunnel ne s'ouvre pas, et une requête venue par lui est refusée.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Plate-forme par défaut (`EKO_RIP_CLOUD_URL` pour une autre, essais compris).
pub const URL_DEFAUT: &str = "https://platform.e-ko.rip";
/// Interrogation de la plate-forme pendant un appairage.
const INTERVALLE_APPAIRAGE: Duration = Duration::from_secs(3);
const ATTENTE_MAX: Duration = Duration::from_secs(60);

pub const CLOUD_SANS_COMPTE: &str = "cloud_sans_compte";
pub const CLOUD_INJOIGNABLE: &str = "cloud_injoignable";
pub const APPAIRAGE_EXPIRE: &str = "appairage_expire";

/// Marque des requêtes arrivées par le tunnel (posée par une couche propre au tunnel :
/// une requête du réseau local ne peut pas la porter).
#[derive(Clone, Copy, Debug)]
pub struct ParCloud;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub url: String,
    pub site: String,
    pub cle: String,
}

pub fn fichier(donnees: &Path) -> PathBuf {
    donnees.join("cloud.json")
}

pub fn lire(donnees: &Path) -> Option<Config> {
    crate::etat::lire(&fichier(donnees)).ok().flatten()
}

fn ecrire(donnees: &Path, c: &Config) -> std::io::Result<()> {
    crate::etat::ecrire(&fichier(donnees), c, true)
}

pub fn url_plateforme() -> String {
    std::env::var("EKO_RIP_CLOUD_URL").ok().filter(|v| !v.is_empty()).unwrap_or_else(|| URL_DEFAUT.into()).trim_end_matches('/').to_string()
}

struct Appairage {
    url: String,
    demande: String,
    code: String,
    lien: String,
    expire: Instant,
}

#[derive(Default)]
struct Vivant {
    /// `inactif`, `connexion`, `connecte`, `attente` (nouvel essai prévu).
    etat: &'static str,
    erreur: Option<String>,
    depuis: Option<u64>,
    appairage: Option<Appairage>,
}

static VIVANT: Mutex<Vivant> = Mutex::new(Vivant { etat: "inactif", erreur: None, depuis: None, appairage: None });
/// Réveille la boucle du tunnel (appairage terminé, déconnexion).
static CHANGEMENT: tokio::sync::Notify = tokio::sync::Notify::const_new();

fn vivant() -> std::sync::MutexGuard<'static, Vivant> {
    VIVANT.lock().unwrap_or_else(|p| p.into_inner())
}

fn noter(etat: &'static str, erreur: Option<String>) {
    let mut v = vivant();
    if v.etat != etat {
        v.depuis = Some(crate::maintenant());
    }
    v.etat = etat;
    v.erreur = erreur;
}

/// État pour l'écran Système (jamais la clé).
pub fn etat_public(donnees: &Path) -> serde_json::Value {
    let c = lire(donnees);
    let v = vivant();
    let (sauvegarde, abonnement) = crate::abonnement::etat_public(donnees);
    let appairage = v.appairage.as_ref().filter(|a| a.expire > Instant::now()).map(|a| serde_json::json!({ "code": a.code, "lien": a.lien, "expire_dans": a.expire.saturating_duration_since(Instant::now()).as_secs() }));
    serde_json::json!({
        "rattache": c.is_some(),
        "url": c.as_ref().map_or_else(url_plateforme, |c| c.url.clone()),
        "site": c.as_ref().map(|c| c.site.clone()),
        "adresse": c.as_ref().map(|c| eko_rip_cloud::relais::adresse_du_site(&c.url, &c.site)),
        "etat": if c.is_some() { v.etat } else { "inactif" },
        "depuis": v.depuis,
        "erreur": v.erreur,
        "appairage": appairage,
        "sans_compte": crate::acces::libre(donnees),
        "sauvegarde": sauvegarde,
        "abonnement": abonnement,
    })
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(20)))
        .user_agent(format!("eko-rip-serveur/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

/// Nom proposé pour le site : `EKO_RIP_CLOUD_NOM`, sinon le nom de la machine.
fn nom_du_site() -> String {
    let n = std::env::var("EKO_RIP_CLOUD_NOM").ok().or_else(|| std::fs::read_to_string("/etc/hostname").ok()).unwrap_or_default();
    let n: String = n.trim().chars().filter(|c| !c.is_control()).take(60).collect();
    if n.is_empty() { "Serveur E-KO RIP".into() } else { n }
}

/// Demande un code d'appairage (bloquant : à lancer hors du fil asynchrone). Rend
/// `(code, lien, durée)` ; l'interrogation qui suit est à lancer avec [`suivre`].
pub fn demander(url: &str) -> Result<(String, String, u64), &'static str> {
    let mut r = agent().post(format!("{url}/api/appairage")).send_json(serde_json::json!({ "nom": nom_du_site() })).map_err(|_| CLOUD_INJOIGNABLE)?;
    if r.status().as_u16() != 201 {
        return Err(CLOUD_INJOIGNABLE);
    }
    let v: serde_json::Value = r.body_mut().read_json().map_err(|_| CLOUD_INJOIGNABLE)?;
    let (Some(demande), Some(code), Some(lien)) = (v["demande"].as_str(), v["code"].as_str(), v["lien"].as_str()) else { return Err(CLOUD_INJOIGNABLE) };
    let duree = v["expire_dans"].as_u64().unwrap_or(600);
    vivant().appairage = Some(Appairage { url: url.into(), demande: demande.into(), code: code.into(), lien: lien.into(), expire: Instant::now() + Duration::from_secs(duree) });
    Ok((code.into(), lien.into(), duree))
}

/// Interroge la plate-forme jusqu'à la validation du code, son expiration ou son abandon
/// (déconnexion, nouvelle demande).
pub async fn suivre(donnees: PathBuf) {
    loop {
        tokio::time::sleep(INTERVALLE_APPAIRAGE).await;
        let Some((url, demande, expire)) = vivant().appairage.as_ref().map(|a| (a.url.clone(), a.demande.clone(), a.expire)) else { return };
        if Instant::now() > expire {
            let mut v = vivant();
            if v.appairage.as_ref().is_some_and(|a| a.demande == demande) {
                v.appairage = None;
                v.erreur = Some(APPAIRAGE_EXPIRE.into());
            }
            return;
        }
        let u = url.clone();
        let d = demande.clone();
        let reponse = tokio::task::spawn_blocking(move || -> Option<(u16, serde_json::Value)> {
            let mut r = agent().get(format!("{u}/api/appairage/{d}")).call().ok()?;
            Some((r.status().as_u16(), r.body_mut().read_json().unwrap_or_default()))
        })
        .await
        .ok()
        .flatten();
        // Une autre demande a pris la place pendant l'attente : celle-ci est abandonnée.
        if vivant().appairage.as_ref().is_none_or(|a| a.demande != demande) {
            return;
        }
        match reponse {
            Some((200, v)) if v["etat"] == "valide" => {
                let (Some(site), Some(cle)) = (v["site"].as_str(), v["cle"].as_str()) else { continue };
                let c = Config { url, site: site.into(), cle: cle.into() };
                let ecrit = ecrire(&donnees, &c);
                let mut vv = vivant();
                vv.appairage = None;
                match ecrit {
                    Ok(()) => {
                        eprintln!("cloud : rattaché à la plate-forme (site {})", c.site);
                        vv.erreur = None;
                        drop(vv);
                        CHANGEMENT.notify_one();
                    }
                    Err(e) => vv.erreur = Some(format!("cloud.json : {e}")),
                }
                return;
            }
            Some((404, _)) => {
                let mut v = vivant();
                v.appairage = None;
                v.erreur = Some(APPAIRAGE_EXPIRE.into());
                return;
            }
            // En attente, ou plate-forme momentanément injoignable : on réessaie.
            _ => {}
        }
    }
}

/// Oublie le rattachement (le tunnel se ferme aussitôt) et tout appairage en cours.
pub fn detacher(donnees: &Path) -> std::io::Result<()> {
    vivant().appairage = None;
    let r = match std::fs::remove_file(fichier(donnees)) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    };
    noter("inactif", None);
    CHANGEMENT.notify_one();
    r
}

/// Boucle du tunnel : ouvert tant que le serveur est rattaché et a des comptes.
pub async fn boucle(donnees: PathBuf, service: axum::Router) {
    let agent = format!("eko-rip-serveur/{}", env!("CARGO_PKG_VERSION"));
    let mut attente = Duration::from_secs(1);
    loop {
        let Some(c) = lire(&donnees) else {
            noter("inactif", None);
            CHANGEMENT.notified().await;
            continue;
        };
        if crate::acces::libre(&donnees) {
            noter("attente", Some(CLOUD_SANS_COMPTE.into()));
            let _ = tokio::time::timeout(ATTENTE_MAX, CHANGEMENT.notified()).await;
            continue;
        }
        let precedente = vivant().erreur.clone();
        noter("connexion", precedente);
        let ouvert = std::sync::atomic::AtomicBool::new(false);
        let servir = eko_rip_cloud::client::servir(&c.url, &c.site, &c.cle, &agent, service.clone(), || {
            ouvert.store(true, std::sync::atomic::Ordering::Relaxed);
            noter("connecte", None);
            crate::abonnement::reveiller_inventaire();
            eprintln!("cloud : tunnel ouvert");
        });
        let resultat = tokio::select! {
            r = servir => Some(r),
            // Rattachement changé ou retiré : on coupe et on relit.
            _ = CHANGEMENT.notified() => None,
        };
        let Some(resultat) = resultat else {
            attente = Duration::from_secs(1);
            continue;
        };
        if ouvert.load(std::sync::atomic::Ordering::Relaxed) {
            // Le tunnel a tenu : la coupure est passagère, on rouvre vite.
            attente = Duration::from_secs(1);
        }
        let erreur = match resultat {
            Ok(()) => None,
            Err(e) => {
                eprintln!("cloud : {e}");
                Some(match e {
                    eko_rip_cloud::client::Erreur::Refus { code, .. } => code,
                    eko_rip_cloud::client::Erreur::Reseau(m) => m,
                })
            }
        };
        noter("attente", erreur);
        let _ = tokio::time::timeout(attente, CHANGEMENT.notified()).await;
        attente = (attente * 2).min(ATTENTE_MAX);
    }
}
