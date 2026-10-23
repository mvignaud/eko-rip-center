//! E-KO RIP Center — serveur local : reçoit les PDF des postes clients, les rend en
//! plaques d'encre et les garde prêts pour les machines branchées sur le serveur.

mod decoupe;
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
    statut: Statut,
    /// Code stable traduit par l'écran (`rendu_echoue`, `interrompu`).
    #[serde(default)]
    code_erreur: Option<String>,
    /// Détail technique, non traduit (message de Ghostscript…).
    erreur: Option<String>,
    plaques: Vec<String>,
    /// Tracés de découpe trouvés dans le PDF (0 : rien à découper).
    #[serde(default)]
    traces_decoupe: usize,
    /// Dernier envoi de la découpe au plotter.
    #[serde(default)]
    envoi_decoupe: Option<EnvoiDecoupe>,
    /// Secondes depuis l'époque Unix.
    cree_le: u64,
}

#[derive(Clone, Serialize, Deserialize)]
struct EnvoiDecoupe {
    statut: Statut,
    code_erreur: Option<String>,
    erreur: Option<String>,
    /// Secondes depuis l'époque Unix.
    le: u64,
}

fn maintenant() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
}

struct Config {
    donnees: PathBuf,
    gs: String,
    max_octets: usize,
    delai: Duration,
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
    let config = Config {
        donnees: env_ou("EKO_RIP_DATA", "eko-rip-data").into(),
        gs: env_ou("EKO_RIP_GS", if cfg!(windows) { "gswin64c" } else { "gs" }),
        max_octets: env_ou("EKO_RIP_MAX_MO", "2048").parse::<usize>().expect("EKO_RIP_MAX_MO") * 1024 * 1024,
        delai: Duration::from_secs(env_ou("EKO_RIP_DELAI_S", "1800").parse().expect("EKO_RIP_DELAI_S")),
        plotter: env_ou("EKO_RIP_PLOTTER", "/dev/kona").into(),
        plotter_stty: env_ou("EKO_RIP_PLOTTER_STTY", plotter::STTY_DEFAUT),
    };
    let adresse = env_ou("EKO_RIP_ADDR", "0.0.0.0:8640");
    std::fs::create_dir_all(config.donnees.join("travaux")).expect("dossier de données");
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
        .route("/api/travaux/{id}/apercu.png", get(apercu))
        .route("/api/travaux/{id}/decoupe.plt", get(fichier_decoupe))
        .route("/api/travaux/{id}/decoupe.svg", get(apercu_decoupe))
        .route("/api/travaux/{id}/decouper", axum::routing::post(decouper))
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
    let provisoire = rep.join("source.tmp");
    let (mut nom, mut dpi, mut format) = (String::new(), 720u32, None);
    let (mut mode, mut tous_les_traces, mut reperage) = (Mode::Imprimer, false, reperage::Reperage::Aucun);

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
    // Rien n'est imprimé en découpe seule : pas de repères à poser.
    if mode == Mode::Decouper {
        reperage = reperage::Reperage::Aucun;
    }
    if let Err(e) = tokio::fs::rename(&provisoire, rep.join(format.fichier())).await {
        return abandon(&rep, StatusCode::INTERNAL_SERVER_ERROR, ("erreur_interne", Some(e.to_string()))).await;
    }

