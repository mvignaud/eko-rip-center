import SwiftUI

/// Fenêtre de travail : barre latérale, contenu de la rubrique, inspecteur à droite.
/// Le verre (matériaux translucides) reste sur la barre latérale et la barre d'outils, que le
/// système habille seul ; l'aperçu a son fond opaque.
@MainActor
struct PrincipalVue: View {
    @Environment(Etat.self) private var etat

    private var travail: Travail? { etat.travailSelectionne }

    private var machines: [Machine] {
        travail.map { etat.machines(pour: $0) } ?? []
    }

    private var peutImprimer: Bool {
        etat.rubrique == .travaux && travail?.statut == .termine && !machines.isEmpty
    }

    private var peutBasculer: Bool {
        guard etat.rubrique == .travaux, let t = travail else { return false }
        return etat.peutSuspendre(t) || etat.peutReprendre(t)
    }

    private var reprendre: Bool {
        guard let t = travail else { return false }
        return !etat.peutSuspendre(t) && etat.peutReprendre(t)
    }

    var body: some View {
        @Bindable var etat = etat
        NavigationSplitView {
            barreLaterale
        } detail: {
            detail
        }
        .searchable(text: $etat.recherche, prompt: "Nom ou dossier")
        .onChange(of: etat.selection) { _, _ in etat.page = 1 }
        .onChange(of: etat.rubrique) { _, _ in Task { await etat.rafraichir() } }
        .frame(minWidth: 980, minHeight: 560)
    }

    // MARK: Barre latérale

    private var barreLaterale: some View {
        List(selection: Binding<Rubrique?>(
            get: { etat.rubrique },
            set: { if let r = $0 { etat.rubrique = r } }
        )) {
            Section("Production") { entree(.travaux) }
            Section("Atelier") { entree(.machines) }
            Section("Serveur") { entree(.systeme) }
        }
        .navigationSplitViewColumnWidth(min: 170, ideal: 200, max: 260)
    }

    private func entree(_ r: Rubrique) -> some View {
        Label(r.titre, systemImage: r.symbole)
            .tag(r)
            .badge(compte(r))
    }

    /// Pastille de la barre latérale : travaux en cours de rendu, sorties à passer.
    private func compte(_ r: Rubrique) -> Int {
        switch r {
        case .travaux: return etat.travaux.filter { $0.statut == .enCours || $0.statut == .enAttente }.count
        case .machines: return etat.sorties.filter { $0.etat.active && ($0.archivee ?? false) == false }.count
        case .systeme: return 0
        }
    }

    // MARK: Contenu

    private var detail: some View {
        Group {
            switch etat.rubrique {
            case .travaux: VueTravaux()
            case .machines: VueMachines()
            case .systeme: VueSysteme()
            }
        }
        .navigationTitle(etat.rubrique.titre)
        .navigationSubtitle(etat.adresse)
        .safeAreaInset(edge: .bottom, spacing: 0) { BarreEtat() }
        .inspector(isPresented: Binding(
            get: { etat.inspecteurVisible && etat.rubrique == .travaux },
            set: { etat.inspecteurVisible = $0 }
        )) {
            InspecteurVue()
        }
        .toolbar { barreOutils }
    }

    // MARK: Barre d'outils

    @ToolbarContentBuilder
    private var barreOutils: some ToolbarContent {
        ToolbarItemGroup(placement: .primaryAction) {
            Button {
                etat.choisirFichiers()
            } label: {
                Label("Envoyer", systemImage: "plus")
            }
            .help("Envoyer des fichiers au RIP (⌘O)")

            Menu {
                if let t = travail {
                    ForEach(machines) { m in
                        Button(t.nbPages > 1 ? "Page \(etat.pageValide(t)) sur \(m.nom)" : m.nom) {
                            Task { await etat.imprimer(t, page: etat.pageValide(t), sur: m) }
                        }
                    }
                }
            } label: {
                Label(travail?.decoupeSeule == true ? "Découper" : "Imprimer", systemImage: "printer")
            }
            .disabled(!peutImprimer)
            .help("Mettre la page affichée en file sur une machine")

            Button {
                if let t = travail { Task { await etat.basculerSuspension(t) } }
            } label: {
                Label(reprendre ? "Reprendre" : "Suspendre", systemImage: reprendre ? "play.fill" : "pause.fill")
            }
            .disabled(!peutBasculer)
            .help(reprendre ? "Reprendre les sorties suspendues du travail" : "Suspendre les sorties en attente du travail")

            Button {
                if let t = travail { Task { await etat.epreuveTiff(t, page: etat.pageValide(t)) } }
            } label: {
                Label("Épreuve TIFF", systemImage: "square.and.arrow.down")
            }
            .disabled(etat.rubrique != .travaux || travail?.statut != .termine)
            .help("Télécharger l'épreuve TIFF de la page affichée")

            Button {
                etat.inspecteurVisible.toggle()
            } label: {
                Label("Inspecteur", systemImage: "sidebar.right")
            }
            .disabled(etat.rubrique != .travaux)
            .help("Afficher ou masquer l'inspecteur")
        }
    }
}
