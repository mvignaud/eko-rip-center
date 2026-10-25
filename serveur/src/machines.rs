//! Catalogue des machines de l'atelier (`<données>/machines.json`) : modèle, laize,
//! connexion. Sans fichier, le catalogue d'origine : l'Epson SC-S80600 et le Mutoh Kona.
//! Chaque modèle sait imprimer, découper, ou les deux (Roland VersaCAMM).

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Modele {
    /// Epson SureColor SC-S80600 (impression : pilote direct par le réseau, port 9100, ou
    /// plaques copiées en dossier).
    EpsonScS80600,
    /// Mutoh Kona (découpe, HP-GL à 0,01 mm).
    MutohKona,
    /// Graphtec FC7000 (découpe).
    GraphtecFc7000,
    /// Roland VersaCAMM VP-300 / VP-540 (impression et découpe par la machine).
    RolandVersacamm,
}

impl Modele {
    pub fn imprime(self) -> bool {
        matches!(self, Modele::EpsonScS80600 | Modele::RolandVersacamm)
    }

    pub fn decoupe(self) -> bool {
        !matches!(self, Modele::EpsonScS80600)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Impression,
    Decoupe,
}

/// Comment joindre la machine.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Connexion {
    /// Port série (adaptateur USB-série compris) réglé par `stty`.
    Serie { port: PathBuf, stty: String },
    /// Flux brut sur le réseau (TCP, port 9100 en général).
    Reseau { hote: String, port: u16 },
    /// Périphérique ou fichier écrit tel quel (`/dev/usb/lp0`…).
    Fichier { chemin: PathBuf },
    /// Pas de pilote : la sortie est copiée dans `<données>/sorties/<machine>/`.
    Dossier,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Machine {
    /// Identifiant stable (lettres, chiffres, `-`) : file d'attente, dossiers de sortie.
    pub id: String,
    /// Identifiant immuable pour les échanges (ERP) ; attribué par le serveur.
    #[serde(default)]
    pub uuid: Option<uuid::Uuid>,
    pub nom: String,
    pub modele: Modele,
    /// Laize utile, en mm.
    pub laize_mm: f64,
    pub connexion: Connexion,
    /// Adresse SNMP pour lire le média chargé (Printer-MIB).
    #[serde(default)]
    pub snmp: Option<String>,
    /// Pas de la découpe en HP-GL, en mm (par défaut : 0,01 pour le Kona, 0,025 sinon).
    #[serde(default)]
    pub pas_mm: Option<f64>,
    /// Décalage de la découpe (x, y) en mm, ±20 : rattrape l'écart entre le couteau et
    /// l'impression mesuré sur une coupe d'essai.
    #[serde(default)]
    pub decalage_mm: [f64; 2],
    /// Pilote direct de l'Epson : position (x, y) en mm de l'image sur la page envoyée.
    /// x depuis le bord de référence de la laize (arrondi aux 16 points sous la valeur,
    /// alignement du flux), y après la marge haute de 5 mm du flux. (0, 0) par défaut : à
    /// régler sur l'essai papier (la capture d'un autre RIP plaçait une page étroite vers
    /// x ≈ 636 mm, sans qu'on sache encore d'où se compte cette position).
    #[serde(default)]
    pub origine_impression_mm: [f64; 2],
    /// Cadence d'impression en m²/h, sur toute la laize (sinon : tableau de l'Epson).
    #[serde(default)]
    pub m2_heure: Option<f64>,
    /// Réglages de coupe par support (le premier sert par défaut), comme les « couches de
    /// découpe » de CutServer.
    #[serde(default)]
    pub parametres_decoupe: Vec<ParametreDecoupe>,
    /// Kona avec cadre : feuille chargée dans l'autre sens (découpe tournée de 180°).
    #[serde(default)]
    pub decoupe_retournee: bool,
    /// Pool (roadmap 1.7) : une file commune à des machines identiques (même modèle, même
    /// laize) ; une sortie envoyée au pool part sur la première machine prête.
    #[serde(default)]
    pub pool: Option<String>,
    /// Découpeuse sans changement d'outil ni de réglage en cours de coupe (roadmap 4.3) :
    /// une sortie par couche, la suivante retenue jusqu'à ce que l'opérateur la libère.
    #[serde(default)]
    pub mono_couche: bool,
    /// Couper dès l'arrivée en file (roadmap 4.12, actif par défaut : comportement d'avant) ;
    /// sinon chaque découpe arrive suspendue et part quand l'opérateur la reprend.
    #[serde(default = "vrai")]
    pub coupe_auto: bool,
    /// Variante d'une autre machine (roadmap 4.15) : la même machine physique, son propre
    /// jeu de repères. Connexion, modèle et réglages sont repris de la machine d'origine
    /// (`accorder_variantes`) ; ses sorties passent par elle, l'une après l'autre.
    #[serde(default)]
    pub variante_de: Option<String>,
    /// Jeu de repères imposé aux travaux affectés à cette découpeuse.
    #[serde(default)]
    pub reperage: Option<crate::reperage::Reperage>,
    /// Imbrication des impressions en file (roadmap 5.1 à 5.5) ; un par un par défaut.
    #[serde(default)]
    pub imbrication: crate::nid::Reglage,
}

fn vrai() -> bool {
    true
}

/// Couches de découpe de CutServer à l'atelier (relevées le 24/10), reprises pour le Kona.
fn couches_cutserver() -> Vec<ParametreDecoupe> {
    let couche = |nom: &str, force_g| ParametreDecoupe { uuid: None, nom: nom.into(), vitesse_cm_s: 15.0, force_g, acceleration_g: 1.0, valeurs_machine: false, outil: None, motif_mm: vec![], diffusion: false };
    vec![couche("Mi-chair V3000", 145.0), couche("V3000WG - Laminé", 305.0)]
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParametreDecoupe {
    #[serde(default)]
    pub uuid: Option<uuid::Uuid>,
    pub nom: String,
    pub vitesse_cm_s: f64,
    pub force_g: f64,
    pub acceleration_g: f64,
    /// « Utiliser les valeurs par défaut du massicot » : ni vitesse, ni force, ni accélération.
    #[serde(default)]
    pub valeurs_machine: bool,
    /// Outil (`SPn`, 1 à 4) ; sinon celui du ton de découpe.
    #[serde(default)]
    pub outil: Option<u8>,
    /// Motif de découpe (comme CutServer) : jusqu'à 10 paires (coupé, relevé) en mm.
    #[serde(default)]
    pub motif_mm: Vec<[f64; 2]>,
    /// Diffusion du motif : un nombre entier de motifs par tronçon droit, coupé aux angles.
    #[serde(default)]
    pub diffusion: bool,
}

impl ParametreDecoupe {
    fn valide(&self) -> bool {
        !self.nom.trim().is_empty()
            && self.nom.chars().count() <= 40
            && (1.0..=100.0).contains(&self.vitesse_cm_s)
            && (1.0..=600.0).contains(&self.force_g)
            && (0.1..=10.0).contains(&self.acceleration_g)
            && self.outil.is_none_or(|o| (1..=4).contains(&o))
            && self.motif_mm.len() <= 10
            && self.motif_mm.iter().flatten().all(|v| (0.0..=500.0).contains(v))
            && (self.motif_mm.is_empty() || self.motif_mm.iter().any(|[b, _]| *b > 0.0))
    }
}

/// Motif de découpe sur le HP-GL : chaque tracé (un `PU` puis ses `PD`) est coupé par
/// tronçons, outil bas puis haut, selon `motif` (paires coupé/relevé, unités du HP-GL).
/// `diffusion` : le tracé est coupé en tronçons aux angles vifs (> 30°), et le motif de
/// chaque tronçon est étiré pour en faire un nombre entier : la coupe tombe aux angles.
pub fn pointiller(hpgl: &str, motif: &[[f64; 2]], diffusion: bool) -> String {
    let pas: Vec<f64> = motif.iter().flatten().copied().collect();
    let periode: f64 = pas.iter().sum();
    if periode <= 0.0 {
        return hpgl.to_string();
    }
    let mut sortie = String::new();
    let mut trace: Vec<(f64, f64)> = vec![];
    let vider = |trace: &mut Vec<(f64, f64)>, sortie: &mut String| {
        for troncon in troncons(trace, diffusion) {
            let l: f64 = troncon.windows(2).map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1)).sum();
            let echelle = if diffusion { l / ((l / periode).round().max(1.0) * periode) } else { 1.0 };
            // Position dans le motif : à zéro au début de chaque tronçon (outil bas).
            let (mut k, mut dans) = (0usize, 0.0f64);
            for w in troncon.windows(2) {
                let (a, b) = (w[0], w[1]);
                let lg = (b.0 - a.0).hypot(b.1 - a.1);
                let mut fait = 0.0;
                while fait < lg - 1e-9 {
                    let reste = pas[k] * echelle - dans;
                    let avance = reste.min(lg - fait);
                    fait += avance;
                    dans += avance;
                    let q = (a.0 + (b.0 - a.0) * fait / lg, a.1 + (b.1 - a.1) * fait / lg);
                    *sortie += &format!("{}{},{};", if k % 2 == 0 { "PD" } else { "PU" }, q.0.round() as i64, q.1.round() as i64);
                    if dans >= pas[k] * echelle - 1e-9 {
                        (k, dans) = ((k + 1) % pas.len(), 0.0);
                    }
                }
            }
        }
        trace.clear();
    };
    for cmd in hpgl.split(';').map(str::trim).filter(|c| !c.is_empty()) {
        let points: Vec<(f64, f64)> = match cmd.get(..2) {
            Some("PU" | "PD") if cmd.len() > 2 => cmd[2..].split(',').filter_map(|n| n.trim().parse().ok()).collect::<Vec<f64>>().as_chunks::<2>().0.iter().map(|&[x, y]| (x, y)).collect(),
            _ => vec![],
        };
        if cmd.starts_with("PD") && !points.is_empty() {
            trace.extend(points);
            continue;
        }
        vider(&mut trace, &mut sortie);
        sortie += cmd;
        sortie += ";";
        if let Some(&p) = points.last() {
            trace.push(p);
        }
    }
    vider(&mut trace, &mut sortie);
    sortie
}

