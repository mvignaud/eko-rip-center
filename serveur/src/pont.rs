//! Pont ERP ↔ RIP, lot R2 (contrat `eko-rip-erp/1`) : journal des événements sortants et
//! canal vers l'ERP.
//!
//! - **Journal** (`<données>/pont/journal.jsonl`) : un événement par ligne, ajouté puis
//!   synchronisé sur le disque avant toute autre chose ; `seq` croît sans trou pour une
//!   instance. Une ligne coupée par une coupure de courant est retirée à la relecture (elle
//!   n'a jamais été envoyée ni comptée). Les événements accusés par l'ERP sont purgés après
//!   `RETENTION_S` ; les autres restent jusqu'à leur accusé.
//! - **État** (`<données>/pont/etat.json`) : dernier `seq`, dernier accusé, statut (actif,
//!   remplacé, anomalie), dernière erreur, empreinte du dernier catalogue, et le dernier état
//!   publié de chaque travail et de chaque sortie (un événement part quand il change).
//! - **Canal** : `GET <base>/en-attente?instance=<uuid>` puis `POST <base>/evenements` par
//!   lots, signés (`X-EKO-RIP-Signature`). Toutes les 30 s, 5 s tant qu'une demande attend,
//!   aussitôt qu'un événement s'ajoute ; après une erreur, 60 s puis 5 min.
//!
//! - **Travaux de l'ERP** (lot R3) : chaque élément nouveau de `travaux` est téléchargé
//!   (`GET <base><fichier.chemin>`, signé) dans `<données>/pont/recus/<id>/`, puis pris par le
//!   serveur (`main::prendre_envois`), qui en fait un travail de même uuid avec le réglage
//!   désigné. Le premier `travail.etat` porte `envoi_id` : c'est l'accusé. Un envoi qui ne
//!   peut pas devenir un travail part en `travail.refuse`. Un envoi déjà pris n'est jamais
//!   repris (`Etat::envois`), même reproposé.
//!
//! Règles d'instance convenues avec l'ERP (25/10) : réponse 0 pour une instance jamais
//! accusée → tout est rejoué depuis 1 ; pour une instance déjà accusée → « remplacé » : plus
//! rien n'est envoyé, jusqu'à une nouvelle instance tirée par l'administrateur ; un accusé
//! plus grand que le dernier `seq` → « anomalie » : arrêt, jamais de renumérotation.

use crate::erp::{self, Evenement};
use crate::file::{self, Sortie};
use crate::machines::{self, Machine};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Les événements accusés sont gardés 30 jours (rejeu sur demande), puis purgés.
const RETENTION_S: u64 = 30 * 86_400;
/// Événements par `POST` au plus.
const LOT: usize = 100;
/// Lots par cycle au plus (un rejeu de 5 000 événements prend plusieurs cycles).
const LOTS_PAR_CYCLE: usize = 50;
pub const CADENCE: Duration = Duration::from_secs(30);
pub const PRESSE: Duration = Duration::from_secs(5);
pub const ATTENTE_PREMIERE_ERREUR: Duration = Duration::from_secs(60);
pub const ATTENTE_ERREURS: Duration = Duration::from_secs(300);

// ponytail: un seul verrou pour tous les journaux du processus (un seul en production) ;
// les écritures sont courtes, jamais faites pendant un appel réseau.
static VERROU: Mutex<()> = Mutex::new(());
static REVEIL: LazyLock<tokio::sync::Notify> = LazyLock::new(tokio::sync::Notify::new);
static RECUS: LazyLock<tokio::sync::Notify> = LazyLock::new(tokio::sync::Notify::new);

/// Téléchargements d'envois de l'ERP par cycle au plus : le reste attend le cycle suivant.
const TELECHARGEMENTS_PAR_CYCLE: usize = 5;
/// Taille d'un fichier envoyé par l'ERP au plus.
const FICHIER_MAX: u64 = 4 << 30;

/// Réveille la boucle du pont (événement ajouté, réglage ou catalogue modifié).
pub fn reveiller() {
    REVEIL.notify_one();
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Statut {
    #[default]
    Actif,
    /// L'ERP a répondu 0 pour une instance qu'il avait déjà accusée : une autre installation
    /// a pris la place. Plus rien n'est envoyé.
    Remplace,
    /// Accusé plus grand que le dernier `seq`, ou rejeu impossible : arrêt.
    Anomalie,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Erreur {
    /// Code stable : celui de l'ERP (`signature_invalide`…) ou du pont (`erp_injoignable`…).
    pub code: String,
    pub detail: Option<String>,
    /// Statut HTTP de la réponse, s'il y en a eu une.
    pub statut: Option<u16>,
    pub le: u64,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Etat {
    /// Instance du journal ; une autre instance dans `erp.json` repart d'un journal vide.
    #[serde(default)]
    pub instance: Option<Uuid>,
    /// Ouverture du journal : un travail ou une sortie créés avant, vus pour la première
    /// fois, sont pris tels quels sans événement (pas de déluge à l'activation).
    #[serde(default)]
    pub depuis: u64,
    #[serde(default)]
    pub dernier_seq: u64,
    /// Dernier `dernier_seq_traite` reçu pour cette instance : tout ce qui suit est renvoyé.
    #[serde(default)]
    pub accuse: u64,
    /// L'ERP a déjà accusé un `seq` > 0 pour cette instance.
    #[serde(default)]
    pub deja_accuse: bool,
    #[serde(default)]
    pub statut: Statut,
    #[serde(default)]
    pub erreur: Option<Erreur>,
    #[serde(default)]
    pub dernier_contact: Option<u64>,
    #[serde(default)]
    pub dernier_envoi: Option<u64>,
    /// Empreinte SHA-256 du dernier catalogue journalisé.
    #[serde(default)]
    pub catalogue: Option<String>,
    /// Dernier état publié (`etat|code_erreur`) de chaque travail et de chaque sortie.
    #[serde(default)]
    pub travaux: BTreeMap<Uuid, String>,
    #[serde(default)]
    pub sorties: BTreeMap<Uuid, String>,
    /// Dernier état publié de chaque machine (données de `machine.etat` sans l'heure).
    #[serde(default)]
    pub machines: BTreeMap<String, String>,
    #[serde(default)]
    pub purge_le: u64,
    /// Envois de l'ERP déjà pris (téléchargés ou refusés), avec l'heure : jamais repris.
    /// Oubliés quand l'ERP ne les propose plus et après `RETENTION_S`.
    #[serde(default)]
    pub envois: BTreeMap<Uuid, u64>,
    /// `ref_erp` de chaque travail né d'un envoi, recopié dans ses sorties.
    #[serde(default)]
    pub refs: BTreeMap<Uuid, String>,
}

/// Lien d'un travail à l'ERP (lot R3) : l'envoi qui l'a créé, et le dossier de l'ERP.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lien {
    pub envoi: Uuid,
    pub ref_erp: String,
}

/// Un envoi de l'ERP téléchargé, qui attend de devenir un travail (`pont/recus/<id>/`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Recu {
    pub id: Uuid,
    pub ref_erp: String,
    /// Nom du fichier chez l'ERP, pour l'affichage seulement.
    pub nom: String,
    pub prereglage_uuid: Uuid,
    pub copies: u32,
    pub echeance: Option<u64>,
    /// SHA-256 du fichier donné par l'ERP (hexadécimal), vérifié au téléchargement.
    #[serde(default)]
    pub sha256: Option<String>,
}

fn dossier(donnees: &Path) -> PathBuf {
    donnees.join("pont")
}

fn fichier_etat(donnees: &Path) -> PathBuf {
    dossier(donnees).join("etat.json")
}

fn fichier_journal(donnees: &Path) -> PathBuf {
    dossier(donnees).join("journal.jsonl")
}

fn maintenant() -> u64 {
    crate::maintenant()
}

fn dossier_recus(donnees: &Path) -> PathBuf {
    dossier(donnees).join("recus")
}

/// Journal ouvert : son état, sous le verrou.
struct Ouvert<'a> {
    _verrou: MutexGuard<'a, ()>,
    donnees: PathBuf,
    e: Etat,
    instance: Uuid,
}

impl Ouvert<'_> {
    /// Ajoute un événement au journal (ligne synchronisée sur le disque), puis avance `seq`.
    fn ajouter(&mut self, genre: &str, d: Value) -> std::io::Result<()> {
        let ev = Evenement { contrat: erp::CONTRAT.into(), instance: self.instance, seq: self.e.dernier_seq + 1, id: Uuid::new_v4(), genre: genre.into(), le: maintenant(), donnees: d };
        let chemin = fichier_journal(&self.donnees);
        let nouveau = !chemin.exists();
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&chemin)?;
        f.write_all(format!("{}\n", serde_json::to_string(&ev)?).as_bytes())?;
        f.sync_data()?;
        if nouveau {
            let _ = std::fs::File::open(dossier(&self.donnees)).and_then(|d| d.sync_all());
        }
        self.e.dernier_seq = ev.seq;
        reveiller();
        Ok(())
    }

    fn fermer(self) {
        if let Err(e) = crate::etat::ecrire(&fichier_etat(&self.donnees), &self.e, false) {
            eprintln!("pont ERP : état non écrit : {e}");
        }
    }
}

/// Dernier événement du journal. Une dernière ligne sans fin de ligne (coupure pendant
/// l'écriture : jamais envoyée, jamais comptée) est retirée du fichier.
fn dernier_du_journal(chemin: &Path) -> Option<Evenement> {
    let mut f = std::fs::OpenOptions::new().read(true).write(true).open(chemin).ok()?;
    let longueur = f.metadata().ok()?.len();
    let mut taille = 4096u64;
    loop {
        let debut = longueur.saturating_sub(taille);
        let mut fin = vec![];
        f.seek(std::io::SeekFrom::Start(debut)).ok()?;
        f.read_to_end(&mut fin).ok()?;
        let derniere_nl = fin.iter().rposition(|&b| b == b'\n');
        if derniere_nl.is_none() && debut > 0 {
            taille *= 4;
            continue;
        }
        // Fin coupée : on retire ce qui suit la dernière fin de ligne.
        let garde = derniere_nl.map_or(0, |i| i + 1);
        if garde < fin.len() {
            eprintln!("pont ERP : dernière ligne du journal coupée ({} octets), retirée", fin.len() - garde);
            f.set_len(debut + garde as u64).ok()?;
            let _ = f.sync_all();
            fin.truncate(garde);
        }
        let corps = fin.strip_suffix(b"\n").unwrap_or(&fin);
        let ligne = match corps.iter().rposition(|&b| b == b'\n') {
            Some(i) => &corps[i + 1..],
            None if debut == 0 || corps.is_empty() => corps,
            None => {
                taille *= 4;
                continue;
            }
        };
        return serde_json::from_slice(ligne).ok();
    }
}

