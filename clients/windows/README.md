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
dotnet publish EkoRip.App -c Release -p:Platform=x64  # dossier autonome

# Lancement
.\EkoRip.App\bin\x64\Debug\net8.0-windows10.0.19041.0\win-x64\EkoRipCenter.exe
.\EkoRip.App\bin\x64\Release\net8.0-windows10.0.19041.0\win-x64\publish\EkoRipCenter.exe

# Version livrable : tests, publish autonome à la version du Cargo.toml, puis dans dist\ :
#   E-KO-RIP-Center_<version>_x64-setup.exe   installateur (environ 45 Mo)
#   E-KO-RIP-Center-windows-x64-<version>.zip  même contenu à décompresser (environ 60 Mo)
.\construire.ps1                         # -Architecture arm64|x86, -Sortie <dossier>, -SansInstallateur
```

L'installateur demande Inno Setup 6 sur le poste qui construit (`winget install JRSoftware.InnoSetup`) ;
le script est `installeur.iss`. C'est le fichier `…_x64-setup.exe` qu'on joint à la release GitHub :
le bouton « Télécharger pour Windows » du site le prend dans la dernière release.

- Installation par utilisateur, **sans droits administrateur**, dans
  `%LOCALAPPDATA%\Programs\E-KO RIP Center` ; raccourci dans le menu Démarrer, et sur le Bureau
  si on coche la case. Assistant en français.
- Une nouvelle version s'installe par-dessus (même `AppId`) : le client ouvert est fermé, le
  dossier est remplacé.
- Désinstallation : Paramètres › Applications, ou `unins000.exe` dans le dossier d'installation.
  Elle retire le programme et les raccourcis, et garde les réglages de l'utilisateur
  (`%LOCALAPPDATA%\E-KO RIP Center`).
- Installation silencieuse : `E-KO-RIP-Center_<version>_x64-setup.exe /VERYSILENT [/TASKS=bureau]`.

Le zip se décompresse n'importe où et `EkoRipCenter.exe` se lance tel quel : runtime .NET et
Windows App SDK sont dedans, rien à installer.

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
- Pas de MSIX : les notifications passent par l'enregistrement COM de Windows App SDK pour
  applications non empaquetées, avec le nom « E-KO RIP Center » et l'icône Rosace passés à
  l'enregistrement (sans eux, Windows affiche le nom de l'exécutable).