/// Tronçons d'un tracé : entier, ou coupé aux angles vifs (> 30°) avec la diffusion.
fn troncons(trace: &[(f64, f64)], diffusion: bool) -> Vec<Vec<(f64, f64)>> {
    if trace.len() < 2 {
        return vec![];
    }
    let mut sortie = vec![vec![trace[0]]];
    for i in 1..trace.len() {
        sortie.last_mut().unwrap().push(trace[i]);
        if diffusion && i + 1 < trace.len() {
            let (a, b, c) = (trace[i - 1], trace[i], trace[i + 1]);
            let (u, v) = ((b.0 - a.0, b.1 - a.1), (c.0 - b.0, c.1 - b.1));
            let cos = (u.0 * v.0 + u.1 * v.1) / (u.0.hypot(u.1) * v.0.hypot(v.1)).max(1e-12);
            if cos < 30f64.to_radians().cos() {
                sortie.push(vec![trace[i]]);
            }
        }
    }
    sortie
}

impl Machine {
    /// Réglage de coupe d'une couche : celui choisi à la main, sinon celui qui porte le nom
    /// de la couche (sans casse, espaces ni tirets), sinon le premier (défaut de la
    /// machine). `Err` : le réglage choisi n'existe pas sur cette machine.
    pub fn reglage_couche(&self, c: &crate::decoupe::Couche) -> Result<Option<&ParametreDecoupe>, String> {
        match &c.reglage {
            Some(nom) => self.parametres_decoupe.iter().find(|p| &p.nom == nom).map(Some).ok_or_else(|| nom.clone()),
            None => Ok(self.parametres_decoupe.iter().find(|p| crate::decoupe::meme_couche(&p.nom, &c.nom)).or(self.parametres_decoupe.first())),
        }
    }

