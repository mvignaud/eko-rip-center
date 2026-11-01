using EkoRip.Coeur;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Windows.Graphics;

namespace EkoRip.Client;

/// <summary>
/// Fenêtre « Administration… » : la racine du serveur dans un WebView2. Le cookie de session de
/// l'API est recopié dans le WebView2 (sinon le serveur redemanderait l'identifiant), et le
/// User-Agent porte la signature <c>EKO-RIP-Client/…</c>. Fenêtre sans XAML : un seul contrôle.
/// </summary>
public sealed class FenetreAdministration : Window
{
    private readonly WebView2 _vue = new();
    private readonly Api _api;

    public FenetreAdministration(Api api)
    {
        _api = api;
        Title = "Administration — E-KO RIP Center";
        try
        {
            AppWindow.Resize(new SizeInt32(1200, 820));
            AppWindow.SetIcon(Path.Combine(AppContext.BaseDirectory, "Assets", "icone.ico"));
        }
        catch (Exception e)
        {
            Journal.Ecrire(e);
        }
        Content = _vue;
        _ = Charger();
    }

    private async Task Charger()
    {
        try
        {
            await _vue.EnsureCoreWebView2Async();
            var web = _vue.CoreWebView2;
            web.Settings.UserAgent = "Mozilla/5.0 " + VersionClient.Agent;
            // Liens target=_blank (bouton « Aide » de l'interface, liens externes) : dans le
            // navigateur par défaut plutôt que dans une fenêtre WebView2 sans session ni barre.
            web.NewWindowRequested += (_, args) =>
            {
                args.Handled = true;
                if (Uri.TryCreate(args.Uri, UriKind.Absolute, out var lien)
                    && (lien.Scheme == Uri.UriSchemeHttps || lien.Scheme == Uri.UriSchemeHttp))
                {
                    _ = Windows.System.Launcher.LaunchUriAsync(lien);
                }
            };

            var racine = new Uri(_api.Racine + "/");
            if (_api.CookieSession() is { } cookie)
            {
                var copie = web.CookieManager.CreateCookie(cookie.Name, cookie.Value, racine.Host, "/");
                copie.IsHttpOnly = cookie.HttpOnly;
                copie.IsSecure = cookie.Secure;
                web.CookieManager.AddOrUpdateCookie(copie);
            }
            _vue.Source = racine;
        }
        catch (Exception e)
        {
            Journal.Ecrire(e);
            Content = new TextBlock
            {
                Text = "Impossible d'ouvrir l'administration : le composant WebView2 est introuvable ou n'a pas démarré.\n\n" + e.Message,
                Margin = new Thickness(24),
                TextWrapping = TextWrapping.Wrap,
            };
        }
    }
}
