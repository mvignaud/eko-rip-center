//! Motif de nom de fichier à balises (roadmap 9.10) : dans le dossier `entrees-auto/`, le nom
//! d'un fichier peut porter le réglage, la machine, les copies… Le motif dit où chaque balise
//! se trouve, par exemple `{nom}__{machine}__{reglage}__{copies}` pour
//! `Affiche__epson__bache__30.pdf`.
//!
//! Le motif alterne balises et séparateurs ; deux balises doivent être séparées (le double
//! `__` est conseillé : un `_` seul se trouve souvent dans les noms). Chaque balise prend le
//! texte jusqu'à la première occurrence du séparateur qui la suit ; la dernière va jusqu'à
//! l'extension, qui ne compte pas. Un nom qui ne suit pas le motif n'est pas refusé : il
//! passe par les règles, avec une alerte.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Ce qu'une balise du nom renseigne.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Balise {
    /// Texte libre, sans effet (le nom du visuel).
    Nom,
    /// Réglage enregistré (nom sans casse ni accents).
    Reglage,
    /// Machine du catalogue (identifiant ou nom).
    Machine,
    /// Support du catalogue (nom).
    Support,
    /// Copies dues (fiche du travail).
    Copies,
    /// Impressions de chaque page mises en file d'office.
    Repetition,
    /// Laize de la mosaïque du réglage (mm).
    Laize,
    /// Échelle (%).
    Echelle,
    /// Largeur voulue (mm) : l'échelle en découle.
    Largeur,
    /// Hauteur voulue (mm).
    Hauteur,
}

impl Balise {
    const TOUTES: [(&'static str, Balise); 10] = [
        ("nom", Balise::Nom),
        ("reglage", Balise::Reglage),
        ("machine", Balise::Machine),
        ("support", Balise::Support),
        ("copies", Balise::Copies),
        ("repetition", Balise::Repetition),
        ("laize", Balise::Laize),
        ("echelle", Balise::Echelle),
        ("largeur", Balise::Largeur),
        ("hauteur", Balise::Hauteur),
    ];

    fn depuis(nom: &str) -> Option<Balise> {
        Self::TOUTES.iter().find(|(n, _)| *n == cle(nom)).map(|(_, b)| *b)
    }

    pub fn nom(self) -> &'static str {
        Self::TOUTES.iter().find(|(_, b)| *b == self).map(|(n, _)| *n).unwrap_or_default()
    }
}

/// Réglage du serveur : le motif, vide quand les noms ne portent rien.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub motif: String,
}

#[derive(Debug, PartialEq)]
enum Morceau {
    Texte(String),
    Balise(Balise),
}

/// Découpe le motif ; `Err` : code d'erreur traduit par l'écran.
fn compiler(motif: &str) -> Result<Vec<Morceau>, &'static str> {
    let mut morceaux = vec![];
    let mut reste = motif;
    while !reste.is_empty() {
        match reste.find('{') {
            Some(0) => {
                let fin = reste.find('}').ok_or("motif_balises_invalide")?;
                let b = Balise::depuis(&reste[1..fin]).ok_or("motif_balises_invalide")?;
                // Deux balises collées : rien ne dit où finit la première.
                if matches!(morceaux.last(), Some(Morceau::Balise(_))) {
                    return Err("motif_balises_invalide");
                }
                morceaux.push(Morceau::Balise(b));
                reste = &reste[fin + 1..];
            }
            Some(i) => {
                morceaux.push(Morceau::Texte(reste[..i].to_string()));
                reste = &reste[i..];
            }
            None => {
                morceaux.push(Morceau::Texte(reste.to_string()));
                reste = "";
            }
        }
    }
    if morceaux.iter().any(|m| matches!(m, Morceau::Texte(t) if t.contains(['}', '/', '\\']))) {
        return Err("motif_balises_invalide");
    }
    Ok(morceaux)
}

/// Vérifie le motif : vide, ou au moins une balise, 200 caractères au plus, chaque balise
/// une seule fois.
pub fn verifier(c: &Config) -> Result<(), &'static str> {
    if c.motif.is_empty() {
        return Ok(());
    }
    let m = compiler(&c.motif)?;
    let balises: Vec<Balise> = m.iter().filter_map(|x| if let Morceau::Balise(b) = x { Some(*b) } else { None }).collect();
    let uniques = balises.iter().enumerate().all(|(i, b)| !balises[..i].contains(b));
    if c.motif.chars().count() > 200 || balises.is_empty() || !uniques {
        return Err("motif_balises_invalide");
    }
    Ok(())
}

