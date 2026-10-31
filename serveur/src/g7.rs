//! Étalonnage du gris de type G7 (roadmap 8.10), côté serveur : mire des deux rampes (CMY à
//! parts égales, noir seul), courbe cible du gris tirée du papier et de l'aplat, courbes de
//! correction par encre, et historique des itérations où la meilleure est gardée (une
//! itération moins bonne que la précédente ne l'efface pas).
//!
//! Méthode et valeurs à nous, sans table ni courbe d'un autre logiciel :
//! - courbe cible : Murray-Davies entre le papier et l'aplat mesurés, avec un engraissement
//!   de référence en parabole (`engraissement` à 50 %, 22 % par défaut) ;
//! - gris cible : la teinte du papier, qui s'efface à mesure que le gris fonce ;
//! - équilibre du gris : l'écart a*b* est rattrapé par C, M et Y dans la direction de leurs
//!   aplats, à densité constante (dC + dM + dY = 0), amorti pour ne pas dépasser.
//!
//! Les courbes s'appliqueront avec la linéarisation par mode (8.2) ; d'ici là, elles se
//! lisent et se gardent ici.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const ENGRAISSEMENT_DEFAUT: f64 = 0.22;
/// Seuils indicatifs de réussite : écart moyen de clarté et de gris (ΔL*, Δa*b*).
pub const SEUIL_L: f64 = 1.5;
pub const SEUIL_GRIS: f64 = 1.5;
/// Part de l'écart de gris rattrapée à chaque itération : la suivante finit le travail.
const AMORTI: f64 = 0.8;
pub const ITERATIONS_MAX: usize = 50;
const INVALIDES: &str = "mesures_invalides";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rampe {
    /// C = M = Y.
    Cmy,
    K,
}

/// Une plage mesurée : rampe, valeur demandée (%), Lab mesuré.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Mesure {
    pub rampe: Rampe,
    pub pct: f64,
    pub l: f64,
    pub a: f64,
    pub b: f64,
}

/// a*b* des aplats C, M et Y, s'ils sont mesurés (sinon des valeurs moyennes d'encres
/// d'impression numérique).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Aplats {
    pub c: [f64; 2],
    pub m: [f64; 2],
    pub y: [f64; 2],
}

impl Default for Aplats {
    fn default() -> Self {
        Aplats { c: [-37.0, -50.0], m: [75.0, -5.0], y: [-5.0, 90.0] }
    }
}

/// Écarts d'une itération à la cible, sur les plages mesurées.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Ecarts {
    /// ΔL* moyen, les deux rampes.
    pub l: f64,
    /// Δa*b* moyen de la rampe CMY.
    pub gris: f64,
    pub conforme: bool,
}

impl Ecarts {
    fn note(&self) -> f64 {
        self.l + self.gris
    }
}

/// Courbes de correction : (valeur voulue %, valeur à envoyer %), de 0 à 100.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Courbes {
    pub c: Vec<(f64, f64)>,
    pub m: Vec<(f64, f64)>,
    pub y: Vec<(f64, f64)>,
    pub k: Vec<(f64, f64)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Iteration {
    pub numero: u32,
    pub le: u64,
    pub mesures: Vec<Mesure>,
    pub ecarts: Ecarts,
    pub courbes: Courbes,
}

/// Un étalonnage nommé (par exemple une imprimante et un support), avec ses itérations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Etalonnage {
    pub nom: String,
    pub engraissement: f64,
    #[serde(default)]
    pub aplats: Option<Aplats>,
    pub iterations: Vec<Iteration>,
}

impl Etalonnage {
    /// L'itération la plus proche de la cible (la plus ancienne à égalité).
    pub fn meilleure(&self) -> Option<&Iteration> {
        self.iterations.iter().min_by(|a, b| a.ecarts.note().total_cmp(&b.ecarts.note()))
    }
}

