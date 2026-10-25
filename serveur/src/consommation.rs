//! Consommation par impression (roadmap phase 6).
//!
//! - **Réel** : relevés SNMP de la machine (`releves.rs`) entre le début et la fin de la
//!   sortie, un relevé étant forcé à chaque bout (6.1). L'encre n'est connue qu'au % près
//!   (1 % d'une poche de 700 ml = 7 ml) : une baisse est répartie entre toutes les
//!   impressions faites depuis la variation précédente du même canal, au prorata de leur
//!   encre estimée (6.2) ; une impression qui ne voit aucune baisse affiche « < 7 ml ».
//!   Le support avancé donne la longueur réelle, et la gâche (amorce, marges) par
//!   différence avec la longueur de la page (6.3).
//! - **Estimé** : points posés par le pilote direct, comptés au tramage, × volume de goutte
//!   du mode (6.9), × facteur de correction validé (6.6). Hors pilote direct (plaques en
//!   dossier, ton continu), l'encre n'est pas calculable et le dit (6.7).
//! - **Journal** des impressions (`journal-impressions.jsonl`), purgé au-delà de la
//!   rétention (6.4), et **rapport CSV** (6.5).
//!
//! Réglages (rétention, facteurs) dans `consommation.json`.

use crate::couverture::{Encre, Tarifs};
use crate::releves::{self, Releve};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Volume d'une goutte petite, moyenne, grosse (pL), quand le mode n'en donne pas.
/// Valeurs de départ choisies par nous, proportionnelles aux poids du tramage
/// (0,33 / 0,66 / 1) : à étalonner, par le facteur de correction ou dans le mode.
pub const GOUTTES_PL: [f64; 3] = [5.0, 10.0, 15.0];

/// Contenance d'une poche quand les tarifs n'en donnent pas (ml).
const CONTENANCE_DEFAUT: f64 = 700.0;

/// Ce qui a été relevé sur la machine entre le début et la fin d'une sortie (6.1).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Reel {
    /// Bornes de la fenêtre (secondes Unix) : relevés forcés au début et à la fin.
    pub du: u64,
    pub au: u64,
    /// Points de % perdus par encre (noms de l'imprimante : BK, C, LC…).
    pub encres_pc: BTreeMap<String, u32>,
    /// Ce que vaut 1 % de chaque encre (ml) : la résolution de la mesure.
    pub seuil_ml: BTreeMap<String, f64>,
    /// Support avancé (mm), si l'imprimante donne la longueur restante.
    #[serde(default)]
    pub longueur_mm: Option<f64>,
}

impl Reel {
    /// Fenêtre `du..=au` des relevés ; `None` sans au moins deux relevés.
    pub fn mesurer(releves: &[Releve], du: u64, au: u64, contenance: impl Fn(&str) -> f64) -> Option<Reel> {
        let c = releves::consommation(releves, du, au);
        (c.releves >= 2).then(|| Reel {
            du,
            au,
            seuil_ml: c.encres.keys().map(|n| (n.clone(), contenance(n) / 100.0)).collect(),
            encres_pc: c.encres,
            longueur_mm: c.longueur.then(|| (c.metres * 1000.0).round()),
        })
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Methode {
    /// Points posés par le pilote direct × volume de goutte (6.9).
    Points,
    /// Ton continu (plaques en dossier) : le tramage n'est pas le nôtre (6.7).
    #[default]
    NonCalculable,
}

/// Encre estimée d'une impression, figée à l'impression.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Estime {
    pub methode: Methode,
    /// ml par encre, avant facteur de correction.
    #[serde(default)]
    pub encres_ml: BTreeMap<String, f64>,
    /// Facteur de correction appliqué à chaque encre (absent : 1).
    #[serde(default)]
    pub facteurs: BTreeMap<String, f64>,
    /// Coût de l'encre corrigée ; `None` : non calculable.
    #[serde(default)]
    pub cout_encres: Option<f64>,
    #[serde(default)]
    pub cout_support: f64,
    #[serde(default)]
    pub ttc: bool,
}

impl Estime {
    /// ml par encre, facteur de correction appliqué.
    pub fn corrige(&self) -> BTreeMap<String, f64> {
        self.encres_ml.iter().map(|(n, &ml)| (n.clone(), ml * self.facteurs.get(n).copied().unwrap_or(1.0))).collect()
    }
}

/// ml d'encre de `points` (petits, moyens, gros) à `pl` picolitres la goutte.
pub fn ml_des_points(points: [u64; 3], pl: [f64; 3]) -> f64 {
    points.iter().zip(pl).map(|(&n, v)| n as f64 * v).sum::<f64>() * 1e-9
}

/// Tarif d'une encre de l'imprimante : par son nom (`LC`), sinon par la plaque qui la
/// porte (`Cyan` pour `C`…), sinon la ligne « Toutes les autres ».
fn tarif<'a>(t: &'a Tarifs, encre: &str) -> &'a Encre {
    let plaque = match encre {
        "BK" => "Black",
        "C" => "Cyan",
        "M" => "Magenta",
        "Y" => "Yellow",
        "WH" => "White",
        "MS" => "Silver",
        _ => "",
    };
    t.encres.get(encre).or_else(|| t.encres.get(plaque)).unwrap_or(&t.encre_defaut)
}

