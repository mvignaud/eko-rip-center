# Construit le client Windows autonome, le zip « E-KO-RIP-Center-windows-<arch>-<version>.zip »
# puis l'installateur « E-KO-RIP-Center_<version>_<arch>-setup.exe » (Inno Setup, installeur.iss).
# À lancer sur un PC Windows avec le SDK .NET 8 (Visual Studio inutile) et Inno Setup 6
# (winget install JRSoftware.InnoSetup). Sortie dans clients/windows/dist/ (ou -Sortie).
#
#   .\construire.ps1                       # x64
#   .\construire.ps1 -Architecture arm64   # ou x86
#   .\construire.ps1 -Sortie D:\livraison
#   .\construire.ps1 -SansInstallateur     # zip seul
#
# La version est lue dans le Cargo.toml racine ([workspace.package] version), comme pour le .dmg
# Mac ; elle est passée à dotnet publish, donc l'écran Système affiche la même.
[CmdletBinding()]
param(
    [ValidateSet('x64', 'arm64', 'x86')]
    [string]$Architecture = 'x64',
    [string]$Sortie,
    [switch]$SansInstallateur
)
$ErrorActionPreference = 'Stop'

$ici = $PSScriptRoot
$racine = (Resolve-Path (Join-Path $ici '..\..')).Path
if (-not $Sortie) { $Sortie = Join-Path $ici 'dist' }

# Version : première ligne « version = "x.y.z" » de la section [workspace.package].
$version = $null
$dedans = $false
foreach ($ligne in Get-Content (Join-Path $racine 'Cargo.toml')) {
    if ($ligne -match '^\[workspace\.package\]') { $dedans = $true; continue }
    if ($ligne -match '^\[') { $dedans = $false }
    if ($dedans -and $ligne -match '^version\s*=\s*"([^"]+)"') { $version = $Matches[1]; break }
}
if (-not $version) { throw "construire.ps1 : version introuvable dans $racine\Cargo.toml" }
Write-Host "==> E-KO RIP Center $version ($Architecture)"

# 1. Tests de la logique.
& dotnet test (Join-Path $ici 'EkoRip.Coeur.Tests') --nologo
if ($LASTEXITCODE -ne 0) { throw 'construire.ps1 : tests en échec.' }

# 2. Publication autonome (runtime .NET et Windows App SDK livrés avec l'exe).
$publication = Join-Path ([IO.Path]::GetTempPath()) "eko-rip-center-$([guid]::NewGuid().ToString('N'))"
& dotnet publish (Join-Path $ici 'EkoRip.App') -c Release "-p:Platform=$Architecture" `
    -r "win-$Architecture" "-p:Version=$version" -o $publication --nologo
if ($LASTEXITCODE -ne 0) { throw 'construire.ps1 : échec de dotnet publish.' }
if (-not (Test-Path (Join-Path $publication 'EkoRipCenter.exe'))) { throw 'construire.ps1 : EkoRipCenter.exe absent.' }

# 3. Zip : un dossier « E-KO RIP Center » à la racine de l'archive.
try {
    $atelier = Join-Path ([IO.Path]::GetTempPath()) "eko-rip-zip-$([guid]::NewGuid().ToString('N'))"
    $dossier = Join-Path $atelier 'E-KO RIP Center'
    New-Item -ItemType Directory -Force $atelier | Out-Null
    Move-Item $publication $dossier

    New-Item -ItemType Directory -Force $Sortie | Out-Null
    $zip = Join-Path (Resolve-Path $Sortie).Path "E-KO-RIP-Center-windows-$Architecture-$version.zip"
    if (Test-Path $zip) { Remove-Item $zip -Force }
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    [IO.Compression.ZipFile]::CreateFromDirectory($dossier, $zip, [IO.Compression.CompressionLevel]::Optimal, $true)
    Write-Host "==> $zip ($([math]::Round((Get-Item $zip).Length / 1MB, 1)) Mo)"

    # 4. Installateur Inno Setup, à partir du même dossier publié.
    if (-not $SansInstallateur) {
        $iscc = @(
            (Get-Command ISCC.exe -ErrorAction SilentlyContinue).Source,
            (Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 6\ISCC.exe'),
            (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe'),
            (Join-Path $env:ProgramFiles 'Inno Setup 6\ISCC.exe')
        ) | Where-Object { $_ -and (Test-Path $_) } | Select-Object -First 1
        if (-not $iscc) { throw 'construire.ps1 : Inno Setup 6 introuvable (winget install JRSoftware.InnoSetup), ou -SansInstallateur.' }
        $sortieAbsolue = (Resolve-Path $Sortie).Path
        & $iscc /Q "/DVersion=$version" "/DSource=$dossier" "/DSortie=$sortieAbsolue" "/DArchitecture=$Architecture" (Join-Path $ici 'installeur.iss')
        if ($LASTEXITCODE -ne 0) { throw 'construire.ps1 : échec de la compilation de l''installateur.' }
        $setup = Join-Path $sortieAbsolue "E-KO-RIP-Center_${version}_$Architecture-setup.exe"
        Write-Host "==> $setup ($([math]::Round((Get-Item $setup).Length / 1MB, 1)) Mo)"
    }
}
finally {
    if (Test-Path $publication) { Remove-Item $publication -Recurse -Force }
    if ($atelier -and (Test-Path $atelier)) { Remove-Item $atelier -Recurse -Force }
}
