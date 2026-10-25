//! Découpe : les tracés peints dans un ton de découpe (`CutContour`, `Thrucut`…) sont
//! extraits du PDF puis écrits en HP-GL pour le plotter.
//!
//! On lit le contenu de la page : état graphique (`q`/`Q`/`cm`), formulaires XObject,
//! chemins (`m l c v y h re`) et couleur courante. Seule une couleur en Séparation dont
//! le nom commence par un préfixe de découpe rend un chemin « à découper ».
//!
//! Chaque nom de ton est une **couche** (roadmap 4.2) : l'ordre des couches est l'ordre de
//! coupe, une couche se désactive, et le réglage de coupe du même nom s'y applique seul.
//! La lecture relève aussi les défauts de préparation du fichier (roadmap 4.5).

use lopdf::{Dictionary, Document, Object, ObjectId, content::Content};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// Préfixes de découpe d'origine (ceux reconnus avant la 0.20), comparés sans casse,
/// espaces, tirets ni soulignés.
pub const PREFIXES_DEFAUT: [&str; 8] = ["CutContour", "Thrucut", "Kisscut", "InsideCut", "Perforation", "Creasing", "Score", "VCut"];

/// Réglages de découpe du serveur (`<données>/decoupe.json`). Globaux et non par machine :
/// le préfixe est une convention de préparation des fichiers (comme le Quick Set d'ONYX),
/// lue au rendu, avant qu'une découpeuse ne soit choisie ; il vaut pour toutes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Reglages {
    /// Préfixes lus en début de nom : `CutContour` reconnaît `CutContour1` et
    /// `CutContourKissCut`, pas `1CutContour`.
    pub prefixes: Vec<String>,
    /// Couper aussi les tracés des calques non imprimables (ignorés par défaut, avec alerte).
    #[serde(default)]
    pub calques_non_imprimables: bool,
    /// Tracés ouverts tolérés (roadmap 4.11, actif par défaut : évents d'une bâche) ; sinon
    /// ils ne sont pas coupés, avec alerte.
    #[serde(default = "vrai")]
    pub traces_ouverts: bool,
    /// Tri des tracés (intérieurs d'abord, plus proche voisin) ; sinon l'ordre de création.
    #[serde(default = "vrai")]
    pub trier: bool,
}

impl Default for Reglages {
    fn default() -> Self {
        Reglages { prefixes: PREFIXES_DEFAUT.map(String::from).to_vec(), calques_non_imprimables: false, traces_ouverts: true, trier: true }
    }
}

impl Reglages {
    pub fn verifier(&self) -> Result<(), &'static str> {
        let bon = (1..=30).contains(&self.prefixes.len()) && self.prefixes.iter().all(|p| p.chars().count() <= 40 && !cle(p).is_empty());
        if bon { Ok(()) } else { Err("prefixes_invalides") }
    }

    /// Le nom commence-t-il par un des préfixes ?
    pub fn reconnait(&self, nom: &str) -> bool {
        let c = cle(nom);
        self.prefixes.iter().any(|p| c.starts_with(&cle(p)))
    }
}

static REGLAGES: RwLock<Option<Reglages>> = RwLock::new(None);

/// Réglages en vigueur (ceux d'origine tant que rien n'est installé).
pub fn reglages() -> Reglages {
    REGLAGES.read().unwrap().clone().unwrap_or_default()
}

pub fn installer(r: Reglages) {
    *REGLAGES.write().unwrap() = Some(r);
}

pub fn fichier(donnees: &Path) -> PathBuf {
    donnees.join("decoupe.json")
}

pub fn lire(donnees: &Path) -> Reglages {
    crate::etat::lire(&fichier(donnees)).ok().flatten().unwrap_or_default()
}

pub fn ecrire(donnees: &Path, r: &Reglages) -> std::io::Result<()> {
    crate::etat::ecrire(&fichier(donnees), r, false)
}

/// Écart maximal entre une courbe et ses segments : 0,025 mm, en points PDF.
const TOLERANCE_PT: f64 = 0.025 * 72.0 / 25.4;

/// Unités HP-GL par point PDF : 100 par mm (pas de 0,010 mm). La découpe de contour du
/// Kona n'accepte que ce pas ; le menu « Resolution » du plotter doit être à 0,010 mm,
/// sinon tout sort 2,5 fois trop grand ou trop petit (manuel Kona, chap. 4).
const UNITES_PAR_PT: f64 = 100.0 * 25.4 / 72.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Trace {
    /// Nom du ton tel qu'écrit dans le PDF.
    pub ton: String,
    /// Points en points PDF, origine au coin bas gauche de la page.
    pub points: Vec<(f64, f64)>,
    pub ferme: bool,
}

fn cle(nom: &str) -> String {
    nom.chars().filter(|c| !matches!(c, ' ' | '-' | '_')).flat_map(char::to_lowercase).collect()
}

/// Nom de ton reconnu par les préfixes en vigueur.
pub fn est_ton_de_decoupe(nom: &str) -> bool {
    reglages().reconnait(nom)
}

/// Deux noms de ton de la même couche (`cut contour` et `CutContour`).
pub fn meme_couche(a: &str, b: &str) -> bool {
    cle(a) == cle(b)
}

/// Matrice PDF `[a b c d e f]`.
type Matrice = [f64; 6];
const IDENTITE: Matrice = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// `m` puis `n` (convention PDF : nouvelle CTM = `cm` × CTM).
fn produit(m: &Matrice, n: &Matrice) -> Matrice {
    [
        m[0] * n[0] + m[1] * n[2],
        m[0] * n[1] + m[1] * n[3],
        m[2] * n[0] + m[3] * n[2],
        m[2] * n[1] + m[3] * n[3],
        m[4] * n[0] + m[5] * n[2] + n[4],
        m[4] * n[1] + m[5] * n[3] + n[5],
    ]
}

fn transformer(m: &Matrice, (x, y): (f64, f64)) -> (f64, f64) {
    (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
}

#[derive(Clone)]
struct Etat {
    ctm: Matrice,
    /// Ton de découpe de l'espace de couleur courant (trait, fond), qu'il soit encré ou non.
    espace_trait: Option<String>,
    espace_fond: Option<String>,
    /// Ton de découpe effectivement encré (teinte > 0).
    trait_: Option<String>,
    fond: Option<String>,
    /// Sous-chemins des rognages en vigueur (`W n`), pour reconnaître un trait non centré.
    rognages: Vec<Vec<(f64, f64)>>,
    /// Contenu d'un calque non imprimable (formulaire ou contenu marqué appelant).
    masque: bool,
}

struct SousChemin {
    points: Vec<(f64, f64)>,
    ferme: bool,
}

/// Défaut de préparation relevé à la lecture : code (traduit par l'écran) et détail.
pub type Signal = (&'static str, String);

struct Lecteur<'a> {
    doc: &'a Document,
    /// Mode « tous les tracés » : tout chemin peint est découpé, quelle que soit sa couleur.
    tous: bool,
    reglages: &'a Reglages,
    /// Calques éteints de la configuration par défaut (`/OCProperties /D /OFF`).
    calques_eteints: Vec<ObjectId>,
    traces: Vec<Trace>,
    signaux: Vec<Signal>,
}

fn nombres(operandes: &[Object]) -> Vec<f64> {
    operandes.iter().filter_map(|o| o.as_float().ok()).map(f64::from).collect()
}

/// Courbe de Bézier aplatie : nombre de segments tiré de l'écart au polygone de contrôle.
fn bezier(sortie: &mut Vec<(f64, f64)>, p0: (f64, f64), p1: (f64, f64), p2: (f64, f64), p3: (f64, f64)) {
    let ecart = |a: (f64, f64), b: (f64, f64), c: (f64, f64)| ((a.0 - 2.0 * b.0 + c.0).powi(2) + (a.1 - 2.0 * b.1 + c.1).powi(2)).sqrt();
    let d = ecart(p0, p1, p2).max(ecart(p1, p2, p3));
    let n = ((0.75 * d / TOLERANCE_PT).sqrt().ceil() as usize).clamp(1, 1000);
    for i in 1..=n {
        let t = i as f64 / n as f64;
        let u = 1.0 - t;
        let (a, b, c, e) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
        sortie.push((a * p0.0 + b * p1.0 + c * p2.0 + e * p3.0, a * p0.1 + b * p1.1 + c * p2.1 + e * p3.1));
    }
}

fn deref<'d>(doc: &'d Document, o: &'d Object) -> &'d Object {
    doc.dereference(o).map(|(_, o)| o).unwrap_or(o)
}

fn sous_dict<'d>(doc: &'d Document, ressources: Option<&'d Dictionary>, cle: &[u8]) -> Option<&'d Dictionary> {
    deref(doc, ressources?.get(cle).ok()?).as_dict().ok()
}

/// Ton de découpe d'un espace de couleur nommé : `[/Separation /CutContour …]`, et vrai si
/// son espace de remplacement est en RVB (la découpe attend un ton défini en CMJN).
fn ton_espace(doc: &Document, ressources: Option<&Dictionary>, nom: &[u8], r: &Reglages) -> Option<(String, bool)> {
    let espaces = sous_dict(doc, ressources, b"ColorSpace")?;
    let tableau = deref(doc, espaces.get(nom).ok()?).as_array().ok()?;
    if deref(doc, tableau.first()?).as_name().ok()? != b"Separation" {
        return None;
    }
    let ton = String::from_utf8_lossy(deref(doc, tableau.get(1)?).as_name().ok()?).into_owned();
    let rvb = tableau.get(2).map(|o| deref(doc, o)).is_some_and(|alt| match alt {
        Object::Name(n) => n == b"DeviceRGB",
        Object::Array(a) => match a.first().and_then(|g| g.as_name().ok()) {
            Some(b"CalRGB") => true,
            Some(b"ICCBased") => a.get(1).and_then(|f| deref(doc, f).as_stream().ok()).and_then(|f| f.dict.get(b"N").ok()?.as_i64().ok()) == Some(3),
            _ => false,
        },
        _ => false,
    });
    r.reconnait(&ton).then_some((ton, rvb))
}

/// Calque (OCG) ou appartenance (OCMD) non imprimable : `/Usage /Print /PrintState /OFF`, à
/// défaut éteint dans la configuration par défaut. Une appartenance n'est non imprimable que
/// si tous ses calques le sont (politique `AnyOn`, la plus courante ; les autres politiques
/// et les expressions `/VE` sont lues comme `AnyOn`).
fn non_imprimable(doc: &Document, o: &Object, eteints: &[ObjectId]) -> bool {
    let id = o.as_reference().ok();
    let Ok(d) = deref(doc, o).as_dict() else { return false };
    if d.get(b"Type").and_then(Object::as_name).ok() == Some(b"OCMD".as_slice()) {
        let calques: Vec<&Object> = match d.get(b"OCGs").map(|o| deref(doc, o)) {
            Ok(Object::Array(a)) => a.iter().collect(),
            Ok(_) => d.get(b"OCGs").into_iter().collect(),
            Err(_) => vec![],
        };
        return !calques.is_empty() && calques.iter().all(|c| non_imprimable(doc, c, eteints));
    }
    let impression = d
        .get(b"Usage")
        .ok()
        .and_then(|u| deref(doc, u).as_dict().ok())
        .and_then(|u| deref(doc, u.get(b"Print").ok()?).as_dict().ok())
        .and_then(|p| p.get(b"PrintState").ok()?.as_name().ok());
    match impression {
        Some(etat) => etat == b"OFF",
        None => id.is_some_and(|i| eteints.contains(&i)),
    }
}

fn boite(points: &[(f64, f64)]) -> [f64; 4] {
    points.iter().fold([f64::MAX, f64::MAX, f64::MIN, f64::MIN], |b, &(x, y)| [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)])
}

