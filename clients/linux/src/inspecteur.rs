//! Inspecteur du travail choisi : aperçu, onglets (fiche, sorties, alertes) et actions.

use crate::commande::Commande;
use crate::libelles;
use crate::machines::ligne_sortie;
use crate::modele::{Machine, Sortie, StatutTravail, Travail, date};
use adw::prelude::*;
use async_channel::Sender;
use gtk::{gdk, glib};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct Inspecteur {
    pub racine: gtk::Stack,
    cmd: Sender<Commande>,
    apercu: gtk::Picture,
    message_apercu: gtk::Label,
    pile_apercu: gtk::Stack,
    precedente: gtk::Button,
    suivante: gtk::Button,
    pagination: gtk::Label,
    titre: gtk::Label,
    statut: gtk::Label,
    onglet_alertes: gtk::ToggleButton,
    fiche: gtk::ListBox,
    sorties: gtk::ListBox,
    alertes: gtk::ListBox,
    machine: gtk::DropDown,
    modele_machines: gtk::StringList,
    imprimer: gtk::Button,
    relancer: gtk::Button,
    epreuve: gtk::Button,
    supprimer: gtk::Button,
    /// Travail affiché, page courante (à partir de 1), machines proposées.
    travail: RefCell<Option<Travail>>,
    page: Cell<usize>,
    machines_ids: RefCell<Vec<String>>,
    machine_choisie: RefCell<Option<String>>,
    /// Ce qui est dessiné (pour ne rien refaire quand rien n'a changé) et l'aperçu demandé.
    dessine: RefCell<Option<(Travail, Vec<Sortie>, usize)>>,
    apercu_demande: RefCell<Option<(String, usize, StatutTravail)>>,
}

fn vider(liste: &gtk::ListBox) {
    while let Some(enfant) = liste.first_child() {
        liste.remove(&enfant);
    }
}

fn boite_liste() -> gtk::ListBox {
    gtk::ListBox::builder().selection_mode(gtk::SelectionMode::None).css_classes(["boxed-list"]).build()
}

fn propriete(titre: &str, valeur: &str) -> adw::ActionRow {
    let v = if valeur.is_empty() { "—" } else { valeur };
    adw::ActionRow::builder().title(titre).subtitle(glib::markup_escape_text(v)).subtitle_selectable(true).css_classes(["property"]).build()
}

