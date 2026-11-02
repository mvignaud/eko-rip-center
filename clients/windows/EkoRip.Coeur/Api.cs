using System.Net;
using System.Net.Http.Headers;
using System.Text;
using System.Text.Json;
using System.Text.RegularExpressions;

namespace EkoRip.Coeur;

/// <summary>
/// Client de l'API JSON du serveur. Un <see cref="HttpClient"/> et un <see cref="CookieContainer"/>
/// à lui : <c>eko_rip_session</c> reste dans cette instance, rien n'est partagé entre deux
/// serveurs (changer d'adresse crée une nouvelle <see cref="Api"/>). Les méthodes lèvent
/// <see cref="ErreurApi"/> (réseau : statut 0) ou <see cref="OperationCanceledException"/>.
/// </summary>
public sealed class Api : IDisposable
{
    public const string NomCookieSession = "eko_rip_session";

    /// <summary>Délai des requêtes courtes.</summary>
    public static readonly TimeSpan DelaiCourt = TimeSpan.FromSeconds(60);

    /// <summary>Délai des envois de fichiers et des épreuves TIFF (fabriquées à la demande).</summary>
    public static readonly TimeSpan DelaiLong = TimeSpan.FromMinutes(60);

    private readonly HttpClient _http;

    /// <summary>Cookies de la session de ce serveur.</summary>
    public CookieContainer Cookies { get; }

    /// <summary>Adresse du serveur sans barre finale : <c>http://hôte:8640</c>.</summary>
    public string Racine { get; }

    /// <param name="racine">Racine déjà normalisée (voir <see cref="Normaliser"/>).</param>
    /// <param name="gestionnaire">Pour les tests : remplace le gestionnaire réseau (sans cookies).</param>
    public Api(string racine, HttpMessageHandler? gestionnaire = null)
    {
        Racine = racine.TrimEnd('/');
        Cookies = new CookieContainer();
        gestionnaire ??= new HttpClientHandler
        {
            CookieContainer = Cookies,
            UseCookies = true,
        };
        // Les délais sont gérés requête par requête (voir Executer).
        _http = new HttpClient(gestionnaire, disposeHandler: true) { Timeout = Timeout.InfiniteTimeSpan };
    }

    public void Dispose() => _http.Dispose();

    // ------------------------------------------------------------ Adresse

    /// <summary>
    /// Adresse saisie (<c>hôte</c>, <c>hôte:8640</c>, <c>http://hôte:8640/</c>) vers la racine du
    /// serveur. Port 8640 par défaut, http par défaut (le serveur est en HTTP sur le réseau local).
    /// </summary>
    public static string? Normaliser(string? saisie)
    {
        var texte = (saisie ?? "").Trim();
        if (texte.Length == 0) return null;
        if (!texte.Contains("://", StringComparison.Ordinal)) texte = "http://" + texte;
        if (!Uri.TryCreate(texte, UriKind.Absolute, out var uri)) return null;
        if (uri.Scheme != Uri.UriSchemeHttp && uri.Scheme != Uri.UriSchemeHttps) return null;
        if (string.IsNullOrEmpty(uri.Host)) return null;

        // Uri ne dit pas si le port a été écrit : on le cherche dans la partie « autorité ».
        var reste = texte[(texte.IndexOf("://", StringComparison.Ordinal) + 3)..];
        var fin = reste.IndexOfAny(new[] { '/', '?', '#' });
        var autorite = fin < 0 ? reste : reste[..fin];
        var arobase = autorite.LastIndexOf('@');
        if (arobase >= 0) autorite = autorite[(arobase + 1)..];
        var portEcrit = Regex.IsMatch(autorite, @":\d+$");

        int? port = portEcrit ? uri.Port : (uri.Scheme == Uri.UriSchemeHttp ? 8640 : null);
        return $"{uri.Scheme}://{uri.Host}" + (port is { } p ? $":{p}" : "");
    }

    /// <summary>Forme courte pour l'affichage et la mémorisation : sans « http:// ».</summary>
    public static string Affichable(string racine) =>
        racine.StartsWith("http://", StringComparison.Ordinal) ? racine["http://".Length..] : racine;

    // ------------------------------------------------------------ Requêtes

    private HttpRequestMessage Requete(HttpMethod methode, string chemin, object? json = null)
    {
        if (!Uri.TryCreate(Racine + chemin, UriKind.Absolute, out var url))
            throw new ErreurApi(0, "adresse_invalide", null);
        var r = new HttpRequestMessage(methode, url);
        r.Headers.TryAddWithoutValidation("User-Agent", VersionClient.Agent);
        r.Headers.Accept.Add(new MediaTypeWithQualityHeaderValue("application/json"));
        if (json is not null)
            r.Content = new StringContent(JsonSerializer.Serialize(json), Encoding.UTF8, "application/json");
        return r;
    }

