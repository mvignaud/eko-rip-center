//! `eko-rip-cloud` : la plate-forme E-KO RIP Cloud (relais des tunnels et appairage).
//!
//! Commandes :
//!   eko-rip-cloud                         lance le service
//!   eko-rip-cloud valider <CODE> [qui]    valide un code d'appairage affiché par un serveur RIP
//!   eko-rip-cloud sites                   liste les sites
//!   eko-rip-cloud supprimer <site>        retire un site (son tunnel est coupé dans la minute)
//!   eko-rip-cloud installations           statistiques anonymes des installations (JSON)
//!   eko-rip-cloud admin <e-mail>          donne l'espace d'administration à un compte du portail

use eko_rip_cloud::{appairage, installations, relais, sites};
use std::path::PathBuf;

fn env_ou(nom: &str, defaut: &str) -> String {
    std::env::var(nom).ok().filter(|v| !v.is_empty()).unwrap_or_else(|| defaut.to_string())
}

#[tokio::main]
async fn main() {
    let donnees = PathBuf::from(env_ou("EKO_CLOUD_DATA", "eko-cloud-data"));
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("valider") => std::process::exit(match args.get(1).map(|c| appairage::valider(&donnees, c, args.get(2).map_or("", String::as_str))) {
            Some(Ok(c)) => {
                println!("Code {} validé : le serveur RIP reçoit sa clé à sa prochaine interrogation (quelques secondes).", appairage::afficher(&c));
                0
            }
            Some(Err(e)) => {
                eprintln!("{e}");
                1
            }
            None => {
                eprintln!("Usage : eko-rip-cloud valider <CODE> [propriétaire]");
                2
            }
        }),
        Some("sites") => std::process::exit(match sites::lire(&donnees) {
            Ok(liste) => {
                for s in liste {
                    println!("{}\t{}\t{}\t{}", s.id, s.nom, s.proprietaire.unwrap_or_default(), s.derniere_connexion.map_or("jamais connecté".into(), |t| t.to_string()));
                }
                0
            }
            Err(e) => {
                eprintln!("{e}");
                1
            }
        }),
        Some("supprimer") => std::process::exit(match args.get(1).map(|id| sites::supprimer(&donnees, id)) {
            Some(Ok(true)) => {
                println!("Site retiré ; son tunnel est coupé dans la minute.");
                0
            }
            Some(Ok(false)) => {
                eprintln!("Site inconnu.");
                1
            }
            Some(Err(e)) => {
                eprintln!("{e}");
                1
            }
            None => {
                eprintln!("Usage : eko-rip-cloud supprimer <site>");
                2
            }
        }),
        // Le service écrit le registre au plus une minute après un ping.
        Some("installations") => std::process::exit(match installations::lire(&donnees) {
            Ok(r) => {
                println!("{}", serde_json::to_string_pretty(&r.statistiques(eko_rip_cloud::maintenant())).expect("statistiques"));
                0
            }
            Err(e) => {
                eprintln!("{e}");
                1
            }
        }),
        Some("admin") => std::process::exit(match args.get(1) {
            Some(email) => match eko_rip_cloud::base::ouvrir(&donnees).and_then(|c| eko_rip_cloud::base::nommer_super_admin(&c, email)) {
                Ok(true) => {
                    println!("{email} est super-administrateur de la plate-forme.");
                    0
                }
                Ok(false) => {
                    eprintln!("Aucun compte avec cet e-mail : le créer d'abord sur le portail (/inscription).");
                    1
                }
                Err(e) => {
                    eprintln!("{e}");
                    1
                }
            },
            None => {
                eprintln!("Usage : eko-rip-cloud admin <e-mail>");
                2
            }
        }),
        Some(autre) => {
            eprintln!("Commande inconnue : {autre}");
            std::process::exit(2);
        }
        None => {}
    }
    std::fs::create_dir_all(&donnees).expect("dossier de données");
    sites::lire(&donnees).expect("sites.json");
    let config = relais::Config {
        donnees,
        url_publique: env_ou("EKO_CLOUD_URL", "https://platform.e-ko.rip").trim_end_matches('/').to_string(),
        max_octets: env_ou("EKO_CLOUD_MAX_MO", "2048").parse::<u64>().expect("EKO_CLOUD_MAX_MO") * 1024 * 1024,
        jeton_stats: env_ou("EKO_CLOUD_JETON_STATS", ""),
    };
    let adresse = env_ou("EKO_CLOUD_ADDR", "127.0.0.1:8650");
    let r = relais::nouveau(config);
    tokio::spawn(relais::entretenir(r.clone()));
    let ecoute = tokio::net::TcpListener::bind(&adresse).await.expect("adresse d'écoute");
    println!("E-KO RIP Cloud à l'écoute sur http://{adresse}");
    axum::serve(ecoute, relais::routes(r.clone())).with_graceful_shutdown(arret()).await.expect("service");
    relais::enregistrer_installations(&r).await;
}

/// Ctrl-C ou SIGTERM (arrêt par systemd) : on écrit le registre des installations avant de sortir.
async fn arret() {
    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).expect("signal SIGTERM");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = term.recv() => {}
    }
}
