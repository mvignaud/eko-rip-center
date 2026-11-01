import SwiftUI
import AppKit

/// Écran d'adresse du serveur, puis d'identifiant quand le serveur n'a pas l'accès libre.
/// Deux choix en tête : « Serveur local » (adresse, découverte réseau) ou « Se connecter au
/// cloud » (compte E-KO, liste des sites : voir `SectionsCloud`).
@MainActor
struct ConnexionVue: View {
    @Environment(Etat.self) private var etat
    @AppStorage(Cles.adresse) private var adresseRetenue = ""
    @State private var saisie = ""
    @State private var nom = ""
    @State private var motDePasse = ""
    @AppStorage("dernierUtilisateur") private var dernierUtilisateur = ""
    @AppStorage(Cles.modeConnexion) private var mode = "local"
    @State private var decouverte = Decouverte()
    @FocusState private var champ: Champ?

    private enum Champ { case adresse, nom, motDePasse }

    private var demandeIdentifiant: Bool { etat.phase == .connexion }
    private var enCloud: Bool { !demandeIdentifiant && mode == "cloud" }

    var body: some View {
        VStack(spacing: 22) {
            Image(nsImage: NSApp.applicationIconImage)
                .resizable()
                .frame(width: 88, height: 88)
            VStack(spacing: 4) {
                Text("E-KO RIP Center")
                    .font(.largeTitle.bold())
                Text(demandeIdentifiant ? "Connexion au serveur \(etat.adresse)" : (enCloud ? "Compte E-KO" : "Adresse du serveur RIP"))
                    .foregroundStyle(.secondary)
            }

            if !demandeIdentifiant {
                Picker("Mode de connexion", selection: $mode) {
                    Text(Libelles.cloudChoixLocal).tag("local")
                    Text(Libelles.cloudChoixCloud).tag("cloud")
                }
                .pickerStyle(.segmented)
                .labelsHidden()
                .frame(width: 400)
                .disabled(etat.occupe)
            }

            Form {
                if demandeIdentifiant {
                    TextField("Identifiant", text: $nom)
                        .focused($champ, equals: .nom)
                    SecureField("Mot de passe", text: $motDePasse)
                        .focused($champ, equals: .motDePasse)
                } else if enCloud {
                    SectionsCloud()
                } else {
                    Section("Serveurs trouvés sur le réseau") {
                        if decouverte.recherche {
                            HStack(spacing: 8) {
                                ProgressView().controlSize(.small)
                                Text("Recherche en cours…").foregroundStyle(.secondary)
                            }
                        }
                        ForEach(decouverte.serveurs) { serveur in
                            Button {
                                saisie = serveur.adresse
                                Task { await etat.connecter(serveur.adresse) }
                            } label: {
                                VStack(alignment: .leading, spacing: 2) {
                                    Text(serveur.nom)
                                    Text(serveur.adresse.replacingOccurrences(of: "http://", with: "") + (serveur.version.isEmpty ? "" : " · \(serveur.version)"))
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                }
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .contentShape(Rectangle())
                            }
                            .buttonStyle(.plain)
                            .disabled(etat.occupe)
                        }
                        if !decouverte.recherche && decouverte.serveurs.isEmpty {
                            Text("Aucun serveur trouvé sur le réseau : saisissez son adresse.")
                                .foregroundStyle(.secondary)
                        }
                        Button("Rechercher à nouveau") { decouverte.chercher() }
                            .disabled(decouverte.recherche)
                    }
                    Section("Saisir une adresse") {
                        TextField("Adresse", text: $saisie, prompt: Text("hôte:8640"))
                            .focused($champ, equals: .adresse)
                            .autocorrectionDisabled()
                    }
                }
            }
            .formStyle(.grouped)
            .scrollDisabled(demandeIdentifiant)
            .frame(width: 400, height: demandeIdentifiant ? 130 : 320)

            if let erreur = etat.erreurConnexion {
                Text(erreur)
                    .foregroundStyle(.red)
                    .multilineTextAlignment(.center)
                    .frame(maxWidth: 400)
            }

            if !enCloud {
                HStack(spacing: 12) {
                    if demandeIdentifiant {
                        Button("Changer de serveur") { etat.changerServeur() }
                    } else if !adresseRetenue.isEmpty {
                        Button("Annuler") { Task { await etat.connecter(adresseRetenue) } }
                    }
                    Button {
                        valider()
                    } label: {
                        if etat.occupe {
                            ProgressView().controlSize(.small)
                        } else {
                            Text("Se connecter")
                        }
                    }
                    .keyboardShortcut(.defaultAction)
                    .buttonStyle(.borderedProminent)
                    .disabled(etat.occupe || !pret)
                }
            }
        }
        .padding(40)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color(nsColor: .windowBackgroundColor))
        .onAppear {
            saisie = etat.adresse.isEmpty ? adresseRetenue : etat.adresse
            nom = dernierUtilisateur
            champ = demandeIdentifiant ? (nom.isEmpty ? .nom : .motDePasse) : .adresse
            if !demandeIdentifiant && mode == "local" { decouverte.chercher() }
            if !demandeIdentifiant && mode == "cloud" { Task { await actualiserCloudSiBesoin() } }
        }
        .onChange(of: mode) { _, nouveau in
            etat.erreurConnexion = nil
            if nouveau == "local" {
                decouverte.chercher()
            } else {
                decouverte.arreter()
                Task { await actualiserCloudSiBesoin() }
            }
        }
        .onDisappear { decouverte.arreter() }
        .onChange(of: etat.phase) { _, nouvelle in
            if nouvelle == .connexion {
                nom = dernierUtilisateur
                champ = nom.isEmpty ? .nom : .motDePasse
            } else if nouvelle == .adresse {
                champ = .adresse
                if mode == "local" { decouverte.chercher() }
                else { Task { await actualiserCloudSiBesoin() } }
            }
        }
    }

    /// Onglet cloud : relit le compte et les sites à chaque retour sur la liste si un jeton est
    /// gardé (un site hors ligne a pu revenir entre-temps).
    private func actualiserCloudSiBesoin() async {
        if etat.jetonCloudPresent && !etat.cloudEnLecture {
            await etat.actualiserCloud()
        }
    }

    private var pret: Bool {
        demandeIdentifiant
            ? !nom.trimmingCharacters(in: .whitespaces).isEmpty && !motDePasse.isEmpty
            : !saisie.trimmingCharacters(in: .whitespaces).isEmpty
    }

    private func valider() {
        if demandeIdentifiant {
            let utilisateur = nom.trimmingCharacters(in: .whitespaces)
            dernierUtilisateur = utilisateur
            let secret = motDePasse
            motDePasse = ""
            Task { await etat.ouvrirSession(nom: utilisateur, motDePasse: secret) }
        } else {
            Task { await etat.connecter(saisie) }
        }
    }
}

