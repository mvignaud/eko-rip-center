//! E-KO RIP Center — serveur local : reçoit les PDF des postes clients, les rend en
//! plaques d'encre et les garde prêts pour les machines branchées sur le serveur.

mod couleur;
mod decoupe;
mod imbrication;
mod mosaique;
mod plotter;
mod render;
mod reperage;

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

/// Imprimer (rendu en plaques, plus la découpe s'il y a des tracés) ou découper seulement.
#[derive(Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Mode {
    #[default]
    Imprimer,
    Decouper,
}

/// Format reconnu aux premiers octets du fichier reçu.
#[derive(Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Format {
    #[default]
    Pdf,
    Svg,
}

impl Format {
    fn fichier(self) -> &'static str {
        match self {
            Format::Pdf => "source.pdf",
            Format::Svg => "source.svg",
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Travail {
    id: Uuid,
    /// Nom du fichier envoyé, pour l'affichage seulement : jamais utilisé comme chemin.
    nom: String,
    dpi: u32,
    #[serde(default)]
    mode: Mode,
    #[serde(default)]
    format: Format,
    /// Découper tous les tracés, pas seulement ceux des tons de découpe.
    #[serde(default)]
    tous_les_traces: bool,
    /// Repères imprimés pour la découpe sur le Kona (mode Imprimer).
    #[serde(default)]
    reperage: reperage::Reperage,
    /// Profils ICC et intention de rendu (mode Imprimer).
    #[serde(default)]
    couleur: couleur::Couleur,
    /// Mosaïque appliquée à l'envoi (laize et recouvrement, en mm).
    #[serde(default)]
    mosaique: Option<Mosaique>,
    statut: Statut,
    /// Code stable traduit par l'écran (`rendu_echoue`, `interrompu`).
    #[serde(default)]
    code_erreur: Option<String>,
    /// Détail technique, non traduit (message de Ghostscript…).
    erreur: Option<String>,
    /// Pages rendues, dans l'ordre du document.
    #[serde(default)]
    pages: Vec<Page>,
    /// Dernier envoi de la découpe au plotter.
    #[serde(default)]
    envoi_decoupe: Option<EnvoiDecoupe>,
    /// Secondes depuis l'époque Unix.
    cree_le: u64,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
struct Mosaique {
    laize_mm: f64,
    recouvrement_mm: f64,
}

#[derive(Clone, Serialize, Deserialize)]
struct Page {
    /// Plaques de la page (vide en découpe seule).
    plaques: Vec<String>,
    /// Parmi elles, les tons de découpe (`CutContour`…) : jamais envoyés à l'imprimante.
    #[serde(default)]
    plaques_decoupe: Vec<String>,
    /// Tracés de découpe de la page (0 : rien à découper).
    traces_decoupe: usize,
}

#[derive(Clone, Serialize, Deserialize)]
struct EnvoiDecoupe {
    /// Page envoyée (numérotée à partir de 1).
    #[serde(default = "premiere_page")]
    page: usize,
    statut: Statut,
    code_erreur: Option<String>,
    erreur: Option<String>,
    /// Secondes depuis l'époque Unix.
    le: u64,
}

fn premiere_page() -> usize {
    1
}

fn maintenant() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
}

struct Config {
    donnees: PathBuf,
    gs: String,
    max_octets: usize,
    delai: Duration,
    /// Profils ICC déposés depuis l'écran.
    profils: PathBuf,
    /// Port du plotter (le Kona : `/dev/kona`, créé par la règle udev du paquet).
    plotter: PathBuf,
    plotter_stty: String,
}

struct Etat {
    config: Config,
    // ponytail: un seul rendu à la fois sur le serveur ; un sémaphore par machine quand
    // plusieurs imprimantes tourneront en parallèle.
    rendu: Semaphore,
    /// Un seul envoi à la fois vers le plotter.
    plotter: Semaphore,
}

type Partage = Arc<Etat>;

fn env_ou(nom: &str, defaut: &str) -> String {
    std::env::var(nom).unwrap_or_else(|_| defaut.to_string())
}

#[tokio::main]
async fn main() {
    let donnees = PathBuf::from(env_ou("EKO_RIP_DATA", "eko-rip-data"));
    let config = Config {
        profils: donnees.join("profils"),
        donnees,
        gs: env_ou("EKO_RIP_GS", if cfg!(windows) { "gswin64c" } else { "gs" }),
        max_octets: env_ou("EKO_RIP_MAX_MO", "2048").parse::<usize>().expect("EKO_RIP_MAX_MO") * 1024 * 1024,
        delai: Duration::from_secs(env_ou("EKO_RIP_DELAI_S", "1800").parse().expect("EKO_RIP_DELAI_S")),
        plotter: env_ou("EKO_RIP_PLOTTER", "/dev/kona").into(),
        plotter_stty: env_ou("EKO_RIP_PLOTTER_STTY", plotter::STTY_DEFAUT),
    };
    let adresse = env_ou("EKO_RIP_ADDR", "0.0.0.0:8640");
    std::fs::create_dir_all(config.donnees.join("travaux")).expect("dossier de données");
    std::fs::create_dir_all(&config.profils).expect("dossier des profils");
    reprendre_travaux_interrompus(&config.donnees);

    let max = config.max_octets;
    let etat: Partage = Arc::new(Etat { config, rendu: Semaphore::new(1), plotter: Semaphore::new(1) });
    let app = routes(etat).layer(DefaultBodyLimit::max(max));

    let ecoute = tokio::net::TcpListener::bind(&adresse).await.expect("adresse d'écoute");
    println!("E-KO RIP Center à l'écoute sur http://{adresse}");
    axum::serve(ecoute, app).await.unwrap();
}

fn routes(etat: Partage) -> Router {
    Router::new()
        .route("/", get(|| async { Html(include_str!("../static/index.html")) }))
        .route("/api/travaux", get(lister).post(envoyer))
        .route("/api/profils", get(lister_profils).post(deposer_profil))
        .route("/api/planches", axum::routing::post(creer_planche))
        .route("/api/travaux/{id}", axum::routing::delete(supprimer))
        .route("/api/travaux/{id}/relancer", axum::routing::post(relancer))
        .route("/api/travaux/{id}/pages/{page}/apercu.png", get(apercu))
        .route("/api/travaux/{id}/pages/{page}/plaques/{nom}", get(plaque))
        .route("/api/travaux/{id}/pages/{page}/decoupe.plt", get(fichier_decoupe))
        .route("/api/travaux/{id}/pages/{page}/decoupe.svg", get(apercu_decoupe))
        .route("/api/travaux/{id}/pages/{page}/decouper", axum::routing::post(decouper))
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
    let provisoire = rep.join("source.tmp");
    let (mut nom, mut dpi, mut format) = (String::new(), 720u32, None);
    let (mut mode, mut tous_les_traces, mut reperage) = (Mode::Imprimer, false, reperage::Reperage::Aucun);
    let mut couleur = couleur::Couleur::default();
    let (mut mosaique, mut laize_mm, mut recouvrement_mm) = (false, 1600.0f64, 20.0f64);

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
            Some("mode") => {
                mode = match champ.text().await.ok().as_deref().map(str::trim) {
                    Some("imprimer") => Mode::Imprimer,
                    Some("decouper") => Mode::Decouper,
                    _ => return abandon(&rep, StatusCode::BAD_REQUEST, ("mode_invalide", None)).await,
                };
            }
            Some("traces") => {
                tous_les_traces = match champ.text().await.ok().as_deref().map(str::trim) {
                    Some("tons") => false,
                    Some("tous") => true,
                    _ => return abandon(&rep, StatusCode::BAD_REQUEST, ("mode_invalide", None)).await,
                };
            }
            Some("reperage") => {
                reperage = match champ.text().await.ok().as_deref().map(str::trim) {
                    Some("aucun") => reperage::Reperage::Aucun,
                    Some("manuel") => reperage::Reperage::Manuel,
                    Some("cadre") => reperage::Reperage::Cadre,
                    _ => return abandon(&rep, StatusCode::BAD_REQUEST, ("mode_invalide", None)).await,
                };
            }
            Some(champ_profil @ ("profil_cmjn" | "profil_rvb" | "profil_sortie")) => {
                let champ_profil = champ_profil.to_string();
                let choix = champ.text().await.unwrap_or_default().trim().to_string();
                let choix = (!choix.is_empty()).then_some(choix);
                match champ_profil.as_str() {
                    "profil_cmjn" => couleur.entree_cmjn = choix,
                    "profil_rvb" => couleur.entree_rvb = choix,
                    _ => couleur.sortie = choix,
                }
            }
            Some("intention") => {
                couleur.intention = match champ.text().await.ok().and_then(|t| t.trim().parse().ok()) {
                    Some(i @ 0..=3) => i,
                    _ => return abandon(&rep, StatusCode::BAD_REQUEST, ("profil_inadapte", None)).await,
                };
            }
            Some(champ_nombre @ ("laize_mm" | "recouvrement_mm")) => {
                let est_laize = champ_nombre == "laize_mm";
                let Some(v) = champ.text().await.ok().and_then(|t| t.trim().parse::<f64>().ok()) else {
                    return abandon(&rep, StatusCode::BAD_REQUEST, ("mosaique_invalide", None)).await;
                };
                if est_laize { laize_mm = v } else { recouvrement_mm = v }
            }
            Some("mosaique") => mosaique = champ.text().await.is_ok_and(|t| t.trim() == "oui"),
            Some("forcer_profils") => couleur.forcer = champ.text().await.is_ok_and(|t| t.trim() == "oui"),
            Some("fichier") => {
                nom = champ.file_name().unwrap_or("sans nom").chars().take(200).collect();
                match enregistrer(champ, &provisoire).await {
                    Ok(f) => format = Some(f),
                    Err(r) => return abandon(&rep, StatusCode::BAD_REQUEST, r).await,
                }
            }
            _ => {}
        }
    }
    let Some(format) = format else {
        return abandon(&rep, StatusCode::BAD_REQUEST, ("aucun_fichier", None)).await;
    };
    // Le rendu en plaques passe par Ghostscript, qui ne lit pas le SVG.
    if format == Format::Svg && mode == Mode::Imprimer {
        return abandon(&rep, StatusCode::BAD_REQUEST, ("svg_decoupe_seulement", None)).await;
    }
    // Rien n'est imprimé en découpe seule : ni repères ni couleur.
    if mode == Mode::Decouper {
        reperage = reperage::Reperage::Aucun;
        couleur = couleur::Couleur::default();
    }
    if let Err(code) = couleur::verifier(&couleur, &couleur::lister(&etat.config.profils)) {
        return abandon(&rep, StatusCode::BAD_REQUEST, (code, None)).await;
    }
    if let Err(e) = tokio::fs::rename(&provisoire, rep.join(format.fichier())).await {
        return abandon(&rep, StatusCode::INTERNAL_SERVER_ERROR, ("erreur_interne", Some(e.to_string()))).await;
    }
    // Mosaïque : la source est remplacée par ses lés ; l'original est gardé à côté.
    let mosaique = (mosaique && mode == Mode::Imprimer && format == Format::Pdf).then_some(Mosaique { laize_mm, recouvrement_mm });
    if let Some(m) = mosaique {
        if !(50.0..=5000.0).contains(&m.laize_mm) || !(0.0..=200.0).contains(&m.recouvrement_mm) {
            return abandon(&rep, StatusCode::BAD_REQUEST, ("mosaique_invalide", Some("laize 50–5000 mm, recouvrement 0–200 mm".into()))).await;
        }
        let source = rep.join(Format::Pdf.fichier());
        let original = rep.join("source-original.pdf");
        let resultat = match tokio::fs::rename(&source, &original).await.and(tokio::fs::read(&original).await) {
            Ok(pdf) => tokio::task::spawn_blocking(move || mosaique::composer(&pdf, m.laize_mm, m.recouvrement_mm)).await.unwrap_or_else(|e| Err(e.to_string())),
            Err(e) => Err(e.to_string()),
        };
        match resultat {
            Ok(les) => {
                if let Err(e) = tokio::fs::write(&source, les).await {
                    return abandon(&rep, StatusCode::INTERNAL_SERVER_ERROR, ("erreur_interne", Some(e.to_string()))).await;
                }
            }
            Err(e) => return abandon(&rep, StatusCode::BAD_REQUEST, ("mosaique_invalide", Some(e))).await,
        }
    }

