//! Support d'E-KO RIP Cloud : demandes et discussion en direct des clients de la plate-forme.
//!
//! Tout est stocké et traité dans l'ERP « E-KO » par son API (`/support`, même jeton et même
//! adresse que la facturation, droits `support.view` et `support.create`). La plate-forme n'en
//! garde qu'un index local, tenu par les webhooks `support.*` :
//! - les demandes du compte (statut, note), pour la liste et pour prévenir une seule fois ;
//! - les discussions en direct et leurs messages, que la bulle du navigateur relit toutes les
//!   3 secondes (interrogation légère de la plate-forme : le frontal d'hébergement garde le
//!   sens montant des WebSocket, il n'y en a donc pas ici).
//!
//! L'ERP n'écrit jamais aux clients du Cloud : les e-mails (demande reçue, réponse, demande
//! résolue avec le lien de notation, discussion transformée en demande) partent d'ici, avec le
//! gabarit E-KO RIP Cloud, ainsi que la notification du portail.
//!
//! Le client de la plate-forme est désigné dans l'ERP par `account_reference` (`cloud-<id du
//! compte>`), lié à un tiers par son code client (abonnement) ou à défaut par l'e-mail du compte.

use crate::base::{self, Compte};
use crate::courriels;
use crate::erp::Erp;
use crate::notifications::{self, Notification};
use crate::portail::{
    self, Champs, Session, champ, csrf_entete_ok, csrf_ok, echapper, encoder, exiger, hidden_csrf,
    origine_ok, page, refuse_origine, reponse_json, session,
};
use crate::relais::Partage;
use axum::Json;
use axum::extract::{Multipart, Path as Chemin, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::time::Duration;

/// Pièces jointes d'une demande : au plus 5 fichiers de 10 Mo (limites de l'API).
pub const MAX_PIECES: usize = 5;
pub const MAX_PIECE: usize = 10 * 1024 * 1024;
/// Taille maximale d'une requête de création (5 pièces et le texte).
pub const MAX_REQUETE: usize = MAX_PIECES * MAX_PIECE + 1024 * 1024;
const MAX_MESSAGE_DEMANDE: usize = 20_000;
const MAX_MESSAGE_DISCUSSION: usize = 5_000;
/// Durée de vie des files (catégories) et de l'état « ouvert / fermé » gardés en mémoire.
const CACHE_CATEGORIES: u64 = 600;
const CACHE_OUVERT: u64 = 3600;
/// Une discussion terminée reste visible dans la bulle une heure, puis la bulle repart à neuf.
const VISIBLE_APRES_FIN: u64 = 3600;
/// Les discussions terminées sont oubliées après 30 jours (la demande, elle, reste dans l'ERP).
const CONSERVATION_DISCUSSIONS: u64 = 30 * 86_400;

// ————— Identité du client —————

/// Référence du compte de la plate-forme dans l'ERP.
pub fn reference(compte: i64) -> String {
    format!("cloud-{compte}")
}

/// Compte désigné par une référence de l'ERP, s'il existe.
pub fn compte_de_reference(c: &Connection, reference: &str) -> Option<i64> {
    let id: i64 = reference.strip_prefix("cloud-")?.parse().ok()?;
    c.query_row("SELECT 1 FROM comptes WHERE id = ?1", [id], |_| Ok(()))
        .optional()
        .ok()
        .flatten()
        .map(|_| id)
}

/// Les champs qui désignent le client et la personne qui écrit (création d'une demande ou d'une
/// discussion).
pub struct Identite {
    pub compte: i64,
    pub email: String,
    pub prenom: String,
    pub nom: String,
    pub tier_code: Option<String>,
}

impl Identite {
    pub fn depuis(c: &Connection, compte: &Compte) -> Identite {
        let (prenom, nom) = match compte.nom.trim().split_once(' ') {
            Some((p, n)) => (p.trim().to_string(), n.trim().to_string()),
            None => (compte.nom.trim().to_string(), String::new()),
        };
        let tier_code = crate::abonnements::lire(c, compte.id)
            .ok()
            .and_then(|a| a.tier_code)
            .filter(|t| !t.is_empty());
        Identite {
            compte: compte.id,
            email: compte.email.clone(),
            prenom,
            nom,
            tier_code,
        }
    }

    fn champs(&self) -> Vec<(String, String)> {
        let rogner = |s: &str, n: usize| s.chars().take(n).collect::<String>();
        let mut v = vec![("account_reference".to_string(), reference(self.compte))];
        match &self.tier_code {
            Some(t) => v.push(("tier_code".into(), rogner(t, 64))),
            None => v.push(("tier_email".into(), rogner(&self.email, 191))),
        }
        v.push(("contact_email".into(), rogner(&self.email, 191)));
        if !self.prenom.is_empty() {
            v.push(("contact_first_name".into(), rogner(&self.prenom, 120)));
        }
        if !self.nom.is_empty() {
            v.push(("contact_last_name".into(), rogner(&self.nom, 120)));
        }
        v
    }

    fn json(&self) -> serde_json::Map<String, Value> {
        self.champs()
            .into_iter()
            .map(|(k, v)| (k, Value::String(v)))
            .collect()
    }
}

// ————— Client de l'API —————

/// Message à montrer au client pour un refus de l'ERP.
fn refus(rep: &crate::erp::Reponse) -> String {
    match rep.statut {
        401 | 403 => "Le support n'est pas joignable pour le moment (accès refusé par l'ERP). Réessayez plus tard.".into(),
        404 => "Introuvable.".into(),
        409 => "Cette discussion est terminée.".into(),
        422 if rep.corps["errors"].get("tier_code").is_some() => "Votre compte n'est pas encore relié à un dossier client E-KO : le support ne peut pas encore l'enregistrer. Souscrivez une offre ou contactez E-KO.".into(),
        429 => "Trop de demandes en peu de temps : réessayez dans une minute.".into(),
        s if s >= 500 => "Le support est momentanément indisponible. Réessayez dans un instant.".into(),
        _ => rep.erreur(),
    }
}

fn erp_pret(r: &Partage) -> Result<Erp, String> {
    Erp::depuis(&r.base())
        .ok_or_else(|| "Le support n'est pas encore ouvert sur cette plate-forme.".to_string())
}

/// Les files proposées (`GET /support/categories`) : `(code, libellé)`. Gardées 10 minutes ;
/// à défaut de réponse de l'ERP, la dernière liste connue.
pub fn categories(r: &Partage) -> Result<Vec<(String, String)>, String> {
    let t = crate::maintenant();
    let lire_cache = |c: &Connection| -> Option<Vec<(String, String)>> {
        let v: Value =
            serde_json::from_str(&base::reglage(c, "support_categories").ok().flatten()?).ok()?;
        Some(
            v.as_array()?
                .iter()
                .filter_map(|x| {
                    Some((
                        x["code"].as_str()?.to_string(),
                        x["label"].as_str()?.to_string(),
                    ))
                })
                .collect(),
        )
    };
    {
        let c = r.base();
        let le: u64 = base::reglage(&c, "support_categories_le")
            .ok()
            .flatten()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        if t.saturating_sub(le) < CACHE_CATEGORIES
            && let Some(l) = lire_cache(&c).filter(|l| !l.is_empty())
        {
            return Ok(l);
        }
    }
    let frais = erp_pret(r).and_then(|erp| {
        let rep = erp.get("/support/categories")?;
        if !rep.ok() {
            return Err(refus(&rep));
        }
        let l: Vec<(String, String)> = rep
            .donnees()
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(|x| {
                Some((
                    x["code"].as_str()?.to_string(),
                    x["label"].as_str()?.to_string(),
                ))
            })
            .collect();
        Ok(l)
    });
    let c = r.base();
    match frais {
        Ok(l) if !l.is_empty() => {
            let _ = base::regler(
                &c,
                "support_categories",
                &json!(
                    l.iter()
                        .map(|(c, l)| json!({ "code": c, "label": l }))
                        .collect::<Vec<_>>()
                )
                .to_string(),
            );
            let _ = base::regler(&c, "support_categories_le", &t.to_string());
            Ok(l)
        }
        Ok(_) => Err("Aucune file de support n'est ouverte pour le moment.".into()),
        Err(e) => lire_cache(&c).filter(|l| !l.is_empty()).ok_or(e),
    }
}

/// Une pièce jointe reçue du navigateur : (nom, type, octets).
pub type Piece = (String, String, Vec<u8>);

/// Ouvre une demande (`POST /support/tickets`) et la range dans l'index local. `cle` rend le
/// renvoi du formulaire inoffensif (`Idempotency-Key`).
pub fn creer_demande(
    r: &Partage,
    ident: &Identite,
    cle: &str,
    categorie: &str,
    sujet: &str,
    message: &str,
    pieces: &[Piece],
) -> Result<Value, String> {
    let erp = erp_pret(r)?;
    // Le webhook `support.ticket.created` peut arriver avant la réponse : il reconnaît ainsi
    // une demande ouverte ici (et la signale par e-mail) d'une discussion transformée.
    let t = crate::maintenant();
    let marque = {
        let c = r.base();
        let _ = c.execute(
            "DELETE FROM support_creations WHERE debut < ?1",
            [t.saturating_sub(300) as i64],
        );
        c.execute(
            "INSERT INTO support_creations (compte, debut) VALUES (?1, ?2)",
            params![ident.compte, t as i64],
        )
        .ok()
        .map(|_| c.last_insert_rowid())
    };
    let idem = format!("cloud-{}-{cle}", ident.compte);
    let rep = if pieces.is_empty() {
        let mut corps = ident.json();
        corps.insert("category".into(), json!(categorie));
        corps.insert("subject".into(), json!(sujet));
        corps.insert("message".into(), json!(message));
        erp.post("/support/tickets", &Value::Object(corps), Some(&idem))
    } else {
        let mut champs = ident.champs();
        champs.push(("category".into(), categorie.into()));
        champs.push(("subject".into(), sujet.into()));
        champs.push(("message".into(), message.into()));
        let fichiers: Vec<(String, String, String, Vec<u8>)> = pieces
            .iter()
            .map(|(n, g, o)| ("attachments[]".to_string(), n.clone(), g.clone(), o.clone()))
            .collect();
        erp.post_multipart("/support/tickets", &champs, &fichiers, Some(&idem))
    };
    if let Some(m) = marque {
        let _ = r
            .base()
            .execute("DELETE FROM support_creations WHERE rowid = ?1", [m]);
    }
    let rep = rep?;
    if !rep.ok() {
        return Err(refus(&rep));
    }
    let d = rep.donnees().clone();
    enregistrer_demande(&r.base(), ident.compte, &d);
    Ok(d)
}

fn creation_en_cours(c: &Connection, compte: i64, t: u64) -> bool {
    c.query_row(
        "SELECT 1 FROM support_creations WHERE compte = ?1 AND debut >= ?2",
        params![compte, t.saturating_sub(300) as i64],
        |_| Ok(()),
    )
    .optional()
    .ok()
    .flatten()
    .is_some()
}

/// Détail d'une demande du compte (`GET /support/tickets/{code}?account_reference=`).
pub fn detail_demande(r: &Partage, compte: i64, code: &str) -> Result<Value, String> {
    let erp = erp_pret(r)?;
    let rep = erp.get(&format!(
        "/support/tickets/{}?account_reference={}",
        encoder(code),
        encoder(&reference(compte))
    ))?;
    if !rep.ok() {
        return Err(refus(&rep));
    }
    let d = rep.donnees().clone();
    enregistrer_demande(&r.base(), compte, &d);
    Ok(d)
}

/// Remet l'index local des demandes à jour d'après l'ERP (au plus les 100 plus récentes).
pub fn synchroniser_demandes(r: &Partage, compte: i64) -> Result<(), String> {
    let erp = erp_pret(r)?;
    let rep = erp.get(&format!(
        "/support/tickets?account_reference={}&per_page=100",
        encoder(&reference(compte))
    ))?;
    if !rep.ok() {
        return Err(refus(&rep));
    }
    let c = r.base();
    for t in rep.donnees().as_array().cloned().unwrap_or_default() {
        enregistrer_demande(&c, compte, &t);
    }
    Ok(())
}

fn repondre_demande(r: &Partage, compte: i64, code: &str, message: &str) -> Result<(), String> {
    let erp = erp_pret(r)?;
    let rep = erp.post(
        &format!("/support/tickets/{}/comments", encoder(code)),
        &json!({ "message": message, "account_reference": reference(compte) }),
        None,
    )?;
    if !rep.ok() {
        return Err(
            if rep.statut == 422
                && rep.corps["errors"].get("message").is_none()
                && rep.corps["errors"].get("account_reference").is_none()
            {
                "Cette demande est fermée : ouvrez-en une nouvelle.".to_string()
            } else {
                refus(&rep)
            },
        );
    }
    let _ = r.base().execute("UPDATE support_demandes SET maj_le = ?1, statut = COALESCE(NULLIF(?2, ''), statut), resolue_prevenue = 0 WHERE code = ?3 AND compte = ?4", params![crate::maintenant() as i64, rep.donnees()["ticket"]["status"].as_str().unwrap_or(""), code, compte]);
    Ok(())
}

fn noter_demande(
    r: &Partage,
    compte: i64,
    code: &str,
    note: i64,
    commentaire: &str,
) -> Result<(), String> {
    let erp = erp_pret(r)?;
    let mut corps = json!({ "rating": note, "account_reference": reference(compte) });
    if !commentaire.is_empty() {
        corps["comment"] = json!(commentaire);
    }
    let rep = erp.post(
        &format!("/support/tickets/{}/satisfaction", encoder(code)),
        &corps,
        None,
    )?;
    if !rep.ok() {
        return Err(if rep.statut == 422 {
            "La note se donne une fois la demande résolue.".to_string()
        } else {
            refus(&rep)
        });
    }
    let _ = r.base().execute(
        "UPDATE support_demandes SET note = ?1 WHERE code = ?2 AND compte = ?3",
        params![note, code, compte],
    );
    Ok(())
}

// ————— Index local —————

fn texte(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

fn date_json(v: &Value) -> Option<i64> {
    v.as_str()
        .and_then(crate::offres::date_iso)
        .map(|t| t as i64)
}

/// Note du client dans un objet demande (`satisfaction` : un nombre, ou `{rating}`).
fn note_json(t: &Value) -> Option<i64> {
    let s = &t["satisfaction"];
    s.as_i64()
        .or_else(|| s["rating"].as_i64())
        .filter(|n| (1..=5).contains(n))
}

/// Range (ou met à jour) une demande de l'ERP dans l'index du compte. Les indicateurs d'envoi
/// des e-mails ne sont jamais touchés ici.
pub fn enregistrer_demande(c: &Connection, compte: i64, t: &Value) {
    let code = texte(&t["code"]);
    if code.is_empty() {
        return;
    }
    let maintenant = crate::maintenant() as i64;
    let statut = t["status"].as_str().unwrap_or("open");
    let _ = c.execute(
        "INSERT INTO support_demandes (code, compte, sujet, categorie, statut, ouverte_le, maj_le, resolue_le, note) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(code) DO UPDATE SET sujet = CASE WHEN excluded.sujet = '' THEN sujet ELSE excluded.sujet END, categorie = CASE WHEN excluded.categorie = '' THEN categorie ELSE excluded.categorie END, statut = excluded.statut, maj_le = excluded.maj_le, resolue_le = excluded.resolue_le, note = COALESCE(excluded.note, note)
         WHERE compte = excluded.compte",
        params![code, compte, texte(&t["label"]), t["category"]["label"].as_str().or(t["category"]["code"].as_str()).unwrap_or(""), statut, date_json(&t["opened_at"]).unwrap_or(maintenant), maintenant, date_json(&t["resolved_at"]), note_json(t)],
    );
}

struct Demande {
    code: String,
    sujet: String,
    categorie: String,
    statut: String,
    maj_le: u64,
    note: Option<i64>,
}

fn demandes_du_compte(c: &Connection, compte: i64) -> Vec<Demande> {
    c.prepare("SELECT code, sujet, categorie, statut, maj_le, note FROM support_demandes WHERE compte = ?1 ORDER BY maj_le DESC, ouverte_le DESC LIMIT 200")
        .and_then(|mut q| q.query_map([compte], |l| Ok(Demande { code: l.get(0)?, sujet: l.get(1)?, categorie: l.get(2)?, statut: l.get(3)?, maj_le: l.get::<_, i64>(4)? as u64, note: l.get(5)? })).and_then(|i| i.collect()))
        .unwrap_or_default()
}

/// Libellé français d'un statut de demande.
pub fn libelle_statut(s: &str) -> &'static str {
    match s {
        "open" => "Reçue",
        "in_progress" => "En cours",
        "waiting_client" => "En attente de votre réponse",
        "waiting_internal" => "En cours de traitement",
        "resolved" => "Résolue",
        "closed" => "Fermée",
        _ => "—",
    }
}

fn demande_connue(c: &Connection, compte: i64, code: &str) -> bool {
    c.query_row(
        "SELECT 1 FROM support_demandes WHERE code = ?1 AND compte = ?2",
        params![code, compte],
        |_| Ok(()),
    )
    .optional()
    .ok()
    .flatten()
    .is_some()
}

// ————— Discussions en direct : index local —————

/// Une discussion terminée n'accepte plus de messages.
fn terminee(statut: &str) -> bool {
    matches!(statut, "close" | "convertie")
}

pub fn enregistrer_discussion(c: &Connection, compte: i64, d: &Value) -> Option<String> {
    let uuid = d["uuid"].as_str()?.to_string();
    let maintenant = crate::maintenant() as i64;
    let _ = c.execute(
        "INSERT INTO support_discussions (uuid, compte, statut, categorie, agent, ticket_code, cree_le, maj_le) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(uuid) DO UPDATE SET statut = excluded.statut, categorie = CASE WHEN excluded.categorie = '' THEN categorie ELSE excluded.categorie END, agent = CASE WHEN excluded.agent = '' THEN agent ELSE excluded.agent END, ticket_code = COALESCE(excluded.ticket_code, ticket_code), maj_le = excluded.maj_le
         WHERE compte = excluded.compte",
        params![uuid, compte, d["status"].as_str().unwrap_or("en_attente"), d["category"]["label"].as_str().or(d["category"]["code"].as_str()).unwrap_or(""), d["agent_name"].as_str().unwrap_or(""), d["ticket_code"].as_str().filter(|s| !s.is_empty()), date_json(&d["created_at"]).unwrap_or(maintenant), maintenant],
    );
    Some(uuid)
}

/// Range un message (idempotent : le webhook rejoue ce que la réponse de l'API a déjà donné).
pub fn enregistrer_message(c: &Connection, uuid: &str, m: &Value) {
    let Some(id) = m["id"].as_i64() else { return };
    let _ = c.execute(
        "INSERT OR IGNORE INTO support_messages (discussion, id, auteur_type, auteur_nom, corps, cree_le) SELECT ?1, ?2, ?3, ?4, ?5, ?6 WHERE EXISTS (SELECT 1 FROM support_discussions WHERE uuid = ?1)",
        params![uuid, id, m["author_type"].as_str().unwrap_or("agent"), m["author_name"].as_str().unwrap_or(""), m["body"].as_str().unwrap_or(""), date_json(&m["created_at"]).unwrap_or(crate::maintenant() as i64)],
    );
}

struct Discussion {
    uuid: String,
    statut: String,
    agent: String,
    ticket_code: Option<String>,
}

/// La discussion à montrer dans la bulle : celle en cours, ou la dernière si elle vient de finir.
fn discussion_courante(c: &Connection, compte: i64, t: u64) -> Option<Discussion> {
    c.query_row(
        "SELECT uuid, statut, agent, ticket_code, maj_le FROM support_discussions WHERE compte = ?1 ORDER BY cree_le DESC, rowid DESC LIMIT 1",
        [compte],
        |l| Ok((Discussion { uuid: l.get(0)?, statut: l.get(1)?, agent: l.get(2)?, ticket_code: l.get(3)? }, l.get::<_, i64>(4)? as u64)),
    )
    .optional()
    .ok()
    .flatten()
    .filter(|(d, maj)| !terminee(&d.statut) || t.saturating_sub(*maj) < VISIBLE_APRES_FIN)
    .map(|(d, _)| d)
}

fn discussion_active(c: &Connection, compte: i64) -> Option<Discussion> {
    discussion_courante(c, compte, crate::maintenant()).filter(|d| !terminee(&d.statut))
}

fn retenir_ouverture(c: &Connection, d: &Value) {
    if let Some(o) = d["open_now"].as_bool() {
        let _ = base::regler(c, "support_ouvert", if o { "1" } else { "0" });
        let _ = base::regler(c, "support_ouvert_le", &crate::maintenant().to_string());
    }
}

/// Le support est-il ouvert ? Dernier état connu, rendu par l'ERP à chaque discussion ; `None`
/// s'il date de plus d'une heure (l'API ne le donne que dans une discussion).
fn ouvert_maintenant(c: &Connection) -> Option<bool> {
    let le: u64 = base::reglage(c, "support_ouvert_le")
        .ok()
        .flatten()?
        .parse()
        .ok()?;
    if crate::maintenant().saturating_sub(le) > CACHE_OUVERT {
        return None;
    }
    Some(base::reglage(c, "support_ouvert").ok().flatten()? == "1")
}

fn messages_apres(c: &Connection, uuid: &str, apres: i64) -> Vec<Value> {
    c.prepare("SELECT id, auteur_type, auteur_nom, corps, cree_le FROM support_messages WHERE discussion = ?1 AND id > ?2 ORDER BY id LIMIT 200")
        .and_then(|mut q| {
            q.query_map(params![uuid, apres], |l| Ok(json!({ "id": l.get::<_, i64>(0)?, "type": l.get::<_, String>(1)?, "auteur": l.get::<_, String>(2)?, "texte": l.get::<_, String>(3)?, "le": l.get::<_, i64>(4)? })))
                .and_then(|i| i.collect())
        })
        .unwrap_or_default()
}

/// Corps de `GET /support/direct/etat` et des réponses d'écriture : tout vient de la base locale.
fn etat_json(c: &Connection, compte: i64, uuid_connu: &str, apres: i64) -> Value {
    let t = crate::maintenant();
    let d = discussion_courante(c, compte, t);
    let (disc, messages) = match &d {
        Some(d) => {
            let depuis = if d.uuid == uuid_connu { apres } else { 0 };
            (
                json!({ "uuid": d.uuid, "statut": d.statut, "agent": d.agent, "ticket": d.ticket_code, "fin": terminee(&d.statut) }),
                messages_apres(c, &d.uuid, depuis),
            )
        }
        None => (Value::Null, Vec::new()),
    };
    json!({ "actif": true, "ouvert": ouvert_maintenant(c), "discussion": disc, "messages": messages })
}

// ————— Webhooks `support.*` —————

fn compte_du_webhook(r: &Partage, data: &Value) -> Option<(i64, Compte)> {
    let reference = data["ticket"]["account_reference"]
        .as_str()
        .or_else(|| data["discussion"]["account_reference"].as_str())?;
    let c = r.base();
    let id = compte_de_reference(&c, reference)?;
    let compte = base::compte_par_id(&c, id).ok().flatten()?;
    Some((id, compte))
}

/// Texte lisible d'un message de l'ERP : balises retirées, entités décodées.
pub fn html_en_texte(h: &str) -> String {
    let mut sortie = String::with_capacity(h.len());
    let mut dans_balise = false;
    let mut balise = String::new();
    for c in h.chars() {
        match c {
            '<' => {
                dans_balise = true;
                balise.clear();
            }
            '>' if dans_balise => {
                dans_balise = false;
                let nom = balise
                    .trim_start_matches('/')
                    .split(|x: char| x.is_whitespace() || x == '/')
                    .next()
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if matches!(
                    nom.as_str(),
                    "br" | "p" | "div" | "li" | "tr" | "h1" | "h2" | "h3" | "blockquote"
                ) {
                    sortie.push('\n');
                }
            }
            _ if dans_balise => balise.push(c),
            _ => sortie.push(c),
        }
    }
    let sortie = sortie
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#039;", "'")
        .replace("&apos;", "'")
        .replace("&amp;", "&");
    let mut propre = String::new();
    let mut vides = 0;
    for l in sortie.lines().map(str::trim_end) {
        if l.trim().is_empty() {
            vides += 1;
            if vides > 1 || propre.is_empty() {
                continue;
            }
        } else {
            vides = 0;
        }
        propre.push_str(l);
        propre.push('\n');
    }
    propre.trim().to_string()
}

/// Rend vrai si l'événement concernait un compte de la plate-forme.
pub fn traiter_webhook(r: &Partage, evenement: &str, data: &Value) -> bool {
    let Some((id, cp)) = compte_du_webhook(r, data) else {
        eprintln!("webhook {evenement} : aucun compte ne correspond");
        return false;
    };
    let url = r.config.url_publique.clone();
    let a = cp.email.clone();
    let t = crate::maintenant();
    eprintln!("webhook {evenement} : compte {id}");
    match evenement {
        "support.ticket.created" => {
            let ticket = &data["ticket"];
            let code = texte(&ticket["code"]);
            let (connu, ici, prevenu) = {
                let c = r.base();
                let connu = demande_connue(&c, id, &code);
                let prevenu = c
                    .query_row(
                        "SELECT recu_prevenu FROM support_demandes WHERE code = ?1",
                        [&code],
                        |l| l.get::<_, i64>(0),
                    )
                    .optional()
                    .ok()
                    .flatten()
                    .unwrap_or(0)
                    == 1;
                let ici = creation_en_cours(&c, id, t);
                enregistrer_demande(&c, id, ticket);
                (connu, ici, prevenu)
            };
            // Une demande qu'on ne connaît pas et qu'on n'est pas en train d'ouvrir vient d'une
            // discussion transformée : son e-mail (avec la transcription) part au webhook `converted`.
            if (connu || ici) && !prevenu {
                let _ = r.base().execute(
                    "UPDATE support_demandes SET recu_prevenu = 1 WHERE code = ?1",
                    [&code],
                );
                let echeance = date_json(&ticket["first_response_due_at"]).map(|d| d as u64);
                let file = ticket["category"]["label"].as_str().unwrap_or("");
                notifications::notifier(
                    r,
                    id,
                    Some(courriels::support_demande_recue(
                        &url,
                        &a,
                        &cp.nom,
                        &code,
                        ticket["label"].as_str().unwrap_or(""),
                        file,
                        echeance,
                    )),
                    Notification::autre(
                        notifications::SUPPORT,
                        None,
                        "Demande reçue par le support",
                        &format!(
                            "{code} : {}",
                            courriels::extrait(ticket["label"].as_str().unwrap_or(""), 100)
                        ),
                        &format!("{url}/support/{}", encoder(&code)),
                    ),
                );
            }
        }
        "support.ticket.replied" => {
            let ticket = &data["ticket"];
            let code = texte(&ticket["code"]);
            enregistrer_demande(&r.base(), id, ticket);
            let commentaire = &data["comment"];
            if commentaire["from_customer"].as_bool() != Some(true) {
                let message = html_en_texte(commentaire["body"].as_str().unwrap_or(""));
                let agent = commentaire["author_name"].as_str().unwrap_or("");
                let sujet = ticket["label"].as_str().unwrap_or("");
                notifications::notifier(
                    r,
                    id,
                    Some(courriels::support_reponse(
                        &url, &a, &cp.nom, &code, sujet, agent, &message,
                    )),
                    Notification::autre(
                        notifications::SUPPORT,
                        None,
                        "Réponse du support",
                        &format!("{code} : {}", courriels::extrait(&message, 110)),
                        &format!("{url}/support/{}", encoder(&code)),
                    ),
                );
            }
        }
        "support.ticket.status_changed" => {
            let ticket = &data["ticket"];
            let code = texte(&ticket["code"]);
            let vers = data["to"]
                .as_str()
                .or_else(|| ticket["status"].as_str())
                .unwrap_or("");
            let deja = {
                let c = r.base();
                enregistrer_demande(&c, id, ticket);
                if !vers.is_empty() {
                    let _ = c.execute(
                        "UPDATE support_demandes SET statut = ?1 WHERE code = ?2 AND compte = ?3",
                        params![vers, code, id],
                    );
                }
                let deja = c
                    .query_row(
                        "SELECT resolue_prevenue FROM support_demandes WHERE code = ?1",
                        [&code],
                        |l| l.get::<_, i64>(0),
                    )
                    .optional()
                    .ok()
                    .flatten()
                    .unwrap_or(0)
                    == 1;
                // Rouverte puis résolue de nouveau : l'e-mail repart.
                if vers != "resolved" && vers != "closed" {
                    let _ = c.execute(
                        "UPDATE support_demandes SET resolue_prevenue = 0 WHERE code = ?1",
                        [&code],
                    );
                }
                deja
            };
            if vers == "resolved" && !deja {
                let _ = r.base().execute(
                    "UPDATE support_demandes SET resolue_prevenue = 1 WHERE code = ?1",
                    [&code],
                );
                let sujet = ticket["label"].as_str().unwrap_or("");
                notifications::notifier(
                    r,
                    id,
                    Some(courriels::support_resolue(&url, &a, &cp.nom, &code, sujet)),
                    Notification::autre(
                        notifications::SUPPORT,
                        None,
                        "Demande résolue",
                        &format!("{code} est résolue : donnez une note au support."),
                        &format!("{url}/support/{}?note=1#note", encoder(&code)),
                    ),
                );
            }
        }
        "support.discussion.message" => {
            let c = r.base();
            if let Some(uuid) = enregistrer_discussion(&c, id, &data["discussion"]) {
                enregistrer_message(&c, &uuid, &data["message"]);
            }
        }
        "support.discussion.converted" => {
            let ticket = &data["ticket"];
            let code = texte(&ticket["code"]);
            let (transcription, deja) = {
                let c = r.base();
                let uuid = enregistrer_discussion(&c, id, &data["discussion"]);
                let mut deja = false;
                if let Some(u) = &uuid {
                    deja = c
                        .query_row(
                            "SELECT prevenue FROM support_discussions WHERE uuid = ?1",
                            [u],
                            |l| l.get::<_, i64>(0),
                        )
                        .unwrap_or(0)
                        == 1;
                    let _ = c.execute("UPDATE support_discussions SET statut = 'convertie', ticket_code = ?1, prevenue = 1 WHERE uuid = ?2 AND compte = ?3", params![code, u, id]);
                }
                enregistrer_demande(&c, id, ticket);
                let _ = c.execute(
                    "UPDATE support_demandes SET recu_prevenu = 1 WHERE code = ?1",
                    [&code],
                );
                let msgs: Vec<(String, String)> = uuid
                    .map(|u| {
                        c.prepare("SELECT auteur_type, auteur_nom, corps FROM support_messages WHERE discussion = ?1 AND auteur_type != 'systeme' ORDER BY id")
                            .and_then(|mut q| q.query_map([&u], |l| Ok((l.get::<_, String>(0)?, l.get::<_, String>(1)?, l.get::<_, String>(2)?))).and_then(|i| i.collect::<rusqlite::Result<Vec<_>>>()))
                            .unwrap_or_default()
                    })
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(genre, nom, corps)| (if genre == "abonne" { "Vous".to_string() } else if nom.is_empty() { "Support E-KO".to_string() } else { format!("{nom} (support E-KO)") }, corps))
                    .collect();
                (msgs, deja)
            };
            if !deja {
                let sujet = ticket["label"].as_str().unwrap_or("");
                notifications::notifier(
                    r,
                    id,
                    Some(courriels::support_discussion_convertie(
                        &url,
                        &a,
                        &cp.nom,
                        &code,
                        sujet,
                        &transcription,
                    )),
                    Notification::autre(
                        notifications::SUPPORT,
                        None,
                        "Discussion transformée en demande",
                        &format!("Votre discussion est devenue la demande {code}."),
                        &format!("{url}/support/{}", encoder(&code)),
                    ),
                );
            }
        }
        _ => {
            eprintln!("webhook {evenement} ignoré");
            return false;
        }
    }
    true
}

