import Foundation

/// Version du client, lue dans l'Info.plist de l'application (« dev » hors du .app).
enum VersionClient {
    static let numero: String = Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "dev"
    /// Sans cette signature, le serveur traite le poste comme un navigateur (`client_requis`).
    static let agent = "EKO-RIP-Client/\(numero) (macOS)"
}

/// Erreur renvoyée par le serveur (`{"code","detail"}`) ou née du réseau (statut 0).
struct ErreurApi: Error, LocalizedError, Sendable {
    let statut: Int
    let code: String
    let detail: String?

    var errorDescription: String? { Libelles.erreur(code, detail) }

    /// Le serveur demande de se connecter (cookie absent ou expiré).
    var connexionRequise: Bool { statut == 401 && code != "identifiants_invalides" }

    static func reseau(_ erreur: Error) -> ErreurApi {
        ErreurApi(statut: 0, code: "reseau", detail: (erreur as? URLError)?.localizedDescription ?? erreur.localizedDescription)
    }

    static func depuis(statut: Int, corps: Data) -> ErreurApi {
        if let c = try? JSONDecoder().decode(CorpsErreur.self, from: corps), let code = c.code {
            return ErreurApi(statut: statut, code: code, detail: c.detail)
        }
        // Corps absent ou pas du JSON du serveur (proxy, limite de taille, identifiant mal formé).
        if statut == 413 { return ErreurApi(statut: statut, code: "trop_gros", detail: nil) }
        return ErreurApi(statut: statut, code: "erreur_http", detail: "HTTP \(statut)")
    }
}

/// Client de l'API JSON du serveur. Une session `URLSession` éphémère à lui : son stockage
/// de cookies garde `eko_rip_session` sans toucher à celui du système. Rien n'est partagé
/// entre deux serveurs : changer d'adresse crée une nouvelle `Api`.
struct Api: Sendable {
    /// Adresse du serveur sans barre finale : `http://hôte:8640`.
    let racine: String
    let session: URLSession

    init(racine: String) {
        self.racine = racine
        let cfg = URLSessionConfiguration.ephemeral
        cfg.httpCookieAcceptPolicy = .always
        cfg.httpShouldSetCookies = true
        cfg.timeoutIntervalForRequest = 60
        self.session = URLSession(configuration: cfg)
    }

    /// Adresse saisie (`hôte`, `hôte:8640`, `http://hôte:8640/`) vers la racine du serveur.
    /// Port 8640 par défaut, http par défaut (le serveur est en HTTP sur le réseau local).
    static func normaliser(_ saisie: String) -> String? {
        var texte = saisie.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !texte.isEmpty else { return nil }
        if !texte.contains("://") { texte = "http://" + texte }
        guard var c = URLComponents(string: texte), let hote = c.host, !hote.isEmpty,
              c.scheme == "http" || c.scheme == "https" else { return nil }
        if c.port == nil && c.scheme == "http" { c.port = 8640 }
        c.path = ""
        c.query = nil
        c.fragment = nil
        c.user = nil
        c.password = nil
        guard let url = c.url else { return nil }
        return url.absoluteString
    }

    /// Forme courte pour l'affichage et la mémorisation : sans « http:// ».
    static func affichable(_ racine: String) -> String {
        racine.hasPrefix("http://") ? String(racine.dropFirst("http://".count)) : racine
    }

    // MARK: Requêtes

    private func requete(_ methode: String, _ chemin: String, json: Data? = nil, delai: TimeInterval? = nil) throws -> URLRequest {
        guard let url = URL(string: racine + chemin) else {
            throw ErreurApi(statut: 0, code: "adresse_invalide", detail: nil)
        }
        var r = URLRequest(url: url)
        r.httpMethod = methode
        r.setValue(VersionClient.agent, forHTTPHeaderField: "User-Agent")
        r.setValue("application/json", forHTTPHeaderField: "Accept")
        if let json {
            r.httpBody = json
            r.setValue("application/json", forHTTPHeaderField: "Content-Type")
        }
        if let delai { r.timeoutInterval = delai }
        return r
    }

    private func reponse(_ resultat: (Data, URLResponse)) throws -> Data {
        guard let http = resultat.1 as? HTTPURLResponse else {
            throw ErreurApi(statut: 0, code: "reponse_illisible", detail: nil)
        }
        guard (200..<300).contains(http.statusCode) else {
            throw ErreurApi.depuis(statut: http.statusCode, corps: resultat.0)
        }
        return resultat.0
    }