fn chevauchent(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}

fn memes_points(a: &[(f64, f64)], b: &[(f64, f64)]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(p, q)| (p.0 - q.0).abs() < 0.01 && (p.1 - q.1).abs() < 0.01)
}

/// Contour vectorisé : un remplissage de deux sous-chemins fermés emboîtés, presque
/// parallèles (retraits de la boîte égaux, 2 mm au plus) — un filet devenu surface, que la
/// lame suivrait deux fois.
fn contour_vectorise(chemin: &[SousChemin]) -> bool {
    let boites: Vec<[f64; 4]> = chemin.iter().filter(|s| s.points.len() >= 3).map(|s| boite(&s.points)).collect();
    boites.iter().enumerate().any(|(i, a)| {
        boites.iter().enumerate().any(|(j, b)| {
            let retraits = [b[0] - a[0], b[1] - a[1], a[2] - b[2], a[3] - b[3]];
            let (min, max) = retraits.iter().fold((f64::MAX, f64::MIN), |(m, n), r| (m.min(*r), n.max(*r)));
            i != j && min > 0.0 && max <= 2.0 * 72.0 / 25.4 && max - min <= (0.25 * max).max(0.5)
        })
    })
}

impl<'a> Lecteur<'a> {
    fn signaler(&mut self, code: &'static str, detail: &str) {
        if !self.tous && !self.signaux.iter().any(|(c, d)| *c == code && d == detail) {
            self.signaux.push((code, detail.to_string()));
        }
    }

    /// Encre peinte (ni découpe ni masquée) : un tracé de découpe déjà posé dessous n'est
    /// plus au premier plan.
    // ponytail: comparaison de boîtes, tracés × peintures ; s'arrête au premier défaut.
    fn peinture(&mut self, b: [f64; 4]) {
        if self.tous || self.signaux.iter().any(|(c, _)| *c == "decoupe_pas_au_premier_plan") {
            return;
        }
        if let Some(t) = self.traces.iter().find(|t| chevauchent(boite(&t.points), b)) {
            let ton = t.ton.clone();
            self.signaler("decoupe_pas_au_premier_plan", &ton);
        }
    }

