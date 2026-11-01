//! Abonnements de la plate-forme : offre de chaque compte (le propriétaire de ses sites),
//! souscription et paiement (carte Stancer.js avec 3-D Secure dans la page, mandat SEPA signé
//! par SMS), factures, webhooks de l'ERP, sièges facturés et dépassements d'offre.
//!
//! L'ERP « E-KO » (tenant GOLDEN PRINT) facture ; il n'envoie aucun e-mail aux clients du
//! cloud : chaque message part d'ici, avec le gabarit E-KO RIP Cloud, en reprenant les statuts
//! et les étapes de relance de l'ERP.
//!
//! Pages sans script, sauf la saisie de la carte (Stancer.js), la fenêtre 3-D Secure et les
//! vérifications en direct (SIREN, code promo), qui ont toutes un repli par formulaire.

use crate::base::{self, Compte};
use crate::courriels::{self, Courriel};
use crate::erp::{self, Erp};
use crate::notifications::{self, Notification};
use crate::offres::{self, Cycle, Offre, Parc};
use crate::portail::{self, Champs, Session, champ, csrf_ok, date, echapper, encoder, erreur_page, exiger, exiger_admin, hidden_csrf, origine_ok, page, refuse_origine};
use crate::relais::Partage;
use crate::sites;
use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path as Chemin, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use rusqlite::types::Value as Sql;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

/// Au plus un changement de sièges envoyé à l'ERP par heure et par compte.
pub const INTERVALLE_SIEGES: u64 = 3600;
/// Les webhooks déjà traités sont oubliés après 30 jours.
pub const CONSERVATION_WEBHOOKS: u64 = 30 * 86_400;
/// Rappel de fin d'essai.
pub const RAPPEL_ESSAI: u64 = 3 * 86_400;

/// Politique de sécurité des pages de paiement : Stancer.js, sa saisie de carte et la fenêtre
/// 3-D Secure (la banque du porteur répond sur son propre domaine : `frame-src https:`).
pub const CSP_PAIEMENT: &str = "default-src 'none'; script-src 'self' 'unsafe-inline' https://js.stancer.com; style-src 'self' 'unsafe-inline' https://*.stancer.com; frame-src 'self' https:; connect-src 'self' https://*.stancer.com; img-src 'self' data: https://*.stancer.com; font-src https://*.stancer.com; form-action 'self'; frame-ancestors 'none'; base-uri 'none'";

// ————— Données —————

#[derive(Clone, Debug)]
pub struct Abonnement {
    pub compte: i64,
    pub tier_code: Option<String>,
    pub subscription_code: Option<String>,
    pub supplement_code: Option<String>,
    pub offre: Offre,
    pub cycle: Cycle,
    pub statut: String,
    pub seats: i64,
    pub seats_supplement: i64,
    pub seats_envoyes_le: u64,
    pub periode_fin: Option<u64>,
    pub essai_fin: Option<u64>,
    pub essai_prevenu: bool,
    pub moyen_paiement: String,
    pub carte_id: Option<String>,
    pub rum: Option<String>,
    pub rum_prevenu: bool,
    pub relance: i64,
    pub fin_periode_resiliation: bool,
    pub gratuit_depuis: Option<u64>,
    pub depassement_depuis: Option<u64>,
    pub depassement_raison: String,
    pub suspension_prevenue: bool,
    pub quota_alerte: i64,
    pub bon_commande: String,
    /// Souscription en cours : offre, cycle, contact de facturation, codes, mandat…
    pub brouillon: Value,
    /// Supplément résilié à l'échéance (référence gardée jusqu'au webhook de fin).
    pub supplement_resilie: bool,
}

impl Abonnement {
    pub fn gratuit(compte: i64) -> Self {
        Abonnement { compte, tier_code: None, subscription_code: None, supplement_code: None, offre: Offre::Gratuit, cycle: Cycle::Mensuel, statut: String::new(), seats: 0, seats_supplement: 0, seats_envoyes_le: 0, periode_fin: None, essai_fin: None, essai_prevenu: false, moyen_paiement: String::new(), carte_id: None, rum: None, rum_prevenu: false, relance: 0, fin_periode_resiliation: false, gratuit_depuis: None, depassement_depuis: None, depassement_raison: String::new(), suspension_prevenue: false, quota_alerte: 0, bon_commande: String::new(), brouillon: json!({}), supplement_resilie: false }
    }

    /// Abonnement payant en cours dans l'ERP.
    pub fn actif(&self) -> bool {
        self.subscription_code.is_some() && self.statut != "cancelled"
    }

    fn texte_brouillon(&self, cle: &str) -> String {
        self.brouillon.get(cle).and_then(Value::as_str).unwrap_or("").to_string()
    }
}

const COLONNES: &str = "compte, tier_code, subscription_code, supplement_code, offre, cycle, statut, seats, seats_supplement, seats_envoyes_le, periode_fin, essai_fin, essai_prevenu, moyen_paiement, carte_id, rum, rum_prevenu, relance, fin_periode_resiliation, gratuit_depuis, depassement_depuis, depassement_raison, suspension_prevenue, quota_alerte, bon_commande, brouillon, supplement_resilie";

fn ligne(r: &rusqlite::Row) -> rusqlite::Result<Abonnement> {
    let u = |i: usize| -> rusqlite::Result<Option<u64>> { Ok(r.get::<_, Option<i64>>(i)?.map(|v| v as u64)) };
    Ok(Abonnement {
        compte: r.get(0)?,
        tier_code: r.get(1)?,
        subscription_code: r.get(2)?,
        supplement_code: r.get(3)?,
        offre: Offre::depuis(&r.get::<_, String>(4)?).unwrap_or(Offre::Gratuit),
        cycle: Cycle::depuis(&r.get::<_, String>(5)?).unwrap_or(Cycle::Mensuel),
        statut: r.get(6)?,
        seats: r.get(7)?,
        seats_supplement: r.get(8)?,
        seats_envoyes_le: r.get::<_, i64>(9)? as u64,
        periode_fin: u(10)?,
        essai_fin: u(11)?,
        essai_prevenu: r.get(12)?,
        moyen_paiement: r.get(13)?,
        carte_id: r.get(14)?,
        rum: r.get(15)?,
        rum_prevenu: r.get(16)?,
        relance: r.get(17)?,
        fin_periode_resiliation: r.get(18)?,
        gratuit_depuis: u(19)?,
        depassement_depuis: u(20)?,
        depassement_raison: r.get(21)?,
        suspension_prevenue: r.get(22)?,
        quota_alerte: r.get(23)?,
        bon_commande: r.get(24)?,
        brouillon: serde_json::from_str(&r.get::<_, String>(25)?).unwrap_or_else(|_| json!({})),
        supplement_resilie: r.get(26)?,
    })
}

/// Abonnement d'un compte (Gratuit s'il n'a jamais rien souscrit).
pub fn lire(c: &Connection, compte: i64) -> rusqlite::Result<Abonnement> {
    Ok(c.query_row(&format!("SELECT {COLONNES} FROM abonnements_cloud WHERE compte = ?1"), [compte], ligne).optional()?.unwrap_or_else(|| Abonnement::gratuit(compte)))
}

fn texte(v: &str) -> Sql {
    Sql::Text(v.to_string())
}
fn entier(v: i64) -> Sql {
    Sql::Integer(v)
}
fn option_date(v: Option<u64>) -> Sql {
    v.map_or(Sql::Null, |t| Sql::Integer(t as i64))
}

/// Met à jour des colonnes de la ligne du compte (créée au besoin). Noms de colonnes : constantes du code.
pub fn maj(c: &Connection, compte: i64, champs: &[(&str, Sql)]) -> rusqlite::Result<()> {
    c.execute("INSERT OR IGNORE INTO abonnements_cloud (compte, maj_le) VALUES (?1, ?2)", params![compte, crate::maintenant() as i64])?;
    if champs.is_empty() {
        return Ok(());
    }
    let set: Vec<String> = champs.iter().map(|(n, _)| format!("{n} = ?")).collect();
    let mut v: Vec<Sql> = champs.iter().map(|(_, x)| x.clone()).collect();
    v.push(entier(crate::maintenant() as i64));
    v.push(entier(compte));
    c.execute(&format!("UPDATE abonnements_cloud SET {}, maj_le = ? WHERE compte = ?", set.join(", ")), rusqlite::params_from_iter(v))?;
    for (n, x) in champs {
        if let (&"subscription_code" | &"supplement_code", Sql::Text(code)) = (n, x) {
            c.execute("INSERT OR IGNORE INTO abonnements_historique (compte, code, cree_le) VALUES (?1, ?2, ?3)", params![compte, code, crate::maintenant() as i64])?;
        }
    }
    Ok(())
}

fn regler_brouillon(c: &Connection, compte: i64, f: impl FnOnce(&mut serde_json::Map<String, Value>)) -> rusqlite::Result<Value> {
    let mut b = lire(c, compte)?.brouillon;
    if !b.is_object() {
        b = json!({});
    }
    f(b.as_object_mut().expect("objet"));
    maj(c, compte, &[("brouillon", texte(&b.to_string()))])?;
    Ok(b)
}

pub fn compte_par_abonnement(c: &Connection, code: &str) -> rusqlite::Result<Option<i64>> {
    c.query_row("SELECT compte FROM abonnements_cloud WHERE subscription_code = ?1 OR supplement_code = ?1", [code], |r| r.get(0)).optional()
}

pub fn compte_par_tiers(c: &Connection, tier: &str) -> rusqlite::Result<Option<i64>> {
    c.query_row("SELECT compte FROM abonnements_cloud WHERE tier_code = ?1 ORDER BY maj_le DESC LIMIT 1", [tier], |r| r.get(0)).optional()
}

pub fn proprietaire_du_site(c: &Connection, site: &str) -> rusqlite::Result<Option<i64>> {
    c.query_row("SELECT compte FROM droits WHERE site = ?1 AND role = 'proprietaire'", [site], |r| r.get(0)).optional()
}

pub fn sites_possedes(c: &Connection, compte: i64) -> rusqlite::Result<Vec<String>> {
    c.prepare("SELECT site FROM droits WHERE compte = ?1 AND role = 'proprietaire' ORDER BY site")?.query_map([compte], |r| r.get(0))?.collect()
}

