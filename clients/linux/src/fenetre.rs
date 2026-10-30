//! Fenêtre principale : navigation, connexion, rafraîchissement toutes les 3 s, commandes des
//! vues, notifications et glisser-déposer. Seule elle parle au serveur.

use crate::api::{Client, Erreur, Resultat, VERSION};
use crate::commande::Commande;
use crate::config;
use crate::connexion::{ICONE, PageIdentification, PageServeur};
use crate::envoi;
use crate::inspecteur::Inspecteur;
use crate::machines::Machines;
use crate::modele::{EtatSortie, Machine, Session, Sortie, StatutTravail, Systeme as EtatSysteme, Travail};
use crate::systeme::Systeme;
use crate::travaux::Travaux;
use adw::prelude::*;
use async_channel::{Receiver, Sender};
use gtk::{gdk, gio, glib};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

/// Dernier état connu du serveur.
#[derive(Default)]
struct Donnees {
    travaux: Vec<Travail>,
    file: Vec<Sortie>,
    machines: Vec<Machine>,
}

/// Ce que l'on savait au tour précédent, pour détecter les fins de travail.
#[derive(Default)]
struct Suivi {
    travaux: HashMap<String, StatutTravail>,
    sorties: HashMap<String, EtatSortie>,
    amorce: bool,
}

type Instantane = (Vec<Travail>, Vec<Sortie>, Vec<Machine>, Option<EtatSysteme>);

pub struct Fenetre {
    app: adw::Application,
    fen: adw::ApplicationWindow,
    toasts: adw::ToastOverlay,
    pile: gtk::Stack,
    serveur: PageServeur,
    ident: PageIdentification,
    contenu: gtk::Stack,
    titre_pile: gtk::Stack,
    titre_page: adw::WindowTitle,
    recherche: gtk::SearchEntry,
    bascule_inspecteur: gtk::ToggleButton,
    separation_inspecteur: adw::OverlaySplitView,
    groupe_boutons: gtk::Box,
    bouton_imprimer: gtk::Button,
    bouton_epreuve: gtk::Button,
    bouton_suspendre: gtk::Button,
    contenu_suspendre: adw::ButtonContent,
    action_deconnecter: gio::SimpleAction,
    travaux: Rc<Travaux>,
    machines: Machines,
    systeme: Systeme,
    inspecteur: Rc<Inspecteur>,
    cmd: Sender<Commande>,
    client: RefCell<Option<Arc<Client>>>,
    session: RefCell<Session>,
    donnees: RefCell<Donnees>,
    choisi: RefCell<Option<String>>,
    page: RefCell<String>,
    precedent: RefCell<String>,
    en_vol: Cell<bool>,
    hors_ligne: Cell<bool>,
    minuterie: RefCell<Option<glib::SourceId>>,
    suivi: RefCell<Suivi>,
    fichiers_en_attente: RefCell<Vec<PathBuf>>,
}

fn entree_barre(icone: &str, titre: &str, nom: &str) -> gtk::ListBoxRow {
    let boite = gtk::Box::builder().spacing(12).margin_top(8).margin_bottom(8).margin_start(6).margin_end(6).build();
    boite.append(&gtk::Image::from_icon_name(icone));
    boite.append(&gtk::Label::new(Some(titre)));
    let ligne = gtk::ListBoxRow::builder().child(&boite).build();
    ligne.set_widget_name(nom);
    ligne
}

fn titre_de_section(titre: &str) -> gtk::ListBoxRow {
    let l = gtk::Label::builder().label(titre).xalign(0.0).margin_top(14).margin_bottom(4).margin_start(6).css_classes(["caption-heading", "dim-label"]).build();
    gtk::ListBoxRow::builder().child(&l).selectable(false).activatable(false).build()
}

fn bouton_barre(icone: &str, titre: &str) -> gtk::Button {
    gtk::Button::builder().child(&adw::ButtonContent::builder().icon_name(icone).label(titre).build()).build()
}