    /// Connexion utilisable : port série présent, hôte et port renseignés, dossier du
    /// fichier présent. `Err` : la raison, non traduite.
    pub fn joignable(&self) -> Result<(), String> {
        match &self.connexion {
            Connexion::Serie { port, .. } if !port.exists() => Err(format!("port {} absent", port.display())),
            Connexion::Reseau { hote, port } if hote.trim().is_empty() || *port == 0 => Err("adresse réseau incomplète".into()),
            Connexion::Fichier { chemin } if !chemin.parent().is_some_and(Path::exists) && !chemin.exists() => Err(format!("{} introuvable", chemin.display())),
            _ => Ok(()),
        }
    }

    /// Minutes pour imprimer `longueur_mm` de rouleau à `dpi` ; `None` sans cadence connue.
    /// Epson SC-S80600 (fiche publique) : 360 dpi 94 m²/h (1 passe), 720 dpi 17 m²/h
    /// (720 × 1440, 6 passes), au-delà 6,6 m²/h (1440 × 1440, 16 passes).
    pub fn duree_impression_min(&self, longueur_mm: f64, dpi: u32) -> Option<f64> {
        let cadence = self.m2_heure.or_else(|| (self.modele == Modele::EpsonScS80600).then_some(if dpi <= 360 { 94.0 } else if dpi <= 720 { 17.0 } else { 6.6 }))?;
        (self.modele.imprime() && longueur_mm > 0.0).then(|| self.laize_mm * longueur_mm / 1e6 / cadence * 60.0)
    }

