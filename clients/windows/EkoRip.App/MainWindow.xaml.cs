using System.Collections.ObjectModel;
using System.Text.Json;
using EkoRip.Coeur;
using Microsoft.UI;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Windows.Graphics;
using Windows.System;

namespace EkoRip.Client;

/// <summary>
/// Fenêtre unique : écran d'adresse et d'identifiant, puis barre latérale (Travaux, Machines,
/// Système), barre de commandes, tableau des travaux et inspecteur. Toute la logique est dans
/// <see cref="Controleur"/> (EkoRip.Coeur) ; ici on ne fait que dessiner son état et lui transmettre
/// les gestes. Fichiers : ce fichier (cadre, connexion, travaux), <c>MainWindow.Inspecteur.cs</c>,
/// <c>MainWindow.Vues.cs</c> (machines, système, barre d'état) et <c>MainWindow.Dialogues.cs</c>.
/// </summary>
public sealed partial class MainWindow : Window
{
    private readonly Controleur _c;
    private readonly ObservableCollection<LigneTravail> _lignes = new();

    /// <summary>Vrai pendant qu'on met l'interface à jour : les gestionnaires d'événements ne font rien.</summary>
    private bool _synchro;
    private bool _ferme;
    private Phase? _phaseAffichee;

    public MainWindow(Controleur controleur)
    {
        _c = controleur;
        InitializeComponent();
        Title = "E-KO RIP Center";
        ConfigurerFenetre();

        ListeTravaux.ItemsSource = _lignes;
        _c.Changed += SurChangement;
        _c.FichiersProposes += SurFichiersProposes;
        Closed += SurFermeture;
        MettreAJour();
    }

    private void ConfigurerFenetre()
    {
        try
        {
            AppWindow.Resize(new SizeInt32(1280, 780));
            AppWindow.SetIcon(Path.Combine(AppContext.BaseDirectory, "Assets", "icone.ico"));
        }
        catch (Exception e)
        {
            // Taille ou icône par défaut : sans gravité.
            Journal.Ecrire(e);
        }
    }

    private void SurFermeture(object sender, WindowEventArgs args)
    {
        _ferme = true;
        _c.Changed -= SurChangement;
        _c.FichiersProposes -= SurFichiersProposes;
        _administration?.Close();
        NotificationsSysteme.Arreter();
        _c.Dispose();
    }

    private static Visibility Vis(bool visible) => visible ? Visibility.Visible : Visibility.Collapsed;

    /// <summary>Exécute une action asynchrone d'un gestionnaire ; toute erreur est dite à l'opérateur.</summary>
    private async Task Proteger(Func<Task> action)
    {
        try
        {
            await action();
        }
        catch (Exception e)
        {
            TraiterErreur(e);
        }
    }

    /// <summary>
    /// Erreur d'une action de l'opérateur : dite à l'écran, et notée dans erreurs.log quand elle
    /// ne vient pas de l'API (exception Windows inattendue, souvent sans message).
    /// </summary>
    private void TraiterErreur(Exception e)
    {
        if (e is not ErreurApi and not OperationCanceledException) Journal.Ecrire(e);
        _c.Traiter(e);
    }

    // ------------------------------------------------------------ Mise à jour de l'interface

    private void SurChangement()
    {
        if (_ferme) return;
        if (DispatcherQueue.HasThreadAccess) MettreAJour();
        else DispatcherQueue.TryEnqueue(MettreAJour);
    }

    private async void Racine_Loaded(object sender, RoutedEventArgs e)
    {
        await Proteger(_c.Demarrer);
    }

    /// <summary>Redessine tout ce qui dépend de l'état du contrôleur ; ne reconstruit que ce qui a changé.</summary>
    private void MettreAJour()
    {
        if (_synchro || _ferme) return;
        _synchro = true;
        try
        {
            var phase = _c.Phase;
            VueDemarrage.Visibility = Vis(phase == Phase.Demarrage);
            VueConnexion.Visibility = Vis(phase is Phase.Adresse or Phase.Connexion);
            Navigation.Visibility = Vis(phase == Phase.Connecte);

            if (phase is Phase.Adresse or Phase.Connexion)
            {
                MettreAJourConnexion();
            }
            else
            {
                _phaseAffichee = null;
            }

            if (phase == Phase.Connecte)
            {
                MettreAJourPrincipal();
            }
            else
            {
                ViderPrincipal();
            }

            if (_c.Message is { } message && !_messageAffiche)
            {
                _messageAffiche = true;
                _ = AfficherMessage(message);
            }
        }
        finally
        {
            _synchro = false;
        }
    }

    // ------------------------------------------------------------ Connexion