/// Sites et machines d'un compte (inventaires de ses sites).
pub fn parc(c: &Connection, compte: i64) -> rusqlite::Result<Parc> {
    let (sites, imprimantes, decoupeurs): (i64, i64, i64) = c.query_row(
        "SELECT COUNT(*), COALESCE(SUM(i.nb_imprimantes), 0), COALESCE(SUM(i.nb_decoupeurs), 0) FROM droits d LEFT JOIN inventaires i ON i.site = d.site WHERE d.compte = ?1 AND d.role = 'proprietaire'",
        [compte],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    Ok(Parc { sites: sites as usize, imprimantes: imprimantes as usize, decoupeurs: decoupeurs as usize })
}

/// Quota de sauvegarde d'un compte, en octets.
pub fn quota(c: &Connection, compte: i64) -> rusqlite::Result<u64> {
    Ok(lire(c, compte)?.offre.quota(parc(c, compte)?.machines()))
}

fn supplement_regle(c: &Connection, offre: Offre, cycle: Cycle) -> bool {
    offre.machines_forfait().is_some() && offres::code_plan(c, None, cycle).is_some()
}

fn deploye_le(c: &Connection) -> u64 {
    base::reglage(c, "offres_deployees_le").ok().flatten().and_then(|v| v.parse().ok()).unwrap_or(0)
}

/// Sites dont l'accès à distance doit être suspendu pour ce compte à l'instant `t`.
pub fn sites_suspendus_du_compte(c: &Connection, compte: i64, tous: &[sites::Site], t: u64) -> rusqlite::Result<Vec<String>> {
    let ab = lire(c, compte)?;
    let Some(depuis) = ab.depassement_depuis else { return Ok(vec![]) };
    if t < offres::date_suspension(depuis, deploye_le(c)) {
        return Ok(vec![]);
    }
    let p = parc(c, compte)?;
    let mut possedes: Vec<(u64, String)> = sites_possedes(c, compte)?.into_iter().map(|s| (tous.iter().find(|x| x.id == s).map_or(0, |x| x.cree_le), s)).collect();
    possedes.sort();
    let sup = supplement_regle(c, ab.offre, ab.cycle);
    // Machines en trop : tous les sites ; sites en trop seulement : les plus récents.
    let machines_en_trop = offres::depassement(ab.offre, &Parc { sites: p.sites.min(ab.offre.sites()), ..p }, sup).is_some();
    Ok(if machines_en_trop { possedes.into_iter().map(|(_, s)| s).collect() } else { possedes.into_iter().skip(ab.offre.sites()).map(|(_, s)| s).collect() })
}

// ————— Envois —————

fn compte(r: &Partage, id: i64) -> Option<Compte> {
    base::compte_par_id(&r.base(), id).ok().flatten()
}

/// Adresse des e-mails de facturation : celle du contact de facturation, sinon celle du compte.
fn destinataire_facturation(ab: &Abonnement, c: &Compte) -> String {
    let e = ab.texte_brouillon("email");
    if base::email_valide(&e) { e } else { c.email.clone() }
}

fn prevenir(r: &Partage, id: i64, courriel: Courriel, push: Option<Notification>) {
    match push {
        Some(n) => notifications::notifier(r, id, Some(courriel), n),
        None => r.envoyer(courriel),
    }
}

fn libelle_prix(offre: Offre, cycle: Cycle, sieges: i64) -> String {
    let n = if offre == Offre::Machine { sieges.max(1) } else { 1 };
    format!("{} par {}{}", offres::euros(offre.prix(cycle) * n), if cycle == Cycle::Annuel { "an" } else { "mois" }, if offre == Offre::Machine { format!(" ({n} machine{})", if n > 1 { "s" } else { "" }) } else { String::new() })
}

// ————— ERP : abonnement, sièges —————

fn texte_json(v: &Value) -> Option<String> {
    match v {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// Reporte un abonnement de l'ERP (`data` d'une réponse ou d'un webhook) sur le compte.
pub fn appliquer(c: &Connection, compte: i64, sub: &Value) -> rusqlite::Result<()> {
    let mut champs: Vec<(&str, Sql)> = Vec::new();
    if let Some(code) = texte_json(&sub["code"]) {
        champs.push(("subscription_code", texte(&code)));
    }
    if let Some(s) = sub["status"].as_str() {
        champs.push(("statut", texte(s)));
    }
    if let Some(n) = sub["seats"].as_i64() {
        champs.push(("seats", entier(n)));
    }
    if let Some(f) = sub.pointer("/period/current_end").and_then(Value::as_str).and_then(offres::date_iso) {
        champs.push(("periode_fin", entier(f as i64)));
    }
    if let Some(v) = sub.pointer("/period/trial_ends_at") {
        champs.push(("essai_fin", option_date(v.as_str().and_then(offres::date_iso))));
    }
    if let Some(plan) = sub.pointer("/plan/code").and_then(texte_json).or_else(|| texte_json(&sub["plan_code"]))
        && let Some((o, cy)) = offres::offre_du_plan(c, &plan)
    {
        champs.push(("offre", texte(o.code())));
        champs.push(("cycle", texte(cy.code())));
    }
    maj(c, compte, &champs)
}

/// Crée l'abonnement dans l'ERP d'après le brouillon (offre, cycle, codes) ; `paiement` :
/// `{provider: "stancer_sepa", mandate_code}` pour un mandat signé. Un abonnement précédent
/// (changement d'offre) est résilié aussitôt après. Bloquant.
pub fn creer_abonnement(r: &Partage, id: i64, paiement: Option<Value>) -> Result<(), String> {
    let (erp, ab, p, plan) = {
        let c = r.base();
        let ab = lire(&c, id).map_err(|e| e.to_string())?;
        let offre = Offre::depuis(&ab.texte_brouillon("offre")).filter(|o| o.payante()).ok_or("Choisissez d'abord une offre.")?;
        let cycle = Cycle::depuis(&ab.texte_brouillon("cycle")).unwrap_or(Cycle::Mensuel);
        let plan = offres::code_plan(&c, Some(offre), cycle).ok_or("Cette offre n'est pas encore ouverte à la souscription.")?;
        (Erp::depuis(&c).ok_or("Facturation indisponible pour le moment.")?, ab, parc(&c, id).map_err(|e| e.to_string())?, (offre, cycle, plan))
    };
    let (offre, cycle, plan) = plan;
    let tier = ab.tier_code.clone().ok_or("Renseignez d'abord les informations de facturation.")?;
    let (principal, _) = offres::sieges(offre, &p);
    let mut corps = json!({ "tier_code": tier, "plan_code": plan, "customer_notifications": false, "bill_now": true });
    if let Some(n) = principal {
        corps["seats"] = json!(n);
    }
    for (cle, champ) in [("coupon", "coupon_code"), ("partenaire", "partner_code")] {
        let v = ab.texte_brouillon(cle);
        if !v.is_empty() {
            corps[champ] = json!(v);
        }
    }
    if let Some(pm) = &paiement {
        corps["payment_method"] = pm.clone();
    }
    let cle = format!("cloud-{id}-{plan}-{}", ab.texte_brouillon("cle"));
    let rep = erp.post("/subscriptions", &corps, Some(&cle))?;
    if !rep.ok() {
        eprintln!("abonnement du compte {id} refusé par l'ERP (HTTP {}) : {}", rep.statut, rep.erreur());
        return Err(format!("L'abonnement n'a pas pu être créé : {}", rep.erreur()));
    }
    let sub = rep.donnees().clone();
    // `bill_now` : la première facture est émise tout de suite (absente pendant un essai).
    let premiere_facture = sub.pointer("/first_cycle/invoice_code").and_then(texte_json);
    let ancien = ab.subscription_code.clone().filter(|_| ab.actif());
    let ancien_sup = ab.supplement_code.clone();
    {
        let c = r.base();
        appliquer(&c, id, &sub).map_err(|e| e.to_string())?;
        maj(&c, id, &[("offre", texte(offre.code())), ("cycle", texte(cycle.code())), ("bon_commande", texte(&ab.texte_brouillon("bon_commande"))), ("gratuit_depuis", Sql::Null), ("relance", entier(0)), ("fin_periode_resiliation", entier(0)), ("supplement_code", Sql::Null), ("seats_supplement", entier(0)), ("supplement_resilie", entier(0))]).map_err(|e| e.to_string())?;
    }
    eprintln!("abonnement {} créé pour le compte {id} ({} {}, première facture : {})", texte_json(&sub["code"]).unwrap_or_default(), offre.code(), cycle.code(), premiere_facture.as_deref().unwrap_or("aucune"));
    // Changement d'offre : l'ancien abonnement (et son supplément) s'arrête.
    for code in ancien.iter().chain(ancien_sup.iter()) {
        if Some(code) == texte_json(&sub["code"]).as_ref() {
            continue;
        }
        match erp.post(&format!("/subscriptions/{}/cancel", encoder(code)), &json!({ "at_period_end": false, "reason": "Changement d'offre E-KO RIP Cloud" }), None) {
            Ok(rep) if rep.ok() => eprintln!("abonnement {code} résilié (changement d'offre du compte {id})"),
            Ok(rep) => eprintln!("résiliation de l'abonnement {code} refusée : {}", rep.erreur()),
            Err(e) => eprintln!("résiliation de l'abonnement {code} : {e}"),
        }
    }
    if let Some(cp) = compte(r, id) {
        let ab2 = lire(&r.base(), id).map_err(|e| e.to_string())?;
        r.envoyer(courriels::abonnement_souscrit(&r.config.url_publique, &destinataire_facturation(&ab2, &cp), &cp.nom, offre.nom(), &libelle_prix(offre, cycle, ab2.seats), ab2.periode_fin, ab2.essai_fin, ancien.is_some(), premiere_facture.as_deref()));
    }
    Ok(())
}

/// Envoie à l'ERP les sièges facturés s'ils ont changé (au plus une fois par heure). Bloquant.
pub fn recalculer_sieges(r: &Partage, id: i64, t: u64) -> Result<bool, String> {
    let (erp, ab, p, plan_sup) = {
        let c = r.base();
        let ab = lire(&c, id).map_err(|e| e.to_string())?;
        let plan_sup = if ab.offre.machines_forfait().is_some() { offres::code_plan(&c, None, ab.cycle) } else { None };
        (Erp::depuis(&c), ab, parc(&c, id).map_err(|e| e.to_string())?, plan_sup)
    };
    let Some(code) = ab.subscription_code.clone().filter(|_| ab.actif()) else { return Ok(false) };
    let (principal, sup) = offres::sieges(ab.offre, &p);
    let change_principal = principal.filter(|n| *n != ab.seats);
    let change_sup = plan_sup.as_ref().filter(|_| sup != ab.seats_supplement).map(|_| sup);
    if change_principal.is_none() && change_sup.is_none() {
        return Ok(false);
    }
    if t < ab.seats_envoyes_le + INTERVALLE_SIEGES {
        return Ok(false);
    }
    let erp = erp.ok_or("ERP non réglé")?;
    let _ = maj(&r.base(), id, &[("seats_envoyes_le", entier(t as i64))]);
    if let Some(n) = change_principal {
        let rep = erp.envoyer("PATCH", &format!("/subscriptions/{}", encoder(&code)), &json!({ "seats": n }), None)?;
        if rep.ok() {
            eprintln!("sièges du compte {id} : {} → {n} (abonnement {code})", ab.seats);
            let _ = maj(&r.base(), id, &[("seats", entier(n))]);
        } else {
            eprintln!("sièges du compte {id} refusés par l'ERP : {}", rep.erreur());
        }
    }
    if let (Some(n), Some(plan)) = (change_sup, plan_sup) {
        let rep = match (&ab.supplement_code, n) {
            (None, 0) => None,
            (None, n) => {
                let mut corps = json!({ "tier_code": ab.tier_code, "plan_code": plan, "seats": n, "customer_notifications": false, "bill_now": true });
                if let Some(rum) = &ab.rum {
                    corps["payment_method"] = json!({ "provider": "stancer_sepa", "mandate_code": rum });
                }
                let rep = erp.post("/subscriptions", &corps, Some(&format!("cloud-{id}-supplement-{t}")))?;
                if rep.ok()
                    && let Some(sc) = texte_json(&rep.donnees()["code"])
                {
                    let _ = maj(&r.base(), id, &[("supplement_code", texte(&sc))]);
                    if ab.rum.is_none()
                        && let Some(carte) = &ab.carte_id
                    {
                        let _ = erp.envoyer("PUT", &format!("/subscriptions/{}/payment-method", encoder(&sc)), &json!({ "provider": "stancer_card", "card_id": carte }), None);
                    }
                }
                Some(rep)
            }
            // Plus de machine en trop : résiliation à l'échéance (la période est payée). La
            // référence reste jusqu'au webhook `subscription.cancelled` : des machines revenues
            // d'ici là sont reportées sur ce supplément (PATCH), jamais sur un second.
            (Some(_), 0) if ab.supplement_resilie => None,
            (Some(sc), 0) => {
                let rep = erp.post(&format!("/subscriptions/{}/cancel", encoder(sc)), &json!({ "at_period_end": true, "reason": "Plus de machine au-delà du forfait" }), None)?;
                if rep.ok() {
                    let _ = maj(&r.base(), id, &[("supplement_resilie", entier(1))]);
                }
                Some(rep)
            }
            // L'API n'a pas de reprise d'une résiliation programmée : le supplément en attente
            // garde ses sièges jusqu'à l'échéance, puis le webhook de fin en recrée un si
            // des machines dépassent encore le forfait.
            (Some(sc), n) => Some(erp.envoyer("PATCH", &format!("/subscriptions/{}", encoder(sc)), &json!({ "seats": n }), None)?),
        };
        match rep {
            Some(rep) if !rep.ok() => eprintln!("machines supplémentaires du compte {id} refusées par l'ERP : {}", rep.erreur()),
            _ => {
                eprintln!("machines supplémentaires du compte {id} : {} → {n}", ab.seats_supplement);
                let _ = maj(&r.base(), id, &[("seats_supplement", entier(n))]);
            }
        }
    }
    Ok(true)
}

/// Dépassement d'offre d'un compte : début (e-mail « suspendu dans 14 jours »), fin du délai
/// de grâce (e-mail « suspendu »), ou retour dans l'offre.
pub fn verifier_depassement(r: &Partage, id: i64, t: u64) {
    let (ab, raison, deploye) = {
        let c = r.base();
        let Ok(ab) = lire(&c, id) else { return };
        let Ok(p) = parc(&c, id) else { return };
        let raison = offres::depassement(ab.offre, &p, supplement_regle(&c, ab.offre, ab.cycle));
        (ab, raison, deploye_le(&c))
    };
    let url = &r.config.url_publique;
    match (raison, ab.depassement_depuis) {
        (Some(raison), None) => {
            let _ = maj(&r.base(), id, &[("depassement_depuis", entier(t as i64)), ("depassement_raison", texte(&raison)), ("suspension_prevenue", entier(0))]);
            let quand = offres::date_suspension(t, deploye);
            eprintln!("compte {id} : offre dépassée ({raison}), suspension le {quand}");
            if let Some(cp) = compte(r, id) {
                let c = courriels::offre_depassee(url, &cp.email, &cp.nom, &raison, quand, false);
                prevenir(r, id, c, Some(Notification::autre(notifications::DEPASSEMENT, None, "Offre dépassée", &format!("{raison}. Accès à distance suspendu le {} sans changement.", crate::temps::date_fr(quand)), &format!("{url}/abonnement"))));
            }
        }
        (Some(raison), Some(depuis)) => {
            if raison != ab.depassement_raison {
                let _ = maj(&r.base(), id, &[("depassement_raison", texte(&raison))]);
            }
            let quand = offres::date_suspension(depuis, deploye);
            if t >= quand && !ab.suspension_prevenue {
                let _ = maj(&r.base(), id, &[("suspension_prevenue", entier(1))]);
                eprintln!("compte {id} : accès à distance suspendu (offre dépassée)");
                if let Some(cp) = compte(r, id) {
                    let c = courriels::offre_depassee(url, &cp.email, &cp.nom, &raison, quand, true);
                    prevenir(r, id, c, Some(Notification::autre(notifications::DEPASSEMENT, None, "Accès à distance suspendu", &format!("{raison}. L'usage dans l'atelier n'est pas touché."), &format!("{url}/abonnement"))));
                }
            }
        }
        (None, Some(_)) => {
            eprintln!("compte {id} : de nouveau dans son offre");
            let _ = maj(&r.base(), id, &[("depassement_depuis", Sql::Null), ("depassement_raison", texte("")), ("suspension_prevenue", entier(0))]);
        }
        (None, None) => {}
    }
}

/// Chaque minute : dépassements, sites suspendus, rappels de fin d'essai, sièges, purge.
pub async fn entretenir(r: &Partage) {
    let r2 = r.clone();
    let _ = tokio::task::spawn_blocking(move || entretenir_bloquant(&r2, crate::maintenant())).await;
}

/// Corps de [`entretenir`], à l'instant `t` (appelable dans les tests).
pub fn entretenir_bloquant(r: &Partage, t: u64) {
    let comptes: Vec<i64> = {
        let c = r.base();
        let _ = c.execute("DELETE FROM webhooks_erp WHERE recu_le < ?1", [t.saturating_sub(CONSERVATION_WEBHOOKS) as i64]);
        c.prepare("SELECT compte FROM droits WHERE role = 'proprietaire' UNION SELECT compte FROM abonnements_cloud").and_then(|mut q| q.query_map([], |l| l.get(0))?.collect()).unwrap_or_default()
    };
    for &id in &comptes {
        verifier_depassement(r, id, t);
    }
    // Sites suspendus : mémoire du relais et table (relue au démarrage).
    let tous = sites::lire(&r.config.donnees).unwrap_or_default();
    let mut suspendus = HashSet::new();
    {
        let c = r.base();
        for &id in &comptes {
            suspendus.extend(sites_suspendus_du_compte(&c, id, &tous, t).unwrap_or_default());
        }
        if let Ok(tx) = c.unchecked_transaction() {
            let _ = tx.execute("DELETE FROM sites_suspendus WHERE site NOT IN (SELECT value FROM json_each(?1))", [serde_json::to_string(&suspendus).unwrap_or_default()]);
            for s in &suspendus {
                let _ = tx.execute("INSERT OR IGNORE INTO sites_suspendus (site, depuis) VALUES (?1, ?2)", params![s, t as i64]);
            }
            let _ = tx.commit();
        }
    }
    r.regler_suspendus(suspendus);
    // Fin d'essai dans 3 jours, et sièges.
    for &id in &comptes {
        let Ok(ab) = lire(&r.base(), id) else { continue };
        if let Some(fin) = ab.essai_fin
            && ab.actif()
            && !ab.essai_prevenu
            && t + RAPPEL_ESSAI >= fin
            && t < fin
        {
            let _ = maj(&r.base(), id, &[("essai_prevenu", entier(1))]);
            if let Some(cp) = compte(r, id) {
                r.envoyer(courriels::fin_essai(&r.config.url_publique, &destinataire_facturation(&ab, &cp), &cp.nom, ab.offre.nom(), fin, &ab.moyen_paiement));
            }
        }
        if ab.actif()
            && let Err(e) = recalculer_sieges(r, id, t)
        {
            eprintln!("sièges du compte {id} : {e}");
        }
    }
}

// ————— Webhooks de l'ERP —————

fn premier_texte(data: &Value, chemins: &[&str]) -> Option<String> {
    chemins.iter().find_map(|p| data.pointer(p).and_then(texte_json))
}

/// `POST /api/erp/webhooks` : événements de l'ERP, signés (`X-EKO-Signature`).
pub async fn webhook(State(r): State<Partage>, h: HeaderMap, corps: Bytes) -> Response {
    let Some(secret) = base::reglage(&r.base(), "erp_webhook_secret").ok().flatten() else { return portail::reponse_json(StatusCode::SERVICE_UNAVAILABLE, "webhooks_non_regles") };
    let signature = h.get("x-eko-signature").and_then(|v| v.to_str().ok()).unwrap_or("");
    if let Err(code) = erp::verifier_signature(&secret, &corps, signature, crate::maintenant()) {
        eprintln!("webhook de l'ERP refusé : {code}");
        return portail::reponse_json(StatusCode::UNAUTHORIZED, code);
    }
    let Ok(m) = serde_json::from_slice::<Value>(&corps) else { return portail::reponse_json(StatusCode::BAD_REQUEST, "json_invalide") };
    let evenement = m["event"].as_str().unwrap_or("").to_string();
    let empreinte = crate::sha256_octets_hex(&corps);
    let neuf = r.base().execute("INSERT OR IGNORE INTO webhooks_erp (empreinte, evenement, recu_le) VALUES (?1, ?2, ?3)", params![empreinte, evenement, crate::maintenant() as i64]).map(|n| n == 1).unwrap_or(false);
    if !neuf {
        return Json(json!({ "code": "deja_traite" })).into_response();
    }
    let r2 = r.clone();
    let traite = tokio::task::spawn_blocking(move || traiter_webhook(&r2, &evenement, &m["data"])).await.unwrap_or(false);
    Json(json!({ "code": if traite { "traite" } else { "ignore" } })).into_response()
}

/// Rend vrai si l'événement concernait un compte connu.
pub fn traiter_webhook(r: &Partage, evenement: &str, data: &Value) -> bool {
    let code_sub = premier_texte(data, &["/subscription/code", "/subscription_code", "/cycle/subscription_code", "/cycle/subscription/code"]);
    let tier = premier_texte(data, &["/tier/code", "/tier_code", "/subscription/tier_code", "/mandate/tier_code", "/tier"]);
    let id = {
        let c = r.base();
        code_sub.as_deref().and_then(|s| compte_par_abonnement(&c, s).ok().flatten()).or_else(|| tier.as_deref().and_then(|t| compte_par_tiers(&c, t).ok().flatten()))
    };
    let Some(id) = id else {
        eprintln!("webhook {evenement} : aucun compte ne correspond");
        return false;
    };
    let Ok(ab) = lire(&r.base(), id) else { return false };
    let Some(cp) = compte(r, id) else { return false };
    let url = r.config.url_publique.clone();
    let a = destinataire_facturation(&ab, &cp);
    let du_supplement = code_sub.is_some() && code_sub == ab.supplement_code;
    let du_principal = code_sub.is_none() || code_sub == ab.subscription_code;
    eprintln!("webhook {evenement} : compte {id}");
    match evenement {
        "subscription.created" | "subscription.paused" | "subscription.resumed" if du_principal => {
            if let Some(sub) = data.get("subscription") {
                let _ = appliquer(&r.base(), id, sub);
            }
        }
        "subscription.cancel_scheduled" if du_principal => {
            let _ = maj(&r.base(), id, &[("fin_periode_resiliation", entier(1))]);
        }
        "subscription.cancelled" if du_supplement => {
            let _ = maj(&r.base(), id, &[("supplement_code", Sql::Null), ("seats_supplement", entier(0)), ("supplement_resilie", entier(0)), ("seats_envoyes_le", entier(0))]);
            // Des machines sont revenues pendant le préavis : nouveau supplément maintenant.
            if let Err(e) = recalculer_sieges(r, id, crate::maintenant()) {
                eprintln!("sièges du compte {id} : {e}");
            }
        }
        "subscription.cancelled" if du_principal && code_sub.is_some() => {
            let impaye = ab.statut == "past_due" || ab.relance > 0;
            retour_au_gratuit(r, id);
            prevenir(r, id, courriels::retour_gratuit(&url, &a, &cp.nom, impaye), Some(Notification::autre(notifications::PAIEMENT, None, "Retour à l'offre Gratuite", "Votre abonnement est terminé : votre compte revient à l'offre Gratuite.", &format!("{url}/abonnement"))));
        }
        "dunning.exhausted" => {
            retour_au_gratuit(r, id);
            prevenir(r, id, courriels::retour_gratuit(&url, &a, &cp.nom, true), Some(Notification::autre(notifications::PAIEMENT, None, "Abonnement résilié pour impayé", "Votre compte revient à l'offre Gratuite.", &format!("{url}/abonnement"))));
        }
        "cycle.billed" => {
            let code = premier_texte(data, &["/invoice/code", "/cycle/invoice/code", "/invoice_code"]);
            let reference = premier_texte(data, &["/invoice/reference", "/cycle/invoice/reference", "/invoice_reference"]).or(code.clone()).unwrap_or_default();
            let montant = ["/invoice/total_ttc", "/cycle/total_ttc", "/total_ttc"].iter().find_map(|p| data.pointer(p).and_then(offres::centimes));
            if let Some(code) = code {
                let lien = format!("{url}/abonnement/factures/{}.pdf", encoder(&code));
                r.envoyer(courriels::facture_emise(&url, &a, &cp.nom, &reference, &montant.map(offres::euros).unwrap_or_else(|| "—".into()), &lien));
            }
            if let Some(f) = premier_texte(data, &["/cycle/period_end", "/period_end"]).and_then(|d| offres::date_iso(&d)) {
                let _ = maj(&r.base(), id, &[("periode_fin", entier(f as i64))]);
            }
        }
        "cycle.paid" => {
            let mut champs = vec![("relance", entier(0))];
            if du_principal {
                champs.push(("statut", texte("active")));
            }
            let _ = maj(&r.base(), id, &champs);
        }
        "cycle.failed" => {
            let _ = maj(&r.base(), id, &[("statut", texte("past_due"))]);
            let montant = ["/cycle/total_ttc", "/total_ttc", "/invoice/total_ttc"].iter().find_map(|p| data.pointer(p).and_then(offres::centimes)).map(offres::euros);
            let raison = premier_texte(data, &["/cycle/failure_reason", "/failure_reason"]);
            prevenir(r, id, courriels::paiement_refuse(&url, &a, &cp.nom, 0, montant.as_deref(), raison.as_deref()), Some(Notification::autre(notifications::PAIEMENT, None, "Paiement refusé", "Le paiement de votre abonnement E-KO RIP Cloud a été refusé.", &format!("{url}/abonnement/paiement"))));
        }
        "dunning.reminder_sent" => {
            let etape = data.get("step").or_else(|| data.pointer("/dunning/step")).and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))).unwrap_or(1).clamp(1, 3);
            let _ = maj(&r.base(), id, &[("relance", entier(etape)), ("statut", texte("past_due"))]);
            let montant = ["/cycle/total_ttc", "/total_ttc", "/invoice/total_ttc"].iter().find_map(|p| data.pointer(p).and_then(offres::centimes)).map(offres::euros);
            let (sujet, titre, _) = courriels::etape_relance(etape as u8);
            prevenir(r, id, courriels::paiement_refuse(&url, &a, &cp.nom, etape as u8, montant.as_deref(), None), Some(Notification::autre(notifications::PAIEMENT, None, titre, sujet, &format!("{url}/abonnement/paiement"))));
        }
        "sepa_mandate.signed" => {
            let rum = premier_texte(data, &["/mandate/rum", "/rum", "/sepa_mandate/rum"]);
            if rum.is_some() && (ab.rum != rum || !ab.rum_prevenu) {
                let rum = rum.unwrap_or_default();
                let fin = premier_texte(data, &["/mandate/iban_last4", "/iban_last4", "/sepa_mandate/iban_last4"]).unwrap_or_default();
                let _ = maj(&r.base(), id, &[("rum", texte(&rum)), ("rum_prevenu", entier(1))]);
                r.envoyer(courriels::mandat_signe(&url, &a, &cp.nom, &rum, &fin, &format!("{url}/abonnement/mandat.pdf")));
            }
        }
        _ => {
            eprintln!("webhook {evenement} ignoré");
            return false;
        }
    }
    true
}

