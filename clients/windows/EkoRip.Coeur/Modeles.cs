using System.Globalization;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace EkoRip.Coeur;

// Formes JSON du serveur (serveur/src/main.rs, file.rs, machines.rs).
// Décodage en snake_case (« cree_le » devient CreeLe). Tout ce que le serveur peut omettre
// est facultatif ici, et les champs inconnus sont ignorés : le serveur en ajoute à chaque
// version. Les valeurs d'énumération inconnues donnent « Inconnu » au lieu d'une erreur.

/// <summary>Options de décodage des réponses de l'API.</summary>
public static class JsonServeur
{
    public static readonly JsonSerializerOptions Options = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
        PropertyNameCaseInsensitive = false,
    };

    public static T? Lire<T>(string json) => JsonSerializer.Deserialize<T>(json, Options);

    public static T? Lire<T>(byte[] json) => JsonSerializer.Deserialize<T>(json, Options);
}

/// <summary>Convertit une chaîne du serveur en énumération, « Inconnu » si elle n'est pas connue.</summary>
public abstract class ConvertisseurTolerant<T> : JsonConverter<T> where T : struct, Enum
{
    protected abstract T Depuis(string? brut);
    protected abstract string Vers(T valeur);

    public override T Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        if (reader.TokenType != JsonTokenType.String)
        {
            reader.Skip();
            return Depuis(null);
        }
        return Depuis(reader.GetString());
    }

    public override void Write(Utf8JsonWriter writer, T value, JsonSerializerOptions options) =>
        writer.WriteStringValue(Vers(value));
}

internal static class Texte
{
    public static string? NonVide(string? s) => string.IsNullOrEmpty(s) ? null : s;

    /// <summary>Culture française ; la culture invariante si le système n'a pas ses données (ICU absent).</summary>
    public static readonly CultureInfo Fr = Obtenir();

    private static CultureInfo Obtenir()
    {
        try { return CultureInfo.GetCultureInfo("fr-FR"); }
        catch (CultureNotFoundException) { return CultureInfo.InvariantCulture; }
    }
}

// ---------------------------------------------------------------- Travaux

[JsonConverter(typeof(ConvertisseurStatutTravail))]
public enum StatutTravail
{
    EnAttente,
    EnCours,
    Termine,
    Echec,
    /// <summary>Valeur ajoutée par un serveur plus récent.</summary>
    Inconnu,
}

public sealed class ConvertisseurStatutTravail : ConvertisseurTolerant<StatutTravail>
{
    protected override StatutTravail Depuis(string? brut) => brut switch
    {
        "en_attente" => StatutTravail.EnAttente,
        "en_cours" => StatutTravail.EnCours,
        "termine" => StatutTravail.Termine,
        "echec" => StatutTravail.Echec,
        _ => StatutTravail.Inconnu,
    };

    protected override string Vers(StatutTravail valeur) => valeur.Brut();
}

public static class StatutTravailExt
{
    public static string Brut(this StatutTravail s) => s switch
    {
        StatutTravail.EnAttente => "en_attente",
        StatutTravail.EnCours => "en_cours",
        StatutTravail.Termine => "termine",
        StatutTravail.Echec => "echec",
        _ => "inconnu",
    };

    public static string Libelle(this StatutTravail s) => s switch
    {
        StatutTravail.EnAttente => "En attente",
        StatutTravail.EnCours => "En cours",
        StatutTravail.Termine => "Prêt",
        StatutTravail.Echec => "Échec",
        _ => "Inconnu",
    };

    /// <summary>Rang pour le tri de la colonne Statut : ce qui travaille d'abord.</summary>
    public static int Rang(this StatutTravail s) => s switch
    {
        StatutTravail.EnCours => 0,
        StatutTravail.EnAttente => 1,
        StatutTravail.Echec => 2,
        StatutTravail.Termine => 3,
        _ => 4,
    };

    /// <summary>Le rendu est fini (réussi ou non).</summary>
    public static bool Fini(this StatutTravail s) => s is StatutTravail.Termine or StatutTravail.Echec;