impl Inspecteur {
    pub fn new(cmd: Sender<Commande>) -> Rc<Self> {
        let racine = gtk::Stack::new();
        racine.add_named(&adw::StatusPage::builder().icon_name("document-open-symbolic").title("Aucun travail choisi").description("Choisir un travail dans la liste pour voir son aperçu et ses sorties.").build(), Some("vide"));

        // Aperçu, sur fond gris neutre et opaque.
        let apercu = gtk::Picture::builder().content_fit(gtk::ContentFit::Contain).can_shrink(true).build();
        let message_apercu = gtk::Label::builder().wrap(true).justify(gtk::Justification::Center).margin_start(12).margin_end(12).css_classes(["apercu-message"]).build();
        let pile_apercu = gtk::Stack::new();
        pile_apercu.add_named(&apercu, Some("image"));
        pile_apercu.add_named(&message_apercu, Some("message"));
        let cadre = gtk::Box::builder().css_classes(["apercu"]).height_request(240).build();
        cadre.append(&pile_apercu);
        pile_apercu.set_hexpand(true);
        pile_apercu.set_vexpand(true);

        let precedente = gtk::Button::builder().icon_name("go-previous-symbolic").css_classes(["flat", "circular"]).tooltip_text("Page précédente").build();
        let suivante = gtk::Button::builder().icon_name("go-next-symbolic").css_classes(["flat", "circular"]).tooltip_text("Page suivante").build();
        let pagination = gtk::Label::builder().hexpand(true).css_classes(["dim-label", "numeric"]).build();
        let barre_pages = gtk::Box::builder().spacing(6).build();
        barre_pages.append(&precedente);
        barre_pages.append(&pagination);
        barre_pages.append(&suivante);

        let titre = gtk::Label::builder().xalign(0.0).wrap(true).wrap_mode(gtk::pango::WrapMode::WordChar).selectable(true).css_classes(["title-3"]).build();
        let statut = gtk::Label::builder().xalign(0.0).wrap(true).build();

        // Onglets segmentés.
        let fiche = boite_liste();
        let sorties = boite_liste();
        let alertes = boite_liste();
        let pile_onglets = gtk::Stack::new();
        pile_onglets.add_named(&fiche, Some("fiche"));
        pile_onglets.add_named(&sorties, Some("sorties"));
        pile_onglets.add_named(&alertes, Some("alertes"));
        let onglets = gtk::Box::builder().css_classes(["linked"]).halign(gtk::Align::Center).build();
        let onglet_fiche = gtk::ToggleButton::builder().label("Fiche").active(true).build();
        let onglet_sorties = gtk::ToggleButton::builder().label("Sorties").group(&onglet_fiche).build();
        let onglet_alertes = gtk::ToggleButton::builder().label("Alertes").group(&onglet_fiche).build();
        for (b, nom) in [(&onglet_fiche, "fiche"), (&onglet_sorties, "sorties"), (&onglet_alertes, "alertes")] {
            let pile = pile_onglets.clone();
            b.connect_toggled(move |b| {
                if b.is_active() {
                    pile.set_visible_child_name(nom);
                }
            });
            onglets.append(b);
        }

        // Actions.
        let modele_machines = gtk::StringList::new(&[]);
        let machine = gtk::DropDown::builder().model(&modele_machines).hexpand(true).tooltip_text("Machine d'impression").build();
        let imprimer = gtk::Button::builder().label("Imprimer").css_classes(["suggested-action"]).build();
        let ligne_impression = gtk::Box::builder().spacing(6).build();
        ligne_impression.append(&machine);
        ligne_impression.append(&imprimer);
        let relancer = gtk::Button::with_label("Relancer");
        let epreuve = gtk::Button::with_label("Épreuve TIFF");
        let supprimer = gtk::Button::builder().label("Supprimer").css_classes(["destructive-action"]).build();
        let autres = gtk::Box::builder().spacing(6).homogeneous(true).build();
        autres.append(&relancer);
        autres.append(&epreuve);
        autres.append(&supprimer);

        let contenu = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(12).margin_top(12).margin_bottom(12).margin_start(12).margin_end(12).build();
        for w in [cadre.upcast_ref::<gtk::Widget>(), barre_pages.upcast_ref(), titre.upcast_ref(), statut.upcast_ref(), onglets.upcast_ref(), pile_onglets.upcast_ref(), ligne_impression.upcast_ref(), autres.upcast_ref()] {
            contenu.append(w);
        }
        racine.add_named(&gtk::ScrolledWindow::builder().child(&contenu).hscrollbar_policy(gtk::PolicyType::Never).build(), Some("contenu"));

        let this = Rc::new(Inspecteur {
            racine,
            cmd,
            apercu,
            message_apercu,
            pile_apercu,
            precedente,
            suivante,
            pagination,
            titre,
            statut,
            onglet_alertes,
            fiche,
            sorties,
            alertes,
            machine,
            modele_machines,
            imprimer,
            relancer,
            epreuve,
            supprimer,
            travail: RefCell::new(None),
            page: Cell::new(1),
            machines_ids: RefCell::new(vec![]),
            machine_choisie: RefCell::new(None),
            dessine: RefCell::new(None),
            apercu_demande: RefCell::new(None),
        });
        this.brancher();
        this
    }

    fn brancher(self: &Rc<Self>) {
        let faible = Rc::downgrade(self);
        let pages = |sens: isize| {
            let faible = faible.clone();
            move |_: &gtk::Button| {
                if let Some(i) = faible.upgrade() {
                    i.aller_a_la_page(sens);
                }
            }
        };
        self.precedente.connect_clicked(pages(-1));
        self.suivante.connect_clicked(pages(1));

        let i = Rc::downgrade(self);
        self.machine.connect_selected_notify(move |d| {
            if let Some(i) = i.upgrade() {
                let ids = i.machines_ids.borrow();
                if let Some(id) = ids.get(d.selected() as usize) {
                    *i.machine_choisie.borrow_mut() = Some(id.clone());
                }
            }
        });
        let i = Rc::downgrade(self);
        self.imprimer.connect_clicked(move |_| {
            if let Some(i) = i.upgrade() {
                let _ = i.cmd.try_send(Commande::ImprimerSelection);
            }
        });
        let i = Rc::downgrade(self);
        self.relancer.connect_clicked(move |_| {
            if let Some(i) = i.upgrade()
                && let Some(t) = i.travail.borrow().clone()
            {
                let _ = i.cmd.try_send(Commande::Relancer(t.id));
            }
        });
        let i = Rc::downgrade(self);
        self.epreuve.connect_clicked(move |_| {
            if let Some(i) = i.upgrade() {
                let _ = i.cmd.try_send(Commande::EpreuveSelection);
            }
        });
        let i = Rc::downgrade(self);
        self.supprimer.connect_clicked(move |_| {
            if let Some(i) = i.upgrade()
                && let Some(t) = i.travail.borrow().clone()
            {
                let _ = i.cmd.try_send(Commande::Supprimer(t.id));
            }
        });
    }

