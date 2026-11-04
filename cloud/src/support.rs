//! Support d'E-KO RIP Cloud : le support en direct, dans la bulle en bas à droite des pages
//! connectées.
//!
//! La discussion, son historique et les e-mails de suivi sont tenus par le service de support
//! en direct de la Suite E-KO (`https://help.suite-eko.com`, boîte « Plateforme E-KO RIP »).
//! La plate-forme ne fait que charger son widget (`/assets/chat.js`) et lui présenter le client
//! sous une identité signée : `cloud-<id du compte>` (la même référence que l'ancien support de
//! l'ERP) et `identifier_hash = hex(HMAC-SHA256(clé de la boîte, identifiant))`, calculé ici.
//! La clé ne quitte jamais le serveur (ni page, ni journal).
//!
//! Configuration, par l'environnement (`deploy-cloud.yml` les pose) :
//! - `CHATWOOT_WEBSITE_TOKEN` : jeton public de la boîte ;
//! - `CHATWOOT_HMAC_TOKEN` : clé secrète de vérification d'identité de la boîte.
//!
//! Si l'une manque (développement, tests, CI), aucune balise n'est posée.
//!
//! Transition : les demandes encore ouvertes au moment de la bascule restent lisibles 30 jours
//! sur `/support` (lecture seule, depuis l'index local), puis l'index est vidé. Ce reste de
//! l'ancien support (`support_demandes`, pages `/support` et `/support/{code}`) sera retiré
//! ensuite.

use crate::base;
use crate::erp::Erp;
use crate::portail::{Session, echapper, encoder, exiger, page};
use crate::relais::Partage;
use axum::extract::{Path as Chemin, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use rusqlite::{Connection, params};
use serde_json::Value;
use std::sync::RwLock;

/// Adresse du service de support en direct (widget, API temps réel, images).
pub const ORIGINE_SUPPORT: &str = "https://help.suite-eko.com";
/// Les demandes de l'ancien support restent lisibles ce temps après la bascule.
pub const LECTURE_SEULE: u64 = 30 * 86_400;
/// Réglage : date de la bascule vers le support en direct (posée au premier entretien).
const CLE_BASCULE: &str = "support_bascule_le";

pub const BANDEAU: &str = "Le support se fait désormais dans la bulle en bas à droite.";

// ————— Configuration du widget —————

/// Jetons de la boîte de support en direct.
#[derive(Clone)]
pub struct Chatwoot {
    /// Jeton public du widget (`websiteToken`).
    pub jeton: String,
    /// Clé de vérification d'identité (HMAC). Jamais affichée.
    cle_hmac: String,
}

impl std::fmt::Debug for Chatwoot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chatwoot").field("jeton", &self.jeton).field("cle_hmac", &"…").finish()
    }
}

impl Chatwoot {
    /// Les deux jetons, ou rien si l'un manque (widget désactivé).
    pub fn depuis(jeton: Option<&str>, cle_hmac: Option<&str>) -> Option<Chatwoot> {
        let jeton = jeton.map(str::trim).filter(|v| !v.is_empty())?;
        let cle = cle_hmac.map(str::trim).filter(|v| !v.is_empty())?;
        Some(Chatwoot { jeton: jeton.to_string(), cle_hmac: cle.to_string() })
    }

    /// `CHATWOOT_WEBSITE_TOKEN` et `CHATWOOT_HMAC_TOKEN`.
    pub fn depuis_env() -> Option<Chatwoot> {
        let v = |n: &str| std::env::var(n).ok();
        Chatwoot::depuis(v("CHATWOOT_WEBSITE_TOKEN").as_deref(), v("CHATWOOT_HMAC_TOKEN").as_deref())
    }

    /// `identifier_hash` attendu par le service pour cet identifiant.
    pub fn empreinte(&self, identifiant: &str) -> String {
        crate::hex(&crate::hmac_sha256(self.cle_hmac.as_bytes(), identifiant.as_bytes()))
    }
}

static CONFIGURATION: RwLock<Option<Chatwoot>> = RwLock::new(None);

/// Règle (ou retire, avec `None`) le widget de toutes les pages connectées.
pub fn configurer(c: Option<Chatwoot>) {
    *CONFIGURATION.write().unwrap_or_else(|e| e.into_inner()) = c;
}

