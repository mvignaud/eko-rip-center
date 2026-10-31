//! Page Système : version, disque, tampon, rendus, alertes de dossiers du serveur.

use crate::api::VERSION;
use crate::modele::{Systeme as Etat, taille};
use adw::prelude::*;
use std::cell::RefCell;

pub struct Systeme {
    pub racine: gtk::ScrolledWindow,
    serveur: adw::ActionRow,
    disque: adw::ActionRow,
    tampon: adw::ActionRow,
    rendus: adw::ActionRow,
    alertes: gtk::ListBox,
    groupe_alertes: adw::PreferencesGroup,
    derniere: RefCell<Option<Etat>>,
}

impl Systeme {
    pub fn new() -> Self {
        let ligne = |titre: &str| adw::ActionRow::builder().title(titre).subtitle("—").subtitle_selectable(true).css_classes(["property"]).build();
        let (serveur, disque, tampon, rendus) = (ligne("Version du serveur"), ligne("Disque"), ligne("Tampon des sorties"), ligne("Rendus"));
        let client = adw::ActionRow::builder().title("Version du client").subtitle(format!("{VERSION} (Linux)")).css_classes(["property"]).build();

        let serveur_groupe = adw::PreferencesGroup::builder().title("Serveur").build();
        for l in [&serveur, &disque, &tampon, &rendus] {
            serveur_groupe.add(l);
        }
        let client_groupe = adw::PreferencesGroup::builder().title("Ce poste").build();
        client_groupe.add(&client);

        let alertes = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::None).css_classes(["boxed-list"]).build();
        let groupe_alertes = adw::PreferencesGroup::builder().title("Fichiers et dossiers").description("Fichiers d'état illisibles ou dossiers recréés depuis le démarrage du serveur.").visible(false).build();
        groupe_alertes.add(&alertes);

        let boite = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(18).margin_top(18).margin_bottom(18).margin_start(18).margin_end(18).build();
        boite.append(&serveur_groupe);
        boite.append(&groupe_alertes);
        boite.append(&client_groupe);
        let racine = gtk::ScrolledWindow::builder().child(&adw::Clamp::builder().maximum_size(720).child(&boite).build()).hscrollbar_policy(gtk::PolicyType::Never).vexpand(true).build();
        Systeme { racine, serveur, disque, tampon, rendus, alertes, groupe_alertes, derniere: RefCell::new(None) }
    }

    pub fn maj(&self, s: &Etat) {
        if self.derniere.borrow().as_ref() == Some(s) {
            return;
        }
        self.serveur.set_subtitle(&s.version);
        self.disque.set_subtitle(&match (s.disque_libre, s.disque_total) {
            (Some(l), Some(t)) => format!("{} libres sur {}", taille(l), taille(t)),
            _ => "inconnu".into(),
        });
        self.tampon.set_subtitle(&taille(s.tampon));
        self.rendus.set_subtitle(&format!("{} en cours, {} en attente ({} simultanés au plus)", s.rendus.en_cours, s.rendus.en_attente, s.rendus.simultanes));
        while let Some(enfant) = self.alertes.first_child() {
            self.alertes.remove(&enfant);
        }
        for a in &s.alertes {
            let l = adw::ActionRow::builder().title(gtk::glib::markup_escape_text(&a.libelle())).build();
            l.add_prefix(&gtk::Image::from_icon_name("dialog-warning-symbolic"));
            self.alertes.append(&l);
        }
        self.groupe_alertes.set_visible(!s.alertes.is_empty());
        *self.derniere.borrow_mut() = Some(s.clone());
    }
}
