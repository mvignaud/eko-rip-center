import Foundation
import Network

/// Un serveur E-KO RIP annoncé sur le réseau local (DNS-SD `_eko-rip._tcp`).
struct ServeurTrouve: Identifiable, Hashable {
    let service: String
    let nom: String
    /// `http://ip:port`
    let adresse: String
    let version: String

    var id: String { service }

    /// `http://ip:port` à partir d'une adresse IPv4 (suffixe de zone « %en0 » retiré).
    static func adresse(ip: String, port: UInt16) -> String {
        let propre = ip.split(separator: "%").first.map(String.init) ?? ip
        return "http://\(propre):\(port)"
    }
}

/// Recherche des serveurs du réseau local : `NWBrowser` trouve les services, puis une connexion
/// TCP éphémère vers chaque service donne son adresse IPv4 et son port réels (plus fiable que de
/// deviner un nom « .local »). La recherche dure environ 3 s.
@MainActor
@Observable
final class Decouverte {
    private(set) var serveurs: [ServeurTrouve] = []
    private(set) var recherche = false

    private var navigateur: NWBrowser?
    private var connexions: [String: NWConnection] = [:]
    private var generation = 0

    func chercher() {
        arreter()
        generation += 1
        let courante = generation
        serveurs = []
        recherche = true
        let nav = NWBrowser(for: .bonjourWithTXTRecord(type: "_eko-rip._tcp", domain: nil), using: .tcp)
        nav.browseResultsChangedHandler = { [weak self] resultats, _ in
            Task { @MainActor in self?.traiter(resultats, generation: courante) }
        }
        nav.stateUpdateHandler = { [weak self] etat in
            if case .failed = etat {
                Task { @MainActor in
                    if self?.generation == courante { self?.recherche = false }
                }
            }
        }
        nav.start(queue: .main)
        navigateur = nav
        Task { [weak self] in
            try? await Task.sleep(for: .seconds(3))
            guard let self, courante == self.generation else { return }
            self.navigateur?.cancel()
            self.navigateur = nil
            self.recherche = false
            // Les résolutions en cours disposent encore de quelques secondes.
            try? await Task.sleep(for: .seconds(3))
            if courante == self.generation { self.annulerConnexions() }
        }
    }

    func arreter() {
        generation += 1
        navigateur?.cancel()
        navigateur = nil
        annulerConnexions()
        recherche = false
    }

    private func annulerConnexions() {
        connexions.values.forEach { $0.cancel() }
        connexions = [:]
    }

    private func traiter(_ resultats: Set<NWBrowser.Result>, generation courante: Int) {
        guard courante == generation else { return }
        for resultat in resultats {
            guard case let .service(nomService, _, _, _) = resultat.endpoint else { continue }
            if connexions[nomService] != nil || serveurs.contains(where: { $0.service == nomService }) { continue }
            var nom = nomService
            var version = ""
            if case let .bonjour(txt) = resultat.metadata {
                if let n = txt["nom"], !n.isEmpty { nom = n }
                version = txt["version"] ?? ""
            }
            resoudre(resultat.endpoint, service: nomService, nom: nom, version: version, generation: courante)
        }
    }

    private func resoudre(_ endpoint: NWEndpoint, service: String, nom: String, version: String, generation courante: Int) {
        let parametres = NWParameters.tcp
        if let ip = parametres.defaultProtocolStack.internetProtocol as? NWProtocolIP.Options {
            ip.version = .v4
        }
        let connexion = NWConnection(to: endpoint, using: parametres)
        connexions[service] = connexion
        connexion.stateUpdateHandler = { [weak self] etat in
            switch etat {
            case .ready:
                let distant = connexion.currentPath?.remoteEndpoint
                connexion.cancel()
                Task { @MainActor in
                    self?.ajouter(distant, service: service, nom: nom, version: version, generation: courante)
                }
            case .failed:
                connexion.cancel()
            default:
                break
            }
        }
        connexion.start(queue: .main)
    }

    private func ajouter(_ distant: NWEndpoint?, service: String, nom: String, version: String, generation courante: Int) {
        guard courante == generation, case let .hostPort(hote, port)? = distant else { return }
        let ip = hote.debugDescription
        // IPv4 seulement : une adresse IPv6 contient des « : ».
        guard !ip.contains(":"), !serveurs.contains(where: { $0.service == service }) else { return }
        let trouve = ServeurTrouve(service: service, nom: nom, adresse: ServeurTrouve.adresse(ip: ip, port: port.rawValue), version: version)
        serveurs.append(trouve)
        serveurs.sort { $0.nom.localizedCaseInsensitiveCompare($1.nom) == .orderedAscending }
    }
}