impl Fenetre {
    pub fn new(app: &adw::Application) -> Rc<Self> {
        let (cmd, reception) = async_channel::unbounded::<Commande>();

        let travaux = Travaux::new(cmd.clone());
        let machines = Machines::new(cmd.clone());
        let systeme = Systeme::new();
        let inspecteur = Inspecteur::new(cmd.clone());

        // Barre latérale : Production, Atelier, Serveur.
        let liste = gtk::ListBox::builder().css_classes(["navigation-sidebar"]).build();
        let ligne_travaux = entree_barre("view-list-symbolic", "Travaux", "travaux");
        liste.append(&titre_de_section("Production"));
        liste.append(&ligne_travaux);
        liste.append(&titre_de_section("Atelier"));
        liste.append(&entree_barre("printer-symbolic", "Machines", "machines"));
        liste.append(&titre_de_section("Serveur"));
        liste.append(&entree_barre("computer-symbolic", "Système", "systeme"));
        let entete_barre = adw::HeaderBar::builder().title_widget(&adw::WindowTitle::new("E-KO RIP Center", "")).show_end_title_buttons(false).build();
        let vue_barre = adw::ToolbarView::new();
        vue_barre.add_top_bar(&entete_barre);
        vue_barre.set_content(Some(&gtk::ScrolledWindow::builder().child(&liste).hscrollbar_policy(gtk::PolicyType::Never).vexpand(true).build()));

        // Centre : barre d'outils unifiée et pages.
        let contenu = gtk::Stack::builder().transition_type(gtk::StackTransitionType::Crossfade).build();
        contenu.add_named(&gtk::ScrolledWindow::builder().child(&travaux.vue).build(), Some("travaux"));
        contenu.add_named(&machines.racine, Some("machines"));
        contenu.add_named(&systeme.racine, Some("systeme"));

        let bouton_envoyer = bouton_barre("list-add-symbolic", "Envoyer");
        bouton_envoyer.set_tooltip_text(Some("Envoyer des fichiers au RIP (Ctrl+O)"));
        bouton_envoyer.set_action_name(Some("win.envoyer"));
        let bouton_imprimer = bouton_barre("printer-symbolic", "Imprimer");
        let contenu_suspendre = adw::ButtonContent::builder().icon_name("media-playback-pause-symbolic").label("Suspendre").build();
        let bouton_suspendre = gtk::Button::builder().child(&contenu_suspendre).build();
        let bouton_epreuve = bouton_barre("image-x-generic-symbolic", "Épreuve");
        bouton_epreuve.set_tooltip_text(Some("Enregistrer l'épreuve TIFF de la page affichée"));
        let groupe_boutons = gtk::Box::builder().spacing(6).build();
        for b in [&bouton_imprimer, &bouton_suspendre, &bouton_epreuve] {
            groupe_boutons.append(b);
        }

        let recherche = gtk::SearchEntry::builder().placeholder_text("Rechercher un travail…").width_chars(28).build();
        let titre_page = adw::WindowTitle::new("", "");
        let titre_pile = gtk::Stack::new();
        titre_pile.add_named(&recherche, Some("recherche"));
        titre_pile.add_named(&titre_page, Some("titre"));

        let menu = gio::Menu::new();
        menu.append(Some("Changer de serveur…"), Some("win.changer-serveur"));
        menu.append(Some("Se déconnecter"), Some("win.deconnecter"));
        let menu_bas = gio::Menu::new();
        menu_bas.append(Some("À propos d'E-KO RIP Center"), Some("win.apropos"));
        menu_bas.append(Some("Quitter"), Some("app.quitter"));
        menu.append_section(None, &menu_bas);
        let bouton_menu = gtk::MenuButton::builder().icon_name("open-menu-symbolic").menu_model(&menu).primary(true).tooltip_text("Menu principal").build();
        let bascule_inspecteur = gtk::ToggleButton::builder().icon_name("sidebar-show-right-symbolic").active(true).tooltip_text("Afficher l'inspecteur").build();

        let entete = adw::HeaderBar::builder().title_widget(&titre_pile).build();
        entete.pack_start(&bouton_envoyer);
        entete.pack_start(&groupe_boutons);
        entete.pack_end(&bouton_menu);
        entete.pack_end(&bascule_inspecteur);
        let vue_centre = adw::ToolbarView::new();
        vue_centre.add_top_bar(&entete);
        vue_centre.set_content(Some(&contenu));

        // Trois volets : barre latérale / contenu / inspecteur.
        let separation_inspecteur = adw::OverlaySplitView::builder().sidebar_position(gtk::PackType::End).min_sidebar_width(320.0).max_sidebar_width(420.0).sidebar_width_fraction(0.3).content(&vue_centre).sidebar(&inspecteur.racine).build();
        let separation = adw::OverlaySplitView::builder().min_sidebar_width(210.0).max_sidebar_width(210.0).sidebar(&vue_barre).content(&separation_inspecteur).build();

        let serveur = PageServeur::new();
        let ident = PageIdentification::new();
        let pile = gtk::Stack::new();
        pile.add_named(&serveur.racine, Some("serveur"));
        pile.add_named(&ident.racine, Some("identification"));
        pile.add_named(&separation, Some("principal"));
        let toasts = adw::ToastOverlay::new();
        toasts.set_child(Some(&pile));
        let fen = adw::ApplicationWindow::builder().application(app).title("E-KO RIP Center").icon_name(ICONE).default_width(1360).default_height(820).content(&toasts).build();

        let action_deconnecter = gio::SimpleAction::new("deconnecter", None);
        let moi = Rc::new(Fenetre {
            app: app.clone(),
            fen,
            toasts,
            pile,
            serveur,
            ident,
            contenu,
            titre_pile,
            titre_page,
            recherche,
            bascule_inspecteur,
            separation_inspecteur,
            groupe_boutons,
            bouton_imprimer,
            bouton_epreuve,
            bouton_suspendre,
            contenu_suspendre,
            action_deconnecter,
            travaux,
            machines,
            systeme,
            inspecteur,
            cmd,
            client: RefCell::new(None),
            session: RefCell::new(Session::default()),
            donnees: RefCell::new(Donnees::default()),
            choisi: RefCell::new(None),
            page: RefCell::new("travaux".into()),
            precedent: RefCell::new("serveur".into()),
            en_vol: Cell::new(false),
            hors_ligne: Cell::new(false),
            minuterie: RefCell::new(None),
            suivi: RefCell::new(Suivi::default()),
            fichiers_en_attente: RefCell::new(vec![]),
        });
        moi.cabler(&liste, &ligne_travaux, reception);
        moi
    }

