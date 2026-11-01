import SwiftUI
import AppKit
import UniformTypeIdentifiers
import UserNotifications

// État de l'application : connexion, données du serveur, actions. Tout vit sur l'acteur
// principal ; les appels réseau (Api) s'exécutent hors de lui.

/// Clés de UserDefaults (et d'@AppStorage), hors de tout acteur.
enum Cles {
    static let adresse = "adresseServeur"
    /// Identifiant du dernier site cloud ouvert (absent si le dernier choix est un serveur local).
    static let siteCloud = "dernierSiteCloud"
    /// Dernier onglet de l'écran de connexion : « local » ou « cloud ».
    static let modeConnexion = "modeConnexion"
}

enum Rubrique: String, CaseIterable, Identifiable {
    case travaux, machines, systeme

    var id: String { rawValue }

    var titre: String {
        switch self {
        case .travaux: return "Travaux"
        case .machines: return "Machines"
        case .systeme: return "Système"
        }
    }

    var symbole: String {
        switch self {
        case .travaux: return "tray.full"
        case .machines: return "printer"
        case .systeme: return "gearshape"
        }
    }
}

enum Phase: Equatable {
    /// Au lancement, avant d'avoir interrogé le serveur.
    case demarrage
    /// Il faut une adresse de serveur (première fois, serveur injoignable, changement).
    case adresse
    /// Le serveur demande un identifiant.
    case connexion
    case connecte
}

struct MessageUtilisateur: Identifiable {
    let id = UUID()
    let titre: String
    let texte: String
}

/// Un fichier en cours d'envoi, avec sa progression.
@MainActor
@Observable
final class EnvoiEnCours: Identifiable {
    let id = UUID()
    let nom: String
    var progression: Double = 0
    var termine = false
    var erreur: String?
    @ObservationIgnored var tache: Task<Void, Never>?

    init(nom: String) {
        self.nom = nom
    }
}

@MainActor
@Observable
final class Etat {
    static let partage = Etat()
    /// Extensions proposées à l'envoi (l'ouverture par le Dock ne déclare que les cinq formats
    /// du cahier des charges ; `.ai` est un PDF pour le serveur).
    static let extensionsAcceptees: Set<String> = ["pdf", "ai", "svg", "tif", "tiff", "png", "jpg", "jpeg"]

    // Connexion
    var phase: Phase = .demarrage
    var occupe = false
    var erreurConnexion: String?
    var session: InfoSession?
    var adresse = ""
    /// Compte et sites du cloud (rempli quand un jeton est dans le trousseau et valide).
    var cloudCompte: CompteCloud?
    var cloudSites: [SiteCloud] = []
    /// Site cloud ouvert ; nil sur un serveur local.
    var siteCloud: SiteCloud?
    /// Vrai pendant la lecture de `/api/moi`.
    var cloudEnLecture = false

    // Données du serveur
    var travaux: [Travail] = []
    var sorties: [Sortie] = []
    var machines: [Machine] = []
    var reglages: [Reglage] = []
    var systeme: InfoSysteme?
    /// Dernière erreur d'actualisation (serveur injoignable…), effacée au prochain succès.
    var serveurHorsLigne: String?

    // Interface
    var rubrique: Rubrique = .travaux
    var selection: String?
    /// Page affichée dans l'inspecteur, à partir de 1.
    var page = 1
    var recherche = ""
    var inspecteurVisible = true
    var message: MessageUtilisateur?
    var annonce: String?
    var fichiersAEnvoyer: [URL] = []
    var signalementOuvert = false
    var envois: [EnvoiEnCours] = []

    private(set) var api: Api?
    @ObservationIgnored private let autorisationCloud = AutorisationCloud()

    @ObservationIgnored private var boucle: Task<Void, Never>?
    @ObservationIgnored private var annonceTache: Task<Void, Never>?
    @ObservationIgnored private var statutsVus: [String: StatutTravail] = [:]
    @ObservationIgnored private var etatsVus: [String: EtatSortie] = [:]
    @ObservationIgnored private var suivis: Set<String> = []
    @ObservationIgnored private var premierPassage = true
    @ObservationIgnored private var cycle = 0

    // MARK: Sélection et filtres

    var travailSelectionne: Travail? {
        guard let selection else { return nil }
        return travaux.first { $0.id == selection }
    }