    private bool ConnexionPrete() =>
        _c.Phase == Phase.Connexion
            ? ChampNom.Text.Trim().Length > 0 && ChampMotDePasse.Password.Length > 0
            : ChampAdresse.Text.Trim().Length > 0;

    private void MettreAJourConnexion()
    {
        var identifiant = _c.Phase == Phase.Connexion;
        ChampAdresse.Visibility = Vis(!identifiant);
        ChampNom.Visibility = Vis(identifiant);
        ChampMotDePasse.Visibility = Vis(identifiant);
        SousTitreConnexion.Text = identifiant ? $"Connexion au serveur {_c.Adresse}" : "Adresse du serveur RIP";
        TexteErreurConnexion.Text = _c.ErreurConnexion ?? "";
        TexteErreurConnexion.Visibility = Vis(!string.IsNullOrEmpty(_c.ErreurConnexion));
        BoutonChangerServeur.Visibility = Vis(identifiant);
        BoutonAnnulerAdresse.Visibility = Vis(!identifiant && !string.IsNullOrEmpty(_c.Parametres.Adresse));
        AnneauConnexion.IsActive = _c.Occupe;

        if (_phaseAffichee != _c.Phase)
        {
            _phaseAffichee = _c.Phase;
            if (identifiant)
            {
                ChampNom.Text = _c.Parametres.DernierUtilisateur;
                ChampMotDePasse.Password = "";
                if (ChampNom.Text.Length == 0) ChampNom.Focus(FocusState.Programmatic);
                else ChampMotDePasse.Focus(FocusState.Programmatic);
            }
            else
            {
                ChampAdresse.Text = _c.Adresse.Length > 0 ? _c.Adresse : _c.Parametres.Adresse;
                ChampAdresse.Focus(FocusState.Programmatic);
            }
        }
        BoutonConnecter.IsEnabled = !_c.Occupe && ConnexionPrete();
    }

    private void ChampConnexion_TextChanged(object sender, TextChangedEventArgs e)
    {
        BoutonConnecter.IsEnabled = !_c.Occupe && ConnexionPrete();
    }

    private void ChampMotDePasse_PasswordChanged(object sender, RoutedEventArgs e)
    {
        BoutonConnecter.IsEnabled = !_c.Occupe && ConnexionPrete();
    }

    private async void ChampConnexion_KeyDown(object sender, KeyRoutedEventArgs e)
    {
        if (e.Key == VirtualKey.Enter && BoutonConnecter.IsEnabled)
        {
            e.Handled = true;
            await Valider();
        }
    }

    private async void Connecter_Click(object sender, RoutedEventArgs e)
    {
        await Valider();
    }

    private async Task Valider()
    {
        if (_c.Phase == Phase.Connexion)
        {
            var nom = ChampNom.Text.Trim();
            var secret = ChampMotDePasse.Password;
            ChampMotDePasse.Password = "";
            await Proteger(() => _c.OuvrirSession(nom, secret));
        }
        else
        {
            var saisie = ChampAdresse.Text;
            await Proteger(() => _c.Connecter(saisie));
        }
    }

    private void ChangerServeurConnexion_Click(object sender, RoutedEventArgs e) => _c.ChangerServeur();

    private async void AnnulerAdresse_Click(object sender, RoutedEventArgs e)
    {
        var retenue = _c.Parametres.Adresse;
        await Proteger(() => _c.Connecter(retenue));
    }

    // ------------------------------------------------------------ Écran principal

    private void ViderPrincipal()
    {
        if (_lignes.Count > 0) _lignes.Clear();
        _cleInspecteur = null;
        _cleApercu = null;
        _cleMachines = null;
        _cleSysteme = null;
        _administration?.Close();
    }

    private void MettreAJourPrincipal()
    {
        var rubrique = _c.Rubrique;
        var entree = rubrique switch
        {
            Rubrique.Machines => EntreeMachines,
            Rubrique.Systeme => EntreeSysteme,
            _ => EntreeTravaux,
        };
        if (!ReferenceEquals(Navigation.SelectedItem, entree)) Navigation.SelectedItem = entree;

        PanneauTravaux.Visibility = Vis(rubrique == Rubrique.Travaux);
        PanneauMachines.Visibility = Vis(rubrique == Rubrique.Machines);
        PanneauSysteme.Visibility = Vis(rubrique == Rubrique.Systeme);

        var montrerInspecteur = rubrique == Rubrique.Travaux && BoutonInspecteur.IsChecked == true;
        ColonneInspecteur.Width = new GridLength(montrerInspecteur ? 380 : 0);
        Inspecteur.Visibility = Vis(montrerInspecteur);
        BoutonInspecteur.IsEnabled = rubrique == Rubrique.Travaux;

        MettreAJourBadges();
        MettreAJourTableau();
        MettreAJourCommandes();
        if (montrerInspecteur) MettreAJourInspecteur(false);
        if (rubrique == Rubrique.Machines) MettreAJourMachines();
        if (rubrique == Rubrique.Systeme) MettreAJourSysteme();
        MettreAJourBarreEtat();

        BoutonDeconnecter.IsEnabled = _c.Session?.AccesLibre != true;
    }

