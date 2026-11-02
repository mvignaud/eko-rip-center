using System.Text.Json;
using EkoRip.Coeur;
using Microsoft.UI.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;

namespace EkoRip.Client;

// Machines, Système, barre d'état, et les petits éléments d'interface construits en code.
public sealed partial class MainWindow
{
    private string? _cleMachines;
    private string? _cleSysteme;

    // ------------------------------------------------------------ Éléments d'interface

    private TextBlock Texte(string texte, string? style = null)
    {
        var t = new TextBlock { Text = texte, TextWrapping = TextWrapping.Wrap };
        if (style is not null) t.Style = (Style)Racine.Resources[style];
        return t;
    }

    private FontIcon Icone(string glyphe, string? style = null, double taille = 16)
    {
        var i = new FontIcon { Glyph = glyphe, FontSize = taille, VerticalAlignment = VerticalAlignment.Top };
        if (style is not null) i.Style = (Style)Racine.Resources[style];
        return i;
    }

    /// <summary>Titre à gauche (grisé), valeur à droite.</summary>
    private UIElement LigneInfo(string titre, string valeur)
    {
        var g = new Grid { ColumnSpacing = 12 };
        g.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(140) });
        g.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        g.Children.Add(Texte(titre, "TexteSecondaire"));
        var v = new TextBlock { Text = valeur, TextWrapping = TextWrapping.Wrap, IsTextSelectionEnabled = true };
        Grid.SetColumn(v, 1);
        g.Children.Add(v);
        return g;
    }

    private static UIElement Separateur() =>
        new Border { Height = 1, Opacity = 0.25, Background = new SolidColorBrush(Microsoft.UI.Colors.Gray), Margin = new Thickness(0, 4, 0, 4) };

    private Border Carte(UIElement contenu)
    {
        var c = new Border { Child = contenu };
        c.Style = (Style)Racine.Resources["CarteStyle"];
        return c;
    }

    private Border Puce(string texte)
    {
        var p = new Border { Child = new TextBlock { Text = texte, FontSize = 12 } };
        p.Style = (Style)Racine.Resources["PuceStyle"];
        return p;
    }

    // ------------------------------------------------------------ Une sortie de la file

    private static string GlypheSortie(EtatSortie e) => e switch
    {
        EtatSortie.EnCours => "",
        EtatSortie.EnAttente => "",
        EtatSortie.Suspendue => "",
        EtatSortie.Terminee => "",
        EtatSortie.Echec => "",
        EtatSortie.Annulee => "",
        EtatSortie.Imbriquee => "",
        _ => "",
    };

    private static string? StyleSortie(EtatSortie e) => e switch
    {
        EtatSortie.EnCours => "IconeAccent",
        EtatSortie.Terminee => "IconeSucces",
        EtatSortie.Echec => "IconeCritique",
        EtatSortie.Suspendue => "IconeAttention",
        _ => null,
    };

    /// <summary>Nom, état, raison d'attente et menu d'actions d'une sortie.</summary>
    private UIElement LigneSortie(Sortie s, bool avecMachine)
    {
        var g = new Grid { ColumnSpacing = 8 };
        g.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        g.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        g.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });

        g.Children.Add(Icone(GlypheSortie(s.Etat), StyleSortie(s.Etat)));

        var textes = new StackPanel { Spacing = 2 };
        var titre = Texte($"{s.Nom} · page {s.Page}");
        titre.TextWrapping = TextWrapping.NoWrap;
        titre.TextTrimming = TextTrimming.CharacterEllipsis;
        ToolTipService.SetToolTip(titre, s.Nom);
        textes.Children.Add(titre);

        var detail = s.Etat.Libelle();
        if (avecMachine) detail += " · " + _c.NomMachine(s.Machine);
        if (s.DureeMin is { } duree && duree > 0 && s.Etat.Active()) detail += $" · {(int)Math.Ceiling(duree)} min";
        var ligneDetail = Texte(detail, "TexteSecondaire");
        ligneDetail.FontSize = 12;
        textes.Children.Add(ligneDetail);

        if (s.RaisonAttente is { } code && code != "attente_operateur")
        {
            var raison = Libelles.Raison(code);
            var t = Texte(raison.Titre, "TexteAttention");
            t.FontSize = 12;
            if (raison.Aide.Length > 0) ToolTipService.SetToolTip(t, raison.Aide);
            textes.Children.Add(t);
        }
        if (s.Etat == EtatSortie.Echec)
        {
            var erreur = Texte(Libelles.Erreur(s.CodeErreur ?? "sortie_echouee", s.Erreur), "TexteCritique");
            erreur.FontSize = 12;
            erreur.MaxLines = 3;
            textes.Children.Add(erreur);
        }
        Grid.SetColumn(textes, 1);
        g.Children.Add(textes);

        var droite = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 4, VerticalAlignment = VerticalAlignment.Top };
        if (s.Etat == EtatSortie.EnCours) droite.Children.Add(new ProgressRing { Width = 16, Height = 16, IsActive = true });
        if (s.Actions.Count > 0)
        {
            var menu = new MenuFlyout();
            foreach (var action in s.Actions)
            {
                var a = action;
                var element = new MenuFlyoutItem { Text = a.Libelle() };
                element.Click += async (_, _) => await Proteger(() => _c.Agir(s, a));
                menu.Items.Add(element);
            }
            var bouton = new Button
            {
                Content = new FontIcon { Glyph = "", FontSize = 14 },
                Flyout = menu,
                Padding = new Thickness(6),
                Background = new SolidColorBrush(Microsoft.UI.Colors.Transparent),
                BorderThickness = new Thickness(0),
            };
            ToolTipService.SetToolTip(bouton, "Actions sur cette sortie");
            droite.Children.Add(bouton);
        }
        Grid.SetColumn(droite, 2);
        g.Children.Add(droite);
        return g;
    }

    // ------------------------------------------------------------ Machines

    private void MettreAJourMachines()
    {
        var cle = JsonSerializer.Serialize(new { m = _c.Machines, s = _c.Sorties }, JsonServeur.Options);
        VideMachines.Visibility = Vis(_c.Machines.Count == 0);
        if (cle == _cleMachines) return;
        _cleMachines = cle;

        ListeMachines.Children.Clear();
        foreach (var machine in _c.Machines) ListeMachines.Children.Add(CarteMachine(machine));
    }

    private UIElement CarteMachine(Machine machine)
    {
        var sorties = _c.SortiesActivesPour(machine);
        var contenu = new StackPanel { Spacing = 12 };

        var entete = new Grid { ColumnSpacing = 12 };
        entete.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        entete.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        entete.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        entete.Children.Add(Icone(machine.Imprime ? "" : "", "IconeAccent", 22));
        var noms = new StackPanel { Spacing = 2 };
        noms.Children.Add(new TextBlock { Text = machine.Nom, FontWeight = FontWeights.SemiBold, FontSize = 16 });
        noms.Children.Add(Texte(Libelles.Modele(machine.Modele), "TexteSecondaire"));
        Grid.SetColumn(noms, 1);
        entete.Children.Add(noms);
        var travaille = sorties.Any(s => s.Etat == EtatSortie.EnCours);
        var etat = Texte(travaille ? "Au travail" : "Libre", travaille ? "TexteSucces" : "TexteSecondaire");
        etat.FontSize = 12;
        Grid.SetColumn(etat, 2);
        entete.Children.Add(etat);
        contenu.Children.Add(entete);

        var infos = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8 };
        foreach (var operation in machine.Operations) infos.Children.Add(Puce(operation));
        if (machine.LaizeMm is { } laize)
        {
            var t = Texte($"Laize {(int)Math.Round(laize)} mm", "TexteSecondaire");
            t.FontSize = 12;
            t.VerticalAlignment = VerticalAlignment.Center;
            infos.Children.Add(t);
        }
        if (machine.Pool is { } pool)
        {
            var t = Texte($"Pool {pool}", "TexteSecondaire");
            t.FontSize = 12;
            t.VerticalAlignment = VerticalAlignment.Center;
            infos.Children.Add(t);
        }
        contenu.Children.Add(infos);
        contenu.Children.Add(Separateur());

        if (sorties.Count == 0)
        {
            contenu.Children.Add(Texte("Aucune sortie en attente.", "TexteSecondaire"));
        }
        else
        {
            var liste = new StackPanel { Spacing = 8 };
            foreach (var s in sorties) liste.Children.Add(LigneSortie(s, avecMachine: false));
            contenu.Children.Add(liste);
        }
        return Carte(contenu);
    }

    // ------------------------------------------------------------ Système

    private void MettreAJourSysteme()
    {
        var s = _c.Systeme;
        AttenteSysteme.Visibility = Vis(s is null);
        ContenuSysteme.Visibility = Vis(s is not null);
        if (s is null)
        {
            _cleSysteme = null;
            return;
        }
        var cle = JsonSerializer.Serialize(new { s, _c.Adresse, u = _c.Session }, JsonServeur.Options);
        if (cle == _cleSysteme) return;
        _cleSysteme = cle;

        ContenuSysteme.Children.Clear();

        var serveur = new StackPanel { Spacing = 8 };
        serveur.Children.Add(LigneInfo("Adresse", _c.Adresse));
        serveur.Children.Add(LigneInfo("Version du serveur", s.Version ?? "inconnue"));
        serveur.Children.Add(LigneInfo("Version du client", VersionClient.Numero));
        if (_c.Session?.Utilisateur is { } utilisateur) serveur.Children.Add(LigneInfo("Utilisateur", utilisateur));
        else if (_c.Session?.AccesLibre == true) serveur.Children.Add(LigneInfo("Accès", "Libre (aucun compte)"));
        ContenuSysteme.Children.Add(Section("Serveur", serveur));

        var disque = new StackPanel { Spacing = 8 };
        if (s.DisqueTotal is { } total && total > 0 && s.DisqueLibre is { } libre)
        {
            var occupe = (double)(total - libre) / total;
            var bas = libre < (s.SeuilLibre ?? 0);
            disque.Children.Add(new ProgressBar
            {
                Minimum = 0,
                Maximum = 100,
                Value = Math.Clamp(occupe, 0, 1) * 100,
                ShowError = bas,
            });
            disque.Children.Add(Texte($"{Regles.Octets(libre)} libres sur {Regles.Octets(total)}", bas ? "TexteCritique" : "TexteSecondaire"));
            if (bas)
            {
                disque.Children.Add(Texte(
                    $"Espace libre sous le seuil du serveur ({Regles.Octets(s.SeuilLibre)}) : les nouveaux travaux risquent d'être refusés.",
                    "TexteCritique"));
            }
        }
        else
        {
            disque.Children.Add(LigneInfo("Espace libre", "inconnu"));
        }
        disque.Children.Add(LigneInfo("Tampon de la file", Regles.Octets(s.Tampon)));
        if (s.ConservationJours is { } jours) disque.Children.Add(LigneInfo("Conservation", $"{jours} jours"));
        ContenuSysteme.Children.Add(Section("Disque des données", disque));

        var rendus = new StackPanel { Spacing = 8 };
        rendus.Children.Add(LigneInfo("En cours", (s.Rendus?.EnCours ?? 0).ToString()));
        rendus.Children.Add(LigneInfo("En attente", (s.Rendus?.EnAttente ?? 0).ToString()));
        if (s.Rendus?.Simultanes is { } n) rendus.Children.Add(LigneInfo("Simultanés au plus", n.ToString()));
        ContenuSysteme.Children.Add(Section("Rendus", rendus));

        var alertes = new StackPanel { Spacing = 8 };
        if (s.Alertes is { Count: > 0 } liste)
        {
            foreach (var a in liste)
            {
                var ligne = new Grid { ColumnSpacing = 8 };
                ligne.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
                ligne.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
                ligne.Children.Add(Icone("", "IconeAttention"));
                var textes = new StackPanel { Spacing = 2 };
                textes.Children.Add(Texte(a.Texte));
                var fichier = Texte(a.Fichier, "TexteSecondaire");
                fichier.FontSize = 12;
                fichier.IsTextSelectionEnabled = true;
                textes.Children.Add(fichier);
                Grid.SetColumn(textes, 1);
                ligne.Children.Add(textes);
                alertes.Children.Add(ligne);
            }
        }
        else
        {
            alertes.Children.Add(Texte("Aucune alerte.", "TexteSecondaire"));
        }
        ContenuSysteme.Children.Add(Section("Alertes de dossiers", alertes));
    }

    private UIElement Section(string titre, UIElement contenu)
    {
        var p = new StackPanel { Spacing = 10 };
        p.Children.Add(new TextBlock { Text = titre, FontWeight = FontWeights.SemiBold, FontSize = 16 });
        p.Children.Add(contenu);
        return Carte(p);
    }

    // ------------------------------------------------------------ Barre d'état

    /// <summary>Serveur injoignable, envois en cours (progression, annulation), annonce brève.</summary>
    private void MettreAJourBarreEtat()
    {
        var visible = _c.Envois.Count > 0 || _c.Annonce is not null || _c.ServeurHorsLigne is not null;
        BarreEtat.Visibility = Vis(visible);
        ContenuBarreEtat.Children.Clear();
        if (!visible) return;

        if (_c.ServeurHorsLigne is { } horsLigne)
        {
            var l = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8 };
            l.Children.Add(Icone("", "IconeAttention", 14));
            l.Children.Add(Texte(horsLigne, "TexteAttention"));
            ContenuBarreEtat.Children.Add(l);
        }

        foreach (var envoi in _c.Envois) ContenuBarreEtat.Children.Add(LigneEnvoi(envoi));

        if (_c.Annonce is { } annonce) ContenuBarreEtat.Children.Add(Texte(annonce, "TexteSecondaire"));
    }

    private UIElement LigneEnvoi(EnvoiEnCours envoi)
    {
        var g = new Grid { ColumnSpacing = 10 };
        g.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        g.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(220) });
        g.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        g.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        g.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });

        var glyphe = envoi.Erreur is not null ? "" : envoi.Termine ? "" : "";
        var style = envoi.Erreur is not null ? "IconeCritique" : envoi.Termine ? "IconeSucces" : "IconeAccent";
        g.Children.Add(Icone(glyphe, style, 14));

        var nom = new TextBlock { Text = envoi.Nom, TextTrimming = TextTrimming.CharacterEllipsis, VerticalAlignment = VerticalAlignment.Center };
        ToolTipService.SetToolTip(nom, envoi.Nom);
        Grid.SetColumn(nom, 1);
        g.Children.Add(nom);

        if (envoi.Erreur is { } erreur)
        {
            var t = Texte(erreur, "TexteCritique");
            t.MaxLines = 2;
            t.VerticalAlignment = VerticalAlignment.Center;
            Grid.SetColumn(t, 2);
            g.Children.Add(t);
            var fermer = new HyperlinkButton { Content = "Fermer" };
            fermer.Click += (_, _) => _c.Fermer(envoi);
            Grid.SetColumn(fermer, 4);
            g.Children.Add(fermer);
        }
        else if (envoi.Termine)
        {
            var t = Texte("Envoyé", "TexteSecondaire");
            t.VerticalAlignment = VerticalAlignment.Center;
            Grid.SetColumn(t, 2);
            g.Children.Add(t);
        }
        else
        {
            var barre = new ProgressBar { Minimum = 0, Maximum = 100, Value = envoi.Progression * 100, VerticalAlignment = VerticalAlignment.Center };
            Grid.SetColumn(barre, 2);
            g.Children.Add(barre);
            var pourcent = new TextBlock { Text = $"{(int)Math.Round(envoi.Progression * 100)} %", VerticalAlignment = VerticalAlignment.Center, Width = 44, TextAlignment = TextAlignment.Right };
            Grid.SetColumn(pourcent, 3);
            g.Children.Add(pourcent);
            var annuler = new Button
            {
                Content = new FontIcon { Glyph = "", FontSize = 12 },
                Padding = new Thickness(6),
                Background = new SolidColorBrush(Microsoft.UI.Colors.Transparent),
                BorderThickness = new Thickness(0),
            };
            ToolTipService.SetToolTip(annuler, "Annuler l'envoi");
            annuler.Click += (_, _) => _c.AnnulerEnvois();
            Grid.SetColumn(annuler, 4);
            g.Children.Add(annuler);
        }
        return g;
    }
}
