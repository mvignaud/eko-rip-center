namespace EkoRip.Coeur.Tests;

public class ReglesTests
{
    private static Travail T(string id, string nom, StatutTravail statut = StatutTravail.Termine, long cree = 1, string? commande = null, string? erp = null, int pages = 1, string? mode = null) => new()
    {
        Id = id,
        Nom = nom,
        Statut = statut,
        CreeLe = cree,
        Mode = mode,
        Fiche = commande is null ? null : new Fiche { Commande = commande },
        Erp = erp is null ? null : new LienErp { RefErp = erp },
        Pages = Enumerable.Range(0, pages).Select(_ => new Page()).ToList(),
    };

    [Fact]
    public void Recherche_sans_casse_ni_accents_sur_le_nom_et_le_dossier()
    {
        var travaux = new[]
        {
            T("1", "Bâche Été.pdf", commande: "C-1"),
            T("2", "plan.pdf", erp: "ERP-ÉCOLE"),
            T("3", "autre.pdf"),
        };
        Assert.Equal(new[] { "1" }, Regles.Filtrer(travaux, "bache ete").Select(t => t.Id));
        Assert.Equal(new[] { "2" }, Regles.Filtrer(travaux, "ecole").Select(t => t.Id));
        Assert.Equal(new[] { "1" }, Regles.Filtrer(travaux, " c-1 ").Select(t => t.Id));
        Assert.Equal(3, Regles.Filtrer(travaux, "   ").Count);
        Assert.Equal(3, Regles.Filtrer(travaux, null).Count);
        Assert.Empty(Regles.Filtrer(travaux, "zzz"));
    }

    [Fact]
    public void Tri_par_defaut_les_plus_recents_en_haut()
    {
        var travaux = new[] { T("a", "a", cree: 10), T("b", "b", cree: 30), T("c", "c", cree: 20) };
        Assert.Equal(new[] { "b", "c", "a" }, Regles.Trier(travaux, ColonneTri.Date, true).Select(t => t.Id));
        Assert.Equal(new[] { "a", "c", "b" }, Regles.Trier(travaux, ColonneTri.Date, false).Select(t => t.Id));
    }

    [Fact]
    public void Tri_par_statut_ce_qui_travaille_d_abord()
    {
        var travaux = new[]
        {
            T("pret", "x", StatutTravail.Termine), T("echec", "x", StatutTravail.Echec),
            T("attente", "x", StatutTravail.EnAttente), T("cours", "x", StatutTravail.EnCours),
        };
        Assert.Equal(new[] { "cours", "attente", "echec", "pret" }, Regles.Trier(travaux, ColonneTri.Statut, false).Select(t => t.Id));
    }

    [Fact]
    public void Tri_par_nom_et_pages_et_dossier()
    {
        var travaux = new[] { T("1", "banane", pages: 3, commande: "Z"), T("2", "Abricot", pages: 1, commande: "A"), T("3", "cerise", pages: 2, commande: "M") };
        Assert.Equal(new[] { "2", "1", "3" }, Regles.Trier(travaux, ColonneTri.Nom, false).Select(t => t.Id));
        Assert.Equal(new[] { "2", "3", "1" }, Regles.Trier(travaux, ColonneTri.Pages, false).Select(t => t.Id));
        Assert.Equal(new[] { "2", "3", "1" }, Regles.Trier(travaux, ColonneTri.Dossier, false).Select(t => t.Id));
        Assert.Equal(new[] { "3", "1", "2" }, Regles.Trier(travaux, ColonneTri.Nom, true).Select(t => t.Id));
    }

    [Fact]
    public void Tri_stable_a_egalite()
    {
        var travaux = new[] { T("1", "x", pages: 2), T("2", "x", pages: 2), T("3", "x", pages: 2) };
        Assert.Equal(new[] { "1", "2", "3" }, Regles.Trier(travaux, ColonneTri.Pages, false).Select(t => t.Id));
        Assert.Equal(new[] { "1", "2", "3" }, Regles.Trier(travaux, ColonneTri.Pages, true).Select(t => t.Id));
    }

    [Fact]
    public void Machines_utilisables_selon_le_mode_du_travail()
    {
        var machines = new[]
        {
            new Machine { Id = "e", Nom = "Epson", Modele = "epson_sc_s80600" },
            new Machine { Id = "k", Nom = "Kona", Modele = "mutoh_kona" },
            new Machine { Id = "r", Nom = "Roland", Modele = "roland_versacamm" },
        };
        Assert.Equal(new[] { "e", "r" }, Regles.MachinesPour(machines, T("1", "a")).Select(m => m.Id));
        Assert.Equal(new[] { "k", "r" }, Regles.MachinesPour(machines, T("1", "a", mode: "decouper")).Select(m => m.Id));
        Assert.Equal("impression", Regles.Operation(T("1", "a")));
        Assert.Equal("decoupe", Regles.Operation(T("1", "a", mode: "decouper")));
        Assert.Equal("inconnue", Regles.NomMachine(machines, "inconnue"));
        Assert.Equal("Kona", Regles.NomMachine(machines, "k"));
    }

