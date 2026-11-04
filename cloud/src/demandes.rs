//! « Mes demandes » : le formulaire de demande et l'historique des demandes du compte, tenus
//! par le support de la Suite E-KO (`https://help.suite-eko.com`, boîte « Demandes E-KO RIP
//! Cloud »). La plate-forme n'a aucune base de tickets : elle affiche et envoie par l'API client
//! du support, toujours depuis le serveur (la clé et l'identifiant de contact ne vont jamais au
//! navigateur).
//!
//! Identité : la même que la bulle, `cloud-<id du compte>`, signée par
//! `identifier_hash = hex(HMAC-SHA256(CHATWOOT_DEMANDES_HMAC, identifiant))`. Le contact créé une
//! fois par compte est gardé dans `comptes.support_source_id`.
//!
//! Configuration, par l'environnement (`deploy-cloud.yml` les pose) :
//! - `CHATWOOT_DEMANDES_INBOX` : identifiant public de la boîte ;
//! - `CHATWOOT_DEMANDES_HMAC` : clé HMAC de la boîte ;
//! - `CHATWOOT_DEMANDES_WEBHOOK_SECRET` : secret de signature du webhook `/webhooks/support`.
//!
//! Si l'une manque, « Mes demandes » est masqué : le menu Support ouvre la bulle seule.

use crate::courriels;
use crate::portail::{Session, csrf_ok, echapper, exiger, hidden_csrf, origine_ok, page, refuse_origine};
use crate::relais::Partage;
use crate::support::{ORIGINE_SUPPORT, reference};
use axum::body::Bytes;
use axum::extract::{Multipart, Path as Chemin, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Duration;

/// Pièces jointes : 5 au plus, 10 Mo chacune.
pub const MAX_PIECES: usize = 5;
pub const MAX_OCTETS_PIECE: usize = 10 * 1024 * 1024;
/// Corps d'un formulaire avec ses pièces jointes (5 × 10 Mo, plus le texte).
pub const MAX_CORPS: usize = MAX_PIECES * MAX_OCTETS_PIECE + 1024 * 1024;
/// Tolérance sur l'horodatage du webhook.
pub const TOLERANCE_WEBHOOK: u64 = 300;

pub const TYPES: [(&str, &str); 4] = [("technique", "Question technique"), ("facturation", "Facturation"), ("commercial", "Commercial"), ("autre", "Autre")];
pub const URGENCES: [(&str, &str); 3] = [("normale", "Normale"), ("haute", "Haute"), ("bloquante", "Bloquante (plus rien ne fonctionne)")];

// ————— Configuration —————

#[derive(Clone)]
pub struct Demandes {
    /// Identifiant public de la boîte (`inbox_identifier`).
    pub boite: String,
    cle_hmac: String,
    secret_webhook: String,
    /// Adresse du support (remplacée dans les tests).
    pub base: String,
}

impl std::fmt::Debug for Demandes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Demandes").field("boite", &self.boite).field("cle_hmac", &"…").field("secret_webhook", &"…").field("base", &self.base).finish()
    }
}

impl Demandes {
    /// Les trois valeurs, ou rien si l'une manque.
    pub fn depuis(boite: Option<&str>, cle_hmac: Option<&str>, secret_webhook: Option<&str>, base: Option<&str>) -> Option<Demandes> {
        let v = |x: Option<&str>| x.map(str::trim).filter(|v| !v.is_empty()).map(str::to_string);
        Some(Demandes {
            boite: v(boite)?,
            cle_hmac: v(cle_hmac)?,
            secret_webhook: v(secret_webhook)?,
            base: v(base).unwrap_or_else(|| ORIGINE_SUPPORT.to_string()).trim_end_matches('/').to_string(),
        })
    }

    pub fn depuis_env() -> Option<Demandes> {
        let v = |n: &str| std::env::var(n).ok();
        Demandes::depuis(v("CHATWOOT_DEMANDES_INBOX").as_deref(), v("CHATWOOT_DEMANDES_HMAC").as_deref(), v("CHATWOOT_DEMANDES_WEBHOOK_SECRET").as_deref(), v("CHATWOOT_DEMANDES_URL").as_deref())
    }

    /// `identifier_hash` du contact.
    pub fn empreinte(&self, identifiant: &str) -> String {
        crate::hex(&crate::hmac_sha256(self.cle_hmac.as_bytes(), identifiant.as_bytes()))
    }

    /// Signature attendue d'un webhook : `sha256=hex(HMAC-SHA256(secret, "{horodatage}.{corps}"))`.
    pub fn signature(&self, horodatage: &str, corps: &[u8]) -> String {
        let mut m = format!("{horodatage}.").into_bytes();
        m.extend_from_slice(corps);
        format!("sha256={}", crate::hex(&crate::hmac_sha256(self.secret_webhook.as_bytes(), &m)))
    }

    /// Vérifie signature et fraîcheur d'un webhook.
    pub fn verifier(&self, h: &HeaderMap, corps: &[u8], maintenant: u64) -> Result<(), &'static str> {
        let lire = |n: &str| h.get(n).and_then(|v| v.to_str().ok()).map(str::trim).unwrap_or("");
        let horodatage = lire("x-chatwoot-timestamp");
        let signature = lire("x-chatwoot-signature");
        let t: u64 = horodatage.parse().map_err(|_| "horodatage_absent")?;
        if t.abs_diff(maintenant) > TOLERANCE_WEBHOOK {
            return Err("horodatage_trop_vieux");
        }
        if signature.is_empty() || !crate::egal_constant(&signature.to_ascii_lowercase(), &self.signature(horodatage, corps)) {
            return Err("signature_fausse");
        }
        Ok(())
    }

    fn api(&self, chemin: &str) -> String {
        format!("{}/public/api/v1/inboxes/{}{chemin}", self.base, crate::portail::encoder(&self.boite))
    }
}

static CONFIGURATION: RwLock<Option<Demandes>> = RwLock::new(None);

pub fn configurer(c: Option<Demandes>) {
    *CONFIGURATION.write().unwrap_or_else(|e| e.into_inner()) = c;
}

pub fn configurer_depuis_env() -> bool {
    let c = Demandes::depuis_env();
    let actif = c.is_some();
    configurer(c);
    actif
}