/// Contenance de la poche d'une encre (ml) : celle de sa cartouche dans les tarifs.
pub fn contenance_ml(t: &Tarifs, encre: &str) -> f64 {
    tarif(t, encre).cartouche.filter(|c| c.contenance_ml > 0.0).map_or(CONTENANCE_DEFAUT, |c| c.contenance_ml)
}

/// Estimation par comptage des points (6.9) : `points` par encre, `pl` du mode,
/// `facteur` validé par encre (6.6).
pub fn estimer_points(points: &[(String, [u64; 3])], pl: [f64; 3], facteur: impl Fn(&str) -> f64, t: &Tarifs, cout_support: f64) -> Estime {
    let encres_ml: BTreeMap<String, f64> = points.iter().map(|(n, p)| (n.clone(), ml_des_points(*p, pl))).filter(|(_, ml)| *ml > 0.0).collect();
    let facteurs: BTreeMap<String, f64> = encres_ml.keys().map(|n| (n.clone(), facteur(n))).filter(|(_, f)| *f != 1.0).collect();
    let mut e = Estime { methode: Methode::Points, encres_ml, facteurs, cout_encres: None, cout_support, ttc: t.affichage_ttc };
    e.cout_encres = Some(e.corrige().iter().map(|(n, ml)| ml / 1000.0 * tarif(t, n).prix_litre() * t.conversion()).sum());
    e
}

/// Une ligne du journal des impressions (6.4).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Impression {
    pub sortie: Uuid,
    pub travail: Uuid,
    pub nom: String,
    pub page: usize,
    pub machine: String,
    pub debut: u64,
    pub fin: u64,
    #[serde(default)]
    pub support: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    pub dpi: u32,
    #[serde(default)]
    pub operateur: String,
    pub surface_m2: f64,
    /// Longueur de la page imprimée (mm), sans amorce ni marges de la machine.
    pub longueur_prevue_mm: f64,
    #[serde(default)]
    pub reel: Option<Reel>,
    #[serde(default)]
    pub estime: Estime,
}

fn fichier_journal(donnees: &Path) -> PathBuf {
    donnees.join("journal-impressions.jsonl")
}

pub fn noter(donnees: &Path, i: &Impression) -> std::io::Result<()> {
    let mut o = std::fs::OpenOptions::new().create(true).append(true).open(fichier_journal(donnees))?;
    o.write_all(format!("{}\n", serde_json::to_string(i)?).as_bytes())
}

/// Journal dans l'ordre des fins d'impression ; les lignes illisibles sont ignorées.
pub fn journal(donnees: &Path) -> Vec<Impression> {
    let mut v: Vec<Impression> = std::fs::read_to_string(fichier_journal(donnees)).unwrap_or_default().lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
    v.sort_by_key(|i| i.fin);
    v
}

/// Retire du journal les impressions finies avant `avant` ; rend leur nombre.
pub fn purger(donnees: &Path, avant: u64) -> std::io::Result<usize> {
    let tout = journal(donnees);
    let garde: Vec<&Impression> = tout.iter().filter(|i| i.fin >= avant).collect();
    let retirees = tout.len() - garde.len();
    if retirees > 0 {
        let texte: String = garde.iter().filter_map(|i| serde_json::to_string(i).ok()).map(|l| l + "\n").collect();
        let temp = fichier_journal(donnees).with_extension("tmp");
        std::fs::write(&temp, texte)?;
        std::fs::rename(temp, fichier_journal(donnees))?;
    }
    Ok(retirees)
}

/// Facteur de correction d'une encre, pour une machine à une résolution (6.6).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Facteur {
    pub machine: String,
    pub dpi: u32,
    pub encre: String,
    pub valeur: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Reglages {
    /// Jours gardés dans le journal des impressions.
    #[serde(default = "retention_defaut")]
    pub retention_jours: u32,
    /// Facteurs validés ; une combinaison absente vaut 1.
    #[serde(default)]
    pub facteurs: Vec<Facteur>,
}

fn retention_defaut() -> u32 {
    90
}

impl Default for Reglages {
    fn default() -> Self {
        Reglages { retention_jours: retention_defaut(), facteurs: vec![] }
    }
}

impl Reglages {
    pub fn verifier(&self) -> Result<(), &'static str> {
        let bons = self.facteurs.iter().enumerate().all(|(i, f)| {
            (1..=30).contains(&f.machine.len())
                && (1..=8).contains(&f.encre.len())
                && (36..=2880).contains(&f.dpi)
                && f.valeur.is_finite()
                && f.valeur > 0.0
                && f.valeur <= 10.0
                && !self.facteurs[..i].iter().any(|x| (&x.machine, x.dpi, &x.encre) == (&f.machine, f.dpi, &f.encre))
        });
        if bons && (1..=3650).contains(&self.retention_jours) { Ok(()) } else { Err("consommation_invalide") }
    }

    pub fn facteur(&self, machine: &str, dpi: u32, encre: &str) -> f64 {
        self.facteurs.iter().find(|f| f.machine == machine && f.dpi == dpi && f.encre == encre).map_or(1.0, |f| f.valeur)
    }
}