/// Au démarrage du service : d'après l'environnement.
pub fn configurer_depuis_env() -> bool {
    let c = Chatwoot::depuis_env();
    let actif = c.is_some();
    configurer(c);
    actif
}

fn configuration() -> Option<Chatwoot> {
    CONFIGURATION.read().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Le widget est-il en service ?
pub fn actif() -> bool {
    configuration().is_some()
}

/// Référence du compte de la plate-forme auprès du support (identifiant du widget).
pub fn reference(compte: i64) -> String {
    format!("cloud-{compte}")
}

/// Balise du widget pour une page connectée (vide si le widget n'est pas configuré).
pub(crate) fn balise_chatwoot(s: &Session) -> String {
    balise_avec(configuration().as_ref(), s)
}

/// Corps de [`balise_chatwoot`] pour une configuration donnée.
pub(crate) fn balise_avec(config: Option<&Chatwoot>, s: &Session) -> String {
    let Some(cfg) = config else { return String::new() };
    let id = reference(s.compte.id);
    format!(
        r#"<script src="/assets/chat.js" defer data-jeton="{}" data-id="{}" data-hash="{}" data-nom="{}" data-email="{}" data-societe="{}"></script>"#,
        echapper(&cfg.jeton),
        echapper(&id),
        cfg.empreinte(&id),
        echapper(s.compte.nom.trim()),
        echapper(&s.compte.email),
        echapper(s.societe.trim()),
    )
}

/// Raison sociale du compte (saisie à la souscription), pour le support.
pub fn societe(c: &Connection, compte: i64) -> String {
    crate::abonnements::lire(c, compte)
        .ok()
        .and_then(|a| a.brouillon.get("raison_sociale").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_default()
}

/// `GET /assets/chat.js` : chargement du widget, identité, lien « Support » et déconnexion.
pub async fn chat_js() -> Response {
    let mut rep = (StatusCode::OK, SCRIPT_CHAT).into_response();
    rep.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("text/javascript; charset=utf-8"));
    rep.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    rep
}

/// Lit ses réglages dans les attributs `data-*` de sa balise (aucun script en ligne).
const SCRIPT_CHAT: &str = r##"(function () {
  "use strict";
  var balise = document.currentScript;
  if (!balise || !balise.dataset.jeton) return;
  var d = balise.dataset;
  var ORIGINE = "https://help.suite-eko.com";
  var langue = (document.documentElement.lang || "fr").slice(0, 2).toLowerCase();
  window.chatwootSettings = { locale: langue === "en" ? "en" : "fr", position: "right", type: "standard" };
  window.addEventListener("chatwoot:ready", function () {
    if (!window.$chatwoot) return;
    // Attributs dans le même appel que l'identité : un setCustomAttributes séparé partirait
    // avec le jeton du contact anonyme et pourrait se perdre lors de la fusion des contacts.
    var attributs = { appli: "rip-cloud", compte_rip: d.id, page: location.pathname };
    if (d.societe) attributs.societe = d.societe;
    var identite = { name: d.nom || d.email, email: d.email, identifier_hash: d.hash, custom_attributes: attributs };
    if (d.societe) identite.company_name = d.societe;
    window.$chatwoot.setUser(d.id, identite);
  });
  var sdk = document.createElement("script");
  sdk.src = ORIGINE + "/packs/js/sdk.js";
  sdk.async = true;
  sdk.onload = function () {
    if (window.chatwootSDK) window.chatwootSDK.run({ websiteToken: d.jeton, baseUrl: ORIGINE });
  };
  document.head.appendChild(sdk);
  // Lien « Support » du menu (et boutons de la page Support) : ouvre la bulle.
  document.addEventListener("click", function (e) {
    var lien = e.target.closest && e.target.closest("[data-support]");
    if (!lien || !window.$chatwoot) return;
    e.preventDefault();
    window.$chatwoot.toggle("open");
  });
  // Déconnexion : la discussion de ce navigateur est oubliée avant l'envoi du formulaire.
  document.addEventListener("submit", function (e) {
    var f = e.target;
    if (!f || f.getAttribute("action") !== "/deconnexion" || f.dataset.oublie) return;
    f.dataset.oublie = "1";
    if (window.$chatwoot) {
      e.preventDefault();
      try { window.$chatwoot.reset(); } catch (x) {}
      f.submit();
    }
  });
})();
"##;

