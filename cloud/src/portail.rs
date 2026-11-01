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

/// Noms des cookies du portail. En HTTPS, préfixe `__Host-` : un site servi sur
/// `<site>.platform.e-ko.rip` ne peut ni les poser ni les écraser depuis son sous-domaine.
fn nom_cookie(r: &Partage, nom: &'static str) -> String {
    if https(r) { format!("__Host-{nom}") } else { nom.to_string() }
}
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

/// Inverse d'`encoder` (séquences `%XX` ; invalide : texte vide).
fn decoder(s: &str) -> String {
    let o = s.as_bytes();
    let mut v = Vec::with_capacity(o.len());
    let mut i = 0;
    while i < o.len() {
        if o[i] == b'%' {
            let Some(b) = s.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok()) else { return String::new() };
            v.push(b);
            i += 3;
        } else {
            v.push(o[i]);
            i += 1;
        }
    }
    String::from_utf8(v).unwrap_or_default()
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
    let jeton = cookie(entetes, &nom_cookie(r, COOKIE_SESSION))?;
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
    HeaderValue::from_str(&format!("{}={jeton}; HttpOnly; SameSite=Lax; Path=/; Max-Age={duree}{secure}", nom_cookie(r, COOKIE_SESSION))).expect("cookie")
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

/// Feuille de style du portail : celle des clients natifs (macOS, GTK) — barre latérale
/// translucide, barre d'outils unifiée, listes groupées arrondies, accent bleu système.
const STYLE: &str = r#"
:root { color-scheme: light dark; --fenetre: #ececec; --fond: #f5f5f7; --groupe: #ffffff; --barre: rgba(232,232,237,.82); --laterale: rgba(226,226,231,.78); --texte: #1d1d1f; --doux: #86868b; --trait: rgba(0,0,0,.1); --champ: #ffffff; --champ-bord: rgba(0,0,0,.16); --accent: #007aff; --accent-doux: rgba(0,122,255,.14); --selection: rgba(0,0,0,.08); --erreur: #ff3b30; --ok: #34c759; --ombre: 0 1px 2px rgba(0,0,0,.06), 0 0 0 .5px rgba(0,0,0,.06); }
@media (prefers-color-scheme: dark) { :root { --fenetre: #1e1e1e; --fond: #1e1e1e; --groupe: #2a2a2c; --barre: rgba(42,42,44,.82); --laterale: rgba(38,38,40,.8); --texte: #f5f5f7; --doux: #98989d; --trait: rgba(255,255,255,.1); --champ: rgba(255,255,255,.06); --champ-bord: rgba(255,255,255,.14); --accent: #0a84ff; --accent-doux: rgba(10,132,255,.22); --selection: rgba(255,255,255,.1); --erreur: #ff453a; --ok: #30d158; --ombre: 0 0 0 .5px rgba(255,255,255,.08); } }
* { box-sizing: border-box; }
html { background: var(--fond); }
body { margin: 0; min-height: 100vh; background: var(--fond); color: var(--texte); font: 15px/1.45 -apple-system, BlinkMacSystemFont, "SF Pro Text", "Inter", "Segoe UI Variable", "Segoe UI", system-ui, sans-serif; -webkit-font-smoothing: antialiased; }
a { color: var(--accent); text-decoration: none; } a:hover { text-decoration: underline; }
.rosace { width: 28px; height: 28px; flex: none; }
/* Fenêtre : barre latérale + contenu (connecté). */
.app { display: grid; grid-template-columns: 236px minmax(0, 1fr); min-height: 100vh; }
.laterale { position: sticky; top: 0; height: 100vh; display: flex; flex-direction: column; gap: .2rem; padding: .9rem .6rem; background: var(--laterale); -webkit-backdrop-filter: saturate(180%) blur(20px); backdrop-filter: saturate(180%) blur(20px); border-right: .5px solid var(--trait); }
.marque { display: flex; align-items: center; gap: .55rem; padding: .2rem .5rem .9rem; color: var(--texte); font-weight: 600; font-size: 14px; white-space: nowrap; }
.marque:hover { text-decoration: none; }
.rubrique { padding: .7rem .6rem .25rem; color: var(--doux); font-size: 11px; font-weight: 600; letter-spacing: .02em; }
.entree { display: flex; align-items: center; gap: .55rem; padding: .32rem .6rem; border-radius: 6px; color: var(--texte); font-size: 14px; }
.entree:hover { text-decoration: none; background: var(--selection); }
.entree svg { width: 17px; height: 17px; flex: none; color: var(--accent); }
.entree.active { background: var(--accent); color: #fff; } .entree.active svg { color: #fff; }
.laterale .bas { margin-top: auto; display: grid; gap: .5rem; padding: .4rem .2rem 0; }
.laterale .bas form { margin: 0; }
.laterale .bas button { width: 100%; }
.liens { display: flex; flex-wrap: wrap; gap: .2rem .7rem; padding: 0 .4rem; color: var(--doux); font-size: 11px; }
.liens a { color: inherit; }
.contenu { min-width: 0; }
.outils { position: sticky; top: 0; z-index: 2; display: flex; align-items: center; justify-content: center; height: 52px; padding: 0 1rem; background: var(--barre); -webkit-backdrop-filter: saturate(180%) blur(20px); backdrop-filter: saturate(180%) blur(20px); border-bottom: .5px solid var(--trait); font-size: 14px; font-weight: 600; }
.app main { max-width: 46rem; margin: 0 auto; padding: 1.4rem 1.2rem 3rem; }
.app main > h1:first-child { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); }
/* Feuille centrée (accueil, connexion, inscription). */
.feuille { min-height: 100vh; display: flex; flex-direction: column; align-items: center; padding: 3rem 1rem 1.5rem; background: var(--fenetre); }
.feuille .tete { display: flex; flex-direction: column; align-items: center; gap: .6rem; margin-bottom: 1.4rem; color: var(--texte); font-size: 17px; font-weight: 600; }
.feuille .tete:hover { text-decoration: none; }
.feuille .tete .rosace { width: 72px; height: 72px; }
.feuille main { width: 100%; max-width: 27rem; }
.feuille main.large { max-width: 46rem; }
.feuille .liens { justify-content: center; margin-top: auto; padding-top: 2rem; }
/* Listes groupées (Form .grouped). */
.carte { background: var(--groupe); border-radius: 10px; box-shadow: var(--ombre); padding: 1rem 1.1rem; margin: 0 0 1.1rem; }
.carte > :first-child { margin-top: 0; } .carte > :last-child { margin-bottom: 0; }
h1 { font-size: 22px; font-weight: 700; letter-spacing: -.01em; margin: 0 0 .8rem; }
h2 { font-size: 13px; font-weight: 600; margin: 1.3rem 0 .5rem; }
.carte h2 { font-size: 15px; }
p { margin: .5rem 0; }
ul { padding-left: 1.2rem; }
label { display: block; margin: .75rem 0; font-size: 13px; font-weight: 500; }
input[type=text], input[type=email], input[type=password], select { display: block; width: 100%; margin-top: .3rem; min-height: 30px; padding: .3rem .6rem; border: .5px solid var(--champ-bord); border-radius: 6px; background: var(--champ); color: var(--texte); font: inherit; font-size: 14px; box-shadow: inset 0 .5px 1px rgba(0,0,0,.06); }
input:focus-visible { outline: 3.5px solid color-mix(in srgb, var(--accent) 45%, transparent); outline-offset: 0; border-color: var(--accent); }
input[type=checkbox] { accent-color: var(--accent); width: 15px; height: 15px; margin: .15rem 0 0; flex: none; }
label.case { font-weight: normal; display: flex; gap: .5rem; align-items: flex-start; }
button, .bouton { display: inline-flex; align-items: center; justify-content: center; gap: .4rem; min-height: 30px; padding: .3rem .9rem; border: 0; border-radius: 6px; background: var(--accent); color: #fff; font: inherit; font-size: 13px; font-weight: 500; cursor: pointer; text-decoration: none; box-shadow: 0 .5px 1px rgba(0,0,0,.15); }
button:hover, .bouton:hover { text-decoration: none; filter: brightness(1.06); }
button:focus-visible, .bouton:focus-visible { outline: 3.5px solid color-mix(in srgb, var(--accent) 45%, transparent); outline-offset: 1px; }
form > button[type=submit] { margin-top: .4rem; }
button.discret, .bouton.discret { background: var(--champ); color: var(--texte); box-shadow: 0 0 0 .5px var(--champ-bord), 0 .5px 1px rgba(0,0,0,.08); }
button.danger { background: var(--erreur); }
.meta { color: var(--doux); font-size: 13px; }
.erreur { color: var(--erreur); } .ok { color: var(--ok); }
p.erreur, p.ok { padding: .55rem .8rem; border-radius: 8px; font-size: 13px; background: color-mix(in srgb, currentColor 12%, transparent); }
table { width: 100%; border-collapse: collapse; font-size: 13px; }
th { text-align: left; padding: .35rem .5rem; color: var(--doux); font-size: 11px; font-weight: 600; border-bottom: .5px solid var(--trait); }
td { text-align: left; padding: .55rem .5rem; border-bottom: .5px solid var(--trait); vertical-align: middle; }
tr:last-child td { border-bottom: 0; }
td form { margin: 0; }
td.ok, td.meta { white-space: nowrap; color: var(--texte); }
td.ok::before, td.meta::before { content: ""; display: inline-block; width: 8px; height: 8px; margin-right: .4rem; border-radius: 50%; background: var(--doux); vertical-align: 1px; }
td.ok::before { background: var(--ok); }
.tableau { overflow-x: auto; margin: 0 -.5rem; }
code { font-family: ui-monospace, "SF Mono", SFMono-Regular, Menlo, monospace; font-size: 12px; word-break: break-all; color: var(--doux); }
.ligne { display: flex; gap: .6rem; flex-wrap: wrap; align-items: center; }
.ligne input { margin-top: 0; }
.fournisseurs { display: grid; gap: .5rem; margin: 1rem 0 .6rem; }
.fournisseurs .bouton { min-height: 34px; font-size: 14px; }
/* Pages légales : texte courant. */
.texte h1 { font-size: 22px; } .texte h2 { font-size: 15px; margin-top: 1.5rem; } .texte p, .texte li { font-size: 14px; }
@media (max-width: 760px) {
  .app { display: block; }
  .laterale { position: sticky; top: 0; z-index: 3; height: auto; flex-direction: row; flex-wrap: wrap; align-items: center; gap: .3rem; padding: .5rem .7rem; border-right: 0; border-bottom: .5px solid var(--trait); }
  .marque { padding: .2rem .3rem; flex: 1 0 100%; }
  .rubrique, .liens, .outils { display: none; }
  .entree { padding: .3rem .55rem; font-size: 13px; }
  .entree svg { width: 15px; height: 15px; }
  .laterale .bas { margin: 0 0 0 auto; padding: 0; }
  .laterale .bas button { width: auto; }
  .app main { padding: 1rem .8rem 2rem; }
  .app main > h1:first-child { position: static; width: auto; height: auto; clip: auto; }
  .app::after { content: ""; }
  .pied-mobile { display: flex !important; }
  .feuille { padding-top: 2rem; }
}
.pied-mobile { display: none; justify-content: center; padding: 0 1rem 1.5rem; }
"#;

/// L'icône de l'application (rosace CMJN des clients natifs).
const ROSACE: &str = r##"<svg class="rosace" viewBox="0 0 1024 1024" aria-hidden="true"><path d="M512 100c205 0 260 0 312 52s52 107 52 312v96c0 205 0 260-52 312s-107 52-312 52-260 0-312-52-52-107-52-312v-96c0-205 0-260 52-312s107-52 312-52z" fill="#fff" stroke="#000" stroke-opacity=".12" stroke-width="12"/><g style="isolation:isolate"><circle cx="420" cy="420" r="165" fill="#00aeef" style="mix-blend-mode:multiply"/><circle cx="604" cy="420" r="165" fill="#ec008c" style="mix-blend-mode:multiply"/><circle cx="420" cy="604" r="165" fill="#ffe600" style="mix-blend-mode:multiply"/><circle cx="604" cy="604" r="165" fill="#231f20" fill-opacity=".85" style="mix-blend-mode:multiply"/></g></svg>"##;

const ICONE_SITES: &str = r#"<svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true"><rect x="3" y="3" width="14" height="6" rx="1.6"/><rect x="3" y="11" width="14" height="6" rx="1.6"/><circle cx="6.5" cy="6" r=".6" fill="currentColor"/><circle cx="6.5" cy="14" r=".6" fill="currentColor"/></svg>"#;
const ICONE_COMPTE: &str = r#"<svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true"><circle cx="10" cy="10" r="7.5"/><circle cx="10" cy="8" r="2.6"/><path d="M5.2 15.3c1.1-1.8 2.8-2.7 4.8-2.7s3.7.9 4.8 2.7"/></svg>"#;
const ICONE_ADMIN: &str = r#"<svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true"><path d="M10 2.5 16 5v4.6c0 3.6-2.5 6.6-6 7.9-3.5-1.3-6-4.3-6-7.9V5z"/><path d="m7.3 10 1.9 1.9 3.6-3.8"/></svg>"#;

const LIENS_LEGAUX: &str = r#"<span>SAS GOLDEN PRINT</span><a href="/mentions-legales">Mentions légales</a><a href="/cgu">CGU</a><a href="/confidentialite">Confidentialité</a>"#;

fn page(titre: &str, s: Option<&Session>, corps: &str) -> Html<String> {
    let tete = format!(r##"<!doctype html><html lang="fr"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><meta name="theme-color" content="#f5f5f7" media="(prefers-color-scheme: light)"><meta name="theme-color" content="#1e1e1e" media="(prefers-color-scheme: dark)"><title>{} · E-KO RIP Cloud</title><style>{STYLE}</style></head>"##, echapper(titre));
    let legal = corps.contains(r#"class="carte texte""#);
    let html = match s {
        Some(s) => {
            let entree = |lien: &str, nom: &str, icone: &str| format!(r#"<a class="entree{}" href="{lien}"{}>{icone}<span>{nom}</span></a>"#, if titre == nom { " active" } else { "" }, if titre == nom { r#" aria-current="page""# } else { "" });
            let admin = if s.compte.super_admin { format!(r#"<div class="rubrique">Plate-forme</div>{}"#, entree("/admin", "Administration", ICONE_ADMIN)) } else { String::new() };
            format!(
                r#"{tete}<body><div class="app"><nav class="laterale" aria-label="Navigation"><a class="marque" href="/sites">{ROSACE}<span>E-KO RIP Cloud</span></a><div class="rubrique">Atelier</div>{}{}{admin}<div class="bas"><form method="post" action="/deconnexion"><input type="hidden" name="csrf" value="{}"><button class="discret" type="submit">Se déconnecter</button></form><div class="liens">{LIENS_LEGAUX}</div></div></nav><div class="contenu"><div class="outils">{}</div><main>{corps}</main><div class="pied-mobile liens">{LIENS_LEGAUX}</div></div></div></body></html>"#,
                entree("/sites", "Mes sites", ICONE_SITES),
                entree("/compte", "Mon compte", ICONE_COMPTE),
                echapper(&s.csrf),
                echapper(titre)
            )
        }
        None => format!(
            r#"{tete}<body><div class="feuille"><a class="tete" href="/">{ROSACE}<span>E-KO RIP Cloud</span></a><main{}>{corps}</main><div class="liens">{LIENS_LEGAUX}</div></div></body></html>"#,
            if legal { r#" class="large""# } else { "" }
        ),
    };
    Html(html)
}

fn erreur_page(r: &Partage, s: Option<&Session>, texte: &str) -> Response {
    let _ = r;
    (StatusCode::BAD_REQUEST, page("Erreur", s, &format!(r#"<div class="carte"><p class="erreur">{}</p><p><a href="/">Retour</a></p></div>"#, echapper(texte)))).into_response()
}

// Pour l'autorisation des apps (`applis`).
pub(crate) fn session_ouverte(r: &Partage, h: &HeaderMap) -> Option<(Compte, String)> {
    session(r, h).map(|s| (s.compte, s.csrf))
}

pub(crate) fn page_erreur(r: &Partage, h: &HeaderMap, texte: &str) -> Response {
    let s = session(r, h);
    erreur_page(r, s.as_ref(), texte)
}

pub(crate) fn page_publique(titre: &str, r: &Partage, h: &HeaderMap, corps: &str) -> Response {
    let s = session(r, h);
    page(titre, s.as_ref(), corps).into_response()
}

pub(crate) fn origine_acceptee(r: &Partage, h: &HeaderMap) -> bool {
    origine_ok(r, h)
}

pub(crate) fn adresse_visiteur(h: &HeaderMap) -> String {
    ip(h)
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

fn boutons_fournisseurs(r: &Partage, verbe: &str, suite: &str) -> String {
    let b = r.base();
    let suite = if suite.is_empty() || suite == "/sites" { String::new() } else { format!("?suite={}", encoder(suite)) };
    let mut html = String::new();
    if base::reglage(&b, "google_client_id").ok().flatten().is_some() && base::reglage(&b, "google_secret").ok().flatten().is_some() {
        html.push_str(&format!(r#"<a class="bouton discret" href="/auth/google{suite}">{verbe} avec Google</a>"#));
    }
    if base::reglage(&b, "facebook_app_id").ok().flatten().is_some() && base::reglage(&b, "facebook_secret").ok().flatten().is_some() {
        html.push_str(&format!(r#"<a class="bouton discret" href="/auth/facebook{suite}">{verbe} avec Facebook</a>"#));
    }
    if html.is_empty() { String::new() } else { format!(r#"<div class="fournisseurs">{html}</div><p class="meta">En continuant avec Google ou Facebook, vous acceptez les <a href="/cgu">CGU</a> et la <a href="/confidentialite">politique de confidentialité</a>.</p>"#) }
}

fn formulaire_inscription(r: &Partage, erreur: &str, email: &str, nom: &str) -> String {
    format!(
        r#"<div class="carte"><h1>Créer un compte</h1>{}{}<form method="post" action="/inscription"><label>Nom ou atelier<input type="text" name="nom" required maxlength="80" value="{}"></label><label>E-mail<input type="email" name="email" required autocomplete="email" value="{}"></label><label>Mot de passe (10 caractères au moins)<input type="password" name="mot_de_passe" required minlength="10" autocomplete="new-password"></label><label class="case"><input type="checkbox" name="cgu" value="1" required> <span>J'accepte les <a href="/cgu">conditions générales d'utilisation</a> et la <a href="/confidentialite">politique de confidentialité</a>.</span></label><button type="submit">Créer mon compte</button></form><p class="meta">Déjà inscrit ? <a href="/connexion">Se connecter</a></p></div>"#,
        if erreur.is_empty() { String::new() } else { format!(r#"<p class="erreur">{}</p>"#, echapper(erreur)) },
        boutons_fournisseurs(r, "S'inscrire", ""),
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
        Ok(Some(id)) => {
            envoyer_confirmation(&r, id, &email, &nom);
            connecte(&r, id, "/sites")
        }
        Ok(None) => (StatusCode::CONFLICT, page("Créer un compte", None, &formulaire_inscription(&r, "Un compte existe déjà avec cet e-mail : se connecter.", &email, &nom))).into_response(),
        Err(_) => erreur_page(&r, None, "Service momentanément indisponible."),
    }
}

fn formulaire_connexion(r: &Partage, erreur: &str, email: &str, suite: &str) -> String {
    formulaire_connexion_info(r, erreur, "", email, suite)
}

fn formulaire_connexion_info(r: &Partage, erreur: &str, info: &str, email: &str, suite: &str) -> String {
    format!(
        r#"<div class="carte"><h1>Se connecter</h1>{}{}{}<form method="post" action="/connexion"><input type="hidden" name="suite" value="{}"><label>E-mail<input type="email" name="email" required autocomplete="email" value="{}"></label><label>Mot de passe<input type="password" name="mot_de_passe" required autocomplete="current-password"></label><button type="submit">Se connecter</button></form><p class="meta"><a href="/mot-de-passe/oublie">Mot de passe oublié ?</a></p><p class="meta">Pas encore de compte ? <a href="/inscription">Créer un compte</a></p></div>"#,
        if erreur.is_empty() { String::new() } else { format!(r#"<p class="erreur">{}</p>"#, echapper(erreur)) },
        if info.is_empty() { String::new() } else { format!(r#"<p class="ok">{}</p>"#, echapper(info)) },
        boutons_fournisseurs(r, "Continuer", suite),
        echapper(suite),
        echapper(email)
    )
}

pub async fn connexion(State(r): State<Partage>, h: HeaderMap, Query(q): Query<HashMap<String, String>>) -> Response {
    let suite = suite_sure(q.get("suite"));
    if session(&r, &h).is_some() {
        return Redirect::to(&suite).into_response();
    }
    let info = if q.get("ok").map(String::as_str) == Some("reinit") { "Mot de passe modifié : vous pouvez vous connecter." } else { "" };
    page("Connexion", None, &formulaire_connexion_info(&r, "", info, "", &suite)).into_response()
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

// ————— Mot de passe oublié, confirmation de l'adresse —————

const MESSAGE_CONFIRMATION_ENVOYEE: &str = r#"<p class="ok">E-mail envoyé : ouvrez le lien qu'il contient pour confirmer votre adresse.</p>"#;

/// Crée le lien de confirmation et envoie l'e-mail (hors de la requête).
fn envoyer_confirmation(r: &Partage, compte: i64, email: &str, nom: &str) {
    let jeton = base::creer_jeton_courriel(&r.base(), compte, base::OBJET_CONFIRMATION, base::DUREE_JETON_CONFIRMATION);
    match jeton {
        Ok(j) => r.envoyer(crate::courriels::confirmation_adresse(&r.config.url_publique, email, nom, &format!("{}/email/confirmer?jeton={j}", r.config.url_publique))),
        Err(e) => eprintln!("jeton de confirmation : {e}"),
    }
}

/// Bandeau « Confirmez votre adresse » (sur /sites et /compte), seulement si les e-mails partent.
fn bandeau_confirmation(r: &Partage, s: &Session, retour: &str) -> String {
    if s.compte.email_verifie || !r.courriels_actifs() {
        return String::new();
    }
    format!(
        r#"<div class="carte"><p><strong>Confirmez votre adresse e-mail.</strong> Un lien vous a été envoyé à {} ; tant qu'il n'est pas ouvert, vous ne pouvez pas rattacher de serveur RIP.</p><form method="post" action="/email/renvoyer">{}<input type="hidden" name="retour" value="{retour}"><button class="discret" type="submit">Renvoyer l'e-mail</button></form></div>"#,
        echapper(&s.compte.email),
        hidden_csrf(s)
    )
}

pub async fn renvoyer_confirmation(State(r): State<Partage>, h: HeaderMap, Form(f): Champs) -> Response {
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
    let retour = if f.get("retour").map(String::as_str) == Some("/compte") { "/compte" } else { "/sites" };
    if s.compte.email_verifie {
        return Redirect::to(retour).into_response();
    }
    if !r.limiter(&format!("renvoi:{}", s.compte.id), 3, Duration::from_secs(3600)) {
        return erreur_page(&r, Some(&s), "Trop de demandes : réessayer dans une heure.");
    }
    envoyer_confirmation(&r, s.compte.id, &s.compte.email, &s.compte.nom);
    Redirect::to(&format!("{retour}?ok=confirmation")).into_response()
}

fn page_sans_cache(titre: &str, s: Option<&Session>, corps: &str, statut: StatusCode) -> Response {
    let mut rep = (statut, page(titre, s, corps)).into_response();
    rep.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    rep
}

const LIEN_INVALIDE: &str = r#"<div class="carte"><h1>Lien invalide ou expiré</h1><p>Ce lien a déjà servi ou n'est plus valable.</p><p class="ligne"><a class="bouton" href="/mot-de-passe/oublie">Recevoir un nouveau lien</a><a class="bouton discret" href="/connexion">Se connecter</a></p></div>"#;

fn formulaire_oubli(erreur: &str, email: &str) -> String {
    format!(
        r#"<div class="carte"><h1>Mot de passe oublié</h1>{}<p class="meta">Indiquez l'adresse de votre compte : si elle existe, vous recevez un lien (valable 1 heure) pour choisir un nouveau mot de passe.</p><form method="post" action="/mot-de-passe/oublie"><label>E-mail<input type="email" name="email" required autocomplete="email" value="{}"></label><button type="submit">Envoyer le lien</button></form><p class="meta"><a href="/connexion">Retour à la connexion</a></p></div>"#,
        if erreur.is_empty() { String::new() } else { format!(r#"<p class="erreur">{}</p>"#, echapper(erreur)) },
        echapper(email)
    )
}

pub async fn oubli(State(r): State<Partage>, h: HeaderMap) -> Response {
    if session(&r, &h).is_some() {
        return Redirect::to("/compte").into_response();
    }
    page("Mot de passe oublié", None, &formulaire_oubli("", "")).into_response()
}

pub async fn oublier(State(r): State<Partage>, h: HeaderMap, Form(f): Champs) -> Response {
    if !origine_ok(&r, &h) {
        return refuse_origine();
    }
    let email = champ(&f, "email");
    if !r.limiter(&format!("oubli:{}", ip(&h)), 10, Duration::from_secs(600)) {
        return (StatusCode::TOO_MANY_REQUESTS, page("Mot de passe oublié", None, &formulaire_oubli("Trop d'essais depuis cette adresse : réessayer dans quelques minutes.", &email))).into_response();
    }
    // Même réponse que le compte existe, soit suspendu, ou n'existe pas : rien à deviner.
    // Au plus 3 e-mails par heure et par adresse (pas de harcèlement d'un titulaire).
    if base::email_valide(&email) && r.limiter(&format!("oubli-email:{}", email.to_lowercase()), 3, Duration::from_secs(3600)) {
        let envoi = {
            let b = r.base();
            base::compte_par_email(&b, &email).ok().flatten().filter(|c| !c.suspendu).and_then(|c| base::creer_jeton_courriel(&b, c.id, base::OBJET_REINIT, base::DUREE_JETON_REINIT).ok().map(|j| (c, j)))
        };
        if let Some((c, jeton)) = envoi {
            r.envoyer(crate::courriels::mot_de_passe_oublie(&r.config.url_publique, &c.email, &c.nom, &format!("{}/mot-de-passe/nouveau?jeton={jeton}", r.config.url_publique)));
        }
    }
    page("Mot de passe oublié", None, r#"<div class="carte"><h1>Vérifiez votre boîte e-mail</h1><p>Si un compte existe avec cette adresse, un e-mail contenant un lien vient de lui être envoyé. Le lien est valable 1 heure.</p><p class="meta">Rien reçu ? Regardez dans les courriers indésirables, ou <a href="/mot-de-passe/oublie">recommencez</a>.</p><p><a href="/connexion">Retour à la connexion</a></p></div>"#).into_response()
}

fn formulaire_nouveau(jeton: &str, erreur: &str) -> String {
    format!(
        r#"<div class="carte"><h1>Nouveau mot de passe</h1>{}<form method="post" action="/mot-de-passe/nouveau"><input type="hidden" name="jeton" value="{}"><label>Nouveau mot de passe (10 caractères au moins)<input type="password" name="mot_de_passe" required minlength="10" autocomplete="new-password"></label><label>Confirmation<input type="password" name="confirmation" required minlength="10" autocomplete="new-password"></label><p class="meta">Toutes vos sessions et les apps connectées seront déconnectées.</p><button type="submit">Enregistrer</button></form></div>"#,
        if erreur.is_empty() { String::new() } else { format!(r#"<p class="erreur">{}</p>"#, echapper(erreur)) },
        echapper(jeton)
    )
}

pub async fn nouveau_mot_de_passe(State(r): State<Partage>, h: HeaderMap, Query(q): Query<HashMap<String, String>>) -> Response {
    let jeton = q.get("jeton").cloned().unwrap_or_default();
    if !r.limiter(&format!("reinit:{}", ip(&h)), 30, Duration::from_secs(600)) {
        return erreur_page(&r, None, "Trop d'essais : réessayer dans quelques minutes.");
    }
    let valide = base::jeton_courriel_valide(&r.base(), &jeton, base::OBJET_REINIT).ok().flatten().is_some();
    if valide { page_sans_cache("Nouveau mot de passe", None, &formulaire_nouveau(&jeton, ""), StatusCode::OK) } else { page_sans_cache("Lien invalide", None, LIEN_INVALIDE, StatusCode::BAD_REQUEST) }
}

pub async fn reinitialiser(State(r): State<Partage>, h: HeaderMap, Form(f): Champs) -> Response {
    if !origine_ok(&r, &h) {
        return refuse_origine();
    }
    let jeton = champ(&f, "jeton");
    if !r.limiter(&format!("reinit:{}", ip(&h)), 30, Duration::from_secs(600)) {
        return erreur_page(&r, None, "Trop d'essais : réessayer dans quelques minutes.");
    }
    if base::jeton_courriel_valide(&r.base(), &jeton, base::OBJET_REINIT).ok().flatten().is_none() {
        return page_sans_cache("Lien invalide", None, LIEN_INVALIDE, StatusCode::BAD_REQUEST);
    }
    let mdp = f.get("mot_de_passe").cloned().unwrap_or_default();
    let erreur = if !base::mot_de_passe_valide(&mdp) {
        "Mot de passe trop court : 10 caractères au moins."
    } else if f.get("confirmation") != Some(&mdp) {
        "Les deux mots de passe ne sont pas identiques."
    } else {
        ""
    };
    if !erreur.is_empty() {
        return page_sans_cache("Nouveau mot de passe", None, &formulaire_nouveau(&jeton, erreur), StatusCode::BAD_REQUEST);
    }
    let hachage = tokio::task::spawn_blocking(move || base::hacher(&mdp)).await.expect("hachage");
    match base::reinitialiser_mot_de_passe(&r.base(), &jeton, &hachage) {
        Ok(Some(_)) => Redirect::to("/connexion?ok=reinit").into_response(),
        Ok(None) => page_sans_cache("Lien invalide", None, LIEN_INVALIDE, StatusCode::BAD_REQUEST),
        Err(_) => erreur_page(&r, None, "Service momentanément indisponible."),
    }
}

/// Le lien de l'e-mail ouvre une page avec un bouton : c'est le clic (POST) qui consomme le
/// jeton, pas la simple ouverture (les antivirus et aperçus de messagerie suivent les liens).
pub async fn confirmation(State(r): State<Partage>, h: HeaderMap, Query(q): Query<HashMap<String, String>>) -> Response {
    let jeton = q.get("jeton").cloned().unwrap_or_default();
    if !r.limiter(&format!("confirmation:{}", ip(&h)), 30, Duration::from_secs(600)) {
        return erreur_page(&r, None, "Trop d'essais : réessayer dans quelques minutes.");
    }
    let s = session(&r, &h);
    if base::jeton_courriel_valide(&r.base(), &jeton, base::OBJET_CONFIRMATION).ok().flatten().is_none() {
        return page_sans_cache("Lien invalide", s.as_ref(), r#"<div class="carte"><h1>Lien invalide ou expiré</h1><p>Ce lien a déjà servi ou n'est plus valable. Connectez-vous : un bandeau vous permet de recevoir un nouveau lien.</p><p><a class="bouton" href="/sites">Mes sites</a></p></div>"#, StatusCode::BAD_REQUEST);
    }
    page_sans_cache("Confirmer mon adresse", s.as_ref(), &format!(r#"<div class="carte"><h1>Confirmer mon adresse</h1><p>Un clic pour confirmer que cette adresse e-mail est bien la vôtre.</p><form method="post" action="/email/confirmer"><input type="hidden" name="jeton" value="{}"><button type="submit">Confirmer mon adresse</button></form></div>"#, echapper(&jeton)), StatusCode::OK)
}

pub async fn confirmer(State(r): State<Partage>, h: HeaderMap, Form(f): Champs) -> Response {
    if !origine_ok(&r, &h) {
        return refuse_origine();
    }
    if !r.limiter(&format!("confirmation:{}", ip(&h)), 30, Duration::from_secs(600)) {
        return erreur_page(&r, None, "Trop d'essais : réessayer dans quelques minutes.");
    }
    let s = session(&r, &h);
    match base::confirmer_email(&r.base(), &champ(&f, "jeton")) {
        Ok(Some(_)) => page_sans_cache("Adresse confirmée", s.as_ref(), r#"<div class="carte"><h1>Adresse confirmée</h1><p class="ok">Merci : votre adresse e-mail est confirmée. Vous pouvez rattacher vos serveurs RIP.</p><p><a class="bouton" href="/sites">Mes sites</a></p></div>"#, StatusCode::OK),
        Ok(None) => page_sans_cache("Lien invalide", s.as_ref(), r#"<div class="carte"><h1>Lien invalide ou expiré</h1><p>Ce lien a déjà servi ou n'est plus valable.</p><p><a class="bouton" href="/sites">Mes sites</a></p></div>"#, StatusCode::BAD_REQUEST),
        Err(_) => erreur_page(&r, s.as_ref(), "Service momentanément indisponible."),
    }
}

fn carte_emails(s: &Session) -> String {
    let c = &s.compte;
    format!(
        r#"<div class="carte"><h2>E-mails</h2><form method="post" action="/compte/preferences">{}<label class="case"><input type="checkbox" name="alertes" value="1"{}> <span>Recevoir les alertes (site hors ligne depuis 15 minutes, puis de retour en ligne)</span></label><label class="case"><input type="checkbox" name="resume" value="1"{}> <span>Recevoir le résumé hebdomadaire (le lundi vers 8 h)</span></label><button type="submit">Enregistrer</button></form><p class="meta">Les e-mails de sécurité (mot de passe oublié, confirmation d'adresse, nouvelle app connectée) sont toujours envoyés.</p></div>"#,
        hidden_csrf(s),
        if c.alertes { " checked" } else { "" },
        if c.resume { " checked" } else { "" }
    )
}

pub async fn regler_preferences(State(r): State<Partage>, h: HeaderMap, Form(f): Champs) -> Response {
    if !origine_ok(&r, &h) {
        return refuse_origine();
    }
    let s = match exiger(&r, &h, "/compte") {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    if !csrf_ok(&s, &f) {
        return refuse_origine();
    }
    if base::regler_preferences(&r.base(), s.compte.id, champ(&f, "alertes") == "1", champ(&f, "resume") == "1").is_err() {
        return erreur_page(&r, Some(&s), "Enregistrement impossible.");
    }
    Redirect::to("/compte?ok=preferences").into_response()
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
pub(crate) fn civil(z: i64) -> (i64, u32, u32) {
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
        let adresse = echapper(&crate::relais::adresse_du_site(&r.config.url_publique, id));
        lignes.push_str(&format!(
            r#"<tr><td><strong>{}</strong><br><a href="{adresse}" rel="noopener"><code>{adresse}</code></a></td><td class="{}">{}</td><td>{}</td><td>{}</td><td>{}</td></tr>"#,
            echapper(&site.nom),
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
        Some("confirmation") => MESSAGE_CONFIRMATION_ENVOYEE,
        _ => "",
    };
    let bandeau = bandeau_confirmation(&r, &s, "/sites");
    let code = q.get("code").map(|c| echapper(c)).unwrap_or_default();
    let corps = format!(
        r#"<h1>Mes sites</h1>{message}{bandeau}<div class="carte">{liste}</div><div class="carte"><h2>Rattacher un serveur RIP</h2><p class="meta">Sur le serveur : Système › Cloud E-KO RIP › « Connecter à la plate-forme ». Saisissez ici le code affiché (valable 10 minutes).</p><form method="post" action="/sites/rattacher" class="ligne">{}<input type="text" name="code" placeholder="ABCD-1234" required maxlength="12" value="{code}" style="max-width: 12rem; text-transform: uppercase" autocomplete="off"><button type="submit">Rattacher</button></form></div>"#,
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
    if !s.compte.email_verifie && r.courriels_actifs() {
        return erreur_page(&r, Some(&s), "Confirmez d'abord votre adresse e-mail (lien reçu à l'inscription, à renvoyer depuis « Mes sites » ou « Mon compte ») avant de rattacher un serveur.");
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
    if let Err(e) = r.retirer_site(&site) {
        eprintln!("retrait du site {site} : {e}");
        return erreur_page(&r, Some(&s), "Service momentanément indisponible.");
    }
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
        Some("preferences") => r#"<p class="ok">Préférences d'e-mail enregistrées.</p>"#,
        Some("confirmation") => MESSAGE_CONFIRMATION_ENVOYEE,
        _ => "",
    };
    let applis = base::jetons_du_compte(&r.base(), c.id).unwrap_or_default();
    let carte_applis = if applis.is_empty() {
        r#"<div class="carte"><h2>Apps connectées</h2><p class="meta">Aucune. Dans une app E-KO RIP Center, « Se connecter au cloud » vous connecte avec ce compte.</p></div>"#.to_string()
    } else {
        let lignes: String = applis.iter().map(|(appareil, cree, vu)| format!("<li>{} <span class=\"meta\">— connectée le {}, vue le {}</span></li>", echapper(appareil), date(Some(*cree)), date(Some(*vu)))).collect();
        format!(r#"<div class="carte"><h2>Apps connectées</h2><ul>{lignes}</ul><form method="post" action="/compte/applis/deconnecter">{}<button class="discret" type="submit">Déconnecter toutes les apps</button></form></div>"#, hidden_csrf(&s))
    };
    let message = if q.get("ok").map(String::as_str) == Some("applis") { r#"<p class="ok">Toutes les apps ont été déconnectées.</p>"# } else { message };
    let corps = format!(
        r#"<h1>Mon compte</h1>{message}{bandeau}<div class="carte"><p><strong>{}</strong><br>{}</p><p class="meta">Inscrit le {}</p><h2>Connexions</h2><ul><li>Mot de passe : {}</li>{}{}</ul></div>{carte_emails}{carte_applis}<div class="carte"><h2>Supprimer mon compte</h2><p>Le compte, ses droits et <strong>les sites dont vous êtes propriétaire</strong> sont effacés ; leur accès à distance s'arrête aussitôt. Irréversible.</p><form method="post" action="/compte/supprimer">{}<label class="case"><input type="checkbox" name="confirmer" value="1" required> <span>Je confirme la suppression définitive de mon compte.</span></label><button class="danger" type="submit">Supprimer mon compte</button></form></div>"#,
        echapper(&c.nom),
        echapper(&c.email),
        date(Some(c.cree_le)),
        if c.a_mot_de_passe { "oui" } else { "non (connexion par Google ou Facebook)" },
        lien("google", "Google", c.google),
        lien("facebook", "Facebook", c.facebook),
        hidden_csrf(&s),
        bandeau = bandeau_confirmation(&r, &s, "/compte"),
        carte_emails = carte_emails(&s),
    );
    page("Mon compte", Some(&s), &corps).into_response()
}

pub async fn deconnecter_applis(State(r): State<Partage>, h: HeaderMap, Form(f): Champs) -> Response {
    if !origine_ok(&r, &h) {
        return refuse_origine();
    }
    let s = match exiger(&r, &h, "/compte") {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    if !csrf_ok(&s, &f) {
        return refuse_origine();
    }
    let _ = base::revoquer_jetons_du_compte(&r.base(), s.compte.id);
    Redirect::to("/compte?ok=applis").into_response()
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
        if let Err(e) = r.retirer_site(&site) {
            eprintln!("retrait du site {site} : {e}");
            return erreur_page(&r, Some(&s), "Service momentanément indisponible.");
        }
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
    // Avant de prendre la base (verrou non réentrant, `courriels_actifs` la prend aussi).
    let actif = r.courriels_actifs();
    let b = r.base();
    let valeur = |cle: &str| base::reglage(&b, cle).ok().flatten().unwrap_or_default();
    let secret = |cle: &str| if valeur(cle).is_empty() { "non renseigné" } else { "enregistré (laisser vide pour le garder)" };
    let message = match q.get("ok").map(String::as_str) {
        Some("oauth") => r#"<p class="ok">Clés enregistrées.</p>"#,
        Some("compte") => r#"<p class="ok">Compte mis à jour.</p>"#,
        Some("github") => r#"<p class="ok">Accès GitHub enregistré.</p>"#,
        Some("courriels") => r#"<p class="ok">Réglages d'e-mail enregistrés.</p>"#,
        Some("essai") => r#"<p class="ok">E-mail d'essai envoyé à votre adresse.</p>"#,
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
{installations}
{rapports}
{courriels}
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
        installations = carte_installations(&r.statistiques_installations()),
        rapports = carte_rapports(&r, &s, &valeur("github_depot"), !valeur("github_jeton").is_empty()),
        courriels = carte_courriels(&b, &s, actif),
    );
    page("Administration", Some(&s), &corps).into_response()
}

/// Statistiques anonymes des installations E-KO RIP (ping des serveurs et des clients).
fn carte_installations(st: &crate::installations::Statistiques) -> String {
    let tableau = |titre: &str, m: &std::collections::BTreeMap<String, usize>| {
        let mut v: Vec<_> = m.iter().collect();
        v.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        let lignes: String = v.iter().map(|(k, n)| format!("<tr><td>{}</td><td>{n}</td></tr>", echapper(k))).collect();
        format!(r#"<div class="tableau"><table><thead><tr><th>{titre}</th><th>Actives</th></tr></thead><tbody>{lignes}</tbody></table></div>"#)
    };
    let semaines: Vec<String> = st.nouvelles_par_semaine.iter().map(usize::to_string).collect();
    format!(
        r#"<div class="carte"><h2>Installations</h2>
<p><strong>{actives}</strong> actives (vues depuis moins de 15 jours) sur <strong>{total}</strong> installations vues · <strong>{machines}</strong> machines pilotées par les serveurs actifs</p>
<p class="meta">Nouvelles installations par semaine, de la plus récente à il y a 8 semaines : {semaines}</p>
{types}{os}{versions}</div>"#,
        actives = st.actives,
        total = st.total,
        machines = st.machines,
        semaines = semaines.join(" · "),
        types = tableau("Type de poste", &st.par_type),
        os = tableau("Système", &st.par_os),
        versions = tableau("Version", &st.par_version),
    )
}

/// Rapports de bug reçus (90 jours) et accès GitHub pour les issues.
fn carte_rapports(r: &Partage, s: &Session, depot: &str, jeton: bool) -> String {
    let tous = crate::rapports::lister(&r.config.donnees, 100);
    let lignes: String = tous
        .iter()
        .map(|x| {
            let issue = x.issue.as_deref().map_or("—".to_string(), |u| format!(r#"<a href="{}" rel="noopener" target="_blank">issue</a>"#, echapper(u)));
            format!(
                r#"<tr><td><a href="/admin/rapports/{re}"><code>{re}</code></a><br><span class="meta">{date}</span></td><td>{nature}{code}</td><td>{poste}</td><td>{titre}</td><td>{issue}</td></tr>"#,
                re = echapper(&x.reference),
                date = date(Some(x.recu_le)),
                nature = echapper(&x.nature),
                code = if x.code.is_empty() { String::new() } else { format!("<br><code>{}</code>", echapper(&x.code)) },
                poste = echapper(&format!("{} {}", x.type_poste, x.version)),
                titre = echapper(&x.titre),
            )
        })
        .collect();
    format!(
        r#"<div class="carte"><h2>Rapports ({n}, les 100 derniers affichés)</h2>
<p class="meta">« Signaler un problème » et rapports automatiques des installations, gardés 90 jours. Le journal et l'e-mail restent ici ; l'issue GitHub n'a que le titre, la description, le poste, l'OS et la référence.</p>
<div class="tableau"><table><thead><tr><th>Référence</th><th>Nature</th><th>Poste</th><th>Titre</th><th>GitHub</th></tr></thead><tbody>{lignes}</tbody></table></div>
<h2>Issues GitHub</h2>
<p class="meta">Jeton à grain fin (Settings › Developer settings › Fine-grained tokens) limité au dépôt, droit « Issues : lecture et écriture ». Sans jeton, les rapports sont seulement gardés ici.</p>
<form method="post" action="/admin/github">{csrf}
<label>Dépôt (propriétaire/nom)<input type="text" name="github_depot" value="{depot}" placeholder="mvignaud/eko-rip-center" autocomplete="off"></label>
<label>Jeton ({etat})<input type="password" name="github_jeton" autocomplete="new-password"></label>
<label class="case"><input type="checkbox" name="effacer" value="1"> <span>Effacer le jeton enregistré</span></label>
<button type="submit">Enregistrer</button></form></div>"#,
        n = crate::rapports::nombre(&r.config.donnees),
        csrf = hidden_csrf(s),
        depot = echapper(depot),
        etat = if jeton { "enregistré (laisser vide pour le garder)" } else { "non renseigné" },
    )
}

/// Carte « E-mails » de l'administration : serveur SMTP et e-mail d'essai.
fn carte_courriels(b: &rusqlite::Connection, s: &Session, actif: bool) -> String {
    let valeur = |cle: &str| base::reglage(b, cle).ok().flatten().unwrap_or_default();
    let securite = valeur("smtp_securite");
    let impose = ["HOTE", "PORT", "SECURITE", "IDENTIFIANT", "MOT_DE_PASSE", "EXPEDITEUR"].iter().any(|v| std::env::var(format!("EKO_CLOUD_SMTP_{v}")).is_ok_and(|x| !x.trim().is_empty()));
    let etat = if actif { r#"<p class="ok">Envoi d'e-mails actif.</p>"# } else { r#"<p class="meta">Envoi d'e-mails inactif : sans hôte SMTP, les e-mails sont ignorés (mot de passe oublié compris).</p>"# };
    format!(
        r#"<div class="carte"><h2>E-mails</h2>{etat}
<p class="meta">Serveur SMTP d'envoi (celui de l'ERP). L'expéditeur est « {nom} » ; la connexion est toujours chiffrée.{impose}</p>
<form method="post" action="/admin/courriels">{csrf}
<label>Hôte<input type="text" name="smtp_hote" value="{hote}" placeholder="smtp.exemple.fr" autocomplete="off"></label>
<label>Port (587 en STARTTLS, 465 en TLS direct)<input type="text" name="smtp_port" value="{port}" inputmode="numeric" placeholder="587" autocomplete="off"></label>
<label>Sécurité<select name="smtp_securite"><option value="starttls"{o1}>STARTTLS</option><option value="tls"{o2}>TLS direct</option></select></label>
<label>Identifiant<input type="text" name="smtp_identifiant" value="{id}" autocomplete="off"></label>
<label>Mot de passe ({mdp})<input type="password" name="smtp_mot_de_passe" autocomplete="new-password"></label>
<label>Adresse de l'expéditeur<input type="email" name="smtp_expediteur" value="{exp}" placeholder="{defaut}" autocomplete="off"></label>
<label class="case"><input type="checkbox" name="effacer" value="1"> <span>Effacer le mot de passe enregistré</span></label>
<button type="submit">Enregistrer</button></form>
<form method="post" action="/admin/courriels/essai">{csrf}<button class="discret" type="submit">Envoyer un e-mail d'essai</button> <span class="meta">à {moi}</span></form></div>"#,
        nom = crate::courriels::NOM_EXPEDITEUR,
        impose = if impose { " Des variables <code>EKO_CLOUD_SMTP_*</code> sont définies : elles priment sur ces réglages." } else { "" },
        csrf = hidden_csrf(s),
        hote = echapper(&valeur("smtp_hote")),
        port = echapper(&valeur("smtp_port")),
        o1 = if securite != "tls" { " selected" } else { "" },
        o2 = if securite == "tls" { " selected" } else { "" },
        id = echapper(&valeur("smtp_identifiant")),
        mdp = if valeur("smtp_mot_de_passe").is_empty() { "non renseigné" } else { "enregistré (laisser vide pour le garder)" },
        exp = echapper(&valeur("smtp_expediteur")),
        defaut = crate::courriels::EXPEDITEUR_DEFAUT,
        moi = echapper(&s.compte.email),
    )
}

pub async fn regler_courriels(State(r): State<Partage>, h: HeaderMap, Form(f): Champs) -> Response {
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
    let (hote, port, securite, expediteur) = (champ(&f, "smtp_hote"), champ(&f, "smtp_port"), champ(&f, "smtp_securite"), champ(&f, "smtp_expediteur"));
    if !hote.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ':' | '[' | ']')) || hote.len() > 253 {
        return erreur_page(&r, Some(&s), "Hôte SMTP invalide.");
    }
    if !port.is_empty() && port.parse::<u16>().ok().filter(|p| *p > 0).is_none() {
        return erreur_page(&r, Some(&s), "Port invalide.");
    }
    if !matches!(securite.as_str(), "starttls" | "tls" | "") {
        return erreur_page(&r, Some(&s), "Sécurité invalide.");
    }
    if !expediteur.is_empty() && !base::email_valide(&expediteur) {
        return erreur_page(&r, Some(&s), "Adresse d'expéditeur invalide.");
    }
    let b = r.base();
    let mut ok = true;
    for (cle, v) in [("smtp_hote", hote), ("smtp_port", port), ("smtp_securite", securite), ("smtp_identifiant", champ(&f, "smtp_identifiant")), ("smtp_expediteur", expediteur)] {
        ok &= base::regler(&b, cle, &v).is_ok();
    }
    let mdp = champ(&f, "smtp_mot_de_passe");
    if champ(&f, "effacer") == "1" {
        ok &= base::regler(&b, "smtp_mot_de_passe", "").is_ok();
    } else if !mdp.is_empty() {
        ok &= base::regler(&b, "smtp_mot_de_passe", &mdp).is_ok();
    }
    drop(b);
    if !ok {
        return erreur_page(&r, Some(&s), "Enregistrement impossible.");
    }
    Redirect::to("/admin?ok=courriels").into_response()
}

/// E-mail d'essai à l'administrateur connecté ; le résultat de l'envoi est affiché.
pub async fn essai_courriel(State(r): State<Partage>, h: HeaderMap, Form(f): Champs) -> Response {
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
    if !r.limiter(&format!("essai-courriel:{}", s.compte.id), 10, Duration::from_secs(3600)) {
        return erreur_page(&r, Some(&s), "Trop d'essais : réessayer dans une heure.");
    }
    match r.envoyer_et_attendre(crate::courriels::essai(&r.config.url_publique, &s.compte.email, &s.compte.nom, crate::maintenant())).await {
        Ok(()) => Redirect::to("/admin?ok=essai").into_response(),
        Err(e) => {
            eprintln!("e-mail d'essai non envoyé : {e}");
            erreur_page(&r, Some(&s), &format!("E-mail d'essai non envoyé : {e}"))
        }
    }
}

pub async fn regler_github(State(r): State<Partage>, h: HeaderMap, Form(f): Champs) -> Response {
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
    let depot = champ(&f, "github_depot");
    let parties: Vec<&str> = depot.split('/').collect();
    let nom_ok = |p: &str| !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if !depot.is_empty() && !(parties.len() == 2 && parties.iter().all(|p| nom_ok(p))) {
        return erreur_page(&r, Some(&s), "Dépôt invalide (forme attendue : propriétaire/nom).");
    }
    let b = r.base();
    let mut ok = base::regler(&b, "github_depot", &depot).is_ok();
    let jeton = champ(&f, "github_jeton");
    if champ(&f, "effacer") == "1" {
        ok &= base::regler(&b, "github_jeton", "").is_ok();
    } else if !jeton.is_empty() {
        ok &= base::regler(&b, "github_jeton", &jeton).is_ok();
    }
    drop(b);
    if !ok {
        return erreur_page(&r, Some(&s), "Enregistrement impossible.");
    }
    Redirect::to("/admin?ok=github").into_response()
}

/// Un rapport en entier, journal compris (super-admin).
pub async fn voir_rapport(State(r): State<Partage>, h: HeaderMap, Chemin(reference): Chemin<String>) -> Response {
    let s = match exiger_admin(&r, &h) {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    let Some(x) = crate::rapports::lire(&r.config.donnees, &reference) else { return erreur_page(&r, Some(&s), "Rapport introuvable (effacé après 90 jours ?).") };
    let ligne = |k: &str, v: &str| if v.is_empty() { String::new() } else { format!("<dt>{k}</dt><dd>{}</dd>", echapper(v)) };
    let corps = format!(
        r#"<h1>Rapport {re}</h1><div class="carte"><dl>{a}{b}{c}{d}{e}{f}{g}{h}</dl>{issue}</div>
<div class="carte"><h2>Description</h2><pre>{desc}</pre></div>
<div class="carte"><h2>Journal</h2><pre>{journal}</pre></div><p><a href="/admin">Retour</a></p>"#,
        re = echapper(&x.reference),
        a = ligne("Reçu le", &date(Some(x.recu_le))),
        b = ligne("Nature", &x.nature),
        c = ligne("Code", &x.code),
        d = ligne("Poste", &format!("{} {}", x.type_poste, x.version)),
        e = ligne("Serveur", &x.serveur),
        f = ligne("Système", &x.os),
        g = ligne("Titre", &x.titre),
        h = ligne("E-mail", &x.email),
        issue = x.issue.as_deref().map_or(String::new(), |u| format!(r#"<p><a href="{}" rel="noopener" target="_blank">Issue GitHub</a></p>"#, echapper(u))),
        desc = echapper(&x.description),
        journal = if x.journal.is_empty() { "(aucun journal joint)".into() } else { echapper(&x.journal) },
    );
    page("Rapport", Some(&s), &corps).into_response()
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
    page(titre, s.as_ref(), &format!(r#"<div class="carte texte">{contenu}</div>"#)).into_response()
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

pub async fn partir_oauth(State(r): State<Partage>, Chemin(nom): Chemin<String>, Query(q): Query<HashMap<String, String>>) -> Response {
    // Destination après connexion (autorisation d'une app…), gardée dans le cookie d'état.
    let suite = suite_sure(q.get("suite"));
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
    rep.headers_mut().append(header::SET_COOKIE, HeaderValue::from_str(&format!("{}={}:{etat}:{}; HttpOnly; SameSite=Lax; Path=/; Max-Age=600{secure}", nom_cookie(&r, COOKIE_OAUTH), f.nom, encoder(&suite))).expect("cookie"));
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
/// - e-mail d'un compte existant : si le fournisseur l'a vérifié (Google), son titulaire
///   reprend le compte et tout autre accès est retiré (mot de passe, sessions) ; sinon refus.
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
            // E-mail prouvé par le fournisseur : son titulaire reprend le compte (les autres accès,
            // dont un mot de passe posé par quelqu'un d'autre, sont retirés).
            if base::reprendre_par_fournisseur(b, c.id, fournisseur, &id.ident).unwrap_or(false) { Issue::Connecter(c.id) } else { Issue::Refus("Liaison impossible.") }
        }
        Ok(Some(_)) => Issue::Refus("Un compte existe déjà avec cet e-mail : se connecter avec le mot de passe, puis lier ce fournisseur depuis « Mon compte »."),
        Ok(None) => {
            let nom = if base::nom_valide(&id.nom) { id.nom.clone() } else { email.split('@').next().unwrap_or("Compte").to_string() };
            match base::creer_compte(b, &email, &nom, None) {
                Ok(Some(cid)) if base::lier_fournisseur(b, cid, fournisseur, &id.ident).unwrap_or(false) => {
                    if id.email_verifie {
                        let _ = base::marquer_email_verifie(b, cid);
                    }
                    Issue::Connecter(cid)
                }
                _ => Issue::Refus("Création du compte impossible."),
            }
        }
        Err(_) => Issue::Refus("Service momentanément indisponible."),
    }
}

pub async fn retour_oauth(State(r): State<Partage>, h: HeaderMap, Chemin(nom): Chemin<String>, Query(q): Query<HashMap<String, String>>) -> Response {
    let Some(f) = fournisseur(&nom) else { return StatusCode::NOT_FOUND.into_response() };
    let valeur = cookie(&h, &nom_cookie(&r, COOKIE_OAUTH));
    // Cookie : `<fournisseur>:<état>:<suite encodée>`.
    let (attendu, suite) = match valeur.as_deref().map(|v| v.splitn(3, ':').collect::<Vec<_>>()) {
        Some(p) if p.len() >= 2 => (Some(format!("{}:{}", p[0], p[1])), suite_sure(p.get(2).map(|s| decoder(s)).as_ref())),
        _ => (None, "/sites".to_string()),
    };
    let recu = q.get("state").map(|s| format!("{}:{s}", f.nom));
    let etat_ok = matches!((&attendu, &recu), (Some(a), Some(b)) if crate::egal_constant(a, b));
    let effacer = HeaderValue::from_str(&format!("{}=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0{}", nom_cookie(&r, COOKIE_OAUTH), if https(&r) { "; Secure" } else { "" })).expect("cookie");
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
        Issue::Connecter(id) => connecte(&r, id, &suite),
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
