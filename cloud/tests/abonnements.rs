//! Abonnements, facturation et sauvegardes de bout en bout, avec un faux ERP (API v1) et un
//! faux stockage S3 (axum sur 127.0.0.1), réglés par les formulaires d'Administration.

use axum::body::Bytes;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use eko_rip_cloud::{abonnements, base, client, erp, relais, sauvegardes, sites};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const URL: &str = "http://plateforme.test";

// ————— Faux ERP —————

#[derive(Clone, Debug)]
struct Appel {
    methode: String,
    chemin: String,
    corps: Value,
    autorisation: String,
}

#[derive(Default)]
struct EtatErp {
    appels: Vec<Appel>,
    abonnements: u32,
    suivis: u32,
    /// Mandats SEPA : RUM -> (tiers, signé).
    mandats: HashMap<String, (String, bool)>,
}

type Erp = Arc<Mutex<EtatErp>>;

const PDF: &[u8] = b"%PDF-1.4 document de test";

async fn faux_erp(State(e): State<Erp>, q: Request) -> Response {
    let methode = q.method().to_string();
    let chemin = q.uri().path().trim_start_matches("/api/v1").to_string();
    let requete = q.uri().query().unwrap_or("").to_string();
    let autorisation = q.headers().get("authorization").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    let octets = axum::body::to_bytes(q.into_body(), 1 << 20).await.unwrap();
    let corps: Value = serde_json::from_slice(&octets).unwrap_or(Value::Null);
    let mut e = e.lock().unwrap();
    e.appels.push(Appel { methode: methode.clone(), chemin: chemin.clone(), corps: corps.clone(), autorisation: autorisation.clone() });
    if autorisation != "Bearer jeton-secret-erp" {
        return (StatusCode::UNAUTHORIZED, axum::Json(json!({ "error": "unauthenticated" }))).into_response();
    }
    let p: Vec<&str> = chemin.trim_start_matches('/').split('/').collect();
    let j = |s: StatusCode, v: Value| (s, axum::Json(v)).into_response();
    match (methode.as_str(), p.as_slice()) {
        ("GET", ["companies", "939018024"]) => j(StatusCode::OK, json!({ "data": { "siren": "939018024", "identity": { "name": "GOLDEN PRINT", "siret": "93901802400017", "vat_intracom": "FR66939018024", "address": { "line1": "14 Route d'Espas", "postal_code": "32370", "city": "Manciet" } } } })),
        ("GET", ["companies", _]) => j(StatusCode::UNPROCESSABLE_ENTITY, json!({ "error": "siren.invalid" })),
        ("GET", ["tiers"]) => j(StatusCode::OK, json!({ "data": if requete.contains("552100554") { json!([{ "code": "CLI-EXISTANT", "name": "Existant" }]) } else { json!([]) } })),
        ("POST", ["tiers"]) => j(StatusCode::CREATED, json!({ "data": { "code": "CLI-1", "name": corps["name"] } })),
        ("GET", ["subscription-plans"]) => j(StatusCode::OK, json!({ "data": [{ "code": "MACH-M", "name": "À la machine", "pricing_mode": "per_seat", "unit_price_ht": "6.00", "currency": "EUR", "billing_cycle": "monthly", "trial_days": 30 }, { "code": "ATEL-M", "name": "Atelier", "pricing_mode": "flat", "unit_price_ht": "19.00", "currency": "EUR", "billing_cycle": "monthly", "trial_days": 30 }] })),
        ("GET", ["subscription-coupons", "BIENVENUE"]) => j(StatusCode::OK, json!({ "data": { "code": "BIENVENUE", "name": "Bienvenue", "kind": "discount_percent", "value": 20, "currency": "EUR", "is_recurring": false, "valid_until": null } })),
        ("GET", ["subscription-coupons", "EXPIRE"]) => j(StatusCode::UNPROCESSABLE_ENTITY, json!({ "error": "coupon.expired" })),
        ("GET", ["subscription-coupons", _]) => j(StatusCode::NOT_FOUND, json!({ "error": "not_found" })),
        ("GET", ["partner-codes", "PARTENAIRE"]) => j(StatusCode::OK, json!({ "data": { "code": "PARTENAIRE", "partner": { "tier_code": "APP-1", "name": "Imprimerie Partenaire" } } })),
        ("GET", ["partner-codes", _]) => j(StatusCode::NOT_FOUND, json!({ "error": "not_found" })),
        ("POST", ["subscriptions"]) => {
            // Contrat : un mandat SEPA en moyen de paiement doit être signé et du même tiers.
            if corps["payment_method"]["provider"] == "stancer_sepa" && e.mandats.get(corps["payment_method"]["mandate_code"].as_str().unwrap_or("")) != Some(&(corps["tier_code"].as_str().unwrap_or("").to_string(), true)) {
                return j(StatusCode::UNPROCESSABLE_ENTITY, json!({ "error": "payment_method.mandate_invalid" }));
            }
            e.abonnements += 1;
            let code = format!("SUB-{}", e.abonnements);
            let premier = if corps["bill_now"] == true { json!({ "invoice_id": 100 + e.abonnements, "invoice_code": format!("FA-{}", e.abonnements) }) } else { Value::Null };
            j(StatusCode::CREATED, json!({ "data": { "code": code, "status": "trial", "seats": corps["seats"].as_i64().unwrap_or(1), "period": { "current_start": "2026-11-01", "current_end": "2026-12-01", "trial_ends_at": "2026-12-01T00:00:00Z" }, "plan": { "code": corps["plan_code"], "name": "Plan" }, "first_cycle": premier } }))
        }
        ("GET", ["subscriptions", code]) => j(StatusCode::OK, json!({ "data": { "code": code, "status": "active", "period": { "current_end": "2026-12-01", "trial_ends_at": null } } })),
        ("PATCH", ["subscriptions", code]) => j(StatusCode::OK, json!({ "data": { "code": code, "seats": corps["seats"] } })),
        ("PUT", ["subscriptions", _, "payment-method"]) => j(StatusCode::OK, json!({ "data": { "provider": corps["provider"] } })),
        ("POST", ["subscriptions", _, "card-authentication"]) => j(StatusCode::CREATED, json!({ "data": { "payment_id": "paym_1", "status": "pending", "redirect_url": "https://3ds.stancer.test/auth/paym_1", "amount": 0, "currency": "eur" } })),
        ("GET", ["subscriptions", _, "card-authentication", "paym_1"]) => {
            e.suivis += 1;
            j(StatusCode::OK, json!({ "data": { "status": if e.suivis > 1 { "authorized" } else { "pending" }, "paid": false, "authorized": e.suivis > 1 } }))
        }
        ("GET", ["subscriptions", code, "billing-cycles"]) => {
            // SUB-1 : facture FA-1 ; autres abonnements : facture FA-<code>.
            let facture = if *code == "SUB-1" { "FA-1".to_string() } else { format!("FA-{code}") };
            j(StatusCode::OK, json!({ "data": [{ "period_start": "2026-11-01", "period_end": "2026-12-01", "status": "paid", "total_ht": "19.00", "total_ttc": "22.80", "invoice": { "code": facture, "reference": format!("REF-{facture}") }, "failure_reason": null }] }))
        }
        ("POST", ["subscriptions", "SUP-PANNE", "cancel"]) => j(StatusCode::SERVICE_UNAVAILABLE, json!({ "error": "maintenance" })),
        ("POST", ["subscriptions", _, "cancel"]) => j(StatusCode::OK, json!({ "data": { "status": "active" } })),
        ("GET", ["invoices", _, "pdf"]) | ("GET", ["sepa-mandates", _, "pdf"]) => ([("content-type", "application/pdf")], PDF).into_response(),
        ("POST", ["sepa-mandates"]) => {
            e.mandats.insert("RUM-1".into(), (corps["tier_code"].as_str().unwrap_or("").to_string(), false));
            j(StatusCode::CREATED, json!({ "data": { "rum": "RUM-1", "status": "pending_signature", "iban_last4": "0189", "pdf_url": "https://erp.test/mandat.pdf" } }))
        }
        ("POST", ["sepa-mandates", _, "signature", "otp"]) => j(StatusCode::ACCEPTED, json!({ "data": { "sent": true } })),
        ("POST", ["sepa-mandates", _, "signature", "verify"]) => if corps["otp"] == "123456" { j(StatusCode::OK, json!({ "data": { "verified": true } })) } else { j(StatusCode::UNPROCESSABLE_ENTITY, json!({ "error": "otp.invalid" })) },
        ("POST", ["sepa-mandates", rum, "signature", "finalize"]) => {
            if let Some(m) = e.mandats.get_mut(*rum) {
                m.1 = true;
            }
            j(StatusCode::OK, json!({ "data": { "rum": rum, "status": "signed" } }))
        }
        _ => j(StatusCode::NOT_FOUND, json!({ "error": "route inconnue du faux ERP" })),
    }
}

// ————— Faux S3 —————

#[derive(Default)]
struct EtatS3 {
    objets: HashMap<String, Vec<u8>>,
    requetes: Vec<(String, String, bool)>,
}

