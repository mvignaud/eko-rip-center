//! Notifications de la plate-forme, en plus des e-mails : mêmes événements (site hors ligne,
//! de retour en ligne, nouvelle app connectée), un seul point d'émission : [`notifier`].
//!
//! Trois canaux :
//! - le **journal** (table `notifications`, 30 jours) que les apps de bureau interrogent chaque
//!   minute (`GET /api/moi/notifications`) pour afficher des notifications natives ;
//! - le **push mobile** (Expo) pour les apps mobiles qui ont donné leur jeton (`POST /api/moi/push`) ;
//! - le **Web Push** des navigateurs inscrits depuis « Mon compte » (module `webpush`).
//!
//! Les envois réseau partent hors de la requête (`spawn_blocking`) ; en cas d'échec, une ligne de
//! journal sans secret. Dans les tests, un transport en mémoire remplace le réseau.

use crate::applis::{bearer, compte_bearer};
use crate::base;
use crate::courriels::Courriel;
use crate::relais::{Partage, Relais};
use crate::temps::{date_heure_fr, duree_fr};
use crate::webpush;
use axum::Json;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use rusqlite::{Connection, params};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Conservation du journal.
pub const CONSERVATION: u64 = 30 * 24 * 3600;
/// Nombre maximal de notifications rendues par requête.
pub const MAX_LISTE: usize = 50;
/// Navigateurs inscrits au plus par compte.
pub const MAX_ABONNEMENTS: i64 = 10;
pub const URL_EXPO: &str = "https://exp.host/--/api/v2/push/send";

pub const SITE_HORS_LIGNE: &str = "site_hors_ligne";
pub const SITE_EN_LIGNE: &str = "site_en_ligne";
pub const NOUVELLE_APP: &str = "nouvelle_app";
/// Paiement refusé, relance, résiliation pour impayé, retour au Gratuit.
pub const PAIEMENT: &str = "paiement";
/// Quota de sauvegarde, sauvegarde en échec.
pub const SAUVEGARDE: &str = "sauvegarde";
/// Offre dépassée, accès à distance suspendu.
pub const DEPASSEMENT: &str = "depassement";
/// Réponse du support, demande résolue.
pub const SUPPORT: &str = "support";

// ————— Contenu —————

/// Une notification : le genre (`type` dans le journal), le site concerné s'il y en a un, le
/// texte et la page à ouvrir au clic.
#[derive(Clone, Debug, PartialEq)]
pub struct Notification {
    pub genre: &'static str,
    pub site: Option<String>,
    pub titre: String,
    pub corps: String,
    pub lien: String,
}

impl Notification {
    /// `alerte` (hors ligne, nouvelle app : à traiter) ou `info` (retour en ligne).
    pub fn gravite(&self) -> &'static str {
        if self.genre == SITE_EN_LIGNE { "info" } else { "alerte" }
    }

    pub fn site_hors_ligne(url: &str, id: &str, nom: &str, depuis: u64, maintenant: u64) -> Self {
        Notification {
            genre: SITE_HORS_LIGNE,
            site: Some(id.into()),
            titre: format!("Site « {nom} » hors ligne"),
            corps: format!("Plus de connexion depuis {} (depuis le {}).", duree_fr(maintenant.saturating_sub(depuis)), date_heure_fr(depuis)),
            lien: format!("{url}/sites"),
        }
    }

    pub fn site_en_ligne(url: &str, id: &str, nom: &str, debut: u64, fin: u64) -> Self {
        Notification {
            genre: SITE_EN_LIGNE,
            site: Some(id.into()),
            titre: format!("Site « {nom} » de retour en ligne"),
            corps: format!("Coupure de {}.", duree_fr(fin.saturating_sub(debut))),
            lien: format!("{url}/sites"),
        }
    }

    /// Notification de facturation ou de sauvegarde (genre `PAIEMENT`, `SAUVEGARDE`, `DEPASSEMENT`).
    pub fn autre(genre: &'static str, site: Option<&str>, titre: &str, corps: &str, lien: &str) -> Self {
        Notification { genre, site: site.map(str::to_string), titre: titre.into(), corps: corps.into(), lien: lien.into() }
    }

    pub fn nouvelle_app(url: &str, appareil: &str) -> Self {
        Notification { genre: NOUVELLE_APP, site: None, titre: "Nouvelle app connectée".into(), corps: format!("{appareil} a été connecté à votre compte."), lien: format!("{url}/compte") }
    }
}

