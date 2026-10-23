//! Client de bureau E-KO RIP Center : une fenêtre sur l'interface du serveur.
//! La page locale `ui/index.html` demande l'adresse du serveur et la retient.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;
use tauri::menu::{Menu, MenuItem, Submenu};

/// Page locale de choix du serveur ; son origine dépend du système (règle de Tauri 2).
const PAGE_CHOIX: &str = if cfg!(windows) {
    "http://tauri.localhost/index.html?changer=1"
} else {
    "tauri://localhost/index.html?changer=1"
};

fn main() {
    tauri::Builder::default()
        .menu(|app| {
            let menu = Menu::default(app)?;
            let changer = MenuItem::with_id(app, "changer", "Changer de serveur / Change server", true, Some("CmdOrCtrl+Shift+S"))?;
            let recharger = MenuItem::with_id(app, "recharger", "Recharger / Reload", true, Some("CmdOrCtrl+R"))?;
            menu.append(&Submenu::with_items(app, "Serveur / Server", true, &[&changer, &recharger])?)?;
            Ok(menu)
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
