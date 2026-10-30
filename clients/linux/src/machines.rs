//! Page Machines : une carte par machine, avec ses sorties actives et leurs actions.

use crate::commande::Commande;
use crate::libelles;
use crate::modele::{EtatSortie, Machine, Sortie};
use adw::prelude::*;
use async_channel::Sender;
use std::cell::RefCell;

pub struct Machines {
    pub racine: gtk::ScrolledWindow,
    boite: gtk::Box,
    cmd: Sender<Commande>,
    dernier: RefCell<Option<(Vec<Machine>, Vec<Sortie>)>>,
}

/// Actions proposées selon l'état de la sortie (le serveur refuse celles qui n'ont pas de sens).
fn actions_de(etat: EtatSortie) -> &'static [(&'static str, &'static str)] {
    match etat {
        EtatSortie::EnCours => &[("suspendre", "Suspendre"), ("annuler", "Annuler")],
        EtatSortie::EnAttente => &[("suspendre", "Suspendre"), ("en_tete", "En tête"), ("monter", "Monter"), ("descendre", "Descendre"), ("annuler", "Annuler")],
        EtatSortie::Suspendue => &[("reprendre", "Reprendre"), ("en_tete", "En tête"), ("monter", "Monter"), ("descendre", "Descendre"), ("annuler", "Annuler")],
        EtatSortie::Terminee | EtatSortie::Echec | EtatSortie::Annulee => &[("refaire", "Refaire")],
        EtatSortie::Imbriquee => &[],
    }
}

/// Bouton « … » d'une sortie, avec ses actions.
pub fn menu_actions(s: &Sortie, cmd: &Sender<Commande>) -> Option<gtk::MenuButton> {
    let actions = actions_de(s.etat);
    if actions.is_empty() {
        return None;
    }
    let boite = gtk::Box::builder().orientation(gtk::Orientation::Vertical).build();
    let fenetre = gtk::Popover::builder().child(&boite).build();
    for (action, libelle) in actions {
        let b = gtk::Button::builder().label(*libelle).css_classes(["flat"]).build();
        if let Some(etiquette) = b.child().and_downcast::<gtk::Label>() {
            etiquette.set_xalign(0.0);
        }
        let (cmd, id, action, fenetre) = (cmd.clone(), s.id.clone(), *action, fenetre.clone());
        b.connect_clicked(move |_| {
            fenetre.popdown();
            let _ = cmd.try_send(Commande::Sortie { id: id.clone(), action });
        });
        boite.append(&b);
    }
    Some(gtk::MenuButton::builder().icon_name("view-more-symbolic").popover(&fenetre).valign(gtk::Align::Center).css_classes(["flat", "circular"]).tooltip_text("Actions").build())
}

/// Ligne d'une sortie : nom et page, état, raison d'attente.
pub fn ligne_sortie(s: &Sortie, cmd: &Sender<Commande>, avec_machine: Option<&str>) -> adw::ActionRow {
    let titre = match avec_machine {
        Some(m) => format!("{} · page {}", m, s.page),
        None => format!("{} · page {}", s.nom, s.page),
    };
    let mut sous = s.etat_libelle();
    if let Some(c) = &s.couche {
        sous.push_str(&format!(" · couche {c}"));
    }
    if let Some((titre_raison, _)) = s.raison_attente.as_deref().and_then(libelles::raison) {
        sous.push_str(&format!(" · {titre_raison}"));
    }
    let ligne = adw::ActionRow::builder().title(gtk::glib::markup_escape_text(&titre)).subtitle(gtk::glib::markup_escape_text(&sous)).build();
    if let Some((_, explication)) = s.raison_attente.as_deref().and_then(libelles::raison) {
        ligne.set_tooltip_text(Some(explication));
    }
    if let Some(m) = menu_actions(s, cmd) {
        ligne.add_suffix(&m);
    }
    ligne
}

impl Machines {
    pub fn new(cmd: Sender<Commande>) -> Self {
        let boite = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(18).margin_top(18).margin_bottom(18).margin_start(18).margin_end(18).build();
        let racine = gtk::ScrolledWindow::builder().child(&adw::Clamp::builder().maximum_size(820).child(&boite).build()).hscrollbar_policy(gtk::PolicyType::Never).vexpand(true).build();
        Machines { racine, boite, cmd, dernier: RefCell::new(None) }
    }

    pub fn maj(&self, machines: &[Machine], file: &[Sortie]) {
        let actives: Vec<Sortie> = file.iter().filter(|s| matches!(s.etat, EtatSortie::EnCours | EtatSortie::EnAttente | EtatSortie::Suspendue) && !s.archivee).cloned().collect();
        let nouveau = (machines.to_vec(), actives);
        if self.dernier.borrow().as_ref() == Some(&nouveau) {
            return;
        }
        while let Some(enfant) = self.boite.first_child() {
            self.boite.remove(&enfant);
        }
        if machines.is_empty() {
            let vide = adw::StatusPage::builder().icon_name("printer-symbolic").title("Aucune machine").description("Le catalogue du serveur est vide.").build();
            self.boite.append(&vide);
        }
        for m in &nouveau.0 {
            let sorties: Vec<&Sortie> = nouveau.1.iter().filter(|s| s.machine == m.id).collect();
            self.boite.append(&self.carte(m, &sorties));
        }
        *self.dernier.borrow_mut() = Some(nouveau);
    }

    fn carte(&self, m: &Machine, sorties: &[&Sortie]) -> adw::PreferencesGroup {
        let mut detail = format!("{} · {}", m.modele_libelle(), m.operations());
        if m.laize_mm > 0.0 {
            detail.push_str(&format!(" · laize {} mm", m.laize_mm.round()));
        }
        let groupe = adw::PreferencesGroup::builder().title(gtk::glib::markup_escape_text(if m.nom.is_empty() { &m.id } else { &m.nom })).description(gtk::glib::markup_escape_text(&detail)).build();
        let liste = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::None).css_classes(["boxed-list"]).build();
        if sorties.is_empty() {
            let vide = adw::ActionRow::builder().title("Rien en file").build();
            vide.add_css_class("dim-label");
            liste.append(&vide);
        }
        for s in sorties {
            liste.append(&ligne_sortie(s, &self.cmd, None));
        }
        groupe.add(&liste);
        groupe
    }
}