    /// <summary>Le rendu travaille encore : le travail ne peut ni être relancé ni supprimé.</summary>
    public static bool Occupe(this StatutTravail s) => s is StatutTravail.EnAttente or StatutTravail.EnCours;
}

public sealed class Travail
{
    public string Id { get; init; } = "";
    public string Nom { get; init; } = "";
    public StatutTravail Statut { get; init; } = StatutTravail.Inconnu;
    public string? CodeErreur { get; init; }
    public string? Erreur { get; init; }
    public List<Page>? Pages { get; init; }
    public List<Alerte>? Alertes { get; init; }
    /// <summary>Secondes Unix.</summary>
    public long CreeLe { get; init; }
    public Fiche? Fiche { get; init; }
    public LienErp? Erp { get; init; }
    /// <summary><c>imprimer</c> ou <c>decouper</c>.</summary>
    public string? Mode { get; init; }
    public string? Format { get; init; }
    public List<int>? PagesExclues { get; init; }
    public double? DureeImpressionMin { get; init; }
    public double? PrixDeRevient { get; init; }

    [JsonIgnore]
    public int NbPages => Pages?.Count ?? 0;

    /// <summary>Dossier ERP s'il existe, sinon numéro de commande de la fiche.</summary>
    [JsonIgnore]
    public string Dossier => Texte.NonVide(Erp?.RefErp) ?? Texte.NonVide(Fiche?.Commande) ?? "";

    [JsonIgnore]
    public DateTime Date => DateTimeOffset.FromUnixTimeSeconds(CreeLe).LocalDateTime;

    [JsonIgnore]
    public bool DecoupeSeule => Mode == "decouper";

    /// <summary>Libellé de l'échec, dans la langue de l'écran.</summary>
    [JsonIgnore]
    public string? TexteErreur =>
        Statut == StatutTravail.Echec ? Libelles.Erreur(CodeErreur ?? "rendu_echoue", Erreur) : null;
}

public sealed class Page
{
    public List<string>? Plaques { get; init; }
    public List<string>? PlaquesDecoupe { get; init; }
    public double? LargeurMm { get; init; }
    public double? HauteurMm { get; init; }
    public double? SurfaceM2 { get; init; }

    /// <summary>« 594 × 841 mm », ou null si le serveur ne connaît pas le format (travaux anciens).</summary>
    [JsonIgnore]
    public string? FormatTexte
    {
        get
        {
            if (LargeurMm is not { } l || HauteurMm is not { } h || l <= 0 || h <= 0) return null;
            return $"{Mm(l)} × {Mm(h)} mm";
        }
    }

    // Au plus une décimale, virgule française, sans zéro inutile (comme NumberFormatter).
    private static string Mm(double v) => Math.Round(v, 1).ToString("0.#", Texte.Fr);
}

public sealed class Alerte
{
    public string Code { get; init; } = "";
    public int? Page { get; init; }
    public string? Detail { get; init; }
}

/// <summary>Fiche travail : champs vides quand ils ne sont pas renseignés.</summary>
public sealed class Fiche
{
    public string? Commande { get; init; }
    public string? Client { get; init; }
    public string? Societe { get; init; }
    public int? Quantite { get; init; }
    /// <summary>Secondes Unix.</summary>
    public long? Echeance { get; init; }
    public string? Notes { get; init; }
    public string? Operateur { get; init; }

    [JsonIgnore]
    public bool Vide =>
        Texte.NonVide(Commande) is null && Texte.NonVide(Client) is null && Texte.NonVide(Societe) is null
        && (Quantite ?? 0) == 0 && Echeance is null && Texte.NonVide(Notes) is null && Texte.NonVide(Operateur) is null;
}

public sealed class LienErp
{
    public string? Envoi { get; init; }
    public string? RefErp { get; init; }
}

// ---------------------------------------------------------------- File d'attente

[JsonConverter(typeof(ConvertisseurEtatSortie))]
public enum EtatSortie
{
    EnAttente,
    Suspendue,
    EnCours,
    Terminee,
    Echec,
    Annulee,
    Imbriquee,
    Inconnu,
}

