import SwiftUI

/// État du serveur (`GET /api/systeme`) et version du client.
@MainActor
struct VueSysteme: View {
    @Environment(Etat.self) private var etat
    @AppStorage(ClesTelemetrie.desactive) private var statistiquesDesactivees = false

    private func octets(_ n: Int?) -> String {
        guard let n else { return "inconnu" }
        return ByteCountFormatter.string(fromByteCount: Int64(n), countStyle: .file)
    }

    var body: some View {
        Group {
            if let s = etat.systeme {
                Form {
                    Section("Serveur") {
                        LabeledContent("Adresse", value: etat.adresse)
                        LabeledContent("Version du serveur", value: s.version ?? "inconnue")
                        LabeledContent("Version du client", value: VersionClient.numero)
                        if let u = etat.session?.utilisateur {
                            LabeledContent("Utilisateur", value: u)
                        } else if etat.session?.accesLibre == true {
                            LabeledContent("Accès", value: "Libre (aucun compte)")
                        }
                    }

                    Section("Disque des données") {
                        disque(s)
                        LabeledContent("Tampon de la file", value: octets(s.tampon))
                        if let jours = s.conservationJours {
                            LabeledContent("Conservation des travaux", value: "\(jours) jours")
                        }
                    }

                    Section("Rendus") {
                        LabeledContent("En cours", value: "\(s.rendus?.enCours ?? 0)")
                        LabeledContent("En attente", value: "\(s.rendus?.enAttente ?? 0)")
                        if let n = s.rendus?.simultanes {
                            LabeledContent("Simultanés au plus", value: "\(n)")
                        }
                    }

                    Section("Confidentialité") {
                        Toggle("Envoyer des statistiques anonymes d'installation", isOn: Binding(
                            get: { !statistiquesDesactivees },
                            set: { statistiquesDesactivees = !$0 }
                        ))
                        Text("Un identifiant aléatoire, le type de poste, l'OS et la version, pour compter les installations. Aucune donnée personnelle ni fichier.")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }

                    Section("Alertes de dossiers") {
                        if let alertes = s.alertes, !alertes.isEmpty {
                            ForEach(alertes) { a in
                                Label {
                                    VStack(alignment: .leading, spacing: 2) {
                                        Text(texte(a))
                                        Text(a.fichier)
                                            .font(.caption)
                                            .foregroundStyle(.secondary)
                                            .textSelection(.enabled)
                                    }
                                } icon: {
                                    Image(systemName: "exclamationmark.triangle.fill")
                                        .foregroundStyle(.orange)
                                }
                            }
                        } else {
                            Text("Aucune alerte.").foregroundStyle(.secondary)
                        }
                    }
                }
                .formStyle(.grouped)
            } else {
                ProgressView("Lecture de l'état du serveur…")
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
    }

    /// Place libre, avec une alerte sous le seuil réglé sur le serveur.
    @ViewBuilder
    private func disque(_ s: InfoSysteme) -> some View {
        if let total = s.disqueTotal, let libre = s.disqueLibre, total > 0 {
            let occupe = Double(total - libre) / Double(total)
            let bas = libre < (s.seuilLibre ?? 0)
            VStack(alignment: .leading, spacing: 6) {
                ProgressView(value: min(max(occupe, 0), 1))
                    .tint(bas ? .red : .accentColor)
                Text("\(octets(libre)) libres sur \(octets(total))")
                    .font(.callout)
                    .foregroundStyle(bas ? .red : .secondary)
            }
            if bas {
                Label("Espace libre sous le seuil du serveur (\(octets(s.seuilLibre))) : les nouveaux travaux risquent d'être refusés.", systemImage: "exclamationmark.triangle.fill")
                    .foregroundStyle(.red)
                    .font(.callout)
            }
        } else {
            LabeledContent("Espace libre", value: "inconnu")
        }
    }

    private func texte(_ a: AlerteDossier) -> String {
        let reprise = a.reprise ?? false
        if a.dossier == true {
            return reprise ? "Dossier absent, recréé" : "Dossier absent, impossible à recréer"
        }
        return reprise ? "Fichier d'état illisible, version précédente reprise" : "Fichier d'état illisible"
    }
}
