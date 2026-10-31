//! « Signaler un problème… » : corps JSON de `POST /api/rapports` et fenêtre de saisie.
//! L'envoi lui-même part dans un fil à part (voir `Fenetre::signaler`).

use crate::api::Erreur;

pub const MAX_TITRE: usize = 120;
pub const MAX_DESCRIPTION: usize = 5000;
pub const MAX_EMAIL: usize = 120;

/// Corps JSON du rapport. Champs rognés ; l'e-mail vide est omis.
pub fn corps(titre: &str, description: &str, email: &str, joindre_journal: bool, version: &str, os: &str) -> serde_json::Value {
    let rogner = |s: &str, max: usize| s.trim().chars().take(max).collect::<String>();
    let mut v = serde_json::json!({
        "titre": rogner(titre, MAX_TITRE),
        "description": rogner(description, MAX_DESCRIPTION),
        "joindre_journal": joindre_journal,
        "client": { "type": "client-linux", "version": version, "os": os },
    });
    let email = rogner(email, MAX_EMAIL);
    if !email.is_empty() {
        v["email"] = email.into();
    }
    v
}

/// Message d'erreur : libellé habituel du code s'il existe, sinon « Envoi impossible : <code> ».
pub fn message_erreur(e: &Erreur) -> String {
    if e.code.starts_with("http_") || crate::libelles::libelle(&e.code).is_some() {
        e.message()
    } else {
        format!("Envoi impossible : {}", e.code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corps_complet() {
        let v = corps("  Titre ", "Description", " a@b.fr ", true, "0.30.0", "Ubuntu 26.04");
        assert_eq!(
            v,
            serde_json::json!({
                "titre": "Titre", "description": "Description", "email": "a@b.fr", "joindre_journal": true,
                "client": { "type": "client-linux", "version": "0.30.0", "os": "Ubuntu 26.04" },
            })
        );
    }

    #[test]
    fn email_vide_omis_et_longueurs_bornees() {
        let v = corps(&"é".repeat(200), &"d".repeat(6000), "  ", false, "1", "Linux");
        assert!(v.get("email").is_none());
        assert_eq!(v["joindre_journal"], false);
        assert_eq!(v["titre"].as_str().unwrap().chars().count(), MAX_TITRE);
        assert_eq!(v["description"].as_str().unwrap().chars().count(), MAX_DESCRIPTION);
    }

    #[test]
    fn erreurs() {
        assert_eq!(message_erreur(&Erreur::locale("rapports_sature", None)), "Envoi impossible : rapports_sature");
        assert_eq!(message_erreur(&Erreur::locale("connexion_requise", None)), "Connexion requise.");
    }
}