/// Directives CSP des pages connectées pour le widget : ajoutées aux sources de chaque
/// directive, ou posées si elles manquent (sinon `default-src 'none'` les bloquerait).
pub fn csp_avec_support(csp: &str) -> String {
    let ajout = |nom: &str| match nom {
        "script-src" | "frame-src" | "img-src" | "style-src" => Some(ORIGINE_SUPPORT.to_string()),
        "connect-src" => Some(format!("{ORIGINE_SUPPORT} wss://help.suite-eko.com")),
        _ => None,
    };
    let mut vues = Vec::new();
    let mut sortie: Vec<String> = csp
        .split(';')
        .map(str::trim)
        .filter(|d| !d.is_empty())
        .map(|d| {
            let nom = d.split_whitespace().next().unwrap_or("");
            vues.push(nom.to_string());
            match ajout(nom) {
                Some(a) => format!("{d} {a}"),
                None => d.to_string(),
            }
        })
        .collect();
    for nom in ["script-src", "frame-src", "img-src", "connect-src"] {
        if !vues.iter().any(|v| v == nom) {
            sortie.push(format!("{nom} {}", ajout(nom).unwrap_or_default()));
        }
    }
    sortie.join("; ")
}

// ————— Ancien support : demandes en lecture seule (30 jours après la bascule) —————

/// Date de la bascule, posée la première fois qu'on la demande.
pub fn date_bascule(c: &Connection, t: u64) -> u64 {
    if let Some(v) = base::reglage(c, CLE_BASCULE).ok().flatten().and_then(|v| v.parse().ok()) {
        return v;
    }
    let _ = base::regler(c, CLE_BASCULE, &t.to_string());
    t
}

/// Les demandes de l'ancien support sont-elles encore lisibles à l'instant `t` ?
pub fn lecture_seule_en_cours(c: &Connection, t: u64) -> bool {
    t < date_bascule(c, t).saturating_add(LECTURE_SEULE)
}

/// Chaque minute : pose la date de bascule ; 30 jours après, vide l'ancien index.
pub fn entretenir(c: &Connection, t: u64) {
    if !lecture_seule_en_cours(c, t) {
        let _ = c.execute_batch("DELETE FROM support_demandes; DELETE FROM support_messages; DELETE FROM support_discussions;");
    }
}

/// Une demande encore ouverte (ni résolue, ni fermée).
fn ouverte(statut: &str) -> bool {
    !matches!(statut, "resolved" | "closed")
}

struct Demande {
    code: String,
    sujet: String,
    categorie: String,
    statut: String,
    maj_le: u64,
}

fn demandes_ouvertes(c: &Connection, compte: i64) -> Vec<Demande> {
    c.prepare("SELECT code, sujet, categorie, statut, maj_le FROM support_demandes WHERE compte = ?1 AND statut NOT IN ('resolved', 'closed') ORDER BY maj_le DESC, ouverte_le DESC LIMIT 200")
        .and_then(|mut q| q.query_map([compte], |l| Ok(Demande { code: l.get(0)?, sujet: l.get(1)?, categorie: l.get(2)?, statut: l.get(3)?, maj_le: l.get::<_, i64>(4)? as u64 })).and_then(|i| i.collect()))
        .unwrap_or_default()
}