    let cree_le = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let travail = Travail { id, nom, dpi, mode, format, tous_les_traces, reperage, statut: Statut::EnAttente, code_erreur: None, erreur: None, plaques: vec![], traces_decoupe: 0, envoi_decoupe: None, cree_le };
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
            Ok((pdf, mise)) => render::rendre(&etat.config.gs, &pdf, &rep, t.dpi, etat.config.delai).await.map(|r| (r.plaques, mise)),
            Err(e) => Err(e),
        },
        Mode::Decouper => Ok((vec![], None)),
    };
    match rendu {
        Ok((plaques, mise)) => {
            t.plaques = plaques;
            match preparer_decoupe(&rep, t.format, t.tous_les_traces, mise.as_ref()).await {
                Ok(n) => {
                    t.statut = Statut::Termine;
                    t.traces_decoupe = n;
                }
                Err(e) => {
                    eprintln!("travail {} : {e}", t.id);
                    t.statut = Statut::Echec;
                    t.code_erreur = Some("decoupe_illisible".into());
                    t.erreur = Some(e);
                }
            }
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

/// PDF à rendre : la source telle quelle, ou `imprime.pdf` agrandi avec ses repères.
async fn preparer_impression(rep: &std::path::Path, choix: reperage::Reperage) -> Result<(PathBuf, Option<reperage::Mise>), String> {
    let source = rep.join(Format::Pdf.fichier());
    if choix == reperage::Reperage::Aucun {
        return Ok((source, None));
    }
    let pdf = tokio::fs::read(&source).await.map_err(|e| e.to_string())?;
    let (imprime, mise) = tokio::task::spawn_blocking(move || reperage::envelopper(&pdf, choix)).await.map_err(|e| e.to_string())??;
    let chemin = rep.join("imprime.pdf");
    tokio::fs::write(&chemin, imprime).await.map_err(|e| e.to_string())?;
    Ok((chemin, Some(mise)))
}

/// Extrait les tracés à découper ; quand il y en a, écrit `prod/decoupe.plt` (HP-GL)
/// et `apercu/decoupe.svg`. Avec un repérage, les tracés suivent le décalage de l'image.
async fn preparer_decoupe(rep: &std::path::Path, format: Format, tous: bool, mise: Option<&reperage::Mise>) -> Result<usize, String> {
    let source = tokio::fs::read(rep.join(format.fichier())).await.map_err(|e| e.to_string())?;
    let d = tokio::task::spawn_blocking(move || match format {
        Format::Pdf => decoupe::extraire(&source, tous),
        Format::Svg => decoupe::extraire_svg(&source, tous),
    })
    .await
    .map_err(|e| e.to_string())??;
    let mut d = d;
    if let Some(m) = mise {
        d.decaler(m.decalage, m.page);
    }
    if !d.traces.is_empty() {
        for sous in ["prod", "apercu"] {
            tokio::fs::create_dir_all(rep.join(sous)).await.map_err(|e| e.to_string())?;
        }
        tokio::fs::write(rep.join("prod").join("decoupe.plt"), decoupe::hpgl(&d.traces)).await.map_err(|e| e.to_string())?;
        tokio::fs::write(rep.join("apercu").join("decoupe.svg"), decoupe::apercu_svg(&d)).await.map_err(|e| e.to_string())?;
    }
    Ok(d.traces.len())
}

fn lire_travail(donnees: &std::path::Path, id: Uuid) -> Option<Travail> {
    serde_json::from_slice(&std::fs::read(dossier(donnees, id).join("job.json")).ok()?).ok()
}

/// Envoie `prod/decoupe.plt` au plotter ; l'état de l'envoi est suivi dans le travail.
async fn decouper(State(etat): State<Partage>, Chemin(id): Chemin<Uuid>) -> Response {
    let donnees = &etat.config.donnees;
    let Some(mut t) = lire_travail(donnees, id) else { return refus(StatusCode::NOT_FOUND, "introuvable", None) };
    if t.traces_decoupe == 0 {
        return refus(StatusCode::CONFLICT, "rien_a_decouper", None);
    }
    // ponytail: le contrôle puis l'écriture ne sont pas atomiques ; deux clics dans la même
    // milliseconde enverraient deux fois. Le sémaphore les fait au moins passer l'un après l'autre.
    if t.envoi_decoupe.as_ref().is_some_and(|e| matches!(e.statut, Statut::EnAttente | Statut::EnCours)) {
        return refus(StatusCode::CONFLICT, "envoi_en_cours", None);
    }
    t.envoi_decoupe = Some(EnvoiDecoupe { statut: Statut::EnAttente, code_erreur: None, erreur: None, le: maintenant() });
    if let Err(e) = ecrire(donnees, &t) {
        return refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string()));
    }
    tokio::spawn(envoyer_au_plotter(etat.clone(), id));
    (StatusCode::ACCEPTED, Json(t)).into_response()
}

async fn envoyer_au_plotter(etat: Partage, id: Uuid) {
    let _jeton = etat.plotter.acquire().await.unwrap();
    let donnees = &etat.config.donnees;
    let maj = |statut: Statut, detail: Option<String>| {
        if let Some(mut t) = lire_travail(donnees, id) {
            let code_erreur = matches!(statut, Statut::Echec).then_some("envoi_decoupe_echoue".to_string());
            t.envoi_decoupe = Some(EnvoiDecoupe { statut, code_erreur, erreur: detail, le: maintenant() });
            let _ = ecrire(donnees, &t);
        }
    };
    maj(Statut::EnCours, None);
    let plt = dossier(donnees, id).join("prod").join("decoupe.plt");
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

async fn apercu_decoupe(State(etat): State<Partage>, Chemin(id): Chemin<Uuid>) -> Response {
    match tokio::fs::read(dossier(&etat.config.donnees, id).join("apercu").join("decoupe.svg")).await {
        Ok(o) => ([(header::CONTENT_TYPE, "image/svg+xml")], o).into_response(),
        Err(_) => refus(StatusCode::NOT_FOUND, "introuvable", None),
    }
}

async fn fichier_decoupe(State(etat): State<Partage>, Chemin(id): Chemin<Uuid>) -> Response {
    let f = dossier(&etat.config.donnees, id).join("prod").join("decoupe.plt");
    match tokio::fs::read(f).await {
        Ok(o) => (
            [(header::CONTENT_TYPE, "application/vnd.hp-hpgl"), (header::CONTENT_DISPOSITION, "attachment; filename=\"decoupe.plt\"")],
            o,
        )
            .into_response(),
        Err(_) => refus(StatusCode::NOT_FOUND, "introuvable", None),
    }
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
