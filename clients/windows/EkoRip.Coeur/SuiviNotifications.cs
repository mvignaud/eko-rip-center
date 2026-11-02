namespace EkoRip.Coeur;

/// <summary>Une notification du système à afficher.</summary>
public sealed record NotificationRip(string Titre, string Texte);

/// <summary>
/// Détecte les passages à <c>termine</c> ou <c>echec</c> d'un travail et la fin d'une sortie
/// entre deux actualisations. Le premier passage ne fait que mémoriser l'état.
/// </summary>
public sealed class SuiviNotifications
{
    private Dictionary<string, StatutTravail> _statuts = new();
    private Dictionary<string, EtatSortie> _etats = new();
    private readonly HashSet<string> _suivis = new();
    private bool _premierPassage = true;

    /// <summary>Un travail qu'on vient d'envoyer : on notifie même s'il est fini avant la première vue.</summary>
    public void Suivre(string idTravail) => _suivis.Add(idTravail);

    public void Reinitialiser()
    {
        _statuts = new();
        _etats = new();
        _suivis.Clear();
        _premierPassage = true;
    }

    public IReadOnlyList<NotificationRip> Analyser(IReadOnlyList<Travail> travaux, IReadOnlyList<Sortie> sorties, Func<string, string> nomMachine)
    {
        var notifications = new List<NotificationRip>();
        if (!_premierPassage)
        {
            foreach (var t in travaux.Where(t => t.Statut.Fini()))
            {
                var vu = _statuts.TryGetValue(t.Id, out var avant) ? avant : (StatutTravail?)null;
                var enCours = vu is StatutTravail.EnAttente or StatutTravail.EnCours;
                var envoiSuivi = vu is null && _suivis.Contains(t.Id);
                if (!enCours && !envoiSuivi) continue;
                _suivis.Remove(t.Id);
                if (t.Statut == StatutTravail.Termine)
                    notifications.Add(new NotificationRip("Travail prêt", t.Nom));
                else
                    notifications.Add(new NotificationRip("Échec du rendu", $"{t.Nom} : {t.TexteErreur ?? ""}"));
            }

            foreach (var s in sorties)
            {
                if (!_etats.TryGetValue(s.Id, out var avant) || !avant.Active()) continue;
                var machine = nomMachine(s.Machine);
                if (s.Etat == EtatSortie.Terminee)
                {
                    notifications.Add(new NotificationRip(
                        s.EstDecoupe ? "Découpe terminée" : "Impression terminée",
                        $"{s.Nom}, page {s.Page} · {machine}"));
                }
                else if (s.Etat == EtatSortie.Echec)
                {
                    notifications.Add(new NotificationRip(
                        s.EstDecoupe ? "Découpe en échec" : "Impression en échec",
                        $"{s.Nom}, page {s.Page} · {machine} : {Libelles.Erreur(s.CodeErreur ?? "sortie_echouee", s.Erreur)}"));
                }
            }
        }

        _statuts = travaux.GroupBy(t => t.Id).ToDictionary(g => g.Key, g => g.First().Statut);
        _etats = sorties.GroupBy(s => s.Id).ToDictionary(g => g.Key, g => g.First().Etat);
        _premierPassage = false;
        return notifications;
    }
}
