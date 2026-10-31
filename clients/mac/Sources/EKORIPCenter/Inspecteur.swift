import SwiftUI
import AppKit

extension Color {
    /// Fond d'aperçu : gris neutre et opaque, clair ou sombre selon l'apparence, jamais de verre.
    static let fondApercu = Color(nsColor: NSColor(name: nil) { apparence in
        apparence.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua
            ? NSColor(white: 0.22, alpha: 1)
            : NSColor(white: 0.84, alpha: 1)
    })
}

/// Inspecteur (colonne de droite) du travail choisi dans le tableau.
@MainActor
struct InspecteurVue: View {
    @Environment(Etat.self) private var etat
    @State private var onglet = Onglet.apercu
    @State private var suppression = false

    private enum Onglet: String, CaseIterable, Identifiable {
        case apercu = "Aperçu"
        case fiche = "Fiche"
        case sorties = "Sorties"
        var id: String { rawValue }
    }

    var body: some View {
        Group {
            if let t = etat.travailSelectionne {
                contenu(t)
            } else {
                ContentUnavailableView {
                    Label("Aucun travail choisi", systemImage: "sidebar.right")
                } description: {
                    Text("Choisissez un travail dans la file pour voir son aperçu, sa fiche et ses sorties.")
                }
            }
        }
        .inspectorColumnWidth(min: 300, ideal: 360, max: 560)
    }

    @ViewBuilder
    private func contenu(_ t: Travail) -> some View {
        VStack(spacing: 0) {
            VStack(alignment: .leading, spacing: 10) {
                Text(t.nom)
                    .font(.headline)
                    .lineLimit(2)
                    .truncationMode(.middle)
                    .textSelection(.enabled)
                HStack(spacing: 8) {
                    PastilleStatut(travail: t)
                    if !t.dossier.isEmpty {
                        Text("· \(t.dossier)").foregroundStyle(.secondary)
                    }
                }
                .font(.subheadline)
                Picker("Onglet", selection: $onglet) {
                    ForEach(Onglet.allCases) { Text($0.rawValue).tag($0) }
                }
                .pickerStyle(.segmented)
                .labelsHidden()
            }
            .padding(14)

            Divider()

            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    switch onglet {
                    case .apercu: PanneauApercu(travail: t)
                    case .fiche: PanneauFiche(travail: t)
                    case .sorties: PanneauSorties(travail: t)
                    }
                }
                .padding(14)
                .frame(maxWidth: .infinity, alignment: .leading)
            }

            Divider()
            actions(t)
        }
        .confirmationDialog(
            "Supprimer « \(t.nom) » ?",
            isPresented: $suppression,
            titleVisibility: .visible
        ) {
            Button("Supprimer", role: .destructive) { Task { await etat.supprimer(t) } }
            Button("Annuler", role: .cancel) {}
        } message: {
            Text("Le travail et ses fichiers seront effacés du serveur.")
        }
    }

    /// Imprimer, Relancer, Épreuve TIFF, Supprimer.
    private func actions(_ t: Travail) -> some View {
        let occupe = t.statut == .enCours || t.statut == .enAttente
        let machines = etat.machines(pour: t)
        let page = etat.pageValide(t)
        return VStack(spacing: 8) {
            HStack(spacing: 8) {
                Menu {
                    ForEach(machines) { m in
                        if t.nbPages > 1 {
                            Menu(m.nom) {
                                Button("Page \(page)") { Task { await etat.imprimer(t, page: page, sur: m) } }
                                Button("Toutes les pages (\(t.nbPages))") { Task { await etat.imprimerToutes(t, sur: m) } }
                            }
                        } else {
                            Button(m.nom) { Task { await etat.imprimer(t, page: 1, sur: m) } }
                        }
                    }
                } label: {
                    Label(t.decoupeSeule ? "Découper" : "Imprimer", systemImage: "printer")
                        .frame(maxWidth: .infinity)
                }
                .menuStyle(.button)
                .disabled(t.statut != .termine || machines.isEmpty)
                .help(machines.isEmpty ? "Aucune machine ne fait cette opération." : "Mettre la page en file sur une machine")

                Button {
                    Task { await etat.epreuveTiff(t, page: page) }
                } label: {
                    Label("Épreuve TIFF", systemImage: "square.and.arrow.down")
                        .frame(maxWidth: .infinity)
                }
                .disabled(t.statut != .termine)
            }
            HStack(spacing: 8) {
                Button {
                    Task { await etat.relancer(t) }
                } label: {
                    Label("Relancer", systemImage: "arrow.clockwise")
                        .frame(maxWidth: .infinity)
                }
                .disabled(occupe)

                Button(role: .destructive) {
                    suppression = true
                } label: {
                    Label("Supprimer", systemImage: "trash")
                        .frame(maxWidth: .infinity)
                }
                .disabled(occupe)
            }
        }
        .controlSize(.large)
        .padding(14)
    }
}

