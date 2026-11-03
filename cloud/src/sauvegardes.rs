//! Sauvegardes en ligne des serveurs RIP et inventaire de leurs machines.
//!
//! Côté serveur RIP (mêmes identifiants que le tunnel : `Authorization: Bearer <site>.<clé>`) :
//! - `POST /api/site/inventaire` : machines du site → `{offre, sauvegardes, depassement}` ;
//! - `POST /api/site/sauvegardes` : une archive (200 Mo au plus, en-têtes `X-EKO-Motif` et
//!   `X-EKO-Empreinte`) → 201 `{id}`, 402 `offre`, 409 `identique`, 413 `quota` ;
//! - `GET /api/site/sauvegardes` (liste) et `GET /api/site/sauvegardes/{id}` (archive).
//!
//! Côté portail : liste par site, téléchargement (membres du site) et restauration (propriétaire
//! ou administrateur) : la plate-forme envoie `POST /api/cloud/restaurer` dans le tunnel du
//! site, signé par `X-EKO-Cloud-Service`.
//!
//! Les archives sont des objets S3 opaques (`sauvegardes/<compte>/<site>/<id>.tar`), indexés
//! dans la table `sauvegardes`. Conservation et quota selon l'offre du propriétaire (module
//! `offres`), appliqués à chaque envoi et chaque jour ; copie quotidienne de la base du portail.

