//! « Mes demandes » de bout en bout, contre un faux support : menu masqué sans réglages, contact
//! créé avec l'identité signée, création avec pièce jointe, fil sans notes internes, réponse,
//! clôture, webhook signé (refusé si faux ou trop vieux) et e-mails sans le texte de l'agent.
//!
//! Un seul test : la configuration des demandes est commune au processus.

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use eko_rip_cloud::demandes::{self, Demandes};
use eko_rip_cloud::{base, relais};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const URL: &str = "http://plateforme.test";
const BOITE: &str = "boite-demandes";
const CLE: &str = "cle-hmac-des-demandes-0123456789";
const SECRET: &str = "secret-du-webhook-0123456789";
const SOURCE: &str = "source-contact-1";

#[derive(Default)]
struct Faux {
    contacts: Vec<Value>,
    conversations: Vec<Value>,
    /// Attributs reçus à la création (l'API client du support ne les rend jamais).
    attributs: Vec<Value>,
    messages: Vec<Value>,
    /// Corps bruts des messages envoyés (type de contenu, octets).
    envois: Vec<(String, Vec<u8>)>,
}

type Etat = Arc<Mutex<Faux>>;

async fn servir(app: Router) -> String {
    let ecoute = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let adresse = format!("http://{}", ecoute.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(ecoute, app).await.unwrap() });
    adresse
}

fn faux_support(etat: Etat) -> Router {
    async fn contact(State(e): State<Etat>, Path(boite): Path<String>, Json(v): Json<Value>) -> Json<Value> {
        assert_eq!(boite, BOITE);
        e.lock().unwrap().contacts.push(v.clone());
        Json(json!({ "source_id": SOURCE, "name": v["name"] }))
    }
    async fn conversations(State(e): State<Etat>, Path((_, s)): Path<(String, String)>) -> Result<Json<Value>, axum::http::StatusCode> {
        if s != SOURCE {
            return Err(axum::http::StatusCode::NOT_FOUND);
        }
        let f = e.lock().unwrap();
        let v: Vec<Value> = f.conversations.iter().map(|c| {
            let mut c = c.clone();
            c["messages"] = json!(f.messages.iter().filter(|m| m["conversation_id"] == c["id"]).cloned().collect::<Vec<_>>());
            c
        }).collect();
        Ok(Json(json!(v)))
    }
    async fn creer(State(e): State<Etat>, Path((_, s)): Path<(String, String)>, Json(v): Json<Value>) -> Json<Value> {
        assert_eq!(s, SOURCE);
        let mut f = e.lock().unwrap();
        let id = 101 + f.conversations.len() as u64;
        // Comme le vrai support : pas de custom_attributes dans les réponses de l'API client.
        let c = json!({ "id": id, "status": "open", "agent_last_seen_at": 0, "contact_last_seen_at": 0 });
        f.attributs.push(v["custom_attributes"].clone());
        f.conversations.push(c.clone());
        Json(c)
    }
    async fn une(State(e): State<Etat>, Path((_, s, id)): Path<(String, String, u64)>) -> Result<Json<Value>, axum::http::StatusCode> {
        let f = e.lock().unwrap();
        match f.conversations.iter().find(|c| c["id"] == id) {
            Some(c) if s == SOURCE => Ok(Json(c.clone())),
            _ => Err(axum::http::StatusCode::NOT_FOUND),
        }
    }
    async fn messages(State(e): State<Etat>, Path((_, _, id)): Path<(String, String, u64)>) -> Json<Value> {
        let f = e.lock().unwrap();
        Json(json!(f.messages.iter().filter(|m| m["conversation_id"] == id).cloned().collect::<Vec<_>>()))
    }
    async fn ecrire(State(e): State<Etat>, Path((_, _, id)): Path<(String, String, u64)>, h: HeaderMap, corps: Bytes) -> Json<Value> {
        let genre = h.get("content-type").map(|v| v.to_str().unwrap().to_string()).unwrap_or_default();
        let texte = String::from_utf8_lossy(&corps).to_string();
        let contenu = if genre.starts_with("application/json") {
            serde_json::from_str::<Value>(&texte).unwrap()["content"].as_str().unwrap_or("").to_string()
        } else {
            texte.split("name=\"content\"\r\n\r\n").nth(1).and_then(|x| x.split("\r\n--").next()).unwrap_or("").to_string()
        };
        let mut f = e.lock().unwrap();
        let n = f.messages.len() as u64;
        let m = json!({ "id": 1000 + n, "conversation_id": id, "content": contenu, "message_type": 0, "private": false, "created_at": 1_800_000_000 + n, "sender": { "name": "Claire Dupont" } });
        f.messages.push(m.clone());
        f.envois.push((genre, corps.to_vec()));
        Json(m)
    }
    async fn basculer(State(e): State<Etat>, Path((_, _, id)): Path<(String, String, u64)>) -> Json<Value> {
        let mut f = e.lock().unwrap();
        let c = f.conversations.iter_mut().find(|c| c["id"] == id).unwrap();
        c["status"] = json!(if c["status"] == "resolved" { "open" } else { "resolved" });
        Json(c.clone())
    }
    async fn vu() -> Json<Value> {
        Json(json!({}))
    }
    let p = "/public/api/v1/inboxes/{boite}/contacts";
    Router::new()
        .route(p, post(contact))
        .route(&format!("{p}/{{s}}/conversations"), get(conversations).post(creer))
        .route(&format!("{p}/{{s}}/conversations/{{id}}"), get(une))
        .route(&format!("{p}/{{s}}/conversations/{{id}}/messages"), get(messages).post(ecrire))
        .route(&format!("{p}/{{s}}/conversations/{{id}}/toggle_status"), post(basculer))
        .route(&format!("{p}/{{s}}/conversations/{{id}}/update_last_seen"), post(vu))
        .with_state(etat)
}