    fn cabler(self: &Rc<Self>, liste: &gtk::ListBox, ligne_travaux: &gtk::ListBoxRow, reception: Receiver<Commande>) {
        // Commandes des vues.
        let faible = Rc::downgrade(self);
        glib::spawn_future_local(async move {
            while let Ok(c) = reception.recv().await {
                match faible.upgrade() {
                    Some(f) => f.traiter(c),
                    None => break,
                }
            }
        });

        // Navigation.
        let f = Rc::downgrade(self);
        liste.connect_row_selected(move |_, ligne| {
            if let (Some(f), Some(l)) = (f.upgrade(), ligne) {
                f.aller_a(&l.widget_name());
            }
        });
        liste.select_row(Some(ligne_travaux));

        // Barre d'outils.
        for (bouton, c) in [(&self.bouton_imprimer, 0), (&self.bouton_suspendre, 1), (&self.bouton_epreuve, 2)] {
            let cmd = self.cmd.clone();
            bouton.connect_clicked(move |_| {
                let _ = cmd.try_send(match c {
                    0 => Commande::ImprimerSelection,
                    1 => Commande::SuspendreSelection,
                    _ => Commande::EpreuveSelection,
                });
            });
        }
        let f = Rc::downgrade(self);
        self.recherche.connect_search_changed(move |_| {
            if let Some(f) = f.upgrade() {
                f.afficher_travaux();
            }
        });
        let sep = self.separation_inspecteur.clone();
        self.bascule_inspecteur.connect_toggled(move |b| sep.set_show_sidebar(b.is_active()));

        // Pages de connexion.
        let f = Rc::downgrade(self);
        let se_connecter = move || {
            if let Some(f) = f.upgrade() {
                f.valider_serveur();
            }
        };
        let aller = se_connecter.clone();
        self.serveur.bouton.connect_clicked(move |_| aller());
        self.serveur.adresse.connect_entry_activated(move |_| se_connecter());
        let f = Rc::downgrade(self);
        self.serveur.annuler.connect_clicked(move |_| {
            if let Some(f) = f.upgrade() {
                let p = f.precedent.borrow().clone();
                f.pile.set_visible_child_name(&p);
            }
        });
        let f = Rc::downgrade(self);
        let ouvrir_session = move || {
            if let Some(f) = f.upgrade() {
                f.valider_identification();
            }
        };
        let aller = ouvrir_session.clone();
        self.ident.bouton.connect_clicked(move |_| aller());
        let aller = ouvrir_session.clone();
        self.ident.mot_de_passe.connect_entry_activated(move |_| aller());
        let nom_suivant = self.ident.mot_de_passe.clone();
        self.ident.nom.connect_entry_activated(move |_| {
            nom_suivant.grab_focus();
        });
        let f = Rc::downgrade(self);
        self.ident.changer.connect_clicked(move |_| {
            if let Some(f) = f.upgrade() {
                f.montrer_serveur(None);
            }
        });

        // Actions de la fenêtre.
        let ajouter = |nom: &str, f: fn(&Rc<Fenetre>)| {
            let a = gio::SimpleAction::new(nom, None);
            let faible = Rc::downgrade(self);
            a.connect_activate(move |_, _| {
                if let Some(m) = faible.upgrade() {
                    f(&m);
                }
            });
            self.fen.add_action(&a);
        };
        ajouter("changer-serveur", |f| f.montrer_serveur(None));
        ajouter("envoyer", |f| f.choisir_fichiers());
        ajouter("rechercher", |f| {
            f.recherche.grab_focus();
        });
        ajouter("apropos", |f| f.a_propos());
        // « Se déconnecter » garde son action en champ : elle s'active selon la session.
        let faible = Rc::downgrade(self);
        self.action_deconnecter.connect_activate(move |_, _| {
            if let Some(m) = faible.upgrade() {
                m.deconnecter();
            }
        });
        self.fen.add_action(&self.action_deconnecter);
        self.action_deconnecter.set_enabled(false);

        // Glisser-déposer de fichiers sur la fenêtre.
        let cible = gtk::DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::COPY);
        let f = Rc::downgrade(self);
        cible.connect_drop(move |_, valeur, _, _| {
            let Some(f) = f.upgrade() else { return false };
            let Ok(liste) = valeur.get::<gdk::FileList>() else { return false };
            let chemins: Vec<PathBuf> = liste.files().iter().filter_map(|x| x.path()).collect();
            if chemins.is_empty() {
                return false;
            }
            let _ = f.cmd.try_send(Commande::Envoyer(chemins));
            true
        });
        self.toasts.add_controller(cible);

