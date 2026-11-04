//! Support et discussion en direct de bout en bout, avec un faux ERP (API `/support`) : demande
//! avec pièce jointe, réponse, résolution et note, bulle de discussion (interrogation locale),
//! webhooks `support.*` signés et e-mails de la plate-forme.

use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use eko_rip_cloud::courriels::Courriel;
use eko_rip_cloud::{base, erp, relais};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const URL: &str = "http://plateforme.test";
const SECRET: &str = "secret-webhook";

// ————— Faux ERP —————

#[derive(Clone, Debug)]
struct Appel {
    methode: String,
    chemin: String,
    corps: Value,
    brut: String,
    entetes: HashMap<String, String>,
}

#[derive(Default)]
struct EtatErp {
    appels: Vec<Appel>,
    tickets: Vec<Value>,
    /// Idempotency-Key -> code de la demande déjà créée.
    cles: HashMap<String, String>,
    /// Fil de chaque demande (`thread`).
    fils: HashMap<String, Vec<Value>>,
    /// Statut de chaque demande (`satisfaction` n'est acceptée que résolue).
    notes: HashMap<String, Value>,
    ouvert: bool,
    message: i64,
    /// Discussions : uuid -> objet.
    discussions: HashMap<String, Value>,
}

type FauxErp = Arc<Mutex<EtatErp>>;

fn extraire(brut: &str, nom: &str) -> Option<String> {
    let debut = brut.find(&format!("name=\"{nom}\"\r\n\r\n"))? + nom.len() + 11;
    let fin = brut[debut..].find("\r\n--")?;
    Some(brut[debut..debut + fin].to_string())
}

async fn faux_erp(State(e): State<FauxErp>, q: Request) -> Response {
    let methode = q.method().to_string();
    let chemin = q.uri().path().trim_start_matches("/api/v1").to_string();
    let requete = q.uri().query().unwrap_or("").to_string();
    let entetes: HashMap<String, String> = q
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    let octets = axum::body::to_bytes(q.into_body(), 1 << 22).await.unwrap();
    let brut = String::from_utf8_lossy(&octets).to_string();
    let corps: Value = serde_json::from_slice(&octets).unwrap_or(Value::Null);
    let mut e = e.lock().unwrap();
    e.appels.push(Appel {
        methode: methode.clone(),
        chemin: chemin.clone(),
        corps: corps.clone(),
        brut: brut.clone(),
        entetes: entetes.clone(),
    });
    if entetes.get("authorization").map(String::as_str) != Some("Bearer jeton-secret-erp") {
        return (
            StatusCode::UNAUTHORIZED,
            axum::Json(json!({ "message": "Unauthenticated." })),
        )
            .into_response();
    }
    let p: Vec<&str> = chemin.trim_start_matches('/').split('/').collect();
    let j = |s: StatusCode, v: Value| (s, axum::Json(v)).into_response();
    let reference_demandee = requete
        .split('&')
        .find_map(|x| x.strip_prefix("account_reference="))
        .unwrap_or("")
        .to_string();
    match (methode.as_str(), p.as_slice()) {
        ("GET", ["support", "categories"]) => {
            let ouvert = e.ouvert;
            let horaires = if ouvert {
                json!({ "start": "00:00", "end": "23:59", "days": [1, 2, 3, 4, 5, 6, 7], "timezone": "Europe/Paris" })
            } else {
                json!({ "start": "09:00", "end": "18:00", "days": [], "timezone": "Europe/Paris" })
            };
            j(
                StatusCode::OK,
                json!({ "data": [{ "id": 1, "code": "TECH", "label": "Technique", "color": "#007aff" }, { "id": 2, "code": "COMPTA", "label": "Comptabilité", "color": "#34c759" }], "meta": { "open_now": ouvert, "opening_hours": horaires } }),
            )
        }
        ("POST", ["support", "tickets"]) => {
            let cle = entetes.get("idempotency-key").cloned().unwrap_or_default();
            if let Some(code) = e.cles.get(&cle) {
                let t = e
                    .tickets
                    .iter()
                    .find(|t| t["code"] == code.as_str())
                    .cloned()
                    .unwrap();
                return j(StatusCode::OK, json!({ "data": t }));
            }
            let champ = |n: &str| {
                if corps.is_null() {
                    extraire(&brut, n).unwrap_or_default()
                } else {
                    corps[n].as_str().unwrap_or("").to_string()
                }
            };
            if champ("account_reference").is_empty()
                || (champ("tier_code").is_empty() && champ("tier_email").is_empty())
            {
                return j(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    json!({ "message": "x", "errors": { "tier_code": ["Aucun tiers."] } }),
                );
            }
            let code = format!("TKT-{}", e.tickets.len() + 1);
            let t = json!({ "code": code, "label": champ("subject"), "status": "open", "source": "rip-cloud", "origin": "request", "discussion_uuid": null, "category": { "code": champ("category"), "label": if champ("category") == "TECH" { "Technique" } else { "Comptabilité" } }, "account_reference": champ("account_reference"), "opened_at": "2026-11-04T09:00:00+00:00", "first_response_due_at": "2026-11-04T16:00:00+00:00", "first_response_at": null, "resolved_at": null, "satisfaction": null, "description": format!("&lt;p&gt;{}&lt;/p&gt;", champ("message")) });
            e.cles.insert(cle, code.clone());
            e.fils.insert(code.clone(), Vec::new());
            e.tickets.push(t.clone());
            j(StatusCode::CREATED, json!({ "data": t }))
        }
        ("GET", ["support", "tickets"]) => {
            let l: Vec<Value> = e
                .tickets
                .iter()
                .filter(|t| t["account_reference"] == reference_demandee.as_str())
                .cloned()
                .collect();
            j(
                StatusCode::OK,
                json!({ "data": l, "meta": { "total": l.len() } }),
            )
        }
        ("GET", ["support", "tickets", code]) => {
            match e.tickets.iter().find(|t| {
                t["code"] == *code && t["account_reference"] == reference_demandee.as_str()
            }) {
                Some(t) => {
                    let mut t = t.clone();
                    t["thread"] = json!(e.fils.get(*code).cloned().unwrap_or_default());
                    t["attachments"] = json!([{ "id": 1, "name": "journal.txt", "size": 12 }]);
                    t["satisfaction_comment"] = e
                        .notes
                        .get(*code)
                        .map(|n| n["comment"].clone())
                        .unwrap_or(Value::Null);
                    j(StatusCode::OK, json!({ "data": t }))
                }
                None => j(StatusCode::NOT_FOUND, json!({ "message": "Not found." })),
            }
        }
        ("POST", ["support", "tickets", code, "comments"]) => {
            e.message += 1;
            let m = json!({ "id": 800 + e.message, "type": "message", "body": format!("<p>{}</p>", corps["message"].as_str().unwrap_or("")), "from_customer": true, "created_at": "2026-11-04T10:00:00+00:00" });
            e.fils.entry(code.to_string()).or_default().push(m.clone());
            j(
                StatusCode::CREATED,
                json!({ "data": { "id": 800 + e.message, "type": "message", "ticket": { "code": code, "status": "waiting_internal" } } }),
            )
        }
        ("POST", ["support", "tickets", code, "satisfaction"]) => {
            let t = e
                .tickets
                .iter()
                .find(|t| t["code"] == *code)
                .cloned()
                .unwrap();
            if t["status"] != "resolved" {
                return j(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    json!({ "message": "Pas résolue." }),
                );
            }
            e.notes.insert(code.to_string(), corps.clone());
            j(StatusCode::OK, json!({ "data": t }))
        }
        ("POST", ["support", "discussions"]) => {
            let uuid = format!("uuid-{}", e.discussions.len() + 1);
            let ouvert = e.ouvert;
            e.message += 1;
            let premier = json!({ "id": e.message, "author_type": "abonne", "author_name": "Claire Dupont", "body": corps["message"], "created_at": "2026-11-04T09:00:00+00:00" });
            let mut d = json!({ "uuid": uuid, "status": if ouvert { "en_attente" } else { "convertie" }, "account_reference": corps["account_reference"], "category": { "code": "TECH", "label": "Technique" }, "customer_name": "Claire Dupont", "agent_name": null, "ticket_code": if ouvert { Value::Null } else { json!("TKT-77") }, "open_now": ouvert, "created_at": "2026-11-04T09:00:00+00:00", "messages": [premier] });
            if !ouvert {
                e.message += 1;
                d["messages"].as_array_mut().unwrap().push(json!({ "id": e.message, "author_type": "systeme", "author_name": null, "body": "Le support est fermé : votre message devient une demande.", "created_at": "2026-11-04T09:00:01+00:00" }));
            }
            e.discussions.insert(uuid, d.clone());
            j(StatusCode::CREATED, json!({ "data": d }))
        }
        ("POST", ["support", "discussions", uuid, "messages"]) => {
            let Some(d) = e.discussions.get(*uuid).cloned() else {
                return j(StatusCode::NOT_FOUND, json!({}));
            };
            if d["status"] == "close" || d["status"] == "convertie" {
                return j(StatusCode::CONFLICT, json!({ "message": "Terminée." }));
            }
            e.message += 1;
            j(
                StatusCode::CREATED,
                json!({ "data": { "id": 50 + e.message, "author_type": "abonne", "author_name": "Claire Dupont", "body": corps["message"], "created_at": "2026-11-04T09:03:00+00:00" } }),
            )
        }
        ("POST", ["support", "discussions", uuid, "close"]) => {
            let d = e.discussions.get_mut(*uuid).unwrap();
            d["status"] = json!("close");
            j(StatusCode::OK, json!({ "data": d.clone() }))
        }
        _ => j(
            StatusCode::NOT_FOUND,
            json!({ "message": "route inconnue du faux ERP" }),
        ),
    }
}