    /// Travaux dont le nom ou le dossier contient la recherche (sans tenir compte de la casse
    /// ni des accents).
    var travauxFiltres: [Travail] {
        let q = recherche.trimmingCharacters(in: .whitespaces)
        guard !q.isEmpty else { return travaux }
        let options: String.CompareOptions = [.caseInsensitive, .diacriticInsensitive]
        return travaux.filter { $0.nom.range(of: q, options: options) != nil || $0.dossier.range(of: q, options: options) != nil }
    }

    func sorties(de t: Travail) -> [Sortie] {
        sorties.filter { $0.travail == t.id && ($0.archivee ?? false) == false }
    }

    func sortiesActives(pour m: Machine) -> [Sortie] {
        sorties.filter { m.vise($0) && $0.etat.active && ($0.archivee ?? false) == false }
    }

    /// Machines sur lesquelles ce travail peut partir : celles qui impriment, ou qui
    /// découpent pour un travail en découpe seule.
    func machines(pour t: Travail) -> [Machine] {
        machines.filter { t.decoupeSeule ? $0.decoupe : $0.imprime }
    }

    func nomMachine(_ id: String) -> String {
        machines.first { $0.id == id }?.nom ?? id
    }

    func peutSuspendre(_ t: Travail) -> Bool {
        sorties.contains { $0.travail == t.id && $0.etat == .enAttente }
    }

    func peutReprendre(_ t: Travail) -> Bool {
        sorties.contains { $0.travail == t.id && $0.etat == .suspendue }
    }

    func pageValide(_ t: Travail) -> Int {
        min(max(page, 1), max(t.nbPages, 1))
    }

    // MARK: Connexion

    /// Au lancement : reprend l'adresse retenue, ou demande une adresse.
    func demarrer() async {
        guard phase == .demarrage else { return }
        Notifier.demander()
        // Dernier choix : un site du cloud (jeton dans le trousseau), sinon le serveur local.
        if let site = UserDefaults.standard.string(forKey: Cles.siteCloud), !site.isEmpty, Trousseau.lire() != nil {
            await rouvrirCloud(siteId: site)
            return
        }
        let retenue = UserDefaults.standard.string(forKey: Cles.adresse) ?? ""
        if retenue.isEmpty {
            phase = .adresse
        } else {
            await connecter(retenue)
        }
    }

    /// Interroge `GET /api/session` ; puis le tableau de bord, ou l'écran d'identifiant.
    func connecter(_ saisie: String) async {
        guard let racine = Api.normaliser(saisie) else {
            erreurConnexion = Libelles.erreur("adresse_invalide", nil)
            phase = .adresse
            return
        }
        arreterBoucle()
        vider()
        api?.session.invalidateAndCancel()
        let nouvelle = Api(racine: racine)
        api = nouvelle
        siteCloud = nil
        adresse = Api.affichable(racine)
        erreurConnexion = nil
        occupe = true
        defer { occupe = false }
        do {
            let s = try await nouvelle.infoSession()
            UserDefaults.standard.set(adresse, forKey: Cles.adresse)
            UserDefaults.standard.removeObject(forKey: Cles.siteCloud)
            UserDefaults.standard.set("local", forKey: Cles.modeConnexion)
            session = s
            if s.accesLibre == true || s.utilisateur != nil {
                await entrer()
            } else {
                phase = .connexion
            }
        } catch {
            erreurConnexion = texte(de: error)
            phase = .adresse
        }
    }

    func ouvrirSession(nom: String, motDePasse: String) async {
        guard let api else { return }
        erreurConnexion = nil
        occupe = true
        defer { occupe = false }
        do {
            try await api.ouvrirSession(nom: nom, motDePasse: motDePasse)
            session = try await api.infoSession()
            await entrer()
        } catch {
            erreurConnexion = texte(de: error)
        }
    }

    func deconnecter() async {
        guard siteCloud == nil else { return }
        arreterBoucle()
        try? await api?.fermerSession()
        vider()
        session = nil
        erreurConnexion = nil
        phase = .connexion
    }

    /// Menu « Changer de serveur… » : retour à l'écran d'adresse.
    func changerServeur() {
        arreterBoucle()
        vider()
        if siteCloud != nil {
            siteCloud = nil
            adresse = ""
        }
        erreurConnexion = nil
        phase = .adresse
    }

    private func entrer() async {
        phase = .connecte
        await chargerReference()
        await rafraichir()
        lancerBoucle()
    }

