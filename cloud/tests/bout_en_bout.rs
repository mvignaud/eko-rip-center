//! Bout en bout : relais réel sur un port local, « serveur RIP » de test branché par le
//! vrai client de tunnel, requêtes HTTP ordinaires vers `/s/<site>/…`.

use axum::Router;
use axum::http::{HeaderMap, header};
use axum::routing::{get, post};
use eko_rip_cloud::{appairage, client, relais, sites};
use std::path::PathBuf;
use std::time::Duration;

fn dossier() -> PathBuf {
    std::env::temp_dir().join(format!("eko-cloud-e2e-{}", eko_rip_cloud::aleatoire_hex(6)))
}

/// Service du faux serveur RIP : renvoie ce qu'il a reçu.
fn rip() -> Router {
    Router::new()
        .route(
            "/api/echo",
            get(|h: HeaderMap, uri: axum::http::Uri| async move {
                let v = |n: header::HeaderName| h.get(n).and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
                (
                    [(header::SET_COOKIE, "eko_rip_session=jeton; HttpOnly; SameSite=Strict; Path=/; Max-Age=60")],
                    axum::Json(serde_json::json!({ "cookie": v(header::COOKIE), "authorization": v(header::AUTHORIZATION), "agent": v(header::USER_AGENT), "uri": uri.to_string(), "eko": v(header::HeaderName::from_static("x-eko-cloud-identite")) })),
                )
            }),
        )
        .route("/api/taille", post(|corps: axum::body::Bytes| async move { corps.len().to_string() }))
        .layer(axum::extract::DefaultBodyLimit::disable())
}

async fn relais_local(donnees: &std::path::Path, max: u64) -> String {
    let r = relais::nouveau(relais::Config { donnees: donnees.to_path_buf(), url_publique: "https://platform.test".into(), max_octets: max, jeton_stats: "jeton-stats".into() });
    let ecoute = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let adresse = format!("http://{}", ecoute.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(ecoute, relais::routes(r)).await.unwrap() });
    adresse
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder().http_status_as_error(false).timeout_global(Some(Duration::from_secs(20))).build().into()
}

