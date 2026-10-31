import Foundation
import Testing
@testable import EKORIPCenter

/// Corps de `POST /api/rapports`.
struct SignalementTests {
    private func json(_ c: CorpsRapport) throws -> [String: Any] {
        let donnees = try JSONEncoder().encode(c)
        return try #require(try JSONSerialization.jsonObject(with: donnees) as? [String: Any])
    }

    @Test func testCles() throws {
        let c = try #require(CorpsRapport.fabriquer(
            titre: "  Plantage  ", description: "Détail", email: "a@b.fr", joindreJournal: true,
            type: "client-mac", version: "0.30.0", os: "macOS 26.1.0"))
        let o = try json(c)
        #expect(Set(o.keys) == ["titre", "description", "email", "joindre_journal", "client"])
        #expect(o["titre"] as? String == "Plantage")
        #expect(o["joindre_journal"] as? Bool == true)
        let client = try #require(o["client"] as? [String: Any])
        #expect(client["type"] as? String == "client-mac")
        #expect(client["version"] as? String == "0.30.0")
        #expect(client["os"] as? String == "macOS 26.1.0")
    }

    @Test func testJournalDecoche() throws {
        let c = try #require(CorpsRapport.fabriquer(titre: "T", description: "D", email: "", joindreJournal: false))
        let o = try json(c)
        #expect(o["joindre_journal"] as? Bool == false)
        #expect(o["email"] == nil)
        #expect((o["client"] as? [String: Any])?["type"] as? String == "client-mac")
    }

    @Test func testLimites() {
        #expect(CorpsRapport.fabriquer(titre: " ", description: "D", email: "", joindreJournal: true) == nil)
        #expect(CorpsRapport.fabriquer(titre: "T", description: "", email: "", joindreJournal: true) == nil)
        #expect(CorpsRapport.fabriquer(titre: String(repeating: "a", count: 121), description: "D", email: "", joindreJournal: true) == nil)
        #expect(CorpsRapport.fabriquer(titre: String(repeating: "a", count: 120), description: "D", email: "", joindreJournal: true) != nil)
        #expect(CorpsRapport.fabriquer(titre: "T", description: String(repeating: "a", count: 5001), email: "", joindreJournal: true) == nil)
        #expect(CorpsRapport.fabriquer(titre: "T", description: "D", email: String(repeating: "a", count: 121), joindreJournal: true) == nil)
    }

    @Test func testLibellesDesCodes() {
        for code in ["rapport_invalide", "rapports_sature", "plateforme_injoignable"] {
            #expect(Libelles.erreur(code, nil) != code)
        }
    }
}
