import Foundation

// Formes JSON du serveur (serveur/src/main.rs, file.rs, machines.rs).
// Décodage avec .convertFromSnakeCase (`cree_le` devient `creeLe`). Tout ce que le serveur
// peut omettre (`#[serde(default)]`, `Option`) est facultatif ici, et les champs inconnus
// sont ignorés : le serveur en ajoute à chaque version.

extension JSONDecoder {
    /// Décodeur pour les réponses de l'API.
    static var serveur: JSONDecoder {
        let d = JSONDecoder()
        d.keyDecodingStrategy = .convertFromSnakeCase
        return d
    }
}

private let formateurMm: NumberFormatter = {
    let f = NumberFormatter()
    f.minimumFractionDigits = 0
    f.maximumFractionDigits = 1
    return f
}()

private func nonVide(_ s: String?) -> String? {
    guard let s, !s.isEmpty else { return nil }
    return s
}

// MARK: - Travaux

enum StatutTravail: String, Decodable, Sendable {
    case enAttente = "en_attente"
    case enCours = "en_cours"
    case termine
    case echec
    /// Valeur ajoutée par un serveur plus récent.
    case inconnu

    init(from decoder: Decoder) throws {
        let brut = try decoder.singleValueContainer().decode(String.self)
        self = StatutTravail(rawValue: brut) ?? .inconnu
    }

    var libelle: String {
        switch self {
        case .enAttente: return "En attente"
        case .enCours: return "En cours"
        case .termine: return "Prêt"
        case .echec: return "Échec"
        case .inconnu: return "Inconnu"
        }
    }

    /// Rang pour le tri de la colonne Statut : ce qui travaille d'abord.
    var rang: Int {
        switch self {
        case .enCours: return 0
        case .enAttente: return 1
        case .echec: return 2
        case .termine: return 3
        case .inconnu: return 4
        }
    }

    /// Le rendu est fini (réussi ou non).
    var fini: Bool { self == .termine || self == .echec }
}

struct Travail: Identifiable, Decodable, Sendable {
    let id: String
    let nom: String
    let statut: StatutTravail
    let codeErreur: String?
    let erreur: String?
    let pages: [Page]?
    let alertes: [Alerte]?
    /// Secondes Unix.
    let creeLe: Int
    let fiche: Fiche?
    let erp: LienErp?
    /// `imprimer` ou `decouper`.
    let mode: String?
    let format: String?
    let pagesExclues: [Int]?
    let dureeImpressionMin: Double?
    let prixDeRevient: Double?

    var nbPages: Int { pages?.count ?? 0 }

    /// Dossier ERP s'il existe, sinon numéro de commande de la fiche.
    var dossier: String { nonVide(erp?.refErp) ?? nonVide(fiche?.commande) ?? "" }

    var date: Date { Date(timeIntervalSince1970: TimeInterval(creeLe)) }

    var decoupeSeule: Bool { mode == "decouper" }

    /// Libellé de l'échec, dans la langue de l'écran.
    var texteErreur: String? {
        guard statut == .echec else { return nil }
        return Libelles.erreur(codeErreur ?? "rendu_echoue", erreur)
    }
}

struct Page: Decodable, Sendable {
    let plaques: [String]?
    let plaquesDecoupe: [String]?
    let largeurMm: Double?
    let hauteurMm: Double?
    let surfaceM2: Double?

    /// « 594 × 841 mm », ou nil si le serveur ne connaît pas le format (travaux anciens).
    var format: String? {
        guard let l = largeurMm, let h = hauteurMm, l > 0, h > 0 else { return nil }
        let texte = { (v: Double) in formateurMm.string(from: NSNumber(value: v)) ?? "\(v)" }
        return "\(texte(l)) × \(texte(h)) mm"
    }
}

struct Alerte: Decodable, Sendable {
    let code: String
    let page: Int?
    let detail: String?
}

/// Fiche travail : champs vides quand ils ne sont pas renseignés.
struct Fiche: Decodable, Sendable {
    let commande: String?
    let client: String?
    let societe: String?
    let quantite: Int?
    /// Secondes Unix.
    let echeance: Int?
    let notes: String?
    let operateur: String?

    var vide: Bool {
        nonVide(commande) == nil && nonVide(client) == nil && nonVide(societe) == nil
            && (quantite ?? 0) == 0 && echeance == nil && nonVide(notes) == nil && nonVide(operateur) == nil
    }
}

struct LienErp: Decodable, Sendable {
    let envoi: String?
    let refErp: String?
}

// MARK: - File d'attente

enum EtatSortie: String, Decodable, Sendable {
    case enAttente = "en_attente"
    case suspendue
    case enCours = "en_cours"
    case terminee
    case echec
    case annulee
    case imbriquee
    case inconnu