async fn bloquant<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    tokio::task::spawn_blocking(f).await.unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn tunnel_appairage_et_isolement() {
    let d = dossier();
    let base = relais_local(&d, 1024 * 1024).await;

    // Appairage : demande, code validé en ligne de commande, clé reçue une seule fois.
    let b = base.clone();
    let demande: serde_json::Value = bloquant(move || agent().post(format!("{b}/api/appairage")).send_json(serde_json::json!({ "nom": "Atelier test" })).unwrap().body_mut().read_json().unwrap()).await;
    let id = demande["demande"].as_str().unwrap().to_string();
    let code = demande["code"].as_str().unwrap().to_string();
    assert!(appairage::normaliser(&code).is_some());
    assert_eq!(demande["lien"], format!("https://platform.test/appairer?code={code}"));
    let (b, i) = (base.clone(), id.clone());
    let attente: serde_json::Value = bloquant(move || agent().get(format!("{b}/api/appairage/{i}")).call().unwrap().body_mut().read_json().unwrap()).await;
    assert_eq!(attente["etat"], "en_attente");
    appairage::valider(&d, &code, "essai").unwrap();
    let (b, i) = (base.clone(), id.clone());
    let valide: serde_json::Value = bloquant(move || agent().get(format!("{b}/api/appairage/{i}")).call().unwrap().body_mut().read_json().unwrap()).await;
    assert_eq!(valide["etat"], "valide");
    let site = valide["site"].as_str().unwrap().to_string();
    let cle = valide["cle"].as_str().unwrap().to_string();
    assert_eq!(valide["adresse"], format!("https://platform.test/s/{site}"));
    let (b, i) = (base.clone(), id.clone());
    assert_eq!(bloquant(move || agent().get(format!("{b}/api/appairage/{i}")).call().unwrap().status().as_u16()).await, 404);
    assert_eq!(sites::lire(&d).unwrap()[0].proprietaire.as_deref(), Some("essai"));

    // Site pas encore connecté : hors ligne.
    let (b, s) = (base.clone(), site.clone());
    assert_eq!(bloquant(move || agent().get(format!("{b}/s/{s}/api/echo")).call().unwrap().status().as_u16()).await, 502);

    // Mauvaise clé : refusée avec son code.
    let r = client::servir(&base, &site, "mauvaise", "essai", rip(), || {}).await;
    assert_eq!(r, Err(client::Erreur::Refus { statut: 401, code: "cle_invalide".into() }));
    let r = client::servir(&base, "inconnu", &cle, "essai", rip(), || {}).await;
    assert_eq!(r, Err(client::Erreur::Refus { statut: 401, code: "cle_invalide".into() }));

    // Bonne clé : le tunnel tient.
    let (tx, rx) = tokio::sync::oneshot::channel();
    let (b, s, c) = (base.clone(), site.clone(), cle.clone());
    let tunnel = tokio::spawn(async move { client::servir(&b, &s, &c, "essai", rip(), move || tx.send(()).unwrap()).await });
    rx.await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let (b, s) = (base.clone(), site.clone());
    let (cookies, json): (Vec<String>, serde_json::Value) = bloquant(move || {
        let mut r = agent()
            .get(format!("{b}/s/{s}/api/echo?a=1&b=2"))
            .header("Cookie", "portail_session=secret; eko_rip_session=abc")
            .header("Authorization", "Bearer jeton-du-cloud")
            .header("X-EKO-Cloud-Identite", "falsifiee")
            .header("User-Agent", "EKO-RIP-Client/1.0")
            .call()
            .unwrap();
        assert_eq!(r.status().as_u16(), 200);
        assert_eq!(r.headers()["content-security-policy"], "sandbox; frame-ancestors 'none'");
        let c = r.headers().get_all("set-cookie").iter().map(|v| v.to_str().unwrap().to_string()).collect();
        (c, r.body_mut().read_json().unwrap())
    })
    .await;
    assert_eq!(json["uri"].as_str().unwrap().rsplit_once("rip").unwrap().1, "/api/echo?a=1&b=2");
    assert_eq!(json["cookie"], "eko_rip_session=abc", "seul le cookie du RIP passe");
    assert_eq!(json["authorization"], "", "Authorization ne sort jamais vers un atelier");
    assert_eq!(json["eko"], "", "en-tête réservé au relais retiré");
    assert_eq!(json["agent"], "EKO-RIP-Client/1.0");
    assert_eq!(cookies, vec![format!("eko_rip_session=jeton; HttpOnly; SameSite=Strict; Max-Age=60; Path=/s/{site}/; Secure")]);

    // Envoi en flux : sous la limite, transmis ; au-dessus, refusé.
    let (b, s) = (base.clone(), site.clone());
    let n: String = bloquant(move || agent().post(format!("{b}/s/{s}/api/taille")).send(vec![7u8; 600_000]).unwrap().body_mut().read_to_string().unwrap()).await;
    assert_eq!(n, "600000");
    let (b, s) = (base.clone(), site.clone());
    // Le relais répond 413 sans lire la suite : le client peut voir la réponse ou une coupure.
    let r = bloquant(move || agent().post(format!("{b}/s/{s}/api/taille")).send(vec![7u8; 2 * 1024 * 1024]).map(|r| r.status().as_u16())).await;
    assert!(matches!(r, Ok(413) | Err(ureq::Error::Io(_))), "{r:?}");

    // Requêtes en parallèle dans le même tunnel.
    let mut taches = vec![];
    for _ in 0..20 {
        let (b, s) = (base.clone(), site.clone());
        taches.push(tokio::task::spawn_blocking(move || agent().get(format!("{b}/s/{s}/api/echo")).call().unwrap().status().as_u16()));
    }
    for t in taches {
        assert_eq!(t.await.unwrap(), 200);
    }

    // Un autre site n'atteint pas celui-ci.
    let b = base.clone();
    assert_eq!(bloquant(move || agent().get(format!("{b}/s/autresite/api/echo")).call().unwrap().status().as_u16()).await, 502);

    // Fin du tunnel côté RIP : le site repasse hors ligne.
    tunnel.abort();
    tokio::time::sleep(Duration::from_millis(300)).await;
    let (b, s) = (base.clone(), site.clone());
    assert_eq!(bloquant(move || agent().get(format!("{b}/s/{s}/api/echo")).call().unwrap().status().as_u16()).await, 502);
    let _ = std::fs::remove_dir_all(&d);
}

#[tokio::test(flavor = "multi_thread")]
async fn ping_d_installation_et_statistiques() {
    let d = dossier();
    let base = relais_local(&d, 1024).await;
    let envoyer = |corps: serde_json::Value| {
        let b = base.clone();
        bloquant(move || agent().post(format!("{b}/api/ping")).send_json(corps).unwrap().status().as_u16())
    };
    let ping = serde_json::json!({ "id": "0123456789abcdef0123456789abcdef", "type": "serveur", "evenement": "installation", "version": "0.30.0", "os": "Ubuntu 26.04 LTS", "machines": 2, "reglages": 4 });
    assert_eq!(envoyer(ping.clone()).await, 204);
    assert_eq!(envoyer(serde_json::json!({ "id": "x", "type": "serveur", "evenement": "installation", "version": "1", "os": "x" })).await, 400);
    assert_eq!(envoyer(serde_json::json!({ "os": "x".repeat(5000) })).await, 413);

    let stats = |jeton: Option<&'static str>| {
        let b = base.clone();
        bloquant(move || {
            let mut r = agent().get(format!("{b}/api/installations"));
            if let Some(j) = jeton {
                r = r.header("Authorization", format!("Bearer {j}"));
            }
            let mut rep = r.call().unwrap();
            (rep.status().as_u16(), rep.body_mut().read_json::<serde_json::Value>().ok())
        })
    };
    assert_eq!(stats(None).await.0, 401);
    assert_eq!(stats(Some("mauvais")).await.0, 401);
    let (statut, corps) = stats(Some("jeton-stats")).await;
    assert_eq!(statut, 200);
    let corps = corps.unwrap();
    assert_eq!((corps["total"].as_u64(), corps["actives"].as_u64(), corps["machines"].as_u64()), (Some(1), Some(1), Some(2)));
    assert_eq!(corps["par_type"]["serveur"], 1);
    let _ = std::fs::remove_dir_all(&d);
}

