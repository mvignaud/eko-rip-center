//! `eko-rip-cloud` : la plate-forme E-KO RIP Cloud (relais des tunnels et appairage).
//!
//! Commandes :
//!   eko-rip-cloud                         lance le service
//!   eko-rip-cloud valider <CODE> [qui]    valide un code d'appairage affiché par un serveur RIP
//!   eko-rip-cloud sites                   liste les sites
//!   eko-rip-cloud supprimer <site>        retire un site (son tunnel est coupé dans la minute)

use eko_rip_cloud::{appairage, relais, sites};
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
    };
    let adresse = env_ou("EKO_CLOUD_ADDR", "127.0.0.1:8650");
    let r = relais::nouveau(config);
    tokio::spawn(relais::entretenir(r.clone()));
    let ecoute = tokio::net::TcpListener::bind(&adresse).await.expect("adresse d'écoute");
    println!("E-KO RIP Cloud à l'écoute sur http://{adresse}");
    axum::serve(ecoute, relais::routes(r)).await.expect("service");
}