type S3 = Arc<Mutex<EtatS3>>;

async fn faux_s3(State(e): State<S3>, h: HeaderMap, methode: Method, uri: axum::http::Uri, corps: Bytes) -> Response {
    let auth = h.get("authorization").and_then(|v| v.to_str().ok()).unwrap_or("");
    let forme = auth.starts_with("AWS4-HMAC-SHA256 Credential=AK-ESSAI/") && auth.contains("/eu-west-par/s3/aws4_request,SignedHeaders=host;x-amz-content-sha256;x-amz-date") && auth.split("Signature=").nth(1).is_some_and(|s| s.len() == 64);
    let date = h.get("x-amz-date").is_some();
    let empreinte = h.get("x-amz-content-sha256").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    let chemin = uri.path().to_string();
    let mut e = e.lock().unwrap();
    e.requetes.push((methode.to_string(), chemin.clone(), h.get("x-amz-server-side-encryption").is_some()));
    if !forme || !date {
        return (StatusCode::FORBIDDEN, "SignatureDoesNotMatch").into_response();
    }
    match methode {
        Method::PUT => {
            if empreinte != eko_rip_cloud::sha256_octets_hex(&corps) {
                return (StatusCode::BAD_REQUEST, "XAmzContentSHA256Mismatch").into_response();
            }
            e.objets.insert(chemin, corps.to_vec());
            StatusCode::OK.into_response()
        }
        Method::GET => match e.objets.get(&chemin) {
            Some(o) => o.clone().into_response(),
            None => StatusCode::NOT_FOUND.into_response(),
        },
        Method::DELETE => {
            e.objets.remove(&chemin);
            StatusCode::NO_CONTENT.into_response()
        }
        _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
    }
}

// ————— Plate-forme —————

type Boite = Arc<Mutex<Vec<eko_rip_cloud::courriels::Courriel>>>;

struct Banc {
    base: String,
    donnees: std::path::PathBuf,
    r: relais::Partage,
    boite: Boite,
    erp: Erp,
    s3: S3,
    url_erp: String,
    url_s3: String,
}

async fn servir(app: axum::Router) -> String {
    let ecoute = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let adresse = format!("http://{}", ecoute.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(ecoute, app).await.unwrap() });
    adresse
}

async fn banc() -> Arc<Banc> {
    let donnees = std::env::temp_dir().join(format!("eko-cloud-abo-{}", eko_rip_cloud::aleatoire_hex(6)));
    let r = relais::nouveau(relais::Config { donnees: donnees.clone(), url_publique: URL.into(), max_octets: 1 << 20, jeton_stats: String::new() });
    let boite = r.utiliser_memoire();
    let _ = r.utiliser_memoire_push();
    let base = servir(relais::routes(r.clone())).await;
    let erp: Erp = Default::default();
    let url_erp = format!("{}/api/v1", servir(axum::Router::new().fallback(faux_erp).with_state(erp.clone())).await);
    let s3: S3 = Default::default();
    let url_s3 = servir(axum::Router::new().fallback(faux_s3).with_state(s3.clone()).layer(axum::extract::DefaultBodyLimit::max(1 << 26))).await;
    Arc::new(Banc { base, donnees, r, boite, erp, s3, url_erp, url_s3 })
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder().http_status_as_error(false).max_redirects(0).timeout_global(Some(Duration::from_secs(20))).build().into()
}

struct Rep {
    statut: u16,
    corps: String,
    octets: Vec<u8>,
    lieu: String,
    entetes: HeaderMap,
}

impl Rep {
    fn json(&self) -> Value {
        serde_json::from_slice(&self.octets).unwrap_or(Value::Null)
    }
}

fn lire(r: Result<ureq::http::Response<ureq::Body>, ureq::Error>) -> Rep {
    let mut r = r.unwrap();
    let lieu = r.headers().get("location").map(|v| v.to_str().unwrap().to_string()).unwrap_or_default();
    let entetes = r.headers().clone();
    let octets = r.body_mut().with_config().limit(1 << 26).read_to_vec().unwrap_or_default();
    Rep { statut: r.status().as_u16(), corps: String::from_utf8_lossy(&octets).to_string(), octets, lieu, entetes }
}

/// Requête quelconque : en-têtes, corps JSON ou formulaire ou octets.
enum Corps<'a> {
    Rien,
    Formulaire(&'a [(&'a str, &'a str)]),
    Json(Value),
    Octets(Vec<u8>),
}

fn appel(b: &Banc, methode: &str, chemin: &str, entetes: &[(&str, &str)], corps: Corps) -> Rep {
    let url = format!("{}{chemin}", b.base);
    let a = agent();
    macro_rules! avec {
        ($q:expr) => {{
            let mut q = $q;
            for (n, v) in entetes {
                q = q.header(*n, *v);
            }
            q
        }};
    }
    match (methode, corps) {
        ("GET", _) => lire(avec!(a.get(&url)).call()),
        ("POST", Corps::Formulaire(f)) => lire(avec!(a.post(&url)).send_form(f.iter().copied())),
        ("POST", Corps::Json(v)) => lire(avec!(a.post(&url)).send_json(v)),
        ("POST", Corps::Octets(o)) => lire(avec!(a.post(&url)).send(&o[..])),
        ("POST", Corps::Rien) => lire(avec!(a.post(&url)).send_empty()),
        _ => panic!("méthode"),
    }
}

async fn bl<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    tokio::task::spawn_blocking(f).await.unwrap()
}

fn cookie(r: &Rep) -> String {
    r.entetes.get_all("set-cookie").iter().map(|v| v.to_str().unwrap()).find(|c| c.starts_with("plateforme_session=")).expect("session").split(';').next().unwrap().to_string()
}

fn csrf(page: &str) -> String {
    let i = page.find("name=\"csrf\" value=\"").expect("jeton csrf") + 19;
    page[i..i + 32].to_string()
}

/// Compte inscrit et confirmé : (cookie, csrf, id).
fn inscrire(b: &Banc, email: &str) -> (String, String, i64) {
    let r = appel(b, "POST", "/inscription", &[("Origin", URL)], Corps::Formulaire(&[("email", email), ("nom", "Claire Dupont"), ("mot_de_passe", "phrase-longue-1"), ("cgu", "1")]));
    assert_eq!(r.statut, 303, "{}", r.corps);
    let c = cookie(&r);
    let id = {
        let cx = b.r.base();
        let id = base::compte_par_email(&cx, email).unwrap().unwrap().id;
        base::marquer_email_verifie(&cx, id).unwrap();
        id
    };
    let page = appel(b, "GET", "/compte", &[("Cookie", &c)], Corps::Rien);
    (c, csrf(&page.corps), id)
}

fn admin(b: &Banc) -> (String, String) {
    let (c, j, _) = inscrire(b, "admin@e-ko.fr");
    base::nommer_super_admin(&b.r.base(), "admin@e-ko.fr").unwrap();
    (c, j)
}

/// Facturation et stockage réglés par les formulaires d'Administration.
fn regler(b: &Banc) -> (String, String) {
    let (c, j) = admin(b);
    let r = appel(b, "POST", "/admin/facturation", &[("Origin", URL), ("Cookie", &c)], Corps::Formulaire(&[("csrf", &j), ("erp_url", &b.url_erp), ("erp_jeton", "jeton-secret-erp"), ("erp_webhook_secret", "secret-webhook"), ("plan_machine_mensuel", "MACH-M"), ("plan_atelier_mensuel", "ATEL-M"), ("plan_pro_mensuel", "PRO-M"), ("plan_pro_annuel", "PRO-A"), ("plan_supplement_mensuel", "SUP-M"), ("stancer_cle_publique", "pk_test_123")]));
    assert_eq!(r.lieu, "/admin?ok=facturation", "{}", r.corps);
    let r = appel(b, "POST", "/admin/stockage", &[("Origin", URL), ("Cookie", &c)], Corps::Formulaire(&[("csrf", &j), ("s3_endpoint", &b.url_s3), ("s3_region", "eu-west-par"), ("s3_bucket", "seau"), ("s3_acces", "AK-ESSAI"), ("s3_secret", "secret-s3-tres-prive"), ("s3_sse", "1"), ("s3_base_bucket", "copies")]));
    assert_eq!(r.lieu, "/admin?ok=stockage", "{}", r.corps);
    (c, j)
}

fn appels(b: &Banc, methode: &str, chemin: &str) -> Vec<Appel> {
    b.erp.lock().unwrap().appels.iter().filter(|a| a.methode == methode && a.chemin.split('?').next() == Some(chemin)).cloned().collect()
}

fn sujets(b: &Banc) -> Vec<String> {
    b.boite.lock().unwrap().iter().map(|c| c.sujet.clone()).collect()
}

/// Site rattaché à `compte` : (id, clé).
fn site_de(b: &Banc, compte: i64, nom: &str) -> (String, String) {
    let (s, cle) = sites::creer(&b.donnees, nom, None).unwrap();
    base::donner_droit(&b.r.base(), &s.id, compte, "proprietaire").unwrap();
    (s.id, cle)
}

