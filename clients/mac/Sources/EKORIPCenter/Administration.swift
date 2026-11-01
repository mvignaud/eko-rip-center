import AppKit
import WebKit

/// Écrans d'administration (supports, règles, tarifs, pont ERP, sauvegarde…) pas encore
/// natifs : en attendant, l'écran complet du serveur dans une fenêtre à part, avec la
/// signature du client. Passerelle temporaire, à retirer quand ces écrans seront natifs.
@MainActor
enum Administration {
    private static var fenetre: NSWindow?
    private static let liens = LiensExternes()

    static func ouvrir(racine: String) {
        guard let url = URL(string: racine + "/") else { return }
        if let f = fenetre, let vue = f.contentView as? WKWebView {
            if vue.url?.host != url.host || vue.url?.port != url.port { vue.load(URLRequest(url: url)) }
            f.makeKeyAndOrderFront(nil)
            return
        }
        let vue = WKWebView(frame: NSRect(x: 0, y: 0, width: 1200, height: 820), configuration: WKWebViewConfiguration())
        vue.customUserAgent = "Mozilla/5.0 " + VersionClient.agent
        vue.uiDelegate = liens
        vue.load(URLRequest(url: url))
        let f = NSWindow(contentRect: vue.frame, styleMask: [.titled, .closable, .miniaturizable, .resizable], backing: .buffered, defer: false)
        f.title = "Administration — E-KO RIP Center"
        f.contentView = vue
        f.isReleasedWhenClosed = false
        f.center()
        f.makeKeyAndOrderFront(nil)
        fenetre = f
    }
}

/// Liens ouverts dans un nouvel onglet par l'écran du serveur (« Aide », documentation
/// sur e-ko.rip) : WebKit ne les ouvre pas seul, ils partent dans le navigateur par défaut.
@MainActor
final class LiensExternes: NSObject, WKUIDelegate {
    func webView(_ webView: WKWebView, createWebViewWith configuration: WKWebViewConfiguration, for navigationAction: WKNavigationAction, windowFeatures: WKWindowFeatures) -> WKWebView? {
        if let url = navigationAction.request.url, url.scheme == "https" {
            NSWorkspace.shared.open(url)
        }
        return nil
    }
}