    private func retourConnexion() {
        guard phase == .connecte else { return }
        arreterBoucle()
        vider()
        if siteCloud != nil {
            // Jeton révoqué ou expiré : on l'oublie, retour à l'écran de connexion (onglet cloud).
            oublierCloud()
            erreurConnexion = Libelles.erreur("jeton_invalide", nil)
            phase = .adresse
            return
        }
        erreurConnexion = "Session expirée : reconnectez-vous."
        phase = .connexion
    }

    private func vider() {
        travaux = []
        sorties = []
        machines = []
        reglages = []
        systeme = nil
        selection = nil
        page = 1
        serveurHorsLigne = nil
        statutsVus = [:]
        etatsVus = [:]
        suivis = []
        premierPassage = true
        cycle = 0
    }

    private func chargerReference() async {
        guard let api else { return }
        if let m = try? await api.machines() { machines = m }
        if let r = try? await api.reglages() { reglages = r }
    }

    // MARK: Cloud

    /// Vrai si un jeton est gardé dans le trousseau.
    var jetonCloudPresent: Bool { Trousseau.lire() != nil }

    /// Oublie le jeton (trousseau) et tout ce qui en dépend, sans appel réseau.
    private func oublierCloud() {
        Trousseau.oublier()
        cloudCompte = nil
        cloudSites = []
        siteCloud = nil
        api?.session.invalidateAndCancel()
        api = nil
        session = nil
        UserDefaults.standard.removeObject(forKey: Cles.siteCloud)
    }

    /// Relit `/api/moi`. 401 : le jeton est oublié.
    func actualiserCloud() async {
        guard let jeton = Trousseau.lire() else {
            cloudCompte = nil
            cloudSites = []
            return
        }
        cloudEnLecture = true
        defer { cloudEnLecture = false }
        do {
            let moi = try await ClientCloud().moi(jeton: jeton)
            cloudCompte = moi.compte
            cloudSites = moi.sites
        } catch let e as ErreurApi where e.statut == 401 {
            oublierCloud()
            erreurConnexion = Libelles.erreur("jeton_invalide", nil)
        } catch is CancellationError {
        } catch {
            erreurConnexion = texte(de: error)
        }
    }

    /// Bouton « Se connecter au cloud » : navigateur, autorisation, jeton au trousseau, sites.
    func connecterAuCloud() async {
        guard !occupe else { return }
        erreurConnexion = nil
        occupe = true
        defer { occupe = false }
        do {
            let jeton = try await autorisationCloud.autoriser()
            try Trousseau.ecrire(jeton)
            await actualiserCloud()
        } catch let e as ErreurApi where e.code == "cloud_annule" {
            // Fenêtre fermée par l'opérateur : rien à dire.
        } catch is CancellationError {
        } catch {
            erreurConnexion = texte(de: error)
        }
    }

    /// Au lancement : rouvre directement le dernier site s'il est en ligne ; sinon la liste.
    private func rouvrirCloud(siteId: String) async {
        await actualiserCloud()
        if let site = cloudSites.first(where: { $0.id == siteId }), site.enLigne {
            await ouvrirSite(site)
        } else {
            phase = .adresse
        }
    }

    /// Un clic sur un site : l'API vise son adresse, avec le jeton sur chaque requête.
    func ouvrirSite(_ site: SiteCloud) async {
        guard let jeton = Trousseau.lire() else {
            phase = .adresse
            return
        }
        guard site.enLigne else {
            erreurConnexion = Libelles.erreur("site_hors_ligne", nil)
            phase = .adresse
            return
        }
        guard let racine = Api.normaliser(site.adresse) else {
            erreurConnexion = Libelles.erreur("adresse_invalide", nil)
            phase = .adresse
            return
        }
        arreterBoucle()
        vider()
        api?.session.invalidateAndCancel()
        let nouvelle = Api(racine: racine, jeton: jeton)
        api = nouvelle
        erreurConnexion = nil
        occupe = true
        defer { occupe = false }
        do {
            let s = try await nouvelle.infoSession()
            // Via le cloud il n'y a pas de mot de passe du RIP : le site doit rendre l'utilisateur.
            guard s.utilisateur != nil else {
                throw ErreurApi(statut: 0, code: "cloud_session_inattendue", detail: nil)
            }
            session = s
            siteCloud = site
            adresse = site.nom
            UserDefaults.standard.set(site.id, forKey: Cles.siteCloud)
            UserDefaults.standard.set("cloud", forKey: Cles.modeConnexion)
            await entrer()
        } catch let e as ErreurApi where e.statut == 401 {
            oublierCloud()
            erreurConnexion = Libelles.erreur("jeton_invalide", nil)
            phase = .adresse
        } catch {
            api?.session.invalidateAndCancel()
            api = nil
            siteCloud = nil
            erreurConnexion = texte(de: error)
            phase = .adresse
        }
    }

