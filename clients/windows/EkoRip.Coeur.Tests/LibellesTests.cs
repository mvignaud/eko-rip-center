namespace EkoRip.Coeur.Tests;

public class LibellesTests
{
    [Fact]
    public void Code_connu_donne_le_libelle_francais()
    {
        Assert.Equal("Connexion requise.", Libelles.Erreur("connexion_requise", null));
        Assert.Equal("Utilisateur ou mot de passe incorrect.", Libelles.Erreur("identifiants_invalides", ""));
    }

    [Fact]
    public void Detail_ajoute_entre_parentheses()
    {
        Assert.Equal("Le rendu a échoué. (gs)", Libelles.Erreur("rendu_echoue", "gs"));
    }

    [Fact]
    public void Code_inconnu_rendu_tel_quel_avec_son_detail()
    {
        Assert.Equal("code_inconnu (détail)", Libelles.Erreur("code_inconnu", "détail"));
        Assert.Equal("code_inconnu", Libelles.Erreur("code_inconnu", null));
    }

    [Fact]
    public void Code_absent_donne_erreur_du_serveur()
    {
        Assert.Equal("Erreur du serveur.", Libelles.Erreur(null, null));
    }

    [Fact]
    public void Erreurs_nees_dans_le_client()
    {
        Assert.Equal("Serveur injoignable.", Libelles.Erreur("reseau", null));
        Assert.Equal("Réponse inattendue du serveur. (HTTP 502)", Libelles.Erreur("erreur_http", "HTTP 502"));
        Assert.Equal("Adresse du serveur invalide : hôte:8640.", Libelles.Erreur("adresse_invalide", null));
        Assert.Equal("Fichier illisible.", Libelles.Erreur("fichier_illisible", null));
    }

    [Fact]
    public void La_table_reprend_les_125_codes_du_serveur()
    {
        Assert.Equal(125, Libelles.Erreurs.Count);
        Assert.All(Libelles.Erreurs, e => Assert.False(string.IsNullOrWhiteSpace(e.Value)));
    }

    [Fact]
    public void Raisons_d_attente()
    {
        Assert.Equal("Retenue", Libelles.Raison("attente_operateur").Titre);
        Assert.Equal(("autre_code", ""), Libelles.Raison("autre_code"));
        Assert.Equal(9, Libelles.Raisons.Count);
    }

    [Theory]
    [InlineData("police_non_incorporee", "Helvetica", null, "Police non incorporée : Helvetica — une autre sera prise au rendu. Vectoriser le texte ou incorporer la police.")]
    [InlineData("image_basse_definition", "Im : 72 ppi", 1, "Image peu définie, page 1 : Im : 72 ppi (moins de 100 ppi à la taille imprimée).")]
    [InlineData("decoupe_traces_ouverts", "3", null, "Page ? : 3 tracé(s) ouvert(s) non coupé(s).")]
    [InlineData("code_futur", "x", null, "code_futur : x")]
    [InlineData("code_futur", null, null, "code_futur")]
    public void Alertes_de_pre_verification(string code, string? detail, int? page, string attendu)
    {
        Assert.Equal(attendu, Libelles.TexteAlerte(new Alerte { Code = code, Detail = detail, Page = page }));
    }

    [Fact]
    public void Modeles_de_machines()
    {
        Assert.Equal("Epson SC-S80600 (impression)", Libelles.Modele("epson_sc_s80600"));
        Assert.Equal("Roland VersaCAMM VP (impression et découpe)", Libelles.Modele("roland_versacamm"));
        Assert.Equal("nouveau", Libelles.Modele("nouveau"));
        Assert.Equal("Machine", Libelles.Modele(null));
    }

    [Fact]
    public void Libelles_de_statuts_et_d_etats()
    {
        Assert.Equal("Prêt", StatutTravail.Termine.Libelle());
        Assert.Equal("Échec", StatutTravail.Echec.Libelle());
        Assert.Equal("Terminée", EtatSortie.Terminee.Libelle());
        Assert.Equal("Retirer de la file", ActionSortie.Retirer.Libelle());
    }

    [Fact]
    public void Erreurs_de_l_API_decodees_depuis_le_corps()
    {
        var e = ErreurApi.Depuis(401, """{"code":"connexion_requise","detail":null}"""u8.ToArray());
        Assert.True(e.ConnexionRequise);
        Assert.Equal("Connexion requise.", e.Message);

        var mauvais = ErreurApi.Depuis(401, """{"code":"identifiants_invalides","detail":null}"""u8.ToArray());
        Assert.False(mauvais.ConnexionRequise);

        Assert.Equal("client_requis", ErreurApi.Depuis(403, System.Text.Encoding.UTF8.GetBytes(Reponses.Lire("erreur-client-requis.json"))).Code);
        Assert.Equal("introuvable", ErreurApi.Depuis(404, System.Text.Encoding.UTF8.GetBytes(Reponses.Lire("erreur-introuvable.json"))).Code);
    }

    [Fact]
    public void Corps_qui_n_est_pas_du_JSON_du_serveur()
    {
        var texte = System.Text.Encoding.UTF8.GetBytes(Reponses.Lire("erreur-422.txt"));
        var e = ErreurApi.Depuis(422, texte);
        Assert.Equal("erreur_http", e.Code);
        Assert.Equal("HTTP 422", e.Detail);
        Assert.Equal("trop_gros", ErreurApi.Depuis(413, Array.Empty<byte>()).Code);
        Assert.Equal("erreur_http", ErreurApi.Depuis(400, "Invalid URL"u8.ToArray()).Code);
    }
}
