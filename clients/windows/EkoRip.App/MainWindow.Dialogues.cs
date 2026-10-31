using EkoRip.Coeur;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Windows.ApplicationModel.DataTransfer;
using Windows.Storage;
using Windows.Storage.Pickers;
using WinRT.Interop;

namespace EkoRip.Client;

// Boîtes de dialogue, sélecteurs de fichiers, glisser-déposer, fenêtre Administration.
public sealed partial class MainWindow
{
    private readonly SemaphoreSlim _verrouDialogue = new(1, 1);
    private bool _messageAffiche;
    private FenetreAdministration? _administration;

    // ------------------------------------------------------------ Dialogues

    /// <summary>
    /// Une seule ContentDialog peut être ouverte à la fois par fenêtre (sinon exception) : les
    /// demandes se mettent en file.
    /// </summary>
    private async Task<ContentDialogResult> Afficher(ContentDialog dialogue)
    {
        await _verrouDialogue.WaitAsync();
        try
        {
            dialogue.XamlRoot = Racine.XamlRoot;
            return await dialogue.ShowAsync();
        }
        finally
        {
            _verrouDialogue.Release();
        }
    }

    private async Task AfficherMessage(MessageUtilisateur message)
    {
        try
        {
            await Afficher(new ContentDialog
            {
                Title = message.Titre,
                Content = new TextBlock { Text = message.Texte, TextWrapping = TextWrapping.Wrap, IsTextSelectionEnabled = true },
                CloseButtonText = "OK",
                DefaultButton = ContentDialogButton.Close,
            });
        }
        catch (Exception e)
        {
            Journal.Ecrire(e);
        }
        finally
        {
            _messageAffiche = false;
            _c.FermerMessage();
        }
    }

    private async Task ConfirmerSuppression(Travail t)
    {
        var reponse = await Afficher(new ContentDialog
        {
            Title = $"Supprimer « {t.Nom} » ?",
            Content = new TextBlock { Text = "Le travail et ses fichiers seront effacés du serveur.", TextWrapping = TextWrapping.Wrap },
            PrimaryButtonText = "Supprimer",
            CloseButtonText = "Annuler",
            DefaultButton = ContentDialogButton.Close,
        });
        if (reponse == ContentDialogResult.Primary) await Proteger(() => _c.Supprimer(t));
    }

    private async Task AfficherAPropos()
    {
        await Afficher(new ContentDialog
        {
            Title = "E-KO RIP Center",
            Content = new StackPanel
            {
                Spacing = 4,
                Children =
                {
                    new TextBlock { Text = $"Version {VersionClient.Numero}" },
                    new TextBlock { Text = "Client Windows du RIP d'impression grand format." },
                },
            },
            CloseButtonText = "OK",
            DefaultButton = ContentDialogButton.Close,
        });
    }

    // ------------------------------------------------------------ Envoi

    private nint Fenetre() => WindowNative.GetWindowHandle(this);

    /// <summary>Bouton « Envoyer » : choix de fichiers, puis feuille d'envoi.</summary>
    private async Task ChoisirFichiers()
    {
        var selecteur = new FileOpenPicker
        {
            ViewMode = PickerViewMode.List,
            SuggestedStartLocation = PickerLocationId.DocumentsLibrary,
        };
        foreach (var extension in new[] { ".pdf", ".ai", ".svg", ".tif", ".tiff", ".png", ".jpg", ".jpeg" })
            selecteur.FileTypeFilter.Add(extension);
        // Sans le HWND de la fenêtre, un sélecteur WinUI 3 de bureau échoue.
        InitializeWithWindow.Initialize(selecteur, Fenetre());

        var fichiers = await selecteur.PickMultipleFilesAsync();
        if (fichiers is { Count: > 0 })
            _c.ProposerEnvoi(fichiers.Select(f => f.Path));
    }

    private void SurFichiersProposes(IReadOnlyList<string> fichiers)
    {
        _ = OuvrirFeuilleEnvoi(fichiers);
    }

