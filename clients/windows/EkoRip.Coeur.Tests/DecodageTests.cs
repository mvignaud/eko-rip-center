namespace EkoRip.Coeur.Tests;

/// <summary>
/// Décodage de réponses réelles du serveur 0.28.0 (les mêmes captures que les clients Mac et
/// Linux ; le travail et la sortie du premier exemple ont été fabriqués à la main).
/// </summary>
public class DecodageTests
{
    [Fact]
    public void Session()
    {
        var s = JsonServeur.Lire<InfoSession>("""{"acces_libre":true,"role":"admin","utilisateur":null}""")!;
        Assert.True(s.AccesLibre);
        Assert.Null(s.Utilisateur);
        Assert.Equal("admin", s.Role);
    }

    [Fact]
    public void Session_du_fichier_de_capture()
    {
        var s = JsonServeur.Lire<InfoSession>(Reponses.Lire("session.json"))!;
        Assert.True(s.AccesLibre);
    }

    [Fact]
    public void Travaux_reels_avec_deux_pages_et_alertes()
    {
        const string json = """
[{"abandon_polices":false,"alertes":[{"code":"image_basse_definition","detail":"Im : 72 ppi","page":1},{"code":"image_sans_resolution","detail":"72 ppi · 90 × 90 mm","page":null}],"auto":{"decouper":false,"imprimer":false,"suspendre":false},"blanc":{"alpha":"aucune","mode":"aucun","retraction_mm":0.2,"seuil":5.0},"code_erreur":null,"couches":"dessous","couches_decoupe":[],"couleur":{"entree_cmjn":null,"entree_rvb":null,"forcer":false,"intention":0,"intention_tons":null,"sortie":null},"cree_le":1793368144,"decoupeuse":null,"dpi":720,"duree_impression_min":2.220924705882353,"edition":{"code":null,"contour_mm":null,"contour_sur":"image","echelle":100.0,"echelle_xy":null,"etiquette":null,"filtre":null,"fond_perdu":null,"marge_mm":0.0,"marques":null,"miroir":false,"miroir_portee":"image_et_decoupe","oeillets":null,"recadrage":null},"erreur":null,"etendre_coupe":null,"fiche":{"client":"Mairie","commande":"C-42","echeance":null,"notes":"","operateur":"","quantite":3,"societe":""},"fond_reperes":null,"format":"pdf","id":"6c8d3b11-861b-4661-a729-45f9c80dba20","imbrication":null,"mode":"imprimer","nom":"t.png","pages":[{"cadre_pt":null,"couverture":{},"detourage":0,"geometrie":null,"hauteur_mm":90.0,"largeur_mm":90.0,"plaques":["Cyan","Magenta","Yellow","Black"],"plaques_decoupe":[],"reperage":null,"surface_m2":0.0081,"taille_coupe_pt":null,"traces_decoupe":0,"traces_par_couche":{},"tuile":null},{"hauteur_mm":297.0,"largeur_mm":210.0,"plaques":["Cyan","Magenta","Yellow","Black"],"plaques_decoupe":[],"surface_m2":0.0}],"pages_exclues":[],"prix_de_revient":0.0,"statut":"termine"}]
""";
        var travaux = JsonServeur.Lire<List<Travail>>(json)!;
        var t = Assert.Single(travaux);
        Assert.Equal("t.png", t.Nom);
        Assert.Equal(StatutTravail.Termine, t.Statut);
        Assert.Equal(2, t.NbPages);
        Assert.Equal("C-42", t.Dossier);
        Assert.Equal("Mairie", t.Fiche?.Client);
        Assert.Equal(3, t.Fiche?.Quantite);
        Assert.Equal("90 × 90 mm", t.Pages![0].FormatTexte);
        Assert.Equal("210 × 297 mm", t.Pages![1].FormatTexte);
        Assert.Equal(new[] { "Cyan", "Magenta", "Yellow", "Black" }, t.Pages![0].Plaques);
        Assert.Equal(2, t.Alertes!.Count);
        Assert.Null(t.Alertes[1].Page);
        Assert.Equal(1793368144, t.CreeLe);
        Assert.False(t.DecoupeSeule);
        Assert.Null(t.TexteErreur);
    }

    [Fact]
    public void Travaux_du_fichier_de_capture_dont_un_echec()
    {
        var travaux = JsonServeur.Lire<List<Travail>>(Reponses.Lire("travaux.json"))!;
        Assert.True(travaux.Count >= 2);
        var casse = travaux.First(t => t.Nom == "casse.pdf");
        Assert.Equal(StatutTravail.Echec, casse.Statut);
        Assert.Equal("rendu_echoue", casse.CodeErreur);
        Assert.Equal(0, casse.NbPages);
        Assert.Equal("Le rendu a échoué. (PDF illisible : failed parsing cross reference table)", casse.TexteErreur);
        // -0.0 du serveur : décodé sans erreur.
        Assert.NotNull(casse.DureeImpressionMin);
    }

