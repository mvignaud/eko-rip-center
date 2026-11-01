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
use hyper_util::rt::{TokioExecutor, TokioIo, TokioTimer};
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
    /// Codes d'autorisation des apps en attente d'échange.
    codes_applis: Mutex<crate::applis::Codes>,
    /// Où partent les e-mails (SMTP, ou mémoire dans les tests).
    pub(crate) courriels: Mutex<crate::courriels::Expediteur>,
}

pub type Partage = Arc<Relais>;

/// Panique si `installations.json` ou la base sont illisibles (on ne repart pas de zéro en silence).
pub fn nouveau(config: Config) -> Partage {
    let registre = installations::lire(&config.donnees).expect("installations.json");
    let base = crate::base::ouvrir(&config.donnees).expect("base du portail (portail.sqlite)");
    Arc::new(Relais { config, tunnels: Default::default(), appairages: Default::default(), installations: Mutex::new(registre), generation: AtomicU64::new(0), base: Mutex::new(base), limites: Default::default(), codes_applis: Default::default(), courriels: Mutex::new(crate::courriels::Expediteur::Smtp) })
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

    pub(crate) fn codes_applis(&self) -> std::sync::MutexGuard<'_, crate::applis::Codes> {
        self.codes_applis.lock().unwrap_or_else(|p| p.into_inner())
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

    /// Retire un site : sites.json, droits, et tunnel coupé aussitôt. Si sites.json ne peut
    /// pas être écrit, rien n'est retiré (la clé du site resterait valable) : erreur.
    pub fn retirer_site(&self, site: &str) -> Result<(), String> {
        sites::supprimer(&self.config.donnees, site)?;
        let _ = crate::base::oublier_site(&self.base(), site);
        if let Some(t) = self.tunnels.lock().unwrap().remove(site) {
            t.tache.abort();
        }
        Ok(())
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
        .route("/compte/applis/deconnecter", post(portail::deconnecter_applis))
        .route("/compte/preferences", post(portail::regler_preferences))
        .route("/mot-de-passe/oublie", get(portail::oubli).post(portail::oublier))
        .route("/mot-de-passe/nouveau", get(portail::nouveau_mot_de_passe).post(portail::reinitialiser))
        .route("/email/confirmer", get(portail::confirmation).post(portail::confirmer))
        .route("/email/renvoyer", post(portail::renvoyer_confirmation))
        .route("/oauth/autoriser", get(crate::applis::autoriser).post(crate::applis::decider))
        .route("/oauth/jeton", post(crate::applis::jeton))
        .route("/api/moi", get(crate::applis::moi))
        .route("/api/moi/deconnexion", post(crate::applis::deconnecter))
        .route("/compte/supprimer", post(portail::supprimer_compte))
        .route("/admin", get(portail::admin))
        .route("/admin/oauth", post(portail::regler_oauth))
        .route("/admin/github", post(portail::regler_github))
        .route("/admin/courriels", post(portail::regler_courriels))
        .route("/admin/courriels/essai", post(portail::essai_courriel))
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
        .route("/api/certificat", get(autoriser_certificat))
        .route("/api/rapports", post(recevoir_rapport).layer(axum::extract::DefaultBodyLimit::max(crate::rapports::MAX_CORPS)))
        .route("/s/{site}", any(relayer))
        .route("/s/{site}/", any(relayer))
        .route("/s/{site}/{*reste}", any(relayer))
        .layer(axum::middleware::map_response(entetes_du_portail))
        .layer(axum::middleware::from_fn_with_state(relais.clone(), aiguiller))
        .with_state(relais)
}

/// Domaine des sites : `<site>.platform.e-ko.rip` pour `https://platform.e-ko.rip`.
pub fn domaine_des_sites(url_publique: &str) -> &str {
    let hote = url_publique.split("://").nth(1).unwrap_or(url_publique);
    hote.split('/').next().unwrap_or(hote)
}

/// Adresse publique d'un site : sa propre origine, `https://<site>.platform.e-ko.rip`.
pub fn adresse_du_site(url_publique: &str, site: &str) -> String {
    let schema = url_publique.split("://").next().unwrap_or("https");
    format!("{schema}://{site}.{}", domaine_des_sites(url_publique))
}

/// Site visé par l'en-tête `Host` (`<site>.<domaine>[:port]`), s'il y en a un.
fn site_de_l_hote(entetes: &HeaderMap, url_publique: &str) -> Option<String> {
    let hote = entetes.get(header::HOST)?.to_str().ok()?.to_ascii_lowercase();
    let domaine = domaine_des_sites(url_publique).to_ascii_lowercase();
    // Le port éventuel fait partie du domaine (essais en local) ; sinon on l'ignore.
    let hote = if domaine.contains(':') { hote.as_str() } else { hote.split(':').next().unwrap_or("") };
    let site = hote.strip_suffix(&domaine)?.strip_suffix('.')?;
    (!site.is_empty() && site.len() <= 32 && site.bytes().all(|o| o.is_ascii_lowercase() || o.is_ascii_digit())).then(|| site.to_string())
}

/// Question de Caddy avant d'obtenir un certificat à la demande (`on_demand_tls { ask … }`) :
/// oui pour la plate-forme et pour chaque site existant, non pour tout autre nom.
async fn autoriser_certificat(State(relais): State<Partage>, axum::extract::Query(q): axum::extract::Query<HashMap<String, String>>) -> StatusCode {
    let nom = q.get("domain").map(|d| d.to_ascii_lowercase()).unwrap_or_default();
    let domaine = domaine_des_sites(&relais.config.url_publique).to_ascii_lowercase();
    if nom == domaine {
        return StatusCode::OK;
    }
    let Some(site) = nom.strip_suffix(&domaine).and_then(|s| s.strip_suffix('.')) else { return StatusCode::NOT_FOUND };
    let donnees = relais.config.donnees.clone();
    let site = site.to_string();
    let existe = tokio::task::spawn_blocking(move || sites::lire(&donnees).map(|l| l.iter().any(|s| s.id == site)).unwrap_or(false)).await.unwrap_or(false);
    if existe { StatusCode::OK } else { StatusCode::NOT_FOUND }
}

/// Une requête adressée à `<site>.<domaine>` part tout entière vers ce site, avant le portail.
async fn aiguiller(State(relais): State<Partage>, requete: Request, suite: axum::middleware::Next) -> Response {
    match site_de_l_hote(requete.headers(), &relais.config.url_publique) {
        Some(site) => {
            let sous = requete.uri().path().trim_start_matches('/').to_string();
            transmettre(&relais, site, &sous, requete, Mode::Origine).await
        }
        None => suite.run(requete).await,
    }
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
        crate::evenements::entretenir(&relais).await;
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
        if !matches!(tokio::task::spawn_blocking(move || installations::ecrire(&donnees, &c)).await, Ok(Ok(()))) {
            eprintln!("installations.json non écrit : nouvel essai dans une minute");
            relais.installations.lock().unwrap().remettre_a_ecrire();
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

/// Forme de la poignée de main du tunnel.
enum Poignee {
    /// `Upgrade: websocket` : la clé `Sec-WebSocket-Accept` à rendre.
    WebSocket(String),
    /// `Upgrade: eko-tunnel/1` seul (serveurs RIP 0.30).
    Directe,
}

fn poignee(entetes: &HeaderMap) -> Option<Poignee> {
    let upgrade = entetes.get(header::UPGRADE)?.to_str().ok()?;
    if upgrade.eq_ignore_ascii_case(PROTOCOLE) {
        return Some(Poignee::Directe);
    }
    if !upgrade.eq_ignore_ascii_case("websocket") {
        return None;
    }
    let version_ok = entetes.get(header::SEC_WEBSOCKET_VERSION).is_some_and(|v| v.as_bytes() == b"13");
    let protocole_ok = entetes
        .get_all(header::SEC_WEBSOCKET_PROTOCOL)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .any(|p| p.trim().eq_ignore_ascii_case(PROTOCOLE));
    let cle = entetes.get(header::SEC_WEBSOCKET_KEY)?;
    (version_ok && protocole_ok).then(|| Poignee::WebSocket(crate::ws::acceptation(cle.as_bytes())))
}

async fn ouvrir_tunnel(State(relais): State<Partage>, mut requete: Request) -> Response {
    let Some(forme) = poignee(requete.headers()) else { return refus(StatusCode::BAD_REQUEST, "protocole_inconnu") };
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
    let websocket = matches!(forme, Poignee::WebSocket(_));
    tokio::spawn(async move {
        let Ok(flux) = montee.await else { return };
        if websocket {
            let flux = crate::ws::FluxWs::nouveau(TokioIo::new(flux), tokio_tungstenite::tungstenite::protocol::Role::Server).await;
            brancher(r, id, donnees, TokioIo::new(flux)).await;
        } else {
            brancher(r, id, donnees, flux).await;
        }
    });
    let reponse = Response::builder().status(StatusCode::SWITCHING_PROTOCOLS).header(header::CONNECTION, "upgrade");
    match forme {
        Poignee::Directe => reponse.header(header::UPGRADE, PROTOCOLE),
        Poignee::WebSocket(acceptation) => {
            reponse.header(header::UPGRADE, "websocket").header(header::SEC_WEBSOCKET_ACCEPT, acceptation).header(header::SEC_WEBSOCKET_PROTOCOL, PROTOCOLE)
        }
    }
    .body(Body::empty())
    .expect("réponse 101")
}

/// Ouvre l'HTTP/2 dans le tunnel et le garde dans la table jusqu'à sa fermeture.
async fn brancher<T>(r: Partage, id: String, donnees: PathBuf, flux: T)
where
    T: hyper::rt::Read + hyper::rt::Write + Unpin + Send + 'static,
{
    let mut b = hyper::client::conn::http2::Builder::new(TokioExecutor::new());
    b.timer(TokioTimer::new()).keep_alive_interval(Duration::from_secs(25)).keep_alive_timeout(Duration::from_secs(20)).keep_alive_while_idle(true).adaptive_window(true);
    let Ok((envoi, connexion)) = b.handshake::<_, Body>(flux).await else { return };
    let generation = r.generation.fetch_add(1, Ordering::Relaxed);
    let (fin_tx, fin_rx) = tokio::sync::oneshot::channel::<()>();
    let site = id.clone();
    let tache = tokio::spawn(async move {
        if let Err(e) = connexion.await {
            eprintln!("tunnel {site} : {}", crate::client::chaine(&e));
        }
        let _ = fin_tx.send(());
    });
    // Une nouvelle connexion du même site remplace l'ancienne (reconnexion après une
    // coupure que l'ancienne n'a pas encore vue).
    if let Some(ancien) = r.tunnels.lock().unwrap().insert(id.clone(), Tunnel { envoi, generation, tache: tache.abort_handle() }) {
        ancien.tache.abort();
    }
    eprintln!("tunnel ouvert : site {id}");
    crate::evenements::site_en_ligne(&r, &id);
    let _ = tokio::task::spawn_blocking({
        let (donnees, id) = (donnees.clone(), id.clone());
        move || sites::noter_connexion(&donnees, &id)
    })
    .await;
    let _ = fin_rx.await;
    let ferme = {
        let mut t = r.tunnels.lock().unwrap();
        let ferme = t.get(&id).is_some_and(|x| x.generation == generation);
        if ferme {
            t.remove(&id);
        }
        ferme
    };
    if ferme {
        eprintln!("tunnel fermé : site {id}");
        crate::evenements::site_hors_ligne(&r, &id);
    }
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
                    let adresse = adresse_du_site(&relais.config.url_publique, &site.id);
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
    cookie_relaye(valeur, Some(site), https)
}

/// `Set-Cookie` d'un serveur RIP : sans `Domain` (le cookie reste sur l'hôte qui répond),
/// limité à `/s/<site>/` en mode chemin, `Secure` en HTTPS.
fn cookie_relaye(valeur: &str, chemin_du_site: Option<&str>, https: bool) -> String {
    let mut parties: Vec<String> = vec![];
    for (i, p) in valeur.split(';').enumerate() {
        let t = p.trim();
        let attr = t.split('=').next().unwrap_or("").trim().to_ascii_lowercase();
        let a_retirer = match chemin_du_site {
            Some(_) => matches!(attr.as_str(), "path" | "domain" | "secure"),
            None => matches!(attr.as_str(), "domain" | "secure"),
        };
        if i > 0 && a_retirer {
            continue;
        }
        parties.push(t.to_string());
    }
    if let Some(site) = chemin_du_site {
        parties.push(format!("Path=/s/{site}/"));
    }
    if https {
        parties.push("Secure".into());
    }
    parties.join("; ")
}

/// Comment un site est servi : sous `/s/<site>/` de la plate-forme (clients d'API), ou sur
/// sa propre origine `<site>.<domaine>` (navigateur, client Windows : l'interface y tourne).
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Chemin,
    Origine,
}

async fn relayer(State(relais): State<Partage>, requete: Request) -> Response {
    let chemin = requete.uri().path().to_string();
    let reste = chemin.strip_prefix("/s/").unwrap_or("");
    let (site, sous) = reste.split_once('/').unwrap_or((reste, ""));
    let (site, sous) = (site.to_string(), sous.to_string());
    transmettre(&relais, site, &sous, requete, Mode::Chemin).await
}

/// Délai maximal pour recevoir les en-têtes de la réponse d'un site à une requête sans corps
/// (le corps de la réponse, lui, peut durer : téléchargements).
const DELAI_REPONSE_SITE: Duration = Duration::from_secs(120);

async fn transmettre(relais: &Partage, site: String, sous: &str, requete: Request, mode: Mode) -> Response {
    let envoi = relais.tunnels.lock().unwrap().get(&site).map(|t| t.envoi.clone());
    // Site inconnu ou hors ligne : même réponse (on ne dit pas quels sites existent).
    let Some(mut envoi) = envoi else { return refus(StatusCode::SERVICE_UNAVAILABLE, "site_hors_ligne") };
    let max = relais.config.max_octets;
    let longueur = requete.headers().get(header::CONTENT_LENGTH).and_then(|v| v.to_str().ok()).and_then(|v| v.parse::<u64>().ok());
    if longueur.is_some_and(|l| l > max) {
        return refus(StatusCode::PAYLOAD_TOO_LARGE, "envoi_trop_gros");
    }
    // Jeton d'app : le compte doit avoir un droit sur ce site ; le serveur RIP reçoit son
    // identité (le jeton, lui, ne sort jamais vers un atelier).
    let identite = if crate::applis::porte_jeton_appli(requete.headers()) {
        let r = relais.clone();
        let (h, s) = (requete.headers().clone(), site.clone());
        let verdict = tokio::task::spawn_blocking(move || {
            let compte = crate::applis::compte_bearer(&r, &h)?;
            let role = crate::base::droit(&r.base(), &s, compte.id).ok().flatten();
            Some((compte, role))
        })
        .await
        .ok()
        .flatten();
        match verdict {
            None => return refus(StatusCode::UNAUTHORIZED, "jeton_invalide"),
            // Même réponse qu'un site inconnu : on ne dit pas quels sites existent.
            Some((_, None)) => return refus(StatusCode::SERVICE_UNAVAILABLE, "site_hors_ligne"),
            Some((compte, Some(role))) => Some(crate::applis::entete_identite(&compte, &role)),
        }
    } else {
        None
    };
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
        if let Some(i) = identite {
            h.insert(HeaderName::from_static(crate::applis::ENTETE_IDENTITE), i);
        }
    }
    let corps = Body::new(http_body_util::Limited::new(corps, max as usize));
    let Ok(sortante) = sortante.body(corps) else { return refus(StatusCode::BAD_REQUEST, "requete_invalide") };
    // Tunnel mort mais pas encore vu (le keep-alive met jusqu'à 45 s) : on ne fait pas
    // attendre indéfiniment.
    if !matches!(tokio::time::timeout(Duration::from_secs(10), envoi.ready()).await, Ok(Ok(()))) {
        return refus(StatusCode::SERVICE_UNAVAILABLE, "site_hors_ligne");
    }
    // Sans corps à envoyer, la réponse doit venir vite ; un envoi de fichier, lui, n'a de
    // réponse qu'une fois le fichier parti.
    let delai = if matches!(parties.method, Method::GET | Method::HEAD | Method::OPTIONS | Method::DELETE) { DELAI_REPONSE_SITE } else { Duration::from_secs(24 * 3600) };
    let reponse = match tokio::time::timeout(delai, envoi.send_request(sortante)).await {
        Err(_) => return refus(StatusCode::GATEWAY_TIMEOUT, "site_muet"),
        Ok(r) => r,
    };
    let reponse = match reponse {
        Ok(r) => r,
        Err(e) => {
            // Corps trop gros découvert en route (sans Content-Length).
            let trop_gros = std::error::Error::source(&e).is_some_and(|s| s.to_string().contains("length limit"));
            return if trop_gros || e.to_string().contains("length limit") { refus(StatusCode::PAYLOAD_TOO_LARGE, "envoi_trop_gros") } else { refus(StatusCode::SERVICE_UNAVAILABLE, "site_hors_ligne") };
        }
    };
    let (mut p, corps) = reponse.into_parts();
    let https = relais.config.url_publique.starts_with("https://");
    let cookies: Vec<HeaderValue> = p.headers.get_all(header::SET_COOKIE).iter().filter_map(|v| v.to_str().ok()).filter_map(|v| HeaderValue::from_str(&cookie_relaye(v, (mode == Mode::Chemin).then_some(site.as_str()), https)).ok()).collect();
    p.headers.remove(header::SET_COOKIE);
    for c in cookies {
        p.headers.append(header::SET_COOKIE, c);
    }
    // Sous `/s/<site>/`, la page partage l'origine du portail : `sandbox` l'en isole (pas de
    // script). Sur sa propre origine, l'interface du serveur RIP tourne normalement.
    if mode == Mode::Chemin {
        p.headers.insert(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static("sandbox; frame-ancestors 'none'"));
    } else if !p.headers.contains_key(header::CONTENT_SECURITY_POLICY) {
        p.headers.insert(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static("frame-ancestors 'none'"));
    }
    for (nom, valeur) in [
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
    fn site_vise_par_l_hote() {
        let h = |v: &'static str| {
            let mut h = HeaderMap::new();
            h.insert(header::HOST, HeaderValue::from_static(v));
            h
        };
        let url = "https://platform.e-ko.rip";
        assert_eq!(site_de_l_hote(&h("k3x9ab.platform.e-ko.rip"), url).as_deref(), Some("k3x9ab"));
        assert_eq!(site_de_l_hote(&h("K3X9AB.Platform.E-KO.rip:443"), url).as_deref(), Some("k3x9ab"));
        assert_eq!(site_de_l_hote(&h("platform.e-ko.rip"), url), None);
        assert_eq!(site_de_l_hote(&h("a.b.platform.e-ko.rip"), url), None);
        assert_eq!(site_de_l_hote(&h("k3x9abplatform.e-ko.rip"), url), None);
        assert_eq!(site_de_l_hote(&h("k3x9ab.platform.e-ko.rip.evil.test"), url), None);
        assert_eq!(adresse_du_site(url, "k3x9ab"), "https://k3x9ab.platform.e-ko.rip");
        assert_eq!(site_de_l_hote(&h("s1.127.0.0.1:8650"), "http://127.0.0.1:8650").as_deref(), Some("s1"));
    }

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
