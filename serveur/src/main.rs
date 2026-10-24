//! E-KO RIP Center — serveur local : reçoit les PDF des postes clients, les rend en
//! plaques d'encre et les garde prêts pour les machines branchées sur le serveur.

mod acces;
mod bat;
mod blanc;
mod couleur;
mod controle;
mod couverture;
mod decoupe;
mod edition;
mod erp;
mod etat;
mod epson;
// Branché sur la file par l'item 2.4 (envoi TCP 9100).
#[cfg_attr(not(test), allow(dead_code))]
mod escp2;
mod machines;
mod media;
mod file;
mod forme;
mod imbrication;
mod mosaique;
mod plotter;
mod render;
mod reglages;
mod releves;
mod reperage;
mod sauvegarde;
mod supports;

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
use reglages::{Demande, Mode, Mosaique};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Statut {
    EnAttente,
    EnCours,
    Termine,
    Echec,
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
    #[serde(default)]
    format: Format,
    /// Réglages du travail, à plat dans le JSON (`mode`, `dpi`, `reperage`…).
    #[serde(flatten)]
    demande: Demande,
    statut: Statut,
    /// Code stable traduit par l'écran (`rendu_echoue`, `interrompu`).
    #[serde(default)]
    code_erreur: Option<String>,
    /// Détail technique, non traduit (message de Ghostscript…).
    erreur: Option<String>,
    /// Pages rendues, dans l'ordre du document.
    #[serde(default)]
    pages: Vec<Page>,
    /// Pré-vérification du fichier reçu (polices, définition des images).
    #[serde(default)]
    alertes: Vec<controle::Alerte>,
    /// Secondes depuis l'époque Unix.
    cree_le: u64,
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct Page {
    /// Plaques de la page (vide en découpe seule).
    plaques: Vec<String>,
    /// Parmi elles, les tons de découpe (`CutContour`…) : jamais envoyés à l'imprimante.
    #[serde(default)]
    plaques_decoupe: Vec<String>,
    /// Tracés de découpe de la page (0 : rien à découper).
    traces_decoupe: usize,
    /// Surface imprimée de la page, en m² (0 en découpe seule).
    #[serde(default)]
    surface_m2: f64,
    /// Part d'encre de chaque plaque, de 0 à 1.
    #[serde(default)]
    couverture: std::collections::BTreeMap<String, f64>,
    /// Format de la page imprimée, en mm (0 : inconnu, travaux d'avant la 0.9).
    #[serde(default)]
    largeur_mm: f64,
    #[serde(default)]
    hauteur_mm: f64,
    /// Cadre Mutoh imprimé : coins au milieu des traits (x1, y1, x2, y2), en points.
    #[serde(default)]
    cadre_pt: Option<[f64; 4]>,
}


fn maintenant() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
}

#[derive(Clone)]
struct Config {
    donnees: PathBuf,
    gs: String,
    max_octets: usize,
    delai: Duration,
    /// Profils ICC déposés depuis l'écran.
    profils: PathBuf,
    /// Durée de conservation des travaux, en jours (0 : jamais purgés).
    conservation_jours: u64,
    /// Espace libre à garder sur le disque des données, en octets (0 : aucun seuil) : en
    /// dessous, les plus vieux travaux inactifs sont purgés et un nouveau travail qui ne
    /// tiendrait pas est refusé (roadmap 0.6).
    seuil_libre: u64,
    /// Port du plotter (le Kona : `/dev/kona`, créé par la règle udev du paquet).
    plotter: PathBuf,
    plotter_stty: String,
    /// Adresse de l'Epson pour lire le média chargé par SNMP (aucune : pas de lecture).
    epson_snmp: Option<String>,
    /// Interface complète aussi dans un navigateur (développement) ; sinon clients seulement.
    acces_navigateur: bool,
}

struct Etat {
    config: Config,
    // ponytail: un seul rendu à la fois sur le serveur ; un sémaphore par machine quand
    // plusieurs imprimantes tourneront en parallèle.
    rendu: Semaphore,
    /// File des sorties vers les machines, gardée dans `file.json` à chaque changement.
    file: std::sync::Mutex<file::File>,
    /// Écritures de `reglages.json` l'une après l'autre.
    reglages: tokio::sync::Mutex<()>,
    sessions: std::sync::Mutex<acces::Sessions>,
    /// Catalogue des machines (`machines.json`), modifiable depuis le client.
    machines: std::sync::RwLock<Vec<machines::Machine>>,
}

type Partage = Arc<Etat>;

fn env_ou(nom: &str, defaut: &str) -> String {
    std::env::var(nom).unwrap_or_else(|_| defaut.to_string())
}

#[tokio::main]
async fn main() {
    let donnees = PathBuf::from(env_ou("EKO_RIP_DATA", "eko-rip-data"));
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("utilisateur") {
        std::process::exit(commande_utilisateur(&donnees, &args[1..]));
    }
    if args.first().map(String::as_str) == Some("erp") {
        std::process::exit(commande_erp(&donnees, args.get(1).map(String::as_str)));
    }
    let config = Config {
        profils: donnees.join("profils"),
        donnees,
        gs: env_ou("EKO_RIP_GS", if cfg!(windows) { "gswin64c" } else { "gs" }),
        max_octets: env_ou("EKO_RIP_MAX_MO", "2048").parse::<usize>().expect("EKO_RIP_MAX_MO") * 1024 * 1024,
        delai: Duration::from_secs(env_ou("EKO_RIP_DELAI_S", "1800").parse().expect("EKO_RIP_DELAI_S")),
        conservation_jours: env_ou("EKO_RIP_CONSERVATION_JOURS", "30").parse().expect("EKO_RIP_CONSERVATION_JOURS"),
        seuil_libre: env_ou("EKO_RIP_SEUIL_LIBRE_MO", "5120").parse::<u64>().expect("EKO_RIP_SEUIL_LIBRE_MO") * 1024 * 1024,
        plotter: env_ou("EKO_RIP_PLOTTER", "/dev/kona").into(),
        plotter_stty: env_ou("EKO_RIP_PLOTTER_STTY", plotter::STTY_DEFAUT),
        epson_snmp: std::env::var("EKO_RIP_EPSON_SNMP").ok().filter(|v| !v.is_empty()),
        acces_navigateur: env_ou("EKO_RIP_ACCES_NAVIGATEUR", "0") == "1",
    };
    let adresse = env_ou("EKO_RIP_ADDR", "0.0.0.0:8640");
    std::fs::create_dir_all(config.donnees.join("travaux")).expect("dossier de données");
    std::fs::create_dir_all(&config.profils).expect("dossier des profils");
    reprendre_travaux_interrompus(&config.donnees);

    let max = config.max_octets;
    let file = file::File::lire(&config.donnees);
    file.ecrire(&config.donnees).expect("file d'attente");
    let mut catalogue = machines::lire(&config.donnees).unwrap_or_else(|| machines::catalogue_initial(&config.plotter, &config.plotter_stty, config.epson_snmp.clone()));
    // Identifiants immuables (pont ERP) : attribués une fois, puis gardés.
    let avant = catalogue.clone();
    machines::identifier(&mut catalogue, &avant).expect("identifiants des machines");
    if catalogue != avant {
        machines::ecrire(&config.donnees, &catalogue).expect("machines.json");
    }
    let mut liste_reglages = reglages::lire(&config.donnees);
    if liste_reglages.iter().any(|r| r.uuid.is_none()) {
        reglages::identifier(&mut liste_reglages, &[]);
        reglages::ecrire(&config.donnees, &liste_reglages).expect("reglages.json");
    }
    // Dossiers attendus (dossiers surveillés, sorties des machines sans pilote) : un dossier
    // supprimé pendant l'arrêt est recréé et signalé, sans bloquer le démarrage.
    for r in &liste_reglages {
        etat::assurer_dossier(&reglages::dossier_entree(&config.donnees, &r.nom));
    }
    for m in catalogue.iter().filter(|m| m.connexion == machines::Connexion::Dossier) {
        etat::assurer_dossier(&config.donnees.join("sorties").join(&m.id));
    }
    let etat: Partage = Arc::new(Etat {
        config,
        rendu: Semaphore::new(1),
        file: std::sync::Mutex::new(file),
        reglages: tokio::sync::Mutex::new(()),
        sessions: Default::default(),
        machines: std::sync::RwLock::new(catalogue),
    });
    tokio::spawn(repartir(etat.clone()));
    if acces::libre(&etat.config.donnees) {
        eprintln!("ATTENTION : aucun utilisateur déclaré, l'accès est libre. Créer un compte : eko-rip-serveur utilisateur ajouter <nom>");
    }
    tokio::spawn(surveiller(etat.clone()));
    tokio::spawn(purger(etat.clone()));
    tokio::spawn(relever(etat.clone()));
    let app = routes(etat).layer(DefaultBodyLimit::max(max));

    let ecoute = tokio::net::TcpListener::bind(&adresse).await.expect("adresse d'écoute");
    println!("E-KO RIP Center à l'écoute sur http://{adresse}");
    axum::serve(ecoute, app).await.unwrap();
}

fn routes(etat: Partage) -> Router {
    Router::new()
        .route("/", get(accueil))
        .route("/api/travaux", get(lister).post(envoyer))
        .route("/api/profils", get(lister_profils).post(deposer_profil))
        .route("/api/planches", axum::routing::post(creer_planche))
        .route("/api/planches/apercu", axum::routing::post(apercu_planche))
        .route("/api/systeme", get(systeme))
        .route("/api/machines", get(lister_machines).post(enregistrer_machines))
        .route("/api/machines/{id}/media", get(media_machine))
        .route("/api/machines/{id}/consommation", get(consommation_machine))
        .route("/api/machines/{id}/supports", get(lister_supports).post(enregistrer_supports))
        .route("/api/machines/{id}/charge", get(lire_charge).post(saisir_charge))
        .route("/api/sauvegarde", get(exporter_config).post(restaurer_config))
        .route("/api/reglages", get(lister_reglages).post(enregistrer_reglage))
        .route("/api/reglages/{nom}", axum::routing::delete(supprimer_reglage))
        .route("/api/travaux/{id}", axum::routing::delete(supprimer))
        .route("/api/travaux/{id}/relancer", axum::routing::post(relancer))
        .route("/api/travaux/{id}/editer", axum::routing::post(editer))
        .route("/api/travaux/{id}/reglages", axum::routing::post(modifier_reglages))
        .route("/api/travaux/{id}/bat.pdf", get(bat_pdf))
        .route("/api/travaux/{id}/pages/{page}/apercu.png", get(apercu))
        .route("/api/travaux/{id}/pages/{page}/plaques/{nom}", get(plaque))
        .route("/api/travaux/{id}/pages/{page}/decoupe.plt", get(fichier_decoupe))
        .route("/api/travaux/{id}/pages/{page}/decoupe.svg", get(apercu_decoupe))
        .route("/api/tarifs", get(lire_tarifs).post(enregistrer_tarifs))
        .route("/api/rapports/consommation.csv", get(rapport_consommation))
        .route("/api/file", get(lister_file).post(mettre_en_file))
        .route("/api/file/{id}", axum::routing::delete(retirer_de_la_file))
        .route("/api/file/{id}/{action}", axum::routing::post(agir_sur_la_file))
        .route("/api/erp/contrat", get(contrat_erp))
        .route("/api/session", get(lire_session).post(ouvrir_session).delete(fermer_session))
        .layer(axum::middleware::from_fn_with_state(etat.clone(), garder))
        .with_state(etat)
}

fn dossier(donnees: &std::path::Path, id: Uuid) -> PathBuf {
    donnees.join("travaux").join(id.to_string())
}

fn ecrire(donnees: &std::path::Path, t: &Travail) -> std::io::Result<()> {
    etat::ecrire(&dossier(donnees, t.id).join("job.json"), t, false)
}

fn lire_tous(donnees: &std::path::Path) -> Vec<Travail> {
    let Ok(entrees) = std::fs::read_dir(donnees.join("travaux")) else { return vec![] };
    let mut v: Vec<Travail> = entrees
        .filter_map(|e| etat::lire(&e.ok()?.path().join("job.json")).ok().flatten())
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

/// Travaux, avec pour chaque page l'estimation d'encre et de coût aux tarifs du moment.
async fn lister(State(etat): State<Partage>) -> Json<Vec<serde_json::Value>> {
    let tarifs = couverture::lire(&etat.config.donnees);
    let imprimante = etat.machines.read().unwrap().iter().find(|m| m.fait(machines::Operation::Impression)).cloned();
    Json(
        lire_tous(&etat.config.donnees)
            .into_iter()
            .map(|t| {
                let estimations: Vec<_> = t.pages.iter().map(|p| couverture::estimer(p.surface_m2, p.hauteur_mm, &p.couverture, &p.plaques_decoupe, &tarifs, t.demande.support.as_deref())).collect();
                let prix_de_revient: f64 = estimations.iter().map(|e| e.total).sum();
                let mut v = serde_json::to_value(&t).unwrap();
                for (page, e) in v["pages"].as_array_mut().into_iter().flatten().zip(estimations) {
                    page["estimation"] = serde_json::to_value(e).unwrap();
                }
                v["prix_de_revient"] = serde_json::json!(prix_de_revient);
                // Durée sur la première imprimante du catalogue, toutes pages.
                let duree: Option<f64> = imprimante.as_ref().filter(|_| t.demande.mode == Mode::Imprimer).and_then(|m| t.pages.iter().map(|p| m.duree_impression_min(p.hauteur_mm, t.demande.dpi)).sum());
                v["duree_impression_min"] = serde_json::json!(duree);
                v
            })
            .collect(),
    )
}

async fn lire_tarifs(State(etat): State<Partage>) -> Json<couverture::Tarifs> {
    Json(couverture::lire(&etat.config.donnees))
}

async fn enregistrer_tarifs(State(etat): State<Partage>, Json(t): Json<couverture::Tarifs>) -> Response {
    if let Err(code) = t.verifier() {
        return refus(StatusCode::BAD_REQUEST, code, None);
    }
    match couverture::ecrire(&etat.config.donnees, &t) {
        Ok(()) => Json(t).into_response(),
        Err(e) => refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string())),
    }
}

/// `2026-10-24 03:21:00` (UTC) depuis des secondes Unix (algorithme civil de H. Hinnant).
fn date_utc(s: u64) -> String {
    let (jours, reste) = ((s / 86_400) as i64, s % 86_400);
    let z = jours + 719_468;
    let ere = z.div_euclid(146_097);
    let jde = z - ere * 146_097;
    let ade = (jde - jde / 1460 + jde / 36_524 - jde / 146_096) / 365;
    let jda = jde - (365 * ade + ade / 4 - ade / 100);
    let mp = (5 * jda + 2) / 153;
    let (j, m) = (jda - (153 * mp + 2) / 5 + 1, if mp < 10 { mp + 3 } else { mp - 9 });
    let a = ade + ere * 400 + i64::from(m <= 2);
    format!("{a:04}-{m:02}-{j:02} {:02}:{:02}:{:02}", reste / 3600, reste % 3600 / 60, reste % 60)
}

/// Consommation par page imprimée, pour un tableur ou l'ERP : séparateur `;`, décimales
/// avec un point, une colonne de ml par encre rencontrée.
async fn rapport_consommation(State(etat): State<Partage>) -> Response {
    let tarifs = couverture::lire(&etat.config.donnees);
    let travaux = lire_tous(&etat.config.donnees);
    let mut encres = std::collections::BTreeSet::new();
    let mut lignes = vec![];
    for t in travaux.iter().filter(|t| matches!(t.statut, Statut::Termine) && t.demande.mode == Mode::Imprimer) {
        for (i, p) in t.pages.iter().enumerate() {
            let e = couverture::estimer(p.surface_m2, p.hauteur_mm, &p.couverture, &p.plaques_decoupe, &tarifs, t.demande.support.as_deref());
            encres.extend(e.encres.iter().map(|l| l.nom.clone()));
            lignes.push((t, i + 1, e));
        }
    }
    let csv_texte = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));
    let mut csv = String::from("date;travail;id;page;support;surface_m2;surface_support_m2");
    for n in &encres {
        csv.push_str(&format!(";{}", csv_texte(&format!("{n}_ml"))));
    }
    csv.push_str(";encre_ml;cout_encres;cout_support;total;base\n");
    for (t, page, e) in lignes {
        csv.push_str(&format!("{};{};{};{page};{};{:.4};{:.4}", date_utc(t.cree_le), csv_texte(&t.nom), t.id, csv_texte(e.support.as_deref().unwrap_or("")), e.surface_m2, e.surface_support_m2));
        for n in &encres {
            let ml = e.encres.iter().find(|l| &l.nom == n).map_or(0.0, |l| l.ml);
            csv.push_str(&format!(";{ml:.2}"));
        }
        csv.push_str(&format!(";{:.2};{:.4};{:.4};{:.4};{}\n", e.ml, e.cout_encres, e.cout_support, e.total, if e.ttc { "TTC" } else { "HT" }));
    }
    ([(header::CONTENT_TYPE, "text/csv; charset=utf-8"), (header::CONTENT_DISPOSITION, "attachment; filename=\"consommation.csv\"")], csv).into_response()
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
    let (mut nom, mut format) = (String::new(), None);
    let mut d = Demande::default();
    let (mut mosaique, mut laize_mm, mut recouvrement_mm, mut reperes_les) = (false, 1600.0f64, 20.0f64, false);

    loop {
        let champ = match form.next_field().await {
            Ok(Some(c)) => c,
            Ok(None) => break,
            Err(e) => return abandon(&rep, StatusCode::BAD_REQUEST, ("envoi_illisible", Some(e.to_string()))).await,
        };
        let Some(nom_champ) = champ.name().map(str::to_string) else { continue };
        if nom_champ == "fichier" {
            nom = champ.file_name().unwrap_or("sans nom").chars().take(200).collect();
            match enregistrer(champ, &provisoire).await {
                Ok(f) => format = Some(f),
                Err(r) => return abandon(&rep, StatusCode::BAD_REQUEST, r).await,
            }
            continue;
        }
        let valeur = champ.text().await.unwrap_or_default().trim().to_string();
        let erreur = |code: &'static str| (code, None);
        let r: Result<(), Refus> = match nom_champ.as_str() {
            "dpi" => valeur.parse().map(|v| d.dpi = v).map_err(|_| erreur("dpi_invalide")),
            "mode" => match valeur.as_str() {
                "imprimer" => {
                    d.mode = Mode::Imprimer;
                    Ok(())
                }
                "decouper" => {
                    d.mode = Mode::Decouper;
                    Ok(())
                }
                _ => Err(erreur("mode_invalide")),
            },
            "traces" => match valeur.as_str() {
                "tons" => {
                    d.tous_les_traces = false;
                    Ok(())
                }
                "tous" => {
                    d.tous_les_traces = true;
                    Ok(())
                }
                _ => Err(erreur("mode_invalide")),
            },
            "reperage" => match valeur.as_str() {
                "aucun" => {
                    d.reperage = reperage::Reperage::Aucun;
                    Ok(())
                }
                "manuel" => {
                    d.reperage = reperage::Reperage::Manuel;
                    Ok(())
                }
                "cadre" => {
                    d.reperage = reperage::Reperage::Cadre;
                    Ok(())
                }
                "cadre_code" => {
                    d.reperage = reperage::Reperage::CadreCode;
                    Ok(())
                }
                "cadre_multi" => {
                    d.reperage = reperage::Reperage::CadreMulti;
                    Ok(())
                }
                _ => Err(erreur("mode_invalide")),
            },
            "profil_cmjn" => {
                d.couleur.entree_cmjn = (!valeur.is_empty()).then_some(valeur);
                Ok(())
            }
            "profil_rvb" => {
                d.couleur.entree_rvb = (!valeur.is_empty()).then_some(valeur);
                Ok(())
            }
            "profil_sortie" => {
                d.couleur.sortie = (!valeur.is_empty()).then_some(valeur);
                Ok(())
            }
            "intention" => valeur.parse().map(|v| d.couleur.intention = v).map_err(|_| erreur("profil_inadapte")),
            "forcer_profils" => {
                    d.couleur.forcer = valeur == "oui";
                    Ok(())
                }
            "mosaique" => {
                    mosaique = valeur == "oui";
                    Ok(())
                }
            "laize_mm" => valeur.parse().map(|v| laize_mm = v).map_err(|_| erreur("mosaique_invalide")),
            "reperes_les" => {
                reperes_les = valeur == "oui";
                Ok(())
            }
            "recouvrement_mm" => valeur.parse().map(|v| recouvrement_mm = v).map_err(|_| erreur("mosaique_invalide")),
            "edition" => serde_json::from_str(&valeur).map(|e| d.edition = e).map_err(|_| erreur("edition_invalide")),
            "auto_imprimer" => {
                d.auto.imprimer = valeur == "oui";
                Ok(())
            }
            "auto_decouper" => {
                d.auto.decouper = valeur == "oui";
                Ok(())
            }
            "auto_suspendre" => {
                d.auto.suspendre = valeur == "oui";
                Ok(())
            }
            "blanc" => serde_json::from_value(serde_json::Value::String(valeur)).map(|m| d.blanc.mode = m).map_err(|_| erreur("blanc_invalide")),
            "blanc_retraction_mm" => valeur.parse().map(|v| d.blanc.retraction_mm = v).map_err(|_| erreur("blanc_invalide")),
            "support" => {
                d.support = Some(valeur).filter(|v| !v.is_empty());
                Ok(())
            }
            "blanc_seuil" => valeur.parse().map(|v| d.blanc.seuil = v).map_err(|_| erreur("blanc_invalide")),
            _ => Ok(()),
        };
        if let Err(r) = r {
            return abandon(&rep, StatusCode::BAD_REQUEST, r).await;
        }
    }
    let Some(format) = format else {
        return abandon(&rep, StatusCode::BAD_REQUEST, ("aucun_fichier", None)).await;
    };
    d.mosaique = mosaique.then_some(Mosaique { laize_mm, recouvrement_mm, reperes: reperes_les });
    match creer_travail(&etat, id, nom, format, d).await {
        Ok(t) => (StatusCode::ACCEPTED, Json(t)).into_response(),
        Err((statut, r)) => abandon(&rep, statut, r).await,
    }
}