/// Paliers des deux rampes, régulières de 0 à 100 % (`n` de 5 à 51).
pub fn mire_cgats(n: usize) -> Result<String, &'static str> {
    if !(5..=51).contains(&n) {
        return Err("mire_invalide");
    }
    let paliers: Vec<f64> = (0..n).map(|i| (i as f64 * 100.0 / (n - 1) as f64 * 10.0).round() / 10.0).collect();
    let mut s = String::from("CGATS.17\nORIGINATOR \"E-KO RIP Center\"\nDESCRIPTOR \"Mire G7 : rampe CMY et rampe noire\"\n");
    s += "NUMBER_OF_FIELDS 5\nBEGIN_DATA_FORMAT\nSAMPLE_ID CMYK_C CMYK_M CMYK_Y CMYK_K\nEND_DATA_FORMAT\n";
    s += &format!("NUMBER_OF_SETS {}\nBEGIN_DATA\n", 2 * n);
    for (i, p) in paliers.iter().enumerate() {
        s += &format!("{} {p:.1} {p:.1} {p:.1} 0.0\n", i + 1);
    }
    for (i, p) in paliers.iter().enumerate() {
        s += &format!("{} 0.0 0.0 0.0 {p:.1}\n", n + i + 1);
    }
    s += "END_DATA\n";
    Ok(s)
}

/// Y relatif (0..1) d'un L*.
fn y_de(l: f64) -> f64 {
    let f = (l + 16.0) / 116.0;
    if f > 6.0 / 29.0 { f.powi(3) } else { 3.0 * (6.0f64 / 29.0).powi(2) * (f - 4.0 / 29.0) }
}

fn l_de(y: f64) -> f64 {
    let f = if y > (6.0f64 / 29.0).powi(3) { y.cbrt() } else { y / (3.0 * (6.0f64 / 29.0).powi(2)) + 4.0 / 29.0 };
    116.0 * f - 16.0
}

/// Clarté cible à `t` (0..1) entre le papier et l'aplat.
fn l_cible(t: f64, l_papier: f64, l_aplat: f64, engraissement: f64) -> f64 {
    let couverture = (t + engraissement * 4.0 * t * (1.0 - t)).clamp(0.0, 1.0);
    let (yp, ya) = (y_de(l_papier), y_de(l_aplat));
    l_de(yp - couverture * (yp - ya))
}

/// Interpolation linéaire de `y` en `x` sur des points triés par x.
fn interpoler(points: &[(f64, f64)], x: f64) -> f64 {
    match points.iter().position(|p| p.0 >= x) {
        Some(0) => points[0].1,
        Some(i) => {
            let (a, b) = (points[i - 1], points[i]);
            if b.0 == a.0 { b.1 } else { a.1 + (b.1 - a.1) * (x - a.0) / (b.0 - a.0) }
        }
        None => points.last().map_or(0.0, |p| p.1),
    }
}

/// Valeur à demander (%) pour obtenir la clarté `l`, sur une rampe mesurée rendue
/// décroissante (une plage plus claire que la précédente ne fait pas reculer la courbe).
fn valeur_pour(rampe: &[(f64, f64)], l: f64) -> f64 {
    let mut plus_sombre = f64::INFINITY;
    let monotone: Vec<(f64, f64)> = rampe.iter().map(|&(p, x)| {
        plus_sombre = plus_sombre.min(x);
        (p, plus_sombre)
    }).collect();
    if l >= monotone[0].1 {
        return 0.0;
    }
    match monotone.windows(2).find(|w| w[1].1 <= l) {
        Some(w) if w[0].1 == w[1].1 => w[1].0,
        Some(w) => w[0].0 + (w[1].0 - w[0].0) * (w[0].1 - l) / (w[0].1 - w[1].1),
        None => 100.0,
    }
}

/// Résout dC·vc + dM·vm + dY·vy = e (2 équations) avec dC + dM + dY = 0.
fn equilibrer(e: [f64; 2], ap: &Aplats) -> [f64; 3] {
    // Variables dC, dM (dY = −dC − dM) : (vc − vy)·dC + (vm − vy)·dM = e.
    let u = [ap.c[0] - ap.y[0], ap.c[1] - ap.y[1]];
    let v = [ap.m[0] - ap.y[0], ap.m[1] - ap.y[1]];
    let det = u[0] * v[1] - u[1] * v[0];
    if det.abs() < 1e-9 {
        return [0.0; 3];
    }
    let dc = (e[0] * v[1] - e[1] * v[0]) / det;
    let dm = (u[0] * e[1] - u[1] * e[0]) / det;
    [dc, dm, -dc - dm]
}