pub fn configuration() -> Option<Demandes> {
    CONFIGURATION.read().unwrap_or_else(|e| e.into_inner()).clone()
}

/// « Mes demandes » est-il en service ?
pub fn actif() -> bool {
    configuration().is_some()
}

// ————— Appels à l'API client du support —————

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder().http_status_as_error(false).max_redirects(0).timeout_global(Some(Duration::from_secs(30))).build().into()
}

/// Erreur d'appel : `Introuvable` quand le contact ou la demande n'existe pas (ou plus).
#[derive(Debug, PartialEq)]
pub enum Echec {
    Introuvable,
    Service(String),
}

const INDISPONIBLE: &str = "Le support est momentanément injoignable. Réessayez dans quelques minutes, ou écrivez-nous dans la bulle.";

fn lire(r: Result<ureq::http::Response<ureq::Body>, ureq::Error>) -> Result<Value, Echec> {
    let mut r = r.map_err(|e| Echec::Service(format!("support injoignable : {e}")))?;
    let statut = r.status().as_u16();
    let texte = r.body_mut().with_config().limit(8 * 1024 * 1024).read_to_string().unwrap_or_default();
    match statut {
        200..=299 => Ok(serde_json::from_str(&texte).unwrap_or(Value::Null)),
        401 | 404 => Err(Echec::Introuvable),
        _ => Err(Echec::Service(format!("support : HTTP {statut}"))),
    }
}

fn get(d: &Demandes, chemin: &str) -> Result<Value, Echec> {
    lire(agent().get(d.api(chemin)).header("Accept", "application/json").call())
}

fn patch(d: &Demandes, chemin: &str, corps: &Value) -> Result<Value, Echec> {
    lire(agent().patch(d.api(chemin)).header("Accept", "application/json").send_json(corps))
}

fn post(d: &Demandes, chemin: &str, corps: &Value) -> Result<Value, Echec> {
    lire(agent().post(d.api(chemin)).header("Accept", "application/json").send_json(corps))
}

/// Une pièce jointe reçue du formulaire.
pub struct Piece {
    pub nom: String,
    pub genre: String,
    pub octets: Vec<u8>,
}

fn post_message(d: &Demandes, chemin: &str, contenu: &str, pieces: &[Piece]) -> Result<Value, Echec> {
    if pieces.is_empty() {
        return post(d, chemin, &json!({ "content": contenu }));
    }
    let limite = format!("eko{}", crate::aleatoire_hex(12));
    let mut corps = Vec::new();
    corps.extend_from_slice(format!("--{limite}\r\nContent-Disposition: form-data; name=\"content\"\r\n\r\n{contenu}\r\n").as_bytes());
    for p in pieces {
        let nom = p.nom.replace(['"', '\r', '\n'], "_");
        corps.extend_from_slice(format!("--{limite}\r\nContent-Disposition: form-data; name=\"attachments[]\"; filename=\"{nom}\"\r\nContent-Type: {}\r\n\r\n", p.genre).as_bytes());
        corps.extend_from_slice(&p.octets);
        corps.extend_from_slice(b"\r\n");
    }
    corps.extend_from_slice(format!("--{limite}--\r\n").as_bytes());
    lire(agent().post(d.api(chemin)).header("Accept", "application/json").header("Content-Type", &format!("multipart/form-data; boundary={limite}")).send(&corps[..]))
}

/// Ce qu'il faut pour présenter le compte au support.
#[derive(Clone)]
pub struct Client {
    pub compte: i64,
    pub nom: String,
    pub email: String,
    pub societe: String,
}

impl Client {
    pub(crate) fn de(s: &Session) -> Client {
        Client { compte: s.compte.id, nom: s.compte.nom.trim().to_string(), email: s.compte.email.clone(), societe: s.societe.trim().to_string() }
    }
}

fn source_connue(c: &Connection, compte: i64) -> Option<String> {
    c.query_row("SELECT support_source_id FROM comptes WHERE id = ?1", [compte], |l| l.get::<_, Option<String>>(0)).optional().ok().flatten().flatten().filter(|s| !s.is_empty())
}

fn retenir_source(c: &Connection, compte: i64, source: Option<&str>) {
    let _ = c.execute("UPDATE comptes SET support_source_id = ?1 WHERE id = ?2", params![source, compte]);
}

/// Compte qui possède ce contact du support.
pub fn compte_de_source(c: &Connection, source: &str) -> Option<i64> {
    c.query_row("SELECT id FROM comptes WHERE support_source_id = ?1", [source], |l| l.get(0)).optional().ok().flatten()
}

/// Crée le contact (une fois) et rend son `source_id`.
fn creer_contact(d: &Demandes, r: &Partage, cl: &Client) -> Result<String, Echec> {
    let id = reference(cl.compte);
    let mut corps = json!({ "identifier": id, "identifier_hash": d.empreinte(&id), "name": if cl.nom.is_empty() { &cl.email } else { &cl.nom }, "email": cl.email });
    if !cl.societe.is_empty() {
        corps["custom_attributes"] = json!({ "societe": cl.societe });
    }
    let v = post(d, "/contacts", &corps)?;
    let source = v["source_id"].as_str().or_else(|| v["payload"]["source_id"].as_str()).filter(|s| !s.is_empty()).ok_or_else(|| Echec::Service("support : contact sans source_id".into()))?;
    // À la création, le support ne peut pas encore marquer l'identité vérifiée : la mise à jour
    // signée qui suit le fait, et rattache les discussions de la bulle à ce contact.
    if let Err(e) = patch(d, &format!("/contacts/{}", crate::portail::encoder(source)), &corps) {
        eprintln!("mes demandes : identité du contact non vérifiée ({e:?})");
    }
    // Base verrouillée seulement après l'appel au support : un support lent ne bloque pas le portail.
    retenir_source(&r.base(), cl.compte, Some(source));
    Ok(source.to_string())
}

/// Une seule création de contact à la fois (verrou à part, jamais celui de la base) : deux
/// onglets ouverts au premier passage partagent le même contact au lieu d'en créer deux.
static CREATIONS: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn contact_unique(d: &Demandes, r: &Partage, cl: &Client) -> Result<String, Echec> {
    let _seul = CREATIONS.lock().unwrap_or_else(|e| e.into_inner());
    let actuelle = source_connue(&r.base(), cl.compte);
    match actuelle {
        Some(s) => Ok(s),
        None => creer_contact(d, r, cl),
    }
}

