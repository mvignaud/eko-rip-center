//! Support en direct de bout en bout : widget des pages connectées (identité signée, politique
//! de sécurité), lien « Support », anciennes demandes en lecture seule et anciennes routes
//! retirées.
//!
//! Un seul test : la configuration du widget est commune au processus.

use axum::http::HeaderMap;
use eko_rip_cloud::support::{self, Chatwoot};
use eko_rip_cloud::{abonnements, base, erp, relais};
use serde_json::json;
use std::time::Duration;

const URL: &str = "http://plateforme.test";
const JETON: &str = "jeton-public-de-la-boite";
const CLE: &str = "cle-hmac-tres-secrete-0123";

async fn servir(app: axum::Router) -> String {
    let ecoute = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let adresse = format!("http://{}", ecoute.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(ecoute, app).await.unwrap() });
    adresse
}

struct Rep {
    statut: u16,
    corps: String,
    lieu: String,
    entetes: HeaderMap,
}

impl Rep {
    fn csp(&self) -> String {
        self.entetes.get("content-security-policy").map(|v| v.to_str().unwrap().to_string()).unwrap_or_default()
    }
}

fn appel(base: &str, methode: &str, chemin: &str, cookie: Option<&str>, formulaire: &[(&str, &str)]) -> Rep {
    let a: ureq::Agent = ureq::Agent::config_builder().http_status_as_error(false).max_redirects(0).timeout_global(Some(Duration::from_secs(20))).build().into();
    let url = format!("{base}{chemin}");
    let r = match methode {
        "GET" => {
            let mut q = a.get(&url);
            if let Some(c) = cookie {
                q = q.header("Cookie", c);
            }
            q.call()
        }
        _ => {
            let mut q = a.post(&url).header("Origin", URL);
            if let Some(c) = cookie {
                q = q.header("Cookie", c);
            }
            q.send_form(formulaire.iter().copied())
        }
    };
    let mut r = r.unwrap();
    let lieu = r.headers().get("location").map(|v| v.to_str().unwrap().to_string()).unwrap_or_default();
    let entetes = r.headers().clone();
    let corps = r.body_mut().read_to_string().unwrap_or_default();
    Rep { statut: r.status().as_u16(), corps, lieu, entetes }
}

fn cookie(r: &Rep) -> String {
    r.entetes.get_all("set-cookie").iter().map(|v| v.to_str().unwrap()).find(|c| c.starts_with("plateforme_session=")).expect("session").split(';').next().unwrap().to_string()
}

