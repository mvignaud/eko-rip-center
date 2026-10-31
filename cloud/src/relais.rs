//! Le relais de la plate-forme : reçoit les tunnels des serveurs RIP (`/tunnel`), fait
//! passer les requêtes `/s/<site>/…` dans le bon tunnel, et sert l'appairage.
//!
//! Isolement entre sites (un atelier ne doit jamais rien obtenir d'un autre, ni du portail) :
//! - vers un serveur RIP ne part que son cookie de session (`eko_rip_session`) ; ni
//!   `Authorization`, ni les cookies du portail ;
//! - le `Set-Cookie` d'un serveur RIP est limité à son chemin `/s/<site>/` ;
//! - toute réponse d'un serveur RIP porte `Content-Security-Policy: sandbox` : une page
//!   servie par un atelier n'a pas l'origine de la plate-forme et ne peut rien y lire.

use crate::appairage::{self, Appairages, Suivi};
use crate::installations::{self, Registre};
use crate::{CHEMIN_TUNNEL, COOKIE_RIP, PROTOCOLE, portail, sites};
use axum::body::Body;
use axum::extract::{Path as Chemin, Request, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get, post};
use axum::{Json, Router};
use hyper::client::conn::http2::SendRequest;
use hyper_util::rt::{TokioExecutor, TokioTimer};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct Config {
    pub donnees: PathBuf,
    /// Adresse publique, sans barre finale : `https://platform.e-ko.rip`.
    pub url_publique: String,
    /// Taille maximale d'une requête vers un site.
    pub max_octets: u64,
    /// Jeton des statistiques d'installations (`GET /api/installations`) ; vide : route fermée.
    pub jeton_stats: String,
}

struct Tunnel {
    envoi: SendRequest<Body>,
    generation: u64,
    tache: tokio::task::AbortHandle,
}

pub struct Relais {
    pub(crate) config: Config,
    tunnels: Mutex<HashMap<String, Tunnel>>,
    appairages: Mutex<Appairages>,
    installations: Mutex<Registre>,
    generation: AtomicU64,
    /// Base du portail (comptes, sessions, droits, réglages).
    base: Mutex<rusqlite::Connection>,
    /// Limites par adresse ou par compte : (essais, début de la fenêtre).
    limites: Mutex<HashMap<String, (u32, std::time::Instant)>>,
}

pub type Partage = Arc<Relais>;

/// Panique si `installations.json` ou la base sont illisibles (on ne repart pas de zéro en silence).
pub fn nouveau(config: Config) -> Partage {
    let registre = installations::lire(&config.donnees).expect("installations.json");
    let base = crate::base::ouvrir(&config.donnees).expect("base du portail (portail.sqlite)");
    Arc::new(Relais { config, tunnels: Default::default(), appairages: Default::default(), installations: Mutex::new(registre), generation: AtomicU64::new(0), base: Mutex::new(base), limites: Default::default() })
}

impl Relais {
    /// Statistiques anonymes des installations (page d'administration).
    pub fn statistiques_installations(&self) -> installations::Statistiques {
        self.installations.lock().unwrap().statistiques(crate::maintenant())
    }

    /// Accès GitHub réglé dans Administration (jeton et dépôt), sinon aucun.
    pub fn github(&self) -> Option<crate::rapports::Github> {
        let b = self.base();
        let v = |cle: &str| crate::base::reglage(&b, cle).ok().flatten().filter(|v| !v.is_empty());
        let (jeton, depot) = (v("github_jeton")?, v("github_depot")?);
        let api = std::env::var("EKO_CLOUD_GITHUB_API").ok().filter(|v| !v.is_empty()).unwrap_or_else(|| "https://api.github.com".into());
        Some(crate::rapports::Github { jeton, depot, api })
    }

    pub fn connectes(&self) -> Vec<String> {
        self.tunnels.lock().unwrap().keys().cloned().collect()
    }