pub fn fichier_reglages(donnees: &Path) -> PathBuf {
    donnees.join("consommation.json")
}

pub fn lire_reglages(donnees: &Path) -> Reglages {
    crate::etat::lire(&fichier_reglages(donnees)).ok().flatten().unwrap_or_default()
}

pub fn ecrire_reglages(donnees: &Path, r: &Reglages) -> std::io::Result<()> {
    crate::etat::ecrire(&fichier_reglages(donnees), r, false)
}

/// Encre réelle de chaque fenêtre `(du, au, estimé par encre)` (6.2). Une baisse vue au
/// relevé `t` s'est faite depuis la variation précédente du même canal (`t0`) : elle est
/// partagée entre les fenêtres qui recoupent `]t0, t]`, au prorata de leur encre estimée,
/// ou de leur durée si aucune n'a d'estimation. Une baisse hors de toute fenêtre (purge,
/// nettoyage) n'est attribuée à personne. Une hausse est un changement de poche.
pub fn repartir(releves: &[Releve], fenetres: &[(u64, u64, &BTreeMap<String, f64>)], contenance: impl Fn(&str) -> f64) -> Vec<BTreeMap<String, f64>> {
    let mut reel = vec![BTreeMap::<String, f64>::new(); fenetres.len()];
    let mut depuis: BTreeMap<&str, u64> = BTreeMap::new();
    for w in releves.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        for (nom, pc) in &b.encres {
            let Some((_, avant)) = a.encres.iter().find(|(n, _)| n == nom) else { continue };
            let t0 = *depuis.entry(nom.as_str()).or_insert(releves[0].le);
            if pc == avant {
                continue;
            }
            depuis.insert(nom.as_str(), b.le);
            if pc > avant {
                continue;
            }
            let ml = (avant - pc) as f64 * contenance(nom) / 100.0;
            let concernees: Vec<usize> = (0..fenetres.len()).filter(|&i| fenetres[i].1 > t0 && fenetres[i].0 < b.le).collect();
            let estime = |i: usize| fenetres[i].2.get(nom).copied().unwrap_or(0.0);
            let duree = |i: usize| (fenetres[i].1.min(b.le) - fenetres[i].0.max(t0)).max(1) as f64;
            let par_estime = concernees.iter().map(|&i| estime(i)).sum::<f64>() > 0.0;
            let poids = |i: usize| if par_estime { estime(i) } else { duree(i) };
            let total: f64 = concernees.iter().map(|&i| poids(i)).sum();
            for &i in &concernees {
                *reel[i].entry(nom.clone()).or_default() += ml * poids(i) / total;
            }
        }
    }
    reel
}

/// Une impression du journal avec son encre corrigée, son encre réelle répartie et ses
/// écarts, pour l'écran et le rapport.
#[derive(Clone, Debug, Serialize)]
pub struct Ligne {
    #[serde(flatten)]
    pub impression: Impression,
    /// Encre estimée, facteur appliqué (ml par encre) ; vide si non calculable.
    pub estime_ml: BTreeMap<String, f64>,
    pub estime_total_ml: Option<f64>,
    /// Encre réelle répartie (ml par encre) ; absente sans relevé.
    pub reel_ml: Option<BTreeMap<String, f64>>,
    pub reel_total_ml: Option<f64>,
    /// Rien de mesurable : plus petite quantité que la mesure verrait (1 % d'une poche, ml),
    /// affichée « < seuil » plutôt qu'un 0.
    pub sous_seuil_ml: Option<f64>,
    pub longueur_reelle_mm: Option<f64>,
    /// Longueur réelle moins longueur de la page : amorce, marges.
    pub gache_mm: Option<f64>,
    /// (réel − estimé) / estimé, en %.
    pub ecart_pct: Option<f64>,
}

fn ecart(estime: Option<f64>, reel: Option<f64>) -> Option<f64> {
    match (estime, reel) {
        (Some(e), Some(r)) if e > 0.0 => Some((r - e) / e * 100.0),
        _ => None,
    }
}

