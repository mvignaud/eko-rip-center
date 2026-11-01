import Foundation
import CryptoKit
import Security
import AppKit
import AuthenticationServices

// « Se connecter au cloud » : OAuth 2 (code d'autorisation + PKCE, RFC 6749 et 7636) contre
// la plate-forme E-KO, jeton gardé dans le trousseau. Contrat côté serveur : docs/fr/cloud.md,
// section « Se connecter au cloud depuis les apps ». Le jeton ne passe jamais par
// UserDefaults ni par un journal.

// MARK: - PKCE (pur)

enum Pkce {
    /// Base64 « URL » (RFC 4648 §5) sans remplissage.
    static func base64url(_ donnees: Data) -> String {
        donnees.base64EncodedString()
            .replacingOccurrences(of: "+", with: "-")
            .replacingOccurrences(of: "/", with: "_")
            .replacingOccurrences(of: "=", with: "")
    }

    /// `n` octets aléatoires du système, en base64url (32 octets : 43 caractères).
    static func aleatoire(octets n: Int = 32) -> String {
        var octets = [UInt8](repeating: 0, count: n)
        let statut = SecRandomCopyBytes(kSecRandomDefault, n, &octets)
        if statut != errSecSuccess {
            // Ne devrait jamais arriver ; repli sur le générateur du système de Swift (sûr aussi).
            var generateur = SystemRandomNumberGenerator()
            octets = (0..<n).map { _ in UInt8.random(in: 0...255, using: &generateur) }
        }
        return base64url(Data(octets))
    }

    /// `code_challenge` (méthode S256) : base64url(SHA-256(verifier)).
    static func defi(verifier: String) -> String {
        base64url(Data(SHA256.hash(data: Data(verifier.utf8))))
    }
}

// MARK: - Contrat d'autorisation (pur)

enum Cloud {
    static let plateformeParDefaut = "https://platform.e-ko.rip"
    /// Clé UserDefaults pour viser une autre plate-forme (essais).
    static let clePlateforme = "plateformeCloud"
    static let identifiantClient = "eko-rip-center"
    static let schema = "eko-rip"
    static let redirection = "eko-rip://oauth"

    /// Plate-forme réglée (clé `plateformeCloud`) ou celle par défaut, sans barre finale.
    static func plateforme(_ defauts: UserDefaults = .standard) -> String {
        let saisie = defauts.string(forKey: clePlateforme)?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        guard !saisie.isEmpty, let url = URL(string: saisie), let sch = url.scheme,
              sch == "https" || sch == "http", url.host != nil else {
            return plateformeParDefaut
        }
        return saisie.hasSuffix("/") ? String(saisie.dropLast()) : saisie
    }

    /// Nom de l'appareil montré sur la page d'autorisation de la plate-forme.
    static func nomAppareil() -> String {
        let nom = Host.current().localizedName ?? ""
        return nom.isEmpty ? "Mac" : nom
    }

    /// Encodage strict d'une valeur de requête : seuls `A-Z a-z 0-9 - . _ ~` restent tels quels.
    static func encoder(_ texte: String) -> String {
        // `CharacterSet.alphanumerics` contient aussi les lettres accentuées : liste ASCII explicite.
        let autorises = CharacterSet(charactersIn: "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~")
        return texte.addingPercentEncoding(withAllowedCharacters: autorises) ?? texte
    }

    /// `GET {plateforme}/oauth/autoriser?…`, ouvert dans la session d'authentification.
    static func urlAutorisation(plateforme: String, defi: String, state: String, appareil: String) -> URL? {
        let parametres: [(String, String)] = [
            ("client_id", identifiantClient),
            ("response_type", "code"),
            ("redirect_uri", redirection),
            ("code_challenge", defi),
            ("code_challenge_method", "S256"),
            ("state", state),
            ("appareil", appareil),
        ]
        let requete = parametres.map { "\($0.0)=\(encoder($0.1))" }.joined(separator: "&")
        return URL(string: plateforme + "/oauth/autoriser?" + requete)
    }

    /// Lit `eko-rip://oauth?code=…&state=…` (ou `?error=access_denied&state=…`) et rend le code.
    /// Le `state` doit être celui qu'on a envoyé, sinon le retour est refusé (même en cas d'erreur).
    static func lireRetour(_ url: URL, etatAttendu: String) throws -> String {
        guard url.scheme == schema,
              let c = URLComponents(url: url, resolvingAgainstBaseURL: false) else {
            throw ErreurApi(statut: 0, code: "cloud_retour_invalide", detail: nil)
        }
        func valeur(_ nom: String) -> String? { c.queryItems?.first { $0.name == nom }?.value }
        guard let state = valeur("state"), state == etatAttendu else {
            throw ErreurApi(statut: 0, code: "cloud_etat_invalide", detail: nil)
        }
        if let erreur = valeur("error") {
            if erreur == "access_denied" { throw ErreurApi(statut: 0, code: "cloud_refuse", detail: nil) }
            throw ErreurApi(statut: 0, code: "cloud_retour_invalide", detail: erreur)
        }
        guard let code = valeur("code"), !code.isEmpty else {
            throw ErreurApi(statut: 0, code: "cloud_retour_invalide", detail: nil)
        }
        return code
    }