/// Rapports : gardés sur la plate-forme ; issue GitHub (faux GitHub local) sans journal ni
/// e-mail ; même signature automatique = même issue.
#[tokio::test(flavor = "multi_thread")]
async fn rapports_et_issues_github() {
    use std::sync::{Arc, Mutex};
    let recus: Arc<Mutex<Vec<(String, serde_json::Value)>>> = Default::default();
    let r2 = recus.clone();
    let faux_github = Router::new().route(
        "/repos/{proprio}/{depot}/{*reste}",
        post(move |uri: axum::http::Uri, h: HeaderMap, axum::Json(v): axum::Json<serde_json::Value>| {
            let r2 = r2.clone();
            async move {
                assert_eq!(h.get(header::AUTHORIZATION).unwrap(), "Bearer jeton-gh");
                r2.lock().unwrap().push((uri.path().to_string(), v));
                (axum::http::StatusCode::CREATED, axum::Json(serde_json::json!({ "number": 7, "html_url": "https://github.test/o/d/issues/7" })))
            }
        }),
    );
    let ecoute = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let api = format!("http://{}", ecoute.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(ecoute, faux_github).await.unwrap() });
    // SAFETY : variable lue seulement par la plate-forme de ce test.
    unsafe { std::env::set_var("EKO_CLOUD_GITHUB_API", &api) };

    let d = dossier();
    {
        let b = eko_rip_cloud::base::ouvrir(&d).unwrap();
        eko_rip_cloud::base::regler(&b, "github_jeton", "jeton-gh").unwrap();
        eko_rip_cloud::base::regler(&b, "github_depot", "o/d").unwrap();
    }
    let base = relais_local(&d, 1024).await;
    let envoyer = |corps: serde_json::Value| {
        let b = base.clone();
        bloquant(move || {
            let mut r = agent().post(format!("{b}/api/rapports")).send_json(corps).unwrap();
            (r.status().as_u16(), r.body_mut().read_json::<serde_json::Value>().unwrap_or_default())
        })
    };
    let manuel = serde_json::json!({ "type": "client-linux", "version": "0.30.0", "os": "Ubuntu 26.04 LTS", "nature": "manuel", "titre": "Aperçu blanc", "description": "Ça ne marche pas @moi", "email": "a@b.fr", "journal": "ligne secrète" });
    let (statut, rep) = envoyer(manuel).await;
    assert_eq!(statut, 201);
    let reference = rep["reference"].as_str().unwrap().to_string();
    assert_eq!(envoyer(serde_json::json!({ "type": "x" })).await.0, 422);
    assert_eq!(envoyer(serde_json::json!({ "type": "serveur", "version": "1", "os": "x", "nature": "manuel", "titre": "t", "description": "" })).await.0, 400);
    let auto = serde_json::json!({ "id": "0123456789abcdef0123456789abcdef", "type": "serveur", "version": "0.30.0", "os": "Ubuntu", "nature": "erreur", "code": "rendu_echoue", "titre": "Rendu échoué", "journal": "x" });
    assert_eq!(envoyer(auto.clone()).await.0, 201);
    assert_eq!(envoyer(auto).await.0, 201);

    // L'issue part en arrière-plan.
    for _ in 0..50 {
        if recus.lock().unwrap().len() >= 2 && eko_rip_cloud::rapports::lire(&d, &reference).and_then(|r| r.issue).is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    tokio::time::sleep(Duration::from_millis(300)).await;
    let recus = recus.lock().unwrap().clone();
    // Deux issues (manuel, puis la signature automatique) ; la 2e occurrence auto, dans la
    // journée, n'ajoute pas de commentaire.
    assert_eq!(recus.iter().filter(|(p, _)| p == "/repos/o/d/issues").count(), 2, "{recus:?}");
    assert!(!recus.iter().any(|(p, _)| p.ends_with("/comments")));
    let corps = recus[0].1["body"].as_str().unwrap();
    assert!(!corps.contains("a@b.fr") && !corps.contains("ligne secrète") && !corps.contains("@moi"));
    let gardé = eko_rip_cloud::rapports::lire(&d, &reference).unwrap();
    assert_eq!((gardé.email.as_str(), gardé.journal.as_str(), gardé.issue.as_deref()), ("a@b.fr", "ligne secrète", Some("https://github.test/o/d/issues/7")));
    assert_eq!(eko_rip_cloud::rapports::lister(&d).len(), 3);
    let _ = std::fs::remove_dir_all(&d);
}
