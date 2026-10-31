using System.Text.Json;

namespace EkoRip.Coeur;

/// <summary>Version du client et signature envoyée au serveur.</summary>
public static class VersionClient
{
    /// <summary>Numéro de version ; l'application peut le remplacer au démarrage.</summary>
    public static string Numero { get; set; } = LireVersion();

    /// <summary>Sans cette signature, le serveur traite le poste comme un navigateur (<c>client_requis</c>).</summary>
    public static string Agent => $"EKO-RIP-Client/{Numero} (Windows)";

    private static string LireVersion()
    {
        var v = typeof(VersionClient).Assembly.GetName().Version;
        return v is null ? "dev" : $"{v.Major}.{v.Minor}.{v.Build}";
    }
}

/// <summary>Erreur renvoyée par le serveur (<c>{"code","detail"}</c>) ou née du réseau (statut 0).</summary>
public sealed class ErreurApi : Exception
{
    public int Statut { get; }
    public string Code { get; }
    public string? Detail { get; }

    public ErreurApi(int statut, string code, string? detail)
        : base(Libelles.Erreur(code, detail))
    {
        Statut = statut;
        Code = code;
        Detail = detail;
    }

    /// <summary>Le serveur demande de se connecter (cookie absent ou expiré).</summary>
    public bool ConnexionRequise => Statut == 401 && Code != "identifiants_invalides";

    public static ErreurApi Reseau(Exception e) => new(0, "reseau", e.Message);

    public static ErreurApi Depuis(int statut, byte[] corps)
    {
        try
        {
            using var doc = JsonDocument.Parse(corps);
            if (doc.RootElement.ValueKind == JsonValueKind.Object
                && doc.RootElement.TryGetProperty("code", out var c) && c.ValueKind == JsonValueKind.String)
            {
                string? detail = null;
                if (doc.RootElement.TryGetProperty("detail", out var d) && d.ValueKind == JsonValueKind.String)
                    detail = d.GetString();
                return new ErreurApi(statut, c.GetString() ?? "erreur_http", detail);
            }
        }
        catch (JsonException)
        {
            // Corps absent ou pas du JSON du serveur : voir plus bas.
        }
        // Proxy, limite de taille, identifiant mal formé…
        if (statut == 413) return new ErreurApi(statut, "trop_gros", null);
        return new ErreurApi(statut, "erreur_http", $"HTTP {statut}");
    }
}