    fn aller_a_la_page(&self, sens: isize) {
        let Some(t) = self.travail.borrow().clone() else { return };
        let n = t.pages.len().max(1) as isize;
        let nouvelle = (self.page.get() as isize + sens).clamp(1, n) as usize;
        if nouvelle != self.page.get() {
            self.page.set(nouvelle);
            self.demander_apercu(&t);
            let sorties = self.dessine.borrow().as_ref().map(|d| d.1.clone()).unwrap_or_default();
            self.dessiner(&t, &sorties);
        }
    }

    /// Travail et page pour la barre d'outils.
    pub fn courant(&self) -> Option<(Travail, usize)> {
        self.travail.borrow().clone().map(|t| (t, self.page.get()))
    }

    /// Machine choisie pour imprimer.
    pub fn machine_choisie(&self) -> Option<String> {
        self.machine_choisie.borrow().clone()
    }

    /// Met l'inspecteur à jour : `None` quand plus aucun travail n'est choisi.
    pub fn maj(&self, travail: Option<&Travail>, file: &[Sortie], machines: &[Machine]) {
        let Some(t) = travail else {
            *self.travail.borrow_mut() = None;
            *self.dessine.borrow_mut() = None;
            *self.apercu_demande.borrow_mut() = None;
            self.racine.set_visible_child_name("vide");
            return;
        };
        let change = self.travail.borrow().as_ref().is_none_or(|a| a.id != t.id);
        if change {
            self.page.set(1);
            *self.dessine.borrow_mut() = None;
            self.apercu.set_paintable(gdk::Paintable::NONE);
        }
        if self.page.get() > t.pages.len().max(1) {
            self.page.set(1);
        }
        *self.travail.borrow_mut() = Some(t.clone());
        self.racine.set_visible_child_name("contenu");
        self.maj_machines(machines);
        let sorties: Vec<Sortie> = file.iter().filter(|s| s.travail == t.id).cloned().collect();
        self.dessiner(t, &sorties);
        self.demander_apercu(t);
    }

    fn maj_machines(&self, machines: &[Machine]) {
        let imprimantes: Vec<&Machine> = machines.iter().filter(|m| m.imprime()).collect();
        let ids: Vec<String> = imprimantes.iter().map(|m| m.id.clone()).collect();
        if *self.machines_ids.borrow() != ids {
            let noms: Vec<&str> = imprimantes.iter().map(|m| if m.nom.is_empty() { m.id.as_str() } else { m.nom.as_str() }).collect();
            self.modele_machines.splice(0, self.modele_machines.n_items(), &noms);
            let precedente = self.machine_choisie.borrow().clone();
            *self.machines_ids.borrow_mut() = ids.clone();
            let position = precedente.and_then(|p| ids.iter().position(|i| *i == p)).unwrap_or(0);
            self.machine.set_selected(position as u32);
            *self.machine_choisie.borrow_mut() = ids.get(position).cloned();
        }
    }