/// Une ligne du journal.
#[derive(Clone, Debug, PartialEq)]
pub struct Entree {
    pub id: i64,
    pub genre: String,
    pub site: Option<String>,
    pub titre: String,
    pub corps: String,
    pub lien: String,
    pub cree_le: u64,
}

impl Entree {
    fn json(&self) -> serde_json::Value {
        serde_json::json!({ "id": self.id, "type": self.genre, "site": self.site, "titre": self.titre, "corps": self.corps, "lien": self.lien, "cree_le": self.cree_le })
    }
}

// ————— Journal —————

pub fn ajouter(c: &Connection, compte: i64, n: &Notification, t: u64) -> rusqlite::Result<i64> {
    c.execute("INSERT INTO notifications (compte, type, site, titre, corps, lien, cree_le) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)", params![compte, n.genre, n.site, n.titre, n.corps, n.lien, t as i64])?;
    Ok(c.last_insert_rowid())
}

/// Au plus `MAX_LISTE` notifications d'identifiant > `apres`, dans l'ordre croissant.
pub fn depuis(c: &Connection, compte: i64, apres: i64) -> rusqlite::Result<Vec<Entree>> {
    let mut q = c.prepare("SELECT id, type, site, titre, corps, lien, cree_le FROM notifications WHERE compte = ?1 AND id > ?2 ORDER BY id LIMIT ?3")?;
    q.query_map(params![compte, apres, MAX_LISTE as i64], |r| Ok(Entree { id: r.get(0)?, genre: r.get(1)?, site: r.get(2)?, titre: r.get(3)?, corps: r.get(4)?, lien: r.get(5)?, cree_le: r.get::<_, i64>(6)? as u64 }))?.collect()
}

/// Plus grand identifiant du compte (0 si le journal est vide). Les identifiants ne sont jamais
/// réutilisés (AUTOINCREMENT) : même purgé, le « dernier » ne recule pas.
pub fn dernier(c: &Connection, compte: i64) -> rusqlite::Result<i64> {
    c.query_row("SELECT COALESCE(MAX(id), 0) FROM notifications WHERE compte = ?1", [compte], |r| r.get(0))
}

pub fn purger(c: &Connection, t: u64) -> rusqlite::Result<()> {
    c.execute("DELETE FROM notifications WHERE cree_le < ?1", [t.saturating_sub(CONSERVATION) as i64])?;
    Ok(())
}

// ————— Jetons Expo (rattachés au jeton d'app) —————