// ————— Banc —————

struct Banc {
    base: String,
    donnees: std::path::PathBuf,
    r: relais::Partage,
    boite: Arc<Mutex<Vec<Courriel>>>,
    erp: FauxErp,
    url_erp: String,
}

async fn servir(app: axum::Router) -> String {
    let ecoute = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let adresse = format!("http://{}", ecoute.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(ecoute, app).await.unwrap() });
    adresse
}

async fn banc() -> Arc<Banc> {
    let donnees = std::env::temp_dir().join(format!(
        "eko-cloud-support-{}",
        eko_rip_cloud::aleatoire_hex(6)
    ));
    let r = relais::nouveau(relais::Config {
        donnees: donnees.clone(),
        url_publique: URL.into(),
        max_octets: 1 << 20,
        jeton_stats: String::new(),
    });
    let boite = r.utiliser_memoire();
    let _ = r.utiliser_memoire_push();
    let base = servir(relais::routes(r.clone())).await;
    let erp: FauxErp = Arc::new(Mutex::new(EtatErp {
        ouvert: true,
        ..Default::default()
    }));
    let url_erp = format!(
        "{}/api/v1",
        servir(
            axum::Router::new()
                .fallback(faux_erp)
                .with_state(erp.clone())
        )
        .await
    );
    Arc::new(Banc {
        base,
        donnees,
        r,
        boite,
        erp,
        url_erp,
    })
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .max_redirects(0)
        .timeout_global(Some(Duration::from_secs(20)))
        .build()
        .into()
}

struct Rep {
    statut: u16,
    corps: String,
    lieu: String,
    entetes: HeaderMap,
}

impl Rep {
    fn json(&self) -> Value {
        serde_json::from_str(&self.corps).unwrap_or(Value::Null)
    }
}

fn lire(r: Result<ureq::http::Response<ureq::Body>, ureq::Error>) -> Rep {
    let mut r = r.unwrap();
    let lieu = r
        .headers()
        .get("location")
        .map(|v| v.to_str().unwrap().to_string())
        .unwrap_or_default();
    let entetes = r.headers().clone();
    let octets = r
        .body_mut()
        .with_config()
        .limit(1 << 26)
        .read_to_vec()
        .unwrap_or_default();
    Rep {
        statut: r.status().as_u16(),
        corps: String::from_utf8_lossy(&octets).to_string(),
        lieu,
        entetes,
    }
}

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
    r.entetes
        .get_all("set-cookie")
        .iter()
        .map(|v| v.to_str().unwrap())
        .find(|c| c.starts_with("plateforme_session="))
        .expect("session")
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

fn csrf(page: &str) -> String {
    let i = page.find("name=\"csrf\" value=\"").expect("jeton csrf") + 19;
    page[i..i + 32].to_string()
}