/// Crée un travail à partir de `source.tmp`, déjà écrit dans son dossier : contrôles,
/// mosaïque, `job.json`, puis traitement en tâche de fond. Commun à l'écran, aux planches
/// et aux dossiers surveillés. En cas de refus, le dossier reste à effacer par l'appelant.
async fn creer_travail(etat: &Partage, id: Uuid, nom: String, format: Format, d: Demande) -> Result<Travail, (StatusCode, Refus)> {
    let rep = dossier(&etat.config.donnees, id);
    let mauvais = |code: &'static str, detail: Option<String>| (StatusCode::BAD_REQUEST, (code, detail));
    let interne = |e: String| (StatusCode::INTERNAL_SERVER_ERROR, ("erreur_interne", Some(e)));
    let d = d.normaliser();
    // Le rendu en plaques passe par Ghostscript, qui ne lit pas le SVG.
    if format == Format::Svg && d.mode == Mode::Imprimer {
        return Err(mauvais("svg_decoupe_seulement", None));
    }
    let mut d = d;
    supports::appliquer(&mut d, &supports::lire(&etat.config.donnees, "epson")).map_err(|code| mauvais(code, None))?;
    d.verifier(&couleur::lister(&etat.config.profils)).map_err(|code| mauvais(code, None))?;
    tokio::fs::rename(rep.join("source.tmp"), rep.join(format.fichier())).await.map_err(|e| interne(e.to_string()))?;
    // CropBox et /Rotate ramenés à la MediaBox, une fois pour toutes : le rendu et la découpe
    // voient alors la même page. Un PDF que lopdf ne lit pas reste tel quel ; la découpe
    // (qui le lit aussi) le signalera.
    if format == Format::Pdf {
        let chemin = rep.join(format.fichier());
        let pdf = tokio::fs::read(&chemin).await.map_err(|e| interne(e.to_string()))?;
        if let Ok(Ok(Some(normalise))) = tokio::task::spawn_blocking(move || imbrication::normaliser_pages(&pdf)).await {
            tokio::fs::write(&chemin, normalise).await.map_err(|e| interne(e.to_string()))?;
        }
    }
    // Refus anticipé si le rendu ne tiendrait pas au-dessus du seuil d'espace libre.
    if format == Format::Pdf && d.mode == Mode::Imprimer && etat.config.seuil_libre > 0 {
        let pdf = tokio::fs::read(rep.join(format.fichier())).await.map_err(|e| interne(e.to_string()))?;
        let surface = (1..).map_while(|n| imbrication::taille_page(&pdf, n).ok()).map(|t| t.largeur * t.hauteur / (imbrication::PT_PAR_MM * imbrication::PT_PAR_MM) / 1e6).sum::<f64>();
        let besoin = place_estimee(surface, d.dpi);
        if let Some((_, libre)) = espace_disque(&etat.config.donnees).await
            && libre < etat.config.seuil_libre.saturating_add(besoin)
        {
            return Err((StatusCode::INSUFFICIENT_STORAGE, ("espace_insuffisant", Some(format!("≈ {} Mo nécessaires, {} Mo libres, seuil {} Mo", besoin >> 20, libre >> 20, etat.config.seuil_libre >> 20)))));
        }
    }
    let alertes = if format == Format::Pdf && d.mode == Mode::Imprimer {
        let pdf = tokio::fs::read(rep.join(format.fichier())).await.map_err(|e| interne(e.to_string()))?;
        tokio::task::spawn_blocking(move || controle::verifier(&pdf)).await.unwrap_or_default()
    } else {
        vec![]
    };
    let mosaique = d.mosaique.filter(|_| d.mode == Mode::Imprimer && format == Format::Pdf);
    let mut alertes = alertes;
    let tourne = construire_source(&rep, format, &Demande { mosaique, ..d.clone() }, laize_imprimante(etat)).await.map_err(|(code, e)| if code == "erreur_interne" { interne(e) } else { mauvais(code, Some(e)) })?;
    alertes.extend(tourne);
    let travail = Travail {
        id,
        nom,
        format,
        demande: Demande { mosaique, ..d },
        statut: Statut::EnAttente,
        code_erreur: None,
        erreur: None,
        pages: vec![],
        alertes,
        cree_le: maintenant(),
    };
    ecrire(&etat.config.donnees, &travail).map_err(|e| interne(e.to_string()))?;
    tokio::spawn(traiter(etat.clone(), travail.clone()));
    Ok(travail)
}

/// `source.pdf` à rendre, depuis l'original : édition rapide puis mosaïque. L'original est
/// gardé dans `source-original.pdf` dès qu'une transformation s'applique, pour pouvoir
/// rééditer sans renvoyer le fichier. Erreur : (code, détail).
async fn construire_source(rep: &std::path::Path, format: Format, d: &Demande, laize_mm: Option<f64>) -> Result<Option<controle::Alerte>, (&'static str, String)> {
    let interne = |e: std::io::Error| ("erreur_interne", e.to_string());
    if format != Format::Pdf {
        return Ok(None);
    }
    let (source, original) = (rep.join(Format::Pdf.fichier()), rep.join("source-original.pdf"));
    let (mut edition, mosaique, reperage) = (d.edition, d.mosaique, d.reperage);
    let a_original = tokio::fs::try_exists(&original).await.unwrap_or(false);
    let pdf = tokio::fs::read(if a_original { &original } else { &source }).await.map_err(interne)?;
    // Rotation automatique : décidée sur la page éditée (échelle, marge…), repérage compris.
    let auto = (d.rotation_auto && d.mode == Mode::Imprimer && edition.rotation == 0 && mosaique.is_none()).then_some(laize_mm).flatten();
    let tourne = match auto {
        Some(laize) => {
            let copie = pdf.clone();
            tokio::task::spawn_blocking(move || -> Result<Option<controle::Alerte>, (&'static str, String)> {
                let edite = if edition.neutre() { copie } else { edition::appliquer(&copie, &edition).map_err(|e| ("edition_invalide", e))? };
                Ok(trop_large(&edite, reperage, laize))
            })
            .await
            .map_err(|e| ("erreur_interne", e.to_string()))??
        }
        None => None,
    };
    if tourne.is_some() {
        edition.rotation = 90;
    }
    if edition.neutre() && mosaique.is_none() {
        if a_original {
            tokio::fs::copy(&original, &source).await.map_err(interne)?;
        }
        return Ok(None);
    }
    if !a_original {
        tokio::fs::copy(&source, &original).await.map_err(interne)?;
    }
    let produit = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, (&'static str, String)> {
        let pdf = if edition.neutre() { pdf } else { edition::appliquer(&pdf, &edition).map_err(|e| ("edition_invalide", e))? };
        match mosaique {
            Some(m) => mosaique::composer(&pdf, m.laize_mm, m.recouvrement_mm, m.reperes).map_err(|e| ("mosaique_invalide", e)),
            None => Ok(pdf),
        }
    })
    .await
    .map_err(|e| ("erreur_interne", e.to_string()))??;
    tokio::fs::write(&source, produit).await.map_err(interne)?;
    Ok(tourne)
}

/// Une page plus large que la laize (repérage compris) qui y tient tournée de 90° : rend
/// l'alerte à joindre au travail. Toutes les pages doivent tenir une fois tournées.
fn trop_large(pdf: &[u8], reperage: reperage::Reperage, laize_mm: f64) -> Option<controle::Alerte> {
    let n = lopdf::Document::load_mem(pdf).ok()?.get_pages().len();
    let mm = imbrication::PT_PAR_MM;
    let tailles: Vec<imbrication::Taille> = (1..=n).map(|p| imbrication::taille_page(pdf, p)).collect::<Result<_, _>>().ok()?;
    let largeur = |l: f64, h: f64| reperage::mise_en_page(reperage, l, h, 0).page.0 / mm;
    let trop = tailles.iter().find(|t| largeur(t.largeur, t.hauteur) > laize_mm + 0.01)?;
    tailles.iter().all(|t| largeur(t.hauteur, t.largeur) <= laize_mm + 0.01).then(|| controle::Alerte {
        code: "rotation_auto".into(),
        page: None,
        detail: format!("{:.0} mm > {:.0} mm", largeur(trop.largeur, trop.hauteur), laize_mm),
    })
}

/// Laize de la première imprimante du catalogue (celle de la rotation automatique).
fn laize_imprimante(etat: &Etat) -> Option<f64> {
    etat.machines.read().unwrap().iter().find(|m| m.fait(machines::Operation::Impression)).map(|m| m.laize_mm)
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
    let rendu = match t.demande.mode {
        Mode::Imprimer => match preparer_impression(&rep, t.demande.reperage, t.id).await {
            Ok((pdf, mises)) => {
                let options = couleur::options_ghostscript(&t.demande.couleur, &etat.config.profils);
                match render::rendre(&etat.config.gs, &pdf, &rep, t.demande.dpi, &options, etat.config.delai).await {
                    Ok(r) => ajouter_blanc(&rep, r.pages, t.demande.blanc, t.demande.dpi).await.map(|p| (Some(p), mises)),
                    Err(e) => Err(e),
                }
            }
            Err(e) => Err(e),
        },
        Mode::Decouper => Ok((None, vec![])),
    };
    match rendu {
        Ok((plaques, mises)) => match preparer_decoupe(&rep, t.format, t.demande.tous_les_traces, &mises).await {
            Ok(traces) => {
                // En découpe seule, il n'y a pas de plaques : une page par jeu de tracés.
                let plaques = plaques.unwrap_or_else(|| vec![vec![]; traces.len()]);
                t.pages = plaques
                    .into_iter()
                    .enumerate()
                    .map(|(i, plaques)| {
                        let apercus: Vec<_> = plaques.iter().map(|p| (p.clone(), render::chemin_apercu_plaque(&rep, i + 1, p))).collect();
                        // Une mesure impossible n'empêche pas d'imprimer : le coût reste à zéro.
                        let mesure = couverture::mesurer(&apercus, render::DPI_APERCU as f64).unwrap_or_else(|e| {
                            eprintln!("travail {} : couverture de la page {} : {e}", t.id, i + 1);
                            couverture::Mesure::default()
                        });
                        Page {
                            plaques_decoupe: plaques.iter().filter(|p| decoupe::est_ton_de_decoupe(p)).cloned().collect(),
                            plaques,
                            traces_decoupe: traces.get(i).copied().unwrap_or(0),
                            surface_m2: mesure.surface_m2,
                            couverture: mesure.couverture,
                            largeur_mm: mesure.largeur_mm,
                            hauteur_mm: mesure.hauteur_mm,
                            cadre_pt: mises.get(i).and_then(|m| m.coins),
                        }
                    })
                    .collect();
                t.statut = Statut::Termine;
                mettre_en_file_auto(&etat, &t);
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
    /// Autorise le quart de tour quand il raccourcit la planche.
    #[serde(default)]
    rotation: bool,
    /// Imbrication sur la forme des contours de découpe (sinon en rangées de rectangles).
    #[serde(default)]
    forme: bool,
    /// Step & repeat d'une seule page (colonnes × rangées, demi-saut).
    #[serde(default)]
    motif: Option<imbrication::Motif>,
    elements: Vec<ElementPlanche>,
}

#[derive(Deserialize)]
struct ElementPlanche {
    id: Uuid,
    page: usize,
    copies: usize,
}

impl DemandePlanche {
    fn motif_invalide(&self) -> Option<&'static str> {
        let m = self.motif?;
        let bon = self.elements.len() == 1 && (1..=100).contains(&m.colonnes) && (1..=500).contains(&m.rangees) && m.colonnes * m.rangees <= 2000;
        (!bon).then_some("motif : une seule page, 1 à 100 colonnes, 1 à 500 rangées, 2000 copies au plus")
    }
}

fn espace_defaut() -> f64 {
    5.0
}

fn dpi_defaut() -> u32 {
    720
}

#[derive(Deserialize)]
struct ChoixLangue {
    #[serde(default)]
    langue: Option<String>,
}

/// Bon à tirer du travail : image en épreuve écran du support, découpe superposée,
/// caractéristiques, cadre « Bon pour accord ». `?langue=en` pour l'anglais.
async fn bat_pdf(State(etat): State<Partage>, Chemin(id): Chemin<Uuid>, axum::extract::Query(q): axum::extract::Query<ChoixLangue>) -> Response {
    let donnees = &etat.config.donnees;
    let Some(t) = lire_travail(donnees, id) else { return introuvable() };
    if !matches!(t.statut, Statut::Termine) || t.format != Format::Pdf {
        return refus(StatusCode::CONFLICT, "travail_occupe", None);
    }
    let en = q.langue.as_deref() == Some("en");
    let rep = dossier(donnees, id);
    let tmp = rep.join("bat");
    let _ = tokio::fs::remove_dir_all(&tmp).await;
    if let Err(e) = tokio::fs::create_dir_all(&tmp).await {
        return refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string()));
    }
    let resultat: Result<Vec<u8>, String> = async {
        let source = tokio::fs::read(rep.join(Format::Pdf.fichier())).await.map_err(|e| e.to_string())?;
        let tous = t.demande.tous_les_traces;
        let (neutre, decoupes, tailles) = tokio::task::spawn_blocking(move || -> Result<_, String> {
            let decoupes = decoupe::extraire(&source, tous)?;
            let tailles: Vec<(f64, f64)> = decoupes.iter().map(|d| (d.largeur, d.hauteur)).collect();
            Ok((decoupe::neutraliser(&source)?.0, decoupes, tailles))
        })
        .await
        .map_err(|e| e.to_string())??;
        // Définition : la plus grande page tient en 1800 px, entre 10 et 150 dpi.
        let plus_grand = tailles.iter().map(|(l, h)| l.max(*h)).fold(1.0, f64::max) / 72.0;
        let dpi = (1800.0 / plus_grand).clamp(10.0, 150.0) as u32;
        tokio::fs::write(tmp.join("bat.pdf"), neutre).await.map_err(|e| e.to_string())?;
        let epreuve: Vec<String> = couleur::options_ghostscript(&t.demande.couleur, &etat.config.profils).iter().map(|o| o.replace("-sOutputICCProfile=", "-sProofProfile=")).collect();
        render::ghostscript(&etat.config.gs, &tmp.join("bat.pdf"), "png16m", dpi, &epreuve, &tmp.join("p-%03d.png"), etat.config.delai).await?;
        let mut pages = vec![];
        for (i, d) in decoupes.into_iter().enumerate() {
            let image = image::open(tmp.join(format!("p-{:03}.png", i + 1))).map_err(|e| e.to_string())?.to_rgb8();
            pages.push(bat::PageBat { image, largeur: d.largeur, hauteur: d.hauteur, traces: d.traces });
        }
        let d = &t.demande;
        let oui_non = |b: bool| if b { if en { "yes" } else { "oui" } } else if en { "no" } else { "non" };
        let traces: usize = t.pages.iter().map(|p| p.traces_decoupe).sum();
        let mut lignes = if en {
            vec![
                format!("Mode: {}", if d.mode == Mode::Imprimer { "print" } else { "cut only" }),
                format!("Resolution: {} dpi", d.dpi),
                format!("Media profile: {}", d.couleur.sortie.as_deref().unwrap_or("default")),
                format!("White: {}", match d.blanc.mode { blanc::Mode::Aucun => "file's own", blanc::Mode::SousCouche => "underlay", blanc::Mode::Aplat => "flood" }),
                format!("Scale: {} % · rotation {}° · mirror {}", d.edition.echelle, d.edition.rotation, oui_non(d.edition.miroir)),
                format!("Grommets: {}", oui_non(d.edition.oeillets.is_some())),
                format!("Cut paths: {traces}"),
            ]
        } else {
            vec![
                format!("Mode : {}", if d.mode == Mode::Imprimer { "impression" } else { "découpe seule" }),
                format!("Résolution : {} dpi", d.dpi),
                format!("Profil du support : {}", d.couleur.sortie.as_deref().unwrap_or("par défaut")),
                format!("Blanc : {}", match d.blanc.mode { blanc::Mode::Aucun => "celui du fichier", blanc::Mode::SousCouche => "sous-couche", blanc::Mode::Aplat => "aplat" }),
                format!("Échelle : {} % · rotation {}° · miroir {}", d.edition.echelle, d.edition.rotation, oui_non(d.edition.miroir)),
                format!("Œillets : {}", oui_non(d.edition.oeillets.is_some())),
                format!("Tracés de découpe : {traces}"),
            ]
        };
        for a in &t.alertes {
            lignes.push(format!("{} {}", if en { "Warning:" } else { "Alerte :" }, a.detail));
        }
        let date = date_utc(t.cree_le);
        let textes = bat::Textes {
            titre: format!("{} — {}", if en { "Proof" } else { "Bon à tirer" }, t.nom),
            sous_titre: format!("E-KO RIP Center · {} {} · {}", if en { "job" } else { "travail" }, &t.id.to_string()[..8], &date[..16]),
            lignes,
            page: "Page".into(),
            dimensions: if en { "Size: {l} × {h} mm".into() } else { "Format : {l} × {h} mm".into() },
            accord: if en { "Approved for printing" } else { "Bon pour accord" }.into(),
            signature: if en { "Date, name and signature:" } else { "Date, nom et signature :" }.into(),
            legende_decoupe: if en { "Cut line (not printed)" } else { "Tracé de découpe (non imprimé)" }.into(),
        };
        tokio::task::spawn_blocking(move || bat::composer(&textes, &pages)).await.map_err(|e| e.to_string())?
    }
    .await;
    let _ = tokio::fs::remove_dir_all(&tmp).await;
    match resultat {
        Ok(pdf) => {
            let nom: String = t.nom.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).collect();
            let entete = format!("inline; filename=\"BAT-{}.pdf\"", nom.trim_end_matches("_pdf"));
            ([(header::CONTENT_TYPE, "application/pdf".to_string()), (header::CONTENT_DISPOSITION, entete)], pdf).into_response()
        }
        Err(e) => refus(StatusCode::INTERNAL_SERVER_ERROR, "rendu_echoue", Some(e)),
    }
}

fn machine(etat: &Etat, id: &str) -> Option<machines::Machine> {
    etat.machines.read().unwrap().iter().find(|m| m.id == id).cloned()
}

async fn lister_machines(State(etat): State<Partage>) -> Json<Vec<machines::Machine>> {
    Json(etat.machines.read().unwrap().clone())
}

/// Remplace le catalogue ; une machine retirée garde ses sorties finies dans l'historique.
async fn enregistrer_machines(State(etat): State<Partage>, Json(mut liste): Json<Vec<machines::Machine>>) -> Response {
    let connues = etat.machines.read().unwrap().clone();
    if let Err(code) = machines::identifier(&mut liste, &connues).and(machines::verifier(&liste)) {
        return refus(StatusCode::BAD_REQUEST, code, None);
    }
    let a_passer = etat.file.lock().unwrap().sorties.iter().any(|s| s.active() && !liste.iter().any(|m| m.id == s.machine));
    if a_passer {
        return refus(StatusCode::CONFLICT, "machine_occupee", None);
    }
    if let Err(e) = machines::ecrire(&etat.config.donnees, &liste) {
        return refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string()));
    }
    *etat.machines.write().unwrap() = liste.clone();
    Json(liste).into_response()
}

/// Configuration du serveur en un fichier (machines, tarifs, réglages, profils ICC).
async fn exporter_config(State(etat): State<Partage>) -> Response {
    let mut profils = std::collections::BTreeMap::new();
    for p in couleur::lister(&etat.config.profils) {
        match tokio::fs::read(etat.config.profils.join(&p.fichier)).await {
            Ok(o) => profils.insert(p.fichier, sauvegarde::hex(&o)),
            Err(e) => return refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(format!("{} : {e}", p.fichier))),
        };
    }
    let s = sauvegarde::Sauvegarde {
        format: sauvegarde::FORMAT.into(),
        version: env!("CARGO_PKG_VERSION").into(),
        cree_le: maintenant(),
        machines: etat.machines.read().unwrap().clone(),
        tarifs: couverture::lire(&etat.config.donnees),
        reglages: reglages::lire(&etat.config.donnees),
        profils,
        supports: Some(supports::lire(&etat.config.donnees, "epson")),
    };
    let nom = format!("attachment; filename=\"eko-rip-config-{}.json\"", date_utc(s.cree_le).replace([' ', ':'], "-"));
    ([(header::CONTENT_TYPE, "application/json".to_string()), (header::CONTENT_DISPOSITION, nom)], serde_json::to_vec_pretty(&s).unwrap_or_default()).into_response()
}

/// Remplace machines, tarifs et réglages, ajoute ou remplace les profils. Tout est
/// vérifié avant la première écriture : une sauvegarde fausse ne laisse rien à moitié.
async fn restaurer_config(State(etat): State<Partage>, Json(s): Json<sauvegarde::Sauvegarde>) -> Response {
    if s.format != sauvegarde::FORMAT {
        return refus(StatusCode::BAD_REQUEST, "sauvegarde_invalide", Some(format!("format « {} »", s.format)));
    }
    let mut profils = vec![];
    for (nom, texte) in &s.profils {
        let octets = sauvegarde::depuis_hex(texte);
        match octets.as_deref().filter(|_| couleur::nom_valide(nom)).and_then(|o| couleur::lire_entete(o, nom)) {
            Some(p) => profils.push((p, octets.unwrap_or_default())),
            None => return refus(StatusCode::BAD_REQUEST, "profil_invalide", Some(nom.clone())),
        }
    }
    let mut s = s;
    let connues = etat.machines.read().unwrap().clone();
    // Une sauvegarde porte ses identifiants : ils sont repris tels quels sur un serveur neuf.
    let mut connues_et_sauvees = connues.clone();
    connues_et_sauvees.extend(s.machines.iter().filter(|m| m.uuid.is_some()).cloned());
    if let Err(code) = machines::identifier(&mut s.machines, &connues_et_sauvees).and(machines::verifier(&s.machines)).and(s.tarifs.verifier()) {
        return refus(StatusCode::BAD_REQUEST, code, None);
    }
    if let Some(c) = &s.supports {
        let mut ids: Vec<_> = c.supports.iter().map(|x| x.id).chain(c.supports.iter().flat_map(|x| x.modes.iter().map(|m| m.id))).collect();
        let nb = ids.len();
        ids.sort();
        ids.dedup();
        let par_machine_bons = s.machines.iter().all(|m| supports::verifier(&c.supports.iter().filter(|x| x.machine == m.id).cloned().collect::<Vec<_>>()).is_ok());
        if c.version > supports::VERSION || ids.len() != nb || ids.contains(&None) || !par_machine_bons || c.supports.iter().any(|x| !s.machines.iter().any(|m| m.id == x.machine)) {
            return refus(StatusCode::BAD_REQUEST, "supports_invalides", None);
        }
    }
    // Les réglages peuvent citer un profil de la sauvegarde ou un profil déjà là.
    let mut connus = couleur::lister(&etat.config.profils);
    connus.extend(profils.iter().map(|(p, _)| p.clone()));
    let mut liste = vec![];
    for r in s.reglages {
        if !reglages::nom_valide(&r.nom) || liste.iter().any(|x: &reglages::Reglage| x.nom == r.nom) {
            return refus(StatusCode::BAD_REQUEST, "reglage_invalide", Some(r.nom));
        }
        let r = reglages::Reglage { demande: r.demande.normaliser(), ..r };
        if let Err(code) = r.demande.verifier(&connus) {
            return refus(StatusCode::BAD_REQUEST, code, Some(r.nom));
        }
        liste.push(r);
    }
    let connus = reglages::lire(&etat.config.donnees);
    reglages::identifier(&mut liste, &connus);
    let _verrou = etat.reglages.lock().await;
    if etat.file.lock().unwrap().sorties.iter().any(|x| x.active() && !s.machines.iter().any(|m| m.id == x.machine)) {
        return refus(StatusCode::CONFLICT, "machine_occupee", None);
    }
    let ecrire = || -> std::io::Result<()> {
        std::fs::create_dir_all(&etat.config.profils)?;
        for (p, o) in &profils {
            std::fs::write(etat.config.profils.join(&p.fichier), o)?;
        }
        machines::ecrire(&etat.config.donnees, &s.machines)?;
        couverture::ecrire(&etat.config.donnees, &s.tarifs)?;
        if let Some(c) = &s.supports {
            supports::ecrire(&etat.config.donnees, c)?;
        }
        reglages::ecrire(&etat.config.donnees, &liste)
    };
    if let Err(e) = ecrire() {
        return refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string()));
    }
    *etat.machines.write().unwrap() = s.machines.clone();
    Json(serde_json::json!({ "machines": s.machines.len(), "reglages": liste.len(), "profils": profils.len() })).into_response()
}

