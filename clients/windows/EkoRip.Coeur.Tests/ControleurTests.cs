using System.Net;

namespace EkoRip.Coeur.Tests;

/// <summary>Parcours complet du contrôleur contre un faux serveur.</summary>
public class ControleurTests
{
    private const string Travail1 = """{"id":"t1","nom":"affiche.pdf","statut":"termine","cree_le":10,"pages":[{"largeur_mm":100,"hauteur_mm":100}]}""";

    private static HttpResponseMessage Repondre(RequeteVue r, bool libre, ref bool connecte, string travaux) => r.Chemin switch
    {
        "/api/session" when r.Methode == HttpMethod.Get =>
            GestionnaireFictif.Json(libre ? """{"acces_libre":true,"utilisateur":null}""" : connecte ? """{"acces_libre":false,"utilisateur":"marie"}""" : """{"acces_libre":false,"utilisateur":null}"""),
        "/api/session" => GestionnaireFictif.Json("{}"),
        "/api/travaux" => GestionnaireFictif.Json(travaux),
        "/api/file" => GestionnaireFictif.Json("[]"),
        "/api/machines" => GestionnaireFictif.Json("""[{"id":"epson","nom":"Epson","modele":"epson_sc_s80600"}]"""),
        "/api/reglages" => GestionnaireFictif.Json("""[{"nom":"Vinyle"}]"""),
        _ => GestionnaireFictif.Json("""{"code":"introuvable","detail":null}""", HttpStatusCode.NotFound),
    };

    private static (Controleur, GestionnaireFictif, Parametres, List<Parametres>) Fabriquer(Func<RequeteVue, HttpResponseMessage> reponse, string adresse = "")
    {
        var g = new GestionnaireFictif(reponse);
        var p = new Parametres { Adresse = adresse };
        var sauvegardes = new List<Parametres>();
        var c = new Controleur(p, x => sauvegardes.Add(x), racine => new Api(racine, g), TimeSpan.FromMilliseconds(40));
        return (c, g, p, sauvegardes);
    }

    [Fact]
    public async Task Sans_adresse_retenue_on_demande_l_adresse()
    {
        var (c, _, _, _) = Fabriquer(_ => GestionnaireFictif.Json("{}"));
        await c.Demarrer();
        Assert.Equal(Phase.Adresse, c.Phase);
        c.Dispose();
    }

    [Fact]
    public async Task Adresse_retenue_et_acces_libre_entrent_directement()
    {
        var connecte = false;
        var (c, g, p, sauvegardes) = Fabriquer(r => Repondre(r, true, ref connecte, "[" + Travail1 + "]"), "rip.local:8640");
        await c.Demarrer();
        Assert.Equal(Phase.Connecte, c.Phase);
        Assert.Equal("rip.local:8640", c.Adresse);
        Assert.Equal("t1", Assert.Single(c.Travaux).Id);
        Assert.Equal("Vinyle", Assert.Single(c.Reglages).Nom);
        Assert.Equal("Epson", Assert.Single(c.Machines).Nom);
        Assert.NotEmpty(sauvegardes);
        Assert.Equal("rip.local:8640", p.Adresse);
        Assert.All(g.Requetes, r => Assert.StartsWith("EKO-RIP-Client/", r.UserAgent));
        c.Dispose();
    }

    [Fact]
    public async Task Serveur_sans_acces_libre_demande_l_identifiant_puis_entre()
    {
        var connecte = false;
        var (c, _, p, _) = Fabriquer(r =>
        {
            if (r.Chemin == "/api/session" && r.Methode == HttpMethod.Post) connecte = true;
            return Repondre(r, false, ref connecte, "[]");
        });
        await c.Connecter("rip.local");
        Assert.Equal(Phase.Connexion, c.Phase);
        Assert.Equal("rip.local:8640", p.Adresse);

        await c.OuvrirSession("marie", "secret");
        Assert.Equal(Phase.Connecte, c.Phase);
        Assert.Equal("marie", c.Session?.Utilisateur);
        Assert.Equal("marie", p.DernierUtilisateur);
        c.Dispose();
    }

