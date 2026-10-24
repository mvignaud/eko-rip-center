//! Client de bureau E-KO RIP Center : une fenêtre sur l'interface du serveur.
//! La page locale `ui/index.html` demande l'adresse du serveur et la retient.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem as P, Submenu};

/// Page locale de choix du serveur ; son origine dépend du système (règle de Tauri 2).
const PAGE_CHOIX: &str = if cfg!(windows) {
    "http://tauri.localhost/index.html?changer=1"
} else {
    "tauri://localhost/index.html?changer=1"
};

fn main() {
    tauri::Builder::default()
        // Menu en français : celui de Tauri par défaut (Menu::default) est en anglais, sur
        // macOS comme sur Windows. Éléments standard du système, avec leurs raccourcis.
        .menu(|app| {
            let changer = MenuItem::with_id(app, "changer", "Changer de serveur…", true, Some("CmdOrCtrl+Shift+S"))?;
            let recharger = MenuItem::with_id(app, "recharger", "Recharger", true, Some("CmdOrCtrl+R"))?;
            let serveur = Submenu::with_items(app, "Serveur", true, &[&changer, &recharger])?;
            let edition = Submenu::with_items(
                app,
                "Édition",
                true,
                &[
                    &P::undo(app, Some("Annuler"))?,
                    &P::redo(app, Some("Rétablir"))?,
                    &P::separator(app)?,
                    &P::cut(app, Some("Couper"))?,
                    &P::copy(app, Some("Copier"))?,
                    &P::paste(app, Some("Coller"))?,
                    &P::select_all(app, Some("Tout sélectionner"))?,
                ],
            )?;
            let fenetre = Submenu::with_items(app, "Fenêtre", true, &[&P::minimize(app, Some("Réduire"))?, &P::maximize(app, Some("Agrandir"))?, &P::separator(app)?, &P::close_window(app, Some("Fermer la fenêtre"))?])?;
            #[cfg(target_os = "macos")]
            {
                let nom = "E-KO RIP Center";
                let application = Submenu::with_items(
                    app,
                    nom,
                    true,
                    &[
                        &P::about(app, Some("À propos d'E-KO RIP Center"), None)?,
                        &P::separator(app)?,
                        &P::services(app, Some("Services"))?,
                        &P::separator(app)?,
                        &P::hide(app, Some("Masquer E-KO RIP Center"))?,
                        &P::hide_others(app, Some("Masquer les autres"))?,
                        &P::show_all(app, Some("Tout afficher"))?,
                        &P::separator(app)?,
                        &P::quit(app, Some("Quitter E-KO RIP Center"))?,
                    ],
                )?;
                let presentation = Submenu::with_items(app, "Présentation", true, &[&P::fullscreen(app, Some("Plein écran"))?])?;
                Menu::with_items(app, &[&application, &edition, &presentation, &serveur, &fenetre])
            }
            #[cfg(not(target_os = "macos"))]
            {
                let fichier = Submenu::with_items(app, "Fichier", true, &[&P::quit(app, Some("Quitter"))?])?;
                Menu::with_items(app, &[&fichier, &edition, &serveur, &fenetre])
            }
        })
        .on_menu_event(|app, evenement| {
            let Some(fenetre) = app.get_webview_window("main") else { return };
            match evenement.id().as_ref() {
                "changer" => {
                    let _ = fenetre.navigate(PAGE_CHOIX.parse().expect("adresse de la page locale"));
                }
                "recharger" => {
                    let _ = fenetre.eval("location.reload()");
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("démarrage du client E-KO RIP Center");
}
