//! Planche manuelle (roadmap 5.10) : copies posées à la main (verrou, grille, magnétisme),
//! gabarits de cellules (cases fixes, une copie centrée par case) et données variables (une
//! copie par ligne d'un CSV, avec des textes posés sur la page). Le placement rend des
//! `Place` comme les autres imbrications ; le PDF passe ensuite par `composer_places`.

use crate::imbrication::{PT_PAR_MM, Place, Taille, placer};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Copie posée à la main : `copie` dans l'ordre des copies déroulées (à partir de 0), coin
/// bas gauche en mm sur la planche, angle (0, 90, 180, 270). Verrouillée, elle est prise
/// telle quelle ; sinon elle est calée sur la grille puis aimantée aux bords voisins.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
pub struct Pose {
    pub copie: usize,
    pub x_mm: f64,
    pub y_mm: f64,
    #[serde(default)]
    pub angle: u16,
    #[serde(default)]
    pub verrou: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct Manuel {
    #[serde(default)]
    pub poses: Vec<Pose>,
    /// Pas de la grille (mm, 0 : sans grille).
    #[serde(default)]
    pub grille_mm: f64,
    /// Portée de l'aimantation aux bords de la laize et des copies déjà posées (mm, 0 : sans).
    #[serde(default)]
    pub magnetisme_mm: f64,
}

impl Manuel {
    pub fn verifier(&self) -> Result<(), &'static str> {
        let bon = self.poses.len() <= 2000
            && (0.0..=1000.0).contains(&self.grille_mm)
            && (0.0..=100.0).contains(&self.magnetisme_mm)
            && self.poses.iter().all(|p| p.angle % 90 == 0 && p.angle < 360 && p.x_mm.is_finite() && p.y_mm.is_finite() && (-1e5..=1e6).contains(&p.y_mm) && (-1e5..=1e5).contains(&p.x_mm));
        if bon { Ok(()) } else { Err("poses : 2000 au plus, angle 0/90/180/270, grille 0–1000 mm, magnétisme 0–100 mm") }
    }
}

fn posee(t: Taille, angle: u16) -> Taille {
    if angle % 180 == 90 { Taille { largeur: t.hauteur, hauteur: t.largeur } } else { t }
}

/// Valeur de `candidats` la plus proche de `v` à moins de `portee`, sinon `v`.
fn aimanter(v: f64, candidats: impl Iterator<Item = f64>, portee: f64) -> f64 {
    candidats.filter(|c| (c - v).abs() <= portee).min_by(|a, b| (a - v).abs().total_cmp(&(b - v).abs())).unwrap_or(v)
}

/// Places des copies (points) : les verrouillées d'abord, puis les autres posées à la main
/// (grille, aimantation) ; refus si une copie sort de la laize ou en chevauche une autre.
/// Les copies sans pose vont en rangées au-dessus de celles posées à la main.
pub fn placer_manuel(tailles: &[Taille], m: &Manuel, laize: f64, espace: f64, rotation: bool) -> Result<(Vec<Place>, Taille), String> {
    let mut posees = vec![false; tailles.len()];
    for p in &m.poses {
        if p.copie >= tailles.len() {
            return Err(format!("pose : pas de copie {}", p.copie + 1));
        }
        if std::mem::replace(&mut posees[p.copie], true) {
            return Err(format!("la copie {} est posée deux fois", p.copie + 1));
        }
    }
    let boite = |p: &Place| {
        let t = posee(tailles[p.indice], p.angle);
        [p.x, p.y, p.x + t.largeur, p.y + t.hauteur]
    };
    let mut places: Vec<Place> = vec![];
    for p in m.poses.iter().filter(|p| p.verrou).chain(m.poses.iter().filter(|p| !p.verrou)) {
        let t = posee(tailles[p.copie], p.angle);
        let (mut x, mut y) = (p.x_mm * PT_PAR_MM, p.y_mm * PT_PAR_MM);
        if !p.verrou {
            let g = m.grille_mm * PT_PAR_MM;
            if g > 0.0 {
                (x, y) = ((x / g).round() * g, (y / g).round() * g);
            }
            let a = m.magnetisme_mm * PT_PAR_MM;
            if a > 0.0 {
                let voisins: Vec<[f64; 4]> = places.iter().map(boite).collect();
                x = aimanter(x, [0.0, laize - t.largeur].into_iter().chain(voisins.iter().flat_map(|b| [b[2] + espace, b[0] - espace - t.largeur, b[0], b[2] - t.largeur])), a);
                y = aimanter(y, [0.0].into_iter().chain(voisins.iter().flat_map(|b| [b[3] + espace, b[1] - espace - t.hauteur, b[1], b[3] - t.hauteur])), a);
            }
        }
        let place = Place { indice: p.copie, x, y, angle: p.angle };
        let b = boite(&place);
        if b[0] < -1e-6 || b[1] < -1e-6 || b[2] > laize + 1e-6 {
            return Err(format!("la copie {} sort de la laize ({:.0} mm)", p.copie + 1, laize / PT_PAR_MM));
        }
        if let Some(o) = places.iter().find(|o| {
            let q = boite(o);
            b[0] < q[2] - 1e-6 && q[0] < b[2] - 1e-6 && b[1] < q[3] - 1e-6 && q[1] < b[3] - 1e-6
        }) {
            return Err(format!("la copie {} chevauche la copie {}", p.copie + 1, o.indice + 1));
        }
        places.push(place);
    }
    let reste: Vec<usize> = (0..tailles.len()).filter(|&i| !posees[i]).collect();
    if !reste.is_empty() {
        let base = if places.is_empty() { 0.0 } else { places.iter().map(|p| boite(p)[3]).fold(0.0, f64::max) + espace };
        let (auto, _) = placer(&reste.iter().map(|&i| tailles[i]).collect::<Vec<_>>(), laize, espace, rotation)?;
        places.extend(auto.into_iter().map(|p| Place { indice: reste[p.indice], y: p.y + base, ..p }));
    }
    let (largeur, hauteur) = places.iter().map(boite).fold((0.0f64, 0.0f64), |(l, h), b| (l.max(b[2]), h.max(b[3])));
    Ok((places, Taille { largeur, hauteur }))
}

