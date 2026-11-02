using System.Text.Json;

namespace EkoRip.Coeur;

/// <summary>
/// Réglages retenus d'une session à l'autre : adresse du serveur, dernier utilisateur, dernier
/// réglage d'envoi. Fichier JSON dans le dossier de données locales de l'utilisateur.
/// </summary>
public sealed class Parametres
{
    public string Adresse { get; set; } = "";
    public string DernierUtilisateur { get; set; } = "";
    public string DernierReglage { get; set; } = "";

    private static readonly JsonSerializerOptions Options = new() { WriteIndented = true };

    /// <summary>Emplacement par défaut : <c>%LOCALAPPDATA%\E-KO RIP Center\parametres.json</c>.</summary>
    public static string CheminParDefaut() =>
        Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "E-KO RIP Center", "parametres.json");

    /// <summary>Lit le fichier ; absent ou illisible : réglages vides.</summary>
    public static Parametres Charger(string chemin)
    {
        try
        {
            if (!File.Exists(chemin)) return new Parametres();
            return JsonSerializer.Deserialize<Parametres>(File.ReadAllText(chemin), Options) ?? new Parametres();
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException or JsonException)
        {
            return new Parametres();
        }
    }

    /// <summary>Écrit le fichier ; un échec d'écriture n'est pas une erreur pour l'opérateur.</summary>
    public void Enregistrer(string chemin)
    {
        try
        {
            var dossier = Path.GetDirectoryName(chemin);
            if (!string.IsNullOrEmpty(dossier)) Directory.CreateDirectory(dossier);
            File.WriteAllText(chemin, JsonSerializer.Serialize(this, Options));
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException)
        {
            // Pas de dossier de données accessible : on repartira sans mémoire.
        }
    }
}
