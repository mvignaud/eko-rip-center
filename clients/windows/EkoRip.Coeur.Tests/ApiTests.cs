using System.Net;
using System.Text;

namespace EkoRip.Coeur.Tests;

public class ApiTests
{
    private static Api Fabriquer(GestionnaireFictif g) => new("http://rip.local:8640", g);

    [Fact]
    public async Task Chaque_requete_porte_la_signature_du_client()
    {
        var g = new GestionnaireFictif(_ => GestionnaireFictif.Json("""{"acces_libre":true,"role":"admin","utilisateur":null}"""));
        using var api = Fabriquer(g);
        await api.LireSession();
        await api.Travaux().ContinueWith(_ => { });
        Assert.All(g.Requetes, r => Assert.StartsWith("EKO-RIP-Client/", r.UserAgent));
        Assert.All(g.Requetes, r => Assert.EndsWith("(Windows)", r.UserAgent));
    }

    [Fact]
    public async Task Les_lectures_visent_les_bonnes_routes()
    {
        var g = new GestionnaireFictif(r => r.Chemin switch
        {
            "/api/systeme" => GestionnaireFictif.Json("{}"),
            "/api/session" => GestionnaireFictif.Json("{}"),
            _ => GestionnaireFictif.Json("[]"),
        });
        using var api = Fabriquer(g);
        await api.Travaux();
        await api.File();
        await api.Machines();
        await api.Reglages();
        await api.Systeme();
        Assert.Equal(new[] { "/api/travaux", "/api/file", "/api/machines", "/api/reglages", "/api/systeme" }, g.Requetes.Select(r => r.Chemin));
    }

    [Fact]
    public async Task Ouverture_de_session_en_JSON()
    {
        var g = new GestionnaireFictif(_ => GestionnaireFictif.Json("{}"));
        using var api = Fabriquer(g);
        await api.OuvrirSession("marie", "se\"cret");
        var r = Assert.Single(g.Requetes);
        Assert.Equal(HttpMethod.Post, r.Methode);
        Assert.Equal("/api/session", r.Chemin);
        Assert.StartsWith("application/json", r.TypeContenu);
        Assert.Equal("""{"nom":"marie","mot_de_passe":"se\u0022cret"}""", r.CorpsTexte);
    }

    [Fact]
    public async Task Mise_en_file_avec_l_operation()
    {
        var g = new GestionnaireFictif(_ => GestionnaireFictif.Json("{}"));
        using var api = Fabriquer(g);
        await api.MettreEnFile("t1", 2, "epson", "impression");
        Assert.Equal("""{"travail":"t1","page":2,"machine":"epson","operation":"impression"}""", g.Requetes[0].CorpsTexte);
    }

    [Fact]
    public async Task Actions_sur_une_sortie_et_retrait()
    {
        var g = new GestionnaireFictif(_ => GestionnaireFictif.Json("{}"));
        using var api = Fabriquer(g);
        await api.Agir("s1", ActionSortie.EnTete);
        await api.Agir("s1", ActionSortie.Retirer);
        await api.Relancer("t1");
        await api.Supprimer("t1");
        Assert.Equal(
            new[] { "POST /api/file/s1/en_tete", "DELETE /api/file/s1", "POST /api/travaux/t1/relancer", "DELETE /api/travaux/t1" },
            g.Requetes.Select(r => $"{r.Methode} {r.Chemin}"));
    }

    [Fact]
    public async Task Erreur_du_serveur_avec_code_et_detail()
    {
        var g = new GestionnaireFictif(_ => GestionnaireFictif.Json("""{"code":"rien_a_imprimer","detail":null}""", HttpStatusCode.UnprocessableEntity));
        using var api = Fabriquer(g);
        var e = await Assert.ThrowsAsync<ErreurApi>(() => api.Relancer("t"));
        Assert.Equal(422, e.Statut);
        Assert.Equal("rien_a_imprimer", e.Code);
        Assert.Equal("Cette page n'a rien à imprimer.", e.Message);
    }