    pub fn pas_mm(&self) -> f64 {
        self.pas_mm.unwrap_or(if self.modele == Modele::MutohKona { 0.01 } else { 0.025 })
    }

    pub fn fait(&self, o: Operation) -> bool {
        match o {
            Operation::Impression => self.modele.imprime(),
            Operation::Decoupe => self.modele.decoupe(),
        }
    }
}

pub fn fichier(donnees: &Path) -> PathBuf {
    donnees.join("machines.json")
}

/// Catalogue d'origine : l'Epson (sorties en dossier) et le Kona sur son port série.
pub fn catalogue_initial(port_kona: &Path, stty: &str, snmp_epson: Option<String>) -> Vec<Machine> {
    vec![
        Machine { id: "epson".into(), uuid: None, nom: "Epson SC-S80600".into(), modele: Modele::EpsonScS80600, laize_mm: 1626.0, connexion: Connexion::Dossier, snmp: snmp_epson, pas_mm: None, decalage_mm: [0.0; 2], origine_impression_mm: [0.0; 2], m2_heure: None, parametres_decoupe: vec![], decoupe_retournee: false, pool: None, mono_couche: false, coupe_auto: true, variante_de: None, reperage: None, imbrication: Default::default() },
        Machine { id: "kona".into(), uuid: None, nom: "Mutoh Kona 1650".into(), modele: Modele::MutohKona, laize_mm: 1650.0, connexion: Connexion::Serie { port: port_kona.into(), stty: stty.into() }, snmp: None, pas_mm: None, decalage_mm: [0.0; 2], origine_impression_mm: [0.0; 2], m2_heure: None, parametres_decoupe: couches_cutserver(), decoupe_retournee: false, pool: None, mono_couche: false, coupe_auto: true, variante_de: None, reperage: None, imbrication: Default::default() },
    ]
}

pub fn lire(donnees: &Path) -> Option<Vec<Machine>> {
    crate::etat::lire(&fichier(donnees)).ok().flatten()
}

pub fn ecrire(donnees: &Path, machines: &[Machine]) -> std::io::Result<()> {
    crate::etat::ecrire(&fichier(donnees), machines, false)
}

/// Identifiants immuables : repris de la machine connue de même `uuid`, sinon de même `id`
/// texte (le client peut ne pas les renvoyer), sinon nouveaux ; de même pour les couches de
/// découpe (par nom dans la machine). Un `uuid` inconnu est refusé : on ne le choisit pas.
pub fn identifier(machines: &mut [Machine], connues: &[Machine]) -> Result<(), &'static str> {
    for m in machines.iter_mut() {
        let avant = match m.uuid {
            Some(u) => Some(connues.iter().find(|c| c.uuid == Some(u)).ok_or("identifiant_inconnu")?),
            None => connues.iter().find(|c| c.id == m.id),
        };
        m.uuid = Some(avant.and_then(|c| c.uuid).unwrap_or_else(uuid::Uuid::new_v4));
        let couches_avant = avant.map(|c| c.parametres_decoupe.as_slice()).unwrap_or_default();
        for p in m.parametres_decoupe.iter_mut() {
            let ancien = match p.uuid {
                Some(u) => Some(couches_avant.iter().find(|c| c.uuid == Some(u)).ok_or("identifiant_inconnu")?),
                None => couches_avant.iter().find(|c| c.nom == p.nom),
            };
            p.uuid = Some(ancien.and_then(|c| c.uuid).unwrap_or_else(uuid::Uuid::new_v4));
        }
    }
    Ok(())
}

