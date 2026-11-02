; Installateur Windows d'E-KO RIP Center (Inno Setup 6).
;
; Appelé par construire.ps1 après `dotnet publish` :
;   ISCC.exe /DVersion=<x.y.z> /DSource=<dossier publié> /DSortie=<dossier> /DArchitecture=x64 installeur.iss
;
; Installation par utilisateur, sans droits administrateur, dans
; %LOCALAPPDATA%\Programs\E-KO RIP Center. Sortie : E-KO-RIP-Center_<version>_<arch>-setup.exe.
; Les réglages de l'utilisateur (%LOCALAPPDATA%\E-KO RIP Center) sont gardés à la désinstallation.

#ifndef Version
  #error Passer /DVersion=<x.y.z> (lu dans le Cargo.toml racine par construire.ps1)
#endif
#ifndef Source
  #error Passer /DSource=<dossier de dotnet publish>
#endif
#ifndef Sortie
  #define Sortie "dist"
#endif
#ifndef Architecture
  #define Architecture "x64"
#endif

#define Nom "E-KO RIP Center"
#define Exe "EkoRipCenter.exe"

[Setup]
; AppId fixe : c'est lui qui relie une mise à jour ou une désinstallation à l'installation en place.
AppId={{8081BE75-85F6-43EF-A295-7B25AFE10BE2}
AppName={#Nom}
AppVersion={#Version}
AppVerName={#Nom} {#Version}
AppPublisher=E-KO
AppPublisherURL=https://e-ko.rip
AppSupportURL=https://e-ko.rip
VersionInfoVersion={#Version}
VersionInfoProductName={#Nom}
VersionInfoDescription=Installateur de {#Nom}

; Par utilisateur, sans élévation : {userpf} = %LOCALAPPDATA%\Programs.
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=
DefaultDirName={userpf}\{#Nom}
DisableDirPage=yes
DefaultGroupName={#Nom}
DisableProgramGroupPage=yes
UsedUserAreasWarning=no

#if Architecture == "x64"
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
#elif Architecture == "arm64"
ArchitecturesAllowed=arm64
ArchitecturesInstallIn64BitMode=arm64
#endif
MinVersion=10.0.17763

SetupIconFile=EkoRip.App\Assets\icone.ico
UninstallDisplayIcon={app}\{#Exe}
UninstallDisplayName={#Nom}
WizardStyle=modern
WizardSmallImageFile=EkoRip.App\Assets\icone-256.png
ShowLanguageDialog=no
; Le client ouvert pendant une mise à jour est fermé puis relancé.
CloseApplications=yes
RestartApplications=no

OutputDir={#Sortie}
OutputBaseFilename=E-KO-RIP-Center_{#Version}_{#Architecture}-setup
Compression=lzma2/max
SolidCompression=yes

[Languages]
Name: "fr"; MessagesFile: "compiler:Languages\French.isl"

[Tasks]
Name: "bureau"; Description: "Créer un raccourci sur le Bureau"; GroupDescription: "Raccourcis :"; Flags: unchecked

[Files]
Source: "{#Source}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[InstallDelete]
; Une mise à jour remplace tout le dossier : pas de reste d'une version précédente.
Type: filesandordirs; Name: "{app}\*"

[Icons]
Name: "{autoprograms}\{#Nom}"; Filename: "{app}\{#Exe}"; IconFilename: "{app}\Assets\icone.ico"; Comment: "Client du RIP d'impression grand format"
Name: "{autodesktop}\{#Nom}"; Filename: "{app}\{#Exe}"; IconFilename: "{app}\Assets\icone.ico"; Tasks: bureau

[Run]
Filename: "{app}\{#Exe}"; Description: "Lancer {#Nom}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
Type: filesandordirs; Name: "{app}"