    let cree_le = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let travail = Travail { id, nom, dpi, mode, format, tous_les_traces, reperage, couleur, mosaique, statut: Statut::EnAttente, code_erreur: None, erreur: None, pages: vec![], envoi_decoupe: None, cree_le };
    if let Err(e) = ecrire(&etat.config.donnees, &travail) {
        return abandon(&rep, StatusCode::INTERNAL_SERVER_ERROR, ("erreur_interne", Some(e.to_string()))).await;
    }
    tokio::spawn(traiter(etat.clone(), travail.clone()));
    (StatusCode::ACCEPTED, Json(travail)).into_response()
}

/// Octets lus avant de reconnaître le format.
const EN_TETE: usize = 1024;

/// PDF (un fichier Illustrator « compatible PDF » en est un), SVG, ou refus.
fn reconnaitre(debut: &[u8]) -> Result<Format, Refus> {
    if debut.starts_with(b"%PDF-") {
        return Ok(Format::Pdf);
    }
    if debut.starts_with(b"%!PS-Adobe") {
        return Err(("ai_sans_pdf", None));
    }
    let texte = String::from_utf8_lossy(debut);
    if texte.trim_start_matches('\u{feff}').trim_start().starts_with('<') && texte.contains("<svg") {
        return Ok(Format::Svg);
    }
    Err(("format_inconnu", None))
}