    [Fact]
    public async Task Adresse_invalide_reste_sur_l_ecran_d_adresse()
    {
        var (c, _, _, _) = Fabriquer(_ => GestionnaireFictif.Json("{}"));
        await c.Connecter("ftp://x");
        Assert.Equal(Phase.Adresse, c.Phase);
        Assert.Equal("Adresse du serveur invalide : hôte:8640.", c.ErreurConnexion);
        c.Dispose();
    }

    [Fact]
    public async Task Serveur_injoignable_revient_a_l_adresse_avec_le_message()
    {
        var (c, _, _, _) = Fabriquer(_ => throw new HttpRequestException("refusé"));
        await c.Connecter("rip.local");
        Assert.Equal(Phase.Adresse, c.Phase);
        Assert.Equal("Serveur injoignable. (refusé)", c.ErreurConnexion);
        c.Dispose();
    }

    [Fact]
    public async Task Un_401_pendant_l_actualisation_ramene_a_la_connexion()
    {
        var expire = false;
        var (c, _, _, _) = Fabriquer(r =>
        {
            if (expire && r.Chemin == "/api/travaux")
                return GestionnaireFictif.Json("""{"code":"connexion_requise","detail":null}""", HttpStatusCode.Unauthorized);
            var inutile = false;
            return Repondre(r, true, ref inutile, "[" + Travail1 + "]");
        }, "rip.local");
        await c.Demarrer();
        Assert.Equal(Phase.Connecte, c.Phase);

        expire = true;
        await c.Rafraichir();
        Assert.Equal(Phase.Connexion, c.Phase);
        Assert.Equal("Session expirée : reconnectez-vous.", c.ErreurConnexion);
        Assert.Empty(c.Travaux);
        c.Dispose();
    }

    [Fact]
    public async Task Panne_pendant_l_actualisation_est_notee_sans_quitter()
    {
        var panne = false;
        var (c, _, _, _) = Fabriquer(r =>
        {
            if (panne && r.Chemin == "/api/travaux") throw new HttpRequestException("coupé");
            var inutile = false;
            return Repondre(r, true, ref inutile, "[" + Travail1 + "]");
        }, "rip.local");
        await c.Demarrer();
        panne = true;
        await c.Rafraichir();
        Assert.Equal(Phase.Connecte, c.Phase);
        Assert.Equal("Serveur injoignable. (coupé)", c.ServeurHorsLigne);
        Assert.Single(c.Travaux);
        panne = false;
        await c.Rafraichir();
        Assert.Null(c.ServeurHorsLigne);
        c.Dispose();
    }

    [Fact]
    public async Task Actions_mise_en_file_et_annonce()
    {
        var (c, g, _, _) = Fabriquer(r =>
        {
            if (r.Chemin == "/api/file" && r.Methode == HttpMethod.Post) return GestionnaireFictif.Json("{}", HttpStatusCode.Accepted);
            var inutile = false;
            return Repondre(r, true, ref inutile, "[" + Travail1 + "]");
        }, "rip.local");
        await c.Demarrer();
        await c.Imprimer(c.Travaux[0], 1, c.Machines[0]);
        var post = g.Requetes.Last(r => r.Methode == HttpMethod.Post && r.Chemin == "/api/file");
        Assert.Equal("""{"travail":"t1","page":1,"machine":"epson","operation":"impression"}""", post.CorpsTexte);
        Assert.Equal("Page 1 de « affiche.pdf » mise en file sur Epson.", c.Annonce);
        c.Dispose();
    }

