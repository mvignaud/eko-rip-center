using System.Net;
using System.Text;

namespace EkoRip.Coeur.Tests;

/// <summary>Une requête reçue par le faux serveur, avec son corps déjà lu.</summary>
public sealed record RequeteVue(HttpMethod Methode, string Chemin, string UserAgent, string? TypeContenu, long? Longueur, byte[] Corps)
{
    public string CorpsTexte => Encoding.UTF8.GetString(Corps);
}

/// <summary>Faux réseau : répond avec une fonction et garde les requêtes reçues.</summary>
public sealed class GestionnaireFictif : HttpMessageHandler
{
    private readonly Func<RequeteVue, HttpResponseMessage> _reponse;
    public List<RequeteVue> Requetes { get; } = new();

    public GestionnaireFictif(Func<RequeteVue, HttpResponseMessage> reponse) => _reponse = reponse;

    protected override async Task<HttpResponseMessage> SendAsync(HttpRequestMessage requete, CancellationToken jeton)
    {
        var corps = requete.Content is null ? Array.Empty<byte>() : await requete.Content.ReadAsByteArrayAsync(jeton);
        var vue = new RequeteVue(
            requete.Method,
            requete.RequestUri!.AbsolutePath,
            requete.Headers.UserAgent.ToString(),
            requete.Content?.Headers.ContentType?.ToString(),
            requete.Content?.Headers.ContentLength,
            corps);
        lock (Requetes) Requetes.Add(vue);
        return _reponse(vue);
    }

    public static HttpResponseMessage Json(string json, HttpStatusCode statut = HttpStatusCode.OK) =>
        new(statut) { Content = new StringContent(json, Encoding.UTF8, "application/json") };
}

public static class Reponses
{
    public static string Lire(string nom) =>
        File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "Reponses", nom), Encoding.UTF8);
}