    /// <summary>Feuille d'envoi : réglage à appliquer, et fiche facultative (client, commande, quantité).</summary>
    private async Task OuvrirFeuilleEnvoi(IReadOnlyList<string> fichiers)
    {
        try
        {
            var panneau = new StackPanel { Spacing = 14, MinWidth = 380 };

            var liste = new StackPanel { Spacing = 2 };
            foreach (var f in fichiers)
            {
                var ligne = new TextBlock { Text = Path.GetFileName(f), TextTrimming = TextTrimming.CharacterEllipsis };
                ToolTipService.SetToolTip(ligne, f);
                liste.Children.Add(ligne);
            }
            panneau.Children.Add(new TextBlock { Text = fichiers.Count > 1 ? $"Fichiers ({fichiers.Count})" : "Fichier", FontWeight = Microsoft.UI.Text.FontWeights.SemiBold });
            panneau.Children.Add(liste);

            ComboBox? reglages = null;
            if (_c.Reglages.Count == 0)
            {
                panneau.Children.Add(Texte("Aucun réglage enregistré : le serveur applique ses réglages par défaut.", "TexteSecondaire"));
            }
            else
            {
                reglages = new ComboBox { Header = "Réglage", HorizontalAlignment = HorizontalAlignment.Stretch };
                foreach (var r in _c.Reglages) reglages.Items.Add(r.Nom);
                var retenu = _c.Parametres.DernierReglage;
                reglages.SelectedItem = _c.Reglages.Any(r => r.Nom == retenu) ? retenu : _c.Reglages[0].Nom;
                panneau.Children.Add(reglages);
            }

            panneau.Children.Add(new TextBlock { Text = "Fiche travail (facultatif)", FontWeight = Microsoft.UI.Text.FontWeights.SemiBold });
            var client = new TextBox { Header = "Client" };
            var commande = new TextBox { Header = "Commande" };
            var quantite = new TextBox { Header = "Quantité" };
            var erreurQuantite = Texte("La quantité est un nombre entier, 100 000 au plus.", "TexteCritique");
            erreurQuantite.Visibility = Visibility.Collapsed;
            panneau.Children.Add(client);
            panneau.Children.Add(commande);
            panneau.Children.Add(quantite);
            panneau.Children.Add(erreurQuantite);

            var dialogue = new ContentDialog
            {
                Title = fichiers.Count > 1 ? $"Envoyer {fichiers.Count} fichiers" : "Envoyer un fichier",
                Content = new ScrollViewer { Content = panneau, MaxHeight = 520 },
                PrimaryButtonText = "Envoyer",
                CloseButtonText = "Annuler",
                DefaultButton = ContentDialogButton.Primary,
            };
            quantite.TextChanged += (_, _) =>
            {
                var valide = Regles.QuantiteValide(quantite.Text);
                erreurQuantite.Visibility = Vis(!valide);
                dialogue.IsPrimaryButtonEnabled = valide;
            };

            var reponse = await Afficher(dialogue);
            if (reponse == ContentDialogResult.Primary)
            {
                _c.LancerEnvoi(fichiers, reglages?.SelectedItem as string, client.Text, commande.Text, quantite.Text);
            }
        }
        catch (Exception e)
        {
            _c.Traiter(e);
        }
    }

    // ------------------------------------------------------------ Épreuve TIFF

    /// <summary>Télécharge l'épreuve TIFF d'une page puis propose de l'enregistrer.</summary>
    private async Task EpreuveTiff(Travail t, int page)
    {
        var temporaire = await _c.PreparerEpreuve(t, page);
        if (temporaire is null) return;
        try
        {
            var selecteur = new FileSavePicker
            {
                SuggestedStartLocation = PickerLocationId.DocumentsLibrary,
                SuggestedFileName = Path.GetFileNameWithoutExtension(Regles.NomEpreuve(t.Nom, page)),
            };
            selecteur.FileTypeChoices.Add("Image TIFF", new List<string> { ".tif" });
            InitializeWithWindow.Initialize(selecteur, Fenetre());

            StorageFile? destination = await selecteur.PickSaveFileAsync();
            if (destination is not null)
            {
                var chemin = destination.Path;
                // Le fichier peut peser plusieurs Go : la copie ne doit pas bloquer l'interface.
                await Task.Run(() => File.Copy(temporaire, chemin, overwrite: true));
                _c.Annoncer($"Épreuve TIFF enregistrée : {chemin}");
            }
        }
        catch (Exception e)
        {
            _c.Traiter(e);
        }
        finally
        {
            try { File.Delete(temporaire); }
            catch (IOException) { }
            catch (UnauthorizedAccessException) { }
        }
    }

    // ------------------------------------------------------------ Glisser-déposer

    private bool AccepteDepot(DragEventArgs e) =>
        _c.Phase == Phase.Connecte && e.DataView.Contains(StandardDataFormats.StorageItems);

    private void Racine_DragOver(object sender, DragEventArgs e)
    {
        if (!AccepteDepot(e)) return;
        e.AcceptedOperation = DataPackageOperation.Copy;
        e.DragUIOverride.Caption = "Envoyer au RIP";
        CadreDepot.Visibility = Visibility.Visible;
    }

    private void Racine_DragLeave(object sender, DragEventArgs e)
    {
        CadreDepot.Visibility = Visibility.Collapsed;
    }

    private async void Racine_Drop(object sender, DragEventArgs e)
    {
        CadreDepot.Visibility = Visibility.Collapsed;
        if (!AccepteDepot(e)) return;
        var report = e.GetDeferral();
        try
        {
            var elements = await e.DataView.GetStorageItemsAsync();
            var chemins = elements
                .OfType<StorageFile>()
                .Select(f => f.Path)
                .Where(p => !string.IsNullOrEmpty(p))
                .ToList();
            _c.ProposerEnvoi(chemins);
        }
        catch (Exception ex)
        {
            _c.Traiter(ex);
        }
        finally
        {
            report.Complete();
        }
    }

    // ------------------------------------------------------------ Administration

    /// <summary>
    /// Écrans d'administration (supports, règles, tarifs, pont ERP, sauvegarde…) pas encore natifs :
    /// l'écran complet du serveur dans une fenêtre à part (WebView2), avec la signature du client et
    /// le cookie de session recopié. Passerelle temporaire, à retirer quand ces écrans seront natifs.
    /// </summary>
    private void OuvrirAdministration()
    {
        var api = _c.Api;
        if (api is null || _c.Phase != Phase.Connecte) return;
        if (_administration is not null)
        {
            _administration.Activate();
            return;
        }
        var fenetre = new FenetreAdministration(api);
        fenetre.Closed += (_, _) => _administration = null;
        _administration = fenetre;
        fenetre.Activate();
    }
}
