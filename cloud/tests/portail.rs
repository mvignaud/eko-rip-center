//! Portail de bout en bout : inscription, rattachement d'un serveur par son code, droits,
//! administration (clés Google / Facebook), protections des formulaires.

use eko_rip_cloud::{base, client, relais};
use std::time::Duration;

const URL: &str = "http://plateforme.test";

struct Plateforme {
    base: String,
    donnees: std::path::PathBuf,
}

async fn plateforme() -> Plateforme {
    let donnees = std::env::temp_dir().join(format!("eko-cloud-portail-{}", eko_rip_cloud::aleatoire_hex(6)));
    let r = relais::nouveau(relais::Config { donnees: donnees.clone(), url_publique: URL.into(), max_octets: 1 << 20, jeton_stats: String::new() });
    let ecoute = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", ecoute.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(ecoute, relais::routes(r)).await.unwrap() });
    Plateforme { base, donnees }
}

type Boite = std::sync::Arc<std::sync::Mutex<Vec<eko_rip_cloud::courriels::Courriel>>>;

/// Plate-forme dont les e-mails restent en mémoire (pas de SMTP).
async fn plateforme_courriels() -> (Plateforme, Boite) {
    let donnees = std::env::temp_dir().join(format!("eko-cloud-portail-{}", eko_rip_cloud::aleatoire_hex(6)));
    let r = relais::nouveau(relais::Config { donnees: donnees.clone(), url_publique: URL.into(), max_octets: 1 << 20, jeton_stats: String::new() });
    let boite = r.utiliser_memoire();
    let ecoute = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", ecoute.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(ecoute, relais::routes(r)).await.unwrap() });
    (Plateforme { base, donnees }, boite)
}

/// Chemin (`/mot-de-passe/nouveau?jeton=…`) du premier lien de la version texte portant `chemin`.
fn lien_du_courriel(c: &eko_rip_cloud::courriels::Courriel, chemin: &str) -> String {
    let i = c.texte.find(&format!("{URL}{chemin}")).unwrap_or_else(|| panic!("lien {chemin} absent de {}", c.texte)) + URL.len();
    c.texte[i..].split_whitespace().next().unwrap().to_string()
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder().http_status_as_error(false).max_redirects(0).timeout_global(Some(Duration::from_secs(20))).build().into()
}

struct Rep {
    statut: u16,
    corps: String,
    cookies: Vec<String>,
    lieu: String,
}

fn lire(r: Result<ureq::http::Response<ureq::Body>, ureq::Error>) -> Rep {
    let mut r = r.unwrap();
    let cookies = r.headers().get_all("set-cookie").iter().map(|v| v.to_str().unwrap().to_string()).collect();
    let lieu = r.headers().get("location").map(|v| v.to_str().unwrap().to_string()).unwrap_or_default();
    Rep { statut: r.status().as_u16(), corps: r.body_mut().read_to_string().unwrap_or_default(), cookies, lieu }
}

fn get(p: &Plateforme, chemin: &str, cookie: Option<&str>) -> Rep {
    let mut q = agent().get(format!("{}{chemin}", p.base));
    if let Some(c) = cookie {
        q = q.header("Cookie", c);
    }
    lire(q.call())
}

fn post(p: &Plateforme, chemin: &str, cookie: Option<&str>, champs: &[(&str, &str)], origine: bool) -> Rep {
    let mut q = agent().post(format!("{}{chemin}", p.base));
    if let Some(c) = cookie {
        q = q.header("Cookie", c);
    }
    if origine {
        q = q.header("Origin", URL);
    }
    lire(q.send_form(champs.iter().copied()))
}

fn session(r: &Rep) -> String {
    r.cookies.iter().find(|c| c.starts_with("plateforme_session=")).expect("cookie de session").split(';').next().unwrap().to_string()
}

fn csrf(page: &str) -> String {
    let i = page.find("name=\"csrf\" value=\"").expect("jeton csrf") + 19;
    page[i..i + 32].to_string()
}

