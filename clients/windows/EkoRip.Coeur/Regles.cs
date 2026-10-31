using System.Globalization;
using System.Text;

namespace EkoRip.Coeur;

public enum ColonneTri { Nom, Dossier, Pages, Statut, Date }

/// <summary>
/// Règles pures de l'interface (tri, recherche, sorties d'un travail, machines utilisables,
/// fichiers acceptés, champs d'envoi, formats d'affichage), reprises d'<c>Etat.swift</c>.
/// </summary>
public static class Regles
{
    /// <summary>Extensions proposées à l'envoi (<c>.ai</c> est un PDF pour le serveur).</summary>
    public static readonly IReadOnlySet<string> ExtensionsAcceptees =
        new HashSet<string> { ".pdf", ".ai", ".svg", ".tif", ".tiff", ".png", ".jpg", ".jpeg" };

    // ------------------------------------------------------------ Recherche et tri

    /// <summary>Minuscules sans accents, pour comparer sans tenir compte de la casse ni des accents.</summary>
    public static string SansAccents(string texte)
    {
        var d = texte.Normalize(NormalizationForm.FormD);
        var sb = new StringBuilder(d.Length);
        foreach (var c in d)
        {
            if (CharUnicodeInfo.GetUnicodeCategory(c) != UnicodeCategory.NonSpacingMark) sb.Append(c);
        }
        return sb.ToString().Normalize(NormalizationForm.FormC).ToLowerInvariant();
    }

    /// <summary>Travaux dont le nom ou le dossier contient la recherche.</summary>
    public static IReadOnlyList<Travail> Filtrer(IEnumerable<Travail> travaux, string? recherche)
    {
        var q = (recherche ?? "").Trim();
        if (q.Length == 0) return travaux.ToList();
        q = SansAccents(q);
        return travaux
            .Where(t => SansAccents(t.Nom).Contains(q, StringComparison.Ordinal)
                        || SansAccents(t.Dossier).Contains(q, StringComparison.Ordinal))
            .ToList();
    }

    /// <summary>Tri stable ; à égalité, les plus récents d'abord.</summary>
    public static IReadOnlyList<Travail> Trier(IEnumerable<Travail> travaux, ColonneTri colonne, bool decroissant)
    {
        var comparateur = StringComparer.Create(Texte.Fr, ignoreCase: true);
        Func<Travail, Travail, int> comparer = colonne switch
        {
            ColonneTri.Nom => (a, b) => comparateur.Compare(a.Nom, b.Nom),
            ColonneTri.Dossier => (a, b) => comparateur.Compare(a.Dossier, b.Dossier),
            ColonneTri.Pages => (a, b) => a.NbPages.CompareTo(b.NbPages),
            ColonneTri.Statut => (a, b) => a.Statut.Rang().CompareTo(b.Statut.Rang()),
            _ => (a, b) => a.CreeLe.CompareTo(b.CreeLe),
        };
        var sens = decroissant ? -1 : 1;
        // OrderBy est stable ; la clé de départage est l'ordre d'arrivée.
        return travaux
            .Select((t, i) => (t, i))
            .OrderBy(x => x, Comparer<(Travail t, int i)>.Create((x, y) =>
            {
                var c = sens * comparer(x.t, y.t);
                return c != 0 ? c : x.i.CompareTo(y.i);
            }))
            .Select(x => x.t)
            .ToList();
    }

    // ------------------------------------------------------------ Sorties et machines

    public static IReadOnlyList<Sortie> SortiesDe(IEnumerable<Sortie> sorties, Travail t) =>
        sorties.Where(s => s.Travail == t.Id && !s.EstArchivee).ToList();

    /// <summary>Sorties actives d'une machine : celle qui tourne d'abord, puis la file dans son ordre.</summary>
    public static IReadOnlyList<Sortie> SortiesActivesPour(IEnumerable<Sortie> sorties, Machine m)
    {
        var actives = sorties.Where(s => m.Vise(s) && s.Etat.Active() && !s.EstArchivee).ToList();
        return actives.Where(s => s.Etat == EtatSortie.EnCours).Concat(actives.Where(s => s.Etat != EtatSortie.EnCours)).ToList();
    }

    /// <summary>Machines sur lesquelles ce travail peut partir : celles qui impriment, ou qui découpent pour un travail en découpe seule.</summary>
    public static IReadOnlyList<Machine> MachinesPour(IEnumerable<Machine> machines, Travail t) =>
        machines.Where(m => t.DecoupeSeule ? m.Decoupe : m.Imprime).ToList();

    public static string NomMachine(IEnumerable<Machine> machines, string id) =>
        machines.FirstOrDefault(m => m.Id == id)?.Nom ?? id;

    public static bool PeutSuspendre(IEnumerable<Sortie> sorties, Travail t) =>
        sorties.Any(s => s.Travail == t.Id && s.Etat == EtatSortie.EnAttente);

