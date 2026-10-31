//! Envoi de fichiers : corps `multipart/form-data` construit en flux (un fichier de plusieurs Go
//! ne passe jamais en mémoire), progression lue par la feuille d'envoi, et la feuille elle-même.

use crate::api::{Client, Erreur, Resultat};
use crate::commande::Commande;
use adw::prelude::*;
use async_channel::Sender;
use gtk::{gio, glib};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

/// Extensions que le serveur accepte (PDF, SVG, TIFF, PNG, JPEG).
pub const EXTENSIONS: [&str; 7] = ["pdf", "svg", "tif", "tiff", "png", "jpg", "jpeg"];

pub fn extension_admise(chemin: &Path) -> bool {
    chemin.extension().and_then(|e| e.to_str()).is_some_and(|e| EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

/// Champs de texte envoyés avant le fichier.
#[derive(Clone, Default)]
pub struct Champs {
    pub reglage: Option<String>,
    pub client: String,
    pub commande: String,
    pub quantite: Option<u32>,
}

/// Avancement partagé entre le fil d'envoi et la feuille.
#[derive(Default)]
pub struct Progression {
    /// Fichier en cours (à partir de 0).
    pub fichier: AtomicUsize,
    pub envoyes: AtomicU64,
    pub total: AtomicU64,
    pub annule: AtomicBool,
    pub fini: AtomicBool,
}

/// Corps de la requête : en-têtes des champs, fichier lu au fil de l'eau, pied.
struct Flux<'a> {
    tete: Cursor<Vec<u8>>,
    fichier: std::fs::File,
    queue: Cursor<Vec<u8>>,
    fichier_fini: bool,
    progression: Option<&'a Progression>,
}

impl Read for Flux<'_> {
    fn read(&mut self, tampon: &mut [u8]) -> std::io::Result<usize> {
        if let Some(p) = self.progression
            && p.annule.load(Ordering::Relaxed)
        {
            return Err(std::io::Error::other("envoi annulé"));
        }
        let n = self.tete.read(tampon)?;
        if n > 0 {
            return Ok(n);
        }
        if !self.fichier_fini {
            let n = self.fichier.read(tampon)?;
            if n > 0 {
                if let Some(p) = self.progression {
                    p.envoyes.fetch_add(n as u64, Ordering::Relaxed);
                }
                return Ok(n);
            }
            self.fichier_fini = true;
        }
        self.queue.read(tampon)
    }
}

fn frontiere() -> String {
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    format!("----EKORIPClient{t:x}{:x}", std::process::id())
}

/// Valeur de `filename="…"` : sans guillemet ni saut de ligne.
fn nom_pour_entete(chemin: &Path) -> String {
    chemin.file_name().map_or_else(|| "sans nom".into(), |n| n.to_string_lossy().replace(['"', '\r', '\n', '\\'], "_"))
}

/// Construit le corps pour `chemin` : (flux, longueur totale, frontière). `fichier` vient en dernier.
fn corps<'a>(chemin: &Path, champs: &Champs, progression: Option<&'a Progression>) -> std::io::Result<(Flux<'a>, u64, String)> {
    let fichier = std::fs::File::open(chemin)?;
    let taille = fichier.metadata()?.len();
    let b = frontiere();
    let mut tete = String::new();
    let mut texte = |nom: &str, valeur: &str| {
        if !valeur.is_empty() {
            tete.push_str(&format!("--{b}\r\nContent-Disposition: form-data; name=\"{nom}\"\r\n\r\n{valeur}\r\n"));
        }
    };
    texte("reglage", champs.reglage.as_deref().unwrap_or(""));
    texte("client", &champs.client);
    texte("commande", &champs.commande);
    texte("quantite", &champs.quantite.map(|q| q.to_string()).unwrap_or_default());
    tete.push_str(&format!("--{b}\r\nContent-Disposition: form-data; name=\"fichier\"; filename=\"{}\"\r\nContent-Type: application/octet-stream\r\n\r\n", nom_pour_entete(chemin)));
    let queue = format!("\r\n--{b}--\r\n");
    let total = tete.len() as u64 + taille + queue.len() as u64;
    Ok((Flux { tete: Cursor::new(tete.into_bytes()), fichier, queue: Cursor::new(queue.into_bytes()), fichier_fini: false, progression }, total, b))
}