async fn b<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    tokio::task::spawn_blocking(f).await.unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn inscription_rattachement_administration() {
    let p = std::sync::Arc::new(plateforme().await);

    // Pages publiques et légales.
    let q = p.clone();
    let (accueil, cgu, mentions) = b(move || (get(&q, "/", None), get(&q, "/cgu", None), get(&q, "/mentions-legales", None))).await;
    assert_eq!(accueil.statut, 200);
    assert!(accueil.corps.contains("/inscription"));
    assert!(cgu.statut == 200 && cgu.corps.contains("Conditions générales"));
    assert!(mentions.corps.contains("GOLDEN PRINT"));

    // Formulaire venu d'ailleurs : refusé.
    let q = p.clone();
    let r = b(move || post(&q, "/inscription", None, &[("email", "a@b.fr"), ("nom", "A"), ("mot_de_passe", "phrase-longue-1"), ("cgu", "1")], false)).await;
    assert_eq!(r.statut, 403);
    // CGU non acceptées, mot de passe court : refusés.
    let q = p.clone();
    let r = b(move || post(&q, "/inscription", None, &[("email", "chef@atelier.fr"), ("nom", "Atelier"), ("mot_de_passe", "phrase-longue-1")], true)).await;
    assert_eq!(r.statut, 400);
    let q = p.clone();
    let r = b(move || post(&q, "/inscription", None, &[("email", "chef@atelier.fr"), ("nom", "Atelier"), ("mot_de_passe", "court"), ("cgu", "1")], true)).await;
    assert_eq!(r.statut, 400);

    // Inscription : session ouverte, cookie limité au portail.
    let q = p.clone();
    let r = b(move || post(&q, "/inscription", None, &[("email", "chef@atelier.fr"), ("nom", "Atelier <Dupont>"), ("mot_de_passe", "phrase-longue-1"), ("cgu", "1")], true)).await;
    assert_eq!((r.statut, r.lieu.as_str()), (303, "/sites"));
    assert!(r.cookies[0].contains("HttpOnly") && r.cookies[0].contains("SameSite=Lax"));
    let chef = session(&r);
    let q = p.clone();
    let c = chef.clone();
    let sites = b(move || get(&q, "/sites", Some(&c))).await;
    assert!(sites.corps.contains("Aucun site"));
    assert!(!sites.corps.contains("<Dupont>"), "noms échappés");
    assert_eq!(sites.cookies.len(), 0);
    let jeton = csrf(&sites.corps);

    // Le serveur RIP demande un code ; le compte le valide ; le serveur reçoit sa clé.
    let q = p.clone();
    let demande: serde_json::Value = b(move || agent().post(format!("{}/api/appairage", q.base)).send_json(serde_json::json!({ "nom": "RIP atelier" })).unwrap().body_mut().read_json().unwrap()).await;
    let code = demande["code"].as_str().unwrap().to_string();
    let q = p.clone();
    let (c, j) = (chef.clone(), jeton.clone());
    let c1 = c.clone();
    let r = b(move || post(&q, "/sites/rattacher", Some(&c1), &[("csrf", "faux"), ("code", "ABCD-1234")], true)).await;
    assert_eq!(r.statut, 403, "jeton anti-falsification exigé");
    let q = p.clone();
    let c2 = c.clone();
    let r = b(move || post(&q, "/sites/rattacher", Some(&c2), &[("csrf", &j), ("code", "ABCD-1234")], true)).await;
    assert!(r.statut == 400 && r.corps.contains("inconnu"), "code qui n'existe pas");
    let q = p.clone();
    let (c3, j3, code3) = (c.clone(), jeton.clone(), code.to_lowercase());
    let r = b(move || post(&q, "/sites/rattacher", Some(&c3), &[("csrf", &j3), ("code", &code3)], true)).await;
    assert_eq!((r.statut, r.lieu.as_str()), (303, "/sites?ok=rattache"));
    let q = p.clone();
    let d = demande["demande"].as_str().unwrap().to_string();
    let valide: serde_json::Value = b(move || agent().get(format!("{}/api/appairage/{d}", q.base)).call().unwrap().body_mut().read_json().unwrap()).await;
    assert_eq!(valide["etat"], "valide");
    let site = valide["site"].as_str().unwrap().to_string();
    let cle = valide["cle"].as_str().unwrap().to_string();

    // Le site apparaît, en ligne une fois le tunnel ouvert.
    let (tx, rx) = tokio::sync::oneshot::channel();
    let (bb, s2, k2) = (p.base.clone(), site.clone(), cle.clone());
    let tunnel = tokio::spawn(async move { client::servir(&bb, &s2, &k2, "essai", axum::Router::new(), move || tx.send(()).unwrap()).await });
    rx.await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    let q = p.clone();
    let c4 = chef.clone();
    let sites = b(move || get(&q, "/sites", Some(&c4))).await;
    assert!(sites.corps.contains("RIP atelier") && sites.corps.contains("En ligne") && sites.corps.contains(&relais::adresse_du_site(URL, &site)), "{}", sites.corps);
    assert!(sites.corps.contains("Propriétaire"));

    // Un autre compte ne peut ni voir ni retirer ce site, ni ouvrir l'administration.
    let q = p.clone();
    let r = b(move || post(&q, "/inscription", None, &[("email", "autre@atelier.fr"), ("nom", "Autre"), ("mot_de_passe", "phrase-longue-2"), ("cgu", "1")], true)).await;
    let autre = session(&r);
    let q = p.clone();
    let a = autre.clone();
    let (ses_sites, adm) = b(move || (get(&q, "/sites", Some(&a)), get(&q, "/admin", Some(&a)))).await;
    assert!(!ses_sites.corps.contains("RIP atelier"));
    assert_eq!(adm.statut, 403);
    let q = p.clone();
    let (a2, j2, s3) = (autre.clone(), csrf(&ses_sites.corps), site.clone());
    let r = b(move || post(&q, &format!("/sites/{s3}/retirer"), Some(&a2), &[("csrf", &j2)], true)).await;
    assert_eq!(r.statut, 403);

    // Super-admin : clés Google / Facebook, URL de retour affichées.
    {
        let c = base::ouvrir(&p.donnees).unwrap();
        assert!(base::nommer_super_admin(&c, "chef@atelier.fr").unwrap());
    }
    let q = p.clone();
    let c5 = chef.clone();
    let adm = b(move || get(&q, "/admin", Some(&c5))).await;
    assert_eq!(adm.statut, 200);
    assert!(adm.corps.contains(&format!("{URL}/auth/google/retour")) && adm.corps.contains(&format!("{URL}/auth/facebook/retour")));
    assert!(adm.corps.contains("autre@atelier.fr") && adm.corps.contains(&site));
    let q = p.clone();
    let (c6, j6) = (chef.clone(), csrf(&adm.corps));
    let r = b(move || post(&q, "/admin/oauth", Some(&c6), &[("csrf", &j6), ("google_client_id", "id-google.apps"), ("google_secret", "secret-google"), ("facebook_app_id", ""), ("facebook_secret", "")], true)).await;
    assert_eq!(r.lieu, "/admin?ok=oauth");
    let q = p.clone();
    let c7 = chef.clone();
    let adm = b(move || get(&q, "/admin", Some(&c7))).await;
    assert!(adm.corps.contains("id-google.apps") && !adm.corps.contains("secret-google"), "le secret n'est jamais réaffiché");
    let q = p.clone();
    let (connexion, depart) = b(move || (get(&q, "/connexion", None), get(&q, "/auth/google", None))).await;
    assert!(connexion.corps.contains("/auth/google") && !connexion.corps.contains("/auth/facebook"), "seul un fournisseur configuré s'affiche");
    assert!(depart.lieu.starts_with("https://accounts.google.com/o/oauth2/v2/auth?client_id=id-google.apps&redirect_uri=http%3A%2F%2Fplateforme.test%2Fauth%2Fgoogle%2Fretour"), "{}", depart.lieu);
    assert!(depart.cookies[0].starts_with("plateforme_oauth=google:"));
    let q = p.clone();
    let r = b(move || get(&q, "/auth/google/retour?code=x&state=invente", Some("plateforme_oauth=google:autre"))).await;
    assert!(r.statut == 400 && r.corps.contains("interrompue"), "état OAuth vérifié");

    // Suspendre « autre » ferme sa session.
    let id_autre = {
        let c = base::ouvrir(&p.donnees).unwrap();
        base::compte_par_email(&c, "autre@atelier.fr").unwrap().unwrap().id
    };
    let q = p.clone();
    let (c8, j8) = (chef.clone(), csrf(&adm.corps));
    b(move || post(&q, &format!("/admin/comptes/{id_autre}/suspendre"), Some(&c8), &[("csrf", &j8), ("suspendu", "1")], true)).await;
    let q = p.clone();
    let a3 = autre.clone();
    assert_eq!(b(move || get(&q, "/sites", Some(&a3))).await.statut, 303);

    // Le propriétaire retire son site : tunnel coupé aussitôt.
    let q = p.clone();
    let (c9, j9, s9) = (chef.clone(), jeton.clone(), site.clone());
    let r = b(move || post(&q, &format!("/sites/{s9}/retirer"), Some(&c9), &[("csrf", &j9)], true)).await;
    assert_eq!(r.statut, 303);
    let fin = tokio::time::timeout(Duration::from_secs(5), tunnel).await.expect("tunnel coupé").unwrap();
    assert!(fin.is_ok() || fin.is_err());
    let q = p.clone();
    let s10 = site.clone();
    assert_eq!(b(move || get(&q, &format!("/s/{s10}/api/x"), None)).await.statut, 503);
    let _ = std::fs::remove_dir_all(&p.donnees);
}