    /// « Se déconnecter du cloud » : révoque le jeton côté plate-forme (au mieux), puis l'oublie.
    func deconnecterCloud() async {
        arreterBoucle()
        vider()
        let jeton = Trousseau.lire()
        oublierCloud()
        erreurConnexion = nil
        phase = .adresse
        if let jeton { try? await ClientCloud().deconnecter(jeton: jeton) }
    }

    // MARK: Actualisation

    private func lancerBoucle() {
        boucle?.cancel()
        boucle = Task { [weak self] in
            while !Task.isCancelled {
                guard let self else { return }
                await self.rafraichir()
                try? await Task.sleep(for: .seconds(3))
            }
        }
    }

    private func arreterBoucle() {
        boucle?.cancel()
        boucle = nil
    }

    /// Une actualisation : travaux et file à chaque tour, les machines de temps en temps
    /// (ou à l'écran Machines), le système seulement à l'écran Système.
    func rafraichir() async {
        guard let api, phase == .connecte else { return }
        cycle += 1
        do {
            async let t = api.travaux()
            async let f = api.file()
            let (nouveauxTravaux, nouvellesSorties) = try await (t, f)
            var nouvellesMachines: [Machine]?
            if rubrique == .machines || machines.isEmpty || cycle % 10 == 0 {
                nouvellesMachines = try await api.machines()
            }
            var nouveauSysteme: InfoSysteme?
            if rubrique == .systeme {
                nouveauSysteme = try await api.systeme()
            }
            // Déconnecté ou changé de serveur pendant l'attente : on jette la réponse.
            guard phase == .connecte else { return }
            appliquer(travaux: nouveauxTravaux, sorties: nouvellesSorties)
            if let nouvellesMachines { machines = nouvellesMachines }
            if let nouveauSysteme { systeme = nouveauSysteme }
            serveurHorsLigne = nil
        } catch {
            traiter(error, silencieux: true)
        }
    }

    private func appliquer(travaux nt: [Travail], sorties ns: [Sortie]) {
        notifier(travaux: nt, sorties: ns)
        travaux = nt
        sorties = ns
        if let s = selection, !nt.contains(where: { $0.id == s }) { selection = nil }
    }

    // MARK: Notifications

    /// Travail passé à `termine` ou `echec`, sortie terminée ou en échec : notification du
    /// système. Le premier passage ne fait que mémoriser l'état.
    private func notifier(travaux nt: [Travail], sorties ns: [Sortie]) {
        defer {
            statutsVus = Dictionary(nt.map { ($0.id, $0.statut) }, uniquingKeysWith: { a, _ in a })
            etatsVus = Dictionary(ns.map { ($0.id, $0.etat) }, uniquingKeysWith: { a, _ in a })
            premierPassage = false
        }
        guard !premierPassage else { return }

        for t in nt where t.statut.fini {
            let avant = statutsVus[t.id]
            let enCours = avant == .enAttente || avant == .enCours
            let envoiSuivi = avant == nil && suivis.contains(t.id)
            guard enCours || envoiSuivi else { continue }
            suivis.remove(t.id)
            if t.statut == .termine {
                Notifier.envoyer(titre: "Travail prêt", texte: t.nom)
            } else {
                Notifier.envoyer(titre: "Échec du rendu", texte: "\(t.nom) : \(t.texteErreur ?? "")")
            }
        }

        for s in ns {
            guard let avant = etatsVus[s.id], avant.active else { continue }
            let machine = nomMachine(s.machine)
            if s.etat == .terminee {
                Notifier.envoyer(titre: s.estDecoupe ? "Découpe terminée" : "Impression terminée", texte: "\(s.nom), page \(s.page) · \(machine)")
            } else if s.etat == .echec {
                Notifier.envoyer(titre: s.estDecoupe ? "Découpe en échec" : "Impression en échec", texte: "\(s.nom), page \(s.page) · \(machine) : \(Libelles.erreur(s.codeErreur ?? "sortie_echouee", s.erreur))")
            }
        }
    }

    // MARK: Erreurs et messages

    private func texte(de erreur: Error) -> String {
        (erreur as? LocalizedError)?.errorDescription ?? erreur.localizedDescription
    }