/// Écrit le fichier au fil de l'eau ; le format se décide sur les premiers octets.
async fn enregistrer(mut champ: axum::extract::multipart::Field<'_>, chemin: &std::path::Path) -> Result<Format, Refus> {
    let interne = |e: std::io::Error| ("erreur_interne", Some(e.to_string()));
    let mut f = tokio::fs::File::create(chemin).await.map_err(interne)?;
    let (mut debut, mut format) = (Vec::new(), None);
    while let Some(morceau) = champ.chunk().await.map_err(|e| ("envoi_interrompu", Some(e.to_string())))? {
        if format.is_none() {
            debut.extend_from_slice(&morceau);
            if debut.len() < EN_TETE {
                continue;
            }
            format = Some(reconnaitre(&debut)?);
            f.write_all(&debut).await.map_err(interne)?;
            continue;
        }
        f.write_all(&morceau).await.map_err(interne)?;
    }
    let format = match format {
        Some(f) => f,
        None if debut.is_empty() => return Err(("fichier_vide", None)),
        None => {
            let reconnu = reconnaitre(&debut)?;
            f.write_all(&debut).await.map_err(interne)?;
            reconnu
        }
    };
    f.flush().await.map_err(interne)?;
    Ok(format)
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
    let rendu = match t.mode {
        Mode::Imprimer => match preparer_impression(&rep, t.reperage).await {
            Ok((pdf, mises)) => {
                let options = couleur::options_ghostscript(&t.couleur, &etat.config.profils);
                render::rendre(&etat.config.gs, &pdf, &rep, t.dpi, &options, etat.config.delai).await.map(|r| (Some(r.pages), mises))
            }
            Err(e) => Err(e),
        },
        Mode::Decouper => Ok((None, vec![])),
    };
    match rendu {
        Ok((plaques, mises)) => match preparer_decoupe(&rep, t.format, t.tous_les_traces, &mises).await {
            Ok(traces) => {
                // En découpe seule, il n'y a pas de plaques : une page par jeu de tracés.
                let plaques = plaques.unwrap_or_else(|| vec![vec![]; traces.len()]);
                t.pages = plaques
                    .into_iter()
                    .enumerate()
                    .map(|(i, plaques)| Page {
                        plaques_decoupe: plaques.iter().filter(|p| decoupe::est_ton_de_decoupe(p)).cloned().collect(),
                        plaques,
                        traces_decoupe: traces.get(i).copied().unwrap_or(0),
                    })
                    .collect();
                t.statut = Statut::Termine;
            }
            Err(e) => {
                eprintln!("travail {} : {e}", t.id);
                t.statut = Statut::Echec;
                t.code_erreur = Some("decoupe_illisible".into());
                t.erreur = Some(e);
            }
        },
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

#[derive(Deserialize)]
struct DemandePlanche {
    laize_mm: f64,
    #[serde(default = "espace_defaut")]
    espacement_mm: f64,
    #[serde(default = "dpi_defaut")]
    dpi: u32,
    #[serde(default)]
    reperage: reperage::Reperage,
    elements: Vec<ElementPlanche>,
}

#[derive(Deserialize)]
struct ElementPlanche {
    id: Uuid,
    page: usize,
    copies: usize,
}

fn espace_defaut() -> f64 {
    5.0
}

fn dpi_defaut() -> u32 {
    720
}

/// Imbrique des pages de travaux existants sur la laize, en un nouveau travail à imprimer.
async fn creer_planche(State(etat): State<Partage>, Json(d): Json<DemandePlanche>) -> Response {
    let mauvais = |detail: &str| refus(StatusCode::BAD_REQUEST, "planche_invalide", Some(detail.to_string()));
    if !(50.0..=5000.0).contains(&d.laize_mm) || !(0.0..=100.0).contains(&d.espacement_mm) || !(36..=2880).contains(&d.dpi) {
        return mauvais("laize 50–5000 mm, espacement 0–100 mm, 36–2880 dpi");
    }
    if d.elements.is_empty() || d.elements.len() > 200 || d.elements.iter().any(|e| !(1..=500).contains(&e.copies)) {
        return mauvais("1 à 200 éléments, 1 à 500 copies chacun");
    }
    let donnees = &etat.config.donnees;
    let mut pdfs = vec![];
    for e in &d.elements {
        let Some((t, _)) = lire_page(donnees, e.id, e.page) else { return refus(StatusCode::NOT_FOUND, "introuvable", None) };
        if t.format != Format::Pdf {
            return mauvais("seules des pages PDF s'imbriquent");
        }
        match tokio::fs::read(dossier(donnees, e.id).join(Format::Pdf.fichier())).await {
            Ok(o) => pdfs.push(o),
            Err(err) => return refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(err.to_string())),
        }
    }
    let (laize, espace) = (d.laize_mm, d.espacement_mm);
    let pages: Vec<(usize, usize)> = d.elements.iter().map(|e| (e.page, e.copies)).collect();
    let compose = tokio::task::spawn_blocking(move || {
        let sources: Vec<imbrication::Source> = pdfs.iter().zip(&pages).map(|(pdf, &(page, copies))| imbrication::Source { pdf, page, copies }).collect();
        imbrication::composer(&sources, laize, espace)
    })
    .await;
    let planche = match compose {
        Ok(Ok((pdf, _))) => pdf,
        Ok(Err(e)) => return mauvais(&e),
        Err(e) => return refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string())),
    };

    let id = Uuid::new_v4();
    let rep = dossier(donnees, id);
    if let Err(e) = tokio::fs::create_dir_all(&rep).await.and(tokio::fs::write(rep.join(Format::Pdf.fichier()), planche).await) {
        return abandon(&rep, StatusCode::INTERNAL_SERVER_ERROR, ("erreur_interne", Some(e.to_string()))).await;
    }
    let copies: usize = d.elements.iter().map(|e| e.copies).sum();
    let travail = Travail {
        id,
        nom: format!("Planche {laize:.0} mm — {copies} élément(s)"),
        dpi: d.dpi,
        mode: Mode::Imprimer,
        format: Format::Pdf,
        tous_les_traces: false,
        reperage: d.reperage,
        couleur: couleur::Couleur::default(),
        mosaique: None,
        statut: Statut::EnAttente,
        code_erreur: None,
        erreur: None,
        pages: vec![],
        envoi_decoupe: None,
        cree_le: maintenant(),
    };
    if let Err(e) = ecrire(donnees, &travail) {
        return abandon(&rep, StatusCode::INTERNAL_SERVER_ERROR, ("erreur_interne", Some(e.to_string()))).await;
    }
    tokio::spawn(traiter(etat.clone(), travail.clone()));
    (StatusCode::ACCEPTED, Json(travail)).into_response()
}