/// Ouvre le journal si le pont est actif. Une instance changée (nouvelle instance tirée,
/// sauvegarde restaurée) repart d'un journal vide ; un état illisible bloque tout plutôt
/// que de repartir de zéro (l'alerte de `etat` le signale).
fn ouvrir(donnees: &Path) -> Option<Ouvert<'static>> {
    let c = erp::lire(donnees);
    let instance = c.instance.filter(|_| c.actif)?;
    let verrou = VERROU.lock().unwrap_or_else(|p| p.into_inner());
    let _ = std::fs::create_dir_all(dossier(donnees));
    let mut e: Etat = match crate::etat::lire(&fichier_etat(donnees)) {
        Ok(e) => e.unwrap_or_default(),
        Err(()) => return None,
    };
    let dernier = dernier_du_journal(&fichier_journal(donnees));
    if e.instance != Some(instance) {
        if dernier.as_ref().is_some_and(|d| d.instance == instance) {
            // État perdu, journal intact : on le reprend.
            e = Etat { instance: Some(instance), depuis: maintenant(), ..Default::default() };
        } else {
            let _ = std::fs::remove_file(fichier_journal(donnees));
            e = Etat { instance: Some(instance), depuis: maintenant(), ..Default::default() };
            return Some(Ouvert { _verrou: verrou, donnees: donnees.into(), e, instance });
        }
    }
    // L'état peut être en retard d'un événement (coupure entre l'ajout et son écriture) : le
    // journal fait foi.
    if let Some(d) = dernier.filter(|d| d.instance == instance) {
        e.dernier_seq = e.dernier_seq.max(d.seq);
    }
    Some(Ouvert { _verrou: verrou, donnees: donnees.into(), e, instance })
}

/// Au démarrage du serveur : reprise du journal après une coupure, oubli des travaux effacés.
pub fn demarrer(donnees: &Path) {
    if let Some(mut o) = ouvrir(donnees) {
        let existe = |id: &Uuid| crate::dossier(donnees, *id).exists();
        o.e.travaux.retain(|id, _| existe(id));
        o.e.refs.retain(|id, _| existe(id));
        o.fermer();
    }
}

fn lire_journal(donnees: &Path) -> Vec<Evenement> {
    std::fs::read_to_string(fichier_journal(donnees))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| match serde_json::from_str(l) {
            Ok(e) => Some(e),
            Err(_) => {
                eprintln!("pont ERP : ligne illisible dans le journal, ignorée");
                None
            }
        })
        .collect()
}

/// Retire les événements accusés plus vieux que la rétention (fichier réécrit puis renommé).
fn purger(o: &mut Ouvert) {
    let limite = maintenant().saturating_sub(RETENTION_S);
    let tout = lire_journal(&o.donnees);
    let garde: Vec<&Evenement> = tout.iter().filter(|e| e.seq > o.e.accuse || e.le >= limite).collect();
    if garde.len() < tout.len() {
        let texte: String = garde.iter().filter_map(|e| serde_json::to_string(e).ok()).map(|l| l + "\n").collect();
        let chemin = fichier_journal(&o.donnees);
        let temp = chemin.with_extension("tmp");
        let ecrit = std::fs::File::create(&temp).and_then(|mut f| f.write_all(texte.as_bytes()).and_then(|()| f.sync_all())).and_then(|()| std::fs::rename(&temp, &chemin));
        if let Err(e) = ecrit {
            eprintln!("pont ERP : purge du journal impossible : {e}");
        }
    }
    o.e.purge_le = maintenant();
}

// --- Événements d'état (roadmap 7.4) et de consommation (7.5) ---

/// Un travail a été écrit (`job.json`) : `travail.etat` si son état a changé. Né d'un envoi
/// de l'ERP (`lien`), il porte `ref_erp` et `envoi_id` (l'accusé de l'envoi).
pub fn noter_travail(donnees: &Path, id: Uuid, etat: &str, code_erreur: Option<&str>, pages: usize, cree_le: u64, lien: Option<&Lien>) {
    let Some(mut o) = ouvrir(donnees) else { return };
    let signe = format!("{etat}|{code_erreur:?}");
    if let Some(l) = lien {
        o.e.refs.insert(id, l.ref_erp.clone());
    }
    match o.e.travaux.get(&id) {
        Some(s) if *s == signe => return,
        None if cree_le < o.e.depuis => {}
        _ => {
            let mut d = json!({ "travail_uuid": id, "ref_erp": lien.map(|l| &l.ref_erp), "etat": etat, "code_erreur": code_erreur, "pages": pages });
            if let Some(l) = lien {
                d["envoi_id"] = json!(l.envoi);
            }
            if let Err(e) = o.ajouter("travail.etat", d) {
                eprintln!("pont ERP : événement non journalisé : {e}");
                return;
            }
        }
    }
    o.e.travaux.insert(id, signe);
    o.fermer();
}

/// État relevé d'une machine (roadmap 7.9) : `machine.etat` quand il change (l'heure seule ne
/// compte pas). Le bac d'encre usagée n'est pas encore lu sur l'Epson : `null`.
pub fn noter_machine(donnees: &Path, machine: &str, r: &crate::releves::Releve, chauffages: &[(u8, u8)], alertes: &[String]) {
    let Some(mut o) = ouvrir(donnees) else { return };
    let machines = machines::lire(donnees).unwrap_or_default();
    let charge = crate::supports::lire_charges(donnees).remove(machine);
    let d = json!({
        "machine_uuid": uuid_machine(&machines, machine),
        "machine": machine,
        "encres": r.encres.iter().map(|(e, pc)| json!({ "encre": e, "pourcent": pc })).collect::<Vec<_>>(),
        "longueur_restante_m": r.longueur_m,
        "support_uuid": charge.as_ref().and_then(|c| c.support_uuid),
        "laize_mm": charge.as_ref().and_then(|c| c.laize_mm),
        "chauffages": chauffages.iter().map(|&(ecran, autre)| json!({ "ecran_c": ecran, "autre_c": autre })).collect::<Vec<_>>(),
        "alertes": alertes,
        "bac_encre_usagee_pourcent": null,
    });
    let signe = d.to_string();
    if o.e.machines.get(machine) == Some(&signe) {
        return;
    }
    let mut avec_heure = d;
    avec_heure["releve_le"] = json!(r.le);
    if let Err(e) = o.ajouter("machine.etat", avec_heure) {
        eprintln!("pont ERP : événement non journalisé : {e}");
        return;
    }
    o.e.machines.insert(machine.into(), signe);
    o.fermer();
}

fn code<T: Serialize>(v: T) -> String {
    serde_json::to_value(v).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default()
}

/// La file a été écrite (`file.json`) : `sortie.etat` pour chaque sortie dont l'état a
/// changé, et `travail.consommation` pour chaque impression ou découpe terminée.
pub fn noter_sorties(donnees: &Path, sorties: &[Sortie]) {
    let Some(mut o) = ouvrir(donnees) else { return };
    let mut change = false;
    let presentes: std::collections::HashSet<Uuid> = sorties.iter().map(|s| s.id).collect();
    let avant = o.e.sorties.len();
    o.e.sorties.retain(|id, _| presentes.contains(id));
    change |= o.e.sorties.len() != avant;
    let mut catalogue: Option<Vec<Machine>> = None;
    for s in sorties {
        let signe = format!("{}|{:?}", code(s.etat), s.code_erreur);
        match o.e.sorties.get(&s.id) {
            Some(x) if *x == signe => continue,
            None if s.cree_le < o.e.depuis => {}
            _ => {
                let machines = catalogue.get_or_insert_with(|| machines::lire(donnees).unwrap_or_default());
                let mut evenements = vec![("sortie.etat", donnees_sortie(donnees, s, sorties, machines, o.e.refs.get(&s.travail)))];
                if s.etat == file::Etat::Terminee && s.planche.is_none() {
                    evenements.push(("travail.consommation", consommation(donnees, s, machines, o.e.refs.get(&s.travail))));
                }
                for (genre, d) in evenements {
                    if let Err(e) = o.ajouter(genre, d) {
                        eprintln!("pont ERP : événement non journalisé : {e}");
                        o.fermer();
                        return;
                    }
                }
            }
        }
        o.e.sorties.insert(s.id, signe);
        change = true;
    }
    if change {
        o.fermer();
    }
}

fn uuid_machine(machines: &[Machine], id: &str) -> Option<Uuid> {
    machines.iter().find(|m| m.id == id).and_then(|m| m.uuid)
}

fn donnees_sortie(donnees: &Path, s: &Sortie, sorties: &[Sortie], machines: &[Machine], ref_erp: Option<&String>) -> Value {
    let charges = crate::supports::lire_charges(donnees);
    json!({
        "sortie_uuid": s.id,
        "travail_uuid": s.travail,
        "ref_erp": ref_erp,
        "page": s.page,
        "machine_uuid": uuid_machine(machines, &s.machine),
        "machine": s.machine,
        "operation": code(s.operation()),
        "etat": code(s.etat),
        "code_erreur": s.code_erreur,
        "raison_attente": crate::raison_de(donnees, s, sorties, machines, &charges),
        "planche_uuid": s.planche,
    })
}

/// Consommation d'une sortie terminée (roadmap 7.5). Rien n'est inventé : sans relevé de
/// la machine, l'encre et la longueur réelles valent `null` et `encre_source` dit d'où vient
/// l'encre donnée (`reel`, `estime` par les points posés, ou `null`). Aucun montant.
fn consommation(donnees: &Path, s: &Sortie, machines: &[Machine], ref_erp: Option<&String>) -> Value {
    let page = crate::lire_page(donnees, s.travail, s.page);
    let demande = page.as_ref().map(|(t, _)| &t.demande);
    let impression = s.operation() == machines::Operation::Impression;
    let ligne = impression.then(|| crate::lignes_du_journal(donnees, None, None).into_iter().rev().find(|l| l.impression.sortie == s.id)).flatten();
    let reel_ml = ligne.as_ref().and_then(|l| l.reel_ml.clone());
    let estime_ml = ligne.as_ref().filter(|l| l.impression.estime.methode == crate::consommation::Methode::Points).map(|l| l.estime_ml.clone());
    let encre_source = if reel_ml.is_some() { Some("reel") } else { estime_ml.as_ref().map(|_| "estime") };
    let longueur_mm = ligne.as_ref().and_then(|l| l.longueur_reelle_mm);
    let copies = match &page {
        Some((t, _)) if !impression => t.mise_coupe.copies.iter().product::<u32>(),
        _ => 1,
    };
    json!({
        "travail_uuid": s.travail,
        "sortie_uuid": s.id,
        "ref_erp": ref_erp,
        "page": s.page,
        "operation": code(s.operation()),
        "machine_uuid": uuid_machine(machines, &s.machine),
        "machine": s.machine,
        "support_uuid": demande.and_then(|d| d.support_uuid),
        "mode_uuid": demande.and_then(|d| d.mode_uuid),
        "couche": s.couche,
        "copies": copies,
        "surface_m2": page.as_ref().filter(|_| impression).map(|(_, p)| p.surface_m2),
        "longueur_prevue_mm": page.as_ref().filter(|_| impression).map(|(_, p)| p.hauteur_mm),
        "longueur_mm": longueur_mm,
        "longueur_source": longueur_mm.map(|_| "reel"),
        "gache_mm": ligne.as_ref().and_then(|l| l.gache_mm),
        "encre_ml": reel_ml,
        "encre_sous_seuil_ml": ligne.as_ref().and_then(|l| l.sous_seuil_ml),
        "encre_ml_estimee": estime_ml,
        "encre_source": encre_source,
        "duree_s": s.debut.zip(s.fin).map(|(d, f)| f.saturating_sub(d)),
        "debut": s.debut,
        "fin": s.fin,
    })
}