/// Ménage : créations en cours oubliées, discussions terminées anciennes.
pub fn entretenir(c: &Connection, t: u64) {
    let _ = c.execute(
        "DELETE FROM support_creations WHERE debut < ?1",
        [t.saturating_sub(300) as i64],
    );
    let _ = c.execute(
        "DELETE FROM support_discussions WHERE statut IN ('close', 'convertie') AND maj_le < ?1",
        [t.saturating_sub(CONSERVATION_DISCUSSIONS) as i64],
    );
}

// ————— Pages —————

const STYLE_SUPPORT: &str = r#"<style>
.fil { display: grid; gap: .7rem; margin: 0 0 1rem; }
.msg { max-width: 90%; padding: .6rem .8rem; border-radius: 12px; background: var(--champ); box-shadow: 0 0 0 .5px var(--champ-bord); }
.msg.moi { justify-self: end; background: var(--accent-doux); }
.msg .qui { color: var(--doux); font-size: 12px; margin-bottom: .15rem; }
.msg .corps { white-space: pre-wrap; word-break: break-word; }
.msg .pj { margin-top: .35rem; font-size: 12px; color: var(--doux); }
.etat-ligne { text-align: center; color: var(--doux); font-size: 12px; }
textarea { display: block; width: 100%; margin-top: .3rem; min-height: 7rem; padding: .5rem .6rem; border: .5px solid var(--champ-bord); border-radius: 6px; background: var(--champ); color: var(--texte); font: inherit; resize: vertical; }
textarea:focus-visible { outline: 3.5px solid color-mix(in srgb, var(--accent) 45%, transparent); outline-offset: 0; border-color: var(--accent); }
.etoiles { display: flex; gap: .4rem; flex-wrap: wrap; margin: .5rem 0; }
.etoiles label { display: flex; align-items: center; gap: .3rem; margin: 0; padding: .3rem .7rem; border-radius: 6px; background: var(--champ); box-shadow: 0 0 0 .5px var(--champ-bord); font-weight: 500; }
.badge { display: inline-block; padding: .05rem .5rem; border-radius: 999px; background: var(--accent-doux); color: var(--accent); font-size: 12px; font-weight: 600; white-space: nowrap; }
.badge.fin { background: color-mix(in srgb, var(--ok) 16%, transparent); color: var(--ok); }
.badge.attente { background: color-mix(in srgb, #ff9f0a 18%, transparent); color: #b36b00; }
</style>"#;

fn code_valide(code: &str) -> bool {
    !code.is_empty()
        && code.len() <= 64
        && code
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-._".contains(c))
}

async fn bloquant<T: Send + 'static>(
    r: &Partage,
    f: impl FnOnce(&Partage) -> T + Send + 'static,
) -> Option<T> {
    let r = r.clone();
    tokio::task::spawn_blocking(move || f(&r)).await.ok()
}

fn badge(statut: &str) -> String {
    let classe = match statut {
        "resolved" | "closed" => " fin",
        "waiting_client" => " attente",
        _ => "",
    };
    format!(
        r#"<span class="badge{classe}">{}</span>"#,
        libelle_statut(statut)
    )
}

fn page_support(s: &Session, titre_message: &str, corps: &str) -> Response {
    page(
        "Support",
        Some(s),
        &format!("{STYLE_SUPPORT}<h1>Support</h1>{titre_message}{corps}"),
    )
    .into_response()
}

/// `GET /support` : mes demandes.
pub async fn liste(
    State(r): State<Partage>,
    h: HeaderMap,
    Query(q): Query<HashMap<String, String>>,
) -> Response {
    let s = match exiger(&r, &h, "/support") {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    let id = s.compte.id;
    let actif = Erp::depuis(&r.base()).is_some();
    let synchro = if actif {
        bloquant(&r, move |r| synchroniser_demandes(r, id))
            .await
            .unwrap_or(Err("Service momentanément indisponible.".into()))
    } else {
        Ok(())
    };
    let message = match q.get("ok").map(String::as_str) {
        Some("creee") => r#"<p class="ok">Demande envoyée : l'équipe du support vous répond dès que possible, par e-mail et ici.</p>"#.to_string(),
        _ => String::new(),
    };
    let avertissement = match &synchro {
        Err(e) if actif => format!(
            r#"<p class="meta">Liste peut-être incomplète : {}</p>"#,
            echapper(e)
        ),
        _ => String::new(),
    };
    if !actif {
        return page_support(
            &s,
            "",
            r#"<div class="carte"><p>Le support n'est pas encore ouvert sur cette plate-forme.</p></div>"#,
        );
    }
    let demandes = demandes_du_compte(&r.base(), id);
    let corps = if demandes.is_empty() {
        r#"<div class="carte"><p>Vous n'avez encore aucune demande. Une question, un problème, une idée ? Écrivez-nous : l'équipe d'E-KO vous répond.</p></div>"#.to_string()
    } else {
        let lignes: String = demandes
            .iter()
            .map(|d| {
                format!(
                    r#"<tr><td><a href="/support/{c}"><strong>{sujet}</strong></a><br><code>{code}</code></td><td>{cat}</td><td>{badge}{note}</td><td>{maj}</td></tr>"#,
                    c = encoder(&d.code),
                    code = echapper(&d.code),
                    sujet = echapper(if d.sujet.is_empty() { &d.code } else { &d.sujet }),
                    cat = echapper(&d.categorie),
                    badge = badge(&d.statut),
                    note = d.note.map(|n| format!(r#" <span class="meta">{n}/5</span>"#)).unwrap_or_default(),
                    maj = crate::temps::date_fr(d.maj_le),
                )
            })
            .collect();
        format!(
            r#"<div class="carte"><div class="tableau"><table><thead><tr><th>Demande</th><th>File</th><th>État</th><th>Mise à jour</th></tr></thead><tbody>{lignes}</tbody></table></div></div>"#
        )
    };
    page_support(
        &s,
        &format!("{message}{avertissement}"),
        &format!(
            r#"<p class="ligne"><a class="bouton" href="/support/nouvelle">Nouvelle demande</a><span class="meta">Pour une réponse tout de suite, utilisez la bulle de discussion en bas à droite.</span></p>{corps}"#
        ),
    )
}

fn formulaire_nouvelle(
    s: &Session,
    categories: &[(String, String)],
    erreur: &str,
    categorie: &str,
    sujet: &str,
    message: &str,
    cle: &str,
) -> Response {
    let options: String = categories
        .iter()
        .map(|(code, libelle)| {
            format!(
                r#"<option value="{c}"{sel}>{l}</option>"#,
                c = echapper(code),
                l = echapper(libelle),
                sel = if code == categorie { " selected" } else { "" }
            )
        })
        .collect();
    let erreur = if erreur.is_empty() {
        String::new()
    } else {
        format!(r#"<p class="erreur">{}</p>"#, echapper(erreur))
    };
    let corps = format!(
        r#"<div class="carte"><h2>Nouvelle demande</h2>{erreur}<form method="post" action="/support/nouvelle" enctype="multipart/form-data">{csrf}<input type="hidden" name="cle" value="{cle}">
<label>File<select name="categorie" required>{options}</select></label>
<label>Sujet<input type="text" name="sujet" maxlength="255" required value="{sujet}"></label>
<label>Votre message<textarea name="message" maxlength="{max}" required>{message}</textarea></label>
<label>Pièces jointes <span class="meta">(5 fichiers de 10 Mo au plus : copie d'écran, journal…)</span><input type="file" name="pieces" multiple></label>
<button type="submit">Envoyer la demande</button> <a class="bouton discret" href="/support">Annuler</a></form></div>"#,
        csrf = hidden_csrf(s),
        cle = echapper(cle),
        sujet = echapper(sujet),
        message = echapper(message),
        max = MAX_MESSAGE_DEMANDE,
    );
    page_support(s, "", &corps)
}

/// `GET /support/nouvelle`.
pub async fn nouvelle(State(r): State<Partage>, h: HeaderMap) -> Response {
    let s = match exiger(&r, &h, "/support/nouvelle") {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    match bloquant(&r, categories)
        .await
        .unwrap_or(Err("Service momentanément indisponible.".into()))
    {
        Ok(cats) => formulaire_nouvelle(&s, &cats, "", "", "", "", &crate::aleatoire_hex(16)),
        Err(e) => page_support(
            &s,
            "",
            &format!(
                r#"<div class="carte"><p class="erreur">{}</p><p><a href="/support">Retour</a></p></div>"#,
                echapper(&e)
            ),
        ),
    }
}

/// `POST /support/nouvelle` (multipart : csrf, cle, categorie, sujet, message, pieces).
pub async fn creer(State(r): State<Partage>, h: HeaderMap, mut mp: Multipart) -> Response {
    if !origine_ok(&r, &h) {
        return refuse_origine();
    }
    let s = match exiger(&r, &h, "/support/nouvelle") {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    let mut f: HashMap<String, String> = HashMap::new();
    let mut pieces: Vec<Piece> = Vec::new();
    let mut trop = false;
    loop {
        let champ_mp = match mp.next_field().await {
            Ok(Some(c)) => c,
            Ok(None) => break,
            Err(_) => {
                return portail::erreur_page(
                    &r,
                    Some(&s),
                    "Envoi interrompu ou trop volumineux (5 fichiers de 10 Mo au plus).",
                );
            }
        };
        let nom = champ_mp.name().unwrap_or("").to_string();
        if let Some(fichier) = champ_mp.file_name().map(str::to_string) {
            let genre = champ_mp
                .content_type()
                .unwrap_or("application/octet-stream")
                .to_string();
            let Ok(octets) = champ_mp.bytes().await else {
                return portail::erreur_page(
                    &r,
                    Some(&s),
                    "Pièce jointe illisible ou trop volumineuse (10 Mo au plus).",
                );
            };
            if fichier.is_empty() && octets.is_empty() {
                continue;
            }
            if octets.len() > MAX_PIECE || pieces.len() >= MAX_PIECES {
                trop = true;
                continue;
            }
            pieces.push((fichier, genre, octets.to_vec()));
        } else if let Ok(v) = champ_mp.text().await {
            f.insert(nom, v);
        }
    }
    if !csrf_ok(&s, &f) {
        return refuse_origine();
    }
    let (categorie, sujet, message) = (
        champ(&f, "categorie"),
        champ(&f, "sujet"),
        champ(&f, "message"),
    );
    let cle = Some(champ(&f, "cle"))
        .filter(|c| (8..=64).contains(&c.len()) && c.chars().all(|x| x.is_ascii_hexdigit()))
        .unwrap_or_else(|| crate::aleatoire_hex(16));
    let cats = bloquant(&r, categories)
        .await
        .unwrap_or(Err(String::new()))
        .unwrap_or_default();
    let erreur = if trop {
        "Trop de pièces jointes, ou l'une dépasse 10 Mo (5 fichiers de 10 Mo au plus).".to_string()
    } else if !cats.iter().any(|(c, _)| c.eq_ignore_ascii_case(&categorie)) {
        "Choisissez une file.".into()
    } else if sujet.is_empty() || sujet.chars().count() > 255 {
        "Le sujet est obligatoire (255 caractères au plus).".into()
    } else if message.is_empty() || message.chars().count() > MAX_MESSAGE_DEMANDE {
        format!("Le message est obligatoire ({MAX_MESSAGE_DEMANDE} caractères au plus).")
    } else if !r.limiter(
        &format!("support-demande:{}", s.compte.id),
        10,
        Duration::from_secs(3600),
    ) {
        "Trop de demandes en peu de temps : réessayez dans une heure.".into()
    } else {
        String::new()
    };
    if !erreur.is_empty() {
        return formulaire_nouvelle(&s, &cats, &erreur, &categorie, &sujet, &message, &cle);
    }
    let ident = Identite::depuis(&r.base(), &s.compte);
    let resultat = bloquant(&r, move |r| {
        creer_demande(r, &ident, &cle, &categorie, &sujet, &message, &pieces)
    })
    .await
    .unwrap_or(Err("Service momentanément indisponible.".into()));
    match resultat {
        Ok(d) => Redirect::to(&format!(
            "/support/{}?ok=creee",
            encoder(&texte(&d["code"]))
        ))
        .into_response(),
        Err(e) => {
            // Le formulaire est rendu de nouveau sans le texte tapé : il est dans la requête, perdue ; on garde au moins la file.
            formulaire_nouvelle(&s, &cats, &e, "", "", "", &crate::aleatoire_hex(16))
        }
    }
}

fn fil_html(d: &Value) -> String {
    let mut sortie = String::new();
    // Premier message : la description de la demande.
    let mut entrees: Vec<(bool, String, String, u64, Vec<String>)> = Vec::new();
    if let Some(desc) = d["description"].as_str().filter(|s| !s.is_empty()) {
        let pj: Vec<String> = d["attachments"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(|a| a["name"].as_str().map(str::to_string))
            .collect();
        entrees.push((
            true,
            "Vous".into(),
            html_en_texte(desc),
            date_json(&d["opened_at"]).unwrap_or(0) as u64,
            pj,
        ));
    }
    for m in d["thread"].as_array().cloned().unwrap_or_default() {
        let quand = date_json(&m["created_at"]).unwrap_or(0) as u64;
        if m["type"].as_str() == Some("status_change") {
            let vers = m["status_change"]["to"].as_str().unwrap_or("");
            entrees.push((
                false,
                String::new(),
                format!("Statut : {}", libelle_statut(vers)),
                quand,
                Vec::new(),
            ));
            continue;
        }
        let moi = m["from_customer"].as_bool() == Some(true);
        let qui = if moi {
            "Vous".to_string()
        } else {
            m["author_name"]
                .as_str()
                .filter(|n| !n.is_empty())
                .map_or("Support E-KO".to_string(), |n| {
                    format!("{n} · support E-KO")
                })
        };
        let pj: Vec<String> = m["attachments"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(|a| a["name"].as_str().map(str::to_string))
            .collect();
        entrees.push((
            moi,
            qui,
            html_en_texte(m["body"].as_str().unwrap_or("")),
            quand,
            pj,
        ));
    }
    for (moi, qui, corps, quand, pj) in entrees {
        if qui.is_empty() {
            sortie.push_str(&format!(
                r#"<div class="etat-ligne">{} · {}</div>"#,
                echapper(&corps),
                crate::temps::date_heure_fr(quand)
            ));
            continue;
        }
        let pieces = if pj.is_empty() {
            String::new()
        } else {
            format!(
                r#"<div class="pj">Pièces jointes : {}</div>"#,
                pj.iter()
                    .map(|n| echapper(n))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        sortie.push_str(&format!(r#"<div class="msg{}"><div class="qui">{} · {}</div><div class="corps">{}</div>{pieces}</div>"#, if moi { " moi" } else { "" }, echapper(&qui), crate::temps::date_heure_fr(quand), echapper(&corps)));
    }
    sortie
}

/// `GET /support/{code}` : le fil des messages, la réponse, la note.
pub async fn demande(
    State(r): State<Partage>,
    h: HeaderMap,
    Chemin(code): Chemin<String>,
    Query(q): Query<HashMap<String, String>>,
) -> Response {
    let ici = format!("/support/{}", encoder(&code));
    let s = match exiger(&r, &h, &ici) {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    if !code_valide(&code) {
        return page_support(
            &s,
            "",
            r#"<div class="carte"><p class="erreur">Demande introuvable.</p><p><a href="/support">Retour</a></p></div>"#,
        );
    }
    let id = s.compte.id;
    let c2 = code.clone();
    let detail = bloquant(&r, move |r| detail_demande(r, id, &c2))
        .await
        .unwrap_or(Err("Service momentanément indisponible.".into()));
    let d = match detail {
        Ok(d) => d,
        Err(e) => {
            return page_support(
                &s,
                "",
                &format!(
                    r#"<div class="carte"><p class="erreur">{}</p><p><a href="/support">Retour à mes demandes</a></p></div>"#,
                    echapper(&e)
                ),
            );
        }
    };
    let statut = d["status"].as_str().unwrap_or("open").to_string();
    let message = match q.get("ok").map(String::as_str) {
        Some("creee") => {
            r#"<p class="ok">Demande envoyée : l'équipe du support vous répond dès que possible, par e-mail et ici.</p>"#
        }
        Some("repondu") => r#"<p class="ok">Réponse envoyée.</p>"#,
        Some("note") => r#"<p class="ok">Merci pour votre note.</p>"#,
        _ => "",
    };
    let erreur = q
        .get("erreur")
        .map(|e| format!(r#"<p class="erreur">{}</p>"#, echapper(e)))
        .unwrap_or_default();
    let note_actuelle = note_json(&d);
    let formulaire_reponse = if statut == "closed" {
        r#"<div class="carte"><p class="meta">Cette demande est fermée. Pour un nouveau sujet, <a href="/support/nouvelle">ouvrez une demande</a>.</p></div>"#.to_string()
    } else {
        format!(
            r#"<div class="carte"><h2>Répondre</h2><form method="post" action="{ici}/repondre">{}<label><span class="meta">{}</span><textarea name="message" maxlength="{MAX_MESSAGE_DEMANDE}" required></textarea></label><button type="submit">Envoyer</button></form></div>"#,
            hidden_csrf(&s),
            if statut == "resolved" {
                "Le problème persiste ? Répondez : la demande sera rouverte."
            } else {
                "Votre message"
            }
        )
    };
    let formulaire_note = if statut == "resolved" || statut == "closed" {
        let radios: String = (1..=5)
            .map(|n| {
                format!(
                    r#"<label><input type="radio" name="note" value="{n}"{} required> {n}</label>"#,
                    if note_actuelle == Some(n) {
                        " checked"
                    } else {
                        ""
                    }
                )
            })
            .collect();
        format!(
            r#"<div class="carte" id="note"><h2>{}</h2><form method="post" action="{ici}/note">{}<div class="etoiles" role="radiogroup" aria-label="Note de 1 à 5">{radios}</div><p class="meta">1 : pas du tout satisfait, 5 : très satisfait.</p><label>Commentaire (facultatif)<textarea name="commentaire" maxlength="2000" style="min-height:4rem">{}</textarea></label><button type="submit">{}</button></form></div>"#,
            if note_actuelle.is_some() {
                "Votre note"
            } else {
                "Notez le support"
            },
            hidden_csrf(&s),
            echapper(d["satisfaction_comment"].as_str().unwrap_or("")),
            if note_actuelle.is_some() {
                "Modifier ma note"
            } else {
                "Envoyer ma note"
            },
        )
    } else {
        String::new()
    };
    let echeance = date_json(&d["first_response_due_at"])
        .filter(|_| d["first_response_at"].is_null() && statut == "open")
        .map(|t| {
            format!(
                r#"<p class="meta">Première réponse attendue avant le {}.</p>"#,
                crate::temps::date_heure_fr(t as u64)
            )
        })
        .unwrap_or_default();
    let corps = format!(
        r#"<p><a href="/support">← Mes demandes</a></p>{message}{erreur}<div class="carte"><h2>{sujet}</h2><p>{badge} <span class="meta">{code} · {cat} · ouverte le {ouverte}</span></p>{echeance}</div><div class="fil">{fil}</div>{formulaire_reponse}{formulaire_note}"#,
        sujet = echapper(d["label"].as_str().unwrap_or(&code)),
        badge = badge(&statut),
        code = echapper(&code),
        cat = echapper(d["category"]["label"].as_str().unwrap_or("")),
        ouverte = crate::temps::date_fr(date_json(&d["opened_at"]).unwrap_or(0) as u64),
        fil = fil_html(&d),
    );
    page_support(&s, "", &corps)
}

/// `POST /support/{code}/repondre`.
pub async fn repondre(
    State(r): State<Partage>,
    h: HeaderMap,
    Chemin(code): Chemin<String>,
    axum::Form(f): Champs,
) -> Response {
    if !origine_ok(&r, &h) {
        return refuse_origine();
    }
    let ici = format!("/support/{}", encoder(&code));
    let s = match exiger(&r, &h, &ici) {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    if !csrf_ok(&s, &f) || !code_valide(&code) {
        return refuse_origine();
    }
    let message = champ(&f, "message");
    let erreur = if message.is_empty() || message.chars().count() > MAX_MESSAGE_DEMANDE {
        Some("Écrivez un message (20 000 caractères au plus).".to_string())
    } else if !r.limiter(
        &format!("support-reponse:{}", s.compte.id),
        20,
        Duration::from_secs(600),
    ) {
        Some("Trop de messages en peu de temps : réessayez dans quelques minutes.".to_string())
    } else {
        None
    };
    if let Some(e) = erreur {
        return Redirect::to(&format!("{ici}?erreur={}", encoder(&e))).into_response();
    }
    let id = s.compte.id;
    let c2 = code.clone();
    let resultat = bloquant(&r, move |r| {
        // La demande doit être au compte : l'index local, sinon l'ERP (404 si elle est à un autre).
        if !demande_connue(&r.base(), id, &c2) {
            detail_demande(r, id, &c2)?;
        }
        repondre_demande(r, id, &c2, &message)
    })
    .await
    .unwrap_or(Err("Service momentanément indisponible.".into()));
    match resultat {
        Ok(()) => Redirect::to(&format!("{ici}?ok=repondu")).into_response(),
        Err(e) => Redirect::to(&format!("{ici}?erreur={}", encoder(&e))).into_response(),
    }
}

/// `POST /support/{code}/note`.
pub async fn noter(
    State(r): State<Partage>,
    h: HeaderMap,
    Chemin(code): Chemin<String>,
    axum::Form(f): Champs,
) -> Response {
    if !origine_ok(&r, &h) {
        return refuse_origine();
    }
    let ici = format!("/support/{}", encoder(&code));
    let s = match exiger(&r, &h, &ici) {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    if !csrf_ok(&s, &f) || !code_valide(&code) {
        return refuse_origine();
    }
    let Some(note) = champ(&f, "note")
        .parse::<i64>()
        .ok()
        .filter(|n| (1..=5).contains(n))
    else {
        return Redirect::to(&format!(
            "{ici}?erreur={}#note",
            encoder("Choisissez une note de 1 à 5.")
        ))
        .into_response();
    };
    let commentaire = champ(&f, "commentaire");
    if commentaire.chars().count() > 2000 {
        return Redirect::to(&format!(
            "{ici}?erreur={}#note",
            encoder("Le commentaire fait 2 000 caractères au plus.")
        ))
        .into_response();
    }
    let id = s.compte.id;
    let c2 = code.clone();
    let resultat = bloquant(&r, move |r| {
        if !demande_connue(&r.base(), id, &c2) {
            detail_demande(r, id, &c2)?;
        }
        noter_demande(r, id, &c2, note, &commentaire)
    })
    .await
    .unwrap_or(Err("Service momentanément indisponible.".into()));
    match resultat {
        Ok(()) => Redirect::to(&format!("{ici}?ok=note")).into_response(),
        Err(e) => Redirect::to(&format!("{ici}?erreur={}#note", encoder(&e))).into_response(),
    }
}

// ————— Bulle de discussion en direct —————

fn json_erreur(statut: StatusCode, code: &str, message: &str) -> Response {
    (
        statut,
        [(header::CACHE_CONTROL, "no-store")],
        Json(json!({ "code": code, "message": message })),
    )
        .into_response()
}

fn sans_cache(v: Value) -> Response {
    ([(header::CACHE_CONTROL, "no-store")], Json(v)).into_response()
}

/// Session, origine et jeton anti-falsification d'un appel du script.
fn appel_bulle(r: &Partage, h: &HeaderMap, ecriture: bool) -> Result<Session, Box<Response>> {
    if ecriture && !origine_ok(r, h) {
        return Err(Box::new(refuse_origine()));
    }
    let Some(s) = session(r, h) else {
        return Err(Box::new(reponse_json(
            StatusCode::UNAUTHORIZED,
            "session_requise",
        )));
    };
    if ecriture && !csrf_entete_ok(&s, h) {
        return Err(Box::new(refuse_origine()));
    }
    Ok(s)
}

/// `GET /support/direct/etat?uuid=&apres=&files=1` : l'interrogation de la bulle (base locale).
pub async fn etat(
    State(r): State<Partage>,
    h: HeaderMap,
    Query(q): Query<HashMap<String, String>>,
) -> Response {
    let s = match appel_bulle(&r, &h, false) {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    if !r.limiter(
        &format!("support-etat:{}", s.compte.id),
        150,
        Duration::from_secs(60),
    ) {
        return json_erreur(
            StatusCode::TOO_MANY_REQUESTS,
            "trop_de_requetes",
            "Trop de requêtes.",
        );
    }
    if Erp::depuis(&r.base()).is_none() {
        return sans_cache(json!({ "actif": false }));
    }
    let apres = q
        .get("apres")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(0);
    let uuid = q.get("uuid").cloned().unwrap_or_default();
    let mut v = etat_json(&r.base(), s.compte.id, &uuid, apres);
    if (q.get("files").is_some_and(|f| f == "1") && v["discussion"].is_null())
        || v["discussion"]["fin"] == true
    {
        let cats = bloquant(&r, categories)
            .await
            .and_then(Result::ok)
            .unwrap_or_default();
        v["categories"] = json!(
            cats.iter()
                .map(|(c, l)| json!({ "code": c, "libelle": l }))
                .collect::<Vec<_>>()
        );
        v["ouvert"] = json!(ouvert_maintenant(&r.base()));
    }
    sans_cache(v)
}

/// `POST /support/direct/ouvrir` : `{categorie, message}`.
pub async fn ouvrir_discussion(
    State(r): State<Partage>,
    h: HeaderMap,
    Json(c): Json<Value>,
) -> Response {
    let s = match appel_bulle(&r, &h, true) {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    let message = c["message"].as_str().unwrap_or("").trim().to_string();
    let categorie = c["categorie"].as_str().unwrap_or("").trim().to_string();
    if message.is_empty() || message.chars().count() > MAX_MESSAGE_DISCUSSION {
        return json_erreur(
            StatusCode::BAD_REQUEST,
            "message_invalide",
            "Écrivez un message (5 000 caractères au plus).",
        );
    }
    if discussion_active(&r.base(), s.compte.id).is_some() {
        return json_erreur(
            StatusCode::CONFLICT,
            "deja_ouverte",
            "Une discussion est déjà en cours.",
        );
    }
    if !r.limiter(
        &format!("support-discussion:{}", s.compte.id),
        5,
        Duration::from_secs(3600),
    ) {
        return json_erreur(
            StatusCode::TOO_MANY_REQUESTS,
            "trop_de_discussions",
            "Trop de discussions en peu de temps : réessayez plus tard.",
        );
    }
    let ident = Identite::depuis(&r.base(), &s.compte);
    let compte = s.compte.id;
    let resultat = bloquant(&r, move |r| -> Result<(), String> {
        if !categorie.is_empty() && !categories(r).is_ok_and(|l| l.iter().any(|(c, _)| c.eq_ignore_ascii_case(&categorie))) {
            return Err("File inconnue.".into());
        }
        let erp = erp_pret(r)?;
        let mut corps = ident.json();
        corps.insert("message".into(), json!(message));
        corps.insert("locale".into(), json!("fr"));
        if !categorie.is_empty() {
            corps.insert("category".into(), json!(categorie));
        }
        let rep = erp.post("/support/discussions", &Value::Object(corps), None)?;
        if !rep.ok() {
            return Err(refus(&rep));
        }
        let d = rep.donnees();
        let c = r.base();
        retenir_ouverture(&c, d);
        if let Some(uuid) = enregistrer_discussion(&c, compte, d) {
            for m in d["messages"].as_array().cloned().unwrap_or_default() {
                enregistrer_message(&c, &uuid, &m);
            }
            if let Some(code) = d["ticket_code"].as_str().filter(|_| d["status"] == "convertie") {
                let _ = c.execute("UPDATE support_discussions SET statut = 'convertie', ticket_code = ?1 WHERE uuid = ?2", params![code, uuid]);
            }
        }
        Ok(())
    })
    .await
    .unwrap_or(Err("Service momentanément indisponible.".into()));
    match resultat {
        Ok(()) => sans_cache(etat_json(&r.base(), compte, "", 0)),
        Err(e) => json_erreur(StatusCode::BAD_GATEWAY, "support_indisponible", &e),
    }
}

/// `POST /support/direct/envoyer` : `{message}` dans la discussion en cours.
pub async fn envoyer_message(
    State(r): State<Partage>,
    h: HeaderMap,
    Json(c): Json<Value>,
) -> Response {
    let s = match appel_bulle(&r, &h, true) {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    let message = c["message"].as_str().unwrap_or("").trim().to_string();
    if message.is_empty() || message.chars().count() > MAX_MESSAGE_DISCUSSION {
        return json_erreur(
            StatusCode::BAD_REQUEST,
            "message_invalide",
            "Écrivez un message (5 000 caractères au plus).",
        );
    }
    let Some(d) = discussion_active(&r.base(), s.compte.id) else {
        return json_erreur(
            StatusCode::CONFLICT,
            "aucune_discussion",
            "Aucune discussion en cours.",
        );
    };
    if !r.limiter(
        &format!("support-message:{}", s.compte.id),
        30,
        Duration::from_secs(60),
    ) {
        return json_erreur(
            StatusCode::TOO_MANY_REQUESTS,
            "trop_de_messages",
            "Trop de messages en peu de temps.",
        );
    }
    let compte = s.compte.id;
    let uuid = d.uuid.clone();
    let resultat = bloquant(&r, move |r| -> Result<(), String> {
        let erp = erp_pret(r)?;
        let rep = erp.post(&format!("/support/discussions/{}/messages", encoder(&uuid)), &json!({ "message": message }), None)?;
        if rep.statut == 409 {
            let _ = r.base().execute("UPDATE support_discussions SET statut = 'close', maj_le = ?1 WHERE uuid = ?2 AND compte = ?3 AND statut NOT IN ('close', 'convertie')", params![crate::maintenant() as i64, uuid, compte]);
        }
        if !rep.ok() {
            return Err(refus(&rep));
        }
        enregistrer_message(&r.base(), &uuid, rep.donnees());
        Ok(())
    })
    .await
    .unwrap_or(Err("Service momentanément indisponible.".into()));
    match resultat {
        Ok(()) => sans_cache(etat_json(
            &r.base(),
            compte,
            &d.uuid,
            c["apres"].as_i64().unwrap_or(0),
        )),
        Err(e) => json_erreur(StatusCode::BAD_GATEWAY, "support_indisponible", &e),
    }
}

/// `POST /support/direct/fermer` : le client met fin à la discussion.
pub async fn fermer_discussion(State(r): State<Partage>, h: HeaderMap) -> Response {
    let s = match appel_bulle(&r, &h, true) {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    let Some(d) = discussion_active(&r.base(), s.compte.id) else {
        return sans_cache(etat_json(&r.base(), s.compte.id, "", 0));
    };
    let compte = s.compte.id;
    let uuid = d.uuid.clone();
    let resultat = bloquant(&r, move |r| -> Result<(), String> {
        let erp = erp_pret(r)?;
        let rep = erp.post(&format!("/support/discussions/{}/close", encoder(&uuid)), &json!({}), None)?;
        if !rep.ok() {
            return Err(refus(&rep));
        }
        let statut = rep.donnees()["status"].as_str().unwrap_or("close").to_string();
        let _ = r.base().execute("UPDATE support_discussions SET statut = ?1, maj_le = ?2 WHERE uuid = ?3 AND compte = ?4", params![statut, crate::maintenant() as i64, uuid, compte]);
        Ok(())
    })
    .await
    .unwrap_or(Err("Service momentanément indisponible.".into()));
    match resultat {
        Ok(()) => sans_cache(etat_json(&r.base(), compte, &d.uuid, i64::MAX)),
        Err(e) => json_erreur(StatusCode::BAD_GATEWAY, "support_indisponible", &e),
    }
}

/// `GET /support/bulle.js` : la bulle de discussion, chargée par chaque page connectée.
pub async fn bulle_js() -> Response {
    let mut rep = (StatusCode::OK, SCRIPT_BULLE).into_response();
    rep.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/javascript; charset=utf-8"),
    );
    rep.headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    rep
}

/// Balise des pages connectées : la bulle (le jeton anti-falsification passe en attribut).
pub fn balise_bulle(csrf: &str) -> String {
    format!(
        r#"<script src="/support/bulle.js" data-csrf="{}" defer></script>"#,
        echapper(csrf)
    )
}

const SCRIPT_BULLE: &str = r##"(function () {
  'use strict';
  var script = document.currentScript;
  var csrf = (script && script.getAttribute('data-csrf')) || '';
  var etat = { uuid: '', apres: 0, ouvert: false, actif: false, discussion: null, categories: [], enCours: false };
  var temps = null;
  function el(tag, attrs, texte) {
    var e = document.createElement(tag);
    for (var k in (attrs || {})) { if (k === 'class') e.className = attrs[k]; else e.setAttribute(k, attrs[k]); }
    if (texte) e.textContent = texte;
    return e;
  }
  function memo(cle, valeur) {
    try { if (valeur === undefined) return window.localStorage.getItem(cle); window.localStorage.setItem(cle, valeur); } catch (e) { return null; }
  }
  var css = '#eko-bulle{position:fixed;right:18px;bottom:18px;z-index:50;width:54px;height:54px;border-radius:50%;border:0;background:var(--accent,#007aff);color:#fff;box-shadow:0 4px 14px rgba(0,0,0,.28);cursor:pointer;display:flex;align-items:center;justify-content:center}' +
    '#eko-bulle svg{width:26px;height:26px}#eko-bulle .pastille{position:absolute;top:-2px;right:-2px;min-width:18px;height:18px;padding:0 5px;border-radius:9px;background:#ff3b30;color:#fff;font:600 11px/18px system-ui,sans-serif;text-align:center}' +
    '#eko-chat{position:fixed;right:18px;bottom:84px;z-index:50;width:min(360px,calc(100vw - 24px));height:min(520px,calc(100vh - 110px));display:none;flex-direction:column;background:var(--groupe,#fff);color:var(--texte,#1d1d1f);border-radius:14px;box-shadow:0 10px 36px rgba(0,0,0,.3),0 0 0 .5px rgba(0,0,0,.15);overflow:hidden;font:14px/1.4 -apple-system,BlinkMacSystemFont,"Segoe UI",system-ui,sans-serif}' +
    '#eko-chat.ouvert{display:flex}#eko-chat header{display:flex;align-items:center;gap:.5rem;padding:.7rem .9rem;background:var(--accent,#007aff);color:#fff}' +
    '#eko-chat header strong{flex:1;font-size:15px}#eko-chat header .etat{font-size:12px;display:flex;align-items:center;gap:.3rem}#eko-chat header .etat i{width:8px;height:8px;border-radius:50%;background:#9a9a9a;display:inline-block}' +
    '#eko-chat header .etat.on i{background:#34c759}#eko-chat header .etat.off i{background:#ff9f0a}#eko-chat header button{background:transparent;border:0;color:#fff;font-size:20px;line-height:1;cursor:pointer;padding:0 .2rem}' +
    '#eko-chat .corps{flex:1;overflow-y:auto;padding:.8rem;display:flex;flex-direction:column;gap:.5rem}#eko-chat .m{max-width:85%;padding:.45rem .7rem;border-radius:12px;background:var(--champ,#f1f1f3);box-shadow:0 0 0 .5px var(--champ-bord,rgba(0,0,0,.12));white-space:pre-wrap;word-break:break-word;align-self:flex-start}' +
    '#eko-chat .m.moi{align-self:flex-end;background:var(--accent-doux,rgba(0,122,255,.14))}#eko-chat .m .q{display:block;font-size:11px;color:var(--doux,#86868b);margin-bottom:.1rem}#eko-chat .sys{align-self:center;color:var(--doux,#86868b);font-size:12px;text-align:center}' +
    '#eko-chat form{padding:.6rem .7rem;border-top:.5px solid var(--trait,rgba(0,0,0,.1));display:flex;flex-direction:column;gap:.45rem}#eko-chat textarea,#eko-chat select{width:100%;margin:0;min-height:0;font:inherit;padding:.4rem .55rem;border:.5px solid var(--champ-bord,rgba(0,0,0,.16));border-radius:8px;background:var(--champ,#fff);color:inherit;resize:none}' +
    '#eko-chat .ligne2{display:flex;gap:.5rem;align-items:center;justify-content:space-between}#eko-chat .ligne2 button{margin:0}#eko-chat .discret{background:transparent;color:var(--doux,#86868b);box-shadow:none;padding:0;min-height:0;font-size:12px;text-decoration:underline}' +
    '#eko-chat .err{color:var(--erreur,#ff3b30);font-size:12px;margin:0}#eko-chat .intro{margin:0;color:var(--doux,#86868b);font-size:13px}@media (max-width:480px){#eko-chat{right:8px;bottom:78px;width:calc(100vw - 16px)}}';
  function monter() {
    var st = el('style'); st.textContent = css; document.head.appendChild(st);
    var b = el('button', { id: 'eko-bulle', type: 'button', 'aria-label': 'Discussion en direct avec le support', 'aria-expanded': 'false' });
    b.innerHTML = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M21 12a8 8 0 0 1-11.6 7.1L4 20l1-4.6A8 8 0 1 1 21 12z"/></svg>';
    var pastille = el('span', { class: 'pastille' }); pastille.style.display = 'none'; b.appendChild(pastille);
    var fen = el('section', { id: 'eko-chat', role: 'dialog', 'aria-label': 'Discussion en direct' });
    var tete = el('header');
    tete.appendChild(el('strong', null, 'Discussion en direct'));
    var ouvertEl = el('span', { class: 'etat' }); ouvertEl.appendChild(el('i')); var ouvertTxt = el('span'); ouvertEl.appendChild(ouvertTxt); tete.appendChild(ouvertEl);
    var fermerBtn = el('button', { type: 'button', 'aria-label': 'Réduire' }, '×'); tete.appendChild(fermerBtn);
    var corps = el('div', { class: 'corps', 'aria-live': 'polite' });
    var formulaire = el('form');
    fen.appendChild(tete); fen.appendChild(corps); fen.appendChild(formulaire);
    document.body.appendChild(fen); document.body.appendChild(b);
    var nonLus = 0;

    function repondre(r) { return r.json().then(function (j) { return { ok: r.ok, statut: r.status, j: j }; }); }
    function appeler(methode, chemin, corpsJson) {
      var o = { method: methode, credentials: 'same-origin', headers: { 'X-CSRF-Token': csrf } };
      if (corpsJson !== undefined) { o.headers['Content-Type'] = 'application/json'; o.body = JSON.stringify(corpsJson); }
      return fetch(chemin, o).then(repondre);
    }
    function defiler() { corps.scrollTop = corps.scrollHeight; }
    function bulle(m) {
      if (m.type === 'systeme') { return el('div', { class: 'sys' }, m.texte); }
      var d = el('div', { class: 'm' + (m.type === 'abonne' ? ' moi' : '') });
      d.appendChild(el('span', { class: 'q' }, m.type === 'abonne' ? 'Vous' : (m.auteur || 'Support E-KO')));
      d.appendChild(document.createTextNode(m.texte));
      return d;
    }
    function indicateur(e) {
      ouvertEl.className = 'etat' + (e.ouvert === true ? ' on' : e.ouvert === false ? ' off' : '');
      ouvertTxt.textContent = e.ouvert === true ? 'Ouvert' : e.ouvert === false ? 'Fermé' : '';
    }
    function erreurForm(texte) {
      var p = formulaire.querySelector('.err'); if (!p) { p = el('p', { class: 'err', role: 'alert' }); formulaire.insertBefore(p, formulaire.firstChild); }
      p.textContent = texte || '';
    }
    function dessinerFormulaire() {
      var d = etat.discussion;
      var pourquoi = d ? (d.fin ? 'fin' : 'direct') : 'debut';
      if (formulaire.getAttribute('data-vue') === pourquoi && pourquoi !== 'debut') return;
      if (formulaire.getAttribute('data-vue') === pourquoi && pourquoi === 'debut' && formulaire.querySelector('select')) {
        var s = formulaire.querySelector('select');
        if (s.options.length !== etat.categories.length + 0) { /* files changées : on redessine */ } else return;
      }
      formulaire.setAttribute('data-vue', pourquoi);
      formulaire.textContent = '';
      if (pourquoi === 'debut') {
        formulaire.appendChild(el('p', { class: 'intro' }, 'Posez votre question à l’équipe d’E-KO. Hors des heures d’ouverture, ou si personne n’est disponible, votre message devient une demande suivie par e-mail.'));
        if (etat.categories.length) {
          var sel = el('select', { 'aria-label': 'Sujet' });
          etat.categories.forEach(function (c) { var o = el('option', { value: c.code }, c.libelle); sel.appendChild(o); });
          formulaire.appendChild(sel);
        }
        var ta = el('textarea', { rows: '3', maxlength: '5000', placeholder: 'Votre message', 'aria-label': 'Votre message' });
        formulaire.appendChild(ta);
        var l = el('div', { class: 'ligne2' }); var go = el('button', { type: 'submit' }, 'Démarrer la discussion'); l.appendChild(go); formulaire.appendChild(l);
      } else if (pourquoi === 'direct') {
        var ta2 = el('textarea', { rows: '2', maxlength: '5000', placeholder: 'Votre message', 'aria-label': 'Votre message' });
        ta2.addEventListener('keydown', function (ev) { if (ev.key === 'Enter' && !ev.shiftKey) { ev.preventDefault(); formulaire.requestSubmit(); } });
        formulaire.appendChild(ta2);
        var l2 = el('div', { class: 'ligne2' });
        var fin = el('button', { type: 'button', class: 'discret' }, 'Terminer la discussion');
        fin.addEventListener('click', function () { appeler('POST', '/support/direct/fermer').then(function (r) { appliquer(r.j); }); });
        l2.appendChild(fin); l2.appendChild(el('button', { type: 'submit' }, 'Envoyer')); formulaire.appendChild(l2);
      } else {
        var nv = el('div', { class: 'ligne2' }); var nb = el('button', { type: 'button' }, 'Nouvelle discussion');
        nb.addEventListener('click', function () { etat.discussion = null; etat.uuid = ''; etat.apres = 0; corps.textContent = ''; formulaire.removeAttribute('data-vue'); charger(true); });
        nv.appendChild(nb); formulaire.appendChild(nv);
      }
    }
    function appliquer(j) {
      if (!j || j.actif === false) { b.style.display = 'none'; fen.style.display = 'none'; arreter(); return; }
      etat.actif = true;
      if (j.categories) etat.categories = j.categories;
      if (j.ouvert !== undefined) etat.ouvert_connu = j.ouvert;
      indicateur({ ouvert: j.ouvert });
      var d = j.discussion;
      if (d && d.uuid !== etat.uuid) { corps.textContent = ''; etat.apres = 0; etat.uuid = d.uuid; }
      if (!d && etat.uuid) { corps.textContent = ''; etat.uuid = ''; etat.apres = 0; }
      etat.discussion = d;
      var nouveaux = 0;
      (j.messages || []).forEach(function (m) {
        if (m.id <= etat.apres) return;
        corps.appendChild(bulle(m)); etat.apres = m.id;
        if (m.type !== 'abonne' && m.type !== 'systeme') nouveaux++;
      });
      if (!d) {
        corps.textContent = '';
        corps.appendChild(el('div', { class: 'sys' }, 'Aucune discussion en cours.'));
      } else if (d.fin && !corps.querySelector('.fin-msg')) {
        corps.appendChild(el('div', { class: 'sys fin-msg' }, d.ticket ? 'Cette discussion est devenue la demande ' + d.ticket + '. Vous recevez la suite par e-mail et dans la page Support.' : 'Discussion terminée.'));
      } else if (!d.fin && d.statut === 'en_attente' && etat.apres > 0 && !corps.querySelector('.attente-msg')) {
        corps.appendChild(el('div', { class: 'sys attente-msg' }, 'Un conseiller va vous répondre…'));
      } else if (d.statut === 'en_cours') {
        var a = corps.querySelector('.attente-msg'); if (a) a.remove();
      }
      dessinerFormulaire();
      if (nouveaux) { if (etat.ouvert) defiler(); else { nonLus += nouveaux; pastille.textContent = String(nonLus); pastille.style.display = ''; } }
      if (etat.ouvert && (j.messages || []).length) defiler();
    }
    function charger(avecFiles) {
      if (etat.enCours) return; etat.enCours = true;
      var q = '/support/direct/etat?uuid=' + encodeURIComponent(etat.uuid) + '&apres=' + etat.apres + (avecFiles ? '&files=1' : '');
      fetch(q, { credentials: 'same-origin' }).then(function (r) { return r.ok ? r.json() : null; }).then(function (j) { etat.enCours = false; if (j) appliquer(j); }).catch(function () { etat.enCours = false; });
    }
    function cadence() {
      arreter();
      if (document.hidden) return;
      var actif = etat.discussion && !etat.discussion.fin;
      var delai = etat.ouvert ? 3000 : (actif ? 20000 : 0);
      if (delai) temps = setInterval(function () { charger(false); }, delai);
    }
    function arreter() { if (temps) { clearInterval(temps); temps = null; } }
    function basculer(ouvrir) {
      etat.ouvert = ouvrir; memo('eko_bulle', ouvrir ? '1' : '0');
      fen.className = ouvrir ? 'ouvert' : ''; b.setAttribute('aria-expanded', ouvrir ? 'true' : 'false');
      if (ouvrir) { nonLus = 0; pastille.style.display = 'none'; charger(true); defiler(); var t = formulaire.querySelector('textarea'); if (t) t.focus(); }
      cadence();
    }
    b.addEventListener('click', function () { basculer(!etat.ouvert); });
    fermerBtn.addEventListener('click', function () { basculer(false); });
    document.addEventListener('keydown', function (ev) { if (ev.key === 'Escape' && etat.ouvert) basculer(false); });
    document.addEventListener('visibilitychange', cadence);
    formulaire.addEventListener('submit', function (ev) {
      ev.preventDefault();
      var ta = formulaire.querySelector('textarea'); var texte = ta ? ta.value.trim() : '';
      if (!texte) return;
      var bouton = formulaire.querySelector('button[type=submit]'); if (bouton) bouton.disabled = true;
      erreurForm('');
      var d = etat.discussion;
      var envoi = d && !d.fin ? appeler('POST', '/support/direct/envoyer', { message: texte, apres: etat.apres }) : appeler('POST', '/support/direct/ouvrir', { message: texte, categorie: (formulaire.querySelector('select') || {}).value || '' });
      envoi.then(function (r) {
        if (bouton) bouton.disabled = false;
        if (!r.ok) { erreurForm((r.j && r.j.message) || 'Envoi impossible.'); return; }
        if (ta) ta.value = '';
        appliquer(r.j); defiler();
      }).catch(function () { if (bouton) bouton.disabled = false; erreurForm('Connexion impossible.'); });
    });
    // Premier état : la bulle n'apparaît que si le support est ouvert sur cette plate-forme.
    b.style.display = 'none';
    fetch('/support/direct/etat?files=1', { credentials: 'same-origin' }).then(function (r) { return r.ok ? r.json() : null; }).then(function (j) {
      if (!j || j.actif === false) return;
      b.style.display = '';
      appliquer(j);
      if (memo('eko_bulle') === '1') basculer(true); else cadence();
    }).catch(function () {});
  }
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', monter); else monter();
})();
"##;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_en_texte_nettoie() {
        assert_eq!(
            html_en_texte(
                "<p>Bonjour &amp; merci</p><p>Voici <b>le</b> journal.<br>Ligne 2 &lt;ok&gt;</p>"
            ),
            "Bonjour & merci\n\nVoici le journal.\nLigne 2 <ok>"
        );
        assert_eq!(html_en_texte("<script>x</script>\n\n\n<p>a</p>"), "x\n\na");
        assert_eq!(html_en_texte(""), "");
    }

    #[test]
    fn references_et_identite() {
        assert_eq!(reference(42), "cloud-42");
        let d = std::env::temp_dir().join(format!("eko-cloud-support-{}", crate::aleatoire_hex(6)));
        let c = base::ouvrir(&d).unwrap();
        let id = base::creer_compte(&c, "lea@atelier.fr", "Léa Martin", None)
            .unwrap()
            .unwrap();
        assert_eq!(compte_de_reference(&c, &reference(id)), Some(id));
        assert_eq!(compte_de_reference(&c, "cloud-9999"), None);
        assert_eq!(compte_de_reference(&c, "acc_1"), None);
        let compte = base::compte_par_id(&c, id).unwrap().unwrap();
        let i = Identite::depuis(&c, &compte);
        assert_eq!(
            (i.prenom.as_str(), i.nom.as_str(), i.tier_code.as_deref()),
            ("Léa", "Martin", None)
        );
        let j = i.json();
        assert_eq!(j["tier_email"], "lea@atelier.fr");
        assert!(!j.contains_key("tier_code"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn index_des_demandes_et_des_messages() {
        let d = std::env::temp_dir().join(format!("eko-cloud-support-{}", crate::aleatoire_hex(6)));
        let c = base::ouvrir(&d).unwrap();
        let id = base::creer_compte(&c, "a@b.fr", "A", None)
            .unwrap()
            .unwrap();
        let autre = base::creer_compte(&c, "c@d.fr", "C", None)
            .unwrap()
            .unwrap();
        enregistrer_demande(
            &c,
            id,
            &json!({ "code": "TKT-1", "label": "Sujet", "status": "open", "category": { "code": "TECH", "label": "Technique" }, "opened_at": "2026-11-04T09:00:00+00:00" }),
        );
        enregistrer_demande(
            &c,
            id,
            &json!({ "code": "TKT-1", "label": "Sujet", "status": "resolved", "satisfaction": { "rating": 4 }, "resolved_at": "2026-11-05T09:00:00+00:00" }),
        );
        // Une demande n'est jamais reprise par un autre compte.
        enregistrer_demande(
            &c,
            autre,
            &json!({ "code": "TKT-1", "label": "Pris", "status": "closed" }),
        );
        let l = demandes_du_compte(&c, id);
        assert_eq!(
            (
                l.len(),
                l[0].statut.as_str(),
                l[0].note,
                l[0].categorie.as_str(),
                l[0].sujet.as_str()
            ),
            (1, "resolved", Some(4), "Technique", "Sujet")
        );
        assert!(demandes_du_compte(&c, autre).is_empty());

        let disc = json!({ "uuid": "u-1", "status": "en_cours", "agent_name": "Camille", "account_reference": reference(id), "created_at": "2026-11-04T09:00:00+00:00" });
        let uuid = enregistrer_discussion(&c, id, &disc).unwrap();
        enregistrer_message(
            &c,
            &uuid,
            &json!({ "id": 1, "author_type": "abonne", "author_name": "A", "body": "Bonjour", "created_at": "2026-11-04T09:00:00+00:00" }),
        );
        enregistrer_message(
            &c,
            &uuid,
            &json!({ "id": 1, "author_type": "abonne", "author_name": "A", "body": "Bonjour", "created_at": "2026-11-04T09:00:00+00:00" }),
        );
        enregistrer_message(
            &c,
            &uuid,
            &json!({ "id": 2, "author_type": "agent", "author_name": "Camille", "body": "Salut", "created_at": "2026-11-04T09:01:00+00:00" }),
        );
        // Message d'une discussion inconnue : ignoré.
        enregistrer_message(
            &c,
            "inconnue",
            &json!({ "id": 1, "author_type": "agent", "body": "x" }),
        );
        assert_eq!(messages_apres(&c, "u-1", 0).len(), 2);
        assert_eq!(messages_apres(&c, "u-1", 1).len(), 1);
        let t = crate::maintenant();
        assert_eq!(
            discussion_courante(&c, id, t).map(|d| d.agent),
            Some("Camille".to_string())
        );
        assert!(discussion_courante(&c, autre, t).is_none());
        let _ = c.execute(
            "UPDATE support_discussions SET statut = 'close', maj_le = ?1",
            [(t - 2 * VISIBLE_APRES_FIN) as i64],
        );
        assert!(
            discussion_courante(&c, id, t).is_none(),
            "terminée depuis plus d'une heure"
        );
        let _ = std::fs::remove_dir_all(&d);
    }
}