    /// 401 : retour à l'écran de connexion. Sinon l'erreur est dite à l'opérateur, ou, pendant
    /// l'actualisation (`silencieux`), seulement notée dans la barre d'état.
    func traiter(_ erreur: Error, silencieux: Bool = false) {
        if erreur is CancellationError { return }
        if let e = erreur as? ErreurApi, e.connexionRequise {
            retourConnexion()
            return
        }
        if silencieux {
            serveurHorsLigne = texte(de: erreur)
        } else {
            message = MessageUtilisateur(titre: "Action impossible", texte: texte(de: erreur))
        }
    }

    /// Phrase brève dans la barre d'état, effacée au bout de quelques secondes.
    func annoncer(_ texte: String) {
        annonce = texte
        annonceTache?.cancel()
        annonceTache = Task { [weak self] in
            try? await Task.sleep(for: .seconds(5))
            if !Task.isCancelled { self?.annonce = nil }
        }
    }

    // MARK: Actions sur les travaux

    /// Exécute un appel d'API, traite l'erreur, puis actualise. Rend vrai si tout a réussi.
    private func tenter(_ operation: (Api) async throws -> Void) async -> Bool {
        guard let api else { return false }
        do {
            try await operation(api)
            await rafraichir()
            return true
        } catch {
            traiter(error)
            return false
        }
    }

    func imprimer(_ t: Travail, page: Int, sur m: Machine) async {
        let operation = t.decoupeSeule ? "decoupe" : "impression"
        let ok = await tenter { try await $0.mettreEnFile(travail: t.id, page: page, machine: m.id, operation: operation) }
        if ok { annoncer("Page \(page) de « \(t.nom) » mise en file sur \(m.nom).") }
    }

    func imprimerToutes(_ t: Travail, sur m: Machine) async {
        let exclues = Set(t.pagesExclues ?? [])
        let pages = (1...max(t.nbPages, 1)).filter { !exclues.contains($0) }
        let operation = t.decoupeSeule ? "decoupe" : "impression"
        let ok = await tenter { api in
            for p in pages {
                try await api.mettreEnFile(travail: t.id, page: p, machine: m.id, operation: operation)
            }
        }
        if ok { annoncer("\(pages.count) page(s) de « \(t.nom) » mise(s) en file sur \(m.nom).") }
    }

    func relancer(_ t: Travail) async {
        let ok = await tenter { try await $0.relancer(travail: t.id) }
        if ok { annoncer("Rendu de « \(t.nom) » relancé.") }
    }

    func supprimer(_ t: Travail) async {
        let ok = await tenter { try await $0.supprimer(travail: t.id) }
        if ok {
            if selection == t.id { selection = nil }
            annoncer("« \(t.nom) » supprimé.")
        }
    }

    /// Bouton Suspendre de la barre d'outils : suspend les sorties en attente du travail,
    /// ou, s'il n'en reste pas, reprend les suspendues.
    func basculerSuspension(_ t: Travail) async {
        let suspendre = peutSuspendre(t)
        let cible: EtatSortie = suspendre ? .enAttente : .suspendue
        let action: ActionSortie = suspendre ? .suspendre : .reprendre
        let ids = sorties.filter { $0.travail == t.id && $0.etat == cible }.map(\.id)
        guard !ids.isEmpty else { return }
        _ = await tenter { api in
            for id in ids { try await api.agir(sortie: id, action: action) }
        }
    }

    func agir(_ s: Sortie, _ action: ActionSortie) async {
        _ = await tenter { try await $0.agir(sortie: s.id, action: action) }
    }

    /// Télécharge l'épreuve TIFF d'une page puis propose de l'enregistrer.
    func epreuveTiff(_ t: Travail, page: Int) async {
        guard let api else { return }
        annoncer("Préparation de l'épreuve TIFF…")
        let temporaire = FileManager.default.temporaryDirectory.appendingPathComponent("eko-epreuve-\(UUID().uuidString).tif")
        do {
            try await api.telechargerEpreuve(travail: t.id, page: page, vers: temporaire)
            annonce = nil
            let base = (t.nom as NSString).deletingPathExtension
                .components(separatedBy: CharacterSet(charactersIn: "/:\\")).joined(separator: "_")
            let panneau = NSSavePanel()
            panneau.allowedContentTypes = [.tiff]
            panneau.canCreateDirectories = true
            panneau.nameFieldStringValue = "\(base)-p\(page)-epreuve.tif"
            panneau.message = "Enregistrer l'épreuve TIFF"
            if panneau.runModal() == .OK, let destination = panneau.url {
                try? FileManager.default.removeItem(at: destination)
                try FileManager.default.moveItem(at: temporaire, to: destination)
            } else {
                try? FileManager.default.removeItem(at: temporaire)
            }
        } catch {
            try? FileManager.default.removeItem(at: temporaire)
            annonce = nil
            traiter(error)
        }
    }