async fn lister_profils(State(etat): State<Partage>) -> Json<Vec<couleur::Profil>> {
    Json(couleur::lister(&etat.config.profils))
}

/// Dépose un profil ICC : nom contrôlé, en-tête ICC vérifié, 50 Mo au plus.
async fn deposer_profil(State(etat): State<Partage>, mut form: Multipart) -> Response {
    let Ok(Some(champ)) = form.next_field().await else { return refus(StatusCode::BAD_REQUEST, "aucun_fichier", None) };
    let nom = champ.file_name().unwrap_or_default().to_string();
    if !couleur::nom_valide(&nom) {
        return refus(StatusCode::BAD_REQUEST, "profil_invalide", Some(nom));
    }
    let octets = match champ.bytes().await {
        Ok(o) if o.len() <= 50 * 1024 * 1024 => o,
        Ok(_) => return refus(StatusCode::BAD_REQUEST, "profil_invalide", Some("plus de 50 Mo".into())),
        Err(e) => return refus(StatusCode::BAD_REQUEST, "envoi_interrompu", Some(e.to_string())),
    };
    let Some(profil) = couleur::lire_entete(&octets, &nom) else { return refus(StatusCode::BAD_REQUEST, "profil_invalide", None) };
    match tokio::fs::write(etat.config.profils.join(&nom), &octets).await {
        Ok(()) => (StatusCode::CREATED, Json(profil)).into_response(),
        Err(e) => refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string())),
    }
}