    private void MettreAJourBadges()
    {
        var travaux = Regles.CompteTravaux(_c.Travaux);
        BadgeTravaux.Value = travaux;
        BadgeTravaux.Visibility = Vis(travaux > 0);
        var sorties = Regles.CompteSorties(_c.Sorties);
        BadgeMachines.Value = sorties;
        BadgeMachines.Visibility = Vis(sorties > 0);
    }

    private async void Navigation_SelectionChanged(NavigationView sender, NavigationViewSelectionChangedEventArgs args)
    {
        if (_synchro) return;
        if (args.SelectedItem is not NavigationViewItem { Tag: string tag }) return;
        var rubrique = tag switch
        {
            "Machines" => Rubrique.Machines,
            "Systeme" => Rubrique.Systeme,
            _ => Rubrique.Travaux,
        };
        await Proteger(() => _c.ChoisirRubrique(rubrique));
    }

    // ------------------------------------------------------------ Tableau des travaux

    private void MettreAJourTableau()
    {
        var cible = _c.TravauxAffiches;
        Reconcilier(cible);

        // Sélection du contrôleur (par exemple après un envoi) : on la reflète dans la liste.
        var voulue = _c.Selection is null ? null : _lignes.FirstOrDefault(l => l.Id == _c.Selection);
        if (!ReferenceEquals(ListeTravaux.SelectedItem, voulue))
        {
            ListeTravaux.SelectedItem = voulue;
            if (voulue is not null) ListeTravaux.ScrollIntoView(voulue);
        }

        if (_c.Travaux.Count == 0)
        {
            TitreVide.Text = "Aucun travail";
            TexteVide.Text = "Glissez un fichier ici ou utilisez le bouton « Envoyer » pour en envoyer un au RIP.";
            VideTravaux.Visibility = Visibility.Visible;
        }
        else if (cible.Count == 0)
        {
            TitreVide.Text = "Aucun résultat";
            TexteVide.Text = $"Aucun travail ne correspond à « {_c.Recherche.Trim()} ».";
            VideTravaux.Visibility = Visibility.Visible;
        }
        else
        {
            VideTravaux.Visibility = Visibility.Collapsed;
        }

        MettreAJourEnTetes();
    }

    /// <summary>Aligne la collection affichée sur la liste voulue sans la remplacer (sélection et défilement gardés).</summary>
    private void Reconcilier(IReadOnlyList<Travail> cible)
    {
        for (var i = 0; i < cible.Count; i++)
        {
            var t = cible[i];
            if (i < _lignes.Count && _lignes[i].Id == t.Id)
            {
                _lignes[i].Mettre(t);
                continue;
            }
            var existant = -1;
            for (var j = i + 1; j < _lignes.Count; j++)
            {
                if (_lignes[j].Id == t.Id) { existant = j; break; }
            }
            if (existant >= 0)
            {
                _lignes.Move(existant, i);
                _lignes[i].Mettre(t);
            }
            else
            {
                _lignes.Insert(i, new LigneTravail(t));
            }
        }
        while (_lignes.Count > cible.Count) _lignes.RemoveAt(_lignes.Count - 1);
    }

    private void MettreAJourEnTetes()
    {
        void Regler(Button bouton, string titre, ColonneTri colonne)
        {
            var fleche = _c.ColonneTri == colonne ? (_c.TriDecroissant ? " ▼" : " ▲") : "";
            bouton.Content = titre + fleche;
        }
        Regler(EnTeteNom, "Nom", ColonneTri.Nom);
        Regler(EnTeteDossier, "Dossier", ColonneTri.Dossier);
        Regler(EnTetePages, "Pages", ColonneTri.Pages);
        Regler(EnTeteStatut, "Statut", ColonneTri.Statut);
        Regler(EnTeteDate, "Date", ColonneTri.Date);
    }

    private void EnTete_Click(object sender, RoutedEventArgs e)
    {
        if (sender is Button { Tag: string nom } && Enum.TryParse<ColonneTri>(nom, out var colonne))
            _c.Trier(colonne);
    }

