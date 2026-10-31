//! Ordres envoyés par les vues (tableau, inspecteur, cartes des machines…) à la fenêtre, qui
//! seule parle au serveur.

use crate::api::Erreur;
use std::path::PathBuf;

pub enum Commande {
    /// Relit tout de suite le serveur.
    Rafraichir,
    /// Travail choisi dans le tableau (`None` : plus aucun).
    Choisir(Option<String>),
    /// Demande l'aperçu d'une page.
    Apercu { travail: String, page: usize },
    Imprimer { travail: String, page: usize, machine: String },
    /// Boutons de la barre d'outils : agissent sur le travail choisi.
    ImprimerSelection,
    EpreuveSelection,
    SuspendreSelection,
    Epreuve { travail: String, page: usize },
    Relancer(String),
    Supprimer(String),
    /// Action sur une sortie de la file (`suspendre`, `reprendre`, `annuler`, `en_tete`…).
    Sortie { id: String, action: &'static str },
    /// Fichiers à envoyer (bouton +, glisser-déposer, ouverture depuis le système).
    Envoyer(Vec<PathBuf>),
    Erreur(Erreur),
    Message(String),
}
