//! « Se connecter au cloud » depuis les apps (Mac, Linux, Windows, mobile) : OAuth 2 avec
//! code d'autorisation et PKCE (RFC 7636), pour des clients publics (RFC 8252).
//!
//! 1. L'app ouvre le navigateur sur `/oauth/autoriser?client_id=eko-rip-center&redirect_uri=…
//!    &code_challenge=…&code_challenge_method=S256&state=…` ; la personne se connecte au
//!    portail (mot de passe, Google, Facebook) et accepte.
//! 2. Le navigateur revient vers l'app (`http://127.0.0.1:<port>/…` ou `eko-rip://…`) avec
//!    `code` et `state`.
//! 3. L'app échange le code (`POST /oauth/jeton`, avec `code_verifier`) contre un jeton
//!    `ekoa_…` (180 jours), qu'elle garde dans le trousseau du système.
//! 4. `GET /api/moi` (Bearer) liste ses sites ; l'app ouvre `https://<site>.platform.e-ko.rip`
//!    avec le même Bearer. Le relais vérifie le droit sur le site, retire le jeton et passe au
//!    serveur RIP l'identité (`X-EKO-Cloud-Identite`) : pas de second mot de passe.

use crate::base;
use crate::portail::{echapper, encoder};
use crate::relais::Partage;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::{Form, Json};
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub const CLIENT_ID: &str = "eko-rip-center";
/// En-tête d'identité posé par le relais vers un serveur RIP (base64url d'un JSON).
pub const ENTETE_IDENTITE: &str = "x-eko-cloud-identite";
const DUREE_CODE: Duration = Duration::from_secs(120);

/// Code d'autorisation en attente d'échange (en mémoire, usage unique).
pub struct Code {
    compte: i64,
    redirection: String,
    defi: String,
    appareil: String,
    cree: Instant,
}

#[derive(Default)]
pub struct Codes(HashMap<String, Code>);

impl Codes {
    fn ajouter(&mut self, code: String, c: Code) {
        self.0.retain(|_, c| c.cree.elapsed() < DUREE_CODE);
        if self.0.len() < 10_000 {
            self.0.insert(code, c);
        }
    }

    fn prendre(&mut self, code: &str) -> Option<Code> {
        self.0.remove(code).filter(|c| c.cree.elapsed() < DUREE_CODE)
    }
}

/// Adresses de retour acceptées : boucle locale avec port (RFC 8252 § 7.3) ou schéma propre
/// aux apps (`eko-rip://`, `ekorip://` pour le mobile).
pub fn redirection_valide(uri: &str) -> bool {
    if uri.len() > 300 || uri.contains(['#', ' ', '\\', '"', '<', '>']) {
        return false;
    }
    if let Some(reste) = uri.strip_prefix("eko-rip://").or_else(|| uri.strip_prefix("ekorip://")) {
        return !reste.is_empty();
    }
    let Some(reste) = uri.strip_prefix("http://") else { return false };
    let hote_port = reste.split(['/', '?']).next().unwrap_or("");
    let Some((hote, port)) = hote_port.rsplit_once(':') else { return false };
    matches!(hote, "127.0.0.1" | "[::1]" | "localhost") && port.parse::<u16>().is_ok_and(|p| p > 0)
}

/// `code_challenge` S256 : base64url sans remplissage d'une empreinte SHA-256 (43 caractères).
fn defi_valide(defi: &str) -> bool {
    defi.len() == 43 && defi.bytes().all(|o| o.is_ascii_alphanumeric() || o == b'-' || o == b'_')
}

fn verificateur_valide(v: &str) -> bool {
    (43..=128).contains(&v.len()) && v.bytes().all(|o| o.is_ascii_alphanumeric() || matches!(o, b'-' | b'.' | b'_' | b'~'))
}

pub fn defi_de(verificateur: &str) -> String {
    use sha2::Digest;
    crate::base64url(&sha2::Sha256::digest(verificateur.as_bytes()))
}

fn ajouter_parametres(uri: &str, params: &[(&str, &str)]) -> String {
    let sep = if uri.contains('?') { '&' } else { '?' };
    let q: Vec<String> = params.iter().map(|(k, v)| format!("{k}={}", encoder(v))).collect();
    format!("{uri}{sep}{}", q.join("&"))
}

/// Demande valide (paramètres de `/oauth/autoriser`), sinon le message d'erreur.
struct Demande {
    redirection: String,
    defi: String,
    etat: String,
    appareil: String,
}