/// Gabarit de cellules : `colonnes` × `rangees` cases de `largeur_mm` × `hauteur_mm`,
/// séparées de `espace_mm` ; chaque copie est centrée dans sa case, couchée si elle n'y
/// tient que tournée. Gardés par nom dans `gabarits.json`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Gabarit {
    pub nom: String,
    pub largeur_mm: f64,
    pub hauteur_mm: f64,
    pub colonnes: usize,
    pub rangees: usize,
    #[serde(default)]
    pub espace_mm: f64,
}

impl Gabarit {
    fn valide(&self) -> bool {
        (1..=60).contains(&self.nom.chars().count())
            && (5.0..=5000.0).contains(&self.largeur_mm)
            && (5.0..=100_000.0).contains(&self.hauteur_mm)
            && (1..=100).contains(&self.colonnes)
            && (1..=500).contains(&self.rangees)
            && self.colonnes * self.rangees <= 2000
            && (0.0..=100.0).contains(&self.espace_mm)
    }
}

pub fn verifier_gabarits(liste: &[Gabarit]) -> Result<(), &'static str> {
    let noms_uniques = liste.iter().enumerate().all(|(i, g)| !liste[..i].iter().any(|h| h.nom == g.nom));
    if liste.len() <= 100 && noms_uniques && liste.iter().all(Gabarit::valide) { Ok(()) } else { Err("gabarit_invalide") }
}

fn fichier(donnees: &Path) -> PathBuf {
    donnees.join("gabarits.json")
}

pub fn lire(donnees: &Path) -> Vec<Gabarit> {
    crate::etat::lire(&fichier(donnees)).ok().flatten().unwrap_or_default()
}

pub fn ecrire(donnees: &Path, liste: &[Gabarit]) -> std::io::Result<()> {
    crate::etat::ecrire(&fichier(donnees), liste, false)
}

/// Places des copies, case par case (ligne par ligne depuis le bas gauche).
pub fn placer_en_cellules(tailles: &[Taille], g: &Gabarit, laize: f64) -> Result<(Vec<Place>, Taille), String> {
    if !g.valide() {
        return Err("gabarit : 1 à 100 colonnes, 1 à 500 rangées, cases de 5 mm au moins".into());
    }
    let (w, h, e) = (g.largeur_mm * PT_PAR_MM, g.hauteur_mm * PT_PAR_MM, g.espace_mm * PT_PAR_MM);
    let largeur = g.colonnes as f64 * (w + e) - e;
    if largeur > laize + 1e-6 {
        return Err(format!("{} colonnes de {:.0} mm dépassent la laize", g.colonnes, g.largeur_mm));
    }
    if tailles.len() > g.colonnes * g.rangees {
        return Err(format!("{} copies pour {} cases", tailles.len(), g.colonnes * g.rangees));
    }
    let mut places = vec![];
    for (i, t) in tailles.iter().enumerate() {
        let angle = if t.largeur <= w + 1e-6 && t.hauteur <= h + 1e-6 {
            0
        } else if t.hauteur <= w + 1e-6 && t.largeur <= h + 1e-6 {
            90
        } else {
            return Err(format!("la copie {} ({:.0} × {:.0} mm) ne tient pas dans une case", i + 1, t.largeur / PT_PAR_MM, t.hauteur / PT_PAR_MM));
        };
        let p = posee(*t, angle);
        let (c, r) = (i % g.colonnes, i / g.colonnes);
        places.push(Place { indice: i, x: c as f64 * (w + e) + (w - p.largeur) / 2.0, y: r as f64 * (h + e) + (h - p.hauteur) / 2.0, angle });
    }
    let rangees = tailles.len().div_ceil(g.colonnes);
    Ok((places, Taille { largeur, hauteur: rangees as f64 * (h + e) - e }))
}