    /// Corps `application/x-www-form-urlencoded` de `POST /oauth/jeton`.
    static func corpsJeton(code: String, verifier: String) -> Data {
        let champs: [(String, String)] = [
            ("grant_type", "authorization_code"),
            ("client_id", identifiantClient),
            ("code", code),
            ("redirect_uri", redirection),
            ("code_verifier", verifier),
        ]
        return Data(champs.map { "\($0.0)=\(encoder($0.1))" }.joined(separator: "&").utf8)
    }
}

// MARK: - Formes JSON de la plate-forme

struct CompteCloud: Decodable, Sendable, Equatable {
    let email: String
    let nom: String?

    /// Nom à afficher : le nom du compte, sinon l'e-mail.
    var affichable: String {
        if let nom, !nom.isEmpty { return nom }
        return email
    }
}

struct SiteCloud: Decodable, Sendable, Identifiable, Equatable {
    let id: String
    let nom: String
    let adresse: String
    let enLigne: Bool
    let role: String?
}

struct ReponseMoi: Decodable, Sendable {
    let compte: CompteCloud
    let sites: [SiteCloud]
}

private struct ReponseJeton: Decodable {
    let accessToken: String?
    let tokenType: String?
    let error: String?
}

// MARK: - Appels à la plate-forme

/// Appels HTTP à la plate-forme (hors site). Session éphémère, sans cookies.
struct ClientCloud: Sendable {
    let plateforme: String
    let session: URLSession

    init(plateforme: String = Cloud.plateforme()) {
        self.plateforme = plateforme
        let cfg = URLSessionConfiguration.ephemeral
        cfg.httpShouldSetCookies = false
        cfg.timeoutIntervalForRequest = 30
        self.session = URLSession(configuration: cfg)
    }

    private func requete(_ methode: String, _ chemin: String, jeton: String? = nil) throws -> URLRequest {
        guard let url = URL(string: plateforme + chemin) else {
            throw ErreurApi(statut: 0, code: "adresse_invalide", detail: nil)
        }
        var r = URLRequest(url: url)
        r.httpMethod = methode
        r.setValue(VersionClient.agent, forHTTPHeaderField: "User-Agent")
        r.setValue("application/json", forHTTPHeaderField: "Accept")
        if let jeton { r.setValue("Bearer \(jeton)", forHTTPHeaderField: "Authorization") }
        return r
    }

    private func envoyer(_ r: URLRequest) async throws -> (Data, HTTPURLResponse) {
        do {
            let (donnees, reponse) = try await session.data(for: r)
            guard let http = reponse as? HTTPURLResponse else {
                throw ErreurApi(statut: 0, code: "reponse_illisible", detail: nil)
            }
            return (donnees, http)
        } catch let e as ErreurApi {
            throw e
        } catch let e as URLError where e.code == .cancelled {
            throw CancellationError()
        } catch {
            throw ErreurApi.reseau(error)
        }
    }

    /// `POST /oauth/jeton` : échange le code contre le jeton.
    func echanger(code: String, verifier: String) async throws -> String {
        var r = try requete("POST", "/oauth/jeton")
        r.setValue("application/x-www-form-urlencoded", forHTTPHeaderField: "Content-Type")
        r.httpBody = Cloud.corpsJeton(code: code, verifier: verifier)
        let (donnees, http) = try await envoyer(r)
        let lu = try? JSONDecoder.serveur.decode(ReponseJeton.self, from: donnees)
        guard (200..<300).contains(http.statusCode) else {
            throw ErreurApi(statut: http.statusCode, code: "cloud_jeton_refuse", detail: lu?.error)
        }
        guard let jeton = lu?.accessToken, !jeton.isEmpty else {
            throw ErreurApi(statut: 0, code: "reponse_illisible", detail: nil)
        }
        return jeton
    }

