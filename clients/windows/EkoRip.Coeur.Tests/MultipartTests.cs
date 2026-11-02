using System.Text;

namespace EkoRip.Coeur.Tests;

public class MultipartTests
{
    private static async Task<(string Texte, long? Longueur)> Fabriquer(
        IEnumerable<(string, string)> champs, byte[] fichier, string nom = "plan.pdf", IProgress<double>? progression = null)
    {
        using var flux = new MemoryStream(fichier);
        using var contenu = new ContenuMultipart(champs, nom, flux, fichier.Length, "XX", progression);
        var octets = await contenu.ReadAsByteArrayAsync();
        return (Encoding.UTF8.GetString(octets), contenu.Headers.ContentLength);
    }

    [Fact]
    public async Task Le_fichier_vient_en_dernier_apres_les_champs()
    {
        var (texte, _) = await Fabriquer(new[] { ("reglage", "Vinyle"), ("quantite", "3") }, Encoding.UTF8.GetBytes("%PDF-test"));
        var ordre = new[] { "name=\"reglage\"", "name=\"quantite\"", "name=\"fichier\"", "%PDF-test", "--XX--" }
            .Select(s => texte.IndexOf(s, StringComparison.Ordinal)).ToList();
        Assert.All(ordre, i => Assert.True(i >= 0));
        Assert.Equal(ordre.OrderBy(i => i), ordre);
        Assert.Contains("filename=\"plan.pdf\"", texte);
        Assert.EndsWith("\r\n--XX--\r\n", texte);
    }

    [Fact]
    public async Task La_longueur_annoncee_est_exacte()
    {
        var contenu = new byte[300_000];
        new Random(1).NextBytes(contenu);
        using var flux = new MemoryStream(contenu);
        using var c = new ContenuMultipart(new[] { ("client", "Mairie") }, "gros.tif", flux, contenu.Length, "XX");
        var octets = await c.ReadAsByteArrayAsync();
        Assert.Equal(octets.Length, c.Headers.ContentLength);
    }

    [Fact]
    public async Task Les_octets_du_fichier_sont_envoyes_tels_quels()
    {
        var contenu = Enumerable.Range(0, 700_000).Select(i => (byte)(i % 251)).ToArray();
        using var flux = new MemoryStream(contenu);
        using var c = new ContenuMultipart(Array.Empty<(string, string)>(), "x.bin", flux, contenu.Length, "XX");
        var octets = await c.ReadAsByteArrayAsync();
        var debut = Encoding.UTF8.GetByteCount("--XX\r\nContent-Disposition: form-data; name=\"fichier\"; filename=\"x.bin\"\r\nContent-Type: application/octet-stream\r\n\r\n");
        Assert.Equal(contenu, octets.Skip(debut).Take(contenu.Length).ToArray());
    }

    [Fact]
    public async Task Le_nom_de_fichier_est_assaini()
    {
        var (texte, _) = await Fabriquer(Array.Empty<(string, string)>(), new byte[] { 1 }, "a\"b\r\nc.pdf");
        Assert.Contains("filename=\"a%22bc.pdf\"", texte);
    }

    [Fact]
    public async Task La_progression_monte_jusqu_a_un()
    {
        var valeurs = new List<double>();
        var progression = new SynchroneProgress(valeurs.Add);
        await Fabriquer(new[] { ("a", "b") }, new byte[1_000_000], progression: progression);
        Assert.NotEmpty(valeurs);
        Assert.Equal(1.0, valeurs[^1]);
        Assert.Equal(valeurs.OrderBy(v => v), valeurs);
        Assert.True(valeurs.Count < 400, "au plus ~200 notifications");
    }

    private sealed class SynchroneProgress : IProgress<double>
    {
        private readonly Action<double> _action;
        public SynchroneProgress(Action<double> action) => _action = action;
        public void Report(double value) => _action(value);
    }
}
