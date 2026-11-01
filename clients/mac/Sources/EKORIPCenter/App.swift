import SwiftUI
import AppKit
import UserNotifications

@main
@MainActor
struct EKORIPCenterApp: App {
    @NSApplicationDelegateAdaptor(DelegueApp.self) private var delegue
    private let etat = Etat.partage

    var body: some Scene {
        Window("E-KO RIP Center", id: "principal") {
            RacineVue()
                .environment(etat)
        }
        .defaultSize(width: 1280, height: 780)
        .windowResizability(.contentMinSize)
        .windowToolbarStyle(.unified)
        .commands {
            // Une seule fenêtre : « Nouvelle fenêtre » laisse la place à l'envoi.
            CommandGroup(replacing: .newItem) {
                Button("Envoyer un fichier…") { etat.choisirFichiers() }
                    .keyboardShortcut("o")
                    .disabled(etat.phase != .connecte)
            }
            CommandGroup(replacing: .help) {
                Button("Signaler un problème…") { etat.signalementOuvert = true }
                    .disabled(etat.phase != .connecte)
            }
            CommandMenu("Serveur") {
                Button("Actualiser") { Task { await etat.rafraichir() } }
                    .keyboardShortcut("r")
                    .disabled(etat.phase != .connecte)
                // La page d'administration est une page web : le jeton du cloud ne peut pas y
                // passer. En mode cloud, le menu est grisé avec sa raison.
                Button(etat.siteCloud == nil ? "Administration…" : "Administration… (\(Libelles.cloudAdminIndisponible.lowercased()))") {
                    if let racine = etat.api?.racine { Administration.ouvrir(racine: racine) }
                }
                    .keyboardShortcut(",", modifiers: [.command, .option])
                    .disabled(etat.phase != .connecte || etat.siteCloud != nil)
                Divider()
                Button("Changer de serveur…") { etat.changerServeur() }
                    .keyboardShortcut("s", modifiers: [.command, .shift])
                Button("Se déconnecter") { Task { await etat.deconnecter() } }
                    .disabled(etat.phase != .connecte || etat.siteCloud != nil || etat.session?.accesLibre == true)
                Button(Libelles.cloudDeconnexion) { Task { await etat.deconnecterCloud() } }
                    .disabled(etat.cloudCompte == nil)
            }
        }
    }
}

/// Écran affiché selon la phase de connexion ; feuille d'envoi et messages d'erreur.
@MainActor
struct RacineVue: View {
    @Environment(Etat.self) private var etat

    var body: some View {
        Group {
            switch etat.phase {
            case .demarrage:
                ProgressView()
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            case .adresse, .connexion:
                ConnexionVue()
                    .frame(minWidth: 520, minHeight: 420)
            case .connecte:
                PrincipalVue()
            }
        }
        .task { await etat.demarrer() }
        .sheet(isPresented: Binding(
            get: { !etat.fichiersAEnvoyer.isEmpty && etat.phase == .connecte },
            set: { if !$0 { etat.fichiersAEnvoyer = [] } }
        )) {
            FeuilleEnvoi(fichiers: etat.fichiersAEnvoyer)
        }
        .sheet(isPresented: Binding(
            get: { etat.signalementOuvert && etat.phase == .connecte },
            set: { etat.signalementOuvert = $0 }
        )) {
            FeuilleSignalement()
        }
        .alert(
            etat.message?.titre ?? "",
            isPresented: Binding(get: { etat.message != nil }, set: { if !$0 { etat.message = nil } }),
            presenting: etat.message
        ) { _ in
            Button("OK") {}
        } message: { m in
            Text(m.texte)
        }
    }
}

/// Ouverture de fichiers par le Dock (ou « Ouvrir avec ») et notifications au premier plan.
@MainActor
final class DelegueApp: NSObject, NSApplicationDelegate, UNUserNotificationCenterDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        if Notifier.disponible {
            UNUserNotificationCenter.current().delegate = self
        }
        // Ping anonyme d'installation, hors de l'interface : au lancement puis toutes les 6 h.
        Task.detached(priority: .background) { await Telemetrie.boucle() }
    }

    func application(_ application: NSApplication, open urls: [URL]) {
        Etat.partage.proposerEnvoi(urls)
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }

    /// Affiche aussi la notification quand l'application est au premier plan.
    nonisolated func userNotificationCenter(_ center: UNUserNotificationCenter, willPresent notification: UNNotification) async -> UNNotificationPresentationOptions {
        [.banner, .sound]
    }
}