/// Lignes de tout le journal ; `releves(machine)` lit les relevés d'une machine.
pub fn lignes(journal: Vec<Impression>, releves: impl Fn(&str) -> Vec<Releve>, contenance: impl Fn(&str) -> f64) -> Vec<Ligne> {
    let estimes: Vec<BTreeMap<String, f64>> = journal.iter().map(|i| i.estime.corrige()).collect();
    let mut reparti: Vec<Option<BTreeMap<String, f64>>> = vec![None; journal.len()];
    let machines: std::collections::BTreeSet<&str> = journal.iter().map(|i| i.machine.as_str()).collect();
    for m in machines {
        let mesurees: Vec<usize> = (0..journal.len()).filter(|&k| journal[k].machine == m && journal[k].reel.is_some()).collect();
        let fenetres: Vec<(u64, u64, &BTreeMap<String, f64>)> = mesurees.iter().map(|&k| journal[k].reel.as_ref().map(|r| (r.du, r.au, &estimes[k])).unwrap()).collect();
        for (k, ml) in mesurees.iter().zip(repartir(&releves(m), &fenetres, &contenance)) {
            reparti[*k] = Some(ml);
        }
    }
    journal
        .into_iter()
        .zip(estimes)
        .zip(reparti)
        .map(|((i, estime_ml), reel_ml)| {
            let estime_total_ml = (i.estime.methode == Methode::Points).then(|| estime_ml.values().sum());
            let reel_total_ml = reel_ml.as_ref().map(|m| m.values().sum::<f64>());
            let sous_seuil_ml = i.reel.as_ref().filter(|_| reel_total_ml == Some(0.0)).and_then(|r| r.seuil_ml.values().copied().reduce(f64::max));
            let longueur_reelle_mm = i.reel.as_ref().and_then(|r| r.longueur_mm);
            let gache_mm = longueur_reelle_mm.filter(|_| i.longueur_prevue_mm > 0.0).map(|l| l - i.longueur_prevue_mm);
            // Sous le seuil, le réel n'est pas 0 : pas d'écart chiffré.
            let ecart_pct = ecart(estime_total_ml, reel_total_ml.filter(|_| sous_seuil_ml.is_none()));
            Ligne { ecart_pct, impression: i, estime_ml, estime_total_ml, reel_ml, reel_total_ml, sous_seuil_ml, longueur_reelle_mm, gache_mm }
        })
        .collect()
}

/// Facteur proposé pour une machine, une résolution et une encre (6.6).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Proposition {
    pub machine: String,
    pub dpi: u32,
    pub encre: String,
    /// Réel / estimé avant correction, sur les `impressions` dernières.
    pub valeur: f64,
    pub impressions: usize,
    pub estime_ml: f64,
    pub reel_ml: f64,
    /// Facteur appliqué aujourd'hui.
    pub actuel: f64,
}

/// Facteurs proposés d'après les `n` dernières impressions mesurées de chaque combinaison
/// machine × résolution × encre ; rien n'est appliqué sans validation.
pub fn proposer(lignes: &[Ligne], n: usize, r: &Reglages) -> Vec<Proposition> {
    // (machine, dpi, encre) → (fin, estimé brut, réel) de chaque impression.
    type Cle = (String, u32, String);
    let mut groupes: BTreeMap<Cle, Vec<(u64, f64, f64)>> = BTreeMap::new();
    for l in lignes.iter().filter(|l| l.impression.estime.methode == Methode::Points) {
        let Some(reel) = &l.reel_ml else { continue };
        for (encre, &brut) in l.impression.estime.encres_ml.iter().filter(|(_, ml)| **ml > 0.0) {
            let cle = (l.impression.machine.clone(), l.impression.dpi, encre.clone());
            groupes.entry(cle).or_default().push((l.impression.fin, brut, reel.get(encre).copied().unwrap_or(0.0)));
        }
    }
    groupes
        .into_iter()
        .filter_map(|((machine, dpi, encre), mut v)| {
            v.sort_by_key(|x| std::cmp::Reverse(x.0));
            v.truncate(n);
            let (estime_ml, reel_ml) = v.iter().fold((0.0, 0.0), |(e, r), x| (e + x.1, r + x.2));
            (reel_ml > 0.0).then(|| Proposition { actuel: r.facteur(&machine, dpi, &encre), valeur: (reel_ml / estime_ml * 1000.0).round() / 1000.0, impressions: v.len(), estime_ml, reel_ml, machine, dpi, encre })
        })
        .collect()
}

/// Cellule du rapport : texte, ou nombre arrondi (décimales) qui s'additionne ou non.
#[derive(Clone, Debug, PartialEq)]
enum Cellule {
    Texte(String),
    Nombre(Option<f64>, usize, bool),
}

fn nombre(v: Option<f64>, decimales: usize, somme: bool) -> Cellule {
    let p = 10f64.powi(decimales as i32);
    Cellule::Nombre(v.map(|x| (x * p).round() / p), decimales, somme)
}

/// Colonnes du rapport, toujours les mêmes et dans cet ordre (une paire estimé/réel par
/// encre connue du catalogue, utilisée ou non).
pub fn colonnes() -> Vec<String> {
    let mut c: Vec<String> = ["date", "machine", "travail", "id", "page", "impressions", "support", "mode", "dpi", "operateur", "surface_m2", "longueur_prevue_mm", "longueur_reelle_mm", "gache_mm", "estimation", "encre_estimee_ml", "encre_reelle_ml", "sous_seuil", "ecart_pct"]
        .map(String::from)
        .into();
    for e in crate::supports::ENCRES {
        c.extend([format!("{e}_estime_ml"), format!("{e}_reel_ml")]);
    }
    c.extend(["cout_encres", "cout_support", "total", "base"].map(String::from));
    c
}

const ESTIMEE: usize = 15;
const REELLE: usize = 16;
const ECART: usize = 18;

