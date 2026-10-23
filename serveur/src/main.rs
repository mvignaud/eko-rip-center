//! E-KO RIP Center — serveur local : reçoit les PDF des postes clients, les rend en
//! plaques d'encre et les garde prêts pour les machines branchées sur le serveur.

mod render;

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path as Chemin, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::{io::AsyncWriteExt, sync::Semaphore};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Statut {
    EnAttente,
    EnCours,
    Termine,
    Echec,
}

#[derive(Clone, Serialize, Deserialize)]
struct Travail {
    id: Uuid,
    /// Nom du fichier envoyé, pour l'affichage seulement : jamais utilisé comme chemin.
    nom: String,
    dpi: u32,
    statut: Statut,
    /// Code stable traduit par l'écran (`rendu_echoue`, `interrompu`).
    #[serde(default)]
    code_erreur: Option<String>,
    /// Détail technique, non traduit (message de Ghostscript…).
    erreur: Option<String>,
    plaques: Vec<String>,
    /// Secondes depuis l'époque Unix.
    cree_le: u64,
}

struct Config {
    donnees: PathBuf,
    gs: String,
    max_octets: usize,
    delai: Duration,
}

struct Etat {
    config: Config,
    // ponytail: un seul rendu à la fois sur le serveur ; un sémaphore par machine quand
    // plusieurs imprimantes tourneront en parallèle.
    rendu: Semaphore,
}

type Partage = Arc<Etat>;

fn env_ou(nom: &str, defaut: &str) -> String {
    std::env::var(nom).unwrap_or_else(|_| defaut.to_string())
}

#[tokio::main]
async fn main() {
    let config = Config {
        donnees: env_ou("EKO_RIP_DATA", "eko-rip-data").into(),
        gs: env_ou("EKO_RIP_GS", if cfg!(windows) { "gswin64c" } else { "gs" }),
        max_octets: env_ou("EKO_RIP_MAX_MO", "2048").parse::<usize>().expect("EKO_RIP_MAX_MO") * 1024 * 1024,
        delai: Duration::from_secs(env_ou("EKO_RIP_DELAI_S", "1800").parse().expect("EKO_RIP_DELAI_S")),
    };
    let adresse = env_ou("EKO_RIP_ADDR", "0.0.0.0:8640");
    std::fs::create_dir_all(config.donnees.join("travaux")).expect("dossier de données");
    reprendre_travaux_interrompus(&config.donnees);

    let max = config.max_octets;
    let etat: Partage = Arc::new(Etat { config, rendu: Semaphore::new(1) });
    let app = routes(etat).layer(DefaultBodyLimit::max(max));

    let ecoute = tokio::net::TcpListener::bind(&adresse).await.expect("adresse d'écoute");
    println!("E-KO RIP Center à l'écoute sur http://{adresse}");
    axum::serve(ecoute, app).await.unwrap();
}

fn routes(etat: Partage) -> Router {
    Router::new()
        .route("/", get(|| async { Html(include_str!("../static/index.html")) }))
        .route("/api/travaux", get(lister).post(envoyer))
        .route("/api/travaux/{id}/apercu.png", get(apercu))
        .route("/api/travaux/{id}/plaques/{nom}", get(plaque))
        .with_state(etat)
}

fn dossier(donnees: &std::path::Path, id: Uuid) -> PathBuf {
    donnees.join("travaux").join(id.to_string())
}

fn ecrire(donnees: &std::path::Path, t: &Travail) -> std::io::Result<()> {
    let chemin = dossier(donnees, t.id).join("job.json");
    let tmp = chemin.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(t).unwrap())?;
    std::fs::rename(tmp, chemin)
}

fn lire_tous(donnees: &std::path::Path) -> Vec<Travail> {
    let Ok(entrees) = std::fs::read_dir(donnees.join("travaux")) else { return vec![] };
    let mut v: Vec<Travail> = entrees
        .filter_map(|e| std::fs::read(e.ok()?.path().join("job.json")).ok())
        .filter_map(|b| serde_json::from_slice(&b).ok())
        .collect();
    v.sort_by_key(|t| std::cmp::Reverse(t.cree_le));
    v
}