/// Onglet « Se connecter au cloud » : bouton de connexion tant qu'aucun compte n'est connecté,
/// puis les sites du compte (nom, en ligne ou hors ligne, rôle) et « Se déconnecter du cloud ».
@MainActor
struct SectionsCloud: View {
    @Environment(Etat.self) private var etat

    var body: some View {
        if let compte = etat.cloudCompte {
            Section {
                HStack {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(compte.affichable)
                        if compte.affichable != compte.email {
                            Text(compte.email).font(.caption).foregroundStyle(.secondary)
                        }
                    }
                    Spacer()
                    Button(Libelles.cloudDeconnexion) { Task { await etat.deconnecterCloud() } }
                        .disabled(etat.occupe)
                }
            }
            Section("Sites") {
                if etat.cloudSites.isEmpty {
                    Text("Aucun site sur ce compte : rattachez un serveur RIP depuis le portail.")
                        .foregroundStyle(.secondary)
                }
                ForEach(etat.cloudSites) { site in
                    Button {
                        Task { await etat.ouvrirSite(site) }
                    } label: {
                        HStack(spacing: 8) {
                            Image(systemName: "circle.fill")
                                .font(.caption2)
                                .foregroundStyle(site.enLigne ? Color.green : Color.secondary)
                            VStack(alignment: .leading, spacing: 2) {
                                Text(site.nom)
                                Text((site.enLigne ? Libelles.cloudEnLigne : Libelles.cloudHorsLigne)
                                     + (Libelles.roleCloud(site.role).isEmpty ? "" : " · " + Libelles.roleCloud(site.role)))
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }
                            Spacer()
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                    .disabled(etat.occupe || !site.enLigne)
                    .opacity(site.enLigne ? 1 : 0.55)
                }
            }
        } else {
            Section {
                Text(Libelles.cloudIntro)
                    .foregroundStyle(.secondary)
                if etat.cloudEnLecture {
                    HStack(spacing: 8) {
                        ProgressView().controlSize(.small)
                        Text("Lecture du compte…").foregroundStyle(.secondary)
                    }
                }
                Button {
                    Task { await etat.connecterAuCloud() }
                } label: {
                    if etat.occupe {
                        ProgressView().controlSize(.small)
                    } else {
                        Text(Libelles.cloudChoixCloud)
                    }
                }
                .buttonStyle(.borderedProminent)
                .disabled(etat.occupe || etat.cloudEnLecture)
            }
        }
    }
}
