//! Consommation réelle d'une imprimante, déduite de relevés SNMP successifs
//! (niveaux d'encre en % et longueur restante du rouleau), gardés dans
//! `releves/<machine>.jsonl`, une ligne par relevé qui change quelque chose.
//!
//! Seules les baisses comptent : une hausse est un changement de cartouche ou de rouleau.
//! ponytail: l'imprimante ne donne l'encre qu'au % près (≈ 7 à 15 ml selon la poche) ;
//! la consommation par travail viendra avec le pilote d'impression (fenêtre début/fin).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Releve {
    /// Secondes depuis l'époque Unix.
    pub le: u64,
    /// (encre, % restant).
    pub encres: Vec<(String, u8)>,
    pub longueur_m: Option<f64>,
}

#[derive(Debug, Default, PartialEq, Serialize)]
pub struct Consommation {
    /// Points de % consommés par encre.
    pub encres: BTreeMap<String, u32>,
    /// Mètres de support consommés.
    pub metres: f64,
    /// Nombre de relevés utilisés, et leur étendue (secondes Unix).
    pub releves: usize,
    pub du: Option<u64>,
    pub au: Option<u64>,
}

fn fichier(donnees: &Path, machine: &str) -> PathBuf {
    donnees.join("releves").join(format!("{machine}.jsonl"))
}

/// Ajoute un relevé au journal de la machine.
pub fn ajouter(donnees: &Path, machine: &str, r: &Releve) -> std::io::Result<()> {
    let f = fichier(donnees, machine);
    std::fs::create_dir_all(f.parent().unwrap())?;
    let mut o = std::fs::OpenOptions::new().create(true).append(true).open(f)?;
    writeln!(o, "{}", serde_json::to_string(r)?)
}

/// Relevés de la machine, dans l'ordre ; les lignes illisibles sont ignorées.
pub fn lire(donnees: &Path, machine: &str) -> Vec<Releve> {
    std::fs::read_to_string(fichier(donnees, machine)).unwrap_or_default().lines().filter_map(|l| serde_json::from_str(l).ok()).collect()
}

/// Même état (encres et longueur), quelle que soit l'heure.
pub fn identique(a: &Releve, b: &Releve) -> bool {
    a.encres == b.encres && a.longueur_m == b.longueur_m
}

/// Consommation entre `depuis` et `jusqua` : le dernier relevé d'avant `depuis` sert de départ.
pub fn consommation(releves: &[Releve], depuis: u64, jusqua: u64) -> Consommation {
    let debut = releves.iter().rposition(|r| r.le <= depuis).unwrap_or(0);
    let fenetre: Vec<&Releve> = releves.iter().skip(debut).take_while(|r| r.le <= jusqua).collect();
    let mut c = Consommation { releves: fenetre.len(), du: fenetre.first().map(|r| r.le), au: fenetre.last().map(|r| r.le), ..Default::default() };
    for w in fenetre.windows(2) {
        let (a, b) = (w[0], w[1]);
        for (nom, pc) in &b.encres {
            if let Some((_, avant)) = a.encres.iter().find(|(n, _)| n == nom) {
                *c.encres.entry(nom.clone()).or_default() += avant.saturating_sub(*pc) as u32;
            }
        }
        if let (Some(x), Some(y)) = (a.longueur_m, b.longueur_m) {
            c.metres += (x - y).max(0.0);
        }
    }
    c.metres = (c.metres * 1000.0).round() / 1000.0;
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(le: u64, bk: u8, c: u8, m: f64) -> Releve {
        Releve { le, encres: vec![("BK".into(), bk), ("C".into(), c)], longueur_m: Some(m) }
    }

    #[test]
    fn seules_les_baisses_comptent() {
        // BK 80→78, cartouche C changée (40→100) puis 100→99, rouleau changé (5→50) puis 50→49,5.
        let v = [r(0, 90, 40, 30.0), r(100, 80, 40, 10.0), r(200, 78, 100, 5.0), r(300, 78, 99, 50.0), r(400, 78, 99, 49.5)];
        let c = consommation(&v, 100, 400);
        assert_eq!(c.encres, BTreeMap::from([("BK".into(), 2), ("C".into(), 1)]));
        assert_eq!(c.metres, 5.5);
        assert_eq!((c.releves, c.du, c.au), (4, Some(100), Some(400)));
    }

    #[test]
    fn journal() {
        let d = std::env::temp_dir().join(format!("releves-{}", uuid::Uuid::new_v4()));
        ajouter(&d, "epson", &r(1, 50, 50, 10.0)).unwrap();
        ajouter(&d, "epson", &r(2, 49, 50, 9.0)).unwrap();
        assert_eq!(lire(&d, "epson").len(), 2);
        assert!(lire(&d, "kona").is_empty());
        std::fs::remove_dir_all(d).unwrap();
    }
}
