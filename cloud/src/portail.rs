//! Portail web de la plate-forme (`platform.e-ko.rip`) : inscription, connexion (mot de passe,
//! Google, Facebook), « Mes sites » (rattacher un serveur par son code, retirer), compte,
//! espace super-administrateur (clés Google / Facebook, comptes, sites), pages légales.
//!
//! Pages rendues côté serveur, sans script. Toute requête POST doit venir de la plate-forme
//! elle-même (`Origin`, ou à défaut `Referer`) et, une fois connecté, porter le jeton
//! anti-falsification de la session.

use crate::base::{self, Compte, Connexion};
use crate::relais::Partage;
use crate::{appairage, sites};
use axum::Form;
use axum::extract::{Path as Chemin, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{Html, IntoResponse, Redirect, Response};
use std::collections::HashMap;
use std::time::Duration;

pub const COOKIE_SESSION: &str = "plateforme_session";
const COOKIE_OAUTH: &str = "plateforme_oauth";
/// Sites qu'un compte peut posséder (service gratuit).
pub const MAX_SITES: i64 = 10;

type Champs = Form<HashMap<String, String>>;

pub fn echapper(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#39;")
}

/// Encodage pour une URL (tout sauf les caractères non réservés).
pub fn encoder(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-._~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

fn cookie(entetes: &HeaderMap, nom: &str) -> Option<String> {
    entetes.get_all(header::COOKIE).iter().filter_map(|v| v.to_str().ok()).flat_map(|v| v.split(';')).filter_map(|c| c.trim().split_once('=')).find(|(n, _)| *n == nom).map(|(_, v)| v.to_string())
}

/// Adresse du visiteur : dernière entrée de `X-Forwarded-For` (posée par Caddy).
fn ip(entetes: &HeaderMap) -> String {
    entetes.get("x-forwarded-for").and_then(|v| v.to_str().ok()).and_then(|v| v.rsplit(',').next()).map(|v| v.trim().to_string()).unwrap_or_else(|| "local".into())
}

struct Session {
    compte: Compte,
    csrf: String,
    jeton: String,
}

fn session(r: &Partage, entetes: &HeaderMap) -> Option<Session> {
    let jeton = cookie(entetes, COOKIE_SESSION)?;
    let (compte, csrf) = base::session(&r.base(), &jeton).ok().flatten()?;
    Some(Session { compte, csrf, jeton })
}

/// Requête POST venue d'une page de la plate-forme.
fn origine_ok(r: &Partage, entetes: &HeaderMap) -> bool {
    let url = &r.config.url_publique;
    match entetes.get(header::ORIGIN).and_then(|v| v.to_str().ok()) {
        Some(o) => o == url,
        None => entetes.get(header::REFERER).and_then(|v| v.to_str().ok()).is_some_and(|v| v.starts_with(&format!("{url}/"))),
    }
}

fn csrf_ok(s: &Session, f: &HashMap<String, String>) -> bool {
    f.get("csrf").is_some_and(|c| crate::egal_constant(c, &s.csrf))
}

fn https(r: &Partage) -> bool {
    r.config.url_publique.starts_with("https://")
}

fn cookie_session(r: &Partage, jeton: &str, duree: u64) -> HeaderValue {
    let secure = if https(r) { "; Secure" } else { "" };
    HeaderValue::from_str(&format!("{COOKIE_SESSION}={jeton}; HttpOnly; SameSite=Lax; Path=/; Max-Age={duree}{secure}")).expect("cookie")
}

/// Redirige en ouvrant une session pour `compte`.
fn connecte(r: &Partage, compte: i64, suite: &str) -> Response {
    match base::ouvrir_session(&r.base(), compte) {
        Ok((jeton, _)) => {
            let mut rep = Redirect::to(suite).into_response();
            rep.headers_mut().append(header::SET_COOKIE, cookie_session(r, &jeton, base::DUREE_SESSION));
            rep
        }
        Err(_) => erreur_page(r, None, "Service momentanément indisponible."),
    }
}

/// Destination après connexion : un chemin local seulement.
fn suite_sure(s: Option<&String>) -> String {
    s.filter(|s| s.starts_with('/') && !s.starts_with("//") && !s.contains('\\')).cloned().unwrap_or_else(|| "/sites".into())
}

const STYLE: &str = r#"
:root { color-scheme: light dark; --fond: #f5f5f7; --carte: #ffffff; --texte: #1d1d1f; --doux: #6e6e73; --bord: #d2d2d7; --accent: #0071e3; --erreur: #d70015; --ok: #248a3d; }
@media (prefers-color-scheme: dark) { :root { --fond: #1c1c1e; --carte: #2c2c2e; --texte: #f5f5f7; --doux: #a1a1a6; --bord: #3a3a3c; --accent: #2997ff; --erreur: #ff453a; --ok: #30d158; } }
* { box-sizing: border-box; }
body { margin: 0; background: var(--fond); color: var(--texte); font: 16px/1.5 -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; }
header { display: flex; align-items: center; justify-content: space-between; gap: 1rem; padding: .8rem 1rem; border-bottom: 1px solid var(--bord); background: var(--carte); }
header a { color: inherit; text-decoration: none; }
header nav { display: flex; gap: 1rem; flex-wrap: wrap; align-items: center; }
header nav form { margin: 0; }
.marque { font-weight: 600; white-space: nowrap; }
main { max-width: 52rem; margin: 0 auto; padding: 1.5rem 1rem 3rem; }
.carte { background: var(--carte); border: 1px solid var(--bord); border-radius: 12px; padding: 1.2rem; margin: 0 0 1.2rem; }
h1 { font-size: 1.7rem; margin: 0 0 1rem; } h2 { font-size: 1.2rem; margin: 1.4rem 0 .6rem; }
.carte h2:first-child { margin-top: 0; }
label { display: block; margin: .7rem 0; font-weight: 500; }
input[type=text], input[type=email], input[type=password] { display: block; width: 100%; margin-top: .3rem; padding: .6rem .7rem; border: 1px solid var(--bord); border-radius: 8px; background: var(--fond); color: var(--texte); font: inherit; }
label.case { font-weight: normal; display: flex; gap: .5rem; align-items: flex-start; }
button, .bouton { display: inline-block; padding: .55rem 1rem; border: 0; border-radius: 8px; background: var(--accent); color: #fff; font: inherit; font-weight: 500; cursor: pointer; text-decoration: none; }
button.discret, .bouton.discret { background: transparent; color: var(--accent); border: 1px solid var(--bord); }
button.danger { background: var(--erreur); }
.meta { color: var(--doux); font-size: .9rem; }
.erreur { color: var(--erreur); } .ok { color: var(--ok); }
table { width: 100%; border-collapse: collapse; } th, td { text-align: left; padding: .5rem .4rem; border-bottom: 1px solid var(--bord); vertical-align: top; }
.tableau { overflow-x: auto; }
code { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: .9em; word-break: break-all; }
.ligne { display: flex; gap: .6rem; flex-wrap: wrap; align-items: center; }
.fournisseurs { display: grid; gap: .6rem; margin: 1rem 0; }
footer { max-width: 52rem; margin: 0 auto; padding: 1rem; color: var(--doux); font-size: .85rem; display: flex; gap: 1rem; flex-wrap: wrap; }
footer a { color: inherit; }
"#;

fn page(titre: &str, s: Option<&Session>, corps: &str) -> Html<String> {
    let nav = match s {
        Some(s) => format!(
            r#"<a href="/sites">Mes sites</a><a href="/compte">Mon compte</a>{}<form method="post" action="/deconnexion"><input type="hidden" name="csrf" value="{}"><button class="discret" type="submit">Déconnexion</button></form>"#,
            if s.compte.super_admin { r#"<a href="/admin">Administration</a>"# } else { "" },
            echapper(&s.csrf)
        ),
        None => r#"<a href="/connexion">Connexion</a><a class="bouton" href="/inscription">Créer un compte</a>"#.into(),
    };
    Html(format!(
        r#"<!doctype html><html lang="fr"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>{} · E-KO RIP Cloud</title><style>{STYLE}</style></head><body><header><a class="marque" href="/">E-KO RIP Cloud</a><nav>{nav}</nav></header><main>{corps}</main><footer><span>SAS GOLDEN PRINT</span><a href="/mentions-legales">Mentions légales</a><a href="/cgu">CGU</a><a href="/confidentialite">Confidentialité</a></footer></body></html>"#,
        echapper(titre)
    ))
}

fn erreur_page(r: &Partage, s: Option<&Session>, texte: &str) -> Response {
    let _ = r;
    (StatusCode::BAD_REQUEST, page("Erreur", s, &format!(r#"<div class="carte"><p class="erreur">{}</p><p><a href="/">Retour</a></p></div>"#, echapper(texte)))).into_response()
}

fn refuse_origine() -> Response {
    (StatusCode::FORBIDDEN, "Requête refusée : elle ne vient pas de la plate-forme.").into_response()
}

fn champ(f: &HashMap<String, String>, nom: &str) -> String {
    f.get(nom).map(|v| v.trim().to_string()).unwrap_or_default()
}

fn hidden_csrf(s: &Session) -> String {
    format!(r#"<input type="hidden" name="csrf" value="{}">"#, echapper(&s.csrf))
}

// ————— Accueil, inscription, connexion —————

pub async fn accueil(State(r): State<Partage>, h: HeaderMap) -> Response {
    if session(&r, &h).is_some() {
        return Redirect::to("/sites").into_response();
    }
    page(
        "Accueil",
        None,
        r#"<div class="carte"><h1>Vos serveurs RIP, partout</h1><p>E-KO RIP Cloud relie le serveur E-KO RIP Center de votre atelier à cette plate-forme par une connexion sortante sécurisée. Hors de l'atelier, vos postes et l'application mobile y accèdent <strong>sans VPN et sans ouvrir de port</strong>.</p><p>Service gratuit.</p><p class="ligne"><a class="bouton" href="/inscription">Créer un compte</a><a class="bouton discret" href="/connexion">Se connecter</a></p></div>"#,
    )
    .into_response()
}

fn boutons_fournisseurs(r: &Partage, verbe: &str) -> String {
    let b = r.base();
    let mut html = String::new();
    if base::reglage(&b, "google_client_id").ok().flatten().is_some() && base::reglage(&b, "google_secret").ok().flatten().is_some() {
        html.push_str(&format!(r#"<a class="bouton discret" href="/auth/google">{verbe} avec Google</a>"#));
    }
    if base::reglage(&b, "facebook_app_id").ok().flatten().is_some() && base::reglage(&b, "facebook_secret").ok().flatten().is_some() {
        html.push_str(&format!(r#"<a class="bouton discret" href="/auth/facebook">{verbe} avec Facebook</a>"#));
    }
    if html.is_empty() { String::new() } else { format!(r#"<div class="fournisseurs">{html}</div><p class="meta">En continuant avec Google ou Facebook, vous acceptez les <a href="/cgu">CGU</a> et la <a href="/confidentialite">politique de confidentialité</a>.</p>"#) }
}

fn formulaire_inscription(r: &Partage, erreur: &str, email: &str, nom: &str) -> String {
    format!(
        r#"<div class="carte"><h1>Créer un compte</h1>{}{}<form method="post" action="/inscription"><label>Nom ou atelier<input type="text" name="nom" required maxlength="80" value="{}"></label><label>E-mail<input type="email" name="email" required autocomplete="email" value="{}"></label><label>Mot de passe (10 caractères au moins)<input type="password" name="mot_de_passe" required minlength="10" autocomplete="new-password"></label><label class="case"><input type="checkbox" name="cgu" value="1" required> <span>J'accepte les <a href="/cgu">conditions générales d'utilisation</a> et la <a href="/confidentialite">politique de confidentialité</a>.</span></label><button type="submit">Créer mon compte</button></form><p class="meta">Déjà inscrit ? <a href="/connexion">Se connecter</a></p></div>"#,
        if erreur.is_empty() { String::new() } else { format!(r#"<p class="erreur">{}</p>"#, echapper(erreur)) },
        boutons_fournisseurs(r, "S'inscrire"),
        echapper(nom),
        echapper(email)
    )
}

pub async fn inscription(State(r): State<Partage>, h: HeaderMap) -> Response {
    if session(&r, &h).is_some() {
        return Redirect::to("/sites").into_response();
    }
    page("Créer un compte", None, &formulaire_inscription(&r, "", "", "")).into_response()
}

pub async fn inscrire(State(r): State<Partage>, h: HeaderMap, Form(f): Champs) -> Response {
    if !origine_ok(&r, &h) {
        return refuse_origine();
    }
    let (email, nom, mdp) = (champ(&f, "email"), champ(&f, "nom"), f.get("mot_de_passe").cloned().unwrap_or_default());
    let erreur = if !r.limiter(&format!("inscription:{}", ip(&h)), 10, Duration::from_secs(3600)) {
        "Trop d'inscriptions depuis cette adresse : réessayer dans une heure."
    } else if !base::email_valide(&email) {
        "E-mail invalide."
    } else if !base::nom_valide(&nom) {
        "Nom invalide (80 caractères au plus)."
    } else if !base::mot_de_passe_valide(&mdp) {
        "Mot de passe trop court : 10 caractères au moins."
    } else if champ(&f, "cgu") != "1" {
        "Il faut accepter les conditions générales d'utilisation."
    } else {
        ""
    };
    if !erreur.is_empty() {
        return (StatusCode::BAD_REQUEST, page("Créer un compte", None, &formulaire_inscription(&r, erreur, &email, &nom))).into_response();
    }
    let hachage = tokio::task::spawn_blocking(move || base::hacher(&mdp)).await.expect("hachage");
    let cree = base::creer_compte(&r.base(), &email, &nom, Some(&hachage));
    match cree {
        Ok(Some(id)) => connecte(&r, id, "/sites"),
        Ok(None) => (StatusCode::CONFLICT, page("Créer un compte", None, &formulaire_inscription(&r, "Un compte existe déjà avec cet e-mail : se connecter.", &email, &nom))).into_response(),
        Err(_) => erreur_page(&r, None, "Service momentanément indisponible."),
    }
}

fn formulaire_connexion(r: &Partage, erreur: &str, email: &str, suite: &str) -> String {
    format!(
        r#"<div class="carte"><h1>Se connecter</h1>{}{}<form method="post" action="/connexion"><input type="hidden" name="suite" value="{}"><label>E-mail<input type="email" name="email" required autocomplete="email" value="{}"></label><label>Mot de passe<input type="password" name="mot_de_passe" required autocomplete="current-password"></label><button type="submit">Se connecter</button></form><p class="meta">Pas encore de compte ? <a href="/inscription">Créer un compte</a></p></div>"#,
        if erreur.is_empty() { String::new() } else { format!(r#"<p class="erreur">{}</p>"#, echapper(erreur)) },
        boutons_fournisseurs(r, "Continuer"),
        echapper(suite),
        echapper(email)
    )
}

pub async fn connexion(State(r): State<Partage>, h: HeaderMap, Query(q): Query<HashMap<String, String>>) -> Response {
    let suite = suite_sure(q.get("suite"));
    if session(&r, &h).is_some() {
        return Redirect::to(&suite).into_response();
    }
    page("Connexion", None, &formulaire_connexion(&r, "", "", &suite)).into_response()
}

pub async fn connecter(State(r): State<Partage>, h: HeaderMap, Form(f): Champs) -> Response {
    if !origine_ok(&r, &h) {
        return refuse_origine();
    }
    let (email, mdp, suite) = (champ(&f, "email"), f.get("mot_de_passe").cloned().unwrap_or_default(), suite_sure(f.get("suite")));
    if !r.limiter(&format!("connexion:{}", ip(&h)), 30, Duration::from_secs(600)) {
        return (StatusCode::TOO_MANY_REQUESTS, page("Connexion", None, &formulaire_connexion(&r, "Trop d'essais depuis cette adresse : réessayer dans quelques minutes.", &email, &suite))).into_response();
    }
    let r2 = r.clone();
    let e2 = email.clone();
    let resultat = tokio::task::spawn_blocking(move || base::connecter(&r2.base(), &e2, &mdp)).await.expect("connexion");
    let erreur = match resultat {
        Ok(Connexion::Ok(id)) => return connecte(&r, id, &suite),
        Ok(Connexion::Refusee) => "E-mail ou mot de passe incorrect.",
        Ok(Connexion::Bloquee) => "Trop d'échecs : compte bloqué 5 minutes.",
        Ok(Connexion::Suspendu) => "Ce compte est suspendu.",
        Err(_) => "Service momentanément indisponible.",
    };
    (StatusCode::UNAUTHORIZED, page("Connexion", None, &formulaire_connexion(&r, erreur, &email, &suite))).into_response()
}

pub async fn deconnecter(State(r): State<Partage>, h: HeaderMap, Form(f): Champs) -> Response {
    if !origine_ok(&r, &h) {
        return refuse_origine();
    }
    if let Some(s) = session(&r, &h)
        && csrf_ok(&s, &f)
    {
        let _ = base::fermer_session(&r.base(), &s.jeton);
    }
    let mut rep = Redirect::to("/").into_response();
    rep.headers_mut().append(header::SET_COOKIE, cookie_session(&r, "", 0));
    rep
}

// ————— Mes sites —————

fn exiger(r: &Partage, h: &HeaderMap, ici: &str) -> Result<Session, Box<Response>> {
    session(r, h).ok_or_else(|| Box::new(Redirect::to(&format!("/connexion?suite={}", encoder(ici))).into_response()))
}

fn libelle_role(role: &str) -> &'static str {
    match role {
        "proprietaire" => "Propriétaire",
        "admin" => "Administrateur",
        "operateur" => "Opérateur",
        _ => "Lecture",
    }
}

fn date(t: Option<u64>) -> String {
    // Jour UTC (aaaa-mm-jj), sans dépendance de calendrier.
    let Some(t) = t else { return "jamais".into() };
    let jours = (t / 86400) as i64;
    let (a, m, j) = civil(jours);
    format!("{a:04}-{m:02}-{j:02}")
}

/// Jours depuis 1970-01-01 → (année, mois, jour) (algorithme de H. Hinnant).
fn civil(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

pub async fn mes_sites(State(r): State<Partage>, h: HeaderMap, Query(q): Query<HashMap<String, String>>) -> Response {
    let ici = match q.get("code") {
        Some(c) => format!("/sites?code={}", encoder(c)),
        None => "/sites".into(),
    };
    let s = match exiger(&r, &h, &ici) {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    let mes = base::sites_du_compte(&r.base(), s.compte.id).unwrap_or_default();
    let tous = sites::lire(&r.config.donnees).unwrap_or_default();
    let connectes = r.connectes();
    let mut lignes = String::new();
    for (id, role) in &mes {
        let Some(site) = tous.iter().find(|x| &x.id == id) else { continue };
        let en_ligne = connectes.contains(id);
        let retirer = if role == "proprietaire" {
            format!(r#"<form method="post" action="/sites/{}/retirer" onsubmit="return confirm('Retirer ce site ? Son accès à distance s’arrête aussitôt.')">{}<button class="discret" type="submit">Retirer</button></form>"#, echapper(id), hidden_csrf(&s))
        } else {
            String::new()
        };
        lignes.push_str(&format!(
            r#"<tr><td><strong>{}</strong><br><code>{}/s/{}</code></td><td class="{}">{}</td><td>{}</td><td>{}</td><td>{}</td></tr>"#,
            echapper(&site.nom),
            echapper(&r.config.url_publique),
            echapper(id),
            if en_ligne { "ok" } else { "meta" },
            if en_ligne { "En ligne" } else { "Hors ligne" },
            libelle_role(role),
            date(site.derniere_connexion),
            retirer
        ));
    }
    let liste = if lignes.is_empty() {
        r#"<p class="meta">Aucun site pour l'instant. Rattachez le serveur RIP de votre atelier avec le code qu'il affiche.</p>"#.to_string()
    } else {
        format!(r#"<div class="tableau"><table><thead><tr><th>Site</th><th>État</th><th>Rôle</th><th>Dernière connexion</th><th></th></tr></thead><tbody>{lignes}</tbody></table></div>"#)
    };
    let message = match q.get("ok").map(String::as_str) {
        Some("rattache") => r#"<p class="ok">Code accepté : le serveur se rattache, il apparaît ici dans quelques secondes (recharger la page).</p>"#,
        Some("retire") => r#"<p class="ok">Site retiré : son accès à distance est coupé.</p>"#,
        _ => "",
    };
    let code = q.get("code").map(|c| echapper(c)).unwrap_or_default();
    let corps = format!(
        r#"<h1>Mes sites</h1>{message}<div class="carte">{liste}</div><div class="carte"><h2>Rattacher un serveur RIP</h2><p class="meta">Sur le serveur : Système › Cloud E-KO RIP › « Connecter à la plate-forme ». Saisissez ici le code affiché (valable 10 minutes).</p><form method="post" action="/sites/rattacher" class="ligne">{}<input type="text" name="code" placeholder="ABCD-1234" required maxlength="12" value="{code}" style="max-width: 12rem; text-transform: uppercase" autocomplete="off"><button type="submit">Rattacher</button></form></div>"#,
        hidden_csrf(&s)
    );
    page("Mes sites", Some(&s), &corps).into_response()
}

pub async fn rattacher(State(r): State<Partage>, h: HeaderMap, Form(f): Champs) -> Response {
    if !origine_ok(&r, &h) {
        return refuse_origine();
    }
    let s = match exiger(&r, &h, "/sites") {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    if !csrf_ok(&s, &f) {
        return refuse_origine();
    }
    if !r.limiter(&format!("rattacher:{}", s.compte.id), 20, Duration::from_secs(3600)) {
        return erreur_page(&r, Some(&s), "Trop d'essais : réessayer dans une heure.");
    }
    let Some(code) = appairage::normaliser(&champ(&f, "code")) else { return erreur_page(&r, Some(&s), "Code invalide : forme attendue ABCD-1234.") };
    if base::nombre_de_sites(&r.base(), s.compte.id).unwrap_or(i64::MAX) >= MAX_SITES {
        return erreur_page(&r, Some(&s), &format!("Limite atteinte : {MAX_SITES} sites par compte."));
    }
    if !r.code_en_attente(&code) {
        return erreur_page(&r, Some(&s), "Code inconnu ou expiré : en demander un nouveau sur le serveur RIP.");
    }
    match appairage::valider(&r.config.donnees, &code, &format!("compte:{}", s.compte.id)) {
        Ok(_) => Redirect::to("/sites?ok=rattache").into_response(),
        Err(_) => erreur_page(&r, Some(&s), "Service momentanément indisponible."),
    }
}

pub async fn retirer(State(r): State<Partage>, h: HeaderMap, Chemin(site): Chemin<String>, Form(f): Champs) -> Response {
    if !origine_ok(&r, &h) {
        return refuse_origine();
    }
    let s = match exiger(&r, &h, "/sites") {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    if !csrf_ok(&s, &f) {
        return refuse_origine();
    }
    let proprietaire = base::droit(&r.base(), &site, s.compte.id).ok().flatten().as_deref() == Some("proprietaire");
    if !proprietaire && !s.compte.super_admin {
        return (StatusCode::FORBIDDEN, page("Refusé", Some(&s), r#"<div class="carte"><p class="erreur">Seul le propriétaire peut retirer ce site.</p></div>"#)).into_response();
    }
    r.retirer_site(&site);
    Redirect::to(if s.compte.super_admin && !proprietaire { "/admin" } else { "/sites?ok=retire" }).into_response()
}

pub async fn appairer(Query(q): Query<HashMap<String, String>>) -> Redirect {
    match q.get("code") {
        Some(c) => Redirect::to(&format!("/sites?code={}", encoder(c))),
        None => Redirect::to("/sites"),
    }
}

// ————— Mon compte —————

pub async fn mon_compte(State(r): State<Partage>, h: HeaderMap, Query(q): Query<HashMap<String, String>>) -> Response {
    let s = match exiger(&r, &h, "/compte") {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    let c = &s.compte;
    let lien = |fournisseur: &str, nom: &str, lie: bool| if lie { format!("<li>{nom} : lié</li>") } else { format!(r#"<li>{nom} : <a href="/auth/{fournisseur}">lier mon compte {nom}</a></li>"#) };
    let message = match q.get("ok").map(String::as_str) {
        Some("lie") => r#"<p class="ok">Connexion liée à votre compte.</p>"#,
        _ => "",
    };
    let corps = format!(
        r#"<h1>Mon compte</h1>{message}<div class="carte"><p><strong>{}</strong><br>{}</p><p class="meta">Inscrit le {}</p><h2>Connexions</h2><ul><li>Mot de passe : {}</li>{}{}</ul></div><div class="carte"><h2>Supprimer mon compte</h2><p>Le compte, ses droits et <strong>les sites dont vous êtes propriétaire</strong> sont effacés ; leur accès à distance s'arrête aussitôt. Irréversible.</p><form method="post" action="/compte/supprimer">{}<label class="case"><input type="checkbox" name="confirmer" value="1" required> <span>Je confirme la suppression définitive de mon compte.</span></label><button class="danger" type="submit">Supprimer mon compte</button></form></div>"#,
        echapper(&c.nom),
        echapper(&c.email),
        date(Some(c.cree_le)),
        if c.a_mot_de_passe { "oui" } else { "non (connexion par Google ou Facebook)" },
        lien("google", "Google", c.google),
        lien("facebook", "Facebook", c.facebook),
        hidden_csrf(&s)
    );
    page("Mon compte", Some(&s), &corps).into_response()
}

pub async fn supprimer_compte(State(r): State<Partage>, h: HeaderMap, Form(f): Champs) -> Response {
    if !origine_ok(&r, &h) {
        return refuse_origine();
    }
    let s = match exiger(&r, &h, "/compte") {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    if !csrf_ok(&s, &f) || champ(&f, "confirmer") != "1" {
        return refuse_origine();
    }
    let possedes: Vec<String> = base::sites_du_compte(&r.base(), s.compte.id).unwrap_or_default().into_iter().filter(|(_, role)| role == "proprietaire").map(|(id, _)| id).collect();
    for site in possedes {
        r.retirer_site(&site);
    }
    let _ = base::supprimer_compte(&r.base(), s.compte.id);
    let mut rep = page("Compte supprimé", None, r#"<div class="carte"><h1>Compte supprimé</h1><p>Votre compte et vos sites ont été effacés.</p></div>"#).into_response();
    rep.headers_mut().append(header::SET_COOKIE, cookie_session(&r, "", 0));
    rep
}

// ————— Administration —————

fn exiger_admin(r: &Partage, h: &HeaderMap) -> Result<Session, Box<Response>> {
    let s = exiger(r, h, "/admin")?;
    if !s.compte.super_admin {
        return Err(Box::new((StatusCode::FORBIDDEN, page("Refusé", Some(&s), r#"<div class="carte"><p class="erreur">Réservé à l'administration de la plate-forme.</p></div>"#)).into_response()));
    }
    Ok(s)
}

pub async fn admin(State(r): State<Partage>, h: HeaderMap, Query(q): Query<HashMap<String, String>>) -> Response {
    let s = match exiger_admin(&r, &h) {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    let url = echapper(&r.config.url_publique);
    let b = r.base();
    let valeur = |cle: &str| base::reglage(&b, cle).ok().flatten().unwrap_or_default();
    let secret = |cle: &str| if valeur(cle).is_empty() { "non renseigné" } else { "enregistré (laisser vide pour le garder)" };
    let message = match q.get("ok").map(String::as_str) {
        Some("oauth") => r#"<p class="ok">Clés enregistrées.</p>"#,
        Some("compte") => r#"<p class="ok">Compte mis à jour.</p>"#,
        _ => "",
    };
    let connectes = r.connectes();
    let tous = sites::lire(&r.config.donnees).unwrap_or_default();
    let mut lignes_sites = String::new();
    for site in &tous {
        let en_ligne = connectes.contains(&site.id);
        lignes_sites.push_str(&format!(
            r#"<tr><td><strong>{}</strong><br><code>{}</code></td><td>{}</td><td class="{}">{}</td><td>{}</td><td><form method="post" action="/sites/{}/retirer" onsubmit="return confirm('Retirer ce site ?')">{}<button class="discret" type="submit">Retirer</button></form></td></tr>"#,
            echapper(&site.nom),
            echapper(&site.id),
            echapper(&base::proprietaire(&b, &site.id).ok().flatten().or_else(|| site.proprietaire.clone()).unwrap_or_else(|| "—".into())),
            if en_ligne { "ok" } else { "meta" },
            if en_ligne { "En ligne" } else { "Hors ligne" },
            date(site.derniere_connexion),
            echapper(&site.id),
            hidden_csrf(&s)
        ));
    }
    let mut lignes_comptes = String::new();
    for (c, n) in base::lister_comptes(&b).unwrap_or_default() {
        let action = if c.super_admin {
            "Super-admin".to_string()
        } else {
            format!(
                r#"<form method="post" action="/admin/comptes/{}/suspendre">{}<input type="hidden" name="suspendu" value="{}"><button class="discret" type="submit">{}</button></form>"#,
                c.id,
                hidden_csrf(&s),
                if c.suspendu { "0" } else { "1" },
                if c.suspendu { "Rétablir" } else { "Suspendre" }
            )
        };
        lignes_comptes.push_str(&format!(
            r#"<tr><td><strong>{}</strong><br>{}</td><td>{}</td><td>{}</td><td class="{}">{}</td><td>{action}</td></tr>"#,
            echapper(&c.nom),
            echapper(&c.email),
            date(Some(c.cree_le)),
            n,
            if c.suspendu { "erreur" } else { "" },
            if c.suspendu { "Suspendu" } else { "Actif" }
        ));
    }
    let corps = format!(
        r#"<h1>Administration</h1>{message}
<div class="carte"><h2>Connexion Google</h2>
<p class="meta">Google Cloud Console › API et services › Identifiants › ID client OAuth (application Web).</p>
<p>Origine JavaScript autorisée : <code>{url}</code><br>URI de redirection autorisé : <code>{url}/auth/google/retour</code></p>
<p class="meta">Écran de consentement : page d'accueil <code>{url}/</code>, règles de confidentialité <code>{url}/confidentialite</code>, conditions d'utilisation <code>{url}/cgu</code>. Champs d'application : <code>openid</code>, <code>email</code>, <code>profile</code>.</p>
<form method="post" action="/admin/oauth">{csrf}
<label>ID client<input type="text" name="google_client_id" value="{gid}" autocomplete="off"></label>
<label>Code secret du client ({gs})<input type="password" name="google_secret" autocomplete="new-password"></label>
<h2>Connexion Facebook</h2>
<p class="meta">Meta for Developers › Mon app › Cas d'utilisation « Authentification et création de compte » › Facebook Login › Paramètres.</p>
<p>URI de redirection OAuth valide : <code>{url}/auth/facebook/retour</code><br>Domaine de l'app : <code>{domaine}</code></p>
<p class="meta">Paramètres de base : URL de la politique de confidentialité <code>{url}/confidentialite</code>, URL des conditions de service <code>{url}/cgu</code>, URL des instructions de suppression des données <code>{url}/confidentialite#suppression</code>. Autorisations : <code>email</code>, <code>public_profile</code>.</p>
<label>ID de l'app<input type="text" name="facebook_app_id" value="{fid}" autocomplete="off"></label>
<label>Clé secrète de l'app ({fs})<input type="password" name="facebook_secret" autocomplete="new-password"></label>
<label class="case"><input type="checkbox" name="effacer" value="1"> <span>Effacer les codes secrets enregistrés</span></label>
<button type="submit">Enregistrer</button></form></div>
<div class="carte"><h2>Sites ({ns})</h2><div class="tableau"><table><thead><tr><th>Site</th><th>Propriétaire</th><th>État</th><th>Dernière connexion</th><th></th></tr></thead><tbody>{lignes_sites}</tbody></table></div></div>
<div class="carte"><h2>Comptes</h2><div class="tableau"><table><thead><tr><th>Compte</th><th>Inscrit le</th><th>Sites</th><th>État</th><th></th></tr></thead><tbody>{lignes_comptes}</tbody></table></div></div>"#,
        csrf = hidden_csrf(&s),
        gid = echapper(&valeur("google_client_id")),
        gs = secret("google_secret"),
        fid = echapper(&valeur("facebook_app_id")),
        fs = secret("facebook_secret"),
        domaine = echapper(r.config.url_publique.split("://").nth(1).unwrap_or("")),
        ns = tous.len(),
    );
    page("Administration", Some(&s), &corps).into_response()
}

pub async fn regler_oauth(State(r): State<Partage>, h: HeaderMap, Form(f): Champs) -> Response {
    if !origine_ok(&r, &h) {
        return refuse_origine();
    }
    let s = match exiger_admin(&r, &h) {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    if !csrf_ok(&s, &f) {
        return refuse_origine();
    }
    let b = r.base();
    let mut ok = base::regler(&b, "google_client_id", &champ(&f, "google_client_id")).is_ok() && base::regler(&b, "facebook_app_id", &champ(&f, "facebook_app_id")).is_ok();
    for cle in ["google_secret", "facebook_secret"] {
        let v = champ(&f, cle);
        if champ(&f, "effacer") == "1" {
            ok &= base::regler(&b, cle, "").is_ok();
        } else if !v.is_empty() {
            ok &= base::regler(&b, cle, &v).is_ok();
        }
    }
    drop(b);
    if !ok {
        return erreur_page(&r, Some(&s), "Enregistrement impossible.");
    }
    Redirect::to("/admin?ok=oauth").into_response()
}

pub async fn suspendre(State(r): State<Partage>, h: HeaderMap, Chemin(id): Chemin<i64>, Form(f): Champs) -> Response {
    if !origine_ok(&r, &h) {
        return refuse_origine();
    }
    let s = match exiger_admin(&r, &h) {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    if !csrf_ok(&s, &f) {
        return refuse_origine();
    }
    let _ = base::suspendre(&r.base(), id, champ(&f, "suspendu") == "1");
    Redirect::to("/admin?ok=compte").into_response()
}

// ————— Pages légales —————

pub async fn legal(State(r): State<Partage>, h: HeaderMap, Chemin(nom): Chemin<String>) -> Response {
    let (titre, contenu) = match nom.as_str() {
        "mentions-legales" => ("Mentions légales", include_str!("../static/legal/mentions-legales.html")),
        "cgu" => ("Conditions générales d'utilisation", include_str!("../static/legal/cgu.html")),
        "confidentialite" => ("Politique de confidentialité", include_str!("../static/legal/confidentialite.html")),
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    let s = session(&r, &h);
    page(titre, s.as_ref(), &format!(r#"<div class="carte">{contenu}</div>"#)).into_response()
}

// ————— Google et Facebook —————

struct Fournisseur {
    nom: &'static str,
    autorisation: &'static str,
    portee: &'static str,
    cle_id: &'static str,
    cle_secret: &'static str,
}

fn fournisseur(nom: &str) -> Option<Fournisseur> {
    match nom {
        "google" => Some(Fournisseur { nom: "google", autorisation: "https://accounts.google.com/o/oauth2/v2/auth", portee: "openid email profile", cle_id: "google_client_id", cle_secret: "google_secret" }),
        "facebook" => Some(Fournisseur { nom: "facebook", autorisation: "https://www.facebook.com/dialog/oauth", portee: "email,public_profile", cle_id: "facebook_app_id", cle_secret: "facebook_secret" }),
        _ => None,
    }
}

fn retour(r: &Partage, f: &Fournisseur) -> String {
    format!("{}/auth/{}/retour", r.config.url_publique, f.nom)
}

pub async fn partir_oauth(State(r): State<Partage>, Chemin(nom): Chemin<String>) -> Response {
    let Some(f) = fournisseur(&nom) else { return StatusCode::NOT_FOUND.into_response() };
    let Some(id) = base::reglage(&r.base(), f.cle_id).ok().flatten() else { return erreur_page(&r, None, "Connexion non configurée.") };
    let etat = crate::aleatoire_hex(16);
    let url = format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}{}",
        f.autorisation,
        encoder(&id),
        encoder(&retour(&r, &f)),
        encoder(f.portee),
        etat,
        if f.nom == "google" { "&prompt=select_account" } else { "" }
    );
    let mut rep = Redirect::to(&url).into_response();
    let secure = if https(&r) { "; Secure" } else { "" };
    rep.headers_mut().append(header::SET_COOKIE, HeaderValue::from_str(&format!("{COOKIE_OAUTH}={}:{etat}; HttpOnly; SameSite=Lax; Path=/auth/; Max-Age=600{secure}", f.nom)).expect("cookie"));
    rep
}

/// Identité rendue par un fournisseur.
pub struct Identite {
    pub ident: String,
    pub email: Option<String>,
    /// E-mail confirmé par le fournisseur (Google : `email_verified`).
    pub email_verifie: bool,
    pub nom: String,
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder().http_status_as_error(false).timeout_global(Some(Duration::from_secs(15))).build().into()
}

fn lire_json(r: Result<ureq::http::Response<ureq::Body>, ureq::Error>) -> Option<serde_json::Value> {
    let mut r = r.ok()?;
    if !r.status().is_success() {
        return None;
    }
    r.body_mut().read_json().ok()
}

/// Échange le code contre l'identité (bloquant).
fn identite(f: &Fournisseur, id: &str, secret: &str, retour: &str, code: &str) -> Option<Identite> {
    let a = agent();
    if f.nom == "google" {
        let jeton = lire_json(a.post("https://oauth2.googleapis.com/token").send_form([("code", code), ("client_id", id), ("client_secret", secret), ("redirect_uri", retour), ("grant_type", "authorization_code")]))?;
        let acces = jeton["access_token"].as_str()?;
        let moi = lire_json(a.get("https://openidconnect.googleapis.com/v1/userinfo").header("Authorization", &format!("Bearer {acces}")).call())?;
        Some(Identite { ident: moi["sub"].as_str()?.to_string(), email: moi["email"].as_str().map(str::to_string), email_verifie: moi["email_verified"].as_bool().unwrap_or(false), nom: moi["name"].as_str().unwrap_or("").to_string() })
    } else {
        let url = format!("https://graph.facebook.com/oauth/access_token?client_id={}&redirect_uri={}&client_secret={}&code={}", encoder(id), encoder(retour), encoder(secret), encoder(code));
        let jeton = lire_json(a.get(&url).call())?;
        let acces = jeton["access_token"].as_str()?;
        let moi = lire_json(a.get(&format!("https://graph.facebook.com/me?fields=id,name,email&access_token={}", encoder(acces))).call())?;
        Some(Identite { ident: moi["id"].as_str()?.to_string(), email: moi["email"].as_str().map(str::to_string), email_verifie: false, nom: moi["name"].as_str().unwrap_or("").to_string() })
    }
}

/// Ce que donne une identité de fournisseur : connexion, liaison, création, ou refus.
pub enum Issue {
    Connecter(i64),
    Lie,
    Refus(&'static str),
}

/// - identité déjà liée : connexion (sauf compte suspendu) ;
/// - déjà connecté : liaison au compte ouvert ;
/// - e-mail inconnu : nouveau compte ;
/// - e-mail d'un compte existant : liaison seulement si le fournisseur l'a vérifié (Google),
///   sinon refus (on ne prend pas la main sur un compte avec un e-mail non prouvé).
pub fn resoudre(b: &rusqlite::Connection, fournisseur: &str, id: Identite, deja: Option<&Compte>) -> Issue {
    if let Ok(Some(c)) = base::compte_par_fournisseur(b, fournisseur, &id.ident) {
        if deja.is_some_and(|d| d.id != c.id) {
            return Issue::Refus("Ce compte Google ou Facebook est déjà lié à un autre compte de la plate-forme.");
        }
        return if c.suspendu { Issue::Refus("Ce compte est suspendu.") } else { Issue::Connecter(c.id) };
    }
    if let Some(d) = deja {
        return if base::lier_fournisseur(b, d.id, fournisseur, &id.ident).unwrap_or(false) { Issue::Lie } else { Issue::Refus("Liaison impossible.") };
    }
    let Some(email) = id.email.filter(|e| base::email_valide(e)) else { return Issue::Refus("Le fournisseur n'a pas transmis d'e-mail : créer un compte avec un mot de passe.") };
    match base::compte_par_email(b, &email) {
        Ok(Some(c)) if id.email_verifie => {
            if c.suspendu {
                return Issue::Refus("Ce compte est suspendu.");
            }
            if base::lier_fournisseur(b, c.id, fournisseur, &id.ident).unwrap_or(false) { Issue::Connecter(c.id) } else { Issue::Refus("Liaison impossible.") }
        }
        Ok(Some(_)) => Issue::Refus("Un compte existe déjà avec cet e-mail : se connecter avec le mot de passe, puis lier ce fournisseur depuis « Mon compte »."),
        Ok(None) => {
            let nom = if base::nom_valide(&id.nom) { id.nom.clone() } else { email.split('@').next().unwrap_or("Compte").to_string() };
            match base::creer_compte(b, &email, &nom, None) {
                Ok(Some(cid)) if base::lier_fournisseur(b, cid, fournisseur, &id.ident).unwrap_or(false) => Issue::Connecter(cid),
                _ => Issue::Refus("Création du compte impossible."),
            }
        }
        Err(_) => Issue::Refus("Service momentanément indisponible."),
    }
}

pub async fn retour_oauth(State(r): State<Partage>, h: HeaderMap, Chemin(nom): Chemin<String>, Query(q): Query<HashMap<String, String>>) -> Response {
    let Some(f) = fournisseur(&nom) else { return StatusCode::NOT_FOUND.into_response() };
    let attendu = cookie(&h, COOKIE_OAUTH);
    let recu = q.get("state").map(|s| format!("{}:{s}", f.nom));
    let etat_ok = matches!((&attendu, &recu), (Some(a), Some(b)) if crate::egal_constant(a, b));
    let effacer = HeaderValue::from_static("plateforme_oauth=; HttpOnly; SameSite=Lax; Path=/auth/; Max-Age=0");
    let avec_effacement = |mut rep: Response| {
        rep.headers_mut().append(header::SET_COOKIE, effacer.clone());
        rep
    };
    let s = session(&r, &h);
    if !etat_ok {
        return avec_effacement(erreur_page(&r, s.as_ref(), "Connexion interrompue ou expirée : recommencer."));
    }
    let Some(code) = q.get("code").cloned() else { return avec_effacement(erreur_page(&r, s.as_ref(), "Connexion annulée.")) };
    if !r.limiter(&format!("oauth:{}", ip(&h)), 30, Duration::from_secs(600)) {
        return avec_effacement(erreur_page(&r, s.as_ref(), "Trop d'essais : réessayer dans quelques minutes."));
    }
    let (id, secret) = {
        let b = r.base();
        (base::reglage(&b, f.cle_id).ok().flatten(), base::reglage(&b, f.cle_secret).ok().flatten())
    };
    let (Some(id), Some(secret)) = (id, secret) else { return avec_effacement(erreur_page(&r, s.as_ref(), "Connexion non configurée.")) };
    let url_retour = retour(&r, &f);
    let nom_f = f.nom;
    let ident = tokio::task::spawn_blocking(move || identite(&f, &id, &secret, &url_retour, &code)).await.ok().flatten();
    let Some(ident) = ident else { return avec_effacement(erreur_page(&r, s.as_ref(), "Le fournisseur n'a pas confirmé la connexion : recommencer.")) };
    let issue = resoudre(&r.base(), nom_f, ident, s.as_ref().map(|s| &s.compte));
    avec_effacement(match issue {
        Issue::Connecter(id) => connecte(&r, id, "/sites"),
        Issue::Lie => Redirect::to("/compte?ok=lie").into_response(),
        Issue::Refus(m) => erreur_page(&r, s.as_ref(), m),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_et_encodage() {
        assert_eq!(date(Some(0)), "1970-01-01");
        assert_eq!(date(Some(1_793_404_800)), "2026-10-31");
        assert_eq!(encoder("a b&c=é"), "a%20b%26c%3D%C3%A9");
        assert_eq!(suite_sure(Some(&"//evil.fr".into())), "/sites");
        assert_eq!(suite_sure(Some(&"/sites?code=AB".into())), "/sites?code=AB");
        assert_eq!(suite_sure(Some(&"https://evil.fr".into())), "/sites");
        assert_eq!(echapper("<a href=\"x\">'"), "&lt;a href=&quot;x&quot;&gt;&#39;");
    }

    fn base_test() -> (std::path::PathBuf, rusqlite::Connection) {
        let d = std::env::temp_dir().join(format!("eko-cloud-portail-{}", crate::aleatoire_hex(6)));
        let c = base::ouvrir(&d).unwrap();
        (d, c)
    }

    fn ident(id: &str, email: Option<&str>, verifie: bool) -> Identite {
        Identite { ident: id.into(), email: email.map(str::to_string), email_verifie: verifie, nom: "Atelier".into() }
    }

    #[test]
    fn oauth_ne_prend_pas_la_main_sur_un_compte() {
        let (d, b) = base_test();
        let existant = base::creer_compte(&b, "chef@atelier.fr", "Chef", Some("x")).unwrap().unwrap();
        // Facebook : e-mail non prouvé → refus, rien n'est lié.
        assert!(matches!(resoudre(&b, "facebook", ident("fb1", Some("chef@atelier.fr"), false), None), Issue::Refus(_)));
        assert!(base::compte_par_fournisseur(&b, "facebook", "fb1").unwrap().is_none());
        // Google, e-mail vérifié → lié au compte existant.
        assert!(matches!(resoudre(&b, "google", ident("g1", Some("chef@atelier.fr"), true), None), Issue::Connecter(i) if i == existant));
        assert!(matches!(resoudre(&b, "google", ident("g1", None, false), None), Issue::Connecter(i) if i == existant));
        // Nouvel e-mail → nouveau compte, sans mot de passe.
        let Issue::Connecter(nouveau) = resoudre(&b, "facebook", ident("fb2", Some("nouveau@atelier.fr"), false), None) else { panic!() };
        assert!(!base::compte_par_id(&b, nouveau).unwrap().unwrap().a_mot_de_passe);
        // Connecté : liaison au compte ouvert ; identité déjà prise ailleurs → refus.
        let chef = base::compte_par_id(&b, existant).unwrap().unwrap();
        assert!(matches!(resoudre(&b, "facebook", ident("fb3", None, false), Some(&chef)), Issue::Lie));
        assert!(matches!(resoudre(&b, "facebook", ident("fb2", None, false), Some(&chef)), Issue::Refus(_)));
        // Sans e-mail et non connecté → refus.
        assert!(matches!(resoudre(&b, "facebook", ident("fb9", None, false), None), Issue::Refus(_)));
        // Suspendu → refus.
        base::suspendre(&b, existant, true).unwrap();
        assert!(matches!(resoudre(&b, "google", ident("g1", None, false), None), Issue::Refus(_)));
        let _ = std::fs::remove_dir_all(&d);
    }
}