    [Fact]
    public async Task Un_401_demande_la_connexion_sauf_identifiants_invalides()
    {
        var g = new GestionnaireFictif(r => r.Chemin == "/api/session"
            ? GestionnaireFictif.Json("""{"code":"identifiants_invalides","detail":null}""", HttpStatusCode.Unauthorized)
            : GestionnaireFictif.Json("""{"code":"connexion_requise","detail":null}""", HttpStatusCode.Unauthorized));
        using var api = Fabriquer(g);
        var a = await Assert.ThrowsAsync<ErreurApi>(() => api.Travaux());
        Assert.True(a.ConnexionRequise);
        var b = await Assert.ThrowsAsync<ErreurApi>(() => api.OuvrirSession("x", "y"));
        Assert.False(b.ConnexionRequise);
    }

    [Fact]
    public async Task Reponse_illisible()
    {
        var g = new GestionnaireFictif(_ => GestionnaireFictif.Json("pas du json"));
        using var api = Fabriquer(g);
        var e = await Assert.ThrowsAsync<ErreurApi>(() => api.Travaux());
        Assert.Equal("reponse_illisible", e.Code);
    }

    [Fact]
    public async Task Panne_reseau_donne_le_statut_zero()
    {
        var g = new GestionnaireFictif(_ => throw new HttpRequestException("connexion refusée"));
        using var api = Fabriquer(g);
        var e = await Assert.ThrowsAsync<ErreurApi>(() => api.Travaux());
        Assert.Equal(0, e.Statut);
        Assert.Equal("reseau", e.Code);
        Assert.Equal("Serveur injoignable. (connexion refusée)", e.Message);
    }

    [Fact]
    public async Task Annulation_demandee_par_l_appelant_n_est_pas_une_erreur_reseau()
    {
        var g = new GestionnaireFictif(_ => GestionnaireFictif.Json("[]"));
        using var api = Fabriquer(g);
        using var cts = new CancellationTokenSource();
        cts.Cancel();
        await Assert.ThrowsAnyAsync<OperationCanceledException>(() => api.Travaux(cts.Token));
    }

    [Fact]
    public async Task Envoi_multipart_depuis_le_disque_avec_progression()
    {
        var chemin = Path.Combine(Path.GetTempPath(), $"eko-test-{Guid.NewGuid():N}.pdf");
        await File.WriteAllBytesAsync(chemin, Encoding.UTF8.GetBytes("%PDF-test"));
        try
        {
            var g = new GestionnaireFictif(_ => GestionnaireFictif.Json(Reponses.Lire("envoi-202.json"), HttpStatusCode.Accepted));
            using var api = Fabriquer(g);
            var valeurs = new List<double>();
            var travail = await api.Envoyer(chemin,
                Regles.ChampsEnvoi("Vinyle", "Mairie", "C-42", "3"),
                new ProgressionSynchrone(valeurs.Add));

            Assert.Equal("casse.pdf", travail.Nom);
            var r = Assert.Single(g.Requetes);
            Assert.Equal(HttpMethod.Post, r.Methode);
            Assert.Equal("/api/travaux", r.Chemin);
            Assert.StartsWith("multipart/form-data; boundary=EKO-", r.TypeContenu);
            Assert.Equal(r.Corps.Length, r.Longueur);
            var texte = r.CorpsTexte;
            var ordre = new[] { "name=\"reglage\"", "name=\"client\"", "name=\"commande\"", "name=\"quantite\"", "name=\"fichier\"", "%PDF-test" }
                .Select(s => texte.IndexOf(s, StringComparison.Ordinal)).ToList();
            Assert.All(ordre, i => Assert.True(i >= 0));
            Assert.Equal(ordre.OrderBy(i => i), ordre);
            Assert.Equal(1.0, valeurs[^1]);
        }
        finally
        {
            File.Delete(chemin);
        }
    }