/// Support chargé dans la machine (roadmap 1.3) : `null` si rien n'est connu.
async fn lire_charge(State(etat): State<Partage>, Chemin(id): Chemin<String>) -> Response {
    if machine(&etat, &id).is_none() {
        return introuvable();
    }
    Json(supports::lire_charges(&etat.config.donnees).remove(&id)).into_response()
}

#[derive(Deserialize)]
struct SaisieCharge {
    /// `None` : revenir à la lecture par la machine.
    support_uuid: Option<Uuid>,
}

/// L'opérateur dit quel support il a chargé ; une saisie prime sur la lecture de la machine
/// jusqu'à ce qu'il revienne à la lecture (`support_uuid` nul).
async fn saisir_charge(State(etat): State<Partage>, Chemin(id): Chemin<String>, Json(c): Json<SaisieCharge>) -> Response {
    if machine(&etat, &id).is_none() {
        return introuvable();
    }
    let catalogue = supports::lire(&etat.config.donnees, "epson");
    if c.support_uuid.is_some_and(|u| !catalogue.supports.iter().any(|s| s.machine == id && s.id == Some(u))) {
        return refus(StatusCode::BAD_REQUEST, "support_inconnu", None);
    }
    let laize_mm = supports::lire_charges(&etat.config.donnees).remove(&id).and_then(|a| a.laize_mm);
    let charge = supports::Charge { support_uuid: c.support_uuid, laize_mm, source: if c.support_uuid.is_some() { "saisie" } else { "machine" }.into(), le: maintenant() };
    match supports::ecrire_charge(&etat.config.donnees, &id, charge.clone()) {
        Ok(()) => Json(charge).into_response(),
        Err(e) => refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string())),
    }
}

/// Supports d'une machine et leurs modes d'impression (`supports.json`).
async fn lister_supports(State(etat): State<Partage>, Chemin(id): Chemin<String>) -> Response {
    if machine(&etat, &id).is_none() {
        return introuvable();
    }
    let c = supports::lire(&etat.config.donnees, "epson");
    Json(c.supports.into_iter().filter(|s| s.machine == id).collect::<Vec<_>>()).into_response()
}

/// Remplace les supports d'une machine ; les identifiants connus sont gardés, les nouveaux
/// attribués par le serveur, un identifiant inventé est refusé.
async fn enregistrer_supports(State(etat): State<Partage>, Chemin(id): Chemin<String>, Json(mut liste): Json<Vec<supports::Support>>) -> Response {
    if machine(&etat, &id).is_none() {
        return introuvable();
    }
    let _verrou = etat.reglages.lock().await;
    let mut c = supports::lire(&etat.config.donnees, "epson");
    liste.iter_mut().for_each(|s| s.machine = id.clone());
    if !supports::attribuer_ids(&mut liste, &c.supports) {
        return refus(StatusCode::BAD_REQUEST, "supports_invalides", Some("identifiant inconnu".into()));
    }
    if let Err(code) = supports::verifier(&liste) {
        return refus(StatusCode::BAD_REQUEST, code, None);
    }
    c.supports.retain(|s| s.machine != id);
    c.supports.extend(liste.iter().cloned());
    if let Err(e) = supports::ecrire(&etat.config.donnees, &c) {
        return refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string()));
    }
    Json(liste).into_response()
}

/// Média chargé dans une machine (largeur réelle), lu par SNMP dans la Printer-MIB.
async fn media_machine(State(etat): State<Partage>, Chemin(id): Chemin<String>) -> Response {
    let Some(m) = machine(&etat, &id) else { return introuvable() };
    let Some(hote) = m.snmp else { return refus(StatusCode::CONFLICT, "media_non_configure", None) };
    let communaute = env_ou("EKO_RIP_EPSON_COMMUNAUTE", "public");
    let epson = m.modele == machines::Modele::EpsonScS80600;
    let lecture = move || {
        let d = Duration::from_secs(2);
        media::lire_medias(&hote, &communaute, d).map(|m| (m, media::lire_alertes(&hote, &communaute, d).unwrap_or_default(), epson.then(|| media::lire_etat_epson(&hote, &communaute, d)).flatten()))
    };
    match tokio::task::spawn_blocking(lecture).await {
        Ok(Ok((m, alertes, etat))) if !m.is_empty() => Json(serde_json::json!({ "medias": m, "alertes": alertes, "epson": etat })).into_response(),
        Ok(Ok((_, alertes, _))) => refus(StatusCode::BAD_GATEWAY, "media_indisponible", Some(if alertes.is_empty() { "aucune largeur dans la Printer-MIB".into() } else { alertes.join(", ") })),
        Ok(Err(e)) => refus(StatusCode::BAD_GATEWAY, "media_indisponible", Some(e)),
        Err(e) => refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string())),
    }
}

/// Relevé SNMP de chaque Epson toutes les minutes (encres et rouleau), gardé quand il change.
async fn relever(etat: Partage) {
    let mut derniers: std::collections::HashMap<String, Option<releves::Releve>> = Default::default();
    loop {
        let cibles: Vec<(String, String)> = etat.machines.read().unwrap().iter().filter(|m| m.modele == machines::Modele::EpsonScS80600).filter_map(|m| Some((m.id.clone(), m.snmp.clone()?))).collect();
        for (id, hote) in cibles {
            let communaute = env_ou("EKO_RIP_EPSON_COMMUNAUTE", "public");
            let lecture = move || {
                let d = Duration::from_secs(2);
                let e = media::lire_etat_epson(&hote, &communaute, d)?;
                let media = media::lire_medias(&hote, &communaute, d).ok()?.into_iter().next()?;
                (!e.encres.is_empty()).then_some((e.encres, media.longueur_m, e.support, media.largeur_mm))
            };
            let Ok(Some((encres, longueur_m, numero, largeur_mm))) = tokio::task::spawn_blocking(lecture).await else { continue };
            // Support chargé lu sur la machine (n° de support), s'il a changé (roadmap 1.3).
            if let Some(n) = numero {
                let support_uuid = supports::par_numero(&supports::lire(&etat.config.donnees, "epson"), &id, n).and_then(|s| s.id);
                let avant = supports::lire_charges(&etat.config.donnees).remove(&id);
                if avant.as_ref().is_none_or(|a| a.source != "saisie" && (a.support_uuid != support_uuid || a.laize_mm != Some(largeur_mm))) {
                    let _ = supports::ecrire_charge(&etat.config.donnees, &id, supports::Charge { support_uuid, laize_mm: Some(largeur_mm), source: "machine".into(), le: maintenant() });
                }
            }
            let r = releves::Releve { le: maintenant(), encres, longueur_m };
            let dernier = derniers.entry(id.clone()).or_insert_with(|| releves::lire(&etat.config.donnees, &id).pop());
            if dernier.as_ref().is_some_and(|d| releves::identique(d, &r)) {
                continue;
            }
            if let Err(e) = releves::ajouter(&etat.config.donnees, &id, &r) {
                eprintln!("relevé {id} : {e}");
            }
            *dernier = Some(r);
        }
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}

#[derive(Deserialize)]
struct Periode {
    /// Secondes Unix ; par défaut les 7 derniers jours.
    depuis: Option<u64>,
    jusqua: Option<u64>,
}

/// Consommation réelle relevée sur la machine (encre en points de %, support en m).
async fn consommation_machine(State(etat): State<Partage>, Chemin(id): Chemin<String>, axum::extract::Query(q): axum::extract::Query<Periode>) -> Response {
    if machine(&etat, &id).is_none() {
        return introuvable();
    }
    let jusqua = q.jusqua.unwrap_or_else(maintenant);
    let depuis = q.depuis.unwrap_or(jusqua.saturating_sub(7 * 86400));
    Json(releves::consommation(&releves::lire(&etat.config.donnees, &id), depuis, jusqua)).into_response()
}

/// Imposition sans rien créer : places des éléments (mm, origine en bas à gauche),
/// longueur de rouleau et part de la surface utilisée.
async fn apercu_planche(State(etat): State<Partage>, Json(d): Json<DemandePlanche>) -> Response {
    let mauvais = |detail: String| refus(StatusCode::BAD_REQUEST, "planche_invalide", Some(detail));
    if !(50.0..=5000.0).contains(&d.laize_mm) || !(0.0..=100.0).contains(&d.espacement_mm) || d.elements.is_empty() || d.elements.len() > 200 || d.elements.iter().any(|e| !(1..=500).contains(&e.copies)) {
        return mauvais("laize 50–5000 mm, espacement 0–100 mm, 1 à 200 éléments, 1 à 500 copies".into());
    }
    if let Some(e) = d.motif_invalide() {
        return mauvais(e.into());
    }
    let donnees = &etat.config.donnees;
    let mut pdfs = vec![];
    let mut noms = vec![];
    for e in &d.elements {
        let Some((t, _)) = lire_page(donnees, e.id, e.page) else { return refus(StatusCode::NOT_FOUND, "introuvable", None) };
        match tokio::fs::read(dossier(donnees, e.id).join(Format::Pdf.fichier())).await {
            Ok(o) => pdfs.push(o),
            Err(err) => return refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(err.to_string())),
        }
        noms.push(format!("{} p{}", t.nom, e.page));
    }
    let pages: Vec<(usize, usize)> = d.elements.iter().map(|e| (e.page, e.copies)).collect();
    let (laize, espace, rotation, forme, motif) = (d.laize_mm, d.espacement_mm, d.rotation, d.forme, d.motif);
    let imp = match tokio::task::spawn_blocking(move || {
        let sources: Vec<imbrication::Source> = pdfs.iter().zip(&pages).map(|(pdf, &(page, copies))| imbrication::Source { pdf, page, copies }).collect();
        imbrication::imposer(&sources, laize, espace, rotation, forme, motif)
    })
    .await
    {
        Ok(Ok(x)) => x,
        Ok(Err(e)) => return mauvais(e),
        Err(e) => return refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string())),
    };
    let mm = imbrication::PT_PAR_MM;
    // Aire du contour de découpe (le plus grand tracé fermé de la page), sinon du rectangle :
    // sur la forme, les rectangles englobants se chevauchent.
    let aire_contour = |n: usize| imp.contours[n].iter().map(|q| forme::aire(q)).fold(0.0, f64::max);
    let utile: f64 = imp
        .places
        .iter()
        .map(|p| {
            let (n, t) = (imp.elements[p.indice], imp.tailles[p.indice]);
            let a = aire_contour(n);
            if a > 0.0 { a } else { t.largeur * t.hauteur }
        })
        .sum();
    let places: Vec<_> = imp
        .places
        .iter()
        .map(|p| {
            let (n, t) = (imp.elements[p.indice], imp.tailles[p.indice]);
            let (l, h) = if p.tourne() { (t.hauteur, t.largeur) } else { (t.largeur, t.hauteur) };
            // Contours posés comme la page (même rotation), en mm sur la planche.
            let contours: Vec<Vec<[f64; 2]>> = imp.contours[n]
                .iter()
                .map(|poly| poly.iter().map(|&q| { let (x, y) = forme::tourner(q, t, p.angle); [(p.x + x) / mm, (p.y + y) / mm] }).collect())
                .collect();
            serde_json::json!({ "nom": noms[n], "x": p.x / mm, "y": p.y / mm, "largeur": l / mm, "hauteur": h / mm, "tourne": p.tourne(), "angle": p.angle, "contours": contours })
        })
        .collect();
    Json(serde_json::json!({
        "laize_mm": d.laize_mm, "longueur_mm": imp.planche.hauteur / mm, "largeur_mm": imp.planche.largeur / mm,
        "utilisation": utile / (d.laize_mm * mm * imp.planche.hauteur).max(1e-9), "places": places,
    }))
    .into_response()
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
    if let Some(e) = d.motif_invalide() {
        return mauvais(e);
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
    let (laize, espace, rotation, forme, motif) = (d.laize_mm, d.espacement_mm, d.rotation, d.forme, d.motif);
    let pages: Vec<(usize, usize)> = d.elements.iter().map(|e| (e.page, e.copies)).collect();
    let compose = tokio::task::spawn_blocking(move || {
        let sources: Vec<imbrication::Source> = pdfs.iter().zip(&pages).map(|(pdf, &(page, copies))| imbrication::Source { pdf, page, copies }).collect();
        imbrication::composer(&sources, laize, espace, rotation, forme, motif)
    })
    .await;
    let planche = match compose {
        Ok(Ok((pdf, _))) => pdf,
        Ok(Err(e)) => return mauvais(&e),
        Err(e) => return refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string())),
    };

    let id = Uuid::new_v4();
    let rep = dossier(donnees, id);
    if let Err(e) = tokio::fs::create_dir_all(&rep).await.and(tokio::fs::write(rep.join("source.tmp"), planche).await) {
        return abandon(&rep, StatusCode::INTERNAL_SERVER_ERROR, ("erreur_interne", Some(e.to_string()))).await;
    }
    let copies: usize = d.elements.iter().map(|e| e.copies).sum();
    let nom = match d.motif {
        Some(m) => format!("Motif {} × {}{} — {laize:.0} mm", m.colonnes, m.rangees, if m.demi_saut { " demi-saut" } else { "" }),
        None => format!("Planche {laize:.0} mm — {copies} élément(s)"),
    };
    match creer_travail(&etat, id, nom, Format::Pdf, Demande { dpi: d.dpi, reperage: d.reperage, ..Default::default() }).await {
        Ok(t) => (StatusCode::ACCEPTED, Json(t)).into_response(),
        Err((statut, r)) => abandon(&rep, statut, r).await,
    }
}

/// `eko-rip-serveur utilisateur ajouter|supprimer|liste [nom] [admin|operateur]` ; le
/// mot de passe est demandé sans écho, jamais passé en argument (il resterait dans
/// l'historique). Sans rôle : administrateur s'il n'y en a aucun, opérateur sinon.
fn commande_utilisateur(donnees: &std::path::Path, args: &[String]) -> i32 {
    let _ = std::fs::create_dir_all(donnees);
    let mut liste = acces::lire(donnees);
    let nom = args.get(1).map(String::as_str).unwrap_or("");
    match args.first().map(String::as_str) {
        Some("liste") => {
            for u in &liste {
                println!("{}\t{}", u.nom, if u.role == acces::Role::Admin { "admin" } else { "operateur" });
            }
            return 0;
        }
        Some("ajouter") if acces::nom_valide(nom) => {
            let role = match args.get(2).map(String::as_str) {
                Some("admin") => acces::Role::Admin,
                Some("operateur") => acces::Role::Operateur,
                None if !liste.iter().any(|u| u.nom != nom && u.role == acces::Role::Admin) => acces::Role::Admin,
                None => acces::Role::Operateur,
                Some(r) => {
                    eprintln!("Rôle inconnu : {r} (admin ou operateur)");
                    return 2;
                }
            };
            let Ok(mdp) = rpassword::prompt_password(format!("Mot de passe pour {nom} : ")) else { return 1 };
            if mdp.chars().count() < 10 {
                eprintln!("Mot de passe trop court (10 caractères au moins).");
                return 1;
            }
            if rpassword::prompt_password("Confirmer : ").ok().as_deref() != Some(mdp.as_str()) {
                eprintln!("Les deux saisies diffèrent.");
                return 1;
            }
            let hachage = match acces::hacher(&mdp) {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("{e}");
                    return 1;
                }
            };
            liste.retain(|u| u.nom != nom);
            liste.push(acces::Utilisateur { nom: nom.into(), hachage, role });
            println!("{nom} : {}", if role == acces::Role::Admin { "administrateur" } else { "opérateur" });
            liste.sort_by(|a, b| a.nom.cmp(&b.nom));
        }
        Some("supprimer") if liste.iter().any(|u| u.nom == nom) => liste.retain(|u| u.nom != nom),
        Some("supprimer") => {
            eprintln!("Utilisateur inconnu : {nom}");
            return 1;
        }
        _ => {
            eprintln!("Usage : eko-rip-serveur utilisateur ajouter|supprimer|liste [nom] [admin|operateur]\\n(nom : lettres, chiffres, . - _ ; 40 caractères au plus)");
            return 2;
        }
    }
    match acces::ecrire(donnees, &liste) {
        Ok(()) => {
            println!("Enregistré dans {} (redémarrer le service n'est pas nécessaire).", acces::fichier(donnees).display());
            0
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

fn jeton_requete(entetes: &axum::http::HeaderMap) -> Option<String> {
    let c = entetes.get(header::COOKIE)?.to_str().ok()?;
    acces::jeton_du_cookie(c).map(str::to_string)
}

/// Toute l'API demande une session, sauf `/api/session` ; sans utilisateur déclaré,
/// l'accès reste libre. La page `/` est servie à tous : elle porte l'écran de connexion.
/// Signature du client de bureau dans son User-Agent (fixée dans `client/tauri.conf.json`).
pub const SIGNATURE_CLIENT: &str = "EKO-RIP-Client/";

fn est_client(etat: &Etat, entetes: &axum::http::HeaderMap) -> bool {
    etat.config.acces_navigateur || entetes.get(header::USER_AGENT).and_then(|v| v.to_str().ok()).is_some_and(|ua| ua.contains(SIGNATURE_CLIENT))
}

/// L'interface et les réglages ne vivent que dans les clients (Mac, PC, Linux) ; un
/// navigateur qui ouvre l'adresse du serveur voit une page d'état simple.
// ponytail: la signature se falsifie ; ce n'est pas un contrôle d'accès (les comptes le
// sont), seulement la règle d'usage : on travaille depuis un client.
async fn accueil(State(etat): State<Partage>, entetes: axum::http::HeaderMap) -> Html<String> {
    if est_client(&etat, &entetes) {
        return Html(include_str!("../static/index.html").to_string());
    }
    let echapper = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;");
    let hote = entetes.get(header::HOST).and_then(|v| v.to_str().ok()).unwrap_or("adresse-du-serveur:8640");
    let file = etat.file.lock().unwrap();
    let lignes: String = etat
        .machines
        .read()
        .unwrap()
        .iter()
        .map(|m| {
            let n = file.sorties.iter().filter(|s| s.machine == m.id && s.active()).count();
            format!("<li><strong>{}</strong> — {} en file</li>", echapper(&m.nom), n)
        })
        .collect();
    Html(include_str!("../static/accueil.html").replace("{version}", env!("CARGO_PKG_VERSION")).replace("{hote}", &echapper(hote)).replace("{machines}", &lignes))
}

async fn garder(State(etat): State<Partage>, requete: axum::extract::Request, suite: axum::middleware::Next) -> Response {
    let chemin = requete.uri().path();
    // Pont ERP (exception accordée par le gérant le 24/10) : le jeton de l'ERP ouvre ses
    // seules routes ; elles restent fermées sans lui, même quand l'accès humain est libre.
    let bearer = requete.headers().get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer "));
    if chemin.starts_with(erp::PREFIXE) {
        return if bearer.is_some_and(|j| erp::jeton_valide(&erp::lire(&etat.config.donnees), j)) { suite.run(requete).await } else { refus(StatusCode::UNAUTHORIZED, "jeton_erp_requis", None) };
    }
    if bearer.is_some() {
        return refus(StatusCode::FORBIDDEN, "jeton_erp_hors_pont", None);
    }
    if chemin.starts_with("/api/") && !est_client(&etat, requete.headers()) {
        return refus(StatusCode::FORBIDDEN, "client_requis", None);
    }
    if !chemin.starts_with("/api/") || chemin == "/api/session" || acces::libre(&etat.config.donnees) {
        return suite.run(requete).await;
    }
    let Some(role) = role_session(&etat, requete.headers()) else { return refus(StatusCode::UNAUTHORIZED, "connexion_requise", None) };
    if role != acces::Role::Admin && acces::reserve_admin(requete.method(), chemin) {
        return refus(StatusCode::FORBIDDEN, "admin_requis", None);
    }
    suite.run(requete).await
}

/// Contrat du pont ERP : nom et version, version du serveur, modèle d'états (lot R1).
async fn contrat_erp() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "contrat": erp::CONTRAT, "serveur": env!("CARGO_PKG_VERSION"), "signature": "X-EKO-RIP-Signature: t=<secondes Unix>,v1=<hex HMAC-SHA256(secret, \"<t>.<corps>\")>", "etats": erp::modele_etats() }))
}

/// `eko-rip-serveur erp jeton|secret|etat` : crée le jeton de l'ERP ou le secret HMAC
/// (affiché une seule fois ; le précédent cesse de valoir), ou dit ce qui est configuré.
fn commande_erp(donnees: &std::path::Path, action: Option<&str>) -> i32 {
    let mut c = erp::lire(donnees);
    let nouveau = erp::aleatoire();
    match action {
        Some("jeton") => c.jeton_sha256 = Some(erp::empreinte(&nouveau)),
        Some("secret") => c.secret_hmac = Some(nouveau.clone()),
        Some("etat") => {
            println!("jeton de l'ERP : {}", if c.jeton_sha256.is_some() { "déclaré" } else { "aucun" });
            println!("secret HMAC : {}", if c.secret_hmac.is_some() { "déclaré" } else { "aucun" });
            return 0;
        }
        _ => {
            eprintln!("usage : eko-rip-serveur erp jeton|secret|etat");
            return 2;
        }
    }
    if let Err(e) = std::fs::create_dir_all(donnees).and_then(|_| erp::ecrire(donnees, &c)) {
        eprintln!("erp.json : {e}");
        return 1;
    }
    println!("{nouveau}");
    eprintln!("À recopier maintenant dans l'ERP : il ne sera plus affiché. Le précédent ne vaut plus.");
    0
}

/// Rôle de l'utilisateur de la session, s'il existe encore.
fn role_session(etat: &Etat, entetes: &axum::http::HeaderMap) -> Option<acces::Role> {
    let nom = etat.sessions.lock().unwrap().utilisateur(&jeton_requete(entetes)?, std::time::Instant::now())?;
    acces::lire(&etat.config.donnees).into_iter().find(|u| u.nom == nom).map(|u| u.role)
}

async fn lire_session(State(etat): State<Partage>, entetes: axum::http::HeaderMap) -> Json<serde_json::Value> {
    let ouvert = acces::libre(&etat.config.donnees);
    let utilisateur = jeton_requete(&entetes).and_then(|j| etat.sessions.lock().unwrap().utilisateur(&j, std::time::Instant::now()));
    // Accès libre : tout est permis, comme un administrateur.
    let role = if ouvert { Some(acces::Role::Admin) } else { role_session(&etat, &entetes) };
    Json(serde_json::json!({ "acces_libre": ouvert, "utilisateur": utilisateur, "role": role }))
}

#[derive(Deserialize)]
struct Identifiants {
    nom: String,
    mot_de_passe: String,
}