/// Appelle `f` avec le contact du compte. Si `f` ne trouve rien, le contact n'est recréé (une
/// fois) que s'il a disparu lui-même : une demande introuvable (supprimée, d'un autre compte)
/// laisse le contact et ses demandes intacts.
fn avec_contact<T>(d: &Demandes, r: &Partage, cl: &Client, f: impl Fn(&str) -> Result<T, Echec>) -> Result<T, Echec> {
    let connue = source_connue(&r.base(), cl.compte);
    let source = match connue {
        Some(s) => s,
        None => contact_unique(d, r, cl)?,
    };
    match f(&source) {
        Err(Echec::Introuvable) => {
            if get(d, &format!("/contacts/{source}/conversations")).is_ok() {
                return Err(Echec::Introuvable);
            }
            let neuf = {
                let _seul = CREATIONS.lock().unwrap_or_else(|e| e.into_inner());
                // Lu à part : un verrou de base pris dans le `match` durerait jusqu'à sa fin.
                let actuelle = source_connue(&r.base(), cl.compte);
                match actuelle {
                    // Déjà remplacé par une requête concurrente.
                    Some(s) if s != source => s,
                    _ => creer_contact(d, r, cl)?,
                }
            };
            f(&neuf)
        }
        autre => autre,
    }
}

