<#
    Inventaire complet de la caisse P70L avant changement de systeme.
    A lancer SUR LA CAISSE, sous Windows 7, en tant qu'administrateur,
    depuis la cle USB : tout est ecrit a cote du script.

    Compatible PowerShell 2.0 (Windows 7 d'origine).
#>

$ErrorActionPreference = 'Continue'

$racine = Split-Path -Parent $MyInvocation.MyCommand.Path
$destination = Join-Path $racine ('inventaire_caisse_' + (Get-Date -Format 'yyyyMMdd_HHmmss'))

New-Item -ItemType Directory -Path $destination -Force | Out-Null

function Ecrire-Etape {
    param([string]$Texte)
    Write-Host ''
    Write-Host ('=== ' + $Texte + ' ===')
}

function Tester-Administrateur {
    $identite = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identite)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

if (-not (Tester-Administrateur)) {
    Write-Host 'ARRET : relancer ce script en tant qu administrateur.'
    Write-Host 'Clic droit sur PowerShell > Executer en tant qu administrateur.'
    Read-Host 'Appuyer sur Entree pour fermer'
    exit 1
}

Write-Host ('Destination : ' + $destination)

Ecrire-Etape 'Materiel branche (hardware IDs)'
Get-WmiObject Win32_PnPEntity |
    Select-Object Name, DeviceID, Manufacturer, Service, Status |
    Sort-Object Name |
    Export-Csv -Path (Join-Path $destination 'materiel.csv') -NoTypeInformation -Encoding UTF8
Write-Host '  materiel.csv'

Ecrire-Etape 'Pilotes installes'
& driverquery.exe /v /fo csv | Out-File (Join-Path $destination 'pilotes.csv') -Encoding UTF8
Write-Host '  pilotes.csv'

Ecrire-Etape 'Paquets de pilotes tiers (les plus critiques)'
& pnputil.exe -e | Out-File (Join-Path $destination 'pilotes_tiers.txt') -Encoding UTF8
Write-Host '  pilotes_tiers.txt'

Ecrire-Etape 'Imprimantes et ports serie'
Get-WmiObject Win32_Printer |
    Select-Object Name, DriverName, PortName, Default, Shared |
    Export-Csv -Path (Join-Path $destination 'imprimantes.csv') -NoTypeInformation -Encoding UTF8

Get-WmiObject Win32_SerialPort |
    Select-Object Name, DeviceID, Description, ProviderType |
    Export-Csv -Path (Join-Path $destination 'ports_serie.csv') -NoTypeInformation -Encoding UTF8
Write-Host '  imprimantes.csv, ports_serie.csv'

Ecrire-Etape 'Disques et partitions'
Get-WmiObject Win32_DiskDrive |
    Select-Object Model, InterfaceType, Size, Partitions, SerialNumber |
    Export-Csv -Path (Join-Path $destination 'disques.csv') -NoTypeInformation -Encoding UTF8

Get-WmiObject Win32_LogicalDisk |
    Select-Object DeviceID, FileSystem, Size, FreeSpace, VolumeName |
    Export-Csv -Path (Join-Path $destination 'volumes.csv') -NoTypeInformation -Encoding UTF8
Write-Host '  disques.csv, volumes.csv'

Ecrire-Etape 'Logiciels installes (identifier les programmes chinois)'
$clesDesinstallation = @(
    'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*',
    'HKLM:\SOFTWARE\Wow6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*'
)

$logiciels = @()
foreach ($cle in $clesDesinstallation) {
    $entrees = Get-ItemProperty $cle -ErrorAction SilentlyContinue |
        Where-Object { $_.DisplayName }
    if ($entrees) { $logiciels += $entrees }
}

$logiciels |
    Select-Object DisplayName, DisplayVersion, Publisher, InstallLocation |
    Sort-Object DisplayName |
    Export-Csv -Path (Join-Path $destination 'logiciels.csv') -NoTypeInformation -Encoding UTF8
Write-Host '  logiciels.csv'

Ecrire-Etape 'Resume systeme'
& systeminfo.exe | Out-File (Join-Path $destination 'systeme.txt') -Encoding UTF8
Write-Host '  systeme.txt'

Ecrire-Etape 'Copie du magasin de pilotes (le plus important)'
$magasin = Join-Path $env:SystemRoot 'System32\DriverStore\FileRepository'
$copieMagasin = Join-Path $destination 'DriverStore_FileRepository'

Write-Host '  copie en cours, compter plusieurs minutes...'
& robocopy.exe $magasin $copieMagasin /E /R:1 /W:1 /NFL /NDL /NJH /NJS | Out-Null

$fichiersCopies = (Get-ChildItem $copieMagasin -Recurse -File -ErrorAction SilentlyContinue | Measure-Object).Count
Write-Host ('  ' + $fichiersCopies + ' fichiers copies')

Ecrire-Etape 'Copie des INF tiers'
$copieInf = Join-Path $destination 'INF_oem'
New-Item -ItemType Directory -Path $copieInf -Force | Out-Null

Get-ChildItem (Join-Path $env:SystemRoot 'INF') -Filter 'oem*.*' -ErrorAction SilentlyContinue |
    ForEach-Object { Copy-Item $_.FullName -Destination $copieInf -Force -ErrorAction SilentlyContinue }

$infCopies = (Get-ChildItem $copieInf -File -ErrorAction SilentlyContinue | Measure-Object).Count
Write-Host ('  ' + $infCopies + ' fichiers INF copies')

Ecrire-Etape 'TERMINE'
$poids = (Get-ChildItem $destination -Recurse -File -ErrorAction SilentlyContinue |
    Measure-Object -Property Length -Sum).Sum

Write-Host ('Dossier  : ' + $destination)
Write-Host ('Volume   : ' + [math]::Round($poids / 1MB, 1) + ' Mo')
Write-Host ''
Write-Host 'Verifier que la cle USB contient bien ce dossier avant de continuer.'
Read-Host 'Appuyer sur Entree pour fermer'
