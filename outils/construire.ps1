<#
    Construction du setup Windows LCOS (NSIS).
    Exemples :
        .\outils\construire.ps1
        .\outils\construire.ps1 -Niveau correctif
        .\outils\construire.ps1 -Version 1.1.0
#>
param(
    [ValidateSet('aucun', 'correctif', 'mineur')]
    [string]$Niveau = 'aucun',

    [string]$Version
)

$ErrorActionPreference = 'Stop'

$racine = Split-Path -Parent $PSScriptRoot

function Test-Outil {
    param([string]$Nom, [string]$Remede)

    if (-not (Get-Command $Nom -ErrorAction SilentlyContinue)) {
        Write-Output "MANQUANT : $Nom"
        Write-Output "  remede : $Remede"
        return $false
    }

    Write-Output "present : $Nom"
    return $true
}

Write-Output '--- outillage ---'
$complet = $true
if (-not (Test-Outil 'cargo' 'winget install --id Rustlang.Rustup --exact')) { $complet = $false }
if (-not (Test-Outil 'rustc' 'winget install --id Rustlang.Rustup --exact')) { $complet = $false }
if (-not $complet) { throw 'chaine d outils incomplete' }

$tauriInstalle = cargo install --list | Select-String -SimpleMatch 'tauri-cli'
if (-not $tauriInstalle) {
    Write-Output 'MANQUANT : tauri-cli'
    Write-Output '  remede : cargo install tauri-cli --version "^2" --locked'
    throw 'chaine d outils incomplete'
}
Write-Output 'present : tauri-cli'

if ($Version) {
    & (Join-Path $PSScriptRoot 'version.ps1') -Version $Version
}
elseif ($Niveau -ne 'aucun') {
    & (Join-Path $PSScriptRoot 'version.ps1') -Niveau $Niveau
}

$versionCourante = & (Join-Path $PSScriptRoot 'version.ps1') -Afficher
Write-Output "--- construction LCOS $versionCourante ---"

Push-Location $racine
try {
    cargo tauri build
    if ($LASTEXITCODE -ne 0) { throw "echec de compilation (code $LASTEXITCODE)" }
}
finally {
    Pop-Location
}

$dossierSetup = Join-Path $racine 'src-tauri\target\release\bundle\nsis'
$setup = Get-ChildItem -Path $dossierSetup -Filter '*-setup.exe' -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1

if (-not $setup) { throw "aucun setup produit dans $dossierSetup" }

Write-Output '--- setup pret ---'
Write-Output $setup.FullName
Write-Output ("taille : {0:N1} Mo" -f ($setup.Length / 1MB))