// --- Catalogue (roadmap 7.2) ---

/// Catalogue complet (`catalogue.modifie`) : seuls les objets qui ont un `uuid` (tous, une
/// fois le serveur démarré) ; les profils ICC sont identifiés par leur fichier. Aucun prix.
pub fn catalogue(donnees: &Path) -> Value {
    let machines = machines::lire(donnees).unwrap_or_default();
    let par_id = |id: &str| uuid_machine(&machines, id);
    let supports = crate::supports::lire(donnees, "epson").supports;
    let reglages = crate::reglages::lire(donnees);
    let mut l_machines = vec![];
    let mut l_decoupeuses = vec![];
    let mut l_couches = vec![];
    for m in &machines {
        let Some(u) = m.uuid else { continue };
        let fiche = json!({ "uuid": u, "id": m.id, "nom": m.nom, "modele": code(m.modele), "laize_mm": m.laize_mm, "cadence_m2_h": m.m2_heure, "pool": m.pool, "variante_de": m.variante_de });
        if m.modele.imprime() {
            l_machines.push(fiche.clone());
        }
        if m.modele.decoupe() {
            l_decoupeuses.push(fiche);
            for p in m.parametres_decoupe.iter().filter(|p| p.uuid.is_some()) {
                l_couches.push(json!({ "uuid": p.uuid, "nom": p.nom, "machine_uuid": u, "vitesse_cm_s": p.vitesse_cm_s, "force_g": p.force_g, "acceleration_g": p.acceleration_g, "valeurs_machine": p.valeurs_machine }));
            }
        }
    }
    let mut l_supports = vec![];
    let mut l_modes = vec![];
    for s in supports.iter().filter(|s| s.id.is_some()) {
        let machine_uuid = par_id(&s.machine);
        l_supports.push(json!({ "uuid": s.id, "nom": s.nom, "machine_uuid": machine_uuid, "laize_mm": s.laize_mm, "encres": s.encres, "numero_machine": s.numero_machine }));
        for m in s.modes.iter().filter(|m| m.id.is_some()) {
            let cadence = machines.iter().find(|x| x.id == s.machine).and_then(|x| x.m2_heure);
            l_modes.push(json!({ "uuid": m.id, "nom": m.nom, "machine_uuid": machine_uuid, "support_uuid": s.id, "resolution_dpi": format!("{}x{}", m.dpi[0], m.dpi[1]), "passes": m.passes, "chauffage_c": m.chauffage_c, "cadence_m2_h": cadence }));
        }
    }
    let prereglages: Vec<Value> = reglages
        .iter()
        .filter(|r| r.uuid.is_some())
        .map(|r| {
            let d = &r.demande;
            let support = supports.iter().find(|s| d.support_uuid.is_some() && s.id == d.support_uuid);
            json!({
                "uuid": r.uuid,
                "nom": r.nom,
                "machine_uuid": support.and_then(|s| par_id(&s.machine)),
                "support_uuid": d.support_uuid,
                "mode_uuid": d.mode_uuid,
                "profil": d.couleur.sortie,
                "decoupeuse_uuid": d.decoupeuse.as_deref().and_then(par_id),
                "operation": code(d.mode),
                "dpi": d.dpi,
            })
        })
        .collect();
    let profils: Vec<Value> = crate::couleur::lister(&donnees.join("profils"))
        .into_iter()
        .map(|p| json!({ "fichier": p.fichier, "nom": if p.description.is_empty() { p.fichier.clone() } else { p.description.clone() }, "classe": p.classe, "espace": p.espace }))
        .collect();
    // Champs inconnus omis plutôt que `null` : l'ERP valide un `machine_uuid` présent.
    let sans_nuls = |v: Vec<Value>| -> Vec<Value> {
        v.into_iter()
            .map(|mut o| {
                if let Some(m) = o.as_object_mut() {
                    m.retain(|_, x| !x.is_null());
                }
                o
            })
            .collect()
    };
    let (l_machines, l_supports, profils, l_modes, prereglages, l_decoupeuses, l_couches) = (sans_nuls(l_machines), sans_nuls(l_supports), sans_nuls(profils), sans_nuls(l_modes), sans_nuls(prereglages), sans_nuls(l_decoupeuses), sans_nuls(l_couches));
    json!({
        "serveur": env!("CARGO_PKG_VERSION"),
        "machines": l_machines,
        "supports": l_supports,
        "profils": profils,
        "modes": l_modes,
        "prereglages": prereglages,
        "decoupeuses": l_decoupeuses,
        "couches_decoupe": l_couches,
    })
}

fn empreinte(v: &Value) -> String {
    erp::empreinte(&v.to_string())
}

// --- Travaux envoyés par l'ERP (lot R3) ---

/// Attend qu'un envoi de l'ERP soit téléchargé, `au_plus` en tout cas.
pub async fn attendre_recus(au_plus: Duration) {
    let _ = tokio::time::timeout(au_plus, RECUS.notified()).await;
}

/// Les envois téléchargés qui attendent de devenir un travail, avec leur fichier.
pub fn recus(donnees: &Path) -> Vec<(Recu, PathBuf)> {
    let Ok(entrees) = std::fs::read_dir(dossier_recus(donnees)) else { return vec![] };
    let mut v: Vec<(Recu, PathBuf)> = entrees
        .flatten()
        .filter_map(|e| {
            let rep = e.path();
            let r: Recu = serde_json::from_slice(&std::fs::read(rep.join("envoi.json")).ok()?).ok()?;
            Some((r, rep.join("source")))
        })
        .collect();
    v.sort_by_key(|(r, _)| r.id);
    v
}

/// L'envoi est devenu un travail, ou refusé : son dossier de réception disparaît.
pub fn oublier_recu(donnees: &Path, id: Uuid) {
    let _ = std::fs::remove_dir_all(dossier_recus(donnees).join(id.to_string()));
}

/// `travail.refuse` : l'envoi ne deviendra pas un travail (`code` stable, `detail` lisible).
pub fn noter_refus(donnees: &Path, envoi: Uuid, ref_erp: Option<&str>, code: &str, detail: Option<&str>) {
    let Some(mut o) = ouvrir(donnees) else { return };
    o.e.envois.insert(envoi, maintenant());
    let d = json!({ "envoi_id": envoi, "ref_erp": ref_erp, "code": code, "detail": detail.map(|d| d.chars().take(500).collect::<String>()) });
    if let Err(e) = o.ajouter("travail.refuse", d) {
        eprintln!("pont ERP : refus non journalisé : {e}");
    }
    o.fermer();
}

/// Codes de `travail.refuse`, pour l'ERP seulement (jamais affichés par l'écran).
pub const REFUS_REGLAGE_INCONNU: &str = "reglage_inconnu";
pub const REFUS_FICHIER_ILLISIBLE: &str = "fichier_illisible";
const REFUS_ENVOI_INVALIDE: &str = "envoi_invalide";
const REFUS_TELECHARGEMENT: &str = "telechargement_impossible";
const REFUS_EMPREINTE: &str = "empreinte_fausse";

enum Echec {
    /// L'envoi ne peut pas être pris : `travail.refuse`.
    Refus(&'static str, String),
    /// Réseau, stockage : on réessaiera au cycle suivant.
    PlusTard(String),
}

/// Lit un élément de `travaux` (§ 11 du contrat côté ERP).
fn lire_envoi(t: &Value) -> Result<(Recu, String), String> {
    let id = t["id"].as_str().and_then(|i| Uuid::parse_str(i).ok()).ok_or("id")?;
    let ref_erp = t["ref_erp"].as_str().map(str::trim).filter(|v| !v.is_empty() && v.chars().count() <= 200).ok_or("ref_erp")?.to_string();
    let prereglage_uuid = t["prereglage_uuid"].as_str().and_then(|i| Uuid::parse_str(i).ok()).ok_or("prereglage_uuid")?;
    let copies = t["copies"].as_u64().filter(|c| (1..=100_000).contains(c)).ok_or("copies")? as u32;
    let echeance = t["echeance"].as_u64();
    let chemin = t["fichier"]["chemin"].as_str().filter(|c| c.starts_with('/') && !c.contains("..") && !c.contains("//") && !c.contains('#') && c.len() <= 500).ok_or("fichier.chemin")?.to_string();
    let sha256 = match &t["fichier"]["sha256"] {
        Value::Null => None,
        v => Some(v.as_str().filter(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit())).ok_or("fichier.sha256")?.to_ascii_lowercase()),
    };
    let nom = t["fichier"]["nom"].as_str().or(t["nom"].as_str()).unwrap_or("sans nom").chars().filter(|c| !c.is_control() && *c != '/' && *c != '\\').take(200).collect::<String>();
    Ok((Recu { id, ref_erp, nom, prereglage_uuid, copies, echeance, sha256 }, chemin))
}

/// Télécharge le fichier d'un envoi dans `recus/<id>/source`, vérifie son SHA-256 s'il est
/// donné, puis écrit `envoi.json` (le dossier n'est pris qu'avec lui).
fn telecharger(donnees: &Path, client: &Client, base: &str, secret: &str, r: &Recu, chemin: &str) -> Result<(), Echec> {
    let rep = dossier_recus(donnees).join(r.id.to_string());
    let plus_tard = |e: std::io::Error| Echec::PlusTard(e.to_string());
    std::fs::create_dir_all(&rep).map_err(plus_tard)?;
    client.a_son_tour();
    let signe = erp::signature_get(secret, maintenant(), &format!("{}{chemin}", chemin_de(base)));
    let mut rep_http = client.agent.get(format!("{base}{chemin}")).header("X-EKO-RIP-Signature", &signe).call().map_err(|e| Echec::PlusTard(e.to_string()))?;
    match rep_http.status().as_u16() {
        200 => {}
        404 => return Err(Echec::Refus(REFUS_TELECHARGEMENT, "404".into())),
        s => return Err(Echec::PlusTard(format!("HTTP {s}"))),
    }
    let partiel = rep.join("source.part");
    let mut f = std::fs::File::create(&partiel).map_err(plus_tard)?;
    let mut lecteur = rep_http.body_mut().with_config().limit(FICHIER_MAX).reader();
    let mut h = Sha256::new();
    let mut tampon = vec![0u8; 1 << 16];
    loop {
        let n = lecteur.read(&mut tampon).map_err(|e| Echec::PlusTard(e.to_string()))?;
        if n == 0 {
            break;
        }
        h.update(&tampon[..n]);
        f.write_all(&tampon[..n]).map_err(plus_tard)?;
    }
    f.sync_all().map_err(plus_tard)?;
    let lu = erp::hex(&h.finalize());
    if let Some(attendu) = r.sha256.as_ref().filter(|a| **a != lu) {
        return Err(Echec::Refus(REFUS_EMPREINTE, format!("SHA-256 {lu}, attendu {attendu}")));
    }
    std::fs::rename(&partiel, rep.join("source")).map_err(plus_tard)?;
    crate::etat::ecrire(&rep.join("envoi.json"), r, false).map_err(plus_tard)?;
    Ok(())
}