    /// <summary>Envoie la requête avec son délai ; les échecs réseau deviennent des <see cref="ErreurApi"/>.</summary>
    private async Task<HttpResponseMessage> Envoyer(HttpRequestMessage r, HttpCompletionOption option, TimeSpan delai, CancellationToken jeton)
    {
        using var delaiCts = CancellationTokenSource.CreateLinkedTokenSource(jeton);
        delaiCts.CancelAfter(delai);
        try
        {
            return await _http.SendAsync(r, option, delaiCts.Token).ConfigureAwait(false);
        }
        catch (OperationCanceledException) when (jeton.IsCancellationRequested)
        {
            throw;
        }
        catch (OperationCanceledException)
        {
            throw new ErreurApi(0, "reseau", "Délai dépassé.");
        }
        catch (HttpRequestException e)
        {
            throw ErreurApi.Reseau(e);
        }
        catch (IOException e)
        {
            throw ErreurApi.Reseau(e);
        }
    }

    /// <summary>Corps de la réponse ; 4xx/5xx → <see cref="ErreurApi"/>.</summary>
    private async Task<byte[]> Executer(HttpRequestMessage r, TimeSpan delai, CancellationToken jeton)
    {
        using var reponse = await Envoyer(r, HttpCompletionOption.ResponseContentRead, delai, jeton).ConfigureAwait(false);
        byte[] corps;
        try
        {
            corps = await reponse.Content.ReadAsByteArrayAsync(jeton).ConfigureAwait(false);
        }
        catch (OperationCanceledException) when (jeton.IsCancellationRequested)
        {
            throw;
        }
        catch (Exception e) when (e is HttpRequestException or IOException)
        {
            throw ErreurApi.Reseau(e);
        }
        if (!reponse.IsSuccessStatusCode) throw ErreurApi.Depuis((int)reponse.StatusCode, corps);
        return corps;
    }

    private static T Decoder<T>(byte[] donnees)
    {
        try
        {
            return JsonServeur.Lire<T>(donnees)
                ?? throw new ErreurApi(0, "reponse_illisible", "réponse vide");
        }
        catch (JsonException e)
        {
            throw new ErreurApi(0, "reponse_illisible", e.Message);
        }
    }

    private async Task<T> Lire<T>(string chemin, CancellationToken jeton)
    {
        using var r = Requete(HttpMethod.Get, chemin);
        return Decoder<T>(await Executer(r, DelaiCourt, jeton).ConfigureAwait(false));
    }

    private async Task Ecrire(HttpMethod methode, string chemin, object? corps, CancellationToken jeton)
    {
        using var r = Requete(methode, chemin, corps);
        await Executer(r, DelaiCourt, jeton).ConfigureAwait(false);
    }

    // ------------------------------------------------------------ Session

    public Task<InfoSession> LireSession(CancellationToken jeton = default) => Lire<InfoSession>("/api/session", jeton);

    public Task OuvrirSession(string nom, string motDePasse, CancellationToken jeton = default) =>
        Ecrire(HttpMethod.Post, "/api/session", new Dictionary<string, string> { ["nom"] = nom, ["mot_de_passe"] = motDePasse }, jeton);

    public Task FermerSession(CancellationToken jeton = default) => Ecrire(HttpMethod.Delete, "/api/session", null, jeton);

    /// <summary>Valeur du cookie de session, à recopier dans une fenêtre web (Administration).</summary>
    public Cookie? CookieSession()
    {
        if (!Uri.TryCreate(Racine + "/", UriKind.Absolute, out var url)) return null;
        return Cookies.GetCookies(url).Cast<Cookie>().FirstOrDefault(c => c.Name == NomCookieSession);
    }

    // ------------------------------------------------------------ Lectures

    public Task<List<Travail>> Travaux(CancellationToken jeton = default) => Lire<List<Travail>>("/api/travaux", jeton);
    public Task<List<Sortie>> File(CancellationToken jeton = default) => Lire<List<Sortie>>("/api/file", jeton);
    public Task<List<Machine>> Machines(CancellationToken jeton = default) => Lire<List<Machine>>("/api/machines", jeton);
    public Task<InfoSysteme> Systeme(CancellationToken jeton = default) => Lire<InfoSysteme>("/api/systeme", jeton);
    public Task<List<Reglage>> Reglages(CancellationToken jeton = default) => Lire<List<Reglage>>("/api/reglages", jeton);

    // ------------------------------------------------------------ Actions

    /// <summary>
    /// Met une page en file. <paramref name="operation"/> (<c>impression</c> ou <c>decoupe</c>) évite
    /// l'ambiguïté sur une machine qui fait les deux.
    /// </summary>
    public Task MettreEnFile(string travail, int page, string machine, string operation, CancellationToken jeton = default) =>
        Ecrire(HttpMethod.Post, "/api/file", new Dictionary<string, object> { ["travail"] = travail, ["page"] = page, ["machine"] = machine, ["operation"] = operation }, jeton);

    public Task Relancer(string travail, CancellationToken jeton = default) =>
        Ecrire(HttpMethod.Post, $"/api/travaux/{travail}/relancer", null, jeton);

