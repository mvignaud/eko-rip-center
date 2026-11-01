import Foundation
import Testing
@testable import EKORIPCenter

/// Notifications du cloud : curseur `dernier`/`apres`, décodage, cible du clic.
struct NotificationsCloudTests {
    private func notif(_ id: Int, site: String? = nil) -> NotificationCloud {
        NotificationCloud(id: id, genre: "nouvelle_app", site: site, titre: "t\(id)", corps: "c", lien: "https://platform.e-ko.rip/x")
    }

    private func reponse(_ ids: [Int], dernier: Int) -> ReponseNotifications {
        ReponseNotifications(notifications: ids.map { notif($0) }, dernier: dernier)
    }

    private func site(_ id: String) -> SiteCloud {
        SiteCloud(id: id, nom: id, adresse: "https://\(id).platform.e-ko.rip", enLigne: true, role: nil)
    }

    @Test func testPremierAppelMemoriseSansRattrapage() {
        var suivi = SuiviNotifications()
        #expect(suivi.apres == nil)
        let suite = suivi.absorber(reponse([], dernier: 42))
        #expect(suite.aAfficher.isEmpty)
        #expect(!suite.relancer)
        #expect(suivi.apres == 42)
    }

    @Test func testPremierAppelIgnoreMemeDesNotifications() {
        var suivi = SuiviNotifications()
        #expect(suivi.absorber(reponse([1, 2], dernier: 2)).aAfficher.isEmpty)
        #expect(suivi.apres == 2)
    }

    @Test func testNouvellesNotificationsAvancentLeCurseur() {
        var suivi = SuiviNotifications(apres: 10)
        let suite = suivi.absorber(reponse([11, 12], dernier: 12))
        #expect(suite.aAfficher.map(\.id) == [11, 12])
        #expect(!suite.relancer)
        #expect(suivi.apres == 12)
    }

    @Test func testRelanceSiDernierDepasse() {
        var suivi = SuiviNotifications(apres: 10)
        let suite = suivi.absorber(reponse([11, 12], dernier: 80))
        #expect(suite.relancer)
        #expect(suivi.apres == 12)
    }

    @Test func testIdsDejaVusNeSontPasReaffiches() {
        var suivi = SuiviNotifications(apres: 12)
        #expect(suivi.absorber(reponse([11, 12, 13], dernier: 13)).aAfficher.map(\.id) == [13])
    }

    @Test func testListeVideRattrapeDernierSansBoucle() {
        var suivi = SuiviNotifications(apres: 10)
        let suite = suivi.absorber(reponse([], dernier: 30))
        #expect(suite.aAfficher.isEmpty)
        #expect(!suite.relancer)
        #expect(suivi.apres == 30)
        _ = suivi.absorber(reponse([], dernier: 5))
        #expect(suivi.apres == 30)
    }

    @Test func testDecodageJson() throws {
        let json = #"{"notifications":[{"id":3,"type":"site_hors_ligne","site":null,"titre":"T","corps":"C","lien":"https://p/s","cree_le":"2026-01-01"}],"dernier":3}"#
        let r = try JSONDecoder.serveur.decode(ReponseNotifications.self, from: Data(json.utf8))
        #expect(r.dernier == 3)
        #expect(r.notifications.first?.genre == "site_hors_ligne")
        #expect(r.notifications.first?.site == nil)
    }

    @Test func testCibleSiteConnuSinonLien() {
        let sites = [site("a"), site("b")]
        #expect(CibleNotification.choisir(site: "b", lien: "https://l.example/x", sites: sites) == .site("b"))
        #expect(CibleNotification.choisir(site: "z", lien: "https://l.example/x", sites: sites) == .lien(URL(string: "https://l.example/x")!))
        #expect(CibleNotification.choisir(site: nil, lien: "https://l.example/x", sites: sites) == .lien(URL(string: "https://l.example/x")!))
        #expect(CibleNotification.choisir(site: "", lien: "https://l.example/x", sites: []) == .lien(URL(string: "https://l.example/x")!))
        #expect(CibleNotification.choisir(site: "a", lien: nil, sites: sites) == .site("a"))
    }

    @Test func testCibleLienInvalide() {
        #expect(CibleNotification.choisir(site: "z", lien: "", sites: []) == nil)
        #expect(CibleNotification.choisir(site: nil, lien: nil, sites: []) == nil)
        #expect(CibleNotification.choisir(site: nil, lien: "file:///etc/passwd", sites: []) == nil)
        #expect(CibleNotification.choisir(site: nil, lien: "javascript:alert(1)", sites: []) == nil)
    }
}