    fn dessiner(&self, t: &Travail, sorties: &[Sortie]) {
        let page = self.page.get();
        let nouveau = (t.clone(), sorties.to_vec(), page);
        if self.dessine.borrow().as_ref() == Some(&nouveau) {
            return;
        }
        self.titre.set_text(&t.nom);
        self.statut.set_text(&t.statut_libelle());
        for c in ["success", "error", "accent", "dim-label"] {
            self.statut.remove_css_class(c);
        }
        self.statut.add_css_class(match t.statut {
            StatutTravail::Termine => "success",
            StatutTravail::Echec => "error",
            StatutTravail::EnCours => "accent",
            StatutTravail::EnAttente => "dim-label",
        });
        let n = t.pages.len();
        self.pagination.set_text(&if n == 0 { String::new() } else { format!("Page {page} sur {n}") });
        self.precedente.set_sensitive(page > 1);
        self.suivante.set_sensitive(page < n);

        // Fiche.
        vider(&self.fiche);
        let f = &t.fiche;
        let lignes: Vec<(&str, String)> = vec![
            ("Client", f.client.clone()),
            ("Société", f.societe.clone()),
            ("Commande", f.commande.clone()),
            ("Dossier ERP", t.erp.as_ref().map(|e| e.ref_erp.clone()).unwrap_or_default()),
            ("Quantité", if f.quantite > 0 { f.quantite.to_string() } else { String::new() }),
            ("Échéance", f.echeance.map(date).unwrap_or_default()),
            ("Notes", f.notes.clone()),
            ("Format de la page", t.pages.get(page - 1).filter(|p| p.largeur_mm > 0.0).map(|p| format!("{} × {} mm", p.largeur_mm.round(), p.hauteur_mm.round())).unwrap_or_default()),
            ("Plaques", t.pages.get(page - 1).map(|p| p.plaques.join(", ")).unwrap_or_default()),
        ];
        for (titre, valeur) in lignes {
            if valeur.is_empty() && matches!(titre, "Société" | "Dossier ERP" | "Notes") {
                continue;
            }
            self.fiche.append(&propriete(titre, &valeur));
        }

        // Sorties de ce travail.
        vider(&self.sorties);
        if sorties.is_empty() {
            self.sorties.append(&adw::ActionRow::builder().title("Aucune sortie dans la file").css_classes(["dim-label"]).build());
        }
        for s in sorties {
            self.sorties.append(&ligne_sortie(s, &self.cmd, Some(&s.machine)));
        }

        // Alertes.
        vider(&self.alertes);
        let mut alertes: Vec<String> = t.alertes.iter().map(|a| libelles::alerte(&a.code, &a.detail, a.page)).collect();
        if let (StatutTravail::Echec, Some(e)) = (t.statut, t.erreur.as_deref().filter(|e| !e.is_empty())) {
            alertes.insert(0, format!("Détail de l'échec : {e}"));
        }
        if alertes.is_empty() {
            self.alertes.append(&adw::ActionRow::builder().title("Aucune alerte").css_classes(["dim-label"]).build());
        }
        for a in &alertes {
            let l = adw::ActionRow::builder().title(glib::markup_escape_text(a)).title_lines(0).build();
            l.add_prefix(&gtk::Image::from_icon_name("dialog-warning-symbolic"));
            self.alertes.append(&l);
        }
        self.onglet_alertes.set_label(&if t.alertes.is_empty() { "Alertes".to_string() } else { format!("Alertes ({})", t.alertes.len()) });

        // Actions : l'impression et l'épreuve demandent un travail rendu.
        let rendu = t.statut == StatutTravail::Termine;
        let imprimable = rendu && !self.machines_ids.borrow().is_empty() && !t.pages.is_empty();
        self.imprimer.set_sensitive(imprimable);
        self.machine.set_sensitive(imprimable);
        self.epreuve.set_sensitive(rendu && !t.pages.is_empty());
        self.relancer.set_sensitive(t.statut != StatutTravail::EnCours);
        *self.dessine.borrow_mut() = Some(nouveau);
    }

    fn demander_apercu(&self, t: &Travail) {
        let page = self.page.get();
        let cle = (t.id.clone(), page, t.statut);
        if self.apercu_demande.borrow().as_ref() == Some(&cle) {
            return;
        }
        *self.apercu_demande.borrow_mut() = Some(cle);
        if t.statut == StatutTravail::Termine && !t.pages.is_empty() {
            self.message_apercu.set_text("Chargement de l'aperçu…");
            self.pile_apercu.set_visible_child_name("message");
            let _ = self.cmd.try_send(Commande::Apercu { travail: t.id.clone(), page });
        } else {
            self.apercu.set_paintable(gdk::Paintable::NONE);
            self.message_apercu.set_text(match t.statut {
                StatutTravail::Echec => "Pas d'aperçu : le rendu a échoué.",
                _ => "Rendu en cours…",
            });
            self.pile_apercu.set_visible_child_name("message");
        }
    }

    /// Aperçu reçu du serveur ; ignoré s'il ne correspond plus à la page affichée.
    pub fn apercu_recu(&self, travail: &str, page: usize, octets: Result<Vec<u8>, String>) {
        let courant = self.travail.borrow().as_ref().map(|t| t.id.clone());
        if courant.as_deref() != Some(travail) || self.page.get() != page {
            return;
        }
        let texture = octets.and_then(|o| gdk::Texture::from_bytes(&glib::Bytes::from_owned(o)).map_err(|e| e.to_string()));
        match texture {
            Ok(tex) => {
                self.apercu.set_paintable(Some(&tex));
                self.pile_apercu.set_visible_child_name("image");
            }
            Err(e) => {
                self.apercu.set_paintable(gdk::Paintable::NONE);
                self.message_apercu.set_text(&format!("Aperçu indisponible : {e}"));
                self.pile_apercu.set_visible_child_name("message");
            }
        }
    }
}