/// Un rendu coupé par un arrêt du serveur ne reprend pas tout seul : on le dit.
fn reprendre_travaux_interrompus(donnees: &std::path::Path) {
    for mut t in lire_tous(donnees) {
        if matches!(t.statut, Statut::EnAttente | Statut::EnCours) {
            t.statut = Statut::Echec;
            t.code_erreur = Some("interrompu".into());
            let _ = ecrire(donnees, &t);
        }
    }
}

async fn lister(State(etat): State<Partage>) -> Json<Vec<Travail>> {
    Json(lire_tous(&etat.config.donnees))
}

/// Réponse d'erreur : `code` est stable et traduit par l'écran, `detail` reste technique.
fn refus(statut: StatusCode, code: &'static str, detail: Option<String>) -> Response {
    (statut, Json(serde_json::json!({ "code": code, "detail": detail }))).into_response()
}

type Refus = (&'static str, Option<String>);

async fn envoyer(State(etat): State<Partage>, mut form: Multipart) -> Response {
    let id = Uuid::new_v4();
    let rep = dossier(&etat.config.donnees, id);
    if let Err(e) = tokio::fs::create_dir_all(&rep).await {
        return refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string()));
    }
    let pdf = rep.join("source.pdf");
    let (mut nom, mut dpi, mut recu) = (String::new(), 720u32, false);

    loop {
        let champ = match form.next_field().await {
            Ok(Some(c)) => c,
            Ok(None) => break,
            Err(e) => return abandon(&rep, StatusCode::BAD_REQUEST, ("envoi_illisible", Some(e.to_string()))).await,
        };
        match champ.name() {
            Some("dpi") => {
                dpi = match champ.text().await.ok().and_then(|t| t.trim().parse().ok()) {
                    Some(d @ 36..=2880) => d,
                    _ => return abandon(&rep, StatusCode::BAD_REQUEST, ("dpi_invalide", None)).await,
                };
            }
            Some("fichier") => {
                nom = champ.file_name().unwrap_or("sans nom").chars().take(200).collect();
                if let Err(r) = enregistrer(champ, &pdf).await {
                    return abandon(&rep, StatusCode::BAD_REQUEST, r).await;
                }
                recu = true;
            }
            _ => {}
        }
    }
    if !recu {
        return abandon(&rep, StatusCode::BAD_REQUEST, ("aucun_fichier", None)).await;
    }

    let cree_le = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let travail = Travail { id, nom, dpi, statut: Statut::EnAttente, code_erreur: None, erreur: None, plaques: vec![], cree_le };
    if let Err(e) = ecrire(&etat.config.donnees, &travail) {
        return abandon(&rep, StatusCode::INTERNAL_SERVER_ERROR, ("erreur_interne", Some(e.to_string()))).await;
    }
    tokio::spawn(traiter(etat.clone(), travail.clone()));
    (StatusCode::ACCEPTED, Json(travail)).into_response()
}

/// Écrit le fichier au fil de l'eau et refuse ce qui n'est pas un PDF.
async fn enregistrer(mut champ: axum::extract::multipart::Field<'_>, pdf: &std::path::Path) -> Result<(), Refus> {
    let interne = |e: std::io::Error| ("erreur_interne", Some(e.to_string()));
    let mut f = tokio::fs::File::create(pdf).await.map_err(interne)?;
    let mut debut = true;
    while let Some(morceau) = champ.chunk().await.map_err(|e| ("envoi_interrompu", Some(e.to_string())))? {
        if debut {
            if !morceau.starts_with(b"%PDF-") {
                return Err(("pas_un_pdf", None));
            }
            debut = false;
        }
        f.write_all(&morceau).await.map_err(interne)?;
    }
    if debut {
        return Err(("fichier_vide", None));
    }
    f.flush().await.map_err(interne)
}