/// `POST /api/travaux` d'un fichier. Rend le nom du travail créé quand le serveur le donne.
pub fn televerser(client: &Client, chemin: &Path, champs: &Champs, progression: &Progression) -> Resultat<()> {
    let (flux, total, frontiere) = corps(chemin, champs, Some(progression)).map_err(|e| Erreur::locale("lecture_impossible", e.to_string()))?;
    let requete = client.avec_cookie(client.agent_long.post(&client.url("/api/travaux"))).set("Content-Type", &format!("multipart/form-data; boundary={frontiere}")).set("Content-Length", &total.to_string());
    let r = client.verifier(requete.send(flux));
    if progression.annule.load(Ordering::Relaxed) {
        return Err(Erreur::locale("envoi_annule", None));
    }
    r.map(|_| ())
}

// --- Feuille d'envoi ---

/// Ouvre la feuille d'envoi des `fichiers` (le serveur décide du reste : réglage, fiche).
pub fn ouvrir(parent: &adw::ApplicationWindow, client: Arc<Client>, fichiers: Vec<PathBuf>, cmd: Sender<Commande>) {
    let fichiers: Vec<PathBuf> = fichiers.into_iter().filter(|f| extension_admise(f)).collect();
    if fichiers.is_empty() {
        let _ = cmd.try_send(Commande::Message("Format non pris en charge : PDF, SVG, TIFF, PNG ou JPEG.".into()));
        return;
    }
    let fenetre = adw::Window::builder().transient_for(parent).modal(true).default_width(480).default_height(560).title("Envoyer au RIP").build();

    let annuler = gtk::Button::with_label("Annuler");
    let envoyer = gtk::Button::builder().label("Envoyer").sensitive(false).css_classes(["suggested-action"]).build();
    let entete = adw::HeaderBar::builder().show_end_title_buttons(false).show_start_title_buttons(false).build();
    entete.pack_start(&annuler);
    entete.pack_end(&envoyer);

    let page = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(18).margin_top(12).margin_bottom(18).margin_start(16).margin_end(16).build();

    let groupe_fichiers = adw::PreferencesGroup::builder().title(if fichiers.len() > 1 { format!("{} fichiers", fichiers.len()) } else { "Fichier".to_string() }).build();
    for f in fichiers.iter().take(8) {
        let taille = std::fs::metadata(f).map_or(String::new(), |m| crate::modele::taille(m.len()));
        let ligne = adw::ActionRow::builder().title(glib::markup_escape_text(&f.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned()))).subtitle(taille).build();
        ligne.add_prefix(&gtk::Image::from_icon_name("x-office-document-symbolic"));
        groupe_fichiers.add(&ligne);
    }
    if fichiers.len() > 8 {
        groupe_fichiers.set_description(Some(&format!("… et {} autres", fichiers.len() - 8)));
    }
    page.append(&groupe_fichiers);

    let groupe_reglage = adw::PreferencesGroup::builder().title("Réglage").description("Appliqué par le serveur à chaque fichier.").build();
    let liste_reglages = gtk::StringList::new(&[]);
    let choix = adw::ComboRow::builder().title("Réglage").model(&liste_reglages).sensitive(false).build();
    groupe_reglage.add(&choix);
    page.append(&groupe_reglage);

    let groupe_fiche = adw::PreferencesGroup::builder().title("Fiche travail").description("Facultatif.").build();
    let client_ligne = adw::EntryRow::builder().title("Client").build();
    let commande_ligne = adw::EntryRow::builder().title("Commande").build();
    let quantite_ligne = adw::EntryRow::builder().title("Quantité").input_purpose(gtk::InputPurpose::Digits).build();
    groupe_fiche.add(&client_ligne);
    groupe_fiche.add(&commande_ligne);
    groupe_fiche.add(&quantite_ligne);
    page.append(&groupe_fiche);

    let barre = gtk::ProgressBar::builder().visible(false).show_text(true).build();
    page.append(&barre);

    let defiler = gtk::ScrolledWindow::builder().child(&adw::Clamp::builder().maximum_size(560).child(&page).build()).vexpand(true).hscrollbar_policy(gtk::PolicyType::Never).build();
    let vue = adw::ToolbarView::new();
    vue.add_top_bar(&entete);
    vue.set_content(Some(&defiler));
    fenetre.set_content(Some(&vue));

    // Réglages du serveur, le dernier utilisé en tête.
    {
        let (client, cmd, liste, choix, envoyer) = (client.clone(), cmd.clone(), liste_reglages.clone(), choix.clone(), envoyer.clone());
        glib::spawn_future_local(async move {
            let r = gio::spawn_blocking(move || client.reglages()).await;
            match r {
                Ok(Ok(reglages)) => {
                    let dernier = crate::config::lire("reglage");
                    for (i, reg) in reglages.iter().enumerate() {
                        liste.append(&reg.nom);
                        if dernier.as_deref() == Some(reg.nom.as_str()) {
                            choix.set_selected(i as u32);
                        }
                    }
                    choix.set_sensitive(!reglages.is_empty());
                }
                Ok(Err(e)) => {
                    let _ = cmd.try_send(Commande::Erreur(e));
                }
                Err(_) => {}
            }
            envoyer.set_sensitive(true);
        });
    }

    let progression = Arc::new(Progression::default());
    let en_cours = Arc::new(AtomicBool::new(false));

    {
        let (fenetre, progression, en_cours) = (fenetre.clone(), progression.clone(), en_cours.clone());
        annuler.connect_clicked(move |_| {
            if en_cours.load(Ordering::Relaxed) {
                progression.annule.store(true, Ordering::Relaxed);
            } else {
                fenetre.close();
            }
        });
    }

    {
        let (fenetre, cmd, barre, annuler, progression, en_cours) = (fenetre.clone(), cmd.clone(), barre.clone(), annuler.clone(), progression.clone(), en_cours.clone());
        envoyer.connect_clicked(move |bouton| {
            let quantite_saisie = quantite_ligne.text().trim().to_string();
            let quantite = if quantite_saisie.is_empty() {
                None
            } else if let Ok(q) = quantite_saisie.parse::<u32>() {
                Some(q)
            } else {
                let _ = cmd.try_send(Commande::Message("Quantité invalide : un nombre entier est attendu.".into()));
                return;
            };
            let reglage = choix.selected_item().and_downcast::<gtk::StringObject>().map(|s| s.string().to_string());
            if let Some(r) = &reglage {
                crate::config::ecrire("reglage", r);
            }
            let champs = Champs { reglage, client: client_ligne.text().trim().to_string(), commande: commande_ligne.text().trim().to_string(), quantite };
            bouton.set_sensitive(false);
            for w in [&groupe_fichiers, &groupe_reglage, &groupe_fiche] {
                w.set_sensitive(false);
            }
            barre.set_visible(true);
            en_cours.store(true, Ordering::Relaxed);

            let total_octets: u64 = fichiers.iter().filter_map(|f| std::fs::metadata(f).ok()).map(|m| m.len()).sum();
            progression.total.store(total_octets.max(1), Ordering::Relaxed);

            // Le fil d'envoi parcourt les fichiers l'un après l'autre.
            let (client, liste, p) = (client.clone(), fichiers.clone(), progression.clone());
            let travail = gio::spawn_blocking(move || -> Result<usize, (String, Erreur)> {
                for (i, f) in liste.iter().enumerate() {
                    p.fichier.store(i, Ordering::Relaxed);
                    let nom = f.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                    if let Err(e) = televerser(&client, f, &champs, &p) {
                        return Err((nom, e));
                    }
                }
                Ok(liste.len())
            });

            // Barre de progression, relue tous les dixièmes de seconde.
            {
                let (barre, progression, nb) = (barre.clone(), progression.clone(), fichiers.len());
                glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                    if progression.fini.load(Ordering::Relaxed) {
                        return glib::ControlFlow::Break;
                    }
                    let (fait, total) = (progression.envoyes.load(Ordering::Relaxed), progression.total.load(Ordering::Relaxed));
                    barre.set_fraction((fait as f64 / total as f64).min(1.0));
                    let i = progression.fichier.load(Ordering::Relaxed) + 1;
                    barre.set_text(Some(&if nb > 1 { format!("Fichier {i} sur {nb} — {} sur {}", crate::modele::taille(fait), crate::modele::taille(total)) } else { format!("{} sur {}", crate::modele::taille(fait), crate::modele::taille(total)) }));
                    glib::ControlFlow::Continue
                });
            }

            let (fenetre, cmd, progression, en_cours, annuler) = (fenetre.clone(), cmd.clone(), progression.clone(), en_cours.clone(), annuler.clone());
            glib::spawn_future_local(async move {
                let r = travail.await;
                progression.fini.store(true, Ordering::Relaxed);
                en_cours.store(false, Ordering::Relaxed);
                annuler.set_sensitive(true);
                match r {
                    Ok(Ok(n)) => {
                        let _ = cmd.try_send(Commande::Message(if n > 1 { format!("{n} fichiers envoyés.") } else { "Fichier envoyé.".into() }));
                        let _ = cmd.try_send(Commande::Rafraichir);
                    }
                    Ok(Err((nom, e))) => {
                        if e.code != "envoi_annule" {
                            let _ = cmd.try_send(Commande::Message(format!("{nom} : {}", e.message())));
                        }
                        if e.session_perdue() {
                            let _ = cmd.try_send(Commande::Erreur(e));
                        }
                        let _ = cmd.try_send(Commande::Rafraichir);
                    }
                    Err(_) => {
                        let _ = cmd.try_send(Commande::Message("L'envoi s'est arrêté de façon inattendue.".into()));
                    }
                }
                fenetre.close();
            });
        });
    }

    fenetre.present();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corps_multipart_complet_et_fichier_en_dernier() {
        let rep = std::env::temp_dir().join(format!("eko-rip-test-{}", std::process::id()));
        std::fs::create_dir_all(&rep).unwrap();
        let chemin = rep.join("affiche \"A0\".pdf");
        std::fs::write(&chemin, b"%PDF-1.4 contenu").unwrap();
        let champs = Champs { reglage: Some("Bâche".into()), client: "Dupont".into(), commande: String::new(), quantite: Some(3) };
        let (mut flux, total, b) = corps(&chemin, &champs, None).unwrap();
        let mut tout = vec![];
        flux.read_to_end(&mut tout).unwrap();
        assert_eq!(tout.len() as u64, total);
        let texte = String::from_utf8(tout).unwrap();
        assert!(texte.starts_with(&format!("--{b}\r\n")));
        assert!(texte.ends_with(&format!("\r\n--{b}--\r\n")));
        let (i_reglage, i_quantite, i_fichier) = (texte.find("name=\"reglage\"").unwrap(), texte.find("name=\"quantite\"").unwrap(), texte.find("name=\"fichier\"").unwrap());
        assert!(i_reglage < i_quantite && i_quantite < i_fichier);
        assert!(!texte.contains("name=\"commande\""));
        assert!(texte.contains("filename=\"affiche _A0_.pdf\""));
        assert!(texte.contains("%PDF-1.4 contenu"));
        let _ = std::fs::remove_dir_all(&rep);
    }

    #[test]
    fn extensions() {
        assert!(extension_admise(Path::new("/x/A.PDF")));
        assert!(extension_admise(Path::new("photo.jpeg")));
        assert!(!extension_admise(Path::new("notes.txt")));
        assert!(!extension_admise(Path::new("sans_extension")));
    }
}