    [Fact]
    public void Travail_minimal_champs_inconnus_et_statut_inconnu()
    {
        const string json = """{"id":"a","nom":"x.pdf","statut":"nouveau_statut","cree_le":1,"champ_futur":{"a":[1,2]},"duree_impression_min":-0.0,"erreur":null}""";
        var t = JsonServeur.Lire<Travail>(json)!;
        Assert.Equal(StatutTravail.Inconnu, t.Statut);
        Assert.Equal(0, t.NbPages);
        Assert.Equal("", t.Dossier);
    }

    [Fact]
    public void Statut_absent_ou_nul_donne_inconnu()
    {
        Assert.Equal(StatutTravail.Inconnu, JsonServeur.Lire<Travail>("""{"id":"a","nom":"x","statut":null,"cree_le":1}""")!.Statut);
        Assert.Equal(StatutTravail.Inconnu, JsonServeur.Lire<Travail>("""{"id":"a","nom":"x","statut":12,"cree_le":1}""")!.Statut);
        Assert.Equal(StatutTravail.Inconnu, JsonServeur.Lire<Travail>("""{"id":"a","nom":"x","cree_le":1}""")!.Statut);
    }

    [Fact]
    public void Dossier_Erp_prioritaire_sur_la_commande()
    {
        const string json = """{"id":"a","nom":"x.pdf","statut":"echec","code_erreur":"rendu_echoue","erreur":"gs","cree_le":1,"erp":{"envoi":"e","ref_erp":"ERP-9"},"fiche":{"commande":"C-1"}}""";
        var t = JsonServeur.Lire<Travail>(json)!;
        Assert.Equal("ERP-9", t.Dossier);
        Assert.Equal("Le rendu a échoué. (gs)", t.TexteErreur);
    }

    [Fact]
    public void Reponse_d_envoi_202()
    {
        var t = JsonServeur.Lire<Travail>(Reponses.Lire("envoi-202.json"))!;
        Assert.Equal("casse.pdf", t.Nom);
        Assert.Equal(StatutTravail.EnAttente, t.Statut);
        Assert.True(t.Fiche!.Vide);
    }

    [Fact]
    public void File_avec_une_sortie_en_echec()
    {
        const string json = """
[{"archivee":false,"avant_imbrication":null,"bat_de":null,"code_erreur":"sortie_echouee","consommation":null,"couche":null,"cree_le":1793368453,"debut":1793368453,"duree_min":0.5164941176470589,"erreur":"copie de plaque-001(Black).tif : No such file or directory (os error 2)","etat":"echec","fiche":{"client":"Mairie","commande":"C-42","echeance":null,"notes":"","operateur":"","quantite":3,"societe":""},"fin":1793368453,"id":"bdd52f91-978e-42dd-8909-e040d8db0006","machine":"epson","nom":"t.png","operation":"impression","page":1,"planche":null,"planche_nom":null,"poids":null,"raison_attente":null,"travail":"6c8d3b11-861b-4661-a729-45f9c80dba20"}]
""";
        var s = Assert.Single(JsonServeur.Lire<List<Sortie>>(json)!);
        Assert.Equal(EtatSortie.Echec, s.Etat);
        Assert.Equal("epson", s.Machine);
        Assert.Equal("sortie_echouee", s.CodeErreur);
        Assert.Equal(1, s.Page);
        Assert.Equal(new[] { ActionSortie.Refaire, ActionSortie.Retirer }, s.Actions);
        Assert.False(s.EstDecoupe);
    }

    [Fact]
    public void File_du_fichier_de_capture()
    {
        var s = Assert.Single(JsonServeur.Lire<List<Sortie>>(Reponses.Lire("file.json"))!);
        Assert.Equal(EtatSortie.Terminee, s.Etat);
        Assert.Equal("test.pdf", s.Nom);
    }

    [Fact]
    public void Machines_reelles()
    {
        var machines = JsonServeur.Lire<List<Machine>>(Reponses.Lire("machines.json"))!;
        Assert.Equal("epson", machines[0].Id);
        Assert.Equal(new[] { "Impression" }, machines[0].Operations);
        Assert.Equal(new[] { "Découpe" }, machines[1].Operations);
        Assert.Equal(1626, machines[0].LaizeMm);
        Assert.True(machines[0].Imprime);
        Assert.False(machines[0].Decoupe);
    }

    [Fact]
    public void Roland_imprime_et_decoupe()
    {
        var m = new Machine { Id = "r", Nom = "Roland", Modele = "roland_versacamm" };
        Assert.Equal(new[] { "Impression", "Découpe" }, m.Operations);
    }