    pub fn base(&self) -> std::sync::MutexGuard<'_, rusqlite::Connection> {
        self.base.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Vrai tant que `cle` a fait moins de `max` essais dans la fenêtre.
    pub fn limiter(&self, cle: &str, max: u32, fenetre: Duration) -> bool {
        let mut l = self.limites.lock().unwrap_or_else(|p| p.into_inner());
        if l.len() > 50_000 {
            l.retain(|_, (_, debut)| debut.elapsed() < Duration::from_secs(3600));
        }
        let e = l.entry(cle.to_string()).or_insert((0, std::time::Instant::now()));
        if e.1.elapsed() >= fenetre {
            *e = (0, std::time::Instant::now());
        }
        e.0 += 1;
        e.0 <= max
    }

    pub fn code_en_attente(&self, code: &str) -> bool {
        self.appairages.lock().unwrap().code_en_attente(code)
    }

    /// Retire un site : sites.json, droits, et tunnel coupé aussitôt.
    pub fn retirer_site(&self, site: &str) {
        let _ = sites::supprimer(&self.config.donnees, site);
        let _ = crate::base::oublier_site(&self.base(), site);
        if let Some(t) = self.tunnels.lock().unwrap().remove(site) {
            t.tache.abort();
        }
    }
}

pub fn routes(relais: Partage) -> Router {
    Router::new()
        .route("/", get(portail::accueil))
        .route("/inscription", get(portail::inscription).post(portail::inscrire))
        .route("/connexion", get(portail::connexion).post(portail::connecter))
        .route("/deconnexion", post(portail::deconnecter))
        .route("/sites", get(portail::mes_sites))
        .route("/sites/rattacher", post(portail::rattacher))
        .route("/sites/{site}/retirer", post(portail::retirer))
        .route("/appairer", get(portail::appairer))
        .route("/compte", get(portail::mon_compte))
        .route("/compte/supprimer", post(portail::supprimer_compte))
        .route("/admin", get(portail::admin))
        .route("/admin/oauth", post(portail::regler_oauth))
        .route("/admin/github", post(portail::regler_github))
        .route("/admin/rapports/{reference}", get(portail::voir_rapport))
        .route("/admin/comptes/{id}/suspendre", post(portail::suspendre))
        .route("/auth/{fournisseur}", get(portail::partir_oauth))
        .route("/auth/{fournisseur}/retour", get(portail::retour_oauth))
        .route("/{page}", get(portail::legal))
        .route("/sante", get(|| async { "ok" }))
        .route(CHEMIN_TUNNEL, get(ouvrir_tunnel))
        .route("/api/appairage", post(demander_appairage))
        .route("/api/appairage/{demande}", get(suivre_appairage))
        .route("/api/ping", post(recevoir_ping).layer(axum::extract::DefaultBodyLimit::max(4096)))
        .route("/api/installations", get(statistiques_installations))
        .route("/api/rapports", post(recevoir_rapport).layer(axum::extract::DefaultBodyLimit::max(crate::rapports::MAX_CORPS)))
        .route("/s/{site}", any(relayer))
        .route("/s/{site}/", any(relayer))
        .route("/s/{site}/{*reste}", any(relayer))
        .layer(axum::middleware::map_response(entetes_du_portail))
        .with_state(relais)
}

/// En-têtes de sécurité des pages du portail ; une réponse d'atelier garde les siens
/// (`sandbox`, posés par `relayer`).
async fn entetes_du_portail(mut rep: Response) -> Response {
    let h = rep.headers_mut();
    if !h.contains_key(header::CONTENT_SECURITY_POLICY) {
        h.insert(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static("default-src 'none'; style-src 'unsafe-inline'; script-src 'unsafe-inline'; img-src 'self' data:; form-action 'self'; frame-ancestors 'none'; base-uri 'none'"));
        h.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
        h.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
        h.insert(header::REFERRER_POLICY, HeaderValue::from_static("same-origin"));
    }
    rep
}