fn inventaire(b: &Banc, site: &str, cle: &str, imprimantes: usize, decoupeurs: usize) -> Rep {
    let m = |n: usize, genre: &str| (0..n).map(|i| json!({ "id": i, "nom": format!("{genre} {i}"), "modele": "Modèle" })).collect::<Vec<_>>();
    appel(b, "POST", "/api/site/inventaire", &[("Authorization", &format!("Bearer {site}.{cle}"))], Corps::Json(json!({ "version": "0.32.0", "imprimantes": m(imprimantes, "Imprimante"), "decoupeurs": m(decoupeurs, "Découpeur") })))
}

#[tokio::test(flavor = "multi_thread")]
async fn administration_secrets_et_essais() {
    let b = banc().await;
    let b2 = b.clone();
    bl(move || {
        let b = b2;
        let (c, j) = regler(&b);
        let page = appel(&b, "GET", "/admin", &[("Cookie", &c)], Corps::Rien);
        assert!(page.corps.contains("MACH-M") && page.corps.contains("pk_test_123") && page.corps.contains(&b.url_erp) && page.corps.contains("AK-ESSAI"), "réglages visibles");
        for secret in ["jeton-secret-erp", "secret-webhook", "secret-s3-tres-prive"] {
            assert!(!page.corps.contains(secret), "secret {secret} réaffiché");
        }
        assert!(page.corps.contains(&format!("{URL}/api/erp/webhooks")), "adresse du webhook affichée");
        // Un formulaire sans secret garde les secrets enregistrés.
        let r = appel(&b, "POST", "/admin/facturation", &[("Origin", URL), ("Cookie", &c)], Corps::Formulaire(&[("csrf", &j), ("erp_url", &b.url_erp), ("plan_machine_mensuel", "MACH-M"), ("plan_atelier_mensuel", "ATEL-M"), ("plan_supplement_mensuel", "SUP-M"), ("stancer_cle_publique", "pk_test_123")]));
        assert_eq!(r.statut, 303);
        let essai = appel(&b, "POST", "/admin/facturation/essai", &[("Origin", URL), ("Cookie", &c)], Corps::Formulaire(&[("csrf", &j)]));
        assert!(essai.corps.contains("Connexion à l&#39;ERP réussie") || essai.corps.contains("Connexion à l'ERP réussie"), "{}", essai.corps);
        assert!(essai.corps.contains("MACH-M") && essai.corps.contains("ATEL-M"));
        assert!(appels(&b, "GET", "/subscription-plans").iter().all(|a| a.autorisation == "Bearer jeton-secret-erp"));
        // Essai du stockage : PUT, GET, DELETE signés, chiffrement côté serveur demandé.
        let essai = appel(&b, "POST", "/admin/stockage/essai", &[("Origin", URL), ("Cookie", &c)], Corps::Formulaire(&[("csrf", &j)]));
        assert!(essai.corps.contains("Stockage S3 opérationnel"), "{}", essai.corps);
        let req = b.s3.lock().unwrap().requetes.clone();
        assert!(req.iter().any(|(m, p, sse)| m == "PUT" && p.starts_with("/seau/essais/") && *sse));
        assert!(req.iter().any(|(m, p, _)| m == "PUT" && p.starts_with("/copies/essais/")), "bucket de la copie de la base essayé aussi");
        assert!(req.iter().any(|(m, _, _)| m == "GET") && req.iter().any(|(m, _, _)| m == "DELETE"));
        assert!(b.s3.lock().unwrap().objets.is_empty(), "objet d'essai supprimé");
        // Effacer : le jeton disparaît, l'essai échoue proprement.
        appel(&b, "POST", "/admin/facturation", &[("Origin", URL), ("Cookie", &c)], Corps::Formulaire(&[("csrf", &j), ("erp_url", &b.url_erp), ("effacer", "1")]));
        let essai = appel(&b, "POST", "/admin/facturation/essai", &[("Origin", URL), ("Cookie", &c)], Corps::Formulaire(&[("csrf", &j)]));
        assert!(essai.corps.contains("impossible"));
        // Pages publiques : CGV, offres, accueil.
        let cgv = appel(&b, "GET", "/cgv", &[], Corps::Rien);
        assert!(cgv.statut == 200 && cgv.corps.contains("Conditions générales de vente") && cgv.corps.contains("939 018 024") && cgv.corps.contains("40 €"));
        let offres = appel(&b, "GET", "/abonnement/offres?cycle=annuel", &[], Corps::Rien);
        assert!(offres.corps.contains("190,00 €") && offres.corps.contains("Sur devis") && offres.corps.contains("bientôt"), "{}", offres.corps);
        let accueil = appel(&b, "GET", "/", &[], Corps::Rien);
        assert!(accueil.corps.contains("/abonnement/offres") && !accueil.corps.contains("Service gratuit."));
        assert!(accueil.corps.contains(r#"href="/cgv""#), "lien CGV dans le pied de page");
    })
    .await;
    let _ = std::fs::remove_dir_all(&b.donnees);
}

#[tokio::test(flavor = "multi_thread")]
async fn souscription_par_prelevement_sepa() {
    let b = banc().await;
    let b2 = b.clone();
    bl(move || {
        let b = b2;
        regler(&b);
        let (c, j, id) = inscrire(&b, "chef@atelier.fr");
        // Recherche par SIREN (proxy de l'ERP) : jeton anti-falsification exigé.
        let r = appel(&b, "GET", "/abonnement/entreprise?siren=939%20018%20024", &[("Cookie", &c)], Corps::Rien);
        assert_eq!(r.statut, 403);
        let r = appel(&b, "GET", "/abonnement/entreprise?siren=939%20018%20024", &[("Cookie", &c), ("X-CSRF-Token", &j)], Corps::Rien);
        assert_eq!((r.statut, r.json()["raison_sociale"].as_str(), r.json()["ville"].as_str()), (200, Some("GOLDEN PRINT"), Some("Manciet")), "{}", r.corps);
        let r = appel(&b, "GET", "/abonnement/entreprise?siren=123456789", &[("Cookie", &c), ("X-CSRF-Token", &j)], Corps::Rien);
        assert_eq!(r.statut, 400);
        // Code promo, puis partenaire, puis inconnu.
        let code = |v: &str| appel(&b, "GET", &format!("/abonnement/code?code={v}&offre=atelier&cycle=mensuel"), &[("Cookie", &c), ("X-CSRF-Token", &j)], Corps::Rien).json();
        let promo = code("BIENVENUE");
        assert_eq!((promo["valide"].as_bool(), promo["type"].as_str()), (Some(true), Some("coupon")));
        assert!(promo["libelle"].as_str().unwrap().contains("20 % de remise"));
        let part = code("PARTENAIRE");
        assert_eq!((part["type"].as_str(), part["libelle"].as_str()), (Some("partenaire"), Some("Partenaire : Imprimerie Partenaire")));
        assert_eq!(code("INCONNU")["valide"], false);
        assert!(code("EXPIRE")["message"].as_str().unwrap().contains("expiré"));
        assert!(appels(&b, "GET", "/subscription-coupons/BIENVENUE")[0].chemin.contains("BIENVENUE"));
        // Formulaire de souscription ; sans CGV : refusé.
        let page = appel(&b, "GET", "/abonnement/souscrire?offre=atelier&cycle=mensuel", &[("Cookie", &c)], Corps::Rien);
        assert!(page.corps.contains("Offre Atelier") && page.corps.contains("chef@atelier.fr") && page.corps.contains("Code promo ou partenaire") && page.corps.contains(r#"href="/cgv""#), "{}", page.corps);
        let mut champs = vec![("csrf", j.as_str()), ("offre", "atelier"), ("cycle", "mensuel"), ("pays", "FR"), ("siren", "939018024"), ("raison_sociale", "GOLDEN PRINT"), ("siret", "93901802400017"), ("tva", "FR66939018024"), ("contact", "Claire Dupont"), ("email", "compta@atelier.fr"), ("telephone", "06 12 34 56 78"), ("bon_commande", "BC-42"), ("code", "PARTENAIRE"), ("moyen", "sepa")];
        let r = appel(&b, "POST", "/abonnement/souscrire", &[("Origin", URL), ("Cookie", &c)], Corps::Formulaire(&champs));
        assert!(r.statut == 400 && r.corps.contains("conditions générales de vente"));
        champs.push(("cgv", "1"));
        let r = appel(&b, "POST", "/abonnement/souscrire", &[("Origin", URL), ("Cookie", &c)], Corps::Formulaire(&champs));
        assert_eq!((r.statut, r.lieu.as_str()), (303, "/abonnement/paiement?moyen=sepa"), "{}", r.corps);
        let tiers = appels(&b, "POST", "/tiers");
        assert_eq!(tiers.len(), 1, "tiers inconnu par SIREN : créé");
        assert_eq!((tiers[0].corps["siren"].as_str(), tiers[0].corps["phone"].as_str(), tiers[0].corps["is_customer"].as_bool()), (Some("939018024"), Some("+33612345678"), Some(true)));
        assert!(appels(&b, "POST", "/subscriptions").is_empty(), "SEPA : l'abonnement attend le mandat signé");
        // Page de paiement : politique de sécurité propre, formulaire IBAN.
        let page = appel(&b, "GET", "/abonnement/paiement?moyen=sepa", &[("Cookie", &c)], Corps::Rien);
        let csp = page.entetes.get("content-security-policy").unwrap().to_str().unwrap().to_string();
        assert!(csp.contains("https://js.stancer.com") && csp.contains("frame-src 'self' https:"), "{csp}");
        assert!(page.corps.contains(r#"name="iban""#));
        let r = appel(&b, "POST", "/abonnement/paiement/sepa", &[("Origin", URL), ("Cookie", &c)], Corps::Formulaire(&[("csrf", &j), ("iban", "FR76 3000 6000 0112 3456 7890 188"), ("prenom", "Claire"), ("nom", "Dupont"), ("telephone", "0612345678")]));
        assert!(r.lieu.contains("erreur=IBAN"), "IBAN invalide refusé : {}", r.lieu);
        let r = appel(&b, "POST", "/abonnement/paiement/sepa", &[("Origin", URL), ("Cookie", &c)], Corps::Formulaire(&[("csrf", &j), ("iban", "FR76 3000 6000 0112 3456 7890 189"), ("bic", "BNPAFRPP"), ("prenom", "Claire"), ("nom", "Dupont"), ("telephone", "0612345678"), ("adresse", "1 rue de l'Atelier"), ("code_postal", "69001"), ("ville", "Lyon")]));
        assert_eq!(r.lieu, "/abonnement/paiement?moyen=sepa");
        let mandat = &appels(&b, "POST", "/sepa-mandates")[0];
        assert_eq!((mandat.corps["tier_code"].as_str(), mandat.corps["iban"].as_str(), mandat.corps["phone"].as_str(), mandat.corps["email"].as_str()), (Some("CLI-1"), Some("FR7630006000011234567890189"), Some("+33612345678"), Some("compta@atelier.fr")));
        let page = appel(&b, "GET", "/abonnement/paiement?moyen=sepa", &[("Cookie", &c)], Corps::Rien);
        assert!(page.corps.contains("RUM-1") && page.corps.contains(r#"src="/abonnement/mandat.pdf""#) && page.corps.contains("Recevoir le code par SMS"), "{}", page.corps);
        let pdf = appel(&b, "GET", "/abonnement/mandat.pdf", &[("Cookie", &c)], Corps::Rien);
        assert_eq!((pdf.statut, pdf.octets.as_slice()), (200, PDF));
        assert!(pdf.entetes.get("content-security-policy").unwrap().to_str().unwrap().contains("frame-ancestors 'self'"), "mandat affichable dans la page de paiement");
        let r = appel(&b, "POST", "/abonnement/paiement/sepa/code", &[("Origin", URL), ("Cookie", &c)], Corps::Formulaire(&[("csrf", &j)]));
        assert_eq!(r.lieu, "/abonnement/paiement?moyen=sepa");
        assert_eq!(appels(&b, "POST", "/sepa-mandates/RUM-1/signature/otp").len(), 1);
        // Mauvais code, puis bon code : finalisation avec l'IP et le navigateur du signataire.
        let signer = |otp: &str| appel(&b, "POST", "/abonnement/paiement/sepa/signer", &[("Origin", URL), ("Cookie", &c), ("X-Forwarded-For", "10.0.0.1, 198.51.100.7"), ("User-Agent", "Navigateur de test")], Corps::Formulaire(&[("csrf", &j), ("otp", otp), ("nom_signe", "Claire Dupont"), ("consentement", "1")]));
        let r = signer("000000");
        assert!(r.lieu.contains("erreur=Code%20incorrect"), "{}", r.lieu);
        // Un mandat ne vaut que pour son tiers : tiers changé entre-temps, mandat à refaire.
        b.r.base().execute("UPDATE abonnements_cloud SET tier_code = 'CLI-AUTRE' WHERE compte = ?1", [id]).unwrap();
        let r = signer("123456");
        assert!(r.lieu.contains("erreur=Les%20informations%20de%20facturation%20ont%20chang"), "{}", r.lieu);
        assert!(appels(&b, "POST", "/sepa-mandates/RUM-1/signature/verify").len() == 1 && abonnements::lire(&b.r.base(), id).unwrap().brouillon.get("mandat").is_none());
        b.r.base().execute("UPDATE abonnements_cloud SET tier_code = 'CLI-1' WHERE compte = ?1", [id]).unwrap();
        let r = appel(&b, "POST", "/abonnement/paiement/sepa", &[("Origin", URL), ("Cookie", &c)], Corps::Formulaire(&[("csrf", &j), ("iban", "FR76 3000 6000 0112 3456 7890 189"), ("bic", "BNPAFRPP"), ("prenom", "Claire"), ("nom", "Dupont"), ("telephone", "0612345678"), ("adresse", "1 rue de l'Atelier"), ("code_postal", "69001"), ("ville", "Lyon")]));
        assert_eq!(r.lieu, "/abonnement/paiement?moyen=sepa");
        let r = signer("123456");
        assert_eq!(r.lieu, "/abonnement?ok=sepa");
        let fin = &appels(&b, "POST", "/sepa-mandates/RUM-1/signature/finalize")[0];
        assert_eq!((fin.corps["signer_ip"].as_str(), fin.corps["signer_user_agent"].as_str(), fin.corps["typed_name"].as_str(), fin.corps["consent"].as_bool()), (Some("198.51.100.7"), Some("Navigateur de test"), Some("Claire Dupont"), Some(true)));
        let sub = &appels(&b, "POST", "/subscriptions")[0];
        assert_eq!(sub.corps["plan_code"], "ATEL-M");
        assert_eq!(sub.corps["tier_code"], "CLI-1");
        assert_eq!(sub.corps["partner_code"], "PARTENAIRE");
        assert_eq!(sub.corps["customer_notifications"], false);
        assert_eq!(sub.corps["bill_now"], true);
        assert_eq!(sub.corps["payment_method"], json!({ "provider": "stancer_sepa", "mandate_code": "RUM-1" }));
        let ab = abonnements::lire(&b.r.base(), id).unwrap();
        assert_eq!((ab.subscription_code.as_deref(), ab.offre.code(), ab.statut.as_str(), ab.rum.as_deref(), ab.bon_commande.as_str()), (Some("SUB-1"), "atelier", "trial", Some("RUM-1"), "BC-42"));
        let s = sujets(&b);
        assert!(s.iter().any(|x| x.contains("Bienvenue dans l'offre E-KO RIP Cloud « Atelier »")), "{s:?}");
        assert!(s.iter().any(|x| x.contains("mandat de prélèvement SEPA")), "{s:?}");
        assert!(b.boite.lock().unwrap().iter().filter(|m| m.sujet.contains("Bienvenue dans l'offre")).all(|m| m.a == "compta@atelier.fr"), "e-mails de facturation au contact de facturation");
        assert!(b.boite.lock().unwrap().iter().any(|m| m.sujet.contains("Bienvenue dans l'offre") && m.texte.contains("première facture est émise")), "first_cycle de l'ERP (bill_now)");
        // Page Abonnement : offre, moyen de paiement, factures (PDF relayé, seulement les siennes).
        let page = appel(&b, "GET", "/abonnement", &[("Cookie", &c)], Corps::Rien);
        assert!(page.corps.contains("Offre Atelier") && page.corps.contains("Prélèvement SEPA (IBAN se terminant par 0189)") && page.corps.contains("REF-FA-1") && page.corps.contains("/abonnement/factures/FA-1.pdf"), "{}", page.corps);
        assert!(page.corps.contains(r#"href="/abonnement""#) && page.corps.contains("entree active"), "entrée Abonnement de la barre latérale");
        let f = appel(&b, "GET", "/abonnement/factures/FA-1.pdf", &[("Cookie", &c)], Corps::Rien);
        assert_eq!((f.statut, f.octets.as_slice()), (200, PDF));
        assert_eq!(appel(&b, "GET", "/abonnement/factures/FA-AUTRE.pdf", &[("Cookie", &c)], Corps::Rien).statut, 404);
        let (autre, _, _) = inscrire(&b, "autre@atelier.fr");
        assert_eq!(appel(&b, "GET", "/abonnement/factures/FA-1.pdf", &[("Cookie", &autre)], Corps::Rien).statut, 404, "facture d'un autre compte");
        // Passage à l'annuel : nouvel abonnement avec le mandat, ancien résilié.
        let r = appel(&b, "POST", "/abonnement/changer", &[("Origin", URL), ("Cookie", &c)], Corps::Formulaire(&[("csrf", &j), ("offre", "pro"), ("cycle", "annuel")]));
        assert_eq!(r.lieu, "/abonnement?ok=change", "{}", r.corps);
        let subs = appels(&b, "POST", "/subscriptions");
        assert_eq!((subs.len(), subs[1].corps["plan_code"].as_str()), (2, Some("PRO-A")), "{:?}", subs.iter().map(|s| s.corps.clone()).collect::<Vec<_>>());
        assert_eq!(subs[1].corps["payment_method"]["mandate_code"], "RUM-1");
        assert_eq!(appels(&b, "POST", "/subscriptions/SUB-1/cancel").len(), 1);
        assert!(sujets(&b).iter().any(|x| x.contains("est maintenant « Pro »")));
        // Résiliation à la fin de la période.
        let r = appel(&b, "POST", "/abonnement/resilier", &[("Origin", URL), ("Cookie", &c)], Corps::Formulaire(&[("csrf", &j), ("confirmer", "1")]));
        assert_eq!(r.lieu, "/abonnement?ok=resilie");
        assert_eq!(appels(&b, "POST", "/subscriptions/SUB-2/cancel")[0].corps["at_period_end"], true);
    })
    .await;
    let _ = std::fs::remove_dir_all(&b.donnees);
}

#[tokio::test(flavor = "multi_thread")]
async fn souscription_par_carte_puis_sieges() {
    let b = banc().await;
    let b2 = b.clone();
    bl(move || {
        let b = b2;
        regler(&b);
        let (c, j, id) = inscrire(&b, "chef@atelier.fr");
        let r = appel(&b, "POST", "/abonnement/souscrire", &[("Origin", URL), ("Cookie", &c)], Corps::Formulaire(&[("csrf", &j), ("offre", "machine"), ("cycle", "mensuel"), ("pays", "FR"), ("siren", "552100554"), ("raison_sociale", "Existant"), ("contact", "Claire Dupont"), ("email", "chef@atelier.fr"), ("moyen", "cb"), ("cgv", "1")]));
        assert_eq!(r.lieu, "/abonnement/paiement?moyen=cb", "{}", r.corps);
        assert!(appels(&b, "POST", "/tiers").is_empty(), "tiers retrouvé par son SIREN");
        let page = appel(&b, "GET", "/abonnement/paiement?moyen=cb", &[("Cookie", &c)], Corps::Rien);
        assert!(page.corps.contains(r#"<script src="https://js.stancer.com/v1.0.0/stancer.js"></script>"#) && page.corps.contains(r#"data-cle="pk_test_123""#) && page.corps.contains("modale-3ds"), "{}", page.corps);
        assert!(page.entetes.get("content-security-policy").unwrap().to_str().unwrap().contains("script-src 'self' 'unsafe-inline' https://js.stancer.com"));
        // Carte tokenisée : abonnement, moyen de paiement, authentification avec fenêtre 3-D Secure.
        let r = appel(&b, "POST", "/abonnement/paiement/carte", &[("Origin", URL), ("Cookie", &c)], Corps::Json(json!({ "card_id": "card_abc" })));
        assert_eq!(r.statut, 403, "jeton anti-falsification exigé");
        let r = appel(&b, "POST", "/abonnement/paiement/carte", &[("Origin", URL), ("Cookie", &c), ("X-CSRF-Token", &j)], Corps::Json(json!({ "card_id": "card_abc", "last4": "4242", "brand": "visa" })));
        assert_eq!((r.statut, r.json()["payment_id"].as_str(), r.json()["redirect_url"].as_str()), (200, Some("paym_1"), Some("https://3ds.stancer.test/auth/paym_1")), "{}", r.corps);
        let sub = &appels(&b, "POST", "/subscriptions")[0];
        assert!(sub.corps.get("payment_method").is_none() && sub.corps["plan_code"] == "MACH-M" && sub.corps["seats"] == 1 && sub.corps["tier_code"] == "CLI-EXISTANT");
        assert_eq!(appels(&b, "PUT", "/subscriptions/SUB-1/payment-method")[0].corps, json!({ "provider": "stancer_card", "card_id": "card_abc" }));
        assert_eq!(appels(&b, "POST", "/subscriptions/SUB-1/card-authentication")[0].corps["return_url"], format!("{URL}/abonnement/3ds-retour"));
        let retour = appel(&b, "GET", "/abonnement/3ds-retour", &[], Corps::Rien);
        assert!(retour.corps.contains("postMessage") && retour.entetes.get("content-security-policy").unwrap().to_str().unwrap().contains("frame-ancestors 'self'"));
        let suivi = |p: &str| appel(&b, "GET", &format!("/abonnement/paiement/carte/{p}"), &[("Cookie", &c), ("X-CSRF-Token", &j)], Corps::Rien);
        assert_eq!(suivi("paym_autre").statut, 404);
        assert_eq!(suivi("paym_1").json()["termine"], false);
        assert_eq!(suivi("paym_1").json()["termine"], true);
        let ab = abonnements::lire(&b.r.base(), id).unwrap();
        assert_eq!((ab.moyen_paiement.as_str(), ab.carte_id.as_deref(), ab.statut.as_str()), ("Carte Visa se terminant par 4242", Some("card_abc"), "active"));
        // Inventaire : 3 imprimantes et 2 découpeurs → 3 machines facturées ; ensuite, au plus un envoi par heure.
        let (site, cle) = site_de(&b, id, "Atelier de Lyon");
        assert_eq!(inventaire(&b, &site, "mauvaise", 1, 0).statut, 401);
        let r = inventaire(&b, &site, &cle, 3, 2);
        assert_eq!(r.json(), json!({ "offre": "machine", "sauvegardes": true, "depassement": false }), "{}", r.corps);
        assert_eq!(appels(&b, "PATCH", "/subscriptions/SUB-1")[0].corps, json!({ "seats": 3 }));
        inventaire(&b, &site, &cle, 4, 2);
        assert_eq!(appels(&b, "PATCH", "/subscriptions/SUB-1").len(), 1, "anti-rebond d'une heure");
        abonnements::entretenir_bloquant(&b.r, eko_rip_cloud::maintenant() + 3601);
        assert_eq!(appels(&b, "PATCH", "/subscriptions/SUB-1").last().unwrap().corps, json!({ "seats": 4 }), "envoyé une heure plus tard");
        let page = appel(&b, "GET", "/abonnement", &[("Cookie", &c)], Corps::Rien);
        assert!(page.corps.contains("Imprimante 3") && page.corps.contains("Atelier de Lyon") && page.corps.contains("Carte Visa"), "{}", page.corps);
    })
    .await;
    let _ = std::fs::remove_dir_all(&b.donnees);
}

/// Supplément « machines en plus » : résilié à l'échéance sans perdre sa référence (pas de
/// second supplément si des machines reviennent), recréé au webhook de fin ; carte propagée au
/// supplément ; factures des anciens abonnements toujours listées et téléchargeables.
#[tokio::test(flavor = "multi_thread")]
async fn supplement_carte_et_historique_des_factures() {
    let b = banc().await;
    let b2 = b.clone();
    bl(move || {
        let b = b2;
        regler(&b);
        let (c, j, id) = inscrire(&b, "chef@atelier.fr");
        let txt = |v: &str| rusqlite::types::Value::Text(v.into());
        abonnements::maj(&b.r.base(), id, &[("offre", txt("atelier")), ("subscription_code", txt("SUB-A")), ("tier_code", txt("CLI-1")), ("statut", txt("active")), ("carte_id", txt("card_old"))]).unwrap();
        let (site, cle) = site_de(&b, id, "Atelier de Lyon");
        let mut t = eko_rip_cloud::maintenant();
        let mut plus_tard = || {
            t += 3601;
            abonnements::entretenir_bloquant(&b.r, t);
        };
        // 7 machines pour un forfait de 5 : supplément de 2 sièges, payé par la carte.
        inventaire(&b, &site, &cle, 5, 2);
        let subs = appels(&b, "POST", "/subscriptions");
        assert_eq!((subs.len(), subs[0].corps["plan_code"].as_str(), subs[0].corps["seats"].as_i64()), (1, Some("SUP-M"), Some(2)));
        let sup = abonnements::lire(&b.r.base(), id).unwrap().supplement_code.unwrap();
        assert_eq!(appels(&b, "PUT", &format!("/subscriptions/{sup}/payment-method"))[0].corps["card_id"], "card_old");
        // Plus que 4 machines : résiliation à l'échéance, référence gardée.
        inventaire(&b, &site, &cle, 3, 1);
        plus_tard();
        assert_eq!(appels(&b, "POST", &format!("/subscriptions/{sup}/cancel"))[0].corps["at_period_end"], true);
        let ab = abonnements::lire(&b.r.base(), id).unwrap();
        assert_eq!((ab.supplement_code.as_deref(), ab.supplement_resilie), (Some(sup.as_str()), true));
        plus_tard();
        assert_eq!(appels(&b, "POST", &format!("/subscriptions/{sup}/cancel")).len(), 1, "résilié une seule fois");
        // 8 machines avant l'échéance : le supplément en attente prend les sièges, pas de second.
        inventaire(&b, &site, &cle, 6, 2);
        plus_tard();
        assert_eq!(appels(&b, "POST", "/subscriptions").len(), 1, "pas de double facturation");
        assert_eq!(appels(&b, "PATCH", &format!("/subscriptions/{sup}")).last().unwrap().corps, json!({ "seats": 3 }));
        // Nouvelle carte : enregistrée aussi sur le supplément après l'authentification.
        let r = appel(&b, "POST", "/abonnement/paiement/carte", &[("Origin", URL), ("Cookie", &c), ("X-CSRF-Token", &j)], Corps::Json(json!({ "card_id": "card_new", "last4": "4242", "brand": "visa" })));
        assert_eq!(r.statut, 200, "{}", r.corps);
        let suivi = || appel(&b, "GET", "/abonnement/paiement/carte/paym_1", &[("Cookie", &c), ("X-CSRF-Token", &j)], Corps::Rien).json();
        assert_eq!(suivi()["termine"], false);
        assert_eq!(appels(&b, "PUT", &format!("/subscriptions/{sup}/payment-method")).len(), 1, "pas avant l'authentification");
        assert_eq!(suivi()["termine"], true);
        assert_eq!(appels(&b, "PUT", &format!("/subscriptions/{sup}/payment-method")).last().unwrap().corps, json!({ "provider": "stancer_card", "card_id": "card_new" }));
        // Fin réelle du supplément (webhook) : un nouveau supplément pour les 3 machines en trop.
        let w = json!({ "event": "subscription.cancelled", "occurred_at": "2026-12-01T00:00:00Z", "tenant": "golden-print", "data": { "subscription": { "code": sup } } });
        assert_eq!(webhook(&b, &w, eko_rip_cloud::maintenant(), "secret-webhook").statut, 200);
        let subs = appels(&b, "POST", "/subscriptions");
        assert_eq!((subs.len(), subs[1].corps["seats"].as_i64()), (2, Some(3)));
        let ab = abonnements::lire(&b.r.base(), id).unwrap();
        assert!(ab.supplement_code.as_deref().is_some_and(|x| x != sup) && !ab.supplement_resilie);
        // Retour au Gratuit : les factures des anciens abonnements restent visibles.
        abonnements::retour_au_gratuit(&b.r, id);
        let page = appel(&b, "GET", "/abonnement", &[("Cookie", &c)], Corps::Rien);
        let facture_sup = if sup == "SUB-1" { "FA-1".to_string() } else { format!("FA-{sup}") };
        for f in ["FA-SUB-A", facture_sup.as_str()] {
            assert!(page.corps.contains(&format!("/abonnement/factures/{f}.pdf")), "{f} : {}", page.corps);
            assert_eq!(appel(&b, "GET", &format!("/abonnement/factures/{f}.pdf"), &[("Cookie", &c)], Corps::Rien).statut, 200, "{f}");
        }
        assert_eq!(appel(&b, "GET", "/abonnement/factures/FA-SUB-Z.pdf", &[("Cookie", &c)], Corps::Rien).statut, 404);
    })
    .await;
    let _ = std::fs::remove_dir_all(&b.donnees);
}

fn webhook(b: &Banc, corps: &Value, t: u64, secret: &str) -> Rep {
    let brut = serde_json::to_vec(corps).unwrap();
    let signature = erp::signer(secret, &brut, t);
    appel(b, "POST", "/api/erp/webhooks", &[("X-EKO-Signature", &signature), ("Content-Type", "application/json")], Corps::Octets(brut))
}

#[tokio::test(flavor = "multi_thread")]
async fn webhooks_de_l_erp() {
    let b = banc().await;
    let b2 = b.clone();
    bl(move || {
        let b = b2;
        regler(&b);
        let (c, _, id) = inscrire(&b, "chef@atelier.fr");
        abonnements::maj(&b.r.base(), id, &[("offre", rusqlite::types::Value::Text("atelier".into())), ("subscription_code", rusqlite::types::Value::Text("SUB-W".into())), ("tier_code", rusqlite::types::Value::Text("CLI-W".into())), ("statut", rusqlite::types::Value::Text("active".into()))]).unwrap();
        let t = eko_rip_cloud::maintenant();
        let facture = json!({ "event": "cycle.billed", "occurred_at": "2026-11-01T08:00:00Z", "tenant": "golden-print", "data": { "subscription": { "code": "SUB-W" }, "cycle": { "period_start": "2026-11-01", "period_end": "2026-12-01", "total_ttc": "22.80" }, "invoice": { "code": "FA-9", "reference": "FA-2026-0009", "total_ttc": "22.80" } } });
        // Signature absente, fausse, expirée : refusées, rien n'est traité.
        let r = appel(&b, "POST", "/api/erp/webhooks", &[], Corps::Json(facture.clone()));
        assert_eq!(r.statut, 401);
        assert_eq!(webhook(&b, &facture, t, "mauvais-secret").statut, 401);
        assert_eq!(webhook(&b, &facture, t - 301, "secret-webhook").statut, 401);
        assert!(sujets(&b).iter().all(|s| !s.contains("facture")));
        // Valide : e-mail de facture avec le lien du portail ; rejoué : ignoré.
        let r = webhook(&b, &facture, t, "secret-webhook");
        assert_eq!((r.statut, r.json()["code"].as_str()), (200, Some("traite")), "{}", r.corps);
        let r = webhook(&b, &facture, t, "secret-webhook");
        assert_eq!(r.json()["code"], "deja_traite");
        {
            let boite = b.boite.lock().unwrap();
            let f: Vec<_> = boite.iter().filter(|m| m.sujet.contains("Votre facture E-KO RIP Cloud FA-2026-0009")).collect();
            assert_eq!(f.len(), 1, "une seule fois");
            assert!(f[0].texte.contains("22,80 € TTC") && f[0].texte.contains(&format!("{URL}/abonnement/factures/FA-9.pdf")));
        }
        // Relance n° 2 : formulation ferme, e-mail et notification.
        let r = webhook(&b, &json!({ "event": "dunning.reminder_sent", "occurred_at": "x", "tenant": "golden-print", "data": { "subscription": { "code": "SUB-W" }, "step": 2, "cycle": { "total_ttc": 22.8 } } }), t, "secret-webhook");
        assert_eq!(r.json()["code"], "traite");
        assert!(sujets(&b).iter().any(|s| s == "Action requise : votre abonnement E-KO RIP Cloud est en retard"), "{:?}", sujets(&b));
        let journal = eko_rip_cloud::notifications::depuis(&b.r.base(), id, 0).unwrap();
        assert!(journal.iter().any(|n| n.genre == "paiement" && n.titre == "Action requise"));
        let ab = abonnements::lire(&b.r.base(), id).unwrap();
        assert_eq!((ab.statut.as_str(), ab.relance), ("past_due", 2));
        let page = appel(&b, "GET", "/abonnement", &[("Cookie", &c)], Corps::Rien);
        assert!(page.corps.contains("relance 2 sur 3"), "bandeau de retard");
        // Relances épuisées sur un ancien abonnement remplacé : le compte n'est pas touché.
        let r = webhook(&b, &json!({ "event": "dunning.exhausted", "occurred_at": "y", "data": { "subscription": { "code": "SUB-ANCIEN" }, "tier": { "code": "CLI-W" } } }), t, "secret-webhook");
        assert_eq!(r.json()["code"], "ignore");
        assert_eq!(abonnements::lire(&b.r.base(), id).unwrap().offre.code(), "atelier");
        // Sur le supplément seul : il s'arrête, l'abonnement principal reste ; une résiliation
        // refusée par l'ERP est gardée pour être retentée.
        abonnements::maj(&b.r.base(), id, &[("supplement_code", rusqlite::types::Value::Text("SUP-PANNE".into())), ("seats_supplement", rusqlite::types::Value::Integer(2))]).unwrap();
        let r = webhook(&b, &json!({ "event": "dunning.exhausted", "occurred_at": "z", "data": { "subscription": { "code": "SUP-PANNE" } } }), t, "secret-webhook");
        assert_eq!(r.json()["code"], "traite");
        let ab = abonnements::lire(&b.r.base(), id).unwrap();
        assert!(ab.offre.code() == "atelier" && ab.actif() && ab.supplement_code.is_none() && ab.seats_supplement == 0);
        let essais: i64 = b.r.base().query_row("SELECT essais FROM resiliations_erp WHERE code = 'SUP-PANNE'", [], |l| l.get(0)).unwrap();
        assert_eq!(essais, 1);
        abonnements::entretenir_bloquant(&b.r, t);
        let essais: i64 = b.r.base().query_row("SELECT essais FROM resiliations_erp WHERE code = 'SUP-PANNE'", [], |l| l.get(0)).unwrap();
        assert_eq!(essais, 2, "retentée par l'entretien");
        // Relances épuisées : retour au Gratuit, e-mail et notification.
        let r = webhook(&b, &json!({ "event": "dunning.exhausted", "occurred_at": "x", "tenant": "golden-print", "data": { "tier": { "code": "CLI-W" } } }), t, "secret-webhook");
        assert_eq!(r.json()["code"], "traite");
        let ab = abonnements::lire(&b.r.base(), id).unwrap();
        assert!(ab.offre.code() == "gratuit" && !ab.actif() && ab.gratuit_depuis.is_some());
        assert!(sujets(&b).iter().any(|s| s == "Abonnement E-KO RIP Cloud résilié pour impayé"));
        // Événement inconnu, compte inconnu : 200, ignorés.
        assert_eq!(webhook(&b, &json!({ "event": "autre.chose", "data": { "tier": { "code": "CLI-W" } } }), t, "secret-webhook").json()["code"], "ignore");
        assert_eq!(webhook(&b, &json!({ "event": "cycle.paid", "data": { "subscription": { "code": "SUB-INCONNU" } } }), t, "secret-webhook").json()["code"], "ignore");
    })
    .await;
    let _ = std::fs::remove_dir_all(&b.donnees);
}

/// Faux serveur RIP : un écho, et la commande de restauration (signature vérifiée).
fn rip(cle_sha256: String, recues: Arc<Mutex<Vec<Value>>>) -> axum::Router {
    axum::Router::new().route("/api/echo", axum::routing::get(|| async { "écho" })).route(
        "/api/cloud/restaurer",
        axum::routing::post(move |h: HeaderMap, corps: Bytes| {
            let (cle, recues) = (cle_sha256.clone(), recues.clone());
            async move {
                let entete = h.get("x-eko-cloud-service").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
                let t: u64 = entete.split(',').next().and_then(|x| x.strip_prefix("t=")).and_then(|x| x.parse().ok()).unwrap_or(0);
                if entete != sauvegardes::signer_service(&cle, &corps, t) {
                    return StatusCode::UNAUTHORIZED;
                }
                recues.lock().unwrap().push(serde_json::from_slice(&corps).unwrap());
                StatusCode::ACCEPTED
            }
        }),
    )
}

async fn tunnel(b: &Banc, site: &str, cle: &str, routeur: axum::Router) {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let (base, s, k) = (b.base.clone(), site.to_string(), cle.to_string());
    tokio::spawn(async move { client::servir(&base, &s, &k, "essai", routeur, move || tx.send(()).unwrap()).await });
    rx.await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn depassement_grace_puis_acces_suspendu() {
    let b = banc().await;
    let b2 = b.clone();
    let (site, cle, id, c) = bl(move || {
        let b = b2;
        regler(&b);
        let (c, _, id) = inscrire(&b, "chef@atelier.fr");
        let (site, cle) = site_de(&b, id, "Atelier");
        (site, cle, id, c)
    })
    .await;
    tunnel(&b, &site, &cle, rip(String::new(), Default::default())).await;
    let b2 = b.clone();
    bl(move || {
        let b = b2;
        // Gratuit avec 3 imprimantes : dépassement, e-mail et notification, accès encore ouvert.
        let r = inventaire(&b, &site, &cle, 3, 1);
        assert_eq!(r.json()["depassement"], true, "{}", r.corps);
        assert!(sujets(&b).iter().any(|s| s.starts_with("Offre E-KO RIP Cloud dépassée : accès à distance suspendu le")), "{:?}", sujets(&b));
        assert!(eko_rip_cloud::notifications::depuis(&b.r.base(), id, 0).unwrap().iter().any(|n| n.genre == "depassement"));
        let echo = || appel(&b, "GET", &format!("/s/{site}/api/echo"), &[], Corps::Rien);
        assert_eq!(echo().statut, 200);
        let page = appel(&b, "GET", "/sites", &[("Cookie", &c)], Corps::Rien);
        assert!(page.corps.contains("Offre dépassée.") && page.corps.contains("Choisir une offre"));
        // 14 jours plus tard, mais pendant la grâce du déploiement (30 jours) : toujours ouvert.
        let t = eko_rip_cloud::maintenant();
        abonnements::maj(&b.r.base(), id, &[("depassement_depuis", rusqlite::types::Value::Integer((t - 15 * 86_400) as i64))]).unwrap();
        abonnements::entretenir_bloquant(&b.r, t);
        assert_eq!(echo().statut, 200, "comptes existants : 30 jours après le déploiement");
        // Grâce du déploiement écoulée : suspendu, e-mail une fois, 402 lisible.
        base::regler(&b.r.base(), "offres_deployees_le", "0").unwrap();
        abonnements::entretenir_bloquant(&b.r, t);
        abonnements::entretenir_bloquant(&b.r, t + 60);
        assert_eq!(sujets(&b).iter().filter(|s| s.as_str() == "Accès à distance suspendu : offre E-KO RIP Cloud dépassée").count(), 1);
        let r = echo();
        assert_eq!((r.statut, r.json()["code"].as_str()), (402, Some("offre_depassee")));
        let r = appel(&b, "GET", &format!("/s/{site}/api/echo"), &[("Accept", "text/html")], Corps::Rien);
        assert!(r.statut == 402 && r.corps.contains("Accès à distance suspendu — offre dépassée") && r.corps.contains("L'usage dans l'atelier n'est pas touché"));
        let r = appel(&b, "GET", "/api/echo", &[("Host", &format!("{site}.plateforme.test"))], Corps::Rien);
        assert_eq!(r.statut, 402, "aussi sur l'origine propre du site");
        let r = appel(&b, "POST", "/api/site/sauvegardes", &[("Authorization", &format!("Bearer {site}.{cle}")), ("X-EKO-Motif", "nuit"), ("X-EKO-Empreinte", &eko_rip_cloud::sha256_octets_hex(b"x"))], Corps::Octets(b"x".to_vec()));
        assert_eq!((r.statut, r.json()["erreur"].as_str()), (402, Some("offre")));
        // Retour dans l'offre : accès rétabli.
        inventaire(&b, &site, &cle, 1, 1);
        abonnements::entretenir_bloquant(&b.r, t + 120);
        assert_eq!(echo().statut, 200);
        assert!(abonnements::lire(&b.r.base(), id).unwrap().depassement_depuis.is_none());
    })
    .await;
    let _ = std::fs::remove_dir_all(&b.donnees);
}

fn envoyer_sauvegarde(b: &Banc, site: &str, cle: &str, contenu: &[u8], motif: &str) -> Rep {
    appel(b, "POST", "/api/site/sauvegardes", &[("Authorization", &format!("Bearer {site}.{cle}")), ("X-EKO-Motif", motif), ("X-EKO-Empreinte", &eko_rip_cloud::sha256_octets_hex(contenu))], Corps::Octets(contenu.to_vec()))
}

#[tokio::test(flavor = "multi_thread")]
async fn sauvegardes_en_ligne() {
    let b = banc().await;
    let b2 = b.clone();
    let (site, cle, site2, cle2, id, c, j) = bl(move || {
        let b = b2;
        regler(&b);
        let (c, j, id) = inscrire(&b, "chef@atelier.fr");
        let (site, cle) = site_de(&b, id, "Atelier de Lyon");
        let (site2, cle2) = site_de(&b, id, "Atelier de Nantes");
        (site, cle, site2, cle2, id, c, j)
    })
    .await;
    let cle_sha = sites::lire(&b.donnees).unwrap().into_iter().find(|s| s.id == site2).unwrap().cle_sha256;
    let recues: Arc<Mutex<Vec<Value>>> = Default::default();
    tunnel(&b, &site2, &cle2, rip(cle_sha, recues.clone())).await;
    let b2 = b.clone();
    bl(move || {
        let b = b2;
        // Gratuit : envoi, doublon, empreinte fausse ; la nouvelle version remplace l'ancienne.
        let r = envoyer_sauvegarde(&b, &site, &cle, b"archive-1", "nuit");
        assert_eq!(r.statut, 201, "{}", r.corps);
        let v1 = r.json()["id"].as_str().unwrap().to_string();
        assert_eq!(envoyer_sauvegarde(&b, &site, &cle, b"archive-1", "changement").json()["erreur"], "identique");
        // Nuit sans changement : 409 identique, mais la nuit est notée (pas d'alerte « en échec »).
        b.r.base().execute("UPDATE inventaires SET nuit_le = 1 WHERE site = ?1", [&site]).unwrap();
        assert_eq!(envoyer_sauvegarde(&b, &site, &cle, b"archive-1", "nuit").statut, 409);
        let nuit: i64 = b.r.base().query_row("SELECT nuit_le FROM inventaires WHERE site = ?1", [&site], |l| l.get(0)).unwrap();
        assert!(nuit > 1_000_000, "nuit_le rafraîchi par un 409 de nuit");
        assert_eq!(envoyer_sauvegarde(&b, &site, &cle, b"archive-1", "inconnu").statut, 400);
        let r = appel(&b, "POST", "/api/site/sauvegardes", &[("Authorization", &format!("Bearer {site}.{cle}")), ("X-EKO-Motif", "nuit"), ("X-EKO-Empreinte", &"0".repeat(64))], Corps::Octets(b"autre".to_vec()));
        assert_eq!((r.statut, r.json()["erreur"].as_str()), (400, Some("empreinte")));
        assert_eq!(envoyer_sauvegarde(&b, &site, "mauvaise", b"z", "nuit").statut, 401);
        {
            let s3 = b.s3.lock().unwrap();
            let cle_objet = format!("/seau/sauvegardes/{id}/{site}/{v1}.tar");
            assert_eq!(s3.objets.get(&cle_objet).map(Vec::as_slice), Some(&b"archive-1"[..]), "{:?}", s3.objets.keys());
            assert!(s3.requetes.iter().filter(|(m, _, _)| m == "PUT").all(|(_, _, sse)| *sse), "chiffrement côté serveur");
        }
        let r = envoyer_sauvegarde(&b, &site, &cle, b"archive-2", "demande");
        assert_eq!(r.statut, 201);
        let v2 = r.json()["id"].as_str().unwrap().to_string();
        assert!(!b.s3.lock().unwrap().objets.contains_key(&format!("/seau/sauvegardes/{id}/{site}/{v1}.tar")), "Gratuit : dernière version seulement");
        // Liste et lecture côté serveur RIP.
        let l = appel(&b, "GET", "/api/site/sauvegardes", &[("Authorization", &format!("Bearer {site}.{cle}"))], Corps::Rien).json();
        assert_eq!(l["sauvegardes"].as_array().unwrap().len(), 1);
        assert_eq!(l["sauvegardes"][0]["id"].as_str(), Some(v2.as_str()));
        let o = appel(&b, "GET", &format!("/api/site/sauvegardes/{v2}"), &[("Authorization", &format!("Bearer {site}.{cle}"))], Corps::Rien);
        assert_eq!(o.octets, b"archive-2");
        assert_eq!(appel(&b, "GET", &format!("/api/site/sauvegardes/{v2}"), &[("Authorization", &format!("Bearer {site2}.{cle2}"))], Corps::Rien).statut, 404, "autre site, offre sans restauration croisée");
        // Quota : 1 Go au Gratuit (usage simulé sur l'autre site).
        b.r.base().execute("INSERT INTO sauvegardes (id, site, compte, cree_le, octets, empreinte, motif, cle_s3) VALUES ('gros', ?1, ?2, ?3, ?4, 'e', 'nuit', 'sauvegardes/gros.tar')", rusqlite::params![site2, id, eko_rip_cloud::maintenant() as i64, (1u64 << 30) as i64]).unwrap();
        let r = envoyer_sauvegarde(&b, &site, &cle, b"archive-3", "nuit");
        assert_eq!((r.statut, r.json()["erreur"].as_str()), (413, Some("quota")));
        assert!(sujets(&b).iter().any(|s| s == "Espace de sauvegarde E-KO RIP Cloud plein"), "{:?}", sujets(&b));
        b.r.base().execute("DELETE FROM sauvegardes WHERE id = 'gros'", []).unwrap();
        // Téléchargement : membres du site seulement.
        let d = appel(&b, "GET", &format!("/sites/{site}/sauvegardes/{v2}"), &[("Cookie", &c)], Corps::Rien);
        assert_eq!((d.statut, d.octets.as_slice()), (200, &b"archive-2"[..]));
        assert!(d.entetes.get("content-disposition").unwrap().to_str().unwrap().starts_with("attachment"));
        let (autre, autre_j, autre_id) = inscrire(&b, "lecteur@atelier.fr");
        assert_eq!(appel(&b, "GET", &format!("/sites/{site}/sauvegardes/{v2}"), &[("Cookie", &autre)], Corps::Rien).statut, 404);
        base::donner_droit(&b.r.base(), &site, autre_id, "lecture").unwrap();
        base::donner_droit(&b.r.base(), &site2, autre_id, "lecture").unwrap();
        assert_eq!(appel(&b, "GET", &format!("/sites/{site}/sauvegardes/{v2}"), &[("Cookie", &autre)], Corps::Rien).statut, 200, "membre en lecture : téléchargement permis");
        let page = appel(&b, "GET", &format!("/sites/{site}/sauvegardes"), &[("Cookie", &autre)], Corps::Rien);
        assert!(page.corps.contains("Télécharger") && !page.corps.contains("/restaurer"), "lecture : pas de restauration");
        // Restauration : site du tunnel (Nantes), commande signée ; refusée au rôle lecture.
        let r = envoyer_sauvegarde(&b, &site2, &cle2, b"archive-nantes", "nuit");
        let vn = r.json()["id"].as_str().unwrap().to_string();
        let r = appel(&b, "POST", &format!("/sites/{site2}/sauvegardes/{vn}/restaurer"), &[("Origin", URL), ("Cookie", &autre)], Corps::Formulaire(&[("csrf", &autre_j), ("confirmer", "1")]));
        assert_eq!(r.statut, 403);
        let page = appel(&b, "GET", &format!("/sites/{site2}/sauvegardes"), &[("Cookie", &c)], Corps::Rien);
        assert!(page.corps.contains(&format!("/sites/{site2}/sauvegardes/{vn}/restaurer")));
        let r = appel(&b, "POST", &format!("/sites/{site2}/sauvegardes/{vn}/restaurer"), &[("Origin", URL), ("Cookie", &c)], Corps::Formulaire(&[("csrf", &j), ("confirmer", "1")]));
        assert_eq!(r.lieu, format!("/sites/{site2}/sauvegardes?ok=restauree"), "{}", r.corps);
        assert_eq!(recues.lock().unwrap().as_slice(), &[json!({ "id": vn })]);
        assert!(sujets(&b).iter().any(|s| s == "Sauvegarde restaurée sur le site « Atelier de Nantes »"));
        // Vers un autre site : réservé au Pro.
        let r = appel(&b, "POST", &format!("/sites/{site}/sauvegardes/{v2}/restaurer"), &[("Origin", URL), ("Cookie", &c)], Corps::Formulaire(&[("csrf", &j), ("confirmer", "1"), ("vers", &site2)]));
        assert_eq!(r.statut, 403);
        abonnements::maj(&b.r.base(), id, &[("offre", rusqlite::types::Value::Text("pro".into()))]).unwrap();
        let r = appel(&b, "POST", &format!("/sites/{site}/sauvegardes/{v2}/restaurer"), &[("Origin", URL), ("Cookie", &c)], Corps::Formulaire(&[("csrf", &j), ("confirmer", "1"), ("vers", &site2)]));
        assert_eq!(r.statut, 303, "{}", r.corps);
        assert_eq!(recues.lock().unwrap().last().unwrap(), &json!({ "id": v2 }));
        assert_eq!(appel(&b, "GET", &format!("/api/site/sauvegardes/{v2}"), &[("Authorization", &format!("Bearer {site2}.{cle2}"))], Corps::Rien).octets, b"archive-2", "Pro : le site cible lit la sauvegarde de l'autre site");
        // Conservation : Atelier, 90 jours ; les versions plus anciennes partent à l'entretien quotidien.
        abonnements::maj(&b.r.base(), id, &[("offre", rusqlite::types::Value::Text("atelier".into()))]).unwrap();
        let t = eko_rip_cloud::maintenant();
        b.r.base().execute("INSERT INTO sauvegardes (id, site, compte, cree_le, octets, empreinte, motif, cle_s3) VALUES ('vieille', ?1, ?2, ?3, 10, 'v', 'nuit', 'sauvegardes/vieille.tar')", rusqlite::params![site, id, (t - 100 * 86_400) as i64]).unwrap();
        b.r.base().execute("INSERT INTO sauvegardes (id, site, compte, cree_le, octets, empreinte, motif, cle_s3) VALUES ('recente', ?1, ?2, ?3, 10, 'r', 'nuit', 'sauvegardes/recente.tar')", rusqlite::params![site, id, (t - 10 * 86_400) as i64]).unwrap();
        sauvegardes::entretien_quotidien(&b.r, t);
        let restent: Vec<String> = sauvegardes::versions_du_site(&b.r.base(), &site).unwrap().into_iter().map(|v| v.id).collect();
        assert!(restent.contains(&"recente".to_string()) && !restent.contains(&"vieille".to_string()) && restent.contains(&v2), "{restent:?}");
        // Deux nuits sans sauvegarde nocturne : propriétaire prévenu une fois.
        b.r.base().execute("UPDATE inventaires SET nuit_le = ?1 WHERE site = ?2", rusqlite::params![(t - 3 * 86_400) as i64, site]).unwrap();
        sauvegardes::entretien_quotidien(&b.r, t);
        sauvegardes::entretien_quotidien(&b.r, t);
        assert_eq!(sujets(&b).iter().filter(|s| s.as_str() == "Sauvegarde du site « Atelier de Lyon » en échec").count(), 1);
        // Copie de la base du portail dans le second bucket.
        sauvegardes::sauvegarder_base(&b.r, t).unwrap();
        assert!(b.s3.lock().unwrap().objets.keys().any(|k| k.starts_with("/copies/portail/portail-") && k.ends_with(".sqlite")));
        // Résumé hebdomadaire : ligne « Sauvegardes ».
        let ligne = sauvegardes::ligne_resume(&b.r.base(), &site).unwrap().unwrap();
        assert_eq!(ligne.2, 25 * eko_rip_cloud::offres::GO);
    })
    .await;
    let _ = std::fs::remove_dir_all(&b.donnees);
}