#[tokio::test(flavor = "multi_thread")]
async fn support_en_direct_dans_la_bulle() {
    let donnees = std::env::temp_dir().join(format!("eko-cloud-support-{}", eko_rip_cloud::aleatoire_hex(6)));
    let r = relais::nouveau(relais::Config { donnees: donnees.clone(), url_publique: URL.into(), max_octets: 1 << 20, jeton_stats: String::new() });
    let _ = r.utiliser_memoire();
    let base_url = servir(relais::routes(r.clone())).await;
    let b = base_url.clone();
    let rb = r.clone();
    tokio::task::spawn_blocking(move || {
        let ins = appel(&b, "POST", "/inscription", None, &[("email", "claire@atelier.fr"), ("nom", "Claire Dupont"), ("mot_de_passe", "phrase-longue-1"), ("cgu", "1")]);
        assert_eq!(ins.statut, 303, "{}", ins.corps);
        let c = cookie(&ins);
        let id = base::compte_par_email(&rb.base(), "claire@atelier.fr").unwrap().unwrap().id;

        // 1. Sans configuration : aucune balise, politique inchangée.
        support::configurer(None);
        let p = appel(&b, "GET", "/sites", Some(&c), &[]);
        assert_eq!(p.statut, 200, "{}", p.corps);
        assert!(!p.corps.contains("/assets/chat.js"), "pas de widget sans jetons");
        assert!(!p.csp().contains("help.suite-eko.com"), "{}", p.csp());
        assert!(!p.corps.contains("/support/bulle.js"), "ancienne bulle retirée");

        // 2. Configuré : balise sur les pages connectées, identité signée, clé jamais montrée.
        support::configurer(Chatwoot::depuis(Some(JETON), Some(CLE)));
        let reference = format!("cloud-{id}");
        let attendu = eko_rip_cloud::hex(&eko_rip_cloud::hmac_sha256(CLE.as_bytes(), reference.as_bytes()));
        for chemin in ["/sites", "/compte", "/abonnement", "/support"] {
            let p = appel(&b, "GET", chemin, Some(&c), &[]);
            assert_eq!(p.statut, 200, "{chemin} : {}", p.corps);
            assert!(p.corps.contains(r#"<script src="/assets/chat.js" defer"#), "{chemin}");
            assert!(p.corps.contains(&format!(r#"data-jeton="{JETON}""#)), "{chemin}");
            assert!(p.corps.contains(&format!(r#"data-id="{reference}""#)), "{chemin}");
            assert!(p.corps.contains(&format!(r#"data-hash="{attendu}""#)), "{chemin}");
            assert!(p.corps.contains(r#"data-nom="Claire Dupont""#) && p.corps.contains(r#"data-email="claire@atelier.fr""#), "{chemin}");
            assert!(!p.corps.contains(CLE), "clé HMAC dans la page {chemin}");
            assert!(p.corps.contains(r#"<a data-support class="entree"#), "lien Support du menu : {chemin}");
            let csp = p.csp();
            for d in ["script-src", "frame-src", "img-src", "connect-src"] {
                let directive = csp.split(';').map(str::trim).find(|x| x.starts_with(d)).unwrap_or_else(|| panic!("{d} absente de {chemin} : {csp}"));
                assert!(directive.contains("https://help.suite-eko.com"), "{chemin} {d} : {csp}");
            }
            assert!(csp.contains("wss://help.suite-eko.com"), "{chemin} : {csp}");
        }
        // Pages publiques : jamais de widget.
        for chemin in ["/connexion", "/inscription", "/cgu"] {
            let p = appel(&b, "GET", chemin, None, &[]);
            assert!(!p.corps.contains("/assets/chat.js"), "{chemin}");
        }
        // Le script lit ses réglages dans sa balise, oublie la discussion à la déconnexion.
        let js = appel(&b, "GET", "/assets/chat.js", None, &[]);
        assert_eq!(js.statut, 200);
        assert!(js.entetes.get("content-type").unwrap().to_str().unwrap().starts_with("text/javascript"));
        for morceau in ["https://help.suite-eko.com", "/packs/js/sdk.js", "chatwootSDK.run", "chatwoot:ready", "setUser(d.id", "identifier_hash: d.hash", "setCustomAttributes", "compte_rip", "location.pathname", "toggle(\"open\")", "/deconnexion", "$chatwoot.reset()"] {
            assert!(js.corps.contains(morceau), "chat.js sans {morceau}");
        }
        assert!(!js.corps.contains(CLE));

        // 3. Anciennes demandes : lecture seule, seulement celles encore ouvertes.
        {
            let cx = rb.base();
            for (code, statut, sujet) in [("TKT-1", "open", "Imprimante absente"), ("TKT-2", "resolved", "Facture"), ("TKT-3", "waiting_client", "Profil ICC")] {
                cx.execute("INSERT INTO support_demandes (code, compte, sujet, statut) VALUES (?1, ?2, ?3, ?4)", rusqlite::params![code, id, sujet, statut]).unwrap();
            }
        }
        let p = appel(&b, "GET", "/support", Some(&c), &[]);
        assert!(p.corps.contains(support::BANDEAU), "{}", p.corps);
        assert!(p.corps.contains("Imprimante absente") && p.corps.contains("Profil ICC"), "{}", p.corps);
        assert!(!p.corps.contains("Facture"), "demande résolue cachée");
        assert!(!p.corps.contains("Nouvelle demande") && !p.corps.contains("/support/nouvelle"), "plus de nouvelle demande");
        // Anciennes routes : plus de création, de réponse ni de discussion maison.
        assert_eq!(appel(&b, "GET", "/support/nouvelle", Some(&c), &[]).lieu, "/support");
        assert_eq!(appel(&b, "GET", "/support/TKT-2", Some(&c), &[]).lieu, "/support", "demande résolue : plus de détail");
        assert!(appel(&b, "GET", "/support/direct/etat", Some(&c), &[]).statut >= 400);
        assert_ne!(appel(&b, "GET", "/support/bulle.js", None, &[]).statut, 200, "ancienne bulle plus servie");
        assert!(appel(&b, "POST", "/support/TKT-1/repondre", Some(&c), &[("message", "x")]).statut >= 400);

        // 4. Plus d'abonnement aux webhooks `support.*` ; ceux qui arrivent encore sont ignorés.
        assert!(erp::EVENEMENTS_WEBHOOK.iter().all(|e| !e.starts_with("support.")));
        assert!(!abonnements::traiter_webhook(&rb, "support.ticket.created", &json!({ "ticket": { "code": "TKT-9", "account_reference": reference } })));

        // 5. Après 30 jours, l'ancien index est vidé et la page ne garde que la bulle.
        support::entretenir(&rb.base(), eko_rip_cloud::maintenant() + support::LECTURE_SEULE + 1);
        let p = appel(&b, "GET", "/support", Some(&c), &[]);
        assert!(!p.corps.contains("Imprimante absente") && p.corps.contains(support::BANDEAU));
        support::configurer(None);
    })
    .await
    .unwrap();
    let _ = std::fs::remove_dir_all(&donnees);
}