/// Tâche de fond : demandes expirées, validations orphelines, et tunnels des sites retirés
/// (coupés dans la minute).
pub async fn entretenir(relais: Partage) {
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
        relais.appairages.lock().unwrap().purger();
        appairage::purger_validations(&relais.config.donnees);
        enregistrer_installations(&relais).await;
        let d = relais.config.donnees.clone();
        let _ = tokio::task::spawn_blocking(move || crate::rapports::purger(&d, crate::maintenant())).await;
        if let Ok(sites) = sites::lire(&relais.config.donnees) {
            relais.tunnels.lock().unwrap().retain(|id, t| {
                let garde = sites.iter().any(|s| &s.id == id);
                if !garde {
                    t.tache.abort();
                }
                garde
            });
        }
    }
}

/// Écrit `installations.json` s'il a changé (au plus une fois par minute, et à l'arrêt).
pub async fn enregistrer_installations(relais: &Partage) {
    let contenu = relais.installations.lock().unwrap().a_ecrire();
    if let Some(c) = contenu {
        let donnees = relais.config.donnees.clone();
        if let Ok(Err(e)) = tokio::task::spawn_blocking(move || installations::ecrire(&donnees, &c)).await {
            eprintln!("installations.json : {e}");
        }
    }
}

async fn recevoir_ping(State(relais): State<Partage>, Json(p): Json<installations::Ping>) -> Response {
    match relais.installations.lock().unwrap().noter(&p, crate::maintenant()) {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(installations::Refus::Invalide) => refus(StatusCode::BAD_REQUEST, "ping_invalide"),
        Err(installations::Refus::Sature) => refus(StatusCode::SERVICE_UNAVAILABLE, "ping_sature"),
    }
}

/// Rapport de bug d'une installation : gardé ici, puis issue GitHub en arrière-plan (sans
/// journal ni e-mail). Au plus 20 rapports par installation et 500 en tout par jour.
async fn recevoir_rapport(State(relais): State<Partage>, Json(d): Json<crate::rapports::Demande>) -> Response {
    let Some(rapport) = crate::rapports::valider(&d, crate::maintenant()) else { return refus(StatusCode::BAD_REQUEST, "rapport_invalide") };
    let jour = Duration::from_secs(86_400);
    if !relais.limiter("rapports", 500, jour) || !d.id.is_empty() && !relais.limiter(&format!("rapport:{}", d.id), 20, jour) {
        return refus(StatusCode::TOO_MANY_REQUESTS, "rapports_sature");
    }
    let donnees = relais.config.donnees.clone();
    let reference = rapport.reference.clone();
    let r = rapport.clone();
    if !matches!(tokio::task::spawn_blocking(move || crate::rapports::enregistrer(&donnees, &r)).await, Ok(Ok(()))) {
        return refus(StatusCode::SERVICE_UNAVAILABLE, "indisponible");
    }
    eprintln!("rapport reçu : {reference} ({} {})", rapport.nature, rapport.type_poste);
    if let Some(gh) = relais.github() {
        let donnees = relais.config.donnees.clone();
        tokio::task::spawn_blocking(move || crate::rapports::publier(&donnees, &gh, rapport, crate::maintenant()));
    }
    (StatusCode::CREATED, Json(serde_json::json!({ "reference": reference }))).into_response()
}

/// `Authorization: Bearer <EKO_CLOUD_JETON_STATS>` ; route absente (404) sans jeton configuré.
async fn statistiques_installations(State(relais): State<Partage>, entetes: HeaderMap) -> Response {
    let jeton = &relais.config.jeton_stats;
    if jeton.is_empty() {
        return StatusCode::NOT_FOUND.into_response();
    }
    let donne = entetes.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer ")).unwrap_or("");
    if !crate::egal_constant(&crate::sha256_hex(donne), &crate::sha256_hex(jeton)) {
        return refus(StatusCode::UNAUTHORIZED, "non_autorise");
    }
    Json(relais.installations.lock().unwrap().statistiques(crate::maintenant())).into_response()
}

fn refus(statut: StatusCode, code: &'static str) -> Response {
    (statut, Json(serde_json::json!({ "code": code }))).into_response()
}

/// `Authorization: Bearer <site>.<clé>`.
fn identifiants(entetes: &HeaderMap) -> Option<(String, String)> {
    let v = entetes.get(header::AUTHORIZATION)?.to_str().ok()?.strip_prefix("Bearer ")?;
    let (site, cle) = v.split_once('.')?;
    Some((site.to_string(), cle.to_string()))
}