    private func executer(_ r: URLRequest) async throws -> Data {
        do {
            return try reponse(try await session.data(for: r))
        } catch let e as ErreurApi {
            throw e
        } catch let e as URLError where e.code == .cancelled {
            throw CancellationError()
        } catch {
            throw ErreurApi.reseau(error)
        }
    }

    private func lire<T: Decodable>(_ chemin: String, _ type: T.Type = T.self) async throws -> T {
        let donnees = try await executer(try requete("GET", chemin))
        do {
            return try JSONDecoder.serveur.decode(T.self, from: donnees)
        } catch {
            throw ErreurApi(statut: 0, code: "reponse_illisible", detail: String(describing: error))
        }
    }

    private func envoyerJSON(_ methode: String, _ chemin: String, _ corps: [String: Any]? = nil) async throws {
        let json = try corps.map { try JSONSerialization.data(withJSONObject: $0) }
        _ = try await executer(try requete(methode, chemin, json: json))
    }

    // MARK: Session

    func infoSession() async throws -> InfoSession { try await lire("/api/session") }

    func ouvrirSession(nom: String, motDePasse: String) async throws {
        try await envoyerJSON("POST", "/api/session", ["nom": nom, "mot_de_passe": motDePasse])
    }

    func fermerSession() async throws { try await envoyerJSON("DELETE", "/api/session") }

    // MARK: Lectures

    func travaux() async throws -> [Travail] { try await lire("/api/travaux") }
    func file() async throws -> [Sortie] { try await lire("/api/file") }
    func machines() async throws -> [Machine] { try await lire("/api/machines") }
    func systeme() async throws -> InfoSysteme { try await lire("/api/systeme") }
    func reglages() async throws -> [Reglage] { try await lire("/api/reglages") }

    // MARK: Actions

    /// Met une page en file. `operation` (`impression` ou `decoupe`) évite l'ambiguïté sur
    /// une machine qui fait les deux.
    func mettreEnFile(travail: String, page: Int, machine: String, operation: String) async throws {
        try await envoyerJSON("POST", "/api/file", ["travail": travail, "page": page, "machine": machine, "operation": operation])
    }

    func relancer(travail: String) async throws { try await envoyerJSON("POST", "/api/travaux/\(travail)/relancer") }
    func supprimer(travail: String) async throws { try await envoyerJSON("DELETE", "/api/travaux/\(travail)") }

    func agir(sortie: String, action: ActionSortie) async throws {
        if action == .retirer {
            try await envoyerJSON("DELETE", "/api/file/\(sortie)")
        } else {
            try await envoyerJSON("POST", "/api/file/\(sortie)/\(action.rawValue)")
        }
    }

    // MARK: Aperçu et épreuve

    /// PNG de la page (numérotée à partir de 1), avec le cookie de la session.
    func apercu(travail: String, page: Int) async throws -> Data {
        try await executer(try requete("GET", "/api/travaux/\(travail)/pages/\(page)/apercu.png"))
    }

    /// Télécharge l'épreuve TIFF dans `destination`, sans la tenir en mémoire. Le serveur la
    /// fabrique parfois à la demande : délai large.
    func telechargerEpreuve(travail: String, page: Int, vers destination: URL) async throws {
        let r = try requete("GET", "/api/travaux/\(travail)/pages/\(page)/epreuve.tif", delai: 900)
        try await withCheckedThrowingContinuation { (suite: CheckedContinuation<Void, Error>) in
            // Le fichier temporaire n'existe que dans ce rappel : on le déplace tout de suite.
            let tache = session.downloadTask(with: r) { local, rep, erreur in
                if let erreur {
                    if (erreur as? URLError)?.code == .cancelled {
                        suite.resume(throwing: CancellationError())
                    } else {
                        suite.resume(throwing: ErreurApi.reseau(erreur))
                    }
                    return
                }
                guard let local, let http = rep as? HTTPURLResponse else {
                    suite.resume(throwing: ErreurApi(statut: 0, code: "reponse_illisible", detail: nil))
                    return
                }
                guard (200..<300).contains(http.statusCode) else {
                    let corps = (try? Data(contentsOf: local)) ?? Data()
                    suite.resume(throwing: ErreurApi.depuis(statut: http.statusCode, corps: corps))
                    return
                }
                do {
                    try? FileManager.default.removeItem(at: destination)
                    try FileManager.default.moveItem(at: local, to: destination)
                    suite.resume()
                } catch {
                    suite.resume(throwing: error)
                }
            }
            tache.resume()
        }
    }

    // MARK: Envoi d'un fichier

