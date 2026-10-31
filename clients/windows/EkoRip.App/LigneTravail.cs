using System.ComponentModel;
using EkoRip.Coeur;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using Windows.UI;

namespace EkoRip.Client;

/// <summary>
/// Une ligne du tableau des travaux. Le contrôleur remplace les <see cref="Travail"/> à chaque
/// actualisation ; la ligne garde son identité (et donc la sélection et le défilement) et prévient
/// la liste par <see cref="INotifyPropertyChanged"/>.
/// </summary>
public sealed class LigneTravail : INotifyPropertyChanged
{
    private static readonly SolidColorBrush Gris = new(Color.FromArgb(255, 138, 138, 138));
    private static readonly SolidColorBrush Vert = new(Color.FromArgb(255, 16, 124, 16));
    private static readonly SolidColorBrush Rouge = new(Color.FromArgb(255, 209, 52, 56));

    public LigneTravail(Travail travail) => Travail = travail;

    public event PropertyChangedEventHandler? PropertyChanged;

    public Travail Travail { get; private set; }

    public string Id => Travail.Id;
    public string Nom => Travail.Nom;
    public string Dossier => Travail.Dossier;
    public string Pages => Travail.NbPages.ToString();
    public string Statut => Travail.Statut.Libelle();
    public string DateTexte => Regles.DateCourte(Travail.Date);

    /// <summary>La cause de l'échec, au survol ; null hors échec (pas d'info-bulle vide).</summary>
    public string? InfoErreur => Travail.TexteErreur;

    public Brush CouleurStatut => Travail.Statut switch
    {
        StatutTravail.Termine => Vert,
        StatutTravail.Echec => Rouge,
        _ => Gris,
    };

    public Visibility VisibiliteAnneau => Travail.Statut == StatutTravail.EnCours ? Visibility.Visible : Visibility.Collapsed;

    public Visibility VisibilitePastille => Travail.Statut == StatutTravail.EnCours ? Visibility.Collapsed : Visibility.Visible;

    /// <summary>Nom lu par le Narrateur pour la ligne (sinon « EkoRip.Client.LigneTravail »).</summary>
    public override string ToString() => $"{Nom}, {Statut}";

    public void Mettre(Travail travail)
    {
        Travail = travail;
        // Chaîne vide : toutes les propriétés ont changé.
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(string.Empty));
    }
}
