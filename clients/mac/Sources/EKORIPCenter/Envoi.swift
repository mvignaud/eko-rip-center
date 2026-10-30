import SwiftUI

/// Feuille d'envoi : réglage à appliquer, et fiche facultative (client, commande, quantité).
/// Les fichiers partent ensuite l'un après l'autre (voir `Etat.lancerEnvoi`).
@MainActor
struct FeuilleEnvoi: View {
    @Environment(Etat.self) private var etat
    @Environment(\.dismiss) private var fermer
    @AppStorage("dernierReglage") private var reglage = ""
    @State private var client = ""
    @State private var commande = ""
    @State private var quantite = ""
    let fichiers: [URL]

    private var quantiteValide: Bool {
        let q = quantite.trimmingCharacters(in: .whitespaces)
        return q.isEmpty || (Int(q).map { $0 >= 0 && $0 <= 100_000 } ?? false)
    }

    var body: some View {
        VStack(spacing: 0) {
            Form {
                Section(fichiers.count > 1 ? "Fichiers (\(fichiers.count))" : "Fichier") {
                    ForEach(fichiers, id: \.self) { url in
                        Label {
                            Text(url.lastPathComponent)
                                .lineLimit(1)
                                .truncationMode(.middle)
                        } icon: {
                            Image(systemName: "doc")
                        }
                        .help(url.path)
                    }
                }

                Section("Réglage") {
                    if etat.reglages.isEmpty {
                        Text("Aucun réglage enregistré : le serveur applique ses réglages par défaut.")
                            .foregroundStyle(.secondary)
                    } else {
                        Picker("Réglage", selection: $reglage) {
                            ForEach(etat.reglages) { r in
                                Text(r.nom).tag(r.nom)
                            }
                        }
                    }
                }

                Section("Fiche travail (facultatif)") {
                    TextField("Client", text: $client)
                    TextField("Commande", text: $commande)
                    TextField("Quantité", text: $quantite)
                    if !quantiteValide {
                        Text("La quantité est un nombre entier, 100 000 au plus.")
                            .font(.caption)
                            .foregroundStyle(.red)
                    }
                }
            }
            .formStyle(.grouped)

            Divider()
            HStack {
                Spacer()
                Button("Annuler", role: .cancel) { fermer() }
                    .keyboardShortcut(.cancelAction)
                Button("Envoyer") { envoyer() }
                    .keyboardShortcut(.defaultAction)
                    .buttonStyle(.borderedProminent)
                    .disabled(!quantiteValide)
            }
            .padding(14)
        }
        .frame(width: 460, height: 520)
        .onAppear {
            if !etat.reglages.contains(where: { $0.nom == reglage }) {
                reglage = etat.reglages.first?.nom ?? ""
            }
        }
    }

    private func envoyer() {
        let choisi = etat.reglages.contains(where: { $0.nom == reglage }) ? reglage : nil
        etat.lancerEnvoi(fichiers: fichiers, reglage: choisi, client: client, commande: commande, quantite: quantite)
        fermer()
    }
}

/// Barre du bas : envois en cours, annonce brève, serveur injoignable. Matériau de barre.
@MainActor
struct BarreEtat: View {
    @Environment(Etat.self) private var etat

    var body: some View {
        if !etat.envois.isEmpty || etat.annonce != nil || etat.serveurHorsLigne != nil {
            VStack(alignment: .leading, spacing: 6) {
                if let erreur = etat.serveurHorsLigne {
                    Label(erreur, systemImage: "wifi.exclamationmark")
                        .foregroundStyle(.orange)
                        .lineLimit(2)
                }
                ForEach(etat.envois) { envoi in
                    LigneEnvoi(envoi: envoi)
                }
                if let annonce = etat.annonce {
                    Text(annonce).foregroundStyle(.secondary)
                }
            }
            .font(.callout)
            .padding(.horizontal, 14)
            .padding(.vertical, 8)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(.bar)
            .overlay(alignment: .top) { Divider() }
        }
    }
}

@MainActor
private struct LigneEnvoi: View {
    @Environment(Etat.self) private var etat
    let envoi: EnvoiEnCours

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: envoi.erreur != nil ? "exclamationmark.triangle.fill" : (envoi.termine ? "checkmark.circle.fill" : "arrow.up.circle"))
                .foregroundStyle(envoi.erreur != nil ? Color.red : (envoi.termine ? Color.green : Color.accentColor))
            Text(envoi.nom)
                .lineLimit(1)
                .truncationMode(.middle)
                .frame(maxWidth: 220, alignment: .leading)
            if let erreur = envoi.erreur {
                Text(erreur)
                    .foregroundStyle(.red)
                    .lineLimit(2)
                Spacer()
                Button("Fermer") { etat.fermer(envoi) }
                    .buttonStyle(.link)
            } else if envoi.termine {
                Text("Envoyé").foregroundStyle(.secondary)
                Spacer()
            } else {
                ProgressView(value: envoi.progression)
                    .frame(maxWidth: .infinity)
                Text(envoi.progression, format: .percent.precision(.fractionLength(0)))
                    .monospacedDigit()
                    .foregroundStyle(.secondary)
                    .frame(width: 44, alignment: .trailing)
                Button {
                    etat.annulerEnvois()
                } label: {
                    Image(systemName: "xmark.circle.fill")
                }
                .buttonStyle(.borderless)
                .help("Annuler l'envoi")
            }
        }
    }
}