/// Données variables : un CSV (première ligne : noms de colonnes, séparateur `;` ou `,`,
/// guillemets doubles) et les champs posés sur la page, une copie par ligne.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct DonneesVariables {
    pub csv: String,
    pub champs: Vec<Champ>,
}

/// Texte d'une colonne, posé sur la page à (x, y) mm de son coin bas gauche, en Helvetica.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Champ {
    pub colonne: String,
    pub x_mm: f64,
    pub y_mm: f64,
    #[serde(default = "taille_defaut")]
    pub taille_pt: f64,
}

fn taille_defaut() -> f64 {
    12.0
}

/// Texte d'une copie, en points de sa page (depuis le coin bas gauche de la MediaBox).
#[derive(Clone, Debug, PartialEq)]
pub struct Texte {
    pub x: f64,
    pub y: f64,
    pub taille: f64,
    pub contenu: String,
}

pub const LIGNES_MAX: usize = 2000;

/// En-tête et lignes du CSV ; le séparateur est celui de l'en-tête (`;` s'il y en a un
/// hors guillemets). Un guillemet jamais refermé est refusé : il fusionnerait des lignes.
pub fn lire_csv(csv: &str) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let csv = csv.trim_start_matches('\u{feff}');
    let (mut dedans, mut point_virgule) = (false, false);
    for c in csv.chars() {
        match c {
            '"' => dedans = !dedans,
            '\n' if !dedans => break,
            ';' if !dedans => point_virgule = true,
            _ => {}
        }
    }
    let sep = if point_virgule { ';' } else { ',' };
    let (mut lignes, mut ligne, mut champ, mut guillemets) = (vec![], vec![], String::new(), false);
    let mut car = csv.chars().peekable();
    while let Some(c) = car.next() {
        match c {
            '"' if guillemets && car.peek() == Some(&'"') => {
                champ.push('"');
                car.next();
            }
            '"' => guillemets = !guillemets,
            c if c == sep && !guillemets => ligne.push(std::mem::take(&mut champ)),
            '\r' if !guillemets => {}
            '\n' if !guillemets => {
                ligne.push(std::mem::take(&mut champ));
                lignes.push(std::mem::take(&mut ligne));
            }
            c => champ.push(c),
        }
    }
    if guillemets {
        return Err("CSV : guillemet jamais refermé".into());
    }
    if !champ.is_empty() || !ligne.is_empty() {
        ligne.push(champ);
        lignes.push(ligne);
    }
    lignes.retain(|l: &Vec<String>| l.iter().any(|c| !c.trim().is_empty()));
    let mut lignes = lignes.into_iter();
    let entete: Vec<String> = lignes.next().ok_or("CSV vide")?.into_iter().map(|c| c.trim().to_string()).collect();
    let lignes: Vec<Vec<String>> = lignes.collect();
    if lignes.is_empty() || lignes.len() > LIGNES_MAX {
        return Err(format!("CSV : 1 à {LIGNES_MAX} lignes après l'en-tête"));
    }
    Ok((entete, lignes))
}