    [Fact]
    public async Task Erreur_d_action_est_dite_a_l_operateur()
    {
        var (c, _, _, _) = Fabriquer(r =>
        {
            if (r.Chemin == "/api/travaux/t1/relancer") return GestionnaireFictif.Json("""{"code":"travail_occupe","detail":null}""", HttpStatusCode.Conflict);
            var inutile = false;
            return Repondre(r, true, ref inutile, "[" + Travail1 + "]");
        }, "rip.local");
        await c.Demarrer();
        await c.Relancer(c.Travaux[0]);
        Assert.Equal("Action impossible", c.Message?.Titre);
        Assert.Equal("Ce travail est en cours de rendu, ou une sortie l'attend dans la file.", c.Message?.Texte);
        c.FermerMessage();
        Assert.Null(c.Message);
        c.Dispose();
    }

    [Fact]
    public async Task Notification_quand_un_travail_passe_a_termine()
    {
        var statut = "en_cours";
        var (c, _, _, _) = Fabriquer(r =>
        {
            var inutile = false;
            return Repondre(r, true, ref inutile, $$"""[{"id":"t1","nom":"affiche.pdf","statut":"{{statut}}","cree_le":10}]""");
        }, "rip.local");
        var recues = new List<NotificationRip>();
        c.NotificationAEnvoyer += recues.Add;
        await c.Demarrer();
        Assert.Empty(recues);
        statut = "termine";
        await c.Rafraichir();
        Assert.Equal(new NotificationRip("Travail prêt", "affiche.pdf"), Assert.Single(recues));
        c.Dispose();
    }

    [Fact]
    public async Task Recherche_et_tri_dans_les_travaux_affiches()
    {
        var connecte = false;
        var json = """[{"id":"a","nom":"zebre.pdf","statut":"termine","cree_le":1},{"id":"b","nom":"abeille.pdf","statut":"termine","cree_le":2}]""";
        var (c, _, _, _) = Fabriquer(r => Repondre(r, true, ref connecte, json), "rip.local");
        await c.Demarrer();
        Assert.Equal(new[] { "b", "a" }, c.TravauxAffiches.Select(t => t.Id));
        c.Trier(ColonneTri.Nom);
        Assert.Equal(new[] { "b", "a" }, c.TravauxAffiches.Select(t => t.Id));
        c.Trier(ColonneTri.Nom);
        Assert.Equal(new[] { "a", "b" }, c.TravauxAffiches.Select(t => t.Id));
        c.ChoisirRecherche("ZEB");
        Assert.Equal(new[] { "a" }, c.TravauxAffiches.Select(t => t.Id));
        c.Dispose();
    }

    [Fact]
    public async Task Selection_page_et_suppression_de_la_selection_disparue()
    {
        var connecte = false;
        var json = """[{"id":"a","nom":"a.pdf","statut":"termine","cree_le":1,"pages":[{},{},{}]}]""";
        var (c, _, _, _) = Fabriquer(r => Repondre(r, true, ref connecte, json), "rip.local");
        await c.Demarrer();
        c.Selectionner("a");
        c.ChoisirPage(3);
        Assert.Equal(3, c.Page);
        c.ChoisirPage(9);
        Assert.Equal(3, c.Page);
        c.Selectionner(null);
        Assert.Equal(1, c.Page);
        c.Dispose();
    }

    [Fact]
    public async Task Proposer_envoi_filtre_les_extensions()
    {
        var (c, _, _, _) = Fabriquer(_ => GestionnaireFictif.Json("{}"));
        IReadOnlyList<string>? proposes = null;
        c.FichiersProposes += l => proposes = l;
        c.ProposerEnvoi(new[] { "a.pdf", "b.txt" });
        Assert.Equal(new[] { "a.pdf" }, proposes);

        proposes = null;
        c.ProposerEnvoi(new[] { "b.txt" });
        Assert.Null(proposes);
        Assert.Equal("Envoi impossible", c.Message?.Titre);
        Assert.StartsWith("Format non reconnu", c.Message?.Texte);
        await Task.CompletedTask;
        c.Dispose();
    }
}