fn liste_de(v: Value) -> Vec<Value> {
    match v {
        Value::Array(a) => a,
        Value::Object(ref o) => o.get("payload").and_then(Value::as_array).cloned().unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Type, urgence, objet et « concerne » d'une demande : l'API client du support ne les rend
/// pas, ils sont gardés ici à la création.
fn retenir_attributs(c: &Connection, compte: i64, id: u64, attributs: &Value) {
    let _ = c.execute(
        "INSERT OR REPLACE INTO demandes_support (conversation, compte, attributs, cree_le) VALUES (?1, ?2, ?3, ?4)",
        params![id as i64, compte, attributs.to_string(), crate::maintenant() as i64],
    );
}

/// Complète `custom_attributes` avec ce qui a été gardé à la création.
fn completer(c: &Connection, compte: i64, conv: &mut Value) {
    let Some(id) = conv["id"].as_u64() else { return };
    let garde: Option<String> = c.query_row("SELECT attributs FROM demandes_support WHERE conversation = ?1 AND compte = ?2", params![id as i64, compte], |l| l.get(0)).optional().ok().flatten();
    let Some(Value::Object(garde)) = garde.and_then(|t| serde_json::from_str::<Value>(&t).ok()) else { return };
    if !conv["custom_attributes"].is_object() {
        conv["custom_attributes"] = json!({});
    }
    for (k, v) in garde {
        if conv["custom_attributes"].get(&k).is_none_or(Value::is_null) {
            conv["custom_attributes"][k] = v;
        }
    }
}

/// Toutes les conversations du contact (demandes et discussions de la bulle).
pub fn lister(d: &Demandes, r: &Partage, cl: &Client) -> Result<Vec<Value>, Echec> {
    let mut v = avec_contact(d, r, cl, |s| get(d, &format!("/contacts/{s}/conversations")).map(liste_de))?;
    let c = r.base();
    for conv in &mut v {
        completer(&c, cl.compte, conv);
    }
    Ok(v)
}

/// Une demande et tous ses messages (pages de 20, par `before`).
pub fn detail(d: &Demandes, r: &Partage, cl: &Client, id: u64) -> Result<(Value, Vec<Value>), Echec> {
    avec_contact(d, r, cl, |s| {
        let mut conv = get(d, &format!("/contacts/{s}/conversations/{id}"))?;
        completer(&r.base(), cl.compte, &mut conv);
        let mut messages: Vec<Value> = Vec::new();
        let mut avant: Option<u64> = None;
        for _ in 0..25 {
            let chemin = match avant {
                Some(b) => format!("/contacts/{s}/conversations/{id}/messages?before={b}"),
                None => format!("/contacts/{s}/conversations/{id}/messages"),
            };
            let lot = liste_de(get(d, &chemin)?);
            let n = lot.len();
            let plus_ancien = lot.iter().filter_map(|m| m["id"].as_u64()).min();
            messages.extend(lot);
            match plus_ancien {
                Some(b) if n >= 20 && avant.is_none_or(|a| b < a) => avant = Some(b),
                _ => break,
            }
        }
        messages.sort_by_key(|m| (m["created_at"].as_u64().unwrap_or(0), m["id"].as_u64().unwrap_or(0)));
        messages.dedup_by_key(|m| m["id"].as_u64());
        let _ = post(d, &format!("/contacts/{s}/conversations/{id}/update_last_seen"), &json!({}));
        Ok((conv, messages))
    })
}

/// Les champs du formulaire.
pub struct Nouvelle {
    pub type_demande: String,
    pub urgence: String,
    pub objet: String,
    pub module: String,
    pub message: String,
}

/// Crée la demande puis son premier message ; rend le numéro de la demande.
pub fn creer(d: &Demandes, r: &Partage, cl: &Client, n: &Nouvelle, pieces: &[Piece]) -> Result<u64, Echec> {
    avec_contact(d, r, cl, |s| {
        let attributs = json!({ "type_demande": n.type_demande, "urgence": n.urgence, "objet": n.objet, "module": n.module, "appli": "rip-cloud" });
        let conv = post(d, &format!("/contacts/{s}/conversations"), &json!({ "custom_attributes": attributs }))?;
        let id = conv["id"].as_u64().ok_or_else(|| Echec::Service("support : demande sans numéro".into()))?;
        retenir_attributs(&r.base(), cl.compte, id, &attributs);
        post_message(d, &format!("/contacts/{s}/conversations/{id}/messages"), &n.message, pieces)?;
        Ok(id)
    })
}

pub fn repondre(d: &Demandes, r: &Partage, cl: &Client, id: u64, message: &str, pieces: &[Piece]) -> Result<(), Echec> {
    avec_contact(d, r, cl, |s| post_message(d, &format!("/contacts/{s}/conversations/{id}/messages"), message, pieces).map(|_| ()))
}

/// Clôt la demande (seulement si elle n'est pas déjà résolue : l'appel bascule l'état).
pub fn resoudre(d: &Demandes, r: &Partage, cl: &Client, id: u64) -> Result<(), Echec> {
    avec_contact(d, r, cl, |s| {
        let conv = get(d, &format!("/contacts/{s}/conversations/{id}"))?;
        if conv["status"].as_str() == Some("resolved") {
            return Ok(());
        }
        post(d, &format!("/contacts/{s}/conversations/{id}/toggle_status"), &json!({})).map(|_| ())
    })
}

// ————— Affichage —————

pub fn libelle_statut(s: &str) -> &'static str {
    match s {
        "resolved" => "Résolue",
        "pending" | "snoozed" => "En attente",
        _ => "Ouverte",
    }
}

fn badge(statut: &str) -> String {
    let classe = match statut {
        "resolved" => " fin",
        "pending" | "snoozed" => " attente",
        _ => "",
    };
    format!(r#"<span class="badge{classe}">{}</span>"#, libelle_statut(statut))
}

fn libelle(liste: &[(&str, &'static str)], code: &str) -> String {
    liste.iter().find(|(c, _)| *c == code).map_or_else(|| if code.is_empty() { "Discussion".to_string() } else { code.to_string() }, |(_, l)| l.to_string())
}

fn attributs(conv: &Value) -> &Value {
    &conv["custom_attributes"]
}

/// Messages visibles du client : jamais les notes internes (l'API ne les rend pas ; filtrées quand même).
fn visibles(messages: &[Value]) -> impl Iterator<Item = &Value> {
    messages.iter().filter(|m| m["private"].as_bool() != Some(true))
}

fn type_message(m: &Value) -> i64 {
    m["message_type"].as_i64().unwrap_or_else(|| match m["message_type"].as_str() {
        Some("outgoing") => 1,
        Some("activity") => 2,
        Some("template") => 3,
        _ => 0,
    })
}

/// Objet de la demande, ou les premiers mots de son premier message.
fn objet(conv: &Value, messages: &[Value]) -> String {
    if let Some(o) = attributs(conv)["objet"].as_str().map(str::trim).filter(|o| !o.is_empty()) {
        return o.to_string();
    }
    let premier = visibles(messages).find(|m| type_message(m) == 0).and_then(|m| m["content"].as_str()).unwrap_or("").trim();
    if premier.is_empty() {
        return "Discussion en direct".into();
    }
    let mots: String = premier.chars().take(70).collect();
    if premier.chars().count() > 70 { format!("{}…", mots.trim_end()) } else { mots }
}

fn dernier(messages: &[Value]) -> Option<&Value> {
    visibles(messages).filter(|m| type_message(m) != 2).max_by_key(|m| m["created_at"].as_u64().unwrap_or(0))
}

/// Réponse du support pas encore lue.
fn non_lu(conv: &Value, messages: &[Value]) -> bool {
    let vu = conv["contact_last_seen_at"].as_u64().unwrap_or(0);
    dernier(messages).is_some_and(|m| type_message(m) == 1 && m["created_at"].as_u64().unwrap_or(0) > vu)
}

const STYLE: &str = r#"<style>
.fil { display: grid; gap: .7rem; margin: 0 0 1rem; }
.msg { max-width: 85%; padding: .6rem .8rem; border-radius: 12px; background: var(--champ); box-shadow: 0 0 0 .5px var(--champ-bord); }
.msg.moi { justify-self: end; background: var(--accent-doux); }
.msg .qui { color: var(--doux); font-size: 12px; margin-bottom: .15rem; }
.msg .corps { white-space: pre-wrap; word-break: break-word; }
.msg .pj { margin-top: .4rem; font-size: 13px; display: grid; gap: .15rem; }
.etat-ligne { text-align: center; color: var(--doux); font-size: 12px; }
.badge { display: inline-block; padding: .05rem .5rem; border-radius: 999px; background: var(--accent-doux); color: var(--accent); font-size: 12px; font-weight: 600; white-space: nowrap; }
.badge.fin { background: color-mix(in srgb, var(--ok) 16%, transparent); color: var(--ok); }
.badge.attente { background: color-mix(in srgb, #ff9f0a 18%, transparent); color: #b36b00; }
.pastille { display: inline-block; width: .55rem; height: .55rem; border-radius: 50%; background: var(--accent); margin-right: .35rem; vertical-align: middle; }
.actions-demandes { display: flex; gap: .6rem; flex-wrap: wrap; align-items: center; }
textarea { width: 100%; min-height: 9rem; font: inherit; }
</style>"#;

fn lien_direct() -> String {
    if crate::support::actif() { r#"<a class="bouton discret" href="/support" data-support>Discuter en direct</a>"#.to_string() } else { String::new() }
}

fn page_demandes(s: &Session, corps: &str) -> Response {
    page("Support", Some(s), &format!("{STYLE}{corps}")).into_response()
}

fn erreur(texte: &str) -> String {
    format!(r#"<div class="carte"><p class="erreur">{}</p></div>"#, echapper(texte))
}

/// Liste « Mes demandes » (rendue par `support::liste` quand le service est en marche).
pub(crate) async fn liste(r: Partage, s: Session, anciennes: String) -> Response {
    let d = match configuration() {
        Some(d) => d,
        None => return Redirect::to("/support").into_response(),
    };
    let (r2, cl) = (r.clone(), Client::de(&s));
    let res = tokio::task::spawn_blocking(move || lister(&d, &r2, &cl)).await.unwrap_or_else(|_| Err(Echec::Service("interrompu".into())));
    let tableau = match res {
        Ok(mut convs) => {
            convs.sort_by_key(|c| std::cmp::Reverse(dernier(c["messages"].as_array().map_or(&[][..], |v| v)).and_then(|m| m["created_at"].as_u64()).unwrap_or(0)));
            if convs.is_empty() {
                r#"<p class="meta">Aucune demande pour l'instant.</p>"#.to_string()
            } else {
                let lignes: String = convs
                    .iter()
                    .filter_map(|c| {
                        let id = c["id"].as_u64()?;
                        let messages = c["messages"].as_array().map_or(&[][..], |v| v);
                        let quand = dernier(messages).and_then(|m| m["created_at"].as_u64());
                        Some(format!(
                            r#"<tr><td>{pastille}<a href="/support/{id}"><strong>{objet}</strong></a><br><span class="meta">n° {id}</span></td><td>{genre}</td><td>{badge}</td><td>{date}</td></tr>"#,
                            pastille = if non_lu(c, messages) { r#"<span class="pastille" title="Nouvelle réponse"></span>"# } else { "" },
                            objet = echapper(&objet(c, messages)),
                            genre = echapper(&libelle(&TYPES, attributs(c)["type_demande"].as_str().unwrap_or(""))),
                            badge = badge(c["status"].as_str().unwrap_or("open")),
                            date = quand.map_or("—".to_string(), crate::temps::date_heure_fr),
                        ))
                    })
                    .collect();
                format!(r#"<div class="tableau"><table><thead><tr><th>Demande</th><th>Type</th><th>Statut</th><th>Dernier message</th></tr></thead><tbody>{lignes}</tbody></table></div>"#)
            }
        }
        Err(e) => {
            eprintln!("mes demandes : liste impossible ({e:?})");
            format!(r#"<p class="erreur">{INDISPONIBLE}</p>"#)
        }
    };
    page_demandes(
        &s,
        &format!(
            r#"<h1>Mes demandes</h1><div class="carte"><div class="actions-demandes"><a class="bouton" href="/support/nouvelle">Nouvelle demande</a>{}</div><p class="meta">Vos demandes et vos discussions avec l'équipe d'E-KO. Vous recevez un e-mail à chaque réponse.</p>{tableau}</div>{anciennes}"#,
            lien_direct()
        ),
    )
}

fn options(liste: &[(&str, &str)], choisi: &str) -> String {
    liste.iter().map(|(c, l)| format!(r#"<option value="{c}"{}>{}</option>"#, if *c == choisi { " selected" } else { "" }, echapper(l))).collect()
}

/// Sites du compte, pour le champ « Concerne ».
fn modules(r: &Partage, compte: i64) -> Vec<(String, String)> {
    let mes = crate::base::sites_du_compte(&r.base(), compte).unwrap_or_default();
    let tous = crate::sites::lire(&r.config.donnees).unwrap_or_default();
    let mut v: Vec<(String, String)> = mes.iter().filter_map(|(id, _)| tous.iter().find(|x| &x.id == id)).map(|s| (s.nom.clone(), format!("Site « {} »", s.nom))).collect();
    v.push(("compte".into(), "Mon compte ou la plate-forme".into()));
    v.push(("abonnement".into(), "Mon abonnement ou une facture".into()));
    v.push(("autre".into(), "Autre".into()));
    v
}

const ACCEPTE: &str = "image/*,application/pdf,.pdf,.log,.txt,.json,.zip";

fn champ_pieces() -> String {
    format!(r#"<label>Pièces jointes (5 au plus, 10 Mo chacune : images, PDF, journaux)<input type="file" name="pieces" multiple accept="{ACCEPTE}"></label>"#)
}

fn formulaire(r: &Partage, s: &Session, n: &Nouvelle, message_erreur: &str) -> String {
    let mods: String = modules(r, s.compte.id).iter().map(|(c, l)| format!(r#"<option value="{}"{}>{}</option>"#, echapper(c), if *c == n.module { " selected" } else { "" }, echapper(l))).collect();
    format!(
        r#"<p><a href="/support">← Mes demandes</a></p><h1>Nouvelle demande</h1>{err}<div class="carte"><form method="post" action="/support/nouvelle" enctype="multipart/form-data">{csrf}<label>Type de demande<select name="type_demande">{types}</select></label><label>Urgence<select name="urgence">{urgences}</select></label><label>Objet<input type="text" name="objet" required maxlength="150" value="{objet}"></label><label>Concerne<select name="module">{mods}</select></label><label>Message<textarea name="message" required maxlength="20000">{message}</textarea></label>{pieces}<button type="submit">Envoyer la demande</button></form><p class="meta">Une demande bloquante est traitée en priorité. Pour une question rapide, utilisez plutôt la discussion en direct.</p></div>"#,
        err = if message_erreur.is_empty() { String::new() } else { erreur(message_erreur) },
        csrf = hidden_csrf(s),
        types = options(&TYPES, &n.type_demande),
        urgences = options(&URGENCES, &n.urgence),
        objet = echapper(&n.objet),
        message = echapper(&n.message),
        pieces = champ_pieces(),
    )
}

/// `GET /support/nouvelle`.
pub async fn nouvelle(State(r): State<Partage>, h: HeaderMap) -> Response {
    let s = match exiger(&r, &h, "/support/nouvelle") {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    if !actif() {
        return Redirect::to("/support").into_response();
    }
    let vide = Nouvelle { type_demande: "technique".into(), urgence: "normale".into(), objet: String::new(), module: String::new(), message: String::new() };
    page_demandes(&s, &formulaire(&r, &s, &vide, ""))
}

/// Formulaire envoyé : champs texte et pièces jointes (contrôlées).
struct Envoi {
    champs: HashMap<String, String>,
    pieces: Vec<Piece>,
}

fn genre_accepte(nom: &str, genre: &str) -> bool {
    let ext = nom.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    genre.starts_with("image/") || genre == "application/pdf" || matches!(ext.as_str(), "pdf" | "png" | "jpg" | "jpeg" | "gif" | "webp" | "heic" | "log" | "txt" | "json" | "zip")
}

async fn lire_envoi(mut m: Multipart) -> Result<Envoi, String> {
    let mut champs = HashMap::new();
    let mut pieces = Vec::new();
    while let Some(f) = m.next_field().await.map_err(|_| "Envoi incomplet : réessayez (10 Mo au plus par pièce jointe).".to_string())? {
        let nom = f.name().unwrap_or("").to_string();
        if nom == "pieces" {
            let fichier = f.file_name().unwrap_or("").trim().to_string();
            let genre = f.content_type().unwrap_or("application/octet-stream").to_string();
            let octets = f.bytes().await.map_err(|_| "Pièce jointe illisible : 10 Mo au plus par fichier.".to_string())?;
            if fichier.is_empty() && octets.is_empty() {
                continue;
            }
            if pieces.len() >= MAX_PIECES {
                return Err(format!("{MAX_PIECES} pièces jointes au plus."));
            }
            if octets.len() > MAX_OCTETS_PIECE {
                return Err(format!("« {fichier} » dépasse 10 Mo."));
            }
            if !genre_accepte(&fichier, &genre) {
                return Err(format!("« {fichier} » : seuls les images, les PDF et les journaux (.log, .txt) sont acceptés."));
            }
            let nom_fichier: String = fichier.chars().filter(|c| !c.is_control() && !"/\\".contains(*c)).take(120).collect();
            pieces.push(Piece { nom: if nom_fichier.is_empty() { "piece-jointe".into() } else { nom_fichier }, genre, octets: octets.to_vec() });
        } else {
            let v = f.text().await.unwrap_or_default();
            champs.insert(nom, v);
        }
    }
    Ok(Envoi { champs, pieces })
}

fn champ(e: &Envoi, nom: &str) -> String {
    e.champs.get(nom).map(|v| v.trim().to_string()).unwrap_or_default()
}

fn code_ou(liste: &[(&str, &str)], v: &str, defaut: &str) -> String {
    if liste.iter().any(|(c, _)| *c == v) { v.to_string() } else { defaut.to_string() }
}

/// `POST /support/nouvelle`.
pub async fn envoyer_nouvelle(State(r): State<Partage>, h: HeaderMap, m: Multipart) -> Response {
    if !origine_ok(&r, &h) {
        return refuse_origine();
    }
    let s = match exiger(&r, &h, "/support/nouvelle") {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    let Some(d) = configuration() else { return Redirect::to("/support").into_response() };
    let envoi = lire_envoi(m).await;
    let vide = Nouvelle { type_demande: "technique".into(), urgence: "normale".into(), objet: String::new(), module: String::new(), message: String::new() };
    let e = match envoi {
        Ok(e) => e,
        Err(texte) => return page_demandes(&s, &formulaire(&r, &s, &vide, &texte)),
    };
    if !csrf_ok(&s, &e.champs) {
        return refuse_origine();
    }
    let n = Nouvelle {
        type_demande: code_ou(&TYPES, &champ(&e, "type_demande"), "autre"),
        urgence: code_ou(&URGENCES, &champ(&e, "urgence"), "normale"),
        objet: champ(&e, "objet").chars().take(150).collect(),
        module: champ(&e, "module").chars().take(120).collect(),
        message: champ(&e, "message").chars().take(20_000).collect(),
    };
    if n.objet.is_empty() || n.message.is_empty() {
        return page_demandes(&s, &formulaire(&r, &s, &n, "L'objet et le message sont obligatoires."));
    }
    let (r2, cl, pieces) = (r.clone(), Client::de(&s), e.pieces);
    let n2 = Nouvelle { type_demande: n.type_demande.clone(), urgence: n.urgence.clone(), objet: n.objet.clone(), module: n.module.clone(), message: n.message.clone() };
    match tokio::task::spawn_blocking(move || creer(&d, &r2, &cl, &n2, &pieces)).await {
        Ok(Ok(id)) => Redirect::to(&format!("/support/{id}")).into_response(),
        other => {
            eprintln!("mes demandes : création impossible ({:?})", other.as_ref().map(|x| x.as_ref().err()));
            page_demandes(&s, &formulaire(&r, &s, &n, INDISPONIBLE))
        }
    }
}

fn nom_piece(a: &Value) -> String {
    let url = a["data_url"].as_str().unwrap_or("");
    let brut = url.split('?').next().unwrap_or("").rsplit('/').next().unwrap_or("");
    let nom = crate::portail::decoder(brut);
    if nom.is_empty() { "pièce jointe".into() } else { nom }
}

/// Lien de téléchargement d'une pièce jointe : seulement vers le support.
fn lien_piece(a: &Value, base: &str) -> Option<String> {
    let url = a["data_url"].as_str()?;
    (url.starts_with(&format!("{base}/")) || url.starts_with(&format!("{ORIGINE_SUPPORT}/"))).then(|| format!(r#"<a href="{}" target="_blank" rel="noopener">📎 {}</a>"#, echapper(url), echapper(&nom_piece(a))))
}

fn fil(messages: &[Value], base: &str) -> String {
    let mut sortie = String::new();
    for m in visibles(messages) {
        let quand = crate::temps::date_heure_fr(m["created_at"].as_u64().unwrap_or(0));
        let contenu = m["content"].as_str().unwrap_or("").trim();
        match type_message(m) {
            2 => sortie.push_str(&format!(r#"<div class="etat-ligne">{} · {quand}</div>"#, echapper(contenu))),
            t => {
                let moi = t == 0;
                let qui = if moi {
                    "Vous".to_string()
                } else {
                    m["sender"]["available_name"].as_str().or(m["sender"]["name"].as_str()).filter(|n| !n.trim().is_empty()).map_or("Support E-KO".to_string(), |n| format!("{} · support E-KO", n.trim()))
                };
                let pj: String = m["attachments"].as_array().map(|v| v.iter().filter_map(|a| lien_piece(a, base)).collect()).unwrap_or_default();
                if contenu.is_empty() && pj.is_empty() {
                    continue;
                }
                sortie.push_str(&format!(
                    r#"<div class="msg{}"><div class="qui">{} · {quand}</div>{}{}</div>"#,
                    if moi { " moi" } else { "" },
                    echapper(&qui),
                    if contenu.is_empty() { String::new() } else { format!(r#"<div class="corps">{}</div>"#, echapper(contenu)) },
                    if pj.is_empty() { String::new() } else { format!(r#"<div class="pj">{pj}</div>"#) },
                ));
            }
        }
    }
    sortie
}

/// `GET /support/{id}` d'une demande du support (numérique).
pub(crate) async fn demande(r: Partage, s: Session, id: u64, message_erreur: &str) -> Response {
    let Some(d) = configuration() else { return Redirect::to("/support").into_response() };
    let base = d.base.clone();
    let (r2, cl) = (r.clone(), Client::de(&s));
    let res = tokio::task::spawn_blocking(move || detail(&d, &r2, &cl, id)).await.unwrap_or_else(|_| Err(Echec::Service("interrompu".into())));
    let (conv, messages) = match res {
        Ok(x) => x,
        Err(Echec::Introuvable) => return Redirect::to("/support").into_response(),
        Err(e) => {
            eprintln!("mes demandes : détail {id} impossible ({e:?})");
            return page_demandes(&s, &format!(r#"<p><a href="/support">← Mes demandes</a></p>{}"#, erreur(INDISPONIBLE)));
        }
    };
    let statut = conv["status"].as_str().unwrap_or("open");
    let a = attributs(&conv);
    let mut infos = vec![format!("n° {id}")];
    if let Some(t) = a["type_demande"].as_str() {
        infos.push(libelle(&TYPES, t));
    }
    if let Some(u) = a["urgence"].as_str().filter(|u| *u != "normale") {
        infos.push(format!("urgence {}", libelle(&URGENCES, u).to_lowercase()));
    }
    if let Some(m) = a["module"].as_str().filter(|m| !m.is_empty()) {
        infos.push(m.to_string());
    }
    let resoudre = if statut == "resolved" {
        String::new()
    } else {
        format!(r#"<form method="post" action="/support/{id}/resoudre">{}<button class="discret" type="submit">Marquer comme résolue</button></form>"#, hidden_csrf(&s))
    };
    let reponse = format!(
        r#"<div class="carte"><h2>Répondre</h2><form method="post" action="/support/{id}/repondre" enctype="multipart/form-data">{}<label>Message<textarea name="message" required maxlength="20000"></textarea></label>{}<div class="actions-demandes"><button type="submit">Envoyer</button></div></form>{}</div>"#,
        hidden_csrf(&s),
        champ_pieces(),
        if statut == "resolved" { r#"<p class="meta">Cette demande est résolue : une réponse la rouvre.</p>"# } else { "" },
    );
    page_demandes(
        &s,
        &format!(
            r#"<p><a href="/support">← Mes demandes</a></p><div class="carte"><h1>{objet}</h1><p>{badge} <span class="meta">{infos}</span></p><div class="actions-demandes">{resoudre}{direct}</div></div>{err}<div class="fil">{fil}</div>{reponse}"#,
            objet = echapper(&objet(&conv, &messages)),
            badge = badge(statut),
            infos = echapper(&infos.join(" · ")),
            direct = lien_direct(),
            err = if message_erreur.is_empty() { String::new() } else { erreur(message_erreur) },
            fil = fil(&messages, &base),
        ),
    )
}

/// `POST /support/{id}/repondre`.
pub async fn envoyer_reponse(State(r): State<Partage>, h: HeaderMap, Chemin(id): Chemin<String>, m: Multipart) -> Response {
    let Ok(id) = id.parse::<u64>() else { return StatusCode::NOT_FOUND.into_response() };
    if !origine_ok(&r, &h) {
        return refuse_origine();
    }
    let s = match exiger(&r, &h, &format!("/support/{id}")) {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    let Some(d) = configuration() else { return Redirect::to("/support").into_response() };
    let e = match lire_envoi(m).await {
        Ok(e) => e,
        Err(texte) => return demande(r, s, id, &texte).await,
    };
    if !csrf_ok(&s, &e.champs) {
        return refuse_origine();
    }
    let message: String = champ(&e, "message").chars().take(20_000).collect();
    if message.is_empty() {
        return demande(r, s, id, "Le message est vide.").await;
    }
    let (r2, cl, pieces) = (r.clone(), Client::de(&s), e.pieces);
    match tokio::task::spawn_blocking(move || repondre(&d, &r2, &cl, id, &message, &pieces)).await {
        Ok(Ok(())) => Redirect::to(&format!("/support/{id}")).into_response(),
        Ok(Err(Echec::Introuvable)) => Redirect::to("/support").into_response(),
        _ => demande(r, s, id, INDISPONIBLE).await,
    }
}

/// `POST /support/{id}/resoudre`.
pub async fn marquer_resolue(State(r): State<Partage>, h: HeaderMap, Chemin(id): Chemin<String>, f: crate::portail::Champs) -> Response {
    let Ok(id) = id.parse::<u64>() else { return StatusCode::NOT_FOUND.into_response() };
    if !origine_ok(&r, &h) {
        return refuse_origine();
    }
    let s = match exiger(&r, &h, &format!("/support/{id}")) {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    if !csrf_ok(&s, &f.0) {
        return refuse_origine();
    }
    let Some(d) = configuration() else { return Redirect::to("/support").into_response() };
    let (r2, cl) = (r.clone(), Client::de(&s));
    match tokio::task::spawn_blocking(move || resoudre(&d, &r2, &cl, id)).await {
        Ok(Ok(())) => Redirect::to(&format!("/support/{id}")).into_response(),
        Ok(Err(Echec::Introuvable)) => Redirect::to("/support").into_response(),
        _ => demande(r, s, id, INDISPONIBLE).await,
    }
}

// ————— Webhook du support —————

/// `POST /webhooks/support` : signature vérifiée, puis traitement hors de la requête.
pub async fn webhook(State(r): State<Partage>, h: HeaderMap, corps: Bytes) -> Response {
    let Some(d) = configuration() else { return StatusCode::SERVICE_UNAVAILABLE.into_response() };
    if let Err(code) = d.verifier(&h, &corps, crate::maintenant()) {
        eprintln!("webhook du support refusé : {code}");
        return (StatusCode::UNAUTHORIZED, code).into_response();
    }
    let empreinte = crate::sha256_octets_hex(&corps);
    let neuf = r.base().execute("INSERT OR IGNORE INTO webhooks_support (empreinte, recu_le) VALUES (?1, ?2)", params![empreinte, crate::maintenant() as i64]).map(|n| n == 1).unwrap_or(false);
    if neuf && let Ok(v) = serde_json::from_slice::<Value>(&corps) {
        let r2 = r.clone();
        tokio::task::spawn_blocking(move || traiter_webhook(&r2, &v));
    }
    StatusCode::OK.into_response()
}

/// Contact du support concerné par un événement (conversation imbriquée ou à la racine).
fn source_de(v: &Value) -> Option<&str> {
    v["conversation"]["contact_inbox"]["source_id"].as_str().or(v["contact_inbox"]["source_id"].as_str()).filter(|s| !s.is_empty())
}

/// Rend vrai si un e-mail est parti.
pub fn traiter_webhook(r: &Partage, v: &Value) -> bool {
    let evenement = v["event"].as_str().unwrap_or("");
    let (numero, resolue) = match evenement {
        "message_created" => {
            let sortant = v["message_type"].as_str() == Some("outgoing") || v["message_type"].as_i64() == Some(1);
            if !sortant || v["private"].as_bool() == Some(true) {
                return false;
            }
            (v["conversation"]["id"].as_u64(), false)
        }
        "conversation_status_changed" => {
            let statut = v["status"].as_str().or(v["conversation"]["status"].as_str());
            if statut != Some("resolved") {
                return false;
            }
            (v["id"].as_u64().or(v["conversation"]["id"].as_u64()), true)
        }
        _ => return false,
    };
    let (Some(numero), Some(source)) = (numero, source_de(v)) else { return false };
    let c = r.base();
    let Some(id) = compte_de_source(&c, source) else { return false };
    let Ok(Some(compte)) = crate::base::compte_par_id(&c, id) else { return false };
    drop(c);
    let url = r.config.url_publique.trim_end_matches('/').to_string();
    let lien = format!("{url}/support/{numero}");
    let nom = if compte.nom.trim().is_empty() { compte.email.clone() } else { compte.nom.trim().to_string() };
    let mail = if resolue { courriels::demande_resolue(&url, &compte.email, &nom, numero, &lien) } else { courriels::demande_reponse(&url, &compte.email, &nom, numero, &lien) };
    r.envoyer(mail);
    true
}

/// Oublie les empreintes de webhooks de plus de 7 jours.
pub fn entretenir(c: &Connection, t: u64) {
    let _ = c.execute("DELETE FROM webhooks_support WHERE recu_le < ?1", [t.saturating_sub(7 * 86_400) as i64]);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> Demandes {
        Demandes::depuis(Some("boite"), Some("cle-secrete-de-test"), Some("secret-webhook"), Some("http://support.test/")).unwrap()
    }

    #[test]
    fn configuration_complete_ou_rien() {
        assert!(Demandes::depuis(None, Some("c"), Some("s"), None).is_none());
        assert!(Demandes::depuis(Some("b"), Some(" "), Some("s"), None).is_none());
        assert!(Demandes::depuis(Some("b"), Some("c"), None, None).is_none());
        let d = Demandes::depuis(Some("b"), Some("cle-tres-secrete"), Some("secret-tres-secret"), None).unwrap();
        assert_eq!(d.base, ORIGINE_SUPPORT);
        let dbg = format!("{d:?}");
        assert!(!dbg.contains("cle-tres-secrete") && !dbg.contains("secret-tres-secret"), "{dbg}");
        assert_eq!(cfg().base, "http://support.test");
    }

    #[test]
    fn empreinte_d_identite() {
        // Même vecteur que la bulle : HMAC-SHA256(« cle-secrete-de-test », « cloud-42 »).
        assert_eq!(cfg().empreinte("cloud-42"), "af70942bf3b4601b1d957255a94911ffdc15acca2412be2dbacad32a37c3412d");
    }

    #[test]
    fn signature_du_webhook() {
        let d = cfg();
        let corps = br#"{"event":"message_created"}"#;
        let t = 1_800_000_000u64;
        let mut h = HeaderMap::new();
        h.insert("x-chatwoot-timestamp", t.to_string().parse().unwrap());
        h.insert("x-chatwoot-signature", d.signature(&t.to_string(), corps).parse().unwrap());
        assert_eq!(d.verifier(&h, corps, t + 10), Ok(()));
        assert_eq!(d.verifier(&h, corps, t + TOLERANCE_WEBHOOK + 1), Err("horodatage_trop_vieux"));
        assert_eq!(d.verifier(&h, b"{}", t), Err("signature_fausse"));
        h.insert("x-chatwoot-signature", "sha256=00".parse().unwrap());
        assert_eq!(d.verifier(&h, corps, t), Err("signature_fausse"));
        h.remove("x-chatwoot-timestamp");
        assert_eq!(d.verifier(&h, corps, t), Err("horodatage_absent"));
        // Vecteur fixe : HMAC-SHA256(« secret-webhook », « 1800000000.{} »).
        assert_eq!(d.signature("1800000000", b"{}"), format!("sha256={}", crate::hex(&crate::hmac_sha256(b"secret-webhook", b"1800000000.{}"))));
    }

    #[test]
    fn objet_statut_et_non_lu() {
        let conv = json!({ "id": 7, "status": "pending", "contact_last_seen_at": 100, "custom_attributes": {} });
        let messages = vec![
            json!({ "id": 1, "message_type": 0, "content": "Bonjour, mon imprimante Epson ne sort plus rien depuis la mise à jour de ce matin, que faire ?", "created_at": 90 }),
            json!({ "id": 2, "message_type": 2, "content": "Conversation attribuée", "created_at": 95 }),
            json!({ "id": 3, "message_type": 1, "content": "Bonjour Claire", "created_at": 120 }),
            json!({ "id": 4, "message_type": 1, "content": "note", "private": true, "created_at": 130 }),
        ];
        assert!(objet(&conv, &messages).starts_with("Bonjour, mon imprimante") && objet(&conv, &messages).ends_with('…'));
        assert!(non_lu(&conv, &messages));
        assert!(!non_lu(&json!({ "contact_last_seen_at": 125 }), &messages), "la note interne ne compte pas");
        assert_eq!(libelle_statut("pending"), "En attente");
        assert_eq!(libelle_statut("resolved"), "Résolue");
        let html = fil(&messages, "http://support.test");
        assert!(!html.contains("note"), "note interne jamais affichée");
        assert!(html.contains("Support E-KO") && html.contains(r#"class="msg moi""#));
        let avec_objet = json!({ "custom_attributes": { "objet": "Profil ICC absent" } });
        assert_eq!(objet(&avec_objet, &messages), "Profil ICC absent");
    }

    #[test]
    fn pieces_jointes_acceptees() {
        assert!(genre_accepte("photo.jpg", "image/jpeg") && genre_accepte("journal.log", "application/octet-stream") && genre_accepte("devis.pdf", "application/pdf"));
        assert!(!genre_accepte("outil.exe", "application/x-msdownload"));
        let a = json!({ "data_url": "http://support.test/rails/active_storage/blobs/x/journal%20du%20jour.log" });
        assert!(lien_piece(&a, "http://support.test").unwrap().contains("journal du jour.log"));
        assert!(lien_piece(&json!({ "data_url": "https://ailleurs.test/x.png" }), "http://support.test").is_none(), "jamais de lien vers un autre site");
    }
}
