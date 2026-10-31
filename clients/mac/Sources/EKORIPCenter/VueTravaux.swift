import SwiftUI

/// File des travaux : tableau triable, plus récents en haut.
@MainActor
struct VueTravaux: View {
    @Environment(Etat.self) private var etat
    @State private var tri = [KeyPathComparator(\Travail.creeLe, order: .reverse)]
    @State private var survol = false
    @State private var aSupprimer: Travail?

    private var lignes: [Travail] {
        etat.travauxFiltres.sorted(using: tri)
    }

    var body: some View {
        @Bindable var etat = etat
        Table(lignes, selection: $etat.selection, sortOrder: $tri) {
            TableColumn("Nom", value: \.nom) { t in
                Text(t.nom)
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .help(t.nom)
            }
            .width(min: 160, ideal: 260)

            TableColumn("Dossier", value: \.dossier) { t in
                Text(t.dossier)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }
            .width(min: 80, ideal: 120)

            TableColumn("Pages", value: \.nbPages) { t in
                Text("\(t.nbPages)")
                    .monospacedDigit()
                    .foregroundStyle(.secondary)
            }
            .width(min: 50, ideal: 60, max: 80)

            TableColumn("Statut", value: \.statut.rang) { t in
                PastilleStatut(travail: t)
            }
            .width(min: 90, ideal: 110, max: 160)

            TableColumn("Date", value: \.creeLe) { t in
                Text(t.date, format: .dateTime.day().month(.abbreviated).hour().minute())
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }
            .width(min: 110, ideal: 130)
        }
        .contextMenu(forSelectionType: String.self) { ids in
            if let id = ids.first, let t = etat.travaux.first(where: { $0.id == id }) {
                Button("Relancer le rendu") { Task { await etat.relancer(t) } }
                    .disabled(t.statut == .enCours || t.statut == .enAttente)
                Divider()
                Button("Supprimer…", role: .destructive) { aSupprimer = t }
            }
        }
        .overlay {
            if etat.travaux.isEmpty {
                ContentUnavailableView {
                    Label("Aucun travail", systemImage: "tray")
                } description: {
                    Text("Glissez un fichier ici ou utilisez le bouton « + » pour en envoyer un au RIP.")
                }
            } else if lignes.isEmpty {
                ContentUnavailableView.search(text: etat.recherche)
            }
        }
        .overlay {
            if survol {
                RoundedRectangle(cornerRadius: 10)
                    .strokeBorder(Color.accentColor, lineWidth: 3)
                    .padding(4)
                    .allowsHitTesting(false)
            }
        }
        .dropDestination(for: URL.self) { urls, _ in
            etat.proposerEnvoi(urls)
            return !urls.isEmpty
        } isTargeted: { cible in
            survol = cible
        }
        .confirmationDialog(
            "Supprimer « \(aSupprimer?.nom ?? "") » ?",
            isPresented: Binding(get: { aSupprimer != nil }, set: { if !$0 { aSupprimer = nil } }),
            titleVisibility: .visible,
            presenting: aSupprimer
        ) { t in
            Button("Supprimer", role: .destructive) { Task { await etat.supprimer(t) } }
            Button("Annuler", role: .cancel) {}
        } message: { _ in
            Text("Le travail et ses fichiers seront effacés du serveur.")
        }
    }
}

/// Point de couleur et libellé du statut ; l'échec dit sa cause au survol.
@MainActor
struct PastilleStatut: View {
    let travail: Travail

    private var couleur: Color {
        switch travail.statut {
        case .enAttente: return .secondary
        case .enCours: return .blue
        case .termine: return .green
        case .echec: return .red
        case .inconnu: return .secondary
        }
    }

    var body: some View {
        HStack(spacing: 6) {
            if travail.statut == .enCours {
                ProgressView().controlSize(.mini)
            } else {
                Circle().fill(couleur).frame(width: 8, height: 8)
            }
            Text(travail.statut.libelle)
        }
        .help(travail.texteErreur ?? "")
    }
}