    /// `GET /api/moi` : le compte et ses sites. 401 : jeton révoqué ou expiré.
    func moi(jeton: String) async throws -> ReponseMoi {
        let (donnees, http) = try await envoyer(try requete("GET", "/api/moi", jeton: jeton))
        guard (200..<300).contains(http.statusCode) else {
            if http.statusCode == 401 { throw ErreurApi(statut: 401, code: "jeton_invalide", detail: nil) }
            throw ErreurApi.depuis(statut: http.statusCode, corps: donnees)
        }
        do {
            return try JSONDecoder.serveur.decode(ReponseMoi.self, from: donnees)
        } catch {
            throw ErreurApi(statut: 0, code: "reponse_illisible", detail: String(describing: error))
        }
    }

    /// `POST /api/moi/deconnexion` (204) : révoque le jeton côté plate-forme.
    func deconnecter(jeton: String) async throws {
        let (donnees, http) = try await envoyer(try requete("POST", "/api/moi/deconnexion", jeton: jeton))
        guard (200..<300).contains(http.statusCode) else {
            throw ErreurApi.depuis(statut: http.statusCode, corps: donnees)
        }
    }
}

// MARK: - Trousseau

/// Le jeton de la plate-forme, dans le trousseau de l'utilisateur : une seule entrée.
enum Trousseau {
    static let service = "fr.e-ko.rip.cloud"
    private static let compte = "jeton"

    private static var base: [String: Any] {
        [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: compte,
        ]
    }

    static func lire() -> String? {
        var requete = base
        requete[kSecReturnData as String] = true
        requete[kSecMatchLimit as String] = kSecMatchLimitOne
        var resultat: CFTypeRef?
        guard SecItemCopyMatching(requete as CFDictionary, &resultat) == errSecSuccess,
              let donnees = resultat as? Data,
              let jeton = String(data: donnees, encoding: .utf8), !jeton.isEmpty else { return nil }
        return jeton
    }

    /// Remplace l'entrée existante.
    static func ecrire(_ jeton: String) throws {
        oublier()
        var entree = base
        entree[kSecValueData as String] = Data(jeton.utf8)
        entree[kSecAttrAccessible as String] = kSecAttrAccessibleWhenUnlockedThisDeviceOnly
        let statut = SecItemAdd(entree as CFDictionary, nil)
        guard statut == errSecSuccess else {
            throw ErreurApi(statut: 0, code: "trousseau_echec", detail: "code \(statut)")
        }
    }

    static func oublier() {
        SecItemDelete(base as CFDictionary)
    }
}

// MARK: - Session d'authentification

/// Ouvre la page d'autorisation de la plate-forme (ASWebAuthenticationSession) et rend le
/// jeton. Une seule autorisation à la fois ; l'objet garde la session vivante jusqu'au retour.
@MainActor
final class AutorisationCloud: NSObject, ASWebAuthenticationPresentationContextProviding {
    private var session: ASWebAuthenticationSession?

    nonisolated func presentationAnchor(for session: ASWebAuthenticationSession) -> ASPresentationAnchor {
        MainActor.assumeIsolated {
            NSApp.keyWindow ?? NSApp.windows.first ?? ASPresentationAnchor()
        }
    }

    /// Déroule tout le protocole et rend le jeton (non stocké : c'est à l'appelant).
    func autoriser(plateforme: String = Cloud.plateforme()) async throws -> String {
        let verifier = Pkce.aleatoire()
        let etat = Pkce.aleatoire(octets: 16)
        guard let url = Cloud.urlAutorisation(plateforme: plateforme, defi: Pkce.defi(verifier: verifier),
                                              state: etat, appareil: Cloud.nomAppareil()) else {
            throw ErreurApi(statut: 0, code: "adresse_invalide", detail: nil)
        }
        let retour = try await ouvrir(url)
        let code = try Cloud.lireRetour(retour, etatAttendu: etat)
        return try await ClientCloud(plateforme: plateforme).echanger(code: code, verifier: verifier)
    }

    private func ouvrir(_ url: URL) async throws -> URL {
        defer { session = nil }
        return try await withCheckedThrowingContinuation { (suite: CheckedContinuation<URL, Error>) in
            let s = ASWebAuthenticationSession(url: url, callbackURLScheme: Cloud.schema) { retour, erreur in
                if let retour {
                    suite.resume(returning: retour)
                } else if let e = erreur as? ASWebAuthenticationSessionError, e.code == .canceledLogin {
                    suite.resume(throwing: ErreurApi(statut: 0, code: "cloud_annule", detail: nil))
                } else {
                    suite.resume(throwing: ErreurApi(statut: 0, code: "cloud_echec", detail: erreur?.localizedDescription))
                }
            }
            s.presentationContextProvider = self
            s.prefersEphemeralWebBrowserSession = false
            session = s
            if !s.start() {
                session = nil
                suite.resume(throwing: ErreurApi(statut: 0, code: "cloud_echec", detail: nil))
            }
        }
    }
}
