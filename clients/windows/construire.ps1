# Construit le client Windows autonome puis le zip « E-KO-RIP-Center-windows-<arch>-<version>.zip ».
# À lancer sur un PC Windows avec le SDK .NET 8 (Visual Studio inutile). Sortie dans
# clients/windows/dist/ (ou le dossier donné par -Sortie).
#
#   .\construire.ps1                       # x64
#   .\construire.ps1 -Architecture arm64   # ou x86
#   .\construire.ps1 -Sortie D:\livraison
#
# La version est lue dans le Cargo.toml racine ([workspace.package] version), comme pour le .dmg
# Mac ; elle est passée à dotnet publish, donc l'écran Système affiche la même.
[CmdletBinding()]
param(
    [ValidateSet('x64', 'arm64', 'x86')]
    [string]$Architecture = 'x64',
    [string]$Sortie
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
}
finally {
    if (Test-Path $publication) { Remove-Item $publication -Recurse -Force }
    if ($atelier -and (Test-Path $atelier)) { Remove-Item $atelier -Recurse -Force }
}

$taille = [math]::Round((Get-Item $zip).Length / 1MB, 1)
Write-Host "==> $zip ($taille Mo)"
