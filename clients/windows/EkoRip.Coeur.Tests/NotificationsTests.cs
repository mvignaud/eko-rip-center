namespace EkoRip.Coeur.Tests;

public class NotificationsTests
{
    private static Travail T(string id, StatutTravail s, string? code = null, string? erreur = null) =>
        new() { Id = id, Nom = $"{id}.pdf", Statut = s, CodeErreur = code, Erreur = erreur };

    private static Sortie S(string id, EtatSortie e, string operation = "impression", string? code = null) =>
        new() { Id = id, Nom = "plan.pdf", Page = 2, Machine = "epson", Operation = operation, Etat = e, CodeErreur = code };

    private static string Nom(string id) => id == "epson" ? "Epson SC-S80600" : id;

    [Fact]
    public void Le_premier_passage_ne_notifie_rien()
    {
        var suivi = new SuiviNotifications();
        var n = suivi.Analyser(new[] { T("a", StatutTravail.Termine), T("b", StatutTravail.Echec) }, new[] { S("s", EtatSortie.Terminee) }, Nom);
        Assert.Empty(n);
    }

    [Fact]
    public void Un_travail_qui_passe_a_termine_est_notifie_une_fois()
    {
        var suivi = new SuiviNotifications();
        suivi.Analyser(new[] { T("a", StatutTravail.EnCours) }, Array.Empty<Sortie>(), Nom);
        var n = suivi.Analyser(new[] { T("a", StatutTravail.Termine) }, Array.Empty<Sortie>(), Nom);
        Assert.Equal(new NotificationRip("Travail prêt", "a.pdf"), Assert.Single(n));
        Assert.Empty(suivi.Analyser(new[] { T("a", StatutTravail.Termine) }, Array.Empty<Sortie>(), Nom));
    }

    [Fact]
    public void Un_echec_de_rendu_dit_sa_cause()
    {
        var suivi = new SuiviNotifications();
        suivi.Analyser(new[] { T("a", StatutTravail.EnAttente) }, Array.Empty<Sortie>(), Nom);
        var n = Assert.Single(suivi.Analyser(new[] { T("a", StatutTravail.Echec, "pdf_illisible") }, Array.Empty<Sortie>(), Nom));
        Assert.Equal("Échec du rendu", n.Titre);
        Assert.StartsWith("a.pdf : PDF illisible", n.Texte);
    }

    [Fact]
    public void Un_travail_deja_fini_ou_nouveau_deja_fini_n_est_pas_notifie()
    {
        var suivi = new SuiviNotifications();
        suivi.Analyser(new[] { T("a", StatutTravail.Termine) }, Array.Empty<Sortie>(), Nom);
        // « b » apparaît déjà terminé : pas d'envoi suivi, pas de notification.
        Assert.Empty(suivi.Analyser(new[] { T("a", StatutTravail.Termine), T("b", StatutTravail.Termine) }, Array.Empty<Sortie>(), Nom));
    }

    [Fact]
    public void Un_envoi_suivi_est_notifie_meme_vu_pour_la_premiere_fois_termine()
    {
        var suivi = new SuiviNotifications();
        suivi.Analyser(Array.Empty<Travail>(), Array.Empty<Sortie>(), Nom);
        suivi.Suivre("nouveau");
        var n = Assert.Single(suivi.Analyser(new[] { T("nouveau", StatutTravail.Termine) }, Array.Empty<Sortie>(), Nom));
        Assert.Equal("Travail prêt", n.Titre);
        Assert.Empty(suivi.Analyser(new[] { T("nouveau", StatutTravail.Termine) }, Array.Empty<Sortie>(), Nom));
    }

    [Fact]
    public void Une_sortie_terminee_ou_en_echec_est_notifiee()
    {
        var suivi = new SuiviNotifications();
        suivi.Analyser(Array.Empty<Travail>(), new[] { S("s1", EtatSortie.EnCours), S("s2", EtatSortie.EnAttente, "decoupe"), S("s3", EtatSortie.Suspendue) }, Nom);
        var n = suivi.Analyser(Array.Empty<Travail>(), new[]
        {
            S("s1", EtatSortie.Terminee), S("s2", EtatSortie.Echec, "decoupe", "sortie_echouee"), S("s3", EtatSortie.Annulee),
        }, Nom);
        Assert.Equal(2, n.Count);
        Assert.Equal(new NotificationRip("Impression terminée", "plan.pdf, page 2 · Epson SC-S80600"), n[0]);
        Assert.Equal("Découpe en échec", n[1].Titre);
        Assert.Equal("plan.pdf, page 2 · Epson SC-S80600 : La sortie vers la machine a échoué.", n[1].Texte);
    }

    [Fact]
    public void Une_sortie_deja_terminee_n_est_pas_re_notifiee()
    {
        var suivi = new SuiviNotifications();
        suivi.Analyser(Array.Empty<Travail>(), new[] { S("s1", EtatSortie.Terminee) }, Nom);
        Assert.Empty(suivi.Analyser(Array.Empty<Travail>(), new[] { S("s1", EtatSortie.Terminee), S("s9", EtatSortie.Terminee) }, Nom));
    }

    [Fact]
    public void Reinitialiser_repart_d_un_premier_passage()
    {
        var suivi = new SuiviNotifications();
        suivi.Analyser(new[] { T("a", StatutTravail.EnCours) }, Array.Empty<Sortie>(), Nom);
        suivi.Reinitialiser();
        Assert.Empty(suivi.Analyser(new[] { T("a", StatutTravail.Termine) }, Array.Empty<Sortie>(), Nom));
    }
}