    fn lire(&mut self, contenu: &[u8], ressources: Option<&'a Dictionary>, etat_initial: Etat, profondeur: u32) -> Result<(), String> {
        let doc = self.doc;
        if profondeur > 20 {
            return Err("formulaires imbriqués trop profondément".into());
        }
        let operations = Content::decode(contenu).map_err(|e| format!("contenu de page illisible : {e}"))?.operations;
        let mut etat = etat_initial;
        let mut pile: Vec<Etat> = vec![];
        let mut chemin: Vec<SousChemin> = vec![];
        let mut courant = (0.0, 0.0);
        let mut debut = (0.0, 0.0);
        // Contenu marqué (`BDC`/`BMC` … `EMC`) : vrai pour un calque non imprimable.
        let mut marques: Vec<bool> = vec![];
        let mut rognage_attendu = false;

        for op in &operations {
            let n = nombres(&op.operands);
            let pt = |i: usize| transformer(&etat.ctm, (n[i], n[i + 1]));
            match op.operator.as_str() {
                "q" => pile.push(etat.clone()),
                "Q" => etat = pile.pop().unwrap_or(etat),
                "cm" if n.len() == 6 => etat.ctm = produit(&[n[0], n[1], n[2], n[3], n[4], n[5]], &etat.ctm),
                "BMC" => marques.push(false),
                "BDC" => {
                    let oc = op.operands.first().and_then(|o| o.as_name().ok()) == Some(b"OC".as_slice());
                    let propriete = match op.operands.get(1) {
                        Some(Object::Name(nom)) => sous_dict(doc, ressources, b"Properties").and_then(|p| p.get(nom).ok()),
                        o => o,
                    };
                    marques.push(oc && propriete.is_some_and(|p| non_imprimable(doc, p, &self.calques_eteints)));
                }
                "EMC" => {
                    marques.pop();
                }
                "CS" | "cs" => {
                    let lu = op.operands.first().and_then(|o| o.as_name().ok()).and_then(|nom| ton_espace(doc, ressources, nom, self.reglages));
                    if let Some((ton, true)) = &lu {
                        self.signaler("decoupe_ton_rvb", ton);
                    }
                    let ton = lu.map(|(t, _)| t);
                    // Une Séparation commence à la teinte 1.
                    if op.operator == "CS" {
                        (etat.espace_trait, etat.trait_) = (ton.clone(), ton);
                    } else {
                        (etat.espace_fond, etat.fond) = (ton.clone(), ton);
                    }
                }
                "SC" | "SCN" => etat.trait_ = etat.espace_trait.clone().filter(|_| n.last().is_some_and(|t| *t > 0.0)),
                "sc" | "scn" => etat.fond = etat.espace_fond.clone().filter(|_| n.last().is_some_and(|t| *t > 0.0)),
                "G" | "RG" | "K" => (etat.espace_trait, etat.trait_) = (None, None),
                "g" | "rg" | "k" => (etat.espace_fond, etat.fond) = (None, None),
                "m" if n.len() == 2 => {
                    courant = pt(0);
                    debut = courant;
                    chemin.push(SousChemin { points: vec![courant], ferme: false });
                }
                "l" if n.len() == 2 => {
                    courant = pt(0);
                    if let Some(s) = chemin.last_mut() {
                        s.points.push(courant);
                    }
                }
                "c" | "v" | "y" if n.len() >= 4 => {
                    let (p1, p2, p3) = match op.operator.as_str() {
                        "c" if n.len() == 6 => (pt(0), pt(2), pt(4)),
                        "v" => (courant, pt(0), pt(2)),
                        "y" => (pt(0), pt(2), pt(2)),
                        _ => continue,
                    };
                    if let Some(s) = chemin.last_mut() {
                        bezier(&mut s.points, courant, p1, p2, p3);
                    }
                    courant = p3;
                }
                "h" => {
                    if let Some(s) = chemin.last_mut() {
                        s.ferme = true;
                    }
                    courant = debut;
                }
                "re" if n.len() == 4 => {
                    let (x, y, w, h) = (n[0], n[1], n[2], n[3]);
                    let coins = [(x, y), (x + w, y), (x + w, y + h), (x, y + h)].map(|p| transformer(&etat.ctm, p));
                    chemin.push(SousChemin { points: coins.to_vec(), ferme: true });
                    (courant, debut) = (coins[0], coins[0]);
                }
                "W" | "W*" => rognage_attendu = true,
                "S" | "s" | "f" | "F" | "f*" | "B" | "B*" | "b" | "b*" | "n" => {
                    let operateur = op.operator.as_str();
                    // Le rognage prend effet après l'opérateur qui termine le chemin.
                    let rognage = std::mem::take(&mut rognage_attendu);
                    let remplissage = matches!(operateur, "f" | "F" | "f*");
                    let mixte = matches!(operateur, "B" | "B*" | "b" | "b*");
                    let ton = match operateur {
                        "n" => None,
                        _ if self.tous => Some(TOUS.to_string()),
                        "S" | "s" => etat.trait_.clone(),
                        _ if remplissage => etat.fond.clone(),
                        _ => etat.trait_.clone().or_else(|| etat.fond.clone()),
                    };
                    // Un remplissage ou une fermeture (s, b) ferme chaque sous-chemin.
                    let fermer = operateur != "S" && operateur != "B" && operateur != "B*";
                    let masque = etat.masque || marques.contains(&true);
                    match ton {
                        Some(ton) => {
                            if remplissage && contour_vectorise(&chemin) {
                                self.signaler("decoupe_contour_vectorise", &ton);
                            } else if remplissage || mixte {
                                self.signaler("decoupe_avec_fond", &ton);
                            }
                            let en_trait = !remplissage && etat.trait_.is_some();
                            if en_trait && chemin.iter().any(|s| etat.rognages.iter().any(|r| memes_points(r, &s.points))) {
                                self.signaler("decoupe_trait_non_centre", &ton);
                            }
                            if masque && !self.reglages.calques_non_imprimables && !self.tous {
                                self.signaler("decoupe_calque_non_imprimable", &ton);
                            } else {
                                for s in chemin.iter().filter(|s| s.points.len() >= 2) {
                                    self.traces.push(Trace { ton: ton.clone(), points: s.points.clone(), ferme: s.ferme || fermer });
                                }
                            }
                        }
                        None if operateur != "n" && !masque => {
                            let points: Vec<(f64, f64)> = chemin.iter().flat_map(|s| s.points.iter().copied()).collect();
                            if !points.is_empty() {
                                self.peinture(boite(&points));
                            }
                        }
                        None => {}
                    }
                    if rognage {
                        etat.rognages.extend(chemin.iter().map(|s| s.points.clone()));
                    }
                    chemin.clear();
                }
                "Do" => {
                    let Some(nom) = op.operands.first().and_then(|o| o.as_name().ok()) else { continue };
                    let Some(xobjets) = sous_dict(doc, ressources, b"XObject") else { continue };
                    let Ok(flux) = xobjets.get(nom).map(|o| deref(doc, o)).and_then(Object::as_stream) else { continue };
                    let masque = etat.masque || marques.contains(&true) || flux.dict.get(b"OC").is_ok_and(|o| non_imprimable(doc, o, &self.calques_eteints));
                    match flux.dict.get(b"Subtype").and_then(Object::as_name).ok() {
                        Some(b"Form") => {}
                        // Une image occupe le carré unité de la matrice courante.
                        Some(b"Image") if !masque => {
                            let coins = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)].map(|p| transformer(&etat.ctm, p));
                            self.peinture(boite(&coins));
                            continue;
                        }
                        _ => continue,
                    }
                    let matrice = flux
                        .dict
                        .get(b"Matrix")
                        .and_then(Object::as_array)
                        .map(|a| nombres(a))
                        .ok()
                        .filter(|m| m.len() == 6)
                        .map(|m| [m[0], m[1], m[2], m[3], m[4], m[5]])
                        .unwrap_or(IDENTITE);
                    let contenu = flux.decompressed_content().unwrap_or_else(|_| flux.content.clone());
                    let ressources_form = flux.dict.get(b"Resources").ok().and_then(|o| deref(doc, o).as_dict().ok()).or(ressources);
                    let mut etat_form = etat.clone();
                    etat_form.ctm = produit(&matrice, &etat.ctm);
                    etat_form.masque = masque;
                    let debut = self.traces.len();
                    self.lire(&contenu, ressources_form, etat_form.clone(), profondeur + 1)?;
                    // La BBox du formulaire rogne son contenu (ISO 32000-1, 8.10.1) : la
                    // découpe aussi. Seulement si elle reste un rectangle droit (échelles,
                    // quarts de tour, miroirs) ; sinon les tracés sont gardés entiers.
                    let bbox = flux.dict.get(b"BBox").and_then(Object::as_array).map(|a| nombres(a)).ok().filter(|b| b.len() == 4);
                    let m = etat_form.ctm;
                    if let Some(b) = bbox.filter(|_| (m[1].abs() < 1e-9 && m[2].abs() < 1e-9) || (m[0].abs() < 1e-9 && m[3].abs() < 1e-9)) {
                        let (p, q) = (transformer(&m, (b[0], b[1])), transformer(&m, (b[2], b[3])));
                        let (x0, y0) = (p.0.min(q.0), p.1.min(q.1));
                        let decales = |traces: Vec<Trace>, dx: f64, dy: f64| traces.into_iter().map(|t| Trace { points: t.points.into_iter().map(|(x, y)| (x + dx, y + dy)).collect(), ..t }).collect::<Vec<_>>();
                        let nouveaux = self.traces.split_off(debut);
                        self.traces.extend(decales(rogner(decales(nouveaux, -x0, -y0), (p.0 - q.0).abs(), (p.1 - q.1).abs()), x0, y0));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

/// PDF à imprimer : chaque ton de découpe des espaces `Separation` et `DeviceN` devient le
/// colorant spécial `None`, qui « ne produit jamais de sortie visible » (ISO 32000-1,
/// 8.6.6.4). Sans cela, un filet `CutContour` sans surimpression défonce l'encre dessous et
/// laisse un liseré non imprimé le long de la découpe. Rend le PDF et le nombre de tons
/// neutralisés (0 : le PDF est rendu tel quel).
pub fn neutraliser(pdf: &[u8]) -> Result<(Vec<u8>, usize), String> {
    fn nom_de_coupe(o: &Object) -> bool {
        o.as_name().is_ok_and(|n| est_ton_de_decoupe(&String::from_utf8_lossy(n)))
    }
    fn visiter(o: &mut Object, n: &mut usize) {
        match o {
            Object::Array(a) => {
                let genre = a.first().and_then(|g| g.as_name().ok()).map(<[u8]>::to_vec);
                match (genre.as_deref(), a.get_mut(1)) {
                    (Some(b"Separation"), Some(nom)) if nom_de_coupe(nom) => {
                        *nom = Object::Name(b"None".to_vec());
                        *n += 1;
                    }
                    (Some(b"DeviceN"), Some(Object::Array(noms))) => {
                        for nom in noms.iter_mut().filter(|x| nom_de_coupe(x)) {
                            *nom = Object::Name(b"None".to_vec());
                            *n += 1;
                        }
                    }
                    _ => {}
                }
                a.iter_mut().for_each(|x| visiter(x, n));
            }
            Object::Dictionary(d) => d.iter_mut().for_each(|(_, v)| visiter(v, n)),
            Object::Stream(s) => s.dict.iter_mut().for_each(|(_, v)| visiter(v, n)),
            _ => {}
        }
    }
    let mut doc = Document::load_mem(pdf).map_err(|e| format!("PDF illisible : {e}"))?;
    let mut n = 0;
    doc.objects.values_mut().for_each(|o| visiter(o, &mut n));
    if n == 0 {
        return Ok((pdf.to_vec(), 0));
    }
    let mut sortie = vec![];
    doc.save_to(&mut sortie).map_err(|e| e.to_string())?;
    Ok((sortie, n))
}

/// Ton attribué aux tracés en mode « tous les tracés » (lame).
pub const TOUS: &str = "Tous";

/// Tracés à découper et format de la page, en points PDF.
pub struct Decoupe {
    pub traces: Vec<Trace>,
    pub largeur: f64,
    pub hauteur: f64,
    /// Défauts de préparation de la page (roadmap 4.5), sans doublon.
    pub signaux: Vec<Signal>,
}

impl Decoupe {
    /// Déplace les tracés comme le contenu de la page (repérage) et prend la nouvelle page.
    pub fn decaler(&mut self, (dx, dy): (f64, f64), (largeur, hauteur): (f64, f64)) {
        for t in &mut self.traces {
            for p in &mut t.points {
                *p = (p.0 + dx, p.1 + dy);
            }
        }
        (self.largeur, self.hauteur) = (largeur, hauteur);
    }
}

/// Tracés de découpe de chaque page d'un PDF (ou d'un fichier Illustrator enregistré
/// avec « Créer un fichier compatible PDF »), dans l'ordre des pages, avec les réglages
/// de découpe en vigueur.
pub fn extraire(pdf: &[u8], tous: bool) -> Result<Vec<Decoupe>, String> {
    extraire_avec(pdf, tous, &reglages())
}

pub fn extraire_avec(pdf: &[u8], tous: bool, r: &Reglages) -> Result<Vec<Decoupe>, String> {
    let doc = Document::load_mem(pdf).map_err(|e| format!("PDF illisible : {e}"))?;
    let pages: Vec<_> = doc.get_pages().into_values().collect();
    if pages.is_empty() {
        return Err("PDF sans page".into());
    }
    pages.into_iter().enumerate().map(|(i, page)| extraire_page(&doc, page, tous, r).map_err(|e| format!("page {} : {e}", i + 1))).collect()
}

/// Calques éteints par défaut : `/Root /OCProperties /D /OFF`.
fn calques_eteints(doc: &Document) -> Vec<ObjectId> {
    let racine = doc.trailer.get(b"Root").ok().and_then(|o| deref(doc, o).as_dict().ok());
    let proprietes = racine.and_then(|r| deref(doc, r.get(b"OCProperties").ok()?).as_dict().ok());
    let defaut = proprietes.and_then(|p| deref(doc, p.get(b"D").ok()?).as_dict().ok());
    defaut.and_then(|d| deref(doc, d.get(b"OFF").ok()?).as_array().ok()).map(|a| a.iter().filter_map(|o| o.as_reference().ok()).collect()).unwrap_or_default()
}

fn extraire_page(doc: &Document, page: lopdf::ObjectId, tous: bool, r: &Reglages) -> Result<Decoupe, String> {
    let dict_page = doc.get_dictionary(page).map_err(|e| e.to_string())?;
    let ressources = match doc.get_page_resources(page).map_err(|e| e.to_string())? {
        (Some(d), _) => Some(d),
        (None, ids) => ids.first().and_then(|id| doc.get_dictionary(*id).ok()),
    };
    // Origine au coin bas gauche de la MediaBox.
    let boite = dict_page
        .get(b"MediaBox")
        .and_then(Object::as_array)
        .map(|a| nombres(a))
        .ok()
        .filter(|m| m.len() == 4)
        .ok_or("page sans MediaBox")?;
    let ctm = [1.0, 0.0, 0.0, 1.0, -boite[0], -boite[1]];
    let etat = Etat { ctm, espace_trait: None, espace_fond: None, trait_: None, fond: None, rognages: vec![], masque: false };

    let mut lecteur = Lecteur { doc, tous, reglages: r, calques_eteints: calques_eteints(doc), traces: vec![], signaux: vec![] };
    lecteur.lire(&doc.get_page_content(page), ressources, etat, 0)?;
    let (largeur, hauteur) = (boite[2] - boite[0], boite[3] - boite[1]);
    // Hors du plan de travail (rogné ensuite à la page), ou posé sur son bord.
    let e = 0.01;
    for t in std::mem::take(&mut lecteur.traces) {
        let dehors = t.points.iter().any(|&(x, y)| x < -e || y < -e || x > largeur + e || y > hauteur + e);
        let bord = t.points.iter().any(|&(x, y)| x.abs() < e || y.abs() < e || (x - largeur).abs() < e || (y - hauteur).abs() < e);
        if dehors {
            lecteur.signaler("decoupe_hors_page", &t.ton);
        } else if bord {
            lecteur.signaler("decoupe_touche_bord", &t.ton);
        }
        lecteur.traces.push(t);
    }
    Ok(Decoupe { traces: rogner(lecteur.traces, largeur, hauteur), largeur, hauteur, signaux: lecteur.signaux })
}

/// Noms des tons en Séparation du PDF : pour dire à l'opérateur ce que le fichier contient
/// quand aucun n'est reconnu.
pub fn tons_du_pdf(pdf: &[u8]) -> Vec<String> {
    fn visiter(o: &Object, tons: &mut Vec<String>) {
        match o {
            Object::Array(a) => {
                if a.first().and_then(|g| g.as_name().ok()) == Some(b"Separation".as_slice())
                    && let Some(n) = a.get(1).and_then(|n| n.as_name().ok())
                {
                    tons.push(String::from_utf8_lossy(n).into_owned());
                }
                a.iter().for_each(|x| visiter(x, tons));
            }
            Object::Dictionary(d) => d.iter().for_each(|(_, v)| visiter(v, tons)),
            Object::Stream(s) => s.dict.iter().for_each(|(_, v)| visiter(v, tons)),
            _ => {}
        }
    }
    let Ok(doc) = Document::load_mem(pdf) else { return vec![] };
    let mut tons = vec![];
    doc.objects.values().for_each(|o| visiter(o, &mut tons));
    tons.sort();
    tons.dedup();
    tons
}

/// Tracés d'un SVG. Sans couleur en Séparation dans ce format, un tracé est « de
/// découpe » quand son `id` ou celui d'un groupe parent porte un nom de ton : c'est le
/// nom du calque ou du tracé qu'Illustrator écrit à l'export.
pub fn extraire_svg(svg: &[u8], tous: bool) -> Result<Decoupe, String> {
    let arbre = usvg::Tree::from_data(svg, &usvg::Options::default()).map_err(|e| format!("SVG illisible : {e}"))?;
    // Unités SVG (1/96 pouce, axe y vers le bas) → points PDF (axe y vers le haut).
    let k = 72.0 / 96.0;
    let hauteur = arbre.size().height() as f64 * k;
    let vers_pt = |p: usvg::tiny_skia_path::Point| (p.x as f64 * k, hauteur - p.y as f64 * k);
    let mut traces = vec![];
    parcourir_svg(arbre.root(), None, tous, &vers_pt, &mut traces);
    Ok(Decoupe { traces, largeur: arbre.size().width() as f64 * k, hauteur, signaux: vec![] })
}

fn parcourir_svg(groupe: &usvg::Group, ton_parent: Option<&str>, tous: bool, vers_pt: &dyn Fn(usvg::tiny_skia_path::Point) -> (f64, f64), traces: &mut Vec<Trace>) {
    let ton_groupe = Some(groupe.id()).filter(|id| est_ton_de_decoupe(id)).or(ton_parent);
    for noeud in groupe.children() {
        match noeud {
            usvg::Node::Group(g) => parcourir_svg(g, ton_groupe, tous, vers_pt, traces),
            usvg::Node::Path(chemin) if chemin.is_visible() => {
                let ton = Some(chemin.id()).filter(|id| est_ton_de_decoupe(id)).or(ton_groupe);
                let ton = if tous { TOUS } else if let Some(ton) = ton { ton } else { continue };
                let Some(donnees) = chemin.data().clone().transform(chemin.abs_transform()) else { continue };
                let mut courant = vec![];
                let mut depart = (0.0, 0.0);
                let mut position = (0.0, 0.0);
                let mut fermer = |points: &mut Vec<(f64, f64)>, ferme: bool| {
                    if points.len() >= 2 {
                        traces.push(Trace { ton: ton.to_string(), points: std::mem::take(points), ferme });
                    }
                    points.clear();
                };
                use usvg::tiny_skia_path::PathSegment as S;
                for segment in donnees.segments() {
                    match segment {
                        S::MoveTo(p) => {
                            fermer(&mut courant, false);
                            position = vers_pt(p);
                            depart = position;
                            courant.push(position);
                        }
                        S::LineTo(p) => {
                            position = vers_pt(p);
                            courant.push(position);
                        }
                        S::QuadTo(c, p) => {
                            // Quadratique en cubique : points de contrôle aux deux tiers.
                            let (c, p) = (vers_pt(c), vers_pt(p));
                            let c1 = (position.0 + 2.0 / 3.0 * (c.0 - position.0), position.1 + 2.0 / 3.0 * (c.1 - position.1));
                            let c2 = (p.0 + 2.0 / 3.0 * (c.0 - p.0), p.1 + 2.0 / 3.0 * (c.1 - p.1));
                            bezier(&mut courant, position, c1, c2, p);
                            position = p;
                        }
                        S::CubicTo(c1, c2, p) => {
                            let p = vers_pt(p);
                            bezier(&mut courant, position, vers_pt(c1), vers_pt(c2), p);
                            position = p;
                        }
                        S::Close => {
                            fermer(&mut courant, true);
                            position = depart;
                        }
                    }
                }
                fermer(&mut courant, false);
            }
            _ => {}
        }
    }
}

/// Aperçu des tracés à découper, à la taille de la page (1 unité = 1 point), en magenta pur :
/// la couleur d'usage d'un tracé de coupe, distincte de toute encre de l'image. Une flèche
/// grise, dans le coin bas gauche, montre le sens d'avance (roadmap 4.8) : vers le haut de
/// la page, le bord qui entre le premier (comme le ▲ du repérage).
pub fn apercu_svg(d: &Decoupe) -> String {
    let (l, h) = (d.largeur, d.hauteur);
    let a = l.min(h) * 0.12;
    let (x, y) = (a * 0.5, h - a * 0.15);
    let fleche = [(x - a * 0.08, y), (x + a * 0.08, y), (x + a * 0.08, y - a * 0.6), (x + a * 0.25, y - a * 0.6), (x, y - a), (x - a * 0.25, y - a * 0.6), (x - a * 0.08, y - a * 0.6)];
    let fleche: Vec<String> = fleche.iter().map(|(x, y)| format!("{x:.1},{y:.1}")).collect();
    let mut svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {l:.1} {h:.1}"><rect width="100%" height="100%" fill="#fff"/><polygon class="sens-avance" fill="#9aa0a6" fill-opacity="0.6" points="{f}"/><g fill="none" stroke="#ff00ff" stroke-width="{e:.2}" stroke-linejoin="round">"##,
        f = fleche.join(" "),
        e = (l.max(h) / 400.0).max(0.5)
    );
    for t in &d.traces {
        let points: Vec<String> = t.points.iter().map(|(x, y)| format!("{x:.1},{:.1}", d.hauteur - y)).collect();
        let balise = if t.ferme { "polygon" } else { "polyline" };
        svg += &format!(r#"<{balise} points="{}"/>"#, points.join(" "));
    }
    svg + "</g></svg>"
}

/// Segment `a`–`b` rogné au rectangle [0, l] × [0, h] (Liang–Barsky) ; `None` s'il est dehors.
fn rogner_segment(a: (f64, f64), b: (f64, f64), l: f64, h: f64) -> Option<((f64, f64), (f64, f64))> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (p, q) in [(-dx, a.0), (dx, l - a.0), (-dy, a.1), (dy, h - a.1)] {
        if p.abs() < 1e-12 {
            if q < -1e-9 {
                return None;
            }
            continue;
        }
        let r = q / p;
        if p < 0.0 {
            t0 = t0.max(r);
        } else {
            t1 = t1.min(r);
        }
        if t0 > t1 {
            return None;
        }
    }
    Some(((a.0 + t0 * dx, a.1 + t0 * dy), (a.0 + t1 * dx, a.1 + t1 * dy)))
}

/// Tracés rognés à la page [0, l] × [0, h] : un tracé qui déborde devient un ou plusieurs
/// tracés ouverts (la lame ne coupe pas hors du support) ; un tracé fermé coupé raccorde
/// son dernier morceau au premier.
pub fn rogner(traces: Vec<Trace>, l: f64, h: f64) -> Vec<Trace> {
    let pres = |p: (f64, f64), q: (f64, f64)| (p.0 - q.0).abs() < 1e-6 && (p.1 - q.1).abs() < 1e-6;
    let mut sortie = vec![];
    for t in traces {
        let mut points = t.points.clone();
        if t.ferme {
            points.push(t.points[0]);
        }
        let mut morceaux: Vec<Vec<(f64, f64)>> = vec![];
        let mut coupe = false;
        for w in points.windows(2) {
            let Some((a, b)) = rogner_segment(w[0], w[1], l, h) else {
                coupe = true;
                continue;
            };
            coupe |= !pres(a, w[0]) || !pres(b, w[1]);
            match morceaux.last_mut() {
                Some(m) if pres(*m.last().unwrap(), a) => m.push(b),
                _ => morceaux.push(vec![a, b]),
            }
        }
        if !coupe && t.ferme {
            sortie.push(t);
            continue;
        }
        // Tracé fermé coupé : le dernier morceau rejoint le premier s'ils se touchent.
        if t.ferme && morceaux.len() > 1 && pres(*morceaux.last().unwrap().last().unwrap(), morceaux[0][0]) {
            let mut fin = morceaux.pop().unwrap();
            fin.extend(morceaux.remove(0).into_iter().skip(1));
            morceaux.insert(0, fin);
        }
        sortie.extend(morceaux.into_iter().filter(|m| m.len() >= 2).map(|points| Trace { ton: t.ton.clone(), points, ferme: false }));
    }
    sortie
}

/// Outil du plotter selon le ton : 1 lame, 2 rainage, 3 perforation, 4 coupe en V.
fn outil(ton: &str) -> u8 {
    let c = cle(ton);
    if c.starts_with("creasing") || c.starts_with("score") {
        2
    } else if c.starts_with("perforation") {
        3
    } else if c.starts_with("vcut") {
        4
    } else {
        1
    }
}

/// Place d'une nouvelle couche dans l'ordre de coupe : rainage, perforation et V-cut
/// d'abord, puis la mi-chair (kiss cut), la coupe de contour, et la coupe à travers en
/// dernier, quand la matière tient encore.
fn rang(ton: &str) -> u8 {
    let c = cle(ton);
    match outil(ton) {
        2 => 0,
        3 => 1,
        4 => 2,
        _ if ["kiss", "inside", "michair"].iter().any(|m| c.contains(m)) => 3,
        _ if ["thru", "through"].iter().any(|m| c.contains(m)) => 5,
        _ => 4,
    }
}

fn vrai() -> bool {
    true
}

/// Couche de découpe d'un travail : les tracés d'un même nom de ton. L'ordre de la liste
/// est l'ordre de coupe.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Couche {
    /// Nom du ton tel qu'écrit dans le fichier (les variantes de casse, espaces et tirets
    /// sont la même couche).
    pub nom: String,
    #[serde(default = "vrai")]
    pub active: bool,
    /// Réglage de coupe choisi à la main (nom) ; sinon celui qui porte le nom de la couche,
    /// sinon le premier de la découpeuse.
    #[serde(default)]
    pub reglage: Option<String>,
}

/// Noms de ton des tracés, dans l'ordre d'apparition, une fois par couche.
pub fn tons(traces: &[Trace]) -> Vec<&str> {
    let mut v: Vec<&str> = vec![];
    for t in traces {
        if !v.iter().any(|x| meme_couche(x, &t.ton)) {
            v.push(&t.ton);
        }
    }
    v
}

/// Ajoute aux couches connues celles des `tons` nouveaux, à leur rang par défaut ; les
/// couches connues gardent leur ordre et leurs réglages (même sans tracé).
pub fn accorder(couches: &mut Vec<Couche>, tons: &[&str]) {
    for ton in tons {
        if couches.iter().any(|c| meme_couche(&c.nom, ton)) {
            continue;
        }
        let r = rang(ton);
        let i = couches.iter().position(|c| rang(&c.nom) > r).unwrap_or(couches.len());
        couches.insert(i, Couche { nom: ton.to_string(), active: true, reglage: None });
    }
}

/// Ancien réglage de coupe du travail entier (avant les couches) porté sur une couche, si
/// c'est sans ambiguïté : la couche du même nom, sinon la seule couche. Une couche déjà
/// réglée à la main n'est pas touchée. Rend vrai s'il est placé (alors il n'a plus lieu
/// d'être gardé à part) ; faux : plusieurs couches possibles, à montrer à l'opérateur.
pub fn reprendre_ancien_reglage(couches: &mut [Couche], reglage: &str) -> bool {
    let cible = match couches.iter().position(|c| meme_couche(&c.nom, reglage)) {
        Some(i) => i,
        None if couches.len() == 1 => 0,
        None => return false,
    };
    couches[cible].reglage.get_or_insert_with(|| reglage.to_string());
    true
}

/// HP-GL de chaque couche active qui a des tracés, dans l'ordre des couches : `SPn;` puis
/// ses tracés (intérieurs d'abord, plus proche voisin, sauf tri désactivé : ordre de
/// création ; les tracés ouverts seulement s'ils sont tolérés), unités 1/100 mm, sans `IN;`
/// ni fin.
pub fn blocs(traces: &[Trace], couches: &[Couche], r: &Reglages) -> Vec<(Couche, String)> {
    let u = |(x, y): (f64, f64)| ((x * UNITES_PAR_PT).round() as i64, (y * UNITES_PAR_PT).round() as i64);
    let mut toutes = couches.to_vec();
    accorder(&mut toutes, &tons(traces));
    toutes
        .into_iter()
        .filter(|c| c.active)
        .filter_map(|c| {
            let de: Vec<&Trace> = traces.iter().filter(|t| meme_couche(&t.ton, &c.nom) && (t.ferme || r.traces_ouverts)).collect();
            if de.is_empty() {
                return None;
            }
            let mut sortie = format!("SP{};", outil(&c.nom));
            for t in if r.trier { ordonner(&de) } else { de } {
                let (x0, y0) = u(t.points[0]);
                sortie += &format!("PU{x0},{y0};PD");
                let mut suite: Vec<String> = t.points[1..].iter().map(|p| u(*p)).map(|(x, y)| format!("{x},{y}")).collect();
                if t.ferme {
                    suite.push(format!("{x0},{y0}"));
                }
                sortie += &suite.join(",");
                sortie += ";";
            }
            Some((c, sortie))
        })
        .collect()
}

/// Tracés relus d'un HP-GL du RIP (`SPn;PUx,y;PD…`, 1/100 mm) : un tracé par `PU`, fermé
/// quand il revient à son départ ; le ton est celui de l'outil (1 CutContour, 2 Creasing,
/// 3 Perforation, 4 VCut). Pour les travaux rendus avant les couches, dont seul le `.plt`
/// est gardé : relu ainsi puis réécrit par `hpgl`, il redonne les mêmes coordonnées.
pub fn depuis_hpgl(texte: &str) -> Vec<Trace> {
    let ton = |o: u8| match o {
        2 => "Creasing",
        3 => "Perforation",
        4 => "VCut",
        _ => "CutContour",
    };
    let mut traces = vec![];
    let mut outil = 1;
    let mut courant: Vec<(f64, f64)> = vec![];
    let vider = |courant: &mut Vec<(f64, f64)>, outil: u8, traces: &mut Vec<Trace>| {
        let mut points = std::mem::take(courant);
        if points.len() >= 2 {
            let ferme = points.len() >= 4 && points.first() == points.last();
            if ferme {
                points.pop();
            }
            traces.push(Trace { ton: ton(outil).into(), points, ferme });
        }
    };
    for cmd in texte.split(';').map(str::trim) {
        let points: Vec<(f64, f64)> = cmd.get(2..).unwrap_or("").split(',').filter_map(|n| n.trim().parse::<f64>().ok()).collect::<Vec<_>>().as_chunks::<2>().0.iter().map(|&[x, y]| (x / UNITES_PAR_PT, y / UNITES_PAR_PT)).collect();
        match cmd.get(..2) {
            Some("SP") => {
                vider(&mut courant, outil, &mut traces);
                outil = cmd[2..].trim().parse().unwrap_or(1);
            }
            Some("PU") => {
                vider(&mut courant, outil, &mut traces);
                courant.extend(points.last());
            }
            Some("PD") => courant.extend(points),
            _ => {}
        }
    }
    vider(&mut courant, outil, &mut traces);
    traces
}

/// Fichier HP-GL : les couches actives dans leur ordre, un `SP` par couche.
pub fn hpgl(traces: &[Trace], couches: &[Couche], r: &Reglages) -> String {
    let corps: String = blocs(traces, couches, r).into_iter().map(|(_, b)| b).collect();
    format!("IN;{corps}PU;SP0;")
}

pub(crate) fn dans_polygone((x, y): (f64, f64), poly: &[(f64, f64)]) -> bool {
    let mut dedans = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let ((xi, yi), (xj, yj)) = (poly[i], poly[j]);
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            dedans = !dedans;
        }
        j = i;
    }
    dedans
}

