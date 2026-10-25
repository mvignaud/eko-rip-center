//! Table de tons directs (roadmap 8.6) : nom exact → couleur Lab (D50), gardée dans
//! `tons_directs.json`. Un ton du PDF trouvé ici est imprimé en CMJN d'après sa valeur Lab
//! (et non d'après l'alternative du PDF) ; un ton absent garde le comportement d'avant.
//!
//! La recherche respecte la casse **et** les espaces : « PANTONE 485 C » ne trouve pas
//! « Pantone 485 C » ni « PANTONE  485 C » (un alias les ajoute au besoin). Aucune
//! bibliothèque de couleurs n'est livrée : l'atelier importe celle de son fournisseur
//! (CSV Lab), jamais une bibliothèque sous licence qu'il n'a pas.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ton {
    pub nom: String,
    /// Autres noms exacts du même ton dans les fichiers reçus.
    #[serde(default)]
    pub alias: Vec<String>,
    /// L* (0 à 100), a*, b* (−128 à 127), illuminant D50.
    pub lab: [f64; 3],
}

pub fn fichier(donnees: &Path) -> PathBuf {
    donnees.join("tons_directs.json")
}

pub fn lire(donnees: &Path) -> Vec<Ton> {
    crate::etat::lire(&fichier(donnees)).ok().flatten().unwrap_or_default()
}

pub fn ecrire(donnees: &Path, table: &[Ton]) -> std::io::Result<()> {
    crate::etat::ecrire(&fichier(donnees), &table, false)
}

/// Ton de la table pour ce nom de séparation (nom ou alias, à l'identique).
pub fn trouver<'a>(table: &'a [Ton], nom: &str) -> Option<&'a Ton> {
    table.iter().find(|t| t.nom == nom || t.alias.iter().any(|a| a == nom))
}

/// Refus (code, détail) : nom vide, Lab hors bornes, nom en double (alias compris), ou nom
/// qui a déjà sa propre encre ou sert à la découpe (la table ne doit pas l'avaler).
pub fn verifier(table: &[Ton]) -> Result<(), (&'static str, String)> {
    let mut vus = std::collections::HashSet::new();
    for t in table {
        let [l, a, b] = t.lab;
        if !(0.0..=100.0).contains(&l) || !(-128.0..=127.0).contains(&a) || !(-128.0..=127.0).contains(&b) {
            return Err(("lab_invalide", t.nom.clone()));
        }
        for n in std::iter::once(&t.nom).chain(&t.alias) {
            if n.trim().is_empty() {
                return Err(("nom_vide", t.nom.clone()));
            }
            if crate::epson::canal(n).is_some() || crate::decoupe::est_ton_de_decoupe(n) {
                return Err(("nom_reserve", n.clone()));
            }
            if !vus.insert(n.as_str()) {
                return Err(("nom_en_double", n.clone()));
            }
        }
    }
    Ok(())
}

/// Lit un CSV `nom;L;a;b` (séparateur `;`, tabulation ou `,` ; avec `;` ou tabulation, la
/// virgule décimale est admise). Une première ligne non numérique est un en-tête. Le nom
/// est gardé tel quel, espaces compris. Refus : ligne illisible (n° de ligne).
pub fn lire_csv(texte: &str) -> Result<Vec<Ton>, String> {
    let texte = texte.strip_prefix('\u{feff}').unwrap_or(texte);
    let mut tons = vec![];
    for (i, ligne) in texte.lines().enumerate() {
        let ligne = ligne.strip_suffix('\r').unwrap_or(ligne);
        if ligne.trim().is_empty() {
            continue;
        }
        let sep = if ligne.contains(';') { ';' } else if ligne.contains('\t') { '\t' } else { ',' };
        let champs: Vec<&str> = ligne.split(sep).collect();
        let nombre = |s: &str| if sep == ',' { s.trim().parse::<f64>().ok() } else { s.trim().replace(',', ".").parse::<f64>().ok() };
        let lab = match champs.as_slice() {
            [_, l, a, b] => [nombre(l), nombre(a), nombre(b)],
            _ => [None; 3],
        };
        match lab {
            [Some(l), Some(a), Some(b)] => tons.push(Ton { nom: champs[0].into(), alias: vec![], lab: [l, a, b] }),
            _ if i == 0 => continue,
            _ => return Err(format!("ligne {} : attendu nom;L;a;b", i + 1)),
        }
    }
    Ok(tons)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ton(nom: &str, lab: [f64; 3]) -> Ton {
        Ton { nom: nom.into(), alias: vec![], lab }
    }

    #[test]
    fn csv_cinquante_teintes_et_formats() {
        let mut csv = String::from("\u{feff}Nom;L*;a*;b*\r\n");
        for i in 0..50 {
            csv.push_str(&format!("HX {i:02} Rouge;{},5;-{i},25;12\r\n", 40 + i % 10));
        }
        let t = lire_csv(&csv).unwrap();
        assert_eq!(t.len(), 50);
        assert_eq!(t[3], ton("HX 03 Rouge", [43.5, -3.25, 12.0]));
        assert_eq!(lire_csv("Bleu fonce,30.1,5,-40").unwrap(), [ton("Bleu fonce", [30.1, 5.0, -40.0])]);
        assert_eq!(lire_csv(" Or\t70\t5\t60").unwrap()[0].nom, " Or", "nom gardé tel quel");
        assert_eq!(lire_csv("a;1;2;3\nb;x;2;3").unwrap_err(), "ligne 2 : attendu nom;L;a;b");
        assert!(verifier(&t).is_ok());
    }

    #[test]
    fn recherche_exacte_et_refus() {
        let mut t = ton("PANTONE 485 C", [50.0, 70.0, 55.0]);
        t.alias = vec!["Pantone 485 C".into()];
        let table = [t];
        assert!(trouver(&table, "PANTONE 485 C").is_some());
        assert!(trouver(&table, "Pantone 485 C").is_some(), "alias");
        assert!(trouver(&table, "pantone 485 c").is_none(), "casse exacte");
        assert!(trouver(&table, "PANTONE  485 C").is_none(), "espaces exacts");
        let double = [ton("A", [50.0, 0.0, 0.0]), Ton { nom: "B".into(), alias: vec!["A".into()], lab: [50.0, 0.0, 0.0] }];
        assert_eq!(verifier(&double), Err(("nom_en_double", "A".into())));
        assert_eq!(verifier(&[ton("White", [95.0, 0.0, 0.0])]), Err(("nom_reserve", "White".into())));
        assert_eq!(verifier(&[ton("CutContour", [50.0, 0.0, 0.0])]), Err(("nom_reserve", "CutContour".into())));
        assert_eq!(verifier(&[ton("X", [101.0, 0.0, 0.0])]), Err(("lab_invalide", "X".into())));
        assert_eq!(verifier(&[ton(" ", [50.0, 0.0, 0.0])]), Err(("nom_vide", " ".into())));
    }
}