/// L'abonnement s'arrête : offre Gratuite (les sauvegardes payantes restent 90 jours).
pub fn retour_au_gratuit(r: &Partage, id: i64) {
    let _ = maj(&r.base(), id, &[("offre", texte(Offre::Gratuit.code())), ("statut", texte("cancelled")), ("subscription_code", Sql::Null), ("supplement_code", Sql::Null), ("supplement_resilie", entier(0)), ("seats", entier(0)), ("seats_supplement", entier(0)), ("gratuit_depuis", entier(crate::maintenant() as i64)), ("fin_periode_resiliation", entier(0)), ("relance", entier(0)), ("periode_fin", Sql::Null), ("essai_fin", Sql::Null)]);
    eprintln!("compte {id} : retour à l'offre Gratuite");
}

// ————— Pages : abonnement —————

fn reponse_bloquante<T: Send + 'static>(r: &Partage, f: impl FnOnce(&Partage) -> T + Send + 'static) -> tokio::task::JoinHandle<T> {
    let r = r.clone();
    tokio::task::spawn_blocking(move || f(&r))
}

/// Bandeau « offre dépassée » (Mes sites, Abonnement).
pub fn bandeau_depassement(r: &Partage, id: i64) -> String {
    let (ab, deploye) = {
        let c = r.base();
        (lire(&c, id).unwrap_or_else(|_| Abonnement::gratuit(id)), deploye_le(&c))
    };
    let Some(depuis) = ab.depassement_depuis else { return String::new() };
    let quand = offres::date_suspension(depuis, deploye);
    let suspendu = crate::maintenant() >= quand;
    format!(
        r#"<div class="carte"><p class="erreur"><strong>{}</strong> {} — {}.</p><p><a class="bouton" href="/abonnement/offres">Choisir une offre</a></p></div>"#,
        if suspendu { "Accès à distance suspendu — offre dépassée." } else { "Offre dépassée." },
        echapper(&ab.depassement_raison),
        if suspendu { "l'usage dans l'atelier n'est pas touché ; changez d'offre ou retirez des machines pour rétablir l'accès".to_string() } else { format!("l'accès à distance des sites concernés sera suspendu le {} (l'usage dans l'atelier n'est jamais touché)", crate::temps::date_fr(quand)) }
    )
}

fn liste_machines(json: &str) -> String {
    let v: Vec<Value> = serde_json::from_str(json).unwrap_or_default();
    if v.is_empty() {
        return "—".into();
    }
    v.iter().map(|m| { let nom = m["nom"].as_str().unwrap_or(""); let modele = m["modele"].as_str().unwrap_or(""); echapper(&if modele.is_empty() || modele == nom { nom.to_string() } else { format!("{nom} ({modele})") }) }).collect::<Vec<_>>().join("<br>")
}

fn libelle_moyen(ab: &Abonnement) -> String {
    if ab.moyen_paiement.is_empty() { "à enregistrer".into() } else { ab.moyen_paiement.clone() }
}