    [Fact]
    public async Task Envoi_d_un_fichier_absent()
    {
        var g = new GestionnaireFictif(_ => GestionnaireFictif.Json("{}"));
        using var api = Fabriquer(g);
        var e = await Assert.ThrowsAsync<ErreurApi>(() => api.Envoyer("/chemin/qui/n/existe/pas.pdf", Array.Empty<(string, string)>()));
        Assert.Equal("fichier_illisible", e.Code);
        Assert.Empty(g.Requetes);
    }

    [Fact]
    public async Task Envoi_refuse_par_le_serveur()
    {
        var chemin = Path.Combine(Path.GetTempPath(), $"eko-test-{Guid.NewGuid():N}.pdf");
        await File.WriteAllBytesAsync(chemin, new byte[] { 1, 2, 3 });
        try
        {
            var g = new GestionnaireFictif(_ => GestionnaireFictif.Json("""{"code":"format_inconnu","detail":null}""", HttpStatusCode.BadRequest));
            using var api = Fabriquer(g);
            var e = await Assert.ThrowsAsync<ErreurApi>(() => api.Envoyer(chemin, Array.Empty<(string, string)>()));
            Assert.Equal("format_inconnu", e.Code);
        }
        finally
        {
            File.Delete(chemin);
        }
    }

    [Fact]
    public async Task Epreuve_telechargee_en_flux_dans_un_fichier()
    {
        var octets = Enumerable.Range(0, 500_000).Select(i => (byte)(i % 200)).ToArray();
        var g = new GestionnaireFictif(_ => new HttpResponseMessage(HttpStatusCode.OK) { Content = new ByteArrayContent(octets) });
        using var api = Fabriquer(g);
        var destination = Path.Combine(Path.GetTempPath(), $"eko-test-{Guid.NewGuid():N}.tif");
        try
        {
            await api.TelechargerEpreuve("t1", 2, destination);
            Assert.Equal(octets, await File.ReadAllBytesAsync(destination));
            Assert.Equal("/api/travaux/t1/pages/2/epreuve.tif", g.Requetes[0].Chemin);
        }
        finally
        {
            File.Delete(destination);
        }
    }

    [Fact]
    public async Task Epreuve_en_erreur_ne_laisse_pas_de_fichier()
    {
        var g = new GestionnaireFictif(_ => GestionnaireFictif.Json("""{"code":"epreuve_echouee","detail":"gs"}""", HttpStatusCode.InternalServerError));
        using var api = Fabriquer(g);
        var destination = Path.Combine(Path.GetTempPath(), $"eko-test-{Guid.NewGuid():N}.tif");
        var e = await Assert.ThrowsAsync<ErreurApi>(() => api.TelechargerEpreuve("t1", 1, destination));
        Assert.Equal("L'épreuve TIFF n'a pas pu être faite. (gs)", e.Message);
        Assert.False(File.Exists(destination));
    }

    [Fact]
    public async Task Apercu_rend_les_octets_du_PNG()
    {
        var png = new byte[] { 0x89, 0x50, 0x4E, 0x47 };
        var g = new GestionnaireFictif(_ => new HttpResponseMessage(HttpStatusCode.OK) { Content = new ByteArrayContent(png) });
        using var api = Fabriquer(g);
        Assert.Equal(png, await api.Apercu("t1", 3));
        Assert.Equal("/api/travaux/t1/pages/3/apercu.png", g.Requetes[0].Chemin);
    }

    [Fact]
    public void Le_cookie_de_session_est_retrouve_dans_le_conteneur()
    {
        using var api = new Api("http://rip.local:8640", new GestionnaireFictif(_ => GestionnaireFictif.Json("{}")));
        Assert.Null(api.CookieSession());
        api.Cookies.Add(new Uri("http://rip.local:8640/"), new Cookie("eko_rip_session", "abc123"));
        Assert.Equal("abc123", api.CookieSession()?.Value);
    }

    private sealed class ProgressionSynchrone : IProgress<double>
    {
        private readonly Action<double> _action;
        public ProgressionSynchrone(Action<double> action) => _action = action;
        public void Report(double value) => _action(value);
    }
}