/// Compte inscrit et confirmé : (cookie, csrf, id).
fn inscrire(b: &Banc, email: &str) -> (String, String, i64) {
    let r = appel(
        b,
        "POST",
        "/inscription",
        &[("Origin", URL)],
        Corps::Formulaire(&[
            ("email", email),
            ("nom", "Claire Dupont"),
            ("mot_de_passe", "phrase-longue-1"),
            ("cgu", "1"),
        ]),
    );
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

fn regler(b: &Banc) -> (String, String) {
    let (c, j, _) = inscrire(b, "admin@e-ko.fr");
    base::nommer_super_admin(&b.r.base(), "admin@e-ko.fr").unwrap();
    let r = appel(
        b,
        "POST",
        "/admin/facturation",
        &[("Origin", URL), ("Cookie", &c)],
        Corps::Formulaire(&[
            ("csrf", &j),
            ("erp_url", &b.url_erp),
            ("erp_jeton", "jeton-secret-erp"),
            ("erp_webhook_secret", SECRET),
        ]),
    );
    assert_eq!(r.lieu, "/admin?ok=facturation", "{}", r.corps);
    (c, j)
}

fn webhook(b: &Banc, corps: &Value, t: u64, secret: &str) -> Rep {
    let brut = serde_json::to_vec(corps).unwrap();
    let signature = erp::signer(secret, &brut, t);
    appel(
        b,
        "POST",
        "/api/erp/webhooks",
        &[
            ("X-EKO-Signature", &signature),
            ("Content-Type", "application/json"),
        ],
        Corps::Octets(brut),
    )
}

fn maintenant() -> u64 {
    eko_rip_cloud::maintenant()
}

fn sujets(b: &Banc) -> Vec<String> {
    b.boite
        .lock()
        .unwrap()
        .iter()
        .map(|c| c.sujet.clone())
        .collect()
}

/// Les e-mails du support (les autres, comme la confirmation d'adresse, ne comptent pas ici).
fn sujets_support(b: &Banc) -> Vec<String> {
    sujets(b)
        .into_iter()
        .filter(|s| s.contains("support") || s.contains("discussion") || s.contains("demande"))
        .collect()
}

fn courriels_de(b: &Banc, debut: &str) -> Vec<Courriel> {
    b.boite
        .lock()
        .unwrap()
        .iter()
        .filter(|c| c.sujet.starts_with(debut))
        .cloned()
        .collect()
}

fn multipart(limite: &str, champs: &[(&str, &str)], fichiers: &[(&str, &str, &[u8])]) -> Vec<u8> {
    let mut o = Vec::new();
    for (n, v) in champs {
        o.extend_from_slice(
            format!("--{limite}\r\nContent-Disposition: form-data; name=\"{n}\"\r\n\r\n{v}\r\n")
                .as_bytes(),
        );
    }
    for (n, f, c) in fichiers {
        o.extend_from_slice(format!("--{limite}\r\nContent-Disposition: form-data; name=\"{n}\"; filename=\"{f}\"\r\nContent-Type: text/plain\r\n\r\n").as_bytes());
        o.extend_from_slice(c);
        o.extend_from_slice(b"\r\n");
    }
    o.extend_from_slice(format!("--{limite}--\r\n").as_bytes());
    o
}

fn ticket_webhook(evenement: &str, ticket: Value, extra: Value) -> Value {
    let mut data = json!({ "ticket": ticket });
    for (k, v) in extra.as_object().cloned().unwrap_or_default() {
        data[k] = v;
    }
    json!({ "event": evenement, "occurred_at": format!("2026-11-04T10:{:02}:00+00:00", maintenant() % 60), "tenant": "support-eko", "data": data })
}

fn depuis_erp(b: &Banc, code: &str) -> Value {
    b.erp
        .lock()
        .unwrap()
        .tickets
        .iter()
        .find(|t| t["code"] == code)
        .cloned()
        .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn demande_de_bout_en_bout() {
    let b = banc().await;
    let b2 = b.clone();
    bl(move || {
        let b = b2;
        // Sans réglage de l'ERP, le support n'est pas ouvert.
        let (c, j, id) = inscrire(&b, "claire@atelier.fr");
        assert!(appel(&b, "GET", "/support", &[("Cookie", &c)], Corps::Rien).corps.contains("pas encore ouvert"));
        assert_eq!(appel(&b, "GET", "/support/direct/etat", &[("Cookie", &c)], Corps::Rien).json()["actif"], false);
        // Non connecté : redirigé vers la connexion ; la bulle et l'état exigent une session.
        assert_eq!(appel(&b, "GET", "/support", &[], Corps::Rien).statut, 303);
        assert_eq!(appel(&b, "GET", "/support/direct/etat", &[], Corps::Rien).statut, 401);
        regler(&b);
        let page = appel(&b, "GET", "/support", &[("Cookie", &c)], Corps::Rien);
        assert!(page.corps.contains("aucune demande") && page.corps.contains("/support/bulle.js"), "{}", page.corps);
        let r = appel(&b, "GET", "/support/bulle.js", &[], Corps::Rien);
        assert!(r.statut == 200 && r.corps.contains("/support/direct/etat"));
        let nouvelle = appel(&b, "GET", "/support/nouvelle", &[("Cookie", &c)], Corps::Rien);
        assert!(nouvelle.corps.contains(r#"value="TECH""#) && nouvelle.corps.contains("Comptabilité") && nouvelle.corps.contains("multipart/form-data"), "{}", nouvelle.corps);

        // Création avec pièce jointe : multipart vers l'ERP, compte désigné par référence et e-mail.
        let limite = "XXlimiteXX";
        let envoyer = |cle: &str, sujet: &str| {
            appel(
                &b,
                "POST",
                "/support/nouvelle",
                &[("Origin", URL), ("Cookie", &c), ("Content-Type", &format!("multipart/form-data; boundary={limite}"))],
                Corps::Octets(multipart(limite, &[("csrf", &j), ("cle", cle), ("categorie", "TECH"), ("sujet", sujet), ("message", "Le RIP ne voit plus l'imprimante")], &[("pieces", "journal.txt", b"ligne de journal")])),
            )
        };
        let r = envoyer("0123456789abcdef", "Imprimante absente");
        assert_eq!((r.statut, r.lieu.as_str()), (303, "/support/TKT-1?ok=creee"), "{}", r.corps);
        {
            let e = b.erp.lock().unwrap();
            let a = e.appels.iter().find(|a| a.methode == "POST" && a.chemin == "/support/tickets").unwrap();
            assert!(a.entetes["content-type"].starts_with("multipart/form-data; boundary="));
            assert_eq!(a.entetes["idempotency-key"], format!("cloud-{id}-0123456789abcdef"));
            assert_eq!(a.entetes["x-integration-source"], erp::SOURCE);
            assert_eq!(extraire(&a.brut, "account_reference").unwrap(), format!("cloud-{id}"));
            assert_eq!(extraire(&a.brut, "tier_email").unwrap(), "claire@atelier.fr");
            assert_eq!(extraire(&a.brut, "contact_first_name").unwrap(), "Claire");
            assert_eq!(extraire(&a.brut, "contact_last_name").unwrap(), "Dupont");
            assert_eq!(extraire(&a.brut, "category").unwrap(), "TECH");
            assert!(a.brut.contains(r#"name="attachments[]"; filename="journal.txt""#) && a.brut.contains("ligne de journal"));
        }
        // Renvoi du même formulaire : même demande (Idempotency-Key), rien de recréé.
        let r = envoyer("0123456789abcdef", "Imprimante absente");
        assert_eq!(r.lieu, "/support/TKT-1?ok=creee");
        assert_eq!(b.erp.lock().unwrap().tickets.len(), 1);
        // Refus du formulaire : file inconnue, sujet vide, jeton anti-falsification, origine.
        let r = appel(&b, "POST", "/support/nouvelle", &[("Origin", URL), ("Cookie", &c), ("Content-Type", &format!("multipart/form-data; boundary={limite}"))], Corps::Octets(multipart(limite, &[("csrf", &j), ("categorie", "NIMPORTE"), ("sujet", "x"), ("message", "y")], &[])));
        assert!(r.corps.contains("Choisissez une file"), "{}", r.corps);
        let r = appel(&b, "POST", "/support/nouvelle", &[("Origin", URL), ("Cookie", &c), ("Content-Type", &format!("multipart/form-data; boundary={limite}"))], Corps::Octets(multipart(limite, &[("csrf", "faux"), ("categorie", "TECH"), ("sujet", "x"), ("message", "y")], &[])));
        assert_eq!(r.statut, 403);
        let r = appel(&b, "POST", "/support/nouvelle", &[("Origin", "http://pirate.test"), ("Cookie", &c), ("Content-Type", &format!("multipart/form-data; boundary={limite}"))], Corps::Octets(multipart(limite, &[("csrf", &j), ("categorie", "TECH"), ("sujet", "x"), ("message", "y")], &[])));
        assert_eq!(r.statut, 403);
        assert_eq!(b.erp.lock().unwrap().tickets.len(), 1);

        let t = maintenant();
        // Webhook : signature vérifiée, puis e-mail « demande reçue » une seule fois.
        let cree = ticket_webhook("support.ticket.created", depuis_erp(&b, "TKT-1"), json!({}));
        assert_eq!(appel(&b, "POST", "/api/erp/webhooks", &[], Corps::Json(cree.clone())).statut, 401);
        assert_eq!(webhook(&b, &cree, t, "mauvais").statut, 401);
        assert!(sujets_support(&b).is_empty());
        let r = webhook(&b, &cree, t, SECRET);
        assert_eq!((r.statut, r.json()["code"].as_str()), (200, Some("traite")), "{}", r.corps);
        let recus = courriels_de(&b, "Votre demande au support E-KO RIP Cloud est reçue");
        assert_eq!(recus.len(), 1);
        assert_eq!(recus[0].a, "claire@atelier.fr");
        assert!(recus[0].texte.contains("TKT-1") && recus[0].texte.contains("Imprimante absente") && recus[0].texte.contains(&format!("{URL}/support/TKT-1")), "{}", recus[0].texte);
        assert!(recus[0].html.contains("E-KO RIP Cloud"));
        // Même événement avec un autre corps (rejoué par l'ERP) : pas de second e-mail.
        let mut cree2 = cree.clone();
        cree2["occurred_at"] = json!("autre");
        webhook(&b, &cree2, t, SECRET);
        assert_eq!(courriels_de(&b, "Votre demande au support E-KO RIP Cloud est reçue").len(), 1);

        // Liste et fil.
        let page = appel(&b, "GET", "/support", &[("Cookie", &c)], Corps::Rien);
        assert!(page.corps.contains("Imprimante absente") && page.corps.contains("TKT-1") && page.corps.contains("Reçue"), "{}", page.corps);
        b.erp.lock().unwrap().fils.get_mut("TKT-1").unwrap().push(json!({ "id": 5, "type": "message", "body": "<p>Réinstallez le pilote &amp; relancez.</p><script>alert(1)</script>", "from_customer": false, "author_name": "Camille", "created_at": "2026-11-04T10:15:00+00:00", "attachments": [] }));
        let page = appel(&b, "GET", "/support/TKT-1", &[("Cookie", &c)], Corps::Rien);
        assert!(page.corps.contains("Le RIP ne voit plus l&#39;imprimante") || page.corps.contains("Le RIP ne voit plus l"), "{}", page.corps);
        assert!(page.corps.contains("Camille") && page.corps.contains("Réinstallez le pilote &amp; relancez."), "{}", page.corps);
        assert!(!page.corps.contains("<script>alert"), "HTML de l'ERP neutralisé");
        assert!(page.corps.contains("journal.txt") && page.corps.contains("Répondre"));
        assert!(!page.corps.contains("Notez le support"));

        // Réponse de l'équipe : e-mail avec extrait sans HTML, et notification du portail.
        let rep = ticket_webhook("support.ticket.replied", depuis_erp(&b, "TKT-1"), json!({ "comment": { "id": 5, "body": "<p>Réinstallez le pilote &amp; relancez.</p>", "from_customer": false, "author_name": "Camille", "created_at": "2026-11-04T10:15:00+00:00" } }));
        assert_eq!(webhook(&b, &rep, t, SECRET).json()["code"], "traite");
        let m = courriels_de(&b, "Réponse du support E-KO RIP Cloud : Imprimante absente");
        assert_eq!(m.len(), 1, "{:?}", sujets(&b));
        assert!(m[0].texte.contains("Camille") && m[0].texte.contains("Réinstallez le pilote & relancez.") && !m[0].texte.contains("<p>"), "{}", m[0].texte);
        let journal = eko_rip_cloud::notifications::depuis(&b.r.base(), id, 0).unwrap();
        assert!(journal.iter().any(|n| n.genre == "support" && n.titre == "Réponse du support" && n.lien == format!("{URL}/support/TKT-1")), "{journal:?}");
        // Message du client lui-même : aucun e-mail.
        let avant = sujets_support(&b).len();
        let moi = ticket_webhook("support.ticket.replied", depuis_erp(&b, "TKT-1"), json!({ "comment": { "id": 6, "body": "<p>Merci</p>", "from_customer": true, "author_name": null, "created_at": "2026-11-04T10:20:00+00:00" } }));
        webhook(&b, &moi, t, SECRET);
        assert_eq!(sujets_support(&b).len(), avant);

        // Réponse du client par le portail.
        let r = appel(&b, "POST", "/support/TKT-1/repondre", &[("Origin", URL), ("Cookie", &c)], Corps::Formulaire(&[("csrf", &j), ("message", "Voici le journal")]));
        assert_eq!(r.lieu, "/support/TKT-1?ok=repondu", "{}", r.corps);
        {
            let e = b.erp.lock().unwrap();
            let a = e.appels.iter().rev().find(|a| a.chemin == "/support/tickets/TKT-1/comments").unwrap();
            assert_eq!((a.corps["message"].as_str(), a.corps["account_reference"].as_str()), (Some("Voici le journal"), Some(format!("cloud-{id}").as_str())));
        }
        let r = appel(&b, "POST", "/support/TKT-1/repondre", &[("Origin", URL), ("Cookie", &c)], Corps::Formulaire(&[("csrf", "faux"), ("message", "x")]));
        assert_eq!(r.statut, 403);

        // Résolution : e-mail avec le lien de note, une seule fois ; la page propose la note.
        b.erp.lock().unwrap().tickets.iter_mut().for_each(|t| t["status"] = json!("resolved"));
        let res = ticket_webhook("support.ticket.status_changed", depuis_erp(&b, "TKT-1"), json!({ "from": "waiting_client", "to": "resolved" }));
        assert_eq!(webhook(&b, &res, t, SECRET).json()["code"], "traite");
        let m = courriels_de(&b, "Votre demande au support E-KO RIP Cloud est résolue");
        assert_eq!(m.len(), 1);
        assert!(m[0].texte.contains(&format!("{URL}/support/TKT-1?note=1#note")), "{}", m[0].texte);
        let mut res2 = res.clone();
        res2["occurred_at"] = json!("encore");
        webhook(&b, &res2, t, SECRET);
        assert_eq!(courriels_de(&b, "Votre demande au support E-KO RIP Cloud est résolue").len(), 1);
        let page = appel(&b, "GET", "/support/TKT-1?note=1", &[("Cookie", &c)], Corps::Rien);
        assert!(page.corps.contains("Notez le support") && page.corps.contains(r#"id="note""#) && page.corps.contains("Résolue"), "{}", page.corps);
        // Note de 1 à 5, avec commentaire.
        let r = appel(&b, "POST", "/support/TKT-1/note", &[("Origin", URL), ("Cookie", &c)], Corps::Formulaire(&[("csrf", &j), ("note", "9")]));
        assert!(r.lieu.contains("erreur=") && b.erp.lock().unwrap().notes.is_empty());
        let r = appel(&b, "POST", "/support/TKT-1/note", &[("Origin", URL), ("Cookie", &c)], Corps::Formulaire(&[("csrf", &j), ("note", "5"), ("commentaire", "Très réactif")]));
        assert_eq!(r.lieu, "/support/TKT-1?ok=note", "{}", r.corps);
        {
            let e = b.erp.lock().unwrap();
            assert_eq!((e.notes["TKT-1"]["rating"].as_i64(), e.notes["TKT-1"]["comment"].as_str()), (Some(5), Some("Très réactif")));
        }
        let page = appel(&b, "GET", "/support", &[("Cookie", &c)], Corps::Rien);
        assert!(page.corps.contains("5/5") && page.corps.contains("Résolue"), "{}", page.corps);

        // Un autre client ne voit rien de cette demande (l'ERP répond 404 à sa référence).
        let (c2, j2, _) = inscrire(&b, "autre@atelier.fr");
        let page = appel(&b, "GET", "/support/TKT-1", &[("Cookie", &c2)], Corps::Rien);
        assert!(page.corps.contains("Introuvable") && !page.corps.contains("Imprimante absente"), "{}", page.corps);
        let r = appel(&b, "POST", "/support/TKT-1/repondre", &[("Origin", URL), ("Cookie", &c2)], Corps::Formulaire(&[("csrf", &j2), ("message", "intrus")]));
        assert!(r.lieu.contains("erreur="));
        assert!(b.erp.lock().unwrap().appels.iter().all(|a| !(a.chemin.ends_with("/comments") && a.corps["message"] == "intrus")));
        let liste = appel(&b, "GET", "/support", &[("Cookie", &c2)], Corps::Rien);
        assert!(!liste.corps.contains("TKT-1"));
        // Webhook d'un compte inconnu : ignoré.
        let inconnu = ticket_webhook("support.ticket.replied", json!({ "code": "TKT-9", "account_reference": "cloud-9999" }), json!({ "comment": { "id": 1, "body": "x", "from_customer": false } }));
        assert_eq!(webhook(&b, &inconnu, t, SECRET).json()["code"], "ignore");
    })
    .await;
    let _ = std::fs::remove_dir_all(&b.donnees);
}

#[tokio::test(flavor = "multi_thread")]
async fn discussion_en_direct_et_conversion() {
    let b = banc().await;
    let b2 = b.clone();
    bl(move || {
        let b = b2;
        regler(&b);
        let (c, j, id) = inscrire(&b, "claire@atelier.fr");
        let en_tete = |c: &str, j: &str| vec![("Origin".to_string(), URL.to_string()), ("Cookie".to_string(), c.to_string()), ("X-CSRF-Token".to_string(), j.to_string())];
        let post = |chemin: &str, corps: Value, c: &str, j: &str| {
            let e = en_tete(c, j);
            let e: Vec<(&str, &str)> = e.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
            appel(&b, "POST", chemin, &e, Corps::Json(corps))
        };
        let etat = |q: &str| appel(&b, "GET", &format!("/support/direct/etat{q}"), &[("Cookie", &c)], Corps::Rien).json();
        // Rien en cours : les files et l'état d'ouverture arrivent avec `files=1`.
        let e = etat("?files=1");
        assert_eq!((e["ouvert"].as_bool(), e["horaires"].as_str()), (Some(true), Some("de 0 h à 23 h 59, du lundi au dimanche")), "{e}");
        assert_eq!((e["actif"].as_bool(), e["discussion"].is_null(), e["categories"].as_array().map(Vec::len)), (Some(true), true, Some(2)), "{e}");
        // Ouverture : l'écriture exige le jeton anti-falsification et l'origine.
        assert_eq!(post("/support/direct/ouvrir", json!({ "message": "Bonjour" }), &c, "faux").statut, 403);
        assert_eq!(appel(&b, "POST", "/support/direct/ouvrir", &[("Cookie", &c), ("X-CSRF-Token", &j)], Corps::Json(json!({ "message": "Bonjour" }))).statut, 403, "origine absente");
        assert_eq!(post("/support/direct/ouvrir", json!({ "message": "  " }), &c, &j).statut, 400);
        let r = post("/support/direct/ouvrir", json!({ "message": "Bonjour, une question", "categorie": "TECH" }), &c, &j);
        assert_eq!(r.statut, 200, "{}", r.corps);
        let e = r.json();
        assert_eq!((e["ouvert"].as_bool(), e["discussion"]["statut"].as_str(), e["messages"].as_array().map(Vec::len)), (Some(true), Some("en_attente"), Some(1)), "{e}");
        let uuid = e["discussion"]["uuid"].as_str().unwrap().to_string();
        {
            let ap = b.erp.lock().unwrap().appels.iter().find(|a| a.chemin == "/support/discussions").cloned().unwrap();
            assert_eq!((ap.corps["account_reference"].as_str(), ap.corps["tier_email"].as_str(), ap.corps["category"].as_str(), ap.corps["locale"].as_str()), (Some(format!("cloud-{id}").as_str()), Some("claire@atelier.fr"), Some("TECH"), Some("fr")));
        }
        assert_eq!(post("/support/direct/ouvrir", json!({ "message": "Encore" }), &c, &j).statut, 409, "une seule discussion à la fois");

        // Le webhook alimente le stockage local ; le navigateur relit depuis son dernier message.
        let t = maintenant();
        let disc = json!({ "uuid": uuid, "status": "en_cours", "origin": "rip_cloud", "account_reference": format!("cloud-{id}"), "category": { "code": "TECH", "label": "Technique" }, "customer_name": "Claire Dupont", "agent_name": "Camille", "ticket_code": null, "created_at": "2026-11-04T09:00:00+00:00" });
        let msg = |n: i64, genre: &str, nom: Value, texte: &str| json!({ "event": "support.discussion.message", "occurred_at": format!("2026-11-04T09:0{n}:00+00:00"), "tenant": "support-eko", "data": { "discussion": disc, "message": { "id": n + 1, "author_type": genre, "author_name": nom, "body": texte, "created_at": format!("2026-11-04T09:0{n}:00+00:00") } } });
        assert_eq!(webhook(&b, &msg(1, "systeme", Value::Null, "Camille a pris la discussion."), t, SECRET).json()["code"], "traite");
        webhook(&b, &msg(2, "agent", json!("Camille"), "Bonjour Claire, je regarde."), t, SECRET);
        let e = etat(&format!("?uuid={uuid}&apres=0"));
        let textes: Vec<&str> = e["messages"].as_array().unwrap().iter().map(|m| m["texte"].as_str().unwrap()).collect();
        assert_eq!(textes, ["Bonjour, une question", "Camille a pris la discussion.", "Bonjour Claire, je regarde."], "{e}");
        assert_eq!((e["discussion"]["statut"].as_str(), e["discussion"]["agent"].as_str()), (Some("en_cours"), Some("Camille")));
        let dernier = e["messages"][2]["id"].as_i64().unwrap();
        assert_eq!(etat(&format!("?uuid={uuid}&apres={dernier}"))["messages"].as_array().unwrap().len(), 0, "rien de nouveau");
        // Un message rejoué n'est pas dupliqué.
        let mut rejoue = msg(2, "agent", json!("Camille"), "Bonjour Claire, je regarde.");
        rejoue["occurred_at"] = json!("autre");
        webhook(&b, &rejoue, t, SECRET);
        assert_eq!(etat(&format!("?uuid={uuid}&apres=0"))["messages"].as_array().unwrap().len(), 3);
        assert!(sujets_support(&b).is_empty(), "aucun e-mail pour un message en direct");
        // Les messages d'une discussion ne sont lisibles que par son compte.
        let (c2, j2, _) = inscrire(&b, "autre@atelier.fr");
        let e2 = appel(&b, "GET", &format!("/support/direct/etat?uuid={uuid}&apres=0"), &[("Cookie", &c2)], Corps::Rien).json();
        assert!(e2["discussion"].is_null() && e2["messages"].as_array().unwrap().is_empty(), "{e2}");
        assert_eq!(post("/support/direct/envoyer", json!({ "message": "intrus" }), &c2, &j2).statut, 409);

        // Réponse du client.
        let r = post("/support/direct/envoyer", json!({ "message": "Merci, voici mon souci", "apres": dernier }), &c, &j);
        assert_eq!(r.statut, 200, "{}", r.corps);
        assert_eq!(r.json()["messages"][0]["texte"], "Merci, voici mon souci");
        assert_eq!(r.json()["messages"][0]["type"], "abonne");
        // Fin de la discussion par le client : elle reste visible, sans nouvel envoi.
        let r = post("/support/direct/fermer", json!({}), &c, &j);
        assert_eq!((r.statut, r.json()["discussion"]["fin"].as_bool()), (200, Some(true)), "{}", r.corps);
        assert_eq!(post("/support/direct/envoyer", json!({ "message": "tard" }), &c, &j).statut, 409);
        let e = etat("?files=1");
        assert!(e["categories"].as_array().is_some_and(|l| l.len() == 2), "files proposées pour recommencer");

        // Support fermé : la discussion devient aussitôt une demande, avec la transcription par e-mail.
        b.erp.lock().unwrap().ouvert = false;
        b.r.base().execute("DELETE FROM reglages WHERE cle LIKE 'support_%'", []).unwrap();
        assert_eq!(etat("?files=1")["ouvert"], false, "horaires de l'ERP : fermé");
        let r = post("/support/direct/ouvrir", json!({ "message": "Question sur la facture" }), &c, &j);
        assert_eq!(r.statut, 200, "{}", r.corps);
        let e = r.json();
        assert_eq!((e["ouvert"].as_bool(), e["discussion"]["statut"].as_str(), e["discussion"]["ticket"].as_str(), e["discussion"]["fin"].as_bool()), (Some(false), Some("convertie"), Some("TKT-77"), Some(true)), "{e}");
        let uuid2 = e["discussion"]["uuid"].as_str().unwrap().to_string();
        let ticket = json!({ "origin": "discussion", "discussion_uuid": "uuid-2", "code": "TKT-77", "label": "Question sur la facture", "status": "open", "category": { "code": "TECH", "label": "Technique" }, "account_reference": format!("cloud-{id}"), "opened_at": "2026-11-04T09:00:02+00:00", "first_response_due_at": "2026-11-05T09:00:00+00:00" });
        // `ticket.created` d'origine `discussion`, avant `converted` : aucun e-mail « reçue ».
        assert_eq!(webhook(&b, &ticket_webhook("support.ticket.created", ticket.clone(), json!({})), t, SECRET).json()["code"], "traite");
        assert!(sujets_support(&b).is_empty(), "{:?}", sujets(&b));
        let conv = json!({ "event": "support.discussion.converted", "occurred_at": "2026-11-04T09:00:03+00:00", "tenant": "support-eko", "data": { "discussion": { "uuid": uuid2, "status": "convertie", "origin": "rip_cloud", "account_reference": format!("cloud-{id}"), "category": { "code": "TECH", "label": "Technique" }, "customer_name": "Claire Dupont", "agent_name": null, "ticket_code": "TKT-77", "created_at": "2026-11-04T09:00:00+00:00" }, "ticket": ticket } });
        assert_eq!(webhook(&b, &conv, t, SECRET).json()["code"], "traite");
        let m = courriels_de(&b, "Votre discussion est devenue la demande TKT-77");
        assert_eq!(m.len(), 1, "{:?}", sujets(&b));
        assert!(m[0].texte.contains("Vous : Question sur la facture") && m[0].texte.contains(&format!("{URL}/support/TKT-77")), "{}", m[0].texte);
        assert!(!m[0].texte.contains("devient une demande"), "messages système hors transcription");
        // Rejoué (autre corps) : une seule fois.
        let mut conv2 = conv.clone();
        conv2["occurred_at"] = json!("autre");
        webhook(&b, &conv2, t, SECRET);
        assert_eq!(courriels_de(&b, "Votre discussion est devenue").len(), 1);
        // La demande est dans la liste locale.
        assert!(appel(&b, "GET", "/support", &[("Cookie", &c)], Corps::Rien).corps.contains("Question sur la facture"));
        // « Nouvelle discussion » : la discussion terminée n'est plus montrée, une nouvelle s'ouvre.
        assert_eq!(post("/support/direct/congedier", json!({}), &c2, &j2).statut, 200);
        assert_eq!(etat("")["discussion"]["fin"], true, "congédier ne touche que le compte de la session");
        let r = post("/support/direct/congedier", json!({}), &c, &j);
        assert!(r.json()["discussion"].is_null(), "{}", r.corps);
        assert!(etat("?files=1")["discussion"].is_null());
        b.erp.lock().unwrap().ouvert = true;
        let r = post("/support/direct/ouvrir", json!({ "message": "Autre question" }), &c, &j);
        assert_eq!((r.statut, r.json()["discussion"]["statut"].as_str()), (200, Some("en_attente")), "{}", r.corps);
    })
    .await;
    let _ = std::fs::remove_dir_all(&b.donnees);
}

#[tokio::test(flavor = "multi_thread")]
async fn demande_ouverte_ici_prevenue_sur_son_origine() {
    let b = banc().await;
    let b2 = b.clone();
    bl(move || {
        let b = b2;
        regler(&b);
        let (c, j, id) = inscrire(&b, "claire@atelier.fr");
        let r = appel(
            &b,
            "POST",
            "/support/nouvelle",
            &[
                ("Origin", URL),
                ("Cookie", &c),
                ("Content-Type", "multipart/form-data; boundary=L"),
            ],
            Corps::Octets(multipart(
                "L",
                &[
                    ("csrf", &j),
                    ("categorie", "COMPTA"),
                    ("sujet", "Facture"),
                    ("message", "Bonjour"),
                ],
                &[],
            )),
        );
        assert_eq!(r.lieu, "/support/TKT-1?ok=creee", "{}", r.corps);
        // Sans pièce jointe : JSON.
        let ap = b
            .erp
            .lock()
            .unwrap()
            .appels
            .iter()
            .find(|a| a.chemin == "/support/tickets")
            .cloned()
            .unwrap();
        assert_eq!(
            (
                ap.corps["category"].as_str(),
                ap.corps["subject"].as_str(),
                ap.corps["account_reference"].as_str()
            ),
            (
                Some("COMPTA"),
                Some("Facture"),
                Some(format!("cloud-{id}").as_str())
            )
        );
        // Le webhook `created` arrive avant la réponse de l'API (demande encore inconnue ici) :
        // `origin: request` suffit pour envoyer « demande reçue ».
        let t = maintenant();
        b.r.base()
            .execute("DELETE FROM support_demandes", [])
            .unwrap();
        webhook(
            &b,
            &ticket_webhook("support.ticket.created", depuis_erp(&b, "TKT-1"), json!({})),
            t,
            SECRET,
        );
        assert_eq!(
            courriels_de(&b, "Votre demande au support E-KO RIP Cloud est reçue").len(),
            1
        );
    })
    .await;
    let _ = std::fs::remove_dir_all(&b.donnees);
}

#[tokio::test(flavor = "multi_thread")]
async fn webhook_unique_abonne_aux_evenements_support() {
    let b = banc().await;
    let b2 = b.clone();
    bl(move || {
        let b = b2;
        let (c, j, _) = inscrire(&b, "admin@e-ko.fr");
        base::nommer_super_admin(&b.r.base(), "admin@e-ko.fr").unwrap();
        let formulaire = [
            ("csrf", j.as_str()),
            ("erp_url", b.url_erp.as_str()),
            ("erp_jeton", "jeton-secret-erp"),
            ("action", "configurer"),
        ];
        let _ = appel(
            &b,
            "POST",
            "/admin/facturation",
            &[("Origin", URL), ("Cookie", &c)],
            Corps::Formulaire(&formulaire),
        );
        // Le faux ERP de ce fichier ne connaît pas les plans (la configuration s'arrête avant le webhook) :
        // les événements sont vérifiés dans la liste de la plate-forme.
        for e in [
            "support.ticket.created",
            "support.ticket.replied",
            "support.ticket.status_changed",
            "support.discussion.message",
            "support.discussion.converted",
        ] {
            assert!(erp::EVENEMENTS_WEBHOOK.contains(&e), "{e}");
        }
        assert!(
            erp::EVENEMENTS_WEBHOOK.contains(&"cycle.paid"),
            "même webhook que la facturation"
        );
        let page = appel(&b, "GET", "/admin", &[("Cookie", &c)], Corps::Rien);
        assert!(
            page.corps.contains("support.view") && page.corps.contains("support.create"),
            "droits de jeton listés dans la carte"
        );
    })
    .await;
    let _ = std::fs::remove_dir_all(&b.donnees);
}

#[test]
fn horaires_calcules_localement() {
    let h = json!({ "start": "09:00", "end": "18:00", "days": [1, 2, 3, 4, 5], "timezone": "Europe/Paris" });
    // Mercredi 4 novembre 2026, heure d'hiver (UTC+1) : 09:00 Paris = 08:00 UTC.
    let mercredi = eko_rip_cloud::offres::date_iso("2026-11-04").unwrap();
    let ouvert = |h_utc: u64, m: u64| {
        eko_rip_cloud::support::ouvert_selon(&h, mercredi + h_utc * 3600 + m * 60)
    };
    assert_eq!(ouvert(7, 59), Some(false));
    assert_eq!(ouvert(8, 0), Some(true));
    assert_eq!(ouvert(16, 59), Some(true));
    assert_eq!(ouvert(17, 0), Some(false));
    // Samedi 7 novembre : fermé toute la journée. Été (UTC+2) : 15 juillet 2026, 09:00 Paris = 07:00 UTC.
    assert_eq!(
        eko_rip_cloud::support::ouvert_selon(
            &h,
            eko_rip_cloud::offres::date_iso("2026-11-07").unwrap() + 10 * 3600
        ),
        Some(false)
    );
    let ete = eko_rip_cloud::offres::date_iso("2026-07-15").unwrap();
    assert_eq!(
        eko_rip_cloud::support::ouvert_selon(&h, ete + 7 * 3600),
        Some(true)
    );
    assert_eq!(
        eko_rip_cloud::support::ouvert_selon(&h, ete + 6 * 3600 + 59 * 60),
        Some(false)
    );
    // Autre fuseau : non calculé ici.
    assert_eq!(
        eko_rip_cloud::support::ouvert_selon(
            &json!({ "start": "09:00", "end": "18:00", "days": [1], "timezone": "America/Cayenne" }),
            mercredi
        ),
        None
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn bulle_autorisee_par_la_politique_de_securite_et_creation_rejouable() {
    let b = banc().await;
    let b2 = b.clone();
    bl(move || {
        let b = b2;
        regler(&b);
        let (c, j, _) = inscrire(&b, "claire@atelier.fr");
        // Le script de la bulle est externe : la CSP réelle de la page doit l'autoriser ('self').
        let page = appel(&b, "GET", "/support", &[("Cookie", &c)], Corps::Rien);
        let csp = page
            .entetes
            .get("content-security-policy")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert!(csp.contains("script-src 'self' 'unsafe-inline'"), "{csp}");
        assert!(page.corps.contains(r#"<script src="/support/bulle.js""#));
        assert_eq!(
            appel(&b, "GET", "/support/bulle.js", &[], Corps::Rien).statut,
            200
        );
        // L'ERP refuse (jeton retiré) : le formulaire revient avec la même clé et les mêmes textes.
        let envoyer = |cle: &str| {
            appel(
                &b,
                "POST",
                "/support/nouvelle",
                &[
                    ("Origin", URL),
                    ("Cookie", &c),
                    ("Content-Type", "multipart/form-data; boundary=L"),
                ],
                Corps::Octets(multipart(
                    "L",
                    &[
                        ("csrf", &j),
                        ("cle", cle),
                        ("categorie", "TECH"),
                        ("sujet", "Sujet gardé"),
                        ("message", "Texte gardé"),
                    ],
                    &[("pieces", "a.txt", b"x")],
                )),
            )
        };
        assert!(
            appel(
                &b,
                "GET",
                "/support/nouvelle",
                &[("Cookie", &c)],
                Corps::Rien
            )
            .corps
            .contains("Comptabilité")
        );
        base::regler(&b.r.base(), "erp_jeton", "mauvais-jeton").unwrap();
        let r = envoyer("abcdef0123456789");
        assert!(
            r.corps.contains(r#"name="cle" value="abcdef0123456789""#),
            "{}",
            r.corps
        );
        assert!(
            r.corps.contains("Sujet gardé")
                && r.corps.contains("Texte gardé")
                && r.corps.contains("sélectionner de nouveau"),
            "{}",
            r.corps
        );
        // Renvoi avec la clé gardée une fois l'ERP revenu : une seule demande, même si l'envoi précédent avait abouti sans réponse.
        base::regler(&b.r.base(), "erp_jeton", "jeton-secret-erp").unwrap();
        assert_eq!(envoyer("abcdef0123456789").lieu, "/support/TKT-1?ok=creee");
        assert_eq!(envoyer("abcdef0123456789").lieu, "/support/TKT-1?ok=creee");
        assert_eq!(b.erp.lock().unwrap().tickets.len(), 1);
    })
    .await;
    let _ = std::fs::remove_dir_all(&b.donnees);
}
