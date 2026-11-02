namespace EkoRip.Coeur.Tests;

public class AdresseTests
{
    [Theory]
    [InlineData("192.168.10.109", "http://192.168.10.109:8640")]
    [InlineData("rip.local:9000/", "http://rip.local:9000")]
    [InlineData("  rip.local  ", "http://rip.local:8640")]
    [InlineData("http://rip.local:8640/api/travaux?x=1", "http://rip.local:8640")]
    [InlineData("https://rip.e-ko.fr", "https://rip.e-ko.fr")]
    [InlineData("https://rip.e-ko.fr:8443/", "https://rip.e-ko.fr:8443")]
    [InlineData("http://utilisateur:secret@rip.local", "http://rip.local:8640")]
    public void Normaliser_ajoute_http_et_le_port_par_defaut(string saisie, string attendu)
    {
        Assert.Equal(attendu, Api.Normaliser(saisie));
    }

    [Theory]
    [InlineData("")]
    [InlineData("   ")]
    [InlineData(null)]
    [InlineData("ftp://rip.local")]
    [InlineData("http://")]
    [InlineData("rip.local:abc")]
    public void Normaliser_refuse_les_adresses_inutilisables(string? saisie)
    {
        Assert.Null(Api.Normaliser(saisie));
    }

    [Fact]
    public void Affichable_retire_http_seulement()
    {
        Assert.Equal("h:8640", Api.Affichable("http://h:8640"));
        Assert.Equal("https://h", Api.Affichable("https://h"));
    }

    [Fact]
    public void Agent_porte_la_signature_du_client_Windows()
    {
        Assert.StartsWith("EKO-RIP-Client/", VersionClient.Agent);
        Assert.EndsWith(" (Windows)", VersionClient.Agent);
    }
}
