//! Écrans de connexion : adresse du serveur, puis identifiants si le serveur en demande.

use adw::prelude::*;

pub const ICONE: &str = "fr.e-ko.rip-center";

pub struct PageServeur {
    pub racine: adw::ToolbarView,
    pub adresse: adw::EntryRow,
    pub bouton: gtk::Button,
    pub annuler: gtk::Button,
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
        contenu.append(&en_tete_de_page("E-KO RIP Center", "Adresse du serveur, par exemple 192.168.10.109:8640."));
        let adresse = adw::EntryRow::builder().title("Adresse du serveur").input_purpose(gtk::InputPurpose::Url).build();
        let groupe = adw::PreferencesGroup::new();
        groupe.add(&adresse);
        contenu.append(&groupe);
        let bouton = gtk::Button::builder().label("Se connecter").css_classes(["suggested-action", "pill"]).halign(gtk::Align::Center).build();
        contenu.append(&bouton);
        let annuler = gtk::Button::builder().label("Annuler").visible(false).build();
        let entete = adw::HeaderBar::new();
        entete.pack_start(&annuler);
        PageServeur { racine: habiller(&contenu, &entete), adresse, bouton, annuler }
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
