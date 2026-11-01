//! Écrans de connexion : adresse du serveur, puis identifiants si le serveur en demande.

use crate::decouverte::Serveur;
use adw::prelude::*;
use gtk::glib;
use std::cell::RefCell;

pub const ICONE: &str = "fr.e-ko.rip-center";

pub struct PageServeur {
    pub racine: adw::ToolbarView,
    pub adresse: adw::EntryRow,
    pub bouton: gtk::Button,
    pub annuler: gtk::Button,
    /// Serveurs trouvés sur le réseau local (découverte DNS-SD).
    pub trouves: adw::PreferencesGroup,
    pub rechercher: gtk::Button,
    attente: gtk::Spinner,
    rangees: RefCell<Vec<adw::ActionRow>>,
}

pub struct PageIdentification {
    pub racine: adw::ToolbarView,
    pub serveur: gtk::Label,
    pub nom: adw::EntryRow,
    pub mot_de_passe: adw::PasswordEntryRow,
    pub bouton: gtk::Button,
    pub changer: gtk::Button,
}

fn en_tete_de_page(titre: &str, description: &str) -> gtk::Box {
    let boite = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(6).halign(gtk::Align::Center).margin_bottom(12).build();
    boite.append(&gtk::Image::builder().icon_name(ICONE).pixel_size(96).margin_bottom(6).build());
    boite.append(&gtk::Label::builder().label(titre).css_classes(["title-1"]).build());
    boite.append(&gtk::Label::builder().label(description).wrap(true).justify(gtk::Justification::Center).css_classes(["dim-label"]).build());
    boite
}

fn habiller(contenu: &gtk::Box, entete: &adw::HeaderBar) -> adw::ToolbarView {
    let vue = adw::ToolbarView::new();
    vue.add_top_bar(entete);
    let clamp = adw::Clamp::builder().maximum_size(420).margin_start(16).margin_end(16).margin_bottom(24).valign(gtk::Align::Center).child(contenu).build();
    vue.set_content(Some(&gtk::ScrolledWindow::builder().child(&clamp).hscrollbar_policy(gtk::PolicyType::Never).vexpand(true).build()));
    vue
}

impl PageServeur {
    pub fn new() -> Self {
        let contenu = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(18).build();
        contenu.append(&en_tete_de_page("E-KO RIP Center", "Choisissez votre serveur, ou saisissez son adresse, par exemple 192.168.10.109:8640."));
        let attente = gtk::Spinner::builder().spinning(true).valign(gtk::Align::Center).build();
        let rechercher = gtk::Button::builder().label("Rechercher à nouveau").css_classes(["flat"]).valign(gtk::Align::Center).build();
        let entete_trouves = gtk::Box::builder().spacing(6).build();
        entete_trouves.append(&attente);
        entete_trouves.append(&rechercher);
        let trouves = adw::PreferencesGroup::builder().title("Serveurs trouvés sur le réseau").header_suffix(&entete_trouves).build();
        contenu.append(&trouves);
        let adresse = adw::EntryRow::builder().title("Adresse du serveur").input_purpose(gtk::InputPurpose::Url).build();
        let groupe = adw::PreferencesGroup::builder().title("Saisir une adresse").build();
        groupe.add(&adresse);
        contenu.append(&groupe);
        let bouton = gtk::Button::builder().label("Se connecter").css_classes(["suggested-action", "pill"]).halign(gtk::Align::Center).build();
        contenu.append(&bouton);
        let annuler = gtk::Button::builder().label("Annuler").visible(false).build();
        let entete = adw::HeaderBar::new();
        entete.pack_start(&annuler);
        PageServeur { racine: habiller(&contenu, &entete), adresse, bouton, annuler, trouves, rechercher, attente, rangees: RefCell::new(Vec::new()) }
    }

    /// Affiche la liste des serveurs trouvés (ou l'état de la recherche) ; `choisir` reçoit
    /// l'adresse `http://ip:port` du serveur sur lequel on clique.
    pub fn maj_trouves(&self, serveurs: &[Serveur], en_cours: bool, choisir: impl Fn(String) + Clone + 'static) {
        for r in self.rangees.borrow_mut().drain(..) {
            self.trouves.remove(&r);
        }
        self.attente.set_visible(en_cours);
        self.rechercher.set_sensitive(!en_cours);
        let mut rangees = Vec::new();
        for s in serveurs {
            let sous_titre = if s.version.is_empty() { s.hote_port().to_string() } else { format!("{} · version {}", s.hote_port(), s.version) };
            let rangee = adw::ActionRow::builder().title(&s.nom).subtitle(glib::markup_escape_text(&sous_titre)).activatable(true).build();
            rangee.set_use_markup(false);
            rangee.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
            let (adresse, choisir) = (s.adresse.clone(), choisir.clone());
            rangee.connect_activated(move |_| choisir(adresse.clone()));
            self.trouves.add(&rangee);
            rangees.push(rangee);
        }
        if serveurs.is_empty() {
            let texte = if en_cours { "Recherche en cours…" } else { "Aucun serveur trouvé : saisissez son adresse ci-dessous." };
            let rangee = adw::ActionRow::builder().title(texte).css_classes(["dim-label"]).build();
            self.trouves.add(&rangee);
            rangees.push(rangee);
        }
        *self.rangees.borrow_mut() = rangees;
    }
}

impl PageIdentification {
    pub fn new() -> Self {
        let contenu = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(18).build();
        contenu.append(&en_tete_de_page("Connexion", "Ce serveur demande un utilisateur et un mot de passe."));
        let serveur = gtk::Label::builder().css_classes(["caption", "dim-label"]).halign(gtk::Align::Center).selectable(true).build();
        contenu.append(&serveur);
        let nom = adw::EntryRow::builder().title("Utilisateur").build();
        let mot_de_passe = adw::PasswordEntryRow::builder().title("Mot de passe").build();
        let groupe = adw::PreferencesGroup::new();
        groupe.add(&nom);
        groupe.add(&mot_de_passe);
        contenu.append(&groupe);
        let bouton = gtk::Button::builder().label("Se connecter").css_classes(["suggested-action", "pill"]).halign(gtk::Align::Center).build();
        contenu.append(&bouton);
        let changer = gtk::Button::builder().label("Changer de serveur…").css_classes(["flat"]).build();
        let entete = adw::HeaderBar::new();
        entete.pack_start(&changer);
        PageIdentification { racine: habiller(&contenu, &entete), serveur, nom, mot_de_passe, bouton, changer }
    }
}
