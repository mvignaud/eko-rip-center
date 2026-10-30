//! Décodage de réponses réelles du serveur, capturées avec
//! `curl -A "EKO-RIP-Client/1" http://127.0.0.1:8640/api/…` (dossier `tests/reponses/`).

use super::*;

#[test]
fn session_libre() {
    let s: Session = serde_json::from_str(include_str!("../../tests/reponses/session.json")).unwrap();
    assert!(s.acces_libre);
    assert_eq!(s.utilisateur, None);
    assert_eq!(s.role.as_deref(), Some("admin"));
}

#[test]
fn travaux() {
    let t: Vec<Travail> = serde_json::from_str(include_str!("../../tests/reponses/travaux.json")).unwrap();
    assert_eq!(t.len(), 2);
    // Les plus récents d'abord : l'envoi cassé, puis le PDF rendu.
    let casse = &t[0];
    assert_eq!(casse.nom, "casse.pdf");
    assert_eq!(casse.statut, StatutTravail::Echec);
    assert_eq!(casse.code_erreur.as_deref(), Some("rendu_echoue"));
    assert_eq!(casse.statut_libelle(), "Échec : Le rendu a échoué.");
    let ok = &t[1];
    assert_eq!(ok.statut, StatutTravail::Termine);
    assert_eq!(ok.pages.len(), 1);
    assert_eq!(ok.pages[0].plaques, ["Black", "Cyan", "Magenta", "Yellow"]);
    assert!((ok.pages[0].largeur_mm - 209.8).abs() < 0.1);
    assert_eq!(ok.fiche.client, "Client d'essai");
    assert_eq!(ok.fiche.quantite, 2);
    assert_eq!(ok.dossier(), "CMD-42");
    assert_eq!(ok.alertes[0].code, "police_non_incorporee");
    assert_eq!(crate::libelles::alerte(&ok.alertes[0].code, &ok.alertes[0].detail, ok.alertes[0].page), "Police non incorporée : Helvetica — une autre sera prise au rendu. Vectoriser le texte ou incorporer la police.");
    assert!(ok.cree_le > 1_700_000_000);
}

#[test]
fn reponse_d_envoi() {
    let t: Travail = serde_json::from_str(include_str!("../../tests/reponses/envoi-202.json")).unwrap();
    assert_eq!(t.nom, "casse.pdf");
    assert!(matches!(t.statut, StatutTravail::EnAttente | StatutTravail::EnCours | StatutTravail::Echec));
}

#[test]
fn file() {
    let f: Vec<Sortie> = serde_json::from_str(include_str!("../../tests/reponses/file.json")).unwrap();
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].machine, "epson");
    assert_eq!(f[0].etat, EtatSortie::Terminee);
    assert_eq!(f[0].operation.as_deref(), Some("impression"));
    assert_eq!(f[0].page, 1);
    assert!(!f[0].etat.active());
    assert_eq!(f[0].etat_libelle(), "Terminée");
}

#[test]
fn machines() {
    let m: Vec<Machine> = serde_json::from_str(include_str!("../../tests/reponses/machines.json")).unwrap();
    assert_eq!(m.len(), 2);
    assert_eq!((m[0].id.as_str(), m[0].imprime(), m[0].decoupe()), ("epson", true, false));
    assert_eq!((m[1].id.as_str(), m[1].imprime(), m[1].decoupe()), ("kona", false, true));
    assert_eq!(m[0].operations(), "Impression");
    assert_eq!(m[1].modele_libelle(), "Mutoh Kona");
    assert!((m[1].laize_mm - 1650.0).abs() < f64::EPSILON);
}

#[test]
fn systeme() {
    let s: Systeme = serde_json::from_str(include_str!("../../tests/reponses/systeme.json")).unwrap();
    assert_eq!(s.version, "0.28.0");
    assert_eq!(s.rendus.simultanes, 4);
    assert!(s.disque_total.unwrap() > s.disque_libre.unwrap());
    assert_eq!(s.alertes.len(), 1);
    assert!(s.alertes[0].libelle().ends_with("absent : recréé"));
}

#[test]
fn reglages() {
    let r: Vec<Reglage> = serde_json::from_str(include_str!("../../tests/reponses/reglages.json")).unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].nom, "Vinyle test");
}

#[test]
fn erreurs_de_l_api() {
    use crate::api::Erreur;
    let corps: serde_json::Value = serde_json::from_str(include_str!("../../tests/reponses/erreur-introuvable.json")).unwrap();
    let e = Erreur { statut: 404, code: corps["code"].as_str().unwrap().into(), detail: None };
    assert_eq!(e.message(), "Élément introuvable.");
    let e = Erreur { statut: 403, code: "client_requis".into(), detail: None };
    assert_eq!(e.message(), "Réservé au client E-KO RIP Center.");
    // Code inconnu : le code et le détail.
    let e = Erreur { statut: 500, code: "code_futur".into(), detail: Some("détail".into()) };
    assert_eq!(e.message(), "code_futur : détail");
    let e = Erreur { statut: 401, code: "connexion_requise".into(), detail: None };
    assert!(e.session_perdue());
    assert_eq!(Erreur { statut: 422, code: "http_422".into(), detail: None }.message(), "Réponse inattendue du serveur (HTTP 422).");
    assert_eq!(Erreur::locale("reseau", "refusé".to_string()).message(), "Serveur injoignable.");
}

#[test]
fn unites() {
    assert_eq!(taille(512), "512 o");
    assert_eq!(taille(1536), "1,5 Ko");
    assert_eq!(taille(5 * 1024 * 1024 * 1024), "5,0 Go");
}