/// Écarts et courbes de correction d'une série de mesures. Il faut, pour chaque rampe, le
/// papier (0 %), l'aplat (100 %) et au moins trois plages entre les deux.
pub fn calculer(mesures: &[Mesure], engraissement: f64, aplats: Option<&Aplats>) -> Result<(Ecarts, Courbes), &'static str> {
    if !(0.0..=0.5).contains(&engraissement) || mesures.iter().any(|m| !(0.0..=100.0).contains(&m.pct) || !(0.0..=100.0).contains(&m.l) || !m.a.is_finite() || !m.b.is_finite() || m.a.abs() > 200.0 || m.b.abs() > 200.0) {
        return Err(INVALIDES);
    }
    let rampe = |r: Rampe| {
        let mut v: Vec<Mesure> = mesures.iter().copied().filter(|m| m.rampe == r).collect();
        v.sort_by(|a, b| a.pct.total_cmp(&b.pct));
        v
    };
    let (cmy, k) = (rampe(Rampe::Cmy), rampe(Rampe::K));
    for r in [&cmy, &k] {
        let complete = r.len() >= 5 && r[0].pct == 0.0 && r[r.len() - 1].pct == 100.0 && r.windows(2).all(|w| w[0].pct < w[1].pct);
        if !complete || r[0].l - r[r.len() - 1].l < 10.0 {
            return Err(INVALIDES);
        }
    }
    let papier = cmy[0];
    let ap = aplats.copied().unwrap_or_default();
    let cible_l = |r: &[Mesure], pct: f64| l_cible(pct / 100.0, papier.l, r[r.len() - 1].l, engraissement);
    // Le gris cible garde la teinte du papier, effacée à mesure qu'il fonce.
    let cible_ab = |r: &[Mesure], pct: f64| {
        let part = ((papier.l - cible_l(r, pct)) / (papier.l - r[r.len() - 1].l)).clamp(0.0, 1.0);
        [papier.a * (1.0 - part), papier.b * (1.0 - part)]
    };

    let interieures = |r: &[Mesure]| r[1..r.len() - 1].to_vec();
    let dl: Vec<f64> = [&cmy, &k].iter().flat_map(|r| interieures(r).into_iter().map(|m| (m.l - cible_l(r, m.pct)).abs()).collect::<Vec<_>>()).collect();
    let dab: Vec<f64> = interieures(&cmy).iter().map(|m| {
        let c = cible_ab(&cmy, m.pct);
        ((m.a - c[0]).powi(2) + (m.b - c[1]).powi(2)).sqrt()
    }).collect();
    let moyenne = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    let ecarts = Ecarts { l: moyenne(&dl), gris: moyenne(&dab), conforme: moyenne(&dl) <= SEUIL_L && moyenne(&dab) <= SEUIL_GRIS };

    let points = |r: &[Mesure]| r.iter().map(|m| (m.pct, m.l)).collect::<Vec<_>>();
    let (lc, lk) = (points(&cmy), points(&k));
    let a_de = cmy.iter().map(|m| (m.pct, m.a)).collect::<Vec<_>>();
    let b_de = cmy.iter().map(|m| (m.pct, m.b)).collect::<Vec<_>>();
    let mut courbes = Courbes { c: vec![], m: vec![], y: vec![], k: vec![] };
    for i in 0..=20 {
        let t = i as f64 * 5.0;
        let gris = valeur_pour(&lc, cible_l(&cmy, t));
        let noir = valeur_pour(&lk, cible_l(&k, t));
        // Écart de gris là où la rampe CMY sera envoyée, rattrapé à densité constante.
        let c = cible_ab(&cmy, t);
        let e = [c[0] - interpoler(&a_de, gris), c[1] - interpoler(&b_de, gris)];
        // Sensibilité : un point d'encre déplace a*b* d'un centième de son aplat.
        let d = if i == 0 || i == 20 { [0.0; 3] } else { equilibrer([e[0] * 100.0 * AMORTI, e[1] * 100.0 * AMORTI], &ap) };
        let borne = |x: f64| ((x * 10.0).round() / 10.0).clamp(0.0, 100.0);
        let (fin_gris, fin_noir) = match i {
            0 => (0.0, 0.0),
            20 => (100.0, 100.0),
            _ => (gris, noir),
        };
        courbes.c.push((t, borne(fin_gris + d[0])));
        courbes.m.push((t, borne(fin_gris + d[1])));
        courbes.y.push((t, borne(fin_gris + d[2])));
        courbes.k.push((t, borne(fin_noir)));
    }
    Ok((ecarts, courbes))
}