    init(from decoder: Decoder) throws {
        let brut = try decoder.singleValueContainer().decode(String.self)
        self = EtatSortie(rawValue: brut) ?? .inconnu
    }

    var libelle: String {
        switch self {
        case .enAttente: return "En attente"
        case .suspendue: return "Suspendue"
        case .enCours: return "En cours"
        case .terminee: return "Terminée"
        case .echec: return "Échec"
        case .annulee: return "Annulée"
        case .imbriquee: return "Imbriquée"
        case .inconnu: return "Inconnu"
        }
    }

    /// Encore à passer : le travail ne peut ni être supprimé ni relancé.
    var active: Bool { self == .enAttente || self == .suspendue || self == .enCours }
}

/// Actions de `POST /api/file/{id}/{action}`, plus le retrait (`DELETE /api/file/{id}`).
enum ActionSortie: String, Identifiable, Sendable {
    case enTete = "en_tete"
    case monter
    case descendre
    case suspendre
    case reprendre
    case annuler
    case refaire
    case retirer

    var id: String { rawValue }

    var libelle: String {
        switch self {
        case .enTete: return "Passer en tête"
        case .monter: return "Monter"
        case .descendre: return "Descendre"
        case .suspendre: return "Suspendre"
        case .reprendre: return "Reprendre"
        case .annuler: return "Annuler"
        case .refaire: return "Refaire"
        case .retirer: return "Retirer de la file"
        }
    }

    var destructive: Bool { self == .annuler || self == .retirer }
}

struct Sortie: Identifiable, Decodable, Sendable {
    let id: String
    let machine: String
    let operation: String?
    let travail: String
    let nom: String
    let page: Int
    let etat: EtatSortie
    let codeErreur: String?
    let erreur: String?
    let creeLe: Int?
    let debut: Int?
    let fin: Int?
    let archivee: Bool?
    let planche: String?
    let plancheNom: String?
    /// Code de `Libelles.raisons` : pourquoi la sortie attend.
    let raisonAttente: String?
    let dureeMin: Double?
    let poids: Int?

    var estDecoupe: Bool { operation == "decoupe" }

    /// Mêmes règles que `actionsPossibles` de l'interface web.
    var actions: [ActionSortie] {
        switch etat {
        case .enAttente: return [.enTete, .monter, .descendre, .suspendre, .annuler]
        case .suspendue: return [.enTete, .monter, .descendre, .reprendre, .annuler]
        case .terminee, .echec, .annulee: return [.refaire, .retirer]
        case .imbriquee: return [.retirer]
        case .enCours, .inconnu: return []
        }
    }
}

// MARK: - Machines

struct Machine: Identifiable, Decodable, Sendable {
    let id: String
    let nom: String
    /// `epson_sc_s80600`, `mutoh_kona`, `graphtec_fc7000`, `roland_versacamm`.
    let modele: String?
    let laizeMm: Double?
    let pool: String?
    let varianteDe: String?

    /// Les opérations se déduisent du modèle (comme `Modele::imprime` / `decoupe` du serveur).
    var imprime: Bool { modele == "epson_sc_s80600" || modele == "roland_versacamm" }
    var decoupe: Bool { modele == "mutoh_kona" || modele == "graphtec_fc7000" || modele == "roland_versacamm" }

    var operations: [String] {
        var o: [String] = []
        if imprime { o.append("Impression") }
        if decoupe { o.append("Découpe") }
        return o
    }

    /// Une sortie de la file vise cette machine, directement ou par son pool.
    func vise(_ s: Sortie) -> Bool { s.machine == id || (pool != nil && s.machine == pool) }
}

struct Reglage: Identifiable, Decodable, Sendable {
    let nom: String
    let uuid: String?
    var id: String { nom }
}

// MARK: - Session et système

struct InfoSession: Decodable, Sendable {
    let accesLibre: Bool?
    let utilisateur: String?
    let role: String?
}

struct InfoSysteme: Decodable, Sendable {
    let version: String?
    /// Octets des plaques en attente dans la file.
    let tampon: Int?
    let rendus: Rendus?
    let disqueTotal: Int?
    let disqueLibre: Int?
    let conservationJours: Int?
    let seuilLibre: Int?
    let alertes: [AlerteDossier]?

    struct Rendus: Decodable, Sendable {
        let simultanes: Int?
        let enCours: Int?
        let enAttente: Int?
    }
}

/// Fichier d'état illisible (`dossier` faux) ou dossier absent (`dossier` vrai) ; `reprise` :
/// la version précédente a été reprise, ou le dossier recréé.
struct AlerteDossier: Decodable, Sendable, Identifiable {
    let fichier: String
    let reprise: Bool?
    let dossier: Bool?
    var id: String { fichier }
}

/// Corps des erreurs du serveur : `{"code": "...", "detail": "..."|null}`.
struct CorpsErreur: Decodable {
    let code: String?
    let detail: String?
}
