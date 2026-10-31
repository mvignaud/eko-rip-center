//! Essai de bout en bout contre un vrai serveur : `EKO_RIP_ESSAI=http://127.0.0.1:8640
//! EKO_RIP_ESSAI_PDF=/chemin/test.pdf cargo test -- --ignored --nocapture`.

use crate::api::Client;
use crate::envoi::{Champs, Progression, televerser};
use crate::modele::{EtatSortie, StatutTravail};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

#[test]
#[ignore = "demande un serveur lancé (EKO_RIP_ESSAI)"]
fn parcours_complet_contre_le_serveur() {
    let (Ok(adresse), Ok(pdf)) = (std::env::var("EKO_RIP_ESSAI"), std::env::var("EKO_RIP_ESSAI_PDF")) else { panic!("EKO_RIP_ESSAI et EKO_RIP_ESSAI_PDF requis") };
    let c = Client::new(&adresse);
    let s = c.session().unwrap();
    println!("session : {s:?}");
    let reglages = c.reglages().unwrap();
    println!("réglages : {reglages:?}");
    let machines = c.machines().unwrap();
    println!("machines : {:?}", machines.iter().map(|m| (&m.id, m.operations())).collect::<Vec<_>>());
    println!("systeme : {:?}", c.systeme().unwrap());

    let p = Progression::default();
    let champs = Champs { reglage: reglages.first().map(|r| r.nom.clone()), client: "Client d'essai".into(), commande: "CMD-42".into(), quantite: Some(2) };
    televerser(&c, std::path::Path::new(&pdf), &champs, &p).unwrap();
    println!("envoyé : {} / {} octets", p.envoyes.load(Ordering::Relaxed), p.total.load(Ordering::Relaxed));

    let debut = Instant::now();
    let travail = loop {
        let liste = c.travaux().unwrap();
        let t = liste.first().cloned().expect("un travail");
        if matches!(t.statut, StatutTravail::Termine | StatutTravail::Echec) || debut.elapsed() > Duration::from_secs(60) {
            break t;
        }
        std::thread::sleep(Duration::from_millis(500));
    };
    println!("travail : {travail:?}");
    assert_eq!(travail.statut, StatutTravail::Termine, "{}", travail.statut_libelle());
    assert_eq!(travail.fiche.client, "Client d'essai");
    assert_eq!(travail.fiche.quantite, 2);
    assert_eq!(travail.dossier(), "CMD-42");
    assert!(!travail.pages.is_empty());

    let png = c.apercu(&travail.id, 1).unwrap();
    assert!(png.starts_with(b"\x89PNG"));
    let tif = std::env::temp_dir().join("eko-rip-essai-epreuve.tif");
    c.epreuve(&travail.id, 1, &tif).unwrap();
    println!("épreuve : {} octets", std::fs::metadata(&tif).unwrap().len());
    let _ = std::fs::remove_file(&tif);

    let imprimante = machines.iter().find(|m| m.imprime()).expect("une imprimante");
    c.imprimer(&travail.id, 1, &imprimante.id).unwrap();
    let file = c.file().unwrap();
    println!("file : {file:?}");
    let sortie = file.iter().find(|s| s.travail == travail.id).expect("sortie en file");
    assert!(sortie.etat.active() || sortie.etat == EtatSortie::Terminee);
    // Une action refusée rend le libellé français du code.
    let e = c.agir_sur_sortie(&sortie.id, "restaurer_n_importe_quoi").unwrap_err();
    println!("erreur : {e}");
    c.relancer("00000000-0000-0000-0000-000000000000").unwrap_err();
}

#[test]
#[ignore = "demande un serveur avec un compte (EKO_RIP_ESSAI, EKO_RIP_ESSAI_COMPTE=nom:mot_de_passe)"]
fn connexion_par_cookie() {
    let (Ok(adresse), Ok(compte)) = (std::env::var("EKO_RIP_ESSAI"), std::env::var("EKO_RIP_ESSAI_COMPTE")) else { panic!("EKO_RIP_ESSAI et EKO_RIP_ESSAI_COMPTE requis") };
    let (nom, mdp) = compte.split_once(':').unwrap();
    let c = Client::new(&adresse);
    let s = c.session().unwrap();
    assert!(!s.acces_libre && s.utilisateur.is_none());
    let e = c.travaux().unwrap_err();
    assert!(e.session_perdue(), "{e:?}");
    assert_eq!(e.message(), "Connexion requise.");
    let e = c.connecter(nom, "mauvais mot de passe").unwrap_err();
    assert_eq!(e.message(), "Utilisateur ou mot de passe incorrect.");
    c.connecter(nom, mdp).unwrap();
    assert_eq!(c.session().unwrap().utilisateur.as_deref(), Some(nom));
    c.travaux().unwrap();
    c.deconnecter().unwrap();
    assert!(c.travaux().unwrap_err().session_perdue());
}
