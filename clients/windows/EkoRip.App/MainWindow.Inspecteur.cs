using System.Text.Json;
using EkoRip.Coeur;
using Microsoft.UI;
using Microsoft.UI.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Windows.Storage.Streams;

namespace EkoRip.Client;

// Inspecteur (colonne de droite) du travail choisi : aperçu, fiche, sorties, actions.
public sealed partial class MainWindow
{
    private string? _cleInspecteur;
    private string? _cleApercu;
    private int _versionApercu;

    private static readonly SolidColorBrush PastilleGris = new(ColorHelper.FromArgb(255, 138, 138, 138));
    private static readonly SolidColorBrush PastilleVerte = new(ColorHelper.FromArgb(255, 16, 124, 16));
    private static readonly SolidColorBrush PastilleRouge = new(ColorHelper.FromArgb(255, 209, 52, 56));

    private int OngletCourant()
    {
        if (ReferenceEquals(Onglets.SelectedItem, OngletFiche)) return 1;
        if (ReferenceEquals(Onglets.SelectedItem, OngletSorties)) return 2;
        return 0;
    }

    private void Onglets_SelectionChanged(SelectorBar sender, SelectorBarSelectionChangedEventArgs args)
    {
        if (_synchro) return;
        MettreAJourInspecteur(true);
    }

    private void MettreAJourInspecteur(bool forcer)
    {
        var t = _c.TravailSelectionne;
        InspecteurVide.Visibility = Vis(t is null);
        InspecteurContenu.Visibility = Vis(t is not null);
        if (t is null)
        {
            _cleInspecteur = null;
            _cleApercu = null;
            return;
        }

        var page = _c.PageValide(t);
        var onglet = OngletCourant();

        TitreInspecteur.Text = t.Nom;
        TexteStatut.Text = t.Statut.Libelle();
        TexteDossier.Text = t.Dossier.Length > 0 ? "· " + t.Dossier : "";
        AnneauStatut.IsActive = t.Statut == StatutTravail.EnCours;
        AnneauStatut.Visibility = Vis(t.Statut == StatutTravail.EnCours);
        PastilleStatut.Visibility = Vis(t.Statut != StatutTravail.EnCours);
        PastilleStatut.Fill = t.Statut switch
        {
            StatutTravail.Termine => PastilleVerte,
            StatutTravail.Echec => PastilleRouge,
            _ => PastilleGris,
        };
        ToolTipService.SetToolTip(TexteStatut, t.TexteErreur);

        PanneauApercu.Visibility = Vis(onglet == 0);
        PanneauFiche.Visibility = Vis(onglet == 1);
        PanneauSorties.Visibility = Vis(onglet == 2);

        MettreAJourActions(t);

        // On ne reconstruit que ce qui a changé : l'actualisation toutes les 3 s ne doit pas
        // faire sauter une sélection de texte ni fermer un menu ouvert.
        var cle = onglet switch
        {
            1 => JsonSerializer.Serialize(new { t.Id, t.Fiche, t.Erp, t.CreeLe, t.Mode, t.Format }, JsonServeur.Options),
            2 => JsonSerializer.Serialize(new { t.Id, s = _c.SortiesDe(t), m = _c.Machines.Select(x => new { x.Id, x.Nom }) }, JsonServeur.Options),
            _ => JsonSerializer.Serialize(new { t.Id, t.Statut, t.CodeErreur, t.Erreur, t.Pages, t.Alertes, t.DureeImpressionMin, page }, JsonServeur.Options),
        };
        cle = onglet + "|" + cle;
        if (!forcer && cle == _cleInspecteur) return;
        _cleInspecteur = cle;

        switch (onglet)
        {
            case 0: ConstruireApercu(t, page); break;
            case 1: ConstruireFiche(t); break;
            default: ConstruireSorties(t); break;
        }
    }

    // ------------------------------------------------------------ Actions

    private void MettreAJourActions(Travail t)
    {
        var machines = _c.MachinesPour(t);
        BoutonImprimerInspecteur.Content = t.DecoupeSeule ? "Découper" : "Imprimer";
        BoutonImprimerInspecteur.IsEnabled = t.Statut == StatutTravail.Termine && machines.Count > 0;
        ToolTipService.SetToolTip(BoutonImprimerInspecteur, machines.Count == 0
            ? "Aucune machine ne fait cette opération."
            : "Mettre la page en file sur une machine");
        BoutonEpreuveInspecteur.IsEnabled = t.Statut == StatutTravail.Termine;
        BoutonRelancer.IsEnabled = !t.Statut.Occupe();
        BoutonSupprimer.IsEnabled = !t.Statut.Occupe();
    }

