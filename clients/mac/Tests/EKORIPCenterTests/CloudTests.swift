import Foundation
import Testing
@testable import EKORIPCenter

/// « Se connecter au cloud » : PKCE, URL d'autorisation, lecture du retour.
struct CloudTests {
    @Test func testBase64urlSansRemplissage() {
        // 0xfb 0xff donne « +/8= » en base64 standard.
        #expect(Pkce.base64url(Data([0xfb, 0xff])) == "-_8")
        #expect(Pkce.base64url(Data([0x66])) == "Zg")
        #expect(Pkce.base64url(Data()) == "")
    }

    @Test func testAleatoire() {
        let a = Pkce.aleatoire()
        let b = Pkce.aleatoire()
        #expect(a.count == 43)
        #expect(a != b)
        #expect(a.allSatisfy { $0.isLetter || $0.isNumber || $0 == "-" || $0 == "_" })
    }

    /// RFC 7636, annexe B.
    @Test func testDefiRfc7636() {
        #expect(Pkce.defi(verifier: "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk") == "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM")
    }

    @Test func testUrlAutorisation() throws {
        let url = try #require(Cloud.urlAutorisation(plateforme: "https://platform.e-ko.rip", defi: "DEFI", state: "ETAT", appareil: "Mac de Léa"))
        #expect(url.absoluteString.hasPrefix("https://platform.e-ko.rip/oauth/autoriser?"))
        let c = try #require(URLComponents(url: url, resolvingAgainstBaseURL: false))
        func v(_ n: String) -> String? { c.queryItems?.first { $0.name == n }?.value }
        #expect(v("client_id") == "eko-rip-center")
        #expect(v("response_type") == "code")
        #expect(v("redirect_uri") == "eko-rip://oauth")
        #expect(v("code_challenge") == "DEFI")
        #expect(v("code_challenge_method") == "S256")
        #expect(v("state") == "ETAT")
        #expect(v("appareil") == "Mac de Léa")
        // Tout est encodé : pas de « : » ni de « / » dans la valeur de redirect_uri.
        #expect(url.absoluteString.contains("redirect_uri=eko-rip%3A%2F%2Foauth"))
        #expect(url.absoluteString.contains("appareil=Mac%20de%20L%C3%A9a"))
    }

    @Test func testCorpsJeton() throws {
        let corps = try #require(String(data: Cloud.corpsJeton(code: "abc", verifier: "v-é"), encoding: .utf8))
        #expect(corps == "grant_type=authorization_code&client_id=eko-rip-center&code=abc&redirect_uri=eko-rip%3A%2F%2Foauth&code_verifier=v-%C3%A9")
    }

    @Test func testRetourValide() throws {
        let url = try #require(URL(string: "eko-rip://oauth?code=XYZ&state=ETAT"))
        #expect(try Cloud.lireRetour(url, etatAttendu: "ETAT") == "XYZ")
    }

    @Test func testRetourStateFauxRefuse() throws {
        let url = try #require(URL(string: "eko-rip://oauth?code=XYZ&state=AUTRE"))
        do {
            _ = try Cloud.lireRetour(url, etatAttendu: "ETAT")
            Issue.record("Un state faux doit être refusé.")
        } catch let e as ErreurApi {
            #expect(e.code == "cloud_etat_invalide")
        }
    }

    @Test func testRetourSansStateRefuse() throws {
        let url = try #require(URL(string: "eko-rip://oauth?code=XYZ"))
        #expect(throws: ErreurApi.self) { try Cloud.lireRetour(url, etatAttendu: "ETAT") }
    }

    @Test func testRetourAccesRefuse() throws {
        let url = try #require(URL(string: "eko-rip://oauth?error=access_denied&state=ETAT"))
        do {
            _ = try Cloud.lireRetour(url, etatAttendu: "ETAT")
            Issue.record("Le refus doit être une erreur.")
        } catch let e as ErreurApi {
            #expect(e.code == "cloud_refuse")
        }
    }

    @Test func testRetourSansCodeRefuse() throws {
        let url = try #require(URL(string: "eko-rip://oauth?state=ETAT"))
        #expect(throws: ErreurApi.self) { try Cloud.lireRetour(url, etatAttendu: "ETAT") }
    }

    @Test func testPlateformeParDefautEtSurcharge() throws {
        let defauts = try #require(UserDefaults(suiteName: "fr.e-ko.rip.tests.cloud"))
        defauts.removePersistentDomain(forName: "fr.e-ko.rip.tests.cloud")
        #expect(Cloud.plateforme(defauts) == "https://platform.e-ko.rip")
        defauts.set("https://essai.example.org/", forKey: Cloud.clePlateforme)
        #expect(Cloud.plateforme(defauts) == "https://essai.example.org")
        defauts.set("n'importe quoi", forKey: Cloud.clePlateforme)
        #expect(Cloud.plateforme(defauts) == "https://platform.e-ko.rip")
        defauts.removePersistentDomain(forName: "fr.e-ko.rip.tests.cloud")
    }

    @Test func testDecodageMoi() throws {
        let json = """
        {"compte":{"email":"a@b.fr","nom":"Léa"},"sites":[{"id":"s1","nom":"Atelier","adresse":"https://k3x9ab.platform.e-ko.rip","en_ligne":true,"role":"operateur"}]}
        """
        let moi = try JSONDecoder.serveur.decode(ReponseMoi.self, from: Data(json.utf8))
        #expect(moi.compte.affichable == "Léa")
        #expect(moi.sites.first?.enLigne == true)
        #expect(moi.sites.first?.adresse == "https://k3x9ab.platform.e-ko.rip")
    }
}
