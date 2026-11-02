using EkoRip.Coeur;
using Microsoft.UI.Xaml;

namespace EkoRip.Client;

/// <summary>Point d'entrée : crée le contrôleur (toute la logique est dans EkoRip.Coeur) et la fenêtre.</summary>
public partial class App : Application
{
    private MainWindow? _fenetre;
    private Controleur? _controleur;

    public App()
    {
        InitializeComponent();
        UnhandledException += (_, e) => Journal.Ecrire(e.Exception);
    }

    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        // WebView2 (fenêtre Administration) range son profil ici : le dossier de l'exécutable
        // peut être en lecture seule (Program Files).
        try
        {
            Environment.SetEnvironmentVariable(
                "WEBVIEW2_USER_DATA_FOLDER",
                Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "E-KO RIP Center", "WebView2"));
        }
        catch (Exception e)
        {
            Journal.Ecrire(e);
        }

        var chemin = Parametres.CheminParDefaut();
        var parametres = Parametres.Charger(chemin);
        _controleur = new Controleur(parametres, p => p.Enregistrer(chemin));

        _fenetre = new MainWindow(_controleur);
        NotificationsSysteme.Initialiser(_fenetre);
        _controleur.NotificationAEnvoyer += NotificationsSysteme.Envoyer;
        _fenetre.Activate();
    }
}

/// <summary>Trace des erreurs non prévues dans %LOCALAPPDATA%\E-KO RIP Center\erreurs.log.</summary>
internal static class Journal
{
    public static void Ecrire(Exception e)
    {
        try
        {
            var dossier = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "E-KO RIP Center");
            Directory.CreateDirectory(dossier);
            File.AppendAllText(Path.Combine(dossier, "erreurs.log"), $"{DateTime.Now:s} {e}{Environment.NewLine}");
        }
        catch (Exception)
        {
            // Pas de journal possible : tant pis.
        }
    }
}