    private void MenuImprimerInspecteur_Opening(object sender, object e)
    {
        RemplirMenuImprimer(MenuImprimerInspecteur, detaille: true);
    }

    /// <summary>
    /// Machines où le travail peut partir. <paramref name="detaille"/> : un sous-menu par machine
    /// (page affichée / toutes les pages) ; sinon la page affichée seulement (barre de commandes).
    /// </summary>
    private void RemplirMenuImprimer(MenuFlyout menu, bool detaille)
    {
        menu.Items.Clear();
        if (_c.TravailSelectionne is not { } t) return;
        var page = _c.PageValide(t);
        foreach (var m in _c.MachinesPour(t))
        {
            var machine = m;
            if (!detaille)
            {
                var libelle = t.NbPages > 1 ? $"Page {page} sur {machine.Nom}" : machine.Nom;
                menu.Items.Add(ElementMenu(libelle, () => _c.Imprimer(t, page, machine)));
            }
            else if (t.NbPages > 1)
            {
                var sous = new MenuFlyoutSubItem { Text = machine.Nom };
                sous.Items.Add(ElementMenu($"Page {page}", () => _c.Imprimer(t, page, machine)));
                sous.Items.Add(ElementMenu($"Toutes les pages ({t.NbPages})", () => _c.ImprimerToutes(t, machine)));
                menu.Items.Add(sous);
            }
            else
            {
                menu.Items.Add(ElementMenu(machine.Nom, () => _c.Imprimer(t, 1, machine)));
            }
        }
    }

    private MenuFlyoutItem ElementMenu(string texte, Func<Task> action)
    {
        var element = new MenuFlyoutItem { Text = texte };
        element.Click += async (_, _) => await Proteger(action);
        return element;
    }

    private async void Relancer_Click(object sender, RoutedEventArgs e)
    {
        if (_c.TravailSelectionne is { } t) await Proteger(() => _c.Relancer(t));
    }

    private async void Supprimer_Click(object sender, RoutedEventArgs e)
    {
        if (_c.TravailSelectionne is { } t) await ConfirmerSuppression(t);
    }

    private void PagePrecedente_Click(object sender, RoutedEventArgs e)
    {
        if (_c.TravailSelectionne is { } t) _c.ChoisirPage(_c.PageValide(t) - 1);
    }

    private void PageSuivante_Click(object sender, RoutedEventArgs e)
    {
        if (_c.TravailSelectionne is { } t) _c.ChoisirPage(_c.PageValide(t) + 1);
    }

    // ------------------------------------------------------------ Aperçu

    private void ConstruireApercu(Travail t, int page)
    {
        NavigationPages.Visibility = Vis(t.NbPages > 1);
        TextePage.Text = $"Page {page} sur {t.NbPages}";
        PagePrecedente.IsEnabled = page > 1;
        PageSuivante.IsEnabled = page < t.NbPages;

        TexteErreurTravail.Text = t.TexteErreur ?? "";
        TexteErreurTravail.Visibility = Vis(t.TexteErreur is not null);

        InfosApercu.Children.Clear();
        var infos = t.Pages is { } pages && page >= 1 && page <= pages.Count ? pages[page - 1] : null;
        if (infos?.FormatTexte is { } format) InfosApercu.Children.Add(LigneInfo("Format", format));
        if (infos?.Plaques is { Count: > 0 } plaques) InfosApercu.Children.Add(LigneInfo("Plaques", string.Join(", ", plaques)));
        if (t.DureeImpressionMin is { } duree && duree > 0) InfosApercu.Children.Add(LigneInfo("Durée estimée", Regles.DureeTexte(duree)));

        ListeAlertes.Children.Clear();
        if (t.Alertes is { Count: > 0 } alertes)
        {
            ListeAlertes.Children.Add(new TextBlock { Text = "Alertes", FontWeight = FontWeights.SemiBold, Margin = new Thickness(0, 6, 0, 0) });
            foreach (var a in alertes)
            {
                var ligne = new Grid { ColumnSpacing = 8 };
                ligne.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
                ligne.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
                ligne.Children.Add(Icone("", "IconeAttention"));
                var texte = Texte(Libelles.TexteAlerte(a), "TexteAttention");
                texte.FontSize = 13;
                Grid.SetColumn(texte, 1);
                ligne.Children.Add(texte);
                ListeAlertes.Children.Add(ligne);
            }
        }

        _ = ChargerApercu(t, page);
    }

