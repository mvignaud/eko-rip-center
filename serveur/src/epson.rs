//! Préparation de l'envoi à l'Epson SC-S80600, en attendant le protocole (capture à
//! l'atelier) : les plaques du rendu deviennent les canaux de l'imprimante.
//!
//! Mode contone (base ONYX, données seulement, et note publique Fiery XF) : cinq canaux
//! 8 bits, K C M Y puis WH ou MS ; l'imprimante répartit sur ses encres claires et trame.
//! Les tons de découpe ne partent jamais ; un autre ton direct n'a pas d'encre sur
//! l'Epson : l'envoi est refusé plutôt que d'imprimer sans lui.
//!
//! La capture du 24/10 a montré que l'Epson reçoit du raster tramé 2 bits sur 9 canaux,
//! pas du ton continu : voir `escp2.rs` ; cet en-tête sera repris par l'item 2.6.

use crate::decoupe;
use serde::{Deserialize, Serialize};

/// Canal de l'imprimante, dans l'ordre d'envoi.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize)]
pub enum Canal {
    K,
    C,
    M,
    Y,
    /// Blanc.
    WH,
    /// Argent métallisé.
    MS,
}

/// Où l'encre d'accompagnement (WH ou MS) se pose par rapport à la couleur.
#[derive(Clone, Copy, Default, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Couches {
    /// Sous la couleur : support transparent ou foncé vu de face, argent teinté.
    #[default]
    Dessous,
    /// Sur la couleur : vitrophanie vue à travers le support.
    Dessus,
}

/// Canal d'une plaque du rendu ; `None` pour un ton sans encre sur l'Epson.
/// Noms conseillés par Epson (« Printing with WH/MS ») : `White` et `Metallic`.
pub fn canal(plaque: &str) -> Option<Canal> {
    let n = plaque.trim().to_lowercase();
    Some(match n.as_str() {
        "black" => Canal::K,
        "cyan" => Canal::C,
        "magenta" => Canal::M,
        "yellow" => Canal::Y,
        "white" | "blanc" | "wh" => Canal::WH,
        "metallic" | "silver" | "ms" | "argent" => Canal::MS,
        _ => return None,
    })
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Envoi {
    /// Canaux présents dans l'ordre K C M Y puis WH ou MS, avec leur plaque.
    pub canaux: Vec<(Canal, String)>,
    pub couches: Couches,
}

/// Plaques d'une page → envoi. Refus : ton direct sans encre, ou WH et MS sur la même
/// page (les réglages Epson de la base ONYX vont par CMYKW ou CMYKMs, pas les deux).
pub fn preparer(plaques: &[String], couches: Couches) -> Result<Envoi, String> {
    let mut canaux = vec![];
    let mut sans_encre = vec![];
    for p in plaques.iter().filter(|p| !decoupe::est_ton_de_decoupe(p)) {
        match canal(p) {
            Some(c) if canaux.iter().any(|(x, _)| *x == c) => return Err(format!("deux plaques pour le canal {c:?} (dont « {p} »)")),
            Some(c) => canaux.push((c, p.clone())),
            None => sans_encre.push(format!("« {p} »")),
        }
    }
    if !sans_encre.is_empty() {
        return Err(format!("ton direct sans encre sur l'Epson : {} ; le convertir en CMJN, ou le nommer White ou Metallic", sans_encre.join(", ")));
    }
    canaux.sort();
    if canaux.iter().any(|(c, _)| *c == Canal::WH) && canaux.iter().any(|(c, _)| *c == Canal::MS) {
        return Err("blanc et argent sur la même page : un passage par encre".into());
    }
    if canaux.is_empty() {
        return Err("rien à imprimer : la page n'a que des tons de découpe".into());
    }
    Ok(Envoi { canaux, couches })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noms(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn plaques_vers_canaux_sans_la_decoupe() {
        let e = preparer(&noms(&["Cyan", "CutContour", "Magenta", "White", "Black", "Yellow", "Kiss-Cut"]), Couches::Dessus).unwrap();
        let ordre: Vec<_> = e.canaux.iter().map(|(c, _)| *c).collect();
        assert_eq!(ordre, [Canal::K, Canal::C, Canal::M, Canal::Y, Canal::WH]);
        assert!(e.canaux.iter().all(|(_, p)| !decoupe::est_ton_de_decoupe(p)), "jamais de plaque de découpe");
        assert_eq!(e.couches, Couches::Dessus);
        assert_eq!(preparer(&noms(&["Argent", "Cyan"]), Couches::Dessous).unwrap().canaux, [(Canal::C, "Cyan".into()), (Canal::MS, "Argent".into())]);
    }

    #[test]
    fn refus_explicites() {
        assert!(preparer(&noms(&["Cyan", "PANTONE 485 C"]), Couches::Dessous).unwrap_err().contains("PANTONE 485 C"));
        assert!(preparer(&noms(&["White", "Silver"]), Couches::Dessous).unwrap_err().contains("blanc et argent"));
        assert!(preparer(&noms(&["White", "Blanc"]), Couches::Dessous).unwrap_err().contains("WH"));
        assert!(preparer(&noms(&["CutContour"]), Couches::Dessous).is_err());
    }
}
