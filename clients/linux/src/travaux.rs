//! Tableau des travaux (`gtk::ColumnView`), plus récents en haut.

use crate::commande::Commande;
use crate::modele::{StatutTravail, Travail, date};
use async_channel::Sender;
use gtk::glib::BoxedAnyObject;
use gtk::prelude::*;
use gtk::{gio, glib};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct Travaux {
    pub vue: gtk::ColumnView,
    magasin: gio::ListStore,
    selection: gtk::SingleSelection,
    affiches: RefCell<Vec<Travail>>,
    /// Vrai pendant que le programme change lui-même la sélection.
    muet: Cell<bool>,
}

fn travail_de(objet: &glib::Object) -> Option<std::cell::Ref<'_, Travail>> {
    objet.downcast_ref::<BoxedAnyObject>().map(|o| o.borrow::<Travail>())
}

impl Travaux {
    pub fn new(cmd: Sender<Commande>) -> Rc<Self> {
        let magasin = gio::ListStore::new::<BoxedAnyObject>();
        let selection = gtk::SingleSelection::builder().model(&magasin).autoselect(false).can_unselect(true).build();
        let vue = gtk::ColumnView::builder().model(&selection).show_row_separators(true).hexpand(true).vexpand(true).build();
        vue.add_css_class("data-table");

        colonne(&vue, "Nom", 260, true, |t| t.nom.clone(), None);
        colonne(&vue, "Dossier", 130, false, |t| t.dossier().to_string(), None);
        colonne(&vue, "Pages", 60, false, |t| t.pages.len().to_string(), None);
        colonne(&vue, "Statut", 220, false, Travail::statut_libelle, Some(classe_statut));
        colonne(&vue, "Date", 150, false, |t| date(t.cree_le), None);

        let this = Rc::new(Travaux { vue, magasin, selection, affiches: RefCell::new(vec![]), muet: Cell::new(false) });
        let faible = Rc::downgrade(&this);
        this.selection.connect_selection_changed(move |sel, _, _| {
            let Some(t) = faible.upgrade() else { return };
            if t.muet.get() {
                return;
            }
            let id = sel.selected_item().and_then(|o| travail_de(&o).map(|t| t.id.clone()));
            let _ = cmd.try_send(Commande::Choisir(id));
        });
        this
    }

    /// Remplace la liste affichée en gardant, si possible, le travail choisi et la position.
    pub fn maj(&self, travaux: &[Travail], choisi: Option<&str>) {
        self.muet.set(true);
        let identiques = {
            let a = self.affiches.borrow();
            a.len() == travaux.len() && a.iter().zip(travaux).all(|(x, y)| x.id == y.id)
        };
        if identiques {
            // Mêmes lignes dans le même ordre : seules celles qui ont changé sont relues.
            let mut a = self.affiches.borrow_mut();
            for (i, nouveau) in travaux.iter().enumerate() {
                if a[i] != *nouveau {
                    if let Some(o) = self.magasin.item(i as u32).and_downcast::<BoxedAnyObject>() {
                        *o.borrow_mut::<Travail>() = nouveau.clone();
                    }
                    a[i] = nouveau.clone();
                    self.magasin.items_changed(i as u32, 1, 1);
                }
            }
        } else {
            let objets: Vec<BoxedAnyObject> = travaux.iter().map(|t| BoxedAnyObject::new(t.clone())).collect();
            self.magasin.splice(0, self.magasin.n_items(), &objets);
            *self.affiches.borrow_mut() = travaux.to_vec();
        }
        let position = choisi.and_then(|id| travaux.iter().position(|t| t.id == id));
        self.selection.set_selected(position.map_or(gtk::INVALID_LIST_POSITION, |p| p as u32));
        self.muet.set(false);
    }
}

fn classe_statut(t: &Travail) -> &'static str {
    match t.statut {
        StatutTravail::Termine => "success",
        StatutTravail::Echec => "error",
        StatutTravail::EnCours => "accent",
        StatutTravail::EnAttente => "dim-label",
    }
}

const CLASSES_STATUT: [&str; 4] = ["success", "error", "accent", "dim-label"];

fn colonne(vue: &gtk::ColumnView, titre: &str, largeur: i32, etire: bool, texte: fn(&Travail) -> String, classe: Option<fn(&Travail) -> &'static str>) {
    let usine = gtk::SignalListItemFactory::new();
    usine.connect_setup(|_, objet| {
        let Some(ligne) = objet.downcast_ref::<gtk::ListItem>() else { return };
        let label = gtk::Label::builder().xalign(0.0).ellipsize(gtk::pango::EllipsizeMode::End).margin_start(6).margin_end(6).margin_top(6).margin_bottom(6).build();
        ligne.set_child(Some(&label));
    });
    usine.connect_bind(move |_, objet| {
        let Some(ligne) = objet.downcast_ref::<gtk::ListItem>() else { return };
        let (Some(label), Some(item)) = (ligne.child().and_downcast::<gtk::Label>(), ligne.item()) else { return };
        let Some(t) = travail_de(&item) else { return };
        label.set_text(&texte(&t));
        for c in CLASSES_STATUT {
            label.remove_css_class(c);
        }
        if let Some(classe) = classe {
            label.add_css_class(classe(&t));
        }
    });
    let col = gtk::ColumnViewColumn::new(Some(titre), Some(usine));
    col.set_fixed_width(largeur);
    col.set_resizable(true);
    col.set_expand(etire);
    vue.append_column(&col);
}