async fn abandon(rep: &std::path::Path, statut: StatusCode, (code, detail): Refus) -> Response {
    let _ = tokio::fs::remove_dir_all(rep).await;
    refus(statut, code, detail)
}

async fn traiter(etat: Partage, mut t: Travail) {
    let _jeton = etat.rendu.acquire().await.unwrap();
    let donnees = &etat.config.donnees;
    t.statut = Statut::EnCours;
    let _ = ecrire(donnees, &t);

    let rep = dossier(donnees, t.id);
    match render::rendre(&etat.config.gs, &rep.join("source.pdf"), &rep, t.dpi, etat.config.delai).await {
        Ok(r) => {
            t.statut = Statut::Termine;
            t.plaques = r.plaques;
        }
        Err(e) => {
            eprintln!("travail {} : {e}", t.id);
            t.statut = Statut::Echec;
            t.code_erreur = Some("rendu_echoue".into());
            t.erreur = Some(e);
        }
    }
    if let Err(e) = ecrire(donnees, &t) {
        eprintln!("travail {} : état non enregistré : {e}", t.id);
    }
}

fn png(octets: Vec<u8>) -> Response {
    ([(header::CONTENT_TYPE, "image/png")], octets).into_response()
}

async fn apercu(State(etat): State<Partage>, Chemin(id): Chemin<Uuid>) -> Response {
    let f = dossier(&etat.config.donnees, id).join("apercu").join("composite.png");
    match tokio::fs::read(f).await {
        Ok(o) => png(o),
        Err(_) => refus(StatusCode::NOT_FOUND, "introuvable", None),
    }
}

async fn plaque(State(etat): State<Partage>, Chemin((id, nom)): Chemin<(Uuid, String)>) -> Response {
    let donnees = &etat.config.donnees;
    // Le nom doit être une plaque connue du travail : jamais un chemin fourni par le client.
    let connu = std::fs::read(dossier(donnees, id).join("job.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<Travail>(&b).ok())
        .is_some_and(|t| t.plaques.contains(&nom));
    if !connu {
        return refus(StatusCode::NOT_FOUND, "introuvable", None);
    }
    match tokio::fs::read(render::chemin_apercu_plaque(&dossier(donnees, id), &nom)).await {
        Ok(o) => png(o),
        Err(_) => refus(StatusCode::NOT_FOUND, "introuvable", None),
    }
}

#[cfg(test)]
mod tests {
    /// Chaque code d'erreur émis par ce fichier doit être traduit en FR et en EN dans l'écran.
    #[test]
    fn chaque_code_erreur_est_traduit_dans_les_deux_langues() {
        let source = include_str!("main.rs");
        let page = include_str!("../static/index.html");
        let (fr, en) = page.split_once("\n  en: {").expect("bloc anglais");
        let mut codes: Vec<&str> = vec![];
        for motif in ["(\"", "Some(\"", "NOT_FOUND, \"", "SERVER_ERROR, \""] {
            for (i, _) in source.match_indices(motif) {
                let reste = &source[i + motif.len()..];
                let fin = reste.find('"').unwrap();
                let (code, apres) = (&reste[..fin], &reste[fin + 1..]);
                let est_un_code = apres.starts_with(", ") || apres.starts_with(".into()");
                if est_un_code
                    && !code.is_empty() && code.chars().all(|c| c.is_ascii_lowercase() || c == '_') && !codes.contains(&code) {
                    codes.push(code);
                }
            }
        }
        for code in ["introuvable", "pas_un_pdf", "rendu_echoue", "interrompu", "dpi_invalide"] {
            assert!(codes.contains(&code), "le balayage n'a pas trouvé {code} : il est cassé");
        }
        for code in codes {
            assert!(fr.contains(&format!("{code}:")), "{code} non traduit en français");
            assert!(en.contains(&format!("{code}:")), "{code} non traduit en anglais");
        }
    }
}
