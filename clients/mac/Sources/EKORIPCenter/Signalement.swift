import SwiftUI

/// Corps de `POST /api/rapports`. Les clés JSON sont celles du serveur.
struct CorpsRapport: Encodable, Sendable, Equatable {
    struct Client: Encodable, Sendable, Equatable {
        let type: String
        let version: String
        let os: String
    }

    static let limiteTitre = 120
    static let limiteDescription = 5000
    static let limiteEmail = 120

    let titre: String
    let description: String
    /// Absent du JSON quand vide : le serveur le tient pour facultatif.
    let email: String?
    let joindreJournal: Bool
    let client: Client

    enum CodingKeys: String, CodingKey {
        case titre, description, email, client
        case joindreJournal = "joindre_journal"
    }

    /// Rapport prêt à partir (textes rognés), ou nil si le titre ou la description sont vides
    /// ou trop longs.
    static func fabriquer(titre: String, description: String, email: String, joindreJournal: Bool,
                          type: String = "client-mac", version: String = VersionClient.numero,
                          os: String = Telemetrie.osCourant()) -> CorpsRapport? {
        let t = titre.trimmingCharacters(in: .whitespacesAndNewlines)
        let d = description.trimmingCharacters(in: .whitespacesAndNewlines)
        let e = email.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !t.isEmpty, t.count <= limiteTitre,
              !d.isEmpty, d.count <= limiteDescription,
              e.count <= limiteEmail else { return nil }
        return CorpsRapport(titre: t, description: d, email: e.isEmpty ? nil : e,
                            joindreJournal: joindreJournal,
                            client: Client(type: type, version: version, os: os))
    }
}

/// Réponse 201 du serveur.
struct ReponseRapport: Decodable, Sendable {
    let reference: String
}

/// Feuille « Signaler un problème… ».
@MainActor
struct FeuilleSignalement: View {
    @Environment(Etat.self) private var etat
    @Environment(\.dismiss) private var fermer
    @State private var titre = ""
    @State private var description = ""
    @State private var email = ""
    @State private var joindreJournal = true
    @State private var envoiEnCours = false
    @State private var erreur: String?

    private var rapport: CorpsRapport? {
        CorpsRapport.fabriquer(titre: titre, description: description, email: email, joindreJournal: joindreJournal)
    }

    var body: some View {
        VStack(spacing: 0) {
            Form {
                Section {
                    TextField("Titre", text: $titre)
                    VStack(alignment: .leading, spacing: 4) {
                        Text("Description")
                        TextEditor(text: $description)
                            .font(.body)
                            .frame(minHeight: 140)
                            .overlay(RoundedRectangle(cornerRadius: 4).stroke(.separator))
                        Text("Le titre et la description seront publiés sur GitHub (dépôt public) : n'y mettez pas d'information confidentielle. Le journal et l'e-mail restent privés.")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                    TextField("E-mail (facultatif, pour être recontacté)", text: $email)
                    Toggle("Joindre le journal du serveur (noms de fichiers masqués)", isOn: $joindreJournal)
                }
                if let erreur {
                    Text(erreur)
                        .foregroundStyle(.red)
                        .font(.callout)
                }
            }
            .formStyle(.grouped)
            .disabled(envoiEnCours)

            Divider()
            HStack {
                if envoiEnCours { ProgressView().controlSize(.small) }
                Spacer()
                Button("Annuler", role: .cancel) { fermer() }
                    .keyboardShortcut(.cancelAction)
                Button("Envoyer") { Task { await envoyer() } }
                    .keyboardShortcut(.defaultAction)
                    .buttonStyle(.borderedProminent)
                    .disabled(rapport == nil || envoiEnCours)
            }
            .padding(14)
        }
        .frame(width: 520, height: 500)
    }

    private func envoyer() async {
        guard let rapport, let api = etat.api else { return }
        envoiEnCours = true
        erreur = nil
        defer { envoiEnCours = false }
        do {
            let reference = try await api.signaler(rapport)
            etat.signalementOuvert = false
            etat.message = MessageUtilisateur(titre: "Rapport envoyé", texte: "Merci, rapport envoyé (référence \(reference)).")
        } catch is CancellationError {
            return
        } catch let e as ErreurApi where e.connexionRequise {
            etat.signalementOuvert = false
            etat.traiter(e)
        } catch {
            let texte = (error as? LocalizedError)?.errorDescription ?? error.localizedDescription
            erreur = "Envoi impossible : " + texte
        }
    }
}