        self.fen.connect_close_request(|_| glib::Propagation::Proceed);
    }

    pub fn presenter(&self) {
        self.fen.present();
    }

    /// Premier affichage : serveur retenu, sinon saisie de l'adresse.
    pub fn demarrer(self: &Rc<Self>) {
        match config::lire("serveur").and_then(|a| config::normaliser_adresse(&a)) {
            Some(adresse) => {
                self.serveur.adresse.set_text(&adresse);
                self.connecter_a(&adresse);
            }
            None => self.montrer_serveur(None),
        }
    }

    /// Fichiers venus du système (ouverture avec l'application, ligne de commande).
    pub fn ouvrir_fichiers(self: &Rc<Self>, chemins: Vec<PathBuf>) {
        let _ = self.cmd.try_send(Commande::Envoyer(chemins));
    }

    // --- Outils ---

    fn toast(&self, message: &str) {
        self.toasts.add_toast(adw::Toast::builder().title(glib::markup_escape_text(message)).timeout(5).build());
    }

    fn erreur(self: &Rc<Self>, e: &Erreur) {
        if e.session_perdue() {
            self.arreter_minuterie();
            if self.client.borrow().is_some() {
                self.montrer_identification(Some("Session expirée : se reconnecter."));
            }
        } else {
            self.toast(&e.message());
        }
    }

    /// Lance `travail` dans un fil à part et rend le résultat au fil GTK.
    fn lancer_res<T: Send + 'static>(self: &Rc<Self>, travail: impl FnOnce(Arc<Client>) -> Resultat<T> + Send + 'static, suite: impl FnOnce(&Rc<Self>, Resultat<T>) + 'static) {
        let Some(client) = self.client.borrow().clone() else { return };
        let moi = self.clone();
        glib::spawn_future_local(async move {
            let r = gio::spawn_blocking(move || travail(client)).await.unwrap_or_else(|_| Err(Erreur::locale("erreur_interne", None)));
            suite(&moi, r);
        });
    }

    /// Comme `lancer_res`, avec les erreurs affichées en toast (ou retour à la connexion).
    fn lancer<T: Send + 'static>(self: &Rc<Self>, travail: impl FnOnce(Arc<Client>) -> Resultat<T> + Send + 'static, suite: impl FnOnce(&Rc<Self>, T) + 'static) {
        self.lancer_res(travail, move |moi, r| match r {
            Ok(v) => suite(moi, v),
            Err(e) => moi.erreur(&e),
        });
    }

    // --- Connexion ---

    fn montrer_serveur(&self, message: Option<&str>) {
        let courant = self.pile.visible_child_name().map(|n| n.to_string()).unwrap_or_default();
        if courant != "serveur" {
            *self.precedent.borrow_mut() = courant;
        }
        self.serveur.annuler.set_visible(self.client.borrow().is_some());
        self.pile.set_visible_child_name("serveur");
        self.serveur.adresse.grab_focus();
        if let Some(m) = message {
            self.toast(m);
        }
    }

    fn montrer_identification(&self, message: Option<&str>) {
        self.arreter_minuterie();
        self.action_deconnecter.set_enabled(false);
        let base = self.client.borrow().as_ref().map(|c| c.base().to_string()).unwrap_or_default();
        self.ident.serveur.set_text(&base);
        self.ident.mot_de_passe.set_text("");
        self.ident.bouton.set_sensitive(true);
        self.pile.set_visible_child_name("identification");
        self.ident.nom.grab_focus();
        if let Some(m) = message {
            self.toast(m);
        }
    }

    fn valider_serveur(self: &Rc<Self>) {
        let saisie = self.serveur.adresse.text().to_string();
        match config::normaliser_adresse(&saisie) {
            Some(a) => self.connecter_a(&a),
            None => self.toast("Adresse invalide : hôte:8640, par exemple 192.168.10.109:8640."),
        }
    }

    fn connecter_a(self: &Rc<Self>, adresse: &str) {
        let client = Arc::new(Client::new(adresse));
        self.serveur.bouton.set_sensitive(false);
        self.serveur.bouton.set_label("Connexion…");
        let (adresse, c) = (adresse.to_string(), client.clone());
        glib::spawn_future_local({
            let moi = self.clone();
            async move {
                let r = gio::spawn_blocking(move || c.session()).await.unwrap_or_else(|_| Err(Erreur::locale("erreur_interne", None)));
                moi.serveur.bouton.set_sensitive(true);
                moi.serveur.bouton.set_label("Se connecter");
                match r {
                    Ok(session) => {
                        config::ecrire("serveur", &adresse);
                        moi.serveur.adresse.set_text(&adresse);
                        moi.arreter_minuterie();
                        *moi.client.borrow_mut() = Some(client);
                        *moi.donnees.borrow_mut() = Donnees::default();
                        *moi.suivi.borrow_mut() = Suivi::default();
                        *moi.choisi.borrow_mut() = None;
                        moi.travaux.maj(&[], None);
                        let libre = session.acces_libre || session.utilisateur.is_some();
                        *moi.session.borrow_mut() = session;
                        if libre {
                            moi.entrer();
                        } else {
                            moi.montrer_identification(None);
                        }
                    }
                    Err(e) => moi.montrer_serveur(Some(&format!("Connexion impossible à {adresse} : {}", e.message()))),
                }
            }
        });
    }

    fn valider_identification(self: &Rc<Self>) {
        let (nom, mdp) = (self.ident.nom.text().to_string(), self.ident.mot_de_passe.text().to_string());
        if nom.trim().is_empty() {
            self.toast("Saisir le nom d'utilisateur.");
            return;
        }
        self.ident.bouton.set_sensitive(false);
        self.lancer_res(
            move |c| {
                c.connecter(nom.trim(), &mdp)?;
                c.session()
            },
            |moi, r| {
                moi.ident.bouton.set_sensitive(true);
                match r {
                    Ok(s) => {
                        *moi.session.borrow_mut() = s;
                        moi.entrer();
                    }
                    Err(e) => moi.toast(&e.message()),
                }
            },
        );
    }

    fn deconnecter(self: &Rc<Self>) {
        self.arreter_minuterie();
        self.lancer_res(|c| c.deconnecter(), |moi, _| {
            *moi.session.borrow_mut() = Session::default();
            moi.montrer_identification(None);
        });
    }

    fn arreter_minuterie(&self) {
        if let Some(id) = self.minuterie.borrow_mut().take() {
            id.remove();
        }
    }

    /// Connecté : écran principal et rafraîchissement toutes les 3 secondes.
    fn entrer(self: &Rc<Self>) {
        self.action_deconnecter.set_enabled(self.session.borrow().utilisateur.is_some());
        self.pile.set_visible_child_name("principal");
        self.hors_ligne.set(false);
        self.en_vol.set(false);
        self.rafraichir();
        self.arreter_minuterie();
        let f = Rc::downgrade(self);
        let id = glib::timeout_add_seconds_local(3, move || match f.upgrade() {
            Some(f) => {
                f.rafraichir();
                glib::ControlFlow::Continue
            }
            None => glib::ControlFlow::Break,
        });
        *self.minuterie.borrow_mut() = Some(id);
        let en_attente = std::mem::take(&mut *self.fichiers_en_attente.borrow_mut());
        if !en_attente.is_empty() {
            let _ = self.cmd.try_send(Commande::Envoyer(en_attente));
        }
    }

    // --- Navigation ---

    fn aller_a(self: &Rc<Self>, nom: &str) {
        *self.page.borrow_mut() = nom.to_string();
        self.contenu.set_visible_child_name(nom);
        let travaux = nom == "travaux";
        self.groupe_boutons.set_visible(travaux);
        self.bascule_inspecteur.set_visible(travaux);
        self.separation_inspecteur.set_show_sidebar(travaux && self.bascule_inspecteur.is_active());
        self.titre_pile.set_visible_child_name(if travaux { "recherche" } else { "titre" });
        self.titre_page.set_title(match nom {
            "machines" => "Machines",
            _ => "Système",
        });
        if self.client.borrow().is_some() {
            self.rafraichir();
        }
    }

    // --- Rafraîchissement ---

    fn rafraichir(self: &Rc<Self>) {
        if self.en_vol.get() || self.client.borrow().is_none() {
            return;
        }
        self.en_vol.set(true);
        let avec_systeme = *self.page.borrow() == "systeme";
        self.lancer_res(
            move |c| -> Resultat<Instantane> {
                let (t, f, m) = (c.travaux()?, c.file()?, c.machines()?);
                let s = if avec_systeme { Some(c.systeme()?) } else { None };
                Ok((t, f, m, s))
            },
            |moi, r| {
                moi.en_vol.set(false);
                match r {
                    Ok(inst) => {
                        moi.hors_ligne.set(false);
                        moi.appliquer(inst);
                    }
                    Err(e) if e.session_perdue() => moi.erreur(&e),
                    Err(e) => {
                        if !moi.hors_ligne.replace(true) {
                            moi.toast(&e.message());
                        }
                    }
                }
            },
        );
    }

    fn appliquer(self: &Rc<Self>, (travaux, file, machines, systeme): Instantane) {
        self.notifier(&travaux, &file);
        *self.donnees.borrow_mut() = Donnees { travaux, file, machines };
        self.afficher_travaux();
        let d = self.donnees.borrow();
        self.machines.maj(&d.machines, &d.file);
        if let Some(s) = systeme {
            self.systeme.maj(&s);
        }
        drop(d);
        self.maj_inspecteur();
    }

    /// Tableau, filtré par la recherche (nom ou dossier).
    fn afficher_travaux(&self) {
        let q = self.recherche.text().to_lowercase();
        let d = self.donnees.borrow();
        let filtres: Vec<Travail> = d.travaux.iter().filter(|t| q.is_empty() || t.nom.to_lowercase().contains(&q) || t.dossier().to_lowercase().contains(&q)).cloned().collect();
        self.travaux.maj(&filtres, self.choisi.borrow().as_deref());
    }

    fn maj_inspecteur(&self) {
        let d = self.donnees.borrow();
        let choisi = self.choisi.borrow().clone();
        let travail = choisi.as_deref().and_then(|id| d.travaux.iter().find(|t| t.id == id));
        self.inspecteur.maj(travail, &d.file, &d.machines);

        // Suspendre / Reprendre selon les sorties du travail.
        let sorties: Vec<&Sortie> = choisi.as_deref().map(|id| d.file.iter().filter(|s| s.travail == id).collect()).unwrap_or_default();
        let a_suspendre = sorties.iter().any(|s| matches!(s.etat, EtatSortie::EnAttente | EtatSortie::EnCours));
        let a_reprendre = sorties.iter().any(|s| s.etat == EtatSortie::Suspendue);
        if !a_suspendre && a_reprendre {
            self.contenu_suspendre.set_label("Reprendre");
            self.contenu_suspendre.set_icon_name("media-playback-start-symbolic");
        } else {
            self.contenu_suspendre.set_label("Suspendre");
            self.contenu_suspendre.set_icon_name("media-playback-pause-symbolic");
        }
        self.bouton_suspendre.set_sensitive(a_suspendre || a_reprendre);
        self.bouton_imprimer.set_sensitive(travail.is_some_and(|t| t.statut == StatutTravail::Termine));
        self.bouton_epreuve.set_sensitive(travail.is_some_and(|t| t.statut == StatutTravail::Termine));
    }

    // --- Notifications ---

    fn notifier(&self, travaux: &[Travail], file: &[Sortie]) {
        let mut suivi = self.suivi.borrow_mut();
        if suivi.amorce {
            for t in travaux {
                let Some(avant) = suivi.travaux.get(&t.id) else { continue };
                if !matches!(avant, StatutTravail::EnAttente | StatutTravail::EnCours) {
                    continue;
                }
                match t.statut {
                    StatutTravail::Termine => self.notification(&format!("travail-{}", t.id), "Travail terminé", &t.nom),
                    StatutTravail::Echec => {
                        let motif = t.code_erreur.as_deref().map(|c| crate::libelles::libelle(c).unwrap_or(c).to_string()).unwrap_or_default();
                        self.notification(&format!("travail-{}", t.id), "Échec d'un travail", &format!("{} {motif}", t.nom));
                    }
                    _ => {}
                }
            }
            for s in file {
                let Some(avant) = suivi.sorties.get(&s.id) else { continue };
                if !avant.active() {
                    continue;
                }
                let machine = self.donnees.borrow().machines.iter().find(|m| m.id == s.machine).map_or_else(|| s.machine.clone(), |m| m.nom.clone());
                let corps = format!("{} · page {} · {machine}", s.nom, s.page);
                let decoupe = s.operation.as_deref() == Some("decoupe");
                match s.etat {
                    EtatSortie::Terminee => self.notification(&format!("sortie-{}", s.id), if decoupe { "Découpe terminée" } else { "Impression terminée" }, &corps),
                    EtatSortie::Echec => self.notification(&format!("sortie-{}", s.id), if decoupe { "Échec de la découpe" } else { "Échec de l'impression" }, &corps),
                    _ => {}
                }
            }
        }
        suivi.travaux = travaux.iter().map(|t| (t.id.clone(), t.statut)).collect();
        suivi.sorties = file.iter().map(|s| (s.id.clone(), s.etat)).collect();
        suivi.amorce = true;
    }

    fn notification(&self, id: &str, titre: &str, corps: &str) {
        let n = gio::Notification::new(titre);
        n.set_body(Some(corps.trim()));
        n.set_default_action("app.montrer");
        self.app.send_notification(Some(id), &n);
    }

    // --- Commandes ---

    fn traiter(self: &Rc<Self>, c: Commande) {
        match c {
            Commande::Rafraichir => self.rafraichir(),
            Commande::Choisir(id) => {
                *self.choisi.borrow_mut() = id;
                self.maj_inspecteur();
            }
            Commande::Apercu { travail, page } => {
                let id = travail.clone();
                self.lancer_res(move |c| c.apercu(&travail, page), move |moi, r| match r {
                    Err(e) if e.session_perdue() => moi.erreur(&e),
                    r => moi.inspecteur.apercu_recu(&id, page, r.map_err(|e| e.message())),
                });
            }
            Commande::ImprimerSelection => {
                let (Some((t, page)), Some(machine)) = (self.inspecteur.courant(), self.inspecteur.machine_choisie()) else { return };
                let _ = self.cmd.try_send(Commande::Imprimer { travail: t.id, page, machine });
            }
            Commande::Imprimer { travail, page, machine } => {
                self.lancer(move |c| c.imprimer(&travail, page, &machine), |moi, ()| {
                    moi.toast("Ajouté à la file d'attente.");
                    moi.en_vol.set(false);
                    moi.rafraichir();
                });
            }
            Commande::EpreuveSelection => {
                if let Some((t, page)) = self.inspecteur.courant() {
                    let _ = self.cmd.try_send(Commande::Epreuve { travail: t.id, page });
                }
            }
            Commande::Epreuve { travail, page } => self.epreuve(travail, page),
            Commande::SuspendreSelection => {
                let Some(id) = self.choisi.borrow().clone() else { return };
                let d = self.donnees.borrow();
                let mine: Vec<&Sortie> = d.file.iter().filter(|s| s.travail == id).collect();
                let (action, etats): (&'static str, &[EtatSortie]) = if mine.iter().any(|s| matches!(s.etat, EtatSortie::EnAttente | EtatSortie::EnCours)) { ("suspendre", &[EtatSortie::EnAttente, EtatSortie::EnCours]) } else { ("reprendre", &[EtatSortie::Suspendue]) };
                let ids: Vec<String> = mine.iter().filter(|s| etats.contains(&s.etat)).map(|s| s.id.clone()).collect();
                drop(d);
                self.lancer(
                    move |c| {
                        for i in &ids {
                            c.agir_sur_sortie(i, action)?;
                        }
                        Ok(())
                    },
                    |moi, ()| {
                        moi.en_vol.set(false);
                        moi.rafraichir();
                    },
                );
            }
            Commande::Relancer(id) => {
                self.lancer(move |c| c.relancer(&id), |moi, ()| {
                    moi.toast("Rendu relancé.");
                    moi.en_vol.set(false);
                    moi.rafraichir();
                });
            }
            Commande::Supprimer(id) => self.confirmer_suppression(id),
            Commande::Sortie { id, action } => {
                self.lancer(move |c| c.agir_sur_sortie(&id, action), |moi, ()| {
                    moi.en_vol.set(false);
                    moi.rafraichir();
                });
            }
            Commande::Envoyer(chemins) => {
                let client = self.client.borrow().clone();
                match client {
                    Some(c) if self.pile.visible_child_name().as_deref() == Some("principal") => envoi::ouvrir(&self.fen, c, chemins, self.cmd.clone()),
                    _ => self.fichiers_en_attente.borrow_mut().extend(chemins),
                }
            }
            Commande::Erreur(e) => self.erreur(&e),
            Commande::Message(m) => self.toast(&m),
        }
    }

    fn choisir_fichiers(self: &Rc<Self>) {
        let filtre = gtk::FileFilter::new();
        filtre.set_name(Some("PDF, SVG, TIFF, PNG, JPEG"));
        for e in envoi::EXTENSIONS {
            filtre.add_suffix(e);
        }
        let tous = gtk::FileFilter::new();
        tous.set_name(Some("Tous les fichiers"));
        tous.add_pattern("*");
        let filtres = gio::ListStore::new::<gtk::FileFilter>();
        filtres.append(&filtre);
        filtres.append(&tous);
        let dialogue = gtk::FileDialog::builder().title("Envoyer au RIP").filters(&filtres).default_filter(&filtre).build();
        let cmd = self.cmd.clone();
        dialogue.open_multiple(Some(&self.fen), gio::Cancellable::NONE, move |r| {
            if let Ok(liste) = r {
                let chemins: Vec<PathBuf> = (0..liste.n_items()).filter_map(|i| liste.item(i).and_downcast::<gio::File>()).filter_map(|f| f.path()).collect();
                if !chemins.is_empty() {
                    let _ = cmd.try_send(Commande::Envoyer(chemins));
                }
            }
        });
    }

    fn epreuve(self: &Rc<Self>, travail: String, page: usize) {
        let nom = self.donnees.borrow().travaux.iter().find(|t| t.id == travail).map(|t| t.nom.clone()).unwrap_or_else(|| "epreuve".into());
        let base = std::path::Path::new(&nom).file_stem().map_or_else(|| "epreuve".to_string(), |s| s.to_string_lossy().into_owned());
        let dialogue = gtk::FileDialog::builder().title("Enregistrer l'épreuve TIFF").initial_name(format!("{base}-p{page}.tif")).build();
        let moi = self.clone();
        dialogue.save(Some(&self.fen), gio::Cancellable::NONE, move |r| {
            let Some(chemin) = r.ok().and_then(|f| f.path()) else { return };
            moi.toast("Préparation de l'épreuve… (elle peut prendre un moment)");
            let affiche = chemin.display().to_string();
            moi.lancer(move |c| c.epreuve(&travail, page, &chemin), move |moi, ()| moi.toast(&format!("Épreuve enregistrée : {affiche}")));
        });
    }

    fn confirmer_suppression(self: &Rc<Self>, id: String) {
        let nom = self.donnees.borrow().travaux.iter().find(|t| t.id == id).map(|t| t.nom.clone()).unwrap_or_default();
        let dialogue = adw::MessageDialog::builder().transient_for(&self.fen).heading("Supprimer ce travail ?").body(format!("« {nom} » sera supprimé du serveur avec ses rendus. Cette action est définitive.")).build();
        dialogue.add_responses(&[("annuler", "Annuler"), ("supprimer", "Supprimer")]);
        dialogue.set_response_appearance("supprimer", adw::ResponseAppearance::Destructive);
        dialogue.set_default_response(Some("annuler"));
        dialogue.set_close_response("annuler");
        let moi = self.clone();
        dialogue.connect_response(None, move |_, reponse| {
            if reponse != "supprimer" {
                return;
            }
            let id = id.clone();
            moi.lancer(move |c| c.supprimer(&id), |moi, ()| {
                moi.toast("Travail supprimé.");
                *moi.choisi.borrow_mut() = None;
                moi.en_vol.set(false);
                moi.rafraichir();
            });
        });
        dialogue.present();
    }

    fn a_propos(&self) {
        let a = adw::AboutWindow::builder().transient_for(&self.fen).application_name("E-KO RIP Center").application_icon(ICONE).version(VERSION).developer_name("E-KO").comments("Client Linux du RIP d'impression grand format.").build();
        a.present();
    }
}