/// `ExponentPushToken[…]` (ou `ExpoPushToken[…]`), contenu en caractères sûrs.
pub fn jeton_expo_valide(j: &str) -> bool {
    let Some(reste) = j.strip_prefix("ExponentPushToken[").or_else(|| j.strip_prefix("ExpoPushToken[")) else { return false };
    let Some(interieur) = reste.strip_suffix(']') else { return false };
    !interieur.is_empty() && interieur.len() <= 100 && interieur.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Rattache le jeton Expo au jeton d'app (`empreinte`) ; un même appareil ne notifie qu'une fois :
/// il est détaché des autres jetons d'app qui le portaient.
pub fn enregistrer_expo(c: &Connection, empreinte: &str, jeton_expo: &str, plateforme: &str) -> rusqlite::Result<bool> {
    c.execute("UPDATE jetons_applis SET jeton_expo = NULL, plateforme_expo = NULL WHERE jeton_expo = ?1 AND empreinte <> ?2", params![jeton_expo, empreinte])?;
    Ok(c.execute("UPDATE jetons_applis SET jeton_expo = ?1, plateforme_expo = ?2 WHERE empreinte = ?3", params![jeton_expo, plateforme, empreinte])? == 1)
}

pub fn retirer_expo(c: &Connection, empreinte: &str) -> rusqlite::Result<()> {
    c.execute("UPDATE jetons_applis SET jeton_expo = NULL, plateforme_expo = NULL WHERE empreinte = ?1", [empreinte])?;
    Ok(())
}

/// Le service Expo ne connaît plus ce jeton.
pub fn oublier_expo(c: &Connection, jeton_expo: &str) -> rusqlite::Result<()> {
    c.execute("UPDATE jetons_applis SET jeton_expo = NULL, plateforme_expo = NULL WHERE jeton_expo = ?1", [jeton_expo])?;
    Ok(())
}

/// Jetons Expo des apps connectées (non expirées) du compte.
pub fn jetons_expo(c: &Connection, compte: i64) -> rusqlite::Result<Vec<String>> {
    let mut q = c.prepare("SELECT DISTINCT jeton_expo FROM jetons_applis WHERE compte = ?1 AND jeton_expo IS NOT NULL AND expire >= ?2")?;
    q.query_map(params![compte, crate::maintenant() as i64], |r| r.get(0))?.collect()
}

// ————— Navigateurs inscrits (Web Push) —————

#[derive(Clone, Debug, PartialEq)]
pub struct Abonnement {
    pub id: i64,
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
    pub appareil: String,
    pub cree_le: u64,
}

/// Adresse d'abonnement acceptable : HTTPS, vers un nom d'hôte public (pas d'adresse IP ni de
/// `localhost` : la plate-forme n'enverra jamais de requête vers un réseau interne).
pub fn endpoint_valide(e: &str) -> bool {
    let Some(reste) = e.strip_prefix("https://") else { return false };
    let hote = reste.split(['/', '?', '#']).next().unwrap_or("");
    let nom = hote.rsplit_once(':').map_or(hote, |(n, _)| n);
    e.len() <= 600
        && !hote.contains('@')
        && nom.contains('.')
        && !nom.starts_with('.')
        && nom.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
        && nom.rsplit('.').next().is_some_and(|tld| tld.chars().any(|c| c.is_ascii_alphabetic()))
        && nom != "localhost"
        && !nom.ends_with(".localhost")
        && !nom.ends_with(".local")
        && !nom.ends_with(".internal")
}

/// Inscrit (ou remplace, si l'adresse existe déjà) un navigateur. Faux si le compte en a trop.
pub fn ajouter_abonnement(c: &Connection, compte: i64, endpoint: &str, p256dh: &str, auth: &str, appareil: &str, t: u64) -> rusqlite::Result<bool> {
    let existe: bool = c.query_row("SELECT EXISTS(SELECT 1 FROM abonnements_web WHERE endpoint = ?1 AND compte = ?2)", params![endpoint, compte], |r| r.get(0))?;
    let n: i64 = c.query_row("SELECT COUNT(*) FROM abonnements_web WHERE compte = ?1", [compte], |r| r.get(0))?;
    if !existe && n >= MAX_ABONNEMENTS {
        return Ok(false);
    }
    c.execute(
        "INSERT INTO abonnements_web (compte, endpoint, p256dh, auth, appareil, cree_le) VALUES (?1, ?2, ?3, ?4, ?5, ?6) ON CONFLICT(endpoint) DO UPDATE SET compte = excluded.compte, p256dh = excluded.p256dh, auth = excluded.auth, appareil = excluded.appareil",
        params![compte, endpoint, p256dh, auth, appareil, t as i64],
    )?;
    Ok(true)
}

pub fn abonnements(c: &Connection, compte: i64) -> rusqlite::Result<Vec<Abonnement>> {
    let mut q = c.prepare("SELECT id, endpoint, p256dh, auth, appareil, cree_le FROM abonnements_web WHERE compte = ?1 ORDER BY id")?;
    q.query_map([compte], |r| Ok(Abonnement { id: r.get(0)?, endpoint: r.get(1)?, p256dh: r.get(2)?, auth: r.get(3)?, appareil: r.get(4)?, cree_le: r.get::<_, i64>(5)? as u64 }))?.collect()
}

pub fn retirer_abonnement(c: &Connection, compte: i64, id: i64) -> rusqlite::Result<bool> {
    Ok(c.execute("DELETE FROM abonnements_web WHERE id = ?1 AND compte = ?2", params![id, compte])? == 1)
}

pub fn oublier_endpoint(c: &Connection, endpoint: &str) -> rusqlite::Result<()> {
    c.execute("DELETE FROM abonnements_web WHERE endpoint = ?1", [endpoint])?;
    Ok(())
}

/// « Firefox sur Linux » d'après le `User-Agent` (pour reconnaître le navigateur dans la liste).
pub fn nom_navigateur(user_agent: &str) -> String {
    let ua = user_agent;
    let navigateur = if ua.contains("Edg/") {
        "Edge"
    } else if ua.contains("OPR/") {
        "Opera"
    } else if ua.contains("Firefox/") {
        "Firefox"
    } else if ua.contains("Chrome/") || ua.contains("CriOS/") {
        "Chrome"
    } else if ua.contains("Safari/") {
        "Safari"
    } else {
        "Navigateur"
    };
    let systeme = if ua.contains("Android") {
        "Android"
    } else if ua.contains("iPhone") || ua.contains("iPad") {
        "iOS"
    } else if ua.contains("Windows") {
        "Windows"
    } else if ua.contains("Mac OS X") || ua.contains("Macintosh") {
        "macOS"
    } else if ua.contains("Linux") || ua.contains("X11") {
        "Linux"
    } else {
        ""
    };
    if systeme.is_empty() { navigateur.into() } else { format!("{navigateur} sur {systeme}") }
}

// ————— Clés VAPID —————

const CLE_VAPID: &str = "vapid_prive";

/// Clé VAPID de la plate-forme, créée au premier démarrage et gardée dans `reglages`. La clé
/// privée n'est jamais affichée ni renvoyée.
pub fn cle_vapid(c: &Connection) -> rusqlite::Result<p256::SecretKey> {
    if let Some(k) = base::reglage(c, CLE_VAPID)?.and_then(|v| webpush::decoder(&v)).and_then(|b| p256::SecretKey::from_slice(&b).ok()) {
        return Ok(k);
    }
    let k = webpush::nouvelle_cle();
    base::regler(c, CLE_VAPID, &crate::base64url(&k.to_bytes()))?;
    Ok(k)
}

/// Clé publique VAPID (point non compressé, base64url) : `applicationServerKey` du navigateur.
pub fn cle_publique(c: &Connection) -> rusqlite::Result<String> {
    Ok(crate::base64url(&webpush::public_sec1(&cle_vapid(c)?)))
}

// ————— Transport —————

/// Un envoi de push, tel que le garde le transport de test.
#[derive(Clone, Debug, PartialEq)]
pub enum Envoi {
    /// Jeton Expo et corps JSON de la requête.
    Expo { jeton: String, corps: serde_json::Value },
    /// Adresse d'abonnement et message (JSON, avant chiffrement).
    Web { endpoint: String, charge: serde_json::Value },
}

/// Où partent les pushs : le réseau, ou la mémoire (tests).
pub enum Transport {
    Reseau,
    Memoire(Arc<Mutex<Vec<Envoi>>>),
}

impl Relais {
    /// Pour les tests : garde les pushs en mémoire au lieu de les envoyer. Rend la boîte.
    pub fn utiliser_memoire_push(&self) -> Arc<Mutex<Vec<Envoi>>> {
        let boite = Arc::new(Mutex::new(Vec::new()));
        *self.push.lock().unwrap_or_else(|p| p.into_inner()) = Transport::Memoire(boite.clone());
        boite
    }
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder().http_status_as_error(false).max_redirects(0).timeout_global(Some(Duration::from_secs(15))).build().into()
}

/// Corps de la requête Expo (format attendu par l'app mobile).
pub fn corps_expo(jeton: &str, id: i64, t: u64, n: &Notification) -> serde_json::Value {
    serde_json::json!({
        "to": jeton,
        "title": n.titre,
        "body": n.corps,
        "sound": "default",
        "channelId": "alertes",
        "data": { "evenement": { "id": id, "le": t, "genre": n.genre, "gravite": n.gravite(), "titre": n.titre, "texte": n.corps, "site": n.site, "lien": n.lien } },
    })
}

/// Message lu par le service worker (`/sw.js`).
pub fn charge_web(id: i64, n: &Notification) -> serde_json::Value {
    serde_json::json!({ "id": id, "titre": n.titre, "corps": n.corps, "lien": n.lien, "tag": format!("{}:{}", n.genre, n.site.as_deref().unwrap_or("")) })
}

/// La réponse d'Expo dit-elle que l'appareil n'est plus enregistré ?
pub fn expo_a_oublier(reponse: &str) -> bool {
    reponse.contains("DeviceNotRegistered")
}

/// Le service de notification du navigateur a-t-il supprimé l'abonnement ?
pub fn web_a_oublier(statut: u16) -> bool {
    statut == 404 || statut == 410
}

fn envoyer_expo(r: &Partage, jeton: &str, corps: &serde_json::Value) {
    let mut q = agent().post(URL_EXPO).header("Accept", "application/json");
    if let Some(acces) = std::env::var("EKO_CLOUD_EXPO_TOKEN").ok().filter(|v| !v.is_empty()) {
        q = q.header("Authorization", format!("Bearer {acces}"));
    }
    match q.send_json(corps) {
        Ok(mut rep) => {
            let statut = rep.status().as_u16();
            let texte = rep.body_mut().read_to_string().unwrap_or_default();
            if expo_a_oublier(&texte) {
                let _ = oublier_expo(&r.base(), jeton);
            } else if statut >= 400 {
                eprintln!("push Expo refusé (HTTP {statut})");
            }
        }
        Err(e) => eprintln!("push Expo non envoyé : {e}"),
    }
}

fn envoyer_web(r: &Partage, cle: &p256::SecretKey, a: &Abonnement, charge: &str) {
    let corps = (|| {
        let autorisation = webpush::autorisation_vapid(cle, &a.endpoint, crate::maintenant())?;
        let ua = webpush::decoder(&a.p256dh).ok_or("clé p256dh illisible")?;
        let auth = webpush::decoder(&a.auth).ok_or("secret auth illisible")?;
        Ok::<_, String>((autorisation, webpush::chiffrer(&ua, &auth, charge.as_bytes())?))
    })();
    let (autorisation, corps) = match corps {
        Ok(v) => v,
        Err(e) => {
            eprintln!("push navigateur non préparé : {e}");
            return;
        }
    };
    let q = agent().post(&a.endpoint).header("TTL", "86400").header("Urgency", "high").header("Content-Encoding", "aes128gcm").header("Content-Type", "application/octet-stream").header("Authorization", autorisation);
    match q.send(&corps[..]) {
        Ok(rep) if web_a_oublier(rep.status().as_u16()) => {
            let _ = oublier_endpoint(&r.base(), &a.endpoint);
        }
        Ok(rep) if rep.status().as_u16() >= 400 => eprintln!("push navigateur refusé (HTTP {})", rep.status().as_u16()),
        Ok(_) => {}
        Err(e) => eprintln!("push navigateur non envoyé : {e}"),
    }
}

/// Point d'émission unique d'un événement pour un compte : l'e-mail correspondant (s'il y en a
/// un), l'entrée du journal, puis les pushs (apps mobiles, navigateurs). Ne bloque jamais :
/// les envois réseau partent dans des tâches à part.
pub fn notifier(r: &Partage, compte: i64, courriel: Option<Courriel>, n: Notification) {
    if let Some(c) = courriel {
        r.envoyer(c);
    }
    let t = crate::maintenant();
    let (id, expo, web) = {
        let b = r.base();
        let id = match ajouter(&b, compte, &n, t) {
            Ok(id) => id,
            Err(e) => {
                eprintln!("journal des notifications : {e}");
                return;
            }
        };
        (id, jetons_expo(&b, compte).unwrap_or_default(), abonnements(&b, compte).unwrap_or_default())
    };
    if expo.is_empty() && web.is_empty() {
        return;
    }
    let envois_expo: Vec<(String, serde_json::Value)> = expo.into_iter().map(|j| (j.clone(), corps_expo(&j, id, t, &n))).collect();
    let charge = charge_web(id, &n);
    {
        let e = r.push.lock().unwrap_or_else(|p| p.into_inner());
        if let Transport::Memoire(boite) = &*e {
            let mut boite = boite.lock().unwrap_or_else(|p| p.into_inner());
            boite.extend(envois_expo.into_iter().map(|(jeton, corps)| Envoi::Expo { jeton, corps }));
            boite.extend(web.into_iter().map(|a| Envoi::Web { endpoint: a.endpoint, charge: charge.clone() }));
            return;
        }
    }
    let Ok(rt) = tokio::runtime::Handle::try_current() else { return };
    let r = r.clone();
    rt.spawn_blocking(move || {
        for (jeton, corps) in &envois_expo {
            envoyer_expo(&r, jeton, corps);
        }
        if !web.is_empty() {
            let Ok(cle) = cle_vapid(&r.base()) else { return };
            let texte = charge.to_string();
            for a in &web {
                envoyer_web(&r, &cle, a, &texte);
            }
        }
    });
}

// ————— Routes —————

fn non_autorise() -> Response {
    let mut rep = (StatusCode::UNAUTHORIZED, Json(serde_json::json!({ "code": "jeton_invalide" }))).into_response();
    rep.headers_mut().insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
    rep
}

fn sans_cache(mut rep: Response) -> Response {
    rep.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    rep
}

/// `GET /api/moi/notifications?apres=<id>` : le journal du compte (jeton d'app).
/// Sans `apres` : seulement `dernier` (pas de rattrapage au premier lancement).
pub async fn liste(State(r): State<Partage>, h: HeaderMap, Query(q): Query<HashMap<String, String>>) -> Response {
    let Some(compte) = compte_bearer(&r, &h) else { return non_autorise() };
    let apres = match q.get("apres") {
        None => None,
        Some(a) => match a.parse::<i64>() {
            Ok(n) if n >= 0 => Some(n),
            _ => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "code": "apres_invalide" }))).into_response(),
        },
    };
    let (entrees, dernier) = {
        let b = r.base();
        let entrees = match apres {
            Some(a) => depuis(&b, compte.id, a).unwrap_or_default(),
            None => Vec::new(),
        };
        (entrees, dernier(&b, compte.id).unwrap_or(0))
    };
    let liste: Vec<serde_json::Value> = entrees.iter().map(Entree::json).collect();
    sans_cache(Json(serde_json::json!({ "notifications": liste, "dernier": dernier })).into_response())
}