pub fn verifier(machines: &[Machine]) -> Result<(), &'static str> {
    let id_valide = |id: &str| (1..=30).contains(&id.len()) && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    let bon = machines.iter().enumerate().all(|(i, m)| {
        id_valide(&m.id)
            && !machines[..i].iter().any(|x| x.id == m.id)
            && !m.nom.trim().is_empty()
            && (100.0..=5000.0).contains(&m.laize_mm)
            && m.pas_mm.is_none_or(|p| [0.005, 0.01, 0.025, 0.05, 0.1].iter().any(|x| (x - p).abs() < 1e-9))
            && m.decalage_mm.iter().all(|d| (-20.0..=20.0).contains(d))
            && (0.0..m.laize_mm).contains(&m.origine_impression_mm[0])
            && (0.0..=1000.0).contains(&m.origine_impression_mm[1])
            && m.m2_heure.is_none_or(|v| (0.1..=500.0).contains(&v))
            && m.parametres_decoupe.len() <= 20
            && m.parametres_decoupe.iter().all(ParametreDecoupe::valide)
            && m.imbrication.valide()
    });
    if !bon {
        return Err("machine_invalide");
    }
    // Pool : nom au format d'un identifiant, distinct de toute machine, membres identiques.
    let pool_bon = machines.iter().filter_map(|m| m.pool.as_deref().map(|p| (m, p))).all(|(m, p)| {
        id_valide(p) && !machines.iter().any(|x| x.id == p) && machines.iter().filter(|x| x.pool.as_deref() == Some(p)).all(|x| x.modele == m.modele && (x.laize_mm - m.laize_mm).abs() < 0.5)
    });
    if !pool_bon {
        return Err("pool_incoherent");
    }
    // Variante : d'une machine du catalogue qui n'en est pas une elle-même, qui découpe.
    let variantes_bonnes = machines.iter().filter_map(|m| m.variante_de.as_deref().map(|b| (m, b))).all(|(m, b)| b != m.id && machines.iter().any(|x| x.id == b && x.variante_de.is_none() && x.modele.decoupe()));
    if variantes_bonnes { Ok(()) } else { Err("variante_invalide") }
}

/// Chaque variante reprend de sa machine d'origine ce qui fait la machine physique :
/// modèle, laize, connexion, pas, décalages, réglages de coupe (sous ses propres
/// identifiants), retournement, mono-couche, coupe à l'arrivée. Elle garde son identifiant,
/// son nom et son jeu de repères ; une variante n'est d'aucun pool.
pub fn accorder_variantes(machines: &mut [Machine]) {
    let originaux: Vec<Machine> = machines.to_vec();
    for m in machines.iter_mut() {
        let Some(base) = m.variante_de.as_ref().and_then(|b| originaux.iter().find(|x| &x.id == b && x.variante_de.is_none())) else { continue };
        let reglages = base.parametres_decoupe.iter().map(|p| ParametreDecoupe { uuid: m.parametres_decoupe.iter().find(|q| q.nom == p.nom).and_then(|q| q.uuid), ..p.clone() }).collect();
        *m = Machine { id: m.id.clone(), uuid: m.uuid, nom: m.nom.clone(), variante_de: m.variante_de.clone(), reperage: m.reperage, pool: None, parametres_decoupe: reglages, snmp: None, ..base.clone() };
    }
}

/// Variantes d'une machine (identifiants).
pub fn variantes<'a>(machines: &'a [Machine], id: &str) -> Vec<&'a str> {
    machines.iter().filter(|m| m.variante_de.as_deref() == Some(id)).map(|m| m.id.as_str()).collect()
}

/// Machines d'un pool.
pub fn membres<'a>(machines: &'a [Machine], pool: &str) -> Vec<&'a Machine> {
    machines.iter().filter(|m| m.pool.as_deref() == Some(pool)).collect()
}

/// Convertit le HP-GL produit par le RIP (pas de 0,01 mm, commandes `PU`/`PD` à coordonnées
/// x,y absolues) au pas `pas_mm` de la machine, décalé de `decalage_mm` (x, y).
pub fn changer_de_pas(hpgl: &str, pas_mm: f64, decalage_mm: [f64; 2]) -> String {
    let f = 0.01 / pas_mm;
    if (f - 1.0).abs() < 1e-12 && decalage_mm == [0.0; 2] {
        return hpgl.to_string();
    }
    hpgl.split(';')
        .map(|cmd| match cmd.get(..2) {
            Some(op @ ("PU" | "PD")) if cmd.len() > 2 => {
                let nombres: Vec<String> = cmd[2..]
                    .split(',')
                    .enumerate()
                    .map(|(i, n)| n.trim().parse::<f64>().map_or_else(|_| n.to_string(), |v| format!("{}", ((v + decalage_mm[i % 2] * 100.0) * f).round() as i64)))
                    .collect();
                format!("{op}{}", nombres.join(","))
            }
            _ => cmd.to_string(),
        })
        .collect::<Vec<_>>()
        .join(";")
}

