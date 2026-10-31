# E-KO RIP Center pour Windows (WinUI 3)

Client natif Windows, lot 1 : mêmes écrans et mêmes règles que le client Mac (voir
`docs/clients-natifs.md`). C#, .NET 8, WinUI 3 (Windows App SDK 1.6), Windows 10 1809 ou plus,
mieux sous Windows 11 (Mica).

| Projet | Rôle |
|---|---|
| `EkoRip.Coeur/` | `net8.0` : API (`HttpClient`), modèles JSON, libellés français, tri, recherche, notifications, contrôleur d'état. Aucune interface, se compile et se teste partout. |
| `EkoRip.Coeur.Tests/` | xunit : adresse, décodage de réponses réelles, libellés, multipart, notifications, contrôleur contre un faux serveur. |
| `EkoRip.App/` | L'application WinUI 3 (`net8.0-windows10.0.19041.0`), non empaquetée (pas de MSIX) et autonome. |

## Prérequis (PC Windows)

- SDK .NET 8, **et** soit Visual Studio 2022 avec la charge de travail « Développement
  d'applications WinUI » (ou « Développement .NET Desktop »), soit seulement le SDK .NET plus le
  runtime : `winget install Microsoft.DotNet.SDK.8`, puis `winget install Microsoft.WindowsAppRuntime.1.6`
  (utile seulement si on retire `WindowsAppSDKSelfContained` du projet : par défaut le runtime est
  livré avec l'exécutable).
- WebView2 Runtime pour « Administration… » (déjà présent sous Windows 11).

## Construire et lancer

```powershell
cd clients\windows

dotnet test EkoRip.Coeur.Tests                        # logique, sans interface (marche aussi sous Linux)
dotnet build EkoRip.App -p:Platform=x64               # débogage
dotnet publish EkoRip.App -c Release -r win-x64       # dossier autonome

# Lancement
.\EkoRip.App\bin\x64\Debug\net8.0-windows10.0.19041.0\win-x64\EkoRipCenter.exe
.\EkoRip.App\bin\Release\net8.0-windows10.0.19041.0\win-x64\publish\EkoRipCenter.exe
```

(`win-arm64` et `win-x86` marchent de la même façon.) Dans Visual Studio : ouvrir `EkoRip.sln`,
plateforme **x64**, projet de démarrage `EkoRip.App`, F5.

`dotnet build EkoRip.sln` ne marche que sous Windows (le projet WinUI a besoin des outils XAML) ;
ailleurs, construire et tester `EkoRip.Coeur` et `EkoRip.Coeur.Tests` seuls.

## Réglages et traces

- Adresse du serveur, dernier utilisateur et dernier réglage d'envoi :
  `%LOCALAPPDATA%\E-KO RIP Center\parametres.json`.
- Erreurs non prévues : `%LOCALAPPDATA%\E-KO RIP Center\erreurs.log`.
- Profil WebView2 (fenêtre Administration) : `%LOCALAPPDATA%\E-KO RIP Center\WebView2`.

## Icône

`EkoRip.App/Assets/icone.ico` (16 à 256 px) est faite à partir de `clients/icone/icone-1024.png`
avec Pillow ; `icone-256.png` sert à l'écran de connexion.

## À savoir

- Glisser-déposer : un exécutable lancé « en administrateur » ne reçoit pas les fichiers glissés
  depuis l'Explorateur (règle Windows) ; le lancer normalement.
- Pas de MSIX pour l'instant : les notifications passent par l'enregistrement COM de Windows App SDK
  pour applications non empaquetées, sans icône ni nom propres tant que l'application n'est pas
  installée par un paquet.
