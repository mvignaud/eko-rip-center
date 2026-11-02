//! E-KO RIP Center — serveur local : reçoit les PDF des postes clients, les rend en
//! plaques d'encre et les garde prêts pour les machines branchées sur le serveur.

mod acces;
mod annonce;
mod bat;
mod blanc;
mod couleur;
mod controle;
mod coupe;
mod consommation;
mod couverture;
mod decoupe;
mod detourage;
mod edition;
mod erp;
mod etat;
mod epson;
mod epreuve;
mod escp2;
mod etiquette;
mod filtre;
mod machines;
mod media;
mod file;
mod forme;
mod images;
mod imbrication;
mod linearisation;
mod g7;
mod mobile;
mod mosaique;
mod nid;
mod pont;
mod ports;
mod plotter;
mod render;
mod reglages;
mod balises;
mod regles;
mod recalibration;
mod cellules;
mod abonnement;
mod cloud;
mod rapports;
mod telemetrie;
mod rendus;
mod releves;
mod reperage;
mod sauvegarde;
mod separation;
mod supports;
mod tons;
mod tramage;

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path as Chemin, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::io::AsyncWriteExt;
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

impl Statut {
    /// Code publié à l'ERP (modèle d'états, `travail.etat`).
    fn code(&self) -> &'static str {
        match self {
            Statut::EnAttente => "en_attente",
            Statut::EnCours => "en_cours",
            Statut::Termine => "termine",
            Statut::Echec => "echec",
        }
    }
}

/// Format reconnu aux premiers octets du fichier reçu.
#[derive(Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Format {
    #[default]
    Pdf,
    Svg,
    /// Images (roadmap 9.2) : converties en PDF à la création du travail, jamais gardées
    /// comme format d'un travail.
    Tiff,
    Png,
    Jpeg,
}

impl Format {
    fn fichier(self) -> &'static str {
        match self {
            Format::Pdf | Format::Tiff | Format::Png | Format::Jpeg => "source.pdf",
            Format::Svg => "source.svg",
        }
    }

    fn image(self) -> Option<images::Genre> {
        match self {
            Format::Tiff => Some(images::Genre::Tiff),
            Format::Png => Some(images::Genre::Png),
            Format::Jpeg => Some(images::Genre::Jpeg),
            _ => None,
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
    /// Couches de découpe, dans l'ordre de coupe (roadmap 4.2, 4.3) : tenues à jour à chaque
    /// rendu, changées seulement par un enregistrement explicite (`/couches`, roadmap 4.4).
    #[serde(default)]
    couches_decoupe: Vec<decoupe::Couche>,
    /// Mise en coupe (roadmap 4.8, 4.9) : rotation, et en découpe seule miroir, échelle,
    /// copies, origine ; appliquée aux tracés à chaque fichier de coupe, sans nouveau rendu.
    #[serde(default)]
    mise_coupe: coupe::Mise,
    /// Fiche travail (roadmap 5.6).
    #[serde(default)]
    fiche: Fiche,
    /// Pages à ne pas imprimer (roadmap 5.12), numérotées de 1 : ni planche ni file, tant
    /// qu'elles ne sont pas recochées.
    #[serde(default)]
    pages_exclues: Vec<usize>,
    /// Planche d'imbrication de la file (roadmap 5.1) : sa machine et les sorties qu'elle
    /// remplace.
    #[serde(default)]
    imbrication: Option<Imbriquee>,
    /// Envoyé par l'ERP (lot R3) : l'envoi et le dossier, repris dans chaque événement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    erp: Option<pont::Lien>,
    /// Suspendu ou annulé par l'ERP (lot R3.2) : les sorties mises en file d'office ensuite
    /// partent suspendues, ou ne partent pas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    retenue_erp: Option<RetenueErp>,
    /// Rendu pour une estimation de l'ERP seulement (lot R4, devis) : ni file, ni liste, ni
    /// `travail.etat` ; effacé une fois l'estimation répondue.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    estimation: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RetenueErp {
    Suspendu,
    Annule,
}

/// Fiche travail (roadmap 5.6) : ce que l'atelier sait de la commande. Champs vides : non
/// renseignés.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct Fiche {
    #[serde(default)]
    commande: String,
    #[serde(default)]
    client: String,
    #[serde(default)]
    societe: String,
    /// Quantité due : copies posées sur la planche d'imbrication et imprimées sur
    /// l'étiquette (0 : non renseignée, une copie).
    #[serde(default)]
    quantite: u32,
    /// Échéance (secondes Unix) : la plus proche passe d'abord dans l'imbrication.
    #[serde(default)]
    echeance: Option<u64>,
    #[serde(default)]
    notes: String,
    #[serde(default)]
    operateur: String,
}

impl Fiche {
    fn valide(&self) -> bool {
        [&self.commande, &self.client, &self.societe, &self.operateur].iter().all(|c| c.chars().count() <= 200) && self.notes.chars().count() <= 2000 && self.quantite <= 100_000
    }

    /// Copies à poser : la quantité due, une au moins.
    fn copies(&self) -> usize {
        self.quantite.max(1) as usize
    }
}

/// Une planche d'imbrication : la machine qui l'imprime, les sorties qu'elle remplace.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Imbriquee {
    machine: String,
    sorties: Vec<Uuid>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct Page {
    /// Repérage réellement appliqué (un cadre simple long devient multi-cadre).
    #[serde(default)]
    reperage: Option<reperage::Reperage>,
    /// Plaques de la page (vide en découpe seule).
    plaques: Vec<String>,
    /// Parmi elles, les tons de découpe (`CutContour`…) : jamais envoyés à l'imprimante.
    #[serde(default)]
    plaques_decoupe: Vec<String>,
    /// Tracés de découpe de la page (0 : rien à découper).
    traces_decoupe: usize,
    /// Tracés par couche (nom de la couche du travail).
    #[serde(default)]
    traces_par_couche: std::collections::BTreeMap<String, usize>,
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
    /// Format de la page des tracés de découpe (largeur, hauteur), en points ; absent des
    /// travaux d'avant la mise en coupe (`taille_coupe`).
    #[serde(default)]
    taille_coupe_pt: Option<[f64; 2]>,
    /// Contours ajoutés par le détourage automatique (roadmap 4.14), comptés dans les tracés.
    #[serde(default)]
    detourage: usize,
    /// Où l'image éditée se trouve sur la page rendue, pour les outils de la visionneuse.
    #[serde(default)]
    geometrie: Option<Geometrie>,
    /// Tuile de mosaïque que la page imprime (rien hors mosaïque).
    #[serde(default)]
    tuile: Option<mosaique::Tuile>,
}

/// Géométrie d'une page éditée (PDF sans mosaïque), en mm, origine en bas à gauche.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct Geometrie {
    /// Image éditée sur la page rendue (marge et repérage exclus) : x, y, l, h.
    image_mm: [f64; 4],
    /// Format de la page source (MediaBox de l'original).
    source_mm: [f64; 2],
    /// Rotation appliquée, rotation automatique comprise.
    rotation: u16,
    /// Du bord de la page éditée au trait de coupe (marge, rogne, fond perdu), en mm :
    /// gauche, bas, droite, haut.
    #[serde(default)]
    bords_mm: [f64; 4],
}

/// Géométrie de chaque page du travail (rien pour un SVG ou des lés de mosaïque).
fn geometries(rep: &std::path::Path, t: &Travail, mises: &[reperage::Mise]) -> Vec<Option<Geometrie>> {
    let lire = |nom: &str| std::fs::read(rep.join(nom)).ok();
    let (Some(produit), true, None) = (lire(Format::Pdf.fichier()), t.format == Format::Pdf, t.demande.mosaique.as_ref()) else { return vec![] };
    let original = lire("source-original.pdf").unwrap_or_else(|| produit.clone());
    let e = &edition_effective(&t.demande).0;
    let rotation = if e.rotation == 0 && t.alertes.iter().any(|a| a.code == "rotation_auto") { 90 } else { e.rotation };
    let mm = imbrication::PT_PAR_MM;
    (1..)
        .map_while(|n| imbrication::taille_page(&produit, n).ok().map(|p| (n, p)))
        .map(|(n, p)| {
            let s = imbrication::taille_page(&original, n).ok()?;
            let (dx, dy) = mises.get(n - 1).map_or((0.0, 0.0), |m| m.decalage);
            let b = e.bords_mm();
            Some(Geometrie { image_mm: [dx / mm + b[0], dy / mm + b[1], p.largeur / mm - b[0] - b[2], p.hauteur / mm - b[1] - b[3]], source_mm: [s.largeur / mm, s.hauteur / mm], rotation, bords_mm: b })
        })
        .collect()
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
    /// Rendus simultanés (`EKO_RIP_RENDUS`, un par cœur et 4 au plus par défaut).
    rendus: usize,
    /// Profils ICC déposés depuis l'écran.
    profils: PathBuf,
    /// Durée de conservation des travaux, en jours (0 : jamais purgés).
    conservation_jours: u64,
    /// Espace libre à garder sur le disque des données, en octets (0 : aucun seuil) : en
    /// dessous, les plus vieux travaux inactifs sont purgés et un nouveau travail qui ne
    /// tiendrait pas est refusé (roadmap 0.6). Seuil critique de la roadmap 0.8.
    seuil_libre: u64,
    /// Seuil bas, en octets (0 : aucun) : en dessous, chaque nouveau travail porte l'alerte
    /// `espace_disque_bas` et `/api/systeme` dit `disque_niveau: bas` (roadmap 0.8).
    seuil_alerte: u64,
    /// Travaux de l'ERP effacés dès qu'ils sont faits, sans attendre la conservation
    /// (`EKO_RIP_PURGER_ERP_FAITS=1`, roadmap 0.8) : l'ERP garde le fichier et le dossier.
    purger_erp_faits: bool,
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
    /// Rendus en parallèle, dans l'ordre d'arrivée (roadmap 9.11).
    rendu: rendus::Rendus<Travail>,
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
    if args.first().map(String::as_str) == Some("renvoi") {
        std::process::exit(commande_renvoi(&donnees, args.get(1).map(String::as_str)));
    }
    let config = Config {
        profils: donnees.join("profils"),
        donnees,
        gs: env_ou("EKO_RIP_GS", if cfg!(windows) { "gswin64c" } else { "gs" }),
        max_octets: env_ou("EKO_RIP_MAX_MO", "8192").parse::<usize>().expect("EKO_RIP_MAX_MO") * 1024 * 1024,
        delai: Duration::from_secs(env_ou("EKO_RIP_DELAI_S", "1800").parse().expect("EKO_RIP_DELAI_S")),
        rendus: std::env::var("EKO_RIP_RENDUS").ok().filter(|v| !v.is_empty()).map_or_else(rendus::par_defaut, |v| v.parse::<usize>().ok().filter(|n| (1..=rendus::MAX).contains(n)).expect("EKO_RIP_RENDUS : 1 à 16")),
        conservation_jours: env_ou("EKO_RIP_CONSERVATION_JOURS", "30").parse().expect("EKO_RIP_CONSERVATION_JOURS"),
        seuil_libre: env_ou("EKO_RIP_SEUIL_LIBRE_MO", "5120").parse::<u64>().expect("EKO_RIP_SEUIL_LIBRE_MO") * 1024 * 1024,
        seuil_alerte: env_ou("EKO_RIP_SEUIL_ALERTE_MO", "20480").parse::<u64>().expect("EKO_RIP_SEUIL_ALERTE_MO") * 1024 * 1024,
        purger_erp_faits: env_ou("EKO_RIP_PURGER_ERP_FAITS", "0") == "1",
        plotter: env_ou("EKO_RIP_PLOTTER", "/dev/kona").into(),
        plotter_stty: env_ou("EKO_RIP_PLOTTER_STTY", plotter::STTY_DEFAUT),
        epson_snmp: std::env::var("EKO_RIP_EPSON_SNMP").ok().filter(|v| !v.is_empty()),
        acces_navigateur: env_ou("EKO_RIP_ACCES_NAVIGATEUR", "0") == "1",
    };
    let adresse = env_ou("EKO_RIP_ADDR", "0.0.0.0:8640");
    std::fs::create_dir_all(config.donnees.join("travaux")).expect("dossier de données");
    std::fs::create_dir_all(&config.profils).expect("dossier des profils");
    erp::assurer_instance(&config.donnees);
    pont::demarrer(&config.donnees);
    reprendre_travaux_interrompus(&config.donnees);
    decoupe::installer(decoupe::lire(&config.donnees));

    let max = config.max_octets;
    let file = file::File::lire(&config.donnees);
    file.ecrire(&config.donnees).expect("file d'attente");
    let mut catalogue = machines::lire(&config.donnees).unwrap_or_else(|| machines::catalogue_initial(&config.plotter, &config.plotter_stty, config.epson_snmp.clone()));
    // Identifiants immuables (pont ERP) : attribués une fois, puis gardés. Les variantes suivent
    // leur machine d'origine.
    let avant = catalogue.clone();
    machines::accorder_variantes(&mut catalogue);
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
    // Dossier des règles (roadmap 9.1) : créé d'office, il n'existait pas avant.
    let _ = std::fs::create_dir_all(regles::dossier(&config.donnees));
    for m in catalogue.iter().filter(|m| m.connexion == machines::Connexion::Dossier) {
        etat::assurer_dossier(&config.donnees.join("sorties").join(&m.id));
    }
    let rendu = rendus::Rendus::new(config.rendus);
    let etat: Partage = Arc::new(Etat {
        config,
        rendu,
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
    tokio::spawn(pont::boucle(etat.config.donnees.clone()));
    tokio::spawn(prendre_envois(etat.clone()));
    tokio::spawn(veiller_la_file(etat.clone()));
    // Cloud E-KO RIP : la même API sert dans le tunnel vers la plate-forme, marquée pour
    // que `garder` en ferme une partie (pont ERP, renvoi, réglages du cloud).
    let par_cloud = routes(etat.clone()).layer(DefaultBodyLimit::max(max)).layer(axum::Extension(cloud::ParCloud));
    tokio::spawn(cloud::boucle(etat.config.donnees.clone(), par_cloud));
    tokio::spawn(abonnement::boucle(etat.clone()));
    // Rapports de bug automatiques (si l'administrateur l'a accepté) : plantage écrit avant l'arrêt.
    rapports::installer_crochet(etat.config.donnees.clone());
    tokio::spawn(rapports::boucle(etat.config.donnees.clone()));
    // Ping anonyme d'installation (à l'installation, à chaque mise à jour, puis chaque semaine).
    let e = etat.clone();
    tokio::spawn(telemetrie::boucle(etat.config.donnees.clone(), move || (e.machines.read().unwrap().len(), reglages::lire(&e.config.donnees).len())));
    let app = routes(etat).layer(DefaultBodyLimit::max(max));

    let ecoute = tokio::net::TcpListener::bind(&adresse).await.expect("adresse d'écoute");
    println!("E-KO RIP Center à l'écoute sur http://{adresse}");
    // Annonce sur le réseau local (les clients trouvent le serveur sans adresse) : sans bloquer ni échouer.
    annonce::demarrer(&adresse);
    axum::serve(ecoute, app).await.unwrap();
}

fn routes(etat: Partage) -> Router {
    Router::new()
        .route("/", get(accueil))
        .route("/api/travaux", get(lister).post(envoyer))
        .route("/api/profils", get(lister_profils).post(deposer_profil))
        .route("/api/planches", axum::routing::post(creer_planche))
        .route("/api/planches/apercu", axum::routing::post(apercu_planche))
        .route("/api/reperage/apercu", axum::routing::post(apercu_reperage))
        .route("/api/systeme", get(systeme))
        .route("/api/machines", get(lister_machines).post(enregistrer_machines))
        .route("/api/machines/{id}/media", get(media_machine))
        .route("/api/renvoi/{id}", axum::routing::post(recevoir_renvoi))
        .route("/api/machines/{id}/consommation", get(consommation_machine))
        .route("/api/machines/{id}/recalibration", get(recalibration_machine))
        .route("/api/machines/{id}/etalonnage", axum::routing::post(etalonner_machine))
        .route("/api/machines/{id}/recalibration/evenement", axum::routing::post(declencher_recalibration))
        .route("/api/recalibrations", get(recalibrations))
        .route("/api/machines/{id}/supports", get(lister_supports).post(enregistrer_supports))
        .route("/api/machines/{id}/charge", get(lire_charge).post(saisir_charge))
        .route("/api/machines/{id}/supports/paquet", axum::routing::post(importer_paquet))
        .route("/api/machines/{id}/supports/{uuid}/paquet", get(exporter_paquet))
        .route("/api/sauvegarde", get(exporter_config).post(restaurer_config))
        .route("/api/reglages", get(lister_reglages).post(enregistrer_reglage))
        .route("/api/reglages/export", get(exporter_reglages))
        .route("/api/reglages/import", axum::routing::post(importer_reglages))
        .route("/api/reglages/{nom}", axum::routing::delete(supprimer_reglage))
        .route("/api/regles", get(lire_regles).post(enregistrer_regles))
        .route("/api/gabarits", get(lire_gabarits).post(enregistrer_gabarits))
        .route("/api/balises", get(lire_balises).post(enregistrer_balises))
        .route("/api/travaux/{id}", axum::routing::delete(supprimer))
        .route("/api/travaux/{id}/relancer", axum::routing::post(relancer))
        .route("/api/travaux/{id}/editer", axum::routing::post(editer))
        .route("/api/travaux/{id}/reglages", axum::routing::post(modifier_reglages))
        .route("/api/travaux/{id}/couches", axum::routing::post(enregistrer_couches))
        .route("/api/travaux/{id}/coupe", axum::routing::post(enregistrer_mise))
        .route("/api/travaux/{id}/pages/{page}/detourage", axum::routing::post(detourer_page).delete(retirer_detourage))
        .route("/api/decoupe", get(lire_decoupe).post(enregistrer_decoupe))
        .route("/api/tons-directs", get(lire_tons).post(enregistrer_tons))
        .route("/api/tons-directs/import", axum::routing::post(importer_tons))
        .route("/api/tons-directs/inconnus", get(lire_tons_inconnus))
        .route("/api/separation/bascules", get(lister_bascules))
        .route("/api/linearisation/mire.cgats", get(mire_de_linearisation))
        .route("/api/linearisation/limite-proposee", axum::routing::post(proposer_limite))
        .route("/api/g7", get(lire_g7))
        .route("/api/g7/mire.cgats", get(mire_g7))
        .route("/api/g7/iterations", axum::routing::post(ajouter_iteration_g7))
        .route("/api/travaux/{id}/bat.pdf", get(bat_pdf))
        .route("/api/travaux/{id}/pages/{page}/epreuve.tif", get(epreuve_tif))
        .route("/api/travaux/{id}/pages/{page}/apercu.png", get(apercu))
        .route("/api/travaux/{id}/pages/{page}/apercu-avant.png", get(apercu_avant))
        .route("/api/travaux/{id}/pages/{page}/plaques/{nom}", get(plaque))
        .route("/api/travaux/{id}/pages/{page}/decoupe.plt", get(fichier_decoupe))
        .route("/api/travaux/{id}/pages/{page}/decoupe.svg", get(apercu_decoupe))
        .route("/api/tarifs", get(lire_tarifs).post(enregistrer_tarifs))
        .route("/api/rapports/consommation.csv", get(rapport_consommation))
        .route("/api/rapports/estimations.csv", get(rapport_estimations))
        .route("/api/consommation/journal", get(journal_des_impressions))
        .route("/api/consommation/journal/purger", axum::routing::post(purger_le_journal))
        .route("/api/consommation/reglages", get(lire_reglages_consommation).post(enregistrer_reglages_consommation))
        .route("/api/file", get(lister_file).post(mettre_en_file))
        .route("/api/imbrications", get(lister_imbrications))
        .route("/api/imbrications/{machine}/{code}/imprimer", axum::routing::post(imprimer_imbrication))
        .route("/api/imbrications/{machine}/{code}/planche.pdf", get(pdf_imbrication))
        .route("/api/travaux/{id}/fiche", axum::routing::post(enregistrer_fiche))
        .route("/api/travaux/{id}/pages/{page}/exclure", axum::routing::post(exclure_page))
        .route("/api/file/{id}", axum::routing::delete(retirer_de_la_file))
        .route("/api/file/{id}/{action}", axum::routing::post(agir_sur_la_file))
        .route("/api/erp/contrat", get(contrat_erp))
        .route("/api/pont", get(lire_pont).post(regler_pont))
        .route("/api/pont/instance", axum::routing::post(nouvelle_instance))
        .route("/api/mobile/etat", get(etat_mobile))
        .route("/api/mobile/evenements", get(evenements_mobiles))
        .route("/api/mobile/appareils", axum::routing::post(inscrire_appareil))
        .route("/api/mobile/appareils/retirer", axum::routing::post(retirer_appareil))
        .route("/api/mobile/essai", axum::routing::post(essai_mobile))
        .route("/api/cloud", get(lire_cloud).delete(detacher_cloud))
        .route("/api/cloud/appairage", axum::routing::post(appairer_cloud))
        .route("/api/cloud/sauvegarder", axum::routing::post(sauvegarder_cloud))
        .route("/api/cloud/restaurer", axum::routing::post(restaurer_par_cloud))
        .route("/api/telemetrie", get(lire_telemetrie).post(regler_telemetrie))
        .route("/api/rapports", axum::routing::post(signaler_probleme))
        .route("/api/rapports/reglage", get(lire_reglage_rapports).post(regler_rapports))
        .route("/api/session", get(lire_session).post(ouvrir_session).delete(fermer_session))
        .layer(axum::middleware::from_fn(signaler_changement))
        .layer(axum::middleware::from_fn_with_state(etat.clone(), garder))
        .with_state(etat)
}

fn dossier(donnees: &std::path::Path, id: Uuid) -> PathBuf {
    donnees.join("travaux").join(id.to_string())
}

/// Écrit `job.json` ; un changement d'état part vers l'ERP (`travail.etat`, pont actif),
/// sauf pour un rendu d'estimation.
fn ecrire(donnees: &std::path::Path, t: &Travail) -> std::io::Result<()> {
    etat::ecrire(&dossier(donnees, t.id).join("job.json"), t, false)?;
    if !t.estimation {
        pont::noter_travail(donnees, t.id, t.statut.code(), t.code_erreur.as_deref(), t.pages.len(), t.cree_le, t.erp.as_ref());
        mobile::noter_travail(donnees, t.id, &t.nom, matches!(t.statut, Statut::Echec), t.erreur.as_deref().or(t.code_erreur.as_deref()));
    }
    Ok(())
}

/// Les travaux de l'atelier, les plus récents d'abord (sans les rendus d'estimation).
fn lire_tous(donnees: &std::path::Path) -> Vec<Travail> {
    lire_tous_avec(donnees).into_iter().filter(|t| !t.estimation).collect()
}

/// Tous les `job.json`, rendus d'estimation compris.
fn lire_tous_avec(donnees: &std::path::Path) -> Vec<Travail> {
    let Ok(entrees) = std::fs::read_dir(donnees.join("travaux")) else { return vec![] };
    let mut v: Vec<Travail> = entrees
        .filter_map(|e| etat::lire(&e.ok()?.path().join("job.json")).ok().flatten())
        .collect();
    v.sort_by_key(|t| std::cmp::Reverse(t.cree_le));
    v
}

/// Un rendu coupé par un arrêt du serveur ne reprend pas tout seul : on le dit.
fn reprendre_travaux_interrompus(donnees: &std::path::Path) {
    for mut t in lire_tous_avec(donnees) {
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

#[derive(Deserialize)]
struct Rapport {
    /// Secondes Unix, bornes incluses ; par défaut tout le journal.
    depuis: Option<u64>,
    jusqua: Option<u64>,
    /// `jour` : une ligne par jour et par machine.
    par: Option<String>,
}

/// Journal des impressions (roadmap 6.4) avec l'encre réelle répartie, dans l'ordre des fins.
fn lignes_du_journal(donnees: &std::path::Path, depuis: Option<u64>, jusqua: Option<u64>) -> Vec<consommation::Ligne> {
    let tarifs = couverture::lire(donnees);
    let lignes = consommation::lignes(consommation::journal(donnees), |m| releves::lire(donnees, m), |n| consommation::contenance_ml(&tarifs, n));
    lignes.into_iter().filter(|l| depuis.is_none_or(|d| l.impression.fin >= d) && jusqua.is_none_or(|j| l.impression.fin <= j)).collect()
}

/// Consommation réelle et estimée par impression ou par jour, pour un tableur ou l'ERP
/// (roadmap 6.5) : séparateur `;`, décimales à la virgule, colonnes fixes, total en fin.
async fn rapport_consommation(State(etat): State<Partage>, axum::extract::Query(q): axum::extract::Query<Rapport>) -> Response {
    let lignes = lignes_du_journal(&etat.config.donnees, q.depuis, q.jusqua);
    let csv = consommation::csv(&lignes, q.par.as_deref() == Some("jour"), date_utc);
    ([(header::CONTENT_TYPE, "text/csv; charset=utf-8"), (header::CONTENT_DISPOSITION, "attachment; filename=\"consommation.csv\"")], csv).into_response()
}

/// Coût estimé de chaque page de chaque travail rendu, imprimé ou non (couverture de
/// l'aperçu × tarifs du moment), comme le rapport d'avant le journal ; `depuis`/`jusqua`
/// portent sur la date de création du travail.
async fn rapport_estimations(State(etat): State<Partage>, axum::extract::Query(q): axum::extract::Query<Rapport>) -> Response {
    let tarifs = couverture::lire(&etat.config.donnees);
    let pages: Vec<consommation::PageEstimee> = lire_tous(&etat.config.donnees)
        .iter()
        .filter(|t| matches!(t.statut, Statut::Termine) && t.demande.mode == Mode::Imprimer)
        .filter(|t| q.depuis.is_none_or(|d| t.cree_le >= d) && q.jusqua.is_none_or(|j| t.cree_le <= j))
        .flat_map(|t| {
            t.pages.iter().enumerate().map(|(i, p)| consommation::PageEstimee {
                date: date_utc(t.cree_le),
                nom: t.nom.clone(),
                id: t.id,
                page: i + 1,
                estimation: couverture::estimer(p.surface_m2, p.hauteur_mm, &p.couverture, &p.plaques_decoupe, &tarifs, t.demande.support.as_deref()),
            })
        })
        .collect();
    ([(header::CONTENT_TYPE, "text/csv; charset=utf-8"), (header::CONTENT_DISPOSITION, "attachment; filename=\"estimations.csv\"")], consommation::csv_estimations(&pages)).into_response()
}

/// Journal pour l'écran : les 200 dernières impressions de la période, les plus récentes d'abord.
async fn journal_des_impressions(State(etat): State<Partage>, axum::extract::Query(q): axum::extract::Query<Rapport>) -> Json<Vec<consommation::Ligne>> {
    let mut v = lignes_du_journal(&etat.config.donnees, q.depuis, q.jusqua);
    v.reverse();
    v.truncate(200);
    Json(v)
}

#[derive(Deserialize)]
struct Purge {
    /// Garder les impressions des `jours` derniers jours (0 : tout retirer).
    jours: u32,
}

/// Purge manuelle du journal ; les travaux et la file ne sont pas touchés.
async fn purger_le_journal(State(etat): State<Partage>, Json(p): Json<Purge>) -> Response {
    match consommation::purger(&etat.config.donnees, maintenant().saturating_sub(p.jours as u64 * 86_400) + u64::from(p.jours == 0)) {
        Ok(n) => Json(serde_json::json!({ "retirees": n })).into_response(),
        Err(e) => refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string())),
    }
}

/// Réglages de la consommation, et les facteurs proposés sur les 10 dernières impressions
/// mesurées de chaque machine × résolution × encre (roadmap 6.6).
async fn lire_reglages_consommation(State(etat): State<Partage>) -> Json<serde_json::Value> {
    let r = consommation::lire_reglages(&etat.config.donnees);
    let proposes = consommation::proposer(&lignes_du_journal(&etat.config.donnees, None, None), 10, &r);
    let mut v = serde_json::to_value(&r).unwrap();
    v["proposes"] = serde_json::json!(proposes);
    Json(v)
}

async fn enregistrer_reglages_consommation(State(etat): State<Partage>, Json(r): Json<consommation::Reglages>) -> Response {
    if let Err(code) = r.verifier() {
        return refus(StatusCode::BAD_REQUEST, code, None);
    }
    match consommation::ecrire_reglages(&etat.config.donnees, &r) {
        Ok(()) => Json(r).into_response(),
        Err(e) => refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string())),
    }
}

/// Préréglages de bascule des claires (roadmap 8.11) et leur couple, pour l'écran.
async fn lister_bascules() -> Json<serde_json::Value> {
    let v: Vec<_> = separation::Bascule::TOUTES.iter().map(|b| serde_json::json!({ "nom": b, "couple": b.appliquer(separation::Couple::default()) })).collect();
    Json(serde_json::json!(v))
}

#[derive(Deserialize)]
struct DemandeMire {
    /// Encres séparées par des virgules ; par défaut les 9 de l'Epson.
    encres: Option<String>,
    n: Option<usize>,
}

/// Mire de linéarisation CGATS, paliers resserrés sous 20 % (roadmap 8.11).
async fn mire_de_linearisation(axum::extract::Query(q): axum::extract::Query<DemandeMire>) -> Response {
    let encres = q.encres.unwrap_or_else(|| separation::ENCRES.iter().map(|e| e.nom()).collect::<Vec<_>>().join(","));
    let encres: Vec<&str> = encres.split(',').map(str::trim).filter(|e| !e.is_empty()).collect();
    match linearisation::mire_cgats(&encres, q.n.unwrap_or(21)) {
        Ok(t) => ([(header::CONTENT_TYPE, "text/plain; charset=utf-8"), (header::CONTENT_DISPOSITION, "attachment; filename=\"mire-linearisation.txt\"")], t).into_response(),
        Err(code) => refus(StatusCode::BAD_REQUEST, code, None),
    }
}

#[derive(Deserialize)]
struct DemandeLimite {
    /// Aplats mesurés : (encre totale %, L*).
    mesures: Vec<(f64, f64)>,
    seuil: Option<f64>,
}

/// Limite d'encre totale proposée (roadmap 8.11) ; rien n'est enregistré : l'opérateur la
/// relit et la saisit dans le mode.
async fn proposer_limite(Json(d): Json<DemandeLimite>) -> Response {
    match linearisation::limite_proposee(&d.mesures, d.seuil.unwrap_or(linearisation::SEUIL_DEFAUT)) {
        Ok(p) => Json(p).into_response(),
        Err(code) => refus(StatusCode::BAD_REQUEST, code, None),
    }
}

#[derive(Deserialize)]
struct DemandeMireG7 {
    n: Option<usize>,
}

/// Mire G7 (roadmap 8.10) : rampe CMY à parts égales et rampe noire, en CGATS.
async fn mire_g7(axum::extract::Query(q): axum::extract::Query<DemandeMireG7>) -> Response {
    match g7::mire_cgats(q.n.unwrap_or(21)) {
        Ok(t) => ([(header::CONTENT_TYPE, "text/plain; charset=utf-8"), (header::CONTENT_DISPOSITION, "attachment; filename=\"mire-g7.txt\"")], t).into_response(),
        Err(code) => refus(StatusCode::BAD_REQUEST, code, None),
    }
}

/// Étalonnages G7, chacun avec ses itérations et le numéro de la meilleure.
async fn lire_g7(State(etat): State<Partage>) -> Json<serde_json::Value> {
    let v: Vec<serde_json::Value> = g7::lire(&etat.config.donnees).iter().map(|e| serde_json::json!({ "etalonnage": e, "meilleure": e.meilleure().map(|i| i.numero) })).collect();
    Json(serde_json::json!(v))
}

#[derive(Deserialize)]
struct DemandeIterationG7 {
    nom: String,
    engraissement: Option<f64>,
    aplats: Option<g7::Aplats>,
    /// Numéro de l'itération dont les courbes ont servi à imprimer la mire.
    avec: Option<u32>,
    mesures: Vec<g7::Mesure>,
}

/// Nouvelle itération G7 mesurée : écarts, courbes de correction, et la meilleure gardée.
async fn ajouter_iteration_g7(State(etat): State<Partage>, Json(d): Json<DemandeIterationG7>) -> Response {
    let _verrou = etat.reglages.lock().await;
    let mut liste = g7::lire(&etat.config.donnees);
    let e = match g7::ajouter(&mut liste, d.nom.trim(), d.engraissement, d.aplats, d.avec, d.mesures, maintenant()) {
        Ok(e) => e,
        Err(code) => return refus(StatusCode::BAD_REQUEST, code, None),
    };
    match g7::ecrire(&etat.config.donnees, &liste) {
        Ok(()) => Json(serde_json::json!({ "etalonnage": e, "meilleure": e.meilleure().map(|i| i.numero) })).into_response(),
        Err(err) => refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(err.to_string())),
    }
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
    let (mut hauteur_max_mm, mut soudure, mut coupe_milieu) = (None, false, false);
    let (mut etendre, mut etendre_largeur, mut etendre_seuil) = (false, 1.0f64, 0.5f64);
    let mut fond = (false, reperage::FondReperes { debord_mm: 3.0, aplat: false, cmjn: None }, false, None::<[f64; 4]>);
    let mut fiche = Fiche::default();
    let (mut epreuve, mut options_epreuve) = (false, epreuve::Options::default());
    // Réglage enregistré, appliqué par le serveur (clients natifs) : il remplace la demande.
    let mut reglage = None::<String>;

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
            "intention_tons" if valeur.is_empty() => {
                d.couleur.intention_tons = None;
                Ok(())
            }
            "intention_tons" => valeur.parse().map(|v| d.couleur.intention_tons = Some(v)).map_err(|_| erreur("profil_inadapte")),
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
            "hauteur_max_mm" if valeur.is_empty() => Ok(()),
            "hauteur_max_mm" => valeur.parse().map(|v| hauteur_max_mm = Some(v)).map_err(|_| erreur("mosaique_invalide")),
            "soudure" => {
                soudure = valeur == "oui";
                Ok(())
            }
            "coupe_milieu" => {
                coupe_milieu = valeur == "oui";
                Ok(())
            }
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
            "epreuve" => {
                epreuve = valeur == "oui";
                Ok(())
            }
            "reechantillonnage" => serde_json::from_value(serde_json::Value::String(valeur)).map(|r| d.reechantillonnage = r).map_err(|_| erreur("demande_invalide")),
            "empilement" => valeur.split(',').map(str::trim).filter(|v| !v.is_empty()).map(|v| serde_json::from_value(serde_json::Value::String(v.into()))).collect::<Result<_, _>>().map(|e| d.empilement = e).map_err(|_| erreur("empilement_invalide")),
            "epreuve_seule" => {
                d.epreuve_seule = valeur == "oui";
                Ok(())
            }
            "epreuve_couleur" => serde_json::from_value(serde_json::Value::String(valeur)).map(|c| options_epreuve.couleur = c).map_err(|_| erreur("epreuve_invalide")),
            "epreuve_tons" => serde_json::from_value(serde_json::Value::String(valeur)).map(|t| options_epreuve.tons = t).map_err(|_| erreur("epreuve_invalide")),
            "epreuve_max_mo" => valeur.parse().map(|v| options_epreuve.max_mo = v).map_err(|_| erreur("epreuve_invalide")),
            "blanc_alpha" => serde_json::from_value(serde_json::Value::String(valeur)).map(|a| d.blanc.alpha = a).map_err(|_| erreur("blanc_invalide")),
            "parametre_decoupe" => {
                d.parametre_decoupe = Some(valeur).filter(|v| !v.is_empty());
                Ok(())
            }
            "decoupeuse" => {
                d.decoupeuse = Some(valeur).filter(|v| !v.is_empty());
                Ok(())
            }
            "etendre" => {
                etendre = valeur == "oui";
                Ok(())
            }
            "etendre_largeur_mm" => valeur.parse().map(|v| etendre_largeur = v).map_err(|_| erreur("etendre_invalide")),
            "etendre_seuil_mm" => valeur.parse().map(|v| etendre_seuil = v).map_err(|_| erreur("etendre_invalide")),
            "fond_reperes" => {
                fond.0 = valeur == "oui";
                Ok(())
            }
            "fond_debord_mm" => valeur.parse().map(|v| fond.1.debord_mm = v).map_err(|_| erreur("fond_reperes_invalide")),
            "fond_aplat" => {
                fond.1.aplat = valeur == "oui";
                Ok(())
            }
            "fond_encre" => {
                fond.2 = valeur == "cmjn";
                Ok(())
            }
            // « 0 0 100 0 » : CMJN en %.
            "fond_cmjn" => {
                let v: Vec<f64> = valeur.split([' ', ';']).filter(|x| !x.is_empty()).filter_map(|x| x.replace(',', ".").parse::<f64>().ok()).map(|x| x / 100.0).collect();
                fond.3 = v.try_into().ok();
                Ok(())
            }
            // Fiche travail (roadmap 5.6) ; échéance en secondes Unix.
            "commande" => {
                fiche.commande = valeur;
                Ok(())
            }
            "client" => {
                fiche.client = valeur;
                Ok(())
            }
            "societe" => {
                fiche.societe = valeur;
                Ok(())
            }
            "notes" => {
                fiche.notes = valeur;
                Ok(())
            }
            "operateur" => {
                fiche.operateur = valeur;
                Ok(())
            }
            "quantite" if valeur.is_empty() => Ok(()),
            "quantite" => valeur.parse().map(|v| fiche.quantite = v).map_err(|_| erreur("fiche_invalide")),
            "page_par_travail" => {
                d.page_par_travail = valeur == "oui";
                Ok(())
            }
            "abandon_polices" => {
                d.abandon_polices = valeur == "oui";
                Ok(())
            }
            "reglage" => {
                reglage = Some(valeur).filter(|v| !v.is_empty());
                Ok(())
            }
            "echeance" if valeur.is_empty() => Ok(()),
            "echeance" => valeur.parse().map(|v| fiche.echeance = Some(v)).map_err(|_| erreur("fiche_invalide")),
            _ => Ok(()),
        };
        if let Err(r) = r {
            return abandon(&rep, StatusCode::BAD_REQUEST, r).await;
        }
    }
    let Some(format) = format else {
        return abandon(&rep, StatusCode::BAD_REQUEST, ("aucun_fichier", None)).await;
    };
    d.epreuve = (epreuve || d.epreuve_seule).then_some(options_epreuve);
    d.mosaique = mosaique.then_some(Mosaique { laize_mm, recouvrement_mm, reperes: reperes_les, hauteur_max_mm, soudure, coupe_milieu, ..Default::default() });
    d.etendre_coupe = etendre.then_some(coupe::Etendre { largeur_mm: etendre_largeur, seuil_mm: etendre_seuil });
    if fond.2 && fond.3.is_none() {
        return abandon(&rep, StatusCode::BAD_REQUEST, ("fond_reperes_invalide", None)).await;
    }
    d.fond_reperes = fond.0.then_some(reperage::FondReperes { cmjn: fond.3.filter(|_| fond.2), ..fond.1 });
    if let Some(n) = reglage {
        match reglages::lire(&etat.config.donnees).into_iter().find(|r| r.nom == n) {
            Some(r) => d = r.demande,
            None => return abandon(&rep, StatusCode::BAD_REQUEST, (pont::REFUS_REGLAGE_INCONNU, Some(n))).await,
        }
    }
    match creer_travail_avec(&etat, id, nom, format, d, fiche, Origine::Aucune, vec![]).await {
        Ok(t) => (StatusCode::ACCEPTED, Json(t)).into_response(),
        Err((statut, r)) => abandon(&rep, statut, r).await,
    }
}

/// Crée un travail à partir de `source.tmp`, déjà écrit dans son dossier : contrôles,
/// mosaïque, `job.json`, puis traitement en tâche de fond. Commun à l'écran, aux planches
/// et aux dossiers surveillés. En cas de refus, le dossier reste à effacer par l'appelant.
async fn creer_travail(etat: &Partage, id: Uuid, nom: String, format: Format, d: Demande) -> Result<Travail, (StatusCode, Refus)> {
    creer_travail_avec(etat, id, nom, format, d, Fiche::default(), Origine::Aucune, vec![]).await
}

/// D'où vient un travail, quand ce n'est pas d'un simple envoi.
enum Origine {
    /// Import à la main (écran) : son rendu passe devant les arrivées automatiques.
    Aucune,
    /// Dossier surveillé : arrivée automatique, sortie dans l'ordre d'arrivée.
    Dossier,
    /// Planche de la file : ce qu'elle imbrique, et les couches de découpe reprises des
    /// travaux posés.
    Planche(Imbriquee, Vec<decoupe::Couche>),
    /// Envoi de l'ERP (lot R3).
    Erp(pont::Lien),
    /// Fichier à estimer pour l'ERP (lot R4, devis) : rendu à part, jamais en file.
    Estimation(pont::Lien),
}

/// `creer_travail` avec sa fiche (roadmap 5.6) et son origine.
/// `depart` : alertes connues avant le rendu (balises du nom de fichier, roadmap 9.10).
#[allow(clippy::too_many_arguments)]
async fn creer_travail_avec(etat: &Partage, id: Uuid, nom: String, format: Format, d: Demande, fiche: Fiche, origine: Origine, depart: Vec<controle::Alerte>) -> Result<Travail, (StatusCode, Refus)> {
    let estimation = matches!(origine, Origine::Estimation(_));
    let automatique = matches!(origine, Origine::Dossier | Origine::Erp(_) | Origine::Estimation(_));
    let dossier_surveille = matches!(origine, Origine::Dossier);
    let (imbrication, couches_decoupe, erp) = match origine {
        Origine::Aucune | Origine::Dossier => (None, vec![], None),
        Origine::Planche(i, c) => (Some(i), c, None),
        Origine::Erp(l) | Origine::Estimation(l) => (None, vec![], Some(l)),
    };
    let rep = dossier(&etat.config.donnees, id);
    let mauvais = |code: &'static str, detail: Option<String>| (StatusCode::BAD_REQUEST, (code, detail));
    let interne = |e: String| (StatusCode::INTERNAL_SERVER_ERROR, ("erreur_interne", Some(e)));
    let d = d.normaliser();
    // Image : un PDF d'une page à sa taille réelle, écrit en flux, qui remplace le fichier
    // reçu ; le travail est ensuite celui d'un PDF.
    let (format, remarques_image) = match format.image() {
        Some(g) => {
            let (src, dst) = (rep.join("source.tmp"), rep.join("source-image.pdf"));
            let plaque_alpha = d.blanc.alpha.plaque();
            let r = tokio::task::spawn_blocking(move || images::vers_pdf(&src, &dst, g, plaque_alpha)).await.map_err(|e| interne(e.to_string()))?;
            let r = r.map_err(|(code, e)| if code == "erreur_interne" { interne(e) } else { mauvais(code, Some(e)) })?;
            tokio::fs::rename(rep.join("source-image.pdf"), rep.join("source.tmp")).await.map_err(|e| interne(e.to_string()))?;
            (Format::Pdf, Some(r))
        }
        None => (format, None),
    };
    // Le rendu en plaques passe par Ghostscript, qui ne lit pas le SVG.
    if format == Format::Svg && d.mode == Mode::Imprimer {
        return Err(mauvais("svg_decoupe_seulement", None));
    }
    let mut d = d;
    // Une estimation garde toutes ses pages dans un seul travail caché.
    d.page_par_travail &= !estimation;
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
    // Espace libre (roadmap 0.6, 0.8) : refus anticipé si le rendu ne tiendrait pas au-dessus
    // du seuil critique, alerte sous le seuil bas. Un dépôt en dossier surveillé n'est pas
    // refusé : il devient un travail en erreur avec son motif, à relancer une fois la place faite.
    let (seuil_bas, seuil_critique) = (etat.config.seuil_alerte, etat.config.seuil_libre);
    let (mut manque_place, mut disque_bas) = (None, None);
    if format == Format::Pdf && d.mode == Mode::Imprimer && (seuil_bas, seuil_critique) != (0, 0) {
        let pdf = tokio::fs::read(rep.join(format.fichier())).await.map_err(|e| interne(e.to_string()))?;
        let surface = (1..).map_while(|n| imbrication::taille_page(&pdf, n).ok()).map(|t| t.largeur * t.hauteur / (imbrication::PT_PAR_MM * imbrication::PT_PAR_MM) / 1e6).sum::<f64>();
        let besoin = place_estimee(surface, d.dpi);
        if let Some((_, libre)) = espace_disque(&etat.config.donnees).await {
            if seuil_critique > 0 && libre < seuil_critique.saturating_add(besoin) {
                let detail = format!("≈ {} Mo nécessaires, {} Mo libres, seuil {} Mo", besoin >> 20, libre >> 20, seuil_critique >> 20);
                if !dossier_surveille {
                    return Err((StatusCode::INSUFFICIENT_STORAGE, ("espace_insuffisant", Some(detail))));
                }
                manque_place = Some(detail);
            } else if seuil_bas > 0 && libre < seuil_bas.saturating_add(besoin) {
                disque_bas = Some(alerte("espace_disque_bas", format!("{} Mo libres, seuil {} Mo", libre >> 20, seuil_bas >> 20)));
            }
        }
    }
    let mut alertes = if format == Format::Pdf && d.mode == Mode::Imprimer {
        let pdf = tokio::fs::read(rep.join(format.fichier())).await.map_err(|e| interne(e.to_string()))?;
        tokio::task::spawn_blocking(move || controle::verifier(&pdf)).await.unwrap_or_default()
    } else {
        vec![]
    };
    // Abandon si une police manque (roadmap 9.3) : tout le document, avant de le couper en pages.
    if d.abandon_polices {
        let manquantes = controle::polices_manquantes(&alertes);
        if !manquantes.is_empty() {
            return Err(mauvais("polices_manquantes", Some(manquantes.join(", "))));
        }
    }
    // Une page par travail (roadmap 9.3) : les pages 2 à n deviennent des travaux à part,
    // créés après celui-ci ; ce travail garde la page 1.
    let mut autres_pages = vec![];
    let mut nom = nom;
    if d.page_par_travail && format == Format::Pdf && imbrication.is_none() {
        let pdf = tokio::fs::read(rep.join(format.fichier())).await.map_err(|e| interne(e.to_string()))?;
        let pages = tokio::task::spawn_blocking(move || imbrication::pages_separees(&pdf)).await.map_err(|e| interne(e.to_string()))?.map_err(|e| mauvais("pdf_illisible", Some(e)))?;
        let n = pages.len();
        if n > 1 {
            let mut pages = pages.into_iter();
            let premiere = pages.next().unwrap_or_default();
            tokio::fs::write(rep.join(format.fichier()), &premiere).await.map_err(|e| interne(e.to_string()))?;
            autres_pages = pages.enumerate().map(|(k, p)| (format!("{nom} (p. {}/{n})", k + 2), p)).collect();
            nom = format!("{nom} (p. 1/{n})");
            // Les alertes ne portent plus que sur la page gardée.
            alertes = controle::verifier(&premiere);
        }
    }
    let mosaique = d.mosaique.clone().filter(|_| d.mode == Mode::Imprimer && format == Format::Pdf);
    alertes.extend(depart.iter().cloned());
    alertes.extend(disque_bas);
    if let Some(r) = &remarques_image {
        if r.sans_resolution {
            alertes.push(controle::Alerte { code: "image_sans_resolution".into(), page: None, detail: format!("{} ppi · {:.0} × {:.0} mm", images::PPI_PAR_DEFAUT, r.largeur_mm, r.hauteur_mm) });
        }
        if r.canaux_ignores > 0 {
            alertes.push(controle::Alerte { code: "image_canaux_ignores".into(), page: None, detail: r.canaux_ignores.to_string() });
        }
        // Sens des valeurs des canaux en plus encore à confirmer sur un vrai fichier : la
        // plaque est signalée pour être regardée avant d'imprimer.
        if !r.tons_directs.is_empty() {
            alertes.push(controle::Alerte { code: "image_tons_directs".into(), page: None, detail: r.tons_directs.join(", ") });
        }
    }
    if !fiche.valide() {
        return Err(mauvais("fiche_invalide", None));
    }
    let infos = infos_travail(etat, id, &nom, &d, &fiche);
    let reperage = reperage_voulu(&etat.machines.read().unwrap(), &d);
    let tourne = construire_source(&rep, format, &Demande { mosaique: mosaique.clone(), reperage, ..d.clone() }, laize_imprimante(etat), infos).await.map_err(|(code, e)| if code == "erreur_interne" { interne(e) } else { mauvais(code, Some(e)) })?;
    alertes.extend(tourne);
    let travail = Travail {
        id,
        nom,
        format,
        demande: Demande { mosaique, ..d },
        statut: if manque_place.is_some() { Statut::Echec } else { Statut::EnAttente },
        code_erreur: manque_place.is_some().then(|| "espace_insuffisant".to_string()),
        erreur: manque_place,
        pages: vec![],
        alertes,
        cree_le: maintenant(),
        couches_decoupe,
        mise_coupe: Default::default(),
        fiche,
        pages_exclues: vec![],
        imbrication,
        erp,
        retenue_erp: None,
        estimation,
    };
    ecrire(&etat.config.donnees, &travail).map_err(|e| interne(e.to_string()))?;
    // Inscrit avant de lancer la tâche : l'ordre d'arrivée ne dépend pas de l'ordonnanceur.
    if matches!(travail.statut, Statut::EnAttente) {
        tokio::spawn(traiter(etat.clone(), travail.clone(), etat.rendu.inscrire(automatique)));
    } else {
        eprintln!("dossier surveillé : {} en erreur, espace insuffisant : {}", travail.nom, travail.erreur.as_deref().unwrap_or_default());
    }
    for (nom_page, pdf) in autres_pages {
        let id = Uuid::new_v4();
        let rep_page = dossier(&etat.config.donnees, id);
        let ecrit = std::fs::create_dir_all(&rep_page).and_then(|()| std::fs::write(rep_page.join("source.tmp"), &pdf));
        let d = Demande { page_par_travail: false, ..travail.demande.clone() };
        let r = match ecrit {
            Ok(()) => Box::pin(creer_travail_avec(etat, id, nom_page.clone(), Format::Pdf, d, travail.fiche.clone(), travail.erp.clone().map_or(if automatique { Origine::Dossier } else { Origine::Aucune }, Origine::Erp), depart.clone())).await.map(|_| ()).map_err(|(_, (code, detail))| format!("{code} {}", detail.unwrap_or_default())),
            Err(e) => Err(e.to_string()),
        };
        if let Err(e) = r {
            eprintln!("une page par travail : {nom_page} refusée : {e}");
            let _ = std::fs::remove_dir_all(&rep_page);
        }
    }
    Ok(travail)
}

/// `source.pdf` à rendre, depuis l'original : édition rapide, mosaïque, puis étiquette et
/// code de chaque page. L'original est gardé dans `source-original.pdf` dès qu'une
/// transformation s'applique, pour pouvoir rééditer sans renvoyer le fichier. Rend les
/// alertes de la construction (`ALERTES_CONSTRUCTION`). Erreur : (code, détail).
async fn construire_source(rep: &std::path::Path, format: Format, d: &Demande, laize_mm: Option<f64>, infos: etiquette::Infos) -> Result<Vec<controle::Alerte>, (&'static str, String)> {
    let interne = |e: std::io::Error| ("erreur_interne", e.to_string());
    if format != Format::Pdf {
        return Ok(vec![]);
    }
    let (source, original) = (rep.join(Format::Pdf.fichier()), rep.join("source-original.pdf"));
    let ((mut edition, neutralisees), mosaique, reperage) = (edition_effective(d), d.mosaique.clone(), d.reperage);
    let mut alertes = vec![];
    if neutralisees {
        alertes.push(controle::Alerte { code: "marques_neutralisees".into(), page: None, detail: String::new() });
    }
    let a_original = tokio::fs::try_exists(&original).await.unwrap_or(false);
    let pdf = tokio::fs::read(if a_original { &original } else { &source }).await.map_err(interne)?;
    // Rotation automatique : décidée sur la page éditée (échelle, marge…), repérage compris.
    let auto = (d.rotation_auto && d.mode == Mode::Imprimer && edition.rotation == 0 && mosaique.is_none()).then_some(laize_mm).flatten();
    let tourne = match auto {
        Some(laize) => {
            let (copie, ed) = (pdf.clone(), edition.clone());
            tokio::task::spawn_blocking(move || -> Result<Option<controle::Alerte>, (&'static str, String)> {
                let edite = if ed.neutre() { copie } else { edition::appliquer(&copie, &ed)? };
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
    alertes.extend(tourne);
    // Tuiles du PDF produit, pour la visionneuse (`Page::tuile`).
    let tuiles = rep.join("tuiles.json");
    if mosaique.is_none() {
        let _ = tokio::fs::remove_file(&tuiles).await;
    }
    let kona = |pdf: &[u8]| if d.mode == Mode::Imprimer { alertes_kona(pdf, reperage) } else { vec![] };
    if edition.neutre() && mosaique.is_none() {
        if a_original {
            tokio::fs::copy(&original, &source).await.map_err(interne)?;
        }
        alertes.extend(kona(&pdf));
        return Ok(alertes);
    }
    if !a_original {
        tokio::fs::copy(&source, &original).await.map_err(interne)?;
    }
    let (produit, decoupe) = tokio::task::spawn_blocking(move || -> Result<(Vec<u8>, mosaique::Tuiles), (&'static str, String)> {
        let pdf = if edition.neutre() { pdf } else { edition::appliquer(&pdf, &edition)? };
        let (pdf, tuiles) = match mosaique {
            Some(m) => mosaique::composer(&pdf, &m)?,
            None => (pdf, vec![]),
        };
        let pdf = etiquette::annoter(&pdf, &edition, &infos, &tuiles).map_err(|e| ("edition_invalide", e))?;
        Ok((pdf, tuiles))
    })
    .await
    .map_err(|e| ("erreur_interne", e.to_string()))??;
    alertes.extend(kona(&produit));
    tokio::fs::write(&source, produit).await.map_err(interne)?;
    // Une mosaïque produit au moins une page : vide, il n'y en a pas.
    if !decoupe.is_empty() {
        tokio::fs::write(&tuiles, serde_json::to_vec(&decoupe).unwrap_or_default()).await.map_err(interne)?;
    }
    Ok(alertes)
}

/// Édition réellement appliquée : avec un repérage Kona, des traits de coupe ou des croix
/// près de ses repères gêneraient leur lecture ; ils ne sont pas imprimés (contour et plis
/// restent). Vrai s'il y en avait.
fn edition_effective(d: &Demande) -> (edition::Edition, bool) {
    let mut e = d.edition.clone();
    let gene = d.reperage != reperage::Reperage::Aucun && e.marques.as_ref().is_some_and(|m| m.coupe || m.reperage);
    if gene && let Some(m) = e.marques.as_mut() {
        (m.coupe, m.reperage) = (false, false);
    }
    (e, gene)
}

#[derive(Deserialize)]
struct DemandeApercuReperage {
    reperage: reperage::Reperage,
    /// Page éditée (marge comprise) à envelopper, en mm.
    largeur_mm: f64,
    hauteur_mm: f64,
}

/// Repères Kona d'une page de cette taille, pour les dessiner autour de l'image pendant
/// l'édition (roadmap 3.D2) : page, place de l'image et rectangles en mm, origine en bas à
/// gauche, et écarts à la consigne du support.
async fn apercu_reperage(Json(d): Json<DemandeApercuReperage>) -> Response {
    if ![d.largeur_mm, d.hauteur_mm].iter().all(|v| (1.0..=20_000.0).contains(v)) {
        return refus(StatusCode::BAD_REQUEST, "edition_invalide", Some("1 à 20 000 mm".into()));
    }
    let mm = imbrication::PT_PAR_MM;
    let m = reperage::mise_en_page(d.reperage, d.largeur_mm * mm, d.hauteur_mm * mm, 0);
    let ecarts: Vec<_> = reperage::ecarts(d.reperage, d.largeur_mm * mm, d.hauteur_mm * mm).into_iter().map(|(code, detail)| serde_json::json!({ "code": code, "detail": detail })).collect();
    Json(serde_json::json!({
        "page_mm": [m.page.0 / mm, m.page.1 / mm],
        "decalage_mm": [m.decalage.0 / mm, m.decalage.1 / mm],
        "rectangles_mm": m.rectangles.iter().map(|r| r.map(|v| v / mm)).collect::<Vec<_>>(),
        "alertes": ecarts,
    }))
    .into_response()
}

/// Codes des alertes que `construire_source` recalcule à chaque reconstruction.
const ALERTES_CONSTRUCTION: [&str; 4] = ["rotation_auto", "marques_neutralisees", "reperes_image_petite", "reperes_blanc"];

/// Consigne des repères Kona (`reperage::ecarts`) sur chaque page à envelopper.
fn alertes_kona(pdf: &[u8], reperage: reperage::Reperage) -> Vec<controle::Alerte> {
    if reperage == reperage::Reperage::Aucun {
        return vec![];
    }
    (1..)
        .map_while(|n| imbrication::taille_page(pdf, n).ok().map(|t| (n, t)))
        .flat_map(|(n, t)| reperage::ecarts(reperage, t.largeur, t.hauteur).into_iter().map(move |(code, detail)| controle::Alerte { code: code.into(), page: Some(n), detail }))
        .collect()
}

/// Ce que l'étiquette et le code impriment d'un travail.
fn infos_travail(etat: &Etat, id: Uuid, nom: &str, d: &Demande, fiche: &Fiche) -> etiquette::Infos {
    let machine = etat.machines.read().unwrap().iter().find(|m| m.fait(machines::Operation::Impression)).map(|m| m.nom.clone()).unwrap_or_default();
    let mode = match &d.support {
        Some(s) => format!("{} dpi · {s}", d.dpi),
        None => format!("{} dpi", d.dpi),
    };
    etiquette::Infos { nom: nom.to_string(), date: format!("{} UTC", date_utc(maintenant())), travail: id, machine, mode, commande: Some(fiche.commande.clone()).filter(|c| !c.is_empty()), copies: (fiche.quantite > 0).then_some(fiche.quantite) }
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

/// PDF (un fichier Illustrator « compatible PDF » en est un), SVG, image TIFF, PNG ou JPEG,
/// ou refus.
fn reconnaitre(debut: &[u8]) -> Result<Format, Refus> {
    if debut.starts_with(b"%PDF-") {
        return Ok(Format::Pdf);
    }
    match images::genre(debut) {
        Some(images::Genre::Tiff) => return Ok(Format::Tiff),
        Some(images::Genre::Png) => return Ok(Format::Png),
        Some(images::Genre::Jpeg) => return Ok(Format::Jpeg),
        None => {}
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

/// Fiche et retenue de l'ERP telles qu'enregistrées : une action de l'ERP (opérateur,
/// échéance, suspension…) reçue pendant le rendu n'est pas effacée par la copie du rendu.
fn reprendre_fiche(donnees: &std::path::Path, t: &mut Travail) {
    if let Some(enregistre) = lire_travail(donnees, t.id) {
        (t.fiche, t.retenue_erp) = (enregistre.fiche, enregistre.retenue_erp);
    }
}

async fn traiter(etat: Partage, mut t: Travail, place: rendus::Place) {
    let jeton = etat.rendu.attendre(place).await;
    let debut_rendu = std::time::Instant::now();
    let mut a_sortir = false;
    let donnees = &etat.config.donnees;
    t.statut = Statut::EnCours;
    reprendre_fiche(donnees, &mut t);
    let _ = ecrire(donnees, &t);

    let rep = dossier(donnees, t.id);
    let reperage = reperage_voulu(&etat.machines.read().unwrap(), &t.demande);
    let rendu = match t.demande.mode {
        Mode::Imprimer => match preparer_impression(&rep, reperage, t.demande.fond_reperes, t.id).await {
            Ok((pdf, mises)) => {
                let options = t.demande.options_ghostscript(&etat.config.profils);
                match render::rendre(&etat.config.gs, &pdf, &rep, t.demande.dpi, &options, etat.config.delai, &tons::pour_le_travail(donnees, &t.demande.couleur)).await {
                    Ok(r) => match filtrer(&rep, &r.pages, t.demande.edition.filtre.clone(), t.demande.dpi).await {
                        Ok(()) => ajouter_blanc(&rep, r.pages, t.demande.blanc, t.demande.dpi).await.map(|p| (Some(p), mises, r.tons_fondus)),
                        Err(e) => Err(e),
                    },
                    Err(e) => Err(e),
                }
            }
            Err(e) => Err(e),
        },
        Mode::Decouper => Ok((None, vec![], vec![])),
    };
    // Tons directs pris dans la table (roadmap 8.6) : dit sur le travail, pour information.
    t.alertes.retain(|a| a.code != "tons_directs_fondus" && a.code != "ton_direct_absent");
    if let Ok((plaques, _, fondus)) = &rendu {
        if !fondus.is_empty() {
            t.alertes.push(controle::Alerte { code: "tons_directs_fondus".into(), page: None, detail: fondus.join(", ") });
        }
        // Tons ni encre, ni découpe, ni dans la table : dit sur la page, et gardé au journal.
        let mut absents = vec![];
        for (i, p) in plaques.iter().flatten().enumerate() {
            let noms = tons::inconnus(p);
            if !noms.is_empty() {
                t.alertes.push(controle::Alerte { code: "ton_direct_absent".into(), page: Some(i + 1), detail: noms.join(", ") });
                absents.extend(noms);
            }
        }
        absents.sort();
        absents.dedup();
        if !absents.is_empty() {
            tons::noter_inconnus(donnees, &absents, maintenant());
        }
    }
    match rendu {
        Ok((plaques, mises, _)) => match preparer_decoupe(&rep, t.format, t.demande.tous_les_traces, &mises).await {
            // L'image est étendue près des tracés dans la garde ; en échec, le rendu échoue.
            Ok(decoupes) if let Some(Err(e)) = etendre_plaques(&rep, plaques.as_deref(), &decoupes, t.demande.etendre_coupe, t.demande.dpi).await => {
                eprintln!("travail {} : {e}", t.id);
                (t.statut, t.code_erreur, t.erreur) = (Statut::Echec, Some("rendu_echoue".into()), Some(e));
            }
            Ok(decoupes) => {
                // Couches : celles connues gardent ordre et réglages, les nouvelles prennent leur rang.
                for d in &decoupes {
                    decoupe::accorder(&mut t.couches_decoupe, &decoupe::tons(&d.traces));
                }
                // Ancien réglage de coupe du travail entier : sur sa couche, sinon gardé et montré.
                if let Some(n) = t.demande.parametre_decoupe.clone()
                    && decoupe::reprendre_ancien_reglage(&mut t.couches_decoupe, &n)
                {
                    t.demande.parametre_decoupe = None;
                }
                let traces: Vec<usize> = decoupes.iter().map(|d| d.traces.len()).collect();
                let par_couche: Vec<std::collections::BTreeMap<String, usize>> = decoupes.iter().map(|d| t.couches_decoupe.iter().map(|c| (c.nom.clone(), d.traces.iter().filter(|x| decoupe::meme_couche(&x.ton, &c.nom)).count())).filter(|(_, n)| *n > 0).collect()).collect();
                t.alertes.retain(|a| !a.code.starts_with("decoupe_"));
                t.alertes.extend(alertes_decoupe(&rep, &t, traces.iter().sum()).await);
                // Tracés ouverts non tolérés (roadmap 4.11) : ils ne seront pas coupés, on le dit.
                if !decoupe::reglages().traces_ouverts {
                    for (i, d) in decoupes.iter().enumerate() {
                        let ouverts = d.traces.iter().filter(|x| !x.ferme).count();
                        if ouverts > 0 {
                            t.alertes.push(controle::Alerte { code: "decoupe_traces_ouverts".into(), page: Some(i + 1), detail: ouverts.to_string() });
                        }
                    }
                }
                // En découpe seule, il n'y a pas de plaques : une page par jeu de tracés.
                let plaques = plaques.unwrap_or_else(|| vec![vec![]; traces.len()]);
                let geometrie = geometries(&rep, &t, &mises);
                let tuiles: mosaique::Tuiles = if t.demande.mosaique.is_some() { std::fs::read(rep.join("tuiles.json")).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_default() } else { vec![] };
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
                            traces_par_couche: par_couche.get(i).cloned().unwrap_or_default(),
                            surface_m2: mesure.surface_m2,
                            couverture: mesure.couverture,
                            largeur_mm: mesure.largeur_mm,
                            hauteur_mm: mesure.hauteur_mm,
                            cadre_pt: mises.get(i).and_then(|m| m.coins),
                            taille_coupe_pt: decoupes.get(i).map(|d| [d.largeur, d.hauteur]),
                            detourage: 0,
                            reperage: mises.get(i).map(|m| m.reperage),
                            geometrie: geometrie.get(i).copied().flatten(),
                            tuile: tuiles.get(i).cloned().flatten().map(|t| mosaique::Tuile { decalage_mm: mises.get(i).map_or([0.0; 2], |m| [m.decalage.0 / imbrication::PT_PAR_MM, m.decalage.1 / imbrication::PT_PAR_MM]), ..t }),
                        }
                    })
                    .collect();
                match ecrire_coupes(&rep, &t) {
                    Ok(()) => {
                        t.statut = Statut::Termine;
                        a_sortir = true;
                    }
                    Err(e) => (t.statut, t.code_erreur, t.erreur) = (Statut::Echec, Some("decoupe_illisible".into()), Some(e.to_string())),
                }
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
    // Épreuve TIFF demandée par le réglage (roadmap 9.13) : un échec n'arrête pas le travail,
    // il le signale.
    if a_sortir && (t.demande.epreuve.is_some() || t.demande.epreuve_seule) {
        let o = t.demande.epreuve.unwrap_or_default();
        for n in 1..=t.pages.len() {
            let Some(src) = source_epreuve(&etat.config.profils, &rep, &t, n) else { continue };
            if let Err(e) = epreuve::produire(&etat.config.gs, &src, o, &chemin_epreuve(&rep, n), etat.config.delai).await {
                t.alertes.push(controle::Alerte { code: "epreuve_echouee".into(), page: Some(n), detail: e });
            }
        }
    }
    reprendre_fiche(donnees, &mut t);
    if let Err(e) = ecrire(donnees, &t) {
        eprintln!("travail {} : état non enregistré : {e}", t.id);
    }
    // Rapport de bug automatique (s'il est accepté) : rendu en échec, ou anormalement long.
    let duree = debut_rendu.elapsed();
    if matches!(t.statut, Statut::Echec) && let Some(code @ ("rendu_echoue" | "decoupe_illisible")) = t.code_erreur.as_deref() {
        rapports::signaler(donnees, "erreur", code, t.erreur.as_deref().unwrap_or(""));
    } else if duree > Duration::from_secs(60 * env_ou("EKO_RIP_LENTEUR_MIN", "30").parse::<u64>().unwrap_or(30).max(1)) {
        rapports::signaler(donnees, "lenteur", "rendu_lent", &format!("rendu en {} min : {} page(s), {} dpi", duree.as_secs() / 60, t.pages.len(), t.demande.dpi));
    }
    // Mise en file automatique à son tour : après les arrivées automatiques d'avant.
    jeton.terminer(a_sortir.then(|| t.clone()), |t| {
        mettre_en_file_auto(&etat, &t, None);
    });
    // Planche d'imbrication qui n'a pas pu se rendre : ses sorties reviennent.
    if t.imbrication.is_some() && matches!(t.statut, Statut::Echec) {
        reprendre_les_imbriquees(&etat);
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
    /// Contour de toute la planche (`NestTrimBox`) et marges pour la table de découpe
    /// (roadmap 5.5).
    #[serde(default)]
    contour: bool,
    #[serde(default)]
    marge_haut_mm: f64,
    #[serde(default)]
    marge_bas_mm: f64,
    /// Planche manuelle (roadmap 5.10) : copies posées à la main (verrou, grille,
    /// magnétisme), ou gabarit de cellules ; données variables d'un CSV sur une seule page.
    #[serde(default)]
    manuel: Option<cellules::Manuel>,
    #[serde(default)]
    gabarit: Option<cellules::Gabarit>,
    #[serde(default)]
    donnees_variables: Option<cellules::DonneesVariables>,
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

    /// Planche manuelle vérifiée (roadmap 5.10) ; rend les textes de chaque copie, une par
    /// ligne du CSV (vide sans données variables).
    fn manuelle(&self) -> Result<Vec<Vec<cellules::Texte>>, String> {
        if [self.motif.is_some(), self.manuel.is_some(), self.gabarit.is_some()].iter().filter(|b| **b).count() > 1 {
            return Err("motif, poses à la main et gabarit s'excluent".into());
        }
        if let Some(m) = &self.manuel {
            m.verifier()?;
        }
        match &self.donnees_variables {
            None => Ok(vec![]),
            Some(_) if self.elements.len() != 1 || self.motif.is_some() => Err("données variables : une seule page, sans motif".into()),
            Some(d) => cellules::textes(d),
        }
    }

    /// (page, copies) de chaque élément : avec des données variables, une copie par ligne.
    fn pages(&self, textes: &[Vec<cellules::Texte>]) -> Vec<(usize, usize)> {
        self.elements.iter().map(|e| (e.page, if textes.is_empty() { e.copies } else { textes.len() })).collect()
    }

    fn agencement(&self) -> (Option<cellules::Manuel>, Option<cellules::Gabarit>) {
        (self.manuel.clone(), self.gabarit.clone())
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
/// Épreuve TIFF d'une page (roadmap 9.13), faite d'office (`epreuve-NNN.tif`).
fn chemin_epreuve(rep: &std::path::Path, page: usize) -> PathBuf {
    rep.join(format!("epreuve-{page:03}.tif"))
}

/// Plaques d'une page pour l'épreuve : CMJN, puis les tons directs imprimés (pas ceux de
/// découpe). `None` : rien d'imprimé (découpe seule).
fn source_epreuve(profils: &std::path::Path, rep: &std::path::Path, t: &Travail, page: usize) -> Option<epreuve::Source> {
    let p = t.pages.get(page.checked_sub(1)?)?;
    if t.demande.mode != Mode::Imprimer || p.plaques.iter().all(|x| p.plaques_decoupe.contains(x)) {
        return None;
    }
    let tons = p.plaques.iter().filter(|x| !epson::PLAQUES_CMJN.contains(&x.as_str()) && !p.plaques_decoupe.contains(x)).map(|x| (x.clone(), render::chemin_plaque(rep, page, x))).collect();
    Some(epreuve::Source { cmjn: epson::PLAQUES_CMJN.map(|c| render::chemin_plaque(rep, page, c)), tons, dpi: t.demande.dpi as f64, profil: t.demande.couleur.sortie.as_ref().map(|f| profils.join(f)) })
}

#[derive(Deserialize)]
struct ChoixEpreuve {
    couleur: Option<epreuve::Couleur>,
    tons: Option<epreuve::Tons>,
    max_mo: Option<u64>,
}

/// Épreuve TIFF d'une page : celle faite d'office si on ne demande rien d'autre, sinon
/// faite à la demande (`couleur`, `tons`, `max_mo`), envoyée en flux puis effacée.
async fn epreuve_tif(State(etat): State<Partage>, Chemin((id, page)): Chemin<(Uuid, usize)>, axum::extract::Query(q): axum::extract::Query<ChoixEpreuve>) -> Response {
    let donnees = &etat.config.donnees;
    let Some((t, _)) = lire_page(donnees, id, page) else { return introuvable() };
    if !matches!(t.statut, Statut::Termine) {
        return page_absente(donnees, id);
    }
    let rep = dossier(donnees, id);
    let Some(src) = source_epreuve(&etat.config.profils, &rep, &t, page) else { return refus(StatusCode::CONFLICT, "rien_a_imprimer", None) };
    let base = t.demande.epreuve.unwrap_or_default();
    let o = epreuve::Options { couleur: q.couleur.unwrap_or(base.couleur), tons: q.tons.unwrap_or(base.tons), max_mo: q.max_mo.unwrap_or(base.max_mo) };
    if o.verifier().is_err() {
        return refus(StatusCode::BAD_REQUEST, "epreuve_invalide", None);
    }
    let faite = chemin_epreuve(&rep, page);
    let demandee = q.couleur.is_some() || q.tons.is_some() || q.max_mo.is_some();
    let (chemin, temporaire) = if faite.exists() && !demandee {
        (faite, false)
    } else {
        let tmp = rep.join(format!("epreuve-{}.tif", Uuid::new_v4()));
        if let Err(e) = epreuve::produire(&etat.config.gs, &src, o, &tmp, etat.config.delai).await {
            let _ = std::fs::remove_file(&tmp);
            return refus(StatusCode::INTERNAL_SERVER_ERROR, "epreuve_echouee", Some(e));
        }
        (tmp, true)
    };
    let f = match tokio::fs::File::open(&chemin).await {
        Ok(f) => f,
        Err(_) => return introuvable(),
    };
    // Fichier ouvert : le temporaire peut s'effacer, le flux le lit jusqu'au bout.
    if temporaire {
        let _ = std::fs::remove_file(&chemin);
    }
    let nom: String = t.nom.chars().map(|c| if c.is_ascii_alphanumeric() || "-_.".contains(c) { c } else { '_' }).take(80).collect();
    let corps = axum::body::Body::from_stream(tokio_util::io::ReaderStream::new(f));
    ([(header::CONTENT_TYPE, "image/tiff".to_string()), (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{nom}-p{page}-epreuve.tif\""))], corps).into_response()
}

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
        let epreuve: Vec<String> = t.demande.options_ghostscript(&etat.config.profils).iter().map(|o| o.replace("-sOutputICCProfile=", "-sProofProfile=")).collect();
        render::ghostscript(&etat.config.gs, &tmp.join("bat.pdf"), "png16m", dpi, &epreuve, &tmp.join("p-%03d.png"), etat.config.delai).await?;
        let mut pages = vec![];
        for (i, d) in decoupes.into_iter().enumerate() {
            let image = image::open(tmp.join(format!("p-{:03}.png", i + 1))).map_err(|e| e.to_string())?.to_rgb8();
            pages.push(bat::PageBat { image, largeur: d.largeur, hauteur: d.hauteur, traces: d.traces });
        }
        let d = &t.demande;
        let oui_non = |b: bool| if b { if en { "yes" } else { "oui" } } else if en { "no" } else { "non" };
        let traces: usize = t.pages.iter().map(|p| p.traces_decoupe).sum();
        let echelle = d.edition.echelle_xy.map_or(d.edition.echelle.to_string(), |[x, y]| format!("{x} × {y}"));
        let image_seule = d.edition.miroir && d.edition.miroir_portee == edition::PorteeMiroir::ImageSeule;
        let mut lignes = if en {
            vec![
                format!("Mode: {}", if d.mode == Mode::Imprimer { "print" } else { "cut only" }),
                format!("Resolution: {} dpi", d.dpi),
                format!("Media profile: {}", d.couleur.sortie.as_deref().unwrap_or("default")),
                format!("White: {}{}", match d.blanc.mode { blanc::Mode::Aucun => "file's own", blanc::Mode::SousCouche => "underlay", blanc::Mode::Aplat => "flood" }, match d.blanc.alpha { blanc::Alpha::Aucune => "", blanc::Alpha::Blanc => " · alpha → white", blanc::Alpha::Vernis => " · alpha → varnish" }),
                format!("Scale: {echelle} % · rotation {}° · mirror {}{}", d.edition.rotation, oui_non(d.edition.miroir), if image_seule { " (image only)" } else { "" }),
                format!("Grommets: {}", oui_non(d.edition.oeillets.is_some())),
                format!("Cut paths: {traces}"),
            ]
        } else {
            vec![
                format!("Mode : {}", if d.mode == Mode::Imprimer { "impression" } else { "découpe seule" }),
                format!("Résolution : {} dpi", d.dpi),
                format!("Profil du support : {}", d.couleur.sortie.as_deref().unwrap_or("par défaut")),
                format!("Blanc : {}{}", match d.blanc.mode { blanc::Mode::Aucun => "celui du fichier", blanc::Mode::SousCouche => "sous-couche", blanc::Mode::Aplat => "aplat" }, match d.blanc.alpha { blanc::Alpha::Aucune => "", blanc::Alpha::Blanc => " · alpha → blanc", blanc::Alpha::Vernis => " · alpha → vernis" }),
                format!("Échelle : {echelle} % · rotation {}° · miroir {}{}", d.edition.rotation, oui_non(d.edition.miroir), if image_seule { " (image seule)" } else { "" }),
                format!("Œillets : {}", oui_non(d.edition.oeillets.is_some())),
                format!("Tracés de découpe : {traces}"),
            ]
        };
        // L'épreuve se rend depuis le PDF : un filtre couleur (posé sur les plaques) n'y paraît pas.
        if d.edition.filtre.as_ref().is_some_and(|f| !f.neutre()) {
            lignes.push(if en { "Colour filter: yes (applied to the plates, not shown in this proof)" } else { "Filtre couleur : oui (appliqué aux plaques, non visible sur cette épreuve)" }.into());
        }
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
    let mut liste = etat.machines.read().unwrap().clone();
    for m in &mut liste {
        if let machines::Connexion::RipEko { jeton, .. } = &mut m.connexion {
            *jeton = machines::JETON_MASQUE.into();
        }
    }
    Json(liste)
}

/// Renvoi reçu d'un autre E-KO RIP Center (roadmap 9.4) : le flux, déjà dans la langue de
/// la machine, part tel quel par sa connexion, sans passer par la file ; la réponse dit
/// si la machine l'a pris. Le flux passe par le disque : la connexion de la machine ne
/// s'ouvre qu'une fois tout reçu.
async fn recevoir_renvoi(State(etat): State<Partage>, Chemin(id): Chemin<String>, mut corps: Multipart) -> Response {
    let Some(m) = machine(&etat, &id) else { return refus(StatusCode::NOT_FOUND, "introuvable", Some(id)) };
    if m.connexion == machines::Connexion::Dossier {
        return refus(StatusCode::CONFLICT, "renvoi_sans_connexion", Some(m.nom));
    }
    // Pas de renvoi en chaîne : un autre RIP ne passe pas par nous, avec notre jeton, vers
    // un troisième.
    if matches!(m.connexion, machines::Connexion::RipEko { .. }) {
        return refus(StatusCode::CONFLICT, "renvoi_en_chaine", Some(m.nom));
    }
    let rep = etat.config.donnees.join("renvoi");
    let chemin = rep.join(format!("{}.bin", Uuid::new_v4()));
    let (mut nom, mut recu) = (String::from("renvoi"), false);
    let lu: Result<(), String> = async {
        tokio::fs::create_dir_all(&rep).await.map_err(|e| e.to_string())?;
        while let Some(mut champ) = corps.next_field().await.map_err(|e| e.to_string())? {
            match champ.name() {
                Some("nom") => nom = champ.text().await.map_err(|e| e.to_string())?.chars().take(255).collect(),
                Some("flux") => {
                    let mut f = tokio::fs::File::create(&chemin).await.map_err(|e| e.to_string())?;
                    while let Some(morceau) = champ.chunk().await.map_err(|e| e.to_string())? {
                        tokio::io::AsyncWriteExt::write_all(&mut f, &morceau).await.map_err(|e| e.to_string())?;
                    }
                    tokio::io::AsyncWriteExt::flush(&mut f).await.map_err(|e| e.to_string())?;
                    recu = true;
                }
                _ => {}
            }
        }
        Ok(())
    }
    .await;
    let reponse = match lu {
        Err(e) => refus(StatusCode::BAD_REQUEST, "envoi_interrompu", Some(e)),
        Ok(()) if !recu => refus(StatusCode::BAD_REQUEST, "renvoi_vide", None),
        Ok(()) => {
            let (c, f, n) = (m.connexion.clone(), chemin.clone(), nom.clone());
            match tokio::task::spawn_blocking(move || machines::envoyer_fichier(&c, &f, &n)).await {
                Ok(Ok(octets)) => {
                    eprintln!("renvoi « {nom} » : {octets} octets passés à {}", m.id);
                    Json(serde_json::json!({ "machine": m.id, "octets": octets })).into_response()
                }
                Ok(Err(e)) => refus(StatusCode::BAD_GATEWAY, "renvoi_echoue", Some(e)),
                Err(e) => refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string())),
            }
        }
    };
    let _ = tokio::fs::remove_file(&chemin).await;
    reponse
}

/// Remplace le catalogue ; une machine retirée garde ses sorties finies dans l'historique.
async fn enregistrer_machines(State(etat): State<Partage>, Json(mut liste): Json<Vec<machines::Machine>>) -> Response {
    let connues = etat.machines.read().unwrap().clone();
    // Jeton de renvoi masqué à l'écran : celui déjà enregistré pour la machine est gardé,
    // vers le même RIP seulement ; un autre hôte ou port demande de ressaisir le jeton (le
    // masque, refusé par `verifier`, ne part jamais vers un nouvel hôte).
    for m in &mut liste {
        if let machines::Connexion::RipEko { hote, port, jeton, .. } = &mut m.connexion
            && jeton == machines::JETON_MASQUE
            && let Some(machines::Connexion::RipEko { hote: h, port: p, jeton: garde, .. }) = connues.iter().find(|c| c.id == m.id).map(|c| &c.connexion)
            && (h, p) == (hote, port)
        {
            *jeton = garde.clone();
        }
    }
    if let Err(code) = machines::verifier(&liste).and_then(|()| {
        machines::accorder_variantes(&mut liste);
        machines::identifier(&mut liste, &connues)
    }) {
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
    let s = match construire_sauvegarde(&etat).await {
        Ok(s) => s,
        Err(e) => return refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e)),
    };
    let nom = format!("attachment; filename=\"eko-rip-config-{}.json\"", date_utc(s.cree_le).replace([' ', ':'], "-"));
    ([(header::CONTENT_TYPE, "application/json".to_string()), (header::CONTENT_DISPOSITION, nom)], serde_json::to_vec_pretty(&s).unwrap_or_default()).into_response()
}

/// La sauvegarde de la configuration, sans requête HTTP : export à la main et sauvegarde
/// en ligne du cloud en partent. Erreur : le profil ICC illisible.
async fn construire_sauvegarde(etat: &Partage) -> Result<sauvegarde::Sauvegarde, String> {
    let mut profils = std::collections::BTreeMap::new();
    for p in couleur::lister(&etat.config.profils) {
        match tokio::fs::read(etat.config.profils.join(&p.fichier)).await {
            Ok(o) => profils.insert(p.fichier, sauvegarde::hex(&o)),
            Err(e) => return Err(format!("{} : {e}", p.fichier)),
        };
    }
    Ok(sauvegarde::Sauvegarde {
        format: sauvegarde::FORMAT.into(),
        version: env!("CARGO_PKG_VERSION").into(),
        cree_le: maintenant(),
        machines: etat.machines.read().unwrap().clone(),
        tarifs: couverture::lire(&etat.config.donnees),
        reglages: reglages::lire(&etat.config.donnees),
        profils,
        supports: Some(supports::lire(&etat.config.donnees, "epson")),
        decoupe: Some(decoupe::lire(&etat.config.donnees)),
        consommation: Some(consommation::lire_reglages(&etat.config.donnees)),
        tons_directs: Some(tons::lire_table(&etat.config.donnees)),
        pont: Some(erp::lire(&etat.config.donnees).sauve()),
        regles: Some(regles::lire(&etat.config.donnees)),
        balises: Some(balises::lire(&etat.config.donnees)),
        gabarits: Some(cellules::lire(&etat.config.donnees)),
        g7: Some(g7::lire(&etat.config.donnees)),
    })
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
    if let Some(p) = &mut s.pont {
        p.url = match p.url.as_deref().map(erp::url_valide) {
            None => None,
            Some(Some(u)) => Some(u),
            Some(None) => return refus(StatusCode::BAD_REQUEST, "url_erp_invalide", p.url.clone()),
        };
    }
    let connues = etat.machines.read().unwrap().clone();
    // Une sauvegarde porte ses identifiants : ils sont repris tels quels sur un serveur neuf.
    let mut connues_et_sauvees = connues.clone();
    connues_et_sauvees.extend(s.machines.iter().filter(|m| m.uuid.is_some()).cloned());
    if let Err(code) = machines::verifier(&s.machines).and_then(|()| {
        machines::accorder_variantes(&mut s.machines);
        machines::identifier(&mut s.machines, &connues_et_sauvees)
    }).and(s.tarifs.verifier()).and(s.consommation.as_ref().map_or(Ok(()), consommation::Reglages::verifier)) {
        return refus(StatusCode::BAD_REQUEST, code, None);
    }
    if let Some(Err((code, detail))) = s.tons_directs.as_ref().map(tons::verifier) {
        return refus(StatusCode::BAD_REQUEST, code, Some(detail));
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
    let noms: Vec<String> = liste.iter().map(|r| r.nom.clone()).collect();
    if let Some(Err(code)) = s.regles.as_deref().map(|r| regles::verifier(r, &noms)) {
        return refus(StatusCode::BAD_REQUEST, code, None);
    }
    if let Some(Err(code)) = s.balises.as_ref().map(balises::verifier) {
        return refus(StatusCode::BAD_REQUEST, code, None);
    }
    if let Some(Err(code)) = s.gabarits.as_deref().map(cellules::verifier_gabarits) {
        return refus(StatusCode::BAD_REQUEST, "sauvegarde_invalide", Some(code.into()));
    }
    if s.g7.as_ref().is_some_and(|l| l.iter().any(|e| !g7::nom_valide(&e.nom))) {
        return refus(StatusCode::BAD_REQUEST, "sauvegarde_invalide", Some("g7".into()));
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
        if let Some(r) = &s.decoupe {
            decoupe::ecrire(&etat.config.donnees, r)?;
            decoupe::installer(r.clone());
        }
        if let Some(r) = &s.consommation {
            consommation::ecrire_reglages(&etat.config.donnees, r)?;
        }
        if let Some(t) = &s.tons_directs {
            tons::ecrire(&etat.config.donnees, t)?;
        }
        // Pont ERP : URL, activation et instance (le jeton et le secret restent ceux du serveur).
        if let Some(p) = &s.pont {
            let mut c = erp::lire(&etat.config.donnees);
            (c.url, c.actif, c.instance) = (p.url.clone(), p.actif && p.url.is_some() && c.secret_hmac.is_some(), p.instance.or(c.instance));
            erp::ecrire(&etat.config.donnees, &c)?;
            pont::reveiller();
        }
        if let Some(r) = &s.regles {
            regles::ecrire(&etat.config.donnees, r)?;
        }
        if let Some(b) = &s.balises {
            balises::ecrire(&etat.config.donnees, b)?;
        }
        if let Some(g) = &s.gabarits {
            cellules::ecrire(&etat.config.donnees, g)?;
        }
        if let Some(g) = &s.g7 {
            g7::ecrire(&etat.config.donnees, g)?;
        }
        reglages::ecrire(&etat.config.donnees, &liste)
    };
    if let Err(e) = ecrire() {
        return refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string()));
    }
    *etat.machines.write().unwrap() = s.machines.clone();
    Json(serde_json::json!({ "machines": s.machines.len(), "reglages": liste.len(), "profils": profils.len() })).into_response()
}

/// Paquet support (roadmap 1.5) : le support, ses modes et les réglages qui s'en servent.
async fn exporter_paquet(State(etat): State<Partage>, Chemin((id, uuid)): Chemin<(String, Uuid)>) -> Response {
    let c = supports::lire(&etat.config.donnees, "epson");
    let Some(support) = c.supports.into_iter().find(|s| s.machine == id && s.id == Some(uuid)) else { return introuvable() };
    let reglages = reglages::lire(&etat.config.donnees).into_iter().filter(|r| r.demande.support_uuid == Some(uuid)).collect();
    let nom = format!("attachment; filename=\"support-{}.json\"", support.nom.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '-' }).collect::<String>());
    let p = supports::Paquet { format: supports::FORMAT_PAQUET.into(), version: supports::VERSION_PAQUET, serveur: env!("CARGO_PKG_VERSION").into(), support, reglages };
    ([(header::CONTENT_TYPE, "application/json".to_string()), (header::CONTENT_DISPOSITION, nom)], serde_json::to_vec_pretty(&p).unwrap_or_default()).into_response()
}

/// Importe un paquet : le support garde son identifiant (remplacé s'il existe déjà sur cette
/// machine), les réglages sont ajoutés ou remplacés par nom. Tout est contrôlé avant d'écrire.
async fn importer_paquet(State(etat): State<Partage>, Chemin(id): Chemin<String>, Json(p): Json<supports::Paquet>) -> Response {
    if machine(&etat, &id).is_none() {
        return introuvable();
    }
    if p.format != supports::FORMAT_PAQUET {
        return refus(StatusCode::BAD_REQUEST, "paquet_invalide", Some(format!("format « {} »", p.format)));
    }
    if p.version > supports::VERSION_PAQUET {
        return refus(StatusCode::BAD_REQUEST, "paquet_trop_recent", Some(format!("paquet v{}, ce serveur lit jusqu'à la v{}", p.version, supports::VERSION_PAQUET)));
    }
    let _verrou = etat.reglages.lock().await;
    let mut c = supports::lire(&etat.config.donnees, "epson");
    let mut support = supports::Support { machine: id.clone(), ..p.support };
    if support.id.is_none() || support.modes.iter().any(|m| m.id.is_none()) || c.supports.iter().any(|s| s.id == support.id && s.machine != id) {
        return refus(StatusCode::BAD_REQUEST, "paquet_invalide", Some("identifiants manquants ou support d'une autre machine".into()));
    }
    c.supports.retain(|s| s.id != support.id);
    // Un support du même nom (autre identifiant) cède la place : le nom est unique par machine.
    c.supports.retain(|s| !(s.machine == id && s.nom == support.nom));
    support.machine = id.clone();
    c.supports.push(support);
    let de_la_machine: Vec<_> = c.supports.iter().filter(|s| s.machine == id).cloned().collect();
    if let Err(code) = supports::verifier(&de_la_machine) {
        return refus(StatusCode::BAD_REQUEST, code, None);
    }
    let connus = couleur::lister(&etat.config.profils);
    let mut liste = reglages::lire(&etat.config.donnees);
    for r in p.reglages {
        let r = reglages::Reglage { demande: r.demande.normaliser(), ..r };
        if !reglages::nom_valide(&r.nom) {
            return refus(StatusCode::BAD_REQUEST, "reglage_invalide", Some(r.nom));
        }
        if let Err(code) = r.demande.verifier(&connus) {
            return refus(StatusCode::BAD_REQUEST, code, Some(r.nom));
        }
        liste.retain(|x| x.nom != r.nom);
        liste.push(r);
    }
    liste.sort_by(|a, b| a.nom.cmp(&b.nom));
    reglages::identifier(&mut liste, &[]);
    if let Err(e) = supports::ecrire(&etat.config.donnees, &c).and_then(|()| reglages::ecrire(&etat.config.donnees, &liste)) {
        return refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string()));
    }
    StatusCode::NO_CONTENT.into_response()
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

/// Relevé SNMP d'une Epson : encres et longueur restante, n° du support chargé et laize ;
/// Une lecture SNMP d'une Epson : relevé (encres, rouleau), n° de support, laize, chauffages, alertes.
struct Lecture {
    releve: releves::Releve,
    numero: Option<u8>,
    largeur_mm: f64,
    chauffages: Vec<(u8, u8)>,
    alertes: Vec<String>,
}

/// `None` si elle ne répond pas.
async fn lire_releve(hote: String) -> Option<Lecture> {
    let communaute = env_ou("EKO_RIP_EPSON_COMMUNAUTE", "public");
    let lecture = move || {
        let d = Duration::from_secs(2);
        let e = media::lire_etat_epson(&hote, &communaute, d)?;
        let media = media::lire_medias(&hote, &communaute, d).ok()?.into_iter().next()?;
        let alertes = media::lire_alertes(&hote, &communaute, d).unwrap_or_default();
        (!e.encres.is_empty()).then(|| Lecture { releve: releves::Releve { le: maintenant(), encres: e.encres, longueur_m: media.longueur_m }, numero: e.support, largeur_mm: media.largeur_mm, chauffages: e.chauffages, alertes })
    };
    tokio::task::spawn_blocking(lecture).await.ok().flatten()
}

/// Relevé forcé au début ou à la fin d'une impression (roadmap 6.1), écrit même s'il ne
/// change rien : il borne la fenêtre de la sortie. Rend son heure.
async fn relever_maintenant(etat: &Etat, m: &machines::Machine) -> Option<u64> {
    if m.modele != machines::Modele::EpsonScS80600 {
        return None;
    }
    let r = lire_releve(m.snmp.clone()?).await?.releve;
    releves::ajouter(&etat.config.donnees, &m.id, &r).map_err(|e| eprintln!("relevé {} : {e}", m.id)).ok()?;
    Some(r.le)
}

/// Relevé SNMP de chaque Epson toutes les minutes (encres et rouleau), gardé quand il change.
async fn relever(etat: Partage) {
    let mut derniers: std::collections::HashMap<String, Option<releves::Releve>> = Default::default();
    loop {
        let cibles: Vec<(String, String, String)> = etat.machines.read().unwrap().iter().filter(|m| m.modele == machines::Modele::EpsonScS80600).filter_map(|m| Some((m.id.clone(), m.nom.clone(), m.snmp.clone()?))).collect();
        for (id, nom, hote) in cibles {
            let Some(Lecture { releve: r, numero, largeur_mm, chauffages, alertes }) = lire_releve(hote).await else {
                mobile::noter_releve_manque(&etat.config.donnees, &id, &nom);
                continue;
            };
            // Notifications de l'app mobile (alertes, encres) : à chaque relevé, même inchangé.
            mobile::noter_releve(&etat.config.donnees, &id, &nom, &r.encres, &alertes, r.longueur_m, r.le);
            // Support chargé lu sur la machine (n° de support), s'il a changé (roadmap 1.3).
            if let Some(n) = numero {
                let support_uuid = supports::par_numero(&supports::lire(&etat.config.donnees, "epson"), &id, n).and_then(|s| s.id);
                let avant = supports::lire_charges(&etat.config.donnees).remove(&id);
                if avant.as_ref().is_none_or(|a| a.source != "saisie" && (a.support_uuid != support_uuid || a.laize_mm != Some(largeur_mm))) {
                    let _ = supports::ecrire_charge(&etat.config.donnees, &id, supports::Charge { support_uuid, laize_mm: Some(largeur_mm), source: "machine".into(), le: maintenant() });
                }
            }
            // État publié à l'ERP (roadmap 7.9), après le support chargé qu'il reprend.
            pont::noter_machine(&etat.config.donnees, &id, &r, &chauffages, &alertes);
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

/// Rappel de recalibration d'une imprimante (roadmap 8.5) : âge de l'étalonnage et
/// déclencheurs depuis (poches changées d'après les relevés SNMP, lots, maintenance).
async fn recalibration_machine(State(etat): State<Partage>, Chemin(id): Chemin<String>) -> Response {
    if machine(&etat, &id).is_none() {
        return introuvable();
    }
    let fiche = recalibration::lire(&etat.config.donnees).remove(&id).unwrap_or_default();
    Json(recalibration::etat(&fiche, &releves::lire(&etat.config.donnees, &id), maintenant())).into_response()
}

/// Rappels de toutes les machines, par identifiant.
async fn recalibrations(State(etat): State<Partage>) -> Json<std::collections::BTreeMap<String, recalibration::Etat>> {
    let donnees = &etat.config.donnees;
    let mut fiches = recalibration::lire(donnees);
    let ids: Vec<String> = etat.machines.read().unwrap().iter().map(|m| m.id.clone()).collect();
    let quand = maintenant();
    Json(ids.into_iter().map(|id| {
        let e = recalibration::etat(&fiches.remove(&id).unwrap_or_default(), &releves::lire(donnees, &id), quand);
        (id, e)
    }).collect())
}

#[derive(Deserialize)]
struct SaisieEtalonnage {
    #[serde(default)]
    note: String,
}

/// Étalonnage refait à l'instant : il remet les rappels à zéro.
async fn etalonner_machine(State(etat): State<Partage>, Chemin(id): Chemin<String>, Json(s): Json<SaisieEtalonnage>) -> Response {
    if machine(&etat, &id).is_none() {
        return introuvable();
    }
    if s.note.chars().count() > 200 {
        return refus(StatusCode::BAD_REQUEST, "recalibration_invalide", None);
    }
    let _verrou = etat.reglages.lock().await;
    let donnees = &etat.config.donnees;
    let mut fiches = recalibration::lire(donnees);
    recalibration::etalonner(fiches.entry(id.clone()).or_default(), maintenant(), s.note);
    if let Err(e) = recalibration::ecrire(donnees, &fiches) {
        return refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e));
    }
    Json(recalibration::etat(&fiches[&id], &releves::lire(donnees, &id), maintenant())).into_response()
}

/// Nouveau lot ou maintenance saisi : rappel de recalibration (une poche changée se voit
/// seule, dans les relevés).
async fn declencher_recalibration(State(etat): State<Partage>, Chemin(id): Chemin<String>, Json(d): Json<recalibration::Declencheur>) -> Response {
    if machine(&etat, &id).is_none() {
        return introuvable();
    }
    let note = match &d {
        recalibration::Declencheur::NouveauLot { note } | recalibration::Declencheur::Maintenance { note } => note,
        recalibration::Declencheur::Poche { .. } => return refus(StatusCode::BAD_REQUEST, "recalibration_invalide", None),
    };
    if note.chars().count() > 200 {
        return refus(StatusCode::BAD_REQUEST, "recalibration_invalide", None);
    }
    let _verrou = etat.reglages.lock().await;
    let donnees = &etat.config.donnees;
    let mut fiches = recalibration::lire(donnees);
    let fiche = fiches.entry(id.clone()).or_default();
    fiche.evenements.push(recalibration::Evenement { le: maintenant(), declencheur: d });
    if let Err(e) = recalibration::ecrire(donnees, &fiches) {
        return refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e));
    }
    Json(recalibration::etat(&fiches[&id], &releves::lire(donnees, &id), maintenant())).into_response()
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
    let textes = match d.manuelle() {
        Ok(t) => t,
        Err(e) => return mauvais(e),
    };
    let donnees = &etat.config.donnees;
    let mut pdfs = vec![];
    let mut noms = vec![];
    for e in &d.elements {
        let Some((t, _)) = lire_page(donnees, e.id, e.page) else { return refus(StatusCode::NOT_FOUND, "introuvable", None) };
        if t.pages_exclues.contains(&e.page) {
            return refus(StatusCode::CONFLICT, "page_exclue", None);
        }
        match tokio::fs::read(dossier(donnees, e.id).join(Format::Pdf.fichier())).await {
            Ok(o) => pdfs.push(o),
            Err(err) => return refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(err.to_string())),
        }
        noms.push(format!("{} p{}", t.nom, e.page));
    }
    let pages = d.pages(&textes);
    let ((laize, espace, rotation, forme, motif), (manuel, gabarit)) = ((d.laize_mm, d.espacement_mm, d.rotation, d.forme, d.motif), d.agencement());
    let imp = match tokio::task::spawn_blocking(move || {
        let sources: Vec<imbrication::Source> = pdfs.iter().zip(&pages).map(|(pdf, &(page, copies))| imbrication::Source { pdf, page, copies }).collect();
        imbrication::imposer(&sources, laize, espace, rotation, forme, motif, imbrication::Manuelle { poses: manuel.as_ref(), gabarit: gabarit.as_ref() })
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
    let textes = match d.manuelle() {
        Ok(t) => t,
        Err(e) => return mauvais(&e),
    };
    let donnees = &etat.config.donnees;
    let mut pdfs = vec![];
    for e in &d.elements {
        let Some((t, _)) = lire_page(donnees, e.id, e.page) else { return refus(StatusCode::NOT_FOUND, "introuvable", None) };
        if t.pages_exclues.contains(&e.page) {
            return refus(StatusCode::CONFLICT, "page_exclue", None);
        }
        if t.format != Format::Pdf {
            return mauvais("seules des pages PDF s'imbriquent");
        }
        match tokio::fs::read(dossier(donnees, e.id).join(Format::Pdf.fichier())).await {
            Ok(o) => pdfs.push(o),
            Err(err) => return refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(err.to_string())),
        }
    }
    if !(0.0..=200.0).contains(&d.marge_haut_mm) || !(0.0..=200.0).contains(&d.marge_bas_mm) {
        return mauvais("marges 0–200 mm");
    }
    let (laize, espace, rotation, forme, motif) = (d.laize_mm, d.espacement_mm, d.rotation, d.forme, d.motif);
    let finition = imbrication::Finition { marge_bas: d.marge_bas_mm * imbrication::PT_PAR_MM, marge_haut: d.marge_haut_mm * imbrication::PT_PAR_MM, contour: d.contour, ..Default::default() };
    let pages = d.pages(&textes);
    let copies: usize = pages.iter().map(|p| p.1).sum();
    let (manuel, gabarit) = d.agencement();
    let compose = tokio::task::spawn_blocking(move || {
        let sources: Vec<imbrication::Source> = pdfs.iter().zip(&pages).map(|(pdf, &(page, copies))| imbrication::Source { pdf, page, copies }).collect();
        imbrication::composer(&sources, laize, espace, rotation, forme, motif, finition, imbrication::Manuelle { poses: manuel.as_ref(), gabarit: gabarit.as_ref() }, &textes)
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
    let nom = match (&d.motif, &d.gabarit) {
        (Some(m), _) => format!("Motif {} × {}{} — {laize:.0} mm", m.colonnes, m.rangees, if m.demi_saut { " demi-saut" } else { "" }),
        (None, Some(g)) => format!("Gabarit {} — {copies} élément(s)", g.nom),
        (None, None) => format!("Planche {laize:.0} mm — {copies} élément(s)"),
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

/// Rôle d'une personne connectée par la plate-forme cloud (« Se connecter au cloud ») ;
/// `lecture` : rôle d'opérateur, en lecture seule.
fn role_cloud(i: &eko_rip_cloud::applis::IdentiteCloud) -> (acces::Role, bool) {
    match i.role.as_str() {
        "admin" => (acces::Role::Admin, false),
        "operateur" => (acces::Role::Operateur, false),
        _ => (acces::Role::Operateur, true),
    }
}

async fn garder(State(etat): State<Partage>, mut requete: axum::extract::Request, suite: axum::middleware::Next) -> Response {
    let par_cloud = requete.extensions().get::<cloud::ParCloud>().is_some();
    // Identité posée par la plate-forme : crue seulement venue par le tunnel (la plate-forme
    // retire cet en-tête des requêtes qu'elle reçoit) ; sur le réseau local, ignorée.
    let identite = if par_cloud { requete.headers().get(eko_rip_cloud::applis::ENTETE_IDENTITE).and_then(|v| v.to_str().ok()).and_then(eko_rip_cloud::applis::lire_identite) } else { None };
    requete.headers_mut().remove(eko_rip_cloud::applis::ENTETE_IDENTITE);
    if let Some(i) = &identite {
        requete.extensions_mut().insert(i.clone());
    }
    let chemin = requete.uri().path().to_string();
    let chemin = chemin.as_str();
    // Ordre de restauration de la plate-forme : pas de session, mais une signature HMAC avec
    // la clé du site (la plate-forme seule la connaît avec ce serveur). Sans signature valide,
    // 401 ; une requête du réseau local n'a jamais la marque `Signee`.
    if par_cloud && chemin == "/api/cloud/restaurer" && requete.method() == axum::http::Method::POST {
        let (parts, corps) = requete.into_parts();
        let Ok(octets) = axum::body::to_bytes(corps, 4096).await else { return refus(StatusCode::UNAUTHORIZED, "signature_cloud_invalide", None) };
        let entete = parts.headers.get(abonnement::ENTETE_SERVICE).and_then(|v| v.to_str().ok());
        if !cloud::lire(&etat.config.donnees).is_some_and(|c| abonnement::signature_valide(&c.cle, entete, &octets, maintenant())) {
            return refus(StatusCode::UNAUTHORIZED, "signature_cloud_invalide", None);
        }
        let mut requete = axum::extract::Request::from_parts(parts, axum::body::Body::from(octets));
        requete.extensions_mut().insert(abonnement::Signee);
        return suite.run(requete).await;
    }
    // Venue par le tunnel du cloud : jamais le pont ERP, le renvoi entre RIP ni les réglages
    // du pont et du cloud ; et jamais sans compte (l'accès libre reste au réseau local), sauf
    // pour une personne identifiée par la plate-forme.
    if par_cloud {
        if chemin.starts_with("/api/renvoi/") || chemin.starts_with(erp::PREFIXE) || chemin.starts_with("/api/pont") || chemin.starts_with("/api/cloud") && requete.method() != axum::http::Method::GET {
            return refus(StatusCode::FORBIDDEN, "ferme_par_cloud", None);
        }
        if identite.is_none() && acces::libre(&etat.config.donnees) {
            return refus(StatusCode::FORBIDDEN, "cloud_sans_compte", None);
        }
    }
    // Pont ERP (exception accordée par le gérant le 24/10) : le jeton de l'ERP ouvre ses
    // seules routes ; elles restent fermées sans lui, même quand l'accès humain est libre.
    let bearer = requete.headers().get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer "));
    // Renvoi d'un autre RIP (roadmap 9.4) : son jeton n'ouvre que cette route.
    if chemin.starts_with("/api/renvoi/") {
        return if bearer.is_some_and(|j| ports::jeton_valide(&etat.config.donnees, j)) { suite.run(requete).await } else { refus(StatusCode::UNAUTHORIZED, "jeton_renvoi_requis", None) };
    }
    if chemin.starts_with(erp::PREFIXE) {
        return if bearer.is_some_and(|j| erp::jeton_valide(&erp::lire(&etat.config.donnees), j)) { suite.run(requete).await } else { refus(StatusCode::UNAUTHORIZED, "jeton_erp_requis", None) };
    }
    if bearer.is_some() {
        return refus(StatusCode::FORBIDDEN, "jeton_erp_hors_pont", None);
    }
    if chemin.starts_with("/api/") && !est_client(&etat, requete.headers()) {
        return refus(StatusCode::FORBIDDEN, "client_requis", None);
    }
    if !chemin.starts_with("/api/") || chemin == "/api/session" {
        return suite.run(requete).await;
    }
    let role = match &identite {
        Some(i) => {
            let (role, lecture_seule) = role_cloud(i);
            // Lecture seule : s'inscrire aux notifications du téléphone reste permis.
            if lecture_seule && requete.method() != axum::http::Method::GET && !chemin.starts_with("/api/mobile/") {
                return refus(StatusCode::FORBIDDEN, "lecture_seule", None);
            }
            role
        }
        None if acces::libre(&etat.config.donnees) => return suite.run(requete).await,
        None => match role_session(&etat, requete.headers()) {
            Some(r) => r,
            None => return refus(StatusCode::UNAUTHORIZED, "connexion_requise", None),
        },
    };
    if role != acces::Role::Admin && acces::reserve_admin(requete.method(), chemin) {
        return refus(StatusCode::FORBIDDEN, "admin_requis", None);
    }
    suite.run(requete).await
}

/// Contrat du pont ERP : nom et version, version du serveur, modèle d'états (lot R1).
async fn contrat_erp() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "contrat": erp::CONTRAT, "serveur": env!("CARGO_PKG_VERSION"), "signature": "X-EKO-RIP-Signature: t=<secondes Unix>,v1=<hex HMAC-SHA256(secret, \"<t>.<corps>\")>", "etats": erp::modele_etats() }))
}

/// Réglage du pont depuis Système (administrateur). Secret absent ou vide : celui en place
/// est gardé. Il n'est jamais renvoyé.
#[derive(Deserialize)]
struct ReglagePont {
    #[serde(default)]
    url: Option<String>,
    actif: bool,
    #[serde(default)]
    secret: Option<String>,
}

/// « Signaler un problème » (Système, clients Mac et Linux) : transmis à la plate-forme, avec
/// le journal masqué du serveur si demandé. Rend la référence du rapport.
async fn signaler_probleme(State(etat): State<Partage>, Json(s): Json<rapports::Signalement>) -> Response {
    let corps = match rapports::preparer(&etat.config.donnees, &s) {
        Ok(c) => c,
        Err(code) => return refus(StatusCode::BAD_REQUEST, code, None),
    };
    let joindre = s.joindre_journal;
    match tokio::task::spawn_blocking(move || rapports::envoyer_signalement(corps, joindre)).await {
        Ok(Ok(reference)) => (StatusCode::CREATED, Json(serde_json::json!({ "reference": reference }))).into_response(),
        Ok(Err(code)) if code == rapports::RAPPORT_INVALIDE => refus(StatusCode::BAD_REQUEST, code, None),
        Ok(Err(code)) if code == rapports::RAPPORTS_SATURE => refus(StatusCode::TOO_MANY_REQUESTS, code, None),
        _ => refus(StatusCode::BAD_GATEWAY, rapports::PLATEFORME_INJOIGNABLE, None),
    }
}

/// Rapports automatiques : `auto` vaut null tant que l'administrateur n'a pas répondu.
async fn lire_reglage_rapports(State(etat): State<Partage>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "auto": rapports::auto(&etat.config.donnees) }))
}

#[derive(Deserialize)]
struct ReglageRapports {
    auto: bool,
}

async fn regler_rapports(State(etat): State<Partage>, Json(r): Json<ReglageRapports>) -> Response {
    match rapports::regler_auto(&etat.config.donnees, r.auto) {
        Ok(()) => Json(serde_json::json!({ "auto": r.auto })).into_response(),
        Err(_) => refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", None),
    }
}

/// Statistiques anonymes d'installation (Système).
async fn lire_telemetrie(State(etat): State<Partage>) -> Json<serde_json::Value> {
    Json(telemetrie::etat_public(&etat.config.donnees))
}

#[derive(Deserialize)]
struct ReglageTelemetrie {
    actif: bool,
}

/// Coche ou décoche l'envoi (administrateur).
async fn regler_telemetrie(State(etat): State<Partage>, Json(r): Json<ReglageTelemetrie>) -> Response {
    let donnees = &etat.config.donnees;
    match telemetrie::modifier(donnees, |e| e.desactive = !r.actif) {
        Ok(_) => Json(telemetrie::etat_public(donnees)).into_response(),
        Err(_) => refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", None),
    }
}

/// Cloud E-KO RIP (Système) : rattachement, état du tunnel, appairage en cours.
async fn lire_cloud(State(etat): State<Partage>) -> Json<serde_json::Value> {
    Json(cloud::etat_public(&etat.config.donnees))
}

/// Après une écriture réussie de l'API : la configuration a pu changer (sauvegarde en ligne
/// 2 minutes plus tard, si elle diffère de la précédente) et, pour les machines, l'inventaire
/// de l'abonnement est renvoyé à la plate-forme.
async fn signaler_changement(requete: axum::extract::Request, suite: axum::middleware::Next) -> Response {
    let (methode, chemin) = (requete.method().clone(), requete.uri().path().to_string());
    let ecriture = ![axum::http::Method::GET, axum::http::Method::HEAD, axum::http::Method::OPTIONS].contains(&methode)
        && chemin.starts_with("/api/")
        && !["/api/file", "/api/travaux", "/api/session", "/api/imbrications", "/api/rapports"].iter().any(|p| chemin.starts_with(p))
        && (!chemin.starts_with("/api/cloud") || chemin == "/api/cloud/restaurer");
    let r = suite.run(requete).await;
    if ecriture && r.status().is_success() {
        abonnement::config_modifiee(chemin == "/api/machines" || chemin == "/api/sauvegarde" || chemin == "/api/cloud/restaurer");
    }
    r
}

/// Sauvegarde en ligne tout de suite (administrateur) : rend l'état de la sauvegarde.
async fn sauvegarder_cloud(State(etat): State<Partage>) -> Response {
    if cloud::lire(&etat.config.donnees).is_none() {
        return refus(StatusCode::CONFLICT, "cloud_non_rattache", None);
    }
    let s = abonnement::sauvegarder(&etat, "demande").await;
    if s.etat == "erreur" {
        return refus(StatusCode::BAD_GATEWAY, "cloud_injoignable", s.message);
    }
    Json(abonnement::etat_public(&etat.config.donnees).0).into_response()
}

/// Restaure une sauvegarde en ligne sur ordre de la plate-forme (signé : voir `garder`) :
/// même chemin que `POST /api/sauvegarde`.
async fn restaurer_par_cloud(State(etat): State<Partage>, signee: Option<axum::Extension<abonnement::Signee>>, corps: axum::body::Bytes) -> Response {
    if signee.is_none() {
        return refus(StatusCode::UNAUTHORIZED, "signature_cloud_invalide", None);
    }
    let Some(c) = cloud::lire(&etat.config.donnees) else { return refus(StatusCode::CONFLICT, "cloud_non_rattache", None) };
    let Some(commande) = serde_json::from_slice::<abonnement::Commande>(&corps).ok().filter(|c| abonnement::id_valide(&c.id)) else {
        return refus(StatusCode::BAD_REQUEST, "sauvegarde_invalide", None);
    };
    let id = commande.id;
    let octets = match tokio::task::spawn_blocking(move || abonnement::telecharger(&c, &id)).await {
        Ok(Ok(o)) => o,
        Ok(Err(404)) => return introuvable(),
        _ => return refus(StatusCode::BAD_GATEWAY, "cloud_injoignable", None),
    };
    let Ok(s) = serde_json::from_slice::<sauvegarde::Sauvegarde>(&octets) else { return refus(StatusCode::BAD_REQUEST, "sauvegarde_invalide", None) };
    let r = restaurer_config(State(etat), Json(s)).await;
    if r.status().is_success() { Json(serde_json::json!({ "restaure": true })).into_response() } else { r }
}

/// Demande un code d'appairage à la plate-forme (administrateur). Refusé sans compte :
/// l'accès libre du réseau local ne doit jamais sortir de l'atelier.
async fn appairer_cloud(State(etat): State<Partage>) -> Response {
    let donnees = etat.config.donnees.clone();
    if acces::libre(&donnees) {
        return refus(StatusCode::CONFLICT, "cloud_sans_compte", None);
    }
    if cloud::lire(&donnees).is_some() {
        return refus(StatusCode::CONFLICT, "cloud_deja_rattache", None);
    }
    let url = cloud::url_plateforme();
    match tokio::task::spawn_blocking(move || cloud::demander(&url)).await {
        Ok(Ok(_)) => {
            tokio::spawn(cloud::suivre(donnees.clone()));
            Json(cloud::etat_public(&donnees)).into_response()
        }
        Ok(Err(code)) => refus(StatusCode::BAD_GATEWAY, code, None),
        Err(_) => refus(StatusCode::BAD_GATEWAY, "cloud_injoignable", None),
    }
}

/// Détache le serveur de la plate-forme : le tunnel se ferme, la clé est oubliée.
async fn detacher_cloud(State(etat): State<Partage>) -> Response {
    match cloud::detacher(&etat.config.donnees) {
        Ok(()) => Json(cloud::etat_public(&etat.config.donnees)).into_response(),
        Err(e) => refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string())),
    }
}

async fn lire_pont(State(etat): State<Partage>) -> Json<serde_json::Value> {
    Json(pont::etat_public(&etat.config.donnees))
}

async fn regler_pont(State(etat): State<Partage>, Json(r): Json<ReglagePont>) -> Response {
    let donnees = &etat.config.donnees;
    let url = match r.url.as_deref().map(str::trim).filter(|u| !u.is_empty()) {
        None => None,
        Some(u) => match erp::url_valide(u) {
            Some(u) => Some(u),
            None => return refus(StatusCode::BAD_REQUEST, "url_erp_invalide", None),
        },
    };
    let secret = r.secret.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    if secret.as_deref().is_some_and(|s| !erp::secret_valide(s)) {
        return refus(StatusCode::BAD_REQUEST, "secret_erp_invalide", None);
    }
    let mut c = erp::lire(donnees);
    c.url = url;
    if secret.is_some() {
        c.secret_hmac = secret;
    }
    c.actif = r.actif;
    if c.actif && (c.url.is_none() || c.secret_hmac.is_none()) {
        return refus(StatusCode::BAD_REQUEST, "pont_incomplet", None);
    }
    if c.instance.is_none() {
        c.instance = Some(Uuid::new_v4());
    }
    if let Err(e) = erp::ecrire(donnees, &c) {
        return refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string()));
    }
    pont::reveiller();
    Json(pont::etat_public(donnees)).into_response()
}

/// Nouvelle instance (installation remplacée, anomalie) : le journal repart de 1 et l'ERP la
/// prend comme une installation neuve.
async fn nouvelle_instance(State(etat): State<Partage>) -> Response {
    let donnees = &etat.config.donnees;
    let mut c = erp::lire(donnees);
    c.instance = Some(Uuid::new_v4());
    if let Err(e) = erp::ecrire(donnees, &c) {
        return refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string()));
    }
    pont::reveiller();
    Json(pont::etat_public(donnees)).into_response()
}

/// `eko-rip-serveur renvoi jeton|retirer|etat` : jeton que les autres RIP présentent pour
/// envoyer à nos machines (roadmap 9.4), affiché une seule fois.
fn commande_renvoi(donnees: &std::path::Path, action: Option<&str>) -> i32 {
    let mut c = ports::lire(donnees);
    let nouveau = erp::aleatoire();
    match action {
        Some("jeton") => c.jeton_sha256 = Some(erp::empreinte(&nouveau)),
        Some("retirer") => c.jeton_sha256 = None,
        Some("etat") => {
            println!("jeton de renvoi : {}", if c.jeton_sha256.is_some() { "déclaré" } else { "aucun (renvoi fermé)" });
            return 0;
        }
        _ => {
            eprintln!("usage : eko-rip-serveur renvoi jeton|retirer|etat");
            return 2;
        }
    }
    if let Err(e) = std::fs::create_dir_all(donnees).and_then(|_| ports::ecrire(donnees, &c)) {
        eprintln!("renvoi.json : {e}");
        return 1;
    }
    if c.jeton_sha256.is_some() {
        println!("{nouveau}");
        eprintln!("À recopier maintenant dans la machine « Renvoi » de l'autre RIP : il ne sera plus affiché. Le précédent ne vaut plus.");
    }
    0
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

async fn lire_session(State(etat): State<Partage>, cloud: Option<axum::Extension<eko_rip_cloud::applis::IdentiteCloud>>, entetes: axum::http::HeaderMap) -> Json<serde_json::Value> {
    // Connecté par la plate-forme : pas de compte local à ouvrir.
    if let Some(axum::Extension(i)) = cloud {
        let (role, lecture_seule) = role_cloud(&i);
        return Json(serde_json::json!({ "acces_libre": false, "utilisateur": i.email, "role": role, "cloud": { "email": i.email, "nom": i.nom, "lecture_seule": lecture_seule } }));
    }
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

// --- Application mobile de suivi (iOS, Android) ---

/// Code d'état d'une sortie, tel que l'API le donne.
fn code_sortie(e: file::Etat) -> &'static str {
    match e {
        file::Etat::EnAttente => "en_attente",
        file::Etat::Suspendue => "suspendue",
        file::Etat::EnCours => "en_cours",
        file::Etat::Terminee => "terminee",
        file::Etat::Echec => "echec",
        file::Etat::Annulee => "annulee",
        file::Etat::Imbriquee => "imbriquee",
    }
}

/// Toutes les 5 s, la file comparée à la précédente : échecs et fins vers les téléphones.
async fn veiller_la_file(etat: Partage) {
    loop {
        let sorties = etat.file.lock().unwrap().sorties.clone();
        let noms: std::collections::HashMap<String, String> = etat.machines.read().unwrap().iter().map(|m| (m.id.clone(), m.nom.clone())).collect();
        let vues: Vec<mobile::VueSortie> = sorties
            .iter()
            .filter(|s| !s.archivee)
            .map(|s| mobile::VueSortie { id: s.id, travail: s.travail, machine: &s.machine, nom: &s.nom, page: s.page, etat: code_sortie(s.etat), decoupe: s.operation() == machines::Operation::Decoupe, erreur: s.erreur.as_deref().or(s.code_erreur.as_deref()) })
            .collect();
        mobile::noter_file(&etat.config.donnees, &vues, &noms);
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

/// Écran d'accueil de l'app : chaque machine avec son dernier relevé, sa sortie en cours
/// et ce qui l'attend ; et le numéro du dernier événement (pastille « Alertes »).
async fn etat_mobile(State(etat): State<Partage>) -> Json<serde_json::Value> {
    let donnees = &etat.config.donnees;
    let sorties = etat.file.lock().unwrap().sorties.clone();
    let catalogue = etat.machines.read().unwrap().clone();
    let machines: Vec<serde_json::Value> = catalogue
        .iter()
        .map(|m| {
            let releve = mobile::etat_machine(donnees, &m.id).unwrap_or_default();
            let a_elle = || sorties.iter().filter(|s| s.machine == m.id && !s.archivee);
            let en_cours = a_elle().find(|s| s.etat == file::Etat::EnCours).map(|s| serde_json::json!({ "sortie": s.id, "nom": s.nom, "page": s.page, "debut": s.debut }));
            let operations: Vec<machines::Operation> = [machines::Operation::Impression, machines::Operation::Decoupe].into_iter().filter(|&o| m.fait(o)).collect();
            serde_json::json!({
                "id": m.id,
                "nom": m.nom,
                "modele": m.modele,
                "operations": operations,
                "snmp": m.snmp.is_some(),
                "joignable": releve.joignable(),
                "releve_le": releve.releve_le,
                "encres": releve.encres.iter().map(|(e, pc)| serde_json::json!({ "encre": e, "pourcent": pc })).collect::<Vec<_>>(),
                "longueur_restante_m": releve.longueur_m,
                "alertes": releve.alertes.iter().map(|a| mobile::alerte_en_francais(a)).collect::<Vec<_>>(),
                "en_cours": en_cours,
                "en_attente": a_elle().filter(|s| s.etat == file::Etat::EnAttente).count(),
                "suspendues": a_elle().filter(|s| s.etat == file::Etat::Suspendue).count(),
            })
        })
        .collect();
    let (_, dernier) = mobile::evenements(donnees, u64::MAX, 0);
    Json(serde_json::json!({ "version": env!("CARGO_PKG_VERSION"), "dernier_evenement": dernier, "machines": machines }))
}

#[derive(Deserialize)]
struct DepuisEvenement {
    #[serde(default)]
    depuis: u64,
    limite: Option<usize>,
}

async fn evenements_mobiles(State(etat): State<Partage>, axum::extract::Query(q): axum::extract::Query<DepuisEvenement>) -> Json<serde_json::Value> {
    let (evenements, dernier) = mobile::evenements(&etat.config.donnees, q.depuis, q.limite.unwrap_or(50).min(200));
    Json(serde_json::json!({ "evenements": evenements, "dernier": dernier }))
}

#[derive(Deserialize)]
struct Inscription {
    jeton: String,
    #[serde(default)]
    nom: String,
    #[serde(default)]
    plateforme: String,
    #[serde(default)]
    fins: bool,
}

/// Inscrit un téléphone pour les notifications, au nom du compte connecté.
async fn inscrire_appareil(State(etat): State<Partage>, cloud: Option<axum::Extension<eko_rip_cloud::applis::IdentiteCloud>>, entetes: axum::http::HeaderMap, Json(i): Json<Inscription>) -> Response {
    if !mobile::jeton_valide(&i.jeton) {
        return refus(StatusCode::BAD_REQUEST, "jeton_push_invalide", None);
    }
    // Par le cloud : l'e-mail du compte de la plate-forme, gardé tant que le téléphone se
    // réinscrit (voir `mobile::DUREE_INSCRIPTION_CLOUD`) ; sinon le compte local de la session.
    let (utilisateur, par_cloud) = match cloud {
        Some(axum::Extension(c)) => (Some(c.email), true),
        None => (jeton_requete(&entetes).and_then(|j| etat.sessions.lock().unwrap().utilisateur(&j, std::time::Instant::now())), false),
    };
    let a = mobile::Appareil { jeton: i.jeton, nom: i.nom, plateforme: i.plateforme, utilisateur, fins: i.fins, inscrit_le: maintenant(), cloud: par_cloud };
    match mobile::inscrire(&etat.config.donnees, a) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string())),
    }
}

#[derive(Deserialize)]
struct JetonPush {
    jeton: String,
}

async fn retirer_appareil(State(etat): State<Partage>, Json(j): Json<JetonPush>) -> Response {
    match mobile::retirer(&etat.config.donnees, &j.jeton) {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string())),
    }
}

/// Notification d'essai, vers un téléphone inscrit seulement (le serveur ne relaie pas
/// vers n'importe quel jeton).
async fn essai_mobile(State(etat): State<Partage>, Json(j): Json<JetonPush>) -> Response {
    if !mobile::appareils(&etat.config.donnees).iter().any(|a| a.jeton == j.jeton) {
        return refus(StatusCode::NOT_FOUND, "appareil_inconnu", None);
    }
    match tokio::task::spawn_blocking(move || mobile::essai(&j.jeton)).await {
        Ok(Ok(())) => StatusCode::NO_CONTENT.into_response(),
        Ok(Err(e)) => refus(StatusCode::BAD_GATEWAY, "envoi_push_echoue", Some(e)),
        Err(e) => refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string())),
    }
}

async fn lister_reglages(State(etat): State<Partage>) -> Json<Vec<reglages::Reglage>> {
    Json(reglages::lire(&etat.config.donnees))
}

/// Règles du dossier `entrees-auto/` (roadmap 9.1), dans leur ordre d'essai.
async fn lire_regles(State(etat): State<Partage>) -> Json<Vec<regles::Regle>> {
    Json(regles::lire(&etat.config.donnees))
}

/// Remplace toutes les règles ; chacune doit citer un réglage enregistré.
async fn enregistrer_regles(State(etat): State<Partage>, Json(r): Json<Vec<regles::Regle>>) -> Response {
    let _verrou = etat.reglages.lock().await;
    let noms: Vec<String> = reglages::lire(&etat.config.donnees).into_iter().map(|x| x.nom).collect();
    if let Err(code) = regles::verifier(&r, &noms) {
        return refus(StatusCode::BAD_REQUEST, code, None);
    }
    match regles::ecrire(&etat.config.donnees, &r) {
        Ok(()) => Json(r).into_response(),
        Err(e) => refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string())),
    }
}

/// Gabarits de cellules de la planche manuelle (roadmap 5.10).
async fn lire_gabarits(State(etat): State<Partage>) -> Json<Vec<cellules::Gabarit>> {
    Json(cellules::lire(&etat.config.donnees))
}

async fn enregistrer_gabarits(State(etat): State<Partage>, Json(g): Json<Vec<cellules::Gabarit>>) -> Response {
    let _verrou = etat.reglages.lock().await;
    if let Err(code) = cellules::verifier_gabarits(&g) {
        return refus(StatusCode::BAD_REQUEST, code, None);
    }
    match cellules::ecrire(&etat.config.donnees, &g) {
        Ok(()) => Json(g).into_response(),
        Err(e) => refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string())),
    }
}

/// Motif de nom à balises du dossier `entrees-auto/` (roadmap 9.10).
async fn lire_balises(State(etat): State<Partage>) -> Json<balises::Config> {
    Json(balises::lire(&etat.config.donnees))
}

async fn enregistrer_balises(State(etat): State<Partage>, Json(c): Json<balises::Config>) -> Response {
    if let Err(code) = balises::verifier(&c) {
        return refus(StatusCode::BAD_REQUEST, code, Some(c.motif));
    }
    match balises::ecrire(&etat.config.donnees, &c) {
        Ok(()) => Json(c).into_response(),
        Err(e) => refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string())),
    }
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

#[derive(Deserialize)]
struct ChoixReglages {
    /// Noms séparés par des virgules ; absent : tous.
    noms: Option<String>,
}

const FORMAT_REGLAGES: &str = "eko-rip-reglages";

/// Réglages choisis, en un fichier (roadmap 1.6).
async fn exporter_reglages(State(etat): State<Partage>, axum::extract::Query(q): axum::extract::Query<ChoixReglages>) -> Response {
    let noms: Option<Vec<&str>> = q.noms.as_deref().map(|n| n.split(',').map(str::trim).filter(|n| !n.is_empty()).collect());
    let liste: Vec<_> = reglages::lire(&etat.config.donnees).into_iter().filter(|r| noms.as_ref().is_none_or(|n| n.contains(&r.nom.as_str()))).collect();
    let corps = serde_json::json!({ "format": FORMAT_REGLAGES, "version": 1, "serveur": env!("CARGO_PKG_VERSION"), "reglages": liste });
    ([(header::CONTENT_TYPE, "application/json"), (header::CONTENT_DISPOSITION, "attachment; filename=\"reglages.json\"")], serde_json::to_vec_pretty(&corps).unwrap_or_default()).into_response()
}

#[derive(Deserialize)]
struct ImportReglages {
    format: String,
    version: u32,
    reglages: Vec<reglages::Reglage>,
    /// `ecraser` ou `renommer` (défaut) quand un réglage du même nom existe.
    #[serde(default)]
    doublon: Option<String>,
}

/// Ajoute les réglages d'un export ; un doublon de nom est écrasé (il garde son
/// identifiant local) ou renommé « nom 2 » (nouvel identifiant). Tout est contrôlé avant.
async fn importer_reglages(State(etat): State<Partage>, Json(i): Json<ImportReglages>) -> Response {
    if i.format != FORMAT_REGLAGES || i.version > 1 {
        return refus(StatusCode::BAD_REQUEST, "paquet_invalide", Some(format!("{} v{}", i.format, i.version)));
    }
    let ecraser = match i.doublon.as_deref() {
        Some("ecraser") => true,
        None | Some("renommer") => false,
        Some(x) => return refus(StatusCode::BAD_REQUEST, "paquet_invalide", Some(format!("doublon « {x} »"))),
    };
    let connus = couleur::lister(&etat.config.profils);
    let catalogue = supports::lire(&etat.config.donnees, "epson");
    let _verrou = etat.reglages.lock().await;
    let mut liste = reglages::lire(&etat.config.donnees);
    let avant = liste.clone();
    let mut noms = vec![];
    for r in i.reglages {
        let mut r = reglages::Reglage { uuid: None, demande: r.demande.normaliser(), ..r };
        if !reglages::nom_valide(&r.nom) {
            return refus(StatusCode::BAD_REQUEST, "reglage_invalide", Some(r.nom));
        }
        if let Err(code) = supports::appliquer(&mut r.demande, &catalogue).and_then(|()| r.demande.verifier(&connus)) {
            return refus(StatusCode::BAD_REQUEST, code, Some(r.nom));
        }
        if liste.iter().any(|x| x.nom == r.nom) {
            if ecraser {
                liste.retain(|x| x.nom != r.nom);
            } else {
                let pris: Vec<String> = liste.iter().map(|x| x.nom.clone()).collect();
                r.nom = reglages::nom_libre(&r.nom, &pris);
            }
        }
        noms.push(r.nom.clone());
        liste.push(r);
    }
    liste.sort_by(|a, b| a.nom.cmp(&b.nom));
    reglages::identifier(&mut liste, &avant);
    match reglages::ecrire(&etat.config.donnees, &liste) {
        Ok(()) => Json(serde_json::json!({ "importes": noms })).into_response(),
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
        // Journal des impressions (roadmap 6.4) : au-delà de la rétention.
        let jours = consommation::lire_reglages(donnees).retention_jours as u64;
        match consommation::purger(donnees, maintenant().saturating_sub(jours * 86_400)) {
            Ok(n) if n > 0 => eprintln!("journal des impressions : {n} impression(s) de plus de {jours} jours retirée(s)"),
            Err(e) => eprintln!("journal des impressions : {e}"),
            _ => {}
        }
        for id in a_purger(&lire_tous(donnees), maintenant(), etat.config.conservation_jours) {
            if effacer_travail(&etat, id).await {
                eprintln!("travail {id} purgé (plus de {} jours)", etat.config.conservation_jours);
            }
        }
        // Travaux de l'ERP faits, si l'option est active (roadmap 0.8).
        if etat.config.purger_erp_faits {
            let sorties = etat.file.lock().unwrap().sorties.clone();
            for id in erp_faits(&lire_tous(donnees), &sorties) {
                // Relu juste avant d'effacer : relancé ou remis en file entre-temps, il reste.
                let encore = lire_travail(donnees, id).is_some_and(|t| !erp_faits(std::slice::from_ref(&t), &etat.file.lock().unwrap().sorties).is_empty());
                if encore && effacer_travail(&etat, id).await {
                    eprintln!("travail {id} de l'ERP purgé (fait)");
                }
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

/// Travaux de l'ERP faits (roadmap 0.8) : rendus, au moins une sortie terminée, aucune à
/// passer, ni posée sur une planche pas encore imprimée, ni en échec (à relancer par
/// l'opérateur). Les estimations ont leur propre purge.
fn erp_faits(travaux: &[Travail], sorties: &[file::Sortie]) -> Vec<Uuid> {
    let fait = |t: &Travail| {
        let siennes = || sorties.iter().filter(|s| s.travail == t.id);
        siennes().any(|s| s.etat == file::Etat::Terminee) && !siennes().any(|s| s.active() || s.etat == file::Etat::Imbriquee || (s.etat == file::Etat::Echec && !s.archivee))
    };
    travaux.iter().filter(|t| t.erp.is_some() && !t.estimation && matches!(t.statut, Statut::Termine) && fait(t)).map(|t| t.id).collect()
}

/// Niveau du disque des données (roadmap 0.8) : `bas` sous le seuil d'alerte, `critique`
/// sous le seuil de refus et de purge (0 : seuil absent).
fn niveau_disque(libre: u64, seuil_alerte: u64, seuil_critique: u64) -> &'static str {
    if seuil_critique > 0 && libre < seuil_critique {
        "critique"
    } else if seuil_alerte > 0 && libre < seuil_alerte {
        "bas"
    } else {
        "ok"
    }
}

/// Travaux inactifs, du plus vieux au plus récent : ordre de purge sous le seuil. Un dépôt
/// mis en erreur faute de place (roadmap 0.8) n'en est pas : son dossier garde le seul
/// exemplaire du fichier, à relancer ; seule la conservation en jours l'efface.
fn plus_vieux_inactifs(travaux: &[Travail]) -> Vec<Uuid> {
    let mut v: Vec<&Travail> = travaux.iter().filter(|t| !actif(&t.statut) && t.code_erreur.as_deref() != Some("espace_insuffisant")).collect();
    v.sort_by_key(|t| t.cree_le);
    v.into_iter().map(|t| t.id).collect()
}

/// Espace du disque des données, poids du tampon de la file (roadmap 9.6) et version.
async fn systeme(State(etat): State<Partage>) -> Json<serde_json::Value> {
    let (total, libre) = espace_disque(&etat.config.donnees).await.map_or((None, None), |(t, l)| (Some(t), Some(l)));
    let sorties = etat.file.lock().unwrap().sorties.clone();
    let donnees = etat.config.donnees.clone();
    let tampon = tokio::task::spawn_blocking(move || poids_tampon(&donnees, &sorties)).await.unwrap_or(0);
    let occupation = etat.rendu.occupation();
    Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "tampon": tampon,
        "rendus": { "simultanes": etat.rendu.simultanes(), "en_cours": occupation.0, "en_attente": occupation.1 },
        "disque_total": total,
        "disque_libre": libre,
        "conservation_jours": etat.config.conservation_jours,
        "seuil_libre": etat.config.seuil_libre,
        "seuil_alerte": etat.config.seuil_alerte,
        "disque_niveau": libre.map(|l| niveau_disque(l, etat.config.seuil_alerte, etat.config.seuil_libre)),
        "alertes": etat::alertes(),
    }))
}

/// Dossiers surveillés : toutes les 5 s, chaque fichier stable de `entrees/<réglage>/`
/// devient un travail avec ce réglage, et chaque fichier de `entrees-auto/` un travail avec
/// le réglage de la première règle qui le prend (roadmap 9.1) ; un refus range le fichier
/// dans `refuses/`.
async fn surveiller(etat: Partage) {
    let mut vus = std::collections::HashMap::new();
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        let donnees = etat.config.donnees.clone();
        let liste = reglages::lire(&donnees);
        let auto = regles::dossier(&donnees);
        let mut fichiers = vec![];
        for dossier in liste.iter().map(|r| reglages::dossier_entree(&donnees, &r.nom)).chain([auto.clone()]) {
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
            let resultat = if chemin.parent() == Some(auto.as_path()) {
                match preparer_auto(&etat, &chemin, &liste).await {
                    Ok((nom, d, fiche, alertes)) => prendre_fichier(&etat, &chemin, d, fiche, alertes).await.map_err(|e| format!("réglage {nom} : {e}")),
                    Err(e) => Err(e),
                }
            } else {
                let Some(r) = chemin.parent().and_then(|d| d.file_name()).and_then(|n| n.to_str()).and_then(|n| liste.iter().find(|r| r.nom == n)) else { continue };
                prendre_fichier(&etat, &chemin, r.demande.clone(), Fiche::default(), vec![]).await
            };
            if let Err(raison) = resultat {
                eprintln!("dossier surveillé : {} refusé : {raison}", chemin.display());
                let refuses = chemin.with_file_name("refuses");
                let _ = std::fs::create_dir_all(&refuses);
                // Nom non UTF-8 : rangé quand même, sous son nom lisible.
                if let Some(nom) = chemin.file_name().map(|n| n.to_string_lossy()) {
                    let cible = regles::place_refus(&refuses, &nom);
                    let _ = std::fs::rename(&chemin, &cible);
                    let _ = std::fs::write(cible.with_extension("refus.txt"), format!("{raison}\n"));
                }
            }
        }
    }
}

/// Réglage d'un fichier de `entrees-auto/` : première règle qui le prend, par son nom et,
/// pour un PDF, le format de sa première page.
async fn reglage_par_regle<'a>(donnees: &std::path::Path, chemin: &std::path::Path, liste: &'a [reglages::Reglage]) -> Result<&'a reglages::Reglage, String> {
    let nom = chemin.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
    let c = chemin.to_path_buf();
    let format = tokio::task::spawn_blocking(move || regles::format_pdf(&c)).await.ok().flatten();
    let regles = regles::lire(donnees);
    let r = regles::choisir(&regles, &nom, format).ok_or_else(|| "aucune_regle : aucune règle ne prend ce fichier".to_string())?;
    liste.iter().find(|x| x.nom == r.reglage).ok_or_else(|| format!("aucune_regle : réglage « {} » introuvable", r.reglage))
}

fn alerte(code: &str, detail: String) -> controle::Alerte {
    controle::Alerte { code: code.into(), page: None, detail }
}

/// Fichier de `entrees-auto/` : son nom lu avec le motif à balises (roadmap 9.10), puis son
/// réglage (balise `{reglage}`, sinon les règles) et ce que les autres balises y changent.
/// Rend le nom du réglage, la demande, la fiche et les alertes de lecture ; chaque alerte est
/// aussi écrite au journal du service. `Err` : raison du refus, alertes comprises.
async fn preparer_auto(etat: &Partage, chemin: &std::path::Path, liste: &[reglages::Reglage]) -> Result<(String, Demande, Fiche, Vec<controle::Alerte>), String> {
    use balises::Balise;
    let donnees = &etat.config.donnees;
    let nom = chemin.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
    let motif = balises::lire(donnees).motif;
    let mut alertes = vec![];
    let valeurs = if motif.is_empty() {
        vec![]
    } else {
        balises::extraire(&motif, &nom).unwrap_or_else(|| {
            alertes.push(alerte("nom_non_conforme", motif.clone()));
            vec![]
        })
    };
    let valeur = |b: Balise| valeurs.iter().find(|(x, _)| *x == b).map(|(_, v)| v.clone());
    let invalide = |b: Balise, v: &str| alerte("balise_invalide", format!("{} : « {v} »", b.nom()));
    let par_balise = valeur(Balise::Reglage).and_then(|v| {
        let r = liste.iter().find(|r| balises::cle(&r.nom) == balises::cle(&v));
        if r.is_none() {
            alertes.push(invalide(Balise::Reglage, &v));
        }
        r
    });
    let journal = |alertes: &[controle::Alerte]| alertes.iter().map(|a| format!("{} ({})", a.code, a.detail)).collect::<Vec<_>>().join(" ; ");
    let r = match par_balise {
        Some(r) => r,
        None => reglage_par_regle(donnees, chemin, liste).await.map_err(|e| if alertes.is_empty() { e } else { format!("{e} ; {}", journal(&alertes)) })?,
    };
    let (mut d, mut fiche) = (r.demande.clone(), Fiche::default());
    for (b, v) in &valeurs {
        let bon = match b {
            Balise::Nom | Balise::Reglage => true,
            Balise::Machine => {
                let c = etat.machines.read().unwrap();
                match c.iter().find(|m| balises::cle(&m.id) == balises::cle(v) || balises::cle(&m.nom) == balises::cle(v)) {
                    Some(m) if m.fait(machines::Operation::Impression) => {
                        d.imprimante = Some(m.id.clone());
                        true
                    }
                    Some(m) => {
                        d.decoupeuse = Some(m.id.clone());
                        true
                    }
                    None => false,
                }
            }
            // Après la boucle : le support dépend de l'imprimante, que `{machine}` peut poser plus loin.
            Balise::Support => true,
            Balise::Copies => balises::entier(v, 100_000).map(|n| fiche.quantite = n).is_some(),
            Balise::Repetition => balises::entier(v, reglages::REPETITION_MAX).map(|n| d.repetition = n).is_some(),
            Balise::Laize => match (balises::nombre(v, "mm").filter(|l| (50.0..=5000.0).contains(l)), d.mosaique.as_mut()) {
                (Some(l), Some(m)) => {
                    m.laize_mm = l;
                    true
                }
                (Some(_), None) => {
                    alertes.push(alerte("balise_sans_effet", format!("laize : « {v} »")));
                    true
                }
                (None, _) => false,
            },
            Balise::Echelle => balises::nombre(v, "%").filter(|e| (1.0..=1000.0).contains(e)).map(|e| (d.edition.echelle, d.edition.echelle_xy) = (e, None)).is_some(),
            Balise::Largeur | Balise::Hauteur => balises::nombre(v, "mm").is_some_and(|x| x > 0.0 && x <= 100_000.0),
        };
        if !bon {
            alertes.push(invalide(*b, v));
        }
    }
    // Support : celui de ce nom sur l'imprimante retenue d'abord, sinon le premier de ce nom.
    if let Some(v) = valeur(Balise::Support) {
        let catalogue = supports::lire(donnees, "epson").supports;
        let du_nom = |s: &&supports::Support| balises::cle(&s.nom) == balises::cle(&v);
        let sur_l_imprimante = |s: &&supports::Support| d.imprimante.as_ref().is_some_and(|i| &s.machine == i);
        match catalogue.iter().filter(du_nom).find(sur_l_imprimante).or_else(|| catalogue.iter().find(du_nom)) {
            Some(s) => {
                // Le mode du réglage reste s'il est de ce support ; sinon la résolution du réglage.
                if !s.modes.iter().any(|m| m.id.is_some() && m.id == d.mode_uuid) {
                    d.mode_uuid = None;
                }
                d.support_uuid = s.id;
            }
            None => alertes.push(invalide(Balise::Support, &v)),
        }
    }
    // Largeur et hauteur voulues : l'échelle en découle, proportions gardées (la plus petite
    // des deux si elles ne s'accordent pas).
    let voulu = |b: Balise| valeur(b).and_then(|v| balises::nombre(&v, "mm")).filter(|x| *x > 0.0 && *x <= 100_000.0);
    let (largeur, hauteur) = (voulu(Balise::Largeur), voulu(Balise::Hauteur));
    if largeur.is_some() || hauteur.is_some() {
        let c = chemin.to_path_buf();
        let page = tokio::task::spawn_blocking(move || regles::dimensions_pdf(&c)).await.ok().flatten();
        match page {
            Some([l, h]) => {
                let echelles: Vec<f64> = [largeur.map(|x| x / l * 100.0), hauteur.map(|x| x / h * 100.0)].into_iter().flatten().collect();
                let e = echelles.iter().copied().fold(f64::INFINITY, f64::min);
                if echelles.iter().any(|x| (x - e).abs() > e * 0.01) {
                    alertes.push(alerte("balise_proportions", format!("{:.0} × {:.0} mm", l * e / 100.0, h * e / 100.0)));
                }
                if (1.0..=1000.0).contains(&e) {
                    (d.edition.echelle, d.edition.echelle_xy) = ((e * 100.0).round() / 100.0, None);
                } else {
                    alertes.push(alerte("balise_invalide", format!("échelle : {e:.0} %")));
                }
            }
            None => alertes.push(alerte("balise_sans_effet", "largeur, hauteur : format de page inconnu".into())),
        }
    }
    for a in &alertes {
        eprintln!("dossier automatique : {nom} : {} {}", a.code, a.detail);
    }
    Ok((r.nom.clone(), d, fiche, alertes))
}

/// Fichier d'un dossier surveillé → nouveau travail. Le fichier est déplacé dans le
/// dossier du travail ; en cas de refus, il est remis à sa place pour être rangé.
async fn prendre_fichier(etat: &Partage, chemin: &std::path::Path, d: Demande, fiche: Fiche, depart: Vec<controle::Alerte>) -> Result<(), String> {
    let mut debut = vec![0u8; EN_TETE];
    let n = std::io::Read::read(&mut std::fs::File::open(chemin).map_err(|e| e.to_string())?, &mut debut).map_err(|e| e.to_string())?;
    let format = reconnaitre(&debut[..n]).map_err(|(code, _)| code.to_string())?;
    let nom: String = chemin.file_name().and_then(|n| n.to_str()).unwrap_or("sans nom").chars().take(200).collect();
    let id = Uuid::new_v4();
    let rep = dossier(&etat.config.donnees, id);
    std::fs::create_dir_all(&rep).map_err(|e| e.to_string())?;
    std::fs::rename(chemin, rep.join("source.tmp")).map_err(|e| e.to_string())?;
    match creer_travail_avec(etat, id, nom, format, d, fiche, Origine::Dossier, depart).await {
        Ok(_) => Ok(()),
        Err((_, (code, detail))) => {
            let _ = std::fs::rename(rep.join("source.tmp"), chemin).or_else(|_| std::fs::rename(rep.join(format.fichier()), chemin));
            let _ = std::fs::remove_dir_all(&rep);
            Err(detail.map_or(code.to_string(), |d| format!("{code} : {d}")))
        }
    }
}

/// Envois de l'ERP téléchargés par le pont (lot R3) → travaux, avec le réglage enregistré
/// désigné. Le travail prend l'uuid de l'envoi : après une coupure, un envoi déjà devenu
/// travail n'en refait pas un second. Un refus part à l'ERP (`travail.refuse`).
async fn prendre_envois(etat: Partage) {
    loop {
        pont::attendre_recus(Duration::from_secs(30)).await;
        prendre_envois_recus(&etat).await;
        appliquer_actions_erp(&etat);
        estimer_pour_l_erp(&etat);
    }
}

/// Estimations demandées par l'ERP (lot R4, feuille de route 7.6), chacune sur les travaux
/// rendus d'un dossier (`ref_erp`), ou sur un fichier rendu à part pour un devis. Un dossier
/// dont un travail est encore en rendu attend le tour suivant ; sans travail rendu :
/// `aucun_travail`.
fn estimer_pour_l_erp(etat: &Etat) {
    let donnees = &etat.config.donnees;
    for e in pont::estimations(donnees) {
        let travaux: Vec<Travail> = lire_tous(donnees).into_iter().filter(|t| t.erp.as_ref().is_some_and(|l| l.ref_erp == e.ref_erp)).collect();
        if travaux.iter().any(|t| matches!(t.statut, Statut::EnAttente | Statut::EnCours)) {
            continue;
        }
        let rendus: Vec<&Travail> = travaux.iter().filter(|t| matches!(t.statut, Statut::Termine)).collect();
        if rendus.is_empty() {
            pont::noter_estimation(donnees, &e, Some(pont::ESTIMATION_SANS_TRAVAIL), serde_json::json!({}));
            continue;
        }
        pont::noter_estimation(donnees, &e, None, chiffres_estimation(etat, &rendus));
    }
    // Fichiers à estimer (devis) : rendus à part, répondus puis effacés.
    for t in lire_tous_avec(donnees).into_iter().filter(|t| t.estimation) {
        let Some(l) = &t.erp else { continue };
        let e = pont::DemandeEstimation { id: t.id, ref_erp: l.ref_erp.clone() };
        match t.statut {
            Statut::EnAttente | Statut::EnCours => continue,
            Statut::Echec => pont::noter_estimation(donnees, &e, Some(t.code_erreur.as_deref().unwrap_or("rendu_echoue")), serde_json::json!({ "detail": t.erreur })),
            Statut::Termine => pont::noter_estimation(donnees, &e, None, chiffres_estimation(etat, &[&t])),
        }
        let _ = std::fs::remove_dir_all(dossier(donnees, t.id));
    }
}

/// Les chiffres d'une estimation : ceux de la fiche de chaque page (surface, encre par
/// plaque, durée sur la première imprimante, comme `lister`), multipliés par les copies du
/// travail, pages exclues à part ; et la longueur de rouleau de toutes les copies imbriquées
/// ensemble sur la laize chargée de cette imprimante, marges de la planche comprises (`null`
/// si une page ne tient pas sur la laize). Aucun montant : l'ERP fait ses prix (décision D8).
fn chiffres_estimation(etat: &Etat, travaux: &[&Travail]) -> serde_json::Value {
    let donnees = &etat.config.donnees;
    let tarifs = couverture::lire(donnees);
    let imprimante = etat.machines.read().unwrap().iter().find(|m| m.fait(machines::Operation::Impression)).cloned();
    let laize = imprimante.as_ref().map(|m| supports::lire_charges(donnees).get(&m.id).and_then(|c| c.laize_mm).map_or(m.laize_mm, |l| l.min(m.laize_mm)));
    let (mut pages, mut copies, mut surface, mut les) = (0usize, 0usize, 0.0, 0usize);
    let mut encres: std::collections::BTreeMap<String, f64> = Default::default();
    let mut duree = Some(0.0);
    let mut candidats = vec![];
    for t in travaux {
        let n = t.fiche.copies();
        let pdf = (t.format == Format::Pdf).then(|| std::fs::read(dossier(donnees, t.id).join(Format::Pdf.fichier())).ok()).flatten();
        for (i, p) in t.pages.iter().enumerate().filter(|(i, _)| !t.pages_exclues.contains(&(i + 1))) {
            pages += 1;
            copies += n;
            surface += p.surface_m2 * n as f64;
            for l in couverture::estimer(p.surface_m2, p.hauteur_mm, &p.couverture, &p.plaques_decoupe, &tarifs, t.demande.support.as_deref()).encres {
                *encres.entry(l.nom).or_default() += l.ml * n as f64;
            }
            if t.demande.mode == Mode::Imprimer {
                duree = duree.zip(imprimante.as_ref().and_then(|m| m.duree_impression_min(p.hauteur_mm, t.demande.dpi))).map(|(d, m)| d + m * n as f64);
            }
            les += if p.tuile.is_some() { n } else { 0 };
            if let Some(taille) = pdf.as_ref().and_then(|pdf| imbrication::taille_page(pdf, i + 1).ok()) {
                candidats.push(nid::Candidat { taille, copies: n, le: p.tuile.as_ref().map(|x| x.numero), echeance: t.fiche.echeance, arrivee: t.cree_le });
            }
        }
    }
    // Toutes les copies sur une planche : ni longueur cible ni rangée pleine.
    let longueur = imprimante.as_ref().zip(laize).filter(|_| !candidats.is_empty()).and_then(|(m, laize)| {
        let r = nid::Reglage { longueur_cible_mm: None, longueur_max_mm: None, rangee_pleine: false, ..m.imbrication.clone() };
        let plan = nid::planifier(&candidats, laize, &r).filter(|p| p.retenus.len() == candidats.len())?;
        Some(plan.planche.hauteur / imbrication::PT_PAR_MM + r.marge_haut_mm + r.marge_bas_mm)
    });
    let arrondi = |v: f64, n: i32| (v * 10f64.powi(n)).round() / 10f64.powi(n);
    let ml: f64 = encres.values().sum();
    serde_json::json!({
        "travaux": travaux.len(),
        "pages": pages,
        "copies": copies,
        "surface_m2": arrondi(surface, 4),
        "encres": encres.iter().map(|(nom, ml)| serde_json::json!({ "nom": nom, "ml": arrondi(*ml, 2) })).collect::<Vec<_>>(),
        "ml": arrondi(ml, 2),
        "duree_impression_min": duree.filter(|_| imprimante.is_some()).map(|d| arrondi(d, 1)),
        "machine_uuid": imprimante.as_ref().and_then(|m| m.uuid),
        "laize_mm": laize,
        "longueur_mm": longueur.map(|l| arrondi(l, 1)),
        "les": les,
    })
}

/// Actions de l'ERP sur les travaux d'un dossier (lot R3.2, feuille de route 7.7), chacune
/// sur tous les travaux de même `ref_erp`. Une sortie lancée, terminée ou posée sur une
/// planche ne change pas : un travail imprimé ne s'annule pas, il se signale
/// (`deja_imprime`). Reprendre ne relance pas une découpe qui attend la fin de son
/// impression (le support doit d'abord passer de machine).
fn appliquer_actions_erp(etat: &Etat) {
    let donnees = &etat.config.donnees;
    for a in pont::actions(donnees) {
        let mut travaux: Vec<Travail> = lire_tous(donnees).into_iter().filter(|t| t.erp.as_ref().is_some_and(|l| l.ref_erp == a.ref_erp)).collect();
        if travaux.is_empty() {
            pont::noter_action(donnees, &a, Some(pont::ACTION_SANS_TRAVAIL), 0, 0);
            continue;
        }
        let ids: Vec<Uuid> = travaux.iter().map(|t| t.id).collect();
        let retenue = match a.action {
            pont::GenreAction::Suspendre => Some(Some(RetenueErp::Suspendu)),
            pont::GenreAction::Annuler => Some(Some(RetenueErp::Annule)),
            pont::GenreAction::Reprendre => Some(None),
            _ => None,
        };
        let (mut touchees, mut non_modifiables) = (0, 0);
        for t in &mut travaux {
            let avant = (t.retenue_erp, t.fiche.echeance, t.fiche.operateur.clone());
            if let Some(r) = retenue {
                t.retenue_erp = r;
            }
            match a.action {
                pont::GenreAction::Echeance => {
                    t.fiche.echeance = a.echeance;
                    touchees += 1;
                }
                pont::GenreAction::Operateur => {
                    t.fiche.operateur = a.operateur.clone().unwrap_or_default();
                    touchees += 1;
                }
                _ => {}
            }
            if (t.retenue_erp, t.fiche.echeance, t.fiche.operateur.clone()) != avant
                && let Err(e) = ecrire(donnees, t)
            {
                eprintln!("action de l'ERP {} : travail {} non écrit : {e}", a.id, t.id);
            }
        }
        // Réimpression : un travail rendu, sans sortie encore en file, repart comme à son
        // arrivée ; un travail en cours de rendu, rendu mais pas encore sorti (il attend son
        // tour derrière un rendu plus ancien), déjà en file ou posé sur une planche pas encore
        // imprimée n'est pas doublé.
        if a.action == pont::GenreAction::Reimprimer {
            for t in &travaux {
                let en_file = etat.file.lock().unwrap().sorties.iter().any(|s| s.travail == t.id && (s.active() || s.etat == file::Etat::Imbriquee));
                if !matches!(t.statut, Statut::Termine) || en_file || etat.rendu.retenu(|x| x.id == t.id) {
                    non_modifiables += 1;
                    continue;
                }
                touchees += mettre_en_file_auto(etat, t, a.page);
            }
            let code = (touchees == 0).then_some(pont::ACTION_RIEN_A_REIMPRIMER);
            pont::noter_action(donnees, &a, code, touchees, non_modifiables);
            continue;
        }
        if !matches!(a.action, pont::GenreAction::Echeance | pont::GenreAction::Operateur) {
            let mut f = etat.file.lock().unwrap();
            let a_imprimer = |f: &file::File, s: &file::Sortie| f.sorties.iter().any(|x| x.travail == s.travail && x.page == s.page && x.operation() == machines::Operation::Impression && x.etat != file::Etat::Terminee);
            let concernees: Vec<Uuid> = f.sorties.iter().filter(|s| ids.contains(&s.travail) && !s.archivee && !matches!(s.etat, file::Etat::Annulee | file::Etat::Echec)).map(|s| s.id).collect();
            // En tête dans l'ordre inverse : les sorties gardent leur ordre entre elles.
            for id in concernees.iter().rev() {
                let Some(s) = f.sorties.iter().find(|s| s.id == *id).cloned() else { continue };
                let modifiable = matches!(s.etat, file::Etat::EnAttente | file::Etat::Suspendue);
                if !modifiable {
                    non_modifiables += 1;
                    continue;
                }
                let action = match a.action {
                    pont::GenreAction::Suspendre if s.etat == file::Etat::EnAttente => Some(file::Action::Suspendre),
                    pont::GenreAction::Reprendre if s.etat == file::Etat::Suspendue && !(s.operation() == machines::Operation::Decoupe && a_imprimer(&f, &s)) => Some(file::Action::Reprendre),
                    pont::GenreAction::Annuler => Some(file::Action::Annuler),
                    pont::GenreAction::Priorite => Some(file::Action::EnTete),
                    _ => None,
                };
                if let Some(action) = action
                    && f.agir(s.id, action, maintenant()).is_ok()
                {
                    touchees += 1;
                }
            }
            if let Err(e) = f.ecrire(donnees) {
                eprintln!("action de l'ERP {} : file non enregistrée : {e}", a.id);
            }
        }
        reprendre_les_imbriquees(etat);
        let code = (a.action == pont::GenreAction::Annuler && non_modifiables > 0).then_some(pont::ACTION_DEJA_IMPRIME);
        pont::noter_action(donnees, &a, code, touchees, non_modifiables);
    }
}

async fn prendre_envois_recus(etat: &Partage) {
    let donnees = etat.config.donnees.clone();
    for (r, fichier) in pont::recus(&donnees) {
        let rep = dossier(&donnees, r.id);
        if rep.join("job.json").exists() {
            pont::oublier_recu(&donnees, r.id);
            continue;
        }
        let refus = |code: &str, detail: Option<String>| {
            if r.estimation {
                pont::noter_estimation(&donnees, &pont::DemandeEstimation { id: r.id, ref_erp: r.ref_erp.clone() }, Some(code), serde_json::json!({ "detail": detail }));
            } else {
                pont::noter_refus(&donnees, r.id, Some(&r.ref_erp), code, detail.as_deref());
            }
            pont::oublier_recu(&donnees, r.id);
        };
        let Some(reglage) = reglages::lire(&donnees).into_iter().find(|x| x.uuid == Some(r.prereglage_uuid)) else {
            refus(pont::REFUS_REGLAGE_INCONNU, Some(r.prereglage_uuid.to_string()));
            continue;
        };
        let mut debut = vec![0u8; EN_TETE];
        let lu = std::fs::File::open(&fichier).and_then(|mut f| std::io::Read::read(&mut f, &mut debut));
        let format = match lu.map(|n| reconnaitre(&debut[..n])) {
            Ok(Ok(f)) => f,
            Ok(Err((code, detail))) => {
                refus(code, detail);
                continue;
            }
            Err(e) => {
                refus(pont::REFUS_FICHIER_ILLISIBLE, Some(e.to_string()));
                continue;
            }
        };
        let deplace = std::fs::create_dir_all(&rep).and_then(|()| std::fs::rename(&fichier, rep.join("source.tmp")));
        if let Err(e) = deplace {
            eprintln!("envoi de l'ERP {} : non pris, repris plus tard : {e}", r.id);
            continue;
        }
        let fiche = Fiche { commande: r.ref_erp.clone(), quantite: r.copies, echeance: r.echeance, ..Default::default() };
        let lien = pont::Lien { envoi: r.id, ref_erp: r.ref_erp.clone(), partie: r.partie.clone() };
        let origine = if r.estimation { Origine::Estimation(lien) } else { Origine::Erp(lien) };
        match creer_travail_avec(etat, r.id, r.nom.clone(), format, reglage.demande, fiche, origine, vec![]).await {
            Ok(_) => pont::oublier_recu(&donnees, r.id),
            Err((_, (code, detail))) => {
                let _ = std::fs::remove_dir_all(&rep);
                refus(code, detail);
            }
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
        Ok(()) => {
            pont::reveiller();
            (StatusCode::CREATED, Json(profil)).into_response()
        }
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
/// `seule` : cette page seulement (page recochée, roadmap 5.12). Les pages exclues n'y vont
/// pas ; une planche d'imbrication va sur sa machine.
/// Rend le nombre de sorties ajoutées.
fn mettre_en_file_auto(etat: &Etat, t: &Travail, seule: Option<usize>) -> usize {
    let mut a = t.demande.auto;
    if (!a.imprimer && !a.decouper) || t.retenue_erp == Some(RetenueErp::Annule) || t.estimation || t.demande.epreuve_seule {
        return 0;
    }
    // Travaux liés (roadmap 7.8) : rang de chaque partie du groupe, lu avant de prendre la file.
    let rangs: Option<(u32, std::collections::HashMap<Uuid, u32>)> = t.erp.as_ref().and_then(|l| l.partie.as_ref()).map(|p| {
        let du_groupe = lire_tous(&etat.config.donnees).into_iter().filter_map(|x| x.erp.and_then(|l| l.partie).filter(|q| q.groupe == p.groupe).map(|q| (x.id, q.rang)));
        (p.rang, du_groupe.chain([(t.id, p.rang)]).collect())
    });
    a.suspendre |= t.retenue_erp == Some(RetenueErp::Suspendu);
    // Première machine du catalogue qui imprime ; pour découper, elle-même si elle découpe
    // aussi (imprimante-découpeuse), sinon la première découpeuse.
    // La découpeuse affectée au travail, si elle est au catalogue.
    let (imprimante, decoupeuse) = {
        let c = etat.machines.read().unwrap();
        let i = match &t.imbrication {
            Some(b) => c.iter().find(|m| m.id == b.machine),
            None => t.demande.imprimante.as_ref().and_then(|id| c.iter().find(|m| &m.id == id && m.fait(machines::Operation::Impression))).or_else(|| c.iter().find(|m| m.fait(machines::Operation::Impression))),
        };
        let choisie = t.demande.decoupeuse.as_ref().and_then(|id| c.iter().find(|m| &m.id == id && m.fait(machines::Operation::Decoupe)));
        let d = choisie.or_else(|| i.filter(|m| m.fait(machines::Operation::Decoupe))).or_else(|| c.iter().find(|m| m.fait(machines::Operation::Decoupe)));
        (i.map(|m| m.id.clone()), d.cloned())
    };
    let mut f = etat.file.lock().unwrap();
    let avant = f.sorties.len();
    for (i, p) in t.pages.iter().enumerate().filter(|(i, _)| seule.is_none_or(|n| n == i + 1) && !t.pages_exclues.contains(&(i + 1))) {
        let imprime = a.imprimer && t.demande.mode == Mode::Imprimer && p.plaques.iter().any(|x| !p.plaques_decoupe.contains(x));
        let mut ajouter = |machine: &str, operation, suspendue: bool| {
            let mut s = file::Sortie::nouvelle(machine, operation, t.id, &t.nom, i + 1, maintenant());
            if suspendue {
                s.etat = file::Etat::Suspendue;
            }
            f.sorties.push(s);
        };
        if let (true, Some(m)) = (imprime, &imprimante) {
            for _ in 0..t.demande.repetition.max(1) {
                ajouter(m, machines::Operation::Impression, a.suspendre);
            }
        }
        // Chaque impression répétée a sa découpe (ses repères), comme la première.
        let fois = if imprime && imprimante.is_some() { t.demande.repetition.max(1) } else { 1 };
        if let (true, Some(m)) = (a.decouper && a_couper(t, p), &decoupeuse) {
            for mut s in (0..fois).flat_map(|_| sorties_de_coupe(t, i + 1, p, &m.id, m)) {
                if a.suspendre || imprime {
                    s.etat = file::Etat::Suspendue;
                }
                f.sorties.push(s);
            }
        }
    }
    let ajoutees = f.sorties.len() - avant;
    if let Some((rang, rangs)) = rangs {
        let nouvelles = f.sorties.split_off(avant);
        let place = place_dans_le_groupe(&f.sorties, rang, &rangs);
        f.sorties.splice(place..place, nouvelles);
    }
    if ajoutees > 0
        && let Err(e) = f.ecrire(&etat.config.donnees)
    {
        eprintln!("travail {} : file non enregistrée : {e}", t.id);
    }
    ajoutees
}

/// Où poser les sorties d'une partie de rang `rang` : juste après la dernière sortie en file
/// d'une partie de rang inférieur ou égal du même groupe, sinon juste avant la première d'une
/// partie de rang supérieur, sinon en fin de file.
fn place_dans_le_groupe(sorties: &[file::Sortie], rang: u32, rangs: &std::collections::HashMap<Uuid, u32>) -> usize {
    let en_file = |s: &file::Sortie| !s.archivee && matches!(s.etat, file::Etat::EnAttente | file::Etat::Suspendue);
    let rang_de = |s: &file::Sortie| rangs.get(&s.travail).copied().filter(|_| en_file(s));
    if let Some(i) = sorties.iter().rposition(|s| rang_de(s).is_some_and(|r| r <= rang)) {
        return i + 1;
    }
    sorties.iter().position(|s| rang_de(s).is_some_and(|r| r > rang)).unwrap_or(sorties.len())
}

/// Filtre couleur (roadmap 3.E2, 3.E3) sur les plaques CMJN de chaque page, production et
/// aperçu, avant le blanc automatique (la sous-couche suit l'encre filtrée).
async fn filtrer(rep: &std::path::Path, pages: &[Vec<String>], f: Option<filtre::Filtre>, dpi: u32) -> Result<(), String> {
    let Some(f) = f.filter(|f| !f.neutre()) else { return Ok(()) };
    for (i, plaques) in pages.iter().enumerate() {
        let n = i + 1;
        if let Some(absente) = filtre::ENCRES.iter().find(|e| !plaques.iter().any(|p| p == *e)) {
            return Err(format!("filtre couleur de la page {n} : plaque {absente} absente"));
        }
        let prod = filtre::ENCRES.map(|e| render::chemin_plaque(rep, n, e));
        let apercu = filtre::ENCRES.map(|e| render::chemin_apercu_plaque(rep, n, e));
        let (composite, avant, f) = (render::chemin_composite(rep, n), render::chemin_composite_avant(rep, n), f.clone());
        tokio::task::spawn_blocking(move || filtre::filtrer_tiff(&prod, &f, dpi).and_then(|()| filtre::filtrer_apercu(&apercu, &composite, &avant, &f)))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| format!("filtre couleur de la page {n} : {e}"))?;
    }
    Ok(())
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

/// Repérage du travail : le jeu de repères de sa découpeuse s'il en impose un (variante,
/// roadmap 4.15), sinon celui choisi.
fn reperage_voulu(catalogue: &[machines::Machine], d: &Demande) -> reperage::Reperage {
    d.decoupeuse.as_ref().and_then(|id| catalogue.iter().find(|m| &m.id == id)).and_then(|m| m.reperage).unwrap_or(d.reperage)
}

async fn preparer_impression(rep: &std::path::Path, choix: reperage::Reperage, fond: Option<reperage::FondReperes>, id: Uuid) -> Result<(PathBuf, Vec<reperage::Mise>), String> {
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
        tokio::task::spawn_blocking(move || reperage::envelopper(&pdf, choix, id, fond)).await.map_err(|e| e.to_string())??
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

/// Tracés de la page après repérage (`prod/decoupe-NNN.json`) : la source des fichiers de
/// coupe, refaits à chaque changement de couches sans refaire le rendu.
fn fichier_traces(rep: &std::path::Path, page: usize) -> PathBuf {
    rep.join("prod").join(format!("decoupe-{page:03}.json"))
}

/// Contours acceptés du détourage automatique (`prod/detourage-NNN.json`, roadmap 4.14) ;
/// effacés avec `prod/` par un nouveau rendu (l'image a pu changer).
fn fichier_detourage(rep: &std::path::Path, page: usize) -> PathBuf {
    rep.join("prod").join(format!("detourage-{page:03}.json"))
}

/// Tracés de la page : ceux du fichier, puis ceux du détourage accepté.
fn lire_traces(rep: &std::path::Path, page: usize) -> Option<Vec<decoupe::Trace>> {
    let detoures: Vec<decoupe::Trace> = std::fs::read(fichier_detourage(rep, page)).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_default();
    // Le `.plt` porte déjà le détourage : on ne le relit pas s'il y en a un.
    match (lire_traces_du_fichier(rep, page, detoures.is_empty()), detoures.is_empty()) {
        (Some(mut t), _) => {
            t.extend(detoures);
            Some(t)
        }
        (None, false) => Some(detoures),
        (None, true) => None,
    }
}

fn lire_traces_du_fichier(rep: &std::path::Path, page: usize, relire_plt: bool) -> Option<Vec<decoupe::Trace>> {
    if let Ok(o) = std::fs::read(fichier_traces(rep, page)) {
        return serde_json::from_slice(&o).ok();
    }
    if !relire_plt {
        return None;
    }
    // Travail rendu avant les couches (sans ce JSON) : son `.plt` est exactement ce que
    // l'ancienne version envoyait ; relu en tracés (une couche par outil), il redonne les
    // mêmes coordonnées. Plus sûr qu'extraire de nouveau la source : ni les préfixes
    // actuels, ni le repérage ou l'édition recalculés ne peuvent le changer. Le JSON est
    // écrit pour la fois suivante.
    let traces = decoupe::depuis_hpgl(&std::fs::read_to_string(fichier_plt(rep, page)).ok()?);
    if traces.is_empty() {
        return None;
    }
    if let Err(e) = serde_json::to_vec(&traces).map_err(|e| e.to_string()).and_then(|o| machines::ecrire_atomique(&fichier_traces(rep, page), &o)) {
        eprintln!("{} : tracés repris du .plt, non enregistrés : {e}", rep.display());
    }
    Some(traces)
}

/// Extrait les tracés à découper de chaque page ; pour une page qui en a, écrit
/// `prod/decoupe-NNN.json` (tracés). Avec un repérage, les tracés suivent le décalage de
/// leur page. Le `.plt` et l'aperçu suivent, une fois les couches connues.
async fn preparer_decoupe(rep: &std::path::Path, format: Format, tous: bool, mises: &[reperage::Mise]) -> Result<Vec<decoupe::Decoupe>, String> {
    let source = tokio::fs::read(rep.join(format.fichier())).await.map_err(|e| e.to_string())?;
    let mut pages = tokio::task::spawn_blocking(move || match format {
        // Une image est déjà devenue un PDF à la création du travail.
        Format::Pdf | Format::Tiff | Format::Png | Format::Jpeg => decoupe::extraire(&source, tous),
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
            tokio::fs::write(fichier_traces(rep, i + 1), serde_json::to_vec(&d.traces).map_err(|e| e.to_string())?).await.map_err(|e| e.to_string())?;
        }
    }
    Ok(pages)
}

/// Format de la page des tracés, en points : relevé au rendu ; pour un travail d'avant, le
/// format imprimé, sinon ce qui contient les tracés.
fn taille_coupe(p: &Page, traces: &[decoupe::Trace]) -> (f64, f64) {
    let mm = imbrication::PT_PAR_MM;
    match p.taille_coupe_pt {
        Some([l, h]) => (l, h),
        None if p.largeur_mm > 0.0 => (p.largeur_mm * mm, p.hauteur_mm * mm),
        None => coupe::boite(traces).map_or((0.0, 0.0), |b| (b[2].max(0.0), b[3].max(0.0))),
    }
}

/// Tracés de la page posés pour la machine par la mise en coupe du travail.
fn posee(t: &Travail, p: &Page, traces: &[decoupe::Trace]) -> coupe::Posee {
    t.mise_coupe.appliquer(traces, taille_coupe(p, traces), p.cadre_pt)
}

/// Étendre l'image près des tracés (roadmap 4.10) : chaque plaque imprimée des pages qui ont
/// des contours fermés, en production et en aperçu (le composite, lui, n'est pas refait).
/// `None` : rien à faire.
async fn etendre_plaques(rep: &std::path::Path, plaques: Option<&[Vec<String>]>, decoupes: &[decoupe::Decoupe], e: Option<coupe::Etendre>, dpi: u32) -> Option<Result<(), String>> {
    let (e, plaques) = (e?, plaques?);
    // Page, plaque, tracés, format de la page.
    type APlaque = (usize, String, Vec<decoupe::Trace>, (f64, f64));
    let travail: Vec<APlaque> = decoupes
        .iter()
        .enumerate()
        .filter(|(_, d)| d.traces.iter().any(|t| t.ferme))
        .flat_map(|(i, d)| plaques.get(i).into_iter().flatten().filter(|n| !decoupe::est_ton_de_decoupe(n)).map(move |n| (i + 1, n.clone(), d.traces.clone(), (d.largeur, d.hauteur))))
        .collect();
    let rep = rep.to_path_buf();
    Some(
        tokio::task::spawn_blocking(move || {
            for (page, nom, traces, format) in travail {
                let tif = render::chemin_plaque(&rep, page, &nom);
                let mut lecteur = blanc::LecteurTiff::ouvrir(&tif)?;
                let (l, h) = (lecteur.largeur, lecteur.hauteur);
                let tmp = tif.with_extension("tif.tmp");
                let mut ecrivain = blanc::EcrivainTiff::creer(&tmp, l as u32, h as u32, dpi).map_err(|e| e.to_string())?;
                let mut bande: Vec<u8> = Vec::with_capacity(blanc::RANGS_PAR_BANDE * l);
                let mut faites = 0;
                coupe::etendre(l, h, || lecteur.ligne(), |ligne| {
                    bande.extend_from_slice(ligne);
                    faites += 1;
                    if faites % blanc::RANGS_PAR_BANDE == 0 || faites == h {
                        ecrivain.bande(&bande).map_err(|e| e.to_string())?;
                        bande.clear();
                    }
                    Ok(())
                }, &traces, format, &e)?;
                ecrivain.finir().map_err(|e| e.to_string())?;
                std::fs::rename(&tmp, &tif).map_err(|e| e.to_string())?;
                let png = render::chemin_apercu_plaque(&rep, page, &nom);
                let img = image::open(&png).map_err(|e| format!("{} : {e}", png.display()))?.to_luma8();
                let (l, h) = (img.width() as usize, img.height() as usize);
                let mut lignes = img.as_raw().chunks(l.max(1)).map(<[u8]>::to_vec);
                let mut sortie = Vec::with_capacity(l * h);
                coupe::etendre(l, h, || lignes.next().ok_or_else(|| "aperçu tronqué".to_string()), |ligne| {
                    sortie.extend_from_slice(ligne);
                    Ok(())
                }, &traces, format, &e)?;
                image::GrayImage::from_raw(l as u32, h as u32, sortie).ok_or("aperçu étendu")?.save(&png).map_err(|e| e.to_string())?;
            }
            Ok(())
        })
        .await
        .unwrap_or_else(|e| Err(e.to_string())),
    )
}

/// `prod/decoupe-NNN.plt` (couches actives dans leur ordre) et `apercu/decoupe-NNN.svg` de
/// chaque page qui a des tracés, posés par la mise en coupe.
fn ecrire_coupes(rep: &std::path::Path, t: &Travail) -> std::io::Result<()> {
    for (i, p) in t.pages.iter().enumerate() {
        let Some(traces) = lire_traces(rep, i + 1) else { continue };
        let pose = posee(t, p, &traces);
        machines::ecrire_atomique(&fichier_plt(rep, i + 1), decoupe::hpgl(&pose.traces, &t.couches_decoupe, &decoupe::reglages()).as_bytes()).map_err(std::io::Error::other)?;
        std::fs::create_dir_all(rep.join("apercu"))?;
        let d = decoupe::Decoupe { traces: pose.traces, largeur: pose.taille.0, hauteur: pose.taille.1, signaux: vec![] };
        machines::ecrire_atomique(&fichier_svg(rep, i + 1), decoupe::apercu_svg(&d).as_bytes()).map_err(std::io::Error::other)?;
    }
    Ok(())
}

/// Contrôle préalable de la découpe (roadmap 4.5), sur le fichier tel que reçu (avant
/// édition, mosaïque, étiquettes : ce que le RIP ajoute n'est pas un défaut du fichier).
async fn alertes_decoupe(rep: &std::path::Path, t: &Travail, traces: usize) -> Vec<controle::Alerte> {
    let mut alertes = vec![];
    let original = [rep.join("source-original.pdf"), rep.join(Format::Pdf.fichier())].into_iter().find(|c| c.exists());
    let pdf = match (t.format, original) {
        (Format::Pdf, Some(c)) => tokio::fs::read(c).await.ok(),
        _ => None,
    };
    if let Some(pdf) = pdf.clone().filter(|_| !t.demande.tous_les_traces) {
        let r = decoupe::reglages();
        let pages = tokio::task::spawn_blocking(move || decoupe::extraire_avec(&pdf, false, &r)).await.ok().and_then(Result::ok).unwrap_or_default();
        for (i, d) in pages.into_iter().enumerate() {
            alertes.extend(d.signaux.into_iter().map(|(code, detail)| controle::Alerte { code: code.into(), page: Some(i + 1), detail }));
        }
    }
    // Une découpeuse est choisie, mais rien n'est reconnu : un préfixe manque ou est mal placé.
    if traces == 0 && !t.demande.tous_les_traces && (t.demande.decoupeuse.is_some() || t.demande.auto.decouper) {
        let tons = pdf.map(|p| decoupe::tons_du_pdf(&p)).unwrap_or_default();
        alertes.push(controle::Alerte { code: "decoupe_sans_prefixe".into(), page: None, detail: if tons.is_empty() { "—".into() } else { tons.join(", ") } });
    }
    // Repères de découpeuse imprimés sans rien à découper (alerte « pas de données de
    // découpe ») : de l'encre et du support pour rien.
    if traces == 0 && t.demande.mode == Mode::Imprimer && t.demande.reperage != reperage::Reperage::Aucun {
        alertes.push(controle::Alerte { code: "decoupe_reperes_sans_trace".into(), page: None, detail: String::new() });
    }
    alertes
}

/// Couches actives qui ont des tracés sur la page, dans l'ordre de coupe.
fn couches_a_couper(t: &Travail, p: &Page) -> Vec<String> {
    t.couches_decoupe.iter().filter(|c| c.active && p.traces_par_couche.get(&c.nom).is_some_and(|n| *n > 0)).map(|c| c.nom.clone()).collect()
}

/// Quelque chose à découper sur la page (travaux d'avant les couches : ses tracés).
fn a_couper(t: &Travail, p: &Page) -> bool {
    if t.couches_decoupe.is_empty() { p.traces_decoupe > 0 } else { !couches_a_couper(t, p).is_empty() }
}

/// Sorties de découpe d'une page : une seule, ou une par couche sur une découpeuse
/// mono-couche, la première prête et les suivantes retenues (changer d'outil ou de réglage
/// avant de les libérer).
fn sorties_de_coupe(t: &Travail, n: usize, p: &Page, cible: &str, m: &machines::Machine) -> Vec<file::Sortie> {
    // Découpeuse sans coupe à l'arrivée (roadmap 4.12) : tout arrive suspendu.
    let arrivee = if m.coupe_auto { file::Etat::EnAttente } else { file::Etat::Suspendue };
    let une = |couche: Option<String>| file::Sortie { couche, etat: arrivee, ..file::Sortie::nouvelle(cible, machines::Operation::Decoupe, t.id, &t.nom, n, maintenant()) };
    if !m.mono_couche || t.couches_decoupe.is_empty() {
        return vec![une(None)];
    }
    couches_a_couper(t, p)
        .into_iter()
        .enumerate()
        .map(|(i, c)| file::Sortie { etat: if i == 0 { arrivee } else { file::Etat::Suspendue }, ..une(Some(c)) })
        .collect()
}

/// Découpeuse prête pour ce travail : elle découpe, sa connexion est valable, et chaque
/// couche active trouve son réglage.
fn decoupeuse_utilisable(t: &Travail, m: &machines::Machine) -> bool {
    m.fait(machines::Operation::Decoupe) && m.joignable().is_ok() && t.couches_decoupe.iter().filter(|c| c.active).all(|c| m.reglage_couche(c).is_ok())
}

/// Découpeuse affectée à la page : celle du travail, sinon celle de sa dernière découpe en file.
fn decoupeuse_affectee(t: &Travail, n: usize, sorties: &[file::Sortie]) -> Option<String> {
    t.demande.decoupeuse.clone().or_else(|| sorties.iter().rev().find(|s| s.travail == t.id && s.page == n && s.active() && s.operation() == machines::Operation::Decoupe).map(|s| s.machine.clone()))
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
                let _ = if e.path().is_dir() { std::fs::remove_dir_all(e.path()) } else { std::fs::remove_file(e.path()) };
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
        Ok(()) => {
            // Planche supprimée avant d'être imprimée : ses sorties reviennent.
            reprendre_les_imbriquees(&etat);
            StatusCode::NO_CONTENT.into_response()
        }
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
    t.demande = Demande { mosaique: d.mosaique.clone().filter(|_| d.mode == Mode::Imprimer && t.format == Format::Pdf), ..d };
    reconstruire(&etat, t).await
}

/// Couches de découpe d'un travail rendu (roadmap 4.2 à 4.4) : ordre de coupe, couche
/// active ou non, réglage choisi. Enregistrées explicitement dans `job.json` (écriture sûre),
/// puis les fichiers de coupe sont refaits sans nouveau rendu. Les mêmes couches seulement :
/// aucune ne se crée ni ne se perd ici.
async fn enregistrer_couches(State(etat): State<Partage>, Chemin(id): Chemin<Uuid>, Json(couches): Json<Vec<decoupe::Couche>>) -> Response {
    let donnees = &etat.config.donnees;
    let mut t = match coupe_modifiable(&etat, id) {
        Ok(t) => t,
        Err(r) => return *r,
    };
    let memes = couches.len() == t.couches_decoupe.len() && t.couches_decoupe.iter().all(|c| couches.iter().filter(|x| x.nom == c.nom).count() == 1);
    if !memes || couches.iter().any(|c| c.reglage.as_ref().is_some_and(|r| r.trim().is_empty() || r.chars().count() > 40)) {
        return refus(StatusCode::BAD_REQUEST, "couches_invalides", None);
    }
    t.couches_decoupe = couches;
    // Les couches sont réglées à la main : l'ancien réglage du travail entier est tranché.
    t.demande.parametre_decoupe = None;
    enregistrer_coupe(donnees, t)
}

/// Travail dont la découpe peut changer : il existe, ne se rend pas et ne se coupe pas.
fn coupe_modifiable(etat: &Etat, id: Uuid) -> Result<Travail, Box<Response>> {
    let t = lire_travail(&etat.config.donnees, id).ok_or_else(|| Box::new(refus(StatusCode::NOT_FOUND, "introuvable", None)))?;
    let coupe_en_cours = etat.file.lock().unwrap().sorties.iter().any(|s| s.travail == id && s.etat == file::Etat::EnCours && s.operation() == machines::Operation::Decoupe);
    if actif(&t.statut) || coupe_en_cours {
        return Err(Box::new(refus(StatusCode::CONFLICT, "travail_occupe", None)));
    }
    Ok(t)
}

/// `job.json` écrit (écriture sûre), puis les fichiers de coupe refaits sans nouveau rendu.
fn enregistrer_coupe(donnees: &std::path::Path, t: Travail) -> Response {
    if let Err(e) = ecrire(donnees, &t).and_then(|()| ecrire_coupes(&dossier(donnees, t.id), &t)) {
        return refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string()));
    }
    Json(t).into_response()
}

#[derive(Deserialize)]
struct DemandeDetourage {
    #[serde(flatten)]
    reglage: detourage::Detourage,
    /// Faux : aperçu seulement ; vrai : les contours deviennent des tracés de la page.
    #[serde(default)]
    accepter: bool,
}

/// Compte des tracés de la page `n` (et des couches) refait après un détourage.
fn recompter(t: &mut Travail, rep: &std::path::Path, n: usize) {
    let traces = lire_traces(rep, n).unwrap_or_default();
    decoupe::accorder(&mut t.couches_decoupe, &decoupe::tons(&traces));
    let par_couche = t.couches_decoupe.iter().map(|c| (c.nom.clone(), traces.iter().filter(|x| decoupe::meme_couche(&x.ton, &c.nom)).count())).filter(|(_, k)| *k > 0).collect();
    let detoures = std::fs::read(fichier_detourage(rep, n)).ok().and_then(|o| serde_json::from_slice::<Vec<decoupe::Trace>>(&o).ok()).map_or(0, |v| v.len());
    if let Some(p) = t.pages.get_mut(n - 1) {
        (p.traces_decoupe, p.traces_par_couche, p.detourage) = (traces.len(), par_couche, detoures);
    }
}

/// Détourage automatique d'une page imprimée (roadmap 4.14) : la page du PDF rendu (tons de
/// découpe neutralisés) seuillée sur le blanc, à l'endroit où elle est posée (repérage).
/// Aperçu (`accepter` faux) : le nombre de contours et leur aperçu SVG ; accepté : ils
/// deviennent des tracés `CutContour` de la page, fichiers de coupe refaits.
async fn detourer_page(State(etat): State<Partage>, Chemin((id, n)): Chemin<(Uuid, usize)>, Json(d): Json<DemandeDetourage>) -> Response {
    if let Err(code) = d.reglage.verifier() {
        return refus(StatusCode::BAD_REQUEST, code, None);
    }
    let donnees = &etat.config.donnees;
    let Some((mut t, p)) = lire_page(donnees, id, n) else { return introuvable() };
    if t.format != Format::Pdf || t.demande.mode != Mode::Imprimer || !matches!(t.statut, Statut::Termine) {
        return refus(StatusCode::CONFLICT, "detourage_impossible", None);
    }
    if d.accepter && let Err(r) = coupe_modifiable(&etat, id) {
        return *r;
    }
    let rep = dossier(donnees, id);
    let (gs, delai) = (etat.config.gs.clone(), etat.config.delai);
    let interne = |e: String| refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e));
    let Ok(pdf) = tokio::fs::read(rep.join(Format::Pdf.fichier())).await else { return introuvable() };
    let prepare = tokio::task::spawn_blocking(move || -> Result<(Vec<u8>, imbrication::Taille), String> { Ok((decoupe::neutraliser(&pdf)?.0, imbrication::taille_page(&pdf, n)?)) }).await;
    let (neutre, taille) = match prepare {
        Ok(Ok(x)) => x,
        Ok(Err(e)) => return interne(e),
        Err(e) => return interne(e.to_string()),
    };
    // Posée comme au rendu : décalée par le repérage de la page.
    let origine = p.reperage.filter(|r| *r != reperage::Reperage::Aucun).map_or((0.0, 0.0), |r| reperage::mise_en_page(r, taille.largeur, taille.hauteur, 0).decalage);
    let dpi = detourage::dpi_pour(taille.largeur, taille.hauteur);
    let (source, image) = (rep.join("detourage.pdf"), rep.join("detourage.png"));
    let rendu = async {
        tokio::fs::write(&source, &neutre).await.map_err(|e| e.to_string())?;
        let pages = [format!("-dFirstPage={n}"), format!("-dLastPage={n}")];
        render::ghostscript(&gs, &source, "png16m", dpi.round() as u32, &pages, &image, delai).await?;
        let reglage = d.reglage;
        let image = image.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<decoupe::Trace>, String> {
            let img = image::open(&image).map_err(|e| e.to_string())?.to_rgb8();
            let (l, h) = (img.width() as usize, img.height() as usize);
            // Résolution vraie de l'image (Ghostscript arrondit le format en pixels).
            Ok(detourage::detourer(img.as_raw(), l, h, l as f64 * 72.0 / taille.largeur, &reglage, origine))
        })
        .await
        .map_err(|e| e.to_string())?
    }
    .await;
    let _ = tokio::fs::remove_file(&source).await;
    let _ = tokio::fs::remove_file(&image).await;
    let traces = match rendu {
        Ok(t) => t,
        Err(e) => return refus(StatusCode::INTERNAL_SERVER_ERROR, "rendu_echoue", Some(e)),
    };
    if !d.accepter {
        let base = lire_traces(&rep, n).unwrap_or_default();
        let (l, h) = taille_coupe(&p, &base);
        let (l, h) = if l > 0.0 { (l, h) } else { (taille.largeur, taille.hauteur) };
        let apercu = decoupe::apercu_svg(&decoupe::Decoupe { traces: traces.clone(), largeur: l, hauteur: h, signaux: vec![] });
        return Json(serde_json::json!({ "contours": traces.len(), "apercu_svg": apercu })).into_response();
    }
    // Travail d'avant les couches : ses tracés passent du `.plt` au JSON avant que le
    // détourage n'existe (ensuite, le `.plt` n'est plus relu).
    let _ = lire_traces(&rep, n);
    if let Err(e) = tokio::fs::create_dir_all(rep.join("prod")).await.map_err(|e| e.to_string()).and_then(|()| serde_json::to_vec(&traces).map_err(|e| e.to_string())).and_then(|o| machines::ecrire_atomique(&fichier_detourage(&rep, n), &o)) {
        return interne(e);
    }
    recompter(&mut t, &rep, n);
    enregistrer_coupe(donnees, t)
}

/// Détourage retiré (« Remove Path ») : la page retrouve ses seuls tracés du fichier.
async fn retirer_detourage(State(etat): State<Partage>, Chemin((id, n)): Chemin<(Uuid, usize)>) -> Response {
    let donnees = &etat.config.donnees;
    let mut t = match coupe_modifiable(&etat, id) {
        Ok(t) => t,
        Err(r) => return *r,
    };
    if t.pages.get(n.wrapping_sub(1)).is_none() {
        return introuvable();
    }
    let rep = dossier(donnees, id);
    let _ = std::fs::remove_file(fichier_detourage(&rep, n));
    // Page sans tracé du fichier : son `.plt` ne portait que le détourage, il ne doit pas être
    // relu comme tracés d'avant les couches.
    if !fichier_traces(&rep, n).exists() {
        let _ = std::fs::remove_file(fichier_plt(&rep, n));
        let _ = std::fs::remove_file(fichier_svg(&rep, n));
    }
    recompter(&mut t, &rep, n);
    enregistrer_coupe(donnees, t)
}

/// Mise en coupe d'un travail rendu (roadmap 4.8, 4.9) : rotation ; en découpe seule,
/// miroir, échelle, copies et départ à l'origine. Enregistrée explicitement, comme les couches.
async fn enregistrer_mise(State(etat): State<Partage>, Chemin(id): Chemin<Uuid>, Json(mise): Json<coupe::Mise>) -> Response {
    let mut t = match coupe_modifiable(&etat, id) {
        Ok(t) => t,
        Err(r) => return *r,
    };
    if let Err(code) = mise.verifier(t.demande.mode == Mode::Imprimer) {
        return refus(StatusCode::BAD_REQUEST, code, None);
    }
    t.mise_coupe = mise;
    enregistrer_coupe(&etat.config.donnees, t)
}

/// Réglages de découpe du serveur : préfixes des tons, calques non imprimables.
async fn lire_decoupe() -> Json<decoupe::Reglages> {
    Json(decoupe::reglages())
}

/// Nouveaux réglages de découpe, pris par les rendus suivants (un travail rendu se relance).
async fn enregistrer_decoupe(State(etat): State<Partage>, Json(r): Json<decoupe::Reglages>) -> Response {
    let r = decoupe::Reglages { prefixes: r.prefixes.iter().map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect(), ..r };
    if let Err(code) = r.verifier() {
        return refus(StatusCode::BAD_REQUEST, code, None);
    }
    if let Err(e) = decoupe::ecrire(&etat.config.donnees, &r) {
        return refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string()));
    }
    decoupe::installer(r.clone());
    Json(r).into_response()
}

/// Journal des tons rencontrés absents de la table (roadmap 8.6).
async fn lire_tons_inconnus(State(etat): State<Partage>) -> Json<std::collections::BTreeMap<String, tons::Rencontre>> {
    Json(tons::lire_inconnus(&etat.config.donnees))
}

async fn lire_tons(State(etat): State<Partage>) -> Json<tons::Table> {
    Json(tons::lire_table(&etat.config.donnees))
}

/// Table complète des tons directs et leur intention (roadmap 8.6, 8.7), prises par les
/// rendus suivants.
async fn enregistrer_tons(State(etat): State<Partage>, Json(table): Json<tons::Table>) -> Response {
    if let Err((code, detail)) = tons::verifier(&table) {
        return refus(StatusCode::BAD_REQUEST, code, Some(detail));
    }
    match tons::ecrire(&etat.config.donnees, &table) {
        Ok(()) => Json(table).into_response(),
        Err(e) => refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string())),
    }
}

/// Bibliothèque Lab d'un fournisseur (CSV `nom;L;a;b`) ajoutée à la table. Un nom déjà
/// présent refuse tout l'import : rien n'est écrit.
async fn importer_tons(State(etat): State<Partage>, corps: String) -> Response {
    let nouveaux = match tons::lire_csv(&corps) {
        Ok(t) if t.is_empty() => return refus(StatusCode::BAD_REQUEST, "csv_vide", None),
        Ok(t) => t,
        Err(e) => return refus(StatusCode::BAD_REQUEST, "csv_illisible", Some(e)),
    };
    let mut table = tons::lire_table(&etat.config.donnees);
    let ajoutes = nouveaux.len();
    table.tons.extend(nouveaux);
    if let Err((code, detail)) = tons::verifier(&table) {
        return refus(StatusCode::BAD_REQUEST, code, Some(detail));
    }
    match tons::ecrire(&etat.config.donnees, &table) {
        Ok(()) => Json(serde_json::json!({ "ajoutes": ajoutes, "total": table.tons.len() })).into_response(),
        Err(e) => refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string())),
    }
}

/// Source refaite depuis l'original (rotation automatique comprise), puis nouveau rendu.
async fn reconstruire(etat: &Partage, mut t: Travail) -> Response {
    let infos = infos_travail(etat, t.id, &t.nom, &t.demande, &t.fiche);
    let reperage = reperage_voulu(&etat.machines.read().unwrap(), &t.demande);
    match construire_source(&dossier(&etat.config.donnees, t.id), t.format, &Demande { reperage, ..t.demande.clone() }, laize_imprimante(etat), infos).await {
        Ok(alertes) => {
            t.alertes.retain(|a| !ALERTES_CONSTRUCTION.contains(&a.code.as_str()));
            t.alertes.extend(alertes);
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
    if let Err(code) = double_impression(&etat.file.lock().unwrap(), &t, None) {
        return refus(StatusCode::CONFLICT, code, None);
    }
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
    tokio::spawn(traiter(etat.clone(), t.clone(), etat.rendu.inscrire(false)));
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
fn raison_attente(donnees: &std::path::Path, s: &file::Sortie, m: Option<&machines::Machine>, charge: Option<&supports::Charge>, sorties: &[file::Sortie], catalogue: &[machines::Machine]) -> Option<&'static str> {
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
    if t.pages_exclues.contains(&s.page) {
        return Some("page_exclue");
    }
    if s.operation() == machines::Operation::Decoupe {
        let rep = dossier(donnees, s.travail);
        if !fichier_traces(&rep, s.page).exists() && lire_traces(&rep, s.page).is_none() {
            return Some("source_absente");
        }
        // Coupe seulement sur une impression finie à 100 % (roadmap 4.6) : la dernière
        // impression de la page doit être terminée (ou annulée : imprimée ailleurs).
        let impression = sorties.iter().rev().find(|x| x.travail == s.travail && x.page == s.page && x.operation() == machines::Operation::Impression);
        if impression.is_some_and(|x| !matches!(x.etat, file::Etat::Terminee | file::Etat::Annulee)) {
            return Some("attente_impression");
        }
        if !decoupeuse_utilisable(&t, m) {
            return Some("decoupeuse_indisponible");
        }
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
    // Pré-contrôle de la découpe : pas d'encre sur un travail que
    // sa découpeuse ne pourra pas couper (absente, hors ligne, connexion ou réglage manquant).
    if impression
        && a_couper(&t, &p)
        && let Some(id) = decoupeuse_affectee(&t, s.page, sorties)
    {
        let candidates: Vec<&machines::Machine> = match catalogue.iter().find(|x| x.id == id) {
            Some(x) => vec![x],
            None => machines::membres(catalogue, &id),
        };
        if !candidates.iter().any(|x| decoupeuse_utilisable(&t, x)) {
            return Some("decoupeuse_indisponible");
        }
    }
    // Page (repères compris) plus large que la laize utile, ou que le rouleau chargé.
    let laize = charge.filter(|_| impression).and_then(|c| c.laize_mm).map_or(m.laize_mm, |l| l.min(m.laize_mm));
    if p.largeur_mm > laize + 0.5 {
        return Some("format_trop_petit");
    }
    // Imbrication (roadmap 5.1) : l'impression attend sa planche, sauf la planche elle-même.
    if impression && m.imbrication.strategie != nid::Strategie::UnParUn && t.imbrication.is_none() {
        return Some("attente_imbrication");
    }
    None
}

/// Raison d'attente d'une sortie sur sa machine, ou sur son pool : en attente seulement si
/// aucun membre ne peut la prendre.
fn raison_de(donnees: &std::path::Path, s: &file::Sortie, tout: &[file::Sortie], catalogue: &[machines::Machine], charges: &std::collections::BTreeMap<String, supports::Charge>) -> Option<&'static str> {
    match catalogue.iter().find(|m| m.id == s.machine) {
        Some(m) => raison_attente(donnees, s, Some(m), charges.get(&m.id), tout, catalogue),
        None => {
            let raisons: Vec<_> = machines::membres(catalogue, &s.machine).iter().map(|m| raison_attente(donnees, s, Some(m), charges.get(&m.id), tout, catalogue)).collect();
            if raisons.is_empty() { raison_attente(donnees, s, None, None, tout, catalogue) } else if raisons.contains(&None) { None } else { raisons[0] }
        }
    }
}

/// Poids dans le tampon d'une sortie (roadmap 9.6) : les plaques que l'impression enverra,
/// ou le fichier de tracés d'une découpe. Une sortie posée sur une planche ne pèse rien : la
/// planche, une sortie à part, porte ce poids.
fn poids_sortie(donnees: &std::path::Path, s: &file::Sortie) -> u64 {
    let taille = |c: PathBuf| std::fs::metadata(c).map_or(0, |m| m.len());
    let rep = dossier(donnees, s.travail);
    if s.etat == file::Etat::Imbriquee {
        return 0;
    }
    if s.operation() == machines::Operation::Decoupe {
        return taille(fichier_traces(&rep, s.page));
    }
    lire_page(donnees, s.travail, s.page).map_or(0, |(_, p)| p.plaques.iter().filter(|n| !p.plaques_decoupe.contains(n)).map(|n| taille(render::chemin_plaque(&rep, s.page, n))).sum())
}

/// Tampon de la file : poids des sorties encore à passer.
fn poids_tampon(donnees: &std::path::Path, sorties: &[file::Sortie]) -> u64 {
    sorties.iter().filter(|s| s.active()).map(|s| poids_sortie(donnees, s)).sum()
}

async fn lister_file(State(etat): State<Partage>) -> Json<Vec<serde_json::Value>> {
    let sorties = etat.file.lock().unwrap().sorties.clone();
    let tout = sorties.clone();
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
                v["poids"] = serde_json::json!(s.active().then(|| poids_sortie(&donnees, &s)));
                // Fiche travail (roadmap 5.6) visible dans la file ; planche d'une imbriquée.
                if let Some(t) = lire_travail(&donnees, s.travail) {
                    v["fiche"] = serde_json::to_value(&t.fiche).unwrap();
                }
                v["planche_nom"] = serde_json::json!(s.planche.and_then(|p| lire_travail(&donnees, p)).map(|p| p.nom));
                v["raison_attente"] = serde_json::json!(raison_de(&donnees, &s, &tout, &catalogue, &charges));
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
    /// Épreuve sur une autre machine (roadmap 9.5) : une copie « -BAT » y part, l'original
    /// attend, suspendu, d'être repris.
    #[serde(default)]
    epreuve: Option<String>,
}

/// L'Epson ne prend que ce que sa connexion sait sortir : le pilote direct n'a pas encore
/// le blanc ni l'argent ; le dossier, si.
fn controler_impression(m: &machines::Machine, t: &Travail, p: &Page) -> Result<(), String> {
    if m.modele != machines::Modele::EpsonScS80600 {
        return Ok(());
    }
    let controle = if m.connexion.flux_direct() { epson::verifier_pour_le_pilote } else { |p: &[String], c, pile: &[epson::Couche]| epson::preparer(p, c, pile).map(|_| ()) };
    controle(&p.plaques, t.demande.couches, &t.demande.empilement)
}

/// Met une page en file : découpe pour le Kona, plaques pour l'Epson.
async fn mettre_en_file(State(etat): State<Partage>, Json(d): Json<DemandeSortie>) -> Response {
    let Some((t, p)) = lire_page(&etat.config.donnees, d.travail, d.page) else { return refus(StatusCode::NOT_FOUND, "introuvable", None) };
    if !matches!(t.statut, Statut::Termine) {
        return refus(StatusCode::CONFLICT, "travail_occupe", None);
    }
    if t.pages_exclues.contains(&d.page) {
        return refus(StatusCode::CONFLICT, "page_exclue", None);
    }
    // Pool : contrôles faits sur un membre ; les supports chargés connus doivent concorder.
    let catalogue = etat.machines.read().unwrap().clone();
    let membres = machines::membres(&catalogue, &d.machine);
    let m = match machine(&etat, &d.machine) {
        Some(m) => m,
        None if !membres.is_empty() => {
            let charges = supports::lire_charges(&etat.config.donnees);
            let mut connus: Vec<_> = membres.iter().filter_map(|x| charges.get(&x.id).and_then(|c| c.support_uuid)).collect();
            connus.dedup();
            if connus.len() > 1 {
                return refus(StatusCode::CONFLICT, "support_different", None);
            }
            membres[0].clone()
        }
        None => return refus(StatusCode::NOT_FOUND, "introuvable", None),
    };
    use machines::Operation::{Decoupe, Impression};
    let operation = match d.operation {
        Some(o) => o,
        None if m.fait(Impression) && m.fait(Decoupe) => return refus(StatusCode::BAD_REQUEST, "operation_impossible", Some("imprimer ou découper ?".into())),
        None if m.fait(Impression) => Impression,
        None => Decoupe,
    };
    if operation == Impression && t.demande.epreuve_seule {
        return refus(StatusCode::CONFLICT, "epreuve_seule", None);
    }
    if !m.fait(operation) {
        return refus(StatusCode::CONFLICT, "operation_impossible", None);
    }
    match operation {
        Decoupe if !a_couper(&t, &p) => return refus(StatusCode::CONFLICT, "rien_a_decouper", None),
        Impression if p.plaques.len() == p.plaques_decoupe.len() => return refus(StatusCode::CONFLICT, "rien_a_imprimer", None),
        Impression => {
            if let Err(e) = controler_impression(&m, &t, &p) {
                return refus(StatusCode::CONFLICT, "envoi_impossible", Some(e));
            }
        }
        _ => {}
    }
    let cible = if membres.is_empty() { m.id.clone() } else { d.machine.clone() };
    // Épreuve : une autre machine qui imprime, hors du pool de la cible.
    let epreuve = match d.epreuve.as_deref().filter(|e| !e.is_empty()) {
        None => None,
        Some(_) if operation != Impression => return refus(StatusCode::BAD_REQUEST, "epreuve_impossible", Some("impression seulement".into())),
        Some(e) => {
            let Some(me) = machine(&etat, e) else { return refus(StatusCode::NOT_FOUND, "introuvable", Some(e.into())) };
            if me.id == m.id || membres.iter().any(|x| x.id == me.id) || !me.fait(Impression) {
                return refus(StatusCode::BAD_REQUEST, "epreuve_impossible", Some(me.nom));
            }
            if let Err(e) = controler_impression(&me, &t, &p) {
                return refus(StatusCode::CONFLICT, "envoi_impossible", Some(format!("{} : {e}", me.nom)));
            }
            Some(me.id)
        }
    };
    let mut nouvelles = match operation {
        Decoupe => sorties_de_coupe(&t, d.page, &p, &cible, &m),
        Impression => vec![file::Sortie::nouvelle(&cible, operation, t.id, &t.nom, d.page, maintenant())],
    };
    if let Some(e) = epreuve {
        nouvelles[0].etat = file::Etat::Suspendue;
        let bat = file::Sortie { bat_de: Some(nouvelles[0].id), ..file::Sortie::nouvelle(&e, Impression, t.id, &format!("{}-BAT", t.nom), d.page, maintenant()) };
        nouvelles.push(bat);
    }
    let rendu = nouvelles[0].clone();
    match modifier_file(&etat, |f| {
        double_impression(f, &t, None)?;
        f.sorties.extend(nouvelles);
        Ok(())
    }) {
        Ok(()) => (StatusCode::CREATED, Json(rendu)).into_response(),
        Err(r) => *r,
    }
}

async fn agir_sur_la_file(State(etat): State<Partage>, Chemin((id, action)): Chemin<(Uuid, String)>) -> Response {
    let Ok(action) = serde_json::from_value::<file::Action>(serde_json::Value::String(action)) else { return refus(StatusCode::NOT_FOUND, "introuvable", None) };
    let catalogue = etat.machines.read().unwrap().clone();
    // Travail de la sortie, lu hors du verrou de la file (pour le garde-fou de « Refaire »).
    let travail = (action == file::Action::Refaire).then(|| etat.file.lock().unwrap().sorties.iter().find(|s| s.id == id).map(|s| s.travail)).flatten().and_then(|t| lire_travail(&etat.config.donnees, t));
    match modifier_file(&etat, |f| {
        if let Some(t) = &travail {
            double_impression(f, t, f.sorties.iter().find(|s| s.id == id))?;
        }
        f.agir(id, action, maintenant())?;
        // Restaurée sur une découpeuse sans coupe à l'arrivée : elle attend l'opérateur.
        if action == file::Action::Restaurer
            && let Some(s) = f.sorties.iter_mut().find(|s| s.id == id)
            && catalogue.iter().any(|m| m.id == s.machine && !m.coupe_auto)
        {
            s.etat = file::Etat::Suspendue;
        }
        Ok(())
    }) {
        Ok(()) => {
            reprendre_les_imbriquees(&etat);
            StatusCode::NO_CONTENT.into_response()
        }
        Err(r) => *r,
    }
}

async fn retirer_de_la_file(State(etat): State<Partage>, Chemin(id): Chemin<Uuid>) -> Response {
    match modifier_file(&etat, |f| f.retirer(id)) {
        Ok(()) => {
            reprendre_les_imbriquees(&etat);
            StatusCode::NO_CONTENT.into_response()
        }
        Err(r) => *r,
    }
}

// --- Imbrication dans la file (roadmap 5.1 à 5.5) ---

/// Ce qui doit être commun aux travaux d'une même planche (roadmap 5.3) : support, mode,
/// découpeuse (« aucune » comprise), miroir ; et ce qui décide du rendu de la planche
/// (résolution, repérage, couleur, blanc, tracés), qu'elle n'a qu'une fois. Rend la clé et
/// la découpeuse.
fn cle_imbrication(t: &Travail, p: &Page, n: usize, sorties: &[file::Sortie]) -> (String, Option<String>) {
    let decoupeuse = if a_couper(t, p) { decoupeuse_affectee(t, n, sorties) } else { None };
    let d = &t.demande;
    // Réglage de coupe choisi à la main pour une couche active : une couche n'a qu'un réglage
    // sur la planche, deux travaux qui en choisissent deux différents ne s'imbriquent pas.
    let mut reglages: Vec<(String, &str)> = t.couches_decoupe.iter().filter(|c| c.active).filter_map(|c| c.reglage.as_deref().map(|r| (decoupe::cle(&c.nom), r))).collect();
    reglages.sort();
    let cle = serde_json::json!([d.support_uuid, d.mode_uuid, decoupeuse, d.edition.miroir, d.dpi, d.reperage, d.couleur, d.blanc, d.couches, d.empilement, d.fond_reperes, d.tous_les_traces, d.etendre_coupe, reglages]).to_string();
    (cle, decoupeuse)
}

/// Une planche en préparation : les impressions compatibles d'une machine et leur plan.
struct Groupe {
    /// Code court de la clé, pour l'écran et l'API.
    code: String,
    decoupeuse: Option<String>,
    /// Sorties en attente, dans l'ordre de la file, avec leur travail (même rang que
    /// `candidats`).
    sorties: Vec<(file::Sortie, Travail)>,
    candidats: Vec<nid::Candidat>,
    plan: Option<nid::Plan>,
    /// Laize du rouleau (machine, ou rouleau chargé plus étroit).
    laize_mm: f64,
}

fn code_groupe(cle: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    cle.hash(&mut h);
    format!("{:016x}", h.finish())
}

/// Planches en préparation sur `m` (aucune en « un par un ») : ses impressions en attente,
/// pages PDF non exclues, regroupées par clé, chacune planifiée sur la laize moins ce que
/// le repérage de la planche ajoute en largeur.
fn groupes(etat: &Etat, catalogue: &[machines::Machine], m: &machines::Machine) -> Vec<Groupe> {
    if m.imbrication.strategie == nid::Strategie::UnParUn {
        return vec![];
    }
    let sorties = etat.file.lock().unwrap().sorties.clone();
    let donnees = &etat.config.donnees;
    let laize = supports::lire_charges(donnees).get(&m.id).and_then(|c| c.laize_mm).map_or(m.laize_mm, |l| l.min(m.laize_mm));
    let formats = supports::lire(donnees, "epson");
    let mut groupes: Vec<(String, Groupe)> = vec![];
    // Envoyées à la machine ou à son pool (roadmap 1.7) : la première planche lancée les prend.
    let vise = |s: &file::Sortie| s.machine == m.id || m.pool.as_deref() == Some(s.machine.as_str());
    for s in sorties.iter().filter(|s| vise(s) && s.etat == file::Etat::EnAttente && s.operation() == machines::Operation::Impression) {
        let Some((t, p)) = lire_page(donnees, s.travail, s.page) else { continue };
        if t.imbrication.is_some() || t.format != Format::Pdf || t.pages_exclues.contains(&s.page) {
            continue;
        }
        let Some(taille) = std::fs::read(dossier(donnees, t.id).join(Format::Pdf.fichier())).ok().and_then(|pdf| imbrication::taille_page(&pdf, s.page).ok()) else { continue };
        let (cle, decoupeuse) = cle_imbrication(&t, &p, s.page, &sorties);
        // Chargement strict (roadmap 5.11) : une planche par format, posée sur ce format.
        let format = (m.imbrication.chargement == nid::Chargement::Strict).then(|| format_travail(&t, &formats)).flatten();
        let (cle, laize) = match format {
            Some(f) => (format!("{cle}|{f}"), f.min(laize)),
            None => (cle, laize),
        };
        let candidat = nid::Candidat { taille, copies: t.fiche.copies(), le: p.tuile.as_ref().map(|x| x.numero), echeance: t.fiche.echeance, arrivee: s.cree_le };
        let i = groupes.iter().position(|(c, _)| *c == cle).unwrap_or_else(|| {
            groupes.push((cle.clone(), Groupe { code: code_groupe(&cle), decoupeuse, sorties: vec![], candidats: vec![], plan: None, laize_mm: laize }));
            groupes.len() - 1
        });
        groupes[i].1.sorties.push((s.clone(), t));
        groupes[i].1.candidats.push(candidat);
    }
    let mm = imbrication::PT_PAR_MM;
    groupes
        .into_iter()
        .map(|(_, mut g)| {
            let reperage = reperage_voulu(catalogue, &demande_planche(&g));
            let ajout = (reperage::mise_en_page(reperage, g.laize_mm * mm, 1000.0 * mm, 0).page.0 / mm - g.laize_mm).max(0.0);
            g.plan = nid::planifier(&g.candidats, g.laize_mm - ajout, &m.imbrication);
            g
        })
        .collect()
}

/// Format d'un travail (roadmap 5.11) : laize de son support au catalogue, sinon celle pour
/// laquelle sa mosaïque est faite.
fn format_travail(t: &Travail, catalogue: &supports::Catalogue) -> Option<f64> {
    let support = t.demande.support_uuid.and_then(|u| catalogue.supports.iter().find(|s| s.id == Some(u)));
    support.and_then(|s| s.laize_mm).or(t.demande.mosaique.as_ref().map(|m| m.laize_mm))
}

/// Réglages de la planche : ceux de rendu du premier travail (communs au groupe), sans ce
/// qui est déjà dans chaque page posée (édition, mosaïque, étiquette) ; imprimée d'office,
/// découpée sur la découpeuse du groupe s'il en a une.
fn demande_planche(g: &Groupe) -> Demande {
    let d = &g.sorties[0].1.demande;
    Demande {
        mode: Mode::Imprimer,
        tous_les_traces: d.tous_les_traces,
        reperage: d.reperage,
        couleur: d.couleur.clone(),
        dpi: d.dpi,
        blanc: d.blanc,
        couches: d.couches,
        empilement: d.empilement.clone(),
        support: d.support.clone(),
        support_uuid: d.support_uuid,
        mode_uuid: d.mode_uuid,
        decoupeuse: g.decoupeuse.clone(),
        rotation_auto: false,
        etendre_coupe: d.etendre_coupe,
        fond_reperes: d.fond_reperes,
        reechantillonnage: d.reechantillonnage,
        auto: reglages::Auto { imprimer: true, decouper: g.decoupeuse.is_some(), suspendre: false },
        ..Default::default()
    }
}

/// Fiche de la planche : les commandes posées, l'échéance la plus proche, les travaux.
fn fiche_planche(g: &Groupe, plan: &nid::Plan) -> Fiche {
    let travaux: Vec<&Travail> = plan.retenus.iter().map(|&i| &g.sorties[i].1).collect();
    let mut commandes: Vec<&str> = travaux.iter().map(|t| t.fiche.commande.as_str()).filter(|c| !c.is_empty()).collect();
    commandes.dedup();
    let notes: Vec<String> = plan.retenus.iter().map(|&i| format!("{} p{} × {}", g.sorties[i].1.nom, g.sorties[i].0.page, g.candidats[i].copies)).collect();
    Fiche {
        commande: commandes.join(", ").chars().take(200).collect(),
        echeance: travaux.iter().filter_map(|t| t.fiche.echeance).min(),
        notes: notes.join("\n").chars().take(2000).collect(),
        operateur: travaux.iter().map(|t| t.fiche.operateur.as_str()).find(|o| !o.is_empty()).unwrap_or_default().into(),
        ..Default::default()
    }
}

/// Une page posée sur la planche : son travail, sa page, ses couches désactivées.
type SourcePlanche = (Uuid, usize, Vec<String>);

/// PDF de la planche (aperçu = PDF produit) : `sources` des candidats retenus, dans l'ordre
/// de `plan.retenus` ; les couches désactivées d'un travail y restent à part (`exclure_tons`).
fn pdf_planche(donnees: &std::path::Path, sources: &[SourcePlanche], plan: &nid::Plan, finition: imbrication::Finition) -> Result<Vec<u8>, String> {
    let pdfs = sources
        .iter()
        .map(|(id, _, exclus)| {
            let pdf = std::fs::read(dossier(donnees, *id).join(Format::Pdf.fichier())).map_err(|e| e.to_string())?;
            if exclus.is_empty() { Ok(pdf) } else { imbrication::exclure_tons(&pdf, exclus) }
        })
        .collect::<Result<Vec<_>, String>>()?;
    let sources: Vec<imbrication::Source> = sources.iter().zip(&pdfs).map(|((_, page, _), pdf)| imbrication::Source { pdf, page: *page, copies: 1 }).collect();
    // Copie k : candidat `plan.elements[k]`, source = son rang parmi les retenus.
    let elements: Vec<usize> = plan.elements.iter().map(|c| plan.retenus.iter().position(|r| r == c).unwrap_or(0)).collect();
    imbrication::composer_places(&sources, &plan.places, &elements, plan.planche, imbrication::Finition { purges: plan.purges, ..finition }, &[])
}

fn sources_planche(g: &Groupe, plan: &nid::Plan) -> Vec<SourcePlanche> {
    plan.retenus
        .iter()
        .map(|&i| {
            let t = &g.sorties[i].1;
            (t.id, g.sorties[i].0.page, t.couches_decoupe.iter().filter(|c| !c.active).map(|c| c.nom.clone()).collect())
        })
        .collect()
}

/// Couches de la planche reprises des travaux posés : réglage choisi d'une couche active
/// (le même pour tous, par le regroupement), et chaque couche désactivée d'un travail en
/// couche `<ton>_exclu` désactivée. Les autres prennent leur rang au rendu.
fn couches_planche(g: &Groupe, plan: &nid::Plan) -> Vec<decoupe::Couche> {
    let travaux: Vec<&Travail> = plan.retenus.iter().map(|&i| &g.sorties[i].1).collect();
    let nom = |c: &decoupe::Couche| if c.active { c.nom.clone() } else { format!("{}{}", c.nom, imbrication::SUFFIXE_EXCLU) };
    let noms: Vec<String> = travaux.iter().flat_map(|t| t.couches_decoupe.iter().map(nom)).collect();
    let mut couches = vec![];
    decoupe::accorder(&mut couches, &noms.iter().map(String::as_str).collect::<Vec<_>>());
    for c in couches.iter_mut() {
        let de = || travaux.iter().flat_map(|t| t.couches_decoupe.iter()).filter(|x| decoupe::meme_couche(&nom(x), &c.nom));
        c.active = de().any(|x| x.active);
        c.reglage = de().find_map(|x| x.reglage.clone().filter(|_| x.active));
    }
    couches
}

/// Lance la planche du groupe : ses impressions (et les découpes de ces pages) passent
/// « imbriquées » sous le verrou de la file, puis la planche est composée et devient un
/// travail, rendu et mis en file sur la machine. En cas d'échec, les sorties reviennent.
async fn lancer_imbrication(etat: &Partage, m: &machines::Machine, g: Groupe) -> Result<Travail, (StatusCode, Refus)> {
    let Some(plan) = g.plan.as_ref() else { return Err((StatusCode::CONFLICT, ("rien_a_imbriquer", None))) };
    let id = Uuid::new_v4();
    let sources = sources_planche(&g, plan);
    let pages: Vec<(Uuid, usize)> = sources.iter().map(|(t, p, _)| (*t, *p)).collect();
    let ids: Vec<Uuid> = plan.retenus.iter().map(|&i| g.sorties[i].0.id).collect();
    let donnees = etat.config.donnees.clone();
    let prises: Vec<(Uuid, file::Etat)> = {
        let mut f = etat.file.lock().unwrap();
        // Toutes encore en attente, sinon rien : la file a changé depuis le calcul.
        if !ids.iter().all(|i| f.sorties.iter().any(|s| s.id == *i && s.etat == file::Etat::EnAttente)) {
            return Err((StatusCode::CONFLICT, ("imbrication_changee", None)));
        }
        let mut prises = vec![];
        let decoupe_de_la_page = |s: &file::Sortie| s.operation() == machines::Operation::Decoupe && matches!(s.etat, file::Etat::EnAttente | file::Etat::Suspendue) && pages.contains(&(s.travail, s.page));
        for s in f.sorties.iter_mut().filter(|s| ids.contains(&s.id) || decoupe_de_la_page(s)) {
            prises.push((s.id, s.etat));
            (s.avant_imbrication, s.etat, s.planche) = (Some(s.etat), file::Etat::Imbriquee, Some(id));
        }
        let _ = f.ecrire(&donnees);
        prises
    };
    let rendre = || {
        let mut f = etat.file.lock().unwrap();
        for (i, e) in &prises {
            if let Some(s) = f.sorties.iter_mut().find(|s| s.id == *i) {
                (s.etat, s.planche, s.avant_imbrication) = (*e, None, None);
            }
        }
        let _ = f.ecrire(&donnees);
    };
    let (finition, plan_c, sources_c, donnees_c) = (m.imbrication.finition(), plan.clone(), sources.clone(), donnees.clone());
    let pdf = match tokio::task::spawn_blocking(move || pdf_planche(&donnees_c, &sources_c, &plan_c, finition)).await {
        Ok(Ok(p)) => p,
        Ok(Err(e)) => {
            rendre();
            return Err((StatusCode::INTERNAL_SERVER_ERROR, ("erreur_interne", Some(e))));
        }
        Err(e) => {
            rendre();
            return Err((StatusCode::INTERNAL_SERVER_ERROR, ("erreur_interne", Some(e.to_string()))));
        }
    };
    let rep = dossier(&donnees, id);
    if let Err(e) = tokio::fs::create_dir_all(&rep).await.and(tokio::fs::write(rep.join("source.tmp"), pdf).await) {
        let _ = tokio::fs::remove_dir_all(&rep).await;
        rendre();
        return Err((StatusCode::INTERNAL_SERVER_ERROR, ("erreur_interne", Some(e.to_string()))));
    }
    let nom = format!("Imbrication {} — {} page(s)", m.nom, plan.retenus.len());
    let imbriquee = Imbriquee { machine: m.id.clone(), sorties: prises.iter().map(|(i, _)| *i).collect() };
    match creer_travail_avec(etat, id, nom, Format::Pdf, demande_planche(&g), fiche_planche(&g, plan), Origine::Planche(imbriquee, couches_planche(&g, plan)), vec![]).await {
        Ok(t) => Ok(t),
        Err(r) => {
            let _ = tokio::fs::remove_dir_all(&rep).await;
            rendre();
            Err(r)
        }
    }
}

/// Garde-fou contre la double impression (roadmap 5.13), sous le verrou de la file : une
/// planche abandonnée dont les travaux sont revenus en file (plus aucune sortie ne la
/// désigne) ne se refait, ne se relance ni ne se remet en file : ses travaux s'imprimeraient
/// deux fois, sur elle et sur leur nouvelle planche (ou seuls). Refus plutôt que reprise des
/// travaux : depuis leur retour ils ont pu être modifiés, exclus, retirés ou déjà reposés sur
/// une autre planche ; « Imprimer maintenant » refait une planche juste. De même, une sortie
/// « Imbriquée » ne se refait pas tant que sa planche n'est pas imprimée : la planche
/// l'imprime déjà (`sortie`, la sortie à refaire).
fn double_impression(f: &file::File, t: &Travail, sortie: Option<&file::Sortie>) -> Result<(), &'static str> {
    if t.imbrication.is_some() && !f.sorties.iter().any(|s| s.planche == Some(t.id)) {
        return Err("planche_rendue_a_la_file");
    }
    if sortie.is_some_and(|s| s.etat == file::Etat::Imbriquee) {
        return Err("sortie_sur_une_planche");
    }
    Ok(())
}

/// Planches abandonnées : rendu en échec, travail supprimé, ou impression annulée, en échec
/// ou retirée de la file (aucune en attente, en cours ni terminée). Les sorties qu'elles
/// remplaçaient repassent dans leur état d'avant, donc dans leur file et leur imbrication.
/// Rien ne repasse d'une planche dont une page est imprimée (sorties passées `Terminee`).
fn reprendre_les_imbriquees(etat: &Etat) {
    let donnees = &etat.config.donnees;
    let mut f = etat.file.lock().unwrap();
    let mut planches: Vec<Uuid> = f.sorties.iter().filter(|s| s.etat == file::Etat::Imbriquee).filter_map(|s| s.planche).collect();
    planches.sort();
    planches.dedup();
    let mut change = false;
    for p in planches {
        if f.sorties.iter().any(|s| s.planche == Some(p) && s.etat == file::Etat::Terminee) {
            continue;
        }
        let abandonnee = match lire_travail(donnees, p) {
            None => true,
            Some(t) => match t.statut {
                Statut::Echec => true,
                Statut::Termine => !f.sorties.iter().any(|s| s.travail == p && s.operation() == machines::Operation::Impression && (s.active() || s.etat == file::Etat::Terminee)),
                _ => false,
            },
        };
        if abandonnee {
            for s in f.sorties.iter_mut().filter(|s| s.planche == Some(p) && s.etat == file::Etat::Imbriquee) {
                (s.etat, s.planche) = (s.avant_imbrication.take().unwrap_or(file::Etat::EnAttente), None);
            }
            change = true;
        }
    }
    if change && let Err(e) = f.ecrire(donnees) {
        eprintln!("file non enregistrée : {e}");
    }
}

/// Planches dont le déclencheur est atteint (roadmap 5.2) : lancées.
async fn imbriquer_les_pretes(etat: &Partage) {
    reprendre_les_imbriquees(etat);
    let catalogue = etat.machines.read().unwrap().clone();
    for m in catalogue.iter().filter(|m| m.variante_de.is_none()) {
        for g in groupes(etat, &catalogue, m) {
            let pret = g.plan.as_ref().is_some_and(|p| nid::pret(p, &g.candidats, &m.imbrication, maintenant()));
            if pret && let Err((_, (code, detail))) = lancer_imbrication(etat, m, g).await {
                eprintln!("imbrication sur {} : {code} {}", m.id, detail.unwrap_or_default());
            }
        }
    }
}

/// Planches en préparation (roadmap 5.4) : par machine, les travaux, la longueur (marges
/// comprises), le remplissage du rouleau consommé, les places (mm, origine en bas à gauche)
/// et si le déclencheur est atteint.
async fn lister_imbrications(State(etat): State<Partage>) -> Json<Vec<serde_json::Value>> {
    let catalogue = etat.machines.read().unwrap().clone();
    let mm = imbrication::PT_PAR_MM;
    let mut liste = vec![];
    for m in catalogue.iter().filter(|m| m.variante_de.is_none()) {
        for g in groupes(&etat, &catalogue, m) {
            let retenu = |i: usize| g.plan.as_ref().is_some_and(|p| p.retenus.contains(&i));
            let elements: Vec<_> = g
                .sorties
                .iter()
                .zip(&g.candidats)
                .enumerate()
                .map(|(i, ((s, t), c))| serde_json::json!({ "sortie": s.id, "travail": t.id, "page": s.page, "nom": t.nom, "copies": c.copies, "echeance": c.echeance, "commande": t.fiche.commande, "client": t.fiche.client, "retenu": retenu(i) }))
                .collect();
            let f = m.imbrication.finition();
            let (longueur, remplissage, places, pret) = match &g.plan {
                Some(p) => {
                    let places: Vec<_> = p
                        .places
                        .iter()
                        .map(|pl| {
                            let i = p.elements[pl.indice];
                            let t = g.candidats[i].taille;
                            let (l, h) = if pl.tourne() { (t.hauteur, t.largeur) } else { (t.largeur, t.hauteur) };
                            serde_json::json!({ "nom": format!("{} p{}", g.sorties[i].1.nom, g.sorties[i].0.page), "x": pl.x / mm, "y": (pl.y + f.marge_bas) / mm, "largeur": l / mm, "hauteur": h / mm, "angle": pl.angle })
                        })
                        .collect();
                    let longueur = (p.planche.hauteur + f.marge_bas + f.marge_haut) / mm;
                    (longueur, p.remplissage(&g.candidats, g.laize_mm, m.imbrication.marge_bas_mm + m.imbrication.marge_haut_mm), places, nid::pret(p, &g.candidats, &m.imbrication, maintenant()))
                }
                None => (0.0, 0.0, vec![], false),
            };
            let d = &g.sorties[0].1.demande;
            liste.push(serde_json::json!({
                "machine": m.id, "code": g.code, "strategie": m.imbrication.strategie,
                "pages": g.plan.as_ref().map_or(0, |p| p.retenus.len()), "laize_mm": g.laize_mm, "longueur_mm": longueur,
                "remplissage": remplissage, "pret": pret, "contour": f.contour, "places": places, "elements": elements,
                "support": d.support, "decoupeuse": g.decoupeuse, "miroir": d.edition.miroir,
            }));
        }
    }
    Json(liste)
}

/// Machine et planche en préparation désignées par l'écran.
fn groupe_demande(etat: &Etat, machine: &str, code: &str) -> Option<(machines::Machine, Groupe)> {
    let catalogue = etat.machines.read().unwrap().clone();
    let m = catalogue.iter().find(|m| m.id == machine)?.clone();
    let g = groupes(etat, &catalogue, &m).into_iter().find(|g| g.code == code)?;
    Some((m, g))
}

/// « Imprimer maintenant » (roadmap 5.2) : la planche part sans attendre son déclencheur.
async fn imprimer_imbrication(State(etat): State<Partage>, Chemin((machine, code)): Chemin<(String, String)>) -> Response {
    let Some((m, g)) = groupe_demande(&etat, &machine, &code) else { return refus(StatusCode::NOT_FOUND, "introuvable", None) };
    match lancer_imbrication(&etat, &m, g).await {
        Ok(t) => (StatusCode::ACCEPTED, Json(t)).into_response(),
        Err((statut, (code, detail))) => refus(statut, code, detail),
    }
}

/// PDF de la planche en préparation, celui que « Imprimer maintenant » produirait.
async fn pdf_imbrication(State(etat): State<Partage>, Chemin((machine, code)): Chemin<(String, String)>) -> Response {
    let Some((m, g)) = groupe_demande(&etat, &machine, &code) else { return refus(StatusCode::NOT_FOUND, "introuvable", None) };
    let Some(plan) = g.plan.clone() else { return refus(StatusCode::CONFLICT, "rien_a_imbriquer", None) };
    let (sources, donnees, finition) = (sources_planche(&g, &plan), etat.config.donnees.clone(), m.imbrication.finition());
    match tokio::task::spawn_blocking(move || pdf_planche(&donnees, &sources, &plan, finition)).await {
        Ok(Ok(pdf)) => ([(header::CONTENT_TYPE, "application/pdf")], pdf).into_response(),
        Ok(Err(e)) => refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e)),
        Err(e) => refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string())),
    }
}

/// Fiche travail (roadmap 5.6) : enregistrée dans `job.json` ; l'étiquette ou le code
/// imprimés la reprennent (rendu refait si le travail en porte un).
async fn enregistrer_fiche(State(etat): State<Partage>, Chemin(id): Chemin<Uuid>, Json(fiche): Json<Fiche>) -> Response {
    if !fiche.valide() {
        return refus(StatusCode::BAD_REQUEST, "fiche_invalide", None);
    }
    let Some(mut t) = lire_travail(&etat.config.donnees, id) else { return refus(StatusCode::NOT_FOUND, "introuvable", None) };
    let imprimee = t.demande.edition.etiquette.is_some() || t.demande.edition.code.is_some();
    let change = imprimee && (fiche.commande != t.fiche.commande || fiche.quantite != t.fiche.quantite);
    t.fiche = fiche;
    if change && matches!(t.statut, Statut::Termine | Statut::Echec) && !occupe(&etat, &t) {
        return reconstruire(&etat, t).await;
    }
    match ecrire(&etat.config.donnees, &t) {
        Ok(()) => Json(t).into_response(),
        Err(e) => refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(e.to_string())),
    }
}

#[derive(Deserialize)]
struct Exclusion {
    exclue: bool,
}

/// Page à ne pas imprimer (roadmap 5.12) : décochée, elle quitte la file (sorties pas encore
/// parties) et n'entre plus dans aucune planche ; recochée, elle y revient (mise en file
/// automatique du travail, s'il en a une).
async fn exclure_page(State(etat): State<Partage>, Chemin((id, n)): Chemin<(Uuid, usize)>, Json(e): Json<Exclusion>) -> Response {
    let donnees = &etat.config.donnees;
    let Some(mut t) = lire_travail(donnees, id) else { return refus(StatusCode::NOT_FOUND, "introuvable", None) };
    if n == 0 || n > t.pages.len() {
        return refus(StatusCode::NOT_FOUND, "introuvable", None);
    }
    let avant = t.pages_exclues.contains(&n);
    t.pages_exclues.retain(|&p| p != n);
    if e.exclue {
        t.pages_exclues.push(n);
        t.pages_exclues.sort_unstable();
    }
    if let Err(err) = ecrire(donnees, &t) {
        return refus(StatusCode::INTERNAL_SERVER_ERROR, "erreur_interne", Some(err.to_string()));
    }
    if e.exclue {
        let mut f = etat.file.lock().unwrap();
        f.sorties.retain(|s| !(s.travail == id && s.page == n && matches!(s.etat, file::Etat::EnAttente | file::Etat::Suspendue)));
        let _ = f.ecrire(donnees);
    } else if avant && matches!(t.statut, Statut::Termine) && !etat.file.lock().unwrap().sorties.iter().any(|s| s.travail == id && s.page == n && s.active()) {
        mettre_en_file_auto(&etat, &t, Some(n));
    }
    Json(t).into_response()
}

/// Répartiteur : chaque seconde, pour chaque machine du catalogue qui n'a rien en cours,
/// lance la première sortie en attente qui la vise. Une sortie à la fois par machine. Une
/// variante (roadmap 4.15) ne part pas seule : sa machine d'origine prend ses sorties.
async fn repartir(etat: Partage) {
    let mut tour: u64 = 0;
    loop {
        // Imbrication (roadmap 5.2) : toutes les 5 s, les planches prêtes partent.
        tour += 1;
        if tour.is_multiple_of(5) {
            imbriquer_les_pretes(&etat).await;
        }
        let ids: Vec<String> = etat.machines.read().unwrap().iter().filter(|m| m.variante_de.is_none()).map(|m| m.id.clone()).collect();
        for id in ids {
            let suivante = {
                let mut f = etat.file.lock().unwrap();
                let catalogue = etat.machines.read().unwrap().clone();
                let m = catalogue.iter().find(|m| m.id == id).cloned();
                let charge = supports::lire_charges(&etat.config.donnees).remove(&id);
                let mut aussi: Vec<&str> = machines::variantes(&catalogue, &id);
                aussi.extend(m.as_ref().and_then(|m| m.pool.as_deref()));
                f.prochaine_dans(&id, &aussi, |s| raison_attente(&etat.config.donnees, s, m.as_ref(), charge.as_ref(), &f.sorties, &catalogue).is_none()).map(|i| {
                    let s = &mut f.sorties[i];
                    // Envoyée au pool ou à une variante : elle revient à la machine qui la prend.
                    s.machine = id.clone();
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

/// Exécute une sortie selon le modèle et l'opération, et note le résultat. Une impression
/// par le pilote direct est bornée par deux relevés SNMP forcés (roadmap 6.1) ; chaque
/// impression réussie entre au journal (6.4).
async fn executer(etat: Partage, s: file::Sortie) {
    let echec = |e: String| ("sortie_echouee", e);
    let m = machine(&etat, &s.machine);
    let direct = m.as_ref().filter(|m| s.operation() == machines::Operation::Impression && m.connexion.flux_direct());
    let du = match direct {
        Some(m) => relever_maintenant(&etat, m).await,
        None => None,
    };
    let debut = du.or(s.debut).unwrap_or_else(maintenant);
    let resultat = match m.clone() {
        None => Err(echec(format!("machine « {} » absente du catalogue", s.machine))),
        Some(m) => match (s.operation(), m.connexion.clone()) {
            (machines::Operation::Impression, machines::Connexion::Dossier) => sortir_les_plaques(&etat, &s, m.modele).await.map(|()| None).map_err(echec),
            (machines::Operation::Impression, c) if m.modele == machines::Modele::EpsonScS80600 && c.flux_direct() => imprimer_sur_l_epson(&etat, &s, &m, c).await.map(Some),
            (machines::Operation::Impression, _) => Err(echec(format!("pilote d'impression à venir pour {}", m.nom))),
            (machines::Operation::Decoupe, _) => envoyer_la_decoupe(&etat, &s, m).await.map(|()| None).map_err(echec),
        },
    };
    if let Err((_, e)) = &resultat {
        eprintln!("sortie {} ({}, travail {} page {}) : {e}", s.id, s.machine, s.travail, s.page);
    }
    let au = match (direct, du, &resultat) {
        (Some(m), Some(du), Ok(_)) => {
            // Relevés à la seconde : la fin doit tomber après le début pour borner la fenêtre.
            if maintenant() <= du {
                tokio::time::sleep(Duration::from_millis(1100)).await;
            }
            relever_maintenant(&etat, m).await
        }
        _ => None,
    };
    let tarifs = couverture::lire(&etat.config.donnees);
    let reel = du.zip(au).and_then(|(du, au)| consommation::Reel::mesurer(&releves::lire(&etat.config.donnees, &s.machine), du, au, |n| consommation::contenance_ml(&tarifs, n)));
    if let (Ok(points), Some(m)) = (&resultat, &m)
        && s.operation() == machines::Operation::Impression
    {
        noter_impression(&etat, &s, m, debut, reel.clone(), points.as_ref(), &tarifs);
    }
    let mut f = etat.file.lock().unwrap();
    let imprimee = resultat.is_ok() && s.operation() == machines::Operation::Impression;
    if let Some(x) = f.sorties.iter_mut().find(|x| x.id == s.id) {
        x.fin = Some(maintenant());
        x.consommation = reel;
        match resultat {
            Ok(_) => x.etat = file::Etat::Terminee,
            Err((code, e)) => {
                if code == "sortie_echouee" {
                    rapports::signaler(&etat.config.donnees, "erreur", code, &e);
                }
                (x.etat, x.code_erreur, x.erreur) = (file::Etat::Echec, Some(code.into()), Some(e))
            }
        }
    }
    // Planche imprimée : les impressions qu'elle remplaçait le sont aussi (elles ne
    // repasseront plus, même planche supprimée).
    if imprimee {
        for x in f.sorties.iter_mut().filter(|x| x.planche == Some(s.travail) && x.etat == file::Etat::Imbriquee && x.operation() == machines::Operation::Impression) {
            (x.etat, x.fin) = (file::Etat::Terminee, Some(maintenant()));
        }
    }
    // Découpe terminée : laissée, archivée ou retirée (roadmap 4.12) ; sa fin est écrite
    // avant, pour que le pont ERP la voie même retirée.
    if s.operation() == machines::Operation::Decoupe {
        let _ = f.ecrire(&etat.config.donnees);
        let politique = decoupe::reglages().apres_coupe;
        // Archivée ou retirée tout de suite : la veille de la file ne la verrait pas finie.
        if politique != file::ApresCoupe::Rien
            && let Some(x) = f.sorties.iter().find(|x| x.id == s.id && x.etat == file::Etat::Terminee)
        {
            let noms: std::collections::HashMap<String, String> = etat.machines.read().unwrap().iter().map(|m| (m.id.clone(), m.nom.clone())).collect();
            let vue = mobile::VueSortie { id: x.id, travail: x.travail, machine: &x.machine, nom: &x.nom, page: x.page, etat: "terminee", decoupe: true, erreur: None };
            mobile::noter_fin(&etat.config.donnees, &vue, &noms);
        }
        f.apres_coupe(s.id, politique);
    }
    let _ = f.ecrire(&etat.config.donnees);
    drop(f);
    // Planche en échec : ses sorties reviennent.
    reprendre_les_imbriquees(&etat);
}

/// Ajoute une impression réussie au journal (roadmap 6.4) : encre estimée par les points
/// posés quand le pilote direct a tramé (6.9), sinon non calculable (6.7).
fn noter_impression(etat: &Etat, s: &file::Sortie, m: &machines::Machine, debut: u64, reel: Option<consommation::Reel>, points: Option<&epson::Points>, tarifs: &couverture::Tarifs) {
    let donnees = &etat.config.donnees;
    let Some((t, p)) = lire_page(donnees, s.travail, s.page) else { return };
    let catalogue = supports::lire(donnees, "epson");
    let support = catalogue.supports.iter().find(|x| t.demande.support_uuid.is_some() && x.id == t.demande.support_uuid);
    let mode = catalogue.supports.iter().flat_map(|x| &x.modes).find(|x| t.demande.mode_uuid.is_some() && x.id == t.demande.mode_uuid);
    let nom_support = t.demande.support.clone().or_else(|| support.map(|x| x.nom.clone()));
    let cout_support = couverture::estimer(p.surface_m2, p.hauteur_mm, &Default::default(), &[], tarifs, nom_support.as_deref()).cout_support;
    let reglages = consommation::lire_reglages(donnees);
    let dpi = t.demande.dpi;
    let estime = match points {
        Some(points) => {
            let points: Vec<(String, [u64; 3])> = points.iter().map(|(e, n)| (e.nom().to_string(), *n)).collect();
            let pl = mode.and_then(|x| x.gouttes_pl).unwrap_or(consommation::GOUTTES_PL);
            consommation::estimer_points(&points, pl, |n| reglages.facteur(&m.id, dpi, n), tarifs, cout_support)
        }
        None => consommation::Estime { cout_support, ttc: tarifs.affichage_ttc, ..Default::default() },
    };
    let i = consommation::Impression {
        sortie: s.id,
        travail: s.travail,
        nom: t.nom.clone(),
        page: s.page,
        machine: m.id.clone(),
        debut,
        fin: maintenant(),
        support: nom_support,
        mode: Some(mode.map_or(format!("{dpi} dpi"), |x| x.nom.clone())),
        dpi,
        operateur: t.fiche.operateur.clone(),
        surface_m2: p.surface_m2,
        longueur_prevue_mm: p.hauteur_mm,
        reel,
        estime,
    };
    if let Err(e) = consommation::noter(donnees, &i) {
        eprintln!("journal des impressions : {e}");
    }
}

/// Intervalle des relevés SNMP pendant une impression par le pilote direct.
const RELEVE_IMPRESSION: Duration = Duration::from_secs(5);

/// Pilote direct de l'Epson (roadmap 2.4) : plaques CMJN de la page → séparation 9 encres
/// → tramage 2 bits → ESC/P2 écrit dans la connexion TCP au fil des bandes (`epson.rs`).
/// Avec une adresse SNMP, les alertes sont lues avant l'envoi, puis toutes les 5 s pendant
/// l'envoi et jusqu'à la fin de l'impression (`hrPrinterStatus` quitte « imprime ») : une
/// alerte bloquante coupe la connexion et fait échouer la sortie. Un relevé sans réponse
/// ne fait rien échouer : il ne dit rien.
async fn imprimer_sur_l_epson(etat: &Etat, s: &file::Sortie, m: &machines::Machine, c: machines::Connexion) -> Result<epson::Points, (&'static str, String)> {
    let echec = |e: String| ("sortie_echouee", e);
    let (t, p) = lire_page(&etat.config.donnees, s.travail, s.page).ok_or(echec("travail ou page disparu".into()))?;
    epson::verifier_pour_le_pilote(&p.plaques, t.demande.couches, &t.demande.empilement).map_err(|e| ("envoi_impossible", e))?;
    let rep = dossier(&etat.config.donnees, s.travail);
    let plaques = epson::PLAQUES_CMJN.map(|n| render::chemin_plaque(&rep, s.page, n));
    // Séparation portée par le mode du travail (roadmap 8.11), sinon celle par défaut.
    let catalogue = supports::lire(&etat.config.donnees, "epson");
    let mode = catalogue.supports.iter().flat_map(|x| &x.modes).find(|x| t.demande.mode_uuid.is_some() && x.id == t.demande.mode_uuid);
    let separation = separation::Separation::du_mode(mode.and_then(|x| x.bascule), mode.and_then(|x| x.limite_totale));
    let dpi = t.demande.dpi;
    let points = |mm: f64| (mm / 25.4 * dpi as f64).floor().max(0.0) as u32;
    let travail = escp2::Travail {
        // Nom de la sortie : « -BAT » pour une épreuve (roadmap 9.5).
        nom: s.nom.clone(),
        poste: "E-KO RIP Center".into(),
        utilisateur: String::new(),
        largeur: 0,
        hauteur: 0,
        dpi,
        x: points(m.origine_impression_mm[0]) / 16 * 16,
        // Tête laissée vide avant l'image (roadmap 2.4, 1er tirage du 25/10) : une image
        // collée au début de la page donnait une bande foncée, la mise en route des passes ;
        // le flux de référence observé commence 19,8 mm plus bas. L'origine Y se compte après.
        y: points(TETE_EPSON_MM + m.origine_impression_mm[1].max(0.0)),
        page_largeur: points(m.laize_mm),
        page_hauteur: 0,
    };
    let communaute = env_ou("EKO_RIP_EPSON_COMMUNAUTE", "public");
    let alertes = |h: String, c: String| async move {
        let a = tokio::task::spawn_blocking(move || media::lire_alertes_bloquantes(&h, &c, Duration::from_secs(2))).await;
        a.ok().and_then(Result::ok).filter(|a| !a.is_empty()).map(|a| a.join(", "))
    };
    if let Some(h) = &m.snmp
        && let Some(a) = alertes(h.clone(), communaute.clone()).await
    {
        return Err(("imprimante_en_alerte", a));
    }
    // Réseau brut (9100), IPP ou autre RIP (roadmap 9.4) : le même flux ESC/P2.
    let nom = s.nom.clone();
    let connexion = tokio::task::spawn_blocking(move || ports::ouvrir(&c, &nom)).await.map_err(|e| echec(e.to_string()))?.map_err(echec)?;
    let coupure = connexion.coupure().map_err(|e| echec(e.to_string()))?;
    let mut envoi = tokio::task::spawn_blocking(move || {
        let (w, points) = epson::imprimer(plaques.each_ref().map(PathBuf::as_path), travail, &separation, &tramage::Tramage::default(), std::io::BufWriter::new(connexion))?;
        let c = w.into_inner().map_err(|e| format!("envoi du flux : {}", e.error()))?;
        c.finir()?;
        Ok(points)
    });
    let Some(h) = m.snmp.clone() else { return envoi.await.map_err(|e| echec(e.to_string()))?.map_err(echec) };
    let mut tic = tokio::time::interval(RELEVE_IMPRESSION);
    tic.tick().await;
    let points = loop {
        tokio::select! {
            r = &mut envoi => break r.map_err(|e| echec(e.to_string()))?.map_err(echec)?,
            _ = tic.tick() => if let Some(a) = alertes(h.clone(), communaute.clone()).await {
                let _ = coupure.shutdown(std::net::Shutdown::Both);
                let _ = envoi.await;
                return Err(("imprimante_en_alerte", a));
            },
        }
    };
    // Envoyé : l'imprimante finit la page. Surveillée tant qu'elle se dit en train
    // d'imprimer, au plus deux fois la durée estimée plus 10 min.
    let limite = m.duree_impression_min(p.hauteur_mm, dpi).map_or(Duration::from_secs(4 * 3600), |d| Duration::from_secs_f64((2.0 * d + 10.0) * 60.0));
    let fin = tokio::time::Instant::now() + limite;
    while tokio::time::Instant::now() < fin {
        if let Some(a) = alertes(h.clone(), communaute.clone()).await {
            return Err(("imprimante_en_alerte", a));
        }
        let (hh, c) = (h.clone(), communaute.clone());
        if tokio::task::spawn_blocking(move || media::imprime(&hh, &c, Duration::from_secs(2))).await.ok().flatten() != Some(true) {
            break;
        }
        tic.tick().await;
    }
    Ok(points)
}

/// Tête vide en début de page sur l'Epson (mm), avant l'origine Y de la machine.
const TETE_EPSON_MM: f64 = 20.0;

/// Découpe : les couches de la page (ou la seule de la sortie) au pas de la machine, par
/// sa connexion ; pour le Kona, mis en forme comme par CutServer (lecture du cadre s'il y en
/// a un). Sans connexion (dossier), le fichier est déposé d'un coup dans
/// `sorties/<machine>/` : un logiciel de découpe qui surveille le dossier ne lit jamais un
/// fichier à moitié écrit (roadmap 4.6).
async fn envoyer_la_decoupe(etat: &Etat, s: &file::Sortie, m: machines::Machine) -> Result<(), String> {
    let donnees = &etat.config.donnees;
    let (t, p) = lire_page(donnees, s.travail, s.page).ok_or("travail ou page disparu")?;
    let traces = lire_traces(&dossier(donnees, s.travail), s.page).ok_or("tracés de la page introuvables")?;
    let texte = pour_la_machine(&traces, &t, &p, &m, s.couche.as_deref())?;
    let depot = (m.connexion == machines::Connexion::Dossier).then(|| donnees.join("sorties").join(&m.id).join(format!("{}-p{}-{}.plt", s.travail, s.page, s.id)));
    tokio::task::spawn_blocking(move || match depot {
        Some(chemin) => {
            std::fs::create_dir_all(chemin.parent().unwrap()).map_err(|e| e.to_string())?;
            machines::ecrire_atomique(&chemin, texte.as_bytes())
        }
        None => machines::envoyer(&m.connexion, texte.as_bytes()),
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
        let envoi = epson::preparer(&p.plaques, t.demande.couches, &t.demande.empilement)?;
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

/// HP-GL tel que la machine le reçoit : les couches actives dans leur ordre (ou la seule
/// couche `seule`), au pas de la machine, décalées, chacune avec son réglage de coupe (motif ;
/// pour le Kona, `AS`/`ZF`/`VS` après son `SP`, lecture du cadre s'il y en a un).
/// `Err` : un réglage choisi pour une couche n'existe pas sur la machine.
fn pour_la_machine(traces: &[decoupe::Trace], t: &Travail, p: &Page, m: &machines::Machine, seule: Option<&str>) -> Result<String, String> {
    let pose = posee(t, p, traces);
    let mut parties = vec![];
    for (c, bloc) in decoupe::blocs(&pose.traces, &t.couches_decoupe, &decoupe::reglages()).into_iter().filter(|(c, _)| seule.is_none_or(|n| decoupe::meme_couche(n, &c.nom))) {
        let reglage = m.reglage_couche(&c).map_err(|n| format!("couche « {} » : réglage « {n} » absent de {}", c.nom, m.nom))?;
        let mut texte = machines::changer_de_pas(&bloc, m.pas_mm(), m.decalage_mm);
        if let Some(r) = reglage.filter(|r| !r.motif_mm.is_empty()) {
            let motif: Vec<[f64; 2]> = r.motif_mm.iter().map(|p| p.map(|v| v / m.pas_mm())).collect();
            texte = machines::pointiller(&texte, &motif, r.diffusion);
        }
        parties.push((texte, reglage));
    }
    if m.modele != machines::Modele::MutohKona {
        return Ok(format!("IN;{}PU;SP0;", parties.into_iter().map(|(texte, _)| texte).collect::<String>()));
    }
    let u = |v: f64| (v * 100.0 * 25.4 / 72.0).round() as i64;
    Ok(machines::pour_kona(&parties, pose.cadre.map(|c| c.map(u)), m.decoupe_retournee))
}

#[derive(Deserialize)]
struct ChoixMachine {
    #[serde(default)]
    machine: Option<String>,
}

/// Avec un cadre à code-barres, nommé pour Mutoh CutServer : `Nom_SF|MF_<numéro>.plt`.
/// `?machine=<id>` : le fichier tel que cette machine le reçoit (à envoyer à la main).
async fn fichier_decoupe(State(etat): State<Partage>, Chemin((id, page)): Chemin<(Uuid, usize)>, axum::extract::Query(q): axum::extract::Query<ChoixMachine>) -> Response {
    let cutserver = lire_page(&etat.config.donnees, id, page).and_then(|(t, p)| {
        let mot = p.reperage.unwrap_or(t.demande.reperage).code_cutserver()?;
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
            let rep = dossier(&etat.config.donnees, id);
            let o = match (&machine, lire_page(&etat.config.donnees, id, page), lire_traces(&rep, page)) {
                (Some(m), Some((t, p)), Some(traces)) => match pour_la_machine(&traces, &t, &p, m, None) {
                    Ok(texte) => texte.into_bytes(),
                    Err(e) => return refus(StatusCode::CONFLICT, "decoupeuse_indisponible", Some(e)),
                },
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

/// Composite d'avant le filtre couleur : 404 sans filtre.
async fn apercu_avant(State(etat): State<Partage>, Chemin((id, page)): Chemin<(Uuid, usize)>) -> Response {
    match servir(&etat.config.donnees, id, page, |rep, _| Some(render::chemin_composite_avant(rep, page))).await {
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

    /// Les réglages de découpe sont globaux au processus : les essais qui les installent
    /// passent l'un après l'autre (sinon la restauration d'une sauvegarde efface les préfixes
    /// qu'un autre essai vient d'ajouter).
    static DECOUPE_GLOBALE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

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
            couches_decoupe: vec![],
            mise_coupe: Default::default(),
            fiche: Default::default(),
            pages_exclues: vec![],
            imbrication: None,
            erp: None,
            retenue_erp: None,
            estimation: false,
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
        let source = [include_str!("main.rs"), include_str!("reglages.rs"), include_str!("couleur.rs"), include_str!("file.rs"), include_str!("couverture.rs"), include_str!("consommation.rs"), include_str!("blanc.rs"), include_str!("edition.rs"), include_str!("media.rs"), include_str!("machines.rs"), include_str!("supports.rs"), include_str!("mosaique.rs"), include_str!("filtre.rs"), include_str!("decoupe.rs"), include_str!("coupe.rs"), include_str!("detourage.rs"), include_str!("reperage.rs"), include_str!("pont.rs"), include_str!("tons.rs"), include_str!("images.rs")].map(sans_tests).concat();
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
                rendus: 1,
                conservation_jours: 0,
                seuil_libre: 0,
                seuil_alerte: 0,
                purger_erp_faits: false,
                plotter: "/dev/null".into(),
                plotter_stty: String::new(),
                epson_snmp: None,
                acces_navigateur: false,
            };
            let machines = std::sync::RwLock::new(machines::catalogue_initial(&config.plotter, &config.plotter_stty, None));
            Arc::new(Etat { config, rendu: rendus::Rendus::new(1), file: Default::default(), reglages: Default::default(), sessions: Default::default(), machines })
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

        /// Envoi avec `reglage` (clients natifs) : le réglage enregistré remplace la demande ;
        /// un nom inconnu est refusé.
        #[tokio::test]
        async fn envoi_avec_un_reglage_enregistre() {
            let e = etat("envoi-reglage");
            let r = appel(&e, "POST", "/api/reglages", None, Some(serde_json::json!({ "nom": "Coupe", "demande": { "mode": "decouper" } }))).await;
            assert!(r.status().is_success(), "{:?}", r.status());
            let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10mm" height="10mm" viewBox="0 0 10 10"><rect x="1" y="1" width="8" height="8" fill="none" stroke="#000"/></svg>"##;
            let envoi = |reglage: &str| {
                let corps = format!("--x\r\nContent-Disposition: form-data; name=\"reglage\"\r\n\r\n{reglage}\r\n--x\r\nContent-Disposition: form-data; name=\"fichier\"; filename=\"a.svg\"\r\nContent-Type: image/svg+xml\r\n\r\n{svg}\r\n--x--\r\n");
                Request::builder().method("POST").uri("/api/travaux").header(header::USER_AGENT, "Mozilla/5.0 EKO-RIP-Client/0.9").header(header::CONTENT_TYPE, "multipart/form-data; boundary=x").body(Body::from(corps)).unwrap()
            };
            let r = routes(e.clone()).oneshot(envoi("Inconnu")).await.unwrap();
            assert_eq!(r.status(), StatusCode::BAD_REQUEST);
            let v: serde_json::Value = serde_json::from_slice(&axum::body::to_bytes(r.into_body(), usize::MAX).await.unwrap()).unwrap();
            assert_eq!(v["code"], serde_json::json!("reglage_inconnu"));
            let r = routes(e.clone()).oneshot(envoi("Coupe")).await.unwrap();
            assert_eq!(r.status(), StatusCode::ACCEPTED);
            let v: serde_json::Value = serde_json::from_slice(&axum::body::to_bytes(r.into_body(), usize::MAX).await.unwrap()).unwrap();
            assert_eq!(v["mode"], serde_json::json!("decouper"));
        }

        /// Cloud : une requête venue par le tunnel n'atteint ni le pont ERP, ni le renvoi, ni
        /// les réglages du pont et du cloud, et rien du tout sans compte (l'accès libre reste
        /// au réseau local). L'appairage est refusé sans compte.
        #[tokio::test]
        async fn cloud_ferme_ce_qui_ne_sort_pas_de_l_atelier() {
            let e = etat("cloud-garde");
            let par_cloud = |methode: &str, uri: &str| {
                let r = Request::builder().method(methode).uri(uri).header(header::USER_AGENT, "EKO-RIP-Client/1.0").extension(cloud::ParCloud).body(Body::empty()).unwrap();
                let e = e.clone();
                async move { routes(e).oneshot(r).await.unwrap() }
            };
            let code = |r: Response| async move { serde_json::from_slice::<serde_json::Value>(&axum::body::to_bytes(r.into_body(), usize::MAX).await.unwrap()).unwrap()["code"].as_str().unwrap_or("").to_string() };
            // Sans compte : tout est fermé par le cloud, ouvert en local ; pas d'appairage.
            let r = par_cloud("GET", "/api/travaux").await;
            assert_eq!(r.status(), StatusCode::FORBIDDEN);
            assert_eq!(code(r).await, "cloud_sans_compte");
            assert_eq!(appel(&e, "GET", "/api/travaux", None, None).await.status(), StatusCode::OK);
            let r = appel(&e, "POST", "/api/cloud/appairage", None, None).await;
            assert_eq!(r.status(), StatusCode::CONFLICT);
            assert_eq!(code(r).await, "cloud_sans_compte");

            acces::ecrire(&e.config.donnees, &[acces::Utilisateur { role: acces::Role::Admin, nom: "chef".into(), hachage: acces::hacher("phrase-de-passe-longue").unwrap() }]).unwrap();
            for (m, uri) in [("GET", "/api/pont"), ("POST", "/api/pont"), ("POST", "/api/renvoi/epson"), ("GET", &format!("{}contrat", erp::PREFIXE)), ("POST", "/api/cloud/appairage"), ("DELETE", "/api/cloud")] {
                let r = par_cloud(m, uri).await;
                assert_eq!(r.status(), StatusCode::FORBIDDEN, "{m} {uri}");
                assert_eq!(code(r).await, "ferme_par_cloud", "{m} {uri}");
            }
            // Le reste suit les règles habituelles : session demandée.
            assert_eq!(par_cloud("GET", "/api/travaux").await.status(), StatusCode::UNAUTHORIZED);
            assert_eq!(par_cloud("GET", "/api/cloud").await.status(), StatusCode::UNAUTHORIZED);

            // Personne connectée par la plate-forme (« Se connecter au cloud ») : pas de session
            // locale, son rôle sur le site décide ; l'en-tête n'est cru que venu par le tunnel.
            let identite = |role: &str| eko_rip_cloud::base64url(serde_json::json!({ "compte": 1, "email": "chef@atelier.fr", "nom": "Chef", "role": role }).to_string().as_bytes());
            let avec = |methode: &str, uri: &str, role: &str, tunnel: bool| {
                let mut r = Request::builder().method(methode).uri(uri).header(header::USER_AGENT, "EKO-RIP-Client/1.0").header("x-eko-cloud-identite", identite(role)).body(Body::empty()).unwrap();
                if tunnel {
                    r.extensions_mut().insert(cloud::ParCloud);
                }
                let e = e.clone();
                async move { routes(e).oneshot(r).await.unwrap() }
            };
            assert_eq!(avec("GET", "/api/travaux", "admin", true).await.status(), StatusCode::OK);
            assert_eq!(avec("GET", "/api/travaux", "admin", false).await.status(), StatusCode::UNAUTHORIZED, "en local, l'en-tête est ignoré");
            let r = avec("GET", "/api/session", "operateur", true).await;
            let session: serde_json::Value = serde_json::from_slice(&axum::body::to_bytes(r.into_body(), usize::MAX).await.unwrap()).unwrap();
            assert_eq!((session["utilisateur"].as_str(), session["role"].as_str(), session["cloud"]["lecture_seule"].as_bool()), (Some("chef@atelier.fr"), Some("operateur"), Some(false)));
            let r = avec("POST", "/api/tarifs", "operateur", true).await;
            assert_eq!(code(r).await, "admin_requis");
            let r = avec("POST", "/api/travaux/x/relancer", "lecture", true).await;
            assert_eq!(code(r).await, "lecture_seule");
            assert_eq!(avec("GET", "/api/travaux", "lecture", true).await.status(), StatusCode::OK);
            // Lecture seule : le téléphone peut quand même s'inscrire aux notifications.
            let r = avec("POST", "/api/mobile/appareils", "lecture", true).await;
            assert_ne!(r.status(), StatusCode::FORBIDDEN, "inscription mobile permise en lecture seule");
            // Toujours fermé par le cloud, même identifié.
            assert_eq!(code(avec("POST", "/api/pont", "admin", true).await).await, "ferme_par_cloud");
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// Fausse plate-forme de l'abonnement : enregistre les requêtes reçues, répond selon
        /// `statut_sauvegarde` et `archive`.
        #[derive(Default)]
        struct FaussePlateforme {
            recu: std::sync::Mutex<Vec<(String, axum::http::HeaderMap, Vec<u8>)>>,
            statut_sauvegarde: std::sync::atomic::AtomicU16,
            sauvegardes_permises: std::sync::atomic::AtomicBool,
            archive: std::sync::Mutex<Vec<u8>>,
        }

        async fn fausse_plateforme() -> (Arc<FaussePlateforme>, String) {
            use axum::extract::State as Etat;
            use std::sync::atomic::Ordering::Relaxed;
            let f = Arc::new(FaussePlateforme::default());
            f.statut_sauvegarde.store(201, Relaxed);
            f.sauvegardes_permises.store(true, Relaxed);
            let app = axum::Router::new()
                .route("/api/site/inventaire", axum::routing::post(|Etat(f): Etat<Arc<FaussePlateforme>>, h: axum::http::HeaderMap, c: axum::body::Bytes| async move {
                    f.recu.lock().unwrap().push(("inventaire".into(), h, c.to_vec()));
                    Json(serde_json::json!({ "offre": "atelier", "sauvegardes": f.sauvegardes_permises.load(Relaxed), "depassement": true }))
                }))
                .route("/api/site/sauvegardes", axum::routing::post(|Etat(f): Etat<Arc<FaussePlateforme>>, h: axum::http::HeaderMap, c: axum::body::Bytes| async move {
                    f.recu.lock().unwrap().push(("sauvegarde".into(), h, c.to_vec()));
                    let statut = StatusCode::from_u16(f.statut_sauvegarde.load(Relaxed)).unwrap();
                    let erreur = match statut.as_u16() { 402 => "offre", 413 => "quota", 409 => "identique", _ => "" };
                    (statut, Json(serde_json::json!({ "id": "sauv1", "erreur": erreur })))
                }))
                .route("/api/site/sauvegardes/{id}", get(|Etat(f): Etat<Arc<FaussePlateforme>>, h: axum::http::HeaderMap, Chemin(id): Chemin<String>| async move {
                    f.recu.lock().unwrap().push((format!("telechargement {id}"), h, vec![]));
                    if id == "absente" { return StatusCode::NOT_FOUND.into_response(); }
                    f.archive.lock().unwrap().clone().into_response()
                }))
                .with_state(f.clone());
            let ecoute = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let base = format!("http://{}", ecoute.local_addr().unwrap());
            tokio::spawn(async move { axum::serve(ecoute, app).await.unwrap() });
            (f, base)
        }

        async fn json(r: Response) -> serde_json::Value {
            serde_json::from_slice(&axum::body::to_bytes(r.into_body(), 1 << 26).await.unwrap()).unwrap_or_default()
        }

        async fn code(r: Response) -> String {
            json(r).await["code"].as_str().unwrap_or_default().to_string()
        }

        fn rattacher(e: &Partage, base: &str) {
            std::fs::write(cloud::fichier(&e.config.donnees), serde_json::to_vec(&cloud::Config { url: base.into(), site: "s1".into(), cle: "cle-secrete".into() }).unwrap()).unwrap();
        }

        /// Inventaire : imprimantes et découpeurs envoyés avec les identifiants du tunnel ;
        /// la réponse (offre, dépassement) est exposée par `GET /api/cloud`.
        #[tokio::test(flavor = "multi_thread")]
        async fn abonnement_inventaire_envoye_a_la_plateforme() {
            let e = etat("abo-inventaire");
            let (f, base) = fausse_plateforme().await;
            rattacher(&e, &base);
            let liste = e.machines.read().unwrap().clone();
            assert!(abonnement::envoyer_inventaire(&e.config.donnees, liste.clone()).await);
            {
                let recu = f.recu.lock().unwrap();
                let (_, h, corps) = recu.iter().find(|r| r.0 == "inventaire").expect("inventaire reçu");
                assert_eq!(h["authorization"], "Bearer s1.cle-secrete");
                let v: serde_json::Value = serde_json::from_slice(corps).unwrap();
                assert_eq!(v["version"], env!("CARGO_PKG_VERSION"));
                let nb = |c: &str| v[c].as_array().unwrap().len();
                assert_eq!(nb("imprimantes"), liste.iter().filter(|m| m.modele.imprime()).count());
                assert_eq!(nb("imprimantes") + nb("decoupeurs"), liste.len());
            }
            let c = json(appel(&e, "GET", "/api/cloud", None, None).await).await;
            assert_eq!(c["abonnement"], serde_json::json!({ "offre": "atelier", "sauvegardes": true, "depassement": true }));
            assert_eq!(c["sauvegarde"], serde_json::json!({ "derniere": null, "etat": "ok", "message": null }));
        }

        /// Sauvegarde en ligne : en-têtes, empreinte, pas de renvoi si rien n'a changé, 402 (plus
        /// d'essai automatique jusqu'à un inventaire favorable), 413, 409.
        #[tokio::test(flavor = "multi_thread")]
        async fn abonnement_sauvegarde_poussee_et_refus_de_l_offre() {
            use std::sync::atomic::Ordering::Relaxed;
            let e = etat("abo-sauvegarde");
            let (f, base) = fausse_plateforme().await;
            let envois = || f.recu.lock().unwrap().iter().filter(|r| r.0 == "sauvegarde").count();
            // Non rattaché : rien ne part, et la route locale refuse.
            assert_eq!(abonnement::sauvegarder(&e, "demande").await.derniere, None);
            assert_eq!(envois(), 0);
            assert_eq!(appel(&e, "POST", "/api/cloud/sauvegarder", None, None).await.status(), StatusCode::CONFLICT);
            rattacher(&e, &base);

            let r = appel(&e, "POST", "/api/cloud/sauvegarder", None, None).await;
            assert_eq!(r.status(), StatusCode::OK);
            assert_eq!(json(r).await["etat"], "ok");
            {
                let recu = f.recu.lock().unwrap();
                let (_, h, corps) = recu.iter().find(|r| r.0 == "sauvegarde").unwrap();
                assert_eq!(h["authorization"], "Bearer s1.cle-secrete");
                assert_eq!(h["x-eko-motif"], "demande");
                assert_eq!(h["content-type"], "application/octet-stream");
                assert_eq!(h["x-eko-empreinte"].to_str().unwrap(), abonnement::sha256_octets(corps));
                let s: sauvegarde::Sauvegarde = serde_json::from_slice(corps).unwrap();
                assert_eq!(s.format, sauvegarde::FORMAT);
            }
            let derniere = json(appel(&e, "GET", "/api/cloud", None, None).await).await["sauvegarde"]["derniere"].as_u64();
            assert!(derniere.is_some());
            // Rien n'a changé : un « changement » ne renvoie rien ; la nuit renvoie quand même
            // (la plate-forme répond 409 identique et note la nuit), et c'est un succès.
            abonnement::sauvegarder(&e, "changement").await;
            assert_eq!(envois(), 1);
            f.statut_sauvegarde.store(409, Relaxed);
            assert_eq!(abonnement::sauvegarder(&e, "nuit").await.etat, "ok");
            assert_eq!(envois(), 2);
            assert_eq!(f.recu.lock().unwrap().last().unwrap().1["x-eko-motif"], "nuit");
            f.statut_sauvegarde.store(201, Relaxed);
            abonnement::sauvegarder(&e, "changement").await;
            assert_eq!(envois(), 2);
            // Un changement de configuration est envoyé.
            assert!(appel(&e, "POST", "/api/reglages", None, Some(serde_json::json!({ "nom": "Coupe", "demande": { "mode": "decouper" } }))).await.status().is_success());
            abonnement::sauvegarder(&e, "changement").await;
            assert_eq!(envois(), 3);
            assert_eq!(f.recu.lock().unwrap().last().unwrap().1["x-eko-motif"], "changement");

            // 402 : l'offre n'inclut pas la sauvegarde ; plus d'essai automatique.
            f.statut_sauvegarde.store(402, Relaxed);
            let s = abonnement::sauvegarder(&e, "demande").await;
            assert_eq!((s.etat.as_str(), s.sans_offre), ("offre", true));
            assert_eq!(envois(), 4);
            assert!(appel(&e, "POST", "/api/reglages", None, Some(serde_json::json!({ "nom": "Autre", "demande": { "mode": "decouper" } }))).await.status().is_success());
            abonnement::sauvegarder(&e, "nuit").await;
            assert_eq!(envois(), 4, "pas d'essai tant que l'inventaire ne l'autorise pas");
            // Inventaire favorable : l'essai reprend ; 413 = quota.
            f.statut_sauvegarde.store(413, Relaxed);
            assert!(abonnement::envoyer_inventaire(&e.config.donnees, vec![]).await);
            assert!(!abonnement::suivi(&e.config.donnees).sans_offre);
            assert_eq!(abonnement::sauvegarder(&e, "nuit").await.etat, "quota");
            assert_eq!(envois(), 5);
            // 409 : identique chez la plate-forme, c'est bon.
            f.statut_sauvegarde.store(409, Relaxed);
            assert_eq!(abonnement::sauvegarder(&e, "demande").await.etat, "ok");
            // Inventaire qui annonce une offre sans sauvegarde.
            f.sauvegardes_permises.store(false, Relaxed);
            abonnement::envoyer_inventaire(&e.config.donnees, vec![]).await;
            let c = json(appel(&e, "GET", "/api/cloud", None, None).await).await;
            assert_eq!((c["sauvegarde"]["etat"].as_str(), c["abonnement"]["sauvegardes"].as_bool()), (Some("offre"), Some(false)));
        }

        /// Restauration commandée par la plate-forme : signature HMAC, fenêtre de 300 s, rejeu.
        #[tokio::test(flavor = "multi_thread")]
        async fn abonnement_restauration_signee() {
            // Source : un serveur avec un réglage, dont l'archive est servie par la plate-forme.
            let source = etat("abo-restaure-source");
            assert!(appel(&source, "POST", "/api/reglages", None, Some(serde_json::json!({ "nom": "Coupe", "demande": { "mode": "decouper" } }))).await.status().is_success());
            let archive = axum::body::to_bytes(appel(&source, "GET", "/api/sauvegarde", None, None).await.into_body(), 1 << 26).await.unwrap().to_vec();
            let (f, base) = fausse_plateforme().await;
            *f.archive.lock().unwrap() = archive;
            // Cible : un serveur avec un compte, rattaché.
            let e = etat("abo-restaure-cible");
            acces::ecrire(&e.config.donnees, &[acces::Utilisateur { role: acces::Role::Admin, nom: "chef".into(), hachage: acces::hacher("phrase-de-passe-longue").unwrap() }]).unwrap();
            rattacher(&e, &base);
            let envoyer = |entete: Option<String>, corps: &str, par_cloud: bool| {
                let mut r = Request::builder().method("POST").uri("/api/cloud/restaurer").header(header::CONTENT_TYPE, "application/json");
                if let Some(h) = entete {
                    r = r.header(abonnement::ENTETE_SERVICE, h);
                }
                let mut r = r.body(Body::from(corps.to_string())).unwrap();
                if par_cloud {
                    r.extensions_mut().insert(cloud::ParCloud);
                }
                routes(e.clone()).oneshot(r)
            };
            let corps = r#"{"id":"sauv1"}"#;
            let maintenant = maintenant();
            let reglages = || reglages::lire(&e.config.donnees).len();
            assert_eq!(reglages(), 0);

            // Sans signature, mauvaise clé, corps modifié, trop vieille ou trop neuve : 401, rien ne change.
            for entete in [None, Some(abonnement::signature("autre-cle", maintenant, corps.as_bytes())), Some(abonnement::signature("cle-secrete", maintenant, br#"{"id":"autre"}"#)), Some(abonnement::signature("cle-secrete", maintenant - 301, corps.as_bytes())), Some(abonnement::signature("cle-secrete", maintenant + 301, corps.as_bytes())), Some("t=1,v1=zz".to_string())] {
                let r = envoyer(entete.clone(), corps, true).await.unwrap();
                assert_eq!(r.status(), StatusCode::UNAUTHORIZED, "{entete:?}");
                assert_eq!(code(r).await, "signature_cloud_invalide");
            }
            assert_eq!(reglages(), 0);
            assert!(f.recu.lock().unwrap().iter().all(|r| !r.0.starts_with("telechargement")));

            // Signature valide hors tunnel : la marque du tunnel manque, la session aussi.
            let valide = abonnement::signature("cle-secrete", maintenant, corps.as_bytes());
            assert!(envoyer(Some(valide.clone()), corps, false).await.unwrap().status().is_client_error());
            assert_eq!(reglages(), 0);

            // Signature valide par le tunnel (ni session ni agent de client) : restauré.
            let r = envoyer(Some(valide.clone()), corps, true).await.unwrap();
            assert_eq!(r.status(), StatusCode::OK);
            assert_eq!(json(r).await, serde_json::json!({ "restaure": true }));
            assert_eq!(reglages(), 1);
            {
                let recu = f.recu.lock().unwrap();
                let (_, h, _) = recu.iter().find(|r| r.0 == "telechargement sauv1").unwrap();
                assert_eq!(h["authorization"], "Bearer s1.cle-secrete");
            }
            // Rejeu de la même commande : refusé.
            assert_eq!(envoyer(Some(valide), corps, true).await.unwrap().status(), StatusCode::UNAUTHORIZED);

            // Archive inconnue de la plate-forme, identifiant malformé.
            let c = r#"{"id":"absente"}"#;
            let r = envoyer(Some(abonnement::signature("cle-secrete", maintenant, c.as_bytes())), c, true).await.unwrap();
            assert_eq!(r.status(), StatusCode::NOT_FOUND);
            let c = r#"{"id":"../x"}"#;
            let r = envoyer(Some(abonnement::signature("cle-secrete", maintenant, c.as_bytes())), c, true).await.unwrap();
            assert_eq!(r.status(), StatusCode::BAD_REQUEST);
        }

        /// Cloud de bout en bout : vraie plate-forme sur un port local, vrai tunnel ouvert par
        /// la boucle du serveur ; connexion avec un compte du RIP au travers de la plate-forme.
        #[tokio::test(flavor = "multi_thread")]
        async fn cloud_tunnel_de_bout_en_bout() {
            let e = etat("cloud-tunnel");
            acces::ecrire(&e.config.donnees, &[acces::Utilisateur { role: acces::Role::Admin, nom: "chef".into(), hachage: acces::hacher("phrase-de-passe-longue").unwrap() }]).unwrap();
            let plateforme = e.config.donnees.join("plateforme");
            let (site, cle) = eko_rip_cloud::sites::creer(&plateforme, "Atelier", None).unwrap();
            let relais = eko_rip_cloud::relais::nouveau(eko_rip_cloud::relais::Config { donnees: plateforme, url_publique: "http://plateforme.test".into(), max_octets: 1 << 20, jeton_stats: String::new() });
            let ecoute = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let base = format!("http://{}", ecoute.local_addr().unwrap());
            tokio::spawn(async move { axum::serve(ecoute, eko_rip_cloud::relais::routes(relais)).await.unwrap() });
            std::fs::write(cloud::fichier(&e.config.donnees), serde_json::to_vec(&cloud::Config { url: base.clone(), site: site.id.clone(), cle }).unwrap()).unwrap();
            tokio::spawn(cloud::boucle(e.config.donnees.clone(), routes(e.clone()).layer(axum::Extension(cloud::ParCloud))));
            let mut ouvert = false;
            for _ in 0..100 {
                if cloud::etat_public(&e.config.donnees)["etat"] == "connecte" {
                    ouvert = true;
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            assert!(ouvert, "{}", cloud::etat_public(&e.config.donnees));
            let etat = cloud::etat_public(&e.config.donnees);
            assert_eq!(etat["adresse"], eko_rip_cloud::relais::adresse_du_site(&base, &site.id));
            assert!(etat.get("cle").is_none() && !etat.to_string().contains("cle\""), "la clé ne sort jamais");

            let racine = format!("{base}/s/{}", site.id);
            let resultat = tokio::task::spawn_blocking(move || {
                let agent: ureq::Agent = ureq::Agent::config_builder().http_status_as_error(false).build().into();
                let statut = |r: Result<ureq::http::Response<ureq::Body>, ureq::Error>| r.unwrap().status().as_u16();
                let sans = statut(agent.get(format!("{racine}/api/travaux")).header("User-Agent", "EKO-RIP-Client/1.0").call());
                let r = agent.post(format!("{racine}/api/session")).header("User-Agent", "EKO-RIP-Client/1.0").send_json(serde_json::json!({ "nom": "chef", "mot_de_passe": "phrase-de-passe-longue" })).unwrap();
                let cookie = r.headers()["set-cookie"].to_str().unwrap().to_string();
                let jeton = cookie.split(';').next().unwrap().to_string();
                let avec = statut(agent.get(format!("{racine}/api/travaux")).header("User-Agent", "EKO-RIP-Client/1.0").header("Cookie", &jeton).call());
                let pont = statut(agent.get(format!("{racine}/api/pont")).header("User-Agent", "EKO-RIP-Client/1.0").header("Cookie", &jeton).call());
                (sans, r.status().as_u16(), cookie, avec, pont)
            })
            .await
            .unwrap();
            assert_eq!(resultat.0, 401);
            assert_eq!(resultat.1, 204);
            assert!(resultat.2.contains(&format!("Path=/s/{}/", site.id)), "{}", resultat.2);
            assert_eq!(resultat.3, 200);
            assert_eq!(resultat.4, 403);

            // Détaché : le tunnel se ferme aussitôt, le site repasse hors ligne.
            cloud::detacher(&e.config.donnees).unwrap();
            tokio::time::sleep(Duration::from_millis(300)).await;
            let racine = format!("{base}/s/{}", site.id);
            let apres = tokio::task::spawn_blocking(move || {
                let agent: ureq::Agent = ureq::Agent::config_builder().http_status_as_error(false).build().into();
                agent.get(format!("{racine}/api/travaux")).call().unwrap().status().as_u16()
            })
            .await
            .unwrap();
            assert_eq!(apres, 503);
            assert_eq!(cloud::etat_public(&e.config.donnees)["etat"], "inactif");
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// Renvoi vers un autre RIP (roadmap 9.4) : le flux part par la connexion de la
        /// machine de l'autre RIP ; sans le bon jeton, ou vers une machine inconnue, refusé.
        /// Le jeton ne sort jamais vers l'écran.
        #[tokio::test]
        async fn renvoi_vers_un_autre_rip() {
            let b = etat("renvoi");
            let ecoute = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let port_machine = ecoute.local_addr().unwrap().port();
            let recu = std::thread::spawn(move || {
                let (mut f, _) = ecoute.accept().unwrap();
                let mut o = vec![];
                std::io::Read::read_to_end(&mut f, &mut o).unwrap();
                o
            });
            b.machines.write().unwrap()[1].connexion = machines::Connexion::Reseau { hote: "127.0.0.1".into(), port: port_machine };
            let id = b.machines.read().unwrap()[1].id.clone();
            let jeton = erp::aleatoire();
            ports::ecrire(&b.config.donnees, &ports::ConfigRenvoi { jeton_sha256: Some(erp::empreinte(&jeton)) }).unwrap();
            let serveur = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = serveur.local_addr().unwrap().port();
            tokio::spawn(axum::serve(serveur, routes(b.clone())).into_future());
            let vers = |machine: &str, jeton: &str| machines::Connexion::RipEko { hote: "127.0.0.1".into(), port, machine: machine.into(), jeton: jeton.into() };
            let envoyer = |c: machines::Connexion, o: Vec<u8>| tokio::task::spawn_blocking(move || machines::envoyer(&c, &o));
            let flux: Vec<u8> = (0..200_000u32).flat_map(|i| i.to_le_bytes()).collect();
            envoyer(vers(&id, &jeton), flux.clone()).await.unwrap().unwrap();
            assert_eq!(recu.join().unwrap(), flux, "flux passé tel quel à la machine");
            let e = envoyer(vers(&id, &"0".repeat(64)), b"x".to_vec()).await.unwrap().unwrap_err();
            assert!(e.contains("401") && e.contains("jeton_renvoi_requis"), "{e}");
            // Gros flux refusé d'emblée : le refus reste lisible, pas seulement « connexion rompue ».
            let e = envoyer(vers(&id, &"0".repeat(64)), vec![0; 64 << 20]).await.unwrap().unwrap_err();
            assert!(e.contains("jeton_renvoi_requis"), "{e}");
            let e = envoyer(vers("inconnue", &jeton), b"x".to_vec()).await.unwrap().unwrap_err();
            assert!(e.contains("404") && e.contains("introuvable"), "{e}");
            // Sortie en dossier : rien pour recevoir le flux.
            let dossier = b.machines.read().unwrap().iter().find(|m| m.connexion == machines::Connexion::Dossier).map(|m| m.id.clone());
            if let Some(d) = dossier {
                let e = envoyer(vers(&d, &jeton), b"x".to_vec()).await.unwrap().unwrap_err();
                assert!(e.contains("409") && e.contains("renvoi_sans_connexion"), "{e}");
            }

            // Machine de renvoi sur ce serveur : jeton masqué à la lecture, gardé à l'écriture.
            let corps = |r: Response| async move { serde_json::from_slice::<serde_json::Value>(&axum::body::to_bytes(r.into_body(), 1 << 20).await.unwrap()).unwrap() };
            b.machines.write().unwrap()[1].connexion = vers(&id, &jeton);
            let v = corps(appel(&b, "GET", "/api/machines", None, None).await).await;
            assert_eq!(v[1]["connexion"]["jeton"], machines::JETON_MASQUE);
            assert!(!v.to_string().contains(&jeton));
            assert_eq!(appel(&b, "POST", "/api/machines", None, Some(v)).await.status(), StatusCode::OK);
            assert_eq!(b.machines.read().unwrap()[1].connexion, vers(&id, &jeton), "jeton gardé");
            let e = envoyer(vers(&id, &jeton), b"x".to_vec()).await.unwrap().unwrap_err();
            assert!(e.contains("409") && e.contains("renvoi_en_chaine"), "pas de renvoi en chaîne : {e}");
            let mut ailleurs = corps(appel(&b, "GET", "/api/machines", None, None).await).await;
            ailleurs[1]["connexion"]["hote"] = "192.0.2.7".into();
            assert_eq!(appel(&b, "POST", "/api/machines", None, Some(ailleurs)).await.status(), StatusCode::BAD_REQUEST, "autre hôte : jeton à ressaisir");
            assert_eq!(b.machines.read().unwrap()[1].connexion, vers(&id, &jeton), "rien d'enregistré");
            std::fs::remove_dir_all(&b.config.donnees).unwrap();
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
            for (methode, chemin, corps) in [("POST", "/api/reglages", Some(reglage.clone())), ("DELETE", "/api/reglages/Vinyle", None), ("POST", "/api/tarifs", Some(serde_json::json!({}))), ("POST", "/api/consommation/reglages", Some(serde_json::json!({}))), ("POST", "/api/consommation/journal/purger", Some(serde_json::json!({ "jours": 0 })))] {
                assert_eq!(appel(&e, methode, chemin, Some(op), corps).await.status(), StatusCode::FORBIDDEN, "{methode} {chemin}");
            }
            assert_eq!(appel(&e, "GET", "/api/machines", Some(op), None).await.status(), StatusCode::OK, "lecture permise");
            assert_eq!(appel(&e, "GET", "/api/file", Some(op), None).await.status(), StatusCode::OK);
            assert_eq!(appel(&e, "GET", "/api/consommation/journal", Some(op), None).await.status(), StatusCode::OK, "le journal se lit");
            assert_eq!(appel(&e, "POST", "/api/reglages", Some(chef), Some(reglage)).await.status(), StatusCode::CREATED, "l'administrateur règle");
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// Pont ERP depuis Système : réglé par l'administrateur seul ; le secret n'est jamais
        /// renvoyé, ni par l'API ni par la sauvegarde ; l'instance suit la sauvegarde.
        #[tokio::test]
        async fn reglage_du_pont_depuis_systeme() {
            let (e, neuf) = (etat("pont-api"), etat("pont-api-neuf"));
            let compte = |nom: &str, role| acces::Utilisateur { nom: nom.into(), hachage: acces::hacher("phrase-de-passe-longue").unwrap(), role };
            acces::ecrire(&e.config.donnees, &[compte("chef", acces::Role::Admin), compte("op", acces::Role::Operateur)]).unwrap();
            let mut cookies = vec![];
            for nom in ["chef", "op"] {
                let r = appel(&e, "POST", "/api/session", None, Some(serde_json::json!({ "nom": nom, "mot_de_passe": "phrase-de-passe-longue" }))).await;
                cookies.push(r.headers()[header::SET_COOKIE].to_str().unwrap().split(';').next().unwrap().to_string());
            }
            let (chef, op) = (cookies[0].as_str(), cookies[1].as_str());
            // Secret factice des essais.
            let secret = "secret-factice-des-essais-0123456789abcdef";
            let url = "https://erp.exemple.test/api/v1/rip/atelier";
            let corps = |url: &str, actif: bool, secret: Option<&str>| Some(serde_json::json!({ "url": url, "actif": actif, "secret": secret }));
            assert_eq!(appel(&e, "POST", "/api/pont", Some(op), corps(url, true, Some(secret))).await.status(), StatusCode::FORBIDDEN);
            assert_eq!(appel(&e, "POST", "/api/pont/instance", Some(op), None).await.status(), StatusCode::FORBIDDEN);
            let code = |r: Response| async move { serde_json::from_slice::<serde_json::Value>(&axum::body::to_bytes(r.into_body(), 1 << 20).await.unwrap()).unwrap()["code"].clone() };
            assert_eq!(code(appel(&e, "POST", "/api/pont", Some(chef), corps("http://erp.exemple.test/rip", true, Some(secret))).await).await, "url_erp_invalide");
            assert_eq!(code(appel(&e, "POST", "/api/pont", Some(chef), corps(url, true, None)).await).await, "pont_incomplet");
            assert_eq!(code(appel(&e, "POST", "/api/pont", Some(chef), corps(url, true, Some("court"))).await).await, "secret_erp_invalide");
            let r = appel(&e, "POST", "/api/pont", Some(chef), corps(url, true, Some(secret))).await;
            assert_eq!(r.status(), StatusCode::OK);
            let texte = String::from_utf8(axum::body::to_bytes(r.into_body(), 1 << 20).await.unwrap().to_vec()).unwrap();
            assert!(!texte.contains(secret), "le secret n'est jamais renvoyé : {texte}");
            let v: serde_json::Value = serde_json::from_str(&texte).unwrap();
            assert_eq!((v["statut"].as_str(), v["secret_enregistre"].as_bool(), v["url"].as_str()), (Some("actif"), Some(true), Some(url)));
            // Secret laissé vide : gardé.
            assert_eq!(appel(&e, "POST", "/api/pont", Some(chef), corps(url, true, Some(""))).await.status(), StatusCode::OK);
            assert_eq!(erp::lire(&e.config.donnees).secret_hmac.as_deref(), Some(secret));
            let lu = String::from_utf8(axum::body::to_bytes(appel(&e, "GET", "/api/pont", Some(op), None).await.into_body(), 1 << 20).await.unwrap().to_vec()).unwrap();
            assert!(!lu.contains(secret) && lu.contains("\"actif\""), "{lu}");
            let avant = erp::lire(&e.config.donnees).instance;
            assert!(avant.is_some());
            assert_eq!(appel(&e, "POST", "/api/pont/instance", Some(chef), None).await.status(), StatusCode::OK);
            let instance = erp::lire(&e.config.donnees).instance;
            assert_ne!(instance, avant, "nouvelle instance");
            // Sauvegarde : URL et instance, jamais le secret ; reprises sur un serveur neuf.
            let s = String::from_utf8(axum::body::to_bytes(appel(&e, "GET", "/api/sauvegarde", Some(chef), None).await.into_body(), 1 << 26).await.unwrap().to_vec()).unwrap();
            assert!(!s.contains(secret), "jamais le secret dans la sauvegarde");
            let mut s: serde_json::Value = serde_json::from_str(&s).unwrap();
            assert_eq!(s["pont"]["instance"], serde_json::json!(instance));
            // Sans les réglages de découpe : leur restauration les installe pour tout le
            // processus, sous les pieds de l'essai des préfixes qui tourne en même temps.
            s["decoupe"] = serde_json::Value::Null;
            assert_eq!(appel(&neuf, "POST", "/api/sauvegarde", None, Some(s)).await.status(), StatusCode::OK);
            let c = erp::lire(&neuf.config.donnees);
            assert_eq!((c.instance, c.url.as_deref(), c.actif, c.secret_hmac), (instance, Some(url), false, None), "sans secret, le pont reste à activer");
            for x in [e, neuf] {
                std::fs::remove_dir_all(&x.config.donnees).unwrap();
            }
        }

        /// Roadmap 8.6 : import d'une bibliothèque Lab de 50 teintes, puis un doublon refuse
        /// tout l'import sans rien écrire ; un nom réservé est refusé.
        #[tokio::test]
        async fn tons_directs_import_csv() {
            let e = etat("tons-directs");
            let importer = |csv: String| {
                let r = Request::builder().method("POST").uri("/api/tons-directs/import").header(header::USER_AGENT, "Mozilla/5.0 EKO-RIP-Client/0.9").header(header::CONTENT_TYPE, "text/csv").body(Body::from(csv)).unwrap();
                routes(e.clone()).oneshot(r)
            };
            let csv: String = std::iter::once("Nom;L;a;b\n".to_string()).chain((0..50).map(|i| format!("HX {i:02};50;{i};-{i}\n"))).collect();
            let r = importer(csv).await.unwrap();
            assert_eq!(r.status(), StatusCode::OK);
            assert_eq!(super::couches_decoupe::json(r).await, serde_json::json!({ "ajoutes": 50, "total": 50 }));
            let r = importer("HX 99;50;0;0\nHX 07;40;0;0\n".into()).await.unwrap();
            assert_eq!(r.status(), StatusCode::BAD_REQUEST);
            let r = super::couches_decoupe::json(r).await;
            assert_eq!((r["code"].as_str(), r["detail"].as_str()), (Some("nom_en_double"), Some("HX 07")));
            assert_eq!(tons::lire(&e.config.donnees).len(), 50, "rien d'écrit");
            let r = super::couches_decoupe::json(importer("Black;20;0;0".into()).await.unwrap()).await;
            assert_eq!(r["code"], "nom_reserve");
            let lu = super::couches_decoupe::json(appel(&e, "GET", "/api/tons-directs", None, None).await).await;
            assert_eq!((lu["intention"].clone(), lu["tons"][7]["lab"].clone()), (serde_json::json!(1), serde_json::json!([50.0, 7.0, -7.0])));
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// Roadmap 9.1 : `BACHE_*.pdf` → réglage Bâche ; un grand format → réglage XXL ; un
        /// fichier qu'aucune règle ne prend est refusé ; une règle vers un réglage inconnu aussi.
        #[tokio::test]
        async fn regles_du_dossier_automatique() {
            use lopdf::{Document, Object, dictionary};
            let e = etat("regles");
            let d = &e.config.donnees;
            let reglage = |nom: &str| reglages::Reglage { uuid: None, nom: nom.into(), demande: Demande::default() };
            reglages::ecrire(d, &[reglage("Bâche"), reglage("XXL"), reglage("Vinyle")]).unwrap();
            let regles = serde_json::json!([
                { "motif": "BACHE_*", "reglage": "Bâche" },
                { "motif": "*.pdf", "grand_cote_mm": [2000, 100000], "reglage": "XXL" },
                { "motif": "*.pdf", "reglage": "Vinyle" },
            ]);
            assert_eq!(appel(&e, "POST", "/api/regles", None, Some(regles)).await.status(), StatusCode::OK);
            let faux = serde_json::json!([{ "motif": "*", "reglage": "Inconnu" }]);
            assert_eq!(super::couches_decoupe::json(appel(&e, "POST", "/api/regles", None, Some(faux)).await).await["code"], "regles_invalides");
            assert_eq!(super::couches_decoupe::json(appel(&e, "GET", "/api/regles", None, None).await).await[1]["grand_cote_mm"], serde_json::json!([2000.0, 100000.0]));
            let pdf = |l: f32, h: f32| {
                let mut doc = Document::with_version("1.7");
                let pages = doc.new_object_id();
                let page = doc.add_object(dictionary! { "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(), 0.into(), Object::Real(l), Object::Real(h)] });
                doc.objects.insert(pages, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }));
                let c = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
                doc.trailer.set("Root", c);
                let mut v = vec![];
                doc.save_to(&mut v).unwrap();
                v
            };
            let auto = regles::dossier(d);
            std::fs::create_dir_all(&auto).unwrap();
            let liste = reglages::lire(d);
            let essai = |nom: &str, octets: Vec<u8>| {
                let chemin = auto.join(nom);
                std::fs::write(&chemin, octets).unwrap();
                let liste = liste.clone();
                async move { reglage_par_regle(d, &chemin, &liste).await.map(|r| r.nom.clone()) }
            };
            assert_eq!(essai("BACHE_client-12.pdf", pdf(8504.0, 2835.0)).await.unwrap(), "Bâche", "le motif passe avant le format");
            assert_eq!(essai("affiche.pdf", pdf(2835.0, 8504.0)).await.unwrap(), "XXL", "3 m de haut");
            assert_eq!(essai("a3.pdf", pdf(842.0, 1191.0)).await.unwrap(), "Vinyle");
            assert!(essai("photo.jpg", vec![0xff, 0xd8, 0xff]).await.unwrap_err().starts_with("aucune_regle"));
            regles::ecrire(d, &[regles::Regle { motif: "*".into(), petit_cote_mm: None, grand_cote_mm: None, reglage: "Supprimé".into() }]).unwrap();
            assert!(essai("x.pdf", pdf(100.0, 100.0)).await.unwrap_err().contains("Supprimé"));
            std::fs::remove_dir_all(d).unwrap();
        }

        /// Roadmap 9.10 : `Affiche__epson__bache__30.pdf` → réglage Bâche, 30 copies, Epson ;
        /// un nom hors motif passe par les règles avec une alerte ; une balise fausse aussi.
        #[tokio::test]
        async fn motif_de_nom_a_balises() {
            use lopdf::{Document, Object, dictionary};
            let e = etat("balises");
            let d = &e.config.donnees;
            let reglage = |nom: &str| reglages::Reglage { uuid: None, nom: nom.into(), demande: Demande::default() };
            reglages::ecrire(d, &[reglage("Bâche"), reglage("Vinyle")]).unwrap();
            let json = |r: Response| super::couches_decoupe::json(r);
            let faux = serde_json::json!({ "motif": "{nom}{copies}" });
            assert_eq!(json(appel(&e, "POST", "/api/balises", None, Some(faux)).await).await["code"], "motif_balises_invalide");
            let motif = serde_json::json!({ "motif": "{nom}__{machine}__{reglage}__{copies}" });
            assert_eq!(appel(&e, "POST", "/api/balises", None, Some(motif.clone())).await.status(), StatusCode::OK);
            assert_eq!(json(appel(&e, "GET", "/api/balises", None, None).await).await, motif);
            // A3 portrait : 297 × 420 mm.
            let mut doc = Document::with_version("1.7");
            let pages = doc.new_object_id();
            let page = doc.add_object(dictionary! { "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(), 0.into(), Object::Real(841.89), Object::Real(1190.55)] });
            doc.objects.insert(pages, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }));
            let c = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
            doc.trailer.set("Root", c);
            let mut a3 = vec![];
            doc.save_to(&mut a3).unwrap();
            let auto = regles::dossier(d);
            std::fs::create_dir_all(&auto).unwrap();
            let liste = reglages::lire(d);
            let essai = |nom: &str| {
                let chemin = auto.join(nom);
                std::fs::write(&chemin, &a3).unwrap();
                let (liste, e) = (liste.clone(), e.clone());
                async move { preparer_auto(&e, &chemin, &liste).await }
            };
            let codes = |a: &[controle::Alerte]| a.iter().map(|x| x.code.clone()).collect::<Vec<_>>();
            let (nom, demande, fiche, alertes) = essai("Affiche__epson__bache__30.pdf").await.unwrap();
            assert_eq!((nom.as_str(), demande.imprimante.as_deref(), fiche.quantite), ("Bâche", Some("epson"), 30));
            assert!(alertes.is_empty(), "{alertes:?}");
            // Hors motif, sans règle : refusé, la raison dit pourquoi.
            let refus = essai("photo.pdf").await.unwrap_err();
            assert!(refus.starts_with("aucune_regle") && refus.contains("nom_non_conforme"), "{refus}");
            regles::ecrire(d, &[regles::Regle { motif: "*".into(), petit_cote_mm: None, grand_cote_mm: None, reglage: "Vinyle".into() }]).unwrap();
            let (nom, _, _, alertes) = essai("photo.pdf").await.unwrap();
            assert_eq!((nom.as_str(), codes(&alertes)), ("Vinyle", vec!["nom_non_conforme".to_string()]), "importé avec une alerte");
            // Réglage inconnu → règles ; copies fausses ; une découpeuse en machine.
            let (nom, demande, fiche, alertes) = essai("Affiche__kona__inconnu__abc.pdf").await.unwrap();
            assert_eq!((nom.as_str(), demande.decoupeuse.as_deref(), demande.imprimante, fiche.quantite), ("Vinyle", Some("kona"), None, 0));
            assert_eq!(alertes.iter().map(|a| a.detail.as_str()).collect::<Vec<_>>(), ["reglage : « inconnu »", "copies : « abc »"]);
            // Largeur voulue : l'échelle en découle ; avec une hauteur qui ne s'accorde pas, la
            // plus petite gagne et le dit.
            balises::ecrire(d, &balises::Config { motif: "{nom}__{largeur}__{hauteur}__{repetition}".into() }).unwrap();
            let (_, demande, _, alertes) = essai("Plan__594mm__840__3.pdf").await.unwrap();
            assert_eq!((demande.edition.echelle, demande.repetition), (200.0, 3));
            assert!(alertes.is_empty(), "{alertes:?}");
            let (_, demande, _, alertes) = essai("Plan__594__420__1.pdf").await.unwrap();
            assert_eq!((demande.edition.echelle, codes(&alertes)), (100.0, vec!["balise_proportions".to_string()]));
            // La répétition met autant d'impressions de chaque page en file, sur l'imprimante dite.
            let id = Uuid::new_v4();
            let t = serde_json::from_value::<Travail>(serde_json::json!({ "id": id, "nom": "Plan", "statut": "termine", "erreur": null, "cree_le": 0,
                "auto": { "imprimer": true }, "repetition": 3, "imprimante": "epson",
                "pages": [{ "plaques": ["Cyan"], "traces_decoupe": 0, "largeur_mm": 100.0, "hauteur_mm": 100.0 }] })).unwrap();
            mettre_en_file_auto(&e, &t, None);
            let sorties = e.file.lock().unwrap().sorties.clone();
            assert_eq!(sorties.iter().map(|s| s.machine.as_str()).collect::<Vec<_>>(), ["epson"; 3]);
            // Deux supports du même nom : celui de l'imprimante dite gagne, même si `{machine}`
            // vient après `{support}` ; un support inconnu le dit.
            let support = |machine: &str| supports::Support { id: Some(Uuid::new_v4()), machine: machine.into(), laize_mm: None, numero_machine: None, nom: "Bâche".into(), encres: vec![], modes: vec![] };
            let (ailleurs, ici) = (support("autre"), support("epson"));
            supports::ecrire(d, &supports::Catalogue { version: supports::VERSION, supports: vec![ailleurs, ici.clone()] }).unwrap();
            balises::ecrire(d, &balises::Config { motif: "{nom}__{support}__{machine}".into() }).unwrap();
            let (_, demande, _, alertes) = essai("Affiche__bache__epson.pdf").await.unwrap();
            assert_eq!((demande.support_uuid, demande.imprimante.as_deref()), (ici.id, Some("epson")));
            assert!(alertes.is_empty(), "{alertes:?}");
            let (_, _, _, alertes) = essai("Affiche__papier__epson.pdf").await.unwrap();
            assert_eq!(alertes.iter().map(|a| a.detail.as_str()).collect::<Vec<_>>(), ["support : « papier »"]);
            // Avec une découpe : une par impression répétée, chacune avec ses repères.
            e.file.lock().unwrap().sorties.clear();
            let t = serde_json::from_value::<Travail>(serde_json::json!({ "id": id, "nom": "Plan", "statut": "termine", "erreur": null, "cree_le": 0,
                "auto": { "imprimer": true, "decouper": true }, "repetition": 3, "imprimante": "epson", "decoupeuse": "kona",
                "pages": [{ "plaques": ["Cyan"], "traces_decoupe": 1, "largeur_mm": 100.0, "hauteur_mm": 100.0 }] })).unwrap();
            mettre_en_file_auto(&e, &t, None);
            let ops = e.file.lock().unwrap().sorties.iter().map(|s| s.operation()).collect::<Vec<_>>();
            assert_eq!((ops.iter().filter(|o| **o == machines::Operation::Impression).count(), ops.iter().filter(|o| **o == machines::Operation::Decoupe).count()), (3, 3));
            std::fs::remove_dir_all(d).unwrap();
        }

        /// Roadmap 8.11 : préréglages listés, mire CGATS, limite proposée, et un mode qui porte
        /// sa bascule et son encre totale.
        #[tokio::test]
        async fn separation_par_mode_et_linearisation() {
            let e = etat("linearisation");
            let b = super::couches_decoupe::json(appel(&e, "GET", "/api/separation/bascules", None, None).await).await;
            assert_eq!((b[0]["nom"].as_str(), b[2]["couple"]["max"].as_f64()), (Some("douce"), Some(0.6f32 as f64)));
            let r = appel(&e, "GET", "/api/linearisation/mire.cgats?n=21", None, None).await;
            assert_eq!(r.status(), StatusCode::OK);
            let t = String::from_utf8(axum::body::to_bytes(r.into_body(), 1 << 20).await.unwrap().to_vec()).unwrap();
            assert!(t.contains("NUMBER_OF_SETS 189"), "9 encres × 21 paliers");
            assert_eq!(appel(&e, "GET", "/api/linearisation/mire.cgats?n=3", None, None).await.status(), StatusCode::BAD_REQUEST);
            let mesures: Vec<(f64, f64)> = (0..=15).map(|i| (100.0 + 20.0 * i as f64, 18.0 + 40.0 * (-(20.0 * i as f64) / 70.0).exp())).collect();
            let p = super::couches_decoupe::json(appel(&e, "POST", "/api/linearisation/limite-proposee", None, Some(serde_json::json!({ "mesures": mesures }))).await).await;
            assert_eq!(p["limite"], 220.0);
            let r = super::couches_decoupe::json(appel(&e, "POST", "/api/linearisation/limite-proposee", None, Some(serde_json::json!({ "mesures": [[100, 50]] }))).await).await;
            assert_eq!(r["code"], "mesures_invalides");
            // Le mode garde sa bascule et son encre totale ; hors bornes, refusé.
            let mut c = supports::lire(&e.config.donnees, "epson");
            c.supports[0].modes[0].bascule = Some(separation::Bascule::Douce);
            c.supports[0].modes[0].limite_totale = Some(260.0);
            let r = appel(&e, "POST", "/api/machines/epson/supports", None, Some(serde_json::json!(c.supports))).await;
            assert_eq!(r.status(), StatusCode::OK);
            let lu = supports::lire(&e.config.donnees, "epson");
            assert_eq!((lu.supports[0].modes[0].bascule, lu.supports[0].modes[0].limite_totale), (Some(separation::Bascule::Douce), Some(260.0)));
            c.supports[0].modes[0].limite_totale = Some(500.0);
            assert_eq!(appel(&e, "POST", "/api/machines/epson/supports", None, Some(serde_json::json!(c.supports))).await.status(), StatusCode::BAD_REQUEST);
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// Roadmap 8.10 : mire G7, itération mesurée enregistrée, meilleure donnée.
        #[tokio::test]
        async fn etalonnage_g7_par_l_api() {
            let e = etat("g7");
            let r = appel(&e, "GET", "/api/g7/mire.cgats?n=11", None, None).await;
            assert_eq!(r.status(), StatusCode::OK);
            let t = String::from_utf8(axum::body::to_bytes(r.into_body(), 1 << 20).await.unwrap().to_vec()).unwrap();
            assert!(t.contains("NUMBER_OF_SETS 22"));
            // Imprimante qui suit déjà la cible : écarts nuls, première itération = meilleure.
            let mesures: Vec<serde_json::Value> = (0..=10).flat_map(|i| {
                let l = 95.0 - 7.5 * i as f64;
                ["cmy", "k"].map(|r| serde_json::json!({ "rampe": r, "pct": i * 10, "l": l, "a": 0.0, "b": 0.0 }))
            }).collect();
            let v = super::couches_decoupe::json(appel(&e, "POST", "/api/g7/iterations", None, Some(serde_json::json!({ "nom": "Epson / bâche", "mesures": mesures }))).await).await;
            assert_eq!((v["meilleure"].as_u64(), v["etalonnage"]["iterations"][0]["courbes"]["k"].as_array().map(Vec::len)), (Some(1), Some(21)));
            let liste = super::couches_decoupe::json(appel(&e, "GET", "/api/g7", None, None).await).await;
            assert_eq!(liste[0]["etalonnage"]["nom"], "Epson / bâche");
            let r = super::couches_decoupe::json(appel(&e, "POST", "/api/g7/iterations", None, Some(serde_json::json!({ "nom": "x", "mesures": [] }))).await).await;
            assert_eq!(r["code"], "mesures_invalides");
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// Sauvegarde puis restauration sur un serveur neuf ; une sauvegarde fausse n'écrit rien.
        #[tokio::test]
        async fn sauvegarde_et_restauration_de_la_configuration() {
            let _g = DECOUPE_GLOBALE.lock().await;
            let (a, b) = (etat("sauve-a"), etat("sauve-b"));
            let icc = std::fs::read(std::fs::read_dir(couleur::tests::profils_ghostscript()).unwrap().flatten().map(|e| e.path()).find(|p| p.extension().is_some_and(|x| x == "icc")).unwrap()).unwrap();
            std::fs::create_dir_all(&a.config.profils).unwrap();
            std::fs::write(a.config.profils.join("atelier.icc"), &icc).unwrap();
            let reglage = serde_json::json!({ "nom": "Vinyle", "demande": { "dpi": 360 } });
            assert_eq!(appel(&a, "POST", "/api/reglages", None, Some(reglage)).await.status(), StatusCode::CREATED);
            let tons = serde_json::json!({ "intention": 3, "tons": [{ "nom": "HX Rouge", "alias": ["Rouge HX"], "lab": [45.0, 60.0, 40.0] }] });
            assert_eq!(appel(&a, "POST", "/api/tons-directs", None, Some(tons)).await.status(), StatusCode::OK);
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
            // Préfixes de découpe : repris par la sauvegarde (réglages d'origine ici, pour ne pas
            // changer les réglages globaux des autres tests).
            assert!(decoupe::fichier(&b.config.donnees).exists(), "decoupe.json restauré");
            assert_eq!(decoupe::lire(&b.config.donnees), decoupe::lire(&a.config.donnees));
            assert_eq!(tons::lire_table(&b.config.donnees), tons::lire_table(&a.config.donnees), "tons directs et intention repris");
            assert_eq!(tons::lire_table(&b.config.donnees).intention, 3);
            assert_eq!(tons::lire(&b.config.donnees).len(), 1);
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
            erp::ecrire(&e.config.donnees, &erp::Config { jeton_sha256: Some(erp::empreinte(&jeton)), ..Default::default() }).unwrap();
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

        /// Seuil d'espace libre (roadmap 0.6) : refus anticipé, ordre de purge ; deux seuils
        /// (0.8) : alerte sous le bas, dossier surveillé en erreur sous le critique.
        #[tokio::test]
        async fn seuil_d_espace_libre() {
            assert_eq!([niveau_disque(5, 10, 3), niveau_disque(2, 10, 3), niveau_disque(20, 10, 3), niveau_disque(0, 0, 0)], ["bas", "critique", "ok", "ok"]);
            assert_eq!(place_estimee(1.0, 600), 1_000_000_000);
            assert_eq!(place_estimee(1.0, 300), 250_000_000, "au carré de la résolution");
            let t = |cree_le: u64, statut: &str| serde_json::from_value::<Travail>(serde_json::json!({ "id": Uuid::new_v4(), "nom": "x", "statut": statut, "erreur": null, "cree_le": cree_le, "pages": [] })).unwrap();
            let mut v = [t(30, "termine"), t(10, "en_cours"), t(20, "echec"), t(40, "termine"), t(5, "echec")];
            v[4].code_erreur = Some("espace_insuffisant".into());
            assert_eq!(plus_vieux_inactifs(&v), [v[2].id, v[0].id, v[3].id], "du plus vieux au plus récent, jamais un actif ni un dépôt en attente de place");
            // Seuil impossible à tenir : l'envoi est refusé avant tout rendu.
            let e = etat("seuil");
            let mut config = e.config.clone();
            config.seuil_libre = u64::MAX / 2;
            let e2: Partage = Arc::new(Etat { config, rendu: rendus::Rendus::new(1), file: Default::default(), reglages: Default::default(), sessions: Default::default(), machines: std::sync::RwLock::new(e.machines.read().unwrap().clone()) });
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
            std::fs::write(rep.join("source.tmp"), &pdf).unwrap();
            let r = creer_travail(&e2, id, "x.pdf".into(), Format::Pdf, Demande::default()).await;
            assert!(matches!(r, Err((StatusCode::INSUFFICIENT_STORAGE, ("espace_insuffisant", _)))), "{:?}", r.err().map(|e| e.1));
            let deposer = || {
                let id = Uuid::new_v4();
                std::fs::create_dir_all(dossier(&e2.config.donnees, id)).unwrap();
                std::fs::write(dossier(&e2.config.donnees, id).join("source.tmp"), &pdf).unwrap();
                id
            };
            // Dépôt en dossier surveillé : pas de refus, un travail en erreur, sans rendu.
            let id = deposer();
            let t = creer_travail_avec(&e2, id, "auto.pdf".into(), Format::Pdf, Demande::default(), Fiche::default(), Origine::Dossier, vec![]).await.unwrap_or_else(|(s, r)| panic!("refusé : {s} {r:?}"));
            assert!(matches!(t.statut, Statut::Echec) && t.code_erreur.as_deref() == Some("espace_insuffisant") && t.erreur.as_deref().is_some_and(|e| e.contains("Mo libres")));
            assert!(matches!(lire_travail(&e2.config.donnees, id).map(|t| t.statut), Some(Statut::Echec)), "gardé, à relancer");
            // Sous le seul seuil bas : accepté, avec l'alerte.
            let mut config = e2.config.clone();
            (config.seuil_libre, config.seuil_alerte) = (0, u64::MAX / 2);
            let e3: Partage = Arc::new(Etat { config, rendu: rendus::Rendus::new(1), file: Default::default(), reglages: Default::default(), sessions: Default::default(), machines: std::sync::RwLock::new(e.machines.read().unwrap().clone()) });
            let id = deposer();
            let t = creer_travail(&e3, id, "bas.pdf".into(), Format::Pdf, Demande { mode: Mode::Imprimer, dpi: 36, ..Default::default() }).await.unwrap_or_else(|(s, r)| panic!("refusé : {s} {r:?}"));
            assert!(matches!(t.statut, Statut::EnAttente) && t.alertes.iter().any(|a| a.code == "espace_disque_bas"), "{:?}", t.alertes.iter().map(|a| &a.code).collect::<Vec<_>>());
            std::fs::remove_dir_all(&e.config.donnees).ok();
        }

        /// Option de purge des travaux de l'ERP faits (roadmap 0.8).
        #[test]
        fn travaux_erp_faits() {
            let t = |erp: bool, statut: &str| serde_json::from_value::<Travail>(serde_json::json!({ "id": Uuid::new_v4(), "nom": "x", "statut": statut, "erreur": null, "cree_le": 0, "pages": [], "erp": erp.then(|| serde_json::json!({ "envoi": Uuid::new_v4(), "ref_erp": "PROD-2026-00123" })) })).unwrap();
            let v = [t(true, "termine"), t(true, "termine"), t(true, "termine"), t(false, "termine"), t(true, "termine"), t(true, "echec"), t(true, "termine")];
            let s = |i: usize, op, etat| file::Sortie { etat, ..file::Sortie::nouvelle("epson", op, v[i].id, "x", 1, 0) };
            use machines::Operation::{Decoupe, Impression};
            let sorties = [
                s(0, Impression, file::Etat::Terminee),
                s(0, Decoupe, file::Etat::Terminee),
                s(1, Impression, file::Etat::Terminee),
                s(1, Decoupe, file::Etat::Suspendue),
                s(2, Impression, file::Etat::Terminee),
                s(2, Impression, file::Etat::Echec),
                s(3, Impression, file::Etat::Terminee),
                s(5, Impression, file::Etat::Terminee),
                s(6, Impression, file::Etat::Terminee),
                s(6, Impression, file::Etat::Imbriquee),
            ];
            assert_eq!(erp_faits(&v, &sorties), [v[0].id], "découpe à passer, page en échec, hors ERP, jamais imprimé, rendu en échec, page sur une planche : gardés");
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
            assert_eq!(raison_attente(&d, &s, None, None, &[], &[]), Some("machine_hors_ligne"), "machine absente du catalogue");
            assert_eq!(raison_attente(&d, &s, Some(&m(1650.0, "/dev/inexistant")), None, &[], &[]), Some("machine_hors_ligne"));
            assert_eq!(raison_attente(&d, &s, Some(&m(1650.0, p)), None, &[], &[]), Some("source_absente"), "pas de tracé");
            std::fs::write(fichier_traces(&rep, 1), b"[]").unwrap();
            assert_eq!(raison_attente(&d, &s, Some(&m(1650.0, p)), None, &[], &[]), None);
            assert_eq!(raison_attente(&d, &s, Some(&m(400.0, p)), None, &[], &[]), Some("format_trop_petit"));
            s.etat = file::Etat::Suspendue;
            assert_eq!(raison_attente(&d, &s, Some(&m(1650.0, p)), None, &[], &[]), Some("attente_operateur"));
            s.etat = file::Etat::Terminee;
            assert_eq!(raison_attente(&d, &s, None, None, &[], &[]), None, "une sortie finie n'attend rien");
            let mut f = file::File::default();
            let mut bloquee = file::Sortie::nouvelle("kona", machines::Operation::Decoupe, Uuid::new_v4(), "absente", 1, 0);
            bloquee.etat = file::Etat::EnAttente;
            let prete = file::Sortie::nouvelle("kona", machines::Operation::Decoupe, id, "essai", 1, 0);
            f.sorties = vec![bloquee, prete];
            let machine = m(1650.0, p);
            assert_eq!(f.prochaine_si("kona", |x| raison_attente(&d, x, Some(&machine), None, &[], &[]).is_none()), Some(1), "la bloquée garde sa place, la suivante part");
            // Attente de support (roadmap 1.4) : impression d'un travail sur le support A.
            let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
            let mut tr = lire_travail(&d, id).unwrap();
            tr.demande.support_uuid = Some(a);
            ecrire(&d, &tr).unwrap();
            let epson = machines::Machine { connexion: machines::Connexion::Dossier, ..m(1650.0, p) };
            let imp = file::Sortie::nouvelle("epson", machines::Operation::Impression, id, "essai", 1, 0);
            let charge = |u: Uuid, laize: Option<f64>| supports::Charge { support_uuid: Some(u), laize_mm: laize, source: "machine".into(), le: 0 };
            assert_eq!(raison_attente(&d, &imp, Some(&epson), Some(&charge(b, None)), &[], &[]), Some("attente_support"), "B chargé, A voulu");
            assert_eq!(raison_attente(&d, &imp, Some(&epson), Some(&charge(a, None)), &[], &[]), None, "A chargé : elle part");
            assert_eq!(raison_attente(&d, &imp, Some(&epson), Some(&charge(a, Some(400.0))), &[], &[]), Some("format_trop_petit"), "rouleau chargé de 400 mm pour une page de 500");
            assert_eq!(raison_attente(&d, &imp, Some(&epson), None, &[], &[]), None, "support chargé inconnu : on n'arrête pas l'atelier");
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

        /// Pool (roadmap 1.7) : 2 machines en dossier, répartition ; supports différents refusés.
        #[tokio::test]
        async fn pool_d_imprimantes() {
            let e = etat("pool");
            let donnees = e.config.donnees.clone();
            let base = e.machines.read().unwrap()[0].clone();
            let mut cat = vec![
                machines::Machine { id: "epson-1".into(), uuid: None, pool: Some("epsons".into()), ..base.clone() },
                machines::Machine { id: "epson-2".into(), uuid: None, pool: Some("epsons".into()), ..base.clone() },
            ];
            assert_eq!(machines::verifier(&cat), Ok(()));
            let mut faux = cat.clone();
            faux[1].laize_mm = 1000.0;
            assert_eq!(machines::verifier(&faux), Err("pool_incoherent"), "laizes différentes");
            faux[1] = machines::Machine { id: "epsons".into(), pool: None, ..base.clone() };
            assert_eq!(machines::verifier(&faux), Err("pool_incoherent"), "pool au nom d'une machine");
            machines::identifier(&mut cat, &[]).unwrap();
            *e.machines.write().unwrap() = cat;
            // Deux sorties envoyées au pool : chaque machine en prend une.
            let mut f = file::File::default();
            for n in 0..2 {
                f.sorties.push(file::Sortie::nouvelle("epsons", machines::Operation::Impression, Uuid::new_v4(), &format!("t{n}"), 1, 0));
            }
            let i1 = f.prochaine_dans("epson-1", &["epsons"], |_| true).unwrap();
            (f.sorties[i1].etat, f.sorties[i1].machine) = (file::Etat::EnCours, "epson-1".into());
            assert_eq!(f.prochaine_dans("epson-1", &["epsons"], |_| true), None, "une à la fois par machine");
            let i2 = f.prochaine_dans("epson-2", &["epsons"], |_| true).unwrap();
            assert_ne!(i1, i2, "la seconde part sur l'autre machine");
            assert_eq!(f.prochaine_dans("autre", &[], |_| true), None, "hors du pool : rien");
            // Supports chargés différents : refus d'entrer dans le pool.
            let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
            for (m, u) in [("epson-1", a), ("epson-2", b)] {
                supports::ecrire_charge(&donnees, m, supports::Charge { support_uuid: Some(u), laize_mm: None, source: "saisie".into(), le: 0 }).unwrap();
            }
            let id = Uuid::new_v4();
            std::fs::create_dir_all(dossier(&donnees, id)).unwrap();
            let t = serde_json::from_value::<Travail>(serde_json::json!({ "id": id, "nom": "x", "statut": "termine", "erreur": null, "cree_le": 0,
                "pages": [{ "plaques": ["Cyan"], "traces_decoupe": 0, "largeur_mm": 100.0, "hauteur_mm": 100.0 }] })).unwrap();
            ecrire(&donnees, &t).unwrap();
            let r = appel(&e, "POST", "/api/file", None, Some(serde_json::json!({ "travail": id, "page": 1, "machine": "epsons", "operation": "impression" }))).await;
            assert_eq!(r.status(), StatusCode::CONFLICT);
            let v: serde_json::Value = serde_json::from_slice(&axum::body::to_bytes(r.into_body(), 1 << 20).await.unwrap()).unwrap();
            assert_eq!(v["code"], "support_different");
            supports::ecrire_charge(&donnees, "epson-2", supports::Charge { support_uuid: Some(a), laize_mm: None, source: "saisie".into(), le: 0 }).unwrap();
            let r = appel(&e, "POST", "/api/file", None, Some(serde_json::json!({ "travail": id, "page": 1, "machine": "epsons", "operation": "impression" }))).await;
            assert_eq!(r.status(), StatusCode::CREATED, "même support : accepté");
            std::fs::remove_dir_all(&donnees).unwrap();
        }

        /// Épreuve sur une 2e machine (roadmap 9.5) : la copie « -BAT » part sur l'autre
        /// imprimante, l'original attend suspendu ; refusée vers la même machine, une
        /// machine qui n'imprime pas, ou pour une découpe. « Reprendre » libère l'original.
        #[tokio::test]
        async fn epreuve_sur_une_autre_machine() {
            let e = etat("epreuve");
            let donnees = e.config.donnees.clone();
            let base = e.machines.read().unwrap()[0].clone();
            let kona = e.machines.read().unwrap()[1].clone();
            let mut cat = vec![machines::Machine { id: "grande".into(), uuid: None, ..base.clone() }, machines::Machine { id: "petite".into(), uuid: None, laize_mm: 600.0, ..base.clone() }, kona.clone()];
            machines::identifier(&mut cat, &[]).unwrap();
            *e.machines.write().unwrap() = cat;
            let id = Uuid::new_v4();
            std::fs::create_dir_all(dossier(&donnees, id)).unwrap();
            let t = serde_json::from_value::<Travail>(serde_json::json!({ "id": id, "nom": "Vitrine", "statut": "termine", "erreur": null, "cree_le": 0,
                "pages": [{ "plaques": ["Cyan"], "traces_decoupe": 0, "largeur_mm": 100.0, "hauteur_mm": 100.0 }] })).unwrap();
            ecrire(&donnees, &t).unwrap();
            let demande = |machine: &str, epreuve: &str, operation: &str| Some(serde_json::json!({ "travail": id, "page": 1, "machine": machine, "operation": operation, "epreuve": epreuve }));
            let code = |r: Response| async move { serde_json::from_slice::<serde_json::Value>(&axum::body::to_bytes(r.into_body(), 1 << 20).await.unwrap()).unwrap()["code"].clone() };
            for (machine, epreuve, operation, statut, attendu) in [
                ("grande", "grande", "impression", StatusCode::BAD_REQUEST, "epreuve_impossible"),
                ("grande", kona.id.as_str(), "impression", StatusCode::BAD_REQUEST, "epreuve_impossible"),
                ("grande", "absente", "impression", StatusCode::NOT_FOUND, "introuvable"),
            ] {
                let r = appel(&e, "POST", "/api/file", None, demande(machine, epreuve, operation)).await;
                assert_eq!(r.status(), statut, "{epreuve}");
                assert_eq!(code(r).await, attendu);
            }
            assert!(e.file.lock().unwrap().sorties.is_empty(), "rien en file après un refus");
            let r = appel(&e, "POST", "/api/file", None, demande("grande", "petite", "impression")).await;
            assert_eq!(r.status(), StatusCode::CREATED);
            let sorties = e.file.lock().unwrap().sorties.clone();
            assert_eq!(sorties.len(), 2);
            let (original, bat) = (&sorties[0], &sorties[1]);
            assert_eq!((original.machine.as_str(), original.etat, original.nom.as_str()), ("grande", file::Etat::Suspendue, "Vitrine"));
            assert_eq!((bat.machine.as_str(), bat.etat, bat.nom.as_str(), bat.bat_de), ("petite", file::Etat::EnAttente, "Vitrine-BAT", Some(original.id)));
            // BAT accepté : l'original part.
            assert_eq!(appel(&e, "POST", &format!("/api/file/{}/reprendre", original.id), None, None).await.status(), StatusCode::NO_CONTENT);
            assert_eq!(e.file.lock().unwrap().sorties[0].etat, file::Etat::EnAttente);
            // Sans épreuve (champ vide) : comme avant.
            assert_eq!(appel(&e, "POST", "/api/file", None, demande("petite", "", "impression")).await.status(), StatusCode::CREATED);
            assert_eq!(e.file.lock().unwrap().sorties.len(), 3);
            std::fs::remove_dir_all(&donnees).unwrap();
        }

        /// Compteurs de file (roadmap 9.6) : poids de chaque sortie à passer, tampon total.
        #[tokio::test]
        async fn compteurs_de_file() {
            let e = etat("compteurs");
            let donnees = e.config.donnees.clone();
            let machine = e.machines.read().unwrap()[0].id.clone();
            let id = Uuid::new_v4();
            let rep = dossier(&donnees, id);
            std::fs::create_dir_all(rep.join("prod")).unwrap();
            // Le ton de découpe n'est jamais envoyé : il ne pèse pas.
            for (plaque, taille) in [("Cyan", 3000), ("Magenta", 1000), ("CutContour", 500)] {
                std::fs::write(render::chemin_plaque(&rep, 1, plaque), vec![0u8; taille]).unwrap();
            }
            std::fs::write(render::chemin_plaque(&rep, 2, "Magenta"), vec![0u8; 700]).unwrap();
            let t = serde_json::from_value::<Travail>(serde_json::json!({ "id": id, "nom": "Vitrine", "statut": "termine", "erreur": null, "cree_le": 0,
                "pages": [{ "plaques": ["Cyan", "Magenta", "CutContour"], "plaques_decoupe": ["CutContour"], "traces_decoupe": 0, "largeur_mm": 100.0, "hauteur_mm": 100.0 },
                    { "plaques": ["Magenta"], "traces_decoupe": 0, "largeur_mm": 100.0, "hauteur_mm": 100.0 }] })).unwrap();
            ecrire(&donnees, &t).unwrap();
            let corps = |r: Response| async move { serde_json::from_slice::<serde_json::Value>(&axum::body::to_bytes(r.into_body(), 1 << 20).await.unwrap()).unwrap() };
            let tampon = || async { corps(appel(&e, "GET", "/api/systeme", None, None).await).await["tampon"].clone() };
            assert_eq!(tampon().await, 0, "file vide");
            for page in [1, 2] {
                let r = appel(&e, "POST", "/api/file", None, Some(serde_json::json!({ "travail": id, "page": page, "machine": machine, "operation": "impression" }))).await;
                assert_eq!(r.status(), StatusCode::CREATED);
            }
            let file = corps(appel(&e, "GET", "/api/file", None, None).await).await;
            assert_eq!(file.as_array().unwrap().iter().map(|s| s["poids"].clone()).collect::<Vec<_>>(), [4000, 700]);
            assert_eq!(tampon().await, 4700);
            // Une sortie annulée quitte le tampon.
            let premiere = e.file.lock().unwrap().sorties[0].id;
            assert_eq!(appel(&e, "POST", &format!("/api/file/{premiere}/annuler"), None, None).await.status(), StatusCode::NO_CONTENT);
            let file = corps(appel(&e, "GET", "/api/file", None, None).await).await;
            assert_eq!(file[0]["poids"], serde_json::Value::Null);
            assert_eq!(tampon().await, 700);
            std::fs::remove_dir_all(&donnees).unwrap();
        }

        /// Import de réglages choisis (roadmap 1.6) : doublon écrasé ou renommé.
        #[tokio::test]
        async fn import_de_reglages_ecraser_ou_renommer() {
            assert_eq!(reglages::nom_libre("Vinyle", &["Vinyle".into(), "Vinyle 2".into()]), "Vinyle 3");
            assert_eq!(reglages::nom_libre(&"a".repeat(60), &[]).chars().count(), 60, "tronqué pour rester valide");
            let (a, b) = (etat("regl-a"), etat("regl-b"));
            let corps = |r: Response| async move { serde_json::from_slice::<serde_json::Value>(&axum::body::to_bytes(r.into_body(), 1 << 22).await.unwrap()).unwrap() };
            for (nom, dpi) in [("Vinyle", 360), ("Bache", 180), ("Autre", 90)] {
                assert_eq!(appel(&a, "POST", "/api/reglages", None, Some(serde_json::json!({ "nom": nom, "demande": { "dpi": dpi } }))).await.status(), StatusCode::CREATED);
            }
            let export = corps(appel(&a, "GET", "/api/reglages/export?noms=Vinyle,Bache", None, None).await).await;
            assert_eq!(export["reglages"].as_array().unwrap().len(), 2, "seuls les réglages choisis");
            assert_eq!(appel(&b, "POST", "/api/reglages", None, Some(serde_json::json!({ "nom": "Vinyle", "demande": { "dpi": 720 } }))).await.status(), StatusCode::CREATED);
            let uuid_local = reglages::lire(&b.config.donnees)[0].uuid;
            // Renommer (défaut) : le réglage local reste intact.
            let v = corps(appel(&b, "POST", "/api/reglages/import", None, Some(export.clone())).await).await;
            assert_eq!(v["importes"], serde_json::json!(["Bache", "Vinyle 2"]), "{v}");
            let l = reglages::lire(&b.config.donnees);
            assert_eq!(l.iter().find(|r| r.nom == "Vinyle").unwrap().demande.dpi, 720);
            assert_eq!(l.iter().find(|r| r.nom == "Vinyle 2").unwrap().demande.dpi, 360);
            // Écraser : le contenu est remplacé, l'identifiant local reste.
            let mut e = export.clone();
            e["doublon"] = "ecraser".into();
            assert_eq!(appel(&b, "POST", "/api/reglages/import", None, Some(e)).await.status(), StatusCode::OK);
            let l = reglages::lire(&b.config.donnees);
            let vinyle = l.iter().find(|r| r.nom == "Vinyle").unwrap();
            assert_eq!((vinyle.demande.dpi, vinyle.uuid), (360, uuid_local));
            assert_eq!(l.len(), 3, "Vinyle, Vinyle 2, Bache");
            for e in [a, b] {
                std::fs::remove_dir_all(&e.config.donnees).unwrap();
            }
        }

        /// Paquet support (roadmap 1.5) : export puis import sur un serveur vierge = identique.
        #[tokio::test]
        async fn paquet_support_aller_retour() {
            let (a, b) = (etat("paquet-a"), etat("paquet-b"));
            let corps = |r: Response| async move { axum::body::to_bytes(r.into_body(), 1 << 22).await.unwrap() };
            let support = supports::lire(&a.config.donnees, "epson").supports[0].clone();
            let reglage = serde_json::json!({ "nom": "Vinyle-720", "demande": { "dpi": 360, "support_uuid": support.id, "mode_uuid": support.modes[0].id } });
            assert_eq!(appel(&a, "POST", "/api/reglages", None, Some(reglage)).await.status(), StatusCode::CREATED);
            assert_eq!(appel(&a, "POST", "/api/reglages", None, Some(serde_json::json!({ "nom": "Autre", "demande": { "dpi": 360 } }))).await.status(), StatusCode::CREATED);
            let r = appel(&a, "GET", &format!("/api/machines/epson/supports/{}/paquet", support.id.unwrap()), None, None).await;
            assert_eq!(r.status(), StatusCode::OK);
            let paquet: serde_json::Value = serde_json::from_slice(&corps(r).await).unwrap();
            assert_eq!(paquet["reglages"].as_array().unwrap().len(), 1, "seuls les réglages qui se servent du support");
            // Serveur vierge (sans supports.json ni tarifs : catalogue vide).
            std::fs::write(b.config.donnees.join("tarifs.json"), serde_json::to_vec(&serde_json::json!({ "encres": {}, "encre_defaut": { "ml_m2": 10.0, "prix_litre": 100.0 }, "support_m2": 0.0, "supports": [] })).unwrap()).unwrap();
            assert!(supports::lire(&b.config.donnees, "epson").supports.is_empty());
            assert_eq!(appel(&b, "POST", "/api/machines/epson/supports/paquet", None, Some(paquet.clone())).await.status(), StatusCode::NO_CONTENT);
            assert_eq!(supports::lire(&b.config.donnees, "epson").supports, vec![support.clone()], "support et identifiants identiques");
            let rb = reglages::lire(&b.config.donnees);
            assert_eq!(rb.len(), 1);
            assert_eq!((rb[0].nom.as_str(), rb[0].demande.mode_uuid, rb[0].demande.support_uuid), ("Vinyle-720", support.modes[0].id, support.id));
            // Réimporter : rien ne double.
            assert_eq!(appel(&b, "POST", "/api/machines/epson/supports/paquet", None, Some(paquet.clone())).await.status(), StatusCode::NO_CONTENT);
            assert_eq!((supports::lire(&b.config.donnees, "epson").supports.len(), reglages::lire(&b.config.donnees).len()), (1, 1));
            // Paquet d'une version plus récente : refusé, rien d'écrit.
            let mut futur = paquet.clone();
            futur["version"] = 99.into();
            futur["support"]["nom"] = "Futur".into();
            let r = appel(&b, "POST", "/api/machines/epson/supports/paquet", None, Some(futur)).await;
            assert_eq!(r.status(), StatusCode::BAD_REQUEST);
            assert!(supports::lire(&b.config.donnees, "epson").supports.iter().all(|s| s.nom != "Futur"));
            for e in [a, b] {
                std::fs::remove_dir_all(&e.config.donnees).unwrap();
            }
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

        /// Roadmap 8.5 par l'API : jamais étalonnée, étalonnage saisi (à jour), poche
        /// changée dans les relevés SNMP (rappel), étalonnage refait, maintenance saisie.
        #[tokio::test]
        async fn rappels_de_recalibration() {
            let e = etat("recalibration");
            let corps = |r: Response| async move { serde_json::from_slice::<serde_json::Value>(&axum::body::to_bytes(r.into_body(), 1 << 20).await.unwrap()).unwrap() };
            let lire = || appel(&e, "GET", "/api/machines/epson/recalibration", None, None);
            assert_eq!(corps(lire().await).await["statut"], "jamais");
            let v = corps(appel(&e, "POST", "/api/machines/epson/etalonnage", None, Some(serde_json::json!({ "note": "mire HEXIS" }))).await).await;
            assert_eq!(v["statut"], "a_jour");
            let t = v["etalonne_le"].as_u64().unwrap();
            let releve = |le, cyan| releves::Releve { le, encres: vec![("Cyan".into(), cyan)], longueur_m: None };
            releves::ajouter(&e.config.donnees, "epson", &releve(t + 1, 3)).unwrap();
            releves::ajouter(&e.config.donnees, "epson", &releve(t + 2, 100)).unwrap();
            let v = corps(lire().await).await;
            assert_eq!((v["statut"].as_str(), v["declencheurs"][0]["motif"].as_str(), v["declencheurs"][0]["encre"].as_str()), (Some("rappel"), Some("poche"), Some("Cyan")));
            assert_eq!(corps(appel(&e, "GET", "/api/recalibrations", None, None).await).await["epson"]["statut"], "rappel");
            // Étalonnage refait plus tard (relevés datés d'avant lui) : à jour.
            let mut fiches = recalibration::lire(&e.config.donnees);
            fiches.get_mut("epson").unwrap().etalonne_le = Some(t + 3);
            recalibration::ecrire(&e.config.donnees, &fiches).unwrap();
            assert_eq!(corps(lire().await).await["statut"], "a_jour");
            // Maintenance saisie après un étalonnage de la veille, sans relevé : rappel.
            std::fs::remove_dir_all(e.config.donnees.join("releves")).unwrap();
            fiches.get_mut("epson").unwrap().etalonne_le = Some(t - 86_400);
            recalibration::ecrire(&e.config.donnees, &fiches).unwrap();
            assert_eq!(corps(lire().await).await["statut"], "a_jour");
            let r = appel(&e, "POST", "/api/machines/epson/recalibration/evenement", None, Some(serde_json::json!({ "motif": "maintenance", "note": "têtes nettoyées" }))).await;
            assert_eq!(corps(r).await["statut"], "rappel");
            let poche = serde_json::json!({ "motif": "poche", "encre": "Cyan" });
            assert_eq!(appel(&e, "POST", "/api/machines/epson/recalibration/evenement", None, Some(poche)).await.status(), StatusCode::BAD_REQUEST, "une poche se lit, ne se saisit pas");
            assert_eq!(appel(&e, "GET", "/api/machines/inconnue/recalibration", None, None).await.status(), StatusCode::NOT_FOUND);
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
                couches_decoupe: vec![],
                mise_coupe: Default::default(),
                fiche: Default::default(),
                pages_exclues: vec![],
                imbrication: None,
                erp: None,
                retenue_erp: None,
                estimation: false,
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
            assert_eq!(envoi, serde_json::json!({ "canaux": [["C", "Cyan"]], "couches": "dessous", "passages": [["C"]] }));
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
                couches_decoupe: vec![],
                mise_coupe: Default::default(),
                fiche: Default::default(),
                pages_exclues: vec![],
                imbrication: None,
                erp: None,
                retenue_erp: None,
                estimation: false,
            };
            let rep = dossier(&donnees, t.id);
            std::fs::create_dir_all(&rep).unwrap();
            std::fs::write(rep.join("source.pdf"), imbrication::tests::pdf(144.0, 72.0, "1 0 0 0", (0.0, 0.0))).unwrap();
            ecrire(&donnees, &t).unwrap();
            assert_eq!(appel(&e, "GET", &format!("/api/travaux/{}/bat.pdf", t.id), None, None).await.status(), StatusCode::CONFLICT);
            traiter(e.clone(), t.clone(), e.rendu.inscrire(false)).await;
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
                origine_impression_mm: [0.0; 2],
                m2_heure: None,
                parametres_decoupe: vec![],
                decoupe_retournee: false,
                pool: None,
                mono_couche: false,
                coupe_auto: true,
                variante_de: None,
                reperage: None,
                imbrication: Default::default(),
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
                couches_decoupe: vec![],
                mise_coupe: Default::default(),
                fiche: Default::default(),
                pages_exclues: vec![],
                imbrication: None,
                erp: None,
                retenue_erp: None,
                estimation: false,
            };
            let rep = dossier(&donnees, t.id);
            std::fs::create_dir_all(rep.join("prod")).unwrap();
            let trait_ = vec![decoupe::Trace { ton: "CutContour".into(), points: vec![(0.0, 0.0), (72.0, 0.0)], ferme: false }];
            std::fs::write(fichier_traces(&rep, 1), serde_json::to_vec(&trait_).unwrap()).unwrap();
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
                couches_decoupe: vec![],
                mise_coupe: Default::default(),
                fiche: Default::default(),
                pages_exclues: vec![],
                imbrication: None,
                erp: None,
                retenue_erp: None,
                estimation: false,
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
            // Planche manuelle (roadmap 5.10) : poses à la main, gabarit, données variables.
            let apercu = |corps: serde_json::Value| {
                let e = e.clone();
                async move {
                    let r = appel(&e, "POST", "/api/planches/apercu", None, Some(corps)).await;
                    (r.status(), serde_json::from_slice::<serde_json::Value>(&axum::body::to_bytes(r.into_body(), 1 << 20).await.unwrap()).unwrap())
                }
            };
            let el = serde_json::json!([{ "id": t.id, "page": 1, "copies": 2 }]);
            let (s, v) = apercu(serde_json::json!({ "laize_mm": 320.0, "elements": el, "manuel": { "poses": [{ "copie": 1, "x_mm": 100.0, "y_mm": 0.0, "verrou": true }] } })).await;
            assert_eq!((s, v["places"][0]["x"].as_f64().map(f64::round), v["places"][1]["y"].as_f64().map(f64::round)), (StatusCode::OK, Some(100.0), Some(105.0)), "{v}");
            let gabarit = serde_json::json!({ "nom": "A4", "largeur_mm": 210.0, "hauteur_mm": 110.0, "colonnes": 1, "rangees": 3 });
            let (s, v) = apercu(serde_json::json!({ "laize_mm": 320.0, "elements": el, "gabarit": gabarit })).await;
            assert_eq!((s, v["places"][1]["y"].as_f64().map(f64::round), v["longueur_mm"].as_f64().map(f64::round)), (StatusCode::OK, Some(115.0), Some(220.0)), "{v}");
            let dv = serde_json::json!({ "csv": "Nom\nA\nB\nC", "champs": [{ "colonne": "Nom", "x_mm": 10.0, "y_mm": 10.0 }] });
            let (s, v) = apercu(serde_json::json!({ "laize_mm": 320.0, "elements": el, "gabarit": gabarit, "donnees_variables": dv })).await;
            assert_eq!((s, v["places"].as_array().map(Vec::len)), (StatusCode::OK, Some(3)), "une copie par ligne du CSV : {v}");
            let (s, v) = apercu(serde_json::json!({ "laize_mm": 320.0, "elements": el, "gabarit": gabarit, "manuel": {} })).await;
            assert_eq!((s, v["code"].as_str()), (StatusCode::BAD_REQUEST, Some("planche_invalide")), "gabarit et poses s'excluent");
            let gabarits = serde_json::json!([gabarit]);
            assert_eq!(appel(&e, "POST", "/api/gabarits", None, Some(gabarits.clone())).await.status(), StatusCode::OK);
            let lus = super::couches_decoupe::json(appel(&e, "GET", "/api/gabarits", None, None).await).await;
            assert_eq!((lus[0]["nom"].as_str(), lus[0]["espace_mm"].as_f64()), (Some("A4"), Some(0.0)));
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
            c.supports[0].modes.push(supports::ModeImpression { id: None, nom: "Brouillon 90".into(), dpi: [90, 90], passes: 2, chauffage_c: None, gouttes_pl: None, bascule: None, limite_totale: None });
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
                couches_decoupe: vec![],
                mise_coupe: Default::default(),
                fiche: Default::default(),
                pages_exclues: vec![],
                imbrication: None,
                erp: None,
                retenue_erp: None,
                estimation: false,
            };
            let rep = dossier(&donnees, t.id);
            std::fs::create_dir_all(&rep).unwrap();
            std::fs::write(rep.join("source.pdf"), imbrication::tests::pdf(144.0, 72.0, "1 0 0 0", (0.0, 0.0))).unwrap();
            ecrire(&donnees, &t).unwrap();
            traiter(e.clone(), t.clone(), e.rendu.inscrire(false)).await;
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
            let blanc = blanc::Blanc { mode: blanc::Mode::SousCouche, retraction_mm: 0.0, seuil: 5.0, ..Default::default() };
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
                couches_decoupe: vec![],
                mise_coupe: Default::default(),
                fiche: Default::default(),
                pages_exclues: vec![],
                imbrication: None,
                erp: None,
                retenue_erp: None,
                estimation: false,
            };
            let rep = dossier(&donnees, t.id);
            std::fs::create_dir_all(&rep).unwrap();
            // Page cyan pleine de 2 × 1 pouce, contour CutContour à 5 pt du bord.
            std::fs::write(rep.join("source.pdf"), imbrication::tests::pdf(144.0, 72.0, "1 0 0 0", (0.0, 0.0))).unwrap();
            ecrire(&donnees, &t).unwrap();
            traiter(e.clone(), t.clone(), e.rendu.inscrire(false)).await;
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
                couches_decoupe: vec![],
                mise_coupe: Default::default(),
                fiche: Default::default(),
                pages_exclues: vec![],
                imbrication: None,
                erp: None,
                retenue_erp: None,
                estimation: false,
            };
            let rep = dossier(&donnees, t.id);
            std::fs::create_dir_all(&rep).unwrap();
            std::fs::write(rep.join("source.pdf"), imbrication::tests::pdf(144.0, 72.0, "1 0 0 0", (0.0, 0.0))).unwrap();
            ecrire(&donnees, &t).unwrap();
            traiter(e.clone(), t.clone(), e.rendu.inscrire(false)).await;

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
            // Le rapport suit le journal des impressions (roadmap 6.5) : un travail rendu mais
            // jamais imprimé n'y est pas ; l'en-tête est toujours le même.
            let lignes: Vec<&str> = csv.lines().collect();
            assert_eq!(lignes.len(), 1, "{csv}");
            assert!(lignes[0].contains("\"C_estime_ml\";\"C_reel_ml\"") && !lignes[0].contains("CutContour"), "{}", lignes[0]);
            // Le rapport des estimations, lui, a la page : coût estimé, total = somme.
            let r = appel(&e, "GET", "/api/rapports/estimations.csv", None, None).await;
            assert_eq!(r.headers()[header::CONTENT_TYPE], "text/csv; charset=utf-8");
            let est = String::from_utf8(axum::body::to_bytes(r.into_body(), 1 << 20).await.unwrap().to_vec()).unwrap();
            let l: Vec<&str> = est.lines().collect();
            assert_eq!(l.len(), 3, "en-tête, la page, le total : {est}");
            assert_eq!(l[0], consommation::colonnes_estimations().iter().map(|c| format!("\"{c}\"")).collect::<Vec<_>>().join(";"), "colonnes fixes");
            assert!(l[1].contains("\"cyan;\"\"2x1\"\".pdf\""), "nom protégé : {}", l[1]);
            let cyan = (attendu * 100.0).round() / 100.0;
            assert!(l[1].contains(&format!(";{};", format!("{cyan:.2}").replace('.', ","))), "C_ml = {cyan} : {}", l[1]);
            assert!(l[2].starts_with("\"total\";"));
            // Hors période : en-tête seul.
            let r = appel(&e, "GET", "/api/rapports/estimations.csv?jusqua=1", None, None).await;
            assert_eq!(axum::body::to_bytes(r.into_body(), 1 << 20).await.unwrap().split(|b| *b == b'\n').filter(|x| !x.is_empty()).count(), 1);
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
        pub(super) async fn rendre(e: &Partage, pdf: Vec<u8>, d: Demande) -> Travail {
            rendre_format(e, pdf, Format::Pdf, d).await
        }

        pub(super) async fn rendre_format(e: &Partage, octets: Vec<u8>, format: Format, d: Demande) -> Travail {
            let id = Uuid::new_v4();
            let rep = dossier(&e.config.donnees, id);
            std::fs::create_dir_all(&rep).unwrap();
            std::fs::write(rep.join("source.tmp"), octets).unwrap();
            let nom = if format == Format::Svg { "essai.svg" } else { "essai.pdf" };
            creer_travail(e, id, nom.into(), format, d).await.unwrap_or_else(|(s, r)| panic!("refusé : {s} {r:?}"));
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

        /// Roadmap 9.13 : épreuve seule. Le rendu fait l'épreuve TIFF à la taille de la page
        /// (± 0,5 mm), l'impression est refusée ; l'épreuve se télécharge telle quelle ou
        /// refaite en colorimétrique.
        #[tokio::test]
        async fn epreuve_seule_tiff_aux_cotes() {
            use super::controle_d_acces::appel;
            let e = etat("epreuve-seule");
            let pdf = imbrication::tests::pdf(80.0 * MM, 40.0 * MM, "1 0 0 0", (0.0, 0.0));
            let d = Demande { dpi: 100, epreuve_seule: true, ..Default::default() };
            let t = rendre(&e, pdf, d).await;
            let rep = dossier(&e.config.donnees, t.id);
            let tif = chemin_epreuve(&rep, 1);
            assert!(tif.exists(), "épreuve faite d'office : {:?}", t.alertes);
            let i = crate::images::vers_pdf(&tif, &rep.join("relu.pdf"), crate::images::Genre::Tiff, None).unwrap();
            assert!((i.largeur_mm - 80.0).abs() <= 0.5 && (i.hauteur_mm - 40.0).abs() <= 0.5, "{} × {} mm", i.largeur_mm, i.hauteur_mm);
            let m = e.machines.read().unwrap().iter().find(|m| m.fait(machines::Operation::Impression)).unwrap().id.clone();
            let r = appel(&e, "POST", "/api/file", None, Some(serde_json::json!({ "travail": t.id, "page": 1, "machine": m, "operation": "impression" }))).await;
            assert_eq!(r.status(), StatusCode::CONFLICT);
            assert!(e.file.lock().unwrap().sorties.iter().all(|s| s.travail != t.id), "rien en file");
            let r = appel(&e, "GET", &format!("/api/travaux/{}/pages/1/epreuve.tif", t.id), None, None).await;
            assert_eq!(r.status(), StatusCode::OK);
            let o = axum::body::to_bytes(r.into_body(), usize::MAX).await.unwrap();
            assert_eq!(o.len() as u64, std::fs::metadata(&tif).unwrap().len(), "celle faite d'office");
            let r = appel(&e, "GET", &format!("/api/travaux/{}/pages/1/epreuve.tif?couleur=icc&tons=supprimes", t.id), None, None).await;
            assert_eq!(r.status(), StatusCode::OK);
            assert!(axum::body::to_bytes(r.into_body(), usize::MAX).await.unwrap().starts_with(b"II*\0"));
            assert_eq!(appel(&e, "GET", &format!("/api/travaux/{}/pages/1/epreuve.tif?max_mo=0", t.id), None, None).await.status(), StatusCode::BAD_REQUEST);
            assert!(std::fs::read_dir(&rep).unwrap().flatten().all(|f| !f.file_name().to_string_lossy().starts_with("epreuve-") || f.path() == tif), "temporaires effacés");
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// Roadmap 9.11 : 8 fichiers déposés dans un dossier surveillé, 4 rendus à la fois, les
        /// premiers plus gros (plus longs à rendre) : la file reçoit les travaux dans l'ordre
        /// de dépôt.
        #[tokio::test]
        async fn rendus_paralleles_dans_l_ordre_de_depot() {
            let base = etat("rendus-paralleles");
            let machines = std::sync::RwLock::new(base.machines.read().unwrap().clone());
            let e: Partage = Arc::new(Etat { config: Config { rendus: 4, ..base.config.clone() }, rendu: rendus::Rendus::new(4), file: Default::default(), reglages: Default::default(), sessions: Default::default(), machines });
            let d = Demande { dpi: 36, auto: reglages::Auto { imprimer: true, ..Default::default() }, ..Default::default() };
            let depot = e.config.donnees.join("depot");
            std::fs::create_dir_all(&depot).unwrap();
            let noms: Vec<String> = (0..8).map(|i| format!("f{i}.pdf")).collect();
            for (i, nom) in noms.iter().enumerate() {
                let cote = 3000.0 - i as f64 * 350.0;
                let chemin = depot.join(nom);
                std::fs::write(&chemin, pdf_pages(&[PageEssai { media: [0.0, 0.0, cote, cote], crop: None, rotate: 0 }])).unwrap();
                prendre_fichier(&e, &chemin, d.clone(), Fiche::default(), vec![]).await.unwrap();
            }
            for _ in 0..1200 {
                if e.file.lock().unwrap().sorties.len() == noms.len() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            let ordre: Vec<String> = e.file.lock().unwrap().sorties.iter().map(|s| s.nom.clone()).collect();
            assert_eq!(ordre, noms);
            assert_eq!(e.rendu.occupation(), (0, 0));
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
        pub(super) fn encre(rep: &std::path::Path, page: usize, plaque: &str, dpi: u32) -> ((f64, f64), [f64; 4]) {
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
        pub(super) struct PageEssai {
            pub(super) media: [f64; 4],
            pub(super) crop: Option<[f64; 4]>,
            pub(super) rotate: i64,
        }

        /// PDF de plusieurs pages, chacune remplie de cyan sur toute sa MediaBox, avec un
        /// contour `CutContour` à 5 pt du bord.
        pub(super) fn pdf_pages(pages: &[PageEssai]) -> Vec<u8> {
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
            let d = Demande { dpi, mosaique: Some(Mosaique { laize_mm: 300.0, recouvrement_mm: 20.0, ..Default::default() }), ..Default::default() };
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

        /// Roadmap 3.C1 — lés inégaux : 700 × 50 mm en 200, 300 et 240 mm (somme = 700 + 2 × 20) ;
        /// chaque plaque à la largeur saisie ± 0,1 mm (254 dpi : 0,1 mm le pixel), sa coupe rognée
        /// à son bord, et une somme fausse refusée avec son code.
        #[tokio::test]
        async fn regression_les_inegaux() {
            let e = etat("reg-inegaux");
            let dpi = 254;
            let saisies = [200.0, 300.0, 240.0];
            let mosaique = Some(Mosaique { laize_mm: 300.0, recouvrement_mm: 20.0, les_mm: Some(saisies.to_vec()), ..Default::default() });
            let t = rendre(&e, imbrication::tests::pdf(700.0 * MM, 50.0 * MM, "1 0 0 0", (0.0, 0.0)), Demande { dpi, mosaique: mosaique.clone(), ..Default::default() }).await;
            let rep = dossier(&e.config.donnees, t.id);
            assert_eq!(t.pages.len(), 3);
            let mut debut = 0.0;
            for (i, l_mm) in saisies.into_iter().enumerate() {
                let ((l, h), cyan) = encre(&rep, i + 1, "Cyan", dpi);
                assert!((l - l_mm).abs() <= 0.1 && (h - 50.0).abs() <= 0.1, "lé {} : {l} × {h} mm au lieu de {l_mm}", i + 1);
                assert!(proches(cyan, [0.0, 0.0, l_mm, 50.0], 0.1), "lé {} : encre {cyan:?}", i + 1);
                let tuile = t.pages[i].tuile.as_ref().expect("tuile");
                assert_eq!((tuile.numero, tuile.total), (i + 1, 3));
                assert!((tuile.boite_mm[0] - debut).abs() < 1e-3 && (tuile.boite_mm[2] - debut - l_mm).abs() < 1e-3, "lé {} : {:?}", i + 1, tuile.boite_mm);
                let attendu = [(RETRAIT - debut).max(0.0), RETRAIT, (700.0 - RETRAIT - debut).min(l_mm), 50.0 - RETRAIT];
                let coupe = boite_hpgl(&fichier_plt(&rep, i + 1));
                assert!(proches(coupe, attendu, 0.02), "lé {} : coupe {coupe:?}, attendu {attendu:?}", i + 1);
                debut += l_mm - 20.0;
            }
            // Somme fausse : refus traduit, avec le détail.
            let corps = serde_json::json!({ "mosaique": { "laize_mm": 300.0, "recouvrement_mm": 20.0, "les_mm": [200.0, 300.0, 200.0] } });
            let r = super::controle_d_acces::appel(&e, "POST", &format!("/api/travaux/{}/reglages", t.id), None, Some(corps)).await;
            assert_eq!(r.status(), StatusCode::BAD_REQUEST);
            let c: serde_json::Value = serde_json::from_slice(&axum::body::to_bytes(r.into_body(), 1 << 20).await.unwrap()).unwrap();
            assert_eq!(c["code"], "les_incoherents", "{c}");
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// Roadmap 3.C2 — page de 3000 × 2000 mm, tuiles d'au plus 1100 × 1100 mm, 20 mm de
        /// recouvrement : 3 colonnes × 2 rangées de 1013,33 × 1010 mm, numérotées de haut-gauche
        /// à bas-droite ; chaque coupe est la part du contour de la page qui tombe dans sa tuile.
        #[tokio::test]
        async fn regression_tuiles_colonnes_et_rangees() {
            let e = etat("reg-tuiles");
            let dpi = 36;
            let px = 25.4 / dpi as f64;
            let mosaique = Some(Mosaique { laize_mm: 1100.0, hauteur_max_mm: Some(1100.0), recouvrement_mm: 20.0, ..Default::default() });
            let t = rendre(&e, imbrication::tests::pdf(3000.0 * MM, 2000.0 * MM, "1 0 0 0", (0.0, 0.0)), Demande { dpi, mosaique, ..Default::default() }).await;
            let rep = dossier(&e.config.donnees, t.id);
            assert_eq!(t.pages.len(), 6, "3 × 2 tuiles");
            let (l, h) = (3040.0 / 3.0, 1010.0);
            for k in 0..6 {
                let (i, j) = (k % 3, k / 3);
                let tuile = t.pages[k].tuile.as_ref().expect("tuile");
                assert_eq!((tuile.numero, tuile.colonne, tuile.rangee, tuile.colonnes, tuile.rangees), (k + 1, i + 1, j + 1, 3, 2));
                // Rangée 1 en haut : y de 990 à 2000 mm ; rangée 2 de 0 à 1010.
                let (x0, y0) = (i as f64 * (l - 20.0), if j == 0 { 990.0 } else { 0.0 });
                assert!(proches(tuile.boite_mm, [x0, y0, x0 + l, y0 + h], 1e-3), "tuile {} : {:?}", k + 1, tuile.boite_mm);
                let ((pl, ph), cyan) = encre(&rep, k + 1, "Cyan", dpi);
                assert!((pl - l).abs() <= px && (ph - h).abs() <= px, "tuile {} : {pl} × {ph} mm", k + 1);
                assert!(proches(cyan, [0.0, 0.0, l, h], 1.5 * px), "tuile {} : encre {cyan:?}", k + 1);
                // Les côtés du contour rognés à la tuile, dans son repère (un côté dehors devient
                // un point du bord, écarté).
                let (a, b) = (3000.0 - RETRAIT, 2000.0 - RETRAIT);
                let cotes = [[RETRAIT, RETRAIT, a, RETRAIT], [a, RETRAIT, a, b], [RETRAIT, b, a, b], [RETRAIT, RETRAIT, RETRAIT, b]];
                let dedans: Vec<[f64; 4]> = cotes.iter()
                    .map(|c| [c[0].clamp(x0, x0 + l) - x0, c[1].clamp(y0, y0 + h) - y0, c[2].clamp(x0, x0 + l) - x0, c[3].clamp(y0, y0 + h) - y0])
                    .filter(|c| (c[0] == c[2] && c[0] > 0.0 && c[0] < l && c[1] < c[3]) || (c[1] == c[3] && c[1] > 0.0 && c[1] < h && c[0] < c[2]))
                    .collect();
                let attendu = [0, 1, 2, 3].map(|n| dedans.iter().map(|c| c[n]).fold(if n < 2 { f64::MAX } else { f64::MIN }, if n < 2 { f64::min } else { f64::max }));
                let coupe = boite_hpgl(&fichier_plt(&rep, k + 1));
                assert!(proches(coupe, attendu, 0.02), "tuile {} : coupe {coupe:?}, attendu {attendu:?}", k + 1);
            }
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// Roadmap 3.C3 — soudure et coupe au milieu : 700 × 100 mm en 3 lés de 246,67 mm, 20 mm
        /// de recouvrement. Les lés 2 et 3 n'ont pas d'encre dans leurs 20 premiers mm (sous la
        /// voisine) ; le lé 1 imprime le sien jusqu'au bord ; un trait de coupe droit à 10 mm de
        /// chaque jonction. Puis un lé décoché : ni rendu, ni en file.
        #[tokio::test]
        async fn regression_soudure_coupe_au_milieu_et_le_decoche() {
            let e = etat("reg-soudure");
            let dpi = 72;
            let px = 25.4 / dpi as f64;
            let base = Mosaique { laize_mm: 300.0, recouvrement_mm: 20.0, soudure: true, coupe_milieu: true, ..Default::default() };
            let t = rendre(&e, imbrication::tests::pdf(700.0 * MM, 100.0 * MM, "1 0 0 0", (0.0, 0.0)), Demande { dpi, mosaique: Some(base.clone()), ..Default::default() }).await;
            let rep = dossier(&e.config.donnees, t.id);
            let le = 740.0 / 3.0;
            let ((_, _), cyan) = encre(&rep, 1, "Cyan", dpi);
            assert!(proches(cyan, [0.0, 0.0, le, 100.0], 1.5 * px), "lé 1 imprimé jusqu'au bord : {cyan:?}");
            for i in [2, 3] {
                let img = image::open(render::chemin_plaque(&rep, i, "Cyan")).unwrap().to_luma8();
                let bande = (20.0 / px).floor() as u32 - 1;
                assert!(img.enumerate_pixels().filter(|p| p.0 < bande).all(|p| p.2[0] == 255), "lé {i} : encre dans la bande de soudure");
                let ((_, _), cyan) = encre(&rep, i, "Cyan", dpi);
                assert!((cyan[0] - 20.0).abs() <= 1.5 * px, "lé {i} : l'encre reprend à 20 mm : {cyan:?}");
            }
            let pages = decoupe::extraire(&std::fs::read(rep.join("source.pdf")).unwrap(), false).unwrap();
            let droit = |p: &decoupe::Decoupe, x_mm: f64| p.traces.iter().any(|t| t.points.iter().all(|q| (q.0 / MM - x_mm).abs() < 0.01));
            assert!(droit(&pages[0], le - 10.0) && !droit(&pages[0], 10.0), "lé 1 : coupe au milieu de la jonction de droite seulement");
            assert!(droit(&pages[1], 10.0) && droit(&pages[1], le - 10.0), "lé 2 : une coupe à chaque jonction");

            // Lé 2 décoché, file automatique : deux pages (lés 1 et 3), deux sorties.
            let auto = reglages::Auto { imprimer: true, ..Default::default() };
            let mosaique = Some(Mosaique { exclus: vec![2], soudure: false, coupe_milieu: false, ..base });
            let t = rendre(&e, imbrication::tests::pdf(700.0 * MM, 100.0 * MM, "1 0 0 0", (0.0, 0.0)), Demande { dpi: 36, mosaique, auto, ..Default::default() }).await;
            assert_eq!(t.pages.iter().map(|p| p.tuile.as_ref().unwrap().numero).collect::<Vec<_>>(), [1, 3]);
            let file: Vec<usize> = e.file.lock().unwrap().sorties.iter().filter(|s| s.travail == t.id).map(|s| s.page).collect();
            assert_eq!(file, [1, 2], "deux sorties : les pages des lés 1 et 3");
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// Roadmap 3.C4 — recouvrement propre et ordre : 700 × 100 mm, 3 lés, 10 mm de
        /// recouvrement, 20 mm pour le lé 2, ordre 3-1-2. Base : lés de 240 mm ; le lé 2 gagne
        /// 5 mm de chaque côté (250 mm) ; la file suit l'ordre choisi.
        #[tokio::test]
        async fn regression_recouvrement_par_tuile_et_ordre() {
            let e = etat("reg-ordre");
            let dpi = 72;
            let px = 25.4 / dpi as f64;
            let mosaique = Some(Mosaique { laize_mm: 300.0, recouvrement_mm: 10.0, recouvrement_tuiles_mm: vec![None, Some(20.0)], ordre: vec![3, 1, 2], ..Default::default() });
            let auto = reglages::Auto { imprimer: true, ..Default::default() };
            let t = rendre(&e, imbrication::tests::pdf(700.0 * MM, 100.0 * MM, "1 0 0 0", (0.0, 0.0)), Demande { dpi, mosaique, auto, ..Default::default() }).await;
            let rep = dossier(&e.config.donnees, t.id);
            let numeros: Vec<usize> = t.pages.iter().map(|p| p.tuile.as_ref().unwrap().numero).collect();
            assert_eq!(numeros, [3, 1, 2], "pages dans l'ordre choisi");
            let file: Vec<usize> = e.file.lock().unwrap().sorties.iter().filter(|s| s.travail == t.id).map(|s| t.pages[s.page - 1].tuile.as_ref().unwrap().numero).collect();
            assert_eq!(file, [3, 1, 2], "file dans l'ordre choisi");
            for (page, (numero, x0, l_mm)) in [(3, 460.0, 240.0), (1, 0.0, 240.0), (2, 225.0, 250.0)].into_iter().enumerate() {
                let tuile = t.pages[page].tuile.as_ref().unwrap();
                assert_eq!(tuile.numero, numero);
                assert!((tuile.boite_mm[0] - x0).abs() < 1e-3 && (tuile.boite_mm[2] - x0 - l_mm).abs() < 1e-3, "lé {numero} : {:?}", tuile.boite_mm);
                let ((l, _), _) = encre(&rep, page + 1, "Cyan", dpi);
                assert!((l - l_mm).abs() <= px, "lé {numero} : plaque de {l} mm au lieu de {l_mm}");
            }
            assert!((t.pages[2].tuile.as_ref().unwrap().recouvrement_mm[0] - 15.0).abs() < 1e-3, "jonction 1-2 : (10 + 20) / 2");
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

        /// Une page par travail (roadmap 9.3) : un PDF de trois pages donne trois travaux,
        /// chacun d'une page et nommé d'après elle.
        #[tokio::test]
        async fn une_page_par_travail() {
            let e = etat("page-par-travail");
            let pages = [
                PageEssai { media: [0.0, 0.0, 100.0 * MM, 50.0 * MM], crop: None, rotate: 0 },
                PageEssai { media: [0.0, 0.0, 60.0 * MM, 120.0 * MM], crop: None, rotate: 0 },
                PageEssai { media: [0.0, 0.0, 80.0 * MM, 80.0 * MM], crop: None, rotate: 0 },
            ];
            let t = rendre(&e, pdf_pages(&pages), Demande { dpi: 36, page_par_travail: true, ..Default::default() }).await;
            assert_eq!((t.nom.as_str(), t.pages.len()), ("essai.pdf (p. 1/3)", 1));
            let mut autres = vec![];
            for _ in 0..600 {
                autres = lire_tous(&e.config.donnees).into_iter().filter(|a| a.id != t.id).collect::<Vec<_>>();
                if autres.len() == 2 && autres.iter().all(|a| !actif(&a.statut)) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            autres.sort_by(|a, b| a.nom.cmp(&b.nom));
            assert_eq!(autres.iter().map(|a| a.nom.as_str()).collect::<Vec<_>>(), ["essai.pdf (p. 2/3)", "essai.pdf (p. 3/3)"]);
            for (a, l_mm) in autres.iter().zip([60.0, 80.0]) {
                assert!(matches!(a.statut, Statut::Termine), "{:?}", a.erreur);
                assert!(!a.demande.page_par_travail);
                assert_eq!(a.pages.len(), 1);
                let ((l, _), _) = encre(&dossier(&e.config.donnees, a.id), 1, "Cyan", 36);
                assert!((l - l_mm).abs() <= 25.4 / 36.0, "{} : {l} mm", a.nom);
            }
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// Abandon si une police manque (roadmap 9.3) : refusé avant le rendu, sauf pour les
        /// 14 polices de base.
        #[tokio::test]
        async fn abandon_si_police_manquante() {
            let e = etat("abandon-polices");
            let pdf = |police: &str| {
                let mut doc = Document::with_version("1.4");
                let f = doc.add_object(dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => police });
                let c = doc.add_object(Stream::new(dictionary! {}, b"BT /F1 12 Tf 10 50 Td (essai) Tj ET".to_vec()));
                let pages = doc.new_object_id();
                let page = doc.add_object(dictionary! { "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(), 0.into(), 144.into(), 144.into()], "Contents" => c, "Resources" => dictionary! { "Font" => dictionary! { "F1" => f } } });
                doc.objects.insert(pages, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }));
                let cat = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
                doc.trailer.set("Root", cat);
                let mut o = vec![];
                doc.save_to(&mut o).unwrap();
                o
            };
            let d = Demande { dpi: 36, abandon_polices: true, ..Default::default() };
            let id = Uuid::new_v4();
            let rep = dossier(&e.config.donnees, id);
            std::fs::create_dir_all(&rep).unwrap();
            std::fs::write(rep.join("source.tmp"), pdf("ArialMT")).unwrap();
            let (statut, (code, detail)) = creer_travail(&e, id, "essai.pdf".into(), Format::Pdf, d.clone()).await.err().expect("refusé");
            assert_eq!((statut, code, detail.as_deref()), (StatusCode::BAD_REQUEST, "polices_manquantes", Some("ArialMT")));
            // Sans l'option, le même fichier passe avec une alerte ; Helvetica passe avec elle.
            let t = rendre(&e, pdf("ArialMT"), Demande { abandon_polices: false, ..d.clone() }).await;
            assert!(t.alertes.iter().any(|a| a.code == "police_non_incorporee"), "{:?}", t.alertes);
            rendre(&e, pdf("Helvetica"), d).await;
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
            let mosaique = Some(Mosaique { laize_mm: 300.0, recouvrement_mm: 20.0, ..Default::default() });
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

        /// Éditeur, roadmap 3.B : page de 200 × 100 mm, aplat cyan décentré
        /// [10, 10]–[60, 40] mm et `CutContour` [12, 12]–[58, 38] mm : un miroir ou un
        /// recadrage se voit sur l'encre comme sur la coupe.
        fn pdf_decentre() -> Vec<u8> {
            let mut doc = Document::with_version("1.4");
            let sep = Object::Array(vec![
                "Separation".into(),
                "CutContour".into(),
                "DeviceCMYK".into(),
                Object::Dictionary(dictionary! { "FunctionType" => 2, "Domain" => vec![0.into(), 1.into()], "C0" => vec![0.into(); 4], "C1" => vec![0.into(), 1.into(), 0.into(), 0.into()], "N" => 1 }),
            ]);
            let contenu = format!("1 0 0 0 k {} {} {} {} re f /CS0 CS 1 SCN {} {} {} {} re S", 10.0 * MM, 10.0 * MM, 50.0 * MM, 30.0 * MM, 12.0 * MM, 12.0 * MM, 46.0 * MM, 26.0 * MM);
            let c = doc.add_object(Stream::new(dictionary! {}, contenu.into_bytes()));
            let pages = doc.new_object_id();
            let page = doc.add_object(dictionary! { "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(), 0.into(), Object::Real((200.0 * MM) as f32), Object::Real((100.0 * MM) as f32)], "Contents" => c, "Resources" => dictionary! { "ColorSpace" => dictionary! { "CS0" => sep } } });
            doc.objects.insert(pages, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }));
            let cat = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
            doc.trailer.set("Root", cat);
            let mut o = vec![];
            doc.save_to(&mut o).unwrap();
            o
        }

        /// Format (mm) de la MediaBox de la page 1 du PDF produit.
        fn media_mm(rep: &std::path::Path) -> (f64, f64) {
            let t = imbrication::taille_page(&std::fs::read(rep.join("source.pdf")).unwrap(), 1).unwrap();
            (t.largeur / MM, t.hauteur / MM)
        }

        fn edite(edition: edition::Edition) -> Demande {
            Demande { dpi: 36, rotation_auto: false, edition, ..Default::default() }
        }

        /// 3.B1 — recadrage [50, 20] + 100 × 60 mm : la page produite est le cadre, l'encre et
        /// la coupe y sont rognées ; avec 10 mm de marge, la coupe s'arrête au bord du cadre
        /// (BBox du formulaire), pas à celui de la page.
        #[tokio::test]
        async fn editeur_recadrage_page_encre_et_coupe() {
            let e = etat("ed-recadrage");
            let px = 25.4 / 36.0;
            let cadre = Some([50.0, 20.0, 100.0, 60.0]);
            for marge in [0.0, 10.0] {
                let t = rendre(&e, pdf_decentre(), edite(edition::Edition { recadrage: cadre, marge_mm: marge, ..Default::default() })).await;
                let rep = dossier(&e.config.donnees, t.id);
                let (l, h) = media_mm(&rep);
                assert!((l - (100.0 + 2.0 * marge)).abs() < 0.01 && (h - (60.0 + 2.0 * marge)).abs() < 0.01, "marge {marge} : MediaBox {l} × {h}");
                let ((pl, ph), cyan) = encre(&rep, 1, "Cyan", 36);
                assert!((pl - l).abs() <= px && (ph - h).abs() <= px, "plaque {pl} × {ph}");
                // Aplat [10, 60] × [10, 40] dans le cadre [50, 150] × [20, 80] : [0, 10] × [0, 20].
                assert!(proches(cyan, [marge, marge, marge + 10.0, marge + 20.0], 1.5 * px), "marge {marge} : encre {cyan:?}");
                let coupe = boite_hpgl(&fichier_plt(&rep, 1));
                assert!(proches(coupe, [marge, marge, marge + 8.0, marge + 18.0], 0.02), "marge {marge} : coupe {coupe:?}");
                let g = t.pages[0].geometrie.expect("géométrie");
                assert!(proches(g.image_mm, [marge, marge, 100.0, 60.0], 0.01), "{g:?}");
                assert!((g.source_mm[0] - 200.0).abs() < 0.01 && (g.source_mm[1] - 100.0).abs() < 0.01, "{g:?}");
            }
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// 3.B1 — cadre hors de la page ou vide : refus codé, jamais un rendu faux.
        #[tokio::test]
        async fn editeur_recadrage_hors_page_refuse() {
            let e = etat("ed-recadrage-refus");
            let t = rendre(&e, pdf_decentre(), edite(edition::Edition::default())).await;
            for cadre in [[150.0, 0.0, 100.0, 50.0], [0.0, 60.0, 50.0, 50.0], [0.0, 0.0, 0.0, 50.0], [-1.0, 0.0, 50.0, 50.0]] {
                let mut d = serde_json::to_value(&t.demande).unwrap();
                d["edition"]["recadrage"] = serde_json::json!(cadre);
                let r = super::controle_d_acces::appel(&e, "POST", &format!("/api/travaux/{}/reglages", t.id), None, Some(d)).await;
                assert_eq!(r.status(), StatusCode::BAD_REQUEST, "{cadre:?}");
                let corps: serde_json::Value = serde_json::from_slice(&axum::body::to_bytes(r.into_body(), 1 << 20).await.unwrap()).unwrap();
                assert_eq!(corps["code"], "recadrage_invalide", "{cadre:?} : {corps}");
            }
            // Un cadre au bord exact de la page passe.
            let mut d = serde_json::to_value(&t.demande).unwrap();
            d["edition"]["recadrage"] = serde_json::json!([100.0, 50.0, 100.0, 50.0]);
            let r = super::controle_d_acces::appel(&e, "POST", &format!("/api/travaux/{}/reglages", t.id), None, Some(d)).await;
            assert_eq!(r.status(), StatusCode::ACCEPTED);
            for _ in 0..600 {
                if !actif(&lire_travail(&e.config.donnees, t.id).unwrap().statut) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// 3.B2 — largeur imposée à 500 mm (l'écran en tire l'échelle) : hauteur au prorata ;
        /// échelle déliée 250 × 50 % : déformation exacte, la coupe déformée pareil.
        #[tokio::test]
        async fn editeur_largeur_imposee_et_echelle_deliee() {
            let e = etat("ed-echelle");
            let px = 25.4 / 36.0;
            let source = || imbrication::tests::pdf(180.0 * MM, 70.0 * MM, "1 0 0 0", (0.0, 0.0));
            let k = 500.0 / 180.0;
            let t = rendre(&e, source(), edite(edition::Edition { echelle: k * 100.0, ..Default::default() })).await;
            let rep = dossier(&e.config.donnees, t.id);
            let (l, h) = media_mm(&rep);
            assert!((l - 500.0).abs() < 0.01 && (h - 70.0 * k).abs() < 0.01, "{l} × {h} (attendu 500 × {:.2})", 70.0 * k);
            let (_, cyan) = encre(&rep, 1, "Cyan", 36);
            assert!(proches(cyan, [0.0, 0.0, 500.0, 70.0 * k], 1.5 * px), "encre {cyan:?}");
            let r = k * RETRAIT;
            assert!(proches(boite_hpgl(&fichier_plt(&rep, 1)), [r, r, 500.0 - r, 70.0 * k - r], 0.02));

            let t = rendre(&e, source(), edite(edition::Edition { echelle_xy: Some([250.0, 50.0]), ..Default::default() })).await;
            let rep = dossier(&e.config.donnees, t.id);
            let (l, h) = media_mm(&rep);
            assert!((l - 450.0).abs() < 0.01 && (h - 35.0).abs() < 0.01, "déliée : {l} × {h}");
            let (_, cyan) = encre(&rep, 1, "Cyan", 36);
            assert!(proches(cyan, [0.0, 0.0, 450.0, 35.0], 1.5 * px), "déliée : encre {cyan:?}");
            let coupe = boite_hpgl(&fichier_plt(&rep, 1));
            assert!(proches(coupe, [2.5 * RETRAIT, 0.5 * RETRAIT, 450.0 - 2.5 * RETRAIT, 35.0 - 0.5 * RETRAIT], 0.02), "déliée : coupe {coupe:?}");
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// 3.B3 — miroir : image + découpe (défaut) → l'encre et la coupe passent à droite ;
        /// image seule → l'encre passe à droite, la coupe reste où le fichier la met.
        #[tokio::test]
        async fn editeur_miroir_image_et_decoupe_ou_image_seule() {
            let e = etat("ed-miroir");
            let px = 25.4 / 36.0;
            let defaut = edition::Edition { miroir: true, ..Default::default() };
            assert_eq!(defaut.miroir_portee, edition::PorteeMiroir::ImageEtDecoupe, "défaut : comme avant ce champ");
            for (portee, coupe_attendue) in [(edition::PorteeMiroir::ImageEtDecoupe, [142.0, 12.0, 188.0, 38.0]), (edition::PorteeMiroir::ImageSeule, [12.0, 12.0, 58.0, 38.0])] {
                let t = rendre(&e, pdf_decentre(), edite(edition::Edition { miroir_portee: portee, ..defaut.clone() })).await;
                let rep = dossier(&e.config.donnees, t.id);
                let (_, cyan) = encre(&rep, 1, "Cyan", 36);
                assert!(proches(cyan, [140.0, 10.0, 190.0, 40.0], 1.5 * px), "{portee:?} : encre {cyan:?}");
                assert_eq!(t.pages[0].traces_decoupe, 1, "{portee:?} : un seul tracé (pas de doublon)");
                let coupe = boite_hpgl(&fichier_plt(&rep, 1));
                assert!(proches(coupe, coupe_attendue, 0.02), "{portee:?} : coupe {coupe:?}");
                assert!(!t.pages[0].plaques.iter().any(|p| p == "CutContour"), "{portee:?} : la coupe n'imprime rien");
            }
            // Image seule avec recadrage et quart de tour : la coupe suit le cadre et la rotation, pas le miroir.
            let t = rendre(&e, pdf_decentre(), edite(edition::Edition { miroir_portee: edition::PorteeMiroir::ImageSeule, recadrage: Some([0.0, 0.0, 100.0, 50.0]), rotation: 90, ..defaut })).await;
            let rep = dossier(&e.config.donnees, t.id);
            // Cadre 100 × 50, tourné : 50 × 100. Quart de tour à gauche : (x, y) → (50 − y, x).
            let coupe = boite_hpgl(&fichier_plt(&rep, 1));
            assert!(proches(coupe, [12.0, 12.0, 38.0, 58.0], 0.02), "image seule tournée : coupe {coupe:?}");
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }
    }

    /// Pilote direct de l'Epson (roadmap 2.4, 2.6) : PDF CMJN → rendu Ghostscript →
    /// séparation → tramage → ESC/P2 reçu par un faux port 9100, re-décodé.
    mod pilote_epson {
        use super::controle_d_acces::{appel, etat};
        use super::regression_geometrie::rendre;
        use super::*;
        use lopdf::{Document, Object, Stream, dictionary};

        const MM: f64 = imbrication::PT_PAR_MM;

        /// 20 × 10 mm : moitié gauche en tons clairs (C, M, N 15 %, J plein), moitié droite
        /// à 40 % (zone de transition : claire et foncée ensemble) ; `blanc` : un carré
        /// `White` en plus.
        fn pdf(blanc: bool) -> Vec<u8> {
            let (l, h) = (20.0 * MM, 10.0 * MM);
            let mut doc = Document::with_version("1.4");
            let sep = doc.add_object(Object::Array(vec![
                "Separation".into(),
                "White".into(),
                "DeviceCMYK".into(),
                Object::Dictionary(dictionary! { "FunctionType" => 2, "Domain" => vec![0.into(), 1.into()], "C0" => vec![0.into(); 4], "C1" => vec![0.into(), 0.into(), 0.into(), Object::Real(0.1)], "N" => 1 }),
            ]));
            let mut contenu = format!("0.15 0.15 1 0.15 k 0 0 {} {h} re f 0.4 0.4 0 0.4 k {} 0 {} {h} re f", l / 2.0, l / 2.0, l / 2.0);
            if blanc {
                contenu += " /CS0 cs 1 scn 0 0 10 10 re f";
            }
            let c = doc.add_object(Stream::new(dictionary! {}, contenu.into_bytes()));
            let pages = doc.new_object_id();
            let page = doc.add_object(dictionary! { "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(), 0.into(), Object::Real(l as f32), Object::Real(h as f32)], "Contents" => c, "Resources" => dictionary! { "ColorSpace" => dictionary! { "CS0" => sep } } });
            doc.objects.insert(pages, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }));
            let cat = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
            doc.trailer.set("Root", cat);
            let mut o = vec![];
            doc.save_to(&mut o).unwrap();
            o
        }

        /// L'Epson du catalogue passe en réseau sur `port`, SNMP éventuel.
        fn en_reseau(e: &Partage, port: u16, snmp: Option<String>) {
            let mut c = e.machines.write().unwrap();
            (c[0].connexion, c[0].snmp) = (machines::Connexion::Reseau { hote: "127.0.0.1".into(), port }, snmp);
        }

        /// Exécute une sortie de la page 1 sur l'Epson et rend son état final.
        async fn executer_sur_l_epson(e: &Partage, travail: Uuid) -> file::Sortie {
            let s = file::Sortie::nouvelle("epson", machines::Operation::Impression, travail, "essai", 1, maintenant());
            e.file.lock().unwrap().sorties.push(s.clone());
            executer(e.clone(), s.clone()).await;
            e.file.lock().unwrap().sorties.iter().find(|x| x.id == s.id).unwrap().clone()
        }

        #[tokio::test]
        async fn pdf_cmjn_imprime_par_le_reseau() {
            let e = etat("pilote-epson");
            let ecoute = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let port = ecoute.local_addr().unwrap().port();
            let recu = std::thread::spawn(move || {
                let (mut f, _) = ecoute.accept().unwrap();
                let mut o = vec![];
                std::io::Read::read_to_end(&mut f, &mut o).unwrap();
                o
            });
            // SNMP : un avertissement seul (encre basse) ne bloque pas ; sans `hrPrinterStatus`,
            // la surveillance s'arrête à la fin de l'envoi.
            let avertissement = media::tests::alertes_d_essai().into_iter().filter(|(o, _)| o.ends_with(&[1, 3])).collect();
            en_reseau(&e, port, Some(media::tests::agent_factice(avertissement)));
            let dpi = 720;
            let t = rendre(&e, pdf(false), Demande { dpi, ..Default::default() }).await;
            let r = appel(&e, "POST", "/api/file", None, Some(serde_json::json!({ "travail": t.id, "page": 1, "machine": "epson" }))).await;
            assert_eq!(r.status(), StatusCode::CREATED);
            tokio::spawn(repartir(e.clone()));
            let recu = tokio::time::timeout(Duration::from_secs(30), tokio::task::spawn_blocking(move || recu.join().unwrap())).await.expect("rien reçu en 30 s").unwrap();
            let (lignes, blocs) = escp2::tests::decoder(&recu);
            // Résolution du mode (720 : pas 4 sur 2880, `(D` 20/20), bandes de 16 lignes 2 bits.
            assert!(lignes.contains(&"ESC (U 08 04 04 40 0b".to_string()) && lignes.contains(&"ESC (D 40 38 14 14".to_string()), "{lignes:?}");
            let bandes: Vec<&String> = lignes.iter().filter(|l| l.starts_with("BANDES")).collect();
            assert_eq!(bandes.len(), 1, "{lignes:?}");
            assert!(bandes[0].starts_with("BANDES lignes=16 bits=2 x%16=0 ordre="), "{}", bandes[0]);
            // Canaux : les 7 que la séparation tire du CMJN (C Lc M Lm J N Lk) ; Or et Rd
            // n'ont pas de source sans profil de gamut étendu.
            let mut codes: Vec<u8> = blocs.iter().map(|b| b.code).collect();
            codes.sort();
            codes.dedup();
            assert_eq!(codes, [0, 1, 2, 4, 16, 17, 18]);
            assert!(codes.iter().all(|c| escp2::ORDRE.contains(c)), "canal hors des 9 encres");
            // Toute la page, après la tête vide de 20 mm (10 mm à 720 dpi : 283 lignes).
            let hauteur = (10.0 / 25.4 * dpi as f64).round() as u32;
            let tete = (TETE_EPSON_MM / 25.4 * dpi as f64).round() as u32;
            let y0 = blocs.iter().map(|b| b.y).min().unwrap();
            assert!(y0.abs_diff(tete) <= 1, "tête vide : 1re ligne {y0}, attendu {tete}");
            assert_eq!(blocs.iter().map(|b| b.y).max().map(|y| (y - y0) / 16), Some((hauteur - 1) / 16));
            // Jaune plein à gauche, absent à droite : ses blocs s'arrêtent à mi-largeur.
            let largeur = (20.0 / 25.4 * dpi as f64).round() as u32;
            assert!(blocs.iter().filter(|b| b.code == 4).all(|b| b.x + 4 * b.octets as u32 <= largeur / 2 + 16), "jaune à droite");
            for _ in 0..100 {
                if e.file.lock().unwrap().sorties[0].etat == file::Etat::Terminee {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            assert_eq!(e.file.lock().unwrap().sorties[0].etat, file::Etat::Terminee);
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        async fn json(r: Response) -> serde_json::Value {
            serde_json::from_slice(&axum::body::to_bytes(r.into_body(), 1 << 22).await.unwrap()).unwrap()
        }

        /// Phase 6 par le vrai circuit, SNMP et port 9100 simulés : relevés forcés au début et à
        /// la fin (6.1), longueur et gâche (6.3), journal (6.4), encre estimée par les points
        /// posés (6.9), rapport (6.5), purge manuelle qui laisse travaux et file (6.4).
        #[tokio::test]
        async fn consommation_de_la_sortie_et_journal() {
            let e = etat("pilote-epson-conso");
            let ecoute = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let port = ecoute.local_addr().unwrap().port();
            let table = std::sync::Arc::new(std::sync::Mutex::new(media::tests::epson_d_essai(&[(0x01, 50), (0x00, 80)], 50_000_000)));
            let snmp = media::tests::agent_modifiable(table.clone());
            // L'imprimante consomme pendant l'envoi : 2 % de cyan, 574 mm de rouleau.
            let recu = std::thread::spawn(move || {
                let (mut f, _) = ecoute.accept().unwrap();
                *table.lock().unwrap() = media::tests::epson_d_essai(&[(0x01, 48), (0x00, 80)], 49_426_000);
                std::io::Read::read_to_end(&mut f, &mut vec![]).unwrap();
                // Seconde impression (volumes de goutte du mode).
                let (mut f, _) = ecoute.accept().unwrap();
                std::io::Read::read_to_end(&mut f, &mut vec![]).unwrap();
            });
            en_reseau(&e, port, Some(snmp));
            let t = rendre(&e, pdf(false), Demande { dpi: 360, ..Default::default() }).await;
            let mut t = lire_travail(&e.config.donnees, t.id).unwrap();
            t.fiche.operateur = "Léa".into();
            ecrire(&e.config.donnees, &t).unwrap();
            let s = executer_sur_l_epson(&e, t.id).await;
            assert_eq!(s.etat, file::Etat::Terminee, "{:?}", s.erreur);
            let c = s.consommation.expect("consommation rangée dans la sortie");
            assert_eq!(c.encres_pc, std::collections::BTreeMap::from([("BK".into(), 0), ("C".into(), 2)]));
            assert_eq!((c.seuil_ml["C"], c.longueur_mm), (7.0, Some(574.0)));
            assert_eq!(releves::lire(&e.config.donnees, "epson").len(), 2, "un relevé forcé à chaque bout");

            let j = json(appel(&e, "GET", "/api/consommation/journal", None, None).await).await;
            assert_eq!(j.as_array().unwrap().len(), 1, "{j}");
            let l = &j[0];
            assert_eq!((l["operateur"].as_str(), l["dpi"].as_u64(), l["mode"].as_str()), (Some("Léa"), Some(360), Some("360 dpi")));
            assert_eq!(l["estime"]["methode"], "points", "{l}");
            let estime = l["estime_total_ml"].as_f64().unwrap();
            assert!(estime > 0.0 && estime < 1.0, "page de 2 cm² : {estime} ml");
            assert_eq!(l["reel_total_ml"].as_f64(), Some(14.0));
            let p = &t.pages[0];
            assert!((l["gache_mm"].as_f64().unwrap() - (574.0 - p.hauteur_mm)).abs() < 1e-9, "{l}");
            assert!(l["estime"]["cout_encres"].as_f64().unwrap() > 0.0);

            let r = appel(&e, "GET", "/api/rapports/consommation.csv", None, None).await;
            let csv = String::from_utf8(axum::body::to_bytes(r.into_body(), 1 << 20).await.unwrap().to_vec()).unwrap();
            let lignes: Vec<&str> = csv.lines().collect();
            assert_eq!(lignes.len(), 3, "en-tête, l'impression, le total : {csv}");
            assert!(lignes[1].contains(";\"points\";") && lignes[1].contains(";14,00;"), "{}", lignes[1]);

            // 6.9 : volumes de goutte du mode, éditables ; deux fois plus gros, deux fois plus d'encre.
            let mut catalogue = supports::lire(&e.config.donnees, "epson");
            catalogue.supports[0].modes.push(supports::ModeImpression { id: None, nom: "360 étalonné".into(), dpi: [360, 360], passes: 4, chauffage_c: None, gouttes_pl: Some([10.0, 20.0, 30.0]), bascule: None, limite_totale: None });
            let connus = supports::lire(&e.config.donnees, "epson").supports;
            supports::attribuer_ids(&mut catalogue.supports, &connus);
            supports::ecrire(&e.config.donnees, &catalogue).unwrap();
            let mut t2 = lire_travail(&e.config.donnees, t.id).unwrap();
            t2.demande.mode_uuid = catalogue.supports[0].modes.last().unwrap().id;
            ecrire(&e.config.donnees, &t2).unwrap();
            assert_eq!(executer_sur_l_epson(&e, t.id).await.etat, file::Etat::Terminee);
            recu.join().unwrap();
            let j = json(appel(&e, "GET", "/api/consommation/journal", None, None).await).await;
            assert_eq!(j[0]["mode"], "360 étalonné");
            assert!((j[0]["estime_total_ml"].as_f64().unwrap() - 2.0 * estime).abs() < 1e-12, "{} ≠ 2 × {estime}", j[0]["estime_total_ml"]);

            // Purge manuelle de tout le journal : le travail et la sortie restent.
            let r = json(appel(&e, "POST", "/api/consommation/journal/purger", None, Some(serde_json::json!({ "jours": 0 }))).await).await;
            assert_eq!(r["retirees"], 2);
            assert!(json(appel(&e, "GET", "/api/consommation/journal", None, None).await).await.as_array().unwrap().is_empty());
            assert!(lire_travail(&e.config.donnees, t.id).is_some());
            assert!(e.file.lock().unwrap().sorties.iter().any(|x| x.id == s.id));
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// 6.4 : la rétention se règle (défaut 90 j) ; un réglage hors bornes est refusé ; la
        /// sauvegarde porte ces réglages.
        #[tokio::test]
        async fn retention_reglable_et_sauvegardee() {
            let e = etat("conso-reglages");
            let r = json(appel(&e, "GET", "/api/consommation/reglages", None, None).await).await;
            assert_eq!(r["retention_jours"], 90);
            let bon = serde_json::json!({ "retention_jours": 1, "facteurs": [{ "machine": "epson", "dpi": 720, "encre": "C", "valeur": 1.4 }] });
            assert_eq!(appel(&e, "POST", "/api/consommation/reglages", None, Some(bon)).await.status(), StatusCode::OK);
            let mauvais = serde_json::json!({ "retention_jours": 0 });
            assert_eq!(appel(&e, "POST", "/api/consommation/reglages", None, Some(mauvais)).await.status(), StatusCode::BAD_REQUEST);
            let s = json(appel(&e, "GET", "/api/sauvegarde", None, None).await).await;
            assert_eq!(s["consommation"]["facteurs"][0]["valeur"], 1.4, "{}", s["consommation"]);
            // Une sauvegarde d'avant n'a pas la clé : elle se restaure sans y toucher.
            let mut ancienne = s.clone();
            ancienne.as_object_mut().unwrap().remove("consommation");
            assert_eq!(appel(&e, "POST", "/api/sauvegarde", None, Some(ancienne)).await.status(), StatusCode::OK);
            assert_eq!(consommation::lire_reglages(&e.config.donnees).retention_jours, 1);
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        #[tokio::test]
        async fn echecs_codes() {
            let e = etat("pilote-epson-echecs");
            let t = rendre(&e, pdf(false), Demande { dpi: 360, ..Default::default() }).await;

            // Personne au bout du port : échec, code stable.
            let ferme = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
            en_reseau(&e, ferme, None);
            let s = executer_sur_l_epson(&e, t.id).await;
            assert_eq!((s.etat, s.code_erreur.as_deref()), (file::Etat::Echec, Some("sortie_echouee")), "{:?}", s.erreur);

            // Imprimante en alerte (capot ouvert) : rien n'est envoyé.
            let agent = media::tests::agent_factice(media::tests::alertes_d_essai());
            en_reseau(&e, ferme, Some(agent));
            let s = executer_sur_l_epson(&e, t.id).await;
            assert_eq!((s.etat, s.code_erreur.as_deref()), (file::Etat::Echec, Some("imprimante_en_alerte")));
            assert!(s.erreur.as_deref().unwrap().contains("printer cover open"), "{:?}", s.erreur);

            // Blanc : refusé à l'entrée de la file, et par le pilote si la sortie y est déjà.
            en_reseau(&e, ferme, None);
            let t = rendre(&e, pdf(true), Demande { dpi: 360, ..Default::default() }).await;
            assert!(t.pages[0].plaques.contains(&"White".to_string()), "{:?}", t.pages[0].plaques);
            let r = appel(&e, "POST", "/api/file", None, Some(serde_json::json!({ "travail": t.id, "page": 1, "machine": "epson" }))).await;
            assert_eq!(r.status(), StatusCode::CONFLICT);
            let corps: serde_json::Value = serde_json::from_slice(&axum::body::to_bytes(r.into_body(), 1 << 20).await.unwrap()).unwrap();
            assert_eq!(corps["code"], "envoi_impossible", "{corps}");
            let s = executer_sur_l_epson(&e, t.id).await;
            assert_eq!((s.etat, s.code_erreur.as_deref()), (file::Etat::Echec, Some("envoi_impossible")));
            // En dossier, le blanc reste admis.
            e.machines.write().unwrap()[0].connexion = machines::Connexion::Dossier;
            let r = appel(&e, "POST", "/api/file", None, Some(serde_json::json!({ "travail": t.id, "page": 1, "machine": "epson" }))).await;
            assert_eq!(r.status(), StatusCode::CREATED);
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }
    }

    /// Finitions de l'éditeur (roadmap 3.D) par le vrai circuit : création du travail,
    /// édition, mosaïque, étiquette, repérage, rendu et découpe.
    mod finitions {
        use super::controle_d_acces::{appel, etat};
        use super::regression_geometrie::rendre;
        use super::*;

        const MM: f64 = imbrication::PT_PAR_MM;

        fn codes(t: &Travail) -> Vec<&str> {
            t.alertes.iter().map(|a| a.code.as_str()).collect()
        }

        /// 3.D1 et 3.D2 — repérage Kona : les traits de coupe sont neutralisés (et la marge qui
        /// les logeait), une page de 200 mm lève les deux alertes de la consigne ; à 300 mm,
        /// seul le blanc reste signalé ; reconstruire ne les double pas.
        #[tokio::test]
        async fn kona_neutralise_les_traits_et_signale_la_consigne() {
            let e = etat("finitions-kona");
            let edition = edition::Edition { marques: Some(edition::Marques::default()), ..Default::default() };
            let d = Demande { dpi: 36, rotation_auto: false, reperage: reperage::Reperage::Cadre, edition: edition.clone(), ..Default::default() };
            let t = rendre(&e, imbrication::tests::pdf(200.0 * MM, 150.0 * MM, "1 0 0 0", (0.0, 0.0)), d.clone()).await;
            assert_eq!(codes(&t), ["marques_neutralisees", "reperes_image_petite"], "{:?}", t.alertes);
            assert_eq!(t.alertes[1].detail, "200 × 150 mm");
            assert_eq!(t.pages[0].geometrie.unwrap().bords_mm, [0.0; 4], "ni traits ni marge pour eux");
            let source = std::fs::read(dossier(&e.config.donnees, t.id).join("source.pdf")).unwrap();
            assert!((imbrication::taille_page(&source, 1).unwrap().largeur / MM - 200.0).abs() < 1e-3, "page non agrandie");
            // Reconstruire : mêmes alertes, une fois chacune.
            let r = appel(&e, "POST", &format!("/api/travaux/{}/reglages", t.id), None, Some(serde_json::to_value(&d).unwrap())).await;
            assert!(r.status().is_success(), "{}", r.status());
            let t = lire_travail(&e.config.donnees, t.id).unwrap();
            assert_eq!(codes(&t), ["marques_neutralisees", "reperes_image_petite"]);
            let t2 = rendre(&e, imbrication::tests::pdf(300.0 * MM, 260.0 * MM, "1 0 0 0", (0.0, 0.0)), Demande { reperage: reperage::Reperage::Manuel, edition: edition::Edition::default(), ..d }).await;
            assert!(codes(&t2).is_empty(), "{:?}", codes(&t2));
            // Sans repérage, les traits restent et la marge s'agrandit pour eux.
            let t3 = rendre(&e, imbrication::tests::pdf(200.0 * MM, 150.0 * MM, "1 0 0 0", (0.0, 0.0)), Demande { dpi: 36, rotation_auto: false, edition, ..Default::default() }).await;
            assert!(t3.alertes.is_empty() && (t3.pages[0].geometrie.unwrap().bords_mm[0] - 8.25).abs() < 1e-9);
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// 3.D2 — l'aperçu des repères pour l'éditeur : rectangles, place de l'image, alertes.
        #[tokio::test]
        async fn apercu_des_reperes_pour_l_editeur() {
            let e = etat("finitions-apercu");
            let r = appel(&e, "POST", "/api/reperage/apercu", None, Some(serde_json::json!({ "reperage": "manuel", "largeur_mm": 200.0, "hauteur_mm": 150.0 }))).await;
            assert_eq!(r.status(), StatusCode::OK);
            let v: serde_json::Value = serde_json::from_slice(&axum::body::to_bytes(r.into_body(), 1 << 20).await.unwrap()).unwrap();
            assert_eq!(v["rectangles_mm"].as_array().unwrap().len(), 8);
            assert!((v["decalage_mm"][0].as_f64().unwrap() - 35.0).abs() < 1e-9, "{v}");
            let alertes: Vec<&str> = v["alertes"].as_array().unwrap().iter().map(|a| a["code"].as_str().unwrap()).collect();
            assert_eq!(alertes, ["reperes_image_petite"]);
            let r = appel(&e, "POST", "/api/reperage/apercu", None, Some(serde_json::json!({ "reperage": "cadre", "largeur_mm": 0.0, "hauteur_mm": 150.0 }))).await;
            assert_eq!(r.status(), StatusCode::BAD_REQUEST);
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// 3.D5 et 3.D6 — bannière de 500 × 100 mm en deux lés : chaque lé porte son étiquette
        /// (nom, L × H, numéro de lé) et son QR code ; le contour du code s'ajoute à la découpe
        /// de chaque lé, et le QR rendu sur la plaque noire est celui de la chaîne attendue.
        #[tokio::test]
        async fn etiquette_et_qr_sur_chaque_le() {
            let e = etat("finitions-etiquette");
            let code = edition::CodeMarge { contour: true, ..Default::default() };
            let edition = edition::Edition { etiquette: Some(edition::Etiquette::default()), code: Some(code), ..Default::default() };
            let dpi = 254;
            let d = Demande { dpi, rotation_auto: false, edition: edition.clone(), mosaique: Some(Mosaique { laize_mm: 300.0, recouvrement_mm: 20.0, ..Default::default() }), ..Default::default() };
            let t = rendre(&e, imbrication::tests::pdf(500.0 * MM, 100.0 * MM, "1 0 0 0", (0.0, 0.0)), d).await;
            let rep = dossier(&e.config.donnees, t.id);
            let source = std::fs::read(rep.join("source.pdf")).unwrap();
            assert_eq!(t.pages.len(), 2);
            let marge = edition.marge_effective();
            for k in 1..=2 {
                let texte = etiquette::tests::textes(&source, k as u32);
                assert!(texte.contains("essai.pdf \u{b7} ") && texte.contains(&format!("500 \u{d7} 100 mm \u{b7} l\u{e9} {k}/2 \u{b7} copies")), "lé {k} : {texte}");
                // Contour de l'image (lé rogné) et contour du code.
                assert_eq!(t.pages[k - 1].traces_decoupe, 2, "lé {k}");
            }
            // QR du lé 2 (dernier : à 2 mm du bord droit de l'image), lu sur la plaque rendue.
            let (l, _) = (t.pages[1].largeur_mm, t.pages[1].hauteur_mm);
            let attendu = etiquette::modules(&format!("nom=essai.pdf;page=2;tuile=2/2;travail={}", t.id), code.niveau).unwrap();
            let noir = image::open(render::chemin_plaque(&rep, 2, "Black")).unwrap().to_luma8();
            let lire = |x: f64, y: f64| noir.get_pixel((x / 25.4 * dpi as f64) as u32, noir.height() - 1 - (y / 25.4 * dpi as f64) as u32)[0];
            let (x, y) = (l - marge - 2.0 - code.marge_mm - code.taille_mm, (marge - code.taille_mm) / 2.0);
            assert_eq!(etiquette::tests::lire_qr(lire, attendu.len(), (x, y, code.taille_mm)), attendu, "QR du lé 2");
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }
    }

    /// Filtres couleur (roadmap 3.E2) et remplacement de couleur (3.E3), mesurés dans les
    /// plaques, puis exportés vers un réglage et repris à l'import par son dossier surveillé.
    mod filtres {
        use super::controle_d_acces::{appel, etat};
        use super::regression_geometrie::rendre;
        use super::*;

        /// 100 × 50 mm : à gauche C 20 % M 60 %, à droite C 100 % M 50 %.
        fn deux_aplats() -> Vec<u8> {
            let contenu = "0.2 0.6 0 0 k 0 0 141.73 141.73 re f\n1 0.5 0 0 k 141.73 0 141.73 141.73 re f\n";
            let objets = [
                "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 283.46 141.73] /Contents 4 0 R >>".to_string(),
                format!("<< /Length {} >>\nstream\n{contenu}endstream", contenu.len()),
            ];
            let mut pdf = String::from("%PDF-1.4\n");
            let mut positions = vec![];
            for (i, o) in objets.iter().enumerate() {
                positions.push(pdf.len());
                pdf += &format!("{} 0 obj\n{o}\nendobj\n", i + 1);
            }
            let xref = pdf.len();
            pdf += &format!("xref\n0 {}\n0000000000 65535 f \n", objets.len() + 1);
            for p in positions {
                pdf += &format!("{p:010} 00000 n \n");
            }
            pdf += &format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objets.len() + 1);
            pdf.into_bytes()
        }

        /// Valeurs (0 = pleine encre) des quatre plaques au centre de la moitié gauche et de la droite.
        fn mesurer(chemin: impl Fn(&str) -> PathBuf) -> [[u8; 4]; 2] {
            let plaques = filtre::ENCRES.map(|e| image::open(chemin(e)).unwrap().to_luma8());
            let (l, h) = plaques[0].dimensions();
            [l / 4, 3 * l / 4].map(|x| std::array::from_fn(|i| plaques[i].get_pixel(x, h / 2)[0]))
        }

        fn filtre_essai() -> filtre::Filtre {
            filtre::Filtre {
                luminosite: 20.0,
                remplacements: vec![filtre::Remplacement { de: [100.0, 50.0, 0.0, 0.0], vers: [0.0, 100.0, 100.0, 0.0], tolerance: 1.0, actif: true }],
                ..Default::default()
            }
        }

        #[tokio::test]
        async fn filtre_mesure_dans_les_plaques_puis_exporte_et_repris_a_l_import() {
            let e = etat("filtres");
            let base = Demande { dpi: 36, rotation_auto: false, ..Default::default() };
            let brut = rendre(&e, deux_aplats(), base.clone()).await;
            let rep_brut = dossier(&e.config.donnees, brut.id);
            let [gauche, droite] = mesurer(|p| render::chemin_plaque(&rep_brut, 1, p));
            assert_eq!(gauche, [204, 102, 255, 255], "sans filtre : C 20 %, M 60 %");
            assert!(droite[0] == 0 && (127..=128).contains(&droite[1]) && droite[2..] == [255, 255], "sans filtre : {droite:?}");
            assert!(!render::chemin_composite_avant(&rep_brut, 1).exists(), "pas de vue « avant » sans filtre");

            let avec = Demande { edition: edition::Edition { filtre: Some(filtre_essai()), ..Default::default() }, ..base.clone() };
            let t = rendre(&e, deux_aplats(), avec).await;
            let rep = dossier(&e.config.donnees, t.id);
            let prod = mesurer(|p| render::chemin_plaque(&rep, 1, p));
            // Luminosité +20 : 80 % de l'encre (51 → 41, 153 → 122) ; aplat de droite remplacé (3.E3).
            assert_eq!(prod, [[214, 133, 255, 255], [255, 0, 0, 255]], "plaques de production filtrées");
            assert_eq!(mesurer(|p| render::chemin_apercu_plaque(&rep, 1, p)), prod, "aperçu = production");
            // Vue scindée : le composite d'origine est gardé, le filtré en diffère.
            let (avant, apres) = (image::open(render::chemin_composite_avant(&rep, 1)).unwrap().to_rgb8(), image::open(render::chemin_composite(&rep, 1)).unwrap().to_rgb8());
            assert_eq!(avant, image::open(render::chemin_composite(&rep_brut, 1)).unwrap().to_rgb8(), "avant = rendu sans filtre");
            let (x, y) = (3 * avant.width() / 4, avant.height() / 2);
            assert!(apres.get_pixel(x, y)[0] > 200 && apres.get_pixel(x, y)[1] < 80, "après : l'aplat de droite vire au rouge {:?}", apres.get_pixel(x, y));
            let r = appel(&e, "GET", &format!("/api/travaux/{}/pages/1/apercu-avant.png", t.id), None, None).await;
            assert_eq!(r.status(), StatusCode::OK);
            assert_eq!(appel(&e, "GET", &format!("/api/travaux/{}/pages/1/apercu-avant.png", brut.id), None, None).await.status(), StatusCode::NOT_FOUND);

            // « Exporter vers un réglage » : le filtre du travail rejoint le réglage existant.
            let mut reglage = serde_json::json!({ "nom": "Vinyle", "demande": base });
            assert_eq!(appel(&e, "POST", "/api/reglages", None, Some(reglage.clone())).await.status(), StatusCode::CREATED);
            reglage["demande"]["edition"]["filtre"] = serde_json::to_value(&t.demande.edition.filtre).unwrap();
            assert_eq!(appel(&e, "POST", "/api/reglages", None, Some(reglage)).await.status(), StatusCode::CREATED);
            let r = reglages::lire(&e.config.donnees).into_iter().find(|r| r.nom == "Vinyle").unwrap();
            assert_eq!(r.demande.edition.filtre, Some(filtre_essai()));

            // Import par le dossier surveillé du réglage : même rendu, au pixel près.
            let fichier = reglages::dossier_entree(&e.config.donnees, "Vinyle").join("arrivee.pdf");
            std::fs::write(&fichier, deux_aplats()).unwrap();
            prendre_fichier(&e, &fichier, r.demande, Fiche::default(), vec![]).await.unwrap();
            let travaux = dossier(&e.config.donnees, Uuid::nil()).parent().unwrap().to_path_buf();
            let id = std::fs::read_dir(&travaux).unwrap().flatten().filter_map(|d| d.file_name().to_str()?.parse::<Uuid>().ok()).find(|i| ![brut.id, t.id].contains(i)).unwrap();
            let mut fini = None;
            for _ in 0..600 {
                let i = lire_travail(&e.config.donnees, id).unwrap();
                if !actif(&i.statut) {
                    fini = Some(i);
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            assert!(matches!(fini.map(|i| i.statut), Some(Statut::Termine)));
            let rep2 = dossier(&e.config.donnees, id);
            for encre in filtre::ENCRES {
                let (a, b) = (image::open(render::chemin_plaque(&rep, 1, encre)).unwrap().to_luma8(), image::open(render::chemin_plaque(&rep2, 1, encre)).unwrap().to_luma8());
                assert_eq!(a, b, "plaque {encre} : rendu repris à l'import différent");
            }
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// Découpe seule : pas de filtre (rien n'est imprimé) ; un filtre hors bornes est refusé.
        #[test]
        fn decoupe_seule_et_bornes() {
            let d = Demande { mode: Mode::Decouper, edition: edition::Edition { filtre: Some(filtre_essai()), ..Default::default() }, ..Default::default() }.normaliser();
            assert_eq!(d.edition.filtre, None);
            let neutre = edition::Edition { filtre: Some(filtre_essai()), ..Default::default() };
            assert!(neutre.neutre(), "le filtre ne touche pas au PDF");
            let mauvais = Demande { edition: edition::Edition { filtre: Some(filtre::Filtre { saturation: 150.0, ..Default::default() }), ..Default::default() }, ..Default::default() };
            assert_eq!(mauvais.verifier(&[]), Err("filtre_invalide"));
        }
    }

    /// Couches de découpe, préfixes, coupe après impression, pré-contrôle (roadmap 4.1 à
    /// 4.6, 4.16).
    mod couches_decoupe {
        use super::controle_d_acces::{appel, etat};
        use super::regression_geometrie::rendre;
        use super::*;
        use lopdf::{Document, Object, Stream, dictionary};

        /// PDF 600 × 300 pt : un carré de 100 pt par ton, le i-ième en x = 50 + 150·i, y = 50.
        pub(super) fn pdf_tons(tons: &[&str]) -> Vec<u8> {
            let mut doc = Document::with_version("1.4");
            let mut espaces = lopdf::Dictionary::new();
            let mut contenu = String::new();
            for (i, ton) in tons.iter().enumerate() {
                espaces.set(format!("CS{i}"), Object::Array(vec!["Separation".into(), Object::Name(ton.as_bytes().to_vec()), "DeviceCMYK".into(), Object::Dictionary(dictionary! { "FunctionType" => 2, "Domain" => vec![0.into(), 1.into()], "C0" => vec![0.into(); 4], "C1" => vec![0.into(), 1.into(), 0.into(), 0.into()], "N" => 1 })]));
                contenu += &format!("/CS{i} CS 1 SCN {} 50 100 100 re S ", 50 + 150 * i);
            }
            let c = doc.add_object(Stream::new(dictionary! {}, contenu.into_bytes()));
            let parent = doc.new_object_id();
            let page = doc.add_object(dictionary! { "Type" => "Page", "Parent" => parent, "MediaBox" => vec![0.into(), 0.into(), 600.into(), 300.into()], "Contents" => c, "Resources" => dictionary! { "ColorSpace" => espaces } });
            doc.objects.insert(parent, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }));
            let cat = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => parent });
            doc.trailer.set("Root", cat);
            let mut o = vec![];
            doc.save_to(&mut o).unwrap();
            o
        }

        /// Le serveur relu depuis son dossier de données, comme après un redémarrage.
        fn redemarrer(e: &Partage) -> Partage {
            let machines = machines::lire(&e.config.donnees).unwrap_or_else(|| e.machines.read().unwrap().clone());
            Arc::new(Etat { config: e.config.clone(), rendu: rendus::Rendus::new(1), file: std::sync::Mutex::new(file::File::lire(&e.config.donnees)), reglages: Default::default(), sessions: Default::default(), machines: std::sync::RwLock::new(machines) })
        }

        pub(super) async fn json(r: Response) -> serde_json::Value {
            serde_json::from_slice(&axum::body::to_bytes(r.into_body(), 1 << 22).await.unwrap()).unwrap()
        }

        pub(super) fn reglage(nom: &str, force_g: f64) -> machines::ParametreDecoupe {
            machines::ParametreDecoupe { uuid: None, nom: nom.into(), vitesse_cm_s: 15.0, force_g, acceleration_g: 1.0, valeurs_machine: false, outil: None, motif_mm: vec![], diffusion: false }
        }

        fn noms(t: &Travail) -> Vec<(String, bool)> {
            t.couches_decoupe.iter().map(|c| (c.nom.clone(), c.active)).collect()
        }

        /// 4.2 à 4.4 : PDF à deux tons → deux couches ; réglage « Kisscut » appliqué à la
        /// couche Kisscut, défaut machine à l'autre, un bloc AS/ZF/VS par couche ; ordre
        /// inversé et couche décochée enregistrés, gardés au redémarrage et au rendu suivant.
        #[tokio::test]
        async fn deux_tons_deux_couches_reglees_et_gardees() {
            let e = etat("couches");
            let t = rendre(&e, pdf_tons(&["Thrucut", "Kisscut"]), Demande { mode: Mode::Decouper, ..Default::default() }).await;
            assert_eq!(noms(&t), [("Kisscut".into(), true), ("Thrucut".into(), true)], "mi-chair d'abord");
            assert_eq!(t.pages[0].traces_par_couche, std::collections::BTreeMap::from([("Kisscut".into(), 1), ("Thrucut".into(), 1)]));
            let rep = dossier(&e.config.donnees, t.id);
            let traces = lire_traces(&rep, 1).unwrap();
            let mut kona = e.machines.read().unwrap()[1].clone();
            kona.parametres_decoupe = vec![reglage("Défaut atelier", 50.0), reglage("Kisscut", 100.0)];
            let k = pour_la_machine(&traces, &t, &t.pages[0], &kona, None).unwrap();
            // Kona : X = y, Y = −x ; Kisscut en x = 200 pt (7056), Thrucut en x = 50 pt (1764).
            let ou = |motif: &str| k.find(motif).unwrap_or_else(|| panic!("{motif} absent : {k}"));
            assert_eq!(k.matches("AS 1.0;").count(), 2, "un bloc de réglages par couche : {k}");
            assert!(ou("ZF 100;") < ou("PU1764,-7056;") && ou("PU1764,-7056;") < ou("ZF 50;") && ou("ZF 50;") < ou("PU1764,-1764;"), "{k}");
            // Réglage choisi à la main mais absent de la machine : refusé, pas remplacé en silence.
            let mut choisi = t.clone();
            choisi.couches_decoupe[1].reglage = Some("Inexistant".into());
            assert!(pour_la_machine(&traces, &choisi, &t.pages[0], &kona, None).is_err());

            // Thrucut d'abord, Kisscut décochée : enregistré, fichier de coupe refait.
            let mut couches = t.couches_decoupe.clone();
            couches.reverse();
            couches[1].active = false;
            let r = appel(&e, "POST", &format!("/api/travaux/{}/couches", t.id), None, Some(serde_json::to_value(&couches).unwrap())).await;
            assert_eq!(r.status(), StatusCode::OK);
            let plt = std::fs::read_to_string(fichier_plt(&rep, 1)).unwrap();
            assert!(plt.contains("PU1764,1764;") && !plt.contains("PU7056,1764;"), "{plt}");
            let mut fausse = couches.clone();
            fausse[0].nom = "Inventée".into();
            let r = appel(&e, "POST", &format!("/api/travaux/{}/couches", t.id), None, Some(serde_json::to_value(&fausse).unwrap())).await;
            assert_eq!(r.status(), StatusCode::BAD_REQUEST);
            assert_eq!(json(r).await["code"], "couches_invalides");

            // Redémarrage : relu du disque, rien de perdu.
            let e2 = redemarrer(&e);
            let liste = json(appel(&e2, "GET", "/api/travaux", None, None).await).await;
            let lu = liste.as_array().unwrap().iter().find(|x| x["id"] == serde_json::json!(t.id)).unwrap();
            assert_eq!(serde_json::from_value::<Vec<decoupe::Couche>>(lu["couches_decoupe"].clone()).unwrap(), couches);
            // Rendu refait : les choix restent.
            assert_eq!(appel(&e2, "POST", &format!("/api/travaux/{}/relancer", t.id), None, None).await.status(), StatusCode::ACCEPTED);
            for _ in 0..200 {
                if lire_travail(&e2.config.donnees, t.id).is_some_and(|x| !actif(&x.statut)) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            assert_eq!(lire_travail(&e2.config.donnees, t.id).unwrap().couches_decoupe, couches);
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// Travail rendu à la main : une page, un tracé CutContour, découpeuse « fc7000 ».
        fn travail_a_couper(d: &std::path::Path) -> Travail {
            let t = Travail {
                id: Uuid::new_v4(),
                nom: "essai.pdf".into(),
                format: Format::Pdf,
                demande: Demande { decoupeuse: Some("fc7000".into()), ..Default::default() },
                statut: Statut::Termine,
                code_erreur: None,
                erreur: None,
                pages: vec![Page { plaques: vec!["Cyan".into()], traces_decoupe: 1, traces_par_couche: [("CutContour".to_string(), 1)].into(), largeur_mm: 100.0, ..Default::default() }],
                alertes: vec![],
                cree_le: 0,
                couches_decoupe: vec![decoupe::Couche { nom: "CutContour".into(), active: true, reglage: None }],
                mise_coupe: Default::default(),
                fiche: Default::default(),
                pages_exclues: vec![],
                imbrication: None,
                erp: None,
                retenue_erp: None,
                estimation: false,
            };
            let rep = dossier(d, t.id);
            std::fs::create_dir_all(rep.join("prod")).unwrap();
            std::fs::write(fichier_traces(&rep, 1), serde_json::to_vec(&vec![decoupe::Trace { ton: "CutContour".into(), points: vec![(0.0, 0.0), (72.0, 0.0)], ferme: false }]).unwrap()).unwrap();
            ecrire(d, &t).unwrap();
            t
        }

        /// 4.16 : découpeuse affectée absente, mal connectée ou sans réglage → l'impression est
        /// retenue avant tout envoi ; rétablie → elle part. 4.6 : la coupe attend l'impression
        /// terminée.
        #[test]
        fn impression_retenue_puis_coupe_apres_impression() {
            let e = etat("precontrole");
            let d = e.config.donnees.clone();
            let mut t = travail_a_couper(&d);
            let catalogue = e.machines.read().unwrap().clone();
            let epson = machines::Machine { connexion: machines::Connexion::Dossier, ..catalogue[0].clone() };
            let fc7000 = machines::Machine { id: "fc7000".into(), modele: machines::Modele::GraphtecFc7000, connexion: machines::Connexion::Reseau { hote: "127.0.0.1".into(), port: 9100 }, parametres_decoupe: vec![], ..catalogue[1].clone() };
            let imp = file::Sortie::nouvelle("epson", machines::Operation::Impression, t.id, &t.nom, 1, 0);
            let sorties = vec![imp.clone()];
            let raison = |cat: &[machines::Machine]| raison_attente(&d, &imp, Some(&epson), None, &sorties, cat);
            assert_eq!(raison(&catalogue), Some("decoupeuse_indisponible"), "découpeuse absente du catalogue");
            let mut avec = catalogue.clone();
            avec.push(fc7000.clone());
            assert_eq!(raison(&avec), None, "découpeuse rétablie : l'impression part");
            let mut f = file::File { sorties: sorties.clone() };
            assert_eq!(f.prochaine_si("epson", |s| raison_attente(&d, s, Some(&epson), None, &sorties, &catalogue).is_none()), None, "rien n'est envoyé");
            assert_eq!(f.prochaine_si("epson", |s| raison_attente(&d, s, Some(&epson), None, &sorties, &avec).is_none()), Some(0));
            let sans_adresse = machines::Machine { connexion: machines::Connexion::Reseau { hote: String::new(), port: 9100 }, ..fc7000.clone() };
            assert_eq!(raison(&[catalogue[0].clone(), sans_adresse]), Some("decoupeuse_indisponible"), "connexion invalide");
            let hors_ligne = machines::Machine { connexion: machines::Connexion::Serie { port: "/dev/inexistant".into(), stty: String::new() }, ..fc7000.clone() };
            assert_eq!(raison(&[catalogue[0].clone(), hors_ligne]), Some("decoupeuse_indisponible"), "hors ligne");
            t.couches_decoupe[0].reglage = Some("Vinyle épais".into());
            ecrire(&d, &t).unwrap();
            assert_eq!(raison(&avec), Some("decoupeuse_indisponible"), "couche sans son réglage");
            let reglee = machines::Machine { parametres_decoupe: vec![machines::ParametreDecoupe { uuid: None, nom: "Vinyle épais".into(), vitesse_cm_s: 10.0, force_g: 200.0, acceleration_g: 1.0, valeurs_machine: false, outil: None, motif_mm: vec![], diffusion: false }], ..fc7000.clone() };
            assert_eq!(raison(&[catalogue[0].clone(), reglee.clone()]), None);
            f.sorties[0].etat = file::Etat::Echec;
            t.couches_decoupe[0].active = false;
            ecrire(&d, &t).unwrap();
            assert_eq!(raison(&catalogue), None, "rien à couper : pas de contrôle");

            // 4.6 : la coupe attend la fin de l'impression de la page.
            t.couches_decoupe[0].active = true;
            ecrire(&d, &t).unwrap();
            let coupe = file::Sortie::nouvelle("fc7000", machines::Operation::Decoupe, t.id, &t.nom, 1, 0);
            for (etat, attendu) in [(file::Etat::EnAttente, Some("attente_impression")), (file::Etat::EnCours, Some("attente_impression")), (file::Etat::Echec, Some("attente_impression")), (file::Etat::Terminee, None), (file::Etat::Annulee, None)] {
                let file_ = [file::Sortie { etat, ..imp.clone() }, coupe.clone()];
                assert_eq!(raison_attente(&d, &coupe, Some(&reglee), None, &file_, std::slice::from_ref(&reglee)), attendu, "impression {etat:?}");
            }
            assert_eq!(raison_attente(&d, &coupe, Some(&reglee), None, std::slice::from_ref(&coupe), &[]), None, "découpe seule : rien à attendre");
            std::fs::remove_dir_all(&d).unwrap();
        }

        /// 4.3 mono-couche et 4.6 dossier : une sortie par couche, la suivante retenue ; le
        /// fichier de la première couche seule est déposé d'un coup dans le dossier.
        #[tokio::test]
        async fn mono_couche_et_depot_atomique_en_dossier() {
            let e = etat("mono");
            let donnees = e.config.donnees.clone();
            let mut catalogue = e.machines.read().unwrap().clone();
            catalogue.push(machines::Machine { id: "graphtec".into(), uuid: None, modele: machines::Modele::GraphtecFc7000, connexion: machines::Connexion::Dossier, mono_couche: true, parametres_decoupe: vec![], ..catalogue[1].clone() });
            assert_eq!(appel(&e, "POST", "/api/machines", None, Some(serde_json::to_value(&catalogue).unwrap())).await.status(), StatusCode::OK);
            let t = rendre(&e, pdf_tons(&["Thrucut", "Kisscut"]), Demande { mode: Mode::Decouper, ..Default::default() }).await;
            let r = appel(&e, "POST", "/api/file", None, Some(serde_json::json!({ "travail": t.id, "page": 1, "machine": "graphtec" }))).await;
            assert_eq!(r.status(), StatusCode::CREATED);
            let vues: Vec<(Option<String>, file::Etat)> = e.file.lock().unwrap().sorties.iter().map(|s| (s.couche.clone(), s.etat)).collect();
            assert_eq!(vues, [(Some("Kisscut".into()), file::Etat::EnAttente), (Some("Thrucut".into()), file::Etat::Suspendue)]);
            tokio::spawn(repartir(e.clone()));
            for _ in 0..100 {
                if e.file.lock().unwrap().sorties[0].etat == file::Etat::Terminee {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            assert_eq!(e.file.lock().unwrap().sorties[0].etat, file::Etat::Terminee);
            let depot: Vec<PathBuf> = std::fs::read_dir(donnees.join("sorties").join("graphtec")).unwrap().map(|x| x.unwrap().path()).collect();
            assert_eq!(depot.len(), 1, "un seul fichier, pas de temporaire : {depot:?}");
            assert!(depot[0].extension().is_some_and(|x| x == "plt"));
            // Pas de 0,025 mm : Kisscut en (200, 50) pt → (2822, 706) ; Thrucut (50, 50) → (706, 706).
            let texte = std::fs::read_to_string(&depot[0]).unwrap();
            assert!(texte.starts_with("IN;SP1;PU2822,706;") && !texte.contains("PU706,706;"), "{texte}");
            assert_eq!(e.file.lock().unwrap().sorties[1].etat, file::Etat::Suspendue, "la couche suivante attend l'opérateur");
            std::fs::remove_dir_all(&donnees).unwrap();
        }

        /// 4.1 et 4.5 : préfixes réglés depuis Système (gardés sur le disque) ; préfixe absent
        /// alors qu'une découpeuse est choisie → alerte qui nomme les tons du fichier.
        #[tokio::test]
        async fn prefixes_reglables_et_prefixe_absent_signale() {
            let _g = super::DECOUPE_GLOBALE.lock().await;
            let e = etat("prefixes");
            let mut avec_die = decoupe::Reglages::default();
            // Les préfixes sont globaux au processus : la liste d'origine reste dedans, les
            // autres tests ne voient pas la différence.
            avec_die.prefixes.push(" Die ".into());
            let r = appel(&e, "POST", "/api/decoupe", None, Some(serde_json::to_value(&avec_die).unwrap())).await;
            assert_eq!(r.status(), StatusCode::OK);
            assert_eq!(decoupe::lire(&e.config.donnees).prefixes.last().unwrap(), "Die", "enregistré, espaces ôtés");
            let lu = json(appel(&e, "GET", "/api/decoupe", None, None).await).await;
            assert_eq!(lu["prefixes"].as_array().unwrap().len(), 9);
            let r = appel(&e, "POST", "/api/decoupe", None, Some(serde_json::json!({ "prefixes": [" "] }))).await;
            assert_eq!(json(r).await["code"], "prefixes_invalides");

            let d = Demande { mode: Mode::Decouper, decoupeuse: Some("kona".into()), ..Default::default() };
            let t = rendre(&e, pdf_tons(&["1CutContour"]), d.clone()).await;
            assert_eq!(t.pages[0].traces_decoupe, 0, "1CutContour n'est pas reconnu");
            let a = t.alertes.iter().find(|a| a.code == "decoupe_sans_prefixe").expect("alerte préfixe absent");
            assert_eq!(a.detail, "1CutContour");
            let t = rendre(&e, pdf_tons(&["Die-Cut 3"]), d).await;
            assert_eq!(t.pages[0].traces_decoupe, 1, "préfixe ajouté reconnu");
            assert!(t.alertes.is_empty(), "{:?}", t.alertes.iter().map(|a| &a.code).collect::<Vec<_>>());
            // Impression : un ton ni encre, ni découpe, ni dans la table est signalé et journalisé.
            let t = rendre(&e, pdf_tons(&["PANTONE 485 C"]), Demande { mode: Mode::Imprimer, ..Default::default() }).await;
            let absent = t.alertes.iter().find(|a| a.code == "ton_direct_absent").expect("ton absent de la table signalé");
            assert_eq!((absent.page, absent.detail.as_str()), (Some(1), "PANTONE 485 C"));
            assert_eq!(tons::lire_inconnus(&e.config.donnees)["PANTONE 485 C"].travaux, 1, "gardé au journal");
            decoupe::installer(decoupe::Reglages::default());
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }
    
        /// Travail rendu avant les couches : ni JSON des tracés ni couches, seulement son
        /// `.plt`. Sa découpe mise en file part (pas de « Fichier manquant ») et la machine
        /// reçoit le HP-GL qu'envoyait l'ancienne version ; le JSON est écrit au passage.
        #[tokio::test]
        async fn travail_d_avant_les_couches_se_decoupe_encore() {
            let e = etat("ancien-format");
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
            catalogue.push(machines::Machine { id: "fc7000".into(), uuid: None, modele: machines::Modele::GraphtecFc7000, connexion: machines::Connexion::Reseau { hote: "127.0.0.1".into(), port }, parametres_decoupe: vec![], ..catalogue[1].clone() });
            assert_eq!(appel(&e, "POST", "/api/machines", None, Some(serde_json::to_value(&catalogue).unwrap())).await.status(), StatusCode::OK);
            // job.json tel que l'écrivait la 0.19 : pas de couches, pas de tracés par couche.
            let id = Uuid::new_v4();
            let rep = dossier(&donnees, id);
            std::fs::create_dir_all(rep.join("prod")).unwrap();
            let ancien = serde_json::json!({ "id": id, "nom": "ancien.pdf", "mode": "decouper", "statut": "termine", "erreur": null, "cree_le": 0, "parametre_decoupe": null,
                "pages": [{ "plaques": [], "traces_decoupe": 2, "largeur_mm": 100.0, "hauteur_mm": 100.0 }] });
            std::fs::write(rep.join("job.json"), ancien.to_string()).unwrap();
            std::fs::write(fichier_plt(&rep, 1), "IN;SP2;PU0,0;PD0,2540;SP1;PU0,0;PD2540,0,2540,2540,0,0;PU;SP0;").unwrap();
            assert!(!fichier_traces(&rep, 1).exists());
            let r = appel(&e, "POST", "/api/file", None, Some(serde_json::json!({ "travail": id, "page": 1, "machine": "fc7000" }))).await;
            assert_eq!(r.status(), StatusCode::CREATED);
            let file_ = json(appel(&e, "GET", "/api/file", None, None).await).await;
            assert_eq!(file_[0]["raison_attente"], serde_json::Value::Null, "pas de « Fichier manquant » : {file_}");
            tokio::spawn(repartir(e.clone()));
            let recu = tokio::time::timeout(Duration::from_secs(10), tokio::task::spawn_blocking(move || recu.join().unwrap())).await.expect("rien reçu en 10 s").unwrap();
            // Au pas de 0,025 mm, comme l'ancienne version : rainage puis coupe.
            assert_eq!(recu, "IN;SP2;PU0,0;PD0,1016;SP1;PU0,0;PD1016,0,1016,1016,0,0;PU;SP0;");
            assert_eq!(lire_traces(&rep, 1).unwrap().len(), 2);
            assert!(fichier_traces(&rep, 1).exists(), "JSON écrit pour la fois suivante");
            std::fs::remove_dir_all(&donnees).unwrap();
        }

        /// Ancien réglage du travail entier : porté sur la couche du même nom au rendu ;
        /// ambigu, il reste dans le travail (montré à l'écran) jusqu'au choix des couches.
        #[tokio::test]
        async fn ancien_reglage_de_coupe_au_rendu() {
            let e = etat("ancien-reglage");
            let d = |n: &str| Demande { mode: Mode::Decouper, parametre_decoupe: Some(n.into()), ..Default::default() };
            let t = rendre(&e, pdf_tons(&["Thrucut", "Kisscut"]), d("Kisscut")).await;
            assert_eq!(t.couches_decoupe[0].reglage.as_deref(), Some("Kisscut"));
            assert_eq!(t.demande.parametre_decoupe, None, "placé : plus gardé à part");
            let t = rendre(&e, pdf_tons(&["Thrucut", "Kisscut"]), d("Mi-chair V3000")).await;
            assert!(t.couches_decoupe.iter().all(|c| c.reglage.is_none()));
            assert_eq!(t.demande.parametre_decoupe.as_deref(), Some("Mi-chair V3000"), "ambigu : gardé");
            let r = appel(&e, "POST", &format!("/api/travaux/{}/couches", t.id), None, Some(serde_json::to_value(&t.couches_decoupe).unwrap())).await;
            assert_eq!(r.status(), StatusCode::OK);
            assert_eq!(lire_travail(&e.config.donnees, t.id).unwrap().demande.parametre_decoupe, None, "tranché par l'enregistrement des couches");
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }
}
    /// Mise en coupe (roadmap 4.8, 4.9) : rotation, flèche du sens d'avance, découpe seule
    /// d'un SVG en copies.
    mod mise_en_coupe {
        use super::controle_d_acces::{appel, etat};
        use super::couches_decoupe::{json, pdf_tons};
        use super::regression_geometrie::{rendre, rendre_format};
        use super::*;

        /// Boîtes [x0, y0, x1, y1] en mm de chaque tracé d'un HP-GL (un `PU` par tracé).
        fn boites_hpgl(texte: &str) -> Vec<[f64; 4]> {
            decoupe::depuis_hpgl(texte).iter().map(|t| coupe::boite(std::slice::from_ref(t)).unwrap().map(|v| (v / imbrication::PT_PAR_MM * 100.0).round() / 100.0)).collect()
        }

        /// 4.8 : rotation de 90° enregistrée → `.plt` et aperçu refaits sans rendu, coordonnées
        /// attendues, flèche du sens d'avance ; gardée au redémarrage, refusée si invalide.
        #[tokio::test]
        async fn rotation_du_travail_de_coupe() {
            let e = etat("rotation-coupe");
            let t = rendre(&e, pdf_tons(&["CutContour"]), Demande { mode: Mode::Decouper, ..Default::default() }).await;
            assert_eq!(t.pages[0].taille_coupe_pt, Some([600.0, 300.0]));
            let rep = dossier(&e.config.donnees, t.id);
            assert!(std::fs::read_to_string(fichier_plt(&rep, 1)).unwrap().contains("PU1764,1764;"));
            let r = appel(&e, "POST", &format!("/api/travaux/{}/coupe", t.id), None, Some(serde_json::json!({ "rotation": 90 }))).await;
            assert_eq!(r.status(), StatusCode::OK);
            // Carré de 100 pt en (50, 50) sur 600 × 300 : tourné, de (150, 50) à (250, 150).
            let plt = std::fs::read_to_string(fichier_plt(&rep, 1)).unwrap();
            assert_eq!(plt, "IN;SP1;PU8819,1764;PD8819,5292,5292,5292,5292,1764,8819,1764;PU;SP0;");
            let svg = std::fs::read_to_string(fichier_svg(&rep, 1)).unwrap();
            assert!(svg.contains(r#"viewBox="0 0 300.0 600.0""#) && svg.contains(r#"class="sens-avance""#), "{svg}");
            // La machine reçoit la même chose (Graphtec au pas de 0,025 mm : 3528, 706, 2117).
            let lu = lire_travail(&e.config.donnees, t.id).unwrap();
            let fc = machines::Machine { modele: machines::Modele::GraphtecFc7000, pas_mm: None, ..e.machines.read().unwrap()[1].clone() };
            assert_eq!(pour_la_machine(&lire_traces(&rep, 1).unwrap(), &lu, &lu.pages[0], &fc, None).unwrap(), "IN;SP1;PU3528,706;PD3528,2117,2117,2117,2117,706,3528,706;PU;SP0;");
            assert_eq!(lu.mise_coupe.rotation, 90, "enregistrée dans job.json");
            let r = appel(&e, "POST", &format!("/api/travaux/{}/coupe", t.id), None, Some(serde_json::json!({ "rotation": 45 }))).await;
            assert_eq!(json(r).await["code"], "mise_coupe_invalide");
            // Impression puis coupe : la rotation oui, le miroir non (la coupe quitterait l'encre).
            let imprime = rendre(&e, pdf_tons(&["CutContour"]), Demande::default()).await;
            let r = appel(&e, "POST", &format!("/api/travaux/{}/coupe", imprime.id), None, Some(serde_json::json!({ "miroir": true }))).await;
            assert_eq!(json(r).await["code"], "mise_decoupe_seule");
            let r = appel(&e, "POST", &format!("/api/travaux/{}/coupe", imprime.id), None, Some(serde_json::json!({ "rotation": 180 }))).await;
            assert_eq!(r.status(), StatusCode::OK);
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// 4.9 : SVG en découpe seule, rectangle de 50 × 20 mm en (10, 10) depuis le haut ;
        /// 3 × 2 copies écartées de 5 mm → 6 contours aux bonnes positions ; puis départ à
        /// l'origine de la machine, miroir et échelle non proportionnelle.
        #[tokio::test]
        async fn svg_en_copies_avec_ecarts() {
            let e = etat("copies-svg");
            let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="50mm" viewBox="0 0 100 50"><g id="CutContour"><rect x="10" y="10" width="50" height="20"/></g></svg>"#;
            let t = rendre_format(&e, svg.as_bytes().to_vec(), Format::Svg, Demande { mode: Mode::Decouper, ..Default::default() }).await;
            let rep = dossier(&e.config.donnees, t.id);
            async fn poser_(e: &Partage, id: Uuid, rep: &std::path::Path, mise: serde_json::Value) -> Vec<[f64; 4]> {
                let r = appel(e, "POST", &format!("/api/travaux/{id}/coupe"), None, Some(mise)).await;
                assert_eq!(r.status(), StatusCode::OK);
                boites_hpgl(&std::fs::read_to_string(fichier_plt(rep, 1)).unwrap())
            }
            let poser = |mise| poser_(&e, t.id, &rep, mise);
            let mut b = poser(serde_json::json!({ "copies": [3, 2], "ecart_mm": [5, 5] })).await;
            b.sort_by(|p, q| (p[1], p[0]).partial_cmp(&(q[1], q[0])).unwrap());
            let mut attendu = vec![];
            for j in 0..2 {
                for i in 0..3 {
                    // Rectangle à 10–60 mm en x, 20–40 mm en y depuis le bas ; pas de 55 et 25 mm.
                    attendu.push([10.0 + 55.0 * i as f64, 20.0 + 25.0 * j as f64, 60.0 + 55.0 * i as f64, 40.0 + 25.0 * j as f64]);
                }
            }
            assert_eq!(b, attendu, "6 contours");
            let b = poser(serde_json::json!({ "copies": [2, 1], "ecart_mm": [5, 0], "depuis_origine": true })).await;
            assert_eq!(b, [[0.0, 0.0, 50.0, 20.0], [55.0, 0.0, 105.0, 20.0]], "départ à l'origine");
            let b = poser(serde_json::json!({ "miroir": true, "echelle": [200, 100] })).await;
            // Page 200 × 50 mm après échelle ; rectangle 20–120 mm, retourné : 80–180 mm.
            assert_eq!(b, [[80.0, 20.0, 180.0, 40.0]], "miroir et échelle non proportionnelle");
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// 4.10 : aplat cyan de 100 pt bordé d'un contour CutContour, rendu à 144 dpi ; étendre
        /// de 2 mm → l'encre de la plaque de production déborde de 2 mm (± 1 pixel) de chaque
        /// côté, l'aperçu aussi ; le contour, lui, reste où il est. Puis décaler la coupe de
        /// 1 mm : le contour grandit de 1 mm, l'encre ne bouge pas.
        #[tokio::test]
        async fn etendre_l_image_et_decaler_la_coupe() {
            use lopdf::{Document, Object, Stream, dictionary};
            let e = etat("etendre");
            let mut doc = Document::with_version("1.4");
            let sep = Object::Array(vec!["Separation".into(), "CutContour".into(), "DeviceCMYK".into(), Object::Dictionary(dictionary! { "FunctionType" => 2, "Domain" => vec![0.into(), 1.into()], "C0" => vec![0.into(); 4], "C1" => vec![0.into(), 1.into(), 0.into(), 0.into()], "N" => 1 })]);
            let c = doc.add_object(Stream::new(dictionary! {}, b"1 0 0 0 k 50 50 100 100 re f /CS0 CS 1 SCN 0.5 w 50 50 100 100 re S".to_vec()));
            let parent = doc.new_object_id();
            let page = doc.add_object(dictionary! { "Type" => "Page", "Parent" => parent, "MediaBox" => vec![0.into(), 0.into(), 200.into(), 200.into()], "Contents" => c, "Resources" => dictionary! { "ColorSpace" => dictionary! { "CS0" => sep } } });
            doc.objects.insert(parent, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }));
            let cat = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => parent });
            doc.trailer.set("Root", cat);
            let mut pdf = vec![];
            doc.save_to(&mut pdf).unwrap();
            let mm = |pt: f64| pt / imbrication::PT_PAR_MM;
            let (a, b) = (mm(50.0), mm(150.0));
            let sans = rendre(&e, pdf.clone(), Demande { dpi: 144, ..Default::default() }).await;
            let (_, cyan) = super::regression_geometrie::encre(&dossier(&e.config.donnees, sans.id), 1, "Cyan", 144);
            let px = 25.4 / 144.0;
            assert!(cyan.iter().zip([a, a, b, b]).all(|(x, y)| (x - y).abs() <= px), "sans extension : {cyan:?}");
            let d = Demande { dpi: 144, etendre_coupe: Some(coupe::Etendre { largeur_mm: 2.0, seuil_mm: 0.5 }), ..Default::default() };
            let t = rendre(&e, pdf, d).await;
            let rep = dossier(&e.config.donnees, t.id);
            let (_, cyan) = super::regression_geometrie::encre(&rep, 1, "Cyan", 144);
            assert!(cyan.iter().zip([a - 2.0, a - 2.0, b + 2.0, b + 2.0]).all(|(x, y)| (x - y).abs() <= px), "étendue de 2 mm : {cyan:?}");
            // Aperçu (50 dpi) : étendu aussi.
            let apercu = image::open(render::chemin_apercu_plaque(&rep, 1, "Cyan")).unwrap().to_luma8();
            let encres = (0..apercu.width()).filter(|&x| apercu.get_pixel(x, apercu.height() / 2)[0] < 128).count() as f64 * 25.4 / 50.0;
            assert!((encres - (b - a + 4.0)).abs() <= 2.0 * 25.4 / 50.0, "aperçu : {encres} mm");
            let contour = boites_hpgl(&std::fs::read_to_string(fichier_plt(&rep, 1)).unwrap());
            assert_eq!(contour, [[a, a, b, b].map(|v| (v * 100.0).round() / 100.0)], "le contour ne bouge pas");
            let r = appel(&e, "POST", &format!("/api/travaux/{}/coupe", t.id), None, Some(serde_json::json!({ "decalage_mm": 1.0 }))).await;
            assert_eq!(r.status(), StatusCode::OK);
            let contour = boites_hpgl(&std::fs::read_to_string(fichier_plt(&rep, 1)).unwrap());
            assert_eq!(contour, [[a - 1.0, a - 1.0, b + 1.0, b + 1.0].map(|v| (v * 100.0).round() / 100.0)], "coupe décalée de 1 mm");
            let mauvais = Demande { etendre_coupe: Some(coupe::Etendre { largeur_mm: 20.0, seuil_mm: 0.5 }), ..Default::default() };
            assert_eq!(mauvais.verifier(&[]), Err("etendre_invalide"));
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// 4.14 : page imprimée avec repérage manuel, disque cyan de 50 pt de rayon sans aucun
        /// ton de découpe. Aperçu : un contour ; accepté : une couche CutContour, un tracé
        /// fermé autour de l'encre de la plaque, écarté de 2 mm (± 0,3) ; retiré : plus rien.
        #[tokio::test]
        async fn detourage_d_une_page_rendue() {
            use lopdf::{Document, Object, Stream, dictionary};
            let e = etat("detourage");
            let k = 0.552_284_75 * 50.0;
            let disque = format!("1 0 0 0 k 150 100 m 150 {a} {b} 150 100 150 c {c} 150 50 {a} 50 100 c 50 {d} {c} 50 100 50 c {b} 50 150 {d} 150 100 c f", a = 100.0 + k, b = 100.0 + k, c = 100.0 - k, d = 100.0 - k);
            let mut doc = Document::with_version("1.4");
            let c = doc.add_object(Stream::new(dictionary! {}, disque.into_bytes()));
            let parent = doc.new_object_id();
            let page = doc.add_object(dictionary! { "Type" => "Page", "Parent" => parent, "MediaBox" => vec![0.into(), 0.into(), 200.into(), 200.into()], "Contents" => c });
            doc.objects.insert(parent, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }));
            let cat = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => parent });
            doc.trailer.set("Root", cat);
            let mut pdf = vec![];
            doc.save_to(&mut pdf).unwrap();
            let t = rendre(&e, pdf, Demande { dpi: 144, reperage: reperage::Reperage::Manuel, ..Default::default() }).await;
            assert_eq!(t.pages[0].traces_decoupe, 0);
            let url = format!("/api/travaux/{}/pages/1/detourage", t.id);
            let reglage = serde_json::json!({ "seuil": 250, "sans_interieurs": true, "aire_min_mm2": 1.0, "lissage": 1, "decalage_mm": 2.0 });
            let apercu = json(appel(&e, "POST", &url, None, Some(reglage.clone())).await).await;
            assert_eq!(apercu["contours"], 1, "{apercu}");
            assert!(apercu["apercu_svg"].as_str().unwrap().contains("<polygon points"));
            assert_eq!(lire_travail(&e.config.donnees, t.id).unwrap().pages[0].traces_decoupe, 0, "un aperçu ne change rien");
            let mut accepte = reglage.clone();
            accepte["accepter"] = true.into();
            let r = appel(&e, "POST", &url, None, Some(accepte)).await;
            assert_eq!(r.status(), StatusCode::OK);
            let lu = lire_travail(&e.config.donnees, t.id).unwrap();
            assert_eq!((lu.pages[0].traces_decoupe, lu.pages[0].detourage, lu.couches_decoupe[0].nom.as_str()), (1, 1, "CutContour"));
            let rep = dossier(&e.config.donnees, t.id);
            let coupe = boites_hpgl(&std::fs::read_to_string(fichier_plt(&rep, 1)).unwrap());
            let (_, cyan) = super::regression_geometrie::encre(&rep, 1, "Cyan", 144);
            assert_eq!(coupe.len(), 1);
            for (c, attendu) in coupe[0].iter().zip([cyan[0] - 2.0, cyan[1] - 2.0, cyan[2] + 2.0, cyan[3] + 2.0]) {
                assert!((c - attendu).abs() <= 0.3, "contour {:?}, encre {cyan:?}", coupe[0]);
            }
            let r = appel(&e, "DELETE", &url, None, None).await;
            assert_eq!(r.status(), StatusCode::OK);
            let lu = lire_travail(&e.config.donnees, t.id).unwrap();
            assert_eq!((lu.pages[0].traces_decoupe, lu.pages[0].detourage), (0, 0));
            assert!(!fichier_plt(&rep, 1).exists(), "plus de fichier de coupe");
            // Découpe seule : pas d'image à détourer.
            let seule = rendre(&e, pdf_tons(&["CutContour"]), Demande { mode: Mode::Decouper, ..Default::default() }).await;
            let r = appel(&e, "POST", &format!("/api/travaux/{}/pages/1/detourage", seule.id), None, Some(reglage)).await;
            assert_eq!(json(r).await["code"], "detourage_impossible");
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// 4.15 : variante d'une découpeuse (jeu de repères dupliqué). Elle reprend la machine
        /// d'origine (connexion, réglages) ; un travail qui lui est affecté prend son jeu de
        /// repères ; sa découpe passe par la machine d'origine. Variante d'une machine absente :
        /// refusée.
        #[tokio::test]
        async fn variante_de_machine() {
            let e = etat("variante");
            let mut catalogue = e.machines.read().unwrap().clone();
            let base = machines::Machine { id: "graphtec".into(), uuid: None, modele: machines::Modele::GraphtecFc7000, connexion: machines::Connexion::Dossier, parametres_decoupe: vec![super::couches_decoupe::reglage("Vinyle", 120.0)], ..catalogue[1].clone() };
            // La variante arrive avec une autre connexion et sans réglage : on n'en tient pas compte.
            let variante = machines::Machine { id: "graphtec-croix".into(), uuid: None, nom: "Graphtec (croix)".into(), variante_de: Some("graphtec".into()), reperage: Some(reperage::Reperage::Manuel), parametres_decoupe: vec![], ..catalogue[1].clone() };
            catalogue.extend([base, variante.clone()]);
            let mauvaise = [catalogue.clone(), vec![machines::Machine { id: "orpheline".into(), variante_de: Some("inconnue".into()), ..variante.clone() }]].concat();
            let r = appel(&e, "POST", "/api/machines", None, Some(serde_json::to_value(&mauvaise).unwrap())).await;
            assert_eq!(json(r).await["code"], "variante_invalide");
            assert_eq!(appel(&e, "POST", "/api/machines", None, Some(serde_json::to_value(&catalogue).unwrap())).await.status(), StatusCode::OK);
            let lu = e.machines.read().unwrap().iter().find(|m| m.id == "graphtec-croix").cloned().unwrap();
            assert_eq!((&lu.connexion, lu.modele, lu.parametres_decoupe.len(), lu.reperage), (&machines::Connexion::Dossier, machines::Modele::GraphtecFc7000, 1, Some(reperage::Reperage::Manuel)), "reprise de la machine d'origine");
            let origine = e.machines.read().unwrap().iter().find(|m| m.id == "graphtec").unwrap().parametres_decoupe[0].uuid;
            assert_ne!(lu.parametres_decoupe[0].uuid, origine, "ses réglages ont leurs propres identifiants");
            // Travail affecté à la variante, sans repérage choisi : il prend les croix.
            let t = rendre(&e, pdf_tons(&["CutContour"]), Demande { decoupeuse: Some("graphtec-croix".into()), ..Default::default() }).await;
            assert_eq!(t.pages[0].reperage, Some(reperage::Reperage::Manuel), "jeu de repères de la variante");
            let r = appel(&e, "POST", "/api/file", None, Some(serde_json::json!({ "travail": t.id, "page": 1, "machine": "graphtec-croix", "operation": "decoupe" }))).await;
            assert_eq!(r.status(), StatusCode::CREATED);
            tokio::spawn(repartir(e.clone()));
            for _ in 0..100 {
                if e.file.lock().unwrap().sorties[0].etat == file::Etat::Terminee {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            let s = e.file.lock().unwrap().sorties[0].clone();
            assert_eq!((s.etat, s.machine.as_str()), (file::Etat::Terminee, "graphtec"), "coupée par la machine d'origine");
            assert_eq!(std::fs::read_dir(e.config.donnees.join("sorties").join("graphtec")).unwrap().count(), 1);
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// Formats d'avant ces réglages : machine sans coupe à l'arrivée ni variante (coupe à
        /// l'arrivée active), réglages du travail sans extension ni fond sous repères, travail
        /// sans mise en coupe (neutre) ni format de coupe ni détourage.
        #[test]
        fn anciens_formats_lus() {
            let m: machines::Machine = serde_json::from_value(serde_json::json!({ "id": "kona", "nom": "Kona", "modele": "mutoh_kona", "laize_mm": 1650.0, "connexion": { "type": "dossier" } })).unwrap();
            assert!(m.coupe_auto && m.variante_de.is_none() && m.reperage.is_none());
            let d: Demande = serde_json::from_value(serde_json::json!({ "mode": "imprimer" })).unwrap();
            assert!(d.etendre_coupe.is_none() && d.fond_reperes.is_none());
            let t: Travail = serde_json::from_value(serde_json::json!({ "id": Uuid::nil(), "nom": "a.pdf", "statut": "termine", "erreur": null, "cree_le": 0, "pages": [{ "plaques": [], "traces_decoupe": 1 }] })).unwrap();
            assert_eq!(t.mise_coupe, coupe::Mise::default());
            assert_eq!((t.pages[0].taille_coupe_pt, t.pages[0].detourage), (None, 0));
        }

        /// 4.12 : découpeuse sans coupe à l'arrivée → la découpe arrive suspendue (« attente
        /// opérateur ») et ne part qu'une fois reprise ; avec, elle part seule. Archivée, elle
        /// se restaure suspendue sur la première, en attente sur la seconde.
        #[tokio::test]
        async fn coupe_a_l_arrivee_ou_sur_ordre() {
            let e = etat("coupe-auto");
            let mut catalogue = e.machines.read().unwrap().clone();
            let graphtec = |id: &str, coupe_auto| machines::Machine { id: id.into(), uuid: None, modele: machines::Modele::GraphtecFc7000, connexion: machines::Connexion::Dossier, parametres_decoupe: vec![], coupe_auto, ..catalogue[1].clone() };
            catalogue.extend([graphtec("sur-ordre", false), graphtec("auto", true)]);
            assert_eq!(appel(&e, "POST", "/api/machines", None, Some(serde_json::to_value(&catalogue).unwrap())).await.status(), StatusCode::OK);
            assert!(machines::lire(&e.config.donnees).unwrap().iter().any(|m| m.id == "sur-ordre" && !m.coupe_auto), "gardé dans machines.json");
            let t = rendre(&e, pdf_tons(&["CutContour"]), Demande { mode: Mode::Decouper, ..Default::default() }).await;
            for m in ["sur-ordre", "auto"] {
                let r = appel(&e, "POST", "/api/file", None, Some(serde_json::json!({ "travail": t.id, "page": 1, "machine": m }))).await;
                assert_eq!(r.status(), StatusCode::CREATED);
            }
            let file_ = json(appel(&e, "GET", "/api/file", None, None).await).await;
            assert_eq!((file_[0]["etat"].as_str(), file_[0]["raison_attente"].as_str()), (Some("suspendue"), Some("attente_operateur")));
            assert_eq!(file_[1]["etat"], "en_attente");
            tokio::spawn(repartir(e.clone()));
            let etat_de = |i: usize| e.file.lock().unwrap().sorties[i].etat;
            for _ in 0..100 {
                if etat_de(1) == file::Etat::Terminee {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            assert_eq!((etat_de(0), etat_de(1)), (file::Etat::Suspendue, file::Etat::Terminee), "seule la machine « auto » a coupé");
            let id = e.file.lock().unwrap().sorties[0].id;
            assert_eq!(appel(&e, "POST", &format!("/api/file/{id}/reprendre"), None, None).await.status(), StatusCode::NO_CONTENT);
            for _ in 0..100 {
                if etat_de(0) == file::Etat::Terminee {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            assert_eq!(etat_de(0), file::Etat::Terminee, "« Couper » : partie");
            assert_eq!(std::fs::read_dir(e.config.donnees.join("sorties").join("sur-ordre")).unwrap().count(), 1);
            // Après coupe « rien » (défaut) : les deux restent dans l'historique ; archivées à la
            // main, la restauration suit la machine.
            let ids: Vec<Uuid> = e.file.lock().unwrap().sorties.iter().map(|s| s.id).collect();
            {
                let mut f = e.file.lock().unwrap();
                ids.iter().for_each(|i| f.apres_coupe(*i, file::ApresCoupe::Archiver));
                f.sorties.iter_mut().for_each(|s| s.machine = if s.id == ids[0] { "sur-ordre".into() } else { "auto-arrete".into() });
            }
            assert_eq!(appel(&e, "POST", &format!("/api/file/{}/restaurer", ids[0]), None, None).await.status(), StatusCode::NO_CONTENT);
            assert_eq!(appel(&e, "POST", &format!("/api/file/{}/restaurer", ids[1]), None, None).await.status(), StatusCode::NO_CONTENT);
            let f = e.file.lock().unwrap().sorties.clone();
            assert_eq!((f[0].etat, f[0].archivee, f[1].etat), (file::Etat::Suspendue, false, file::Etat::EnAttente));
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }
    }
    /// Imbrication dans la file (roadmap 5.1 à 5.6, 5.12), de bout en bout : vrais rendus,
    /// vraie file, planche composée puis rendue.
    mod imbrication_file {
        use super::controle_d_acces::{appel, etat};
        use super::couches_decoupe::json;
        use super::regression_geometrie::{PageEssai, pdf_pages, rendre};
        use super::*;

        const MM: f64 = imbrication::PT_PAR_MM;

        fn regler(e: &Partage, r: nid::Reglage) {
            e.machines.write().unwrap().iter_mut().find(|m| m.id == "epson").unwrap().imbrication = r;
        }

        async fn en_file(e: &Partage, t: &Travail, page: usize, machine: &str, operation: &str) {
            let r = appel(e, "POST", "/api/file", None, Some(serde_json::json!({ "travail": t.id, "page": page, "machine": machine, "operation": operation }))).await;
            assert_eq!(r.status(), StatusCode::CREATED);
        }

        async fn planches(e: &Partage) -> Vec<serde_json::Value> {
            json(appel(e, "GET", "/api/imbrications", None, None).await).await.as_array().unwrap().clone()
        }

        async fn rendu(e: &Partage, id: Uuid) -> Travail {
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

        fn etiquette_200x100() -> Vec<u8> {
            imbrication::tests::pdf(200.0 * MM, 100.0 * MM, "1 0 0 0", (0.0, 0.0))
        }

        /// 5.1, 5.3, 5.4 : en économie, deux travaux sans découpe et un pour le Kona font deux
        /// planches ; l'aperçu (longueur, remplissage) est celui du PDF produit ; « Imprimer
        /// maintenant » remplace les impressions (et la découpe en file) par la planche, qui
        /// part sur l'Epson et se découpe sur le Kona.
        #[tokio::test]
        async fn deux_planches_selon_la_decoupeuse_apercu_egal_pdf() {
            let e = etat("nid-groupes");
            regler(&e, nid::Reglage { strategie: nid::Strategie::Economie, espace_mm: 0.0, ..Default::default() });
            let d = Demande { dpi: 36, ..Default::default() };
            let a = rendre(&e, etiquette_200x100(), d.clone()).await;
            let b = rendre(&e, etiquette_200x100(), d.clone()).await;
            let k = rendre(&e, etiquette_200x100(), Demande { decoupeuse: Some("kona".into()), ..d }).await;
            for t in [&a, &b, &k] {
                en_file(&e, t, 1, "epson", "impression").await;
            }
            en_file(&e, &k, 1, "kona", "decoupe").await;
            let file = json(appel(&e, "GET", "/api/file", None, None).await).await;
            let raisons: Vec<_> = file.as_array().unwrap().iter().map(|s| s["raison_attente"].as_str().unwrap_or("")).collect();
            assert_eq!(raisons, ["attente_imbrication"; 3].iter().copied().chain(["attente_impression"]).collect::<Vec<_>>());

            let p = planches(&e).await;
            assert_eq!(p.len(), 2, "sans découpeuse et Kona : deux planches");
            let (libre, kona) = (&p[0], &p[1]);
            assert_eq!((libre["pages"].as_u64(), libre["decoupeuse"].as_str()), (Some(2), None));
            assert_eq!((kona["pages"].as_u64(), kona["decoupeuse"].as_str()), (Some(1), Some("kona")));
            // Économie sur 1626 mm : les deux étiquettes côte à côte, 100 mm de long.
            let longueur = libre["longueur_mm"].as_f64().unwrap();
            assert!((longueur - 100.0).abs() < 1e-3, "{libre}");
            let remplissage = libre["remplissage"].as_f64().unwrap();
            assert!((remplissage - 2.0 * 0.02 / (1.626 * 0.1)).abs() < 1e-6, "{remplissage}");
            assert_eq!(libre["places"].as_array().unwrap().len(), 2);

            // L'aperçu est le PDF produit.
            let url = |g: &serde_json::Value| format!("/api/imbrications/epson/{}", g["code"].as_str().unwrap());
            let r = appel(&e, "GET", &format!("{}/planche.pdf", url(libre)), None, None).await;
            assert_eq!(r.status(), StatusCode::OK);
            let apercu = axum::body::to_bytes(r.into_body(), 1 << 24).await.unwrap().to_vec();
            let taille = imbrication::taille_page(&apercu, 1).unwrap();
            assert!((taille.hauteur / MM - longueur).abs() < 1e-3 && (taille.largeur / MM - 400.0).abs() < 1e-3, "{taille:?}");

            let r = appel(&e, "POST", &format!("{}/imprimer", url(libre)), None, None).await;
            assert_eq!(r.status(), StatusCode::ACCEPTED);
            let planche = rendu(&e, Uuid::parse_str(json(r).await["id"].as_str().unwrap()).unwrap()).await;
            let produit = std::fs::read(dossier(&e.config.donnees, planche.id).join("source.pdf")).unwrap();
            assert_eq!(imbrication::taille_page(&produit, 1).unwrap(), taille, "planche produite = aperçu");
            assert_eq!(decoupe::extraire(&produit, false).unwrap()[0].traces.len(), 2, "deux contours posés");
            assert_eq!(planche.imbrication.as_ref().map(|i| i.machine.as_str()), Some("epson"));
            {
                let f = e.file.lock().unwrap();
                for t in [&a, &b] {
                    let s = f.sorties.iter().find(|s| s.travail == t.id).unwrap();
                    assert_eq!((s.etat, s.planche), (file::Etat::Imbriquee, Some(planche.id)));
                }
                let s = f.sorties.iter().find(|s| s.travail == planche.id).unwrap();
                assert_eq!((s.machine.as_str(), s.operation(), s.etat), ("epson", machines::Operation::Impression, file::Etat::EnAttente));
                let catalogue = e.machines.read().unwrap().clone();
                assert_eq!(raison_attente(&e.config.donnees, s, catalogue.iter().find(|m| m.id == "epson"), None, &f.sorties, &catalogue), None, "la planche, elle, part");
            }
            assert_eq!(planches(&e).await.len(), 1, "reste la planche Kona");

            // Planche Kona : la découpe en file du travail est remplacée par celle de la planche.
            let kona = &planches(&e).await[0];
            let r = appel(&e, "POST", &format!("{}/imprimer", url(kona)), None, None).await;
            assert_eq!(r.status(), StatusCode::ACCEPTED);
            let planche_k = rendu(&e, Uuid::parse_str(json(r).await["id"].as_str().unwrap()).unwrap()).await;
            assert_eq!(planche_k.demande.decoupeuse.as_deref(), Some("kona"));
            let f = e.file.lock().unwrap();
            assert!(f.sorties.iter().filter(|s| s.travail == k.id).all(|s| s.etat == file::Etat::Imbriquee), "impression et découpe du travail imbriquées");
            let ops: Vec<_> = f.sorties.iter().filter(|s| s.travail == planche_k.id).map(|s| (s.machine.as_str(), s.etat)).collect();
            assert_eq!(ops, [("epson", file::Etat::EnAttente), ("kona", file::Etat::Suspendue)]);
            drop(f);
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// 5.2 : départ à la surface (0,05 m²) : deux étiquettes de 0,02 m² attendent, la
        /// troisième fait partir la planche au passage du répartiteur.
        #[tokio::test]
        async fn depart_a_la_surface_par_le_repartiteur() {
            let e = etat("nid-surface");
            regler(&e, nid::Reglage { strategie: nid::Strategie::Groupe, surface_m2: Some(0.05), ..Default::default() });
            let d = Demande { dpi: 36, ..Default::default() };
            let mut travaux = vec![];
            for _ in 0..3 {
                travaux.push(rendre(&e, etiquette_200x100(), d.clone()).await);
            }
            for (n, t) in travaux.iter().enumerate() {
                en_file(&e, t, 1, "epson", "impression").await;
                imbriquer_les_pretes(&e).await;
                let lancees = e.file.lock().unwrap().sorties.iter().filter(|s| s.etat == file::Etat::Imbriquee).count();
                assert_eq!(lancees, if n == 2 { 3 } else { 0 }, "après le travail {}", n + 1);
            }
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// 5.5 : contour de toute la planche dans sa couche `NestTrimBox`, coupée en dernier,
        /// présente dans le HP-GL ; marges haute et basse ; alerte des repères sans tracé.
        #[tokio::test]
        async fn contour_nest_trim_box_et_marges() {
            let e = etat("nid-contour");
            regler(&e, nid::Reglage { strategie: nid::Strategie::Economie, contour: true, marge_bas_mm: 20.0, marge_haut_mm: 30.0, espace_mm: 0.0, ..Default::default() });
            let d = Demande { dpi: 36, decoupeuse: Some("kona".into()), ..Default::default() };
            let t = rendre(&e, etiquette_200x100(), d).await;
            en_file(&e, &t, 1, "epson", "impression").await;
            let g = planches(&e).await.remove(0);
            assert!((g["longueur_mm"].as_f64().unwrap() - 150.0).abs() < 1e-3, "100 + 20 + 30 : {g}");
            let r = appel(&e, "POST", &format!("/api/imbrications/epson/{}/imprimer", g["code"].as_str().unwrap()), None, None).await;
            let planche = rendu(&e, Uuid::parse_str(json(r).await["id"].as_str().unwrap()).unwrap()).await;
            let noms: Vec<&str> = planche.couches_decoupe.iter().map(|c| c.nom.as_str()).collect();
            assert_eq!(noms, ["CutContour", decoupe::NEST_TRIM_BOX], "le contour de planche, couché en dernier");
            assert_eq!(planche.pages[0].traces_par_couche.get(decoupe::NEST_TRIM_BOX), Some(&1));
            assert!(!planche.pages[0].plaques.iter().any(|p| p == decoupe::NEST_TRIM_BOX) || planche.pages[0].plaques_decoupe.iter().any(|p| p == decoupe::NEST_TRIM_BOX), "jamais imprimé");
            let plt = std::fs::read_to_string(fichier_plt(&dossier(&e.config.donnees, planche.id), 1)).unwrap();
            // Dernier tracé du HP-GL : le rectangle de 200 × 100 mm posé à 20 mm du bas.
            let dernier = plt.rsplit("SP1;").next().unwrap().split(";PU;").next().unwrap();
            let v: Vec<f64> = dernier.split(|c: char| !c.is_ascii_digit() && c != '.' && c != '-').filter_map(|x| x.parse().ok()).collect();
            let (xs, ys): (Vec<f64>, Vec<f64>) = v.chunks(2).filter(|c| c.len() == 2).map(|c| (c[0] / 100.0, c[1] / 100.0)).unzip();
            let borne = |v: &[f64]| (v.iter().copied().fold(f64::MAX, f64::min), v.iter().copied().fold(f64::MIN, f64::max));
            let ((x0, x1), (y0, y1)) = (borne(&xs), borne(&ys));
            assert!([x0, x1 - 200.0, y0 - 20.0, y1 - 120.0].iter().all(|e| e.abs() < 0.05), "{x0} {x1} {y0} {y1} dans {plt}");
            assert!((planche.pages[0].hauteur_mm - 150.0).abs() < 1.0, "{}", planche.pages[0].hauteur_mm);

            // Repères demandés sans aucun tracé : alerte.
            let mut vide = lopdf::Document::with_version("1.4");
            let c = vide.add_object(lopdf::Stream::new(lopdf::dictionary! {}, b"1 0 0 0 k 0 0 400 400 re f".to_vec()));
            let pages = vide.new_object_id();
            let page = vide.add_object(lopdf::dictionary! { "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(), 0.into(), 800.into(), 800.into()], "Contents" => c });
            vide.objects.insert(pages, lopdf::Object::Dictionary(lopdf::dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }));
            let cat = vide.add_object(lopdf::dictionary! { "Type" => "Catalog", "Pages" => pages });
            vide.trailer.set("Root", cat);
            let mut o = vec![];
            vide.save_to(&mut o).unwrap();
            let t = rendre(&e, o, Demande { dpi: 36, reperage: reperage::Reperage::Cadre, ..Default::default() }).await;
            assert!(t.alertes.iter().any(|a| a.code == "decoupe_reperes_sans_trace"), "{:?}", t.alertes.iter().map(|a| &a.code).collect::<Vec<_>>());
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// 5.6 : fiche dans `job.json` (un job.json d'avant se lit), visible dans la file ;
        /// quantité et n° de commande sur l'étiquette ; l'échéance la plus proche passe d'abord
        /// et la quantité due donne les copies posées.
        #[tokio::test]
        async fn fiche_travail_echeance_et_quantite() {
            let e = etat("nid-fiche");
            regler(&e, nid::Reglage { strategie: nid::Strategie::Groupe, espace_mm: 0.0, ..Default::default() });
            let d = Demande { dpi: 36, edition: edition::Edition { etiquette: Some(edition::Etiquette::default()), ..Default::default() }, ..Default::default() };
            let tard = rendre(&e, etiquette_200x100(), d.clone()).await;
            let tot = rendre(&e, imbrication::tests::pdf(150.0 * MM, 100.0 * MM, "1 0 0 0", (0.0, 0.0)), d).await;
            let fiche = |commande: &str, echeance: u64, quantite: u32| serde_json::json!({ "commande": commande, "client": "Mairie", "societe": "Manciet", "quantite": quantite, "echeance": echeance, "notes": "vernis", "operateur": "Mathieu" });
            let r = appel(&e, "POST", &format!("/api/travaux/{}/fiche", tard.id), None, Some(fiche("C-1", 2_000_000_000, 1))).await;
            assert_eq!(r.status(), StatusCode::ACCEPTED, "étiquette : rendu refait");
            let r = appel(&e, "POST", &format!("/api/travaux/{}/fiche", tot.id), None, Some(fiche("C-42", 1_900_000_000, 3))).await;
            assert_eq!(r.status(), StatusCode::ACCEPTED);
            let tot = rendu(&e, tot.id).await;
            rendu(&e, tard.id).await;
            assert_eq!((tot.fiche.commande.as_str(), tot.fiche.quantite, tot.fiche.echeance, tot.fiche.client.as_str()), ("C-42", 3, Some(1_900_000_000), "Mairie"));
            let source = std::fs::read(dossier(&e.config.donnees, tot.id).join("source.pdf")).unwrap();
            let texte = etiquette::tests::textes(&source, 1);
            assert!(texte.contains("copies 3") && texte.contains("cde C-42"), "{texte}");
            let r = appel(&e, "POST", &format!("/api/travaux/{}/fiche", tot.id), None, Some(serde_json::json!({ "notes": "x".repeat(2001) }))).await;
            assert_eq!(json(r).await["code"], "fiche_invalide");
            // job.json d'avant la fiche : se lit, fiche vide.
            let chemin = dossier(&e.config.donnees, tot.id).join("job.json");
            let mut v: serde_json::Value = serde_json::from_slice(&std::fs::read(&chemin).unwrap()).unwrap();
            for cle in ["fiche", "pages_exclues", "imbrication"] {
                v.as_object_mut().unwrap().remove(cle);
            }
            let ancien: Travail = serde_json::from_value(v).unwrap();
            assert_eq!((ancien.fiche, ancien.pages_exclues.len(), ancien.imbrication.is_none()), (Fiche::default(), 0, true));

            en_file(&e, &tard, 1, "epson", "impression").await;
            en_file(&e, &tot, 1, "epson", "impression").await;
            let file = json(appel(&e, "GET", "/api/file", None, None).await).await;
            assert_eq!(file[1]["fiche"]["commande"], "C-42", "fiche visible dans la file");
            let g = planches(&e).await.remove(0);
            let noms: Vec<_> = g["places"].as_array().unwrap().iter().map(|p| p["nom"].as_str().unwrap().to_string()).collect();
            assert_eq!(noms.len(), 4, "3 copies dues + 1");
            let pos = |x: &serde_json::Value| (x["y"].as_f64().unwrap(), x["x"].as_f64().unwrap());
            let premier = g["places"].as_array().unwrap().iter().min_by(|a, b| pos(a).partial_cmp(&pos(b)).unwrap()).unwrap();
            assert_eq!(premier["x"].as_f64(), Some(0.0));
            let elements = g["elements"].as_array().unwrap();
            assert_eq!(elements.iter().map(|x| x["commande"].as_str().unwrap()).collect::<Vec<_>>(), ["C-1", "C-42"], "éléments dans l'ordre de la file");
            // Groupé : par priorité, les 3 copies de C-42 (échéance la plus proche) d'abord.
            let mut places: Vec<_> = g["places"].as_array().unwrap().clone();
            places.sort_by(|a, b| pos(a).partial_cmp(&pos(b)).unwrap());
            // Les deux travaux se distinguent par leur largeur (150 et 200 mm, étiquette en plus).
            let largeur = |p: &serde_json::Value| p["largeur"].as_f64().unwrap();
            assert!(places[..3].iter().all(|p| (largeur(p) - largeur(&places[0])).abs() < 1e-6) && largeur(&places[3]) - largeur(&places[0]) > 49.0, "{places:?}");
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// 5.12 : PDF de 3 pages en mise en file automatique ; page 2 décochée : ni dans la
        /// file, ni dans la planche, refusée à la main ; recochée, elle revient.
        #[tokio::test]
        async fn page_decochee_ni_file_ni_planche_puis_reactivee() {
            let e = etat("nid-exclure");
            regler(&e, nid::Reglage { strategie: nid::Strategie::Economie, ..Default::default() });
            let page = PageEssai { media: [0.0, 0.0, 200.0, 100.0], crop: None, rotate: 0 };
            let pdf = pdf_pages(&[PageEssai { ..page }, PageEssai { media: [0.0, 0.0, 300.0, 150.0], ..page }, PageEssai { ..page }]);
            let t = rendre(&e, pdf, Demande { dpi: 36, auto: reglages::Auto { imprimer: true, ..Default::default() }, ..Default::default() }).await;
            let pages_en_file = |e: &Partage| { let mut v: Vec<usize> = e.file.lock().unwrap().sorties.iter().filter(|s| s.active()).map(|s| s.page).collect(); v.sort(); v };
            let pages_planche = |g: &serde_json::Value| g["elements"].as_array().unwrap().iter().map(|x| x["page"].as_u64().unwrap()).collect::<Vec<_>>();
            assert_eq!(pages_en_file(&e), [1, 2, 3]);
            let url = format!("/api/travaux/{}/pages/2/exclure", t.id);
            assert_eq!(appel(&e, "POST", &url, None, Some(serde_json::json!({ "exclue": true }))).await.status(), StatusCode::OK);
            assert_eq!(pages_en_file(&e), [1, 3], "page 2 hors de la file");
            assert_eq!(pages_planche(&planches(&e).await[0]), [1, 3], "ni dans la planche");
            assert_eq!(lire_travail(&e.config.donnees, t.id).unwrap().pages_exclues, [2]);
            let r = appel(&e, "POST", "/api/file", None, Some(serde_json::json!({ "travail": t.id, "page": 2, "machine": "epson", "operation": "impression" }))).await;
            assert_eq!(json(r).await["code"], "page_exclue");
            let r = appel(&e, "POST", "/api/planches/apercu", None, Some(serde_json::json!({ "laize_mm": 1000.0, "elements": [{ "id": t.id, "page": 2, "copies": 1 }] }))).await;
            assert_eq!(json(r).await["code"], "page_exclue");
            assert_eq!(appel(&e, "POST", &url, None, Some(serde_json::json!({ "exclue": false }))).await.status(), StatusCode::OK);
            assert_eq!(pages_en_file(&e), [1, 2, 3], "recochée : de retour en file");
            assert_eq!(pages_planche(&planches(&e).await[0]), [1, 3, 2]);
            assert_eq!(appel(&e, "POST", &format!("/api/travaux/{}/pages/4/exclure", t.id), None, Some(serde_json::json!({ "exclue": true }))).await.status(), StatusCode::NOT_FOUND);
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// Page `l × h` pt cyan, un contour `CutContour` à 5 pt du bord et un `Kisscut` à 20 pt.
        fn deux_couches(l: f64, h: f64) -> Vec<u8> {
            use lopdf::{Document, Object, Stream, dictionary};
            let mut doc = Document::with_version("1.4");
            let ton = |nom: &str| Object::Array(vec!["Separation".into(), Object::Name(nom.as_bytes().to_vec()), "DeviceCMYK".into(), Object::Dictionary(dictionary! { "FunctionType" => 2, "Domain" => vec![0.into(), 1.into()], "C0" => vec![0.into(); 4], "C1" => vec![0.into(), 1.into(), 0.into(), 0.into()], "N" => 1 })]);
            let contenu = format!("1 0 0 0 k 0 0 {l} {h} re f /CS0 CS 1 SCN 5 5 {} {} re S /CS1 CS 1 SCN 20 20 {} {} re S", l - 10.0, h - 10.0, l - 40.0, h - 40.0);
            let c = doc.add_object(Stream::new(dictionary! {}, contenu.into_bytes()));
            let pages = doc.new_object_id();
            let page = doc.add_object(dictionary! { "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(), 0.into(), Object::Real(l as f32), Object::Real(h as f32)], "Contents" => c, "Resources" => dictionary! { "ColorSpace" => dictionary! { "CS0" => ton("CutContour"), "CS1" => ton("Kisscut") } } });
            doc.objects.insert(pages, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }));
            let cat = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
            doc.trailer.set("Root", cat);
            let mut o = vec![];
            doc.save_to(&mut o).unwrap();
            o
        }

        async fn regler_couches(e: &Partage, t: &Travail, f: impl Fn(&mut decoupe::Couche)) {
            let mut couches = t.couches_decoupe.clone();
            couches.iter_mut().for_each(f);
            let r = appel(e, "POST", &format!("/api/travaux/{}/couches", t.id), None, Some(serde_json::to_value(&couches).unwrap())).await;
            assert!(r.status().is_success(), "{}", r.status());
        }

        /// Point 1 (couches de chaque travail sur la planche) : Kisscut décochée sur A n'est
        /// pas coupée pour A sur la planche, mais l'est pour B ; un réglage de coupe choisi passe
        /// sur la planche ; deux réglages différents pour une couche ne s'imbriquent pas.
        #[tokio::test]
        async fn couches_de_chaque_travail_sur_la_planche() {
            let e = etat("nid-couches");
            regler(&e, nid::Reglage { strategie: nid::Strategie::Economie, espace_mm: 0.0, ..Default::default() });
            let d = Demande { dpi: 36, ..Default::default() };
            let a = rendre(&e, deux_couches(200.0 * MM, 100.0 * MM), d.clone()).await;
            let b = rendre(&e, deux_couches(150.0 * MM, 100.0 * MM), d.clone()).await;
            let c = rendre(&e, deux_couches(100.0 * MM, 100.0 * MM), d).await;
            assert_eq!(a.couches_decoupe.iter().map(|c| c.nom.as_str()).collect::<Vec<_>>(), ["Kisscut", "CutContour"]);
            regler_couches(&e, &a, |c| c.active = c.nom != "Kisscut").await;
            regler_couches(&e, &c, |c| if c.nom == "CutContour" { c.reglage = Some("Mi-chair V3000".into()) }).await;
            for t in [&a, &b, &c] {
                en_file(&e, t, 1, "epson", "impression").await;
            }
            let p = planches(&e).await;
            assert_eq!(p.iter().map(|g| g["pages"].as_u64().unwrap()).collect::<Vec<_>>(), [2, 1], "C (autre réglage de CutContour) à part");

            let r = appel(&e, "POST", &format!("/api/imbrications/epson/{}/imprimer", p[0]["code"].as_str().unwrap()), None, None).await;
            let planche = rendu(&e, Uuid::parse_str(json(r).await["id"].as_str().unwrap()).unwrap()).await;
            let couches: Vec<(&str, bool)> = planche.couches_decoupe.iter().map(|c| (c.nom.as_str(), c.active)).collect();
            assert_eq!(couches, [("Kisscut_exclu", false), ("Kisscut", true), ("CutContour", true)]);
            let plt = std::fs::read_to_string(fichier_plt(&dossier(&e.config.donnees, planche.id), 1)).unwrap();
            let traces = decoupe::depuis_hpgl(&plt);
            let largeurs: Vec<f64> = traces.iter().map(|t| { let xs = t.points.iter().map(|p| p.0); xs.clone().fold(f64::MIN, f64::max) - xs.fold(f64::MAX, f64::min) }).collect();
            let a_largeur = |l: f64| largeurs.iter().any(|x| (x - l).abs() < 0.5);
            assert_eq!(traces.len(), 3, "2 CutContour + le Kisscut de B : {largeurs:?}");
            assert!(a_largeur(150.0 * MM - 40.0), "Kisscut de B coupé : {largeurs:?}");
            assert!(!a_largeur(200.0 * MM - 40.0), "Kisscut de A absent : {largeurs:?}");
            assert!(a_largeur(200.0 * MM - 10.0) && a_largeur(150.0 * MM - 10.0), "CutContour des deux : {largeurs:?}");
            // L'encre de la planche n'a pas bougé : aucune plaque Kisscut* imprimée.
            assert!(planche.pages[0].plaques.iter().filter(|x| x.starts_with("Kisscut")).all(|x| planche.pages[0].plaques_decoupe.contains(x)));

            let g = planches(&e).await.remove(0);
            let r = appel(&e, "POST", &format!("/api/imbrications/epson/{}/imprimer", g["code"].as_str().unwrap()), None, None).await;
            let planche_c = rendu(&e, Uuid::parse_str(json(r).await["id"].as_str().unwrap()).unwrap()).await;
            let cc = planche_c.couches_decoupe.iter().find(|c| c.nom == "CutContour").unwrap();
            assert_eq!(cc.reglage.as_deref(), Some("Mi-chair V3000"), "réglage choisi repris");
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        fn etats(e: &Partage, travaux: &[&Travail]) -> Vec<(file::Etat, Option<Uuid>)> {
            let f = e.file.lock().unwrap();
            travaux.iter().flat_map(|t| f.sorties.iter().filter(|s| s.travail == t.id).map(|s| (s.etat, s.planche))).collect()
        }

        async fn lancer(e: &Partage) -> Travail {
            let g = planches(e).await.remove(0);
            let r = appel(e, "POST", &format!("/api/imbrications/epson/{}/imprimer", g["code"].as_str().unwrap()), None, None).await;
            assert_eq!(r.status(), StatusCode::ACCEPTED);
            rendu(e, Uuid::parse_str(json(r).await["id"].as_str().unwrap()).unwrap()).await
        }

        /// Point 2 : planche en échec de rendu, puis impression de planche annulée et retirée :
        /// les sorties d'origine repassent d'elles-mêmes dans leur état d'avant (découpe
        /// suspendue comprise) et dans l'imbrication. Planche imprimée : rien ne repasse, même
        /// sortie retirée et planche supprimée.
        #[tokio::test]
        async fn planche_en_echec_annulee_retiree_ou_imprimee() {
            let e = etat("nid-reprise");
            regler(&e, nid::Reglage { strategie: nid::Strategie::Groupe, ..Default::default() });
            let d = Demande { dpi: 36, ..Default::default() };
            let a = rendre(&e, etiquette_200x100(), d.clone()).await;
            let b = rendre(&e, etiquette_200x100(), d).await;
            for t in [&a, &b] {
                en_file(&e, t, 1, "epson", "impression").await;
            }
            let attente = vec![(file::Etat::EnAttente, None); 2];

            // Échec du rendu de la planche.
            let p1 = lancer(&e).await;
            assert_eq!(etats(&e, &[&a, &b]), vec![(file::Etat::Imbriquee, Some(p1.id)); 2]);
            e.file.lock().unwrap().sorties.retain(|s| s.travail != p1.id);
            let mut x = lire_travail(&e.config.donnees, p1.id).unwrap();
            x.statut = Statut::Echec;
            ecrire(&e.config.donnees, &x).unwrap();
            reprendre_les_imbriquees(&e);
            assert_eq!(etats(&e, &[&a, &b]), attente, "échec : de retour en attente");
            assert_eq!(planches(&e).await[0]["pages"], 2, "et dans l'imbrication");

            // Impression de la planche annulée, puis retirée.
            let p2 = lancer(&e).await;
            let sortie = e.file.lock().unwrap().sorties.iter().find(|s| s.travail == p2.id).unwrap().id;
            assert_eq!(appel(&e, "POST", &format!("/api/file/{sortie}/annuler"), None, None).await.status(), StatusCode::NO_CONTENT);
            assert_eq!(etats(&e, &[&a, &b]), attente, "annulée : de retour en attente");
            assert_eq!(appel(&e, "DELETE", &format!("/api/file/{sortie}"), None, None).await.status(), StatusCode::NO_CONTENT);
            assert_eq!(etats(&e, &[&a, &b]), attente, "retirée : toujours en attente");

            // Retrait direct d'une planche en échec d'impression (relancée, l'impression échoue).
            let p3 = lancer(&e).await;
            let sortie = {
                let mut f = e.file.lock().unwrap();
                let s = f.sorties.iter_mut().find(|s| s.travail == p3.id).unwrap();
                s.etat = file::Etat::Echec;
                s.id
            };
            assert_eq!(appel(&e, "DELETE", &format!("/api/file/{sortie}"), None, None).await.status(), StatusCode::NO_CONTENT);
            assert_eq!(etats(&e, &[&a, &b]), attente, "échec retiré : de retour en attente");

            // Planche réellement imprimée : rien ne repasse.
            let p4 = lancer(&e).await;
            let s = e.file.lock().unwrap().sorties.iter().find(|s| s.travail == p4.id).unwrap().clone();
            executer(e.clone(), s.clone()).await;
            assert_eq!(etats(&e, &[&a, &b]), vec![(file::Etat::Terminee, Some(p4.id)); 2], "imprimées sur la planche");
            assert_eq!(appel(&e, "DELETE", &format!("/api/file/{}", s.id), None, None).await.status(), StatusCode::NO_CONTENT);
            assert_eq!(appel(&e, "DELETE", &format!("/api/travaux/{}", p4.id), None, None).await.status(), StatusCode::NO_CONTENT);
            reprendre_les_imbriquees(&e);
            assert_eq!(etats(&e, &[&a, &b]), vec![(file::Etat::Terminee, Some(p4.id)); 2], "imprimées : ne repassent pas");
            assert!(planches(&e).await.is_empty());
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// Point 2 : la découpe suspendue d'une page imbriquée revient suspendue, pas en attente.
        #[tokio::test]
        async fn decoupe_rendue_dans_son_etat_d_avant() {
            let e = etat("nid-reprise-coupe");
            regler(&e, nid::Reglage { strategie: nid::Strategie::Groupe, ..Default::default() });
            let a = rendre(&e, etiquette_200x100(), Demande { dpi: 36, decoupeuse: Some("kona".into()), ..Default::default() }).await;
            en_file(&e, &a, 1, "epson", "impression").await;
            en_file(&e, &a, 1, "kona", "decoupe").await;
            let coupe = e.file.lock().unwrap().sorties[1].id;
            assert_eq!(appel(&e, "POST", &format!("/api/file/{coupe}/suspendre"), None, None).await.status(), StatusCode::NO_CONTENT);
            let p = lancer(&e).await;
            assert_eq!(etats(&e, &[&a]), vec![(file::Etat::Imbriquee, Some(p.id)); 2]);
            assert_eq!(appel(&e, "DELETE", &format!("/api/travaux/{}", p.id), None, None).await.status(), StatusCode::CONFLICT, "planche en file : occupée");
            let s = e.file.lock().unwrap().sorties.iter().find(|s| s.travail == p.id && s.operation() == machines::Operation::Impression).unwrap().id;
            assert_eq!(appel(&e, "POST", &format!("/api/file/{s}/annuler"), None, None).await.status(), StatusCode::NO_CONTENT);
            assert_eq!(etats(&e, &[&a]), [(file::Etat::EnAttente, None), (file::Etat::Suspendue, None)]);
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// 5.13 : planche en échec d'impression, travaux revenus en file : « Refaire » la
        /// sortie de la planche, la relancer ou la remettre en file est refusé ; les travaux ne
        /// partent qu'une fois, sur la planche suivante. Une sortie « Imbriquée » ne se refait
        /// pas non plus tant que sa planche est en file ; une planche imprimée se refait.
        #[tokio::test]
        async fn refaire_une_planche_abandonnee_jamais_deux_fois() {
            let e = etat("nid-refaire");
            regler(&e, nid::Reglage { strategie: nid::Strategie::Groupe, ..Default::default() });
            let d = Demande { dpi: 36, ..Default::default() };
            let a = rendre(&e, etiquette_200x100(), d.clone()).await;
            let b = rendre(&e, etiquette_200x100(), d).await;
            for t in [&a, &b] {
                en_file(&e, t, 1, "epson", "impression").await;
            }
            let p1 = lancer(&e).await;
            // Tant que la planche est en file, la sortie imbriquée d'un travail ne se refait pas.
            let imbriquee = e.file.lock().unwrap().sorties.iter().find(|s| s.travail == a.id).unwrap().id;
            let r = appel(&e, "POST", &format!("/api/file/{imbriquee}/refaire"), None, None).await;
            assert_eq!((r.status(), json(r).await["code"].clone()), (StatusCode::CONFLICT, serde_json::json!("sortie_sur_une_planche")));
            // L'impression de la planche échoue ; ses travaux reviennent en file.
            let sortie = {
                let mut f = e.file.lock().unwrap();
                let s = f.sorties.iter_mut().find(|s| s.travail == p1.id).unwrap();
                s.etat = file::Etat::Echec;
                s.id
            };
            reprendre_les_imbriquees(&e);
            assert_eq!(etats(&e, &[&a, &b]), vec![(file::Etat::EnAttente, None); 2], "revenus en file");
            let avant = e.file.lock().unwrap().sorties.len();
            for (methode, url, corps) in [
                ("POST", format!("/api/file/{sortie}/refaire"), None),
                ("POST", format!("/api/travaux/{}/relancer", p1.id), None),
                ("POST", "/api/file".to_string(), Some(serde_json::json!({ "travail": p1.id, "page": 1, "machine": "epson", "operation": "impression" }))),
            ] {
                let r = appel(&e, methode, &url, None, corps).await;
                assert_eq!((r.status(), json(r).await["code"].clone()), (StatusCode::CONFLICT, serde_json::json!("planche_rendue_a_la_file")), "{url}");
            }
            assert_eq!(e.file.lock().unwrap().sorties.len(), avant, "aucune sortie ajoutée");
            assert!(matches!(lire_travail(&e.config.donnees, p1.id).unwrap().statut, Statut::Termine), "planche pas relancée");
            // Les travaux partent une fois, sur une nouvelle planche ; chaque page n'a qu'une
            // impression qui l'imprime (la sienne, imbriquée sur la planche 2).
            let p2 = lancer(&e).await;
            let f = e.file.lock().unwrap().sorties.clone();
            assert_eq!(f.iter().filter(|s| s.travail == p2.id && s.etat == file::Etat::EnAttente).count(), 1);
            assert!(f.iter().filter(|s| s.travail == p1.id).all(|s| !s.active()), "planche 1 inerte");
            for t in [&a, &b] {
                let s: Vec<_> = f.iter().filter(|s| s.travail == t.id).map(|s| (s.etat, s.planche)).collect();
                assert_eq!(s, [(file::Etat::Imbriquee, Some(p2.id))]);
            }
            // Planche imprimée : un « Refaire » est voulu (réimpression), il est accepté.
            let s2 = f.iter().find(|s| s.travail == p2.id).unwrap().clone();
            executer(e.clone(), s2.clone()).await;
            assert_eq!(appel(&e, "POST", &format!("/api/file/{}/refaire", s2.id), None, None).await.status(), StatusCode::NO_CONTENT);
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// 5.7 par le vrai circuit : centré, décalage d'origine, purge à droite ; la planche
        /// produite a la laize, les éléments à leur place et la bande CMJN au bord droit.
        #[tokio::test]
        async fn purge_et_justification_sur_la_planche_produite() {
            let e = etat("nid-purge");
            let laize = e.machines.read().unwrap().iter().find(|m| m.id == "epson").unwrap().laize_mm;
            regler(&e, nid::Reglage { strategie: nid::Strategie::Groupe, espace_mm: 0.0, justification: nid::Justification::Centre, decalage_x_mm: 30.0, purge_cote: Some(nid::Cote::Droite), purge_mm: 20.0, ..Default::default() });
            let d = Demande { dpi: 36, ..Default::default() };
            for _ in 0..2 {
                let t = rendre(&e, etiquette_200x100(), d.clone()).await;
                en_file(&e, &t, 1, "epson", "impression").await;
            }
            let g = planches(&e).await.remove(0);
            let x0 = 30.0 + (laize - 50.0 - 400.0) / 2.0;
            let xs: Vec<f64> = g["places"].as_array().unwrap().iter().map(|p| p["x"].as_f64().unwrap()).collect();
            assert!((xs[0] - x0).abs() < 1e-3 && (xs[1] - x0 - 200.0).abs() < 1e-3, "{xs:?}, attendu {x0}");
            let planche = lancer(&e).await;
            let produit = std::fs::read(dossier(&e.config.donnees, planche.id).join("source.pdf")).unwrap();
            assert!((imbrication::taille_page(&produit, 1).unwrap().largeur / MM - laize).abs() < 1e-3, "planche à la laize");
            let doc = lopdf::Document::load_mem(&produit).unwrap();
            let contenu = String::from_utf8_lossy(&doc.get_page_content(*doc.get_pages().get(&1).unwrap())).to_string();
            for (n, encre) in ["1 0 0 0", "0 1 0 0", "0 0 1 0", "0 0 0 1"].iter().enumerate() {
                let bande = format!("{encre} k {:.4} 0.0000 {:.4} {:.4} re f", (laize - 20.0 + 5.0 * n as f64) * MM, 5.0 * MM, 100.0 * MM);
                assert!(contenu.contains(&bande), "bande {encre} absente : {contenu}");
            }
            // Contours de découpe des deux étiquettes : à leur place justifiée.
            let traces = &decoupe::extraire(&produit, false).unwrap()[0].traces;
            let mut gauches: Vec<f64> = traces.iter().map(|t| t.points.iter().map(|p| p.0).fold(f64::MAX, f64::min) / MM).collect();
            gauches.sort_by(f64::total_cmp);
            // Le contour de l'étiquette d'essai est à 5 pt de son bord.
            let bord = x0 + 5.0 / MM;
            assert!(gauches.len() == 2 && (gauches[0] - bord).abs() < 0.01 && (gauches[1] - bord - 200.0).abs() < 0.01, "{gauches:?}");
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// 5.11 : chargement libre, une planche pour deux formats ; strict, une par format,
        /// chacune posée sur son format.
        #[tokio::test]
        async fn chargement_strict_une_planche_par_format() {
            let e = etat("nid-strict");
            let d = Demande { dpi: 36, ..Default::default() };
            for f in [1000.0, 1300.0] {
                let mut t = rendre(&e, etiquette_200x100(), d.clone()).await;
                t.demande.mosaique = Some(reglages::Mosaique { laize_mm: f, ..Default::default() });
                ecrire(&e.config.donnees, &t).unwrap();
                en_file(&e, &t, 1, "epson", "impression").await;
            }
            let laize = e.machines.read().unwrap().iter().find(|m| m.id == "epson").unwrap().laize_mm;
            regler(&e, nid::Reglage { strategie: nid::Strategie::Groupe, ..Default::default() });
            let laizes = |p: Vec<serde_json::Value>| p.iter().map(|g| g["laize_mm"].as_f64().unwrap()).collect::<Vec<_>>();
            assert_eq!(laizes(planches(&e).await), [laize], "libre : une planche, sur le rouleau");
            regler(&e, nid::Reglage { strategie: nid::Strategie::Groupe, chargement: nid::Chargement::Strict, ..Default::default() });
            assert_eq!(laizes(planches(&e).await), [1000.0, 1300.0], "strict : une planche par format");
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// machines.json d'avant l'imbrication : un par un ; réglage incohérent refusé.
        #[tokio::test]
        async fn machines_d_avant_et_reglage_refuse() {
            let e = etat("nid-machines");
            let mut v = serde_json::to_value(e.machines.read().unwrap().clone()).unwrap();
            for m in v.as_array_mut().unwrap() {
                m.as_object_mut().unwrap().remove("imbrication");
            }
            let lues: Vec<machines::Machine> = serde_json::from_value(v.clone()).unwrap();
            assert!(lues.iter().all(|m| m.imbrication.strategie == nid::Strategie::UnParUn));
            v[0]["imbrication"] = serde_json::json!({ "strategie": "economie", "remplissage_pct": 80.0 });
            assert_eq!(json(appel(&e, "POST", "/api/machines", None, Some(v.clone())).await).await["code"], "machine_invalide");
            v[0]["imbrication"] = serde_json::json!({ "strategie": "economie", "longueur_cible_mm": 2000.0, "remplissage_pct": 80.0 });
            assert_eq!(appel(&e, "POST", "/api/machines", None, Some(v)).await.status(), StatusCode::OK);
            assert_eq!(e.machines.read().unwrap()[0].imbrication.longueur_cible_mm, Some(2000.0));
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }
    }

    /// Lot R3 : un envoi de l'ERP téléchargé par le pont devient un travail de même uuid, au
    /// réglage désigné ; sinon il part en `travail.refuse`.
    mod envois_erp {
        use super::controle_d_acces::etat;
        use super::regression_geometrie::{PageEssai, pdf_pages};
        use super::*;
        use serde_json::{Value, json};

        fn deposer(e: &Partage, r: &pont::Recu, octets: &[u8]) {
            let rep = e.config.donnees.join("pont").join("recus").join(r.id.to_string());
            std::fs::create_dir_all(&rep).unwrap();
            std::fs::write(rep.join("source"), octets).unwrap();
            std::fs::write(rep.join("envoi.json"), serde_json::to_vec(r).unwrap()).unwrap();
        }

        fn journal(e: &Partage) -> Vec<erp::Evenement> {
            std::fs::read_to_string(e.config.donnees.join("pont").join("journal.jsonl")).unwrap_or_default().lines().map(|l| serde_json::from_str(l).unwrap()).collect()
        }

        async fn attendre_le_rendu(e: &Partage, id: Uuid) {
            for _ in 0..600 {
                if lire_travail(&e.config.donnees, id).is_some_and(|t| !actif(&t.statut)) {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        #[tokio::test]
        async fn un_envoi_devient_un_travail_de_meme_uuid_ou_un_refus() {
            let e = etat("envois-erp");
            let d = e.config.donnees.clone();
            erp::ecrire(&d, &erp::Config { url: Some("http://127.0.0.1:9/rip".into()), secret_hmac: Some("secret-factice".into()), actif: true, instance: Some(Uuid::new_v4()), ..Default::default() }).unwrap();
            let preset = Uuid::new_v4();
            reglages::ecrire(&d, &[reglages::Reglage { uuid: Some(preset), nom: "Bache".into(), demande: Demande { dpi: 36, rotation_auto: false, ..Default::default() } }]).unwrap();
            let pdf = pdf_pages(&[PageEssai { media: [0.0, 0.0, 283.46, 141.73], crop: None, rotate: 0 }]);
            let recu = |copies| pont::Recu { id: Uuid::new_v4(), ref_erp: "PROD-2026-00123".into(), nom: "bache.pdf".into(), prereglage_uuid: preset, copies, echeance: Some(1793600000), sha256: None, estimation: false, partie: None };
            let (bon, sans_reglage, illisible) = (recu(3), pont::Recu { prereglage_uuid: Uuid::new_v4(), ..recu(1) }, recu(1));
            deposer(&e, &bon, &pdf);
            deposer(&e, &sans_reglage, &pdf);
            deposer(&e, &illisible, b"ni PDF ni image");
            prendre_envois_recus(&e).await;

            let t = lire_travail(&d, bon.id).expect("travail créé sous l'uuid de l'envoi");
            assert_eq!(t.erp, Some(pont::Lien { envoi: bon.id, ref_erp: "PROD-2026-00123".into(), partie: None }));
            assert_eq!((t.fiche.commande.as_str(), t.fiche.quantite, t.fiche.echeance, t.nom.as_str(), t.demande.dpi), ("PROD-2026-00123", 3, Some(1793600000), "bache.pdf", 36));
            assert!(pont::recus(&d).is_empty(), "tout est pris ou refusé");
            assert!(lire_travail(&d, sans_reglage.id).is_none() && lire_travail(&d, illisible.id).is_none());
            assert!(!dossier(&d, illisible.id).exists(), "un refus ne laisse pas de dossier de travail");

            let j = journal(&e);
            let accuse = j.iter().find(|x| x.genre == "travail.etat" && x.donnees["travail_uuid"] == json!(bon.id)).expect("travail.etat du travail");
            assert_eq!((accuse.donnees["envoi_id"].clone(), accuse.donnees["ref_erp"].clone()), (json!(bon.id), json!("PROD-2026-00123")));
            let refus: Vec<(Value, Value)> = j.iter().filter(|x| x.genre == "travail.refuse").map(|x| (x.donnees["envoi_id"].clone(), x.donnees["code"].clone())).collect();
            assert_eq!(refus.len(), 2, "{refus:?}");
            assert!(refus.contains(&(json!(sans_reglage.id), json!("reglage_inconnu"))), "{refus:?}");
            assert!(refus.iter().any(|(id, code)| *id == json!(illisible.id) && code.is_string()), "{refus:?}");

            // Coupure entre la création et le ménage : le dossier de réception revient, aucun
            // second travail n'est créé.
            attendre_le_rendu(&e, bon.id).await;
            deposer(&e, &bon, &pdf);
            prendre_envois_recus(&e).await;
            assert!(pont::recus(&d).is_empty());
            assert_eq!(lire_tous(&d).iter().filter(|x| x.erp.is_some()).count(), 1);
            assert_eq!(journal(&e).iter().filter(|x| x.genre == "travail.refuse").count(), 2);
            std::fs::remove_dir_all(&d).unwrap();
        }

        fn travail_erp(ref_erp: Option<&str>, auto: bool) -> Travail {
            let id = Uuid::new_v4();
            Travail {
                id,
                nom: "bache.pdf".into(),
                format: Format::Pdf,
                demande: Demande { auto: reglages::Auto { imprimer: auto, ..Default::default() }, ..Default::default() },
                statut: Statut::Termine,
                code_erreur: None,
                erreur: None,
                pages: vec![Page { plaques: vec!["Cyan".into()], ..Default::default() }],
                alertes: vec![],
                cree_le: maintenant(),
                couches_decoupe: vec![],
                mise_coupe: Default::default(),
                fiche: Default::default(),
                pages_exclues: vec![],
                imbrication: None,
                erp: ref_erp.map(|r| pont::Lien { envoi: id, ref_erp: r.into(), partie: None }),
                retenue_erp: None,
                estimation: false,
            }
        }

        fn agir(e: &Partage, action: pont::GenreAction, ref_erp: &str, echeance: Option<u64>) -> Value {
            agir_avec(e, pont::ActionErp { id: Uuid::new_v4(), ref_erp: ref_erp.into(), action, echeance, operateur: None, page: None })
        }

        fn agir_avec(e: &Partage, a: pont::ActionErp) -> Value {
            let rep = e.config.donnees.join("pont").join("actions");
            std::fs::create_dir_all(&rep).unwrap();
            std::fs::write(rep.join(format!("{}.json", a.id)), serde_json::to_vec(&a).unwrap()).unwrap();
            appliquer_actions_erp(e);
            assert!(pont::actions(&e.config.donnees).is_empty(), "action appliquée puis oubliée");
            let j = journal(e);
            let r = j.iter().rev().find(|x| x.genre == "travail.action").expect("travail.action");
            assert_eq!(r.donnees["action_id"], json!(a.id));
            r.donnees.clone()
        }

        fn etats(e: &Partage) -> Vec<(Uuid, file::Etat)> {
            e.file.lock().unwrap().sorties.iter().map(|s| (s.id, s.etat)).collect()
        }

        #[tokio::test]
        async fn les_actions_de_l_erp_agissent_sur_les_sorties_du_dossier() {
            use pont::GenreAction::*;
            let e = etat("actions-erp");
            let d = e.config.donnees.clone();
            erp::ecrire(&d, &erp::Config { url: Some("http://127.0.0.1:9/rip".into()), secret_hmac: Some("secret-factice".into()), actif: true, instance: Some(Uuid::new_v4()), ..Default::default() }).unwrap();
            let (t, autre) = (travail_erp(Some("PROD-2026-00123"), false), travail_erp(None, false));
            for x in [&t, &autre] {
                std::fs::create_dir_all(dossier(&d, x.id)).unwrap();
                ecrire(&d, x).unwrap();
            }
            let sortie = |t: &Travail, op, page, etat| file::Sortie { etat, ..file::Sortie::nouvelle(if op == machines::Operation::Impression { "epson" } else { "kona" }, op, t.id, &t.nom, page, maintenant()) };
            let u1 = sortie(&autre, machines::Operation::Impression, 1, file::Etat::EnAttente);
            let s1 = sortie(&t, machines::Operation::Impression, 1, file::Etat::EnAttente);
            let s2 = sortie(&t, machines::Operation::Impression, 2, file::Etat::EnCours);
            let s3 = sortie(&t, machines::Operation::Decoupe, 1, file::Etat::Suspendue);
            let (u1, s1, s2, s3) = (u1.id, s1.id, s2.id, s3.id);
            e.file.lock().unwrap().sorties = vec![sortie(&autre, machines::Operation::Impression, 1, file::Etat::EnAttente), sortie(&t, machines::Operation::Impression, 1, file::Etat::EnAttente), sortie(&t, machines::Operation::Impression, 2, file::Etat::EnCours), sortie(&t, machines::Operation::Decoupe, 1, file::Etat::Suspendue)]
                .into_iter()
                .zip([u1, s1, s2, s3])
                .map(|(s, id)| file::Sortie { id, ..s })
                .collect();
            let etat_de = |id: Uuid| etats(&e).into_iter().find(|(x, _)| *x == id).unwrap().1;
            let resultat = |v: &Value| (v["code"].clone(), v["touchees"].clone(), v["non_modifiables"].clone());

            // Suspendre : la sortie en attente seulement ; celle en cours reste, et se compte.
            assert_eq!(resultat(&agir(&e, Suspendre, "PROD-2026-00123", None)), (Value::Null, json!(1), json!(1)));
            assert_eq!((etat_de(s1), etat_de(s2), etat_de(u1)), (file::Etat::Suspendue, file::Etat::EnCours, file::Etat::EnAttente));
            assert_eq!(lire_travail(&d, t.id).unwrap().retenue_erp, Some(RetenueErp::Suspendu));
            // Reprendre : l'impression repart ; la découpe attend toujours la fin de l'impression.
            assert_eq!(resultat(&agir(&e, Reprendre, "PROD-2026-00123", None)), (Value::Null, json!(1), json!(1)));
            assert_eq!((etat_de(s1), etat_de(s3)), (file::Etat::EnAttente, file::Etat::Suspendue));
            assert_eq!(lire_travail(&d, t.id).unwrap().retenue_erp, None);
            // Priorité : les sorties du dossier passent devant celle de l'autre travail, dans leur ordre.
            assert_eq!(resultat(&agir(&e, Priorite, "PROD-2026-00123", None)), (Value::Null, json!(2), json!(1)));
            let ordre: Vec<Uuid> = etats(&e).into_iter().map(|(id, _)| id).filter(|id| [u1, s1].contains(id)).collect();
            assert_eq!(ordre, [s1, u1]);
            // Échéance : la fiche de chaque travail du dossier.
            assert_eq!(resultat(&agir(&e, Echeance, "PROD-2026-00123", Some(1793700000))), (Value::Null, json!(1), json!(0)));
            assert_eq!(lire_travail(&d, t.id).unwrap().fiche.echeance, Some(1793700000));
            assert_eq!(lire_travail(&d, autre.id).unwrap().fiche.echeance, None);
            // Annuler : ce qui reste à passer ; l'impression en cours se signale.
            assert_eq!(resultat(&agir(&e, Annuler, "PROD-2026-00123", None)), (json!("deja_imprime"), json!(2), json!(1)));
            assert_eq!((etat_de(s1), etat_de(s2), etat_de(s3), etat_de(u1)), (file::Etat::Annulee, file::Etat::EnCours, file::Etat::Annulee, file::Etat::EnAttente));
            // Un dossier sans travail au RIP.
            assert_eq!(resultat(&agir(&e, Suspendre, "PROD-2026-99999", None)), (json!("aucun_travail"), json!(0), json!(0)));
            std::fs::remove_dir_all(&d).unwrap();
        }

        #[test]
        fn un_travail_retenu_par_l_erp_part_suspendu_ou_ne_part_pas() {
            let e = etat("retenue-erp");
            let mut t = travail_erp(Some("PROD-2026-00123"), true);
            mettre_en_file_auto(&e, &t, None);
            assert_eq!(etats(&e).iter().map(|(_, x)| *x).collect::<Vec<_>>(), [file::Etat::EnAttente], "sans retenue : en attente");
            t.retenue_erp = Some(RetenueErp::Suspendu);
            mettre_en_file_auto(&e, &t, None);
            assert_eq!(etats(&e).last().unwrap().1, file::Etat::Suspendue);
            t.retenue_erp = Some(RetenueErp::Annule);
            mettre_en_file_auto(&e, &t, None);
            assert_eq!(etats(&e).len(), 2, "annulé : rien de plus en file");
            std::fs::remove_dir_all(&e.config.donnees).unwrap();
        }

        /// Roadmap 7.8 : opérateur affecté et réimpression depuis l'ERP.
        #[test]
        fn l_erp_affecte_l_operateur_et_fait_reimprimer() {
            use pont::GenreAction::*;
            let e = etat("reimprimer-erp");
            let d = e.config.donnees.clone();
            erp::ecrire(&d, &erp::Config { url: Some("http://127.0.0.1:9/rip".into()), secret_hmac: Some("secret-factice".into()), actif: true, instance: Some(Uuid::new_v4()), ..Default::default() }).unwrap();
            let mut t = travail_erp(Some("PROD-2026-00123"), true);
            t.pages.push(t.pages[0].clone());
            let mut en_rendu = travail_erp(Some("PROD-2026-00123"), true);
            en_rendu.statut = Statut::EnCours;
            for x in [&t, &en_rendu] {
                std::fs::create_dir_all(dossier(&d, x.id)).unwrap();
                ecrire(&d, x).unwrap();
            }
            let action = |action, operateur: Option<&str>, page| pont::ActionErp { id: Uuid::new_v4(), ref_erp: "PROD-2026-00123".into(), action, echeance: None, operateur: operateur.map(Into::into), page };
            let resultat = |v: Value| (v["code"].clone(), v["touchees"].clone(), v["non_modifiables"].clone());

            assert_eq!(resultat(agir_avec(&e, action(Operateur, Some("Léa"), None))), (Value::Null, json!(2), json!(0)));
            assert_eq!(lire_travail(&d, t.id).unwrap().fiche.operateur, "Léa");
            // Le rendu qui tournait garde l'opérateur reçu entre-temps.
            let mut copie_du_rendu = t.clone();
            reprendre_fiche(&d, &mut copie_du_rendu);
            assert_eq!(copie_du_rendu.fiche.operateur, "Léa");
            // Les deux pages repartent ; le travail en cours de rendu n'est pas doublé.
            assert_eq!(resultat(agir_avec(&e, action(Reimprimer, None, None))), (Value::Null, json!(2), json!(1)));
            assert_eq!(etats(&e).iter().filter(|(_, x)| *x == file::Etat::EnAttente).count(), 2);
            // Déjà en file : rien de plus.
            assert_eq!(resultat(agir_avec(&e, action(Reimprimer, None, None))), (json!("rien_a_reimprimer"), json!(0), json!(2)));
            for s in e.file.lock().unwrap().sorties.iter_mut() {
                s.etat = file::Etat::Terminee;
            }
            // Une page seulement.
            assert_eq!(resultat(agir_avec(&e, action(Reimprimer, None, Some(2)))), (Value::Null, json!(1), json!(1)));
            assert_eq!(e.file.lock().unwrap().sorties.last().map(|s| (s.page, s.etat)), Some((2, file::Etat::EnAttente)));
            // Posé sur une planche pas encore imprimée : pas doublé.
            e.file.lock().unwrap().sorties.last_mut().unwrap().etat = file::Etat::Imbriquee;
            assert_eq!(resultat(agir_avec(&e, action(Reimprimer, None, None))), (json!("rien_a_reimprimer"), json!(0), json!(2)));
            std::fs::remove_dir_all(&d).unwrap();
        }

        /// Roadmap 7.8 : les parties d'un groupe passent ensemble et dans l'ordre, quel que
        /// soit l'ordre dans lequel leur rendu finit.
        #[test]
        fn les_parties_d_un_groupe_restent_ensemble_et_dans_l_ordre() {
            let e = etat("groupe-erp");
            let d = e.config.donnees.clone();
            let partie = |rang| {
                let mut t = travail_erp(Some("PROD-2026-00123"), true);
                t.erp.as_mut().unwrap().partie = Some(pont::Partie { groupe: "vitrine".into(), rang });
                std::fs::create_dir_all(dossier(&d, t.id)).unwrap();
                ecrire(&d, &t).unwrap();
                t
            };
            let (p1, p2, p3, seul) = (partie(1), partie(2), partie(3), travail_erp(None, true));
            for t in [&p3, &seul, &p1, &p2] {
                assert_eq!(mettre_en_file_auto(&e, t, None), 1);
            }
            let ordre: Vec<Uuid> = e.file.lock().unwrap().sorties.iter().map(|s| s.travail).collect();
            assert_eq!(ordre, [p1.id, p2.id, p3.id, seul.id]);
            std::fs::remove_dir_all(&d).unwrap();
        }

        fn estimer(e: &Partage, ref_erp: &str) -> Option<Value> {
            let demande = pont::DemandeEstimation { id: Uuid::new_v4(), ref_erp: ref_erp.into() };
            let rep = e.config.donnees.join("pont").join("estimations");
            std::fs::create_dir_all(&rep).unwrap();
            std::fs::write(rep.join(format!("{}.json", demande.id)), serde_json::to_vec(&demande).unwrap()).unwrap();
            estimer_pour_l_erp(e);
            let r = journal(e).into_iter().rev().find(|x| x.genre == "travail.estimation" && x.donnees["estimation_id"] == json!(demande.id)).map(|x| x.donnees);
            assert_eq!(r.is_some(), pont::estimations(&e.config.donnees).is_empty(), "répondue puis oubliée, ou gardée");
            let _ = std::fs::remove_dir_all(&rep);
            r
        }

        /// Un travail de l'ERP rendu : une page de `l` × `h` mm, moitié cyan, PDF sur disque.
        fn travail_rendu(e: &Partage, ref_erp: &str, l: f64, h: f64, quantite: u32) -> Travail {
            let mut t = travail_erp(Some(ref_erp), false);
            t.demande.dpi = 720;
            t.fiche.quantite = quantite;
            t.pages = vec![Page { plaques: vec!["Cyan".into()], surface_m2: l * h / 1e6, couverture: [("Cyan".to_string(), 0.5)].into(), largeur_mm: l, hauteur_mm: h, ..Default::default() }];
            let pt = imbrication::PT_PAR_MM;
            std::fs::create_dir_all(dossier(&e.config.donnees, t.id)).unwrap();
            std::fs::write(dossier(&e.config.donnees, t.id).join(Format::Pdf.fichier()), pdf_pages(&[PageEssai { media: [0.0, 0.0, l * pt, h * pt], crop: None, rotate: 0 }])).unwrap();
            ecrire(&e.config.donnees, &t).unwrap();
            t
        }

        #[tokio::test]
        async fn l_estimation_d_un_dossier_reprend_la_fiche_de_ses_travaux_copies_comprises() {
            let e = etat("estimation-erp");
            let d = e.config.donnees.clone();
            erp::ecrire(&d, &erp::Config { url: Some("http://127.0.0.1:9/rip".into()), secret_hmac: Some("secret-factice".into()), actif: true, instance: Some(Uuid::new_v4()), ..Default::default() }).unwrap();
            let a = travail_rendu(&e, "PROD-2026-00123", 1000.0, 500.0, 2);
            let b = travail_rendu(&e, "PROD-2026-00123", 400.0, 300.0, 0);
            let mut exclue = travail_rendu(&e, "PROD-2026-00123", 300.0, 300.0, 1);
            exclue.pages_exclues = vec![1];
            ecrire(&d, &exclue).unwrap();
            let mut echoue = travail_rendu(&e, "PROD-2026-00123", 300.0, 300.0, 1);
            echoue.statut = Statut::Echec;
            ecrire(&d, &echoue).unwrap();
            travail_rendu(&e, "PROD-2026-00999", 300.0, 300.0, 1);

            let r = estimer(&e, "PROD-2026-00123").expect("estimation");
            // Les mêmes valeurs que la fiche (`lister`), multipliées par les copies du travail.
            let fiches = lister(State(e.clone())).await.0;
            let fiche = |t: &Travail| fiches.iter().find(|x| x["id"] == json!(t.id)).unwrap().clone();
            let ml = |t: &Travail| fiche(t)["pages"][0]["estimation"]["ml"].as_f64().unwrap();
            let duree = |t: &Travail| fiche(t)["duree_impression_min"].as_f64().unwrap();
            let arrondi = |v: f64, n: i32| (v * 10f64.powi(n)).round() / 10f64.powi(n);
            assert_eq!((r["code"].clone(), r["travaux"].clone(), r["pages"].clone(), r["copies"].clone()), (Value::Null, json!(3), json!(2), json!(3)), "{r}");
            assert_eq!(r["surface_m2"], json!(arrondi(0.5 * 2.0 + 0.12, 4)));
            assert_eq!(r["ml"], json!(arrondi(ml(&a) * 2.0 + ml(&b), 2)));
            assert_eq!(r["encres"], json!([{ "nom": "Cyan", "ml": arrondi(ml(&a) * 2.0 + ml(&b), 2) }]));
            assert_eq!(r["duree_impression_min"], json!(arrondi(duree(&a) * 2.0 + duree(&b), 1)));
            assert!(r.to_string().find("cout").is_none() && r.get("total").is_none(), "aucun montant (D8) : {r}");
            // Toutes les copies imbriquées sur la laize de l'Epson : moins long que bout à bout.
            assert_eq!(r["laize_mm"], json!(1626.0));
            let longueur = r["longueur_mm"].as_f64().expect("longueur");
            assert!((500.0..1300.0).contains(&longueur), "{longueur}");
            assert_eq!(r["les"], json!(0));

            // Trop large pour la laize : pas de longueur, le reste est donné.
            travail_rendu(&e, "PROD-2026-00124", 2000.0, 500.0, 1);
            let r = estimer(&e, "PROD-2026-00124").unwrap();
            assert_eq!((r["longueur_mm"].clone(), r["copies"].clone()), (Value::Null, json!(1)));
            // Un travail encore en rendu : la demande attend le tour suivant.
            let mut en_cours = travail_rendu(&e, "PROD-2026-00125", 300.0, 300.0, 1);
            en_cours.statut = Statut::EnCours;
            ecrire(&d, &en_cours).unwrap();
            assert!(estimer(&e, "PROD-2026-00125").is_none());
            // Aucun travail de l'ERP rendu pour ce dossier.
            assert_eq!(estimer(&e, "PROD-2026-00126").unwrap(), json!({ "estimation_id": journal(&e).last().unwrap().donnees["estimation_id"], "ref_erp": "PROD-2026-00126", "code": "aucun_travail" }));
            std::fs::remove_dir_all(&d).unwrap();
        }

        #[tokio::test]
        async fn un_fichier_a_estimer_est_rendu_a_part_estime_puis_efface() {
            let e = etat("devis-erp");
            let d = e.config.donnees.clone();
            erp::ecrire(&d, &erp::Config { url: Some("http://127.0.0.1:9/rip".into()), secret_hmac: Some("secret-factice".into()), actif: true, instance: Some(Uuid::new_v4()), ..Default::default() }).unwrap();
            let preset = Uuid::new_v4();
            let auto = reglages::Auto { imprimer: true, ..Default::default() };
            reglages::ecrire(&d, &[reglages::Reglage { uuid: Some(preset), nom: "Bache".into(), demande: Demande { dpi: 36, rotation_auto: false, page_par_travail: true, auto, ..Default::default() } }]).unwrap();
            let page = || PageEssai { media: [0.0, 0.0, 283.46, 141.73], crop: None, rotate: 0 };
            let pdf = pdf_pages(&[page(), page()]);
            let devis = pont::Recu { id: Uuid::new_v4(), ref_erp: "PROD-2026-00123".into(), nom: "devis.pdf".into(), prereglage_uuid: preset, copies: 3, echeance: None, sha256: None, estimation: true, partie: None };
            let sans_reglage = pont::Recu { id: Uuid::new_v4(), prereglage_uuid: Uuid::new_v4(), ..devis.clone() };
            deposer(&e, &devis, &pdf);
            deposer(&e, &sans_reglage, &pdf);
            prendre_envois_recus(&e).await;

            assert!(lire_tous(&d).is_empty(), "un devis n'est pas un travail de l'atelier");
            let t = lire_travail(&d, devis.id).expect("rendu caché");
            assert!(t.estimation && !t.demande.page_par_travail, "une page par travail ne coupe pas un devis");
            attendre_le_rendu(&e, devis.id).await;
            assert_eq!(lire_travail(&d, devis.id).unwrap().pages.len(), 2, "ses deux pages restent ensemble");
            assert!(e.file.lock().unwrap().sorties.is_empty(), "jamais en file, même en impression automatique");
            assert!(!journal(&e).iter().any(|x| x.genre == "travail.etat" || x.genre == "travail.refuse"), "ni travail.etat ni travail.refuse");

            estimer_pour_l_erp(&e);
            let j = journal(&e);
            let reponse = |id: Uuid| j.iter().find(|x| x.genre == "travail.estimation" && x.donnees["estimation_id"] == json!(id)).map(|x| x.donnees.clone()).expect("travail.estimation");
            let r = reponse(devis.id);
            assert_eq!((r["code"].clone(), r["ref_erp"].clone(), r["travaux"].clone(), r["pages"].clone(), r["copies"].clone()), (Value::Null, json!("PROD-2026-00123"), json!(1), json!(2), json!(6)), "{r}");
            assert!(r["surface_m2"].as_f64().unwrap() > 0.0 && r["longueur_mm"].is_number(), "{r}");
            assert_eq!(reponse(sans_reglage.id)["code"], json!("reglage_inconnu"));
            assert!(!dossier(&d, devis.id).exists(), "effacé une fois répondu");
            std::fs::remove_dir_all(&d).unwrap();
        }
    }
}