    /// <summary>
    /// Aperçu PNG de la page, chargé par l'API du client (avec le cookie de connexion). Recharge
    /// quand le travail, la page ou le statut change (un rendu relancé).
    /// </summary>
    private async Task ChargerApercu(Travail t, int page)
    {
        var cle = $"{t.Id}#{page}#{t.Statut.Brut()}";
        if (cle == _cleApercu) return;
        _cleApercu = cle;
        var version = ++_versionApercu;

        ImageApercu.Source = null;
        AnneauApercu.IsActive = false;
        TexteApercu.Text = "";

        if (t.Statut.Occupe())
        {
            AnneauApercu.IsActive = true;
            TexteApercu.Text = "Rendu en cours…";
            return;
        }
        if (t.Statut == StatutTravail.Echec)
        {
            TexteApercu.Text = "Pas d'aperçu";
            return;
        }
        var api = _c.Api;
        if (t.Statut != StatutTravail.Termine || api is null)
        {
            AnneauApercu.IsActive = true;
            return;
        }

        AnneauApercu.IsActive = true;
        try
        {
            var octets = await api.Apercu(t.Id, page);
            if (version != _versionApercu) return;

            using var flux = new InMemoryRandomAccessStream();
            using (var ecrivain = new DataWriter(flux))
            {
                ecrivain.WriteBytes(octets);
                await ecrivain.StoreAsync();
                ecrivain.DetachStream();
            }
            flux.Seek(0);
            var image = new BitmapImage();
            await image.SetSourceAsync(flux);
            if (version != _versionApercu) return;
            ImageApercu.Source = image;
        }
        catch (OperationCanceledException)
        {
            // Une autre page a pris la place.
        }
        catch (ErreurApi e) when (e.ConnexionRequise)
        {
            _c.Traiter(e);
        }
        catch (ErreurApi e)
        {
            if (version == _versionApercu) TexteApercu.Text = e.Message;
        }
        catch (Exception)
        {
            // Image illisible (PNG tronqué, mémoire…).
            if (version == _versionApercu) TexteApercu.Text = "Aperçu illisible";
        }
        finally
        {
            if (version == _versionApercu) AnneauApercu.IsActive = false;
        }
    }

    // ------------------------------------------------------------ Fiche

    private void ConstruireFiche(Travail t)
    {
        PanneauFiche.Children.Clear();
        var f = t.Fiche;
        if (t.Erp?.RefErp is { Length: > 0 } erp) PanneauFiche.Children.Add(LigneInfo("Dossier ERP", erp));
        if (f is not null && !f.Vide)
        {
            AjouterLigne("Client", f.Client);
            AjouterLigne("Société", f.Societe);
            AjouterLigne("Commande", f.Commande);
            if (f.Quantite is { } quantite && quantite > 0) PanneauFiche.Children.Add(LigneInfo("Quantité", quantite.ToString()));
            if (f.Echeance is { } echeance)
                PanneauFiche.Children.Add(LigneInfo("Échéance", Regles.DateLongue(DateTimeOffset.FromUnixTimeSeconds(echeance).LocalDateTime)));
            AjouterLigne("Opérateur", f.Operateur);
            if (!string.IsNullOrEmpty(f.Notes))
            {
                PanneauFiche.Children.Add(new TextBlock { Text = "Notes", FontWeight = FontWeights.SemiBold, Margin = new Thickness(0, 6, 0, 0) });
                PanneauFiche.Children.Add(new TextBlock { Text = f.Notes, TextWrapping = TextWrapping.Wrap, IsTextSelectionEnabled = true });
            }
        }
        else
        {
            PanneauFiche.Children.Add(Texte("Aucune fiche renseignée.", "TexteSecondaire"));
        }

        PanneauFiche.Children.Add(Separateur());
        PanneauFiche.Children.Add(LigneInfo("Reçu le", Regles.DateLongue(t.Date)));
        if (t.Mode is not null) PanneauFiche.Children.Add(LigneInfo("Mode", t.DecoupeSeule ? "Découpe seule" : "Impression"));
        if (t.Format is not null) PanneauFiche.Children.Add(LigneInfo("Fichier", t.Format.ToUpperInvariant()));

        void AjouterLigne(string titre, string? valeur)
        {
            if (!string.IsNullOrEmpty(valeur)) PanneauFiche.Children.Add(LigneInfo(titre, valeur));
        }
    }

    // ------------------------------------------------------------ Sorties du travail

    private void ConstruireSorties(Travail t)
    {
        PanneauSorties.Children.Clear();
        var liste = _c.SortiesDe(t);
        if (liste.Count == 0)
        {
            PanneauSorties.Children.Add(Texte("Ce travail n'a encore aucune sortie dans la file.", "TexteSecondaire"));
            return;
        }
        for (var i = 0; i < liste.Count; i++)
        {
            PanneauSorties.Children.Add(LigneSortie(liste[i], avecMachine: true));
            if (i < liste.Count - 1) PanneauSorties.Children.Add(Separateur());
        }
    }
}
