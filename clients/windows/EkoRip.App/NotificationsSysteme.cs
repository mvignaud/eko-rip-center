using EkoRip.Coeur;
using Microsoft.Windows.AppNotifications;
using Microsoft.Windows.AppNotifications.Builder;

namespace EkoRip.Client;

/// <summary>Notifications Windows (centre de notifications) via Windows App SDK.</summary>
internal static class NotificationsSysteme
{
    private static bool _disponible;

    public static void Initialiser(MainWindow fenetre)
    {
        try
        {
            // Un clic sur la notification ramène la fenêtre au premier plan.
            AppNotificationManager.Default.NotificationInvoked += (sender, args) =>
                fenetre.DispatcherQueue.TryEnqueue(() => fenetre.Activate());
            // Application non empaquetée : sans nom ni icône, Windows titre la notification avec le
            // nom de l'exécutable (« EkoRipCenter »).
            var icone = Path.Combine(AppContext.BaseDirectory, "Assets", "icone-256.png");
            try
            {
                AppNotificationManager.Default.Register("E-KO RIP Center", new Uri(icone));
            }
            catch (Exception e)
            {
                Journal.Ecrire(e);
                AppNotificationManager.Default.Register();
            }
            _disponible = true;
        }
        catch (Exception e)
        {
            // Pas de notifications (compte restreint, service désactivé) : le reste fonctionne.
            Journal.Ecrire(e);
            _disponible = false;
        }
    }

    /// <summary>Appelée sur le fil de l'interface (événement du contrôleur).</summary>
    public static void Envoyer(NotificationRip n)
    {
        if (!_disponible) return;
        try
        {
            var notification = new AppNotificationBuilder()
                .AddText(n.Titre)
                .AddText(n.Texte)
                .BuildNotification();
            AppNotificationManager.Default.Show(notification);
        }
        catch (Exception e)
        {
            Journal.Ecrire(e);
        }
    }

    public static void Arreter()
    {
        if (!_disponible) return;
        try
        {
            AppNotificationManager.Default.Unregister();
        }
        catch (Exception e)
        {
            Journal.Ecrire(e);
        }
        _disponible = false;
    }
}
