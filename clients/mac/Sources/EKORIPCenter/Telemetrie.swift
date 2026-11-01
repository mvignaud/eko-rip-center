import Foundation

// Ping anonyme d'installation (protocole commun E-KO RIP) : un identifiant aléatoire, le type
// de poste, l'OS et la version, pour compter les installations. Jamais bloquant, jamais
// d'erreur affichée : en cas d'échec, on réessaie au tour suivant.

/// Clés de UserDefaults du ping, préfixées `telemetrie.`.
enum ClesTelemetrie {
    static let id = "telemetrie.id"
    static let versionSignalee = "telemetrie.version_signalee"
    static let dernierPing = "telemetrie.dernier_ping"
    static let desactive = "telemetrie.desactive"
}

/// État gardé localement.
struct EtatTelemetrie: Sendable, Equatable {
    var versionSignalee: String?
    /// Secondes epoch du dernier ping réussi.
    var dernierPing: TimeInterval?
    var desactive: Bool
}

enum Telemetrie {
    static let adresseParDefaut = "https://platform.e-ko.rip/api/ping"
    static let intervalle: Duration = .seconds(6 * 3600)
    static let septJours: TimeInterval = 7 * 86_400

    /// Faux pour `EKO_TELEMETRIE` = 0, non, false ou off (sans tenir compte de la casse).
    static func desactiveParEnvironnement(_ valeur: String?) -> Bool {
        guard let v = valeur?.trimmingCharacters(in: .whitespaces).lowercased() else { return false }
        return ["0", "non", "false", "off"].contains(v)
    }

    /// Événement à envoyer, ou nil s'il n'y a rien à signaler.
    static func evenement(etat: EtatTelemetrie, version: String, maintenant: TimeInterval, desactiveParEnv: Bool = false) -> String? {
        if etat.desactive || desactiveParEnv { return nil }
        guard let signalee = etat.versionSignalee else { return "installation" }
        if signalee != version { return "mise_a_jour" }
        if maintenant - (etat.dernierPing ?? 0) >= septJours { return "hebdo" }
        return nil
    }

    /// 32 caractères hexadécimaux minuscules.
    static func nouvelIdentifiant() -> String {
        UUID().uuidString.replacingOccurrences(of: "-", with: "").lowercased()
    }

    static func osCourant() -> String {
        let v = ProcessInfo.processInfo.operatingSystemVersion
        return "macOS \(v.majorVersion).\(v.minorVersion).\(v.patchVersion)"
    }

    /// Lit l'état, tire l'identifiant la première fois.
    static func lireEtat(_ d: UserDefaults = .standard) -> EtatTelemetrie {
        EtatTelemetrie(
            versionSignalee: d.string(forKey: ClesTelemetrie.versionSignalee),
            dernierPing: d.object(forKey: ClesTelemetrie.dernierPing) as? Double,
            desactive: d.bool(forKey: ClesTelemetrie.desactive)
        )
    }

    static func identifiant(_ d: UserDefaults = .standard) -> String {
        if let id = d.string(forKey: ClesTelemetrie.id), id.count == 32 { return id }
        let id = nouvelIdentifiant()
        d.set(id, forKey: ClesTelemetrie.id)
        return id
    }

    /// Boucle d'arrière-plan : un essai au lancement, puis toutes les 6 h.
    static func boucle() async {
        while !Task.isCancelled {
            await essayer()
            try? await Task.sleep(for: intervalle)
        }
    }

    /// Un essai : envoie le ping si la règle de décision le demande ; silencieux en cas d'échec.
    static func essayer() async {
        let version = VersionClient.numero
        // Hors du .app (swift run, tests) la version est « dev » : on ne compte pas ces lancements.
        guard version != "dev" else { return }
        let env = ProcessInfo.processInfo.environment["EKO_TELEMETRIE"]
        let maintenant = Date().timeIntervalSince1970
        guard let nature = evenement(
            etat: lireEtat(), version: version, maintenant: maintenant,
            desactiveParEnv: desactiveParEnvironnement(env)
        ) else { return }

        let adresse = ProcessInfo.processInfo.environment["EKO_TELEMETRIE_URL"] ?? adresseParDefaut
        guard let url = URL(string: adresse) else { return }
        let corps: [String: String] = [
            "id": identifiant(),
            "type": "client-mac",
            "evenement": nature,
            "version": String(version.prefix(32)),
            "os": String(osCourant().prefix(64)),
        ]
        guard let donnees = try? JSONSerialization.data(withJSONObject: corps) else { return }
        var requete = URLRequest(url: url)
        requete.httpMethod = "POST"
        requete.timeoutInterval = 10
        requete.setValue("application/json", forHTTPHeaderField: "Content-Type")
        requete.httpBody = donnees
        guard let (_, reponse) = try? await URLSession.shared.data(for: requete),
              (reponse as? HTTPURLResponse)?.statusCode == 204 else { return }
        let d = UserDefaults.standard
        d.set(version, forKey: ClesTelemetrie.versionSignalee)
        d.set(Date().timeIntervalSince1970, forKey: ClesTelemetrie.dernierPing)
    }
}