/// Ordre de coupe : un contour contenu dans un autre passe avant lui (la pièce ne bouge
/// pas pendant qu'on découpe ses trous) ; à profondeur égale, le plus proche de la lame
/// d'abord, pour limiter les déplacements à vide.
// ponytail: O(n²) en contours ; suffisant pour quelques milliers d'étiquettes par planche.
pub fn ordonner<'t>(traces: &[&'t Trace]) -> Vec<&'t Trace> {
    let boite = |t: &Trace| t.points.iter().fold((f64::MAX, f64::MAX, f64::MIN, f64::MIN), |(a, b, c, d), &(x, y)| (a.min(x), b.min(y), c.max(x), d.max(y)));
    let boites: Vec<_> = traces.iter().map(|t| boite(t)).collect();
    let profondeur = |i: usize| {
        let (bi, p) = (boites[i], traces[i].points[0]);
        (0..traces.len())
            .filter(|&j| j != i && traces[j].ferme && traces[j].points.len() >= 3)
            .filter(|&j| {
                let bj = boites[j];
                bj.0 <= bi.0 && bj.1 <= bi.1 && bj.2 >= bi.2 && bj.3 >= bi.3 && bj != bi && dans_polygone(p, &traces[j].points)
            })
            .count()
    };
    let profondeurs: Vec<usize> = (0..traces.len()).map(profondeur).collect();
    let mut restants: Vec<usize> = (0..traces.len()).collect();
    let mut ordre = Vec::with_capacity(traces.len());
    let mut lame = (0.0, 0.0);
    while !restants.is_empty() {
        let plus_profond = restants.iter().map(|&i| profondeurs[i]).max().unwrap();
        let (k, &i) = restants
            .iter()
            .enumerate()
            .filter(|(_, i)| profondeurs[**i] == plus_profond)
            .min_by(|(_, a), (_, b)| {
                let d = |i: usize| (traces[i].points[0].0 - lame.0).powi(2) + (traces[i].points[0].1 - lame.1).powi(2);
                d(**a).total_cmp(&d(**b))
            })
            .unwrap();
        restants.swap_remove(k);
        let t = traces[i];
        lame = if t.ferme { t.points[0] } else { *t.points.last().unwrap() };
        ordre.push(t);
    }
    ordre
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{Stream, dictionary};

    #[test]
    fn apercu_de_decoupe_en_magenta_pur() {
        let d = Decoupe { traces: vec![Trace { ton: "CutContour".into(), points: vec![(0.0, 0.0), (10.0, 10.0)], ferme: false }], largeur: 20.0, hauteur: 20.0, signaux: vec![] };
        let svg = apercu_svg(&d);
        assert!(svg.contains(r##"stroke="#ff00ff""##), "{svg}");
        assert!(svg.contains(r#"<polyline points="0.0,20.0 10.0,10.0"/>"#), "{svg}");
    }

    fn separation(ton: &str) -> Object {
        Object::Array(vec![
            "Separation".into(),
            Object::Name(ton.as_bytes().to_vec()),
            "DeviceCMYK".into(),
            Object::Dictionary(dictionary! { "FunctionType" => 2, "Domain" => vec![0.into(), 1.into()], "C0" => vec![0.into(); 4], "C1" => vec![0.into(), 1.into(), 0.into(), 0.into()], "N" => 1 }),
        ])
    }

    /// Page : un formulaire tourné de 90° puis décalé, qui découpe un carré de 72 pt, un trait
    /// `Thrucut`, un fond `Silver` et un trait noir (ignorés), un cercle `cut contour`.
    fn pdf_de_test() -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let espaces = dictionary! { "CS0" => separation("CutContour"), "CS1" => separation("Silver"), "CS2" => separation("Thrucut"), "CS3" => separation("cut contour") };
        let espaces_id = doc.add_object(espaces);
        let form = Stream::new(
            dictionary! { "Type" => "XObject", "Subtype" => "Form", "BBox" => vec![0.into(), 0.into(), 72.into(), 72.into()], "Matrix" => vec![0.into(), 1.into(), (-1).into(), 0.into(), 100.into(), 0.into()], "Resources" => dictionary! { "ColorSpace" => espaces_id } },
            b"/CS0 CS 1 SCN 0 0 72 72 re S".to_vec(),
        );
        let form_id = doc.add_object(form);
        // Cercle de rayon 50 centré en (400, 300), en quatre arcs de Bézier.
        let k = 0.5523 * 50.0;
        let cercle = format!(
            "/CS3 CS 1 SCN 450 300 m 450 {a} {b} 350 400 350 c {c} 350 350 {a} 350 300 c 350 {d} {c} 250 400 250 c {b} 250 450 {d} 450 300 c S",
            a = 300.0 + k, b = 400.0 + k, c = 400.0 - k, d = 300.0 - k
        );
        let contenu = format!(
            "q 1 0 0 1 0 50 cm /Fm0 Do Q /CS1 cs 1 scn 0 0 10 10 re f /CS2 CS 1 SCN 200 0 m 200 72 l S /CS2 CS 0 SCN 210 0 m 210 72 l S 0 0 0 1 K 300 0 m 300 72 l S {cercle}"
        );
        let contenu_id = doc.add_object(Stream::new(dictionary! {}, contenu.into_bytes()));
        let pages_id = doc.new_object_id();
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages_id, "MediaBox" => vec![0.into(), 0.into(), 600.into(), 600.into()],
            "Contents" => contenu_id, "Resources" => dictionary! { "ColorSpace" => espaces_id, "XObject" => dictionary! { "Fm0" => form_id } },
        });
        doc.objects.insert(pages_id, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page_id.into()], "Count" => 1 }));
        let catalogue = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        doc.trailer.set("Root", catalogue);
        let mut octets = vec![];
        doc.save_to(&mut octets).unwrap();
        octets
    }

    #[test]
    fn noms_de_tons() {
        assert!(est_ton_de_decoupe("CutContour"));
        assert!(est_ton_de_decoupe("cut contour"));
        assert!(est_ton_de_decoupe("Kiss-Cut"));
        assert!(!est_ton_de_decoupe("White"));
        assert!(!est_ton_de_decoupe("Silver"));
    }

    #[test]
    fn seuls_les_tons_de_decoupe_encres_sont_extraits() {
        let traces = extraire(&pdf_de_test(), false).unwrap().remove(0).traces;
        let tons: Vec<&str> = traces.iter().map(|t| t.ton.as_str()).collect();
        assert_eq!(tons, ["CutContour", "Thrucut", "cut contour"], "Silver, trait noir et Thrucut à 0 % ignorés");

        // Formulaire : Matrix (rotation de 90°, puis +100 en x) PUIS cm (+50 en y) ;
        // l'ordre compte, une rotation ne commute pas avec une translation.
        let carre = &traces[0];
        assert!(carre.ferme);
        assert_eq!(carre.points, [(100.0, 50.0), (100.0, 122.0), (28.0, 122.0), (28.0, 50.0)]);

        let trait_ = &traces[1];
        assert!(!trait_.ferme);
        assert_eq!(trait_.points, [(200.0, 0.0), (200.0, 72.0)]);

        let cercle = &traces[2];
        assert!(cercle.points.len() > 20, "courbes aplaties en {} points", cercle.points.len());
        for (x, y) in &cercle.points {
            let r = ((x - 400.0).powi(2) + (y - 300.0).powi(2)).sqrt();
            assert!((r - 50.0).abs() < 0.1, "point hors du cercle : rayon {r}");
        }
    }

    #[test]
    fn mode_tous_les_traces() {
        let d = extraire(&pdf_de_test(), true).unwrap().remove(0);
        // Carré, fond Silver, deux traits Thrucut (même à 0 %), trait noir, cercle.
        assert_eq!(d.traces.len(), 6);
        assert!(d.traces.iter().all(|t| t.ton == TOUS));
        assert_eq!((d.largeur, d.hauteur), (600.0, 600.0));
    }

    /// 200 × 100 mm (viewBox en mm), un calque `CutContour` avec un rectangle déplacé et
    /// un cercle dans un autre calque.
    const SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="200mm" height="100mm" viewBox="0 0 200 100">
        <g id="CutContour"><rect x="10" y="10" width="50" height="20" transform="translate(5 0)"/></g>
        <g id="Visuel"><circle cx="150" cy="50" r="25" fill="red"/></g>
    </svg>"#;

    #[test]
    fn svg_calque_de_decoupe() {
        let d = extraire_svg(SVG.as_bytes(), false).unwrap();
        let mm = |pt: f64| pt * 25.4 / 72.0;
        assert!((mm(d.largeur) - 200.0).abs() < 0.01 && (mm(d.hauteur) - 100.0).abs() < 0.01, "format {}×{}", mm(d.largeur), mm(d.hauteur));
        assert_eq!(d.traces.len(), 1, "seul le calque CutContour");
        let t = &d.traces[0];
        assert_eq!(t.ton, "CutContour");
        assert!(t.ferme);
        // Rectangle de 15 à 65 mm en x ; en y, 10–30 mm depuis le haut = 70–90 mm depuis le bas.
        let (xs, ys): (Vec<f64>, Vec<f64>) = t.points.iter().map(|(x, y)| (mm(*x), mm(*y))).unzip();
        let bornes = |v: &[f64]| (v.iter().cloned().fold(f64::MAX, f64::min), v.iter().cloned().fold(f64::MIN, f64::max));
        let ((x0, x1), (y0, y1)) = (bornes(&xs), bornes(&ys));
        for (obtenu, attendu) in [(x0, 15.0), (x1, 65.0), (y0, 70.0), (y1, 90.0)] {
            assert!((obtenu - attendu).abs() < 0.01, "{obtenu} au lieu de {attendu}");
        }
    }

    #[test]
    fn svg_tous_les_traces() {
        let d = extraire_svg(SVG.as_bytes(), true).unwrap();
        assert_eq!(d.traces.len(), 2);
        let cercle = &d.traces[1];
        let (cx, cy, r) = (150.0 * 72.0 / 25.4, 50.0 * 72.0 / 25.4, 25.0 * 72.0 / 25.4);
        for (x, y) in &cercle.points {
            assert!((((x - cx).powi(2) + (y - cy).powi(2)).sqrt() - r).abs() < 0.1);
        }
    }

    #[test]
    fn rognage_a_la_page() {
        let carre = |x0: f64, x1: f64| Trace { ton: "CutContour".into(), points: vec![(x0, 10.0), (x1, 10.0), (x1, 90.0), (x0, 90.0)], ferme: true };
        // Dedans : inchangé, reste fermé.
        assert_eq!(rogner(vec![carre(10.0, 50.0)], 100.0, 100.0), vec![carre(10.0, 50.0)]);
        // Dehors : disparaît.
        assert!(rogner(vec![carre(150.0, 190.0)], 100.0, 100.0).is_empty());
        // À cheval sur le bord droit : un seul tracé ouvert, raccordé, de (100,90) à (100,10).
        let r = rogner(vec![carre(50.0, 150.0)], 100.0, 100.0);
        assert_eq!(r.len(), 1, "{r:?}");
        assert!(!r[0].ferme);
        assert_eq!(r[0].points, [(100.0, 90.0), (50.0, 90.0), (50.0, 10.0), (100.0, 10.0)]);
    }

    #[test]
    fn hpgl_en_centiemes_de_millimetre() {
        let traces = vec![
            Trace { ton: "CutContour".into(), points: vec![(0.0, 0.0), (72.0, 0.0), (72.0, 72.0)], ferme: true },
            Trace { ton: "Creasing".into(), points: vec![(0.0, 0.0), (0.0, 72.0)], ferme: false },
        ];
        // 72 pt = 25,4 mm = 2540 unités.
        assert_eq!(hpgl(&traces, &[], &Reglages::default()), "IN;SP2;PU0,0;PD0,2540;SP1;PU0,0;PD2540,0,2540,2540,0,0;PU;SP0;", "rainage avant la coupe");
    }

    #[test]
    fn interieurs_d_abord_puis_le_plus_proche() {
        let carre = |nom: &str, x: f64, y: f64, c: f64| Trace { ton: nom.into(), points: vec![(x, y), (x + c, y), (x + c, y + c), (x, y + c)], ferme: true };
        let loin = carre("loin", 300.0, 0.0, 50.0);
        let exterieur = carre("exterieur", 0.0, 0.0, 100.0);
        let trou = carre("trou", 20.0, 20.0, 20.0);
        let lettre = carre("lettre", 120.0, 0.0, 30.0);
        let dans_le_trou = carre("dans_le_trou", 25.0, 25.0, 5.0);
        let traces = [&loin, &exterieur, &trou, &lettre, &dans_le_trou];
        let noms: Vec<&str> = ordonner(&traces).iter().map(|t| t.ton.as_str()).collect();
        // Profondeurs : dans_le_trou 2, trou 1, les autres 0 ; puis plus proche voisin depuis
        // le départ du trou (20, 20) : extérieur (0, 0), lettre (120, 0), loin (300, 0).
        assert_eq!(noms, ["dans_le_trou", "trou", "exterieur", "lettre", "loin"]);
    }

    /// Page cyan pleine, filet `CutContour` (1 pt) à 5 pt du bord, sans surimpression. À
    /// 72 dpi, rendu de l'original : le filet défonce le cyan. Rendu du PDF neutralisé :
    /// cyan intact sous le filet, aucune plaque de découpe ; les tracés restent extraits.
    #[test]
    fn ton_de_decoupe_neutralise_ne_defonce_plus() {
        let pdf = crate::imbrication::tests::pdf(144.0, 72.0, "1 0 0 0", (0.0, 0.0));
        let (neutre, n) = neutraliser(&pdf).unwrap();
        assert_eq!(n, 1);
        assert_eq!(neutraliser(&neutre).unwrap().1, 0, "rien de plus à neutraliser");
        assert_eq!(extraire(&pdf, false).unwrap()[0].traces.len(), 1, "la découpe se lit sur l'original");
        let d = std::env::temp_dir().join(format!("eko-rip-neutre-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let rendre = |nom: &str, octets: &[u8]| {
            std::fs::write(d.join(format!("{nom}.pdf")), octets).unwrap();
            let st = std::process::Command::new("gs")
                .args(["-q", "-dSAFER", "-dBATCH", "-dNOPAUSE", "-sDEVICE=tiffsep", "-r72"])
                .arg(format!("-sOutputFile={}", d.join(format!("{nom}.tif")).display()))
                .arg(d.join(format!("{nom}.pdf")))
                .status()
                .unwrap();
            assert!(st.success());
        };
        rendre("orig", &pdf);
        rendre("neutre", &neutre);
        // Pixel sur le filet (x = 5 pt, mi-hauteur) et au milieu de la page.
        let cyan = |nom: &str, x: u32| image::open(d.join(format!("{nom}(Cyan).tif"))).unwrap().to_luma8().get_pixel(x, 36)[0];
        assert!(cyan("orig", 5) > 200 && cyan("orig", 72) == 0, "l'original défonce bien le cyan sous le filet");
        assert_eq!((cyan("neutre", 5), cyan("neutre", 72)), (0, 0), "cyan intact sous le filet");
        assert!(d.join("orig(CutContour).tif").exists());
        assert!(!d.join("neutre(CutContour).tif").exists(), "plus de plaque de découpe");
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn prefixes_lus_en_debut_de_nom() {
        let r = Reglages::default();
        assert_eq!(r.prefixes, ["CutContour", "Thrucut", "Kisscut", "InsideCut", "Perforation", "Creasing", "Score", "VCut"], "la liste d'avant reste le défaut");
        for oui in ["CutContour", "CutContour1", "CutContourKissCut", "cut contour", "Kiss-Cut", "Thrucut Épais"] {
            assert!(r.reconnait(oui), "{oui}");
        }
        for non in ["1CutContour", "MyCutContour", "White", "Silver"] {
            assert!(!r.reconnait(non), "{non}");
        }
        let die = Reglages { prefixes: vec!["Die".into()], ..r.clone() };
        assert!(die.reconnait("Die-Cut 3") && !die.reconnait("CutContour"));
        assert!(die.verifier().is_ok());
        assert_eq!(Reglages { prefixes: vec![], ..r.clone() }.verifier(), Err("prefixes_invalides"));
        assert_eq!(Reglages { prefixes: vec!["--".into()], ..r }.verifier(), Err("prefixes_invalides"), "un préfixe sans lettre reconnaîtrait tout");
    }

    fn carre(ton: &str, x: f64) -> Trace {
        Trace { ton: ton.into(), points: vec![(x, 0.0), (x + 72.0, 0.0), (x + 72.0, 72.0), (x, 72.0)], ferme: true }
    }

    #[test]
    fn une_couche_par_ton_dans_l_ordre_de_coupe() {
        let traces = [carre("Thrucut", 0.0), carre("CutContour", 100.0), carre("Kisscut", 200.0), carre("kiss cut", 300.0)];
        let mut couches = vec![];
        accorder(&mut couches, &tons(&traces));
        let noms: Vec<&str> = couches.iter().map(|c| c.nom.as_str()).collect();
        assert_eq!(noms, ["Kisscut", "CutContour", "Thrucut"], "mi-chair avant contour, coupe à travers en dernier ; « kiss cut » = Kisscut");
        // 72 pt = 2540 unités : chaque bloc commence par son outil, puis ses tracés.
        let depart = |h: &str, x: i64| h.find(&format!("PU{x},0;")).unwrap_or_else(|| panic!("{x} absent de {h}"));
        let h = hpgl(&traces, &couches, &Reglages::default());
        assert!(h.starts_with("IN;SP1;") && h.ends_with("PU;SP0;"));
        assert_eq!(h.matches("SP1;").count(), 3, "un bloc par couche : {h}");
        assert!(depart(&h, 7056) < depart(&h, 3528) && depart(&h, 3528) < depart(&h, 0), "Kisscut, CutContour, Thrucut : {h}");
        assert!(depart(&h, 7056) < depart(&h, 10583), "les deux Kisscut ensemble");
        // Ordre inversé : HP-GL inversé.
        couches.reverse();
        let inverse = hpgl(&traces, &couches, &Reglages::default());
        assert!(depart(&inverse, 0) < depart(&inverse, 3528) && depart(&inverse, 3528) < depart(&inverse, 7056), "{inverse}");
        // Couche décochée : absente.
        couches[0].active = false;
        assert!(!hpgl(&traces, &couches, &Reglages::default()).contains("PU0,0;"));
        // Un nouveau ton au rendu suivant prend son rang sans déranger l'ordre choisi.
        accorder(&mut couches, &["Creasing"]);
        assert_eq!(couches[0].nom, "Creasing");
        assert_eq!(couches.len(), 4);
        assert!(!couches[1].active, "les choix déjà faits restent");
    }

    /// PDF d'une page 600 × 600 avec `contenu` ; `/CS0` = CutContour (en RVB si `rvb`),
    /// `/MC0` = calque non imprimable (`/Usage /Print /PrintState /OFF`).
    fn pdf_controle(contenu: &str, rvb: bool) -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let ton = if rvb {
            Object::Array(vec!["Separation".into(), "CutContour".into(), "DeviceRGB".into(), Object::Dictionary(dictionary! { "FunctionType" => 2, "Domain" => vec![0.into(), 1.into()], "C0" => vec![0.into(); 3], "C1" => vec![1.into(), 0.into(), 1.into()], "N" => 1 })])
        } else {
            separation("CutContour")
        };
        let calque = doc.add_object(dictionary! { "Type" => "OCG", "Name" => Object::string_literal("Découpe masquée"), "Usage" => dictionary! { "Print" => dictionary! { "PrintState" => "OFF" } } });
        let contenu_id = doc.add_object(Stream::new(dictionary! {}, contenu.as_bytes().to_vec()));
        let pages_id = doc.new_object_id();
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages_id, "MediaBox" => vec![0.into(), 0.into(), 600.into(), 600.into()],
            "Contents" => contenu_id, "Resources" => dictionary! { "ColorSpace" => dictionary! { "CS0" => ton }, "Properties" => dictionary! { "MC0" => calque } },
        });
        doc.objects.insert(pages_id, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page_id.into()], "Count" => 1 }));
        let catalogue = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id, "OCProperties" => dictionary! { "OCGs" => vec![calque.into()], "D" => dictionary! {} } });
        doc.trailer.set("Root", catalogue);
        let mut octets = vec![];
        doc.save_to(&mut octets).unwrap();
        octets
    }

    fn codes(pdf: &[u8], r: &Reglages) -> (Vec<&'static str>, usize) {
        let d = extraire_avec(pdf, false, r).unwrap().remove(0);
        (d.signaux.iter().map(|(c, _)| *c).collect(), d.traces.len())
    }

    #[test]
    fn controle_prealable_une_alerte_par_defaut() {
        let r = Reglages::default();
        // Propre : l'encre est peinte avant (dessous), le filet seul, loin des bords.
        let propre = "0 0 1 0 k 50 50 400 400 re f /CS0 CS 1 SCN 100 100 200 200 re S";
        assert_eq!(codes(&pdf_controle(propre, false), &r), (vec![], 1), "un fichier propre : aucune alerte");
        let cas = [
            ("decoupe_ton_rvb", propre, true),
            ("decoupe_avec_fond", "1 g /CS0 CS 1 SCN 100 100 200 200 re B", false),
            ("decoupe_touche_bord", "/CS0 CS 1 SCN 0 100 200 200 re S", false),
            ("decoupe_pas_au_premier_plan", "/CS0 CS 1 SCN 100 100 200 200 re S 0 0 1 0 k 150 150 50 50 re f", false),
            ("decoupe_contour_vectorise", "/CS0 cs 1 scn 100 100 200 200 re 101 101 198 198 re f*", false),
            ("decoupe_calque_non_imprimable", "/OC /MC0 BDC /CS0 CS 1 SCN 100 100 200 200 re S EMC", false),
            ("decoupe_trait_non_centre", "q 100 100 200 200 re W n /CS0 CS 1 SCN 2 w 100 100 200 200 re S Q", false),
            ("decoupe_hors_page", "/CS0 CS 1 SCN 500 100 200 200 re S", false),
        ];
        for (code, contenu, rvb) in cas {
            assert_eq!(codes(&pdf_controle(contenu, rvb), &r).0, [code], "{contenu}");
        }
        let d = extraire_avec(&pdf_controle(cas[2].1, false), false, &r).unwrap().remove(0);
        assert_eq!(d.signaux[0].1, "CutContour", "le détail nomme le ton");
        // Calque non imprimable : ignoré par défaut ; avec l'option, coupé sans alerte.
        assert_eq!(codes(&pdf_controle(cas[5].1, false), &r).1, 0);
        let prendre = Reglages { calques_non_imprimables: true, ..r.clone() };
        assert_eq!(codes(&pdf_controle(cas[5].1, false), &prendre), (vec![], 1));
        // Un calque visible (sans état d'impression, pas éteint) n'est pas masqué.
        let visible = pdf_controle("/OC /MC0 BDC /CS0 CS 1 SCN 100 100 200 200 re S EMC", false);
        let mut doc = Document::load_mem(&visible).unwrap();
        for o in doc.objects.values_mut() {
            if let Ok(d) = o.as_dict_mut()
                && d.get(b"Type").and_then(Object::as_name).ok() == Some(b"OCG".as_slice())
            {
                d.remove(b"Usage");
            }
        }
        let mut octets = vec![];
        doc.save_to(&mut octets).unwrap();
        assert_eq!(codes(&octets, &r), (vec![], 1));
        // Mode « tous les tracés » : pas de contrôle des tons.
        assert!(extraire_avec(&pdf_controle(cas[1].1, false), true, &r).unwrap()[0].signaux.is_empty());
    }

    /// 4.11 : tracés ouverts tolérés par défaut, sinon absents du HP-GL ; tri désactivé :
    /// l'ordre de création, même pour un trou créé après son contour.
    #[test]
    fn traces_ouverts_et_tri_desactivable() {
        let carre = |x: f64, c: f64| Trace { ton: "CutContour".into(), points: vec![(x, x), (x + c, x), (x + c, x + c), (x, x + c)], ferme: true };
        let event = Trace { ton: "CutContour".into(), points: vec![(500.0, 0.0), (500.0, 72.0)], ferme: false };
        let traces = [carre(0.0, 288.0), carre(72.0, 72.0), event];
        let r = Reglages::default();
        assert!(r.traces_ouverts && r.trier, "actifs par défaut");
        let ouvert = "PU17639,0;PD17639,2540;";
        let h = hpgl(&traces, &[], &r);
        assert!(h.contains(ouvert), "tracé ouvert coupé : {h}");
        // Trié : le trou (en 72 pt = 2540) avant le contour qui le contient (en 0).
        assert!(h.find("PU2540,2540;").unwrap() < h.find("PU0,0;").unwrap(), "{h}");
        let h = hpgl(&traces, &[], &Reglages { traces_ouverts: false, ..r.clone() });
        assert!(!h.contains("PU17639"), "tracé ouvert non toléré : {h}");
        assert_eq!(h.matches("PU").count(), 3, "deux contours fermés et le PU de fin : {h}");
        let h = hpgl(&traces, &[], &Reglages { trier: false, ..r.clone() });
        assert!(h.find("PU0,0;").unwrap() < h.find("PU2540,2540;").unwrap() && h.find("PU2540,2540;").unwrap() < h.find(ouvert).unwrap(), "ordre de création : {h}");
        // decoupe.json d'avant ces options : les deux restent actives.
        let ancien: Reglages = serde_json::from_str(r#"{"prefixes": ["CutContour"]}"#).unwrap();
        assert!(ancien.traces_ouverts && ancien.trier && !ancien.calques_non_imprimables);
    }

    #[test]
    fn hpgl_d_avant_les_couches_relu_a_l_identique() {
        let ancien = "IN;SP2;PU0,0;PD0,2540;SP1;PU0,0;PD2540,0,2540,2540,0,0;PU5000,0;PD6000,0;PU;SP0;";
        let traces = depuis_hpgl(ancien);
        assert_eq!(traces.iter().map(|t| (t.ton.as_str(), t.ferme)).collect::<Vec<_>>(), [("Creasing", false), ("CutContour", true), ("CutContour", false)]);
        assert_eq!(hpgl(&traces, &[], &Reglages::default()), ancien, "mêmes coordonnées, même ordre");
    }

    #[test]
    fn ancien_reglage_de_coupe_porte_sur_une_couche() {
        let couche = |nom: &str| Couche { nom: nom.into(), active: true, reglage: None };
        let mut une = vec![couche("CutContour")];
        assert!(reprendre_ancien_reglage(&mut une, "Mi-chair V3000"));
        assert_eq!(une[0].reglage.as_deref(), Some("Mi-chair V3000"), "seule couche");
        let mut deux = vec![couche("Kisscut"), couche("Thrucut")];
        assert!(reprendre_ancien_reglage(&mut deux, "Thru-cut"));
        assert_eq!((deux[0].reglage.as_deref(), deux[1].reglage.as_deref()), (None, Some("Thru-cut")), "couche du même nom");
        let mut ambigu = vec![couche("Kisscut"), couche("Thrucut")];
        assert!(!reprendre_ancien_reglage(&mut ambigu, "Mi-chair V3000"), "deux couches, aucun nom commun : rien n'est deviné");
        assert!(ambigu.iter().all(|c| c.reglage.is_none()));
    }
}