/// `POST /api/moi/push` : `{"jeton_expo", "plateforme": "ios"|"android"}` → 204.
pub async fn inscrire_expo(State(r): State<Partage>, h: HeaderMap, Json(corps): Json<serde_json::Value>) -> Response {
    let Some(_) = compte_bearer(&r, &h) else { return non_autorise() };
    let Some(jeton_app) = bearer(&h) else { return non_autorise() };
    let jeton_expo = corps.get("jeton_expo").and_then(|v| v.as_str()).unwrap_or("");
    let plateforme = corps.get("plateforme").and_then(|v| v.as_str()).unwrap_or("");
    if !jeton_expo_valide(jeton_expo) {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "code": "jeton_expo_invalide" }))).into_response();
    }
    if plateforme != "ios" && plateforme != "android" {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "code": "plateforme_invalide" }))).into_response();
    }
    match enregistrer_expo(&r.base(), &crate::sha256_hex(jeton_app), jeton_expo, plateforme) {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        _ => non_autorise(),
    }
}

/// `DELETE /api/moi/push` → 204 : cette app n'est plus notifiée par push.
pub async fn retirer_expo_route(State(r): State<Partage>, h: HeaderMap) -> Response {
    let Some(_) = compte_bearer(&r, &h) else { return non_autorise() };
    if let Some(j) = bearer(&h) {
        let _ = retirer_expo(&r.base(), &crate::sha256_hex(j));
    }
    StatusCode::NO_CONTENT.into_response()
}

