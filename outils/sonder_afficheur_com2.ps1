<#
    Cherche le debit et le jeu de commandes de l afficheur client, sur COM2.

    Etabli le 2026-10-31 : l afficheur est sur COM2. La preuve est une
    extinction reproductible, obtenue deux fois sur deux en martelant ce
    port, et jamais sur COM1.

    AVANT DE LANCER : l ecran doit afficher 0.00. C est son etat
    d initialisation, et le seul temoin fiable — s il change, l envoi a
    porte. Il ne revient qu en DEBRANCHANT LE CORDON SECTEUR une vingtaine
    de secondes ; un redemarrage de Windows ne coupe pas son alimentation.

    UN SEUL ESSAI A LA FOIS, puis arret et question : l ecran est-il encore
    allume ? La reponse va dans recherches/journal_afficheur.md avant
    l essai suivant. Une rafale efface sa propre reponse, et c est ce qui a
    brouille les essais 4 a 17.

    Ce script est volontairement DOUX, a l inverse du martelement qui
    eteint l afficheur : une seule ouverture de port par debit, le port
    maintenu ouvert pendant que l oeil observe. Ne jamais revenir a une
    boucle d ouvertures rapides.

    Trois trames, parce que l octet 0x0C est suspect : il est cense effacer
    l ecran, mais il pourrait l ETEINDRE sur ce modele. Tous nos envois
    commencaient par lui.

      -Trame efface        0C seul. Si l ecran part, ce debit est le bon
                           ET 0C eteint.
      -Trame texte         le texte seul, sans 0C. Le test de l hypothese :
                           si le texte s affiche, tout est trouve.
      -Trame efface+texte  les deux, comme les premiers essais (defaut).

    Usage :
      sonder_afficheur_com2.ps1 -Debits 19200 -Trame efface
      sonder_afficheur_com2.ps1 -Debits 19200 -Trame texte
      sonder_afficheur_com2.ps1 -Tous
#>

param(
    [string]$Port = 'COM2',
    [int[]]$Debits = @(9600),
    [switch]$Tous,
    [ValidateSet('efface', 'texte', 'efface+texte', 'init', 'lumiere', 'lumiere-bin',
                 'cd5220', 'aedex', 'utc', 'adm')]
    [string]$Trame = 'efface+texte',
    [switch]$SansLignes,
    [int]$MaintienMs = 3000,
    # Contenu a afficher. Par defaut le debit lui-meme, pour que l oeil lise
    # la reponse sans chronometrer. L afficheur etant NUMERIQUE, seuls les
    # chiffres et le point decimal ressortent.
    [string]$Texte = ''
)

$ErrorActionPreference = 'Stop'

# Les six debits que LCOS accepte, dans l ordre de vraisemblance. Voir
# DEBITS_CONNUS dans domaine/afficheur.rs.
if ($Tous) { $Debits = @(9600, 19200, 4800, 2400, 38400, 1200) }

if ($Tous -and $Debits.Count -gt 1) {
    Write-Host 'ATTENTION : -Tous enchaine plusieurs debits et efface sa propre reponse.'
    Write-Host 'Le protocole du journal demande UN SEUL debit par essai.'
    Write-Host ''
}

$SAUT_DE_PAGE = [byte[]](0x0C)

<#
    Le texte nomme son propre debit : l oeil lit la reponse sans avoir a
    chronometrer l envoi.
#>
function Composer-Trame {
    param([int]$Debit, [string]$Forme)

    $o = New-Object System.Collections.Generic.List[byte]
    if ($Texte) { $contenu = $Texte } else { $contenu = 'LCOS ' + [string]$Debit }
    $texte = [System.Text.Encoding]::ASCII.GetBytes($contenu)
    $fin = [byte[]](0x0D)

    switch ($Forme) {
        'efface'      { $o.AddRange($SAUT_DE_PAGE) }
        'texte'       { $o.AddRange($texte) }
        # ESC @ : initialise. Le candidat standard pour rallumer un ecran.
        'init'        { $o.AddRange([byte[]](0x1B, 0x40)) }
        # ESC * n : luminosite. Les manuels VFD donnent n en chiffre ASCII,
        # CD 5220 le donne en octet : une forme par essai, pas les deux.
        'lumiere'     { $o.AddRange([byte[]](0x1B, 0x2A, 0x34)) }
        'lumiere-bin' { $o.AddRange([byte[]](0x1B, 0x2A, 0x04)) }
        'cd5220'      { $o.AddRange([byte[]](0x1B, 0x51, 0x41)); $o.AddRange($texte); $o.AddRange($fin) }
        'aedex'       { $o.AddRange([System.Text.Encoding]::ASCII.GetBytes('!#1')); $o.AddRange($texte); $o.AddRange($fin) }
        'utc'         { $o.AddRange([byte[]](0x1B, 0x75, 0x41)); $o.AddRange($texte); $o.AddRange($fin) }
        'adm'         { $o.AddRange([byte[]](0x0E)); $o.AddRange($texte) }
        default       { $o.AddRange($SAUT_DE_PAGE); $o.AddRange($texte) }
    }

    return [byte[]]$o.ToArray()
}

function Vers-Hex {
    param([byte[]]$Octets)
    if ($Octets.Length -eq 0) { return '(vide)' }
    return (($Octets | ForEach-Object { '{0:X2}' -f $_ }) -join ' ')
}

Write-Host ('Afficheur sur ' + $Port + ', trame ' + $Trame + '.')
Write-Host 'Temoin : le 0.00 doit etre a l ecran avant de lancer.'
Write-Host ''

foreach ($debit in $Debits) {
    $octets = Composer-Trame $debit $Trame

    $serie = New-Object System.IO.Ports.SerialPort($Port, $debit, 'None', 8, 'One')
    $serie.Handshake = 'None'
    $serie.WriteTimeout = 3000
    try {
        $serie.Open()
        if (-not $SansLignes) {
            $serie.DtrEnable = $true
            $serie.RtsEnable = $true
        }
        Start-Sleep -Milliseconds 150
        $serie.Write($octets, 0, $octets.Length)

        # Le port reste ouvert : les lignes de controle tiennent, et rien ne
        # bouge pendant que l oeil observe.
        Start-Sleep -Milliseconds $MaintienMs

        Write-Host ('  ' + ([string]$debit).PadLeft(6) + ' 8N1 : ' + (Vers-Hex $octets))
    } catch {
        Write-Host ('  ' + ([string]$debit).PadLeft(6) + ' 8N1 : ECHEC ' + $_.Exception.GetType().Name)
    } finally {
        if ($serie.IsOpen) { $serie.Close() }
        $serie.Dispose()
    }
}

Write-Host ''
Write-Host 'A relever maintenant : l ecran est-il ALLUME ou PARTI ?'
Write-Host 'Consigner la reponse dans recherches/journal_afficheur.md.'