/// Textes de chaque copie (une par ligne du CSV).
pub fn textes(d: &DonneesVariables) -> Result<Vec<Vec<Texte>>, String> {
    if d.champs.is_empty() || d.champs.len() > 20 || d.champs.iter().any(|c| !(4.0..=500.0).contains(&c.taille_pt) || !(0.0..=10_000.0).contains(&c.x_mm) || !(0.0..=100_000.0).contains(&c.y_mm)) {
        return Err("champs : 1 à 20, taille 4–500 pt, position positive".into());
    }
    let (entete, lignes) = lire_csv(&d.csv)?;
    let colonnes = d.champs.iter().map(|c| entete.iter().position(|e| e == &c.colonne).ok_or_else(|| format!("colonne « {} » absente du CSV", c.colonne))).collect::<Result<Vec<_>, _>>()?;
    Ok(lignes
        .iter()
        .map(|l| d.champs.iter().zip(&colonnes).map(|(c, &k)| Texte { x: c.x_mm * PT_PAR_MM, y: c.y_mm * PT_PAR_MM, taille: c.taille_pt, contenu: l.get(k).map(|v| v.trim().to_string()).unwrap_or_default() }).collect())
        .collect())
}

/// Chaîne PDF hexadécimale en WinAnsiEncoding (Helvetica de base) ; un caractère hors de
/// ce jeu devient `?`.
pub fn hex_winansi(s: &str) -> String {
    let octet = |c: char| -> u8 {
        match c {
            '€' => 0x80,
            '…' => 0x85,
            'Œ' => 0x8C,
            '‘' => 0x91,
            '’' => 0x92,
            '“' => 0x93,
            '”' => 0x94,
            '–' => 0x96,
            '—' => 0x97,
            'œ' => 0x9C,
            'Ÿ' => 0x9F,
            c if (' '..='~').contains(&c) || ('\u{a0}'..='\u{ff}').contains(&c) => c as u8,
            _ => b'?',
        }
    };
    s.chars().take(500).map(|c| format!("{:02X}", octet(c))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MM: f64 = PT_PAR_MM;

    fn carre(cote_mm: f64) -> Taille {
        Taille { largeur: cote_mm * MM, hauteur: cote_mm * MM }
    }

    fn pose(copie: usize, x_mm: f64, y_mm: f64, verrou: bool) -> Pose {
        Pose { copie, x_mm, y_mm, angle: 0, verrou }
    }

    /// Verrou pris tel quel, grille puis aimantation pour les autres, refus des chevauchements,
    /// copies sans pose au-dessus.
    #[test]
    fn poses_verrou_grille_magnetisme() {
        let t = [carre(100.0), carre(100.0), carre(50.0), carre(50.0)];
        let m = Manuel { poses: vec![pose(1, 113.0, 4.0, false), pose(0, 3.3, 0.0, true)], grille_mm: 10.0, magnetisme_mm: 8.0 };
        let (places, planche) = placer_manuel(&t, &m, 500.0 * MM, 5.0 * MM, false).unwrap();
        let mm = |p: &Place| ((p.x / MM * 10.0).round() / 10.0, (p.y / MM * 10.0).round() / 10.0);
        let de = |i: usize| places.iter().find(|p| p.indice == i).map(mm).unwrap();
        assert_eq!(de(0), (3.3, 0.0), "verrouillée : telle quelle, hors grille");
        assert_eq!(de(1), (108.3, 0.0), "grille (110, 0), puis aimantée à 5 mm du bord droit de la copie 1");
        assert_eq!((de(2).1, de(3).1), (105.0, 105.0), "sans pose : en rangée au-dessus, après l'espace");
        assert!((planche.hauteur / MM - 155.0).abs() < 1e-6);
        let m = Manuel { poses: vec![pose(0, 0.0, 0.0, true), pose(1, 50.0, 50.0, true)], ..Default::default() };
        assert_eq!(placer_manuel(&t, &m, 500.0 * MM, 0.0, false).unwrap_err(), "la copie 2 chevauche la copie 1");
        let m = Manuel { poses: vec![pose(0, 450.0, 0.0, true)], ..Default::default() };
        assert!(placer_manuel(&t, &m, 500.0 * MM, 0.0, false).unwrap_err().contains("sort de la laize"));
        let m = Manuel { poses: vec![pose(0, 0.0, 0.0, true), pose(0, 200.0, 0.0, true)], ..Default::default() };
        assert!(placer_manuel(&t, &m, 500.0 * MM, 0.0, false).unwrap_err().contains("deux fois"));
        assert!(Manuel { poses: vec![Pose { angle: 45, ..pose(0, 0.0, 0.0, false) }], ..Default::default() }.verifier().is_err());
    }

    #[test]
    fn cellules_centrees_et_couchees() {
        let g = Gabarit { nom: "Cartes".into(), largeur_mm: 90.0, hauteur_mm: 60.0, colonnes: 3, rangees: 2, espace_mm: 10.0 };
        let t = [Taille { largeur: 80.0 * MM, hauteur: 50.0 * MM }, Taille { largeur: 50.0 * MM, hauteur: 80.0 * MM }, carre(40.0), carre(40.0)];
        let (places, planche) = placer_en_cellules(&t, &g, 300.0 * MM).unwrap();
        let r = |v: f64| (v / MM * 10.0).round() / 10.0;
        assert_eq!(places.iter().map(|p| (r(p.x), r(p.y), p.angle)).collect::<Vec<_>>(), [(5.0, 5.0, 0), (105.0, 5.0, 90), (225.0, 10.0, 0), (25.0, 80.0, 0)]);
        assert_eq!((r(planche.largeur), r(planche.hauteur)), (290.0, 130.0), "deux rangées utilisées");
        assert!(placer_en_cellules(&[carre(70.0)], &g, 300.0 * MM).unwrap_err().contains("ne tient pas"));
        assert!(placer_en_cellules(&t, &g, 250.0 * MM).unwrap_err().contains("laize"));
        assert!(placer_en_cellules(&[carre(10.0); 7], &g, 300.0 * MM).unwrap_err().contains("7 copies pour 6 cases"));
        assert_eq!(verifier_gabarits(&[g.clone(), g]), Err("gabarit_invalide"), "noms uniques");
    }

    /// Les textes d'une copie sont posés sur elle, en Helvetica WinAnsi, après la page.
    #[test]
    fn textes_poses_sur_chaque_copie() {
        let pdf = crate::imbrication::tests::pdf(100.0 * MM, 50.0 * MM, "1 0 0 0", (0.0, 0.0));
        let sources = [crate::imbrication::Source { pdf: &pdf, page: 1, copies: 2 }];
        let d = DonneesVariables { csv: "Nom\nÉlise\nMarc".into(), champs: vec![Champ { colonne: "Nom".into(), x_mm: 10.0, y_mm: 5.0, taille_pt: 12.0 }] };
        let t = textes(&d).unwrap();
        let (sortie, places) = crate::imbrication::composer(&sources, 300.0, 0.0, false, false, None, Default::default(), Default::default(), &t).unwrap();
        assert_eq!(places.len(), 2);
        let doc = lopdf::Document::load_mem(&sortie).unwrap();
        let page = *doc.get_pages().get(&1).unwrap();
        let contenu = String::from_utf8_lossy(&doc.get_page_content(page)).into_owned();
        assert!(contenu.contains(&format!("<{}> Tj", hex_winansi("Élise"))) && contenu.contains(&format!("<{}> Tj", hex_winansi("Marc"))), "{contenu}");
        assert!(format!("{:?}", doc.get_page_resources(page).unwrap().0.unwrap()).contains("Helvetica"));
    }

    #[test]
    fn csv_et_textes() {
        let (entete, lignes) = lire_csv("\u{feff}Nom;Ville;N°\r\n\"Dupont; fils\";Nîmes;1\n\nMartin;\"Le \"\"Puy\"\"\";2").unwrap();
        assert_eq!(entete, ["Nom", "Ville", "N°"]);
        assert_eq!(lignes, [vec!["Dupont; fils", "Nîmes", "1"], vec!["Martin", "Le \"Puy\"", "2"]]);
        assert_eq!(lire_csv("a,b\n1,2").unwrap().1, [vec!["1", "2"]], "virgule sans point-virgule dans l'en-tête");
        assert!(lire_csv("a;b\n").is_err(), "aucune ligne");
        assert_eq!(lire_csv("Nom,\"Note;détail\"\nAlice,ok").unwrap().0, ["Nom", "Note;détail"], "point-virgule entre guillemets : pas le séparateur");
        assert_eq!(lire_csv("Nom\n\"Alice\nBob\nCarol").unwrap_err(), "CSV : guillemet jamais refermé");
        let d = DonneesVariables { csv: "Nom;N°\nÉlise;7\nŒdipe;8".into(), champs: vec![Champ { colonne: "Nom".into(), x_mm: 10.0, y_mm: 20.0, taille_pt: 14.0 }] };
        let t = textes(&d).unwrap();
        assert_eq!((t.len(), t[1][0].contenu.as_str(), (t[0][0].y / MM).round()), (2, "Œdipe", 20.0));
        assert_eq!(hex_winansi("Éa€✓"), "C961803F");
        let faux = DonneesVariables { champs: vec![Champ { colonne: "Prénom".into(), x_mm: 0.0, y_mm: 0.0, taille_pt: 12.0 }], ..d };
        assert_eq!(textes(&faux).unwrap_err(), "colonne « Prénom » absente du CSV");
    }
}