// MARK: - Aperçu, format, plaques, alertes

@MainActor
private struct PanneauApercu: View {
    @Environment(Etat.self) private var etat
    let travail: Travail

    var body: some View {
        let page = etat.pageValide(travail)
        let infos = infosPage(page)

        VStack(alignment: .leading, spacing: 14) {
            ZStack {
                Color.fondApercu
                ApercuImage(travail: travail, page: page)
                    .padding(8)
            }
            .frame(height: 300)
            .clipShape(RoundedRectangle(cornerRadius: 8))

            if travail.nbPages > 1 {
                HStack {
                    Button {
                        etat.page = max(page - 1, 1)
                    } label: {
                        Image(systemName: "chevron.left")
                    }
                    .disabled(page <= 1)
                    Spacer()
                    Text("Page \(page) sur \(travail.nbPages)")
                        .monospacedDigit()
                    Spacer()
                    Button {
                        etat.page = min(page + 1, travail.nbPages)
                    } label: {
                        Image(systemName: "chevron.right")
                    }
                    .disabled(page >= travail.nbPages)
                }
            }

            if let texte = travail.texteErreur {
                Label(texte, systemImage: "exclamationmark.triangle.fill")
                    .foregroundStyle(.red)
                    .fixedSize(horizontal: false, vertical: true)
            }

            VStack(alignment: .leading, spacing: 6) {
                if let format = infos?.format {
                    LabeledContent("Format", value: format)
                }
                if let plaques = infos?.plaques, !plaques.isEmpty {
                    LabeledContent("Plaques", value: plaques.joined(separator: ", "))
                }
                if let duree = travail.dureeImpressionMin, duree > 0 {
                    LabeledContent("Durée estimée", value: dureeTexte(duree))
                }
            }

            if let alertes = travail.alertes, !alertes.isEmpty {
                VStack(alignment: .leading, spacing: 6) {
                    Text("Alertes").font(.subheadline.weight(.semibold))
                    ForEach(Array(alertes.enumerated()), id: \.offset) { item in
                        Label(Libelles.alerte(item.element), systemImage: "exclamationmark.triangle")
                            .foregroundStyle(.orange)
                            .font(.callout)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                }
            }
        }
    }

    private func infosPage(_ page: Int) -> Page? {
        guard let pages = travail.pages, pages.indices.contains(page - 1) else { return nil }
        return pages[page - 1]
    }

    private func dureeTexte(_ minutes: Double) -> String {
        let total = Int(minutes.rounded())
        if total < 1 { return "moins d'une minute" }
        if total < 60 { return "\(total) min" }
        return "\(total / 60) h \(String(format: "%02d", total % 60))"
    }
}

/// Aperçu PNG de la page. Chargé par la session de l'application (et non par `AsyncImage`,
/// qui utiliserait `URLSession.shared` et n'enverrait pas le cookie de connexion).
@MainActor
private struct ApercuImage: View {
    @Environment(Etat.self) private var etat
    let travail: Travail
    let page: Int
    @State private var image: NSImage?
    @State private var echec: String?

