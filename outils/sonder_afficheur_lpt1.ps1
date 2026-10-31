<#
    Sonde l afficheur client chaine derriere l imprimante, sur LPT1.

    Etabli le 2026-10-31 : l afficheur de la caisse P70L n est sur aucun
    port COM. Il est branche sur la carte de l imprimante, et c est la voie
    VOIE_IMPRIMANTE de domaine/afficheur.rs.

    Deux etapes, dans cet ordre :

      -Controle   le test qui se lit SANS regarder l ecran. Un texte nu
                  fait avancer le papier ; le meme precede de `ESC = 2` ne
                  fait rien avancer. La difference prouve que la carte
                  comprend la bascule et detourne les octets vers
                  l afficheur. A refaire en premier apres tout doute.

      -Protocoles les six jeux de commandes de LCOS, chacun affichant son
                  propre nom. Le mot qui apparait a l ecran designe le jeu
                  de cet afficheur. Chaque trame est encadree par la
                  bascule : rien ne peut s imprimer.

    Sans option, les deux etapes s enchainent.

    L afficheur doit etre ALLUME : un ecran en veille ne repond a aucun
    jeu de commandes, et le balayage ne veut alors rien dire. Couper et
    retablir l alimentation de la caisse avant de s y fier.
#>

param(
    [switch]$Controle,
    [switch]$Protocoles,
    [int]$PauseMs = 4000
)

$ErrorActionPreference = 'Stop'

# `ESC = n` dans les deux conventions, l ASCII en dernier : masque de bits
# chez Epson, chiffre ASCII dans les manuels d afficheurs VFD. Voir
# VERS_AFFICHEUR dans domaine/afficheur.rs.
$VERS_AFFICHEUR  = [byte[]](0x1B, 0x3D, 0x02, 0x1B, 0x3D, 0x32)
$VERS_IMPRIMANTE = [byte[]](0x1B, 0x3D, 0x01, 0x1B, 0x3D, 0x31)

<#
    Ecrit des octets bruts sur LPT1.

    `copy /b` est le seul chemin simple : un FileStream sur \\.\LPT1 est
    refuse par .NET, qui exige un handle CreateFile pour un peripherique.
    Le spouleur ne gene pas, bien qu une imprimante Generic / Text Only
    soit installee sur LPT1:.
#>
function Envoyer-Lpt1 {
    param([byte[]]$Octets)

    $fichier = Join-Path $env:TEMP ('lcos_lpt1_' + [guid]::NewGuid().ToString('N') + '.bin')
    try {
        [System.IO.File]::WriteAllBytes($fichier, $Octets)
        $sortie = & cmd.exe /c ('copy /b "' + $fichier + '" LPT1: >nul & echo %errorlevel%')
        return ($sortie | Select-Object -Last 1)
    } finally {
        Remove-Item $fichier -Force -ErrorAction SilentlyContinue
    }
}

<#
    Corps de trame d un protocole, repris a l identique de trame() dans
    domaine/afficheur.rs. La bascule est ajoutee par l appelant.
#>
function Corps-Trame {
    param([string]$Protocole, [string]$Ligne)

    $o = New-Object System.Collections.Generic.List[byte]
    $texte = [System.Text.Encoding]::ASCII.GetBytes($Ligne)

    switch ($Protocole) {
        'cd5220' {
            $o.AddRange([byte[]](0x1B, 0x51, 0x41))      # ESC Q A, ligne haute
            $o.AddRange($texte)
            $o.AddRange([byte[]](0x0D))
        }
        'aedex' {
            $o.AddRange([System.Text.Encoding]::ASCII.GetBytes('!#1'))
            $o.AddRange($texte)
            $o.AddRange([byte[]](0x0D))
        }
        'utc' {
            $o.AddRange([byte[]](0x1B, 0x75, 0x41))      # ESC u A, UTC Enhanced
            $o.AddRange($texte)
            $o.AddRange([byte[]](0x0D))
        }
        'adm' {
            $o.AddRange([byte[]](0x0E))                  # SLE1, efface la ligne haute
            $o.AddRange($texte)
        }
        default {
            $o.AddRange([byte[]](0x0C))                  # saut de page, ecran a blanc
            $o.AddRange($texte)
        }
    }

    # Le cast est indispensable : PowerShell deballe le byte[] rendu par une
    # fonction en Object[], que AddRange refuse ensuite.
    return [byte[]]$o.ToArray()
}

function Tester-Controle {
    Write-Host ''
    Write-Host '=== CONTROLE : la bascule detourne-t-elle les octets ? ==='
    Write-Host ''

    $nu = [System.Text.Encoding]::ASCII.GetBytes("TEST LPT1 BRUT`r`n`r`n`r`n")
    Write-Host ('  1. texte nu, sans ESC      : ' + [string]$nu.Length + ' octets, code=' + (Envoyer-Lpt1 $nu))
    Start-Sleep -Milliseconds $PauseMs

    $devie = New-Object System.Collections.Generic.List[byte]
    $devie.AddRange($VERS_AFFICHEUR)
    $devie.AddRange([byte[]](0x0C))
    $devie.AddRange([System.Text.Encoding]::ASCII.GetBytes('LCOS LPT1 OK'))
    $devie.AddRange($VERS_IMPRIMANTE)
    $octets = [byte[]]$devie.ToArray()
    Write-Host ('  2. le meme, precede de ESC = 2 : ' + [string]$octets.Length + ' octets, code=' + (Envoyer-Lpt1 $octets))

    Write-Host ''
    Write-Host '  A lire : le 1 doit faire avancer le papier, le 2 ne doit rien'
    Write-Host '  faire avancer. Cette difference prouve que la carte comprend la'
    Write-Host '  bascule et detourne les octets vers l afficheur.'
    Write-Host '  Les deux font avancer le papier -> la bascule n est pas comprise.'
    Write-Host '  Aucun des deux ne bouge        -> LPT1 ne porte rien, ou l imprimante est hors service.'
}

function Tester-Protocoles {
    Write-Host ''
    Write-Host '=== PROTOCOLES : lequel cet afficheur comprend-il ? ==='
    Write-Host ''
    Write-Host '  Regarde l ecran. Six mots passent, du plus courant au plus rare.'
    Write-Host ''

    # L ordre de la liste de domaine/afficheur.rs : aucun jeu ne se devine,
    # on les essaie l un apres l autre et on regarde l ecran.
    foreach ($protocole in @('texte', 'cd5220', 'escpos', 'aedex', 'utc', 'adm')) {
        $corps = Corps-Trame $protocole $protocole.ToUpper()

        $trame = New-Object System.Collections.Generic.List[byte]
        $trame.AddRange($VERS_AFFICHEUR)
        $trame.AddRange([byte[]]$corps)
        $trame.AddRange($VERS_IMPRIMANTE)
        $octets = [byte[]]$trame.ToArray()

        Write-Host ('  ' + $protocole.PadRight(8) + [string]$octets.Length + ' octets, code=' + (Envoyer-Lpt1 $octets))
        Start-Sleep -Milliseconds $PauseMs
    }

    Write-Host ''
    Write-Host '  Le mot reste a l ecran designe le jeu de commandes a regler dans LCOS.'
}

if (-not $Controle -and -not $Protocoles) {
    $Controle = $true
    $Protocoles = $true
}

if ($Controle)   { Tester-Controle }
if ($Protocoles) { Tester-Protocoles }

Write-Host ''
