import SwiftUI

/// Cartes des machines : modèle, opérations et sorties actives de chacune.
@MainActor
struct VueMachines: View {
    @Environment(Etat.self) private var etat

    var body: some View {
        ScrollView {
            LazyVGrid(columns: [GridItem(.adaptive(minimum: 360), spacing: 16, alignment: .top)], alignment: .leading, spacing: 16) {
                ForEach(etat.machines) { machine in
                    CarteMachine(machine: machine)
                }
            }
            .padding(20)
        }
        .background(Color(nsColor: .windowBackgroundColor))
        .overlay {
            if etat.machines.isEmpty {
                ContentUnavailableView {
                    Label("Aucune machine", systemImage: "printer")
                } description: {
                    Text("Le catalogue du serveur est vide.")
                }
            }
        }
    }
}

@MainActor
private struct CarteMachine: View {
    @Environment(Etat.self) private var etat
    let machine: Machine

    private var sorties: [Sortie] {
        // Celle qui tourne d'abord, puis la file dans son ordre.
        let actives = etat.sortiesActives(pour: machine)
        return actives.filter { $0.etat == .enCours } + actives.filter { $0.etat != .enCours }
    }

    private var symbole: String {
        machine.imprime ? "printer" : "scissors"
    }

    var body: some View {
        let liste = sorties
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .top, spacing: 12) {
                Image(systemName: symbole)
                    .font(.title2)
                    .foregroundStyle(.tint)
                    .frame(width: 28)
                VStack(alignment: .leading, spacing: 2) {
                    Text(machine.nom).font(.headline)
                    Text(Libelles.modele(machine.modele))
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                }
                Spacer()
                if liste.contains(where: { $0.etat == .enCours }) {
                    Label("Au travail", systemImage: "circle.fill")
                        .labelStyle(.titleAndIcon)
                        .font(.caption)
                        .foregroundStyle(.green)
                } else {
                    Text("Libre").font(.caption).foregroundStyle(.secondary)
                }
            }

            HStack(spacing: 6) {
                ForEach(machine.operations, id: \.self) { op in
                    Text(op)
                        .font(.caption)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 2)
                        .background(Capsule().fill(Color.accentColor.opacity(0.15)))
                }
                if let laize = machine.laizeMm {
                    Text("Laize \(Int(laize.rounded())) mm")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                if let pool = machine.pool {
                    Text("Pool \(pool)")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }

            Divider()

            if liste.isEmpty {
                Text("Aucune sortie en attente.")
                    .foregroundStyle(.secondary)
                    .font(.callout)
            } else {
                VStack(alignment: .leading, spacing: 8) {
                    ForEach(liste) { s in
                        LigneSortie(sortie: s, avecMachine: false)
                    }
                }
            }
        }
        .padding(16)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(nsColor: .controlBackgroundColor)))
        .overlay(RoundedRectangle(cornerRadius: 12).strokeBorder(Color(nsColor: .separatorColor)))
    }
}

/// Une sortie de la file : nom, état, raison d'attente et menu d'actions.
@MainActor
struct LigneSortie: View {
    @Environment(Etat.self) private var etat
    let sortie: Sortie
    var avecMachine: Bool

    private var symbole: String {
        switch sortie.etat {
        case .enCours: return "play.circle.fill"
        case .enAttente: return "clock"
        case .suspendue: return "pause.circle"
        case .terminee: return "checkmark.circle.fill"
        case .echec: return "exclamationmark.triangle.fill"
        case .annulee: return "xmark.circle"
        case .imbriquee: return "square.grid.2x2"
        case .inconnu: return "questionmark.circle"
        }
    }

    private var couleur: Color {
        switch sortie.etat {
        case .enCours: return .blue
        case .terminee: return .green
        case .echec: return .red
        case .suspendue: return .orange
        default: return .secondary
        }
    }

    var body: some View {
        HStack(alignment: .top, spacing: 8) {
            Image(systemName: symbole)
                .foregroundStyle(couleur)
                .frame(width: 20)
            VStack(alignment: .leading, spacing: 2) {
                Text("\(sortie.nom) · page \(sortie.page)")
                    .lineLimit(1)
                    .truncationMode(.middle)
                HStack(spacing: 6) {
                    Text(sortie.etat.libelle)
                    if avecMachine {
                        Text("· \(etat.nomMachine(sortie.machine))")
                    }
                    if let d = sortie.dureeMin, d > 0, sortie.etat.active {
                        Text("· \(Int(d.rounded(.up))) min")
                    }
                }
                .font(.caption)
                .foregroundStyle(.secondary)
                if let code = sortie.raisonAttente, code != "attente_operateur" {
                    let r = Libelles.raison(code)
                    Text(r.titre)
                        .font(.caption)
                        .foregroundStyle(.orange)
                        .help(r.aide)
                }
                if sortie.etat == .echec {
                    Text(Libelles.erreur(sortie.codeErreur ?? "sortie_echouee", sortie.erreur))
                        .font(.caption)
                        .foregroundStyle(.red)
                        .lineLimit(3)
                }
            }
            Spacer(minLength: 4)
            if sortie.etat == .enCours {
                ProgressView().controlSize(.small)
            }
            if !sortie.actions.isEmpty {
                Menu {
                    ForEach(sortie.actions) { action in
                        Button(action.libelle, role: action.destructive ? .destructive : nil) {
                            Task { await etat.agir(sortie, action) }
                        }
                    }
                } label: {
                    Image(systemName: "ellipsis.circle")
                }
                .menuStyle(.borderlessButton)
                .menuIndicator(.hidden)
                .fixedSize()
                .help("Actions sur cette sortie")
            }
        }
    }
}