    [Fact]
    public void Sorties_d_un_travail_et_d_une_machine()
    {
        var sorties = new[]
        {
            new Sortie { Id = "s1", Travail = "t1", Machine = "e", Etat = EtatSortie.EnAttente },
            new Sortie { Id = "s2", Travail = "t1", Machine = "e", Etat = EtatSortie.EnCours },
            new Sortie { Id = "s3", Travail = "t1", Machine = "e", Etat = EtatSortie.Terminee },
            new Sortie { Id = "s4", Travail = "t2", Machine = "e", Etat = EtatSortie.Suspendue },
            new Sortie { Id = "s5", Travail = "t1", Machine = "e", Etat = EtatSortie.EnAttente, Archivee = true },
            new Sortie { Id = "s6", Travail = "t2", Machine = "k", Etat = EtatSortie.EnAttente },
        };
        Assert.Equal(new[] { "s1", "s2", "s3" }, Regles.SortiesDe(sorties, T("t1", "a")).Select(s => s.Id));
        var epson = new Machine { Id = "e", Nom = "Epson", Modele = "epson_sc_s80600" };
        // Celle qui tourne d'abord, puis la file dans son ordre ; ni terminée ni archivée.
        Assert.Equal(new[] { "s2", "s1", "s4" }, Regles.SortiesActivesPour(sorties, epson).Select(s => s.Id));
        Assert.Equal(4, Regles.CompteSorties(sorties));
    }

    [Fact]
    public void Suspendre_ou_reprendre_selon_les_sorties()
    {
        var t = T("t1", "a");
        var attente = new[] { new Sortie { Travail = "t1", Etat = EtatSortie.EnAttente } };
        var suspendue = new[] { new Sortie { Travail = "t1", Etat = EtatSortie.Suspendue } };
        Assert.True(Regles.PeutSuspendre(attente, t));
        Assert.False(Regles.PeutReprendre(attente, t));
        Assert.True(Regles.PeutReprendre(suspendue, t));
        Assert.False(Regles.PeutSuspendre(suspendue, t));
        Assert.False(Regles.PeutSuspendre(new[] { new Sortie { Travail = "autre", Etat = EtatSortie.EnAttente } }, t));
    }

    [Fact]
    public void Page_valide_et_pages_a_imprimer()
    {
        var t = new Travail { Pages = new() { new(), new(), new() }, PagesExclues = new() { 2 } };
        Assert.Equal(1, Regles.PageValide(t, 0));
        Assert.Equal(3, Regles.PageValide(t, 9));
        Assert.Equal(new[] { 1, 3 }, Regles.PagesAImprimer(t));
        Assert.Equal(1, Regles.PageValide(new Travail(), 5));
        Assert.Equal(new[] { 1 }, Regles.PagesAImprimer(new Travail()));
    }

    [Fact]
    public void Compte_des_travaux_en_cours_de_rendu()
    {
        var travaux = new[] { T("1", "a", StatutTravail.EnCours), T("2", "b", StatutTravail.EnAttente), T("3", "c", StatutTravail.Termine) };
        Assert.Equal(2, Regles.CompteTravaux(travaux));
    }

    [Fact]
    public void Fichiers_acceptes_a_l_envoi()
    {
        var retenus = Regles.FichiersRetenus(new[]
        {
            @"C:\a\plan.PDF", @"C:\a\logo.svg", @"C:\a\photo.Jpeg", @"C:\a\scan.tif", "/tmp/x.ai", @"C:\a\notes.txt", @"C:\a\archive.zip", @"C:\a\sans_extension",
        });
        Assert.Equal(new[] { @"C:\a\plan.PDF", @"C:\a\logo.svg", @"C:\a\photo.Jpeg", @"C:\a\scan.tif", "/tmp/x.ai" }, retenus);
    }

    [Theory]
    [InlineData("", true)]
    [InlineData("  ", true)]
    [InlineData("0", true)]
    [InlineData("100000", true)]
    [InlineData("100001", false)]
    [InlineData("-1", false)]
    [InlineData("3,5", false)]
    [InlineData("abc", false)]
    [InlineData(null, true)]
    public void Quantite_valide(string? q, bool attendu)
    {
        Assert.Equal(attendu, Regles.QuantiteValide(q));
    }

    [Fact]
    public void Champs_d_envoi_dans_l_ordre_sans_les_vides()
    {
        Assert.Equal(
            new[] { ("reglage", "Vinyle"), ("client", "Mairie"), ("commande", "C-42"), ("quantite", "3") },
            Regles.ChampsEnvoi("Vinyle", " Mairie ", "C-42", "3"));
        Assert.Equal(new[] { ("quantite", "5") }, Regles.ChampsEnvoi(null, "", "  ", " 5 "));
        Assert.Empty(Regles.ChampsEnvoi("", null, null, null));
    }

    [Fact]
    public void Nom_de_l_epreuve_TIFF()
    {
        Assert.Equal("affiche-p2-epreuve.tif", Regles.NomEpreuve("affiche.pdf", 2));
        Assert.Equal("a_b-p1-epreuve.tif", Regles.NomEpreuve("a:b.pdf", 1));
        Assert.Equal("sans-p1-epreuve.tif", Regles.NomEpreuve("sans", 1));
        Assert.Equal("v1.2-p1-epreuve.tif", Regles.NomEpreuve("v1.2.pdf", 1));
    }

    [Theory]
    [InlineData(0.2, "moins d'une minute")]
    [InlineData(2.2, "2 min")]
    [InlineData(59.4, "59 min")]
    [InlineData(65, "1 h 05")]
    [InlineData(150, "2 h 30")]
    public void Duree_estimee(double minutes, string attendu)
    {
        Assert.Equal(attendu, Regles.DureeTexte(minutes));
    }

    [Theory]
    [InlineData(0, "0 octets")]
    [InlineData(999, "999 octets")]
    [InlineData(1500, "1,5 Ko")]
    [InlineData(5368709120, "5,37 Go")]
    [InlineData(270553174016, "271 Go")]
    public void Tailles(long octets, string attendu)
    {
        // Le séparateur décimal dépend de la culture disponible ; on compare avec le point toléré.
        Assert.Equal(attendu, Regles.Octets(octets).Replace('.', ','));
    }

    [Fact]
    public void Taille_inconnue()
    {
        Assert.Equal("inconnu", Regles.Octets(null));
    }
}