/// Valeurs lues dans le nom, dans l'ordre du motif ; `None` : le nom ne suit pas le motif.
/// L'extension (dernier point) ne compte pas.
pub fn extraire(motif: &str, fichier: &str) -> Option<Vec<(Balise, String)>> {
    let morceaux = compiler(motif).ok()?;
    let mut reste = fichier.rsplit_once('.').map_or(fichier, |(base, _)| base);
    let mut valeurs = vec![];
    let mut i = 0;
    while i < morceaux.len() {
        match &morceaux[i] {
            Morceau::Texte(t) => reste = reste.strip_prefix(t.as_str())?,
            Morceau::Balise(b) => {
                let fin = match morceaux.get(i + 1) {
                    Some(Morceau::Texte(t)) => reste.find(t.as_str())?,
                    _ => reste.len(),
                };
                let v = reste[..fin].trim();
                if v.is_empty() {
                    return None;
                }
                valeurs.push((*b, v.to_string()));
                reste = &reste[fin..];
            }
        }
        i += 1;
    }
    reste.is_empty().then_some(valeurs)
}

/// Clé de comparaison : minuscules, sans accents, sans espaces, `-` ni `_` (« Bâche XXL »
/// et « bache-xxl » se valent).
pub fn cle(texte: &str) -> String {
    texte
        .to_lowercase()
        .chars()
        .filter(|c| !matches!(c, ' ' | '-' | '_'))
        .map(|c| match c {
            'à' | 'â' | 'ä' | 'á' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' => 'i',
            'ô' | 'ö' | 'ó' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            'ÿ' => 'y',
            c => c,
        })
        .collect()
}

/// Nombre d'une balise : virgule ou point décimal, unité (`mm`, `%`) tolérée à la fin.
pub fn nombre(texte: &str, unite: &str) -> Option<f64> {
    let t = texte.trim().to_lowercase();
    let t = t.strip_suffix(unite).unwrap_or(&t).trim().replace(',', ".");
    t.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// Entier positif d'une balise (copies, répétition), borné.
pub fn entier(texte: &str, max: u32) -> Option<u32> {
    texte.trim().parse::<u32>().ok().filter(|n| (1..=max).contains(n))
}

pub fn fichier(donnees: &Path) -> PathBuf {
    donnees.join("balises.json")
}

pub fn lire(donnees: &Path) -> Config {
    crate::etat::lire(&fichier(donnees)).ok().flatten().unwrap_or_default()
}

pub fn ecrire(donnees: &Path, c: &Config) -> std::io::Result<()> {
    crate::etat::ecrire(&fichier(donnees), c, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOTIF: &str = "{nom}__{machine}__{reglage}__{copies}";

    #[test]
    fn extraction() {
        let v = extraire(MOTIF, "Affiche__epson__bache__30.pdf").unwrap();
        assert_eq!(v, [(Balise::Nom, "Affiche".into()), (Balise::Machine, "epson".into()), (Balise::Reglage, "bache".into()), (Balise::Copies, "30".into())]);
        // Un « _ » seul reste dans la balise : le séparateur est le double.
        assert_eq!(extraire(MOTIF, "Vitrine_ete__epson__vinyle__2.tif").unwrap()[0].1, "Vitrine_ete");
        assert_eq!(extraire("{nom}", "sans extension").unwrap()[0].1, "sans extension");
        assert_eq!(extraire("CDE-{copies}x-{nom}", "CDE-12x-Panneau.pdf").unwrap(), [(Balise::Copies, "12".into()), (Balise::Nom, "Panneau".into())]);
        for faux in ["Affiche.pdf", "Affiche__epson__bache.pdf", "Affiche__epson____30.pdf"] {
            assert_eq!(extraire(MOTIF, faux), None, "{faux}");
        }
        // La dernière balise prend le reste : sa valeur sera refusée, avec une alerte.
        assert_eq!(extraire(MOTIF, "Affiche__epson__bache__30__en_trop.pdf").unwrap()[3].1, "30__en_trop");
    }

    #[test]
    fn motifs_valides() {
        let ok = |m: &str| verifier(&Config { motif: m.into() });
        assert_eq!(ok(""), Ok(()), "vide : pas de balises");
        assert_eq!(ok(MOTIF), Ok(()));
        assert_eq!(ok("{Réglage}__{Échelle}%"), Ok(()), "casse et accents ignorés");
        for faux in ["{nom}{copies}", "{inconnue}", "{nom}__{nom}", "sans balise", "{nom", "a/{nom}", "x}{nom}"] {
            assert_eq!(ok(faux), Err("motif_balises_invalide"), "{faux}");
        }
    }

    #[test]
    fn valeurs() {
        assert_eq!(cle("Bâche XXL"), cle("bache-xxl"));
        assert_eq!(nombre("1600mm", "mm"), Some(1600.0));
        assert_eq!(nombre("12,5 %", "%"), Some(12.5));
        assert_eq!(nombre("abc", "mm"), None);
        assert_eq!(entier("30", 100_000), Some(30));
        assert_eq!((entier("0", 10), entier("11", 10), entier("2.5", 10)), (None, None, None));
    }
}