fn png(octets: Vec<u8>) -> Response {
    ([(header::CONTENT_TYPE, "image/png")], octets).into_response()
}

/// PDF à rendre : la source telle quelle, ou `imprime.pdf` agrandi avec ses repères
/// (une mise en page par page).
async fn preparer_impression(rep: &std::path::Path, choix: reperage::Reperage) -> Result<(PathBuf, Vec<reperage::Mise>), String> {
    let source = rep.join(Format::Pdf.fichier());
    if choix == reperage::Reperage::Aucun {
        return Ok((source, vec![]));
    }
    let pdf = tokio::fs::read(&source).await.map_err(|e| e.to_string())?;
    let (imprime, mises) = tokio::task::spawn_blocking(move || reperage::envelopper(&pdf, choix)).await.map_err(|e| e.to_string())??;
    let chemin = rep.join("imprime.pdf");
    tokio::fs::write(&chemin, imprime).await.map_err(|e| e.to_string())?;
    Ok((chemin, mises))
}

fn fichier_plt(rep: &std::path::Path, page: usize) -> PathBuf {
    rep.join("prod").join(format!("decoupe-{page:03}.plt"))
}

fn fichier_svg(rep: &std::path::Path, page: usize) -> PathBuf {
    rep.join("apercu").join(format!("decoupe-{page:03}.svg"))
}