async fn ouvrir_tunnel(State(relais): State<Partage>, mut requete: Request) -> Response {
    let protocole_ok = requete.headers().get(header::UPGRADE).and_then(|v| v.to_str().ok()).is_some_and(|v| v.eq_ignore_ascii_case(PROTOCOLE));
    if !protocole_ok {
        return refus(StatusCode::BAD_REQUEST, "protocole_inconnu");
    }
    let Some((id, cle)) = identifiants(requete.headers()) else { return refus(StatusCode::UNAUTHORIZED, "cle_absente") };
    let donnees = relais.config.donnees.clone();
    let site = match tokio::task::spawn_blocking({
        let donnees = donnees.clone();
        move || sites::lire(&donnees)
    })
    .await
    {
        Ok(Ok(s)) => s.into_iter().find(|s| s.id == id),
        _ => return refus(StatusCode::SERVICE_UNAVAILABLE, "indisponible"),
    };
    // Même réponse pour un site inconnu et une mauvaise clé : rien à apprendre en essayant.
    if !site.is_some_and(|s| s.cle_valide(&cle)) {
        return refus(StatusCode::UNAUTHORIZED, "cle_invalide");
    }
    let Some(montee) = requete.extensions_mut().remove::<hyper::upgrade::OnUpgrade>() else { return refus(StatusCode::BAD_REQUEST, "protocole_inconnu") };
    let r = relais.clone();
    tokio::spawn(async move {
        let Ok(flux) = montee.await else { return };
        let mut b = hyper::client::conn::http2::Builder::new(TokioExecutor::new());
        b.timer(TokioTimer::new()).keep_alive_interval(Duration::from_secs(25)).keep_alive_timeout(Duration::from_secs(20)).keep_alive_while_idle(true).adaptive_window(true);
        let Ok((envoi, connexion)) = b.handshake::<_, Body>(flux).await else { return };
        let generation = r.generation.fetch_add(1, Ordering::Relaxed);
        let (fin_tx, fin_rx) = tokio::sync::oneshot::channel::<()>();
        let tache = tokio::spawn(async move {
            let _ = connexion.await;
            let _ = fin_tx.send(());
        });
        // Une nouvelle connexion du même site remplace l'ancienne (reconnexion après une
        // coupure que l'ancienne n'a pas encore vue).
        if let Some(ancien) = r.tunnels.lock().unwrap().insert(id.clone(), Tunnel { envoi, generation, tache: tache.abort_handle() }) {
            ancien.tache.abort();
        }
        eprintln!("tunnel ouvert : site {id}");
        let _ = tokio::task::spawn_blocking({
            let (donnees, id) = (donnees.clone(), id.clone());
            move || sites::noter_connexion(&donnees, &id)
        })
        .await;
        let _ = fin_rx.await;
        let mut t = r.tunnels.lock().unwrap();
        if t.get(&id).is_some_and(|x| x.generation == generation) {
            t.remove(&id);
            eprintln!("tunnel fermé : site {id}");
        }
    });
    Response::builder().status(StatusCode::SWITCHING_PROTOCOLS).header(header::CONNECTION, "upgrade").header(header::UPGRADE, PROTOCOLE).body(Body::empty()).expect("réponse 101")
}

#[derive(serde::Deserialize)]
struct DemandeAppairage {
    nom: String,
}

async fn demander_appairage(State(relais): State<Partage>, Json(d): Json<DemandeAppairage>) -> Response {
    let nom = d.nom.trim();
    if nom.is_empty() || nom.chars().count() > 60 || nom.chars().any(char::is_control) {
        return refus(StatusCode::BAD_REQUEST, "nom_invalide");
    }
    let Some((demande, code)) = relais.appairages.lock().unwrap().demander(nom) else { return refus(StatusCode::SERVICE_UNAVAILABLE, "appairage_sature") };
    let url = &relais.config.url_publique;
    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "demande": demande,
            "code": appairage::afficher(&code),
            "lien": format!("{url}/appairer?code={}", appairage::afficher(&code)),
            "expire_dans": appairage::DUREE.as_secs(),
        })),
    )
        .into_response()
}

