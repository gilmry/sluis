<#
.SYNOPSIS
  Sauvegarde chiffrée de la console du superviseur : la distribution WSL et le
  disque externe, vers le disque externe puis vers un compartiment S3.

.DESCRIPTION
  Trois gestes, dans cet ordre :

    1. la distribution WSL est exportée, chiffrée par GPG au fil de l'eau, et
       écrite sur le disque externe ; rien ne touche un disque en clair ;
    2. cette archive chiffrée est déposée dans le compartiment S3 ;
    3. le reste du disque externe est archivé, chiffré et déposé de même.

  Le chiffrement est asymétrique : ce poste ne détient que des clés PUBLIQUES.
  La clé privée qui déchiffre ne doit vivre NI dans la distribution sauvegardée,
  NI sur ce poste : sur clé matérielle, avec une seconde clé de secours hors
  ligne. Une sauvegarde qu'on ne sait relire qu'avec ce qu'elle contient ne
  sauve rien.

  L'identifiant S3 est en écriture seule : ce poste dépose, il ne relit ni
  n'efface. Il est rangé chiffré par Windows (DPAPI), lié à ce compte et à
  cette machine.

.EXAMPLE
  .\sauvegarder-console.ps1 -Initialiser
  Écrit la configuration et demande l'identifiant S3, sans écho.

.EXAMPLE
  .\sauvegarder-console.ps1
  Sauvegarde complète.

.EXAMPLE
  .\sauvegarder-console.ps1 -Verifier E:\sauvegardes-console\20261116T060000Z\wsl-console.tar.gpg
  Preuve de relecture : déchiffre (clé matérielle branchée) et parcourt
  l'archive, sans rien écrire.