fn lire_demande(q: &HashMap<String, String>) -> Result<Demande, &'static str> {
    let v = |k: &str| q.get(k).map(String::as_str).unwrap_or("");
    if v("client_id") != CLIENT_ID {
        return Err("Application inconnue.");
    }
    if !redirection_valide(v("redirect_uri")) {
        return Err("Adresse de retour refusée.");
    }
    if v("response_type") != "code" || v("code_challenge_method") != "S256" || !defi_valide(v("code_challenge")) {
        return Err("Demande de connexion invalide.");
    }
    let etat = v("state");
    if etat.is_empty() || etat.len() > 200 {
        return Err("Demande de connexion invalide.");
    }
    let appareil: String = v("appareil").chars().filter(|c| !c.is_control()).take(80).collect();
    Ok(Demande { redirection: v("redirect_uri").into(), defi: v("code_challenge").into(), etat: etat.into(), appareil: if appareil.is_empty() { "App E-KO RIP Center".into() } else { appareil } })
}

fn champs_caches(d: &Demande) -> String {
    [("client_id", CLIENT_ID), ("response_type", "code"), ("code_challenge_method", "S256"), ("redirect_uri", d.redirection.as_str()), ("code_challenge", d.defi.as_str()), ("state", d.etat.as_str()), ("appareil", d.appareil.as_str())]
        .iter()
        .map(|(k, v)| format!(r#"<input type="hidden" name="{k}" value="{}">"#, echapper(v)))
        .collect()
}

/// Page de consentement (connexion au portail d'abord si besoin).
pub async fn autoriser(State(r): State<Partage>, h: HeaderMap, Query(q): Query<HashMap<String, String>>) -> Response {
    let d = match lire_demande(&q) {
        Ok(d) => d,
        Err(m) => return crate::portail::page_erreur(&r, &h, m),
    };
    let Some((compte, csrf)) = crate::portail::session_ouverte(&r, &h) else {
        let ici = format!("/oauth/autoriser?{}", q.iter().map(|(k, v)| format!("{k}={}", encoder(v))).collect::<Vec<_>>().join("&"));
        return Redirect::to(&format!("/connexion?suite={}", encoder(&ici))).into_response();
    };
    let corps = format!(
        r#"<div class="carte"><h1>Connecter l'app</h1><p><strong>{}</strong> demande l'accès à vos sites E-KO RIP Cloud, au nom de <strong>{}</strong> ({}).</p><p class="meta">L'app pourra ouvrir les serveurs RIP auxquels vous avez accès, avec vos droits. Vous pourrez la déconnecter depuis « Mon compte ».</p><form method="post" action="/oauth/autoriser"><input type="hidden" name="csrf" value="{}">{}<button type="submit" name="decision" value="oui">Autoriser</button> <button class="discret" type="submit" name="decision" value="non">Annuler</button></form></div>"#,
        echapper(&d.appareil),
        echapper(&compte.nom),
        echapper(&compte.email),
        echapper(&csrf),
        champs_caches(&d)
    );
    crate::portail::page_publique("Connecter l'app", &r, &h, &corps)
}

/// Retour vers l'app par une page (un formulaire ne peut pas rediriger hors de la
/// plate-forme : `form-action 'self'`).
fn retour_vers_app(uri: &str) -> Response {
    let html = format!(
        r#"<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width"><meta http-equiv="refresh" content="0;url={u}"><title>Retour à l'app</title><body style="font:15px -apple-system,system-ui,sans-serif;text-align:center;padding:3em"><p>Retour à l'app E-KO RIP Center…</p><p><a href="{u}">Continuer</a></p><p style="color:#86868b">Vous pouvez fermer cet onglet.</p></body>"#,
        u = echapper(uri)
    );
    let mut rep = Html(html).into_response();
    rep.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    rep
}

pub async fn decider(State(r): State<Partage>, h: HeaderMap, Form(f): Form<HashMap<String, String>>) -> Response {
    if !crate::portail::origine_acceptee(&r, &h) {
        return (StatusCode::FORBIDDEN, "Origine refusée.").into_response();
    }
    let d = match lire_demande(&f) {
        Ok(d) => d,
        Err(m) => return crate::portail::page_erreur(&r, &h, m),
    };
    let Some((compte, csrf)) = crate::portail::session_ouverte(&r, &h) else { return Redirect::to("/connexion").into_response() };
    if !f.get("csrf").is_some_and(|c| crate::egal_constant(c, &csrf)) {
        return (StatusCode::FORBIDDEN, "Formulaire expiré : recommencer depuis l'app.").into_response();
    }
    if f.get("decision").map(String::as_str) != Some("oui") {
        return retour_vers_app(&ajouter_parametres(&d.redirection, &[("error", "access_denied"), ("state", &d.etat)]));
    }
    let code = crate::aleatoire_hex(32);
    r.codes_applis().ajouter(code.clone(), Code { compte: compte.id, redirection: d.redirection.clone(), defi: d.defi, appareil: d.appareil, cree: Instant::now() });
    retour_vers_app(&ajouter_parametres(&d.redirection, &[("code", &code), ("state", &d.etat)]))
}

fn erreur_oauth(statut: StatusCode, code: &str) -> Response {
    let mut rep = (statut, Json(serde_json::json!({ "error": code }))).into_response();
    rep.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    rep
}

/// Échange du code contre un jeton (`application/x-www-form-urlencoded`, RFC 6749 § 4.1.3).
pub async fn jeton(State(r): State<Partage>, h: HeaderMap, Form(f): Form<HashMap<String, String>>) -> Response {
    let ip = crate::portail::adresse_visiteur(&h);
    if !r.limiter(&format!("jeton:{ip}"), 30, Duration::from_secs(600)) {
        return erreur_oauth(StatusCode::TOO_MANY_REQUESTS, "slow_down");
    }
    let v = |k: &str| f.get(k).map(String::as_str).unwrap_or("");
    if v("grant_type") != "authorization_code" {
        return erreur_oauth(StatusCode::BAD_REQUEST, "unsupported_grant_type");
    }
    if v("client_id") != CLIENT_ID {
        return erreur_oauth(StatusCode::BAD_REQUEST, "invalid_client");
    }
    let Some(code) = r.codes_applis().prendre(v("code")) else { return erreur_oauth(StatusCode::BAD_REQUEST, "invalid_grant") };
    let verificateur = v("code_verifier");
    if code.redirection != v("redirect_uri") || !verificateur_valide(verificateur) || !crate::egal_constant(&defi_de(verificateur), &code.defi) {
        return erreur_oauth(StatusCode::BAD_REQUEST, "invalid_grant");
    }
    let r2 = r.clone();
    let cree = tokio::task::spawn_blocking(move || {
        let b = r2.base();
        // Compte supprimé ou suspendu entre-temps : pas de jeton.
        match base::compte_par_id(&b, code.compte) {
            Ok(Some(c)) if !c.suspendu => base::creer_jeton_appli(&b, code.compte, &code.appareil).ok().map(|j| (j, c, code.appareil)),
            _ => None,
        }
    })
    .await
    .ok()
    .flatten();
    let Some((jeton, titulaire, appareil)) = cree else { return erreur_oauth(StatusCode::BAD_REQUEST, "invalid_grant") };
    // Notification de sécurité au titulaire (hors de la réponse, jamais bloquante).
    r.envoyer(crate::courriels::nouvelle_app(&r.config.url_publique, &titulaire.email, &titulaire.nom, &appareil, crate::maintenant()));
    let mut rep = Json(serde_json::json!({ "access_token": jeton, "token_type": "Bearer", "expires_in": base::DUREE_JETON_APPLI })).into_response();
    rep.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    rep
}

fn bearer(h: &HeaderMap) -> Option<&str> {
    h.get(header::AUTHORIZATION)?.to_str().ok()?.strip_prefix("Bearer ").map(str::trim)
}

/// Compte d'un jeton d'app présenté en `Authorization: Bearer`.
pub fn compte_bearer(r: &Partage, h: &HeaderMap) -> Option<base::Compte> {
    let j = bearer(h)?;
    base::compte_du_jeton(&r.base(), j).ok().flatten()
}

/// Le jeton présenté est-il un jeton d'app (même invalide) ?
pub fn porte_jeton_appli(h: &HeaderMap) -> bool {
    bearer(h).is_some_and(|j| j.starts_with(base::PREFIXE_JETON_APPLI))
}

fn non_autorise() -> Response {
    let mut rep = (StatusCode::UNAUTHORIZED, Json(serde_json::json!({ "code": "jeton_invalide" }))).into_response();
    rep.headers_mut().insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
    rep
}

/// Le compte et ses sites, pour l'écran « Mes sites » des apps.
pub async fn moi(State(r): State<Partage>, h: HeaderMap) -> Response {
    let Some(compte) = compte_bearer(&r, &h) else { return non_autorise() };
    let droits = base::sites_du_compte(&r.base(), compte.id).unwrap_or_default();
    let tous = crate::sites::lire(&r.config.donnees).unwrap_or_default();
    let connectes = r.connectes();
    let sites: Vec<serde_json::Value> = droits
        .iter()
        .filter_map(|(id, role)| {
            let s = tous.iter().find(|s| &s.id == id)?;
            Some(serde_json::json!({ "id": id, "nom": s.nom, "adresse": crate::relais::adresse_du_site(&r.config.url_publique, id), "en_ligne": connectes.contains(id), "role": role }))
        })
        .collect();
    Json(serde_json::json!({ "compte": { "email": compte.email, "nom": compte.nom }, "sites": sites })).into_response()
}

/// Déconnexion de l'app : son jeton est révoqué.
pub async fn deconnecter(State(r): State<Partage>, h: HeaderMap) -> Response {
    if let Some(j) = bearer(&h).filter(|j| j.starts_with(base::PREFIXE_JETON_APPLI)) {
        let _ = base::revoquer_jeton_appli(&r.base(), j);
    }
    StatusCode::NO_CONTENT.into_response()
}

/// Rôle transmis au serveur RIP : `admin`, `operateur` ou `lecture`.
pub fn role_pour_rip(role: &str) -> &'static str {
    match role {
        "proprietaire" | "admin" => "admin",
        "operateur" => "operateur",
        _ => "lecture",
    }
}

/// Valeur de `X-EKO-Cloud-Identite` : base64url de `{"compte","email","nom","role"}`.
pub fn entete_identite(compte: &base::Compte, role: &str) -> HeaderValue {
    let json = serde_json::json!({ "compte": compte.id, "email": compte.email, "nom": compte.nom, "role": role_pour_rip(role) });
    HeaderValue::from_str(&crate::base64url(json.to_string().as_bytes())).expect("base64url")
}

/// Identité reçue par un serveur RIP (seulement par le tunnel : ailleurs, l'en-tête est
/// ignoré).
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct IdentiteCloud {
    pub compte: i64,
    pub email: String,
    pub nom: String,
    /// `admin`, `operateur` ou `lecture`.
    pub role: String,
}

pub fn lire_identite(valeur: &str) -> Option<IdentiteCloud> {
    serde_json::from_slice(&crate::base64url_decoder(valeur)?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identite_aller_retour() {
        let c = base::Compte { id: 7, email: "a@b.fr".into(), nom: "Élodie Ünal".into(), super_admin: false, suspendu: false, cree_le: 0, a_mot_de_passe: true, google: false, facebook: false, email_verifie: true, alertes: true, resume: true };
        let v = entete_identite(&c, "proprietaire");
        let i = lire_identite(v.to_str().unwrap()).unwrap();
        assert_eq!(i, IdentiteCloud { compte: 7, email: "a@b.fr".into(), nom: "Élodie Ünal".into(), role: "admin".into() });
        assert_eq!(lire_identite("pas du base64 !"), None);
        for n in 0..10 {
            let o: Vec<u8> = (0..n).collect();
            assert_eq!(crate::base64url_decoder(&crate::base64url(&o)).unwrap(), o);
        }
    }

    #[test]
    fn adresses_de_retour() {
        for ok in ["http://127.0.0.1:53124/retour", "http://127.0.0.1:8080", "http://[::1]:9000/x?y=1", "http://localhost:4000/", "eko-rip://oauth", "ekorip://connexion"] {
            assert!(redirection_valide(ok), "{ok}");
        }
        for non in ["https://evil.test/", "http://127.0.0.1/retour", "http://127.0.0.1:0/", "http://127.0.0.1.evil.test:80/", "http://evil.test:80/127.0.0.1", "javascript:alert(1)", "eko-rip://", "http://127.0.0.1:80/#x", "http://user@127.0.0.1:80/"] {
            assert!(!redirection_valide(non), "{non}");
        }
    }

    #[test]
    fn pkce_rfc7636() {
        // Exemple de la RFC 7636, annexe B.
        assert_eq!(defi_de("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"), "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
        assert!(defi_valide("E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"));
        assert!(!defi_valide("court"));
    }

    #[test]
    fn roles_transmis() {
        assert_eq!(role_pour_rip("proprietaire"), "admin");
        assert_eq!(role_pour_rip("admin"), "admin");
        assert_eq!(role_pour_rip("operateur"), "operateur");
        assert_eq!(role_pour_rip("lecture"), "lecture");
        assert_eq!(role_pour_rip("inconnu"), "lecture");
    }
}