async fn suivre_appairage(State(relais): State<Partage>, Chemin(demande): Chemin<String>) -> Response {
    let suivi = relais.appairages.lock().unwrap().suivre(&relais.config.donnees, &demande);
    match suivi {
        Suivi::Inconnue => refus(StatusCode::NOT_FOUND, "appairage_expire"),
        Suivi::EnAttente(code) => Json(serde_json::json!({ "etat": "en_attente", "code": appairage::afficher(&code) })).into_response(),
        Suivi::Validee { nom, proprietaire } => {
            let donnees = relais.config.donnees.clone();
            let compte = proprietaire.as_deref().and_then(|p| p.strip_prefix("compte:")).and_then(|p| p.parse::<i64>().ok());
            match tokio::task::spawn_blocking(move || sites::creer(&donnees, &nom, proprietaire)).await {
                Ok(Ok((site, cle))) => {
                    // Validé depuis le portail : le compte devient propriétaire du site.
                    if let Some(c) = compte {
                        let _ = crate::base::donner_droit(&relais.base(), &site.id, c, "proprietaire");
                    }
                    eprintln!("site appairé : {} ({})", site.id, site.nom);
                    let adresse = format!("{}/s/{}", relais.config.url_publique, site.id);
                    Json(serde_json::json!({ "etat": "valide", "site": site.id, "cle": cle, "adresse": adresse })).into_response()
                }
                _ => refus(StatusCode::SERVICE_UNAVAILABLE, "indisponible"),
            }
        }
    }
}

/// En-têtes propres à une connexion, interdits en HTTP/2 ou réservés au relais.
fn a_retirer(nom: &HeaderName) -> bool {
    matches!(nom.as_str(), "connection" | "keep-alive" | "proxy-connection" | "proxy-authorization" | "proxy-authenticate" | "te" | "trailer" | "transfer-encoding" | "upgrade" | "host" | "authorization" | "cookie")
        || nom.as_str().starts_with("x-eko-cloud")
}

/// Ne garde, d'un en-tête `Cookie`, que la session du serveur RIP.
fn cookie_rip(entetes: &HeaderMap) -> Option<HeaderValue> {
    let garde: Vec<&str> = entetes
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .map(str::trim)
        .filter(|c| c.split_once('=').is_some_and(|(nom, _)| nom.trim() == COOKIE_RIP))
        .collect();
    (!garde.is_empty()).then(|| HeaderValue::from_str(&garde.join("; ")).ok()).flatten()
}

/// `Set-Cookie` d'un site : chemin forcé à `/s/<site>/`, `Secure` en HTTPS, `Domain` retiré.
pub fn cookie_du_site(valeur: &str, site: &str, https: bool) -> String {
    let mut parties: Vec<String> = vec![];
    for (i, p) in valeur.split(';').enumerate() {
        let t = p.trim();
        let attr = t.split('=').next().unwrap_or("").trim().to_ascii_lowercase();
        if i > 0 && matches!(attr.as_str(), "path" | "domain" | "secure") {
            continue;
        }
        parties.push(t.to_string());
    }
    parties.push(format!("Path=/s/{site}/"));
    if https {
        parties.push("Secure".into());
    }
    parties.join("; ")
}