    // MARK: Envoi

    /// Bouton « + » : choix de fichiers.
    func choisirFichiers() {
        let panneau = NSOpenPanel()
        panneau.allowsMultipleSelection = true
        panneau.canChooseDirectories = false
        panneau.allowedContentTypes = [.pdf, .svg, .tiff, .png, .jpeg]
        panneau.message = "Choisir les fichiers à envoyer au RIP"
        if panneau.runModal() == .OK { proposerEnvoi(panneau.urls) }
    }

    /// Glisser-déposer, Dock, panneau d'ouverture : ouvre la feuille d'envoi.
    func proposerEnvoi(_ urls: [URL]) {
        let retenus = urls.filter { $0.isFileURL && Self.extensionsAcceptees.contains($0.pathExtension.lowercased()) }
        guard !retenus.isEmpty else {
            if !urls.isEmpty {
                message = MessageUtilisateur(titre: "Envoi impossible", texte: Libelles.erreur("format_inconnu", nil))
            }
            return
        }
        fichiersAEnvoyer = retenus
    }

    /// Envoie les fichiers l'un après l'autre (jamais deux envois de plusieurs Go en même temps).
    func lancerEnvoi(fichiers: [URL], reglage: String?, client: String, commande: String, quantite: String) {
        guard let api else { return }
        var champs: [(String, String)] = []
        if let reglage, !reglage.isEmpty { champs.append(("reglage", reglage)) }
        for (nom, valeur) in [("client", client), ("commande", commande), ("quantite", quantite)] {
            let v = valeur.trimmingCharacters(in: .whitespacesAndNewlines)
            if !v.isEmpty { champs.append((nom, v)) }
        }
        let champsEnvoyes = champs
        let nouveaux = fichiers.map { EnvoiEnCours(nom: $0.lastPathComponent) }
        envois.append(contentsOf: nouveaux)

        let tache = Task { [weak self] in
            for (fichier, envoi) in zip(fichiers, nouveaux) {
                if Task.isCancelled {
                    envoi.erreur = "Annulé"
                    continue
                }
                do {
                    let travail = try await api.envoyer(fichier: fichier, champs: champsEnvoyes) { valeur in
                        Task { @MainActor in envoi.progression = valeur }
                    }
                    envoi.progression = 1
                    envoi.termine = true
                    self?.apresEnvoi(travail, envoi: envoi)
                } catch is CancellationError {
                    envoi.erreur = "Annulé"
                } catch {
                    envoi.erreur = (error as? LocalizedError)?.errorDescription ?? error.localizedDescription
                    if let e = error as? ErreurApi, e.connexionRequise { self?.traiter(e) }
                }
            }
        }
        for e in nouveaux { e.tache = tache }
    }

    private func apresEnvoi(_ t: Travail, envoi: EnvoiEnCours) {
        suivis.insert(t.id)
        rubrique = .travaux
        selection = t.id
        page = 1
        Task { [weak self] in
            await self?.rafraichir()
            try? await Task.sleep(for: .seconds(2))
            self?.envois.removeAll { $0.id == envoi.id }
        }
    }

    func annulerEnvois() {
        for e in envois where !e.termine && e.erreur == nil { e.tache?.cancel() }
    }

    func fermer(_ envoi: EnvoiEnCours) {
        envois.removeAll { $0.id == envoi.id }
    }
}

// MARK: - Notifications du système

enum Notifier {
    /// Hors d'un .app (`swift run`), UserNotifications n'est pas utilisable.
    static var disponible: Bool { Bundle.main.bundleURL.pathExtension == "app" }

    static func demander() {
        guard disponible else { return }
        UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound]) { _, _ in }
    }

    static func envoyer(titre: String, texte: String) {
        guard disponible else { return }
        let contenu = UNMutableNotificationContent()
        contenu.title = titre
        contenu.body = texte
        contenu.sound = .default
        let requete = UNNotificationRequest(identifier: UUID().uuidString, content: contenu, trigger: nil)
        UNUserNotificationCenter.current().add(requete)
    }
}
