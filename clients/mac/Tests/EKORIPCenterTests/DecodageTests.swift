import XCTest
@testable import EKORIPCenter

/// Décodage de réponses réelles du serveur 0.28.0 (capturées avec `curl` et un User-Agent
/// `EKO-RIP-Client/1` sur un serveur neuf ; le travail et la sortie ont été fabriqués à la
/// main dans `job.json` faute de Ghostscript dans le conteneur de capture).
final class DecodageTests: XCTestCase {
    private func decoder(_ json: String) -> (Data, JSONDecoder) { (Data(json.utf8), JSONDecoder.serveur) }

    func testSession() throws {
        let (d, dec) = decoder(#"""
{"acces_libre":true,"role":"admin","utilisateur":null}
"""#)
        let s = try dec.decode(InfoSession.self, from: d)
        XCTAssertEqual(s.accesLibre, true)
        XCTAssertNil(s.utilisateur)
        XCTAssertEqual(s.role, "admin")
    }

    func testTravaux() throws {
        let (d, dec) = decoder(#"""
[{"abandon_polices":false,"alertes":[{"code":"image_basse_definition","detail":"Im : 72 ppi","page":1},{"code":"image_sans_resolution","detail":"72 ppi \u00b7 90 \u00d7 90 mm","page":null}],"auto":{"decouper":false,"imprimer":false,"suspendre":false},"blanc":{"alpha":"aucune","mode":"aucun","retraction_mm":0.2,"seuil":5.0},"code_erreur":null,"couches":"dessous","couches_decoupe":[],"couleur":{"entree_cmjn":null,"entree_rvb":null,"forcer":false,"intention":0,"intention_tons":null,"sortie":null},"cree_le":1793368144,"decoupeuse":null,"dpi":720,"duree_impression_min":2.220924705882353,"edition":{"code":null,"contour_mm":null,"contour_sur":"image","echelle":100.0,"echelle_xy":null,"etiquette":null,"filtre":null,"fond_perdu":null,"marge_mm":0.0,"marques":null,"miroir":false,"miroir_portee":"image_et_decoupe","oeillets":null,"recadrage":null,"rotation":0},"erreur":null,"etendre_coupe":null,"fiche":{"client":"Mairie","commande":"C-42","echeance":null,"notes":"","operateur":"","quantite":3,"societe":""},"fond_reperes":null,"format":"pdf","id":"6c8d3b11-861b-4661-a729-45f9c80dba20","imbrication":null,"mise_coupe":{"cintrage":null,"copies":[1,1],"decalage_mm":0.0,"depuis_origine":false,"ecart_mm":[0.0,0.0],"echelle":[100.0,100.0],"miroir":false,"rotation":0},"mode":"imprimer","mode_uuid":null,"mosaique":null,"nom":"t.png","page_par_travail":false,"pages":[{"cadre_pt":null,"couverture":{},"detourage":0,"estimation":{"cout_encres":0.0,"cout_support":0.0,"encres":[],"ml":0.0,"support":null,"surface_m2":0.0081,"surface_support_m2":0.0081,"total":0.0,"ttc":false},"geometrie":null,"hauteur_mm":90.0,"largeur_mm":90.0,"plaques":["Cyan","Magenta","Yellow","Black"],"plaques_decoupe":[],"reperage":null,"surface_m2":0.0081,"taille_coupe_pt":null,"traces_decoupe":0,"traces_par_couche":{},"tuile":null},{"cadre_pt":null,"couverture":{},"detourage":0,"estimation":{"cout_encres":0.0,"cout_support":0.0,"encres":[],"ml":0.0,"support":null,"surface_m2":0.0,"surface_support_m2":0.0,"total":0.0,"ttc":false},"geometrie":null,"hauteur_mm":297.0,"largeur_mm":210.0,"plaques":["Cyan","Magenta","Yellow","Black"],"plaques_decoupe":[],"reperage":null,"surface_m2":0.0,"taille_coupe_pt":null,"traces_decoupe":0,"traces_par_couche":{},"tuile":null}],"pages_exclues":[],"prix_de_revient":0.0,"reperage":"aucun","rotation_auto":true,"statut":"termine","support":null,"support_uuid":null,"tous_les_traces":false}]
"""#)
        let travaux = try dec.decode([Travail].self, from: d)
        let t = try XCTUnwrap(travaux.first)
        XCTAssertEqual(t.nom, "t.png")
        XCTAssertEqual(t.statut, .termine)
        XCTAssertEqual(t.nbPages, 2)
        XCTAssertEqual(t.dossier, "C-42")
        XCTAssertEqual(t.fiche?.client, "Mairie")
        XCTAssertEqual(t.fiche?.quantite, 3)
        XCTAssertEqual(t.pages?.first?.format, "90 × 90 mm")
        XCTAssertEqual(t.pages?.first?.plaques, ["Cyan", "Magenta", "Yellow", "Black"])
        XCTAssertEqual(t.alertes?.count, 2)
        XCTAssertEqual(t.creeLe, 1793368144)
    }

    func testTravailMinimalEtChampsInconnus() throws {
        let json = #"{"id":"a","nom":"x.pdf","statut":"nouveau_statut","cree_le":1,"champ_futur":{"a":[1,2]},"duree_impression_min":-0.0,"erreur":null}"#
        let t = try JSONDecoder.serveur.decode(Travail.self, from: Data(json.utf8))
        XCTAssertEqual(t.statut, .inconnu)
        XCTAssertEqual(t.nbPages, 0)
        XCTAssertEqual(t.dossier, "")
    }

    func testErpPrioritaireSurCommande() throws {
        let json = #"{"id":"a","nom":"x.pdf","statut":"echec","code_erreur":"rendu_echoue","erreur":"gs","cree_le":1,"erp":{"envoi":"e","ref_erp":"ERP-9"},"fiche":{"commande":"C-1"}}"#
        let t = try JSONDecoder.serveur.decode(Travail.self, from: Data(json.utf8))
        XCTAssertEqual(t.dossier, "ERP-9")
        XCTAssertEqual(t.texteErreur, "Le rendu a échoué. (gs)")
    }

    func testFile() throws {
        let (d, dec) = decoder(#"""
[{"archivee":false,"avant_imbrication":null,"bat_de":null,"code_erreur":"sortie_echouee","consommation":null,"couche":null,"cree_le":1793368453,"debut":1793368453,"duree_min":0.5164941176470589,"erreur":"copie de /tmp/claude-0/-home-user/82f2cb04-503e-5c62-a091-bcc87424ca94/scratchpad/data/travaux/6c8d3b11-861b-4661-a729-45f9c80dba20/prod/plaque-001(Black).tif : No such file or directory (os error 2)","etat":"echec","fiche":{"client":"Mairie","commande":"C-42","echeance":null,"notes":"","operateur":"","quantite":3,"societe":""},"fin":1793368453,"id":"bdd52f91-978e-42dd-8909-e040d8db0006","machine":"epson","nom":"t.png","operation":"impression","page":1,"planche":null,"planche_nom":null,"poids":null,"raison_attente":null,"travail":"6c8d3b11-861b-4661-a729-45f9c80dba20"}]
"""#)
        let sorties = try dec.decode([Sortie].self, from: d)
        let s = try XCTUnwrap(sorties.first)
        XCTAssertEqual(s.etat, .echec)
        XCTAssertEqual(s.machine, "epson")
        XCTAssertEqual(s.codeErreur, "sortie_echouee")
        XCTAssertEqual(s.page, 1)
        XCTAssertEqual(s.actions.map(\.rawValue), ["refaire", "retirer"])
    }

    func testMachines() throws {
        let (d, dec) = decoder(#"""
[{"id":"epson","uuid":"c669d090-d028-461b-86cf-0784394b073a","nom":"Epson SC-S80600","modele":"epson_sc_s80600","laize_mm":1626.0,"connexion":{"type":"dossier"},"snmp":null,"pas_mm":null,"decalage_mm":[0.0,0.0],"origine_impression_mm":[0.0,0.0],"m2_heure":null,"parametres_decoupe":[],"decoupe_retournee":false,"pool":null,"mono_couche":false,"coupe_auto":true,"variante_de":null,"reperage":null,"imbrication":{"strategie":"un_par_un","delai_min":null,"surface_m2":null,"longueur_cible_mm":null,"remplissage_pct":null,"inactivite_min":null,"longueur_max_mm":null,"rangee_pleine":false,"chargement":"libre","espace_mm":5.0,"espace_vertical_mm":null,"justification":"gauche","decalage_x_mm":0.0,"purge_cote":null,"purge_mm":10.0,"retourner_les":false,"marge_haut_mm":0.0,"marge_bas_mm":0.0,"contour":false}},{"id":"kona","uuid":"89a6a78a-bcc0-4718-87ee-d6f3f1f2e8fd","nom":"Mutoh Kona 1650","modele":"mutoh_kona","laize_mm":1650.0,"connexion":{"type":"serie","port":"/dev/kona","stty":"9600 cs8 -cstopb -parenb crtscts raw -echo"},"snmp":null,"pas_mm":null,"decalage_mm":[0.0,0.0],"origine_impression_mm":[0.0,0.0],"m2_heure":null,"parametres_decoupe":[{"uuid":"34139b53-4d22-40a5-8250-22c7ccee74d5","nom":"Mi-chair V3000","vitesse_cm_s":15.0,"force_g":145.0,"acceleration_g":1.0,"valeurs_machine":false,"outil":null,"motif_mm":[],"diffusion":false},{"uuid":"d0d65c4d-7601-4d26-9670-dcdd1d340e97","nom":"V3000WG - Lamin\u00e9","vitesse_cm_s":15.0,"force_g":305.0,"acceleration_g":1.0,"valeurs_machine":false,"outil":null,"motif_mm":[],"diffusion":false}],"decoupe_retournee":false,"pool":null,"mono_couche":false,"coupe_auto":true,"variante_de":null,"reperage":null,"imbrication":{"strategie":"un_par_un","delai_min":null,"surface_m2":null,"longueur_cible_mm":null,"remplissage_pct":null,"inactivite_min":null,"longueur_max_mm":null,"rangee_pleine":false,"chargement":"libre","espace_mm":5.0,"espace_vertical_mm":null,"justification":"gauche","decalage_x_mm":0.0,"purge_cote":null,"purge_mm":10.0,"retourner_les":false,"marge_haut_mm":0.0,"marge_bas_mm":0.0,"contour":false}}]
"""#)
        let machines = try dec.decode([Machine].self, from: d)
        XCTAssertEqual(machines.map(\.id), ["epson", "kona"])
        XCTAssertEqual(machines[0].operations, ["Impression"])
        XCTAssertEqual(machines[1].operations, ["Découpe"])
        XCTAssertEqual(machines[0].laizeMm, 1626)
    }

    func testReglages() throws {
        let (d, dec) = decoder(#"""
[{"nom":"Vinyle","uuid":"6d12c066-48a5-4969-8ca2-a2d2879e04d0","demande":{"mode":"imprimer","dpi":720}}]
"""#)
        let r = try dec.decode([Reglage].self, from: d)
        XCTAssertEqual(r.map(\.nom), ["Vinyle"])
    }

    func testSysteme() throws {
        let (d, dec) = decoder(#"""
{"alertes":[{"dossier":true,"fichier":"/tmp/claude-0/-home-user/82f2cb04-503e-5c62-a091-bcc87424ca94/scratchpad/data/sorties/epson","reprise":true}],"conservation_jours":30,"disque_libre":27969183744,"disque_total":270553174016,"rendus":{"en_attente":0,"en_cours":0,"simultanes":4},"seuil_libre":5368709120,"tampon":0,"version":"0.28.0"}
"""#)
        let s = try dec.decode(InfoSysteme.self, from: d)
        XCTAssertEqual(s.version, "0.28.0")
        XCTAssertEqual(s.rendus?.simultanes, 4)
        XCTAssertEqual(s.rendus?.enCours, 0)
        XCTAssertEqual(s.conservationJours, 30)
        XCTAssertEqual(s.alertes?.count, 1)
        XCTAssertEqual(s.alertes?.first?.dossier, true)
    }

    func testErreurs() {
        let e = ErreurApi.depuis(statut: 401, corps: Data(#"{"code":"connexion_requise","detail":null}"#.utf8))
        XCTAssertTrue(e.connexionRequise)
        XCTAssertEqual(e.errorDescription, "Connexion requise.")
        let mauvais = ErreurApi.depuis(statut: 401, corps: Data(#"{"code":"identifiants_invalides","detail":null}"#.utf8))
        XCTAssertFalse(mauvais.connexionRequise)
        XCTAssertEqual(Libelles.erreur("code_inconnu", "détail"), "code_inconnu (détail)")
        XCTAssertEqual(ErreurApi.depuis(statut: 400, corps: Data("Invalid URL".utf8)).code, "erreur_http")
    }

    func testAdresse() {
        XCTAssertEqual(Api.normaliser("192.168.10.109"), "http://192.168.10.109:8640")
        XCTAssertEqual(Api.normaliser("rip.local:9000/"), "http://rip.local:9000")
        XCTAssertEqual(Api.normaliser("https://rip.e-ko.fr"), "https://rip.e-ko.fr")
        XCTAssertNil(Api.normaliser("  "))
        XCTAssertEqual(Api.affichable("http://h:8640"), "h:8640")
    }

    func testMultipart() throws {
        let source = FileManager.default.temporaryDirectory.appendingPathComponent("source-\(UUID()).pdf")
        try Data("%PDF-test".utf8).write(to: source)
        defer { try? FileManager.default.removeItem(at: source) }
        let corps = try Multipart.fabriquer(champs: [("reglage", "Vinyle"), ("quantite", "3")], fichier: source, frontiere: "XX")
        defer { try? FileManager.default.removeItem(at: corps) }
        let texte = try String(contentsOf: corps, encoding: .utf8)
        let ordre = ["name=\"reglage\"", "name=\"quantite\"", "name=\"fichier\"", "%PDF-test", "--XX--"].compactMap { texte.range(of: $0)?.lowerBound }
        XCTAssertEqual(ordre.count, 5)
        XCTAssertEqual(ordre, ordre.sorted(), "le fichier vient en dernier")
    }
}