/// Découpe pour le Mutoh Kona, telle que CutServer l'envoie (capture USB du 24/10) : une
/// commande par ligne (`;\r\n`), `PROGSTEP10` (pas de 0,01 mm), réglages de coupe
/// `AS`/`ZF`/`VS` après chaque `SP`, et avec un cadre la lecture automatique `AL5` : coins
/// opposés du cadre, coupe, retour au premier coin, `AL5,2`. `parties` : le HP-GL de chaque
/// couche (au pas de 0,01 mm) avec son réglage, dans l'ordre de coupe ; `cadre` : coins au
/// milieu des traits (x1, y1, x2, y2) dans les mêmes unités.
///
/// Axes du Kona (essais au point 0) : X suit la longueur de la feuille, Y+ vers la gauche,
/// repère direct ; d'où X = y de la page, Y = −x (comme ONYX). `retourne` : 180° de plus.
pub fn pour_kona(parties: &[(String, Option<&ParametreDecoupe>)], cadre: Option<[i64; 4]>, retourne: bool) -> String {
    let s = if retourne { -1 } else { 1 };
    let axes = |x: i64, y: i64| (s * y, -s * x);
    let mut corps = String::new();
    for (hpgl, p) in parties {
        // Écrit comme CutServer : « AS 1.0;ZF 145;VS 15; » ; rien avec les valeurs du massicot.
        let reglages = p.filter(|p| !p.valeurs_machine).map(|p| format!("AS {:.1};ZF {};VS {};LT;\r\n", p.acceleration_g, p.force_g, p.vitesse_cm_s)).unwrap_or_default();
        let outil = p.and_then(|p| p.outil);
        for cmd in hpgl.split(';').map(str::trim).filter(|c| !c.is_empty()) {
            match cmd.get(..2) {
                Some(op @ ("PU" | "PD")) if cmd.len() > 2 => {
                    let v: Vec<i64> = cmd[2..].split(',').filter_map(|n| n.trim().parse().ok()).collect();
                    // Une commande par point, comme CutServer.
                    for &[x, y] in v.as_chunks::<2>().0 {
                        let (x, y) = axes(x, y);
                        corps += &format!("{op}{x},{y};\r\n");
                    }
                }
                Some("SP") if cmd != "SP0" => corps += &format!("{};\r\n{reglages}", outil.map_or(cmd.to_string(), |o| format!("SP{o}"))),
                _ => {}
            }
        }
    }
    match cadre {
        Some([x1, y1, x2, y2]) => {
            let (a, b) = (axes(x1, y1), axes(x2, y2));
            let (p1, p2) = ((a.0.min(b.0), a.1.min(b.1)), (a.0.max(b.0), a.1.max(b.1)));
            format!(
                "AL5,0;IN;\r\nPA;PROGSTEP10;\r\n@SEGMENTEDFILESTART1;\r\n@SEGMENTSTART1;\r\n\r\nAL5,0,0,1,{},{},{},{};\r\n{corps}PU{},{};\r\nAL5,2;\r\n\r\n@SEGMENTEND1;\r\n\r\nSP0;@SEGMENTEDFILEEND1;\r\n",
                p1.0, p1.1, p2.0, p2.1, p1.0, p1.1
            )
        }
        None => format!("IN;\r\nPA;PROGSTEP10;\r\n{corps}PU0,0;\r\nSP0;\r\n"),
    }
}

/// Écrit `octets` à `chemin` d'un coup : fichier temporaire caché du même dossier,
/// synchronisé, puis renommé. Un logiciel qui surveille le dossier ne voit jamais un
/// fichier à moitié écrit (roadmap 4.6).
pub fn ecrire_atomique(chemin: &Path, octets: &[u8]) -> Result<(), String> {
    let nom = chemin.file_name().ok_or("chemin sans nom de fichier")?.to_string_lossy();
    let tmp = chemin.with_file_name(format!(".{nom}.tmp"));
    let ecrit = std::fs::File::create(&tmp).and_then(|mut f| f.write_all(octets).and_then(|()| f.sync_all())).and_then(|()| std::fs::rename(&tmp, chemin));
    ecrit.map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("{} : {e}", chemin.display())
    })
}