/// `GET /abonnement`.
pub async fn page_abonnement(State(r): State<Partage>, h: HeaderMap, Query(q): Query<HashMap<String, String>>) -> Response {
    let s = match exiger(&r, &h, "/abonnement") {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    let id = s.compte.id;
    let (ab, p, utilise, quota, lignes_sites, erp_ok) = {
        let c = r.base();
        let ab = lire(&c, id).unwrap_or_else(|_| Abonnement::gratuit(id));
        let p = parc(&c, id).unwrap_or_default();
        let utilise = crate::sauvegardes::octets_du_compte(&c, id).unwrap_or(0);
        let tous = sites::lire(&r.config.donnees).unwrap_or_default();
        let mut lignes = String::new();
        for site in sites_possedes(&c, id).unwrap_or_default() {
            let nom = tous.iter().find(|x| x.id == site).map_or(site.clone(), |x| x.nom.clone());
            let inv: Option<(String, String, i64)> = c.query_row("SELECT imprimantes, decoupeurs, recu_le FROM inventaires WHERE site = ?1", [&site], |l| Ok((l.get(0)?, l.get(1)?, l.get(2)?))).optional().ok().flatten();
            let (imp, dec, recu) = match &inv {
                Some((i, d, t)) => (liste_machines(i), liste_machines(d), date(Some(*t as u64))),
                None => ("—".into(), "—".into(), "jamais (serveur RIP à mettre à jour)".into()),
            };
            lignes.push_str(&format!("<tr><td><strong>{}</strong></td><td>{imp}</td><td>{dec}</td><td>{recu}</td></tr>", echapper(&nom)));
        }
        (ab.clone(), p, utilise, ab.offre.quota(p.machines()), lignes, Erp::depuis(&c).is_some())
    };
    let message = match q.get("ok").map(String::as_str) {
        Some("carte") => r#"<p class="ok">Carte enregistrée : votre abonnement est actif.</p>"#,
        Some("sepa") => r#"<p class="ok">Mandat SEPA signé : votre abonnement est actif.</p>"#,
        Some("resilie") => r#"<p class="ok">Résiliation enregistrée : l'abonnement s'arrêtera à la fin de la période en cours.</p>"#,
        Some("change") => r#"<p class="ok">Offre modifiée.</p>"#,
        _ => "",
    };
    let retard = if ab.statut == "past_due" {
        let (_, titre, texte) = courriels::etape_relance(ab.relance as u8);
        format!(r#"<div class="carte"><p class="erreur"><strong>{titre}{}.</strong> {texte}</p><p><a class="bouton" href="/abonnement/paiement">Mettre à jour le moyen de paiement</a></p></div>"#, if ab.relance > 0 { format!(" — relance {} sur 3", ab.relance) } else { String::new() })
    } else {
        String::new()
    };
    let mut faits = format!("<p><strong>Offre {}</strong>{}</p>", ab.offre.nom(), if ab.actif() { format!(" · {}", offres::libelle_statut(&ab.statut)) } else { String::new() });
    if ab.actif() {
        faits.push_str(&format!("<p>{} · facturation {}</p>", libelle_prix(ab.offre, ab.cycle, ab.seats), ab.cycle.libelle()));
        if let Some(e) = ab.essai_fin.filter(|e| *e > crate::maintenant()) {
            faits.push_str(&format!("<p>Essai gratuit jusqu'au {}</p>", crate::temps::date_fr(e)));
        }
        if let Some(f) = ab.periode_fin {
            faits.push_str(&format!("<p>{} le {}</p>", if ab.fin_periode_resiliation { "Fin de l'abonnement" } else { "Prochaine échéance" }, crate::temps::date_fr(f)));
        }
        faits.push_str(&format!("<p>Moyen de paiement : {}</p>", echapper(&libelle_moyen(&ab))));
        if ab.seats_supplement > 0 {
            faits.push_str(&format!("<p>Machines au-delà du forfait : {} × {} HT par mois</p>", ab.seats_supplement, offres::euros(offres::PRIX_SUPPLEMENT)));
        }
        if !ab.bon_commande.is_empty() {
            faits.push_str(&format!(r#"<p class="meta">Bon de commande : {}</p>"#, echapper(&ab.bon_commande)));
        }
    } else {
        faits.push_str(r#"<p class="meta">Offre gratuite, pour toujours : 1 site, 1 imprimante et 1 découpeur, sauvegarde en ligne de la dernière version (1 Go).</p>"#);
    }
    let autre_cycle = if ab.cycle == Cycle::Mensuel { Cycle::Annuel } else { Cycle::Mensuel };
    let actions = if ab.actif() {
        format!(
            r#"<p class="ligne"><a class="bouton" href="/abonnement/offres?cycle={cy}">Changer d'offre</a><a class="bouton discret" href="/abonnement/paiement">Changer de moyen de paiement</a></p>
<form method="post" action="/abonnement/changer" class="ligne">{csrf}<input type="hidden" name="offre" value="{o}"><input type="hidden" name="cycle" value="{ac}"><button class="discret" type="submit">Passer en facturation {acl}</button> <span class="meta">{prix}</span></form>
{resilier}"#,
            cy = ab.cycle.code(),
            csrf = hidden_csrf(&s),
            o = ab.offre.code(),
            ac = autre_cycle.code(),
            acl = autre_cycle.libelle(),
            prix = echapper(&libelle_prix(ab.offre, autre_cycle, ab.seats)),
            resilier = if ab.fin_periode_resiliation { r#"<p class="meta">Résiliation programmée à la fin de la période.</p>"#.to_string() } else { format!(r#"<form method="post" action="/abonnement/resilier">{}<label class="case"><input type="checkbox" name="confirmer" value="1" required> <span>Je résilie à la fin de la période en cours (pas de remboursement de la période entamée ; retour à l'offre Gratuite ensuite).</span></label><button class="danger" type="submit">Résilier</button></form>"#, hidden_csrf(&s)) },
        )
    } else {
        r#"<p class="ligne"><a class="bouton" href="/abonnement/offres">Voir les offres</a></p>"#.to_string()
    };
    // Factures aussi après un retour au Gratuit : celles des anciens abonnements restent.
    let a_eu_des_abonnements = historique(&r.base(), id).map(|h| !h.is_empty()).unwrap_or(false);
    let factures = if (ab.actif() || a_eu_des_abonnements) && erp_ok {
        let liste = reponse_bloquante(&r, move |r| factures(r, id)).await.unwrap_or_else(|_| Err(String::new()));
        match liste {
            Ok(v) if v.is_empty() => r#"<div class="carte"><h2>Factures</h2><p class="meta">Aucune facture pour l'instant.</p></div>"#.to_string(),
            Ok(v) => {
                let lignes: String = v
                    .iter()
                    .map(|c| {
                        let pdf = c.pointer("/invoice/code").and_then(texte_json).map_or("—".to_string(), |code| format!(r#"<a href="/abonnement/factures/{}.pdf">{}</a>"#, encoder(&code), echapper(&c.pointer("/invoice/reference").and_then(texte_json).unwrap_or(code.clone()))));
                        format!(
                            "<tr><td>{} → {}</td><td>{}</td><td>{}</td><td>{pdf}</td></tr>",
                            c["period_start"].as_str().and_then(offres::date_iso).map_or("—".into(), crate::temps::date_fr),
                            c["period_end"].as_str().and_then(offres::date_iso).map_or("—".into(), crate::temps::date_fr),
                            echapper(&statut_cycle(c["status"].as_str().unwrap_or(""))),
                            c.get("total_ttc").and_then(offres::centimes).map_or("—".into(), |m| format!("{} TTC", offres::euros(m))),
                        )
                    })
                    .collect();
                format!(r#"<div class="carte"><h2>Factures</h2><div class="tableau"><table><thead><tr><th>Période</th><th>État</th><th>Montant</th><th>Facture</th></tr></thead><tbody>{lignes}</tbody></table></div></div>"#)
            }
            Err(_) => r#"<div class="carte"><h2>Factures</h2><p class="meta">Factures momentanément indisponibles.</p></div>"#.to_string(),
        }
    } else {
        String::new()
    };
    let machines = match ab.offre.machines_forfait() {
        Some(f) => format!("{} / {f}", p.machines()),
        None if ab.offre == Offre::Gratuit => format!("{} imprimante(s) / 1 · {} découpeur(s) / 1", p.imprimantes, p.decoupeurs),
        None => format!("{} (dont {} facturée(s))", p.machines(), p.au_dela_du_gratuit()),
    };
    let corps = format!(
        r#"<h1>Abonnement</h1>{message}{retard}{depassement}<div class="carte"><h2>Votre offre</h2>{faits}{actions}<p class="meta">Besoin de plus de sites ou d'un accompagnement ? Offre Entreprise sur devis : <a href="mailto:{contact}?subject=Offre%20Entreprise%20E-KO%20RIP%20Cloud">{contact}</a></p></div>
<div class="carte"><h2>Utilisation</h2><p>Sites : <strong>{ns} / {ms}</strong> · Machines : <strong>{machines}</strong> · Sauvegardes : <strong>{ut} / {qu}</strong> ({cons})</p><div class="tableau"><table><thead><tr><th>Site</th><th>Imprimantes</th><th>Découpeurs</th><th>Inventaire reçu</th></tr></thead><tbody>{lignes_sites}</tbody></table></div></div>{factures}"#,
        depassement = bandeau_depassement(&r, id),
        contact = offres::CONTACT,
        ns = p.sites,
        ms = ab.offre.sites(),
        ut = offres::taille(utilise),
        qu = offres::taille(quota),
        cons = ab.offre.libelle_conservation(),
    );
    page("Abonnement", Some(&s), &corps).into_response()
}

fn statut_cycle(s: &str) -> String {
    match s {
        "paid" => "Payée".into(),
        "billed" | "pending" | "open" => "Émise".into(),
        "failed" => "Paiement refusé".into(),
        "void" | "cancelled" => "Annulée".into(),
        "" => "—".into(),
        autre => autre.to_string(),
    }
}

/// Codes de tous les abonnements ERP qu'a eus le compte, du plus ancien au plus récent.
pub fn historique(c: &Connection, compte: i64) -> rusqlite::Result<Vec<String>> {
    c.prepare("SELECT code FROM abonnements_historique WHERE compte = ?1 ORDER BY cree_le, rowid")?.query_map([compte], |l| l.get(0))?.collect()
}

/// Cycles de facturation de tous les abonnements du compte, anciens compris. Bloquant.
pub fn factures(r: &Partage, id: i64) -> Result<Vec<Value>, String> {
    let (erp, ab) = {
        let c = r.base();
        (Erp::depuis(&c).ok_or("ERP non réglé")?, lire(&c, id).map_err(|e| e.to_string())?)
    };
    let mut codes = historique(&r.base(), id).map_err(|e| e.to_string())?;
    for code in ab.subscription_code.iter().chain(ab.supplement_code.iter()) {
        if !codes.contains(code) {
            codes.push(code.clone());
        }
    }
    let mut v = Vec::new();
    for code in &codes {
        let rep = erp.get(&format!("/subscriptions/{}/billing-cycles", encoder(code)))?;
        if !rep.ok() {
            // Un ancien abonnement illisible ne masque pas les factures des autres.
            if Some(code) == ab.subscription_code.as_ref() {
                return Err(rep.erreur());
            }
            eprintln!("factures de l'abonnement {code} (compte {id}) : {}", rep.erreur());
            continue;
        }
        v.extend(rep.donnees().as_array().cloned().unwrap_or_default());
    }
    v.sort_by(|a, b| b["period_start"].as_str().unwrap_or("").cmp(a["period_start"].as_str().unwrap_or("")));
    Ok(v)
}

fn pdf(octets: Vec<u8>, nom: &str) -> Response {
    let mut rep = ([(header::CONTENT_TYPE, "application/pdf")], octets).into_response();
    let h = rep.headers_mut();
    h.insert(header::CONTENT_DISPOSITION, HeaderValue::from_str(&format!("inline; filename=\"{nom}\"")).unwrap_or(HeaderValue::from_static("inline")));
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("private, no-store"));
    // Affichable dans un cadre de la plate-forme (mandat SEPA dans la page de paiement).
    h.insert(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static("default-src 'none'; frame-ancestors 'self'; sandbox"));
    h.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("SAMEORIGIN"));
    h.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    rep
}

/// `GET /abonnement/factures/{code}.pdf` : PDF de l'ERP, seulement pour une facture du compte.
pub async fn facture_pdf(State(r): State<Partage>, h: HeaderMap, Chemin(fichier): Chemin<String>) -> Response {
    let s = match exiger(&r, &h, "/abonnement") {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    let code = fichier.strip_suffix(".pdf").unwrap_or(&fichier).to_string();
    let id = s.compte.id;
    let resultat = reponse_bloquante(&r, move |r| -> Result<Vec<u8>, String> {
        let liste = factures(r, id)?;
        if !liste.iter().any(|c| c.pointer("/invoice/code").and_then(texte_json).as_deref() == Some(code.as_str())) {
            return Err("introuvable".into());
        }
        let erp = Erp::depuis(&r.base()).ok_or("ERP non réglé")?;
        let (statut, _, o) = erp.octets(&erp::chemin(erp::CHEMIN_PDF_FACTURE, &code))?;
        if (200..300).contains(&statut) { Ok(o) } else { Err(format!("HTTP {statut}")) }
    })
    .await
    .unwrap_or_else(|_| Err(String::new()));
    match resultat {
        Ok(o) => pdf(o, &format!("facture-{}.pdf", fichier.trim_end_matches(".pdf").replace(['"', '/', '\\'], ""))),
        Err(e) if e == "introuvable" => (StatusCode::NOT_FOUND, page("Facture", Some(&s), r#"<div class="carte"><p class="erreur">Facture introuvable.</p><p><a href="/abonnement">Retour</a></p></div>"#)).into_response(),
        Err(e) => {
            eprintln!("facture PDF : {e}");
            erreur_page(&r, Some(&s), "Facture momentanément indisponible.")
        }
    }
}

// ————— Pages : offres —————

/// `GET /abonnement/offres?cycle=annuel` (page publique : « Choisir » demande la connexion).
pub async fn page_offres(State(r): State<Partage>, h: HeaderMap, Query(q): Query<HashMap<String, String>>) -> Response {
    let s = portail::session(&r, &h);
    let cycle = q.get("cycle").and_then(|c| Cycle::depuis(c)).unwrap_or(Cycle::Mensuel);
    let ab = s.as_ref().map(|s| lire(&r.base(), s.compte.id).unwrap_or_else(|_| Abonnement::gratuit(s.compte.id)));
    let actuelle = ab.as_ref().map(|a| (a.offre, a.cycle, a.actif()));
    let mut cartes = String::new();
    for o in offres::TOUTES {
        let prix = match o {
            Offre::Gratuit => "0 €".to_string(),
            Offre::Entreprise => "Sur devis".to_string(),
            Offre::Machine => format!("{} HT / machine / {}", offres::euros(o.prix(cycle)), if cycle == Cycle::Annuel { "an" } else { "mois" }),
            _ => format!("{} HT / {}", offres::euros(o.prix(cycle)), if cycle == Cycle::Annuel { "an" } else { "mois" }),
        };
        let est_actuelle = actuelle.is_some_and(|(ao, ac, actif)| ao == o && (o == Offre::Gratuit || (actif && ac == cycle)));
        let bouton = if est_actuelle {
            r#"<span class="meta">Votre offre actuelle</span>"#.to_string()
        } else {
            match o {
                Offre::Gratuit => String::new(),
                Offre::Entreprise => format!(r#"<a class="bouton discret" href="mailto:{0}?subject=Offre%20Entreprise%20E-KO%20RIP%20Cloud">Nous contacter</a>"#, offres::CONTACT),
                _ => match (&s, actuelle) {
                    (Some(s), Some((_, _, true))) => format!(r#"<form method="post" action="/abonnement/changer">{}<input type="hidden" name="offre" value="{}"><input type="hidden" name="cycle" value="{}"><button type="submit">Choisir</button></form>"#, hidden_csrf(s), o.code(), cycle.code()),
                    _ => format!(r#"<a class="bouton" href="/abonnement/souscrire?offre={}&amp;cycle={}">Choisir</a>"#, o.code(), cycle.code()),
                },
            }
        };
        cartes.push_str(&format!(r#"<div class="carte"><h2>{}</h2><p><strong>{}</strong></p><p class="meta">{}</p><div class="ligne">{bouton}</div></div>"#, o.nom(), echapper(&prix), echapper(o.description())));
    }
    let bascule = format!(
        r#"<p class="ligne"><a class="bouton{}" href="/abonnement/offres?cycle=mensuel">Mensuel</a><a class="bouton{}" href="/abonnement/offres?cycle=annuel">Annuel (2 mois offerts)</a></p>"#,
        if cycle == Cycle::Mensuel { "" } else { " discret" },
        if cycle == Cycle::Annuel { "" } else { " discret" }
    );
    let corps = format!(
        r#"<h1>Offres</h1><div class="carte"><h2>Les offres E-KO RIP Cloud</h2><p class="meta">Prix hors taxes, utilisateurs illimités, sans engagement au-delà de la période payée. Offres réservées aux professionnels. 30 jours d'essai de l'offre Atelier à la première souscription payante.</p>{bascule}</div>{cartes}<div class="carte"><h2>Option stockage +50 Go</h2><p>{} HT / mois · <span class="meta">bientôt disponible</span></p></div><p class="meta">Voir les <a href="/cgv">conditions générales de vente</a>.</p>"#,
        offres::euros(offres::PRIX_STOCKAGE)
    );
    page(if s.is_some() { "Abonnement" } else { "Offres" }, s.as_ref(), &corps).into_response()
}

// ————— Pages : souscription —————

const SCRIPT_SOUSCRIPTION: &str = r##"<script>
(function () {
  var f = document.getElementById('souscription');
  if (!f) return;
  var csrf = f.getAttribute('data-csrf');
  function champ(n) { return f.querySelector('[name="' + n + '"]'); }
  var bouton = document.getElementById('chercher-siren'), etat = document.getElementById('etat-siren');
  if (bouton) {
    bouton.style.display = '';
    bouton.addEventListener('click', function () {
      var v = champ('siren').value.replace(/\s/g, '');
      if (!v) return;
      etat.textContent = 'Recherche…';
      fetch('/abonnement/entreprise?siren=' + encodeURIComponent(v), { credentials: 'same-origin', headers: { 'X-CSRF-Token': csrf } }).then(function (r) { return r.json().then(function (d) { return [r.ok, d]; }); }).then(function (x) {
        var d = x[1];
        if (!x[0]) { etat.textContent = d.message || 'Entreprise introuvable : saisir les informations à la main.'; return; }
        ['raison_sociale', 'siret', 'tva', 'adresse', 'code_postal', 'ville'].forEach(function (k) { if (d[k] && champ(k)) champ(k).value = d[k]; });
        etat.textContent = d.raison_sociale ? 'Trouvé : ' + d.raison_sociale : '';
      }).catch(function () { etat.textContent = 'Recherche impossible : saisir les informations à la main.'; });
    });
  }
  var code = champ('code'), info = document.getElementById('etat-code'), minuteur = null;
  if (code) code.addEventListener('input', function () {
    clearTimeout(minuteur);
    var v = code.value.trim();
    if (!v) { info.textContent = ''; return; }
    minuteur = setTimeout(function () {
      fetch('/abonnement/code?code=' + encodeURIComponent(v) + '&offre=' + encodeURIComponent(champ('offre').value) + '&cycle=' + encodeURIComponent(champ('cycle').value), { credentials: 'same-origin', headers: { 'X-CSRF-Token': csrf } }).then(function (r) { return r.json(); }).then(function (d) {
        info.textContent = d.libelle || d.message || '';
        info.className = d.valide ? 'ok' : 'erreur';
      }).catch(function () {});
    }, 400);
  });
})();
</script>"##;

#[derive(Default)]
struct Saisie {
    offre: String,
    cycle: String,
    pays: String,
    siren: String,
    raison_sociale: String,
    siret: String,
    tva: String,
    contact: String,
    email: String,
    telephone: String,
    adresse: String,
    code_postal: String,
    ville: String,
    bon_commande: String,
    code: String,
    moyen: String,
}

impl Saisie {
    fn depuis(f: &HashMap<String, String>) -> Saisie {
        let c = |n: &str| champ(f, n);
        Saisie { offre: c("offre"), cycle: c("cycle"), pays: c("pays").to_ascii_uppercase(), siren: c("siren"), raison_sociale: c("raison_sociale"), siret: c("siret"), tva: c("tva"), contact: c("contact"), email: c("email"), telephone: c("telephone"), adresse: c("adresse"), code_postal: c("code_postal"), ville: c("ville"), bon_commande: c("bon_commande"), code: c("code"), moyen: c("moyen") }
    }
}

fn formulaire_souscription(s: &Session, x: &Saisie, erreur: &str, offre: Offre, cycle: Cycle) -> String {
    let v = |t: &str| echapper(t);
    format!(
        r#"<h1>Souscrire</h1><div class="carte"><h2>Offre {nom} · facturation {cyl}</h2><p><strong>{prix}</strong></p><p class="meta">{desc} 30 jours d'essai à la première souscription.</p></div>{err}
<form method="post" action="/abonnement/souscrire" id="souscription" data-csrf="{csrf_brut}">{csrf}<input type="hidden" name="offre" value="{o}"><input type="hidden" name="cycle" value="{cy}">
<div class="carte"><h2>Entreprise</h2>
<label>Pays<select name="pays"><option value="FR"{fr}>France</option><option value="AUTRE"{autre}>Autre pays (saisie manuelle)</option></select></label>
<label>SIREN ou SIRET (entreprises françaises)<span class="ligne"><input type="text" name="siren" value="{siren}" inputmode="numeric" maxlength="17" autocomplete="off" style="max-width: 14rem"><button type="button" class="discret" id="chercher-siren" style="display:none">Remplir depuis le SIREN</button></span></label><p class="meta" id="etat-siren"></p>
<label>Raison sociale<input type="text" name="raison_sociale" value="{rs}" required maxlength="120"></label>
<label>SIRET<input type="text" name="siret" value="{siret}" maxlength="17"></label>
<label>N° de TVA intracommunautaire<input type="text" name="tva" value="{tva}" maxlength="20"></label>
<label>Adresse<input type="text" name="adresse" value="{adr}" maxlength="160"></label>
<span class="ligne"><label>Code postal<input type="text" name="code_postal" value="{cp}" maxlength="12"></label><label>Ville<input type="text" name="ville" value="{ville}" maxlength="80"></label></span></div>
<div class="carte"><h2>Contact de facturation</h2>
<label>Nom<input type="text" name="contact" value="{contact}" required maxlength="80"></label>
<label>E-mail de facturation<input type="email" name="email" value="{email}" required></label>
<label>Téléphone<input type="text" name="telephone" value="{tel}" maxlength="20" autocomplete="tel"></label>
<label>N° de bon de commande (facultatif)<input type="text" name="bon_commande" value="{bc}" maxlength="60"></label></div>
<div class="carte"><h2>Code promo ou partenaire</h2><label>Code (facultatif)<input type="text" name="code" value="{code}" maxlength="40" autocomplete="off"></label><p id="etat-code" class="meta"></p></div>
<div class="carte"><h2>Moyen de paiement</h2><label class="case"><input type="radio" name="moyen" value="cb"{cb}> <span>Carte bancaire (saisie sécurisée Stancer, 3-D Secure)</span></label><label class="case"><input type="radio" name="moyen" value="sepa"{sepa}> <span>Prélèvement SEPA (mandat signé en ligne par code SMS)</span></label>
<label class="case"><input type="checkbox" name="cgv" value="1" required> <span>J'accepte les <a href="/cgv">conditions générales de vente</a> et je souscris pour les besoins de mon activité professionnelle.</span></label>
<button type="submit">Continuer vers le paiement</button></div></form>{SCRIPT_SOUSCRIPTION}"#,
        nom = offre.nom(),
        cyl = cycle.libelle(),
        prix = echapper(&libelle_prix(offre, cycle, 1)),
        desc = echapper(offre.description()),
        err = if erreur.is_empty() { String::new() } else { format!(r#"<p class="erreur">{}</p>"#, echapper(erreur)) },
        csrf_brut = echapper(&s.csrf),
        csrf = hidden_csrf(s),
        o = offre.code(),
        cy = cycle.code(),
        fr = if x.pays != "AUTRE" { " selected" } else { "" },
        autre = if x.pays == "AUTRE" { " selected" } else { "" },
        siren = v(&x.siren),
        rs = v(&x.raison_sociale),
        siret = v(&x.siret),
        tva = v(&x.tva),
        adr = v(&x.adresse),
        cp = v(&x.code_postal),
        ville = v(&x.ville),
        contact = v(&x.contact),
        email = v(&x.email),
        tel = v(&x.telephone),
        bc = v(&x.bon_commande),
        code = v(&x.code),
        cb = if x.moyen != "sepa" { " checked" } else { "" },
        sepa = if x.moyen == "sepa" { " checked" } else { "" },
    )
}

fn offre_et_cycle(o: &str, c: &str) -> Option<(Offre, Cycle)> {
    Some((Offre::depuis(o).filter(|o| o.payante())?, Cycle::depuis(c).unwrap_or(Cycle::Mensuel)))
}

/// `GET /abonnement/souscrire?offre=…&cycle=…`.
pub async fn souscription(State(r): State<Partage>, h: HeaderMap, Query(q): Query<HashMap<String, String>>) -> Response {
    let ici = format!("/abonnement/souscrire?offre={}&cycle={}", encoder(q.get("offre").map_or("", String::as_str)), encoder(q.get("cycle").map_or("", String::as_str)));
    let s = match exiger(&r, &h, &ici) {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    let Some((offre, cycle)) = offre_et_cycle(q.get("offre").map_or("", String::as_str), q.get("cycle").map_or("", String::as_str)) else { return Redirect::to("/abonnement/offres").into_response() };
    let ab = lire(&r.base(), s.compte.id).unwrap_or_else(|_| Abonnement::gratuit(s.compte.id));
    if ab.actif() {
        return Redirect::to("/abonnement/offres").into_response();
    }
    let b = &ab.brouillon;
    let t = |k: &str| b.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let x = Saisie { pays: if t("pays").is_empty() { "FR".into() } else { t("pays") }, siren: t("siren"), raison_sociale: t("raison_sociale"), siret: t("siret"), tva: t("tva"), contact: if t("contact").is_empty() { s.compte.nom.clone() } else { t("contact") }, email: if t("email").is_empty() { s.compte.email.clone() } else { t("email") }, telephone: t("telephone"), adresse: t("adresse"), code_postal: t("code_postal"), ville: t("ville"), bon_commande: t("bon_commande"), code: t("code_saisi"), moyen: t("moyen"), ..Default::default() };
    page("Abonnement", Some(&s), &formulaire_souscription(&s, &x, "", offre, cycle)).into_response()
}

/// Ce qu'est un code saisi : coupon (remise) ou code partenaire.
#[derive(Debug, Clone, PartialEq)]
pub enum Code {
    Coupon { code: String, libelle: String },
    Partenaire { code: String, libelle: String },
    Inconnu(String),
}

fn libelle_coupon(d: &Value) -> String {
    let valeur = d["value"].as_f64().or_else(|| d["value"].as_str().and_then(|s| s.parse().ok())).unwrap_or(0.0);
    let v = if valeur.fract() == 0.0 { format!("{}", valeur as i64) } else { format!("{valeur}").replace('.', ",") };
    let quoi = match d["kind"].as_str().unwrap_or("") {
        "discount_percent" => format!("{v} % de remise"),
        "discount_amount" => format!("{v} € HT de remise"),
        "free_period" => format!("{v} mois offert{}", if valeur > 1.0 { "s" } else { "" }),
        _ => "remise".into(),
    };
    let duree = if d["is_recurring"].as_bool().unwrap_or(false) { " sur chaque facture" } else { " sur la première facture" };
    let nom = d["name"].as_str().filter(|n| !n.is_empty()).map(|n| format!(" ({n})")).unwrap_or_default();
    format!("Code promo : {quoi}{}{nom}", if d["kind"] == "free_period" { "" } else { duree })
}

/// Coupon d'abord, puis code partenaire. Bloquant.
pub fn verifier_code(erp: &Erp, code: &str, plan: Option<&str>) -> Result<Code, String> {
    let code = code.trim();
    let mut chemin = format!("/subscription-coupons/{}", encoder(code));
    if let Some(p) = plan {
        chemin.push_str(&format!("?plan_code={}", encoder(p)));
    }
    let rep = erp.get(&chemin)?;
    if rep.ok() {
        return Ok(Code::Coupon { code: texte_json(&rep.donnees()["code"]).unwrap_or(code.to_string()), libelle: libelle_coupon(rep.donnees()) });
    }
    if rep.statut == 422 {
        let raison = rep.corps["error"].as_str().unwrap_or("coupon.invalide");
        let texte = match raison.strip_prefix("coupon.").unwrap_or(raison) {
            "expired" | "expire" => "Ce code promo a expiré.",
            "exhausted" | "max_redemptions" => "Ce code promo n'est plus disponible.",
            "plan_mismatch" | "not_applicable" => "Ce code promo ne s'applique pas à cette offre.",
            _ => "Ce code promo n'est pas valable.",
        };
        return Ok(Code::Inconnu(texte.into()));
    }
    let rep = erp.get(&format!("/partner-codes/{}", encoder(code)))?;
    if rep.ok() {
        let nom = rep.donnees().pointer("/partner/name").and_then(Value::as_str).unwrap_or("partenaire");
        return Ok(Code::Partenaire { code: texte_json(&rep.donnees()["code"]).unwrap_or(code.to_string()), libelle: format!("Partenaire : {nom}") });
    }
    Ok(Code::Inconnu("Code inconnu.".into()))
}

fn json_message(statut: StatusCode, code: &str, message: &str) -> Response {
    (statut, Json(json!({ "code": code, "message": message }))).into_response()
}

fn session_json(r: &Partage, h: &HeaderMap) -> Result<Session, Box<Response>> {
    let s = portail::session(r, h).ok_or_else(|| Box::new(json_message(StatusCode::UNAUTHORIZED, "session_requise", "Session expirée : reconnectez-vous.")))?;
    if !portail::csrf_entete_ok(&s, h) {
        return Err(Box::new(json_message(StatusCode::FORBIDDEN, "csrf", "Requête refusée.")));
    }
    Ok(s)
}

/// Fiche d'une entreprise française normalisée pour le formulaire.
pub fn fiche_entreprise(d: &Value) -> Option<Value> {
    let i = d.get("identity").filter(|i| i.is_object())?;
    let s = |k: &str| i.get(k).and_then(texte_json).unwrap_or_default();
    let (mut adresse, mut cp, mut ville) = (String::new(), String::new(), String::new());
    match i.get("address") {
        Some(Value::String(a)) => adresse = a.clone(),
        Some(Value::Object(o)) => {
            let g = |ks: &[&str]| ks.iter().find_map(|k| o.get(*k).and_then(texte_json)).unwrap_or_default();
            adresse = g(&["line1", "address_line1", "street", "line"]);
            cp = g(&["postal_code", "zip", "code_postal"]);
            ville = g(&["city", "ville"]);
        }
        _ => {}
    }
    let nom = if s("name").is_empty() { s("commercial_name") } else { s("name") };
    Some(json!({ "raison_sociale": nom, "siret": s("siret"), "tva": s("vat_intracom"), "naf": s("naf"), "adresse": adresse, "code_postal": cp, "ville": ville }))
}

/// `GET /abonnement/entreprise?siren=` : fiche de l'entreprise (proxy de l'ERP, limité).
pub async fn chercher_entreprise(State(r): State<Partage>, h: HeaderMap, Query(q): Query<HashMap<String, String>>) -> Response {
    let s = match session_json(&r, &h) {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    if !r.limiter(&format!("siren:{}", s.compte.id), 30, Duration::from_secs(600)) {
        return json_message(StatusCode::TOO_MANY_REQUESTS, "trop_de_recherches", "Trop de recherches : réessayer dans quelques minutes.");
    }
    let Some((siren, _)) = erp::siren_siret(q.get("siren").map_or("", String::as_str)) else { return json_message(StatusCode::BAD_REQUEST, "siren_invalide", "SIREN (9 chiffres) ou SIRET (14 chiffres) attendu.") };
    let rep = reponse_bloquante(&r, move |r| {
        let erp = Erp::depuis(&r.base()).ok_or("ERP non réglé".to_string())?;
        erp.get(&format!("/companies/{siren}"))
    })
    .await
    .unwrap_or_else(|_| Err(String::new()));
    match rep {
        Ok(rep) if rep.ok() => match fiche_entreprise(rep.donnees()) {
            Some(f) => Json(f).into_response(),
            None => json_message(StatusCode::NOT_FOUND, "introuvable", "Entreprise introuvable : saisir les informations à la main."),
        },
        Ok(rep) if rep.statut == 422 => json_message(StatusCode::BAD_REQUEST, "siren_invalide", "Ce numéro SIREN n'est pas valide."),
        _ => json_message(StatusCode::SERVICE_UNAVAILABLE, "indisponible", "Recherche indisponible : saisir les informations à la main."),
    }
}

/// `GET /abonnement/code?code=&offre=&cycle=` : coupon ou partenaire, vérifié en direct.
pub async fn verifier_code_route(State(r): State<Partage>, h: HeaderMap, Query(q): Query<HashMap<String, String>>) -> Response {
    let s = match session_json(&r, &h) {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    if !r.limiter(&format!("code:{}", s.compte.id), 60, Duration::from_secs(600)) {
        return json_message(StatusCode::TOO_MANY_REQUESTS, "trop_d_essais", "Trop d'essais : réessayer dans quelques minutes.");
    }
    let code: String = q.get("code").map_or("", String::as_str).trim().chars().take(40).collect();
    if code.is_empty() || !code.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Json(json!({ "valide": false, "message": "Code invalide." })).into_response();
    }
    let plan = offre_et_cycle(q.get("offre").map_or("", String::as_str), q.get("cycle").map_or("", String::as_str));
    let rep = reponse_bloquante(&r, move |r| {
        let (erp, plan) = {
            let c = r.base();
            (Erp::depuis(&c).ok_or("ERP non réglé".to_string())?, plan.and_then(|(o, cy)| offres::code_plan(&c, Some(o), cy)))
        };
        verifier_code(&erp, &code, plan.as_deref())
    })
    .await
    .unwrap_or_else(|_| Err(String::new()));
    match rep {
        Ok(Code::Coupon { code, libelle }) => Json(json!({ "valide": true, "type": "coupon", "code": code, "libelle": libelle })).into_response(),
        Ok(Code::Partenaire { code, libelle }) => Json(json!({ "valide": true, "type": "partenaire", "code": code, "libelle": libelle })).into_response(),
        Ok(Code::Inconnu(m)) => Json(json!({ "valide": false, "message": m })).into_response(),
        Err(_) => json_message(StatusCode::SERVICE_UNAVAILABLE, "indisponible", "Vérification indisponible pour le moment."),
    }
}

/// Trouve le tiers de l'ERP par son SIREN, ou le crée. Bloquant.
fn tiers(erp: &Erp, x: &Saisie, siren: Option<(String, Option<String>)>, compte: i64) -> Result<String, String> {
    if let Some((siren, _)) = &siren {
        let rep = erp.get(&format!("/tiers?siren={siren}"))?;
        if rep.ok()
            && let Some(code) = rep.donnees().as_array().and_then(|a| a.first()).and_then(|t| texte_json(&t["code"]))
        {
            return Ok(code);
        }
    }
    let mut corps = json!({ "name": x.raison_sociale, "email": x.email, "default_country": if x.pays == "AUTRE" { "" } else { "FR" }, "is_customer": true });
    if x.pays == "AUTRE" {
        corps.as_object_mut().expect("objet").remove("default_country");
    }
    if let Some((siren, siret)) = &siren {
        corps["siren"] = json!(siren);
        if let Some(s) = siret.clone().or_else(|| erp::siren_siret(&x.siret).and_then(|(_, s)| s)) {
            corps["siret"] = json!(s);
        }
    }
    if !x.tva.is_empty() {
        corps["vat_intracom"] = json!(x.tva);
    }
    if let Some(t) = erp::telephone_e164(&x.telephone) {
        corps["phone"] = json!(t);
    }
    let rep = erp.post("/tiers", &corps, Some(&format!("cloud-tiers-{compte}-{}", crate::sha256_hex(&format!("{}{}", x.raison_sociale, x.siren)).get(..16).unwrap_or(""))))?;
    if !rep.ok() {
        return Err(format!("Fiche client refusée par la facturation : {}", rep.erreur()));
    }
    texte_json(&rep.donnees()["code"]).ok_or_else(|| "Réponse inattendue de la facturation.".into())
}

/// `POST /abonnement/souscrire` : tiers trouvé ou créé, code vérifié, puis étape de paiement.
pub async fn souscrire(State(r): State<Partage>, h: HeaderMap, axum::Form(f): Champs) -> Response {
    if !origine_ok(&r, &h) {
        return refuse_origine();
    }
    let s = match exiger(&r, &h, "/abonnement/offres") {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    if !csrf_ok(&s, &f) {
        return refuse_origine();
    }
    let x = Saisie::depuis(&f);
    let Some((offre, cycle)) = offre_et_cycle(&x.offre, &x.cycle) else { return Redirect::to("/abonnement/offres").into_response() };
    let siren = erp::siren_siret(&x.siren);
    let erreur = if !r.limiter(&format!("souscrire:{}", s.compte.id), 20, Duration::from_secs(3600)) {
        "Trop d'essais : réessayer dans une heure."
    } else if champ(&f, "cgv") != "1" {
        "Il faut accepter les conditions générales de vente."
    } else if x.raison_sociale.is_empty() || x.raison_sociale.chars().count() > 120 || x.raison_sociale.chars().any(char::is_control) {
        "Raison sociale requise."
    } else if x.pays != "AUTRE" && siren.is_none() {
        "SIREN (9 chiffres) ou SIRET (14 chiffres) requis pour une entreprise française."
    } else if !base::nom_valide(&x.contact) {
        "Nom du contact de facturation requis."
    } else if !base::email_valide(&x.email) {
        "E-mail de facturation invalide."
    } else if !x.telephone.is_empty() && erp::telephone_e164(&x.telephone).is_none() {
        "Téléphone invalide."
    } else if !matches!(x.moyen.as_str(), "cb" | "sepa") {
        "Choisissez un moyen de paiement."
    } else if lire(&r.base(), s.compte.id).map(|a| a.actif()).unwrap_or(false) {
        "Un abonnement est déjà en cours : changez d'offre depuis la page Abonnement."
    } else {
        ""
    };
    if !erreur.is_empty() {
        return (StatusCode::BAD_REQUEST, page("Abonnement", Some(&s), &formulaire_souscription(&s, &x, erreur, offre, cycle))).into_response();
    }
    let id = s.compte.id;
    let code_saisi = x.code.clone();
    let x2 = Saisie { ..Saisie::depuis(&f) };
    let resultat = reponse_bloquante(&r, move |r| -> Result<(String, Option<Code>), String> {
        let (erp, plan) = {
            let c = r.base();
            (Erp::depuis(&c).ok_or("La souscription en ligne n'est pas encore ouverte.")?, offres::code_plan(&c, Some(offre), cycle).ok_or("Cette offre n'est pas encore ouverte à la souscription.")?)
        };
        let code = if code_saisi.is_empty() { None } else { Some(verifier_code(&erp, &code_saisi, Some(&plan))?) };
        if let Some(Code::Inconnu(m)) = &code {
            return Err(m.clone());
        }
        Ok((tiers(&erp, &x2, siren, id)?, code))
    })
    .await
    .unwrap_or_else(|_| Err("Service momentanément indisponible.".into()));
    let (tier, code) = match resultat {
        Ok(v) => v,
        Err(e) => return (StatusCode::BAD_REQUEST, page("Abonnement", Some(&s), &formulaire_souscription(&s, &x, &e, offre, cycle))).into_response(),
    };
    let enregistre = {
        let c = r.base();
        // Un mandat SEPA n'est valable que pour son tiers : autre entreprise, mandat à refaire.
        let autre_tiers = lire(&c, id).map(|a| a.tier_code.is_some_and(|t| t != tier)).unwrap_or(false);
        let mut champs = vec![("tier_code", texte(&tier))];
        if autre_tiers {
            champs.extend([("rum", Sql::Null), ("rum_prevenu", entier(0))]);
        }
        maj(&c, id, &champs).and_then(|_| {
            regler_brouillon(&c, id, |b| {
                if b.get("mandat").and_then(|m| m.get("tier")).and_then(Value::as_str) != Some(tier.as_str()) {
                    b.remove("mandat");
                }
                for (k, v) in [("offre", offre.code()), ("cycle", cycle.code()), ("pays", &x.pays), ("siren", &x.siren), ("raison_sociale", &x.raison_sociale), ("siret", &x.siret), ("tva", &x.tva), ("contact", &x.contact), ("email", &x.email), ("telephone", &x.telephone), ("adresse", &x.adresse), ("code_postal", &x.code_postal), ("ville", &x.ville), ("bon_commande", &x.bon_commande), ("code_saisi", &x.code), ("moyen", &x.moyen)] {
                    b.insert(k.into(), json!(v));
                }
                let (coupon, partenaire) = match &code {
                    Some(Code::Coupon { code, .. }) => (code.clone(), String::new()),
                    Some(Code::Partenaire { code, .. }) => (String::new(), code.clone()),
                    _ => (String::new(), String::new()),
                };
                b.insert("coupon".into(), json!(coupon));
                b.insert("partenaire".into(), json!(partenaire));
                b.insert("cle".into(), json!(crate::aleatoire_hex(8)));
            })
        })
    };
    if enregistre.is_err() {
        return erreur_page(&r, Some(&s), "Enregistrement impossible.");
    }
    Redirect::to(&format!("/abonnement/paiement?moyen={}", x.moyen)).into_response()
}

// ————— Pages : paiement —————

const SCRIPT_CARTE: &str = r##"<script>
(function () {
  var zone = document.getElementById('paiement-carte');
  if (!zone) return;
  var csrf = zone.getAttribute('data-csrf'), cle = zone.getAttribute('data-cle');
  var etat = zone.querySelector('[data-etat]'), bouton = zone.querySelector('[data-payer]');
  var modale = document.getElementById('modale-3ds'), cadre = modale.querySelector('iframe');
  var element = null, paiement = null, suivi = null, essais = 0;
  function dire(t, erreur) { etat.textContent = t; etat.className = erreur ? 'erreur' : 'meta'; }
  function preparer() {
    if (!window.Stancer) { dire('Module de paiement non chargé : recharger la page.', true); return; }
    element = new window.Stancer.CardElement('#carte', { publishable_key: cle, language: 'fr' });
    if (element.mount) element.mount();
    bouton.disabled = false;
  }
  if (document.readyState === 'complete') preparer(); else window.addEventListener('load', preparer);
  function fermer() { modale.hidden = true; cadre.src = 'about:blank'; }
  function verifier() {
    if (!paiement) return;
    if (++essais > 60) { clearInterval(suivi); fermer(); bouton.disabled = false; dire('Paiement non confirmé : réessayer.', true); return; }
    fetch('/abonnement/paiement/carte/' + encodeURIComponent(paiement), { credentials: 'same-origin', headers: { 'X-CSRF-Token': csrf } }).then(function (r) { return r.json(); }).then(function (d) {
      if (d.termine) { clearInterval(suivi); fermer(); dire('Paiement confirmé.'); location.href = '/abonnement?ok=carte'; }
      else if (d.echec) { clearInterval(suivi); fermer(); bouton.disabled = false; dire(d.message || 'Paiement refusé par la banque.', true); }
    }).catch(function () {});
  }
  window.addEventListener('message', function (e) {
    if (e.origin !== location.origin || !e.data || e.data.eko3ds !== 'retour') return;
    fermer(); dire('Vérification du paiement…'); verifier();
  });
  modale.querySelector('[data-fermer]').addEventListener('click', function () { fermer(); bouton.disabled = false; dire('Authentification interrompue.', true); clearInterval(suivi); });
  bouton.addEventListener('click', function () {
    if (!element) return;
    bouton.disabled = true; dire('Enregistrement de la carte…');
    element.createCard().then(function (carte) {
      return fetch('/abonnement/paiement/carte', { method: 'POST', credentials: 'same-origin', headers: { 'Content-Type': 'application/json', 'X-CSRF-Token': csrf }, body: JSON.stringify({ card_id: carte.id, last4: carte.last4 || '', brand: carte.brand || '' }) });
    }).then(function (r) { return r.json().then(function (d) { if (!r.ok) throw new Error(d.message || 'Paiement impossible.'); return d; }); }).then(function (d) {
      paiement = d.payment_id; essais = 0;
      if (d.redirect_url) { cadre.src = d.redirect_url; modale.hidden = false; dire('Authentification 3-D Secure en cours…'); }
      else { dire('Vérification du paiement…'); }
      suivi = setInterval(verifier, 2500); verifier();
    }).catch(function (e) { bouton.disabled = false; dire((e && e.message) || 'Paiement impossible.', true); });
  });
})();
</script>"##;

fn avec_csp_paiement(mut rep: Response) -> Response {
    let h = rep.headers_mut();
    h.insert(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static(CSP_PAIEMENT));
    h.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    h.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    h.insert(header::REFERRER_POLICY, HeaderValue::from_static("same-origin"));
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    rep
}

/// `GET /abonnement/paiement?moyen=cb|sepa` : carte (Stancer.js + 3-D Secure dans la page) ou mandat SEPA.
pub async fn page_paiement(State(r): State<Partage>, h: HeaderMap, Query(q): Query<HashMap<String, String>>) -> Response {
    let s = match exiger(&r, &h, "/abonnement/paiement") {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    let id = s.compte.id;
    let (ab, cle_publique) = {
        let c = r.base();
        (lire(&c, id).unwrap_or_else(|_| Abonnement::gratuit(id)), base::reglage(&c, "stancer_cle_publique").ok().flatten())
    };
    let offre = if ab.actif() { Some((ab.offre, ab.cycle)) } else { offre_et_cycle(&ab.texte_brouillon("offre"), &ab.texte_brouillon("cycle")).filter(|_| ab.tier_code.is_some()) };
    let Some((offre, cycle)) = offre else { return Redirect::to("/abonnement/offres").into_response() };
    let moyen = q.get("moyen").map_or_else(|| ab.texte_brouillon("moyen"), String::clone);
    let sepa = moyen == "sepa";
    let erreur = q.get("erreur").map(|e| format!(r#"<p class="erreur">{}</p>"#, echapper(e))).unwrap_or_default();
    let onglets = format!(r#"<p class="ligne"><a class="bouton{}" href="/abonnement/paiement?moyen=cb">Carte bancaire</a><a class="bouton{}" href="/abonnement/paiement?moyen=sepa">Prélèvement SEPA</a></p>"#, if sepa { " discret" } else { "" }, if sepa { "" } else { " discret" });
    let panneau = if !sepa {
        match cle_publique {
            None => r#"<div class="carte"><p class="meta">Le paiement par carte n'est pas encore disponible : choisissez le prélèvement SEPA.</p></div>"#.to_string(),
            Some(cle) => format!(
                r#"<div class="carte" id="paiement-carte" data-csrf="{csrf}" data-cle="{cle}"><h2>Carte bancaire</h2><p class="meta">La carte est saisie dans un champ sécurisé de Stancer (PCI-DSS) : la plate-forme ne voit jamais son numéro. Votre banque peut vous demander une confirmation (3-D Secure), affichée ici.</p><div id="carte" style="min-height: 64px; padding: .4rem 0"></div><p><button type="button" data-payer disabled>Enregistrer la carte et valider</button></p><p class="meta" data-etat><noscript>Le paiement par carte a besoin de JavaScript ; sinon, choisissez le prélèvement SEPA.</noscript></p></div>
<div id="modale-3ds" hidden style="position: fixed; inset: 0; z-index: 10; background: rgba(0,0,0,.45); display: flex; align-items: center; justify-content: center; padding: 1rem"><div class="carte" style="width: 100%; max-width: 30rem; margin: 0"><h2>Confirmation de votre banque</h2><iframe title="Authentification 3-D Secure" src="about:blank" style="width: 100%; height: 26rem; border: 0; border-radius: 8px; background: #fff"></iframe><p><button type="button" class="discret" data-fermer>Annuler</button></p></div></div>
<style>#modale-3ds[hidden] {{ display: none !important; }}</style>
<script src="https://js.stancer.com/v1.0.0/stancer.js"></script>{SCRIPT_CARTE}"#,
                csrf = echapper(&s.csrf),
                cle = echapper(&cle),
            ),
        }
    } else {
        panneau_sepa(&s, &ab)
    };
    let corps = format!(
        r#"<h1>Paiement</h1>{erreur}<div class="carte"><h2>Offre {} · facturation {}</h2><p><strong>{}</strong>{}</p><p class="meta">Moyen de paiement actuel : {}</p></div>{onglets}{panneau}"#,
        offre.nom(),
        cycle.libelle(),
        echapper(&libelle_prix(offre, cycle, ab.seats.max(1))),
        if ab.actif() { "" } else { " · 30 jours d'essai avant la première échéance" },
        echapper(&libelle_moyen(&ab)),
    );
    avec_csp_paiement(page("Abonnement", Some(&s), &corps).into_response())
}

fn panneau_sepa(s: &Session, ab: &Abonnement) -> String {
    let mandat = ab.brouillon.get("mandat").filter(|m| m.is_object());
    match mandat {
        Some(m) => format!(
            r#"<div class="carte"><h2>Mandat de prélèvement SEPA</h2><p>Mandat <strong>{rum}</strong> pour l'IBAN se terminant par <strong>{fin}</strong>. Lisez-le, puis signez-le avec le code reçu par SMS.</p><iframe title="Mandat SEPA" src="/abonnement/mandat.pdf" style="width: 100%; height: 28rem; border: .5px solid var(--trait); border-radius: 8px"></iframe><p><a href="/abonnement/mandat.pdf" target="_blank" rel="noopener">Ouvrir le mandat (PDF)</a></p>
<form method="post" action="/abonnement/paiement/sepa/code">{csrf}<button class="discret" type="submit">Recevoir le code par SMS</button> <span class="meta">{envoye}</span></form>
<form method="post" action="/abonnement/paiement/sepa/signer">{csrf}<label>Code reçu par SMS<input type="text" name="otp" required inputmode="numeric" maxlength="8" autocomplete="one-time-code" style="max-width: 10rem"></label><label>Nom et prénom du signataire (tapés comme signature)<input type="text" name="nom_signe" required maxlength="80"></label><label class="case"><input type="checkbox" name="consentement" value="1" required> <span>J'autorise SAS GOLDEN PRINT à envoyer des instructions à ma banque pour débiter mon compte, et ma banque à débiter mon compte conformément à ces instructions. Je peux être remboursé par ma banque selon les conditions de ma convention.</span></label><button type="submit">Signer le mandat</button></form>
<form method="post" action="/abonnement/paiement/sepa/annuler">{csrf}<button class="discret" type="submit">Utiliser un autre compte</button></form></div>"#,
            rum = echapper(m["rum"].as_str().unwrap_or("")),
            fin = echapper(m["iban_last4"].as_str().unwrap_or("")),
            csrf = hidden_csrf(s),
            envoye = if m["code_envoye"].as_bool().unwrap_or(false) { "Code envoyé : il peut mettre une minute à arriver." } else { "" },
        ),
        None => {
            let t = |k: &str| echapper(&ab.texte_brouillon(k));
            let (prenom, nom) = ab.texte_brouillon("contact").split_once(' ').map(|(a, b)| (echapper(a), echapper(b))).unwrap_or((String::new(), t("contact")));
            format!(
                r#"<div class="carte"><h2>Prélèvement SEPA</h2><p class="meta">Le mandat est créé au nom de SAS GOLDEN PRINT et signé en ligne : un code vous est envoyé par SMS.</p><form method="post" action="/abonnement/paiement/sepa">{csrf}
<label>IBAN<input type="text" name="iban" required maxlength="42" autocomplete="off" placeholder="FR76 …"></label>
<label>BIC (facultatif)<input type="text" name="bic" maxlength="11" autocomplete="off"></label>
<span class="ligne"><label>Prénom du titulaire<input type="text" name="prenom" required maxlength="60" value="{prenom}"></label><label>Nom du titulaire<input type="text" name="nom" required maxlength="60" value="{nom}"></label></span>
<label>Téléphone mobile (pour le code SMS)<input type="text" name="telephone" required maxlength="20" value="{tel}" autocomplete="tel"></label>
<label>Adresse<input type="text" name="adresse" maxlength="160" value="{adr}"></label>
<span class="ligne"><label>Code postal<input type="text" name="code_postal" maxlength="12" value="{cp}"></label><label>Ville<input type="text" name="ville" maxlength="80" value="{ville}"></label></span>
<button type="submit">Créer le mandat</button></form></div>"#,
                csrf = hidden_csrf(s),
                tel = t("telephone"),
                adr = t("adresse"),
                cp = t("code_postal"),
                ville = t("ville"),
            )
        }
    }
}

#[derive(serde::Deserialize)]
pub struct DemandeCarte {
    card_id: String,
    #[serde(default)]
    last4: String,
    #[serde(default)]
    brand: String,
}

/// Carte tokenisée par Stancer.js : abonnement créé au besoin, carte enregistrée, puis
/// authentification de la carte (3-D Secure). Bloquant.
pub fn payer_par_carte(r: &Partage, id: i64, card_id: &str, libelle: &str) -> Result<Value, String> {
    let actif = lire(&r.base(), id).map_err(|e| e.to_string())?.actif();
    if !actif {
        creer_abonnement(r, id, None)?;
    }
    let (erp, ab) = {
        let c = r.base();
        (Erp::depuis(&c).ok_or("Facturation indisponible.")?, lire(&c, id).map_err(|e| e.to_string())?)
    };
    let code = ab.subscription_code.clone().ok_or("Abonnement introuvable.")?;
    let rep = erp.envoyer("PUT", &format!("/subscriptions/{}/payment-method", encoder(&code)), &json!({ "provider": "stancer_card", "card_id": card_id }), None)?;
    if !rep.ok() {
        return Err(format!("Carte refusée : {}", rep.erreur()));
    }
    let rep = erp.post(&format!("/subscriptions/{}/card-authentication", encoder(&code)), &json!({ "return_url": format!("{}/abonnement/3ds-retour", r.config.url_publique) }), None)?;
    if !rep.ok() {
        return Err(format!("Authentification de la carte impossible : {}", rep.erreur()));
    }
    let d = rep.donnees().clone();
    let paiement = texte_json(&d["payment_id"]).ok_or("Réponse inattendue de la facturation.")?;
    {
        let c = r.base();
        maj(&c, id, &[("carte_id", texte(card_id))]).map_err(|e| e.to_string())?;
        regler_brouillon(&c, id, |b| {
            b.insert("paiement_id".into(), json!(paiement));
            b.insert("libelle_carte".into(), json!(libelle));
        })
        .map_err(|e| e.to_string())?;
    }
    Ok(json!({ "payment_id": paiement, "redirect_url": d["redirect_url"], "status": d["status"] }))
}

/// État de l'authentification de la carte ; payée ou autorisée : moyen de paiement enregistré. Bloquant.
pub fn suivre_carte(r: &Partage, id: i64, paiement: &str) -> Result<Value, String> {
    let (erp, ab) = {
        let c = r.base();
        (Erp::depuis(&c).ok_or("Facturation indisponible.")?, lire(&c, id).map_err(|e| e.to_string())?)
    };
    if ab.texte_brouillon("paiement_id") != paiement || paiement.is_empty() {
        return Err("inconnu".into());
    }
    let code = ab.subscription_code.clone().ok_or("Abonnement introuvable.")?;
    let rep = erp.get(&format!("/subscriptions/{}/card-authentication/{}", encoder(&code), encoder(paiement)))?;
    if !rep.ok() {
        return Err(rep.erreur());
    }
    let d = rep.donnees();
    let fini = d["paid"].as_bool().unwrap_or(false) || d["authorized"].as_bool().unwrap_or(false);
    let statut = d["status"].as_str().unwrap_or("").to_string();
    if fini {
        // La carte vaut aussi pour le supplément (machines en plus), comme un mandat signé.
        if let (Some(sc), Some(carte)) = (&ab.supplement_code, &ab.carte_id) {
            match erp.envoyer("PUT", &format!("/subscriptions/{}/payment-method", encoder(sc)), &json!({ "provider": "stancer_card", "card_id": carte }), None) {
                Ok(rep) if rep.ok() => {}
                Ok(rep) => eprintln!("carte du supplément {sc} refusée : {}", rep.erreur()),
                Err(e) => eprintln!("carte du supplément {sc} : {e}"),
            }
        }
        let libelle = ab.texte_brouillon("libelle_carte");
        let _ = maj(&r.base(), id, &[("moyen_paiement", texte(if libelle.is_empty() { "Carte bancaire" } else { &libelle })), ("rum", Sql::Null)]);
        if let Ok(rep) = erp.get(&format!("/subscriptions/{}", encoder(&code)))
            && rep.ok()
        {
            let _ = appliquer(&r.base(), id, rep.donnees());
        }
        let _ = regler_brouillon(&r.base(), id, |b| {
            b.remove("paiement_id");
        });
        return Ok(json!({ "termine": true, "statut": statut }));
    }
    let echec = matches!(statut.as_str(), "failed" | "refused" | "canceled" | "cancelled" | "expired");
    Ok(json!({ "termine": false, "echec": echec, "statut": statut, "message": if echec { "Paiement refusé par la banque." } else { "" } }))
}

/// `POST /abonnement/paiement/carte` (JSON `{card_id, last4, brand}`).
pub async fn carte(State(r): State<Partage>, h: HeaderMap, Json(d): Json<DemandeCarte>) -> Response {
    if !origine_ok(&r, &h) {
        return refuse_origine();
    }
    let s = match session_json(&r, &h) {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    if !r.limiter(&format!("carte:{}", s.compte.id), 10, Duration::from_secs(3600)) {
        return json_message(StatusCode::TOO_MANY_REQUESTS, "trop_d_essais", "Trop d'essais : réessayer dans une heure.");
    }
    let card_id = d.card_id.trim().to_string();
    if card_id.is_empty() || card_id.len() > 64 || !card_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return json_message(StatusCode::BAD_REQUEST, "carte_invalide", "Carte invalide.");
    }
    let marque: String = d.brand.chars().filter(|c| c.is_ascii_alphanumeric()).take(20).collect();
    let fin: String = d.last4.chars().filter(char::is_ascii_digit).take(4).collect();
    let libelle = match (marque.is_empty(), fin.len() == 4) {
        (_, false) => "Carte bancaire".to_string(),
        (true, true) => format!("Carte se terminant par {fin}"),
        (false, true) => format!("Carte {} se terminant par {fin}", marque[..1].to_uppercase() + &marque[1..]),
    };
    let id = s.compte.id;
    match reponse_bloquante(&r, move |r| payer_par_carte(r, id, &card_id, &libelle)).await.unwrap_or_else(|_| Err("Service momentanément indisponible.".into())) {
        Ok(v) => Json(v).into_response(),
        Err(e) => json_message(StatusCode::BAD_REQUEST, "paiement_refuse", &e),
    }
}

/// `GET /abonnement/paiement/carte/{paiement}` : suivi de l'authentification.
pub async fn suivi_carte(State(r): State<Partage>, h: HeaderMap, Chemin(paiement): Chemin<String>) -> Response {
    let s = match session_json(&r, &h) {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    if !r.limiter(&format!("suivi-carte:{}", s.compte.id), 200, Duration::from_secs(600)) {
        return json_message(StatusCode::TOO_MANY_REQUESTS, "trop_d_essais", "Trop d'essais.");
    }
    let id = s.compte.id;
    match reponse_bloquante(&r, move |r| suivre_carte(r, id, &paiement)).await.unwrap_or_else(|_| Err(String::new())) {
        Ok(v) => Json(v).into_response(),
        Err(e) if e == "inconnu" => json_message(StatusCode::NOT_FOUND, "inconnu", "Paiement inconnu."),
        Err(_) => json_message(StatusCode::SERVICE_UNAVAILABLE, "indisponible", "Vérification momentanément indisponible."),
    }
}

/// `GET /abonnement/3ds-retour` : page chargée dans la fenêtre 3-D Secure à la fin de
/// l'authentification ; elle prévient la page de paiement (jamais de redirection de la page).
pub async fn retour_3ds() -> Response {
    let mut rep = axum::response::Html(r#"<!doctype html><html lang="fr"><head><meta charset="utf-8"><title>Authentification terminée</title></head><body style="font: 15px -apple-system, system-ui, sans-serif; text-align: center; padding: 3rem 1rem"><p>Authentification terminée.</p><script>try { parent.postMessage({ eko3ds: 'retour' }, location.origin); } catch (e) {}</script></body></html>"#).into_response();
    let h = rep.headers_mut();
    h.insert(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static("default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; frame-ancestors 'self'; base-uri 'none'"));
    h.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("SAMEORIGIN"));
    h.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    rep
}

fn vers_paiement_sepa(erreur: Option<&str>) -> Response {
    match erreur {
        Some(e) => Redirect::to(&format!("/abonnement/paiement?moyen=sepa&erreur={}", encoder(e))).into_response(),
        None => Redirect::to("/abonnement/paiement?moyen=sepa").into_response(),
    }
}

fn session_formulaire(r: &Partage, h: &HeaderMap, f: &HashMap<String, String>) -> Result<Session, Box<Response>> {
    if !origine_ok(r, h) {
        return Err(Box::new(refuse_origine()));
    }
    let s = exiger(r, h, "/abonnement/paiement?moyen=sepa")?;
    if !csrf_ok(&s, f) {
        return Err(Box::new(refuse_origine()));
    }
    Ok(s)
}

/// `POST /abonnement/paiement/sepa` : création du mandat dans l'ERP.
pub async fn creer_mandat(State(r): State<Partage>, h: HeaderMap, axum::Form(f): Champs) -> Response {
    let s = match session_formulaire(&r, &h, &f) {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    if !r.limiter(&format!("mandat:{}", s.compte.id), 10, Duration::from_secs(3600)) {
        return vers_paiement_sepa(Some("Trop d'essais : réessayer dans une heure."));
    }
    let Some(iban) = erp::iban_valide(&champ(&f, "iban")) else { return vers_paiement_sepa(Some("IBAN invalide.")) };
    let Some(tel) = erp::telephone_e164(&champ(&f, "telephone")) else { return vers_paiement_sepa(Some("Téléphone mobile invalide (il reçoit le code de signature).")) };
    let (prenom, nom) = (champ(&f, "prenom"), champ(&f, "nom"));
    if !base::nom_valide(&prenom) || !base::nom_valide(&nom) {
        return vers_paiement_sepa(Some("Prénom et nom du titulaire requis."));
    }
    let bic: String = champ(&f, "bic").chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_ascii_uppercase();
    let (adresse, cp, ville) = (champ(&f, "adresse"), champ(&f, "code_postal"), champ(&f, "ville"));
    let id = s.compte.id;
    let email_compte = s.compte.email.clone();
    let resultat = reponse_bloquante(&r, move |r| -> Result<(), String> {
        let (erp, ab) = {
            let c = r.base();
            (Erp::depuis(&c).ok_or("Facturation indisponible.")?, lire(&c, id).map_err(|e| e.to_string())?)
        };
        let tier = ab.tier_code.clone().ok_or("Renseignez d'abord les informations de facturation.")?;
        let email = if base::email_valide(&ab.texte_brouillon("email")) { ab.texte_brouillon("email") } else { email_compte };
        let mut corps = json!({ "tier_code": tier, "iban": iban, "holder_first_name": prenom, "holder_last_name": nom, "email": email, "phone": tel });
        for (k, v) in [("bic", &bic), ("address_line1", &adresse), ("postal_code", &cp), ("city", &ville)] {
            if !v.is_empty() {
                corps[k] = json!(v);
            }
        }
        if !adresse.is_empty() {
            corps["country"] = json!(if ab.texte_brouillon("pays") == "AUTRE" { "" } else { "FR" });
        }
        let rep = erp.post("/sepa-mandates", &corps, Some(&format!("cloud-mandat-{id}-{}", crate::sha256_hex(&format!("{iban}{}", ab.texte_brouillon("cle"))).get(..16).unwrap_or(""))))?;
        if !rep.ok() {
            return Err(format!("Mandat refusé : {}", rep.erreur()));
        }
        let d = rep.donnees();
        let rum = texte_json(&d["rum"]).ok_or("Réponse inattendue de la facturation.")?;
        let fin = texte_json(&d["iban_last4"]).unwrap_or_else(|| iban[iban.len() - 4..].to_string());
        regler_brouillon(&r.base(), id, |b| {
            b.insert("mandat".into(), json!({ "rum": rum, "iban_last4": fin, "statut": d["status"], "code_envoye": false, "tier": tier }));
        })
        .map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .unwrap_or_else(|_| Err("Service momentanément indisponible.".into()));
    vers_paiement_sepa(resultat.err().as_deref())
}

fn rum_en_cours(ab: &Abonnement) -> Option<String> {
    ab.brouillon.pointer("/mandat/rum").and_then(texte_json)
}

/// `POST /abonnement/paiement/sepa/code` : code de signature envoyé par SMS.
pub async fn envoyer_code_sms(State(r): State<Partage>, h: HeaderMap, axum::Form(f): Champs) -> Response {
    let s = match session_formulaire(&r, &h, &f) {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    if !r.limiter(&format!("otp:{}", s.compte.id), 5, Duration::from_secs(3600)) {
        return vers_paiement_sepa(Some("Trop de codes demandés : réessayer dans une heure."));
    }
    let id = s.compte.id;
    let resultat = reponse_bloquante(&r, move |r| -> Result<(), String> {
        let (erp, ab) = {
            let c = r.base();
            (Erp::depuis(&c).ok_or("Facturation indisponible.")?, lire(&c, id).map_err(|e| e.to_string())?)
        };
        let rum = rum_en_cours(&ab).ok_or("Créez d'abord le mandat.")?;
        let rep = erp.post(&format!("/sepa-mandates/{}/signature/otp", encoder(&rum)), &json!({}), None)?;
        if !rep.ok() {
            return Err(format!("Code non envoyé : {}", rep.erreur()));
        }
        regler_brouillon(&r.base(), id, |b| {
            if let Some(m) = b.get_mut("mandat").and_then(Value::as_object_mut) {
                m.insert("code_envoye".into(), json!(true));
            }
        })
        .map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .unwrap_or_else(|_| Err("Service momentanément indisponible.".into()));
    vers_paiement_sepa(resultat.err().as_deref())
}

/// `POST /abonnement/paiement/sepa/annuler` : oublie le mandat en cours (autre IBAN).
pub async fn annuler_mandat(State(r): State<Partage>, h: HeaderMap, axum::Form(f): Champs) -> Response {
    let s = match session_formulaire(&r, &h, &f) {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    let _ = regler_brouillon(&r.base(), s.compte.id, |b| {
        b.remove("mandat");
    });
    vers_paiement_sepa(None)
}

/// Code vérifié, mandat finalisé, puis abonnement créé avec ce mandat (ou moyen de paiement
/// remplacé). Bloquant.
pub fn signer_mandat(r: &Partage, id: i64, otp: &str, nom_signe: &str, ip: &str, agent: &str) -> Result<(), String> {
    let (erp, ab) = {
        let c = r.base();
        (Erp::depuis(&c).ok_or("Facturation indisponible.")?, lire(&c, id).map_err(|e| e.to_string())?)
    };
    let rum = rum_en_cours(&ab).ok_or("Créez d'abord le mandat.")?;
    // L'ERP n'accepte en moyen de paiement qu'un mandat signé du même tiers que l'abonnement.
    if ab.brouillon.pointer("/mandat/tier").and_then(texte_json) != ab.tier_code {
        let _ = regler_brouillon(&r.base(), id, |b| {
            b.remove("mandat");
        });
        return Err("Les informations de facturation ont changé : créez un nouveau mandat.".into());
    }
    let fin = ab.brouillon.pointer("/mandat/iban_last4").and_then(texte_json).unwrap_or_default();
    let rep = erp.post(&format!("/sepa-mandates/{}/signature/verify", encoder(&rum)), &json!({ "otp": otp }), None)?;
    if !rep.ok() {
        return Err(if rep.statut == 422 { "Code incorrect ou expiré : redemandez un code.".into() } else { format!("Vérification impossible : {}", rep.erreur()) });
    }
    let rep = erp.post(&format!("/sepa-mandates/{}/signature/finalize", encoder(&rum)), &json!({ "typed_name": nom_signe, "signer_ip": ip, "signer_user_agent": agent, "consent": true }), None)?;
    if !rep.ok() {
        return Err(format!("Signature refusée : {}", rep.erreur()));
    }
    if rep.donnees().get("status").and_then(texte_json).is_some_and(|st| st != "signed") {
        return Err("Le mandat n'est pas encore signé : réessayez dans un instant.".into());
    }
    eprintln!("mandat SEPA {rum} signé (compte {id})");
    let paiement = json!({ "provider": "stancer_sepa", "mandate_code": rum });
    if ab.actif() {
        let code = ab.subscription_code.clone().unwrap_or_default();
        let rep = erp.envoyer("PUT", &format!("/subscriptions/{}/payment-method", encoder(&code)), &paiement, None)?;
        if !rep.ok() {
            return Err(format!("Mandat signé, mais non enregistré comme moyen de paiement : {}", rep.erreur()));
        }
        if let Some(sc) = &ab.supplement_code {
            let _ = erp.envoyer("PUT", &format!("/subscriptions/{}/payment-method", encoder(sc)), &paiement, None);
        }
    } else {
        creer_abonnement(r, id, Some(paiement))?;
    }
    {
        let c = r.base();
        maj(&c, id, &[("rum", texte(&rum)), ("rum_prevenu", entier(1)), ("carte_id", Sql::Null), ("moyen_paiement", texte(&format!("Prélèvement SEPA (IBAN se terminant par {fin})")))]).map_err(|e| e.to_string())?;
        let _ = regler_brouillon(&c, id, |b| {
            b.remove("mandat");
        });
    }
    if let Some(cp) = compte(r, id) {
        let ab2 = lire(&r.base(), id).map_err(|e| e.to_string())?;
        r.envoyer(courriels::mandat_signe(&r.config.url_publique, &destinataire_facturation(&ab2, &cp), &cp.nom, &rum, &fin, &format!("{}/abonnement/mandat.pdf", r.config.url_publique)));
    }
    Ok(())
}

/// `POST /abonnement/paiement/sepa/signer`.
pub async fn signer(State(r): State<Partage>, h: HeaderMap, axum::Form(f): Champs) -> Response {
    let s = match session_formulaire(&r, &h, &f) {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    if !r.limiter(&format!("signer:{}", s.compte.id), 10, Duration::from_secs(3600)) {
        return vers_paiement_sepa(Some("Trop d'essais : réessayer dans une heure."));
    }
    let otp: String = champ(&f, "otp").chars().filter(char::is_ascii_digit).collect();
    let nom_signe = champ(&f, "nom_signe");
    if otp.is_empty() || otp.len() > 8 {
        return vers_paiement_sepa(Some("Saisissez le code reçu par SMS."));
    }
    if !base::nom_valide(&nom_signe) {
        return vers_paiement_sepa(Some("Tapez vos nom et prénom pour signer."));
    }
    if champ(&f, "consentement") != "1" {
        return vers_paiement_sepa(Some("Il faut cocher l'autorisation de prélèvement."));
    }
    let (ip, agent) = (portail::ip(&h), h.get(header::USER_AGENT).and_then(|v| v.to_str().ok()).unwrap_or("").chars().take(300).collect::<String>());
    let id = s.compte.id;
    match reponse_bloquante(&r, move |r| signer_mandat(r, id, &otp, &nom_signe, &ip, &agent)).await.unwrap_or_else(|_| Err("Service momentanément indisponible.".into())) {
        Ok(()) => Redirect::to("/abonnement?ok=sepa").into_response(),
        Err(e) => vers_paiement_sepa(Some(&e)),
    }
}

/// `GET /abonnement/mandat.pdf` : le mandat en cours de signature, ou le mandat signé.
pub async fn mandat_pdf(State(r): State<Partage>, h: HeaderMap) -> Response {
    let s = match exiger(&r, &h, "/abonnement") {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    let id = s.compte.id;
    let resultat = reponse_bloquante(&r, move |r| -> Result<Vec<u8>, String> {
        let (erp, ab) = {
            let c = r.base();
            (Erp::depuis(&c).ok_or("ERP non réglé")?, lire(&c, id).map_err(|e| e.to_string())?)
        };
        let rum = rum_en_cours(&ab).or(ab.rum.clone()).ok_or("aucun mandat")?;
        let (statut, _, o) = erp.octets(&format!("/sepa-mandates/{}/pdf", encoder(&rum)))?;
        if (200..300).contains(&statut) { Ok(o) } else { Err(format!("HTTP {statut}")) }
    })
    .await
    .unwrap_or_else(|_| Err(String::new()));
    match resultat {
        Ok(o) => pdf(o, "mandat-sepa.pdf"),
        Err(e) => {
            eprintln!("mandat PDF : {e}");
            (StatusCode::NOT_FOUND, page("Mandat", Some(&s), r#"<div class="carte"><p class="erreur">Mandat indisponible.</p></div>"#)).into_response()
        }
    }
}

// ————— Changement d'offre, résiliation —————

/// Changement d'offre ou de cycle d'un abonnement en cours : nouvel abonnement payé avec le
/// moyen enregistré, puis résiliation de l'ancien. Bloquant.
pub fn changer_offre(r: &Partage, id: i64, offre: Offre, cycle: Cycle) -> Result<(), String> {
    let ab = lire(&r.base(), id).map_err(|e| e.to_string())?;
    if !ab.actif() {
        return Err("souscrire".into());
    }
    if ab.offre == offre && ab.cycle == cycle {
        return Ok(());
    }
    let paiement = match (&ab.rum, &ab.carte_id) {
        (Some(rum), _) => Some(json!({ "provider": "stancer_sepa", "mandate_code": rum })),
        (None, Some(_)) => None,
        (None, None) => return Err("paiement".into()),
    };
    regler_brouillon(&r.base(), id, |b| {
        b.insert("offre".into(), json!(offre.code()));
        b.insert("cycle".into(), json!(cycle.code()));
        b.insert("coupon".into(), json!(""));
        b.insert("partenaire".into(), json!(""));
        b.insert("cle".into(), json!(crate::aleatoire_hex(8)));
    })
    .map_err(|e| e.to_string())?;
    creer_abonnement(r, id, paiement.clone())?;
    if paiement.is_none()
        && let Some(carte) = ab.carte_id
    {
        let (erp, code) = {
            let c = r.base();
            (Erp::depuis(&c).ok_or("Facturation indisponible.")?, lire(&c, id).map_err(|e| e.to_string())?.subscription_code.unwrap_or_default())
        };
        let rep = erp.envoyer("PUT", &format!("/subscriptions/{}/payment-method", encoder(&code)), &json!({ "provider": "stancer_card", "card_id": carte }), None)?;
        if !rep.ok() {
            return Err(format!("Nouvelle offre enregistrée, mais la carte n'a pas été reprise : {}", rep.erreur()));
        }
    }
    Ok(())
}

/// `POST /abonnement/changer` (`offre`, `cycle`).
pub async fn changer(State(r): State<Partage>, h: HeaderMap, axum::Form(f): Champs) -> Response {
    if !origine_ok(&r, &h) {
        return refuse_origine();
    }
    let s = match exiger(&r, &h, "/abonnement") {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    if !csrf_ok(&s, &f) {
        return refuse_origine();
    }
    let Some((offre, cycle)) = offre_et_cycle(&champ(&f, "offre"), &champ(&f, "cycle")) else { return Redirect::to("/abonnement/offres").into_response() };
    if !r.limiter(&format!("changer:{}", s.compte.id), 10, Duration::from_secs(3600)) {
        return erreur_page(&r, Some(&s), "Trop d'essais : réessayer dans une heure.");
    }
    let id = s.compte.id;
    match reponse_bloquante(&r, move |r| changer_offre(r, id, offre, cycle)).await.unwrap_or_else(|_| Err("Service momentanément indisponible.".into())) {
        Ok(()) => Redirect::to("/abonnement?ok=change").into_response(),
        Err(e) if e == "souscrire" => Redirect::to(&format!("/abonnement/souscrire?offre={}&cycle={}", offre.code(), cycle.code())).into_response(),
        Err(e) if e == "paiement" => Redirect::to("/abonnement/paiement").into_response(),
        Err(e) => erreur_page(&r, Some(&s), &e),
    }
}

/// `POST /abonnement/resilier` : résiliation à la fin de la période en cours.
pub async fn resilier(State(r): State<Partage>, h: HeaderMap, axum::Form(f): Champs) -> Response {
    if !origine_ok(&r, &h) {
        return refuse_origine();
    }
    let s = match exiger(&r, &h, "/abonnement") {
        Ok(s) => s,
        Err(rep) => return *rep,
    };
    if !csrf_ok(&s, &f) || champ(&f, "confirmer") != "1" {
        return refuse_origine();
    }
    let id = s.compte.id;
    let resultat = reponse_bloquante(&r, move |r| -> Result<(), String> {
        let (erp, ab) = {
            let c = r.base();
            (Erp::depuis(&c).ok_or("Facturation indisponible.")?, lire(&c, id).map_err(|e| e.to_string())?)
        };
        let code = ab.subscription_code.clone().filter(|_| ab.actif()).ok_or("Aucun abonnement en cours.")?;
        for c in std::iter::once(&code).chain(ab.supplement_code.iter()) {
            let rep = erp.post(&format!("/subscriptions/{}/cancel", encoder(c)), &json!({ "at_period_end": true, "reason": "Résiliation demandée depuis le portail E-KO RIP Cloud" }), None)?;
            if !rep.ok() {
                return Err(format!("Résiliation refusée : {}", rep.erreur()));
            }
        }
        maj(&r.base(), id, &[("fin_periode_resiliation", entier(1))]).map_err(|e| e.to_string())?;
        eprintln!("compte {id} : résiliation à la fin de la période");
        Ok(())
    })
    .await
    .unwrap_or_else(|_| Err("Service momentanément indisponible.".into()));
    match resultat {
        Ok(()) => Redirect::to("/abonnement?ok=resilie").into_response(),
        Err(e) => erreur_page(&r, Some(&s), &e),
    }
}

// ————— Administration —————

const PLANS: [(Option<Offre>, &str); 4] = [(Some(Offre::Machine), "À la machine (par siège)"), (Some(Offre::Atelier), "Atelier"), (Some(Offre::Pro), "Pro"), (None, "Machines supplémentaires (par siège, Atelier et Pro)")];

/// Carte « Facturation (E-KO) » : ERP, webhooks, codes de plans, clé publique Stancer.
pub(crate) fn carte_admin(b: &Connection, s: &Session, url: &str) -> String {
    let valeur = |cle: &str| base::reglage(b, cle).ok().flatten().unwrap_or_default();
    let secret = |cle: &str| if valeur(cle).is_empty() { "non renseigné" } else { "enregistré (laisser vide pour le garder)" };
    let mut plans = String::new();
    for (o, nom) in PLANS {
        plans.push_str(&format!(
            r#"<span class="ligne"><label>{nom} — mensuel<input type="text" name="{m}" value="{vm}" autocomplete="off"></label><label>annuel<input type="text" name="{a}" value="{va}" autocomplete="off"></label></span>"#,
            m = offres::cle_plan(o, Cycle::Mensuel),
            a = offres::cle_plan(o, Cycle::Annuel),
            vm = echapper(&valeur(&offres::cle_plan(o, Cycle::Mensuel))),
            va = echapper(&valeur(&offres::cle_plan(o, Cycle::Annuel))),
        ));
    }
    format!(
        r#"<div class="carte"><h2>Facturation (E-KO)</h2>
<p class="meta">API v1 de l'ERP (tenant GOLDEN PRINT). Jeton avec les droits tiers.view, tiers.create, subscriptions.view, subscriptions.create, subscriptions.update, subscriptions.cancel, subscription-plans.view, apporteurs.view, apporteurs.link, sepa-mandates.view, sepa-mandates.create, invoices.view. Webhook à déclarer dans l'ERP : <code>{url}/api/erp/webhooks</code>.</p>
<form method="post" action="/admin/facturation">{csrf}
<label>Adresse de l'API<input type="text" name="erp_url" value="{erp}" placeholder="https://webapp.e-ko.fr/api/v1" autocomplete="off"></label>
<label>Jeton d'API ({j})<input type="password" name="erp_jeton" autocomplete="new-password"></label>
<label>Secret des webhooks ({w})<input type="password" name="erp_webhook_secret" autocomplete="new-password"></label>
<h2>Codes des plans d'abonnement</h2>{plans}
<label>Clé publique Stancer (pk_…)<input type="text" name="stancer_cle_publique" value="{pk}" autocomplete="off"></label>
<label class="case"><input type="checkbox" name="effacer" value="1"> <span>Effacer le jeton et le secret enregistrés</span></label>
<button type="submit">Enregistrer</button></form>
<form method="post" action="/admin/facturation/essai">{csrf}<button class="discret" type="submit">Tester la connexion</button> <span class="meta">liste les plans de l'ERP</span></form></div>"#,
        url = echapper(url),
        csrf = hidden_csrf(s),
        erp = echapper(&valeur("erp_url")),
        j = secret("erp_jeton"),
        w = secret("erp_webhook_secret"),
        pk = echapper(&valeur("stancer_cle_publique")),
    )
}

/// Vue d'ensemble : comptes et offres, sites en dépassement.
pub fn carte_vue_admin(b: &Connection, tous: &[sites::Site]) -> String {
    let lignes: Vec<(String, Abonnement, Parc)> = b
        .prepare("SELECT c.email, a.compte FROM abonnements_cloud a JOIN comptes c ON c.id = a.compte ORDER BY a.maj_le DESC LIMIT 200")
        .and_then(|mut q| q.query_map([], |l| Ok((l.get::<_, String>(0)?, l.get::<_, i64>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(email, id)| Some((email, lire(b, id).ok()?, parc(b, id).ok()?)))
        .collect();
    let html: String = lignes
        .iter()
        .map(|(email, ab, p)| {
            format!(
                "<tr><td>{}</td><td>{} {}</td><td>{}</td><td>{} site(s) · {} machine(s)</td><td>{}</td></tr>",
                echapper(email),
                ab.offre.nom(),
                if ab.actif() { ab.cycle.libelle() } else { "" },
                offres::libelle_statut(&ab.statut),
                p.sites,
                p.machines(),
                ab.depassement_depuis.map_or("—".to_string(), |d| format!("depuis le {} : {}", date(Some(d)), echapper(&ab.depassement_raison)))
            )
        })
        .collect();
    let suspendus: Vec<String> = b.prepare("SELECT site FROM sites_suspendus ORDER BY site").and_then(|mut q| q.query_map([], |l| l.get(0))?.collect()).unwrap_or_default();
    let liste = if suspendus.is_empty() { "aucun".to_string() } else { suspendus.iter().map(|s| format!("{} (<code>{}</code>)", echapper(&tous.iter().find(|x| &x.id == s).map_or(s.clone(), |x| x.nom.clone())), echapper(s))).collect::<Vec<_>>().join(", ") };
    format!(r#"<div class="carte"><h2>Abonnements</h2><p class="meta">Comptes ayant souscrit ou en dépassement d'offre (les autres sont au Gratuit). Sites à l'accès à distance suspendu : {liste}.</p><div class="tableau"><table><thead><tr><th>Compte</th><th>Offre</th><th>État</th><th>Parc</th><th>Dépassement</th></tr></thead><tbody>{html}</tbody></table></div></div>"#)
}

/// `POST /admin/facturation`.
pub async fn regler_facturation(State(r): State<Partage>, h: HeaderMap, axum::Form(f): Champs) -> Response {
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
    let url = champ(&f, "erp_url");
    if !url.is_empty() && !(url.starts_with("https://") || url.starts_with("http://")) || url.chars().any(char::is_whitespace) {
        return erreur_page(&r, Some(&s), "Adresse de l'API invalide (https://…).");
    }
    let pk = champ(&f, "stancer_cle_publique");
    if !pk.is_empty() && (!pk.starts_with("pk_") || !pk.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')) {
        return erreur_page(&r, Some(&s), "Clé publique Stancer invalide (pk_…).");
    }
    let mut cles: Vec<(String, String)> = vec![("erp_url".into(), url.trim_end_matches('/').to_string()), ("stancer_cle_publique".into(), pk)];
    for (o, _) in PLANS {
        for cy in [Cycle::Mensuel, Cycle::Annuel] {
            let k = offres::cle_plan(o, cy);
            let v = champ(&f, &k);
            if v.chars().any(|c| c.is_control()) || v.len() > 64 {
                return erreur_page(&r, Some(&s), "Code de plan invalide.");
            }
            cles.push((k, v));
        }
    }
    let b = r.base();
    let mut ok = true;
    for (k, v) in &cles {
        ok &= base::regler(&b, k, v).is_ok();
    }
    for cle in ["erp_jeton", "erp_webhook_secret"] {
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
    Redirect::to("/admin?ok=facturation").into_response()
}

/// `POST /admin/facturation/essai` : `GET /subscription-plans`.
pub async fn essai_facturation(State(r): State<Partage>, h: HeaderMap, axum::Form(f): Champs) -> Response {
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
    let resultat = reponse_bloquante(&r, |r| -> Result<Vec<String>, String> {
        let erp = Erp::depuis(&r.base()).ok_or("Adresse ou jeton de l'API non renseigné.")?;
        let rep = erp.get("/subscription-plans")?;
        if !rep.ok() {
            return Err(rep.erreur());
        }
        Ok(rep.donnees().as_array().cloned().unwrap_or_default().iter().map(|p| format!("{} — {} ({}, {} {}, {})", texte_json(&p["code"]).unwrap_or_default(), p["name"].as_str().unwrap_or(""), p["pricing_mode"].as_str().unwrap_or(""), texte_json(&p["unit_price_ht"]).unwrap_or_default(), p["currency"].as_str().unwrap_or(""), p["billing_cycle"].as_str().unwrap_or(""))).collect())
    })
    .await
    .unwrap_or_else(|_| Err("Service momentanément indisponible.".into()));
    let corps = match resultat {
        Ok(plans) => format!(r#"<div class="carte"><h2>Connexion à l'ERP réussie</h2><p class="ok">{} plan(s) d'abonnement trouvé(s).</p><ul>{}</ul><p><a href="/admin">Retour</a></p></div>"#, plans.len(), plans.iter().map(|p| format!("<li><code>{}</code></li>", echapper(p))).collect::<String>()),
        Err(e) => format!(r#"<div class="carte"><h2>Connexion à l'ERP impossible</h2><p class="erreur">{}</p><p><a href="/admin">Retour</a></p></div>"#, echapper(&e)),
    };
    page("Administration", Some(&s), &format!("<h1>Administration</h1>{corps}")).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fiche_et_coupons() {
        let f = fiche_entreprise(&json!({ "siren": "939018024", "identity": { "name": "GOLDEN PRINT", "siret": "93901802400017", "vat_intracom": "FR66939018024", "address": { "line1": "14 Route d'Espas", "postal_code": "32370", "city": "Manciet" } } })).unwrap();
        assert_eq!((f["raison_sociale"].as_str(), f["ville"].as_str(), f["tva"].as_str()), (Some("GOLDEN PRINT"), Some("Manciet"), Some("FR66939018024")));
        assert!(fiche_entreprise(&json!({ "siren": "1", "identity": null })).is_none());
        assert_eq!(libelle_coupon(&json!({ "kind": "discount_percent", "value": 20, "is_recurring": true, "name": "Bienvenue" })), "Code promo : 20 % de remise sur chaque facture (Bienvenue)");
        assert_eq!(libelle_coupon(&json!({ "kind": "free_period", "value": "2" })), "Code promo : 2 mois offerts");
    }

    #[test]
    fn ligne_par_defaut_et_maj() {
        let d = std::env::temp_dir().join(format!("eko-cloud-abo-{}", crate::aleatoire_hex(6)));
        let c = base::ouvrir(&d).unwrap();
        let id = base::creer_compte(&c, "a@b.fr", "A", None).unwrap().unwrap();
        assert_eq!(lire(&c, id).unwrap().offre, Offre::Gratuit);
        maj(&c, id, &[("offre", texte("pro")), ("subscription_code", texte("SUB-1")), ("statut", texte("active"))]).unwrap();
        let ab = lire(&c, id).unwrap();
        assert!(ab.actif() && ab.offre == Offre::Pro);
        assert_eq!(compte_par_abonnement(&c, "SUB-1").unwrap(), Some(id));
        regler_brouillon(&c, id, |b| {
            b.insert("offre".into(), json!("atelier"));
        })
        .unwrap();
        assert_eq!(lire(&c, id).unwrap().texte_brouillon("offre"), "atelier");
        base::regler(&c, "plan_pro_annuel", "PRO-A").unwrap();
        appliquer(&c, id, &json!({ "code": "SUB-2", "status": "trial", "seats": 2, "period": { "current_end": "2026-12-01", "trial_ends_at": "2026-12-01T00:00:00Z" }, "plan": { "code": "PRO-A" } })).unwrap();
        let ab = lire(&c, id).unwrap();
        assert_eq!((ab.subscription_code.as_deref(), ab.statut.as_str(), ab.cycle, ab.seats), (Some("SUB-2"), "trial", Cycle::Annuel, 2));
        assert_eq!(ab.essai_fin, offres::date_iso("2026-12-01"));
        let _ = std::fs::remove_dir_all(&d);
    }
}