fn cellules(l: &Ligne, date: &str) -> Vec<Cellule> {
    let i = &l.impression;
    let t = |s: &str| Cellule::Texte(s.to_string());
    let mut c = vec![
        t(date),
        t(&i.machine),
        t(&i.nom),
        t(&i.travail.to_string()),
        nombre(Some(i.page as f64), 0, false),
        nombre(Some(1.0), 0, true),
        t(i.support.as_deref().unwrap_or("")),
        t(i.mode.as_deref().unwrap_or("")),
        nombre(Some(i.dpi as f64), 0, false),
        t(&i.operateur),
        nombre(Some(i.surface_m2), 4, true),
        nombre(Some(i.longueur_prevue_mm), 1, true),
        nombre(l.longueur_reelle_mm, 1, true),
        nombre(l.gache_mm, 1, true),
        t(match i.estime.methode {
            Methode::Points => "points",
            Methode::NonCalculable => "non calculable",
        }),
        nombre(l.estime_total_ml, 2, true),
        nombre(l.reel_total_ml, 2, true),
        t(&l.sous_seuil_ml.map(|s| format!("< {} ml", s.round())).unwrap_or_default()),
        nombre(l.ecart_pct, 1, false),
    ];
    for e in crate::supports::ENCRES {
        c.push(nombre(l.estime_total_ml.map(|_| l.estime_ml.get(e).copied().unwrap_or(0.0)), 2, true));
        c.push(nombre(l.reel_ml.as_ref().map(|m| m.get(e).copied().unwrap_or(0.0)), 2, true));
    }
    let total = i.estime.cout_encres.map(|e| e + i.estime.cout_support);
    c.extend([nombre(i.estime.cout_encres, 4, true), nombre(Some(i.estime.cout_support), 4, true), nombre(total, 4, true), t(if i.estime.ttc { "TTC" } else { "HT" })]);
    c
}

/// Une ligne qui résume `lignes` : sommes des colonnes qui s'additionnent (des valeurs
/// arrondies, pour que le total soit la somme de ce qu'on lit), texte gardé s'il est le
/// même partout, écart recalculé.
fn agreger(lignes: &[Vec<Cellule>]) -> Vec<Cellule> {
    let mut r: Vec<Cellule> = (0..lignes[0].len())
        .map(|k| match &lignes[0][k] {
            Cellule::Texte(s) => Cellule::Texte(if lignes.iter().all(|l| l[k] == lignes[0][k]) { s.clone() } else { String::new() }),
            Cellule::Nombre(_, d, true) => {
                let v: Vec<f64> = lignes.iter().filter_map(|l| if let Cellule::Nombre(v, ..) = l[k] { v } else { None }).collect();
                nombre((!v.is_empty()).then(|| v.iter().sum()), *d, true)
            }
            Cellule::Nombre(v, d, false) => nombre(v.filter(|_| lignes.iter().all(|l| l[k] == lignes[0][k])), *d, false),
        })
        .collect();
    r[ECART] = calculer_ecart(&r);
    r
}

fn calculer_ecart(c: &[Cellule]) -> Cellule {
    let v = |k: usize| if let Cellule::Nombre(v, ..) = c[k] { v } else { None };
    nombre(ecart(v(ESTIMEE), v(REELLE)), 1, false)
}