use crate::abonnements;
use crate::base;
use crate::courriels;
use crate::notifications::{self, Notification};
use crate::offres::{self, Offre};
use crate::portail::{self, Champs, Session, champ, csrf_ok, date, echapper, erreur_page, exiger, exiger_admin, hidden_csrf, origine_ok, page, refuse_origine};
use crate::relais::Partage;
use crate::s3::S3;
use crate::sites;
use axum::Json;
use axum::body::{Body, Bytes};
use axum::extract::{Path as Chemin, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use std::time::Duration;

/// Taille maximale d'une archive envoyée par un serveur RIP.
pub const MAX_ARCHIVE: usize = 200 * 1024 * 1024;
/// Une sauvegarde nocturne absente depuis aussi longtemps : « deux nuits de suite ».
pub const DELAI_ECHEC: u64 = 52 * 3600;
/// Clé de `reglages` : jour (UTC) du dernier entretien quotidien.
pub const CLE_ENTRETIEN: &str = "sauvegardes_entretien_jour";
/// En-tête des commandes de la plate-forme vers un serveur RIP.
pub const ENTETE_SERVICE: &str = "x-eko-cloud-service";

#[derive(Clone, Debug, PartialEq)]
pub struct Version {
    pub id: String,
    pub site: String,
    pub compte: i64,
    pub cree_le: u64,
    pub octets: u64,
    pub empreinte: String,
    pub motif: String,
    pub cle_s3: String,
}

fn version(l: &rusqlite::Row) -> rusqlite::Result<Version> {
    Ok(Version { id: l.get(0)?, site: l.get(1)?, compte: l.get(2)?, cree_le: l.get::<_, i64>(3)? as u64, octets: l.get::<_, i64>(4)? as u64, empreinte: l.get(5)?, motif: l.get(6)?, cle_s3: l.get(7)? })
}

const COLONNES: &str = "id, site, compte, cree_le, octets, empreinte, motif, cle_s3";

pub fn versions_du_site(c: &Connection, site: &str) -> rusqlite::Result<Vec<Version>> {
    c.prepare(&format!("SELECT {COLONNES} FROM sauvegardes WHERE site = ?1 ORDER BY cree_le DESC, rowid DESC"))?.query_map([site], version)?.collect()
}

pub fn version_par_id(c: &Connection, id: &str) -> rusqlite::Result<Option<Version>> {
    c.query_row(&format!("SELECT {COLONNES} FROM sauvegardes WHERE id = ?1"), [id], version).optional()
}

pub fn octets_du_compte(c: &Connection, compte: i64) -> rusqlite::Result<u64> {
    c.query_row("SELECT COALESCE(SUM(octets), 0) FROM sauvegardes WHERE compte = ?1", [compte], |l| l.get::<_, i64>(0)).map(|n| n as u64)
}

/// Ligne « Sauvegardes » du résumé hebdomadaire : (dernière réussie, octets du site, quota du
/// propriétaire) ; `None` si le site n'a jamais rien envoyé.
pub fn ligne_resume(c: &Connection, site: &str) -> rusqlite::Result<Option<(Option<u64>, u64, u64)>> {
    let (n, derniere, octets): (i64, Option<i64>, i64) = c.query_row("SELECT COUNT(*), MAX(cree_le), COALESCE(SUM(octets), 0) FROM sauvegardes WHERE site = ?1", [site], |l| Ok((l.get(0)?, l.get(1)?, l.get(2)?)))?;
    let essai: Option<i64> = c.query_row("SELECT nuit_le FROM inventaires WHERE site = ?1", [site], |l| l.get(0)).optional()?.flatten();
    if n == 0 && essai.is_none() {
        return Ok(None);
    }
    let quota = match abonnements::proprietaire_du_site(c, site)? {
        Some(p) => abonnements::quota(c, p)?,
        None => Offre::Gratuit.quota(0),
    };
    Ok(Some((derniere.map(|d| d as u64), octets as u64, quota)))
}

/// Supprime des versions (objet S3 puis index). Bloquant.
fn supprimer_versions(r: &Partage, s3: &S3, ids: &[String]) {
    for id in ids {
        let Ok(Some(v)) = version_par_id(&r.base(), id) else { continue };
        match s3.supprimer(&v.cle_s3) {
            Ok(()) => {
                let _ = r.base().execute("DELETE FROM sauvegardes WHERE id = ?1", [id]);
            }
            Err(e) => eprintln!("sauvegarde {id} non supprimée : {e}"),
        }
    }
}

// ————— Côté serveur RIP —————

fn refus(statut: StatusCode, erreur: &str) -> Response {
    (statut, Json(json!({ "erreur": erreur }))).into_response()
}

/// Site authentifié par `Authorization: Bearer <site>.<clé>` (mêmes identifiants que le tunnel).
// La réponse d'erreur axum est renvoyée telle quelle au client : la mettre en Box
// n'apporterait rien (clippy 1.99, result_large_err).
#[allow(clippy::result_large_err)]
async fn site_authentifie(r: &Partage, h: &HeaderMap) -> Result<sites::Site, Response> {
    let v = h.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer ")).and_then(|v| v.split_once('.')).map(|(s, c)| (s.to_string(), c.to_string()));
    let Some((id, cle)) = v else { return Err(refus(StatusCode::UNAUTHORIZED, "cle_absente")) };
    let donnees = r.config.donnees.clone();
    let site = tokio::task::spawn_blocking(move || sites::lire(&donnees)).await.map_err(|_| refus(StatusCode::SERVICE_UNAVAILABLE, "indisponible"))?.map_err(|_| refus(StatusCode::SERVICE_UNAVAILABLE, "indisponible"))?.into_iter().find(|s| s.id == id);
    match site {
        Some(s) if s.cle_valide(&cle) => Ok(s),
        _ => Err(refus(StatusCode::UNAUTHORIZED, "cle_invalide")),
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct Machine {
    #[serde(default)]
    pub id: Value,
    #[serde(default)]
    pub nom: String,
    #[serde(default)]
    pub modele: String,
}

#[derive(serde::Deserialize)]
pub struct Inventaire {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub imprimantes: Vec<Machine>,
    #[serde(default)]
    pub decoupeurs: Vec<Machine>,
}

fn nettoyer(v: &[Machine]) -> Vec<Machine> {
    let court = |s: &str| s.chars().filter(|c| !c.is_control()).take(120).collect::<String>();
    v.iter().take(500).map(|m| Machine { id: m.id.clone(), nom: court(&m.nom), modele: court(&m.modele) }).collect()
}

/// `POST /api/site/inventaire`.
pub async fn recevoir_inventaire(State(r): State<Partage>, h: HeaderMap, Json(inv): Json<Inventaire>) -> Response {
    let site = match site_authentifie(&r, &h).await {
        Ok(s) => s,
        Err(rep) => return rep,
    };
    let (imp, dec) = (nettoyer(&inv.imprimantes), nettoyer(&inv.decoupeurs));
    let version: String = inv.version.chars().filter(|c| !c.is_control()).take(40).collect();
    let r2 = r.clone();
    let reponse = tokio::task::spawn_blocking(move || -> rusqlite::Result<Value> {
        let t = crate::maintenant();
        let proprietaire = {
            let c = r2.base();
            c.execute(
                "INSERT INTO inventaires (site, version, imprimantes, decoupeurs, nb_imprimantes, nb_decoupeurs, recu_le) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) ON CONFLICT(site) DO UPDATE SET version = excluded.version, imprimantes = excluded.imprimantes, decoupeurs = excluded.decoupeurs, nb_imprimantes = excluded.nb_imprimantes, nb_decoupeurs = excluded.nb_decoupeurs, recu_le = excluded.recu_le",
                params![site.id, version, serde_json::to_string(&imp).unwrap_or_default(), serde_json::to_string(&dec).unwrap_or_default(), imp.len() as i64, dec.len() as i64, t as i64],
            )?;
            abonnements::proprietaire_du_site(&c, &site.id)?
        };
        let Some(p) = proprietaire else { return Ok(json!({ "offre": Offre::Gratuit.code(), "sauvegardes": false, "depassement": false })) };
        if let Err(e) = abonnements::recalculer_sieges(&r2, p, t) {
            eprintln!("sièges du compte {p} : {e}");
        }
        abonnements::verifier_depassement(&r2, p, t);
        let c = r2.base();
        let ab = abonnements::lire(&c, p)?;
        Ok(json!({ "offre": ab.offre.code(), "sauvegardes": S3::depuis(&c).is_some() && !r2.est_suspendu(&site.id), "depassement": ab.depassement_depuis.is_some() }))
    })
    .await;
    match reponse {
        Ok(Ok(v)) => Json(v).into_response(),
        _ => refus(StatusCode::SERVICE_UNAVAILABLE, "indisponible"),
    }
}

/// Issue d'un envoi d'archive.
#[derive(Debug, PartialEq)]
pub enum Envoi {
    Cree(String),
    Offre,
    Identique,
    Quota,
    Stockage(String),
}

/// Garde une archive reçue (bloquant) : contrôles d'offre, de doublon et de quota, envoi S3,
/// index, puis conservation de l'offre.
pub fn garder(r: &Partage, site: &str, motif: &str, empreinte: &str, corps: &[u8], t: u64) -> Envoi {
    let (proprietaire, s3) = {
        let c = r.base();
        if motif == "nuit" {
            let _ = c.execute("INSERT INTO inventaires (site, recu_le, nuit_le) VALUES (?1, 0, ?2) ON CONFLICT(site) DO UPDATE SET nuit_le = excluded.nuit_le, echec_prevenu = 0", params![site, t as i64]);
        }
        (abonnements::proprietaire_du_site(&c, site).ok().flatten(), S3::depuis(&c))
    };
    let Some(compte) = proprietaire else { return Envoi::Offre };
    if r.est_suspendu(site) {
        return Envoi::Offre;
    }
    let Some(s3) = s3 else { return Envoi::Stockage("stockage non réglé".into()) };
    let id = crate::aleatoire_hex(16);
    let (a_supprimer, depasse) = {
        let c = r.base();
        let Ok(versions) = versions_du_site(&c, site) else { return Envoi::Stockage("index illisible".into()) };
        if versions.first().is_some_and(|v| v.empreinte == empreinte) {
            return Envoi::Identique;
        }
        let ab = abonnements::lire(&c, compte).unwrap_or_else(|_| abonnements::Abonnement::gratuit(compte));
        let quota = abonnements::quota(&c, compte).unwrap_or(0);
        let mut liste: Vec<(String, u64)> = versions.iter().map(|v| (v.id.clone(), v.cree_le)).collect();
        liste.insert(0, (id.clone(), t));
        let a_supprimer = offres::a_supprimer(&liste, ab.offre, ab.gratuit_depuis, t);
        let libere: u64 = versions.iter().filter(|v| a_supprimer.contains(&v.id)).map(|v| v.octets).sum();
        let utilise = octets_du_compte(&c, compte).unwrap_or(0);
        (a_supprimer, utilise.saturating_sub(libere) + corps.len() as u64 > quota)
    };
    if depasse {
        verifier_quota(r, compte, true);
        return Envoi::Quota;
    }
    let cle = format!("sauvegardes/{compte}/{site}/{id}.tar");
    if let Err(e) = s3.mettre(&cle, corps) {
        eprintln!("sauvegarde du site {site} non envoyée au stockage : {e}");
        return Envoi::Stockage(e);
    }
    if let Err(e) = r.base().execute("INSERT INTO sauvegardes (id, site, compte, cree_le, octets, empreinte, motif, cle_s3) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)", params![id, site, compte, t as i64, corps.len() as i64, empreinte, motif, cle]) {
        let _ = s3.supprimer(&cle);
        return Envoi::Stockage(e.to_string());
    }
    supprimer_versions(r, &s3, &a_supprimer);
    verifier_quota(r, compte, false);
    Envoi::Cree(id)
}

/// `POST /api/site/sauvegardes`.
pub async fn recevoir_sauvegarde(State(r): State<Partage>, h: HeaderMap, corps: Bytes) -> Response {
    let site = match site_authentifie(&r, &h).await {
        Ok(s) => s,
        Err(rep) => return rep,
    };
    let motif = h.get("x-eko-motif").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    if !matches!(motif.as_str(), "nuit" | "changement" | "demande") {
        return refus(StatusCode::BAD_REQUEST, "motif");
    }
    let empreinte = h.get("x-eko-empreinte").and_then(|v| v.to_str().ok()).unwrap_or("").to_ascii_lowercase();
    if corps.is_empty() {
        return refus(StatusCode::BAD_REQUEST, "vide");
    }
    let r2 = r.clone();
    let resultat = tokio::task::spawn_blocking(move || {
        if !crate::egal_constant(&crate::sha256_octets_hex(&corps), &empreinte) {
            return None;
        }
        Some(garder(&r2, &site.id, &motif, &empreinte, &corps, crate::maintenant()))
    })
    .await;
    match resultat {
        Ok(None) => refus(StatusCode::BAD_REQUEST, "empreinte"),
        Ok(Some(Envoi::Cree(id))) => (StatusCode::CREATED, Json(json!({ "id": id }))).into_response(),
        Ok(Some(Envoi::Offre)) => refus(StatusCode::PAYMENT_REQUIRED, "offre"),
        Ok(Some(Envoi::Identique)) => refus(StatusCode::CONFLICT, "identique"),
        Ok(Some(Envoi::Quota)) => refus(StatusCode::PAYLOAD_TOO_LARGE, "quota"),
        Ok(Some(Envoi::Stockage(_))) | Err(_) => refus(StatusCode::SERVICE_UNAVAILABLE, "stockage"),
    }
}

/// `GET /api/site/sauvegardes`.
pub async fn lister_pour_site(State(r): State<Partage>, h: HeaderMap) -> Response {
    let site = match site_authentifie(&r, &h).await {
        Ok(s) => s,
        Err(rep) => return rep,
    };
    let v = versions_du_site(&r.base(), &site.id).unwrap_or_default();
    Json(json!({ "sauvegardes": v.iter().map(|x| json!({ "id": x.id, "cree_le": x.cree_le, "octets": x.octets, "empreinte": x.empreinte, "motif": x.motif })).collect::<Vec<_>>() })).into_response()
}

/// Archive en flux depuis le stockage S3, sans tout garder en mémoire.
async fn diffuser(s3: S3, cle: String, nom: &str) -> Response {
    use std::io::Read;
    let ouvert = tokio::task::spawn_blocking(move || s3.lire_flux(&cle)).await;
    let (taille, mut lecteur) = match ouvert {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => {
            eprintln!("lecture d'une sauvegarde : {e}");
            return refus(StatusCode::BAD_GATEWAY, "stockage");
        }
        Err(_) => return refus(StatusCode::SERVICE_UNAVAILABLE, "stockage"),
    };
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Bytes, std::io::Error>>(8);
    tokio::task::spawn_blocking(move || {
        let mut tampon = vec![0u8; 64 * 1024];
        loop {
            match lecteur.read(&mut tampon) {
                Ok(0) => break,
                Ok(n) => {
                    if tx.blocking_send(Ok(Bytes::copy_from_slice(&tampon[..n]))).is_err() {
                        break;
                    }
                }
                Err(e) => {
                    let _ = tx.blocking_send(Err(e));
                    break;
                }
            }
        }
    });
    let flux = futures_util::stream::unfold(rx, |mut rx| async move { rx.recv().await.map(|x| (x, rx)) });
    let mut rep = Response::new(Body::from_stream(flux));
    let h = rep.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/x-tar"));
    if let Some(t) = taille {
        h.insert(header::CONTENT_LENGTH, HeaderValue::from(t));
    }
    h.insert(header::CONTENT_DISPOSITION, HeaderValue::from_str(&format!("attachment; filename=\"{nom}\"")).unwrap_or(HeaderValue::from_static("attachment")));
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("private, no-store"));
    rep
}

/// `GET /api/site/sauvegardes/{id}` : archive du site, ou d'un autre site du même
/// propriétaire si l'offre permet la restauration croisée (Pro).
pub async fn lire_pour_site(State(r): State<Partage>, h: HeaderMap, Chemin(id): Chemin<String>) -> Response {
    let site = match site_authentifie(&r, &h).await {
        Ok(s) => s,
        Err(rep) => return rep,
    };
    let (v, s3, permis) = {
        let c = r.base();
        let v = version_par_id(&c, &id).ok().flatten();
        let permis = v.as_ref().is_some_and(|v| v.site == site.id || (abonnements::proprietaire_du_site(&c, &site.id).ok().flatten() == Some(v.compte) && abonnements::lire(&c, v.compte).map(|a| a.offre.restauration_croisee()).unwrap_or(false)));
        (v, S3::depuis(&c), permis)
    };
    let (Some(v), true) = (v, permis) else { return refus(StatusCode::NOT_FOUND, "inconnue") };
    let Some(s3) = s3 else { return refus(StatusCode::SERVICE_UNAVAILABLE, "stockage") };
    diffuser(s3, v.cle_s3, &format!("{}.tar", v.id)).await
}

// ————— Portail —————

fn role(r: &Partage, s: &Session, site: &str) -> Option<String> {
    base::droit(&r.base(), site, s.compte.id).ok().flatten()
}

fn peut_restaurer(role: &str) -> bool {
    matches!(role, "proprietaire" | "admin")
}

fn libelle_motif(m: &str) -> &'static str {
    match m {
        "nuit" => "Nocturne",
        "changement" => "Après modification",
        _ => "À la demande",
    }
}

/// `GET /sites/{site}/sauvegardes`.
pub async fn page_site(State(r): State<Partage>, h: HeaderMap, Chemin(site): Chemin<String>, axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>) -> Response {
    let s = match exiger(&r, &h, &format!("/sites/{}/sauvegardes", portail::encoder(&site))) {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    let Some(mon_role) = role(&r, &s, &site) else { return (StatusCode::NOT_FOUND, page("Mes sites", Some(&s), r#"<div class="carte"><p class="erreur">Site introuvable.</p><p><a href="/sites">Retour</a></p></div>"#)).into_response() };
    let tous = sites::lire(&r.config.donnees).unwrap_or_default();
    let nom = tous.iter().find(|x| x.id == site).map_or(site.clone(), |x| x.nom.clone());
    let (versions, offre, quota, utilise, cibles, stockage) = {
        let c = r.base();
        let p = abonnements::proprietaire_du_site(&c, &site).ok().flatten();
        let ab = p.and_then(|p| abonnements::lire(&c, p).ok());
        let offre = ab.as_ref().map_or(Offre::Gratuit, |a| a.offre);
        // Restauration vers un autre site : même propriétaire, offre Pro, droits suffisants.
        let cibles: Vec<(String, String)> = if offre.restauration_croisee() && peut_restaurer(&mon_role) {
            p.and_then(|p| abonnements::sites_possedes(&c, p).ok()).unwrap_or_default().into_iter().filter(|x| x != &site && base::droit(&c, x, s.compte.id).ok().flatten().is_some_and(|r| peut_restaurer(&r))).map(|x| (x.clone(), tous.iter().find(|t| t.id == x).map_or(x.clone(), |t| t.nom.clone()))).collect()
        } else {
            vec![]
        };
        (versions_du_site(&c, &site).unwrap_or_default(), offre, p.and_then(|p| abonnements::quota(&c, p).ok()).unwrap_or(0), p.and_then(|p| octets_du_compte(&c, p).ok()).unwrap_or(0), cibles, S3::depuis(&c).is_some())
    };
    let message = match q.get("ok").map(String::as_str) {
        Some("restauree") => r#"<p class="ok">Restauration lancée : le serveur RIP récupère la sauvegarde et redémarre ses services.</p>"#,
        _ => "",
    };
    let lignes: String = versions
        .iter()
        .map(|v| {
            let restaurer = if peut_restaurer(&mon_role) {
                let choix = if cibles.is_empty() { String::new() } else { format!(r#"<select name="vers" style="max-width: 14rem"><option value="{}">Ce site</option>{}</select>"#, echapper(&site), cibles.iter().map(|(i, n)| format!(r#"<option value="{}">{}</option>"#, echapper(i), echapper(n))).collect::<String>()) };
                format!(
                    r#"<form method="post" action="/sites/{}/sauvegardes/{}/restaurer" onsubmit="return confirm('Restaurer cette sauvegarde ? Les réglages actuels du serveur RIP seront remplacés.')">{}{choix}<label class="case"><input type="checkbox" name="confirmer" value="1" required> <span>Remplacer les réglages actuels</span></label><button class="discret" type="submit">Restaurer</button></form>"#,
                    echapper(&site),
                    echapper(&v.id),
                    hidden_csrf(&s)
                )
            } else {
                String::new()
            };
            format!(r#"<tr><td>{}</td><td>{}</td><td>{}</td><td><a href="/sites/{}/sauvegardes/{}">Télécharger</a></td><td>{restaurer}</td></tr>"#, echapper(&crate::temps::date_heure_fr(v.cree_le)), libelle_motif(&v.motif), offres::taille(v.octets), echapper(&site), echapper(&v.id))
        })
        .collect();
    let liste = if versions.is_empty() { r#"<p class="meta">Aucune sauvegarde en ligne pour ce site. Les serveurs RIP à jour envoient une sauvegarde chaque nuit et après chaque modification de leurs réglages.</p>"#.to_string() } else { format!(r#"<div class="tableau"><table><thead><tr><th>Date</th><th>Motif</th><th>Taille</th><th></th><th></th></tr></thead><tbody>{lignes}</tbody></table></div>"#) };
    let corps = format!(
        r#"<h1>Sauvegardes</h1>{message}<div class="carte"><h2>Sauvegardes de « {} »</h2><p class="meta">Offre {} du propriétaire : {} · {} utilisés sur {}.{}</p>{liste}</div><p><a href="/sites">Retour à mes sites</a></p>"#,
        echapper(&nom),
        offre.nom(),
        offre.libelle_conservation(),
        offres::taille(utilise),
        offres::taille(quota),
        if stockage { "" } else { " Le stockage en ligne n'est pas encore ouvert." }
    );
    page("Mes sites", Some(&s), &corps).into_response()
}

/// `GET /sites/{site}/sauvegardes/{id}` : téléchargement (tout membre du site).
pub async fn telecharger(State(r): State<Partage>, h: HeaderMap, Chemin((site, id)): Chemin<(String, String)>) -> Response {
    let s = match exiger(&r, &h, "/sites") {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    let (v, s3) = {
        let c = r.base();
        (version_par_id(&c, &id).ok().flatten(), S3::depuis(&c))
    };
    let (Some(v), Some(_)) = (v.filter(|v| v.site == site), role(&r, &s, &site)) else { return (StatusCode::NOT_FOUND, page("Mes sites", Some(&s), r#"<div class="carte"><p class="erreur">Sauvegarde introuvable.</p></div>"#)).into_response() };
    let Some(s3) = s3 else { return erreur_page(&r, Some(&s), "Stockage indisponible.") };
    let jour = date(Some(v.cree_le));
    diffuser(s3, v.cle_s3, &format!("sauvegarde-{site}-{jour}.tar")).await
}

/// En-tête `X-EKO-Cloud-Service` : `t=<unix>,v1=hex(HMAC-SHA256(cle_sha256 du site, "{t}.{corps}"))`.
pub fn signer_service(cle_sha256: &str, corps: &[u8], t: u64) -> String {
    let mut m = format!("{t}.").into_bytes();
    m.extend_from_slice(corps);
    format!("t={t},v1={}", crate::hex(&crate::hmac_sha256(cle_sha256.as_bytes(), &m)))
}

/// `POST /sites/{site}/sauvegardes/{id}/restaurer` (`confirmer`, `vers` facultatif).
pub async fn restaurer(State(r): State<Partage>, h: HeaderMap, Chemin((site, id)): Chemin<(String, String)>, axum::Form(f): Champs) -> Response {
    if !origine_ok(&r, &h) {
        return refuse_origine();
    }
    let s = match exiger(&r, &h, "/sites") {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    if !csrf_ok(&s, &f) || champ(&f, "confirmer") != "1" {
        return refuse_origine();
    }
    let vers = Some(champ(&f, "vers")).filter(|v| !v.is_empty()).unwrap_or(site.clone());
    let refuse = |m: &str| (StatusCode::FORBIDDEN, page("Mes sites", Some(&s), &format!(r#"<div class="carte"><p class="erreur">{}</p><p><a href="/sites/{}/sauvegardes">Retour</a></p></div>"#, echapper(m), echapper(&site)))).into_response();
    let verdict = {
        let c = r.base();
        let v = version_par_id(&c, &id).ok().flatten().filter(|v| v.site == site);
        let source = base::droit(&c, &site, s.compte.id).ok().flatten();
        let cible = base::droit(&c, &vers, s.compte.id).ok().flatten();
        match v {
            None => Err("Sauvegarde introuvable."),
            Some(_) if !source.as_deref().is_some_and(peut_restaurer) || !cible.as_deref().is_some_and(peut_restaurer) => Err("Seuls le propriétaire et les administrateurs du site peuvent restaurer une sauvegarde."),
            Some(v) if vers != site && (abonnements::proprietaire_du_site(&c, &vers).ok().flatten() != Some(v.compte) || !abonnements::lire(&c, v.compte).map(|a| a.offre.restauration_croisee()).unwrap_or(false)) => Err("La restauration vers un autre site est réservée à l'offre Pro, entre sites du même propriétaire."),
            Some(v) => Ok(v),
        }
    };
    let v = match verdict {
        Ok(v) => v,
        Err(m) => return refuse(m),
    };
    if !r.limiter(&format!("restaurer:{}", s.compte.id), 10, Duration::from_secs(3600)) {
        return erreur_page(&r, Some(&s), "Trop de restaurations : réessayer dans une heure.");
    }
    let tous = sites::lire(&r.config.donnees).unwrap_or_default();
    let Some(cible) = tous.iter().find(|x| x.id == vers).cloned() else { return refuse("Site introuvable.") };
    let corps = serde_json::to_vec(&json!({ "id": v.id })).expect("JSON");
    let t = crate::maintenant();
    let signature = signer_service(&cible.cle_sha256, &corps, t);
    let entetes = vec![(header::CONTENT_TYPE, HeaderValue::from_static("application/json")), (axum::http::HeaderName::from_static(ENTETE_SERVICE), HeaderValue::from_str(&signature).expect("en-tête"))];
    match crate::relais::appeler_site(&r, &vers, Method::POST, "/api/cloud/restaurer", entetes, Bytes::from(corps)).await {
        Ok((statut, _)) if (200..300).contains(&statut) => {
            eprintln!("restauration de la sauvegarde {} sur le site {vers} par le compte {}", v.id, s.compte.id);
            let dest = crate::evenements::destinataires_tous(&r.base(), &vers).unwrap_or_default();
            for (compte, email, qui) in dest {
                let _ = compte;
                r.envoyer(courriels::restauration_effectuee(&r.config.url_publique, &email, &qui, &cible.nom, &s.compte.nom, t, v.cree_le));
            }
            Redirect::to(&format!("/sites/{}/sauvegardes?ok=restauree", portail::encoder(&site))).into_response()
        }
        Ok((statut, _)) => erreur_page(&r, Some(&s), &format!("Le serveur RIP a refusé la restauration (HTTP {statut}).")),
        Err(e) => {
            eprintln!("restauration sur le site {vers} : {e}");
            erreur_page(&r, Some(&s), "Le serveur RIP de ce site n'est pas joignable : il doit être en ligne pour restaurer.")
        }
    }
}

// ————— Entretien quotidien —————

/// Alerte de quota (80 % puis 100 %) une fois par seuil franchi. Bloquant.
pub fn verifier_quota(r: &Partage, compte: i64, plein: bool) {
    let (niveau, deja, utilise, quota) = {
        let c = r.base();
        let Ok(ab) = abonnements::lire(&c, compte) else { return };
        let quota = abonnements::quota(&c, compte).unwrap_or(0).max(1);
        let utilise = octets_du_compte(&c, compte).unwrap_or(0);
        let pourcent = utilise.saturating_mul(100) / quota;
        let niveau = if plein || pourcent >= 100 { 100 } else if pourcent >= 80 { 80 } else { 0 };
        (niveau, ab.quota_alerte, utilise, quota)
    };
    if niveau == deja {
        return;
    }
    let _ = abonnements::maj(&r.base(), compte, &[("quota_alerte", rusqlite::types::Value::Integer(niveau))]);
    if niveau <= deja {
        return;
    }
    let Ok(Some(cp)) = base::compte_par_id(&r.base(), compte) else { return };
    let url = &r.config.url_publique;
    let c = courriels::quota_sauvegarde(url, &cp.email, &cp.nom, niveau as u64, &offres::taille(utilise), &offres::taille(quota));
    notifications::notifier(r, compte, Some(c), Notification::autre(notifications::SAUVEGARDE, None, if niveau >= 100 { "Espace de sauvegarde plein" } else { "Espace de sauvegarde presque plein" }, &format!("{} utilisés sur {}.", offres::taille(utilise), offres::taille(quota)), &format!("{url}/abonnement")));
}

/// Une fois par jour : conservation, quotas, sauvegardes en échec, copie de la base.
pub async fn entretenir(r: &Partage) {
    let t = crate::maintenant();
    let jour = (t / 86_400).to_string();
    {
        let c = r.base();
        if base::reglage(&c, CLE_ENTRETIEN).ok().flatten().as_deref() == Some(jour.as_str()) {
            return;
        }
        let _ = base::regler(&c, CLE_ENTRETIEN, &jour);
    }
    let r2 = r.clone();
    let _ = tokio::task::spawn_blocking(move || {
        entretien_quotidien(&r2, t);
        if let Err(e) = sauvegarder_base(&r2, t) {
            eprintln!("copie de la base du portail : {e}");
        }
    })
    .await;
}

/// Conservation, quotas et sauvegardes en échec à l'instant `t` (bloquant ; appelable dans les tests).
pub fn entretien_quotidien(r: &Partage, t: u64) {
    let s3 = S3::depuis(&r.base());
    if let Some(s3) = &s3 {
        let tous = sites::lire(&r.config.donnees).unwrap_or_default();
        let par_site: Vec<String> = r.base().prepare("SELECT DISTINCT site FROM sauvegardes").and_then(|mut q| q.query_map([], |l| l.get(0))?.collect()).unwrap_or_default();
        for site in par_site {
            let a_supprimer = {
                let c = r.base();
                let versions = versions_du_site(&c, &site).unwrap_or_default();
                let liste: Vec<(String, u64)> = versions.iter().map(|v| (v.id.clone(), v.cree_le)).collect();
                match (tous.iter().any(|x| x.id == site), abonnements::proprietaire_du_site(&c, &site).ok().flatten()) {
                    (true, Some(p)) => {
                        let ab = abonnements::lire(&c, p).ok();
                        offres::a_supprimer(&liste, ab.as_ref().map_or(Offre::Gratuit, |a| a.offre), ab.and_then(|a| a.gratuit_depuis), t)
                    }
                    // Site retiré : ses sauvegardes restent 90 jours après la dernière.
                    _ => {
                        let derniere = versions.iter().map(|v| v.cree_le).max().unwrap_or(0);
                        if t > derniere + offres::CONSERVATION_APRES_GRATUIT { liste.into_iter().map(|(i, _)| i).collect() } else { vec![] }
                    }
                }
            };
            if !a_supprimer.is_empty() {
                eprintln!("sauvegardes du site {site} : {} version(s) supprimée(s) (conservation)", a_supprimer.len());
                supprimer_versions(r, s3, &a_supprimer);
            }
        }
    }
    let comptes: Vec<i64> = r.base().prepare("SELECT DISTINCT compte FROM sauvegardes").and_then(|mut q| q.query_map([], |l| l.get(0))?.collect()).unwrap_or_default();
    for c in comptes {
        verifier_quota(r, c, false);
    }
    // Deux nuits sans sauvegarde nocturne : propriétaire et administrateurs du site.
    let echecs: Vec<String> = r.base().prepare("SELECT site FROM inventaires WHERE nuit_le IS NOT NULL AND nuit_le < ?1 AND echec_prevenu = 0").and_then(|mut q| q.query_map([t.saturating_sub(DELAI_ECHEC) as i64], |l| l.get(0))?.collect()).unwrap_or_default();
    let tous = sites::lire(&r.config.donnees).unwrap_or_default();
    for site in echecs {
        let Some(s) = tous.iter().find(|x| x.id == site) else { continue };
        let (dest, derniere) = {
            let c = r.base();
            let _ = c.execute("UPDATE inventaires SET echec_prevenu = 1 WHERE site = ?1", [&site]);
            (crate::evenements::destinataires_tous(&c, &site).unwrap_or_default(), versions_du_site(&c, &site).ok().and_then(|v| v.first().map(|x| x.cree_le)))
        };
        eprintln!("site {site} : pas de sauvegarde nocturne depuis deux nuits");
        let url = &r.config.url_publique;
        for (compte, email, qui) in dest {
            let c = courriels::sauvegarde_en_echec(url, &email, &qui, &s.nom, derniere);
            notifications::notifier(r, compte, Some(c), Notification::autre(notifications::SAUVEGARDE, Some(&site), &format!("Sauvegarde de « {} » en échec", s.nom), "Aucune sauvegarde nocturne depuis deux nuits.", &format!("{url}/sites/{site}/sauvegardes")));
        }
    }
}

/// Copie de `portail.sqlite` (API de sauvegarde de SQLite) vers le second bucket. Bloquant.
pub fn sauvegarder_base(r: &Partage, t: u64) -> Result<(), String> {
    let Some(s3) = S3::depuis_base(&r.base()) else { return Ok(()) };
    let tmp = r.config.donnees.join(format!("portail-copie-{}.sqlite", crate::aleatoire_hex(4)));
    let resultat = (|| {
        let source = Connection::open(r.config.donnees.join("portail.sqlite")).map_err(|e| e.to_string())?;
        source.busy_timeout(Duration::from_secs(10)).map_err(|e| e.to_string())?;
        source.backup(rusqlite::MAIN_DB, &tmp, None).map_err(|e| e.to_string())?;
        let octets = std::fs::read(&tmp).map_err(|e| e.to_string())?;
        s3.mettre(&format!("portail/portail-{}.sqlite", date(Some(t))), &octets)
    })();
    let _ = std::fs::remove_file(&tmp);
    if resultat.is_ok() {
        eprintln!("base du portail copiée vers le stockage ({})", s3.bucket);
    }
    resultat
}

// ————— Administration —————

/// Carte « Stockage S3 ».
pub(crate) fn carte_admin(b: &Connection, s: &Session) -> String {
    let valeur = |cle: &str| base::reglage(b, cle).ok().flatten().unwrap_or_default();
    format!(
        r#"<div class="carte"><h2>Stockage S3</h2>
<p class="meta">Sauvegardes en ligne des serveurs RIP (OVH Object Storage ou service compatible S3, adresses « path-style »). Objets : <code>sauvegardes/&lt;compte&gt;/&lt;site&gt;/&lt;id&gt;.tar</code>. La base du portail est copiée chaque nuit dans le second bucket.</p>
<form method="post" action="/admin/stockage">{csrf}
<label>Adresse (endpoint)<input type="text" name="s3_endpoint" value="{ep}" placeholder="https://s3.gra.io.cloud.ovh.net" autocomplete="off"></label>
<span class="ligne"><label>Région<input type="text" name="s3_region" value="{rg}" placeholder="gra" autocomplete="off"></label><label>Bucket<input type="text" name="s3_bucket" value="{bk}" autocomplete="off"></label></span>
<label>Clé d'accès<input type="text" name="s3_acces" value="{ac}" autocomplete="off"></label>
<label>Clé secrète ({sec})<input type="password" name="s3_secret" autocomplete="new-password"></label>
<label class="case"><input type="checkbox" name="s3_sse" value="1"{sse}> <span>Chiffrement côté serveur (x-amz-server-side-encryption: AES256)</span></label>
<h2>Copie de la base du portail</h2>
<span class="ligne"><label>Bucket<input type="text" name="s3_base_bucket" value="{bb}" autocomplete="off"></label><label>Région (facultatif)<input type="text" name="s3_base_region" value="{br}" autocomplete="off"></label></span>
<label>Adresse (facultatif, si autre région)<input type="text" name="s3_base_endpoint" value="{be}" placeholder="https://s3.sbg.io.cloud.ovh.net" autocomplete="off"></label>
<label class="case"><input type="checkbox" name="effacer" value="1"> <span>Effacer la clé secrète enregistrée</span></label>
<button type="submit">Enregistrer</button></form>
<form method="post" action="/admin/stockage/essai">{csrf}<button class="discret" type="submit">Tester le stockage</button> <span class="meta">envoie, relit et supprime un petit objet</span></form></div>"#,
        csrf = hidden_csrf(s),
        ep = echapper(&valeur("s3_endpoint")),
        rg = echapper(&valeur("s3_region")),
        bk = echapper(&valeur("s3_bucket")),
        ac = echapper(&valeur("s3_acces")),
        sec = if valeur("s3_secret").is_empty() { "non renseignée" } else { "enregistrée (laisser vide pour la garder)" },
        sse = if valeur("s3_sse") == "1" { " checked" } else { "" },
        bb = echapper(&valeur("s3_base_bucket")),
        br = echapper(&valeur("s3_base_region")),
        be = echapper(&valeur("s3_base_endpoint")),
    )
}

fn adresse_valide(v: &str) -> bool {
    v.is_empty() || ((v.starts_with("https://") || v.starts_with("http://")) && !v.chars().any(|c| c.is_whitespace() || c.is_control()) && v.len() < 200)
}

fn nom_simple(v: &str) -> bool {
    v.len() <= 100 && v.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// `POST /admin/stockage`.
pub async fn regler_stockage(State(r): State<Partage>, h: HeaderMap, axum::Form(f): Champs) -> Response {
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
    let (ep, be) = (champ(&f, "s3_endpoint"), champ(&f, "s3_base_endpoint"));
    if !adresse_valide(&ep) || !adresse_valide(&be) {
        return erreur_page(&r, Some(&s), "Adresse du stockage invalide (https://…).");
    }
    let simples = ["s3_region", "s3_bucket", "s3_base_bucket", "s3_base_region"];
    if simples.iter().any(|k| !nom_simple(&champ(&f, k))) || champ(&f, "s3_acces").chars().any(|c| c.is_whitespace() || c.is_control()) {
        return erreur_page(&r, Some(&s), "Région, bucket ou clé d'accès invalide.");
    }
    let b = r.base();
    let mut ok = true;
    for k in ["s3_endpoint", "s3_region", "s3_bucket", "s3_acces", "s3_base_bucket", "s3_base_region", "s3_base_endpoint"] {
        ok &= base::regler(&b, k, champ(&f, k).trim_end_matches('/')).is_ok();
    }
    ok &= base::regler(&b, "s3_sse", if champ(&f, "s3_sse") == "1" { "1" } else { "" }).is_ok();
    let secret = champ(&f, "s3_secret");
    if champ(&f, "effacer") == "1" {
        ok &= base::regler(&b, "s3_secret", "").is_ok();
    } else if !secret.is_empty() {
        ok &= base::regler(&b, "s3_secret", &secret).is_ok();
    }
    drop(b);
    if !ok {
        return erreur_page(&r, Some(&s), "Enregistrement impossible.");
    }
    Redirect::to("/admin?ok=stockage").into_response()
}

/// `POST /admin/stockage/essai`.
pub async fn essai_stockage(State(r): State<Partage>, h: HeaderMap, axum::Form(f): Champs) -> Response {
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
    let r2 = r.clone();
    let resultat = tokio::task::spawn_blocking(move || {
        let (principal, copie) = {
            let c = r2.base();
            (S3::depuis(&c), S3::depuis_base(&c))
        };
        let principal = principal.ok_or("Adresse, région, bucket ou clés non renseignés.")?;
        principal.essayer()?;
        if let Some(c) = copie {
            c.essayer().map_err(|e| format!("Bucket de la copie de la base : {e}"))?;
        }
        Ok::<_, String>(())
    })
    .await
    .unwrap_or_else(|_| Err("Service momentanément indisponible.".into()));
    let corps = match resultat {
        Ok(()) => r#"<div class="carte"><h2>Stockage S3 opérationnel</h2><p class="ok">Envoi, relecture et suppression d'un objet d'essai réussis.</p><p><a href="/admin">Retour</a></p></div>"#.to_string(),
        Err(e) => format!(r#"<div class="carte"><h2>Stockage S3 injoignable</h2><p class="erreur">{}</p><p><a href="/admin">Retour</a></p></div>"#, echapper(&e)),
    };
    page("Administration", Some(&s), &format!("<h1>Administration</h1>{corps}")).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_des_commandes() {
        let s = signer_service("abc", br#"{"id":"x"}"#, 1000);
        assert!(s.starts_with("t=1000,v1="));
        let mut m = b"1000.".to_vec();
        m.extend_from_slice(br#"{"id":"x"}"#);
        assert_eq!(s, format!("t=1000,v1={}", crate::hex(&crate::hmac_sha256(b"abc", &m))));
    }
}