async fn ouvrir_session(State(etat): State<Partage>, Json(i): Json<Identifiants>) -> Response {
    let utilisateurs = acces::lire(&etat.config.donnees);
    // Argon2 coûte des dizaines de ms : hors du fil d'exécution asynchrone.
    let e = etat.clone();
    let resultat = tokio::task::spawn_blocking(move || e.sessions.lock().unwrap().connecter(&utilisateurs, &i.nom, &i.mot_de_passe, std::time::Instant::now()))
        .await
        .unwrap();
    match resultat {
        Ok(j) => {
            let cookie = format!("{}={j}; HttpOnly; SameSite=Strict; Path=/; Max-Age={}", acces::COOKIE, acces::DUREE_SESSION.as_secs());
            (StatusCode::NO_CONTENT, [(header::SET_COOKIE, cookie)]).into_response()
        }
        Err(acces::Refus::Identifiants) => refus(StatusCode::UNAUTHORIZED, "identifiants_invalides", None),
        Err(acces::Refus::Bloque) => refus(StatusCode::TOO_MANY_REQUESTS, "trop_d_echecs", None),
    }
}

async fn fermer_session(State(etat): State<Partage>, entetes: axum::http::HeaderMap) -> Response {
    if let Some(j) = jeton_requete(&entetes) {
        etat.sessions.lock().unwrap().deconnecter(&j);
    }
    let cookie = format!("{}=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0", acces::COOKIE);
    (StatusCode::NO_CONTENT, [(header::SET_COOKIE, cookie)]).into_response()
}

async fn lister_reglages(State(etat): State<Partage>) -> Json<Vec<reglages::Reglage>> {
    Json(reglages::lire(&etat.config.donnees))
}

/// Crée ou remplace un réglage (même nom) ; son dossier surveillé est créé.
async fn enregistrer_reglage(State(etat): State<Partage>, Json(r): Json<reglages::Reglage>) -> Response {
    if !reglages::nom_valide(&r.nom) {
        return refus(StatusCode::BAD_REQUEST, "reglage_invalide", Some(r.nom));
    }
    let r = reglages::Reglage { demande: r.demande.normaliser(), ..r };
    if let Err(code) = r.demande.verifier(&couleur::lister(&etat.config.profils)) {
        return refus(StatusCode::BAD_REQUEST, code, None);
    }
    let _verrou = etat.reglages.lock().await;
    let mut liste = reglages::lire(&etat.config.donnees);
    let mut r = reglages::Reglage { uuid: None, ..r };
    reglages::identifier(std::slice::from_mut(&mut r), &liste);
    liste.retain(|x| x.nom != r.nom);
    liste.push(r.clone());
    liste.sort_by(|a, b| a.nom.cmp(&b.nom));
    match reglages::ecrire(&etat.config.donnees, &liste) {
        Ok(()) => (StatusCode::CREATED, Json(r)).into_response(),
        Err(e) => refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string())),
    }
}

/// Retire le réglage ; son dossier surveillé reste sur le disque (des fichiers peuvent y être).
async fn supprimer_reglage(State(etat): State<Partage>, Chemin(nom): Chemin<String>) -> Response {
    let _verrou = etat.reglages.lock().await;
    let mut liste = reglages::lire(&etat.config.donnees);
    let avant = liste.len();
    liste.retain(|x| x.nom != nom);
    if liste.len() == avant {
        return introuvable();
    }
    match reglages::ecrire(&etat.config.donnees, &liste) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string())),
    }
}

/// Travaux à purger : plus vieux que `jours` jours et inactifs (`jours` = 0 : aucun).
fn a_purger(travaux: &[Travail], maintenant: u64, jours: u64) -> Vec<Uuid> {
    if jours == 0 {
        return vec![];
    }
    travaux.iter().filter(|t| !actif(&t.statut) && maintenant.saturating_sub(t.cree_le) > jours * 86_400).map(|t| t.id).collect()
}

/// Toutes les 5 min, efface les travaux trop vieux, puis les plus vieux inactifs tant que
/// l'espace libre est sous le seuil (le disque se remplit vite à 720 dpi).
async fn purger(etat: Partage) {
    loop {
        let donnees = &etat.config.donnees;
        for id in a_purger(&lire_tous(donnees), maintenant(), etat.config.conservation_jours) {
            if effacer_travail(&etat, id).await {
                eprintln!("travail {id} purgé (plus de {} jours)", etat.config.conservation_jours);
            }
        }
        // Sous le seuil d'espace libre : le plus vieux inactif d'abord, jusqu'à repasser au-dessus.
        if etat.config.seuil_libre > 0 {
            for id in plus_vieux_inactifs(&lire_tous(donnees)) {
                if espace_disque(donnees).await.is_none_or(|(_, libre)| libre >= etat.config.seuil_libre) {
                    break;
                }
                if effacer_travail(&etat, id).await {
                    eprintln!("travail {id} purgé (espace libre sous le seuil)");
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(300)).await;
    }
}

/// Efface un travail et ses sorties, sauf s'il a encore une sortie à passer.
async fn effacer_travail(etat: &Partage, id: Uuid) -> bool {
    if etat.file.lock().unwrap().active_pour(id) {
        return false;
    }
    let donnees = &etat.config.donnees;
    modifier_file(etat, |f| {
        f.oublier_travail(id);
        Ok(())
    })
    .ok();
    effacer_sorties(donnees, id);
    match tokio::fs::remove_dir_all(dossier(donnees, id)).await {
        Ok(()) => true,
        Err(e) => {
            eprintln!("travail {id} : purge impossible : {e}");
            false
        }
    }
}

/// (total, libre) du disque de `chemin`, en octets (`df -Pk`, présent sous Linux et macOS).
async fn espace_disque(chemin: &std::path::Path) -> Option<(u64, u64)> {
    let o = tokio::process::Command::new("df").arg("-Pk").arg(chemin).output().await.ok()?;
    let texte = String::from_utf8_lossy(&o.stdout).into_owned();
    let champs: Vec<u64> = texte.lines().nth(1)?.split_whitespace().filter_map(|c| c.parse().ok()).collect();
    Some((champs.first()? * 1024, champs.get(2)? * 1024))
}

/// Place estimée d'un travail rendu : ≈ 1 Go/m² à 600 dpi, au carré de la résolution.
fn place_estimee(surface_m2: f64, dpi: u32) -> u64 {
    (surface_m2 * 1e9 * (f64::from(dpi) / 600.0).powi(2)) as u64
}

/// Travaux inactifs, du plus vieux au plus récent : ordre de purge sous le seuil.
fn plus_vieux_inactifs(travaux: &[Travail]) -> Vec<Uuid> {
    let mut v: Vec<&Travail> = travaux.iter().filter(|t| !actif(&t.statut)).collect();
    v.sort_by_key(|t| t.cree_le);
    v.into_iter().map(|t| t.id).collect()
}

/// Espace du disque des données et version.
async fn systeme(State(etat): State<Partage>) -> Json<serde_json::Value> {
    let (total, libre) = espace_disque(&etat.config.donnees).await.map_or((None, None), |(t, l)| (Some(t), Some(l)));
    Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "disque_total": total,
        "disque_libre": libre,
        "conservation_jours": etat.config.conservation_jours,
        "seuil_libre": etat.config.seuil_libre,
        "alertes": etat::alertes(),
    }))
}

/// Dossiers surveillés : toutes les 5 s, chaque fichier stable de `entrees/<réglage>/`
/// devient un travail avec ce réglage ; un refus range le fichier dans `refuses/`.
async fn surveiller(etat: Partage) {
    let mut vus = std::collections::HashMap::new();
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        let donnees = etat.config.donnees.clone();
        let liste = reglages::lire(&donnees);
        let mut fichiers = vec![];
        for r in &liste {
            let dossier = reglages::dossier_entree(&donnees, &r.nom);
            // Disparu en marche : recréé et signalé, la surveillance continue.
            if !etat::assurer_dossier(&dossier) {
                continue;
            }
            let Ok(entrees) = std::fs::read_dir(dossier) else { continue };
            for e in entrees.flatten() {
                let (Ok(meta), Ok(nom)) = (e.metadata(), e.file_name().into_string()) else { continue };
                if meta.is_file() && !nom.starts_with('.') {
                    fichiers.push((e.path(), meta.len()));
                }
            }
        }
        for chemin in reglages::stables(&mut vus, fichiers) {
            let Some(r) = chemin.parent().and_then(|d| d.file_name()).and_then(|n| n.to_str()).and_then(|n| liste.iter().find(|r| r.nom == n)) else { continue };
            if let Err(raison) = prendre_fichier(&etat, &chemin, r.demande.clone()).await {
                eprintln!("dossier surveillé {} : {} refusé : {raison}", r.nom, chemin.display());
                let refuses = chemin.with_file_name("refuses");
                let _ = std::fs::create_dir_all(&refuses);
                if let Some(nom) = chemin.file_name() {
                    let cible = refuses.join(nom);
                    let _ = std::fs::rename(&chemin, &cible);
                    let _ = std::fs::write(cible.with_extension("refus.txt"), format!("{raison}\n"));
                }
            }
        }
    }
}

/// Fichier d'un dossier surveillé → nouveau travail. Le fichier est déplacé dans le
/// dossier du travail ; en cas de refus, il est remis à sa place pour être rangé.
async fn prendre_fichier(etat: &Partage, chemin: &std::path::Path, d: Demande) -> Result<(), String> {
    let mut debut = vec![0u8; EN_TETE];
    let n = std::io::Read::read(&mut std::fs::File::open(chemin).map_err(|e| e.to_string())?, &mut debut).map_err(|e| e.to_string())?;
    let format = reconnaitre(&debut[..n]).map_err(|(code, _)| code.to_string())?;
    let nom: String = chemin.file_name().and_then(|n| n.to_str()).unwrap_or("sans nom").chars().take(200).collect();
    let id = Uuid::new_v4();
    let rep = dossier(&etat.config.donnees, id);
    std::fs::create_dir_all(&rep).map_err(|e| e.to_string())?;
    std::fs::rename(chemin, rep.join("source.tmp")).map_err(|e| e.to_string())?;
    match creer_travail(etat, id, nom, format, d).await {
        Ok(_) => Ok(()),
        Err((_, (code, detail))) => {
            let _ = std::fs::rename(rep.join("source.tmp"), chemin).or_else(|_| std::fs::rename(rep.join(format.fichier()), chemin));
            let _ = std::fs::remove_dir_all(&rep);
            Err(detail.map_or(code.to_string(), |d| format!("{code} : {d}")))
        }
    }
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
/// Sorties ajoutées d'office après le rendu (réglage `auto`). Une découpe qui suit une
/// impression part toujours suspendue : le support doit d'abord passer de l'Epson au Kona.
fn mettre_en_file_auto(etat: &Etat, t: &Travail) {
    let a = t.demande.auto;
    if !a.imprimer && !a.decouper {
        return;
    }
    // Première machine du catalogue qui imprime ; pour découper, elle-même si elle découpe
    // aussi (imprimante-découpeuse), sinon la première découpeuse.
    let (imprimante, decoupeuse) = {
        let c = etat.machines.read().unwrap();
        let i = c.iter().find(|m| m.fait(machines::Operation::Impression));
        let d = i.filter(|m| m.fait(machines::Operation::Decoupe)).or_else(|| c.iter().find(|m| m.fait(machines::Operation::Decoupe)));
        (i.map(|m| m.id.clone()), d.map(|m| m.id.clone()))
    };
    let mut f = etat.file.lock().unwrap();
    for (i, p) in t.pages.iter().enumerate() {
        let imprime = a.imprimer && t.demande.mode == Mode::Imprimer && p.plaques.iter().any(|x| !p.plaques_decoupe.contains(x));
        let mut ajouter = |machine: &str, operation, suspendue: bool| {
            let mut s = file::Sortie::nouvelle(machine, operation, t.id, &t.nom, i + 1, maintenant());
            if suspendue {
                s.etat = file::Etat::Suspendue;
            }
            f.sorties.push(s);
        };
        if let (true, Some(m)) = (imprime, &imprimante) {
            ajouter(m, machines::Operation::Impression, a.suspendre);
        }
        if let (true, Some(m)) = (a.decouper && p.traces_decoupe > 0, &decoupeuse) {
            ajouter(m, machines::Operation::Decoupe, a.suspendre || imprime);
        }
    }
    if let Err(e) = f.ecrire(&etat.config.donnees) {
        eprintln!("travail {} : file non enregistrée : {e}", t.id);
    }
}

/// Génère la plaque `White` de chaque page (production et aperçu) selon le réglage de
/// blanc ; rend les listes de plaques, `White` comprise.
async fn ajouter_blanc(rep: &std::path::Path, mut pages: Vec<Vec<String>>, b: blanc::Blanc, dpi: u32) -> Result<Vec<Vec<String>>, String> {
    if b.mode == blanc::Mode::Aucun {
        return Ok(pages);
    }
    for (i, plaques) in pages.iter_mut().enumerate() {
        let n = i + 1;
        let sources: Vec<&String> = plaques.iter().filter(|p| p.as_str() != blanc::PLAQUE && !decoupe::est_ton_de_decoupe(p)).collect();
        let existe = plaques.iter().any(|p| p == blanc::PLAQUE);
        let chemins = |f: &dyn Fn(&std::path::Path, usize, &str) -> PathBuf| -> (Vec<PathBuf>, Option<PathBuf>, PathBuf) {
            (sources.iter().map(|p| f(rep, n, p)).collect(), existe.then(|| f(rep, n, blanc::PLAQUE)), f(rep, n, blanc::PLAQUE))
        };
        let prod = chemins(&render::chemin_plaque);
        let apercu = chemins(&render::chemin_apercu_plaque);
        tokio::task::spawn_blocking(move || -> Result<(), String> {
            blanc::generer_tiff(&prod.0, prod.1.as_deref(), &prod.2, &b, dpi as f64)?;
            blanc::generer_apercu(&apercu.0, apercu.1.as_deref(), &apercu.2, &b, render::DPI_APERCU as f64)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("blanc de la page {n} : {e}"))?;
        if !existe {
            plaques.push(blanc::PLAQUE.to_string());
            plaques.sort();
        }
    }
    Ok(pages)
}

async fn preparer_impression(rep: &std::path::Path, choix: reperage::Reperage, id: Uuid) -> Result<(PathBuf, Vec<reperage::Mise>), String> {
    let source = rep.join(Format::Pdf.fichier());
    let pdf = tokio::fs::read(&source).await.map_err(|e| e.to_string())?;
    // Les tons de découpe ne marquent jamais le support : neutralisés avant le rendu.
    let (pdf, neutralises) = tokio::task::spawn_blocking(move || decoupe::neutraliser(&pdf)).await.map_err(|e| e.to_string())??;
    if choix == reperage::Reperage::Aucun && neutralises == 0 {
        return Ok((source, vec![]));
    }
    let (imprime, mises) = if choix == reperage::Reperage::Aucun {
        (pdf, vec![])
    } else {
        tokio::task::spawn_blocking(move || reperage::envelopper(&pdf, choix, id)).await.map_err(|e| e.to_string())??
    };
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

fn actif(s: &Statut) -> bool {
    matches!(s, Statut::EnAttente | Statut::EnCours)
}

/// Rendu en cours ou sortie à passer : on ne touche pas au dossier du travail.
fn occupe(etat: &Etat, t: &Travail) -> bool {
    actif(&t.statut) || etat.file.lock().unwrap().active_pour(t.id)
}

/// Efface les sorties en dossier d'un travail (`sorties/<machine>/<travail>-…`).
fn effacer_sorties(donnees: &std::path::Path, id: Uuid) {
    for rep in std::fs::read_dir(donnees.join("sorties")).into_iter().flatten().flatten() {
        for e in std::fs::read_dir(rep.path()).into_iter().flatten().flatten() {
            if e.file_name().to_string_lossy().starts_with(&id.to_string()) {
                let _ = std::fs::remove_dir_all(e.path());
            }
        }
    }
}

async fn supprimer(State(etat): State<Partage>, Chemin(id): Chemin<Uuid>) -> Response {
    let donnees = &etat.config.donnees;
    let Some(t) = lire_travail(donnees, id) else { return refus(StatusCode::NOT_FOUND, "introuvable", None) };
    if occupe(&etat, &t) {
        return refus(StatusCode::CONFLICT, "travail_occupe", None);
    }
    let _ = modifier_file(&etat, |f| {
        f.oublier_travail(id);
        Ok(())
    });
    effacer_sorties(donnees, id);
    match tokio::fs::remove_dir_all(dossier(donnees, id)).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string())),
    }
}

/// Recommence le rendu et la préparation de la découpe à partir du fichier reçu.
async fn relancer(State(etat): State<Partage>, Chemin(id): Chemin<Uuid>) -> Response {
    let Some(t) = lire_travail(&etat.config.donnees, id) else { return refus(StatusCode::NOT_FOUND, "introuvable", None) };
    if occupe(&etat, &t) {
        return refus(StatusCode::CONFLICT, "travail_occupe", None);
    }
    refaire(&etat, t).await
}

/// Édition rapide d'un travail existant : nouvelle édition appliquée à l'original, puis
/// nouveau rendu (le fichier n'est pas à renvoyer).
async fn editer(State(etat): State<Partage>, Chemin(id): Chemin<Uuid>, Json(e): Json<edition::Edition>) -> Response {
    let Some(mut t) = lire_travail(&etat.config.donnees, id) else { return refus(StatusCode::NOT_FOUND, "introuvable", None) };
    if occupe(&etat, &t) {
        return refus(StatusCode::CONFLICT, "travail_occupe", None);
    }
    if t.format != Format::Pdf {
        return refus(StatusCode::CONFLICT, "edition_invalide", Some("PDF seulement".into()));
    }
    t.demande = Demande { edition: e, ..t.demande }.normaliser();
    if let Err(code) = t.demande.edition.verifier() {
        return refus(StatusCode::BAD_REQUEST, code, None);
    }
    reconstruire(&etat, t).await
}

/// Tous les réglages d'un travail déjà reçu (mode, repérage, coupe, lés, blanc, support,
/// couleur…) : rendu refait depuis l'original, comme à l'import.
async fn modifier_reglages(State(etat): State<Partage>, Chemin(id): Chemin<Uuid>, Json(d): Json<Demande>) -> Response {
    let Some(mut t) = lire_travail(&etat.config.donnees, id) else { return refus(StatusCode::NOT_FOUND, "introuvable", None) };
    if occupe(&etat, &t) {
        return refus(StatusCode::CONFLICT, "travail_occupe", None);
    }
    let mut d = d.normaliser();
    if t.format == Format::Svg && d.mode == Mode::Imprimer {
        return refus(StatusCode::BAD_REQUEST, "svg_decoupe_seulement", None);
    }
    if let Err(code) = supports::appliquer(&mut d, &supports::lire(&etat.config.donnees, "epson")).and_then(|()| d.verifier(&couleur::lister(&etat.config.profils))) {
        return refus(StatusCode::BAD_REQUEST, code, None);
    }
    t.demande = Demande { mosaique: d.mosaique.filter(|_| d.mode == Mode::Imprimer && t.format == Format::Pdf), ..d };
    reconstruire(&etat, t).await
}

/// Source refaite depuis l'original (rotation automatique comprise), puis nouveau rendu.
async fn reconstruire(etat: &Partage, mut t: Travail) -> Response {
    match construire_source(&dossier(&etat.config.donnees, t.id), t.format, &t.demande, laize_imprimante(etat)).await {
        Ok(tourne) => {
            t.alertes.retain(|a| a.code != "rotation_auto");
            t.alertes.extend(tourne);
        }
        Err((code, detail)) => {
            let statut = if code == "erreur_interne" { StatusCode::INTERNAL_SERVER_ERROR } else { StatusCode::BAD_REQUEST };
            return refus(statut, code, Some(detail));
        }
    }
    refaire(etat, t).await
}

async fn refaire(etat: &Partage, mut t: Travail) -> Response {
    let (donnees, id) = (&etat.config.donnees, t.id);
    // Les sorties d'avant ne doivent pas survivre au nouveau rendu (un .plt périmé partirait au plotter).
    let rep = dossier(donnees, id);
    for sous in ["prod", "apercu"] {
        let _ = tokio::fs::remove_dir_all(rep.join(sous)).await;
    }
    let _ = tokio::fs::remove_file(rep.join("imprime.pdf")).await;
    t.statut = Statut::EnAttente;
    (t.code_erreur, t.erreur) = (None, None);
    t.pages = vec![];
    if let Err(e) = ecrire(donnees, &t) {
        return refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string()));
    }
    tokio::spawn(traiter(etat.clone(), t.clone()));
    (StatusCode::ACCEPTED, Json(t)).into_response()
}

fn lire_travail(donnees: &std::path::Path, id: Uuid) -> Option<Travail> {
    etat::lire(&dossier(donnees, id).join("job.json")).ok().flatten()
}

/// Travail et page demandés, si la page existe (numérotée à partir de 1).
fn lire_page(donnees: &std::path::Path, id: Uuid, page: usize) -> Option<(Travail, Page)> {
    let t = lire_travail(donnees, id)?;
    let p = t.pages.get(page.checked_sub(1)?)?.clone();
    Some((t, p))
}

/// La file, chaque impression avec sa durée estimée (minutes, sur sa machine).
/// Pourquoi une sortie ne part pas (roadmap 0.4) ; `None` : elle partira à son tour.
/// Hors `attente_operateur`, la sortie est retenue tant que la raison dure.
fn raison_attente(donnees: &std::path::Path, s: &file::Sortie, m: Option<&machines::Machine>, charge: Option<&supports::Charge>) -> Option<&'static str> {
    match s.etat {
        file::Etat::Suspendue => return Some("attente_operateur"),
        file::Etat::EnAttente => {}
        _ => return None,
    }
    let Some(m) = m else { return Some("machine_hors_ligne") };
    if let machines::Connexion::Serie { port, .. } = &m.connexion
        && !port.exists()
    {
        return Some("machine_hors_ligne");
    }
    let Some((t, p)) = lire_page(donnees, s.travail, s.page) else { return Some("source_absente") };
    if s.operation() == machines::Operation::Decoupe && !fichier_plt(&dossier(donnees, s.travail), s.page).exists() {
        return Some("source_absente");
    }
    // Impression : le support chargé (connu) doit être celui du travail (roadmap 1.4) ; la
    // sortie part d'elle-même dès qu'il l'est, on ne la force pas à la main.
    let impression = s.operation() == machines::Operation::Impression;
    if impression
        && let (Some(voulu), Some(charge)) = (t.demande.support_uuid, charge.and_then(|c| c.support_uuid))
        && voulu != charge
    {
        return Some("attente_support");
    }
    // Page (repères compris) plus large que la laize utile, ou que le rouleau chargé.
    let laize = charge.filter(|_| impression).and_then(|c| c.laize_mm).map_or(m.laize_mm, |l| l.min(m.laize_mm));
    if p.largeur_mm > laize + 0.5 {
        return Some("format_trop_petit");
    }
    None
}