/// Connexion TCP brute (port 9100…), délai de 5 s à l'ouverture et de 60 s par écriture.
pub fn ouvrir_reseau(hote: &str, port: u16) -> Result<std::net::TcpStream, String> {
    use std::net::ToSocketAddrs;
    let adresse = (hote, port).to_socket_addrs().map_err(|e| format!("{hote}:{port} : {e}"))?.next().ok_or(format!("{hote} : adresse introuvable"))?;
    let flux = std::net::TcpStream::connect_timeout(&adresse, Duration::from_secs(5)).map_err(|e| format!("{adresse} : {e}"))?;
    flux.set_write_timeout(Some(Duration::from_secs(60))).map_err(|e| e.to_string())?;
    Ok(flux)
}

/// Envoie les octets par la connexion de la machine (bloquant).
pub fn envoyer(c: &Connexion, octets: &[u8]) -> Result<(), String> {
    match c {
        Connexion::Serie { port, stty } => crate::plotter::envoyer(port, stty, octets),
        Connexion::Reseau { hote, port } => {
            let mut flux = ouvrir_reseau(hote, *port)?;
            flux.write_all(octets).map_err(|e| format!("{hote}:{port} : {e}"))?;
            flux.shutdown(std::net::Shutdown::Write).map_err(|e| e.to_string())
        }
        // Un fichier ordinaire est remplacé d'un coup ; un périphérique (`/dev/…`) s'écrit tel quel.
        Connexion::Fichier { chemin } if chemin.is_file() => ecrire_atomique(chemin, octets),
        Connexion::Fichier { chemin } => std::fs::OpenOptions::new().write(true).open(chemin).and_then(|mut f| f.write_all(octets)).map_err(|e| format!("{} : {e}", chemin.display())),
        Connexion::Dossier => Err("machine sans connexion : sortie en dossier seulement".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_et_capacites() {
        let c = catalogue_initial(Path::new("/dev/kona"), "9600", None);
        assert!(verifier(&c).is_ok());
        // 1 m de rouleau sur 1626 mm à 720 dpi : 1,626 m² à 17 m²/h ≈ 5,7 min.
        assert!((c[0].duree_impression_min(1000.0, 720).unwrap() - 1.626 / 17.0 * 60.0).abs() < 1e-9);
        assert_eq!(c[1].duree_impression_min(1000.0, 720), None, "le Kona n'imprime pas");
        assert!(!c[1].fait(Operation::Impression) && c[1].fait(Operation::Decoupe));
        assert!(Modele::RolandVersacamm.imprime() && Modele::RolandVersacamm.decoupe());
        let mut double = c.clone();
        double[1].id = "epson".into();
        assert_eq!(verifier(&double), Err("machine_invalide"));
        // Format du fichier : la connexion est étiquetée par son type.
        let json = serde_json::to_value(&c[1]).unwrap();
        assert_eq!(json["connexion"]["type"], "serie");
        assert_eq!(json["modele"], "mutoh_kona");
    }

    #[test]
    fn pointilles_de_perforation() {
        // Trait de 100 unités, 30 coupées / 20 relevées : bas 0–30, haut –50, bas –80, haut –100.
        let p = pointiller("IN;SP1;PU0,0;PD100,0;PU;SP0;", &[[30.0, 20.0]], false);
        assert_eq!(p, "IN;SP1;PU0,0;PD30,0;PU50,0;PD80,0;PU100,0;PU;SP0;");
        // Le motif continue d'un segment au suivant d'un même tracé ; paires enchaînées.
        assert_eq!(pointiller("PU0,0;PD20,0,40,0;", &[[30.0, 20.0]], false), "PU0,0;PD20,0;PD30,0;PU40,0;");
        assert_eq!(pointiller("PU0,0;PD60,0;", &[[10.0, 5.0], [20.0, 5.0]], false), "PU0,0;PD10,0;PU15,0;PD35,0;PU40,0;PD50,0;PU55,0;PD60,0;");
        // Diffusion : côtés de 90, motif 30/20 → 1,8 motif arrondi à 2, ramené à 27/18 ;
        // chaque côté recommence outil bas à son angle et finit relevé sur l'angle suivant.
        let d = pointiller("PU0,0;PD90,0,90,90;", &[[30.0, 20.0]], true);
        assert_eq!(d, "PU0,0;PD27,0;PU45,0;PD72,0;PU90,0;PD90,27;PU90,45;PD90,72;PU90,90;");
        assert!(pointiller("PU0,0;PD90,0,90,90;", &[[30.0, 20.0]], false).contains("PU90,10;PD90,40;"), "sans diffusion, le motif continue après l'angle");
    }

    #[test]
    fn conversion_de_pas() {
        let rip = "IN;SP1;PU0,0;PD2540,0,2540,2540;PU;SP0;";
        // 25,4 mm : 2540 pas de 0,01 mm, 1016 pas de 0,025 mm (HP-GL standard).
        assert_eq!(changer_de_pas(rip, 0.025, [0.0; 2]), "IN;SP1;PU0,0;PD1016,0,1016,1016;PU;SP0;");
        // Kona avec cadre : axes tournés (X = y, Y = −x), coins min/max, une commande par ligne.
        let p = ParametreDecoupe { uuid: None, nom: "Mi-chair V3000".into(), vitesse_cm_s: 15.0, force_g: 145.0, acceleration_g: 1.0, valeurs_machine: false, outil: None, motif_mm: vec![], diffusion: false };
        let k = pour_kona(&[(rip.into(), Some(&p))], Some([-500, -500, 3000, 3000]), false);
        assert!(k.starts_with("AL5,0;IN;\r\nPA;PROGSTEP10;\r\n@SEGMENTEDFILESTART1;\r\n@SEGMENTSTART1;\r\n\r\nAL5,0,0,1,-500,-3000,3000,500;\r\nSP1;\r\nAS 1.0;ZF 145;VS 15;LT;\r\nPU0,0;\r\nPD0,-2540;\r\nPD2540,-2540;\r\n"), "{k}");
        assert!(k.ends_with("PU-500,-3000;\r\nAL5,2;\r\n\r\n@SEGMENTEND1;\r\n\r\nSP0;@SEGMENTEDFILEEND1;\r\n"), "{k}");
        // Valeurs du massicot : aucun réglage envoyé ; outil imposé.
        let q = ParametreDecoupe { valeurs_machine: true, outil: Some(3), ..p.clone() };
        let k = pour_kona(&[(rip.into(), Some(&q))], None, false);
        assert!(!k.contains("ZF") && k.contains("SP3;\r\n") && !k.contains("SP1;"), "{k}");
        // Sans cadre : outil 1 obligatoire (sans lui, le Kona ne coupe rien : essai du 24/10).
        assert!(pour_kona(&[(rip.into(), None)], None, false).contains("SP1;\r\n"));
        assert_eq!(changer_de_pas(rip, 0.01, [0.0; 2]), rip);
        // 1 mm à droite, 0,5 mm en arrière, au pas de 0,025 mm : +40 et −20.
        assert_eq!(changer_de_pas(rip, 0.025, [1.0, -0.5]), "IN;SP1;PU40,-20;PD1056,-20,1056,996;PU;SP0;");
        let mut m = catalogue_initial(Path::new("/dev/kona"), "9600", None).remove(1);
        assert_eq!(m.pas_mm(), 0.01);
        m.modele = Modele::GraphtecFc7000;
        assert_eq!(m.pas_mm(), 0.025);
        m.pas_mm = Some(0.03);
        assert!(verifier(&[m]).is_err(), "pas inconnu refusé");
    }

    #[test]
    fn envoi_reseau_et_fichier() {
        let ecoute = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = ecoute.local_addr().unwrap().port();
        let recu = std::thread::spawn(move || {
            let (mut f, _) = ecoute.accept().unwrap();
            let mut o = vec![];
            std::io::Read::read_to_end(&mut f, &mut o).unwrap();
            o
        });
        envoyer(&Connexion::Reseau { hote: "127.0.0.1".into(), port }, b"IN;PU0,0;").unwrap();
        assert_eq!(recu.join().unwrap(), b"IN;PU0,0;");
        let f = std::env::temp_dir().join(format!("eko-rip-peripherique-{}", std::process::id()));
        std::fs::write(&f, b"").unwrap();
        envoyer(&Connexion::Fichier { chemin: f.clone() }, b"abc").unwrap();
        assert_eq!(std::fs::read(&f).unwrap(), b"abc");
        std::fs::remove_file(&f).unwrap();
        assert!(envoyer(&Connexion::Reseau { hote: "127.0.0.1".into(), port: 9 }, b"x").is_err());
        assert!(envoyer(&Connexion::Dossier, b"x").is_err());
    }
}