fn fichier(donnees: &Path) -> PathBuf {
    donnees.join("g7.json")
}

pub fn lire(donnees: &Path) -> Vec<Etalonnage> {
    crate::etat::lire(&fichier(donnees)).ok().flatten().unwrap_or_default()
}

pub fn ecrire(donnees: &Path, liste: &[Etalonnage]) -> std::io::Result<()> {
    crate::etat::ecrire(&fichier(donnees), liste, false)
}

pub fn nom_valide(nom: &str) -> bool {
    !nom.trim().is_empty() && nom.chars().count() <= 80 && !nom.chars().any(char::is_control)
}

/// Ajoute une itération à l'étalonnage `nom` (créé au besoin) ; les plus anciennes partent
/// au-delà de [`ITERATIONS_MAX`], jamais la meilleure.
pub fn ajouter(liste: &mut Vec<Etalonnage>, nom: &str, engraissement: Option<f64>, aplats: Option<Aplats>, mesures: Vec<Mesure>, le: u64) -> Result<Etalonnage, &'static str> {
    if !nom_valide(nom) {
        return Err("etalonnage_invalide");
    }
    let i = match liste.iter().position(|e| e.nom == nom) {
        Some(i) => i,
        None => {
            liste.push(Etalonnage { nom: nom.into(), engraissement: ENGRAISSEMENT_DEFAUT, aplats: None, iterations: vec![] });
            liste.len() - 1
        }
    };
    let e = &mut liste[i];
    if let Some(g) = engraissement {
        e.engraissement = g;
    }
    if aplats.is_some() {
        e.aplats = aplats;
    }
    let (ecarts, courbes) = calculer(&mesures, e.engraissement, e.aplats.as_ref())?;
    let numero = e.iterations.last().map_or(1, |x| x.numero + 1);
    e.iterations.push(Iteration { numero, le, mesures, ecarts, courbes });
    while e.iterations.len() > ITERATIONS_MAX {
        let garder = e.meilleure().map(|m| m.numero);
        let Some(j) = e.iterations.iter().position(|x| Some(x.numero) != garder) else { break };
        e.iterations.remove(j);
    }
    Ok(e.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Imprimante simulée : engraissement réel `g`, teinte du gris dérivée vers le rouge de
    /// `derive` a* au milieu, papier légèrement bleuté.
    fn imprimer(courbe: Option<&Courbes>, g: f64, derive: f64) -> Vec<Mesure> {
        let (lp, la) = (95.0, 20.0);
        let envoi = |c: Option<&Vec<(f64, f64)>>, t: f64| c.map_or(t, |c| interpoler(c, t));
        let mut v = vec![];
        for i in 0..=10 {
            let t = i as f64 * 10.0;
            for r in [Rampe::Cmy, Rampe::K] {
                // Encres envoyées : C, M, Y (rampe CMY) ou K.
                let x3 = match (r, courbe) {
                    (Rampe::Cmy, Some(c)) => [envoi(Some(&c.c), t), envoi(Some(&c.m), t), envoi(Some(&c.y), t)],
                    (Rampe::K, Some(c)) => [envoi(Some(&c.k), t); 3],
                    _ => [t; 3],
                };
                let x = (x3[0] + x3[1] + x3[2]) / 3.0;
                let l = l_cible(x / 100.0, lp, la, g);
                let milieu = 4.0 * x / 100.0 * (1.0 - x / 100.0);
                let (a, b) = match r {
                    // Chaque encre en plus du gris tire a*b* vers son aplat, d'un centième par point.
                    Rampe::Cmy => {
                        let ap = Aplats::default();
                        let d = x3.map(|v| v - x);
                        let tire = [0, 1].map(|k| (d[0] * ap.c[k] + d[1] * ap.m[k] + d[2] * ap.y[k]) / 100.0);
                        (-0.5 * (1.0 - x / 100.0) + derive * milieu + tire[0], -2.0 * (1.0 - x / 100.0) + tire[1])
                    }
                    Rampe::K => (0.0, 0.0),
                };
                v.push(Mesure { rampe: r, pct: t, l, a, b });
            }
        }
        v
    }

    #[test]
    fn mire_deux_rampes() {
        let m = mire_cgats(11).unwrap();
        assert!(m.contains("NUMBER_OF_SETS 22") && m.contains("\n6 50.0 50.0 50.0 0.0\n") && m.contains("\n17 0.0 0.0 0.0 50.0\n"));
        assert!(mire_cgats(4).is_err() && mire_cgats(52).is_err());
    }

    #[test]
    fn cible_et_courbes() {
        // L'imprimante a déjà l'engraissement de référence et un gris juste : rien à corriger.
        let juste = imprimer(None, ENGRAISSEMENT_DEFAUT, 0.0);
        let (e, c) = calculer(&juste, ENGRAISSEMENT_DEFAUT, None).unwrap();
        assert!(e.l < 0.05 && e.gris < 0.6 && e.conforme, "{e:?}");
        assert!(c.k.iter().all(|&(t, v)| (t - v).abs() <= 1.0), "{:?}", c.k);
        // Elle engraisse trop (35 %) : il faut envoyer moins au milieu, rien aux extrémités.
        let (e, c) = calculer(&imprimer(None, 0.35, 0.0), ENGRAISSEMENT_DEFAUT, None).unwrap();
        assert!(e.l > SEUIL_L && !e.conforme, "{e:?}");
        let milieu = c.k.iter().find(|p| p.0 == 50.0).unwrap().1;
        assert!(milieu < 45.0, "{milieu}");
        assert_eq!((c.k[0], c.k[20]), ((0.0, 0.0), (100.0, 100.0)));
        assert!(c.k.windows(2).all(|w| w[1].1 >= w[0].1));
        // Gris trop rouge : moins de magenta que de cyan au milieu.
        let (e, c) = calculer(&imprimer(None, ENGRAISSEMENT_DEFAUT, 4.0), ENGRAISSEMENT_DEFAUT, None).unwrap();
        assert!(e.gris > SEUIL_GRIS, "{e:?}");
        let (cm, mm) = (c.c[10].1, c.m[10].1);
        assert!(mm < cm, "magenta {mm} contre cyan {cm}");
        // Entrées refusées : rampe sans aplat, trop courte, ou mesure hors bornes.
        let sans_aplat: Vec<Mesure> = juste.iter().copied().filter(|m| m.pct < 100.0).collect();
        assert_eq!(calculer(&sans_aplat, ENGRAISSEMENT_DEFAUT, None).unwrap_err(), INVALIDES);
        let mut hors = juste.clone();
        hors[3].l = 120.0;
        assert!(calculer(&hors, ENGRAISSEMENT_DEFAUT, None).is_err());
        assert!(calculer(&juste, 0.8, None).is_err());
    }

    #[test]
    fn iterations_convergent_et_la_meilleure_reste() {
        let mut liste = vec![];
        let (g, derive) = (0.32, 5.0);
        let e1 = ajouter(&mut liste, "Epson / bâche", None, None, imprimer(None, g, derive), 1).unwrap();
        let premiere = e1.iterations[0].ecarts;
        let e2 = ajouter(&mut liste, "Epson / bâche", None, None, imprimer(Some(&e1.iterations[0].courbes), g, derive), 2).unwrap();
        let deuxieme = e2.iterations[1].ecarts;
        assert!(deuxieme.l < premiere.l / 3.0 && deuxieme.gris < premiere.gris, "{premiere:?} → {deuxieme:?}");
        assert_eq!(e2.meilleure().map(|i| i.numero), Some(2));
        // Une itération ratée (imprimée sans courbe) ne fait pas oublier la meilleure.
        let e3 = ajouter(&mut liste, "Epson / bâche", None, None, imprimer(None, g, derive), 3).unwrap();
        assert_eq!((e3.iterations.len(), e3.meilleure().map(|i| i.numero)), (3, Some(2)));
        assert_eq!(liste.len(), 1);
        // Au-delà du maximum, les plus anciennes partent, jamais la meilleure.
        for n in 4..(ITERATIONS_MAX as u64 + 6) {
            ajouter(&mut liste, "Epson / bâche", None, None, imprimer(None, g, derive), n).unwrap();
        }
        assert_eq!(liste[0].iterations.len(), ITERATIONS_MAX);
        assert_eq!(liste[0].meilleure().map(|i| i.numero), Some(2));
        assert_eq!(ajouter(&mut liste, " ", None, None, vec![], 0).unwrap_err(), "etalonnage_invalide");
    }
}