#>
[CmdletBinding(DefaultParameterSetName = 'Sauvegarder')]
param(
    [Parameter(ParameterSetName = 'Initialiser', Mandatory = $true)]
    [switch]$Initialiser,

    [Parameter(ParameterSetName = 'Verifier', Mandatory = $true)]
    [string]$Verifier,

    # Ne pas archiver le reste du disque externe (geste 3).
    [Parameter(ParameterSetName = 'Sauvegarder')]
    [switch]$SansDisque,

    # Ne rien déposer sur S3 : le disque externe seulement.
    [Parameter(ParameterSetName = 'Sauvegarder')]
    [switch]$SansS3,

    [string]$Configuration = (Join-Path $env:LOCALAPPDATA 'sluis-console\config.json')
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$Dossier = Split-Path -Parent $Configuration
$FichierIdentifiant = Join-Path $Dossier 's3.xml'
$FichierRclone = Join-Path $Dossier 'rclone.conf'
$DossierSauvegardes = 'sauvegardes-console'

function Arreter([string]$Message) {
    throw "sauvegarder-console : $Message"
}

function Exiger([string[]]$Outils) {
    foreach ($outil in $Outils) {
        if (-not (Get-Command $outil -CommandType Application -ErrorAction SilentlyContinue)) {
            Arreter "$outil introuvable sur ce poste"
        }
    }
}

# Met un argument entre guillemets selon les règles de la ligne de commande
# de Windows : les barres obliques inverses qui précèdent un guillemet se
# doublent, celles de fin aussi.
function Proteger([string]$Argument) {
    if ($Argument -ne '' -and $Argument -notmatch '[\s"]') { return $Argument }
    $protege = $Argument -replace '(\\*)"', '$1$1\"'
    $protege = $protege -replace '(\\+)$', '$1$1'
    return '"' + $protege + '"'
}

# Enchaîne des programmes, la sortie de l'un dans l'entrée du suivant, en
# octets bruts. Un tube de PowerShell 5 abîmerait une archive binaire, et un
# tube de cmd.exe ne rend que le code de sortie du dernier programme : ici,
# le code de CHAQUE étape fait foi.
function Invoke-Chaine {
    param([Parameter(Mandatory = $true)][hashtable[]]$Etapes)

    $processus = @()
    $copies = @()
    try {
        for ($i = 0; $i -lt $Etapes.Count; $i++) {
            $etape = $Etapes[$i]
            $psi = New-Object System.Diagnostics.ProcessStartInfo
            $psi.FileName = $etape.Programme
            $psi.Arguments = (@($etape.Arguments) | ForEach-Object { Proteger $_ }) -join ' '
            $psi.UseShellExecute = $false
            $psi.RedirectStandardInput = ($i -gt 0)
            $psi.RedirectStandardOutput = ($i -lt $Etapes.Count - 1)
            if ($etape.ContainsKey('Environnement')) {
                foreach ($cle in $etape.Environnement.Keys) {
                    $psi.EnvironmentVariables[$cle] = [string]$etape.Environnement[$cle]
                }
            }
            $p = [System.Diagnostics.Process]::Start($psi)
            $processus += $p
            if ($i -gt 0) {
                $copies += $processus[$i - 1].StandardOutput.BaseStream.CopyToAsync($p.StandardInput.BaseStream)
            }
        }

        # Dans l'ordre : quand une étape a tout écrit, on ferme l'entrée de la
        # suivante, qui peut alors finir à son tour.
        $rompu = $false
        for ($i = 0; $i -lt $copies.Count; $i++) {
            try { $copies[$i].Wait() } catch { $rompu = $true }
            try { $processus[$i + 1].StandardInput.Close() } catch { $rompu = $true }
        }
        foreach ($p in $processus) { $p.WaitForExit() }

        for ($i = 0; $i -lt $processus.Count; $i++) {
            if ($processus[$i].ExitCode -ne 0) {
                Arreter "$($Etapes[$i].Programme) a échoué (code $($processus[$i].ExitCode)), étape $($i + 1) sur $($Etapes.Count)"
            }
        }
        if ($rompu) { Arreter 'tube rompu entre deux étapes' }
    }
    finally {
        foreach ($p in $processus) {
            if (-not $p.HasExited) { try { $p.Kill() } catch { } }
            $p.Dispose()
        }
    }
}

function Lire-Configuration {
    if (-not (Test-Path -LiteralPath $Configuration)) {
        Arreter "$Configuration absent : lancer d'abord avec -Initialiser"
    }
    $c = Get-Content -LiteralPath $Configuration -Raw -Encoding UTF8 | ConvertFrom-Json
    foreach ($champ in 'Distribution', 'EtiquetteDisque', 'Destinataires', 'PointAcces', 'Region', 'Compartiment', 'Conserver') {
        if (-not ($c.PSObject.Properties.Name -contains $champ)) { Arreter "champ « $champ » absent de $Configuration" }
    }
    $destinataires = @($c.Destinataires)
    if ($destinataires.Count -lt 1) { Arreter 'aucun destinataire GPG' }
    foreach ($d in $destinataires) {
        if ($d -notmatch '^[0-9A-F]{40}$') { Arreter "destinataire « $d » : empreinte complète attendue, 40 chiffres hexadécimaux en majuscules" }
    }
    if ($destinataires.Count -lt 2) {
        Write-Warning 'Un seul destinataire : la perte de sa clé privée rend toutes les sauvegardes illisibles. En ajouter un de secours, hors ligne.'
    }
    return $c
}

# Les clés publiques des destinataires doivent être dans le trousseau de ce
# poste. Une clé privée complète sur ce disque est signalée : la clé qui
# déchiffre n'a rien à faire à côté de ce qu'elle protège.
function Verifier-Destinataires([string[]]$Destinataires) {
    foreach ($d in $Destinataires) {
        & gpg --batch --list-keys --with-colons $d *> $null
        if ($LASTEXITCODE -ne 0) { Arreter "clé publique $d absente du trousseau GPG de ce poste" }
        $secretes = & gpg --batch --list-secret-keys --with-colons $d 2> $null
        if ($LASTEXITCODE -eq 0) {
            foreach ($ligne in $secretes) {
                $champs = $ligne -split ':'
                if ($champs[0] -in 'sec', 'ssb' -and $champs.Count -gt 14 -and $champs[14] -eq '+') {
                    Write-Warning "La clé privée de $d est sur le disque de ce poste : elle devrait vivre sur clé matérielle ou hors ligne."
                    break
                }
            }
        }
    }
}

# Le disque est désigné par son étiquette de volume, jamais par sa lettre :
# une lettre change, et une sauvegarde écrite sur le mauvais disque est une
# fuite ou une perte.
function Trouver-Disque([string]$Etiquette) {
    $volumes = @(Get-Volume | Where-Object { $_.FileSystemLabel -eq $Etiquette -and $_.DriveLetter })
    if ($volumes.Count -eq 0) { Arreter "aucun volume monté ne porte l'étiquette « $Etiquette » : brancher le disque externe" }
    if ($volumes.Count -gt 1) { Arreter "plusieurs volumes portent l'étiquette « $Etiquette »" }
    return "$($volumes[0].DriveLetter):\"
}

function Arguments-Gpg([string[]]$Destinataires) {
    # --trust-model always : les destinataires sont désignés par leur
    # empreinte complète, c'est elle qui fait foi, pas le réseau de confiance.
    $arguments = @('--batch', '--yes', '--no-tty', '--trust-model', 'always', '--compress-level', '1', '--encrypt')
    foreach ($d in $Destinataires) { $arguments += @('--recipient', $d) }
    return $arguments
}

function Lire-IdentifiantS3 {
    if (-not (Test-Path -LiteralPath $FichierIdentifiant)) { Arreter "$FichierIdentifiant absent : lancer d'abord avec -Initialiser" }
    $identifiant = Import-Clixml -LiteralPath $FichierIdentifiant
    return @{
        RCLONE_CONFIG               = $FichierRclone
        RCLONE_S3_PROVIDER          = 'Other'
        RCLONE_S3_ENDPOINT          = $script:Config.PointAcces
        RCLONE_S3_REGION            = $script:Config.Region
        RCLONE_S3_ACCESS_KEY_ID     = $identifiant.UserName
        RCLONE_S3_SECRET_ACCESS_KEY = $identifiant.GetNetworkCredential().Password
        # Écriture seule : ni lecture du compartiment, ni vérification après envoi.
        RCLONE_S3_NO_CHECK_BUCKET   = 'true'
        RCLONE_S3_NO_HEAD           = 'true'
    }
}

function Initialiser-Console {
    New-Item -ItemType Directory -Force -Path $Dossier | Out-Null
    if (-not (Test-Path -LiteralPath $Configuration)) {
        $modele = [ordered]@{
            Distribution    = 'console-superviseur'
            EtiquetteDisque = 'SAUVEGARDE'
            Destinataires   = @('EMPREINTE-COMPLETE-DE-LA-CLE-PRINCIPALE', 'EMPREINTE-COMPLETE-DE-LA-CLE-DE-SECOURS')
            PointAcces      = 'https://s3.eu-west-par.io.cloud.ovh.net'
            Region          = 'eu-west-par'
            Compartiment    = 'a-remplacer-console'
            # Nombre de sauvegardes gardées sur le disque externe. 0 : toutes.
            Conserver       = 5
        }
        $modele | ConvertTo-Json | Set-Content -LiteralPath $Configuration -Encoding UTF8
        Write-Host "Configuration écrite dans $Configuration : la remplir."
    }
    else {
        Write-Host "Configuration déjà présente : $Configuration"
    }
    if (-not (Test-Path -LiteralPath $FichierRclone)) { Set-Content -LiteralPath $FichierRclone -Value '' -Encoding ASCII }

    $cle = Read-Host "Identifiant S3 d'écriture seule (access key), vide pour ne rien changer"
    if ($cle) {
        $secret = Read-Host 'Secret de cet identifiant' -AsSecureString
        New-Object System.Management.Automation.PSCredential($cle, $secret) | Export-Clixml -LiteralPath $FichierIdentifiant
        Write-Host "Identifiant rangé dans $FichierIdentifiant, chiffré pour ce compte sur cette machine."
    }
    # Le dossier n'est lisible que par ce compte.
    & icacls $Dossier /inheritance:r /grant:r "$($env:USERNAME):(OI)(CI)F" | Out-Null
}

function Verifier-Archive([string]$Fichier) {
    Exiger @('gpg', 'tar')
    if (-not (Test-Path -LiteralPath $Fichier)) { Arreter "$Fichier introuvable" }
    Write-Host 'Déchiffrement et parcours de l''archive (clé matérielle requise). Rien n''est écrit.'
    $sortie = [System.IO.Path]::GetTempFileName()
    try {
        Invoke-Chaine @(
            @{ Programme = 'gpg'; Arguments = @('--decrypt', $Fichier) },
            @{ Programme = 'cmd'; Arguments = @('/d', '/c', "tar -tf - > `"$sortie`"") }
        )
        $entrees = @(Get-Content -LiteralPath $sortie).Count
        if ($entrees -lt 1) { Arreter 'archive vide' }
        Write-Host "Relecture réussie : $entrees entrées."
    }
    finally {
        Remove-Item -LiteralPath $sortie -Force -ErrorAction SilentlyContinue
    }
}

function Sauvegarder-Console {
    Exiger @('wsl', 'gpg', 'tar')
    if (-not $SansS3) { Exiger @('rclone') }
    $script:Config = Lire-Configuration
    $destinataires = @($script:Config.Destinataires)
    Verifier-Destinataires $destinataires

    $distribution = $script:Config.Distribution
    # wsl.exe écrit en UTF-16 : on compare sans les octets nuls.
    $connues = (& wsl --list --quiet) -replace "`0", '' | ForEach-Object { $_.Trim() } | Where-Object { $_ }
    if ($connues -notcontains $distribution) { Arreter "distribution WSL « $distribution » inconnue" }

    $racine = Trouver-Disque $script:Config.EtiquetteDisque
    $horodatage = (Get-Date).ToUniversalTime().ToString('yyyyMMddTHHmmssZ')
    $cible = Join-Path (Join-Path $racine $DossierSauvegardes) $horodatage
    New-Item -ItemType Directory -Force -Path $cible | Out-Null
    $archive = Join-Path $cible "wsl-$distribution.tar.gpg"
    $gpg = Arguments-Gpg $destinataires

    # 1. La distribution, arrêtée pour que l'export soit cohérent.
    Write-Host "Arrêt de la distribution $distribution, puis export chiffré vers $archive"
    & wsl --terminate $distribution | Out-Null
    Invoke-Chaine @(
        @{ Programme = 'wsl'; Arguments = @('--export', $distribution, '-') },
        @{ Programme = 'gpg'; Arguments = $gpg + @('--output', "$archive.partiel") }
    )
    Move-Item -LiteralPath "$archive.partiel" -Destination $archive
    $taille = (Get-Item -LiteralPath $archive).Length
    if ($taille -lt 1MB) { Arreter "archive de $taille octets : trop petite pour une distribution, export suspect" }
    $empreinte = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()

    $manifeste = [ordered]@{
        date          = $horodatage
        distribution  = $distribution
        destinataires = $destinataires
        fichiers      = @([ordered]@{ nom = (Split-Path -Leaf $archive); octets = $taille; sha256 = $empreinte })
        disque        = $null
    }
    $fichierManifeste = Join-Path $cible 'manifeste.json'

    if (-not $SansS3) {
        $s3 = Lire-IdentifiantS3
        $prefixe = ":s3:$($script:Config.Compartiment)/console/$horodatage"

        # 2. L'archive chiffrée de la distribution.
        Write-Host 'Dépôt de l''archive de la distribution sur S3'
        Invoke-Chaine @(
            @{ Programme = 'rclone'; Arguments = @('copyto', '--no-check-dest', $archive, "$prefixe/$(Split-Path -Leaf $archive)"); Environnement = $s3 }
        )

        # 3. Le reste du disque externe, sans ses propres sauvegardes ni les
        # dossiers du système, que Windows refuse de lire.
        if (-not $SansDisque) {
            Write-Host "Archive chiffrée du disque $racine vers S3"
            Invoke-Chaine @(
                @{ Programme = 'tar'; Arguments = @('-cf', '-', '-C', $racine, '--exclude', "./$DossierSauvegardes", '--exclude', './System Volume Information', '--exclude', './$RECYCLE.BIN', '.') },
                @{ Programme = 'gpg'; Arguments = $gpg },
                @{ Programme = 'rclone'; Arguments = @('rcat', "$prefixe/disque.tar.gpg"); Environnement = $s3 }
            )
            $manifeste.disque = 'disque.tar.gpg'
        }

        $manifeste | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $fichierManifeste -Encoding UTF8
        Invoke-Chaine @(
            @{ Programme = 'rclone'; Arguments = @('copyto', '--no-check-dest', $fichierManifeste, "$prefixe/manifeste.json"); Environnement = $s3 }
        )
    }
    else {
        $manifeste | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $fichierManifeste -Encoding UTF8
    }

    # Rétention sur le disque externe : seuls les dossiers horodatés de ce
    # script, sous son propre dossier, les plus anciens d'abord. Sur S3, le
    # verrou et le cycle de vie du compartiment décident, pas ce poste.
    $conserver = [int]$script:Config.Conserver
    if ($conserver -gt 0) {
        $anciennes = @(Get-ChildItem -LiteralPath (Join-Path $racine $DossierSauvegardes) -Directory |
            Where-Object { $_.Name -match '^\d{8}T\d{6}Z$' } |
            Sort-Object Name -Descending |
            Select-Object -Skip $conserver)
        foreach ($ancienne in $anciennes) {
            Write-Host "Retrait de l'ancienne sauvegarde $($ancienne.FullName)"
            Remove-Item -LiteralPath $ancienne.FullName -Recurse -Force
        }
    }

    Set-Content -LiteralPath (Join-Path $Dossier 'derniere-reussite.txt') -Value $horodatage -Encoding ASCII
    Write-Host "Sauvegarde $horodatage terminée. Preuve de relecture : -Verifier `"$archive`""
}

try {
    switch ($PSCmdlet.ParameterSetName) {
        'Initialiser' { Initialiser-Console }
        'Verifier' { Verifier-Archive $Verifier }
        default { Sauvegarder-Console }
    }
}
catch {
    Write-Error $_.Exception.Message -ErrorAction Continue
    exit 1
}