async fn lister_file(State(etat): State<Partage>) -> Json<Vec<serde_json::Value>> {
    let sorties = etat.file.lock().unwrap().sorties.clone();
    let catalogue = etat.machines.read().unwrap().clone();
    let donnees = etat.config.donnees.clone();
    let charges = supports::lire_charges(&donnees);
    Json(
        sorties
            .into_iter()
            .map(|s| {
                let duree = (s.operation() == machines::Operation::Impression)
                    .then(|| catalogue.iter().find(|m| m.id == s.machine))
                    .flatten()
                    .zip(lire_page(&donnees, s.travail, s.page))
                    .and_then(|(m, (t, p))| m.duree_impression_min(p.hauteur_mm, t.demande.dpi));
                let mut v = serde_json::to_value(&s).unwrap();
                v["duree_min"] = serde_json::json!(duree);
                v["raison_attente"] = serde_json::json!(raison_attente(&donnees, &s, catalogue.iter().find(|m| m.id == s.machine), charges.get(&s.machine)));
                v
            })
            .collect(),
    )
}

/// Change la file sous son verrou et l'écrit ; `Err` : code d'erreur pour l'écran.
fn modifier_file(etat: &Etat, f: impl FnOnce(&mut file::File) -> Result<(), &'static str>) -> Result<(), Box<Response>> {
    let mut file = etat.file.lock().unwrap();
    f(&mut file).map_err(|code| Box::new(refus(if code == "introuvable" { StatusCode::NOT_FOUND } else { StatusCode::CONFLICT }, code, None)))?;
    file.ecrire(&etat.config.donnees).map_err(|e| Box::new(refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string()))))
}

#[derive(Deserialize)]
struct DemandeSortie {
    travail: Uuid,
    page: usize,
    machine: String,
    /// Facultative si la machine ne fait qu'une chose.
    #[serde(default)]
    operation: Option<machines::Operation>,
}

/// Met une page en file : découpe pour le Kona, plaques pour l'Epson.
async fn mettre_en_file(State(etat): State<Partage>, Json(d): Json<DemandeSortie>) -> Response {
    let Some((t, p)) = lire_page(&etat.config.donnees, d.travail, d.page) else { return refus(StatusCode::NOT_FOUND, "introuvable", None) };
    if !matches!(t.statut, Statut::Termine) {
        return refus(StatusCode::CONFLICT, "travail_occupe", None);
    }
    let Some(m) = machine(&etat, &d.machine) else { return refus(StatusCode::NOT_FOUND, "introuvable", None) };
    use machines::Operation::{Decoupe, Impression};
    let operation = match d.operation {
        Some(o) => o,
        None if m.fait(Impression) && m.fait(Decoupe) => return refus(StatusCode::BAD_REQUEST, "operation_impossible", Some("imprimer ou découper ?".into())),
        None if m.fait(Impression) => Impression,
        None => Decoupe,
    };
    if !m.fait(operation) {
        return refus(StatusCode::CONFLICT, "operation_impossible", None);
    }
    match operation {
        Decoupe if p.traces_decoupe == 0 => return refus(StatusCode::CONFLICT, "rien_a_decouper", None),
        Impression if p.plaques.len() == p.plaques_decoupe.len() => return refus(StatusCode::CONFLICT, "rien_a_imprimer", None),
        Impression if m.modele == machines::Modele::EpsonScS80600 => {
            if let Err(e) = epson::preparer(&p.plaques, t.demande.couches) {
                return refus(StatusCode::CONFLICT, "envoi_impossible", Some(e));
            }
        }
        _ => {}
    }
    let s = file::Sortie::nouvelle(&m.id, operation, t.id, &t.nom, d.page, maintenant());
    let rendu = s.clone();
    match modifier_file(&etat, |f| {
        f.sorties.push(s);
        Ok(())
    }) {
        Ok(()) => (StatusCode::CREATED, Json(rendu)).into_response(),
        Err(r) => *r,
    }
}

async fn agir_sur_la_file(State(etat): State<Partage>, Chemin((id, action)): Chemin<(Uuid, String)>) -> Response {
    let Ok(action) = serde_json::from_value::<file::Action>(serde_json::Value::String(action)) else { return refus(StatusCode::NOT_FOUND, "introuvable", None) };
    match modifier_file(&etat, |f| f.agir(id, action, maintenant())) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(r) => *r,
    }
}

async fn retirer_de_la_file(State(etat): State<Partage>, Chemin(id): Chemin<Uuid>) -> Response {
    match modifier_file(&etat, |f| f.retirer(id)) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(r) => *r,
    }
}