/// Extrait les tracés à découper de chaque page ; pour une page qui en a, écrit
/// `prod/decoupe-NNN.plt` (HP-GL) et `apercu/decoupe-NNN.svg`. Avec un repérage, les
/// tracés suivent le décalage de leur page. Rend le nombre de tracés par page.
async fn preparer_decoupe(rep: &std::path::Path, format: Format, tous: bool, mises: &[reperage::Mise]) -> Result<Vec<usize>, String> {
    let source = tokio::fs::read(rep.join(format.fichier())).await.map_err(|e| e.to_string())?;
    let mut pages = tokio::task::spawn_blocking(move || match format {
        Format::Pdf => decoupe::extraire(&source, tous),
        Format::Svg => decoupe::extraire_svg(&source, tous).map(|d| vec![d]),
    })
    .await
    .map_err(|e| e.to_string())??;
    for sous in ["prod", "apercu"] {
        tokio::fs::create_dir_all(rep.join(sous)).await.map_err(|e| e.to_string())?;
    }
    for (i, d) in pages.iter_mut().enumerate() {
        if let Some(m) = mises.get(i) {
            d.decaler(m.decalage, m.page);
        }
        if !d.traces.is_empty() {
            tokio::fs::write(fichier_plt(rep, i + 1), decoupe::hpgl(&d.traces)).await.map_err(|e| e.to_string())?;
            tokio::fs::write(fichier_svg(rep, i + 1), decoupe::apercu_svg(d)).await.map_err(|e| e.to_string())?;
        }
    }
    Ok(pages.iter().map(|d| d.traces.len()).collect())
}

/// Rendu ou envoi au plotter en cours : on ne touche pas au dossier du travail.
fn occupe(t: &Travail) -> bool {
    let actif = |s: &Statut| matches!(s, Statut::EnAttente | Statut::EnCours);
    actif(&t.statut) || t.envoi_decoupe.as_ref().is_some_and(|e| actif(&e.statut))
}

async fn supprimer(State(etat): State<Partage>, Chemin(id): Chemin<Uuid>) -> Response {
    let donnees = &etat.config.donnees;
    let Some(t) = lire_travail(donnees, id) else { return refus(StatusCode::NOT_FOUND, "introuvable", None) };
    if occupe(&t) {
        return refus(StatusCode::CONFLICT, "travail_occupe", None);
    }
    match tokio::fs::remove_dir_all(dossier(donnees, id)).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string())),
    }
}

