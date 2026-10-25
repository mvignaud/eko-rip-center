//! Sauvegarde de la configuration en un seul fichier JSON : catalogue des machines, tarifs,
//! réglages enregistrés, supports et modes, profils ICC (en hexadécimal) et réglage du pont ERP
//! (sans son secret). Ni les comptes (mots de passe
//! hachés : ils se recréent sur le serveur), ni les travaux, ni la file.

use crate::{couverture, machines, reglages, supports};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const FORMAT: &str = "eko-rip-sauvegarde-1";

#[derive(Serialize, Deserialize)]
pub struct Sauvegarde {
    pub format: String,
    /// Version du serveur qui l'a faite.
    pub version: String,
    pub cree_le: u64,
    pub machines: Vec<machines::Machine>,
    pub tarifs: couverture::Tarifs,
    pub reglages: Vec<reglages::Reglage>,
    /// Nom du fichier → contenu en hexadécimal.
    pub profils: BTreeMap<String, String>,
    /// Absent des sauvegardes d'avant la 0.14 : les supports en place sont alors gardés.
    #[serde(default)]
    pub supports: Option<supports::Catalogue>,
    /// Préfixes et options de découpe (`decoupe.json`) ; absent des sauvegardes d'avant.
    #[serde(default)]
    pub decoupe: Option<crate::decoupe::Reglages>,
    /// Rétention du journal et facteurs de correction (`consommation.json`) ; absent des
    /// sauvegardes d'avant.
    #[serde(default)]
    pub consommation: Option<crate::consommation::Reglages>,
    /// Pont ERP : URL, activation, instance (ni jeton ni secret) ; absent des sauvegardes
    /// d'avant le lot R2 : le pont en place est gardé.
    #[serde(default)]
    pub pont: Option<crate::erp::Sauve>,
}

pub fn hex(octets: &[u8]) -> String {
    octets.iter().map(|o| format!("{o:02x}")).collect()
}

pub fn depuis_hex(texte: &str) -> Option<Vec<u8>> {
    if !texte.len().is_multiple_of(2) {
        return None;
    }
    (0..texte.len()).step_by(2).map(|i| u8::from_str_radix(texte.get(i..i + 2)?, 16).ok()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_aller_retour() {
        let o: Vec<u8> = (0..=255).collect();
        assert_eq!(depuis_hex(&hex(&o)).unwrap(), o);
        assert_eq!(depuis_hex("abc"), None);
        assert_eq!(depuis_hex("zz"), None);
        assert_eq!(depuis_hex("é0"), None);
    }
}