public sealed class ConvertisseurEtatSortie : ConvertisseurTolerant<EtatSortie>
{
    protected override EtatSortie Depuis(string? brut) => brut switch
    {
        "en_attente" => EtatSortie.EnAttente,
        "suspendue" => EtatSortie.Suspendue,
        "en_cours" => EtatSortie.EnCours,
        "terminee" => EtatSortie.Terminee,
        "echec" => EtatSortie.Echec,
        "annulee" => EtatSortie.Annulee,
        "imbriquee" => EtatSortie.Imbriquee,
        _ => EtatSortie.Inconnu,
    };

    protected override string Vers(EtatSortie valeur) => valeur.Brut();
}

public static class EtatSortieExt
{
    public static string Brut(this EtatSortie e) => e switch
    {
        EtatSortie.EnAttente => "en_attente",
        EtatSortie.Suspendue => "suspendue",
        EtatSortie.EnCours => "en_cours",
        EtatSortie.Terminee => "terminee",
        EtatSortie.Echec => "echec",
        EtatSortie.Annulee => "annulee",
        EtatSortie.Imbriquee => "imbriquee",
        _ => "inconnu",
    };

    public static string Libelle(this EtatSortie e) => e switch
    {
        EtatSortie.EnAttente => "En attente",
        EtatSortie.Suspendue => "Suspendue",
        EtatSortie.EnCours => "En cours",
        EtatSortie.Terminee => "Terminée",
        EtatSortie.Echec => "Échec",
        EtatSortie.Annulee => "Annulée",
        EtatSortie.Imbriquee => "Imbriquée",
        _ => "Inconnu",
    };

    /// <summary>Encore à passer : le travail ne peut ni être supprimé ni relancé.</summary>
    public static bool Active(this EtatSortie e) => e is EtatSortie.EnAttente or EtatSortie.Suspendue or EtatSortie.EnCours;
}

/// <summary>Actions de <c>POST /api/file/{id}/{action}</c>, plus le retrait (<c>DELETE /api/file/{id}</c>).</summary>
public enum ActionSortie
{
    EnTete,
    Monter,
    Descendre,
    Suspendre,
    Reprendre,
    Annuler,
    Refaire,
    Retirer,
}

public static class ActionSortieExt
{
    /// <summary>Segment d'URL de l'action (inutilisé pour le retrait, qui est un DELETE).</summary>
    public static string Brut(this ActionSortie a) => a switch
    {
        ActionSortie.EnTete => "en_tete",
        ActionSortie.Monter => "monter",
        ActionSortie.Descendre => "descendre",
        ActionSortie.Suspendre => "suspendre",
        ActionSortie.Reprendre => "reprendre",
        ActionSortie.Annuler => "annuler",
        ActionSortie.Refaire => "refaire",
        _ => "retirer",
    };

    public static string Libelle(this ActionSortie a) => a switch
    {
        ActionSortie.EnTete => "Passer en tête",
        ActionSortie.Monter => "Monter",
        ActionSortie.Descendre => "Descendre",
        ActionSortie.Suspendre => "Suspendre",
        ActionSortie.Reprendre => "Reprendre",
        ActionSortie.Annuler => "Annuler",
        ActionSortie.Refaire => "Refaire",
        _ => "Retirer de la file",
    };

    public static bool Destructive(this ActionSortie a) => a is ActionSortie.Annuler or ActionSortie.Retirer;
}

public sealed class Sortie
{
    public string Id { get; init; } = "";
    public string Machine { get; init; } = "";
    public string? Operation { get; init; }
    public string Travail { get; init; } = "";
    public string Nom { get; init; } = "";
    public int Page { get; init; }
    public EtatSortie Etat { get; init; } = EtatSortie.Inconnu;
    public string? CodeErreur { get; init; }
    public string? Erreur { get; init; }
    public long? CreeLe { get; init; }
    public long? Debut { get; init; }
    public long? Fin { get; init; }
    public bool? Archivee { get; init; }
    public string? Planche { get; init; }
    public string? PlancheNom { get; init; }
    /// <summary>Code de <see cref="Libelles.Raisons"/> : pourquoi la sortie attend.</summary>
    public string? RaisonAttente { get; init; }
    public double? DureeMin { get; init; }
    public long? Poids { get; init; }