    private void ListeTravaux_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_synchro) return;
        var ligne = ListeTravaux.SelectedItem as LigneTravail;
        _c.Selectionner(ligne?.Id);
    }

    /// <summary>Remplit toute la largeur de la ligne et donne son menu contextuel (clic droit).</summary>
    private void ListeTravaux_ContainerContentChanging(ListViewBase sender, ContainerContentChangingEventArgs args)
    {
        if (args.ItemContainer is not ListViewItem conteneur) return;
        conteneur.HorizontalContentAlignment = HorizontalAlignment.Stretch;
        if (args.Item is not LigneTravail ligne) return;

        var menu = new MenuFlyout();
        var relancer = new MenuFlyoutItem { Text = "Relancer le rendu" };
        relancer.Click += async (_, _) => await Proteger(() => _c.Relancer(ligne.Travail));
        var supprimer = new MenuFlyoutItem { Text = "Supprimer…" };
        supprimer.Click += async (_, _) => await ConfirmerSuppression(ligne.Travail);
        // La ligne est gardée d'une actualisation à l'autre (Mettre) : l'état des actions se
        // calcule à l'ouverture du menu, sur le statut du moment.
        menu.Opening += (_, _) =>
        {
            var libre = !ligne.Travail.Statut.Occupe();
            relancer.IsEnabled = libre;
            supprimer.IsEnabled = libre;
        };
        menu.Items.Add(relancer);
        menu.Items.Add(new MenuFlyoutSeparator());
        menu.Items.Add(supprimer);
        conteneur.ContextFlyout = menu;
    }

    // ------------------------------------------------------------ Barre de commandes

    private void MettreAJourCommandes()
    {
        var travaux = _c.Rubrique == Rubrique.Travaux;
        var t = travaux ? _c.TravailSelectionne : null;
        var machines = t is null ? Array.Empty<Machine>() : _c.MachinesPour(t).ToArray();

        BoutonImprimer.Label = t?.DecoupeSeule == true ? "Découper" : "Imprimer";
        BoutonImprimer.IsEnabled = t is { Statut: StatutTravail.Termine } && machines.Length > 0;

        var peutBasculer = t is not null && (_c.PeutSuspendre(t) || _c.PeutReprendre(t));
        var reprendre = t is not null && !_c.PeutSuspendre(t) && _c.PeutReprendre(t);
        BoutonSuspendre.Label = reprendre ? "Reprendre" : "Suspendre";
        BoutonSuspendre.Icon = new SymbolIcon(reprendre ? Symbol.Play : Symbol.Pause);
        BoutonSuspendre.IsEnabled = peutBasculer;
        ToolTipService.SetToolTip(BoutonSuspendre, reprendre
            ? "Reprendre les sorties suspendues du travail"
            : "Suspendre les sorties en attente du travail");

        BoutonEpreuve.IsEnabled = t is { Statut: StatutTravail.Termine };
    }

    private async void Envoyer_Click(object sender, RoutedEventArgs e)
    {
        await Proteger(ChoisirFichiers);
    }

    private void MenuImprimer_Opening(object sender, object e)
    {
        RemplirMenuImprimer(MenuImprimer, detaille: false);
    }

    private async void Suspendre_Click(object sender, RoutedEventArgs e)
    {
        if (_c.TravailSelectionne is { } t) await Proteger(() => _c.BasculerSuspension(t));
    }

    private async void Epreuve_Click(object sender, RoutedEventArgs e)
    {
        if (_c.TravailSelectionne is { } t) await Proteger(() => EpreuveTiff(t, _c.PageValide(t)));
    }

    private void Inspecteur_Bascule(object sender, RoutedEventArgs e)
    {
        if (_synchro) return;
        MettreAJour();
    }

    private void Recherche_TextChanged(AutoSuggestBox sender, AutoSuggestBoxTextChangedEventArgs args)
    {
        if (_synchro) return;
        if (args.Reason == AutoSuggestionBoxTextChangeReason.UserInput) _c.ChoisirRecherche(sender.Text);
    }

    private async void Actualiser_Click(object sender, RoutedEventArgs e)
    {
        await Proteger(_c.Rafraichir);
        // Actualiser redemande aussi un aperçu resté en échec (réseau revenu).
        if (_c.Rubrique == Rubrique.Travaux) MettreAJourInspecteur(true);
    }

    private void Administration_Click(object sender, RoutedEventArgs e)
    {
        OuvrirAdministration();
    }

    private void ChangerServeur_Click(object sender, RoutedEventArgs e) => _c.ChangerServeur();

    private async void Deconnecter_Click(object sender, RoutedEventArgs e)
    {
        await Proteger(_c.Deconnecter);
    }

    private async void APropos_Click(object sender, RoutedEventArgs e)
    {
        await Proteger(AfficherAPropos);
    }

    /// <summary>Documentation publique, section du client Windows, dans le navigateur par défaut.</summary>
    private async void Aide_Click(object sender, RoutedEventArgs e)
    {
        await Proteger(async () =>
        {
            await Windows.System.Launcher.LaunchUriAsync(new Uri("https://e-ko.rip/documentation/clients/#client-windows"));
        });
    }
}