/// `GET /api/push/cle` : clé publique VAPID (publique par nature).
pub async fn cle_route(State(r): State<Partage>) -> Response {
    match cle_publique(&r.base()) {
        Ok(k) => Json(serde_json::json!({ "cle": k })).into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

const SERVICE_WORKER: &str = r#"'use strict';
// Service worker de E-KO RIP Cloud : affiche les notifications reçues (Web Push) et ouvre la page au clic.
self.addEventListener('push', function (e) {
  var d = {};
  try { d = e.data.json(); } catch (_) {}
  e.waitUntil(self.registration.showNotification(d.titre || 'E-KO RIP Cloud', {
    body: d.corps || '',
    icon: '/logo-courriel.png',
    tag: d.tag || undefined,
    data: { lien: d.lien || '/sites' }
  }));
});
self.addEventListener('notificationclick', function (e) {
  e.notification.close();
  var cible = new URL('/sites', self.location.origin).href;
  try {
    var u = new URL((e.notification.data && e.notification.data.lien) || '/sites', self.location.origin);
    if (u.origin === self.location.origin) cible = u.href;
  } catch (_) {}
  e.waitUntil(self.clients.matchAll({ type: 'window', includeUncontrolled: true }).then(function (l) {
    for (var i = 0; i < l.length; i++) {
      if (l[i].url === cible && 'focus' in l[i]) return l[i].focus();
    }
    return self.clients.openWindow(cible);
  }));
});
"#;

/// `GET /sw.js` : le service worker, à la racine (portée `/`).
pub async fn service_worker() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/javascript; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache"),
            (header::CONTENT_SECURITY_POLICY, "default-src 'none'"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        SERVICE_WORKER,
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_test() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(base::SCHEMA).unwrap();
        c.execute("INSERT INTO comptes (id, email, nom, cree_le) VALUES (1, 'a@x.fr', 'A', 0), (2, 'b@x.fr', 'B', 0)", []).unwrap();
        c
    }

    fn notif(titre: &str) -> Notification {
        Notification { genre: SITE_HORS_LIGNE, site: Some("s1".into()), titre: titre.into(), corps: "c".into(), lien: "https://x/sites".into() }
    }

    #[test]
    fn apres_dernier_et_isolement_des_comptes() {
        let c = base_test();
        assert_eq!(dernier(&c, 1).unwrap(), 0);
        let a = ajouter(&c, 1, &notif("un"), 100).unwrap();
        let b = ajouter(&c, 1, &notif("deux"), 101).unwrap();
        ajouter(&c, 2, &notif("autre"), 102).unwrap();
        assert!(b > a);
        assert_eq!(dernier(&c, 1).unwrap(), b);
        assert_eq!(depuis(&c, 1, 0).unwrap().iter().map(|e| e.titre.as_str()).collect::<Vec<_>>(), ["un", "deux"]);
        assert_eq!(depuis(&c, 1, a).unwrap().len(), 1);
        assert!(depuis(&c, 1, b).unwrap().is_empty());
        for i in 0..60 {
            ajouter(&c, 1, &notif(&format!("n{i}")), 200).unwrap();
        }
        let page = depuis(&c, 1, 0).unwrap();
        assert_eq!(page.len(), MAX_LISTE);
        assert!(page.windows(2).all(|w| w[0].id < w[1].id), "ordre croissant");
    }

    #[test]
    fn purge_a_30_jours_sans_reutiliser_les_identifiants() {
        let c = base_test();
        let t = 100 * 24 * 3600;
        ajouter(&c, 1, &notif("vieille"), t - CONSERVATION - 1).unwrap();
        let recente = ajouter(&c, 1, &notif("récente"), t - CONSERVATION + 10).unwrap();
        purger(&c, t).unwrap();
        assert_eq!(depuis(&c, 1, 0).unwrap().len(), 1);
        c.execute("DELETE FROM notifications", []).unwrap();
        assert_eq!(ajouter(&c, 1, &notif("neuve"), t).unwrap(), recente + 1);
    }

    #[test]
    fn format_du_jeton_expo() {
        assert!(jeton_expo_valide("ExponentPushToken[xxxxxxxxxxxxxxxxxxxxxx]"));
        assert!(jeton_expo_valide("ExpoPushToken[a-b_C1]"));
        for mauvais in ["", "ExponentPushToken[]", "ExponentPushToken[abc", "abc", "ExponentPushToken[a b]", "ExponentPushToken[a\"}]", "Token[abc]", &format!("ExponentPushToken[{}]", "a".repeat(101))] {
            assert!(!jeton_expo_valide(mauvais), "{mauvais}");
        }
    }

    #[test]
    fn jeton_expo_lie_au_jeton_d_app() {
        let c = base_test();
        let j1 = base::creer_jeton_appli(&c, 1, "iPhone").unwrap();
        let j2 = base::creer_jeton_appli(&c, 1, "Pixel").unwrap();
        let (e1, e2) = (crate::sha256_hex(&j1), crate::sha256_hex(&j2));
        assert!(enregistrer_expo(&c, &e1, "ExponentPushToken[a]", "ios").unwrap());
        assert!(enregistrer_expo(&c, &e2, "ExponentPushToken[b]", "android").unwrap());
        assert_eq!(jetons_expo(&c, 1).unwrap().len(), 2);
        // Le même appareil se reconnecte avec un autre jeton d'app : une seule notification.
        assert!(enregistrer_expo(&c, &e2, "ExponentPushToken[a]", "ios").unwrap());
        assert_eq!(jetons_expo(&c, 1).unwrap(), ["ExponentPushToken[a]"]);
        base::revoquer_jeton_appli(&c, &j2).unwrap();
        assert!(jetons_expo(&c, 1).unwrap().is_empty(), "supprimé avec le jeton d'app");
        assert!(!enregistrer_expo(&c, "inconnu", "ExponentPushToken[z]", "ios").unwrap());
        enregistrer_expo(&c, &e1, "ExponentPushToken[c]", "ios").unwrap();
        oublier_expo(&c, "ExponentPushToken[c]").unwrap();
        assert!(jetons_expo(&c, 1).unwrap().is_empty());
        enregistrer_expo(&c, &e1, "ExponentPushToken[d]", "ios").unwrap();
        base::supprimer_compte(&c, 1).unwrap();
        assert!(jetons_expo(&c, 1).unwrap().is_empty());
    }

    #[test]
    fn reponses_qui_font_oublier() {
        assert!(expo_a_oublier(r#"{"data":{"status":"error","details":{"error":"DeviceNotRegistered"}}}"#));
        assert!(!expo_a_oublier(r#"{"data":{"status":"ok","id":"x"}}"#));
        assert!(web_a_oublier(404) && web_a_oublier(410) && !web_a_oublier(201) && !web_a_oublier(500));
    }

    #[test]
    fn corps_expo_conforme_a_l_app() {
        let n = Notification::site_en_ligne("https://p.fr", "s1", "Atelier", 0, 600);
        let c = corps_expo("ExponentPushToken[a]", 7, 1234, &n);
        assert_eq!(c["to"], "ExponentPushToken[a]");
        assert_eq!(c["sound"], "default");
        assert_eq!(c["channelId"], "alertes");
        let e = &c["data"]["evenement"];
        assert_eq!((e["id"].as_i64(), e["le"].as_u64(), e["genre"].as_str(), e["gravite"].as_str(), e["site"].as_str()), (Some(7), Some(1234), Some("site_en_ligne"), Some("info"), Some("s1")));
        assert_eq!(e["titre"], c["title"]);
        assert_eq!(e["texte"], c["body"]);
        assert_eq!(corps_expo("t", 1, 1, &Notification::site_hors_ligne("u", "s", "n", 0, 900))["data"]["evenement"]["gravite"], "alerte");
        let app = Notification::nouvelle_app("https://p.fr", "Mac");
        assert_eq!(corps_expo("t", 1, 1, &app)["data"]["evenement"]["gravite"], "alerte");
        assert!(corps_expo("t", 1, 1, &app)["data"]["evenement"]["site"].is_null());
    }

    #[test]
    fn abonnements_web_et_adresses() {
        let c = base_test();
        assert!(endpoint_valide("https://fcm.googleapis.com/fcm/send/abc"));
        assert!(endpoint_valide("https://updates.push.services.mozilla.com:443/wpush/v2/x"));
        for mauvais in ["http://fcm.googleapis.com/x", "https://localhost/x", "https://127.0.0.1/x", "https://10.0.0.1:8080/x", "https://[::1]/x", "https://a@b.fr/x", "https://intranet/x", "https://x.local/x", "ftp://a.fr"] {
            assert!(!endpoint_valide(mauvais), "{mauvais}");
        }
        assert!(ajouter_abonnement(&c, 1, "https://a.fr/1", "k", "s", "Firefox sur Linux", 5).unwrap());
        assert!(ajouter_abonnement(&c, 1, "https://a.fr/1", "k2", "s2", "Firefox sur Linux", 6).unwrap(), "même adresse : remplacée");
        assert_eq!(abonnements(&c, 1).unwrap().len(), 1);
        assert_eq!(abonnements(&c, 1).unwrap()[0].p256dh, "k2");
        for i in 0..MAX_ABONNEMENTS - 1 {
            assert!(ajouter_abonnement(&c, 1, &format!("https://a.fr/n{i}"), "k", "s", "", 5).unwrap());
        }
        assert!(!ajouter_abonnement(&c, 1, "https://a.fr/trop", "k", "s", "", 5).unwrap());
        let id = abonnements(&c, 1).unwrap()[0].id;
        assert!(!retirer_abonnement(&c, 2, id).unwrap(), "pas celui d'un autre compte");
        assert!(retirer_abonnement(&c, 1, id).unwrap());
        oublier_endpoint(&c, "https://a.fr/n0").unwrap();
        base::supprimer_compte(&c, 1).unwrap();
        assert!(abonnements(&c, 1).unwrap().is_empty());
        assert_eq!(nom_navigateur("Mozilla/5.0 (X11; Linux x86_64; rv:130.0) Gecko/20100101 Firefox/130.0"), "Firefox sur Linux");
        assert_eq!(nom_navigateur("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15"), "Safari sur macOS");
        assert_eq!(nom_navigateur("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/120 Safari/537.36 Edg/120"), "Edge sur Windows");
    }

    #[test]
    fn cle_vapid_creee_une_fois() {
        let c = base_test();
        let k1 = cle_publique(&c).unwrap();
        assert_eq!(k1, cle_publique(&c).unwrap());
        assert_eq!(webpush::decoder(&k1).unwrap().len(), 65);
        assert_ne!(k1, cle_publique(&base_test()).unwrap());
    }
}
