//! Application mobile de suivi (iOS, Android) : téléphones inscrits, événements à
//! signaler, et envoi des notifications par le service push d'Expo.
//!
//! Les événements naissent de trois sources :
//! - le relevé SNMP de chaque minute (`relever`) : nouvelle alerte de la machine (capot
//!   ouvert, support épuisé…), encre qui passe sous 5 % ou à 0 %, machine qui ne répond
//!   plus (3 relevés manqués) puis qui revient ;
//! - la file, comparée toutes les 5 s : sortie en échec, sortie terminée ;
//! - l'écriture d'un travail : rendu en échec.
//!
//! Le premier relevé et la première image de la file après un démarrage ne font que
//! poser l'état de départ : pas de rafale de notifications à chaque redémarrage.
//! Les 200 derniers événements sont gardés dans `<données>/mobile/evenements.json`
//! (l'écran « Alertes » de l'app), les téléphones dans `<données>/mobile/appareils.json`.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use uuid::Uuid;

/// Événements gardés pour l'écran « Alertes ».
const GARDES: usize = 200;
/// Relevés manqués de suite avant de dire qu'une machine ne répond plus.
const MANQUES_MAX: u32 = 3;
/// Seuil « encre basse », en %.
pub const ENCRE_BASSE: u8 = 5;
const EXPO_URL: &str = "https://exp.host/--/api/v2/push/send";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Appareil {
    /// Jeton Expo (`ExponentPushToken[…]`).
    pub jeton: String,
    pub nom: String,
    pub plateforme: String,
    /// Compte qui l'a inscrit ; `None` en accès libre.
    #[serde(default)]
    pub utilisateur: Option<String>,
    /// Prévenir aussi des fins d'impression et de découpe.
    #[serde(default)]
    pub fins: bool,
    pub inscrit_le: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Genre {
    AlerteMachine,
    EncreBasse,
    EncreVide,
    MachineInjoignable,
    MachineRevenue,
    SortieEchec,
    SortieTerminee,
    RenduEchec,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gravite {
    Erreur,
    Alerte,
    Info,
}

impl Genre {
    fn gravite(self) -> Gravite {
        match self {
            Genre::AlerteMachine | Genre::EncreVide | Genre::SortieEchec | Genre::RenduEchec => Gravite::Erreur,
            Genre::EncreBasse | Genre::MachineInjoignable => Gravite::Alerte,
            Genre::MachineRevenue | Genre::SortieTerminee => Gravite::Info,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Evenement {
    pub id: u64,
    /// Secondes Unix.
    pub le: u64,
    pub genre: Genre,
    pub gravite: Gravite,
    pub machine: Option<String>,
    pub titre: String,
    pub texte: String,
    pub sortie: Option<Uuid>,
    pub travail: Option<Uuid>,
}

/// Dernier état connu d'une machine relevée par SNMP.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct EtatMachine {
    pub encres: Vec<(String, u8)>,
    pub alertes: Vec<String>,
    pub longueur_m: Option<f64>,
    pub releve_le: Option<u64>,
    /// Relevés manqués de suite.
    #[serde(skip)]
    pub manques: u32,
    /// `MachineInjoignable` déjà signalé (attend `MachineRevenue`).
    #[serde(skip)]
    pub injoignable: bool,
}

impl EtatMachine {
    /// `None` : jamais relevée.
    pub fn joignable(&self) -> Option<bool> {
        if self.injoignable || self.manques >= MANQUES_MAX {
            Some(false)
        } else {
            self.releve_le.map(|_| true)
        }
    }
}

/// Une sortie de la file, vue par la veille.
pub struct VueSortie<'a> {
    pub id: Uuid,
    pub travail: Uuid,
    pub machine: &'a str,
    pub nom: &'a str,
    pub page: usize,
    /// Code de l'état (`en_cours`, `echec`…).
    pub etat: &'static str,
    pub decoupe: bool,
    pub erreur: Option<&'a str>,
}

#[derive(Default)]
struct Veille {
    charge: bool,
    evenements: Vec<Evenement>,
    machines: HashMap<String, EtatMachine>,
    /// État de chaque sortie à la dernière image ; `None` avant la première.
    sorties: Option<HashMap<Uuid, &'static str>>,
    /// Travaux dont l'échec de rendu a déjà été signalé.
    rendus_signales: HashSet<Uuid>,
}

/// Une veille par dossier de données (les essais en ouvrent plusieurs dans le processus).
static VEILLES: Mutex<BTreeMap<PathBuf, Veille>> = Mutex::new(BTreeMap::new());

fn dossier(donnees: &Path) -> PathBuf {
    donnees.join("mobile")
}

fn avec_veille<R>(donnees: &Path, f: impl FnOnce(&mut Veille) -> R) -> R {
    let mut v = VEILLES.lock().unwrap_or_else(|p| p.into_inner());
    let veille = v.entry(donnees.to_path_buf()).or_default();
    if !veille.charge {
        veille.charge = true;
        veille.evenements = crate::etat::lire(&dossier(donnees).join("evenements.json")).ok().flatten().unwrap_or_default();
    }
    f(veille)
}

fn maintenant() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

// --- Téléphones ---

pub fn appareils(donnees: &Path) -> Vec<Appareil> {
    crate::etat::lire(&dossier(donnees).join("appareils.json")).ok().flatten().unwrap_or_default()
}

fn ecrire_appareils(donnees: &Path, a: &[Appareil]) -> std::io::Result<()> {
    std::fs::create_dir_all(dossier(donnees))?;
    crate::etat::ecrire(&dossier(donnees).join("appareils.json"), a, true)
}

/// Jeton rendu par `getExpoPushTokenAsync` : rien d'autre ne part vers Expo.
pub fn jeton_valide(j: &str) -> bool {
    j.len() <= 200 && (j.starts_with("ExponentPushToken[") || j.starts_with("ExpoPushToken[")) && j.ends_with(']') && j.chars().all(|c| c.is_ascii_graphic())
}

/// Inscrit (ou met à jour) un téléphone : un jeton n'appartient qu'à un compte à la fois.
pub fn inscrire(donnees: &Path, mut a: Appareil) -> std::io::Result<()> {
    let _g = VERROU_APPAREILS.lock().unwrap_or_else(|p| p.into_inner());
    let mut liste = appareils(donnees);
    a.nom = a.nom.chars().filter(|c| !c.is_control()).take(80).collect();
    a.plateforme = if a.plateforme == "ios" { "ios".into() } else { "android".into() };
    liste.retain(|x| x.jeton != a.jeton);
    liste.push(a);
    ecrire_appareils(donnees, &liste)
}

/// Retire un téléphone ; `false` s'il n'était pas inscrit.
pub fn retirer(donnees: &Path, jeton: &str) -> std::io::Result<bool> {
    let _g = VERROU_APPAREILS.lock().unwrap_or_else(|p| p.into_inner());
    let mut liste = appareils(donnees);
    let avant = liste.len();
    liste.retain(|x| x.jeton != jeton);
    if liste.len() == avant {
        return Ok(false);
    }
    ecrire_appareils(donnees, &liste).map(|_| true)
}

static VERROU_APPAREILS: Mutex<()> = Mutex::new(());

// --- Événements ---

/// Événements plus récents que `depuis`, les plus récents d'abord ; et le dernier numéro.
pub fn evenements(donnees: &Path, depuis: u64, limite: usize) -> (Vec<Evenement>, u64) {
    avec_veille(donnees, |v| {
        let dernier = v.evenements.last().map_or(0, |e| e.id);
        (v.evenements.iter().rev().filter(|e| e.id > depuis).take(limite).cloned().collect(), dernier)
    })
}

pub fn etat_machine(donnees: &Path, machine: &str) -> Option<EtatMachine> {
    avec_veille(donnees, |v| v.machines.get(machine).cloned())
}

/// Garde les événements, puis les envoie aux téléphones (dans un fil à part : l'appelant
/// tient parfois un verrou).
/// Genre, machine, titre, texte, sortie, travail.
type Nouveau = (Genre, Option<String>, String, String, Option<Uuid>, Option<Uuid>);

fn publier(donnees: &Path, nouveaux: Vec<Nouveau>) {
    if nouveaux.is_empty() {
        return;
    }
    let publies = avec_veille(donnees, |v| {
        let mut id = v.evenements.last().map_or(0, |e| e.id);
        let le = maintenant();
        let publies: Vec<Evenement> = nouveaux
            .into_iter()
            .map(|(genre, machine, titre, texte, sortie, travail)| {
                id += 1;
                Evenement { id, le, genre, gravite: genre.gravite(), machine, titre, texte, sortie, travail }
            })
            .collect();
        v.evenements.extend(publies.iter().cloned());
        let trop = v.evenements.len().saturating_sub(GARDES);
        v.evenements.drain(..trop);
        let _ = std::fs::create_dir_all(dossier(donnees));
        if let Err(e) = crate::etat::ecrire(&dossier(donnees).join("evenements.json"), &v.evenements, false) {
            eprintln!("mobile : événements non gardés : {e}");
        }
        publies
    });
    let donnees = donnees.to_path_buf();
    std::thread::spawn(move || envoyer_aux_appareils(&donnees, &publies));
}

/// Alertes de la Printer-MIB qui ne disent qu'un état normal : pas de notification.
fn alerte_ordinaire(a: &str) -> bool {
    let a = a.to_ascii_lowercase();
    ["ready", "printing", "warm up", "warming", "sleep", "standby", "power sav", "energy sav", "idle", "online"].iter().any(|o| a.contains(o))
}

/// Ce qui a changé entre deux relevés d'une machine. `avant` absent : premier relevé
/// depuis le démarrage, il pose l'état sans rien signaler.
pub fn comparer_releves(avant: Option<&EtatMachine>, apres: &EtatMachine) -> Vec<(Genre, String)> {
    let Some(avant) = avant.filter(|a| a.releve_le.is_some()) else { return vec![] };
    let mut v = vec![];
    for a in apres.alertes.iter().filter(|a| !avant.alertes.contains(a) && !alerte_ordinaire(a)) {
        v.push((Genre::AlerteMachine, a.clone()));
    }
    for (encre, pc) in &apres.encres {
        let Some((_, pc_avant)) = avant.encres.iter().find(|(e, _)| e == encre) else { continue };
        if *pc == 0 && *pc_avant > 0 {
            v.push((Genre::EncreVide, encre.clone()));
        } else if *pc <= ENCRE_BASSE && *pc > 0 && *pc_avant > ENCRE_BASSE {
            v.push((Genre::EncreBasse, format!("{encre}:{pc}")));
        }
    }
    v
}

/// Relevé SNMP réussi d'une machine.
pub fn noter_releve(donnees: &Path, machine: &str, nom: &str, encres: &[(String, u8)], alertes: &[String], longueur_m: Option<f64>, le: u64) {
    let (changements, revenue) = avec_veille(donnees, |v| {
        let apres = EtatMachine { encres: encres.to_vec(), alertes: alertes.to_vec(), longueur_m, releve_le: Some(le), manques: 0, injoignable: false };
        let avant = v.machines.insert(machine.to_string(), apres.clone());
        let revenue = avant.as_ref().is_some_and(|a| a.injoignable);
        (comparer_releves(avant.as_ref(), &apres), revenue)
    });
    let m = Some(machine.to_string());
    let mut nouveaux = vec![];
    if revenue {
        nouveaux.push((Genre::MachineRevenue, m.clone(), format!("{nom} répond de nouveau"), "La machine répond de nouveau au relevé.".to_string(), None, None));
    }
    for (genre, detail) in changements {
        let (titre, texte) = match genre {
            Genre::AlerteMachine => (format!("{nom} : {detail}"), format!("Alerte de la machine : {detail}.")),
            Genre::EncreVide => (format!("{nom} : cartouche {detail} vide"), format!("L'encre {detail} est à 0 % : changer la cartouche.")),
            _ => {
                let (encre, pc) = detail.split_once(':').unwrap_or((&detail, "?"));
                (format!("{nom} : encre {encre} presque vide"), format!("Il reste {pc} % d'encre {encre}."))
            }
        };
        nouveaux.push((genre, m.clone(), titre, texte, None, None));
    }
    publier(donnees, nouveaux);
}

/// Relevé SNMP manqué (la machine ne répond pas).
pub fn noter_releve_manque(donnees: &Path, machine: &str, nom: &str) {
    let signaler = avec_veille(donnees, |v| {
        let e = v.machines.entry(machine.to_string()).or_default();
        e.manques += 1;
        // Jamais joignable depuis le démarrage : rien à signaler (pas de changement vu).
        let signaler = e.manques == MANQUES_MAX && e.releve_le.is_some() && !e.injoignable;
        if signaler {
            e.injoignable = true;
        }
        signaler
    });
    if signaler {
        publier(donnees, vec![(Genre::MachineInjoignable, Some(machine.to_string()), format!("{nom} ne répond plus"), "Trois relevés manqués : machine éteinte, débranchée du réseau ou bloquée ?".to_string(), None, None)]);
    }
}

/// Image de la file : sorties passées en échec ou terminées depuis la précédente.
pub fn noter_file(donnees: &Path, sorties: &[VueSortie], noms: &HashMap<String, String>) {
    let nouveaux = avec_veille(donnees, |v| {
        let image: HashMap<Uuid, &'static str> = sorties.iter().map(|s| (s.id, s.etat)).collect();
        let Some(avant) = v.sorties.replace(image) else { return vec![] };
        let mut n = vec![];
        for s in sorties {
            if avant.get(&s.id) == Some(&s.etat) || !matches!(s.etat, "echec" | "terminee") {
                continue;
            }
            // Une sortie apparue directement terminée (restauration…) ne compte pas.
            if s.etat == "terminee" && !avant.contains_key(&s.id) {
                continue;
            }
            let machine = noms.get(s.machine).map_or(s.machine, String::as_str);
            let quoi = if s.decoupe { "Découpe" } else { "Impression" };
            let (genre, titre, texte) = if s.etat == "echec" {
                (Genre::SortieEchec, format!("{quoi} en échec sur {machine}"), format!("{} (page {}){}", s.nom, s.page, s.erreur.map(|e| format!(" : {e}")).unwrap_or_default()))
            } else {
                (Genre::SortieTerminee, format!("{quoi} terminée sur {machine}"), format!("{} (page {})", s.nom, s.page))
            };
            n.push((genre, Some(s.machine.to_string()), titre, texte, Some(s.id), Some(s.travail)));
        }
        n
    });
    publier(donnees, nouveaux);
}

/// Travail écrit : un rendu en échec se signale une fois.
pub fn noter_travail(donnees: &Path, id: Uuid, nom: &str, echec: bool, erreur: Option<&str>) {
    let nouveau = avec_veille(donnees, |v| {
        if !echec {
            v.rendus_signales.remove(&id);
            return false;
        }
        v.rendus_signales.insert(id)
    });
    if nouveau {
        publier(donnees, vec![(Genre::RenduEchec, None, "Rendu en échec".to_string(), format!("{nom}{}", erreur.map(|e| format!(" : {e}")).unwrap_or_default()), None, Some(id))]);
    }
}

// --- Envoi par Expo ---

/// Message Expo d'un événement.
fn message(jeton: &str, e: &Evenement) -> Value {
    json!({
        "to": jeton,
        "title": e.titre,
        "body": e.texte,
        "sound": "default",
        "priority": if e.gravite == Gravite::Info { "normal" } else { "high" },
        "channelId": "alertes",
        "data": { "evenement": e },
    })
}

/// Les téléphones qui reçoivent `e` : un compte supprimé ne reçoit plus rien, les fins
/// ne vont qu'à qui les a demandées.
fn destinataires<'a>(appareils: &'a [Appareil], comptes: Option<&[String]>, e: &Evenement) -> Vec<&'a Appareil> {
    appareils
        .iter()
        .filter(|a| comptes.is_none_or(|c| a.utilisateur.as_ref().is_some_and(|u| c.contains(u))))
        .filter(|a| e.genre != Genre::SortieTerminee || a.fins)
        .collect()
}

fn envoyer_aux_appareils(donnees: &Path, evenements: &[Evenement]) {
    let liste = appareils(donnees);
    let comptes = (!crate::acces::libre(donnees)).then(|| crate::acces::lire(donnees).into_iter().map(|u| u.nom).collect::<Vec<_>>());
    let messages: Vec<Value> = evenements.iter().flat_map(|e| destinataires(&liste, comptes.as_deref(), e).into_iter().map(move |a| message(&a.jeton, e))).collect();
    if messages.is_empty() {
        return;
    }
    match pousser(&messages) {
        Ok(perimes) => {
            for j in perimes {
                let _ = retirer(donnees, &j);
            }
        }
        Err(e) => eprintln!("mobile : notifications non envoyées : {e}"),
    }
}

/// Envoie au service push d'Expo (100 messages par requête au plus) ; rend les jetons
/// que le service dit périmés (`DeviceNotRegistered`), à oublier.
pub fn pousser(messages: &[Value]) -> Result<Vec<String>, String> {
    let url = std::env::var("EKO_RIP_EXPO_URL").ok().filter(|u| !u.is_empty()).unwrap_or_else(|| EXPO_URL.into());
    let jeton_acces = std::env::var("EKO_RIP_EXPO_JETON").ok().filter(|j| !j.is_empty());
    let agent: ureq::Agent = ureq::Agent::config_builder().http_status_as_error(false).timeout_global(Some(Duration::from_secs(20))).user_agent(format!("eko-rip-serveur/{}", env!("CARGO_PKG_VERSION"))).build().into();
    let mut perimes = vec![];
    for lot in messages.chunks(100) {
        let mut r = agent.post(&url).header("Content-Type", "application/json").header("Accept", "application/json");
        if let Some(j) = &jeton_acces {
            r = r.header("Authorization", &format!("Bearer {j}"));
        }
        let mut rep = r.send(serde_json::to_vec(lot).unwrap()).map_err(|e| e.to_string())?;
        let statut = rep.status().as_u16();
        let texte = rep.body_mut().with_config().limit(1 << 20).read_to_string().unwrap_or_default();
        if statut != 200 {
            return Err(format!("Expo : HTTP {statut} : {}", texte.chars().take(300).collect::<String>()));
        }
        let corps: Value = serde_json::from_str(&texte).unwrap_or(Value::Null);
        for (m, ticket) in lot.iter().zip(corps["data"].as_array().into_iter().flatten()) {
            if ticket["details"]["error"] == "DeviceNotRegistered"
                && let Some(j) = m["to"].as_str()
            {
                perimes.push(j.to_string());
            }
        }
    }
    Ok(perimes)
}

/// Notification d'essai vers un téléphone (écran Réglages de l'app).
pub fn essai(jeton: &str) -> Result<(), String> {
    let e = Evenement { id: 0, le: maintenant(), genre: Genre::MachineRevenue, gravite: Gravite::Info, machine: None, titre: "E-KO RIP : essai".into(), texte: "Les notifications arrivent bien sur ce téléphone.".into(), sortie: None, travail: None };
    pousser(&[message(jeton, &e)]).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn releve(encres: &[(&str, u8)], alertes: &[&str]) -> EtatMachine {
        EtatMachine { encres: encres.iter().map(|(e, p)| (e.to_string(), *p)).collect(), alertes: alertes.iter().map(|a| a.to_string()).collect(), longueur_m: None, releve_le: Some(1), manques: 0, injoignable: false }
    }

    #[test]
    fn premier_releve_sans_notification() {
        assert!(comparer_releves(None, &releve(&[("C", 0)], &["printer cover open"])).is_empty());
    }

    #[test]
    fn alertes_et_encres_qui_changent() {
        let avant = releve(&[("C", 40), ("M", 6), ("Y", 3), ("BK", 1)], &["ready"]);
        let apres = releve(&[("C", 40), ("M", 5), ("Y", 2), ("BK", 0)], &["ready", "printing", "printer cover open"]);
        assert_eq!(comparer_releves(Some(&avant), &apres), [(Genre::AlerteMachine, "printer cover open".into()), (Genre::EncreBasse, "M:5".into()), (Genre::EncreVide, "BK".into())]);
        // Rien ne change : rien à dire ; une cartouche changée (hausse) non plus.
        assert!(comparer_releves(Some(&apres), &apres).is_empty());
        assert!(comparer_releves(Some(&apres), &releve(&[("C", 40), ("M", 100), ("Y", 2), ("BK", 100)], &["ready"])).is_empty());
    }

    #[test]
    fn jetons_acceptes() {
        assert!(jeton_valide("ExponentPushToken[xxxxxxxxxxxxxxxxxxxxxx]"));
        assert!(!jeton_valide("https://exemple.fr"));
        assert!(!jeton_valide("ExponentPushToken[a b]"));
    }

    #[test]
    fn destinataires_selon_compte_et_fins() {
        let a = |j: &str, u: Option<&str>, fins: bool| Appareil { jeton: j.into(), nom: String::new(), plateforme: "ios".into(), utilisateur: u.map(Into::into), fins, inscrit_le: 0 };
        let liste = [a("1", Some("paul"), false), a("2", Some("supprime"), true), a("3", Some("marie"), true)];
        let ev = |genre| Evenement { id: 1, le: 0, genre, gravite: Genre::gravite(genre), machine: None, titre: String::new(), texte: String::new(), sortie: None, travail: None };
        let comptes = vec!["paul".to_string(), "marie".to_string()];
        let jetons = |g| destinataires(&liste, Some(&comptes), &ev(g)).iter().map(|a| a.jeton.clone()).collect::<Vec<_>>();
        assert_eq!(jetons(Genre::EncreVide), ["1", "3"]);
        assert_eq!(jetons(Genre::SortieTerminee), ["3"]);
        assert_eq!(destinataires(&liste, None, &ev(Genre::EncreVide)).len(), 3, "accès libre : tous");
    }

    #[test]
    fn file_injoignable_et_rendu() {
        let donnees = std::env::temp_dir().join(format!("eko-rip-mobile-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&donnees);
        std::fs::create_dir_all(&donnees).unwrap();
        // Pas de téléphone inscrit : rien ne part vers Expo.
        let (s1, t) = (Uuid::new_v4(), Uuid::new_v4());
        let vue = |etat| [VueSortie { id: s1, travail: t, machine: "epson", nom: "a.pdf", page: 1, etat, decoupe: false, erreur: Some("connexion refusée") }];
        let noms = HashMap::from([("epson".to_string(), "Epson".to_string())]);
        noter_file(&donnees, &vue("en_cours"), &noms);
        noter_file(&donnees, &vue("en_cours"), &noms);
        assert!(evenements(&donnees, 0, 50).0.is_empty(), "première image : état de départ");
        noter_file(&donnees, &vue("echec"), &noms);
        let (ev, dernier) = evenements(&donnees, 0, 50);
        assert_eq!((ev.len(), ev[0].genre, dernier), (1, Genre::SortieEchec, 1));
        assert_eq!(ev[0].texte, "a.pdf (page 1) : connexion refusée");

        noter_releve(&donnees, "epson", "Epson", &[("C".into(), 50)], &[], None, 10);
        for _ in 0..4 {
            noter_releve_manque(&donnees, "epson", "Epson");
        }
        assert_eq!(etat_machine(&donnees, "epson").unwrap().joignable(), Some(false));
        noter_releve(&donnees, "epson", "Epson", &[("C".into(), 0)], &[], None, 20);
        let genres: Vec<Genre> = evenements(&donnees, 1, 50).0.iter().map(|e| e.genre).collect();
        assert_eq!(genres, [Genre::EncreVide, Genre::MachineRevenue, Genre::MachineInjoignable]);

        noter_travail(&donnees, t, "b.pdf", true, None);
        noter_travail(&donnees, t, "b.pdf", true, None);
        assert_eq!(evenements(&donnees, 4, 50).0.len(), 1, "un rendu en échec se signale une fois");
        // Relu depuis le disque après un redémarrage.
        VEILLES.lock().unwrap().remove(&donnees);
        assert_eq!(evenements(&donnees, 0, 50).1, 5);
        let _ = std::fs::remove_dir_all(&donnees);
    }
}