/// Les travaux proposés par l'ERP : chaque envoi jamais pris est téléchargé (ou refusé s'il
/// ne se lit pas). Les envois pris que l'ERP ne propose plus sont oubliés après la rétention.
fn prendre_travaux(donnees: &Path, client: &Client, base: &str, secret: &str, travaux: &Value) {
    let liste = travaux.as_array().map(Vec::as_slice).unwrap_or_default();
    let proposes: Vec<Uuid> = liste.iter().filter_map(|t| t["id"].as_str().and_then(|i| Uuid::parse_str(i).ok())).collect();
    let pris = {
        let Some(mut o) = ouvrir(donnees) else { return };
        let limite = maintenant().saturating_sub(RETENTION_S);
        let avant = o.e.envois.len();
        o.e.envois.retain(|id, le| *le >= limite || proposes.contains(id));
        let pris = o.e.envois.clone();
        if pris.len() != avant {
            o.fermer();
        }
        pris
    };
    let mut telecharges = 0;
    for t in liste {
        if telecharges == TELECHARGEMENTS_PAR_CYCLE {
            break;
        }
        let id = t["id"].as_str().and_then(|i| Uuid::parse_str(i).ok());
        if id.is_some_and(|id| pris.contains_key(&id)) {
            continue;
        }
        let (r, chemin) = match lire_envoi(t) {
            Ok(x) => x,
            Err(champ) => {
                // Sans `id` lisible, l'ERP ne peut rien rattacher : on le dit au journal seulement.
                match id {
                    Some(id) => noter_refus(donnees, id, t["ref_erp"].as_str(), REFUS_ENVOI_INVALIDE, Some(&champ)),
                    None => eprintln!("pont ERP : travail sans id lisible, ignoré"),
                }
                continue;
            }
        };
        telecharges += 1;
        match telecharger(donnees, client, base, secret, &r, &chemin) {
            Ok(()) => {
                if let Some(mut o) = ouvrir(donnees) {
                    o.e.envois.insert(r.id, maintenant());
                    o.fermer();
                }
                RECUS.notify_one();
            }
            Err(Echec::Refus(code, detail)) => {
                oublier_recu(donnees, r.id);
                noter_refus(donnees, r.id, Some(&r.ref_erp), code, Some(&detail));
            }
            Err(Echec::PlusTard(e)) => {
                eprintln!("pont ERP : envoi {} non téléchargé, repris au prochain cycle : {e}", r.id);
                break;
            }
        }
    }
}

// --- Canal HTTP(S) ---

/// Écart minimal entre deux requêtes à l'ERP (GET compris) : 60 par minute au plus, même en
/// rafale, sous la limite de 120 de l'ERP ; les événements arrivés entre-temps partent dans
/// le lot suivant (100 au plus).
pub const INTERVALLE: Duration = Duration::from_secs(1);
/// Attente après un 429 sans `Retry-After` lisible.
pub const ATTENTE_429: Duration = Duration::from_secs(60);

/// Client du pont : délai de 30 s, statuts 4xx/5xx lus comme des réponses (leur corps dit
/// le code de l'ERP), une requête par `intervalle` au plus.
#[derive(Clone)]
pub struct Client {
    agent: ureq::Agent,
    intervalle: Duration,
    derniere: Arc<Mutex<Option<Instant>>>,
}

pub fn client() -> Client {
    client_espace(INTERVALLE)
}

fn client_espace(intervalle: Duration) -> Client {
    let agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(30)))
        .user_agent(format!("eko-rip-serveur/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .into();
    Client { agent, intervalle, derniere: Default::default() }
}

impl Client {
    /// Attend que `intervalle` soit passé depuis la requête précédente.
    fn a_son_tour(&self) {
        let mut d = self.derniere.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(reste) = d.and_then(|t| self.intervalle.checked_sub(t.elapsed())) {
            std::thread::sleep(reste);
        }
        *d = Some(Instant::now());
    }
}

struct Reponse {
    statut: u16,
    corps: Value,
    /// `Retry-After` en secondes (forme date non lue).
    reessayer: Option<u64>,
}

fn lire_reponse(r: Result<ureq::http::Response<ureq::Body>, ureq::Error>) -> Result<Reponse, String> {
    let mut r = r.map_err(|e| e.to_string())?;
    let statut = r.status().as_u16();
    let reessayer = r.headers().get("retry-after").and_then(|v| v.to_str().ok()).and_then(|v| v.trim().parse().ok());
    let texte = r.body_mut().with_config().limit(1 << 20).read_to_string().unwrap_or_default();
    Ok(Reponse { statut, corps: serde_json::from_str(&texte).unwrap_or(Value::Null), reessayer })
}

/// Chemin de la base, tel qu'il part dans la ligne de requête (signé pour un GET).
fn chemin_de(base: &str) -> String {
    base.parse::<axum::http::Uri>().map(|u| u.path().trim_end_matches('/').to_string()).unwrap_or_default()
}

fn en_attente(client: &Client, base: &str, secret: &str, instance: Uuid) -> Result<Reponse, String> {
    client.a_son_tour();
    let requete = format!("/en-attente?instance={instance}");
    let signe = erp::signature_get(secret, maintenant(), &format!("{}{requete}", chemin_de(base)));
    lire_reponse(client.agent.get(format!("{base}{requete}")).header("X-EKO-RIP-Signature", &signe).call())
}

fn envoyer(client: &Client, base: &str, secret: &str, corps: &[u8]) -> Result<Reponse, String> {
    client.a_son_tour();
    let signe = erp::signature(secret, maintenant(), corps);
    lire_reponse(client.agent.post(format!("{base}/evenements")).header("Content-Type", "application/json").header("X-EKO-RIP-Signature", &signe).send(corps))
}

/// Applique un `dernier_seq_traite` de l'ERP pour notre instance.
fn accuser(e: &mut Etat, d: u64) {
    if d > e.dernier_seq {
        e.statut = Statut::Anomalie;
        e.erreur = Some(Erreur { code: "pont_anomalie_seq".into(), detail: Some(format!("{d} > {}", e.dernier_seq)), statut: None, le: maintenant() });
    } else if d == 0 && e.deja_accuse {
        e.statut = Statut::Remplace;
        e.erreur = Some(Erreur { code: "pont_remplace".into(), detail: None, statut: None, le: maintenant() });
    } else {
        e.accuse = d;
        e.deja_accuse |= d > 0;
    }
}

fn noter_erreur(donnees: &Path, code: &str, detail: Option<String>, statut: Option<u16>) {
    eprintln!("pont ERP : {code}{}", detail.as_deref().map(|d| format!(" ({d})")).unwrap_or_default());
    if let Some(mut o) = ouvrir(donnees) {
        o.e.erreur = Some(Erreur { code: code.into(), detail, statut, le: maintenant() });
        o.fermer();
    }
}

/// Erreur d'une réponse HTTP de l'ERP : son code et son détail, ou le statut seul. Un 429
/// fait attendre `Retry-After` (borné à 1 h), sinon 60 s.
fn erreur_http(donnees: &Path, r: &Reponse) -> Fin {
    let code = r.corps["code"].as_str().filter(|c| !c.is_empty() && c.len() <= 64).unwrap_or("erp_http");
    let detail = r.corps["detail"].as_str().map(|d| d.chars().take(500).collect()).or_else(|| Some(format!("HTTP {}", r.statut)));
    noter_erreur(donnees, code, detail, Some(r.statut));
    match r.statut {
        429 => Fin::Attendre(r.reessayer.map_or(ATTENTE_429, |s| Duration::from_secs(s.clamp(1, 3600)))),
        _ => Fin::Erreur,
    }
}

/// Fin d'un cycle : la boucle en déduit l'attente.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fin {
    /// Rien d'urgent (ou pont arrêté) : 30 s, ou dès qu'un événement s'ajoute.
    Repos,
    /// Une demande de l'ERP attend : 5 s.
    Presse,
    /// Erreur : 60 s, puis 5 min tant qu'elle dure.
    Erreur,
    /// 429 : attendre ce que l'ERP demande avant tout nouvel appel.
    Attendre(Duration),
}

static NON_PRIS: Mutex<String> = Mutex::new(String::new());

/// Un cycle du canal : catalogue si besoin, `GET /en-attente`, puis envoi de tout ce que
/// l'ERP n'a pas accusé. `premier` : premier cycle depuis le démarrage (catalogue envoyé).
pub fn cycle(donnees: &Path, client: &Client, premier: &mut bool) -> Fin {
    let c = erp::lire(donnees);
    let (Some(base), Some(secret), Some(instance), true) = (c.url, c.secret_hmac, c.instance, c.actif) else { return Fin::Repos };
    let cat = catalogue(donnees);
    {
        let Some(mut o) = ouvrir(donnees) else { return Fin::Repos };
        if o.e.statut != Statut::Actif {
            return Fin::Repos;
        }
        let h = empreinte(&cat);
        if *premier || o.e.catalogue.as_ref() != Some(&h) {
            if let Err(e) = o.ajouter("catalogue.modifie", cat.clone()) {
                eprintln!("pont ERP : catalogue non journalisé : {e}");
                return Fin::Erreur;
            }
            o.e.catalogue = Some(h);
        }
        *premier = false;
        if maintenant().saturating_sub(o.e.purge_le) > 3600 {
            purger(&mut o);
        }
        o.fermer();
    }

    let r = match en_attente(client, &base, &secret, instance) {
        Ok(r) if r.statut == 200 => r,
        Ok(r) => return erreur_http(donnees, &r),
        Err(e) => {
            noter_erreur(donnees, "erp_injoignable", Some(e), None);
            return Fin::Erreur;
        }
    };
    let instance_lue = r.corps["instance"].as_str();
    let Some(d) = r.corps["dernier_seq_traite"].as_u64().filter(|_| instance_lue.is_none_or(|i| i == instance.to_string())) else {
        noter_erreur(donnees, "erp_reponse_invalide", Some("en-attente".into()), Some(r.statut));
        return Fin::Erreur;
    };
    let demande = r.corps["catalogue_demande"] == Value::Bool(true);
    // Annulations, estimations : lots PT6, PT5 côté ERP, pas encore définis.
    let non_pris: Vec<String> = ["annulations", "estimations"].iter().filter_map(|k| r.corps[*k].as_array().filter(|a| !a.is_empty()).map(|a| format!("{k} : {}", a.len()))).collect();
    {
        let mut deja = NON_PRIS.lock().unwrap_or_else(|p| p.into_inner());
        if non_pris.join(", ") != *deja {
            *deja = non_pris.join(", ");
            if !deja.is_empty() {
                eprintln!("pont ERP : demandes non prises en charge (lots à venir), ignorées : {deja}");
            }
        }
    }
    {
        let Some(mut o) = ouvrir(donnees) else { return Fin::Repos };
        o.e.dernier_contact = Some(maintenant());
        accuser(&mut o.e, d);
        if o.e.statut != Statut::Actif {
            eprintln!("pont ERP : arrêt ({:?}) : {:?}", o.e.statut, o.e.erreur.as_ref().map(|e| &e.code));
            o.fermer();
            return Fin::Repos;
        }
        let catalogue_en_route = lire_journal(donnees).iter().any(|e| e.seq > o.e.accuse && e.genre == "catalogue.modifie");
        if demande
            && !catalogue_en_route
            && let Err(e) = o.ajouter("catalogue.modifie", cat.clone())
        {
            eprintln!("pont ERP : catalogue non journalisé : {e}");
        }
        o.fermer();
    }
    // Les travaux, à la seule instance accusée par l'ERP (il n'en propose qu'à elle).
    prendre_travaux(donnees, client, &base, &secret, &r.corps["travaux"]);

    for _ in 0..LOTS_PAR_CYCLE {
        let lot: Vec<Evenement> = {
            let Some(mut o) = ouvrir(donnees) else { return Fin::Repos };
            if o.e.statut != Statut::Actif {
                return Fin::Repos;
            }
            let lot: Vec<Evenement> = lire_journal(donnees).into_iter().filter(|e| e.seq > o.e.accuse).take(LOT).collect();
            if o.e.accuse < o.e.dernier_seq && lot.first().is_none_or(|e| e.seq != o.e.accuse + 1) {
                // Ce que l'ERP demande a été purgé (ou le journal a été abîmé) : on ne saute rien.
                o.e.statut = Statut::Anomalie;
                o.e.erreur = Some(Erreur { code: "pont_rejeu_impossible".into(), detail: Some(format!("{}", o.e.accuse + 1)), statut: None, le: maintenant() });
                o.fermer();
                return Fin::Repos;
            }
            lot
        };
        if lot.is_empty() {
            break;
        }
        let corps = serde_json::to_vec(&lot).unwrap_or_default();
        let r = match envoyer(client, &base, &secret, &corps) {
            Ok(r) => r,
            Err(e) => {
                noter_erreur(donnees, "erp_injoignable", Some(e), None);
                return Fin::Erreur;
            }
        };
        let d = r.corps["dernier_seq_traite"].as_u64();
        {
            let Some(mut o) = ouvrir(donnees) else { return Fin::Repos };
            o.e.dernier_envoi = Some(maintenant());
            if let Some(d) = d {
                accuser(&mut o.e, d);
            }
            let arret = o.e.statut != Statut::Actif;
            o.fermer();
            if arret {
                return Fin::Repos;
            }
        }
        match (r.statut, d) {
            // Trou chez l'ERP : on renvoie tout ce qui suit son dernier traité.
            (200, Some(_)) | (409, Some(_)) => {}
            _ => return erreur_http(donnees, &r),
        }
    }
    if let Some(mut o) = ouvrir(donnees) {
        o.e.erreur = None;
        o.fermer();
    }
    if demande { Fin::Presse } else { Fin::Repos }
}