/// Faux serveur RIP : renvoie l'identité et l'autorisation reçues.
fn rip_echo() -> axum::Router {
    axum::Router::new().route(
        "/api/echo",
        axum::routing::get(|h: axum::http::HeaderMap| async move {
            let v = |n: &str| h.get(n).and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
            axum::Json(serde_json::json!({ "identite": v("x-eko-cloud-identite"), "authorization": v("authorization") }))
        }),
    )
}

fn parametre(texte: &str, nom: &str) -> String {
    let i = texte.find(&format!("{nom}=")).unwrap_or_else(|| panic!("{nom} absent de {texte}")) + nom.len() + 1;
    texte[i..].split(['&', '"', ';']).next().unwrap().to_string()
}

#[tokio::test(flavor = "multi_thread")]
async fn connexion_des_apps() {
    let (p, boite) = plateforme_courriels().await;
    let p = std::sync::Arc::new(p);
    let inscrire = |email: &'static str| {
        let q = p.clone();
        async move { session(&b(move || post(&q, "/inscription", None, &[("email", email), ("nom", "Chef d'atelier"), ("mot_de_passe", "phrase-longue-1"), ("cgu", "1")], true)).await) }
    };
    let chef = inscrire("chef@atelier.fr").await;
    let autre = inscrire("autre@atelier.fr").await;
    {
        // Adresses confirmées (sinon, avec des e-mails actifs, rattacher un serveur est refusé).
        let c = base::ouvrir(&p.donnees).unwrap();
        for e in ["chef@atelier.fr", "autre@atelier.fr"] {
            base::marquer_email_verifie(&c, base::compte_par_email(&c, e).unwrap().unwrap().id).unwrap();
        }
    }

    // Un site rattaché au chef, tunnel ouvert.
    let q = p.clone();
    let demande: serde_json::Value = b(move || agent().post(format!("{}/api/appairage", q.base)).send_json(serde_json::json!({ "nom": "RIP atelier" })).unwrap().body_mut().read_json().unwrap()).await;
    let q = p.clone();
    let (c, code) = (chef.clone(), demande["code"].as_str().unwrap().to_string());
    b(move || {
        let j = csrf(&get(&q, "/sites", Some(&c)).corps);
        assert_eq!(post(&q, "/sites/rattacher", Some(&c), &[("csrf", &j), ("code", &code)], true).statut, 303);
    })
    .await;
    let q = p.clone();
    let d = demande["demande"].as_str().unwrap().to_string();
    let valide: serde_json::Value = b(move || agent().get(format!("{}/api/appairage/{d}", q.base)).call().unwrap().body_mut().read_json().unwrap()).await;
    let site = valide["site"].as_str().unwrap().to_string();
    let (tx, rx) = tokio::sync::oneshot::channel();
    let (bb, s2, k2) = (p.base.clone(), site.clone(), valide["cle"].as_str().unwrap().to_string());
    tokio::spawn(async move { client::servir(&bb, &s2, &k2, "essai", rip_echo(), move || tx.send(()).unwrap()).await });
    rx.await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // PKCE (RFC 7636, annexe B).
    let verificateur = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    let defi = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
    let retour = "http://127.0.0.1:53124/retour";
    let demande_app = format!("/oauth/autoriser?client_id=eko-rip-center&response_type=code&redirect_uri={}&code_challenge={defi}&code_challenge_method=S256&state=etat-123&appareil=Mac%20de%20test", eko_rip_cloud::portail::encoder(retour));

    // Sans session : connexion d'abord, puis retour à la demande.
    let q = p.clone();
    let da = demande_app.clone();
    let r = b(move || get(&q, &da, None)).await;
    assert_eq!(r.statut, 303);
    assert!(r.lieu.starts_with("/connexion?suite=%2Foauth%2Fautoriser%3F"), "{}", r.lieu);
    // Adresse de retour hors de la machine : refusée.
    let q = p.clone();
    let (c, mauvais) = (chef.clone(), demande_app.replace("127.0.0.1%3A53124", "evil.test"));
    assert_eq!(b(move || get(&q, &mauvais, Some(&c))).await.statut, 400);

    // Consentement puis code, deux fois (le premier sera grillé par un mauvais vérificateur).
    let obtenir_code = || {
        let (q, c, da) = (p.clone(), chef.clone(), demande_app.clone());
        async move {
            b(move || {
                let page = get(&q, &da, Some(&c));
                assert!(page.statut == 200 && page.corps.contains("Mac de test") && page.corps.contains("chef@atelier.fr"), "{}", page.corps);
                let j = csrf(&page.corps);
                let champs = [("csrf", j.as_str()), ("client_id", "eko-rip-center"), ("response_type", "code"), ("code_challenge_method", "S256"), ("redirect_uri", retour), ("code_challenge", defi), ("state", "etat-123"), ("appareil", "Mac de test"), ("decision", "oui")];
                assert_eq!(post(&q, "/oauth/autoriser", Some(&c), &champs, false).statut, 403, "origine exigée");
                let r = post(&q, "/oauth/autoriser", Some(&c), &champs, true);
                assert!(r.corps.contains(&format!("url={retour}?code=")), "{}", r.corps);
                assert_eq!(parametre(&r.corps, "state"), "etat-123");
                parametre(&r.corps, "code")
            })
            .await
        }
    };
    let echanger = |code: String, verif: &'static str| {
        let q = p.clone();
        b(move || {
            let mut r = agent().post(format!("{}/oauth/jeton", q.base)).send_form([("grant_type", "authorization_code"), ("client_id", "eko-rip-center"), ("code", code.as_str()), ("redirect_uri", retour), ("code_verifier", verif)]).unwrap();
            (r.status().as_u16(), r.body_mut().read_json::<serde_json::Value>().unwrap())
        })
    };
    let code1 = obtenir_code().await;
    let (statut, rep) = echanger(code1.clone(), "mauvais-verificateur-mauvais-verificateur-mauvais").await;
    assert_eq!((statut, rep["error"].as_str()), (400, Some("invalid_grant")));
    let (statut, _) = echanger(code1, verificateur).await;
    assert_eq!(statut, 400, "un code ne sert qu'une fois");
    let (statut, rep) = echanger(obtenir_code().await, verificateur).await;
    assert_eq!(statut, 200, "{rep}");
    assert_eq!(rep["token_type"], "Bearer");
    let jeton = rep["access_token"].as_str().unwrap().to_string();
    assert!(jeton.starts_with("ekoa_"));

    // Mes sites, vus par l'app.
    let appel = |chemin: String, hote: Option<String>, jeton: String| {
        let q = p.clone();
        b(move || {
            let mut r = agent().get(format!("{}{chemin}", q.base)).header("Authorization", &format!("Bearer {jeton}"));
            if let Some(h) = hote {
                r = r.header("Host", &h);
            }
            let mut r = r.call().unwrap();
            (r.status().as_u16(), r.body_mut().read_json::<serde_json::Value>().unwrap_or_default())
        })
    };
    let (statut, moi) = appel("/api/moi".into(), None, jeton.clone()).await;
    assert_eq!(statut, 200);
    assert_eq!(moi["compte"]["email"], "chef@atelier.fr");
    assert_eq!(moi["sites"][0]["id"], site.as_str());
    assert_eq!(moi["sites"][0]["en_ligne"], true);
    assert_eq!(moi["sites"][0]["role"], "proprietaire");
    assert_eq!(moi["sites"][0]["adresse"], relais::adresse_du_site(URL, &site));

    // Le site, ouvert avec le jeton : identité transmise, jeton retenu.
    let hote = format!("{site}.plateforme.test");
    let (statut, echo) = appel("/api/echo".into(), Some(hote.clone()), jeton.clone()).await;
    assert_eq!(statut, 200);
    assert_eq!(echo["authorization"], "", "le jeton ne sort jamais vers un atelier");
    let identite = eko_rip_cloud::applis::lire_identite(echo["identite"].as_str().unwrap()).unwrap();
    assert_eq!((identite.email.as_str(), identite.role.as_str()), ("chef@atelier.fr", "admin"));
    let (statut, _) = appel(format!("/s/{site}/api/echo"), None, jeton.clone()).await;
    assert_eq!(statut, 200, "aussi sous /s/<site>");

    // Jeton inconnu : 401 ; compte sans droit sur ce site : même réponse qu'un site absent.
    let (statut, rep) = appel("/api/echo".into(), Some(hote.clone()), "ekoa_invente".into()).await;
    assert_eq!((statut, rep["code"].as_str()), (401, Some("jeton_invalide")));
    let jeton_autre = {
        let c = base::ouvrir(&p.donnees).unwrap();
        let id = base::compte_par_email(&c, "autre@atelier.fr").unwrap().unwrap().id;
        base::creer_jeton_appli(&c, id, "Autre").unwrap()
    };
    let (statut, rep) = appel("/api/echo".into(), Some(hote.clone()), jeton_autre).await;
    assert_eq!((statut, rep["code"].as_str()), (503, Some("site_hors_ligne")));
    let _ = autre;

    // Mon compte : l'app apparaît ; « Déconnecter toutes les apps » révoque le jeton.
    let q = p.clone();
    let c = chef.clone();
    b(move || {
        let compte = get(&q, "/compte", Some(&c));
        assert!(compte.corps.contains("Mac de test"), "{}", compte.corps);
        let j = csrf(&compte.corps);
        assert_eq!(post(&q, "/compte/applis/deconnecter", Some(&c), &[("csrf", &j)], true).lieu, "/compte?ok=applis");
    })
    .await;
    let (statut, _) = appel("/api/moi".into(), None, jeton).await;
    assert_eq!(statut, 401);
    // Une app connectée : e-mail de sécurité au titulaire (appareil, lien vers /compte).
    {
        let boite = boite.lock().unwrap();
        let m = boite.iter().find(|m| m.sujet.contains("Nouvelle app connectée")).expect("e-mail de nouvelle app");
        assert_eq!(m.a, "chef@atelier.fr");
        assert!(m.html.contains("Mac de test") && m.texte.contains(&format!("{URL}/compte")));
    }
    let _ = std::fs::remove_dir_all(&p.donnees);
}