    public Task Supprimer(string travail, CancellationToken jeton = default) =>
        Ecrire(HttpMethod.Delete, $"/api/travaux/{travail}", null, jeton);

    public Task Agir(string sortie, ActionSortie action, CancellationToken jeton = default) =>
        action == ActionSortie.Retirer
            ? Ecrire(HttpMethod.Delete, $"/api/file/{sortie}", null, jeton)
            : Ecrire(HttpMethod.Post, $"/api/file/{sortie}/{action.Brut()}", null, jeton);

    // ------------------------------------------------------------ Aperçu et épreuve

    /// <summary>PNG de la page (numérotée à partir de 1), avec le cookie de la session.</summary>
    public async Task<byte[]> Apercu(string travail, int page, CancellationToken jeton = default)
    {
        using var r = Requete(HttpMethod.Get, $"/api/travaux/{travail}/pages/{page}/apercu.png");
        return await Executer(r, DelaiCourt, jeton).ConfigureAwait(false);
    }

    /// <summary>
    /// Télécharge l'épreuve TIFF dans <paramref name="destination"/>, en flux, sans la tenir en
    /// mémoire. Le serveur la fabrique parfois à la demande : délai large. Le fichier partiel est
    /// supprimé en cas d'échec.
    /// </summary>
    public async Task TelechargerEpreuve(string travail, int page, string destination, IProgress<double>? progression = null, CancellationToken jeton = default)
    {
        using var r = Requete(HttpMethod.Get, $"/api/travaux/{travail}/pages/{page}/epreuve.tif");
        using var reponse = await Envoyer(r, HttpCompletionOption.ResponseHeadersRead, DelaiLong, jeton).ConfigureAwait(false);
        try
        {
            if (!reponse.IsSuccessStatusCode)
            {
                var corps = await reponse.Content.ReadAsByteArrayAsync(jeton).ConfigureAwait(false);
                throw ErreurApi.Depuis((int)reponse.StatusCode, corps);
            }
            var total = reponse.Content.Headers.ContentLength;
            await using var source = await reponse.Content.ReadAsStreamAsync(jeton).ConfigureAwait(false);
            await using var sortie = new FileStream(destination, FileMode.Create, FileAccess.Write, FileShare.None, 81920, useAsync: true);
            var bloc = new byte[256 * 1024];
            long recu = 0;
            int lu;
            while ((lu = await source.ReadAsync(bloc, jeton).ConfigureAwait(false)) > 0)
            {
                await sortie.WriteAsync(bloc.AsMemory(0, lu), jeton).ConfigureAwait(false);
                recu += lu;
                if (total is > 0) progression?.Report(Math.Min(1.0, (double)recu / total.Value));
            }
        }
        catch (Exception e) when (e is IOException or HttpRequestException)
        {
            SupprimerSiExiste(destination);
            throw ErreurApi.Reseau(e);
        }
        catch
        {
            SupprimerSiExiste(destination);
            throw;
        }
    }

    private static void SupprimerSiExiste(string chemin)
    {
        try { System.IO.File.Delete(chemin); } catch (IOException) { } catch (UnauthorizedAccessException) { }
    }

    // ------------------------------------------------------------ Envoi d'un fichier

    /// <summary>
    /// <c>POST /api/travaux</c> en multipart, envoyé en flux depuis le disque : le fichier n'est
    /// jamais entier en mémoire. Les champs (<c>reglage</c>, <c>client</c>, <c>commande</c>,
    /// <c>quantite</c>) partent avant le fichier. <paramref name="progression"/> reçoit une fraction de 0 à 1.
    /// </summary>
    public async Task<Travail> Envoyer(string fichier, IEnumerable<(string Nom, string Valeur)> champs, IProgress<double>? progression = null, CancellationToken jeton = default)
    {
        using var r = Requete(HttpMethod.Post, "/api/travaux");
        FileStream flux;
        long longueur;
        try
        {
            flux = new FileStream(fichier, FileMode.Open, FileAccess.Read, FileShare.Read, 81920, useAsync: true);
            longueur = flux.Length;
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException)
        {
            throw new ErreurApi(0, "fichier_illisible", e.Message);
        }

        var frontiere = "EKO-" + Guid.NewGuid().ToString("N");
        // Le contenu ferme le flux à sa destruction (avec la requête).
        r.Content = new ContenuMultipart(champs, Path.GetFileName(fichier), flux, longueur, frontiere, progression);
        return Decoder<Travail>(await ExecuterEnvoi(r, jeton).ConfigureAwait(false));
    }

    private async Task<byte[]> ExecuterEnvoi(HttpRequestMessage r, CancellationToken jeton)
    {
        using var reponse = await Envoyer(r, HttpCompletionOption.ResponseContentRead, DelaiLong, jeton).ConfigureAwait(false);
        var corps = await reponse.Content.ReadAsByteArrayAsync(jeton).ConfigureAwait(false);
        if (!reponse.IsSuccessStatusCode) throw ErreurApi.Depuis((int)reponse.StatusCode, corps);
        return corps;
    }
}