    public static bool PeutReprendre(IEnumerable<Sortie> sorties, Travail t) =>
        sorties.Any(s => s.Travail == t.Id && s.Etat == EtatSortie.Suspendue);

    public static int PageValide(Travail t, int page) => Math.Min(Math.Max(page, 1), Math.Max(t.NbPages, 1));

    /// <summary>Opération à demander au serveur pour ce travail.</summary>
    public static string Operation(Travail t) => t.DecoupeSeule ? "decoupe" : "impression";

    /// <summary>Pages à mettre en file pour « Toutes les pages » : sans les pages exclues.</summary>
    public static IReadOnlyList<int> PagesAImprimer(Travail t)
    {
        var exclues = new HashSet<int>(t.PagesExclues ?? new List<int>());
        return Enumerable.Range(1, Math.Max(t.NbPages, 1)).Where(p => !exclues.Contains(p)).ToList();
    }

    /// <summary>Pastille de la barre latérale : travaux en cours de rendu.</summary>
    public static int CompteTravaux(IEnumerable<Travail> travaux) => travaux.Count(t => t.Statut.Occupe());

    /// <summary>Pastille de la barre latérale : sorties à passer.</summary>
    public static int CompteSorties(IEnumerable<Sortie> sorties) => sorties.Count(s => s.Etat.Active() && !s.EstArchivee);

    // ------------------------------------------------------------ Envoi

    /// <summary>Fichiers dont l'extension est acceptée (sans tenir compte de la casse).</summary>
    public static IReadOnlyList<string> FichiersRetenus(IEnumerable<string> chemins) =>
        chemins.Where(c => ExtensionsAcceptees.Contains(Path.GetExtension(c).ToLowerInvariant())).ToList();

    /// <summary>La quantité est vide, ou un entier de 0 à 100 000.</summary>
    public static bool QuantiteValide(string? quantite)
    {
        var q = (quantite ?? "").Trim();
        return q.Length == 0 || (int.TryParse(q, NumberStyles.None, CultureInfo.InvariantCulture, out var n) && n is >= 0 and <= 100_000);
    }

    /// <summary>Champs du multipart avant le fichier : <c>reglage</c>, puis <c>client</c>, <c>commande</c>, <c>quantite</c> s'ils sont remplis.</summary>
    public static List<(string Nom, string Valeur)> ChampsEnvoi(string? reglage, string? client, string? commande, string? quantite)
    {
        var champs = new List<(string, string)>();
        if (!string.IsNullOrEmpty(reglage)) champs.Add(("reglage", reglage));
        foreach (var (nom, valeur) in new[] { ("client", client), ("commande", commande), ("quantite", quantite) })
        {
            var v = (valeur ?? "").Trim();
            if (v.Length > 0) champs.Add((nom, v));
        }
        return champs;
    }

    /// <summary>Nom proposé pour l'épreuve TIFF : <c>nom-p1-epreuve.tif</c>.</summary>
    public static string NomEpreuve(string nomTravail, int page)
    {
        // Découpage à la main : sous Windows, Path.GetFileNameWithoutExtension prend « a: » pour
        // un lecteur et renverrait « b » pour « a:b.pdf ».
        var nom = nomTravail.Replace('\\', '/');
        nom = nom[(nom.LastIndexOf('/') + 1)..];
        var point = nom.LastIndexOf('.');
        var sansExtension = point > 0 ? nom[..point] : nom;
        var base_ = new string(sansExtension.Select(c => c is '/' or ':' or '\\' or '*' or '?' or '"' or '<' or '>' or '|' ? '_' : c).ToArray());
        if (base_.Length == 0) base_ = "epreuve";
        return $"{base_}-p{page}-epreuve.tif";
    }

    // ------------------------------------------------------------ Formats d'affichage

    /// <summary>« moins d'une minute », « 25 min », « 2 h 05 ».</summary>
    public static string DureeTexte(double minutes)
    {
        var total = (int)Math.Round(minutes, MidpointRounding.AwayFromZero);
        if (total < 1) return "moins d'une minute";
        if (total < 60) return $"{total} min";
        return $"{total / 60} h {total % 60:00}";
    }

    /// <summary>Taille en octets, unités décimales comme le Finder : « 27,97 Go ».</summary>
    public static string Octets(long? n)
    {
        if (n is not { } v) return "inconnu";
        string[] unites = { "octets", "Ko", "Mo", "Go", "To" };
        double x = v;
        var i = 0;
        while (x >= 1000 && i < unites.Length - 1) { x /= 1000; i++; }
        if (i == 0) return $"{v} octets";
        var chiffres = x >= 100 ? "0" : x >= 10 ? "0.#" : "0.##";
        return x.ToString(chiffres, Texte.Fr) + " " + unites[i];
    }

    public static string DateCourte(DateTime d) => d.ToString("d MMM HH:mm", Texte.Fr);

    public static string DateLongue(DateTime d) => d.ToString("d MMM yyyy HH:mm", Texte.Fr);
}
