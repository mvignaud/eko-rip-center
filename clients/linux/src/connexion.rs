//! Écrans de connexion : choix du serveur (local ou cloud), identifiants si le serveur en demande,
//! sites du compte cloud.

use crate::cloud::{Compte, Site};
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
    /// « Se connecter au cloud ».
    pub cloud: adw::ActionRow,
    /// Serveurs trouvés sur le réseau local (découverte DNS-SD).
    pub trouves: adw::PreferencesGroup,
    pub rechercher: gtk::Button,
    attente: gtk::Spinner,
    rangees: RefCell<Vec<adw::ActionRow>>,
}

/// Connexion au cloud : attente du navigateur, puis sites du compte.
pub struct PageCloud {
    pub racine: adw::ToolbarView,
    pub titre: gtk::Label,
    pub description: gtk::Label,
    /// Adresse d'autorisation, montrée seulement si le navigateur ne s'est pas ouvert.
    pub lien: gtk::Label,
    pub attente: gtk::Box,
    pub annuler: gtk::Button,
    pub sites: adw::PreferencesGroup,
    pub deconnecter: gtk::Button,
    pub changer: gtk::Button,
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
        contenu.append(&en_tete_de_page("E-KO RIP Center", "Où se trouve votre serveur RIP ?"));
        let cloud = adw::ActionRow::builder().title("Se connecter au cloud").subtitle("Avec votre compte E-KO, depuis n'importe où").activatable(true).build();
        cloud.add_prefix(&gtk::Image::from_icon_name("network-wireless-symbolic"));
        cloud.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
        let groupe_cloud = adw::PreferencesGroup::builder().title("Cloud").build();
        groupe_cloud.add(&cloud);
        contenu.append(&groupe_cloud);
        contenu.append(&gtk::Label::builder().label("Serveur local").xalign(0.0).css_classes(["title-4"]).margin_top(6).build());
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
        PageServeur { racine: habiller(&contenu, &entete), adresse, bouton, annuler, cloud, trouves, rechercher, attente, rangees: RefCell::new(Vec::new()) }
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

impl PageCloud {
    pub fn new() -> Self {
        let contenu = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(18).build();
        let entete_page = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(6).halign(gtk::Align::Center).margin_bottom(12).build();
        entete_page.append(&gtk::Image::builder().icon_name(ICONE).pixel_size(96).margin_bottom(6).build());
        let titre = gtk::Label::builder().label("Cloud E-KO").css_classes(["title-1"]).build();
        let description = gtk::Label::builder().wrap(true).justify(gtk::Justification::Center).css_classes(["dim-label"]).build();
        entete_page.append(&titre);
        entete_page.append(&description);
        contenu.append(&entete_page);
        let lien = gtk::Label::builder().wrap(true).wrap_mode(gtk::pango::WrapMode::Char).selectable(true).visible(false).css_classes(["caption"]).build();
        let annuler = gtk::Button::builder().label("Annuler").halign(gtk::Align::Center).build();
        let attente = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(12).build();
        attente.append(&gtk::Spinner::builder().spinning(true).width_request(32).height_request(32).build());
        attente.append(&lien);
        attente.append(&annuler);
        contenu.append(&attente);
        let sites = adw::PreferencesGroup::builder().title("Vos sites").build();
        contenu.append(&sites);
        let deconnecter = gtk::Button::builder().label("Se déconnecter du cloud").css_classes(["destructive-action", "pill"]).halign(gtk::Align::Center).build();
        contenu.append(&deconnecter);
        let changer = gtk::Button::builder().label("Changer de serveur…").css_classes(["flat"]).build();
        let entete = adw::HeaderBar::new();
        entete.pack_start(&changer);
        PageCloud { racine: habiller(&contenu, &entete), titre, description, lien, attente, annuler, sites, deconnecter, changer, rangees: RefCell::new(Vec::new()) }
    }

    fn vider(&self) {
        for r in self.rangees.borrow_mut().drain(..) {
            self.sites.remove(&r);
        }
    }

    /// Navigateur ouvert, retour attendu (ou chargement du compte).
    pub fn montrer_attente(&self, description: &str, annulable: bool) {
        self.vider();
        self.titre.set_label("Cloud E-KO");
        self.description.set_label(description);
        self.lien.set_visible(false);
        self.annuler.set_visible(annulable);
        self.attente.set_visible(true);
        self.sites.set_visible(false);
        self.deconnecter.set_visible(false);
    }

    /// Le navigateur ne s'est pas ouvert : proposer l'adresse à copier.
    pub fn montrer_lien(&self, url: &str) {
        self.lien.set_label(url);
        self.lien.set_visible(true);
    }

    /// Sites du compte ; `ouvrir` reçoit le site sur lequel on clique (les sites hors ligne sont grisés).
    pub fn montrer_sites(&self, compte: &Compte, sites: &[Site], ouvrir: impl Fn(Site) + Clone + 'static) {
        self.vider();
        self.titre.set_label("Vos sites");
        let qui = if compte.nom.is_empty() { compte.email.clone() } else { format!("{} · {}", compte.nom, compte.email) };
        self.description.set_label(&format!("Connecté en tant que {qui}."));
        self.attente.set_visible(false);
        self.sites.set_visible(true);
        self.deconnecter.set_visible(true);
        let mut rangees = Vec::new();
        for s in sites {
            let etat = if s.en_ligne { "en ligne" } else { "hors ligne" };
            let rangee = adw::ActionRow::builder().title(if s.nom.is_empty() { s.id.as_str() } else { s.nom.as_str() }).subtitle(format!("{etat} · {}", s.role_libelle())).activatable(s.en_ligne).sensitive(s.en_ligne).build();
            rangee.set_use_markup(false);
            rangee.add_prefix(&gtk::Image::from_icon_name(if s.en_ligne { "emblem-ok-symbolic" } else { "network-offline-symbolic" }));
            if s.en_ligne {
                rangee.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
                let (site, ouvrir) = (s.clone(), ouvrir.clone());
                rangee.connect_activated(move |_| ouvrir(site.clone()));
            }
            self.sites.add(&rangee);
            rangees.push(rangee);
        }
        if sites.is_empty() {
            let rangee = adw::ActionRow::builder().title("Aucun site sur ce compte.").css_classes(["dim-label"]).build();
            self.sites.add(&rangee);
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