async fn relayer(State(relais): State<Partage>, requete: Request) -> Response {
    let chemin = requete.uri().path().to_string();
    let reste = chemin.strip_prefix("/s/").unwrap_or("");
    let (site, sous) = reste.split_once('/').unwrap_or((reste, ""));
    let site = site.to_string();
    let envoi = relais.tunnels.lock().unwrap().get(&site).map(|t| t.envoi.clone());
    // Site inconnu ou hors ligne : même réponse (on ne dit pas quels sites existent).
    let Some(mut envoi) = envoi else { return refus(StatusCode::BAD_GATEWAY, "site_hors_ligne") };
    let max = relais.config.max_octets;
    let longueur = requete.headers().get(header::CONTENT_LENGTH).and_then(|v| v.to_str().ok()).and_then(|v| v.parse::<u64>().ok());
    if longueur.is_some_and(|l| l > max) {
        return refus(StatusCode::PAYLOAD_TOO_LARGE, "envoi_trop_gros");
    }
    let (parties, corps) = requete.into_parts();
    let requete_site = format!("/{sous}{}", parties.uri.query().map(|q| format!("?{q}")).unwrap_or_default());
    let Ok(uri) = format!("http://rip{requete_site}").parse::<Uri>() else { return refus(StatusCode::BAD_REQUEST, "adresse_invalide") };
    let mut sortante = axum::http::Request::builder().method(parties.method.clone()).uri(uri);
    {
        let h = sortante.headers_mut().expect("requête neuve");
        for (nom, valeur) in parties.headers.iter() {
            if !a_retirer(nom) {
                h.append(nom.clone(), valeur.clone());
            }
        }
        if let Some(c) = cookie_rip(&parties.headers) {
            h.insert(header::COOKIE, c);
        }
    }
    let corps = Body::new(http_body_util::Limited::new(corps, max as usize));
    let Ok(sortante) = sortante.body(corps) else { return refus(StatusCode::BAD_REQUEST, "requete_invalide") };
    if envoi.ready().await.is_err() {
        return refus(StatusCode::BAD_GATEWAY, "site_hors_ligne");
    }
    let reponse = match envoi.send_request(sortante).await {
        Ok(r) => r,
        Err(e) => {
            // Corps trop gros découvert en route (sans Content-Length).
            let trop_gros = std::error::Error::source(&e).is_some_and(|s| s.to_string().contains("length limit"));
            return if trop_gros || e.to_string().contains("length limit") { refus(StatusCode::PAYLOAD_TOO_LARGE, "envoi_trop_gros") } else { refus(StatusCode::BAD_GATEWAY, "site_hors_ligne") };
        }
    };
    let (mut p, corps) = reponse.into_parts();
    let https = relais.config.url_publique.starts_with("https://");
    let cookies: Vec<HeaderValue> = p.headers.get_all(header::SET_COOKIE).iter().filter_map(|v| v.to_str().ok()).filter_map(|v| HeaderValue::from_str(&cookie_du_site(v, &site, https)).ok()).collect();
    p.headers.remove(header::SET_COOKIE);
    for c in cookies {
        p.headers.append(header::SET_COOKIE, c);
    }
    for (nom, valeur) in [
        (header::CONTENT_SECURITY_POLICY, "sandbox; frame-ancestors 'none'"),
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        (header::X_FRAME_OPTIONS, "DENY"),
        (header::REFERRER_POLICY, "no-referrer"),
    ] {
        p.headers.insert(nom, HeaderValue::from_static(valeur));
    }
    p.headers.remove(header::STRICT_TRANSPORT_SECURITY);
    if parties.method == Method::HEAD {
        return Response::from_parts(p, Body::empty());
    }
    Response::from_parts(p, Body::new(corps))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cookie_limite_au_site() {
        assert_eq!(cookie_du_site("eko_rip_session=abc; HttpOnly; SameSite=Strict; Path=/; Max-Age=43200", "k3x9", true), "eko_rip_session=abc; HttpOnly; SameSite=Strict; Max-Age=43200; Path=/s/k3x9/; Secure");
        assert_eq!(cookie_du_site("a=b; Domain=e-ko.rip; secure", "s1", false), "a=b; Path=/s/s1/");
    }

    #[test]
    fn seul_le_cookie_rip_passe() {
        let mut h = HeaderMap::new();
        h.append(header::COOKIE, HeaderValue::from_static("portail_session=secret; eko_rip_session=abc"));
        h.append(header::COOKIE, HeaderValue::from_static("autre=1"));
        assert_eq!(cookie_rip(&h).unwrap(), "eko_rip_session=abc");
        let mut h = HeaderMap::new();
        h.insert(header::COOKIE, HeaderValue::from_static("x_eko_rip_session=1"));
        assert!(cookie_rip(&h).is_none());
    }
}
