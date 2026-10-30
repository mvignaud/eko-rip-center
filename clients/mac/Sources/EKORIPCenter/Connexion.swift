import SwiftUI
import AppKit

/// Écran d'adresse du serveur, puis d'identifiant quand le serveur n'a pas l'accès libre.
@MainActor
struct ConnexionVue: View {
    @Environment(Etat.self) private var etat
    @AppStorage(Cles.adresse) private var adresseRetenue = ""
    @State private var saisie = ""
    @State private var nom = ""
    @State private var motDePasse = ""
    @AppStorage("dernierUtilisateur") private var dernierUtilisateur = ""
    @FocusState private var champ: Champ?

    private enum Champ { case adresse, nom, motDePasse }

    private var demandeIdentifiant: Bool { etat.phase == .connexion }

    var body: some View {
        VStack(spacing: 22) {
            Image(nsImage: NSApp.applicationIconImage)
                .resizable()
                .frame(width: 88, height: 88)
            VStack(spacing: 4) {
                Text("E-KO RIP Center")
                    .font(.largeTitle.bold())
                Text(demandeIdentifiant ? "Connexion au serveur \(etat.adresse)" : "Adresse du serveur RIP")
                    .foregroundStyle(.secondary)
            }

            Form {
                if demandeIdentifiant {
                    TextField("Identifiant", text: $nom)
                        .focused($champ, equals: .nom)
                    SecureField("Mot de passe", text: $motDePasse)
                        .focused($champ, equals: .motDePasse)
                } else {
                    TextField("Adresse", text: $saisie, prompt: Text("hôte:8640"))
                        .focused($champ, equals: .adresse)
                        .autocorrectionDisabled()
                }
            }
            .formStyle(.grouped)
            .scrollDisabled(true)
            .frame(width: 400, height: demandeIdentifiant ? 130 : 80)

            if let erreur = etat.erreurConnexion {
                Text(erreur)
                    .foregroundStyle(.red)
                    .multilineTextAlignment(.center)
                    .frame(maxWidth: 400)
            }

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
        .padding(40)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color(nsColor: .windowBackgroundColor))
        .onAppear {
            saisie = etat.adresse.isEmpty ? adresseRetenue : etat.adresse
            nom = dernierUtilisateur
            champ = demandeIdentifiant ? (nom.isEmpty ? .nom : .motDePasse) : .adresse
        }
        .onChange(of: etat.phase) { _, nouvelle in
            if nouvelle == .connexion {
                nom = dernierUtilisateur
                champ = nom.isEmpty ? .nom : .motDePasse
            } else if nouvelle == .adresse {
                champ = .adresse
            }
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