struct Rep {
    statut: u16,
    corps: String,
    lieu: String,
    entetes: HeaderMap,
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder().http_status_as_error(false).max_redirects(0).timeout_global(Some(Duration::from_secs(20))).build().into()
}

fn lire(r: Result<ureq::http::Response<ureq::Body>, ureq::Error>) -> Rep {
    let mut r = r.unwrap();
    let lieu = r.headers().get("location").map(|v| v.to_str().unwrap().to_string()).unwrap_or_default();
    let entetes = r.headers().clone();
    let corps = r.body_mut().read_to_string().unwrap_or_default();
    Rep { statut: r.status().as_u16(), corps, lieu, entetes }
}

fn page(base: &str, chemin: &str, cookie: &str) -> Rep {
    lire(agent().get(&format!("{base}{chemin}")).header("Cookie", cookie).call())
}

fn formulaire(base: &str, chemin: &str, cookie: Option<&str>, champs: &[(&str, &str)]) -> Rep {
    let mut q = agent().post(&format!("{base}{chemin}")).header("Origin", URL);
    if let Some(c) = cookie {
        q = q.header("Cookie", c);
    }
    lire(q.send_form(champs.iter().copied()))
}

/// Envoi `multipart/form-data` : champs texte, puis pièces (nom, type, octets) dans `pieces`.
fn multipart(base: &str, chemin: &str, cookie: &str, champs: &[(&str, &str)], pieces: &[(&str, &str, &[u8])]) -> Rep {
    let limite = "limite-de-test-0123";
    let mut corps = Vec::new();
    for (n, v) in champs {
        corps.extend_from_slice(format!("--{limite}\r\nContent-Disposition: form-data; name=\"{n}\"\r\n\r\n{v}\r\n").as_bytes());
    }
    for (nom, genre, octets) in pieces {
        corps.extend_from_slice(format!("--{limite}\r\nContent-Disposition: form-data; name=\"pieces\"; filename=\"{nom}\"\r\nContent-Type: {genre}\r\n\r\n").as_bytes());
        corps.extend_from_slice(octets);
        corps.extend_from_slice(b"\r\n");
    }
    corps.extend_from_slice(format!("--{limite}--\r\n").as_bytes());
    lire(agent().post(&format!("{base}{chemin}")).header("Origin", URL).header("Cookie", cookie).header("Content-Type", &format!("multipart/form-data; boundary={limite}")).send(&corps[..]))
}