/// Rapport CSV (6.5) : séparateur `;`, décimales à la virgule, une ligne par impression
/// (ou par jour et machine avec `par_jour`), puis le total. `date` met en forme une
/// heure Unix (`AAAA-MM-JJ hh:mm:ss`).
pub fn csv(lignes: &[Ligne], par_jour: bool, date: impl Fn(u64) -> String) -> String {
    let mut rangs: Vec<Vec<Cellule>> = lignes
        .iter()
        .map(|l| cellules(l, &date(l.impression.fin)))
        .collect();
    if par_jour {
        let mut jours: BTreeMap<(String, String), Vec<Vec<Cellule>>> = BTreeMap::new();
        for (l, mut c) in lignes.iter().zip(rangs) {
            let jour: String = date(l.impression.fin).chars().take(10).collect();
            c[0] = Cellule::Texte(jour.clone());
            jours.entry((jour, l.impression.machine.clone())).or_default().push(c);
        }
        rangs = jours.values().map(|v| agreger(v)).collect();
    }
    let texte = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));
    let ecrire = |c: &[Cellule]| -> String {
        c.iter()
            .map(|x| match x {
                Cellule::Texte(s) => texte(s),
                Cellule::Nombre(Some(v), d, _) => format!("{v:.d$}", d = *d).replace('.', ","),
                Cellule::Nombre(None, ..) => String::new(),
            })
            .collect::<Vec<_>>()
            .join(";")
    };
    let mut csv = colonnes().iter().map(|c| texte(c)).collect::<Vec<_>>().join(";") + "\n";
    for r in &rangs {
        csv += &(ecrire(r) + "\n");
    }
    if !rangs.is_empty() {
        let mut total = agreger(&rangs);
        total[0] = Cellule::Texte(String::from("total"));
        csv += &(ecrire(&total) + "\n");
    }
    csv
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(le: u64, c: u8, bk: u8, m: Option<f64>) -> Releve {
        Releve { le, encres: vec![("BK".into(), bk), ("C".into(), c)], longueur_m: m }
    }

    /// ml perdus par encre dans la fenêtre seule (sans répartition).
    fn ml(r: &Reel) -> BTreeMap<String, f64> {
        r.encres_pc.iter().map(|(n, &pc)| (n.clone(), pc as f64 * r.seuil_ml[n])).collect()
    }

    fn sept(_: &str) -> f64 {
        700.0
    }

    fn impression(debut: u64, fin: u64, estime_c: f64, reel: Option<Reel>) -> Impression {
        Impression {
            sortie: Uuid::new_v4(),
            travail: Uuid::new_v4(),
            nom: format!("t{debut}"),
            page: 1,
            machine: "epson".into(),
            debut,
            fin,
            support: Some("Vinyle".into()),
            mode: Some("720".into()),
            dpi: 720,
            operateur: "Léa".into(),
            surface_m2: 1.0,
            longueur_prevue_mm: 369.8,
            reel,
            estime: Estime { methode: Methode::Points, encres_ml: BTreeMap::from([("C".into(), estime_c)]), facteurs: BTreeMap::new(), cout_encres: Some(estime_c * 0.177), cout_support: 1.0, ttc: false },
        }
    }

    /// 6.1 — deux sorties consécutives : chacune prend les baisses de sa fenêtre, rien de
    /// l'autre ; la frontière (relevé forcé à la fin de A = début de B) n'est comptée qu'une fois.
    #[test]
    fn fenetre_de_chaque_sortie() {
        let v = [r(0, 50, 80, None), r(60, 49, 80, None), r(100, 48, 80, None), r(100, 48, 80, None), r(160, 48, 79, None), r(200, 47, 79, None), r(260, 46, 79, None)];
        let a = Reel::mesurer(&v, 0, 100, sept).unwrap();
        let b = Reel::mesurer(&v, 100, 200, sept).unwrap();
        assert_eq!(a.encres_pc, BTreeMap::from([("BK".into(), 0), ("C".into(), 2)]));
        assert_eq!(b.encres_pc, BTreeMap::from([("BK".into(), 1), ("C".into(), 1)]));
        assert_eq!(ml(&a)["C"], 14.0);
        assert_eq!((ml(&b)["C"], ml(&b)["BK"]), (7.0, 7.0));
        assert_eq!(a.seuil_ml["C"], 7.0);
        // Rien après 200 n'entre dans B (baisse à 260).
        assert_eq!(a.encres_pc["C"] + b.encres_pc["C"], 3);
        assert!(Reel::mesurer(&v[..1], 0, 100, sept).is_none(), "un relevé seul ne mesure rien");
    }

    /// 6.2 — petit travail sans baisse : « < 7 ml », pas 0. Une baisse qui couvre deux
    /// travaux est partagée au prorata de leur encre estimée.
    #[test]
    fn sous_le_seuil_et_prorata() {
        let releves = vec![r(0, 50, 80, None), r(100, 50, 80, None), r(200, 50, 80, None), r(300, 49, 80, None)];
        let a = impression(0, 100, 2.0, Reel::mesurer(&releves, 0, 100, sept));
        let lignes1 = lignes(vec![a.clone()], |_| releves[..2].to_vec(), sept);
        assert_eq!(lignes1[0].reel_total_ml, Some(0.0));
        assert_eq!(lignes1[0].sous_seuil_ml, Some(7.0), "affiché « < 7 ml »");
        assert_eq!(lignes1[0].ecart_pct, None, "pas de « −100 % » sous le seuil");
        let csv1 = csv(&lignes1, false, |_| "2026-10-25 10:00:00".into());
        assert!(csv1.lines().nth(1).unwrap().contains("\"< 7 ml\""), "{csv1}");
        // B (200..300) voit la baisse de 1 % ; A (0..100) avait aussi consommé depuis 0.
        let b = impression(200, 300, 6.0, Reel::mesurer(&releves, 200, 300, sept));
        let l = lignes(vec![a, b], |_| releves.clone(), sept);
        assert!((l[0].reel_total_ml.unwrap() - 7.0 * 2.0 / 8.0).abs() < 1e-9, "{:?}", l[0].reel_ml);
        assert!((l[1].reel_total_ml.unwrap() - 7.0 * 6.0 / 8.0).abs() < 1e-9);
        assert_eq!((l[0].sous_seuil_ml, l[1].sous_seuil_ml), (None, None));
        // Une nouvelle poche (hausse) remet le départ : la baisse suivante ne remonte pas avant.
        let poche = vec![r(0, 50, 80, None), r(100, 50, 80, None), r(150, 100, 80, None), r(300, 99, 80, None)];
        let v = repartir(&poche, &[(0, 100, &BTreeMap::from([("C".into(), 2.0)])), (200, 300, &BTreeMap::from([("C".into(), 6.0)]))], sept);
        assert_eq!((v[0].get("C"), v[1]["C"]), (None, 7.0));
    }

    /// 6.3 — longueur réelle et gâche : page de 369,8 mm, rouleau 50,000 → 49,426 m.
    /// 🏭 À refaire à l'atelier sur un vrai tirage (574 mm relevés le 24/10).
    #[test]
    fn longueur_reelle_et_gache() {
        let v = [r(0, 50, 80, Some(50.0)), r(100, 50, 80, Some(49.426))];
        let i = impression(0, 100, 1.0, Reel::mesurer(&v, 0, 100, sept));
        assert_eq!(i.reel.as_ref().unwrap().longueur_mm, Some(574.0));
        let l = &lignes(vec![i], |_| v.to_vec(), sept)[0];
        assert!((l.gache_mm.unwrap() - 204.2).abs() < 1e-9, "{:?}", l.gache_mm);
        // Sans longueur dans les relevés : inconnue, pas 0.
        let sans = Reel::mesurer(&[r(0, 50, 80, None), r(9, 50, 80, None)], 0, 9, sept).unwrap();
        assert_eq!(sans.longueur_mm, None);
    }

    /// 6.4 — rétention : les impressions finies avant la limite partent, les autres restent.
    #[test]
    fn purge_du_journal() {
        let d = std::env::temp_dir().join(format!("journal-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        let jour = 86_400;
        for fin in [10 * jour, 20 * jour, 30 * jour] {
            noter(&d, &impression(fin - 50, fin, 1.0, None)).unwrap();
        }
        assert_eq!(journal(&d).len(), 3);
        assert_eq!(purger(&d, 29 * jour).unwrap(), 2);
        assert_eq!(journal(&d).iter().map(|i| i.fin).collect::<Vec<_>>(), [30 * jour]);
        assert_eq!(purger(&d, 29 * jour).unwrap(), 0);
        std::fs::remove_dir_all(d).unwrap();
    }

    fn lire_csv(texte: &str) -> Vec<Vec<String>> {
        texte.lines().map(|l| l.split(';').map(|c| c.trim_matches('"').to_string()).collect()).collect()
    }

    fn valeur(c: &str) -> f64 {
        if c.is_empty() { 0.0 } else { c.replace(',', ".").parse().unwrap() }
    }

    /// 6.5 — colonnes stables (les mêmes avec ou sans données, quelles que soient les
    /// encres), décimales à la virgule, total = somme des lignes, estimé et réel côte à côte.
    #[test]
    fn rapport_csv() {
        let date = |s: u64| crate::date_utc(s);
        let vide = csv(&[], false, date);
        assert_eq!(vide.lines().count(), 1);
        let releves = vec![r(0, 50, 80, Some(50.0)), r(100, 48, 80, Some(49.5)), r(86_400, 48, 80, Some(49.5)), r(86_500, 47, 79, Some(49.0))];
        let mut a = impression(0, 100, 10.333, Reel::mesurer(&releves, 0, 100, sept));
        a.nom = "affiche;\"A3\"".into();
        let mut b = impression(86_400, 86_500, 5.0, Reel::mesurer(&releves, 86_400, 86_500, sept));
        b.estime.encres_ml.insert("LC".into(), 1.25);
        let mut c = impression(86_600, 86_700, 0.0, None);
        c.estime = Estime { cout_support: 0.5, ..Estime::default() };
        let l = lignes(vec![a, b, c], |_| releves.clone(), sept);
        let texte = csv(&l, false, date);
        assert_eq!(texte.lines().next(), vide.lines().next(), "en-tête stable");
        let t = lire_csv(&texte);
        let entete = &t[0];
        assert_eq!(entete.len(), colonnes().len());
        let col = |n: &str| entete.iter().position(|c| c == n).unwrap();
        assert!(texte.contains("\"affiche;\"\"A3\"\"\""), "nom protégé : {texte}");
        // Lignes : relire en sautant le nom (qui contient un `;`).
        let corps: Vec<Vec<String>> = texte.lines().skip(1).map(|l| l.replace("\"affiche;\"\"A3\"\"\"", "\"affiche\"").split(';').map(|c| c.trim_matches('"').to_string()).collect()).collect();
        assert_eq!(corps.len(), 4);
        assert_eq!(corps[0][col("encre_estimee_ml")], "10,33");
        // C : 2 % (14 ml) dans sa fenêtre ; BK : 1 % vu le lendemain, depuis le premier relevé,
        // partagé avec B à durée égale faute d'estimation (3,5 ml chacun).
        assert_eq!(corps[0][col("encre_reelle_ml")], "17,50");
        assert_eq!(corps[0][col("BK_reel_ml")], "3,50");
        assert_eq!(corps[0][col("ecart_pct")], "69,4");
        assert_eq!(corps[0][col("gache_mm")], "130,2");
        assert_eq!(corps[0][col("longueur_reelle_mm")], "500,0");
        assert_eq!(corps[1][col("LC_estime_ml")], "1,25");
        assert_eq!((corps[1][col("BK_reel_ml")].as_str(), corps[1][col("C_reel_ml")].as_str()), ("3,50", "7,00"));
        assert_eq!(corps[2][col("estimation")], "non calculable");
        assert_eq!((corps[2][col("encre_estimee_ml")].as_str(), corps[2][col("cout_encres")].as_str(), corps[2][col("total")].as_str()), ("", "", ""), "6.7 : pas de faux chiffre");
        let total = &corps[3];
        assert_eq!(total[0], "total");
        for n in ["impressions", "surface_m2", "encre_estimee_ml", "encre_reelle_ml", "C_estime_ml", "C_reel_ml", "BK_reel_ml", "cout_encres", "cout_support", "total", "longueur_reelle_mm", "gache_mm"] {
            let somme: f64 = corps[..3].iter().map(|x| valeur(&x[col(n)])).sum();
            assert!((valeur(&total[col(n)]) - somme).abs() < 1e-9, "{n} : {} ≠ {somme}", total[col(n)]);
        }
        assert_eq!(total[col("impressions")], "3");
        // Par jour : deux jours, mêmes colonnes, même total.
        let jours = lire_csv(&csv(&l, true, date).replace("\"affiche;\"\"A3\"\"\"", "\"affiche\""));
        assert_eq!(jours[0], *entete);
        assert_eq!(jours.iter().skip(1).map(|x| x[0].as_str()).collect::<Vec<_>>(), ["1970-01-01", "1970-01-02", "total"]);
        assert_eq!(jours[2][col("impressions")], "2");
        assert_eq!(jours[3][col("encre_reelle_ml")], total[col("encre_reelle_ml")]);
    }

    /// 6.6 — facteur proposé = réel / estimé (avant correction) sur les N dernières.
    #[test]
    fn facteur_propose() {
        // Trois impressions estimées à 10 ml de C, chacune avec sa baisse de 2 % (14 ml).
        let releves = vec![r(0, 50, 80, None), r(100, 48, 80, None), r(200, 46, 80, None), r(300, 44, 80, None)];
        let mut j: Vec<Impression> = [(0, 100), (100, 200), (200, 300)].iter().map(|&(a, b)| impression(a, b, 10.0, Reel::mesurer(&releves, a, b, sept))).collect();
        // Un facteur déjà appliqué ne change pas la proposition (elle part du brut).
        j[2].estime.facteurs.insert("C".into(), 1.2);
        let reglages = Reglages { facteurs: vec![Facteur { machine: "epson".into(), dpi: 720, encre: "C".into(), valeur: 1.2 }], ..Default::default() };
        let l = lignes(j, |_| releves.clone(), sept);
        let p = proposer(&l, 10, &reglages);
        assert_eq!(p.len(), 1);
        assert_eq!((p[0].encre.as_str(), p[0].dpi, p[0].impressions, p[0].valeur, p[0].actuel), ("C", 720, 3, 1.4, 1.2));
        assert_eq!(proposer(&l, 2, &reglages)[0].impressions, 2);
        assert_eq!(reglages.facteur("epson", 720, "C"), 1.2);
        assert_eq!(reglages.facteur("epson", 360, "C"), 1.0, "défaut 1");
        assert!(reglages.verifier().is_ok());
        let deux = Reglages { facteurs: vec![reglages.facteurs[0].clone(), reglages.facteurs[0].clone()], ..Default::default() };
        assert!(deux.verifier().is_err());
        assert!(Reglages { retention_jours: 0, ..Default::default() }.verifier().is_err());
    }

    /// Fichiers d'avant la phase 6 : une sortie sans consommation, un mode sans volumes de
    /// goutte se lisent toujours.
    #[test]
    fn anciens_fichiers_lisibles() {
        let sortie = serde_json::json!({ "id": Uuid::nil(), "machine": "epson", "travail": Uuid::nil(), "nom": "a", "page": 1, "etat": "terminee", "cree_le": 1 });
        let s: crate::file::Sortie = serde_json::from_value(sortie).unwrap();
        assert_eq!(s.consommation, None);
        let m: crate::supports::ModeImpression = serde_json::from_value(serde_json::json!({ "nom": "720", "dpi": [720, 720], "passes": 8 })).unwrap();
        assert_eq!(m.gouttes_pl, None);
        let r: Reglages = serde_json::from_str("{}").unwrap();
        assert_eq!(r, Reglages::default());
    }

    /// 6.9 — bande synthétique : N points de chaque taille × volume de goutte, au µl près.
    #[test]
    fn encre_des_points() {
        let n = 1_000_000;
        let ml = ml_des_points([n, n, n], GOUTTES_PL);
        // 10⁶ × (5 + 10 + 15) pL = 30 µl.
        assert!((ml - 0.030).abs() < 1e-6, "{ml}");
        let t = Tarifs::default();
        let e = estimer_points(&[("C".into(), [n, 0, 0]), ("Y".into(), [0; 3])], GOUTTES_PL, |n| if n == "C" { 2.0 } else { 1.0 }, &t, 0.5);
        assert_eq!(e.methode, Methode::Points);
        assert_eq!(e.encres_ml.keys().collect::<Vec<_>>(), ["C"], "encre sans point absente");
        assert!((e.corrige()["C"] - 0.010).abs() < 1e-12, "5 µl × facteur 2");
        // 0,01 ml à 177 €/l TTC, affiché HT (÷ 1,2).
        assert!((e.cout_encres.unwrap() - 0.01 / 1000.0 * 177.0 / 1.2).abs() < 1e-12);
        assert_eq!(contenance_ml(&t, "LC"), 700.0);
    }
}