/// Recommence le rendu et la préparation de la découpe à partir du fichier reçu.
async fn relancer(State(etat): State<Partage>, Chemin(id): Chemin<Uuid>) -> Response {
    let donnees = &etat.config.donnees;
    let Some(mut t) = lire_travail(donnees, id) else { return refus(StatusCode::NOT_FOUND, "introuvable", None) };
    if occupe(&t) {
        return refus(StatusCode::CONFLICT, "travail_occupe", None);
    }
    // Les sorties d'avant ne doivent pas survivre au nouveau rendu (un .plt périmé partirait au plotter).
    let rep = dossier(donnees, id);
    for sous in ["prod", "apercu"] {
        let _ = tokio::fs::remove_dir_all(rep.join(sous)).await;
    }
    let _ = tokio::fs::remove_file(rep.join("imprime.pdf")).await;
    t.statut = Statut::EnAttente;
    (t.code_erreur, t.erreur, t.envoi_decoupe) = (None, None, None);
    t.pages = vec![];
    if let Err(e) = ecrire(donnees, &t) {
        return refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string()));
    }
    tokio::spawn(traiter(etat.clone(), t.clone()));
    (StatusCode::ACCEPTED, Json(t)).into_response()
}

fn lire_travail(donnees: &std::path::Path, id: Uuid) -> Option<Travail> {
    serde_json::from_slice(&std::fs::read(dossier(donnees, id).join("job.json")).ok()?).ok()
}

/// Travail et page demandés, si la page existe (numérotée à partir de 1).
fn lire_page(donnees: &std::path::Path, id: Uuid, page: usize) -> Option<(Travail, Page)> {
    let t = lire_travail(donnees, id)?;
    let p = t.pages.get(page.checked_sub(1)?)?.clone();
    Some((t, p))
}

/// Envoie la découpe d'une page au plotter ; l'état de l'envoi est suivi dans le travail.
async fn decouper(State(etat): State<Partage>, Chemin((id, page)): Chemin<(Uuid, usize)>) -> Response {
    let donnees = &etat.config.donnees;
    let Some((mut t, p)) = lire_page(donnees, id, page) else { return refus(StatusCode::NOT_FOUND, "introuvable", None) };
    if p.traces_decoupe == 0 {
        return refus(StatusCode::CONFLICT, "rien_a_decouper", None);
    }
    // ponytail: le contrôle puis l'écriture ne sont pas atomiques ; deux clics dans la même
    // milliseconde enverraient deux fois. Le sémaphore les fait au moins passer l'un après l'autre.
    if t.envoi_decoupe.as_ref().is_some_and(|e| matches!(e.statut, Statut::EnAttente | Statut::EnCours)) {
        return refus(StatusCode::CONFLICT, "envoi_en_cours", None);
    }
    t.envoi_decoupe = Some(EnvoiDecoupe { page, statut: Statut::EnAttente, code_erreur: None, erreur: None, le: maintenant() });
    if let Err(e) = ecrire(donnees, &t) {
        return refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string()));
    }
    tokio::spawn(envoyer_au_plotter(etat.clone(), id, page));
    (StatusCode::ACCEPTED, Json(t)).into_response()
}

async fn envoyer_au_plotter(etat: Partage, id: Uuid, page: usize) {
    let _jeton = etat.plotter.acquire().await.unwrap();
    let donnees = &etat.config.donnees;
    let maj = |statut: Statut, detail: Option<String>| {
        if let Some(mut t) = lire_travail(donnees, id) {
            let code_erreur = matches!(statut, Statut::Echec).then_some("envoi_decoupe_echoue".to_string());
            t.envoi_decoupe = Some(EnvoiDecoupe { page, statut, code_erreur, erreur: detail, le: maintenant() });
            let _ = ecrire(donnees, &t);
        }
    };
    maj(Statut::EnCours, None);
    let plt = fichier_plt(&dossier(donnees, id), page);
    let (port, reglages) = (etat.config.plotter.clone(), etat.config.plotter_stty.clone());
    let resultat = tokio::task::spawn_blocking(move || {
        let octets = std::fs::read(&plt).map_err(|e| format!("lecture de {} : {e}", plt.display()))?;
        plotter::envoyer(&port, &reglages, &octets)
    })
    .await
    .unwrap_or_else(|e| Err(e.to_string()));
    match resultat {
        Ok(()) => maj(Statut::Termine, None),
        Err(e) => {
            eprintln!("travail {id} : découpe : {e}");
            maj(Statut::Echec, Some(e));
        }
    }
}

