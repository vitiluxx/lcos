<#
    Gestion de version LCOS - schema 1.MINEUR.CORRECTIF
    Source de verite : src-tauri/tauri.conf.json, repercutee sur src-tauri/Cargo.toml
#>
param(
    [ValidateSet('correctif', 'mineur')]
    [string]$Niveau = 'correctif',

    [string]$Version,

    [switch]$Afficher
)

$ErrorActionPreference = 'Stop'

$racine = Split-Path -Parent $PSScriptRoot
$cheminConfig = Join-Path $racine 'src-tauri\tauri.conf.json'
$cheminCargo = Join-Path $racine 'src-tauri\Cargo.toml'

if (-not (Test-Path $cheminConfig)) { throw "introuvable : $cheminConfig" }
if (-not (Test-Path $cheminCargo)) { throw "introuvable : $cheminCargo" }

$contenuConfig = Get-Content $cheminConfig -Raw -Encoding UTF8
$correspondance = [regex]::Match($contenuConfig, '"version"\s*:\s*"(\d+)\.(\d+)\.(\d+)"')

if (-not $correspondance.Success) { throw 'version absente de tauri.conf.json' }

$majeur = [int]$correspondance.Groups[1].Value
$mineur = [int]$correspondance.Groups[2].Value
$correctif = [int]$correspondance.Groups[3].Value
$actuelle = "$majeur.$mineur.$correctif"

if ($Afficher) {
    Write-Output $actuelle
    exit 0
}

if ($Version) {
    if ($Version -notmatch '^1\.\d+\.\d+$') { throw "version invalide : $Version (attendu 1.x.x)" }
    $nouvelle = $Version
}
else {
    if ($Niveau -eq 'mineur') {
        $nouvelle = "$majeur.$($mineur + 1).0"
    }
    else {
        $nouvelle = "$majeur.$mineur.$($correctif + 1)"
    }
}

$contenuConfig = [regex]::Replace($contenuConfig, '"version"\s*:\s*"\d+\.\d+\.\d+"', """version"": ""$nouvelle""", 1)
[System.IO.File]::WriteAllText($cheminConfig, $contenuConfig, (New-Object System.Text.UTF8Encoding $false))

$contenuCargo = Get-Content $cheminCargo -Raw -Encoding UTF8
$contenuCargo = [regex]::Replace($contenuCargo, '(?m)^version\s*=\s*"\d+\.\d+\.\d+"', "version = ""$nouvelle""", 1)
[System.IO.File]::WriteAllText($cheminCargo, $contenuCargo, (New-Object System.Text.UTF8Encoding $false))

Write-Output "version : $actuelle -> $nouvelle"
