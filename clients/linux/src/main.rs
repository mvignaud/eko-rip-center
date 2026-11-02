//! E-KO RIP Center, client Linux natif (GTK 4 et libadwaita) : parle à l'API JSON du serveur.

mod api;
mod cloud;
mod commande;
mod config;
mod connexion;
mod decouverte;
mod envoi;
#[cfg(test)]
mod essai_serveur;
mod fenetre;
mod inspecteur;
mod libelles;
mod machines;
mod modele;
mod notifications_cloud;
mod rapport;
mod systeme;
mod telemetrie;
mod travaux;

use adw::prelude::*;
use fenetre::Fenetre;
use gtk::{gio, glib};
use std::cell::RefCell;
use std::rc::Rc;

const APPLICATION: &str = "fr.e-ko.rip-center";

const STYLE: &str = "
.apercu { background-color: #767676; border-radius: 10px; }
.apercu-message { color: #ffffff; }
";

fn main() -> glib::ExitCode {
    let app = adw::Application::builder().application_id(APPLICATION).flags(gio::ApplicationFlags::HANDLES_OPEN).build();
    glib::set_application_name("E-KO RIP Center");
    let fenetre: Rc<RefCell<Option<Rc<Fenetre>>>> = Rc::new(RefCell::new(None));

    app.connect_startup(|app| {
        telemetrie::demarrer();
        let css = gtk::CssProvider::new();
        css.load_from_data(STYLE);
        if let Some(ecran) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(&ecran, &css, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
        }
        app.set_accels_for_action("win.changer-serveur", &["<Shift><Primary>s"]);
        app.set_accels_for_action("win.envoyer", &["<Primary>o"]);
        app.set_accels_for_action("win.rechercher", &["<Primary>f"]);
        app.set_accels_for_action("app.quitter", &["<Primary>q"]);
        let quitter = gio::SimpleAction::new("quitter", None);
        let a = app.clone();
        quitter.connect_activate(move |_, _| a.quit());
        app.add_action(&quitter);
    });

    // La fenêtre est créée au premier appel (activation ou ouverture de fichiers).
    let creer = {
        let fenetre = fenetre.clone();
        move |app: &adw::Application| -> Rc<Fenetre> {
            if let Some(f) = fenetre.borrow().clone() {
                return f;
            }
            let f = Fenetre::new(app);
            let (montrer, f2) = (gio::SimpleAction::new("montrer", None), f.clone());
            montrer.connect_activate(move |_, _| f2.presenter());
            app.add_action(&montrer);
            // Clic sur une notification du cloud : paramètre « site\nlien ».
            let (ouvrir, f3) = (gio::SimpleAction::new("ouvrir-notification-cloud", Some(glib::VariantTy::STRING)), f.clone());
            ouvrir.connect_activate(move |_, p| f3.ouvrir_notification_cloud(p.and_then(|v| v.str()).unwrap_or_default()));
            app.add_action(&ouvrir);
            f.demarrer();
            *fenetre.borrow_mut() = Some(f.clone());
            f
        }
    };
    let creer_ouverture = creer.clone();
    app.connect_activate(move |app| creer(app).presenter());
    app.connect_open(move |app, fichiers, _| {
        let f = creer_ouverture(app);
        f.presenter();
        f.ouvrir_fichiers(fichiers.iter().filter_map(|x| x.path()).collect());
    });
    app.run()
}