    [JsonIgnore]
    public bool EstDecoupe => Operation == "decoupe";

    [JsonIgnore]
    public bool EstArchivee => Archivee ?? false;

    /// <summary>Mêmes règles que <c>actionsPossibles</c> de l'interface web.</summary>
    [JsonIgnore]
    public IReadOnlyList<ActionSortie> Actions => Etat switch
    {
        EtatSortie.EnAttente => new[] { ActionSortie.EnTete, ActionSortie.Monter, ActionSortie.Descendre, ActionSortie.Suspendre, ActionSortie.Annuler },
        EtatSortie.Suspendue => new[] { ActionSortie.EnTete, ActionSortie.Monter, ActionSortie.Descendre, ActionSortie.Reprendre, ActionSortie.Annuler },
        EtatSortie.Terminee or EtatSortie.Echec or EtatSortie.Annulee => new[] { ActionSortie.Refaire, ActionSortie.Retirer },
        EtatSortie.Imbriquee => new[] { ActionSortie.Retirer },
        _ => Array.Empty<ActionSortie>(),
    };
}

// ---------------------------------------------------------------- Machines

public sealed class Machine
{
    public string Id { get; init; } = "";
    public string Nom { get; init; } = "";
    /// <summary><c>epson_sc_s80600</c>, <c>mutoh_kona</c>, <c>graphtec_fc7000</c>, <c>roland_versacamm</c>.</summary>
    public string? Modele { get; init; }
    public double? LaizeMm { get; init; }
    public string? Pool { get; init; }
    public string? VarianteDe { get; init; }

    /// <summary>Les opérations se déduisent du modèle (comme <c>Modele::imprime</c> / <c>decoupe</c> du serveur).</summary>
    [JsonIgnore]
    public bool Imprime => Modele is "epson_sc_s80600" or "roland_versacamm";

    [JsonIgnore]
    public bool Decoupe => Modele is "mutoh_kona" or "graphtec_fc7000" or "roland_versacamm";

    [JsonIgnore]
    public IReadOnlyList<string> Operations
    {
        get
        {
            var o = new List<string>();
            if (Imprime) o.Add("Impression");
            if (Decoupe) o.Add("Découpe");
            return o;
        }
    }

    /// <summary>Une sortie de la file vise cette machine, directement ou par son pool.</summary>
    public bool Vise(Sortie s) => s.Machine == Id || (Pool is not null && s.Machine == Pool);
}

public sealed class Reglage
{
    public string Nom { get; init; } = "";
    public string? Uuid { get; init; }
}

// ---------------------------------------------------------------- Session et système

public sealed class InfoSession
{
    public bool? AccesLibre { get; init; }
    public string? Utilisateur { get; init; }
    public string? Role { get; init; }
}

public sealed class InfoSysteme
{
    public string? Version { get; init; }
    /// <summary>Octets des plaques en attente dans la file.</summary>
    public long? Tampon { get; init; }
    public Rendus? Rendus { get; init; }
    public long? DisqueTotal { get; init; }
    public long? DisqueLibre { get; init; }
    public int? ConservationJours { get; init; }
    public long? SeuilLibre { get; init; }
    public List<AlerteDossier>? Alertes { get; init; }
}

public sealed class Rendus
{
    public int? Simultanes { get; init; }
    public int? EnCours { get; init; }
    public int? EnAttente { get; init; }
}

/// <summary>
/// Fichier d'état illisible (<c>Dossier</c> faux) ou dossier absent (<c>Dossier</c> vrai) ;
/// <c>Reprise</c> : la version précédente a été reprise, ou le dossier recréé.
/// </summary>
public sealed class AlerteDossier
{
    public string Fichier { get; init; } = "";
    public bool? Reprise { get; init; }
    public bool? Dossier { get; init; }

    public string Texte
    {
        get
        {
            var reprise = Reprise ?? false;
            if (Dossier == true)
                return reprise ? "Dossier absent, recréé" : "Dossier absent, impossible à recréer";
            return reprise ? "Fichier d'état illisible, version précédente reprise" : "Fichier d'état illisible";
        }
    }
}
