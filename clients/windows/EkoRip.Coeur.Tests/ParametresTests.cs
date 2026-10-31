namespace EkoRip.Coeur.Tests;

public class ParametresTests
{
    [Fact]
    public void Aller_retour_dans_un_fichier()
    {
        var dossier = Path.Combine(Path.GetTempPath(), $"eko-test-{Guid.NewGuid():N}");
        var chemin = Path.Combine(dossier, "sous", "parametres.json");
        try
        {
            new Parametres { Adresse = "rip:8640", DernierUtilisateur = "marie", DernierReglage = "Vinyle" }.Enregistrer(chemin);
            var lu = Parametres.Charger(chemin);
            Assert.Equal("rip:8640", lu.Adresse);
            Assert.Equal("marie", lu.DernierUtilisateur);
            Assert.Equal("Vinyle", lu.DernierReglage);
        }
        finally
        {
            if (Directory.Exists(dossier)) Directory.Delete(dossier, true);
        }
    }

    [Fact]
    public void Fichier_absent_ou_illisible_donne_des_reglages_vides()
    {
        Assert.Equal("", Parametres.Charger("/chemin/inexistant/p.json").Adresse);
        var chemin = Path.Combine(Path.GetTempPath(), $"eko-test-{Guid.NewGuid():N}.json");
        File.WriteAllText(chemin, "{ pas du json");
        try { Assert.Equal("", Parametres.Charger(chemin).Adresse); }
        finally { File.Delete(chemin); }
    }
}