/// Répartiteur : chaque seconde, pour chaque machine du catalogue qui n'a rien en cours,
/// lance la première sortie en attente qui la vise. Une sortie à la fois par machine.
async fn repartir(etat: Partage) {
    loop {
        let ids: Vec<String> = etat.machines.read().unwrap().iter().map(|m| m.id.clone()).collect();
        for id in ids {
            let suivante = {
                let mut f = etat.file.lock().unwrap();
                let m = etat.machines.read().unwrap().iter().find(|m| m.id == id).cloned();
                let charge = supports::lire_charges(&etat.config.donnees).remove(&id);
                f.prochaine_si(&id, |s| raison_attente(&etat.config.donnees, s, m.as_ref(), charge.as_ref()).is_none()).map(|i| {
                    let s = &mut f.sorties[i];
                    (s.etat, s.debut) = (file::Etat::EnCours, Some(maintenant()));
                    let s = s.clone();
                    let _ = f.ecrire(&etat.config.donnees);
                    s
                })
            };
            if let Some(s) = suivante {
                tokio::spawn(executer(etat.clone(), s));
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

/// Exécute une sortie selon le modèle et l'opération, et note le résultat.
async fn executer(etat: Partage, s: file::Sortie) {
    let resultat = match machine(&etat, &s.machine) {
        None => Err(format!("machine « {} » absente du catalogue", s.machine)),
        Some(m) => match (s.operation(), &m.connexion) {
            (machines::Operation::Impression, machines::Connexion::Dossier) => sortir_les_plaques(&etat, &s, m.modele).await,
            (machines::Operation::Impression, _) => Err(format!("pilote d'impression à venir pour {}", m.nom)),
            (machines::Operation::Decoupe, _) => envoyer_la_decoupe(&etat, &s, m).await,
        },
    };
    if let Err(e) = &resultat {
        eprintln!("sortie {} ({}, travail {} page {}) : {e}", s.id, s.machine, s.travail, s.page);
    }
    let mut f = etat.file.lock().unwrap();
    if let Some(x) = f.sorties.iter_mut().find(|x| x.id == s.id) {
        x.fin = Some(maintenant());
        match resultat {
            Ok(()) => x.etat = file::Etat::Terminee,
            Err(e) => (x.etat, x.code_erreur, x.erreur) = (file::Etat::Echec, Some("sortie_echouee".into()), Some(e)),
        }
    }
    let _ = f.ecrire(&etat.config.donnees);
}

/// Découpe : le HP-GL du rendu (0,01 mm) passé au pas de la machine, par sa connexion ;
/// pour le Kona, mis en forme comme par CutServer (lecture du cadre s'il y en a un).
async fn envoyer_la_decoupe(etat: &Etat, s: &file::Sortie, m: machines::Machine) -> Result<(), String> {
    let plt = fichier_plt(&dossier(&etat.config.donnees, s.travail), s.page);
    let (t, p) = lire_page(&etat.config.donnees, s.travail, s.page).ok_or("travail ou page disparu")?;
    tokio::task::spawn_blocking(move || {
        let texte = std::fs::read_to_string(&plt).map_err(|e| format!("lecture de {} : {e}", plt.display()))?;
        machines::envoyer(&m.connexion, pour_la_machine(&texte, &t, &p, &m).as_bytes())
    })
    .await
    .unwrap_or_else(|e| Err(e.to_string()))
}

/// Plaques en dossier, sauf la découpe. Pour l'Epson, telles que le pilote les enverra :
/// un fichier par canal (`K.tif`… `WH.tif`) et `envoi.json` (canaux dans l'ordre, couches).
async fn sortir_les_plaques(etat: &Etat, s: &file::Sortie, modele: machines::Modele) -> Result<(), String> {
    let donnees = &etat.config.donnees;
    let (t, p) = lire_page(donnees, s.travail, s.page).ok_or("travail ou page disparu")?;
    let cible = file::dossier_sortie(donnees, s);
    tokio::fs::create_dir_all(&cible).await.map_err(|e| e.to_string())?;
    let fichiers: Vec<(String, String)> = if modele == machines::Modele::EpsonScS80600 {
        let envoi = epson::preparer(&p.plaques, t.demande.couches)?;
        tokio::fs::write(cible.join("envoi.json"), serde_json::to_vec_pretty(&envoi).unwrap_or_default()).await.map_err(|e| e.to_string())?;
        envoi.canaux.into_iter().map(|(c, nom)| (nom, format!("{c:?}"))).collect()
    } else {
        p.plaques.iter().filter(|n| !p.plaques_decoupe.contains(n)).map(|n| (n.clone(), n.clone())).collect()
    };
    let rep = dossier(donnees, s.travail);
    for (nom, fichier) in fichiers {
        let source = render::chemin_plaque(&rep, s.page, &nom);
        tokio::fs::copy(&source, cible.join(format!("{fichier}.tif"))).await.map_err(|e| format!("copie de {} : {e}", source.display()))?;
    }
    Ok(())
}

/// Lit un fichier de sortie d'une page existante du travail ; `None` donne un 404.
async fn servir(donnees: &std::path::Path, id: Uuid, page: usize, chemin: impl FnOnce(&std::path::Path, &Page) -> Option<PathBuf>) -> Option<Vec<u8>> {
    let (_, p) = lire_page(donnees, id, page)?;
    tokio::fs::read(chemin(&dossier(donnees, id), &p)?).await.ok()
}

fn introuvable() -> Response {
    refus(StatusCode::NOT_FOUND, "introuvable", None)
}

/// Fichier d'une page absent : 409 `pas_pret` si le travail existe et se rend encore (à
/// redemander), sinon 404 (roadmap 0.5).
fn page_absente(donnees: &std::path::Path, id: Uuid) -> Response {
    match lire_travail(donnees, id) {
        Some(t) if matches!(t.statut, Statut::EnAttente | Statut::EnCours) => refus(StatusCode::CONFLICT, "pas_pret", None),
        _ => introuvable(),
    }
}

async fn apercu_decoupe(State(etat): State<Partage>, Chemin((id, page)): Chemin<(Uuid, usize)>) -> Response {
    match servir(&etat.config.donnees, id, page, |rep, _| Some(fichier_svg(rep, page))).await {
        Some(o) => ([(header::CONTENT_TYPE, "image/svg+xml")], o).into_response(),
        None => page_absente(&etat.config.donnees, id),
    }
}

/// HP-GL du rendu (0,01 mm) tel que la machine le reçoit : au pas de la machine, décalé ;
/// pour le Kona, mis en forme comme par CutServer (lecture du cadre s'il y en a un).
fn pour_la_machine(hpgl: &str, t: &Travail, p: &Page, m: &machines::Machine) -> String {
    let reglage = t.demande.parametre_decoupe.as_ref().and_then(|n| m.parametres_decoupe.iter().find(|x| &x.nom == n)).or(m.parametres_decoupe.first());
    let mut texte = machines::changer_de_pas(hpgl, m.pas_mm(), m.decalage_mm);
    if let Some(r) = reglage.filter(|r| !r.motif_mm.is_empty()) {
        let motif: Vec<[f64; 2]> = r.motif_mm.iter().map(|p| p.map(|v| v / m.pas_mm())).collect();
        texte = machines::pointiller(&texte, &motif, r.diffusion);
    }
    if m.modele != machines::Modele::MutohKona {
        return texte;
    }
    let u = |v: f64| (v * 100.0 * 25.4 / 72.0).round() as i64;
    machines::pour_kona(&texte, p.cadre_pt.map(|c| c.map(u)), reglage, m.decoupe_retournee)
}

#[derive(Deserialize)]
struct ChoixMachine {
    #[serde(default)]
    machine: Option<String>,
}

/// Avec un cadre à code-barres, nommé pour Mutoh CutServer : `Nom_SF|MF_<numéro>.plt`.
/// `?machine=<id>` : le fichier tel que cette machine le reçoit (à envoyer à la main).
async fn fichier_decoupe(State(etat): State<Partage>, Chemin((id, page)): Chemin<(Uuid, usize)>, axum::extract::Query(q): axum::extract::Query<ChoixMachine>) -> Response {
    let cutserver = lire_page(&etat.config.donnees, id, page).and_then(|(t, _)| {
        let mot = t.demande.reperage.code_cutserver()?;
        let base: String = std::path::Path::new(&t.nom).file_stem()?.to_string_lossy().chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '-' }).collect();
        Some(format!("{base}_{mot}_{:011}.plt", reperage::numero(id, page)))
    });
    let machine = match q.machine.as_deref().map(|m| machine(&etat, m)) {
        Some(None) => return introuvable(),
        Some(m) => m,
        None => None,
    };
    match servir(&etat.config.donnees, id, page, |rep, _| Some(fichier_plt(rep, page))).await {
        Some(o) => {
            let o = match (&machine, lire_page(&etat.config.donnees, id, page)) {
                (Some(m), Some((t, p))) => pour_la_machine(&String::from_utf8_lossy(&o), &t, &p, m).into_bytes(),
                _ => o,
            };
            let nom = format!("attachment; filename=\"{}\"", cutserver.unwrap_or_else(|| format!("decoupe-{page:03}.plt")));
            ([(header::CONTENT_TYPE, "application/vnd.hp-hpgl".to_string()), (header::CONTENT_DISPOSITION, nom)], o).into_response()
        }
        None => page_absente(&etat.config.donnees, id),
    }
}

async fn apercu(State(etat): State<Partage>, Chemin((id, page)): Chemin<(Uuid, usize)>) -> Response {
    match servir(&etat.config.donnees, id, page, |rep, _| Some(render::chemin_composite(rep, page))).await {
        Some(o) => png(o),
        None => page_absente(&etat.config.donnees, id),
    }
}

async fn plaque(State(etat): State<Partage>, Chemin((id, page, nom)): Chemin<(Uuid, usize, String)>) -> Response {
    // Le nom doit être une plaque connue de la page : jamais un chemin fourni par le client.
    let chemin = |rep: &std::path::Path, p: &Page| p.plaques.contains(&nom).then(|| render::chemin_apercu_plaque(rep, page, &nom));
    match servir(&etat.config.donnees, id, page, chemin).await {
        Some(o) => png(o),
        None => page_absente(&etat.config.donnees, id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn purge_des_vieux_travaux_inactifs() {
        let jour = 86_400;
        let t = |age_jours: u64, statut: Statut| Travail {
            id: Uuid::new_v4(),
            nom: String::new(),
            format: Format::Pdf,
            demande: Demande::default(),
            statut,
            code_erreur: None,
            erreur: None,
            pages: vec![],
            alertes: vec![],
            cree_le: 100 * jour - age_jours * jour,
        };
        let travaux = [t(31, Statut::Termine), t(29, Statut::Termine), t(40, Statut::EnCours), t(45, Statut::Echec)];
        let ids = a_purger(&travaux, 100 * jour, 30);
        assert_eq!(ids, [travaux[0].id, travaux[3].id], "vieux et inactifs seulement");
        assert!(a_purger(&travaux, 100 * jour, 0).is_empty(), "0 : jamais");
    }

    #[test]
    fn dates_du_rapport() {
        assert_eq!(date_utc(0), "1970-01-01 00:00:00");
        assert_eq!(date_utc(951_782_400), "2000-02-29 00:00:00", "année bissextile séculaire");
        assert_eq!(date_utc(1_792_805_570), "2026-10-24 01:32:50");
    }

    /// Chaque code d'erreur émis par ce fichier doit être traduit en FR et en EN dans l'écran.
    #[test]
    fn chaque_code_erreur_est_traduit_dans_les_deux_langues() {
        // Les codes viennent de ces fichiers ; une chaîne qui suit `Err(` ou `erreur(` en est un.
        let sans_tests = |s: &'static str| s.split("#[cfg(test)]").next().unwrap();
        let source = [include_str!("main.rs"), include_str!("reglages.rs"), include_str!("couleur.rs"), include_str!("file.rs"), include_str!("couverture.rs"), include_str!("blanc.rs"), include_str!("edition.rs"), include_str!("media.rs"), include_str!("machines.rs"), include_str!("supports.rs")].map(sans_tests).concat();
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
            let apres_err = avant.ends_with("Err(") || avant.ends_with("erreur(");
            if !(avant.ends_with('(') || apres_statut) {
                continue;
            }
            let reste = &source[i + 1..];
            let Some(fin) = reste.find('"') else { continue };
            let (code, apres) = (&reste[..fin], &reste[fin + 1..]);
            let est_un_code = [", ", ".into()", ".to_string()"].iter().any(|s| apres.starts_with(s)) || (apres_err && apres.starts_with(')'));
            if est_un_code && !code.is_empty() && code.chars().all(|c| c.is_ascii_lowercase() || c == '_') && !codes.contains(&code) {
                codes.push(code);
            }
        }
        for code in ["introuvable", "format_inconnu", "ai_sans_pdf", "rendu_echoue", "interrompu", "dpi_invalide", "rien_a_decouper", "sortie_echouee", "action_impossible"] {
            assert!(codes.contains(&code), "le balayage n'a pas trouvé {code} : il est cassé");
        }
        for code in codes {
            assert!(fr.contains(&format!("{code}:")), "{code} non traduit en français");
            assert!(en.contains(&format!("{code}:")), "{code} non traduit en anglais");
        }
    }

    mod controle_d_acces {
        use super::*;
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        pub(super) fn etat(nom: &str) -> Partage {
            let donnees = std::env::temp_dir().join(format!("eko-rip-acces-{nom}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&donnees);
            std::fs::create_dir_all(donnees.join("travaux")).unwrap();
            let config = Config {
                profils: donnees.join("profils"),
                donnees,
                gs: "gs".into(),
                max_octets: 1 << 20,
                delai: Duration::from_secs(5),
                conservation_jours: 0,
                seuil_libre: 0,
                plotter: "/dev/null".into(),
                plotter_stty: String::new(),
                epson_snmp: None,
                acces_navigateur: false,
            };
            let machines = std::sync::RwLock::new(machines::catalogue_initial(&config.plotter, &config.plotter_stty, None));
            Arc::new(Etat { config, rendu: Semaphore::new(1), file: Default::default(), reglages: Default::default(), sessions: Default::default(), machines })
        }

        pub(super) async fn appel(e: &Partage, methode: &str, uri: &str, cookie: Option<&str>, corps: Option<serde_json::Value>) -> Response {
            let mut r = Request::builder().method(methode).uri(uri).header(header::USER_AGENT, "Mozilla/5.0 EKO-RIP-Client/0.9");
            if let Some(c) = cookie {
                r = r.header(header::COOKIE, c);
            }
            let corps = match corps {
                Some(v) => {
                    r = r.header(header::CONTENT_TYPE, "application/json");
                    Body::from(v.to_string())
                }
                None => Body::empty(),
            };
            routes(e.clone()).oneshot(r.body(corps).unwrap()).await.unwrap()
        }

        /// Un navigateur ne voit qu'une page d'état ; l'API et l'interface sont aux clients.
        #[tokio::test]
        async fn navigateur_page_simple_client_interface() {
            let e = etat("navigateur");
            let navigateur = |uri: &str| Request::builder().uri(uri).header(header::USER_AGENT, "Mozilla/5.0 Safari/605").header(header::HOST, "192.168.10.122:8640").body(Body::empty()).unwrap();
            let r = routes(e.clone()).oneshot(navigateur("/")).await.unwrap();
            let page = String::from_utf8(axum::body::to_bytes(r.into_body(), 1 << 20).await.unwrap().to_vec()).unwrap();
            assert!(page.contains("http://192.168.10.122:8640") && page.contains("Epson SC-S80600"), "page d'état : adresse et machines");
            assert!(!page.contains("id=\"envoi\""), "pas d'interface de travail");
            let r = routes(e.clone()).oneshot(navigateur("/api/travaux")).await.unwrap();
            assert_eq!(r.status(), StatusCode::FORBIDDEN);
            let r = appel(&e, "GET", "/", None, None).await;
            let page = String::from_utf8(axum::body::to_bytes(r.into_body(), 1 << 22).await.unwrap().to_vec()).unwrap();
            assert!(page.contains("id=\"envoi\""), "le client reçoit l'interface");
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        #[tokio::test]
        async fn sans_utilisateur_l_acces_reste_libre() {
            let e = etat("libre");
            assert_eq!(appel(&e, "GET", "/api/travaux", None, None).await.status(), StatusCode::OK);
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        #[tokio::test]
        async fn api_fermee_sans_session_ouverte_avec() {
            let e = etat("ferme");
            acces::ecrire(&e.config.donnees, &[acces::Utilisateur { role: acces::Role::Admin, nom: "mathieu".into(), hachage: acces::hacher("phrase-de-passe-longue").unwrap() }]).unwrap();
            assert_eq!(appel(&e, "GET", "/api/travaux", None, None).await.status(), StatusCode::UNAUTHORIZED);
            assert_eq!(appel(&e, "GET", "/api/travaux", Some("eko_rip_session=invente"), None).await.status(), StatusCode::UNAUTHORIZED);
            assert_eq!(appel(&e, "GET", "/", None, None).await.status(), StatusCode::OK, "la page de connexion reste servie");

            let faux = serde_json::json!({ "nom": "mathieu", "mot_de_passe": "faux" });
            assert_eq!(appel(&e, "POST", "/api/session", None, Some(faux)).await.status(), StatusCode::UNAUTHORIZED);

            let bon = serde_json::json!({ "nom": "mathieu", "mot_de_passe": "phrase-de-passe-longue" });
            let r = appel(&e, "POST", "/api/session", None, Some(bon)).await;
            assert_eq!(r.status(), StatusCode::NO_CONTENT);
            let cookie = r.headers()[header::SET_COOKIE].to_str().unwrap().to_string();
            assert!(cookie.contains("HttpOnly") && cookie.contains("SameSite=Strict"), "{cookie}");
            let cookie = cookie.split(';').next().unwrap().to_string();
            assert_eq!(appel(&e, "GET", "/api/travaux", Some(&cookie), None).await.status(), StatusCode::OK);

            // Utilisateur retiré du fichier : sa session ne vaut plus rien.
            acces::ecrire(&e.config.donnees, &[acces::Utilisateur { role: acces::Role::Admin, nom: "autre".into(), hachage: acces::hacher("x-x-x-x-x-x").unwrap() }]).unwrap();
            assert_eq!(appel(&e, "GET", "/api/travaux", Some(&cookie), None).await.status(), StatusCode::UNAUTHORIZED);
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// Comptes abîmés par une coupure : la version précédente est reprise ; tout illisible,
        /// l'accès reste FERMÉ (avant, une liste illisible valait « aucun compte » = accès libre).
        #[tokio::test]
        async fn comptes_illisibles_ne_ouvrent_pas_l_acces() {
            let e = etat("comptes-abimes");
            let u = |nom: &str| acces::Utilisateur { role: acces::Role::Admin, nom: nom.into(), hachage: acces::hacher("phrase-de-passe-longue").unwrap() };
            acces::ecrire(&e.config.donnees, &[u("mathieu")]).unwrap();
            acces::ecrire(&e.config.donnees, &[u("mathieu"), u("atelier")]).unwrap();
            let f = acces::fichier(&e.config.donnees);
            std::fs::write(&f, b"[{\"nom\": \"mathi").unwrap();
            assert_eq!(acces::lire(&e.config.donnees).len(), 1, "N-1 reprise");
            assert_eq!(appel(&e, "GET", "/api/travaux", None, None).await.status(), StatusCode::UNAUTHORIZED);
            std::fs::write(etat::precedent(&f), b"").unwrap();
            assert!(!acces::libre(&e.config.donnees));
            assert_eq!(appel(&e, "GET", "/api/travaux", None, None).await.status(), StatusCode::UNAUTHORIZED);
            let session: serde_json::Value = serde_json::from_slice(&axum::body::to_bytes(appel(&e, "GET", "/api/session", None, None).await.into_body(), 1 << 20).await.unwrap()).unwrap();
            assert_eq!(session["acces_libre"], false, "{session}");
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// L'opérateur travaille (travaux, file, planches) mais ne règle pas le serveur.
        #[tokio::test]
        async fn l_operateur_ne_regle_pas_le_serveur() {
            let e = etat("roles");
            let compte = |nom: &str, role| acces::Utilisateur { nom: nom.into(), hachage: acces::hacher("phrase-de-passe-longue").unwrap(), role };
            acces::ecrire(&e.config.donnees, &[compte("chef", acces::Role::Admin), compte("op", acces::Role::Operateur)]).unwrap();
            let mut cookies = vec![];
            for nom in ["chef", "op"] {
                let r = appel(&e, "POST", "/api/session", None, Some(serde_json::json!({ "nom": nom, "mot_de_passe": "phrase-de-passe-longue" }))).await;
                cookies.push(r.headers()[header::SET_COOKIE].to_str().unwrap().split(';').next().unwrap().to_string());
            }
            let (chef, op) = (cookies[0].as_str(), cookies[1].as_str());
            let session: serde_json::Value = serde_json::from_slice(&axum::body::to_bytes(appel(&e, "GET", "/api/session", Some(op), None).await.into_body(), 1 << 16).await.unwrap()).unwrap();
            assert_eq!(session["role"], "operateur");
            let reglage = serde_json::json!({ "nom": "Vinyle", "demande": {} });
            for (methode, chemin, corps) in [("POST", "/api/reglages", Some(reglage.clone())), ("DELETE", "/api/reglages/Vinyle", None), ("POST", "/api/tarifs", Some(serde_json::json!({})))] {
                assert_eq!(appel(&e, methode, chemin, Some(op), corps).await.status(), StatusCode::FORBIDDEN, "{methode} {chemin}");
            }
            assert_eq!(appel(&e, "GET", "/api/machines", Some(op), None).await.status(), StatusCode::OK, "lecture permise");
            assert_eq!(appel(&e, "GET", "/api/file", Some(op), None).await.status(), StatusCode::OK);
            assert_eq!(appel(&e, "POST", "/api/reglages", Some(chef), Some(reglage)).await.status(), StatusCode::CREATED, "l'administrateur règle");
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// Sauvegarde puis restauration sur un serveur neuf ; une sauvegarde fausse n'écrit rien.
        #[tokio::test]
        async fn sauvegarde_et_restauration_de_la_configuration() {
            let (a, b) = (etat("sauve-a"), etat("sauve-b"));
            let icc = std::fs::read(std::fs::read_dir(couleur::tests::profils_ghostscript()).unwrap().flatten().map(|e| e.path()).find(|p| p.extension().is_some_and(|x| x == "icc")).unwrap()).unwrap();
            std::fs::create_dir_all(&a.config.profils).unwrap();
            std::fs::write(a.config.profils.join("atelier.icc"), &icc).unwrap();
            let reglage = serde_json::json!({ "nom": "Vinyle", "demande": { "dpi": 360 } });
            assert_eq!(appel(&a, "POST", "/api/reglages", None, Some(reglage)).await.status(), StatusCode::CREATED);
            let r = appel(&a, "GET", "/api/sauvegarde", None, None).await;
            assert_eq!(r.status(), StatusCode::OK);
            let mut s: serde_json::Value = serde_json::from_slice(&axum::body::to_bytes(r.into_body(), 1 << 26).await.unwrap()).unwrap();
            assert_eq!(s["format"], "eko-rip-sauvegarde-1");
            assert!(s.get("utilisateurs").is_none(), "jamais les comptes");

            let mut fausse = s.clone();
            fausse["profils"]["atelier.icc"] = "00ff".into();
            assert_eq!(appel(&b, "POST", "/api/sauvegarde", None, Some(fausse)).await.status(), StatusCode::BAD_REQUEST);
            assert!(reglages::lire(&b.config.donnees).is_empty() && !b.config.profils.join("atelier.icc").exists(), "rien d'écrit");

            s["machines"][0]["nom"] = "Epson de l'atelier".into();
            let r = appel(&b, "POST", "/api/sauvegarde", None, Some(s)).await;
            assert_eq!(r.status(), StatusCode::OK);
            assert_eq!(std::fs::read(b.config.profils.join("atelier.icc")).unwrap(), icc);
            assert_eq!(reglages::lire(&b.config.donnees)[0].demande.dpi, 360);
            assert_eq!(b.machines.read().unwrap()[0].nom, "Epson de l'atelier", "catalogue rechargé");
            assert_eq!(supports::lire(&b.config.donnees, "epson"), supports::lire(&a.config.donnees, "epson"), "supports et identifiants repris");
            for e in [a, b] {
                std::fs::remove_dir_all(&e.config.donnees).unwrap();
            }
        }

        /// Jeton de l'ERP : n'ouvre que /api/erp/, qui reste fermé sans lui même en accès libre.
        #[tokio::test]
        async fn jeton_erp_limite_au_pont() {
            let e = etat("jeton-erp");
            let avec = |jeton: &str, uri: &str| {
                let r = Request::builder().method("GET").uri(uri).header(header::AUTHORIZATION, format!("Bearer {jeton}")).body(Body::empty()).unwrap();
                routes(e.clone()).oneshot(r)
            };
            assert!(acces::libre(&e.config.donnees), "accès humain libre (aucun compte)");
            assert_eq!(appel(&e, "GET", "/api/erp/contrat", None, None).await.status(), StatusCode::UNAUTHORIZED, "fermé sans jeton");
            assert_eq!(avec("x", "/api/erp/contrat").await.unwrap().status(), StatusCode::UNAUTHORIZED, "aucun jeton déclaré");
            let jeton = erp::aleatoire();
            erp::ecrire(&e.config.donnees, &erp::Config { jeton_sha256: Some(erp::empreinte(&jeton)), secret_hmac: None }).unwrap();
            let r = avec(&jeton, "/api/erp/contrat").await.unwrap();
            assert_eq!(r.status(), StatusCode::OK);
            let v: serde_json::Value = serde_json::from_slice(&axum::body::to_bytes(r.into_body(), 1 << 20).await.unwrap()).unwrap();
            assert_eq!(v["contrat"], "eko-rip-erp/1");
            assert!(v["etats"]["sortie"]["etats"]["terminee"].is_string(), "{v}");
            assert_eq!(avec("faux", "/api/erp/contrat").await.unwrap().status(), StatusCode::UNAUTHORIZED);
            assert_eq!(avec(&jeton, "/api/tarifs").await.unwrap().status(), StatusCode::FORBIDDEN, "le jeton n'ouvre pas le reste");
            let empreinte = std::fs::read_to_string(e.config.donnees.join("erp.json")).unwrap();
            assert!(!empreinte.contains(&jeton), "le jeton n'est jamais gardé en clair");
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// Seuil d'espace libre (roadmap 0.6) : refus anticipé, ordre de purge.
        #[tokio::test]
        async fn seuil_d_espace_libre() {
            assert_eq!(place_estimee(1.0, 600), 1_000_000_000);
            assert_eq!(place_estimee(1.0, 300), 250_000_000, "au carré de la résolution");
            let t = |cree_le: u64, statut: &str| serde_json::from_value::<Travail>(serde_json::json!({ "id": Uuid::new_v4(), "nom": "x", "statut": statut, "erreur": null, "cree_le": cree_le, "pages": [] })).unwrap();
            let v = [t(30, "termine"), t(10, "en_cours"), t(20, "echec"), t(40, "termine")];
            assert_eq!(plus_vieux_inactifs(&v), [v[2].id, v[0].id, v[3].id], "du plus vieux au plus récent, jamais un actif");
            // Seuil impossible à tenir : l'envoi est refusé avant tout rendu.
            let e = etat("seuil");
            let mut config = e.config.clone();
            config.seuil_libre = u64::MAX / 2;
            let e2: Partage = Arc::new(Etat { config, rendu: Semaphore::new(1), file: Default::default(), reglages: Default::default(), sessions: Default::default(), machines: std::sync::RwLock::new(e.machines.read().unwrap().clone()) });
            let id = Uuid::new_v4();
            let rep = dossier(&e2.config.donnees, id);
            std::fs::create_dir_all(&rep).unwrap();
            let pdf = {
                use lopdf::{Document, Object, Stream, dictionary};
                let mut doc = Document::with_version("1.4");
                let contenu = doc.add_object(Stream::new(dictionary! {}, b"0 0 1 rg 0 0 283 283 re f".to_vec()));
                let pages = doc.new_object_id();
                let page = doc.add_object(dictionary! { "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(), 0.into(), 283.into(), 283.into()], "Contents" => contenu });
                doc.objects.insert(pages, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }));
                let catalogue = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
                doc.trailer.set("Root", catalogue);
                let mut o = vec![];
                doc.save_to(&mut o).unwrap();
                o
            };
            std::fs::write(rep.join("source.tmp"), pdf).unwrap();
            let r = creer_travail(&e2, id, "x.pdf".into(), Format::Pdf, Demande::default()).await;
            assert!(matches!(r, Err((StatusCode::INSUFFICIENT_STORAGE, ("espace_insuffisant", _)))), "{:?}", r.err().map(|e| e.1));
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// Page d'un travail en cours de rendu : 409 pas_pret ; travail inconnu : 404 (roadmap 0.5).
        #[tokio::test]
        async fn page_pas_prete_distincte_d_inexistante() {
            let e = etat("pas-pret");
            let id = Uuid::new_v4();
            std::fs::create_dir_all(dossier(&e.config.donnees, id)).unwrap();
            let t = |statut: &str| serde_json::from_value::<Travail>(serde_json::json!({ "id": id, "nom": "x", "statut": statut, "erreur": null, "cree_le": 0, "pages": [] })).unwrap();
            ecrire(&e.config.donnees, &t("en_cours")).unwrap();
            for uri in ["apercu.png", "decoupe.svg", "decoupe.plt", "plaques/Cyan"] {
                let r = appel(&e, "GET", &format!("/api/travaux/{id}/pages/1/{uri}"), None, None).await;
                assert_eq!(r.status(), StatusCode::CONFLICT, "{uri}");
            }
            ecrire(&e.config.donnees, &t("termine")).unwrap();
            assert_eq!(appel(&e, "GET", &format!("/api/travaux/{id}/pages/1/apercu.png"), None, None).await.status(), StatusCode::NOT_FOUND, "fini sans cette page");
            assert_eq!(appel(&e, "GET", &format!("/api/travaux/{}/pages/1/apercu.png", Uuid::new_v4()), None, None).await.status(), StatusCode::NOT_FOUND);
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// Raison d'attente de chaque sortie (roadmap 0.4) ; une sortie bloquée ne part pas.
        #[test]
        fn raisons_d_attente() {
            let e = etat("raisons");
            let d = e.config.donnees.clone();
            let m = |laize_mm, port: &str| machines::Machine { laize_mm, connexion: machines::Connexion::Serie { port: port.into(), stty: String::new() }, ..e.machines.read().unwrap()[1].clone() };
            let id = Uuid::new_v4();
            let rep = dossier(&d, id);
            std::fs::create_dir_all(rep.join("prod")).unwrap();
            let t = serde_json::json!({ "id": id, "nom": "essai", "statut": "termine", "erreur": null, "cree_le": 0,
                "pages": [{ "plaques": [], "traces_decoupe": 1, "largeur_mm": 500.0, "hauteur_mm": 300.0 }] });
            ecrire(&d, &serde_json::from_value::<Travail>(t).unwrap()).unwrap();
            let mut s = file::Sortie::nouvelle("kona", machines::Operation::Decoupe, id, "essai", 1, 0);
            let present = d.join("port-present");
            std::fs::write(&present, b"").unwrap();
            let p = present.to_str().unwrap();
            assert_eq!(raison_attente(&d, &s, None, None), Some("machine_hors_ligne"), "machine absente du catalogue");
            assert_eq!(raison_attente(&d, &s, Some(&m(1650.0, "/dev/inexistant")), None), Some("machine_hors_ligne"));
            assert_eq!(raison_attente(&d, &s, Some(&m(1650.0, p)), None), Some("source_absente"), "pas de tracé");
            std::fs::write(fichier_plt(&rep, 1), b"IN;").unwrap();
            assert_eq!(raison_attente(&d, &s, Some(&m(1650.0, p)), None), None);
            assert_eq!(raison_attente(&d, &s, Some(&m(400.0, p)), None), Some("format_trop_petit"));
            s.etat = file::Etat::Suspendue;
            assert_eq!(raison_attente(&d, &s, Some(&m(1650.0, p)), None), Some("attente_operateur"));
            s.etat = file::Etat::Terminee;
            assert_eq!(raison_attente(&d, &s, None, None), None, "une sortie finie n'attend rien");
            let mut f = file::File::default();
            let mut bloquee = file::Sortie::nouvelle("kona", machines::Operation::Decoupe, Uuid::new_v4(), "absente", 1, 0);
            bloquee.etat = file::Etat::EnAttente;
            let prete = file::Sortie::nouvelle("kona", machines::Operation::Decoupe, id, "essai", 1, 0);
            f.sorties = vec![bloquee, prete];
            let machine = m(1650.0, p);
            assert_eq!(f.prochaine_si("kona", |x| raison_attente(&d, x, Some(&machine), None).is_none()), Some(1), "la bloquée garde sa place, la suivante part");
            // Attente de support (roadmap 1.4) : impression d'un travail sur le support A.
            let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
            let mut tr = lire_travail(&d, id).unwrap();
            tr.demande.support_uuid = Some(a);
            ecrire(&d, &tr).unwrap();
            let epson = machines::Machine { connexion: machines::Connexion::Dossier, ..m(1650.0, p) };
            let imp = file::Sortie::nouvelle("epson", machines::Operation::Impression, id, "essai", 1, 0);
            let charge = |u: Uuid, laize: Option<f64>| supports::Charge { support_uuid: Some(u), laize_mm: laize, source: "machine".into(), le: 0 };
            assert_eq!(raison_attente(&d, &imp, Some(&epson), Some(&charge(b, None))), Some("attente_support"), "B chargé, A voulu");
            assert_eq!(raison_attente(&d, &imp, Some(&epson), Some(&charge(a, None))), None, "A chargé : elle part");
            assert_eq!(raison_attente(&d, &imp, Some(&epson), Some(&charge(a, Some(400.0)))), Some("format_trop_petit"), "rouleau chargé de 400 mm pour une page de 500");
            assert_eq!(raison_attente(&d, &imp, Some(&epson), None), None, "support chargé inconnu : on n'arrête pas l'atelier");
            std::fs::remove_dir_all(&d).unwrap();
        }

        /// Identifiants immuables des machines, couches et réglages (pont ERP, lot R1).
        #[tokio::test]
        async fn identifiants_immuables_machines_couches_reglages() {
            let e = etat("identifiants");
            let corps = |r: Response| async move { serde_json::from_slice::<serde_json::Value>(&axum::body::to_bytes(r.into_body(), 1 << 20).await.unwrap()).unwrap() };
            // Premier enregistrement, comme l'écran : sans uuid → attribués.
            let mut v = corps(appel(&e, "GET", "/api/machines", None, None).await).await;
            let sans = |v: &mut serde_json::Value| {
                for m in v.as_array_mut().unwrap() {
                    m.as_object_mut().unwrap().remove("uuid");
                    for c in m["parametres_decoupe"].as_array_mut().unwrap() {
                        c.as_object_mut().unwrap().remove("uuid");
                    }
                }
            };
            sans(&mut v);
            assert_eq!(appel(&e, "POST", "/api/machines", None, Some(v.clone())).await.status(), StatusCode::OK);
            let w = corps(appel(&e, "GET", "/api/machines", None, None).await).await;
            let kona = w.as_array().unwrap().iter().find(|m| m["id"] == "kona").unwrap().clone();
            assert!(kona["uuid"].is_string() && kona["parametres_decoupe"][0]["uuid"].is_string(), "{kona}");
            // Renvoyé sans uuid (ancien client) : même clé texte → même uuid.
            assert_eq!(appel(&e, "POST", "/api/machines", None, Some(v)).await.status(), StatusCode::OK);
            assert_eq!(corps(appel(&e, "GET", "/api/machines", None, None).await).await, w, "rien ne change");
            // Renommer l'id texte en gardant l'uuid : l'uuid suit.
            let mut x = w.clone();
            let k = x.as_array_mut().unwrap().iter_mut().find(|m| m["id"] == "kona").unwrap();
            k["id"] = "kona-1".into();
            k["parametres_decoupe"][0]["nom"] = "Mi-chair renommée".into();
            assert_eq!(appel(&e, "POST", "/api/machines", None, Some(x.clone())).await.status(), StatusCode::OK);
            let y = corps(appel(&e, "GET", "/api/machines", None, None).await).await;
            let k = y.as_array().unwrap().iter().find(|m| m["id"] == "kona-1").unwrap();
            assert_eq!((k["uuid"].clone(), k["parametres_decoupe"][0]["uuid"].clone()), (kona["uuid"].clone(), kona["parametres_decoupe"][0]["uuid"].clone()));
            // Uuid inventé : refusé, rien d'écrit.
            x[0]["uuid"] = Uuid::new_v4().to_string().into();
            let r = appel(&e, "POST", "/api/machines", None, Some(x)).await;
            assert_eq!(r.status(), StatusCode::BAD_REQUEST);
            assert_eq!(corps(r).await["code"], "identifiant_inconnu");
            assert_eq!(corps(appel(&e, "GET", "/api/machines", None, None).await).await, y);
            // Réglage réenregistré sous le même nom : même uuid.
            let reglage = serde_json::json!({ "nom": "Vinyle", "demande": { "dpi": 360 } });
            let a = corps(appel(&e, "POST", "/api/reglages", None, Some(reglage.clone())).await).await;
            let b = corps(appel(&e, "POST", "/api/reglages", None, Some(reglage)).await).await;
            assert!(a["uuid"].is_string());
            assert_eq!(a["uuid"], b["uuid"]);
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// Support chargé (roadmap 1.3) : saisi par l'opérateur, qui prime ; retour à la machine.
        #[tokio::test]
        async fn support_charge_saisi() {
            let e = etat("charge");
            let corps = |r: Response| async move { serde_json::from_slice::<serde_json::Value>(&axum::body::to_bytes(r.into_body(), 1 << 20).await.unwrap()).unwrap() };
            assert_eq!(corps(appel(&e, "GET", "/api/machines/epson/charge", None, None).await).await, serde_json::Value::Null);
            let s = supports::lire(&e.config.donnees, "epson").supports[0].id.unwrap();
            let r = appel(&e, "POST", "/api/machines/epson/charge", None, Some(serde_json::json!({ "support_uuid": s }))).await;
            assert_eq!(r.status(), StatusCode::OK);
            let v = corps(appel(&e, "GET", "/api/machines/epson/charge", None, None).await).await;
            assert_eq!((v["support_uuid"].as_str(), v["source"].as_str()), (Some(s.to_string().as_str()), Some("saisie")));
            assert_eq!(appel(&e, "POST", "/api/machines/epson/charge", None, Some(serde_json::json!({ "support_uuid": Uuid::new_v4() }))).await.status(), StatusCode::BAD_REQUEST);
            assert_eq!(appel(&e, "POST", "/api/machines/kona/charge", None, Some(serde_json::json!({ "support_uuid": s }))).await.status(), StatusCode::BAD_REQUEST, "support d'une autre machine");
            let v = corps(appel(&e, "POST", "/api/machines/epson/charge", None, Some(serde_json::json!({ "support_uuid": null }))).await).await;
            assert_eq!(v["source"], "machine", "retour à la lecture par la machine");
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// Supports par machine : repris de tarifs.json, identifiants stables, contrôlés.
        #[tokio::test]
        async fn supports_et_modes_par_machine() {
            let e = etat("supports");
            let corps = |r: Response| async move { serde_json::from_slice::<serde_json::Value>(&axum::body::to_bytes(r.into_body(), 1 << 20).await.unwrap()).unwrap() };
            let v = corps(appel(&e, "GET", "/api/machines/epson/supports", None, None).await).await;
            let premier = v[0].clone();
            assert_eq!(premier["nom"], couverture::lire(&e.config.donnees).supports[0].nom.as_str(), "{v}");
            assert!(premier["id"].is_string() && premier["modes"][0]["id"].is_string(), "{v}");
            assert_eq!(corps(appel(&e, "GET", "/api/machines/epson/supports", None, None).await).await, v, "identifiants stables d'une lecture à l'autre");

            let mut renomme = v.clone();
            renomme[0]["nom"] = "Vinyle mat".into();
            renomme.as_array_mut().unwrap().push(serde_json::json!({ "machine": "x", "nom": "Bâche", "encres": ["C", "M", "Y", "BK"], "modes": [{ "nom": "360", "dpi": [360, 360], "passes": 4 }] }));
            let r = appel(&e, "POST", "/api/machines/epson/supports", None, Some(renomme)).await;
            assert_eq!(r.status(), StatusCode::OK);
            let w = corps(r).await;
            assert_eq!((w[0]["id"].clone(), w[0]["nom"].clone()), (premier["id"].clone(), "Vinyle mat".into()), "renommer garde l'identifiant");
            assert_eq!(w[1]["machine"], "epson");
            assert!(w[1]["id"].is_string());
            assert_eq!(corps(appel(&e, "GET", "/api/machines/kona/supports", None, None).await).await, serde_json::json!([]));

            let mut invente = w.clone();
            invente[0]["id"] = Uuid::new_v4().to_string().into();
            assert_eq!(appel(&e, "POST", "/api/machines/epson/supports", None, Some(invente)).await.status(), StatusCode::BAD_REQUEST);
            let mut casse = w.clone();
            casse[1]["encres"] = serde_json::json!(["WH", "MS"]);
            assert_eq!(appel(&e, "POST", "/api/machines/epson/supports", None, Some(casse)).await.status(), StatusCode::BAD_REQUEST);
            assert_eq!(appel(&e, "GET", "/api/machines/inconnue/supports", None, None).await.status(), StatusCode::NOT_FOUND);
            assert_eq!(corps(appel(&e, "GET", "/api/machines/epson/supports", None, None).await).await, w, "un refus n'écrit rien");
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        #[tokio::test]
        async fn deconnexion_et_blocage() {
            let e = etat("blocage");
            acces::ecrire(&e.config.donnees, &[acces::Utilisateur { role: acces::Role::Admin, nom: "mathieu".into(), hachage: acces::hacher("phrase-de-passe-longue").unwrap() }]).unwrap();
            let bon = serde_json::json!({ "nom": "mathieu", "mot_de_passe": "phrase-de-passe-longue" });
            let r = appel(&e, "POST", "/api/session", None, Some(bon.clone())).await;
            let cookie = r.headers()[header::SET_COOKIE].to_str().unwrap().split(';').next().unwrap().to_string();
            assert_eq!(appel(&e, "DELETE", "/api/session", Some(&cookie), None).await.status(), StatusCode::NO_CONTENT);
            assert_eq!(appel(&e, "GET", "/api/travaux", Some(&cookie), None).await.status(), StatusCode::UNAUTHORIZED);
            for _ in 0..5 {
                appel(&e, "POST", "/api/session", None, Some(serde_json::json!({ "nom": "mathieu", "mot_de_passe": "faux" }))).await;
            }
            assert_eq!(appel(&e, "POST", "/api/session", None, Some(bon)).await.status(), StatusCode::TOO_MANY_REQUESTS);
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// Une page mise en file Epson sort ses plaques d'encre, jamais ses tons de découpe ;
        /// le travail ne peut pas être supprimé tant que la sortie est à passer.
        #[tokio::test]
        async fn file_epson_de_bout_en_bout() {
            let e = etat("file");
            let donnees = e.config.donnees.clone();
            let t = Travail {
                id: Uuid::new_v4(),
                nom: "essai.pdf".into(),
                format: Format::Pdf,
                demande: Demande::default(),
                statut: Statut::Termine,
                code_erreur: None,
                erreur: None,
                pages: vec![Page { plaques: vec!["Cyan".into(), "CutContour".into()], plaques_decoupe: vec!["CutContour".into()], ..Default::default() }],
                alertes: vec![],
                cree_le: maintenant(),
            };
            let rep = dossier(&donnees, t.id);
            std::fs::create_dir_all(rep.join("prod")).unwrap();
            ecrire(&donnees, &t).unwrap();
            for nom in ["Cyan", "CutContour"] {
                std::fs::write(render::chemin_plaque(&rep, 1, nom), nom).unwrap();
            }

            let kona = serde_json::json!({ "travail": t.id, "page": 1, "machine": "kona" });
            assert_eq!(appel(&e, "POST", "/api/file", None, Some(kona)).await.status(), StatusCode::CONFLICT, "rien à découper");
            let epson = serde_json::json!({ "travail": t.id, "page": 1, "machine": "epson" });
            let r = appel(&e, "POST", "/api/file", None, Some(epson)).await;
            assert_eq!(r.status(), StatusCode::CREATED);
            let id = e.file.lock().unwrap().sorties[0].id;
            assert_eq!(appel(&e, "DELETE", &format!("/api/travaux/{}", t.id), None, None).await.status(), StatusCode::CONFLICT, "sortie à passer");
            assert_eq!(appel(&e, "POST", &format!("/api/file/{id}/inventee"), None, None).await.status(), StatusCode::NOT_FOUND);

            tokio::spawn(repartir(e.clone()));
            for _ in 0..50 {
                if e.file.lock().unwrap().sorties[0].etat == file::Etat::Terminee {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            let s = e.file.lock().unwrap().sorties[0].clone();
            assert_eq!(s.etat, file::Etat::Terminee, "{s:?}");
            let sortie = file::dossier_sortie(&donnees, &s);
            assert_eq!(std::fs::read_to_string(sortie.join("C.tif")).unwrap(), "Cyan", "une plaque par canal Epson");
            let envoi: serde_json::Value = serde_json::from_slice(&std::fs::read(sortie.join("envoi.json")).unwrap()).unwrap();
            assert_eq!(envoi, serde_json::json!({ "canaux": [["C", "Cyan"]], "couches": "dessous" }));
            assert_eq!(std::fs::read_dir(&sortie).unwrap().count(), 2, "un ton de découpe ne part jamais à l'imprimante");
            assert_eq!(file::File::lire(&donnees).sorties, [s], "file écrite sur le disque");

            // Réimpression sans nouveau rendu, puis suppression : l'historique part avec le travail.
            assert_eq!(appel(&e, "POST", &format!("/api/file/{id}/refaire"), None, None).await.status(), StatusCode::NO_CONTENT);
            let copie = e.file.lock().unwrap().sorties[1].id;
            assert_eq!(appel(&e, "POST", &format!("/api/file/{copie}/annuler"), None, None).await.status(), StatusCode::NO_CONTENT);
            assert_eq!(appel(&e, "DELETE", &format!("/api/travaux/{}", t.id), None, None).await.status(), StatusCode::NO_CONTENT);
            assert!(e.file.lock().unwrap().sorties.is_empty());
            assert!(!sortie.exists(), "les plaques sorties partent avec le travail");
            std::fs::remove_dir_all(&donnees).unwrap();
        }

        /// BAT d'un travail rendu : PDF A4 paysage, une page par page du travail, refusé tant
        /// que le rendu n'est pas fini.
        #[tokio::test]
        async fn bat_d_un_travail() {
            let e = etat("bat");
            let donnees = e.config.donnees.clone();
            let t = Travail {
                id: Uuid::new_v4(),
                nom: "étiquette.pdf".into(),
                format: Format::Pdf,
                demande: Demande { dpi: 36, ..Default::default() },
                statut: Statut::EnAttente,
                code_erreur: None,
                erreur: None,
                pages: vec![],
                alertes: vec![],
                cree_le: maintenant(),
            };
            let rep = dossier(&donnees, t.id);
            std::fs::create_dir_all(&rep).unwrap();
            std::fs::write(rep.join("source.pdf"), imbrication::tests::pdf(144.0, 72.0, "1 0 0 0", (0.0, 0.0))).unwrap();
            ecrire(&donnees, &t).unwrap();
            assert_eq!(appel(&e, "GET", &format!("/api/travaux/{}/bat.pdf", t.id), None, None).await.status(), StatusCode::CONFLICT);
            traiter(e.clone(), t.clone()).await;
            let r = appel(&e, "GET", &format!("/api/travaux/{}/bat.pdf?langue=en", t.id), None, None).await;
            assert_eq!(r.status(), StatusCode::OK);
            assert_eq!(r.headers()[header::CONTENT_TYPE], "application/pdf");
            assert!(r.headers()[header::CONTENT_DISPOSITION].to_str().unwrap().contains("BAT-_tiquette.pdf"));
            let pdf = axum::body::to_bytes(r.into_body(), 1 << 26).await.unwrap();
            let doc = lopdf::Document::load_mem(&pdf).unwrap();
            assert_eq!(doc.get_pages().len(), 1);
            assert!(!rep.join("bat").exists(), "fichiers de travail effacés");
            std::fs::remove_dir_all(&donnees).unwrap();
        }

        /// Catalogue : une Graphtec en réseau reçoit la découpe au pas de 0,025 mm (le RIP
        /// produit du 0,01 mm), et une sortie vers l'Epson ne peut pas être une découpe.
        #[tokio::test]
        async fn decoupe_graphtec_par_le_reseau() {
            let e = etat("graphtec");
            let donnees = e.config.donnees.clone();
            let ecoute = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let port = ecoute.local_addr().unwrap().port();
            let recu = std::thread::spawn(move || {
                let (mut f, _) = ecoute.accept().unwrap();
                let mut o = String::new();
                std::io::Read::read_to_string(&mut f, &mut o).unwrap();
                o
            });
            let mut catalogue = e.machines.read().unwrap().clone();
            catalogue.push(machines::Machine {
                id: "fc7000".into(),
                uuid: None,
                nom: "Graphtec FC7000-75".into(),
                modele: machines::Modele::GraphtecFc7000,
                laize_mm: 750.0,
                connexion: machines::Connexion::Reseau { hote: "127.0.0.1".into(), port },
                snmp: None,
                pas_mm: None,
                decalage_mm: [0.0; 2],
                m2_heure: None,
                parametres_decoupe: vec![],
                decoupe_retournee: false,
            });
            let r = appel(&e, "POST", "/api/machines", None, Some(serde_json::to_value(&catalogue).unwrap())).await;
            assert_eq!(r.status(), StatusCode::OK);
            assert!(machines::fichier(&donnees).exists());

            let t = Travail {
                id: Uuid::new_v4(),
                nom: "coupe.pdf".into(),
                format: Format::Pdf,
                demande: Demande::default(),
                statut: Statut::Termine,
                code_erreur: None,
                erreur: None,
                pages: vec![Page { traces_decoupe: 1, plaques: vec!["Cyan".into()], ..Default::default() }],
                alertes: vec![],
                cree_le: maintenant(),
            };
            let rep = dossier(&donnees, t.id);
            std::fs::create_dir_all(rep.join("prod")).unwrap();
            std::fs::write(fichier_plt(&rep, 1), "IN;SP1;PU0,0;PD2540,0;PU;SP0;").unwrap();
            ecrire(&donnees, &t).unwrap();
            let refus_epson = serde_json::json!({ "travail": t.id, "page": 1, "machine": "epson", "operation": "decoupe" });
            assert_eq!(appel(&e, "POST", "/api/file", None, Some(refus_epson)).await.status(), StatusCode::CONFLICT);
            let coupe = serde_json::json!({ "travail": t.id, "page": 1, "machine": "fc7000" });
            assert_eq!(appel(&e, "POST", "/api/file", None, Some(coupe)).await.status(), StatusCode::CREATED);
            tokio::spawn(repartir(e.clone()));
            let recu = tokio::time::timeout(Duration::from_secs(10), tokio::task::spawn_blocking(move || recu.join().unwrap())).await.expect("rien reçu en 10 s").unwrap();
            assert_eq!(recu, "IN;SP1;PU0,0;PD1016,0;PU;SP0;");
            for _ in 0..50 {
                if e.file.lock().unwrap().sorties[0].etat == file::Etat::Terminee {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            assert_eq!(e.file.lock().unwrap().sorties[0].etat, file::Etat::Terminee);
            // Retirer la machine d'une sortie encore à passer : refusé.
            e.file.lock().unwrap().sorties[0].etat = file::Etat::Suspendue;
            let sans = serde_json::to_value(&catalogue[..2]).unwrap();
            assert_eq!(appel(&e, "POST", "/api/machines", None, Some(sans)).await.status(), StatusCode::CONFLICT);
            std::fs::remove_dir_all(&donnees).unwrap();
        }

        /// Aperçu d'imposition : 3 copies de 200 × 100 mm sur 320 mm, rotation permise →
        /// trois éléments debout côte à côte, 200 mm de rouleau ; rien n'est créé.
        #[tokio::test]
        async fn apercu_d_imposition() {
            let e = etat("imposition");
            let donnees = e.config.donnees.clone();
            let mm = imbrication::PT_PAR_MM;
            let t = Travail {
                id: Uuid::new_v4(),
                nom: "etiquette.pdf".into(),
                format: Format::Pdf,
                demande: Demande::default(),
                statut: Statut::Termine,
                code_erreur: None,
                erreur: None,
                pages: vec![Page::default()],
                alertes: vec![],
                cree_le: maintenant(),
            };
            let rep = dossier(&donnees, t.id);
            std::fs::create_dir_all(&rep).unwrap();
            std::fs::write(rep.join("source.pdf"), imbrication::tests::pdf(200.0 * mm, 100.0 * mm, "1 0 0 0", (0.0, 0.0))).unwrap();
            ecrire(&donnees, &t).unwrap();
            let corps = serde_json::json!({ "laize_mm": 320.0, "espacement_mm": 5.0, "rotation": true, "elements": [{ "id": t.id, "page": 1, "copies": 3 }] });
            let r = appel(&e, "POST", "/api/planches/apercu", None, Some(corps)).await;
            assert_eq!(r.status(), StatusCode::OK);
            let v: serde_json::Value = serde_json::from_slice(&axum::body::to_bytes(r.into_body(), 1 << 20).await.unwrap()).unwrap();
            assert!((v["longueur_mm"].as_f64().unwrap() - 200.0).abs() < 1e-3, "{v}");
            let places = v["places"].as_array().unwrap();
            assert_eq!(places.len(), 3);
            assert!(places.iter().all(|p| p["tourne"] == true && (p["largeur"].as_f64().unwrap() - 100.0).abs() < 1e-3));
            assert_eq!(places[0]["nom"], "etiquette.pdf p1");
            // 3 contours de découpe (la page moins 5 pt par bord) sur 320 × 200 mm.
            let cote = |c: f64| c * mm - 10.0;
            assert!((v["utilisation"].as_f64().unwrap() - 3.0 * cote(200.0) * cote(100.0) / (320.0 * mm * 200.0 * mm)).abs() < 1e-4, "{v}");
            assert_eq!(lire_tous(&donnees).len(), 1, "rien de créé");
            std::fs::remove_dir_all(&donnees).unwrap();
        }

        /// Mise en file automatique (Epson en attente, Kona suspendu après l'impression),
        /// puis réédition à 200 % : rendu refait depuis l'original, surface × 4.
        /// Page plus large que la laize : tournée seule (alerte) ; « Réglages » sans rotation
        /// automatique et avec un autre repérage : rendu refait depuis l'original.
        #[tokio::test]
        async fn rotation_automatique_et_reglages_apres_import() {
            let e = etat("rotation");
            let donnees = e.config.donnees.clone();
            e.machines.write().unwrap()[0].laize_mm = 300.0;
            let mm = imbrication::PT_PAR_MM;
            let id = Uuid::new_v4();
            let rep = dossier(&donnees, id);
            std::fs::create_dir_all(&rep).unwrap();
            std::fs::write(rep.join("source.tmp"), imbrication::tests::pdf(400.0 * mm, 100.0 * mm, "1 0 0 0", (0.0, 0.0))).unwrap();
            let t = creer_travail(&e, id, "large.pdf".into(), Format::Pdf, Demande { dpi: 36, ..Default::default() }).await.ok().unwrap();
            assert!(t.alertes.iter().any(|a| a.code == "rotation_auto"), "{:?}", t.alertes);
            let attendre = || async {
                for _ in 0..200 {
                    let x = lire_travail(&donnees, id).unwrap();
                    if !matches!(x.statut, Statut::EnAttente | Statut::EnCours) {
                        return x;
                    }
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                panic!("rendu trop long");
            };
            let x = attendre().await;
            assert!((x.pages[0].largeur_mm - 100.0).abs() < 1.0, "tournée : {} mm de large", x.pages[0].largeur_mm);

            let corps = serde_json::json!({ "dpi": 36, "rotation_auto": false, "reperage": "manuel" });
            assert_eq!(appel(&e, "POST", &format!("/api/travaux/{id}/reglages"), None, Some(corps)).await.status(), StatusCode::ACCEPTED);
            let x = attendre().await;
            assert!(x.pages[0].largeur_mm > 400.0, "plus tournée, croix comprises : {} mm", x.pages[0].largeur_mm);
            assert!(!x.alertes.iter().any(|a| a.code == "rotation_auto"));
            assert_eq!(x.demande.reperage, reperage::Reperage::Manuel);
            std::fs::remove_dir_all(&donnees).unwrap();
        }

        /// Le travail désigne son support et son mode ; changer de mode refait le rendu à la
        /// résolution du mode (roadmap 1.2).
        #[tokio::test]
        async fn support_et_mode_du_travail() {
            let e = etat("mode-travail");
            let donnees = e.config.donnees.clone();
            let mut c = supports::lire(&donnees, "epson");
            c.supports[0].modes.push(supports::ModeImpression { id: None, nom: "Brouillon 90".into(), dpi: [90, 90], passes: 2, chauffage_c: None });
            let connus = supports::lire(&donnees, "epson").supports;
            assert!(supports::attribuer_ids(&mut c.supports, &connus));
            supports::ecrire(&donnees, &c).unwrap();
            let (support, mode_720, mode_90) = (c.supports[0].clone(), c.supports[0].modes[0].id.unwrap(), c.supports[0].modes[1].id.unwrap());
            let mm = imbrication::PT_PAR_MM;
            let id = Uuid::new_v4();
            let rep = dossier(&donnees, id);
            std::fs::create_dir_all(&rep).unwrap();
            std::fs::write(rep.join("source.tmp"), imbrication::tests::pdf(20.0 * mm, 10.0 * mm, "1 0 0 0", (0.0, 0.0))).unwrap();
            let t = creer_travail(&e, id, "petit.pdf".into(), Format::Pdf, Demande { mode_uuid: Some(mode_90), ..Default::default() }).await.ok().unwrap();
            assert_eq!((t.demande.dpi, t.demande.support_uuid, t.demande.support.clone()), (90, support.id, Some(support.nom.clone())), "support déduit du mode, résolution du mode");
            let attendre = || async {
                for _ in 0..400 {
                    let x = lire_travail(&donnees, id).unwrap();
                    if !matches!(x.statut, Statut::EnAttente | Statut::EnCours) {
                        return x;
                    }
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                panic!("rendu trop long");
            };
            assert!(matches!(attendre().await.statut, Statut::Termine));
            // Mode inconnu : refusé, le travail ne bouge pas.
            let faux = serde_json::json!({ "mode_uuid": Uuid::new_v4() });
            assert_eq!(appel(&e, "POST", &format!("/api/travaux/{id}/reglages"), None, Some(faux)).await.status(), StatusCode::BAD_REQUEST);
            // Autre mode : rendu refait à sa résolution.
            let corps = serde_json::json!({ "support_uuid": support.id, "mode_uuid": mode_720 });
            assert_eq!(appel(&e, "POST", &format!("/api/travaux/{id}/reglages"), None, Some(corps)).await.status(), StatusCode::ACCEPTED);
            let x = attendre().await;
            assert!(matches!(x.statut, Statut::Termine));
            assert_eq!((x.demande.dpi, x.demande.mode_uuid), (720, Some(mode_720)));
            std::fs::remove_dir_all(&donnees).unwrap();
        }

        #[tokio::test]
        async fn file_automatique_puis_reedition() {
            let e = etat("auto");
            let donnees = e.config.donnees.clone();
            let t = Travail {
                id: Uuid::new_v4(),
                nom: "auto.pdf".into(),
                format: Format::Pdf,
                demande: Demande { dpi: 36, auto: reglages::Auto { imprimer: true, decouper: true, suspendre: false }, ..Default::default() },
                statut: Statut::EnAttente,
                code_erreur: None,
                erreur: None,
                pages: vec![],
                alertes: vec![],
                cree_le: maintenant(),
            };
            let rep = dossier(&donnees, t.id);
            std::fs::create_dir_all(&rep).unwrap();
            std::fs::write(rep.join("source.pdf"), imbrication::tests::pdf(144.0, 72.0, "1 0 0 0", (0.0, 0.0))).unwrap();
            ecrire(&donnees, &t).unwrap();
            traiter(e.clone(), t.clone()).await;
            let etats: Vec<_> = e.file.lock().unwrap().sorties.iter().map(|s| (s.machine.clone(), s.etat)).collect();
            assert_eq!(etats, [("epson".to_string(), file::Etat::EnAttente), ("kona".to_string(), file::Etat::Suspendue)]);
            let surface = lire_travail(&donnees, t.id).unwrap().pages[0].surface_m2;

            // Sorties à passer : réédition refusée, puis acceptée une fois la file vidée.
            let corps = serde_json::json!({ "echelle": 200.0 });
            assert_eq!(appel(&e, "POST", &format!("/api/travaux/{}/editer", t.id), None, Some(corps.clone())).await.status(), StatusCode::CONFLICT);
            e.file.lock().unwrap().sorties.clear();
            assert_eq!(appel(&e, "POST", &format!("/api/travaux/{}/editer", t.id), None, Some(serde_json::json!({ "rotation": 45 }))).await.status(), StatusCode::BAD_REQUEST);
            assert_eq!(appel(&e, "POST", &format!("/api/travaux/{}/editer", t.id), None, Some(corps)).await.status(), StatusCode::ACCEPTED);
            let attendre = || async {
                for _ in 0..100 {
                    let x = lire_travail(&donnees, t.id).unwrap();
                    if !matches!(x.statut, Statut::EnAttente | Statut::EnCours) {
                        return x;
                    }
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                panic!("rendu trop long");
            };
            let x = attendre().await;
            assert!(matches!(x.statut, Statut::Termine), "{:?}", x.erreur);
            assert!((x.pages[0].surface_m2 / surface - 4.0).abs() < 0.05, "{} / {surface}", x.pages[0].surface_m2);
            assert!(rep.join("source-original.pdf").exists());
            assert_eq!(e.file.lock().unwrap().sorties.len(), 2, "le nouveau rendu repasse en file d'office");
            e.file.lock().unwrap().sorties.clear();
            // Retour à 100 % : la source redevient l'original.
            assert_eq!(appel(&e, "POST", &format!("/api/travaux/{}/editer", t.id), None, Some(serde_json::json!({}))).await.status(), StatusCode::ACCEPTED);
            assert_eq!(std::fs::read(rep.join("source.pdf")).unwrap(), std::fs::read(rep.join("source-original.pdf")).unwrap());
            attendre().await;
            std::fs::remove_dir_all(&donnees).unwrap();
        }

        /// Rendu réel avec sous-couche : la plaque `White` rejoint la page (production,
        /// aperçu, couverture), ses pixels suivent l'encre, et elle part à l'Epson.
        #[tokio::test]
        async fn sous_couche_blanche_de_bout_en_bout() {
            let e = etat("blanc");
            let donnees = e.config.donnees.clone();
            let blanc = blanc::Blanc { mode: blanc::Mode::SousCouche, retraction_mm: 0.0, seuil: 5.0 };
            let t = Travail {
                id: Uuid::new_v4(),
                nom: "blanc.pdf".into(),
                format: Format::Pdf,
                demande: Demande { dpi: 72, blanc, ..Default::default() },
                statut: Statut::EnAttente,
                code_erreur: None,
                erreur: None,
                pages: vec![],
                alertes: vec![],
                cree_le: maintenant(),
            };
            let rep = dossier(&donnees, t.id);
            std::fs::create_dir_all(&rep).unwrap();
            // Page cyan pleine de 2 × 1 pouce, contour CutContour à 5 pt du bord.
            std::fs::write(rep.join("source.pdf"), imbrication::tests::pdf(144.0, 72.0, "1 0 0 0", (0.0, 0.0))).unwrap();
            ecrire(&donnees, &t).unwrap();
            traiter(e.clone(), t.clone()).await;
            let t = lire_travail(&donnees, t.id).unwrap();
            assert!(matches!(t.statut, Statut::Termine), "{:?}", t.erreur);
            let p = &t.pages[0];
            assert!(p.plaques.contains(&"White".to_string()), "{:?}", p.plaques);
            let (blanc, cyan) = (p.couverture["White"], p.couverture["Cyan"]);
            assert!((blanc - cyan).abs() < 0.02, "sans rétraction, le blanc suit le cyan : {blanc} / {cyan}");
            let w = image::open(render::chemin_plaque(&rep, 1, "White")).unwrap().to_luma8();
            assert_eq!((w.width(), w.height()), (144, 72));
            assert!(render::chemin_apercu_plaque(&rep, 1, "White").exists());
            std::fs::remove_dir_all(&donnees).unwrap();
        }

        /// Rendu réel d'une page cyan de 2 × 1 pouce : couverture mesurée, coût aux tarifs
        /// enregistrés, tons de découpe hors compte, ligne du rapport CSV.
        #[tokio::test]
        async fn couverture_et_cout_d_un_vrai_rendu() {
            let e = etat("cout");
            let donnees = e.config.donnees.clone();
            let t = Travail {
                id: Uuid::new_v4(),
                nom: "cyan;\"2x1\".pdf".into(),
                format: Format::Pdf,
                demande: Demande { dpi: 72, ..Default::default() },
                statut: Statut::EnAttente,
                code_erreur: None,
                erreur: None,
                pages: vec![],
                alertes: vec![],
                cree_le: maintenant(),
            };
            let rep = dossier(&donnees, t.id);
            std::fs::create_dir_all(&rep).unwrap();
            std::fs::write(rep.join("source.pdf"), imbrication::tests::pdf(144.0, 72.0, "1 0 0 0", (0.0, 0.0))).unwrap();
            ecrire(&donnees, &t).unwrap();
            traiter(e.clone(), t.clone()).await;

            let tarifs = serde_json::json!({ "encre_defaut": { "ml_m2": 10.0, "prix_litre": 100.0 }, "encres": { "Cyan": { "ml_m2": 20.0, "prix_litre": 200.0 } }, "support_m2": 3.0 });
            assert_eq!(appel(&e, "POST", "/api/tarifs", None, Some(tarifs)).await.status(), StatusCode::OK);
            let mauvais = serde_json::json!({ "encre_defaut": { "ml_m2": -1.0, "prix_litre": 100.0 } });
            assert_eq!(appel(&e, "POST", "/api/tarifs", None, Some(mauvais)).await.status(), StatusCode::BAD_REQUEST);

            let r = appel(&e, "GET", "/api/travaux", None, None).await;
            let corps = axum::body::to_bytes(r.into_body(), 1 << 20).await.unwrap();
            let v: serde_json::Value = serde_json::from_slice(&corps).unwrap();
            let page = &v[0]["pages"][0];
            assert_eq!(v[0]["statut"], "termine", "{v}");
            let surface = 2.0 * 0.0254 * 0.0254;
            assert!((page["surface_m2"].as_f64().unwrap() - surface).abs() < surface * 0.03, "{page}");
            let cyan = page["couverture"]["Cyan"].as_f64().unwrap();
            assert!(cyan > 0.995, "page pleine de cyan, le filet de découpe ne défonce plus : {cyan}");
            assert!(!page["plaques"].as_array().unwrap().iter().any(|p| p == "CutContour"), "ton de découpe neutralisé au rendu");
            assert!(page["couverture"]["Magenta"].as_f64().unwrap() < 0.01);
            let est = &page["estimation"];
            assert!(est["encres"].as_array().unwrap().iter().all(|l| l["nom"] != "CutContour"), "{est}");
            let attendu = cyan * page["surface_m2"].as_f64().unwrap() * 20.0;
            assert!((est["encres"].as_array().unwrap().iter().find(|l| l["nom"] == "Cyan").unwrap()["ml"].as_f64().unwrap() - attendu).abs() < 1e-9);

            let r = appel(&e, "GET", "/api/rapports/consommation.csv", None, None).await;
            assert_eq!(r.headers()[header::CONTENT_TYPE], "text/csv; charset=utf-8");
            let csv = String::from_utf8(axum::body::to_bytes(r.into_body(), 1 << 20).await.unwrap().to_vec()).unwrap();
            let lignes: Vec<&str> = csv.lines().collect();
            assert_eq!(lignes.len(), 2, "{csv}");
            assert!(lignes[0].contains("\"Cyan_ml\"") && !lignes[0].contains("CutContour"), "{}", lignes[0]);
            assert!(lignes[1].contains("\"cyan;\"\"2x1\"\".pdf\""), "nom protégé : {}", lignes[1]);
            std::fs::remove_dir_all(&donnees).unwrap();
        }
    }

    /// Non-régression géométrie (roadmap 0.3), d'après des bogues corrigés chez un autre RIP.
    /// Chaque test passe par le vrai circuit (création du travail, rendu Ghostscript,
    /// découpe) et mesure la plaque produite et le HP-GL, pas seulement un statut.
    mod regression_geometrie {
        use super::controle_d_acces::etat;
        use super::*;
        use lopdf::{Document, Object, Stream, dictionary};

        const MM: f64 = imbrication::PT_PAR_MM;
        /// Contour `CutContour` des PDF d'essai : 5 pt à l'intérieur de l'image, en mm.
        const RETRAIT: f64 = 5.0 / MM;

        /// Crée le travail depuis `pdf`, comme un envoi, et attend la fin du rendu.
        async fn rendre(e: &Partage, pdf: Vec<u8>, d: Demande) -> Travail {
            let id = Uuid::new_v4();
            let rep = dossier(&e.config.donnees, id);
            std::fs::create_dir_all(&rep).unwrap();
            std::fs::write(rep.join("source.tmp"), pdf).unwrap();
            creer_travail(e, id, "essai.pdf".into(), Format::Pdf, d).await.unwrap_or_else(|(s, r)| panic!("refusé : {s} {r:?}"));
            for _ in 0..600 {
                let t = lire_travail(&e.config.donnees, id).unwrap();
                if !actif(&t.statut) {
                    assert!(matches!(t.statut, Statut::Termine), "{:?}", t.erreur);
                    return t;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            panic!("rendu trop long");
        }

        /// Boîte [x0, y0, x1, y1] en mm des points d'un fichier HP-GL (unités de 0,01 mm).
        fn boite_hpgl(chemin: &std::path::Path) -> [f64; 4] {
            let texte = std::fs::read_to_string(chemin).unwrap_or_else(|e| panic!("{} : {e}", chemin.display()));
            let v: Vec<f64> = texte.split(';').filter_map(|c| c.strip_prefix("PU").or(c.strip_prefix("PD"))).flat_map(|c| c.split(',')).filter_map(|n| n.parse().ok()).collect();
            assert!(!v.is_empty() && v.len().is_multiple_of(2), "{texte}");
            let (xs, ys): (Vec<f64>, Vec<f64>) = v.chunks(2).map(|p| (p[0] / 100.0, p[1] / 100.0)).unzip();
            let min = |v: &[f64]| v.iter().copied().fold(f64::MAX, f64::min);
            let max = |v: &[f64]| v.iter().copied().fold(f64::MIN, f64::max);
            [min(&xs), min(&ys), max(&xs), max(&ys)]
        }

        /// Format (mm) d'une plaque de production, et boîte [x0, y0, x1, y1] (mm, origine en
        /// bas à gauche comme le HP-GL) de ses pixels encrés.
        fn encre(rep: &std::path::Path, page: usize, plaque: &str, dpi: u32) -> ((f64, f64), [f64; 4]) {
            let img = image::open(render::chemin_plaque(rep, page, plaque)).unwrap().to_luma8();
            let (l, h) = img.dimensions();
            let mm = |px: u32| px as f64 * 25.4 / dpi as f64;
            let encres: Vec<(u32, u32)> = img.enumerate_pixels().filter(|p| p.2[0] < 128).map(|p| (p.0, p.1)).collect();
            assert!(!encres.is_empty(), "plaque {plaque} de la page {page} vide");
            let (c0, c1) = (encres.iter().map(|p| p.0).min().unwrap(), encres.iter().map(|p| p.0).max().unwrap());
            let (r0, r1) = (encres.iter().map(|p| p.1).min().unwrap(), encres.iter().map(|p| p.1).max().unwrap());
            ((mm(l), mm(h)), [mm(c0), mm(h - 1 - r1), mm(c1 + 1), mm(h - r0)])
        }

        fn proches(a: [f64; 4], b: [f64; 4], tolerance: f64) -> bool {
            a.iter().zip(b).all(|(x, y)| (x - y).abs() <= tolerance)
        }

        /// La coupe est dans le repère de la plaque : 5 pt à l'intérieur de l'encre.
        fn coupe_sur_l_encre(coupe: [f64; 4], cyan: [f64; 4], px: f64) {
            let attendu = [cyan[0] + RETRAIT, cyan[1] + RETRAIT, cyan[2] - RETRAIT, cyan[3] - RETRAIT];
            assert!(proches(coupe, attendu, 1.5 * px), "coupe {coupe:?}, encre {cyan:?}");
        }

        /// Une page du PDF d'essai : MediaBox, CropBox éventuelle, /Rotate éventuel.
        struct PageEssai {
            media: [f64; 4],
            crop: Option<[f64; 4]>,
            rotate: i64,
        }

        /// PDF de plusieurs pages, chacune remplie de cyan sur toute sa MediaBox, avec un
        /// contour `CutContour` à 5 pt du bord.
        fn pdf_pages(pages: &[PageEssai]) -> Vec<u8> {
            let mut doc = Document::with_version("1.4");
            let sep = doc.add_object(Object::Array(vec![
                "Separation".into(),
                "CutContour".into(),
                "DeviceCMYK".into(),
                Object::Dictionary(dictionary! { "FunctionType" => 2, "Domain" => vec![0.into(), 1.into()], "C0" => vec![0.into(); 4], "C1" => vec![0.into(), 1.into(), 0.into(), 0.into()], "N" => 1 }),
            ]));
            let parent = doc.new_object_id();
            let boite = |b: [f64; 4]| b.iter().map(|v| Object::Real(*v as f32)).collect::<Vec<_>>();
            let mut enfants: Vec<Object> = vec![];
            for p in pages {
                let [x0, y0, x1, y1] = p.media;
                let contenu = format!("1 0 0 0 k {x0} {y0} {} {} re f /CS0 CS 1 SCN {} {} {} {} re S", x1 - x0, y1 - y0, x0 + 5.0, y0 + 5.0, x1 - x0 - 10.0, y1 - y0 - 10.0);
                let c = doc.add_object(Stream::new(dictionary! {}, contenu.into_bytes()));
                let mut d = dictionary! { "Type" => "Page", "Parent" => parent, "MediaBox" => boite(p.media), "Contents" => c, "Resources" => dictionary! { "ColorSpace" => dictionary! { "CS0" => sep } } };
                if let Some(b) = p.crop {
                    d.set("CropBox", boite(b));
                }
                if p.rotate != 0 {
                    d.set("Rotate", p.rotate);
                }
                enfants.push(doc.add_object(d).into());
            }
            let n = enfants.len() as i64;
            doc.objects.insert(parent, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => enfants, "Count" => n }));
            let cat = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => parent });
            doc.trailer.set("Root", cat);
            let mut o = vec![];
            doc.save_to(&mut o).unwrap();
            o
        }

        /// Cas 1 — rotation automatique + résolution non carrée + fond perdu + étiquette.
        /// La résolution est ici toujours carrée (un seul `dpi`, `-r{dpi}`) : le cas se réduit
        /// à la rotation automatique décidée avec ce qui agrandit la page — la marge ajoutée
        /// (notre bord autour de l'image) et les repères imprimés à côté (croix). Page de
        /// 220 × 100 mm, laize 300 : elle tient seule (290 mm avec les croix), plus avec 10 mm
        /// de marge (310 mm) ; elle doit tourner, et la coupe rester sur l'encre.
        #[tokio::test]
        async fn regression_rotation_auto_avec_marge_et_reperage() {
            let e = etat("reg-rotation");
            e.machines.write().unwrap()[0].laize_mm = 300.0;
            let dpi = 72;
            let px = 25.4 / dpi as f64;
            let d = Demande { dpi, reperage: reperage::Reperage::Manuel, edition: edition::Edition { marge_mm: 10.0, ..Default::default() }, ..Default::default() };
            let t = rendre(&e, imbrication::tests::pdf(220.0 * MM, 100.0 * MM, "1 0 0 0", (0.0, 0.0)), d).await;
            assert!(t.alertes.iter().any(|a| a.code == "rotation_auto"), "{:?}", t.alertes);
            let rep = dossier(&e.config.donnees, t.id);
            let ((l, h), cyan) = encre(&rep, 1, "Cyan", dpi);
            // Tournée : image et marge 120 × 240 mm, plus 2 × 35 mm de croix et de blanc.
            assert!((l - 190.0).abs() <= px && (h - 310.0).abs() <= px, "page {l} × {h} mm");
            assert!(l <= 300.0, "la page tournée tient dans la laize");
            // Image de 100 × 220 mm à 35 + 10 mm du coin, dans les deux sens (pixels carrés).
            assert!(proches(cyan, [45.0, 45.0, 145.0, 265.0], 1.5 * px), "encre {cyan:?}");
            coupe_sur_l_encre(boite_hpgl(&fichier_plt(&rep, 1)), cyan, px);
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// Cas 2 — un fichier de coupe par feuille d'un travail de plusieurs feuilles. Chez
        /// nous, c'est la mosaïque : bannière de 700 × 100 mm, lés de 300 mm au plus avec
        /// 20 mm de recouvrement → 3 lés de 246,67 mm, chacun avec SON fichier de coupe, rogné
        /// à son bord et dans le repère de sa plaque.
        #[tokio::test]
        async fn regression_un_fichier_de_coupe_par_le() {
            let e = etat("reg-les");
            let dpi = 36;
            let px = 25.4 / dpi as f64;
            let d = Demande { dpi, mosaique: Some(Mosaique { laize_mm: 300.0, recouvrement_mm: 20.0, reperes: false }), ..Default::default() };
            let t = rendre(&e, imbrication::tests::pdf(700.0 * MM, 100.0 * MM, "1 0 0 0", (0.0, 0.0)), d).await;
            let rep = dossier(&e.config.donnees, t.id);
            assert_eq!(t.pages.len(), 3, "trois lés");
            let le = (700.0 + 2.0 * 20.0) / 3.0;
            for i in 1..=3 {
                assert!(t.pages[i - 1].traces_decoupe >= 1, "lé {i} sans tracé");
                let ((l, h), cyan) = encre(&rep, i, "Cyan", dpi);
                assert!((l - le).abs() <= px && (h - 100.0).abs() <= px, "lé {i} : {l} × {h} mm");
                // Contour de la bannière (x de 5 pt à 700 mm − 5 pt), vu depuis le début du lé.
                let debut = (i - 1) as f64 * (le - 20.0);
                let attendu = [(RETRAIT - debut).max(0.0), RETRAIT, (700.0 - RETRAIT - debut).min(le), 100.0 - RETRAIT];
                let coupe = boite_hpgl(&fichier_plt(&rep, i));
                assert!(proches(coupe, attendu, 0.02), "lé {i} : coupe {coupe:?}, attendu {attendu:?}");
                assert!(coupe[0] >= cyan[0] - px && coupe[2] <= cyan[2] + px, "lé {i} : coupe hors de la plaque");
                let r = super::controle_d_acces::appel(&e, "GET", &format!("/api/travaux/{}/pages/{i}/decoupe.plt", t.id), None, None).await;
                assert_eq!(r.status(), StatusCode::OK, "lé {i} : fichier servi");
            }
            assert!(!fichier_plt(&rep, 4).exists());
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// Cas 3 — PDF de plusieurs pages, boîte de rognage, une page par travail : une coupe
        /// par page, chacune à sa taille. Nous ne séparons pas les pages en travaux : un travail
        /// garde toutes ses pages, chacune avec son fichier de coupe. Trois formats : page
        /// simple, CropBox 1 mm à l'intérieur (Ghostscript n'imprime qu'elle : la page et la
        /// coupe doivent la suivre), MediaBox décalée.
        #[tokio::test]
        async fn regression_pdf_multipage_une_coupe_par_page() {
            let e = etat("reg-pages");
            let dpi = 36;
            let px = 25.4 / dpi as f64;
            let pages = [
                PageEssai { media: [0.0, 0.0, 100.0 * MM, 50.0 * MM], crop: None, rotate: 0 },
                PageEssai { media: [0.0, 0.0, 60.0 * MM, 120.0 * MM], crop: Some([1.0 * MM, 1.0 * MM, 59.0 * MM, 119.0 * MM]), rotate: 0 },
                PageEssai { media: [30.0, 40.0, 30.0 + 80.0 * MM, 40.0 + 80.0 * MM], crop: None, rotate: 0 },
            ];
            let t = rendre(&e, pdf_pages(&pages), Demande { dpi, ..Default::default() }).await;
            let rep = dossier(&e.config.donnees, t.id);
            assert_eq!(t.pages.len(), 3);
            // Format imprimé (mm) et retrait du contour par rapport à son bord.
            for (i, (l_mm, h_mm, r)) in [(100.0, 50.0, RETRAIT), (58.0, 118.0, RETRAIT - 1.0), (80.0, 80.0, RETRAIT)].into_iter().enumerate() {
                let n = i + 1;
                assert_eq!(t.pages[i].traces_decoupe, 1, "page {n}");
                let ((l, h), cyan) = encre(&rep, n, "Cyan", dpi);
                assert!((l - l_mm).abs() <= px && (h - h_mm).abs() <= px, "page {n} : {l} × {h} mm au lieu de {l_mm} × {h_mm}");
                assert!(proches(cyan, [0.0, 0.0, l_mm, h_mm], 1.5 * px), "page {n} : encre {cyan:?}");
                let coupe = boite_hpgl(&fichier_plt(&rep, n));
                assert!(proches(coupe, [r, r, l_mm - r, h_mm - r], 0.02), "page {n} : coupe {coupe:?}");
            }
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// Cas 4 — échelle d'un travail tourné, sur la bonne dimension. Page de 250 × 100 mm à
        /// 150 % : 375 × 150 mm, trop large pour 300 mm → tournée : 150 × 375 mm (l'échelle
        /// suit chaque côté, la rotation se décide après l'échelle). Puis une page tournée
        /// par le PDF lui-même (/Rotate 90) : la coupe suit le sens imprimé.
        #[tokio::test]
        async fn regression_echelle_d_un_travail_tourne() {
            let e = etat("reg-echelle");
            e.machines.write().unwrap()[0].laize_mm = 300.0;
            let dpi = 36;
            let px = 25.4 / dpi as f64;
            let d = Demande { dpi, edition: edition::Edition { echelle: 150.0, ..Default::default() }, ..Default::default() };
            let t = rendre(&e, imbrication::tests::pdf(250.0 * MM, 100.0 * MM, "1 0 0 0", (0.0, 0.0)), d).await;
            assert!(t.alertes.iter().any(|a| a.code == "rotation_auto"), "{:?}", t.alertes);
            let rep = dossier(&e.config.donnees, t.id);
            let ((l, h), cyan) = encre(&rep, 1, "Cyan", dpi);
            assert!((l - 150.0).abs() <= px && (h - 375.0).abs() <= px, "page {l} × {h} mm");
            assert!(proches(cyan, [0.0, 0.0, 150.0, 375.0], 1.5 * px), "l'image remplit la page tournée : {cyan:?}");
            // Contour à 5 pt du bord, × 1,5.
            let b = 1.5 * RETRAIT;
            assert!(proches(boite_hpgl(&fichier_plt(&rep, 1)), [b, b, 150.0 - b, 375.0 - b], 0.02));

            // /Rotate 90 du PDF : MediaBox 200 × 100 mm, imprimée 100 × 200 mm.
            let tourne = PageEssai { media: [0.0, 0.0, 200.0 * MM, 100.0 * MM], crop: None, rotate: 90 };
            let t = rendre(&e, pdf_pages(&[tourne]), Demande { dpi, rotation_auto: false, ..Default::default() }).await;
            let rep = dossier(&e.config.donnees, t.id);
            let ((l, h), cyan) = encre(&rep, 1, "Cyan", dpi);
            assert!((l - 100.0).abs() <= px && (h - 200.0).abs() <= px, "/Rotate rendu : {l} × {h} mm");
            coupe_sur_l_encre(boite_hpgl(&fichier_plt(&rep, 1)), cyan, px);
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// Cas 6 — rotation automatique refusée en résolution non carrée. Sans résolution non
        /// carrée chez nous, l'équivalent : la rotation automatique s'abstient là où elle
        /// fausserait la géométrie — pages qui ne tiennent pas toutes tournées (elle vaut pour
        /// tout le document), mosaïque (les lés règlent déjà la largeur), découpe seule (la
        /// laize de l'imprimante n'a rien à y voir).
        #[tokio::test]
        async fn regression_rotation_auto_refusee_hors_de_son_cas() {
            let e = etat("reg-refus");
            e.machines.write().unwrap()[0].laize_mm = 300.0;
            let d = Demande { dpi: 36, ..Default::default() };
            let taille = |rep: &std::path::Path, p: usize| {
                let t = imbrication::taille_page(&std::fs::read(rep.join("source.pdf")).unwrap(), p).unwrap();
                (t.largeur / MM, t.hauteur / MM)
            };
            let proche = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).abs() < 0.01 && (a.1 - b.1).abs() < 0.01;

            // 400 × 100 mm (tiendrait tournée) puis 100 × 400 mm (ne tiendrait plus).
            let pages = [PageEssai { media: [0.0, 0.0, 400.0 * MM, 100.0 * MM], crop: None, rotate: 0 }, PageEssai { media: [0.0, 0.0, 100.0 * MM, 400.0 * MM], crop: None, rotate: 0 }];
            let t = rendre(&e, pdf_pages(&pages), d.clone()).await;
            let rep = dossier(&e.config.donnees, t.id);
            assert!(!t.alertes.iter().any(|a| a.code == "rotation_auto"), "{:?}", t.alertes);
            assert!(proche(taille(&rep, 1), (400.0, 100.0)) && proche(taille(&rep, 2), (100.0, 400.0)), "{:?} {:?}", taille(&rep, 1), taille(&rep, 2));

            // Mosaïque : deux lés de 210 × 100 mm, pas une page tournée.
            let mosaique = Some(Mosaique { laize_mm: 300.0, recouvrement_mm: 20.0, reperes: false });
            let t = rendre(&e, imbrication::tests::pdf(400.0 * MM, 100.0 * MM, "1 0 0 0", (0.0, 0.0)), Demande { mosaique, ..d.clone() }).await;
            let rep = dossier(&e.config.donnees, t.id);
            assert!(!t.alertes.iter().any(|a| a.code == "rotation_auto"));
            assert_eq!(t.pages.len(), 2);
            assert!(proche(taille(&rep, 1), (210.0, 100.0)) && proche(taille(&rep, 2), (210.0, 100.0)), "{:?}", taille(&rep, 1));

            // Découpe seule : le tracé garde le sens du fichier.
            let t = rendre(&e, imbrication::tests::pdf(400.0 * MM, 100.0 * MM, "1 0 0 0", (0.0, 0.0)), Demande { mode: Mode::Decouper, ..d }).await;
            let rep = dossier(&e.config.donnees, t.id);
            assert!(!t.alertes.iter().any(|a| a.code == "rotation_auto"));
            assert!(proches(boite_hpgl(&fichier_plt(&rep, 1)), [RETRAIT, RETRAIT, 400.0 - RETRAIT, 100.0 - RETRAIT], 0.02));
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }
    }
}