#[tokio::test(flavor = "multi_thread")]
async fn mot_de_passe_oublie_et_confirmation_de_l_adresse() {
    let (p, boite) = plateforme_courriels().await;
    let p = std::sync::Arc::new(p);
    let q = p.clone();
    let (oubli, formulaire) = b(move || (get(&q, "/mot-de-passe/oublie", None), get(&q, "/connexion", None))).await;
    assert_eq!(oubli.statut, 200);
    assert!(formulaire.corps.contains("/mot-de-passe/oublie") && formulaire.corps.contains("Mot de passe oublié ?"));

    // Inscription : e-mail de confirmation, bandeau, rattachement refusé.
    let q = p.clone();
    let r = b(move || post(&q, "/inscription", None, &[("email", "chef@atelier.fr"), ("nom", "Chef <b>"), ("mot_de_passe", "phrase-longue-1"), ("cgu", "1")], true)).await;
    let chef = session(&r);
    let confirmation = {
        let m = boite.lock().unwrap();
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].a, "chef@atelier.fr");
        assert!(m[0].html.contains("Chef &lt;b&gt;") && !m[0].html.contains("Chef <b>"));
        lien_du_courriel(&m[0], "/email/confirmer?jeton=")
    };
    let q = p.clone();
    let c = chef.clone();
    let sites = b(move || get(&q, "/sites", Some(&c))).await;
    assert!(sites.corps.contains("Confirmez votre adresse") && sites.corps.contains("Renvoyer l&#39;e-mail") || sites.corps.contains("Renvoyer l'e-mail"));
    let jeton = csrf(&sites.corps);
    let q = p.clone();
    let c = chef.clone();
    let compte = b(move || get(&q, "/compte", Some(&c))).await;
    assert!(compte.corps.contains("Confirmez votre adresse") && compte.corps.contains("Recevoir le résumé hebdomadaire") && compte.corps.contains("name=\"alertes\" value=\"1\" checked"));
    let q = p.clone();
    let (c, j) = (chef.clone(), jeton.clone());
    let r = b(move || post(&q, "/sites/rattacher", Some(&c), &[("csrf", &j), ("code", "ABCD-1234")], true)).await;
    assert!(r.statut == 400 && r.corps.contains("Confirmez d&#39;abord votre adresse"), "{}", r.corps);

    // Renvoyer l'e-mail : limité à 3 par heure.
    for n in 0..3 {
        let q = p.clone();
        let (c, j) = (chef.clone(), jeton.clone());
        let r = b(move || post(&q, "/email/renvoyer", Some(&c), &[("csrf", &j), ("retour", "/compte")], true)).await;
        assert_eq!((r.statut, r.lieu.as_str()), (if n < 3 { 303 } else { 400 }, "/compte?ok=confirmation"), "essai {n}");
    }
    let q = p.clone();
    let (c, j) = (chef.clone(), jeton.clone());
    assert_eq!(b(move || post(&q, "/email/renvoyer", Some(&c), &[("csrf", &j)], true)).await.statut, 400, "limite atteinte");
    assert_eq!(boite.lock().unwrap().len(), 4);

    // Le lien : l'ouvrir ne consomme rien, le bouton oui ; un seul usage. Le premier lien a été remplacé par les suivants.
    let dernier = lien_du_courriel(boite.lock().unwrap().last().unwrap(), "/email/confirmer?jeton=");
    let q = p.clone();
    let (ancien, d1) = (confirmation.clone(), dernier.clone());
    let (vieux, ouvert, encore) = b(move || (get(&q, &ancien, None), get(&q, &d1, None), get(&q, &d1, None))).await;
    assert_eq!(vieux.statut, 400, "un nouveau lien remplace l'ancien");
    assert!(ouvert.statut == 200 && encore.statut == 200 && ouvert.corps.contains("Confirmer mon adresse"));
    let jeton_lien = dernier.split("jeton=").nth(1).unwrap().to_string();
    let q = p.clone();
    let jl = jeton_lien.clone();
    let r = b(move || post(&q, "/email/confirmer", None, &[("jeton", &jl)], false)).await;
    assert_eq!(r.statut, 403, "origine exigée");
    let (q, jl) = (p.clone(), jeton_lien.clone());
    let r = b(move || post(&q, "/email/confirmer", None, &[("jeton", &jl)], true)).await;
    assert!(r.statut == 200 && r.corps.contains("Adresse confirmée"), "{}", r.corps);
    let (q, jl) = (p.clone(), jeton_lien.clone());
    assert_eq!(b(move || post(&q, "/email/confirmer", None, &[("jeton", &jl)], true)).await.statut, 400, "usage unique");
    let q = p.clone();
    let (c, j) = (chef.clone(), jeton.clone());
    let r = b(move || post(&q, "/sites/rattacher", Some(&c), &[("csrf", &j), ("code", "ABCD-1234")], true)).await;
    assert!(r.corps.contains("inconnu") && !r.corps.contains("Confirmez d&#39;abord"), "adresse confirmée : le rattachement suit son cours");
    let q = p.clone();
    let c = chef.clone();
    assert!(!b(move || get(&q, "/sites", Some(&c))).await.corps.contains("Confirmez votre adresse"));

    // Mot de passe oublié : réponse identique (compte connu, inconnu, e-mail invalide), un seul e-mail.
    boite.lock().unwrap().clear();
    let q = p.clone();
    let (connu, inconnu, invalide) = b(move || {
        (
            post(&q, "/mot-de-passe/oublie", None, &[("email", "CHEF@atelier.fr")], true),
            post(&q, "/mot-de-passe/oublie", None, &[("email", "personne@atelier.fr")], true),
            post(&q, "/mot-de-passe/oublie", None, &[("email", "pas un e-mail")], true),
        )
    })
    .await;
    assert_eq!((connu.statut, inconnu.statut, invalide.statut), (200, 200, 200));
    assert_eq!(connu.corps, inconnu.corps, "pas d'énumération des comptes");
    assert_eq!(connu.corps, invalide.corps);
    let q = p.clone();
    assert_eq!(b(move || post(&q, "/mot-de-passe/oublie", None, &[("email", "chef@atelier.fr")], false)).await.statut, 403);
    let lien = {
        let m = boite.lock().unwrap();
        assert_eq!(m.len(), 1, "un seul e-mail, pour le compte existant");
        assert_eq!(m[0].a, "chef@atelier.fr");
        assert!(m[0].sujet.contains("mot de passe"));
        lien_du_courriel(&m[0], "/mot-de-passe/nouveau?jeton=")
    };
    let jeton_reinit = lien.split("jeton=").nth(1).unwrap().to_string();
    {
        let c = base::ouvrir(&p.donnees).unwrap();
        let n: i64 = c.query_row("SELECT COUNT(*) FROM jetons_courriel WHERE empreinte = ?1", [eko_rip_cloud::sha256_hex(&jeton_reinit)], |r| r.get(0)).unwrap();
        assert_eq!(n, 1, "seule l'empreinte est en base");
        let n: i64 = c.query_row("SELECT COUNT(*) FROM jetons_courriel WHERE empreinte = ?1", [&jeton_reinit], |r| r.get(0)).unwrap();
        assert_eq!(n, 0);
    }

    // Réinitialisation : formulaire, refus d'un mot de passe court ou différent, puis succès.
    let q = p.clone();
    let l = lien.clone();
    let form = b(move || get(&q, &l, None)).await;
    assert!(form.statut == 200 && form.corps.contains("Nouveau mot de passe"));
    let q = p.clone();
    assert_eq!(b(move || get(&q, "/mot-de-passe/nouveau?jeton=invente", None)).await.statut, 400);
    let champs = |m: &'static str, conf: &'static str| (jeton_reinit.clone(), m, conf);
    for (m, conf) in [("court", "court"), ("nouvelle-phrase-2", "autre-phrase-22")] {
        let q = p.clone();
        let (j, m, conf) = champs(m, conf);
        let r = b(move || post(&q, "/mot-de-passe/nouveau", None, &[("jeton", &j), ("mot_de_passe", m), ("confirmation", conf)], true)).await;
        assert_eq!(r.statut, 400);
    }
    let (q, j) = (p.clone(), jeton_reinit.clone());
    let r = b(move || post(&q, "/mot-de-passe/nouveau", None, &[("jeton", &j), ("mot_de_passe", "nouvelle-phrase-2"), ("confirmation", "nouvelle-phrase-2")], false)).await;
    assert_eq!(r.statut, 403, "origine exigée");
    let (q, j) = (p.clone(), jeton_reinit.clone());
    let r = b(move || post(&q, "/mot-de-passe/nouveau", None, &[("jeton", &j), ("mot_de_passe", "nouvelle-phrase-2"), ("confirmation", "nouvelle-phrase-2")], true)).await;
    assert_eq!((r.statut, r.lieu.as_str()), (303, "/connexion?ok=reinit"));
    let (q, j) = (p.clone(), jeton_reinit.clone());
    let r = b(move || post(&q, "/mot-de-passe/nouveau", None, &[("jeton", &j), ("mot_de_passe", "troisieme-phrase-3"), ("confirmation", "troisieme-phrase-3")], true)).await;
    assert_eq!(r.statut, 400, "usage unique");
    // Les sessions ouvertes sont fermées ; seul le nouveau mot de passe ouvre.
    let q = p.clone();
    let c = chef.clone();
    assert_eq!(b(move || get(&q, "/sites", Some(&c))).await.statut, 303, "session fermée");
    let q = p.clone();
    let (vieux, neuf) = b(move || (post(&q, "/connexion", None, &[("email", "chef@atelier.fr"), ("mot_de_passe", "phrase-longue-1")], true), post(&q, "/connexion", None, &[("email", "chef@atelier.fr"), ("mot_de_passe", "nouvelle-phrase-2")], true))).await;
    assert_eq!(vieux.statut, 401);
    assert_eq!(neuf.statut, 303);

    // Jeton posé en base : valable, puis expiré.
    let (id, posé) = {
        let c = base::ouvrir(&p.donnees).unwrap();
        let id = base::compte_par_email(&c, "chef@atelier.fr").unwrap().unwrap().id;
        (id, base::creer_jeton_courriel(&c, id, base::OBJET_REINIT, base::DUREE_JETON_REINIT).unwrap())
    };
    let (q, j) = (p.clone(), posé.clone());
    assert_eq!(b(move || get(&q, &format!("/mot-de-passe/nouveau?jeton={j}"), None)).await.statut, 200);
    {
        let c = base::ouvrir(&p.donnees).unwrap();
        c.execute("UPDATE jetons_courriel SET expire = 1 WHERE compte = ?1", [id]).unwrap();
    }
    let (q, j) = (p.clone(), posé);
    let r = b(move || post(&q, "/mot-de-passe/nouveau", None, &[("jeton", &j), ("mot_de_passe", "quatrieme-phrase-4"), ("confirmation", "quatrieme-phrase-4")], true)).await;
    assert_eq!(r.statut, 400, "jeton expiré");

    // Compte suspendu : aucun e-mail.
    {
        let c = base::ouvrir(&p.donnees).unwrap();
        base::suspendre(&c, id, true).unwrap();
    }
    boite.lock().unwrap().clear();
    let q = p.clone();
    let r = b(move || post(&q, "/mot-de-passe/oublie", None, &[("email", "chef@atelier.fr")], true)).await;
    assert_eq!(r.statut, 200);
    assert!(boite.lock().unwrap().is_empty());
    let _ = std::fs::remove_dir_all(&p.donnees);
}