/// Boucle du pont, pour toute la vie du serveur.
pub async fn boucle(donnees: PathBuf) {
    boucle_avec(donnees, client()).await;
}

async fn boucle_avec(donnees: PathBuf, client: Client) {
    let mut premier = true;
    let mut erreurs = 0u32;
    loop {
        let (d, c, p) = (donnees.clone(), client.clone(), premier);
        let (fin, p) = tokio::task::spawn_blocking(move || {
            let mut p = p;
            (cycle(&d, &c, &mut p), p)
        })
        .await
        .unwrap_or((Fin::Erreur, premier));
        premier = p;
        match fin {
            Fin::Erreur => {
                erreurs += 1;
                tokio::time::sleep(if erreurs == 1 { ATTENTE_PREMIERE_ERREUR } else { ATTENTE_ERREURS }).await;
            }
            Fin::Attendre(d) => tokio::time::sleep(d).await,
            _ => {
                erreurs = 0;
                let attente = if fin == Fin::Presse { PRESSE } else { CADENCE };
                tokio::select! {
                    () = tokio::time::sleep(attente) => {}
                    () = REVEIL.notified() => {}
                }
            }
        }
    }
}

/// État du pont pour l'écran Système. Jamais le secret.
pub fn etat_public(donnees: &Path) -> Value {
    let c = erp::lire(donnees);
    let e: Etat = crate::etat::lire(&fichier_etat(donnees)).ok().flatten().filter(|e: &Etat| e.instance == c.instance).unwrap_or_default();
    let statut = if !c.actif {
        "desactive"
    } else if c.url.is_none() || c.secret_hmac.is_none() {
        "incomplet"
    } else {
        match e.statut {
            Statut::Remplace => "remplace",
            Statut::Anomalie => "anomalie",
            Statut::Actif if e.erreur.is_some() => "erreur",
            Statut::Actif => "actif",
        }
    };
    json!({
        "statut": statut,
        "actif": c.actif,
        "url": c.url,
        "secret_enregistre": c.secret_hmac.is_some(),
        "instance": c.instance,
        "dernier_seq": e.dernier_seq,
        "accuse": e.accuse,
        "en_attente": e.dernier_seq.saturating_sub(e.accuse),
        "dernier_contact": e.dernier_contact,
        "dernier_envoi": e.dernier_envoi,
        "erreur": e.erreur,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Bytes;
    use axum::extract::State;
    use axum::http::{HeaderMap, StatusCode, Uri};
    use axum::routing::{get, post};
    use axum::{Json, Router};
    use std::sync::Arc;

    /// Secret factice des essais (jamais un secret réel).
    const SECRET: &str = "secret-factice-des-essais-0123456789abcdef-0123456789";

    /// Faux ERP local, aux règles du contrat côté ERP (PT1) : instance active, dernier traité,
    /// doublons, trous (409), instance inconnue avec `seq` > 1 (409, 0).
    struct Erp {
        active: Option<Option<Uuid>>,
        dernier: u64,
        recus: Vec<Evenement>,
        requetes: usize,
        posts: usize,
        catalogue_demande: bool,
        /// Au prochain POST, l'ERP « perd » ce qui suit ce numéro (trou à rejouer).
        perdre: Option<u64>,
        /// Instant de chaque requête reçue (débit).
        instants: Vec<Instant>,
        /// Taille de chaque lot reçu.
        lots: Vec<usize>,
        /// Prochaine requête refusée en 429, avec ce `Retry-After` (en secondes) s'il y en a un.
        trop: Option<Option<u64>>,
        /// `travaux` de `en-attente`, et les fichiers servis par `fichiers/{id}` (lot R3).
        travaux: Vec<Value>,
        fichiers: BTreeMap<String, Vec<u8>>,
        telechargements: usize,
        /// `fichiers/{id}` répond 503 tant que c'est vrai.
        panne: bool,
    }

    /// Compte la requête ; un 429 demandé est rendu à sa place.
    fn compter(e: &mut Erp) -> Option<axum::response::Response> {
        use axum::response::IntoResponse;
        e.requetes += 1;
        e.instants.push(Instant::now());
        let attente = e.trop.take()?;
        let corps = Json(json!({ "code": "trop_de_requetes", "detail": null }));
        Some(match attente {
            Some(s) => (StatusCode::TOO_MANY_REQUESTS, [("retry-after", s.to_string())], corps).into_response(),
            None => (StatusCode::TOO_MANY_REQUESTS, corps).into_response(),
        })
    }
    type Faux = Arc<std::sync::Mutex<Erp>>;

    /// `t` de l'en-tête si le HMAC de `"<t>." + message` est juste (comparaison de l'ERP).
    fn signature_juste(entete: &str, message: &[u8], secret: &str) -> Option<u64> {
        let (t, v1) = entete.strip_prefix("t=")?.split_once(",v1=")?;
        let t: u64 = t.parse().ok()?;
        let mut m = format!("{t}.").into_bytes();
        m.extend_from_slice(message);
        (erp::hmac_sha256(secret.as_bytes(), &m) == v1).then_some(t)
    }

    fn signe(entetes: &HeaderMap, message: &[u8]) -> bool {
        let entete = entetes.get("x-eko-rip-signature").and_then(|v| v.to_str().ok()).unwrap_or_default();
        signature_juste(entete, message, SECRET).is_some_and(|t| t.abs_diff(maintenant()) <= 300)
    }

    #[test]
    fn le_faux_erp_verifie_les_exemples_du_document() {
        // § 6 du contrat côté ERP : secret et horodatage d'exemple.
        let secret = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let corps = r#"{"contrat":"eko-rip-erp/1","seq":1,"id":"6f1c0e8e-1b8e-4b9a-9d5e-2f6f3c1a7b10","type":"catalogue.modifie","le":1793000000,"donnees":{"serveur":"0.18.0","machines":[{"uuid":"3f1c0e8e-1b8e-4b9a-9d5e-2f6f3c1a7b10","id":"epson","nom":"Epson SC-S80600","type":"rouleau","laize_mm":1626}]}}"#;
        let post = "t=1793000000,v1=8daa0d9b09c96fdf0cbcd11689e21f3e5199a35cdc50a0245c8bbf0348e50b04";
        let get = "t=1793000000,v1=1d5fcee0fd5ab7829b5cc2407d7833d2766637d3ba93ba425fdf1b9688a909b4";
        assert_eq!(signature_juste(post, corps.as_bytes(), secret), Some(1793000000));
        assert_eq!(signature_juste(get, b"GET /api/v1/gestion-imprimerie/rip/goldenprint/en-attente", secret), Some(1793000000));
        assert_eq!(signature_juste(post, b"{}", secret), None, "corps re-serialise : refuse");
        assert_eq!(signature_juste(get, b"GET /autre", secret), None, "chemin different : refuse");
    }

    fn reponse(statut: StatusCode, v: Value) -> (StatusCode, Json<Value>) {
        (statut, Json(v))
    }

    async fn evenements(State(f): State<Faux>, entetes: HeaderMap, corps: Bytes) -> axum::response::Response {
        use axum::response::IntoResponse;
        let mut e = f.lock().unwrap();
        if let Some(r) = compter(&mut e) {
            return r;
        }
        e.posts += 1;
        evenements_traites(&mut e, &entetes, &corps).into_response()
    }

    fn evenements_traites(e: &mut Erp, entetes: &HeaderMap, corps: &[u8]) -> (StatusCode, Json<Value>) {
        if !signe(entetes, corps) {
            return reponse(StatusCode::UNAUTHORIZED, json!({ "code": "signature_invalide", "detail": null }));
        }
        if let Some(n) = e.perdre.take() {
            e.dernier = n;
        }
        let liste = match serde_json::from_slice::<Value>(corps) {
            Ok(Value::Array(a)) => a,
            Ok(o @ Value::Object(_)) => vec![o],
            _ => return reponse(StatusCode::BAD_REQUEST, json!({ "code": "corps_invalide", "detail": null })),
        };
        e.lots.push(liste.len());
        for v in liste {
            let ev: Evenement = serde_json::from_value(v).unwrap();
            assert_eq!(ev.contrat, erp::CONTRAT);
            if e.active != Some(Some(ev.instance)) {
                if ev.seq != 1 {
                    return reponse(StatusCode::CONFLICT, json!({ "code": "seq_manquant", "detail": null, "instance": ev.instance, "dernier_seq_traite": 0 }));
                }
                (e.active, e.dernier) = (Some(Some(ev.instance)), 0);
            }
            if ev.seq <= e.dernier {
                continue;
            }
            if ev.seq > e.dernier + 1 {
                let (instance, d) = (e.active, e.dernier);
                return reponse(StatusCode::CONFLICT, json!({ "code": "seq_manquant", "detail": null, "instance": instance, "dernier_seq_traite": d }));
            }
            if ev.genre == "catalogue.modifie" {
                e.catalogue_demande = false;
            }
            e.dernier = ev.seq;
            e.recus.push(ev);
        }
        let (instance, d) = (e.active.flatten(), e.dernier);
        reponse(StatusCode::OK, json!({ "instance": instance, "dernier_seq_traite": d }))
    }

    async fn en_attente_erp(State(f): State<Faux>, entetes: HeaderMap, uri: Uri) -> axum::response::Response {
        use axum::response::IntoResponse;
        let mut e = f.lock().unwrap();
        if let Some(r) = compter(&mut e) {
            return r;
        }
        en_attente_reponse(&e, &entetes, &uri).into_response()
    }

    fn en_attente_reponse(e: &Erp, entetes: &HeaderMap, uri: &Uri) -> (StatusCode, Json<Value>) {
        if !signe(entetes, format!("GET {}", uri.path_and_query().unwrap()).as_bytes()) {
            return reponse(StatusCode::UNAUTHORIZED, json!({ "code": "signature_invalide", "detail": null }));
        }
        let Some(instance) = uri.query().and_then(|q| q.strip_prefix("instance=")).and_then(|i| Uuid::parse_str(i).ok()) else {
            return reponse(StatusCode::BAD_REQUEST, json!({ "code": "requete_invalide", "detail": null }));
        };
        let d = if e.active == Some(Some(instance)) { e.dernier } else { 0 };
        reponse(StatusCode::OK, json!({ "contrat": erp::CONTRAT, "instance": instance, "dernier_seq_traite": d, "catalogue_demande": e.catalogue_demande, "travaux": e.travaux, "annulations": [], "estimations": [] }))
    }

    async fn fichier_erp(State(f): State<Faux>, entetes: HeaderMap, uri: Uri, axum::extract::Path(id): axum::extract::Path<String>) -> axum::response::Response {
        use axum::response::IntoResponse;
        let mut e = f.lock().unwrap();
        e.telechargements += 1;
        if !signe(&entetes, format!("GET {}", uri.path()).as_bytes()) {
            return reponse(StatusCode::UNAUTHORIZED, json!({ "code": "signature_invalide", "detail": null })).into_response();
        }
        if e.panne {
            return reponse(StatusCode::SERVICE_UNAVAILABLE, json!({ "code": "indisponible", "detail": null })).into_response();
        }
        match e.fichiers.get(&id) {
            Some(o) => o.clone().into_response(),
            None => reponse(StatusCode::NOT_FOUND, json!({ "code": "envoi_inconnu", "detail": null })).into_response(),
        }
    }

    async fn faux_erp() -> (Faux, String) {
        let f: Faux = Arc::new(std::sync::Mutex::new(Erp { active: None, dernier: 0, recus: vec![], requetes: 0, posts: 0, catalogue_demande: true, perdre: None, instants: vec![], lots: vec![], trop: None, travaux: vec![], fichiers: BTreeMap::new(), telechargements: 0, panne: false }));
        let app = Router::new()
            .route("/rip/atelier/evenements", post(evenements))
            .route("/rip/atelier/en-attente", get(en_attente_erp))
            .route("/rip/atelier/fichiers/{id}", get(fichier_erp))
            .with_state(f.clone());
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let adresse = l.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
        (f, format!("http://{adresse}/rip/atelier"))
    }

    fn donnees(nom: &str, url: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("eko-rip-pont-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("travaux")).unwrap();
        let mut m = machines::catalogue_initial(Path::new("/dev/null"), "", None);
        machines::identifier(&mut m, &[]).unwrap();
        machines::ecrire(&d, &m).unwrap();
        erp::ecrire(&d, &erp::Config { url: Some(url.into()), secret_hmac: Some(SECRET.into()), actif: true, instance: Some(Uuid::new_v4()), ..Default::default() }).unwrap();
        d
    }

    async fn tourner(d: &Path, premier: bool) -> Fin {
        let d = d.to_path_buf();
        tokio::task::spawn_blocking(move || cycle(&d, &client_espace(Duration::ZERO), &mut { premier })).await.unwrap()
    }

    fn etat(d: &Path) -> Etat {
        crate::etat::lire(&fichier_etat(d)).unwrap().unwrap()
    }

    fn instance(d: &Path) -> Uuid {
        erp::lire(d).instance.unwrap()
    }

    fn seqs(v: &[Evenement]) -> Vec<u64> {
        v.iter().map(|e| e.seq).collect()
    }

    /// Trois `travail.etat` puis un cycle : un seul lot (catalogue compris), tout accusé.
    async fn accuse_un_lot(nom: &str) -> (Faux, PathBuf) {
        let (f, url) = faux_erp().await;
        let d = donnees(nom, &url);
        let t = Uuid::new_v4();
        for etat in ["en_attente", "en_cours", "termine"] {
            noter_travail(&d, t, etat, None, 1, neuf(), None);
        }
        noter_travail(&d, t, "termine", None, 1, neuf(), None);
        assert_eq!(etat(&d).dernier_seq, 3, "un état inchangé ne fait pas d'événement");
        assert_eq!(tourner(&d, true).await, Fin::Presse, "l'ERP demandait le catalogue : 5 s");
        {
            let e = f.lock().unwrap();
            assert_eq!(seqs(&e.recus), [1, 2, 3, 4]);
            assert_eq!(e.posts, 1, "un seul lot");
            assert_eq!(e.recus[1].donnees, json!({ "travail_uuid": t, "ref_erp": null, "etat": "en_cours", "code_erreur": null, "pages": 1 }));
            assert_eq!(e.recus[3].genre, "catalogue.modifie");
            assert!(e.recus.iter().all(|x| x.instance == instance(&d)));
        }
        let s = etat(&d);
        assert_eq!((s.accuse, s.deja_accuse, s.statut, s.erreur), (4, true, Statut::Actif, None));
        (f, d)
    }

    /// Un élément de `travaux` comme l'ERP l'envoie (§ 11 du contrat côté ERP).
    fn travail_erp(id: Uuid, ref_erp: &str, prereglage: Uuid, copies: u64) -> Value {
        json!({ "id": id, "ref_erp": ref_erp, "nom": format!("{ref_erp} — Bâche"), "prereglage_uuid": prereglage, "copies": copies, "echeance": 1793600000, "fichier": { "chemin": format!("/fichiers/{id}"), "nom": "bache.pdf", "taille": 14, "sha256": null } })
    }

    fn refus_recus(f: &Faux) -> Vec<(Value, Value)> {
        f.lock().unwrap().recus.iter().filter(|x| x.genre == "travail.refuse").map(|x| (x.donnees["envoi_id"].clone(), x.donnees["code"].clone())).collect()
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn les_travaux_proposes_sont_telecharges_une_seule_fois_ou_refuses() {
        let (f, d) = accuse_un_lot("travaux").await;
        let (bon, mauvais, absent, abime, preset) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let empreinte = erp::hex(&Sha256::digest(b"%PDF-1.4 essai"));
        {
            let mut e = f.lock().unwrap();
            e.fichiers.insert(bon.to_string(), b"%PDF-1.4 essai".to_vec());
            e.fichiers.insert(abime.to_string(), b"%PDF-1.4 tronqu".to_vec());
            let mut verifie = travail_erp(bon, "PROD-2026-00123", preset, 2);
            verifie["fichier"]["sha256"] = json!(empreinte.to_uppercase());
            let mut faux = travail_erp(abime, "PROD-2026-00126", preset, 1);
            faux["fichier"]["sha256"] = json!(empreinte);
            e.travaux = vec![verifie, travail_erp(mauvais, "PROD-2026-00124", preset, 0), travail_erp(absent, "PROD-2026-00125", preset, 1), faux];
        }
        assert_eq!(tourner(&d, false).await, Fin::Repos);
        let recus = recus(&d);
        assert_eq!(recus.len(), 1, "seul l'envoi lisible et servi est reçu");
        let (r, fichier) = &recus[0];
        assert_eq!(r, &Recu { id: bon, ref_erp: "PROD-2026-00123".into(), nom: "bache.pdf".into(), prereglage_uuid: preset, copies: 2, echeance: Some(1793600000), sha256: Some(empreinte) });
        assert_eq!(std::fs::read(fichier).unwrap(), b"%PDF-1.4 essai");
        // Les refus partent dans le même cycle, journalisés avant l'envoi des lots.
        assert_eq!(refus_recus(&f), [(json!(mauvais), json!("envoi_invalide")), (json!(absent), json!("telechargement_impossible")), (json!(abime), json!("empreinte_fausse"))]);
        assert_eq!(f.lock().unwrap().telechargements, 3, "l'envoi illisible n'est pas téléchargé");
        assert!(!dossier_recus(&d).join(abime.to_string()).exists(), "un fichier à l'empreinte fausse ne reste pas");

        // Reproposés tant que l'ERP ne les a pas tirés : jamais repris.
        tourner(&d, false).await;
        assert_eq!(f.lock().unwrap().telechargements, 3);
        assert_eq!(refus_recus(&f).len(), 3);
        // Plus proposés : gardés jusqu'à la rétention, puis oubliés.
        f.lock().unwrap().travaux.clear();
        tourner(&d, false).await;
        assert_eq!(etat(&d).envois.len(), 4);
        {
            let mut o = ouvrir(&d).unwrap();
            o.e.envois.values_mut().for_each(|le| *le -= RETENTION_S + 1);
            o.fermer();
        }
        tourner(&d, false).await;
        assert!(etat(&d).envois.is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn un_telechargement_en_panne_est_repris_au_cycle_suivant() {
        let (f, d) = accuse_un_lot("panne").await;
        let id = Uuid::new_v4();
        {
            let mut e = f.lock().unwrap();
            e.fichiers.insert(id.to_string(), b"%PDF-1.4".to_vec());
            e.travaux = vec![travail_erp(id, "PROD-2026-00123", Uuid::new_v4(), 1)];
            e.panne = true;
        }
        tourner(&d, false).await;
        assert!(recus(&d).is_empty() && etat(&d).envois.is_empty(), "ni reçu ni marqué pris");
        assert!(refus_recus(&f).is_empty(), "une panne n'est pas un refus");
        f.lock().unwrap().panne = false;
        tourner(&d, false).await;
        assert_eq!(recus(&d).len(), 1);
        assert!(etat(&d).envois.contains_key(&id));
    }

    #[test]
    fn un_travail_lie_a_l_erp_porte_l_envoi_et_la_reference() {
        let d = donnees("lien", "http://127.0.0.1:9/rip/atelier");
        let (t, envoi) = (Uuid::new_v4(), Uuid::new_v4());
        let lien = Lien { envoi, ref_erp: "PROD-2026-00123".into() };
        noter_travail(&d, t, "en_attente", None, 0, neuf(), Some(&lien));
        noter_travail(&d, Uuid::new_v4(), "en_attente", None, 0, neuf(), None);
        let j = lire_journal(&d);
        assert_eq!(j[0].donnees, json!({ "travail_uuid": t, "ref_erp": "PROD-2026-00123", "envoi_id": envoi, "etat": "en_attente", "code_erreur": null, "pages": 0 }));
        assert!(j[1].donnees.get("envoi_id").is_none() && j[1].donnees["ref_erp"].is_null(), "un travail né au RIP : ni envoi ni référence");
        assert_eq!(etat(&d).refs.get(&t).map(String::as_str), Some("PROD-2026-00123"), "recopiée dans les sorties");
        noter_refus(&d, envoi, Some("PROD-2026-00123"), "reglage_inconnu", Some(&"x".repeat(600)));
        let r = lire_journal(&d).pop().unwrap();
        assert_eq!((r.genre.as_str(), r.donnees["code"].as_str(), r.donnees["detail"].as_str().map(|x| x.len())), ("travail.refuse", Some("reglage_inconnu"), Some(500)));
    }

    #[test]
    fn un_element_de_travaux_mal_forme_est_refuse_par_son_champ() {
        let (id, p) = (Uuid::new_v4(), Uuid::new_v4());
        let bon = travail_erp(id, "PROD-2026-00123", p, 1);
        assert!(lire_envoi(&bon).is_ok());
        let avec = |cle: &str, v: Value| {
            let mut t = bon.clone();
            if let Some((a, b)) = cle.split_once('.') {
                t[a][b] = v;
            } else {
                t[cle] = v;
            }
            lire_envoi(&t).map(|_| ()).unwrap_err()
        };
        assert_eq!(avec("ref_erp", json!("  ")), "ref_erp");
        assert_eq!(avec("prereglage_uuid", json!("bache")), "prereglage_uuid");
        assert_eq!(avec("copies", json!(100_001)), "copies");
        assert_eq!(avec("fichier.sha256", json!("abc")), "fichier.sha256");
        for chemin in ["fichiers/x", "/fichiers/../secret", "//autre.hote/x", "/fichiers/x#y"] {
            assert_eq!(avec("fichier.chemin", json!(chemin)), "fichier.chemin", "{chemin}");
        }
        // Nom de fichier : pour l'affichage, sans séparateur ni caractère de contrôle.
        let mut t = bon.clone();
        t["fichier"]["nom"] = json!("../a/b\\c\u{7}.pdf");
        assert_eq!(lire_envoi(&t).unwrap().0.nom, "..abc.pdf");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn lot_envoye_puis_accuse_puis_trou_rejoue() {
        let (f, d) = accuse_un_lot("lot").await;
        // Rien de nouveau : un GET, aucun POST.
        assert_eq!(tourner(&d, false).await, Fin::Repos);
        assert_eq!(f.lock().unwrap().posts, 1);
        assert_eq!(etat_public(&d)["statut"], "actif");
        // L'ERP perd ce qui suit 2 : 409 seq_manquant, le RIP renvoie 3, 4, 5.
        f.lock().unwrap().perdre = Some(2);
        noter_travail(&d, Uuid::new_v4(), "en_attente", None, 0, neuf(), None);
        assert_eq!(tourner(&d, false).await, Fin::Repos);
        let e = f.lock().unwrap();
        assert_eq!(seqs(&e.recus), [1, 2, 3, 4, 3, 4, 5]);
        assert_eq!(e.posts, 3);
        assert_eq!((e.dernier, etat(&d).accuse), (5, 5));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn instance_neuve_et_0_rejoue_depuis_1() {
        let (f, url) = faux_erp().await;
        let d = donnees("neuve", &url);
        // Une autre installation est active chez l'ERP, à 50.
        {
            let mut e = f.lock().unwrap();
            (e.active, e.dernier) = (Some(Some(Uuid::new_v4())), 50);
        }
        noter_travail(&d, Uuid::new_v4(), "en_attente", None, 0, neuf(), None);
        noter_travail(&d, Uuid::new_v4(), "en_attente", None, 0, neuf(), None);
        assert_eq!(tourner(&d, true).await, Fin::Presse, "l'ERP demandait le catalogue : 5 s");
        let e = f.lock().unwrap();
        assert_eq!(e.active, Some(Some(instance(&d))), "la nouvelle instance prend la place");
        assert_eq!(seqs(&e.recus), [1, 2, 3]);
        assert_eq!(etat(&d).accuse, 3);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn instance_accusee_et_0_remplace_puis_nouvelle_instance() {
        let (f, d) = accuse_un_lot("remplace").await;
        // Une autre installation a pris la place.
        {
            let mut e = f.lock().unwrap();
            (e.active, e.dernier) = (Some(Some(Uuid::new_v4())), 1);
        }
        assert_eq!(tourner(&d, false).await, Fin::Repos);
        assert_eq!(etat(&d).statut, Statut::Remplace);
        assert_eq!(etat_public(&d)["statut"], "remplace");
        assert_eq!(etat_public(&d)["erreur"]["code"], "pont_remplace");
        let avant = f.lock().unwrap().requetes;
        noter_travail(&d, Uuid::new_v4(), "en_attente", None, 0, neuf(), None);
        assert_eq!(tourner(&d, false).await, Fin::Repos);
        assert_eq!(f.lock().unwrap().requetes, avant, "remplacé : plus rien n'est envoyé");
        assert_eq!(f.lock().unwrap().posts, 1);
        // L'administrateur tire une nouvelle instance : journal neuf, rejoué depuis 1.
        let mut c = erp::lire(&d);
        c.instance = Some(Uuid::new_v4());
        erp::ecrire(&d, &c).unwrap();
        assert_eq!(etat_public(&d)["statut"], "actif");
        assert_eq!(tourner(&d, true).await, Fin::Repos);
        let e = f.lock().unwrap();
        assert_eq!(e.active, Some(Some(instance(&d))));
        assert_eq!(e.recus.last().map(|x| (x.seq, x.genre.as_str())), Some((1, "catalogue.modifie")));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn anomalie_arrete_tout() {
        let (f, d) = accuse_un_lot("anomalie").await;
        f.lock().unwrap().dernier = 100;
        noter_travail(&d, Uuid::new_v4(), "en_attente", None, 0, neuf(), None);
        assert_eq!(tourner(&d, false).await, Fin::Repos);
        let s = etat(&d);
        assert_eq!((s.statut, s.dernier_seq, s.accuse), (Statut::Anomalie, 5, 4), "rien n'est renuméroté");
        assert_eq!(s.erreur.unwrap().code, "pont_anomalie_seq");
        let requetes = f.lock().unwrap().requetes;
        assert_eq!(tourner(&d, false).await, Fin::Repos);
        let e = f.lock().unwrap();
        assert_eq!((e.posts, e.requetes), (1, requetes));
        drop(e);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Date de création d'un travail créé après l'ouverture du journal (même à la seconde près).
    fn neuf() -> u64 {
        maintenant() + 60
    }

    fn uuid_valide(v: &Value) -> bool {
        v.as_str().is_some_and(|u| Uuid::parse_str(u).is_ok())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn catalogue_demande_puis_machine_modifiee() {
        let (f, d) = accuse_un_lot("catalogue").await;
        // Un support, son mode, un réglage enregistré qui s'en sert.
        let mut c = crate::supports::Catalogue { version: crate::supports::VERSION, supports: vec![crate::supports::Support { id: None, machine: "epson".into(), nom: "Bâche 510 g".into(), encres: vec!["C".into(), "M".into()], laize_mm: Some(1600.0), numero_machine: Some(7), modes: vec![crate::supports::mode_par_defaut()] }] };
        crate::supports::attribuer_ids(&mut c.supports, &[]);
        crate::supports::ecrire(&d, &c).unwrap();
        let demande = crate::reglages::Demande { support_uuid: c.supports[0].id, mode_uuid: c.supports[0].modes[0].id, decoupeuse: Some("kona".into()), ..serde_json::from_value(json!({})).unwrap() };
        let mut r = vec![crate::reglages::Reglage { uuid: None, nom: "Bache 720".into(), demande }];
        crate::reglages::identifier(&mut r, &[]);
        crate::reglages::ecrire(&d, &r).unwrap();
        // Le catalogue a changé : il part au cycle suivant ; puis l'ERP le redemande.
        assert_eq!(tourner(&d, false).await, Fin::Repos);
        let n = f.lock().unwrap().recus.len();
        assert_eq!(tourner(&d, false).await, Fin::Repos);
        assert_eq!(f.lock().unwrap().recus.len(), n, "catalogue inchangé : rien");
        f.lock().unwrap().catalogue_demande = true;
        assert_eq!(tourner(&d, false).await, Fin::Presse, "une demande attendait : 5 s");
        let cat = {
            let e = f.lock().unwrap();
            assert_eq!(e.recus.len(), n + 1);
            assert!(!e.catalogue_demande);
            e.recus.last().unwrap().clone()
        };
        assert_eq!(cat.genre, "catalogue.modifie");
        let v = &cat.donnees;
        for genre in ["machines", "supports", "modes", "prereglages", "decoupeuses", "couches_decoupe"] {
            let liste = v[genre].as_array().unwrap();
            assert!(!liste.is_empty(), "{genre} vide");
            assert!(liste.iter().all(|o| uuid_valide(&o["uuid"]) && o["nom"].is_string()), "{genre} : uuid ou nom manquant");
        }
        assert_eq!(v["modes"][0]["support_uuid"], json!(c.supports[0].id));
        assert_eq!(v["prereglages"][0]["decoupeuse_uuid"], v["decoupeuses"][0]["uuid"]);
        assert!(v["profils"].is_array());
        assert!(v["prereglages"][0].get("profil").is_none(), "inconnu : omis, jamais null");
        let texte = v.to_string();
        assert!(!texte.contains("prix") && !texte.contains("cout") && !texte.contains("tarif"), "aucun montant");
        // Machine renommée : nouveau catalogue, même uuid.
        let mut m = machines::lire(&d).unwrap();
        m[0].nom = "Epson de l'atelier".into();
        machines::ecrire(&d, &m).unwrap();
        assert_eq!(tourner(&d, false).await, Fin::Repos);
        let e = f.lock().unwrap();
        let dernier = e.recus.last().unwrap();
        assert_eq!(dernier.genre, "catalogue.modifie");
        assert_eq!(dernier.donnees["machines"][0]["nom"], "Epson de l'atelier");
        assert_eq!(dernier.donnees["machines"][0]["uuid"], v["machines"][0]["uuid"]);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn sortie_etat_puis_consommation_sans_rien_inventer() {
        let (_f, url) = faux_erp().await;
        let d = donnees("sorties", &url);
        let travail = Uuid::new_v4();
        let ancienne = Sortie { cree_le: 1, ..Sortie::nouvelle("epson", machines::Operation::Impression, Uuid::new_v4(), "vieux", 1, 1) };
        let mut f = file::File { sorties: vec![ancienne, Sortie::nouvelle("epson", machines::Operation::Impression, travail, "affiche", 1, neuf())] };
        f.ecrire(&d).unwrap();
        f.ecrire(&d).unwrap();
        let journal = lire_journal(&d);
        assert_eq!(journal.len(), 1, "une sortie d'avant l'ouverture du journal ne fait rien ; une écriture sans changement non plus");
        let epson = machines::lire(&d).unwrap()[0].uuid;
        let s = &journal[0].donnees;
        assert_eq!((journal[0].genre.as_str(), s["etat"].as_str(), s["operation"].as_str()), ("sortie.etat", Some("en_attente"), Some("impression")));
        assert_eq!((s["sortie_uuid"].clone(), s["travail_uuid"].clone(), s["machine_uuid"].clone(), s["ref_erp"].clone()), (json!(f.sorties[1].id), json!(travail), json!(epson), Value::Null));
        assert_eq!(s["raison_attente"], "source_absente");
        (f.sorties[1].etat, f.sorties[1].debut) = (file::Etat::EnCours, Some(1_000));
        f.ecrire(&d).unwrap();
        (f.sorties[1].etat, f.sorties[1].fin) = (file::Etat::Terminee, Some(1_060));
        f.ecrire(&d).unwrap();
        let journal = lire_journal(&d);
        let genres: Vec<&str> = journal.iter().map(|e| e.genre.as_str()).collect();
        assert_eq!(genres, ["sortie.etat", "sortie.etat", "sortie.etat", "travail.consommation"]);
        assert_eq!(journal[1].donnees["etat"], "en_cours");
        assert_eq!(journal[2].donnees["raison_attente"], Value::Null);
        let c = &journal[3].donnees;
        assert_eq!((c["machine_uuid"].clone(), c["duree_s"].clone(), c["copies"].clone()), (json!(epson), json!(60), json!(1)));
        // Sans relevé de la machine ni journal d'impression : rien d'inventé.
        for champ in ["encre_ml", "encre_ml_estimee", "encre_source", "longueur_mm", "longueur_source", "gache_mm"] {
            assert_eq!(c[champ], Value::Null, "{champ}");
        }
        // Sortie retirée de la file : oubliée, sans événement.
        f.sorties.remove(1);
        f.ecrire(&d).unwrap();
        assert_eq!(lire_journal(&d).len(), 4);
        assert!(etat(&d).sorties.len() == 1);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn coupure_et_erp_injoignable_rien_perdu_ni_renumerote() {
        // ERP injoignable : port fermé.
        let ferme = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap();
        let d = donnees("coupure", &format!("http://{ferme}/rip/atelier"));
        for _ in 0..3 {
            noter_travail(&d, Uuid::new_v4(), "en_attente", None, 0, neuf(), None);
        }
        assert_eq!(tourner(&d, true).await, Fin::Erreur);
        assert_eq!(etat(&d).erreur.unwrap().code, "erp_injoignable");
        assert_eq!(etat_public(&d)["statut"], "erreur");
        let avant = lire_journal(&d);
        assert_eq!(seqs(&avant), [1, 2, 3, 4]);
        // Coupure : dernière ligne à moitié écrite, état en retard d'un événement.
        let mut j = std::fs::OpenOptions::new().append(true).open(fichier_journal(&d)).unwrap();
        j.write_all(br#"{"contrat":"eko-rip-erp/1","instance":"#).unwrap();
        let mut s = etat(&d);
        s.dernier_seq = 3;
        crate::etat::ecrire(&fichier_etat(&d), &s, false).unwrap();
        // Relance du serveur.
        demarrer(&d);
        assert_eq!(etat(&d).dernier_seq, 4, "le journal fait foi");
        assert_eq!(lire_journal(&d), avant, "ligne coupée retirée, rien d'autre");
        noter_travail(&d, Uuid::new_v4(), "en_attente", None, 0, neuf(), None);
        // L'ERP revient.
        let (f, url) = faux_erp().await;
        let mut c = erp::lire(&d);
        c.url = Some(url);
        erp::ecrire(&d, &c).unwrap();
        assert_eq!(tourner(&d, true).await, Fin::Presse, "l'ERP demandait le catalogue : 5 s");
        let e = f.lock().unwrap();
        assert_eq!(seqs(&e.recus), [1, 2, 3, 4, 5, 6], "dans l'ordre, sans trou ni doublon");
        assert_eq!(e.recus[..4], avant[..], "mêmes événements (id compris) qu'avant la coupure");
        assert_eq!(etat(&d).erreur, None);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// 500 événements en rafale, boucle réelle et client de production (une requête par
    /// seconde) : au plus 60 requêtes par minute, lots de 100 au plus, tout accusé dans l'ordre.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn rafale_de_500_sous_60_requetes_par_minute() {
        let (f, url) = faux_erp().await;
        let d = donnees("rafale", &url);
        let boucle = tokio::spawn(boucle_avec(d.clone(), client()));
        let debut = Instant::now();
        let dd = d.clone();
        // Dix salves de 50, verrou rendu entre deux : la boucle envoie pendant la rafale. (Un
        // `noter_travail` par événement réécrirait aussi l'état à chaque fois : trop de
        // synchronisations disque pour les autres essais qui tournent en même temps.)
        tokio::task::spawn_blocking(move || {
            for _ in 0..10 {
                let mut o = ouvrir(&dd).unwrap();
                for _ in 0..50 {
                    o.ajouter("travail.etat", json!({ "travail_uuid": Uuid::new_v4(), "ref_erp": null, "etat": "en_attente", "code_erreur": null, "pages": 0 })).unwrap();
                }
                o.fermer();
                std::thread::sleep(Duration::from_millis(300));
            }
        })
        .await
        .unwrap();
        let total = etat(&d).dernier_seq;
        assert!(total >= 500);
        while f.lock().unwrap().dernier < total {
            assert!(debut.elapsed() < Duration::from_secs(120), "tout doit être accusé");
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        // Pont coupé avant d'arrêter la boucle : le cycle en cours finit sans rien relancer.
        let mut c = erp::lire(&d);
        c.actif = false;
        erp::ecrire(&d, &c).unwrap();
        tokio::time::sleep(Duration::from_secs(2)).await;
        boucle.abort();
        let e = f.lock().unwrap();
        assert_eq!(seqs(&e.recus), (1..=total).collect::<Vec<_>>(), "dans l'ordre, sans trou ni doublon");
        assert!(e.lots.iter().all(|&n| n <= LOT), "{:?}", e.lots);
        // Instants pris à la réception : une requête retardée en route (machine chargée) rapproche
        // la suivante. D'où 250 ms de marge par écart, et sur l'étendue entière (qui ne dépend
        // que du retard de la première et de la dernière) ; une rafale sans espacement donne
        // des écarts de quelques ms.
        let ecarts: Vec<Duration> = e.instants.windows(2).map(|w| w[1] - w[0]).collect();
        let marge = Duration::from_millis(250);
        assert!(ecarts.iter().all(|&x| x + marge >= INTERVALLE), "une requête par seconde au plus : {ecarts:?}");
        assert!(ecarts.iter().sum::<Duration>() + marge >= INTERVALLE * ecarts.len() as u32, "{ecarts:?}");
        for (i, t) in e.instants.iter().enumerate() {
            assert!(e.instants[i..].iter().take_while(|&&u| u - *t < Duration::from_secs(60)).count() <= 60);
        }
        eprintln!("rafale : {total} événements, {} requêtes ({} POST, lots {:?}) en {:.1} s", e.requetes, e.posts, e.lots, debut.elapsed().as_secs_f64());
        drop(e);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Roadmap 7.9 : relevé SNMP → `machine.etat` avec les mêmes valeurs ; même état une
    /// minute plus tard → rien ; une alerte de plus → nouvel événement.
    #[test]
    fn releve_machine_publie_une_fois_par_changement() {
        let d = donnees("machine-etat", "http://127.0.0.1:9/rip/atelier");
        let support = Uuid::new_v4();
        crate::supports::ecrire_charge(&d, "epson", crate::supports::Charge { support_uuid: Some(support), laize_mm: Some(1620.0), source: "machine".into(), le: 1 }).unwrap();
        let r = crate::releves::Releve { le: 100, encres: vec![("BK".into(), 80), ("C".into(), 41)], longueur_m: Some(37.5) };
        noter_machine(&d, "epson", &r, &[(40, 45)], &[]);
        noter_machine(&d, "epson", &crate::releves::Releve { le: 160, ..r.clone() }, &[(40, 45)], &[]);
        noter_machine(&d, "epson", &crate::releves::Releve { le: 220, ..r.clone() }, &[(40, 45)], &["printer cover open".into()]);
        let j: Vec<Evenement> = lire_journal(&d).into_iter().filter(|e| e.genre == "machine.etat").collect();
        assert_eq!(j.len(), 2, "l'heure seule ne publie rien");
        let x = &j[0].donnees;
        assert_eq!(x["machine"], "epson");
        assert_eq!(x["machine_uuid"], json!(machines::lire(&d).unwrap().iter().find(|m| m.id == "epson").unwrap().uuid));
        assert_eq!(x["encres"], json!([{ "encre": "BK", "pourcent": 80 }, { "encre": "C", "pourcent": 41 }]));
        assert_eq!((x["longueur_restante_m"].clone(), x["laize_mm"].clone(), x["support_uuid"].clone()), (json!(37.5), json!(1620.0), json!(support)));
        assert_eq!(x["chauffages"], json!([{ "ecran_c": 40, "autre_c": 45 }]));
        assert_eq!((x["releve_le"].clone(), x["bac_encre_usagee_pourcent"].clone()), (json!(100), Value::Null));
        assert_eq!(j[1].donnees["alertes"], json!(["printer cover open"]));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn trop_de_requetes_attend_retry_after() {
        let (f, d) = accuse_un_lot("429").await;
        f.lock().unwrap().trop = Some(Some(7));
        assert_eq!(tourner(&d, false).await, Fin::Attendre(Duration::from_secs(7)));
        assert_eq!(etat(&d).erreur.unwrap().statut, Some(429));
        f.lock().unwrap().trop = Some(None);
        assert_eq!(tourner(&d, false).await, Fin::Attendre(ATTENTE_429), "sans Retry-After : 60 s");
        noter_travail(&d, Uuid::new_v4(), "en_attente", None, 0, neuf(), None);
        assert_eq!(tourner(&d, false).await, Fin::Repos);
        assert_eq!(f.lock().unwrap().dernier, 5, "puis tout part");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn purge_garde_ce_qui_n_est_pas_accuse() {
        let d = donnees("purge", "https://erp.exemple.test/rip");
        for _ in 0..3 {
            noter_travail(&d, Uuid::new_v4(), "en_attente", None, 0, neuf(), None);
        }
        // Événements vieux de 40 jours, accusés jusqu'à 2.
        let vieux: String = lire_journal(&d).into_iter().map(|e| serde_json::to_string(&Evenement { le: maintenant() - 40 * 86_400, ..e }).unwrap() + "\n").collect();
        std::fs::write(fichier_journal(&d), vieux).unwrap();
        let mut o = ouvrir(&d).unwrap();
        o.e.accuse = 2;
        purger(&mut o);
        o.fermer();
        assert_eq!(seqs(&lire_journal(&d)), [3]);
        // Le pont désactivé ne journalise rien.
        let mut c = erp::lire(&d);
        c.actif = false;
        erp::ecrire(&d, &c).unwrap();
        noter_travail(&d, Uuid::new_v4(), "en_attente", None, 0, neuf(), None);
        assert_eq!(seqs(&lire_journal(&d)), [3]);
        assert_eq!(etat_public(&d)["statut"], "desactive");
        let _ = std::fs::remove_dir_all(&d);
    }
}