/// Lit un fichier de sortie d'une page existante du travail ; `None` donne un 404.
async fn servir(donnees: &std::path::Path, id: Uuid, page: usize, chemin: impl FnOnce(&std::path::Path, &Page) -> Option<PathBuf>) -> Option<Vec<u8>> {
    let (_, p) = lire_page(donnees, id, page)?;
    tokio::fs::read(chemin(&dossier(donnees, id), &p)?).await.ok()
}

fn introuvable() -> Response {
    refus(StatusCode::NOT_FOUND, "introuvable", None)
}

async fn apercu_decoupe(State(etat): State<Partage>, Chemin((id, page)): Chemin<(Uuid, usize)>) -> Response {
    match servir(&etat.config.donnees, id, page, |rep, _| Some(fichier_svg(rep, page))).await {
        Some(o) => ([(header::CONTENT_TYPE, "image/svg+xml")], o).into_response(),
        None => introuvable(),
    }
}

async fn fichier_decoupe(State(etat): State<Partage>, Chemin((id, page)): Chemin<(Uuid, usize)>) -> Response {
    match servir(&etat.config.donnees, id, page, |rep, _| Some(fichier_plt(rep, page))).await {
        Some(o) => {
            let nom = format!("attachment; filename=\"decoupe-{page:03}.plt\"");
            ([(header::CONTENT_TYPE, "application/vnd.hp-hpgl".to_string()), (header::CONTENT_DISPOSITION, nom)], o).into_response()
        }
        None => introuvable(),
    }
}

async fn apercu(State(etat): State<Partage>, Chemin((id, page)): Chemin<(Uuid, usize)>) -> Response {
    match servir(&etat.config.donnees, id, page, |rep, _| Some(render::chemin_composite(rep, page))).await {
        Some(o) => png(o),
        None => introuvable(),
    }
}

async fn plaque(State(etat): State<Partage>, Chemin((id, page, nom)): Chemin<(Uuid, usize, String)>) -> Response {
    // Le nom doit être une plaque connue de la page : jamais un chemin fourni par le client.
    let chemin = |rep: &std::path::Path, p: &Page| p.plaques.contains(&nom).then(|| render::chemin_apercu_plaque(rep, page, &nom));
    match servir(&etat.config.donnees, id, page, chemin).await {
        Some(o) => png(o),
        None => introuvable(),
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
        // Un code est une chaîne en minuscules et `_` placée après `(` ou `StatusCode::X, `
        // et suivie de `, `, `.into()` ou `.to_string()`.
        let mut codes: Vec<&str> = vec![];
        for (i, _) in source.match_indices('"') {
            let avant = &source[..i];
            let apres_statut = avant.strip_suffix(", ").and_then(|a| a.rsplit_once("StatusCode::")).is_some_and(|(_, nom)| {
                nom.chars().all(|c| c.is_ascii_uppercase() || c == '_')
            });
            if !(avant.ends_with('(') || apres_statut) {
                continue;
            }
            let reste = &source[i + 1..];
            let Some(fin) = reste.find('"') else { continue };
            let (code, apres) = (&reste[..fin], &reste[fin + 1..]);
            let est_un_code = [", ", ".into()", ".to_string()"].iter().any(|s| apres.starts_with(s));
            if est_un_code && !code.is_empty() && code.chars().all(|c| c.is_ascii_lowercase() || c == '_') && !codes.contains(&code) {
                codes.push(code);
            }
        }
        for code in ["introuvable", "format_inconnu", "ai_sans_pdf", "rendu_echoue", "interrompu", "dpi_invalide", "rien_a_decouper", "envoi_decoupe_echoue"] {
            assert!(codes.contains(&code), "le balayage n'a pas trouvé {code} : il est cassé");
        }
        for code in codes {
            assert!(fr.contains(&format!("{code}:")), "{code} non traduit en français");
            assert!(en.contains(&format!("{code}:")), "{code} non traduit en anglais");
        }
    }
}
