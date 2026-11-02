import Foundation

// Notifications du cloud : suivi du curseur `dernier`/`apres` et choix de la cible du clic.
// Module pur (ni réseau, ni UserNotifications), testé dans NotificationsCloudTests. L'appel
// HTTP est dans Cloud.swift (ClientCloud.notifications), la boucle et l'affichage dans Etat.swift.

/// Une notification de `GET /api/moi/notifications`.
struct NotificationCloud: Decodable, Sendable, Equatable, Identifiable {
    let id: Int
    let genre: String
    let site: String?
    let titre: String
    let corps: String
    let lien: String

    private enum CodingKeys: String, CodingKey {
        case id, genre = "type", site, titre, corps, lien
    }
}

/// Réponse de `GET /api/moi/notifications`.
struct ReponseNotifications: Decodable, Sendable, Equatable {
    let notifications: [NotificationCloud]
    let dernier: Int
}

/// Curseur : id de la dernière notification traitée. `nil` tant que rien n'est mémorisé
/// (premier appel : sans `apres`, sans rattrapage).
struct SuiviNotifications: Equatable, Sendable {
    private(set) var apres: Int?

    init(apres: Int? = nil) {
        self.apres = apres
    }

    /// Suite à donner à une réponse.
    struct Suite: Equatable {
        /// Notifications nouvelles, à afficher, dans l'ordre.
        var aAfficher: [NotificationCloud] = []
        /// Il en reste côté plate-forme : relancer tout de suite avec le nouvel `apres`.
        var relancer = false
    }

    /// Absorbe une réponse. Sans curseur : on mémorise `dernier`, rien à afficher. Avec : on ne
    /// garde que les ids au-delà du curseur et on avance jusqu'au dernier reçu.
    mutating func absorber(_ reponse: ReponseNotifications) -> Suite {
        guard let courant = apres else {
            apres = reponse.dernier
            return Suite()
        }
        let nouvelles = reponse.notifications.filter { $0.id > courant }
        guard let dernierRecu = nouvelles.map(\.id).max() else {
            // Rien d'affichable : on rattrape `dernier` pour ne pas redemander les mêmes ids.
            apres = max(courant, reponse.dernier)
            return Suite()
        }
        apres = dernierRecu
        return Suite(aAfficher: nouvelles, relancer: reponse.dernier > dernierRecu)
    }
}

/// Ce que fait le clic sur une notification.
enum CibleNotification: Equatable {
    /// Ouvrir ce site du compte dans l'app.
    case site(String)
    /// Ouvrir cette adresse dans le navigateur.
    case lien(URL)

    /// Site connu du compte : on l'ouvre ; sinon le lien (http ou https seulement).
    static func choisir(site: String?, lien: String?, sites: [SiteCloud]) -> CibleNotification? {
        if let site, !site.isEmpty, sites.contains(where: { $0.id == site }) {
            return .site(site)
        }
        guard let lien, let url = URL(string: lien.trimmingCharacters(in: .whitespacesAndNewlines)),
              let schema = url.scheme?.lowercased(), schema == "https" || schema == "http",
              url.host != nil else { return nil }
        return .lien(url)
    }
}