fn demande_ouverte_connue(c: &Connection, compte: i64, code: &str) -> bool {
    c.query_row("SELECT statut FROM support_demandes WHERE code = ?1 AND compte = ?2", params![code, compte], |l| l.get::<_, String>(0)).is_ok_and(|s| ouverte(&s))
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

fn badge(statut: &str) -> String {
    let classe = match statut {
        "resolved" | "closed" => " fin",
        "waiting_client" => " attente",
        _ => "",
    };
    format!(r#"<span class="badge{classe}">{}</span>"#, libelle_statut(statut))
}

const STYLE_SUPPORT: &str = r#"<style>
.fil { display: grid; gap: .7rem; margin: 0 0 1rem; }
.msg { max-width: 90%; padding: .6rem .8rem; border-radius: 12px; background: var(--champ); box-shadow: 0 0 0 .5px var(--champ-bord); }
.msg.moi { justify-self: end; background: var(--accent-doux); }
.msg .qui { color: var(--doux); font-size: 12px; margin-bottom: .15rem; }
.msg .corps { white-space: pre-wrap; word-break: break-word; }
.msg .pj { margin-top: .35rem; font-size: 12px; color: var(--doux); }
.etat-ligne { text-align: center; color: var(--doux); font-size: 12px; }
.badge { display: inline-block; padding: .05rem .5rem; border-radius: 999px; background: var(--accent-doux); color: var(--accent); font-size: 12px; font-weight: 600; white-space: nowrap; }
.badge.fin { background: color-mix(in srgb, var(--ok) 16%, transparent); color: var(--ok); }
.badge.attente { background: color-mix(in srgb, #ff9f0a 18%, transparent); color: #b36b00; }
.bandeau { padding: .7rem .9rem; border-radius: 10px; background: var(--accent-doux); color: var(--texte); }
</style>"#;

/// Bandeau et bouton d'ouverture de la bulle (ou, sans widget, le contact par e-mail).
fn bandeau() -> String {
    if actif() {
        format!(r#"<div class="carte"><p class="bandeau">{BANDEAU}</p><p>Posez votre question à l'équipe d'E-KO : la discussion et son historique restent dans la bulle, et vous recevez les réponses par e-mail si vous avez quitté la page.</p><p><a class="bouton" href="/support" data-support>Ouvrir le support en direct</a></p></div>"#)
    } else {
        r#"<div class="carte"><p>Le support en direct n'est pas encore ouvert sur cette plate-forme. Écrivez-nous à <a href="mailto:contact@e-ko.rip">contact@e-ko.rip</a>.</p></div>"#.to_string()
    }
}

fn page_support(s: &Session, corps: &str) -> Response {
    page("Support", Some(s), &format!("{STYLE_SUPPORT}<h1>Support</h1>{corps}")).into_response()
}

/// `GET /support` : la bulle, et les demandes de l'ancien support encore ouvertes (lecture seule).
pub async fn liste(State(r): State<Partage>, h: HeaderMap) -> Response {
    let s = match exiger(&r, &h, "/support") {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    let demandes = {
        let c = r.base();
        if lecture_seule_en_cours(&c, crate::maintenant()) { demandes_ouvertes(&c, s.compte.id) } else { Vec::new() }
    };
    let anciennes = if demandes.is_empty() {
        String::new()
    } else {
        let lignes: String = demandes
            .iter()
            .map(|d| {
                format!(
                    r#"<tr><td><a href="/support/{c}"><strong>{sujet}</strong></a><br><code>{code}</code></td><td>{cat}</td><td>{badge}</td><td>{maj}</td></tr>"#,
                    c = encoder(&d.code),
                    code = echapper(&d.code),
                    sujet = echapper(if d.sujet.is_empty() { &d.code } else { &d.sujet }),
                    cat = echapper(&d.categorie),
                    badge = badge(&d.statut),
                    maj = crate::temps::date_fr(d.maj_le),
                )
            })
            .collect();
        format!(r#"<div class="carte"><h2>Vos demandes en cours</h2><p class="meta">Ouvertes avant l'arrivée du support en direct : elles restent lisibles ici pendant 30 jours. Pour y donner suite, écrivez dans la bulle en rappelant leur référence.</p><div class="tableau"><table><thead><tr><th>Demande</th><th>File</th><th>État</th><th>Mise à jour</th></tr></thead><tbody>{lignes}</tbody></table></div></div>"#)
    };
    if crate::demandes::actif() {
        return crate::demandes::liste(r, s, anciennes).await;
    }
    page_support(&s, &format!("{}{anciennes}", bandeau()))
}

/// `GET /support/{code}` : une demande de l'ancien support, en lecture seule.
pub async fn demande(State(r): State<Partage>, h: HeaderMap, Chemin(code): Chemin<String>) -> Response {
    let ici = format!("/support/{}", encoder(&code));
    let s = match exiger(&r, &h, &ici) {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    if crate::demandes::actif() && let Ok(numero) = code.parse::<u64>() {
        return crate::demandes::demande(r, s, numero, "").await;
    }
    let id = s.compte.id;
    let connue = {
        let c = r.base();
        code_valide(&code) && lecture_seule_en_cours(&c, crate::maintenant()) && demande_ouverte_connue(&c, id, &code)
    };
    if !connue {
        return Redirect::to("/support").into_response();
    }
    let r2 = r.clone();
    let c2 = code.clone();
    let detail = tokio::task::spawn_blocking(move || detail_demande(&r2, id, &c2)).await.unwrap_or(Err("Service momentanément indisponible.".into()));
    let fil = match detail {
        Ok(d) => format!(
            r#"<div class="carte"><h2>{sujet}</h2><p>{badge} <span class="meta">{code} · {cat} · ouverte le {ouverte}</span></p></div><div class="fil">{fil}</div>"#,
            sujet = echapper(d["label"].as_str().unwrap_or(&code)),
            badge = badge(d["status"].as_str().unwrap_or("open")),
            code = echapper(&code),
            cat = echapper(d["category"]["label"].as_str().unwrap_or("")),
            ouverte = crate::temps::date_fr(date_json(&d["opened_at"]).unwrap_or(0) as u64),
            fil = fil_html(&d),
        ),
        Err(e) => format!(r#"<div class="carte"><p class="erreur">{}</p></div>"#, echapper(&e)),
    };
    page_support(&s, &format!(r#"<p><a href="/support">← Support</a></p>{}{fil}"#, bandeau()))
}

fn code_valide(code: &str) -> bool {
    !code.is_empty() && code.len() <= 64 && code.chars().all(|c| c.is_ascii_alphanumeric() || "-._".contains(c))
}

/// Détail d'une demande du compte, lu dans l'ERP (`GET /support/tickets/{code}`).
fn detail_demande(r: &Partage, compte: i64, code: &str) -> Result<Value, String> {
    let erp = Erp::depuis(&r.base()).ok_or_else(|| "Le détail de cette demande n'est plus disponible.".to_string())?;
    let rep = erp.get(&format!("/support/tickets/{}?account_reference={}", encoder(code), encoder(&reference(compte))))?;
    if !rep.ok() {
        return Err("Le détail de cette demande n'est plus disponible ici.".into());
    }
    Ok(rep.donnees().clone())
}

fn date_json(v: &Value) -> Option<i64> {
    v.as_str().and_then(crate::offres::date_iso).map(|t| t as i64)
}

fn pieces(v: &Value) -> Vec<String> {
    v.as_array().cloned().unwrap_or_default().iter().filter_map(|a| a["name"].as_str().map(str::to_string)).collect()
}

fn fil_html(d: &Value) -> String {
    let mut entrees: Vec<(bool, String, String, u64, Vec<String>)> = Vec::new();
    if let Some(desc) = d["description"].as_str().filter(|s| !s.is_empty()) {
        entrees.push((true, "Vous".into(), html_en_texte(desc), date_json(&d["opened_at"]).unwrap_or(0) as u64, pieces(&d["attachments"])));
    }
    for m in d["thread"].as_array().cloned().unwrap_or_default() {
        let quand = date_json(&m["created_at"]).unwrap_or(0) as u64;
        if m["type"].as_str() == Some("status_change") {
            entrees.push((false, String::new(), format!("Statut : {}", libelle_statut(m["status_change"]["to"].as_str().unwrap_or(""))), quand, Vec::new()));
            continue;
        }
        let moi = m["from_customer"].as_bool() == Some(true);
        let qui = if moi {
            "Vous".to_string()
        } else {
            m["author_name"].as_str().filter(|n| !n.is_empty()).map_or("Support E-KO".to_string(), |n| format!("{n} · support E-KO"))
        };
        entrees.push((moi, qui, html_en_texte(m["body"].as_str().unwrap_or("")), quand, pieces(&m["attachments"])));
    }
    let mut sortie = String::new();
    for (moi, qui, corps, quand, pj) in entrees {
        if qui.is_empty() {
            sortie.push_str(&format!(r#"<div class="etat-ligne">{} · {}</div>"#, echapper(&corps), crate::temps::date_heure_fr(quand)));
            continue;
        }
        let pj = if pj.is_empty() { String::new() } else { format!(r#"<div class="pj">Pièces jointes : {}</div>"#, pj.iter().map(|n| echapper(n)).collect::<Vec<_>>().join(", ")) };
        sortie.push_str(&format!(r#"<div class="msg{}"><div class="qui">{} · {}</div><div class="corps">{}</div>{pj}</div>"#, if moi { " moi" } else { "" }, echapper(&qui), crate::temps::date_heure_fr(quand), echapper(&corps)));
    }
    sortie
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
                let nom = balise.trim_start_matches('/').split(|x: char| x.is_whitespace() || x == '/').next().unwrap_or("").to_ascii_lowercase();
                if matches!(nom.as_str(), "br" | "p" | "div" | "li" | "tr" | "h1" | "h2" | "h3" | "blockquote") {
                    sortie.push('\n');
                }
            }
            _ if dans_balise => balise.push(c),
            _ => sortie.push(c),
        }
    }
    let sortie = sortie.replace("&nbsp;", " ").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'").replace("&#039;", "'").replace("&apos;", "'").replace("&amp;", "&");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_en_texte_nettoie() {
        assert_eq!(html_en_texte("<p>Bonjour &amp; merci</p><p>Voici <b>le</b> journal.<br>Ligne 2 &lt;ok&gt;</p>"), "Bonjour & merci\n\nVoici le journal.\nLigne 2 <ok>");
        assert_eq!(html_en_texte("<script>x</script>\n\n\n<p>a</p>"), "x\n\na");
        assert_eq!(html_en_texte(""), "");
    }

    #[test]
    fn configuration_complete_ou_rien() {
        assert!(Chatwoot::depuis(None, Some("cle")).is_none());
        assert!(Chatwoot::depuis(Some("jeton"), None).is_none());
        assert!(Chatwoot::depuis(Some("jeton"), Some("  ")).is_none());
        assert!(Chatwoot::depuis(Some(""), Some("cle")).is_none());
        let c = Chatwoot::depuis(Some(" jeton "), Some("cle-tres-secrete")).unwrap();
        assert_eq!(c.jeton, "jeton");
        assert!(!format!("{c:?}").contains("cle-tres-secrete"), "clé jamais dans les journaux");
    }

    #[test]
    fn empreinte_d_identite() {
        // Vecteur fixe : HMAC-SHA256(clé « cle-secrete-de-test », « cloud-42 »).
        let c = Chatwoot::depuis(Some("jeton"), Some("cle-secrete-de-test")).unwrap();
        assert_eq!(c.empreinte("cloud-42"), "af70942bf3b4601b1d957255a94911ffdc15acca2412be2dbacad32a37c3412d");
        assert_eq!(reference(42), "cloud-42");
    }

    #[test]
    fn csp_ouverte_au_support() {
        let csp = csp_avec_support("default-src 'none'; script-src 'self'; connect-src 'self'; img-src 'self' data:; frame-ancestors 'none'");
        assert!(csp.contains("script-src 'self' https://help.suite-eko.com"), "{csp}");
        assert!(csp.contains("connect-src 'self' https://help.suite-eko.com wss://help.suite-eko.com"), "{csp}");
        assert!(csp.contains("img-src 'self' data: https://help.suite-eko.com"), "{csp}");
        assert!(csp.contains("frame-ancestors 'none'") && csp.starts_with("default-src 'none'"), "{csp}");
        assert!(csp.ends_with("; frame-src https://help.suite-eko.com"), "directive manquante posée : {csp}");
    }

    #[test]
    fn bascule_et_lecture_seule() {
        let d = std::env::temp_dir().join(format!("eko-cloud-support-{}", crate::aleatoire_hex(6)));
        let c = base::ouvrir(&d).unwrap();
        let t = 1_800_000_000;
        assert_eq!(date_bascule(&c, t), t);
        assert_eq!(date_bascule(&c, t + 10), t, "posée une seule fois");
        let id = base::creer_compte(&c, "a@b.fr", "A", None).unwrap().unwrap();
        for (code, statut) in [("TKT-1", "open"), ("TKT-2", "resolved"), ("TKT-3", "waiting_client")] {
            c.execute("INSERT INTO support_demandes (code, compte, sujet, statut) VALUES (?1, ?2, 'Sujet', ?3)", params![code, id, statut]).unwrap();
        }
        let l = demandes_ouvertes(&c, id);
        assert_eq!(l.iter().map(|d| d.code.as_str()).collect::<std::collections::BTreeSet<_>>(), ["TKT-1", "TKT-3"].into());
        assert!(demande_ouverte_connue(&c, id, "TKT-1") && !demande_ouverte_connue(&c, id, "TKT-2"));
        entretenir(&c, t + LECTURE_SEULE - 1);
        assert_eq!(demandes_ouvertes(&c, id).len(), 2, "encore lisibles");
        entretenir(&c, t + LECTURE_SEULE);
        assert!(demandes_ouvertes(&c, id).is_empty(), "vidées après 30 jours");
        let _ = std::fs::remove_dir_all(&d);
    }
}