    /// `POST /api/travaux` en multipart. Le corps est écrit dans un fichier temporaire puis
    /// envoyé depuis le disque : le fichier n'est jamais entier en mémoire. `progression`
    /// reçoit une fraction de 0 à 1.
    func envoyer(fichier: URL, champs: [(String, String)], progression: @escaping @Sendable (Double) -> Void) async throws -> Travail {
        let frontiere = "EKO-" + UUID().uuidString
        let corps: URL
        do {
            corps = try Multipart.fabriquer(champs: champs, fichier: fichier, frontiere: frontiere)
        } catch {
            throw ErreurApi(statut: 0, code: "fichier_illisible", detail: error.localizedDescription)
        }
        defer { try? FileManager.default.removeItem(at: corps) }

        var r = try requete("POST", "/api/travaux", delai: 900)
        r.setValue("multipart/form-data; boundary=\(frontiere)", forHTTPHeaderField: "Content-Type")
        let suivi = SuiviEnvoi(progression)
        do {
            let resultat = try await session.upload(for: r, fromFile: corps, delegate: suivi)
            let donnees = try reponse(resultat)
            do {
                return try JSONDecoder.serveur.decode(Travail.self, from: donnees)
            } catch {
                throw ErreurApi(statut: 0, code: "reponse_illisible", detail: String(describing: error))
            }
        } catch let e as ErreurApi {
            throw e
        } catch let e as URLError where e.code == .cancelled {
            throw CancellationError()
        } catch {
            throw ErreurApi.reseau(error)
        }
    }
}

/// Délégué d'une tâche d'envoi : transmet la progression, au plus ~200 fois.
final class SuiviEnvoi: NSObject, URLSessionTaskDelegate, @unchecked Sendable {
    private let rappel: @Sendable (Double) -> Void
    private var derniere = -1.0

    init(_ rappel: @escaping @Sendable (Double) -> Void) {
        self.rappel = rappel
        super.init()
    }

    func urlSession(_ session: URLSession, task: URLSessionTask, didSendBodyData bytesSent: Int64, totalBytesSent: Int64, totalBytesExpectedToSend: Int64) {
        guard totalBytesExpectedToSend > 0 else { return }
        let fraction = Double(totalBytesSent) / Double(totalBytesExpectedToSend)
        if fraction - derniere >= 0.005 || fraction >= 1 {
            derniere = fraction
            rappel(fraction)
        }
    }
}

/// Corps `multipart/form-data` écrit sur disque.
enum Multipart {
    static func fabriquer(champs: [(String, String)], fichier: URL, frontiere: String) throws -> URL {
        let fm = FileManager.default
        let destination = fm.temporaryDirectory.appendingPathComponent("eko-envoi-\(UUID().uuidString).multipart")
        guard fm.createFile(atPath: destination.path, contents: nil) else {
            throw CocoaError(.fileWriteUnknown)
        }
        let sortie = try FileHandle(forWritingTo: destination)
        defer { try? sortie.close() }
        do {
            for (nom, valeur) in champs {
                let bloc = "--\(frontiere)\r\nContent-Disposition: form-data; name=\"\(nom)\"\r\n\r\n\(valeur)\r\n"
                try sortie.write(contentsOf: Data(bloc.utf8))
            }
            // Le fichier en dernier, comme l'attend le serveur.
            let nomFichier = fichier.lastPathComponent
                .replacingOccurrences(of: "\"", with: "%22")
                .replacingOccurrences(of: "\r", with: "")
                .replacingOccurrences(of: "\n", with: "")
            let entete = "--\(frontiere)\r\nContent-Disposition: form-data; name=\"fichier\"; filename=\"\(nomFichier)\"\r\nContent-Type: application/octet-stream\r\n\r\n"
            try sortie.write(contentsOf: Data(entete.utf8))

            let entree = try FileHandle(forReadingFrom: fichier)
            defer { try? entree.close() }
            var fini = false
            while !fini {
                // Un pool par tour : sans lui, les blocs d'un fichier de plusieurs Go
                // s'accumuleraient jusqu'à la fin de la boucle.
                try autoreleasepool {
                    if let bloc = try entree.read(upToCount: 1 << 20), !bloc.isEmpty {
                        try sortie.write(contentsOf: bloc)
                    } else {
                        fini = true
                    }
                }
            }
            try sortie.write(contentsOf: Data("\r\n--\(frontiere)--\r\n".utf8))
        } catch {
            try? fm.removeItem(at: destination)
            throw error
        }
        return destination
    }
}