#[tokio::test(flavor = "multi_thread")]
async fn preferences_et_reglages_des_courriels() {
    let (p, boite) = plateforme_courriels().await;
    let p = std::sync::Arc::new(p);
    let q = p.clone();
    let r = b(move || post(&q, "/inscription", None, &[("email", "admin@atelier.fr"), ("nom", "Admin"), ("mot_de_passe", "phrase-longue-1"), ("cgu", "1")], true)).await;
    let admin = session(&r);
    let q = p.clone();
    let c = admin.clone();
    let compte = b(move || get(&q, "/compte", Some(&c))).await;
    let j = csrf(&compte.corps);
    // Cases décochées : enregistrées.
    let (q, c, j2) = (p.clone(), admin.clone(), j.clone());
    let r = b(move || post(&q, "/compte/preferences", Some(&c), &[("csrf", &j2)], true)).await;
    assert_eq!(r.lieu, "/compte?ok=preferences");
    {
        let c = base::ouvrir(&p.donnees).unwrap();
        let cp = base::compte_par_email(&c, "admin@atelier.fr").unwrap().unwrap();
        assert!(!cp.alertes && !cp.resume);
        base::nommer_super_admin(&c, "admin@atelier.fr").unwrap();
    }
    let (q, c, j2) = (p.clone(), admin.clone(), j.clone());
    let r = b(move || post(&q, "/compte/preferences", Some(&c), &[("csrf", &j2), ("alertes", "1"), ("resume", "1")], true)).await;
    assert_eq!(r.lieu, "/compte?ok=preferences");
    // Administration : carte E-mails, secret jamais réaffiché, e-mail d'essai.
    let q = p.clone();
    let c = admin.clone();
    let adm = b(move || get(&q, "/admin", Some(&c))).await;
    assert!(adm.corps.contains("Envoyer un e-mail d'essai") && adm.corps.contains("smtp_mot_de_passe") && adm.corps.contains("no-reply@e-ko.fr"));
    let (q, c, j2) = (p.clone(), admin.clone(), csrf(&adm.corps));
    let r = b(move || post(&q, "/admin/courriels", Some(&c), &[("csrf", &j2), ("smtp_hote", "smtp.exemple.fr"), ("smtp_port", "465"), ("smtp_securite", "tls"), ("smtp_identifiant", "robot"), ("smtp_mot_de_passe", "secret-smtp-de-test"), ("smtp_expediteur", "")], true)).await;
    assert_eq!(r.lieu, "/admin?ok=courriels");
    let (q, c, j2) = (p.clone(), admin.clone(), csrf(&adm.corps));
    let r = b(move || post(&q, "/admin/courriels", Some(&c), &[("csrf", &j2), ("smtp_hote", "smtp exemple"), ("smtp_port", "465")], true)).await;
    assert_eq!(r.statut, 400, "hôte invalide");
    let q = p.clone();
    let c = admin.clone();
    let adm = b(move || get(&q, "/admin", Some(&c))).await;
    assert!(adm.corps.contains("smtp.exemple.fr") && adm.corps.contains("robot") && !adm.corps.contains("secret-smtp-de-test"), "secret jamais réaffiché");
    {
        let c = base::ouvrir(&p.donnees).unwrap();
        assert_eq!(base::reglage(&c, "smtp_mot_de_passe").unwrap().as_deref(), Some("secret-smtp-de-test"));
    }
    // Mot de passe vide : inchangé.
    let (q, c, j2) = (p.clone(), admin.clone(), csrf(&adm.corps));
    b(move || post(&q, "/admin/courriels", Some(&c), &[("csrf", &j2), ("smtp_hote", "smtp.exemple.fr"), ("smtp_mot_de_passe", "")], true)).await;
    {
        let c = base::ouvrir(&p.donnees).unwrap();
        assert!(base::reglage(&c, "smtp_mot_de_passe").unwrap().is_some());
    }
    boite.lock().unwrap().clear();
    let (q, c, j2) = (p.clone(), admin.clone(), csrf(&adm.corps));
    let r = b(move || post(&q, "/admin/courriels/essai", Some(&c), &[("csrf", &j2)], true)).await;
    assert_eq!(r.lieu, "/admin?ok=essai");
    {
        let m = boite.lock().unwrap();
        assert_eq!((m.len(), m[0].a.as_str()), (1, "admin@atelier.fr"));
        assert!(m[0].sujet.contains("essai"));
    }
    // Réservé aux administrateurs.
    let q = p.clone();
    let r = b(move || post(&q, "/inscription", None, &[("email", "simple@atelier.fr"), ("nom", "Simple"), ("mot_de_passe", "phrase-longue-1"), ("cgu", "1")], true)).await;
    let simple = session(&r);
    let (q, c, j2) = (p.clone(), simple, j);
    assert_eq!(b(move || post(&q, "/admin/courriels/essai", Some(&c), &[("csrf", &j2)], true)).await.statut, 403);
    let _ = std::fs::remove_dir_all(&p.donnees);
}
