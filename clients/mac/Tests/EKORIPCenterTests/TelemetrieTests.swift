import Foundation
import Testing
@testable import EKORIPCenter

/// Règle de décision du ping anonyme d'installation.
struct TelemetrieTests {
    private let maintenant: TimeInterval = 1_800_000_000
    private let jour: TimeInterval = 86_400

    @Test func testDesactive() {
        let etat = EtatTelemetrie(versionSignalee: nil, dernierPing: nil, desactive: true)
        #expect(Telemetrie.evenement(etat: etat, version: "1.0.0", maintenant: maintenant) == nil)
        let actif = EtatTelemetrie(versionSignalee: nil, dernierPing: nil, desactive: false)
        #expect(Telemetrie.evenement(etat: actif, version: "1.0.0", maintenant: maintenant, desactiveParEnv: true) == nil)
        #expect(Telemetrie.desactiveParEnvironnement("Off"))
        #expect(Telemetrie.desactiveParEnvironnement("0"))
        #expect(!Telemetrie.desactiveParEnvironnement("1"))
        #expect(!Telemetrie.desactiveParEnvironnement(nil))
    }

    @Test func testInstallation() {
        let etat = EtatTelemetrie(versionSignalee: nil, dernierPing: nil, desactive: false)
        #expect(Telemetrie.evenement(etat: etat, version: "1.0.0", maintenant: maintenant) == "installation")
    }

    @Test func testMiseAJour() {
        let etat = EtatTelemetrie(versionSignalee: "0.9.0", dernierPing: maintenant - jour, desactive: false)
        #expect(Telemetrie.evenement(etat: etat, version: "1.0.0", maintenant: maintenant) == "mise_a_jour")
    }

    @Test func testHebdo() {
        let etat = EtatTelemetrie(versionSignalee: "1.0.0", dernierPing: maintenant - 7 * jour, desactive: false)
        #expect(Telemetrie.evenement(etat: etat, version: "1.0.0", maintenant: maintenant) == "hebdo")
    }

    @Test func testRien() {
        let etat = EtatTelemetrie(versionSignalee: "1.0.0", dernierPing: maintenant - 6 * jour, desactive: false)
        #expect(Telemetrie.evenement(etat: etat, version: "1.0.0", maintenant: maintenant) == nil)
    }

    @Test func testIdentifiant() {
        let id = Telemetrie.nouvelIdentifiant()
        #expect(id.count == 32)
        #expect(id.allSatisfy { $0.isHexDigit && !$0.isUppercase })
    }
}