    [Fact]
    public void Machine_visee_directement_ou_par_son_pool()
    {
        var m = new Machine { Id = "k1", Nom = "Kona 1", Modele = "mutoh_kona", Pool = "kona" };
        Assert.True(m.Vise(new Sortie { Machine = "k1" }));
        Assert.True(m.Vise(new Sortie { Machine = "kona" }));
        Assert.False(m.Vise(new Sortie { Machine = "epson" }));
    }

    [Fact]
    public void Reglages()
    {
        var r = JsonServeur.Lire<List<Reglage>>("""[{"nom":"Vinyle","uuid":"6d12c066","demande":{"mode":"imprimer","dpi":720}}]""")!;
        Assert.Equal(new[] { "Vinyle" }, r.Select(x => x.Nom));
        Assert.Single(JsonServeur.Lire<List<Reglage>>(Reponses.Lire("reglages.json"))!);
    }

    [Fact]
    public void Systeme()
    {
        var s = JsonServeur.Lire<InfoSysteme>("""{"alertes":[{"dossier":true,"fichier":"/tmp/x/sorties/epson","reprise":true}],"conservation_jours":30,"disque_libre":27969183744,"disque_total":270553174016,"rendus":{"en_attente":0,"en_cours":0,"simultanes":4},"seuil_libre":5368709120,"tampon":0,"version":"0.28.0"}""")!;
        Assert.Equal("0.28.0", s.Version);
        Assert.Equal(4, s.Rendus?.Simultanes);
        Assert.Equal(0, s.Rendus?.EnCours);
        Assert.Equal(30, s.ConservationJours);
        Assert.Equal(27969183744, s.DisqueLibre);
        var a = Assert.Single(s.Alertes!);
        Assert.True(a.Dossier);
        Assert.Equal("Dossier absent, recréé", a.Texte);
        Assert.Equal(5368709120, JsonServeur.Lire<InfoSysteme>(Reponses.Lire("systeme.json"))!.SeuilLibre);
    }

    [Theory]
    [InlineData(false, false, "Fichier d'état illisible")]
    [InlineData(false, true, "Fichier d'état illisible, version précédente reprise")]
    [InlineData(true, false, "Dossier absent, impossible à recréer")]
    [InlineData(true, true, "Dossier absent, recréé")]
    public void Alertes_de_dossiers(bool dossier, bool reprise, string attendu)
    {
        Assert.Equal(attendu, new AlerteDossier { Dossier = dossier, Reprise = reprise }.Texte);
    }

    [Fact]
    public void Actions_possibles_d_une_sortie()
    {
        Assert.Equal(
            new[] { ActionSortie.EnTete, ActionSortie.Monter, ActionSortie.Descendre, ActionSortie.Suspendre, ActionSortie.Annuler },
            new Sortie { Etat = EtatSortie.EnAttente }.Actions);
        Assert.Equal(
            new[] { ActionSortie.EnTete, ActionSortie.Monter, ActionSortie.Descendre, ActionSortie.Reprendre, ActionSortie.Annuler },
            new Sortie { Etat = EtatSortie.Suspendue }.Actions);
        Assert.Equal(new[] { ActionSortie.Refaire, ActionSortie.Retirer }, new Sortie { Etat = EtatSortie.Annulee }.Actions);
        Assert.Equal(new[] { ActionSortie.Retirer }, new Sortie { Etat = EtatSortie.Imbriquee }.Actions);
        Assert.Empty(new Sortie { Etat = EtatSortie.EnCours }.Actions);
        Assert.Empty(new Sortie { Etat = EtatSortie.Inconnu }.Actions);
    }

    [Fact]
    public void Etat_de_sortie_inconnu_toleré()
    {
        var s = JsonServeur.Lire<Sortie>("""{"id":"s","machine":"m","travail":"t","nom":"n","page":1,"etat":"etat_futur"}""")!;
        Assert.Equal(EtatSortie.Inconnu, s.Etat);
        Assert.False(s.Etat.Active());
    }

    [Fact]
    public void Segments_d_URL_des_actions()
    {
        Assert.Equal("en_tete", ActionSortie.EnTete.Brut());
        Assert.Equal("reprendre", ActionSortie.Reprendre.Brut());
        Assert.True(ActionSortie.Annuler.Destructive());
        Assert.False(ActionSortie.Monter.Destructive());
    }

    [Fact]
    public void Format_de_page_inconnu_ou_a_decimales()
    {
        Assert.Null(new Page().FormatTexte);
        Assert.Null(new Page { LargeurMm = 0, HauteurMm = 10 }.FormatTexte);
        Assert.Equal("594,5 × 841 mm", new Page { LargeurMm = 594.5, HauteurMm = 841 }.FormatTexte);
    }

    [Fact]
    public void Fiche_vide_ou_non()
    {
        Assert.True(new Fiche { Commande = "", Quantite = 0 }.Vide);
        Assert.False(new Fiche { Notes = "urgent" }.Vide);
        Assert.False(new Fiche { Echeance = 5 }.Vide);
    }
}