    /// Recharge quand le travail, la page ou le statut change (un rendu relancé).
    private var cle: String { "\(travail.id)#\(page)#\(travail.statut.rawValue)" }

    var body: some View {
        Group {
            if let image {
                Image(nsImage: image)
                    .resizable()
                    .interpolation(.medium)
                    .scaledToFit()
            } else if let echec {
                Label(echec, systemImage: "photo")
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
            } else if travail.statut == .enAttente || travail.statut == .enCours {
                VStack(spacing: 8) {
                    ProgressView()
                    Text("Rendu en cours…").foregroundStyle(.secondary)
                }
            } else if travail.statut == .echec {
                Label("Pas d'aperçu", systemImage: "photo")
                    .foregroundStyle(.secondary)
            } else {
                ProgressView()
            }
        }
        .task(id: cle) { await charger() }
    }

    private func charger() async {
        image = nil
        echec = nil
        guard travail.statut == .termine, let api = etat.api else { return }
        do {
            let donnees = try await api.apercu(travail: travail.id, page: page)
            if let i = NSImage(data: donnees) {
                image = i
            } else {
                echec = "Aperçu illisible"
            }
        } catch {
            if error is CancellationError { return }
            if let e = error as? ErreurApi, e.connexionRequise {
                etat.traiter(e)
                return
            }
            echec = (error as? LocalizedError)?.errorDescription ?? error.localizedDescription
        }
    }
}

// MARK: - Fiche

@MainActor
private struct PanneauFiche: View {
    let travail: Travail

    var body: some View {
        let f = travail.fiche
        VStack(alignment: .leading, spacing: 8) {
            if let erp = travail.erp?.refErp, !erp.isEmpty {
                LabeledContent("Dossier ERP", value: erp)
            }
            if let f, !f.vide {
                ligne("Client", f.client)
                ligne("Société", f.societe)
                ligne("Commande", f.commande)
                if let q = f.quantite, q > 0 {
                    LabeledContent("Quantité", value: "\(q)")
                }
                if let e = f.echeance {
                    LabeledContent("Échéance", value: Date(timeIntervalSince1970: TimeInterval(e)).formatted(date: .abbreviated, time: .shortened))
                }
                ligne("Opérateur", f.operateur)
                if let notes = f.notes, !notes.isEmpty {
                    Text("Notes").font(.subheadline.weight(.semibold)).padding(.top, 6)
                    Text(notes).textSelection(.enabled).fixedSize(horizontal: false, vertical: true)
                }
            } else {
                Text("Aucune fiche renseignée.")
                    .foregroundStyle(.secondary)
            }
            Divider().padding(.vertical, 4)
            LabeledContent("Reçu le", value: travail.date.formatted(date: .abbreviated, time: .shortened))
            if let mode = travail.mode {
                LabeledContent("Mode", value: mode == "decouper" ? "Découpe seule" : "Impression")
            }
            if let format = travail.format {
                LabeledContent("Fichier", value: format.uppercased())
            }
        }
    }

    @ViewBuilder
    private func ligne(_ titre: String, _ valeur: String?) -> some View {
        if let valeur, !valeur.isEmpty {
            LabeledContent(titre, value: valeur)
        }
    }
}

// MARK: - Sorties du travail

@MainActor
private struct PanneauSorties: View {
    @Environment(Etat.self) private var etat
    let travail: Travail

    var body: some View {
        let liste = etat.sorties(de: travail)
        VStack(alignment: .leading, spacing: 10) {
            if liste.isEmpty {
                Text("Ce travail n'a encore aucune sortie dans la file.")
                    .foregroundStyle(.secondary)
            } else {
                ForEach(liste) { s in
                    LigneSortie(sortie: s, avecMachine: true)
                    if s.id != liste.last?.id { Divider() }
                }
            }
        }
    }
}