fn webhook(base: &str, corps: &str, horodatage: u64, signature: Option<String>) -> Rep {
    let t = horodatage.to_string();
    let sig = signature.unwrap_or_else(|| demandes::configuration().unwrap().signature(&t, corps.as_bytes()));
    lire(agent().post(&format!("{base}/webhooks/support")).header("Content-Type", "application/json").header("X-Chatwoot-Timestamp", &t).header("X-Chatwoot-Signature", &sig).send(corps.as_bytes()))
}

fn csrf(page: &str) -> String {
    let i = page.find("name=\"csrf\" value=\"").expect("jeton csrf") + 19;
    page[i..].split('"').next().unwrap().to_string()
}

fn cookie(r: &Rep) -> String {
    r.entetes.get_all("set-cookie").iter().map(|v| v.to_str().unwrap()).find(|c| c.starts_with("plateforme_session=")).expect("session").split(';').next().unwrap().to_string()
}

#[tokio::test(flavor = "multi_thread")]
async fn mes_demandes_de_bout_en_bout() {
    let etat: Etat = Arc::new(Mutex::new(Faux::default()));
    let support = servir(faux_support(etat.clone())).await;
    let donnees = std::env::temp_dir().join(format!("eko-cloud-demandes-{}", eko_rip_cloud::aleatoire_hex(6)));
    let r = relais::nouveau(relais::Config { donnees: donnees.clone(), url_publique: URL.into(), max_octets: 1 << 20, jeton_stats: String::new() });
    let boite_mails = r.utiliser_memoire();
    let base_url = servir(relais::routes(r.clone())).await;
    let (b, rb, e) = (base_url.clone(), r.clone(), etat.clone());
    tokio::task::spawn_blocking(move || {
        let ins = formulaire(&b, "/inscription", None, &[("email", "claire@atelier.fr"), ("nom", "Claire Dupont"), ("mot_de_passe", "phrase-longue-1"), ("cgu", "1")]);
        assert_eq!(ins.statut, 303, "{}", ins.corps);
        let c = cookie(&ins);
        let id = base::compte_par_email(&rb.base(), "claire@atelier.fr").unwrap().unwrap().id;

        // Sans réglages : pas de « Mes demandes », le formulaire renvoie vers /support.
        demandes::configurer(None);
        let p = page(&b, "/support", &c);
        assert_eq!(p.statut, 200);
        assert!(!p.corps.contains("Nouvelle demande"), "{}", p.corps);
        assert_eq!(page(&b, "/support/nouvelle", &c).lieu, "/support");
        assert_eq!(webhook(&b, "{}", eko_rip_cloud::maintenant(), Some("sha256=00".into())).statut, 503);

        // Réglages complets.
        demandes::configurer(Demandes::depuis(Some(BOITE), Some(CLE), Some(SECRET), Some(&support)));
        let p = page(&b, "/support", &c);
        assert!(p.corps.contains("Mes demandes") && p.corps.contains("href=\"/support/nouvelle\""), "{}", p.corps);
        assert!(!p.corps.contains(CLE) && !p.corps.contains(SECRET) && !p.corps.contains(SOURCE));
        // Contact créé une seule fois, avec l'identité signée.
        {
            let f = e.lock().unwrap();
            assert_eq!(f.contacts.len(), 1);
            let ident = format!("cloud-{id}");
            assert_eq!(f.contacts[0]["identifier"], ident);
            assert_eq!(f.contacts[0]["identifier_hash"], eko_rip_cloud::hex(&eko_rip_cloud::hmac_sha256(CLE.as_bytes(), ident.as_bytes())));
            assert_eq!(f.contacts[0]["email"], "claire@atelier.fr");
        }

        // Nouvelle demande avec une capture.
        let f = page(&b, "/support/nouvelle", &c);
        assert_eq!(f.statut, 200);
        let jeton = csrf(&f.corps);
        let refus = multipart(&b, "/support/nouvelle", &c, &[("csrf", &jeton), ("objet", "x"), ("message", "y")], &[("outil.exe", "application/octet-stream", b"MZ")]);
        assert!(refus.corps.contains("seuls les images"), "{}", refus.corps);
        let png: &[u8] = b"\x89PNG\r\n\x1a\nfaux";
        let cree = multipart(
            &b,
            "/support/nouvelle",
            &c,
            &[("csrf", &jeton), ("type_demande", "technique"), ("urgence", "bloquante"), ("objet", "Le Kona ne coupe plus"), ("module", "compte"), ("message", "Depuis ce matin la découpe s'arrête.")],
            &[("capture.png", "image/png", png)],
        );
        assert_eq!((cree.statut, cree.lieu.as_str()), (303, "/support/101"), "{}", cree.corps);
        {
            let f = e.lock().unwrap();
            assert_eq!(f.contacts.len(), 1, "contact réutilisé");
            let a = &f.attributs[0];
            assert_eq!((a["type_demande"].as_str(), a["urgence"].as_str(), a["objet"].as_str(), a["appli"].as_str()), (Some("technique"), Some("bloquante"), Some("Le Kona ne coupe plus"), Some("rip-cloud")));
            let (genre, octets) = &f.envois[0];
            assert!(genre.starts_with("multipart/form-data"), "{genre}");
            let brut = String::from_utf8_lossy(octets);
            assert!(brut.contains("name=\"attachments[]\"; filename=\"capture.png\"") && brut.contains("Depuis ce matin"), "{brut}");
        }

        // Réponse de l'agent et note interne : seule la réponse est visible.
        {
            let mut f = e.lock().unwrap();
            f.messages.push(json!({ "id": 2000, "conversation_id": 101, "content": "Pouvez-vous redémarrer le Kona ?", "message_type": 1, "private": false, "created_at": 1_800_000_100u64, "sender": { "available_name": "Julie" }, "attachments": [{ "data_url": format!("{support}/rails/active_storage/notice%20kona.pdf") }] }));
            f.messages.push(json!({ "id": 2001, "conversation_id": 101, "content": "note interne confidentielle", "message_type": 1, "private": true, "created_at": 1_800_000_101u64 }));
            f.conversations[0]["agent_last_seen_at"] = json!(1_800_000_100u64);
        }
        let liste = page(&b, "/support", &c);
        assert!(liste.corps.contains("Le Kona ne coupe plus") && liste.corps.contains("/support/101"), "{}", liste.corps);
        let fil = page(&b, "/support/101", &c);
        assert_eq!(fil.statut, 200);
        assert!(fil.corps.contains("Pouvez-vous redémarrer le Kona ?") && fil.corps.contains("Julie · support E-KO"), "{}", fil.corps);
        assert!(fil.corps.contains("notice kona.pdf"), "{}", fil.corps);
        assert!(!fil.corps.contains("note interne confidentielle"));
        assert!(fil.corps.contains("Marquer comme résolue"));
        // Une demande d'un autre contact n'est pas lisible.
        assert_eq!(page(&b, "/support/999", &c).lieu, "/support");
        assert_eq!(e.lock().unwrap().contacts.len(), 1, "une demande introuvable ne recrée pas le contact");
        // Type, urgence et objet gardés côté plate-forme, puisque le support ne les rend pas.
        assert!(fil.corps.contains("Le Kona ne coupe plus") && fil.corps.contains("Question technique") && fil.corps.contains("urgence bloquante"), "{}", fil.corps);
        assert!(liste.corps.contains("Question technique"), "{}", liste.corps);

        // Réponse du client, puis clôture.
        let jeton = csrf(&fil.corps);
        let rep = multipart(&b, "/support/101/repondre", &c, &[("csrf", &jeton), ("message", "C'est reparti, merci.")], &[]);
        assert_eq!((rep.statut, rep.lieu.as_str()), (303, "/support/101"), "{}", rep.corps);
        assert!(e.lock().unwrap().messages.iter().any(|m| m["content"] == "C'est reparti, merci."));
        assert_eq!(multipart(&b, "/support/101/repondre", &c, &[("csrf", "faux"), ("message", "x")], &[]).statut, 403);
        assert_eq!(formulaire(&b, "/support/101/resoudre", Some(&c), &[("csrf", "faux")]).statut, 403);
        let clos = formulaire(&b, "/support/101/resoudre", Some(&c), &[("csrf", &jeton)]);
        assert_eq!((clos.statut, clos.lieu.as_str()), (303, "/support/101"));
        assert_eq!(e.lock().unwrap().conversations[0]["status"], "resolved");
        // Une seconde clôture ne rouvre pas la demande.
        formulaire(&b, "/support/101/resoudre", Some(&c), &[("csrf", &jeton)]);
        assert_eq!(e.lock().unwrap().conversations[0]["status"], "resolved");
        assert!(!page(&b, "/support/101", &c).corps.contains("Marquer comme résolue"));

        // Webhooks.
        let maintenant = eko_rip_cloud::maintenant();
        let reponse = json!({ "event": "message_created", "message_type": "outgoing", "private": false, "content": "TEXTE-DE-L-AGENT", "conversation": { "id": 101, "contact_inbox": { "source_id": SOURCE } } }).to_string();
        assert_eq!(webhook(&b, &reponse, maintenant, Some("sha256=deadbeef".into())).statut, 401);
        assert_eq!(webhook(&b, &reponse, maintenant - 600, None).statut, 401);
        assert_eq!(webhook(&b, &reponse, maintenant, None).statut, 200);
        // Rejoué : pas de second e-mail.
        assert_eq!(webhook(&b, &reponse, maintenant, None).statut, 200);
        let note = json!({ "event": "message_created", "message_type": "outgoing", "private": true, "content": "interne", "conversation": { "id": 101, "contact_inbox": { "source_id": SOURCE } } }).to_string();
        assert_eq!(webhook(&b, &note, maintenant, None).statut, 200);
        let entrant = json!({ "event": "message_created", "message_type": "incoming", "private": false, "content": "moi", "conversation": { "id": 101, "contact_inbox": { "source_id": SOURCE } } }).to_string();
        assert_eq!(webhook(&b, &entrant, maintenant, None).statut, 200);
        let resolue = json!({ "event": "conversation_status_changed", "id": 101, "status": "resolved", "contact_inbox": { "source_id": SOURCE } }).to_string();
        assert_eq!(webhook(&b, &resolue, maintenant, None).statut, 200);
        let autre = json!({ "event": "conversation_created", "id": 102, "contact_inbox": { "source_id": SOURCE } }).to_string();
        assert_eq!(webhook(&b, &autre, maintenant, None).statut, 200);

        // Traitement hors requête : on attend les deux e-mails.
        let lus = || boite_mails.lock().unwrap().iter().filter(|m| m.sujet.contains("demande n° 101")).cloned().collect::<Vec<_>>();
        for _ in 0..100 {
            if lus().len() >= 2 {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        // Laisse le temps à un éventuel e-mail en trop d'arriver.
        std::thread::sleep(Duration::from_millis(300));
        let mails = lus();
        let sujets: Vec<&str> = mails.iter().map(|m| m.sujet.as_str()).collect();
        assert_eq!(sujets, vec!["Nouvelle réponse à votre demande n° 101", "Votre demande n° 101 est résolue"], "{sujets:?}");
        for m in &mails {
            assert_eq!(m.a, "claire@atelier.fr");
            assert!(m.html.contains(&format!("{URL}/support/101")), "{}", m.html);
            assert!(!m.html.contains("TEXTE-DE-L-AGENT") && !m.texte.contains("TEXTE-DE-L-AGENT"));
        }
    })
    .await
    .unwrap();
    let _ = std::fs::remove_dir_all(donnees);
}
