#!/usr/bin/env bash
# Louer et configurer la plateforme du palier 2 (ADR-044), depuis le poste du
# superviseur, et de là seulement (ADR-037, invariant 1).
#
#   ./deployer.sh regles infra               règles d'accès du jeton d'infrastructure
#   ./deployer.sh regles dns <zone>          règles d'accès du jeton DNS
#   ./deployer.sh plan durable|site          plan OpenTofu, chiffré, rien n'est loué
#   ./deployer.sh appliquer durable|site     applique le dernier plan (COMMANDE PAYANTE)
#   ./deployer.sh etat durable|site          sorties OpenTofu non sensibles
#   ./deployer.sh empreinte <machine> <SHA256:…>  vérifie la clé d'hôte lue dans la console
#   ./deployer.sh totp <machine>             enrôle le TOTP de l'administrateur, sur la machine
#   ./deployer.sh chiffrer <machine>         crée le volume chiffré (phrase tapée par toi)
#   ./deployer.sh deverrouiller <machine>    rouvre le volume après un redémarrage
#   ./deployer.sh configurer                 passage Ansible complet, les deux machines
#   ./deployer.sh kms                        rapatrie la demande de certificat d'OpenBao
#   ./deployer.sh verifier                   dérive : --check --diff, changed=0 exigé
#   ./deployer.sh tunnel                     renvoi de l'API de K3s sur 127.0.0.1:6443
#   ./deployer.sh sauvegarder-etat           dépose états et inventaires à Paris
#   ./deployer.sh sauvegardes <compartiment> [préfixe]   liste ce qui est à Paris
#   ./deployer.sh rapatrier <compartiment> <préfixe> <dossier>   relit des sauvegardes
#   ./deployer.sh identifiant-superviseur    l'identifiant S3 du poste, à ranger hors ligne
#   ./deployer.sh identifiant-console        l'identifiant S3 d'écriture seule de la console
#
# <machine> : sluis ou k3s. Le site est celui de SLUIS_SITE (gra1 par défaut) ;
# reconstruire après la perte d'un centre de données, c'est rejouer les mêmes
# commandes avec un site neuf (SLUIS_SITE=sbg1), voir README.md.
#
# Le script ne conserve rien d'autre que ce qu'OpenTofu et Ansible écrivent
# dans SLUIS_INFRA_ETAT, hors du dépôt. Il n'affiche jamais un secret, sauf
# `identifiant-superviseur` et `identifiant-console`, et dans un terminal
# seulement. Il ne rend jamais
# une machine ni un compartiment : `prevent_destroy` l'interdit, et le jeton
# d'infrastructure n'a aucune route de suppression.
set -euo pipefail

ICI="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ANSIBLE_DIR="$ICI/ansible"
FICHIER_ENV="${SLUIS_INFRA_ENV:-$HOME/.config/sluis-infra/env}"

die() { echo "deployer : $*" >&2; exit 1; }

# Développe un « ~ » en tête de chemin, seule forme admise dans le fichier
# d'environnement (il est lu comme des données, jamais exécuté).
developper() { case "$1" in "~"/*) printf '%s\n' "$HOME/${1#"~/"}" ;; *) printf '%s\n' "$1" ;; esac; }

# Lit le fichier d'environnement ligne à ligne, sans l'exécuter : un `source`
# donnerait au contenu du fichier le pouvoir de lancer des commandes.
charger_env() {
  [ -f "$FICHIER_ENV" ] || die "$FICHIER_ENV absent : cp $ICI/.env.exemple $FICHIER_ENV, puis le remplir"
  local droits
  droits="$(stat -c '%a' "$FICHIER_ENV")"
  [ "$droits" = "600" ] || die "$FICHIER_ENV est en $droits : chmod 600 (il porte des jetons)"
  local ligne cle valeur
  while IFS= read -r ligne || [ -n "$ligne" ]; do
    case "$ligne" in ''|'#'*) continue ;; esac
    cle="${ligne%%=*}"
    valeur="${ligne#*=}"
    [[ "$cle" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] || die "ligne illisible dans $FICHIER_ENV (clé « $cle »)"
    # L'environnement du shell l'emporte sur le fichier.
    if [ -z "${!cle:-}" ]; then
      export "$cle=$valeur"
    fi
  done < "$FICHIER_ENV"

  SLUIS_SITE="${SLUIS_SITE:-gra1}"
  [[ "$SLUIS_SITE" =~ ^[a-z][a-z0-9]{1,11}$ ]] || die "SLUIS_SITE : lettres minuscules et chiffres, 2 à 12 caractères"
  SLUIS_INFRA_ETAT="$(developper "${SLUIS_INFRA_ETAT:-$HOME/.local/state/sluis-infra}")"
  SLUIS_INFRA_CONFIG="$(developper "${SLUIS_INFRA_CONFIG:-$HOME/.config/sluis-infra}")"
  SLUIS_INVENTAIRE="$(developper "${SLUIS_INVENTAIRE:-$SLUIS_INFRA_CONFIG/inventaire.yml}")"
  SLUIS_CLE_SSH="$(developper "${SLUIS_CLE_SSH:-$HOME/.ssh/id_ed25519}")"
  SLUIS_SECRETS_SOPS="$(developper "${SLUIS_SECRETS_SOPS:-}")"
  SLUIS_ADMIN="${SLUIS_ADMIN:-debian}"
  export SLUIS_CLE_SSH SLUIS_SECRETS_SOPS

  case "$SLUIS_INFRA_ETAT" in
    "$ICI"*|"$(cd "$ICI/../.." && pwd)"*) die "SLUIS_INFRA_ETAT pointe dans le dépôt : l'état n'y a pas sa place" ;;
  esac
  mkdir -p "$SLUIS_INFRA_ETAT"
  chmod 700 "$SLUIS_INFRA_ETAT"
  # Demandée ici, une fois : les lectures d'état se font dans des
  # sous-shells, qui la redemanderaient chacun.
  phrase_etat
}

exiger() {
  local outil
  for outil in "$@"; do
    command -v "$outil" >/dev/null || die "$outil introuvable sur ce poste"
  done
}

# La phrase de l'état n'est jamais lue sur la ligne de commande (elle
# finirait dans l'historique du shell) ; demandée sans écho si absente.
phrase_etat() {
  if [ -z "${TF_VAR_phrase_etat:-}" ]; then
    read -r -s -p "Phrase de chiffrement de l'état OpenTofu : " TF_VAR_phrase_etat
    echo >&2
    export TF_VAR_phrase_etat
  fi
  [ "${#TF_VAR_phrase_etat}" -ge 16 ] || die "la phrase de l'état fait au moins 16 caractères"
}

exiger_jeton_infra() {
  local v
  for v in OVH_APPLICATION_KEY OVH_APPLICATION_SECRET OVH_CONSUMER_KEY; do
    [ -n "${!v:-}" ] || die "$v vide : jeton d'infrastructure absent (voir .env.exemple, section 1)"
  done
}

# ── OpenTofu : deux racines, deux états ──────────────────────────────────────
#
#   durable  ce qui survit à la perte d'un centre de données : projet, vRack,
#            réseaux des zones, compartiments de Paris, KMS
#   site     les machines d'un centre de données et leurs noms DNS

racine() {
  case "${1:-}" in
    durable) NOM_ETAT="durable" ;;
    site) NOM_ETAT="site-$SLUIS_SITE" ;;
    *) die "racine attendue : durable ou site" ;;
  esac
  RACINE="$1"
  TOFU_DIR="$ICI/tofu/$RACINE"
  TFVARS="$SLUIS_INFRA_CONFIG/$NOM_ETAT.tfvars"
  PLAN="$SLUIS_INFRA_ETAT/$NOM_ETAT.tfplan"
  export TF_VAR_chemin_etat="$SLUIS_INFRA_ETAT/$NOM_ETAT.tfstate"
  export TF_DATA_DIR="$SLUIS_INFRA_ETAT/tofu-$NOM_ETAT"
}

tofu_init() {
  exiger tofu jq
  phrase_etat
  tofu -chdir="$TOFU_DIR" init -input=false -lockfile=readonly >/dev/null
}

# Sortie d'une racine, en JSON. Change de racine le temps de la lecture.
sortie_json() {
  local racine_voulue="$1" nom="$2"
  (
    racine "$racine_voulue"
    [ -f "$TF_VAR_chemin_etat" ] || die "aucun état « $NOM_ETAT » : lancer d'abord $0 plan $racine_voulue, puis appliquer"
    tofu_init
    tofu -chdir="$TOFU_DIR" output -json "$nom"
  )
}

# Le site reçoit le projet et le réseau de sa région de l'état durable, par
# l'environnement : un fichier de variables l'emporterait sur eux.
variables_du_site() {
  local region
  region="$(sed -n 's/^region_cloud[[:space:]]*=[[:space:]]*"\([A-Z0-9]*\)".*/\1/p' "$TFVARS")"
  region="${region:-GRA11}"
  TF_VAR_projet="$(sortie_json durable projet | jq -er .)" || die "sortie « projet » de l'état durable illisible"
  TF_VAR_reseau_zone_1="$(sortie_json durable reseaux | jq -er --arg c "zone-1/$region" '.[$c].id')" \
    || die "aucun réseau de la zone 1 dans la région $region : l'ajouter à regions_cloud de l'état durable"
  export TF_VAR_projet TF_VAR_reseau_zone_1
}

preparer_racine() {
  charger_env
  racine "${1:-}"
  [ -f "$TFVARS" ] || die "$TFVARS absent : copier l'exemple de tofu/$RACINE/"
  exiger_jeton_infra
  phrase_etat
  if [ "$RACINE" = "site" ]; then
    variables_du_site
  elif [ -f "$SLUIS_INFRA_ETAT/openbao.csr" ]; then
    export TF_VAR_csr_openbao="$SLUIS_INFRA_ETAT/openbao.csr"
  fi
  tofu_init
}

faire_plan() {
  preparer_racine "${1:-}"
  tofu -chdir="$TOFU_DIR" plan -input=false -var-file="$TFVARS" -out="$PLAN"
  echo "Plan chiffré dans $PLAN. Le relire, puis : $0 appliquer $RACINE" >&2
}

appliquer() {
  preparer_racine "${1:-}"
  [ -f "$PLAN" ] || die "aucun plan : lancer d'abord $0 plan $RACINE"
  echo "Appliquer ce plan peut PASSER UNE COMMANDE PAYÉE par le moyen de paiement du compte." >&2
  local reponse
  read -r -p "Taper « appliquer $NOM_ETAT » pour confirmer : " reponse
  [ "$reponse" = "appliquer $NOM_ETAT" ] || die "abandon, rien n'a été appliqué"
  tofu -chdir="$TOFU_DIR" apply -input=false "$PLAN"
  rm -f "$PLAN"
  echo >&2
  if [ "$RACINE" = "site" ]; then
    echo "Étape suivante, pour chaque machine : relever l'empreinte de sa clé d'hôte" >&2
    echo "ED25519 dans le journal de démarrage de l'instance (espace client), puis :" >&2
    echo "  $0 empreinte <machine> SHA256:..." >&2
  else
    echo "Étapes suivantes : $0 identifiant-superviseur (à ranger hors ligne)," >&2
    echo "puis $0 sauvegarder-etat." >&2
  fi
}

etat() {
  charger_env
  racine "${1:-}"
  tofu_init
  # Sans -json ni -raw, tofu masque de lui-même les sorties sensibles.
  tofu -chdir="$TOFU_DIR" output
}

# ── Machines ─────────────────────────────────────────────────────────────────

MACHINES_JSON=""

lire_machines() {
  [ -n "$MACHINES_JSON" ] || MACHINES_JSON="$(sortie_json site machines)"
}

adresse() {
  lire_machines
  jq -er --arg m "$1" '.[$m].publique | select(length > 0)' <<<"$MACHINES_JSON" \
    || die "aucune adresse publique pour « $1 » dans l'état du site $SLUIS_SITE"
}

exiger_machine() {
  case "${1:-}" in
    sluis|k3s) ;;
    *) die "machine attendue : sluis ou k3s" ;;
  esac
}

# ── Clé d'hôte ───────────────────────────────────────────────────────────────
#
# Jamais de confiance au premier contact (ADR-037, phase 2) : l'empreinte vue
# par le réseau doit être celle lue dans la console de l'hébergeur.

empreinte() {
  local machine="${1:-}" attendue="${2:-}"
  exiger_machine "$machine"
  [[ "$attendue" =~ ^SHA256:[A-Za-z0-9+/]{43}$ ]] || die "usage : $0 empreinte <machine> SHA256:<43 caractères>"
  charger_env
  exiger ssh-keyscan ssh-keygen
  local hote vue tmp connus
  lire_machines
  hote="$(adresse "$machine")"
  connus="$SLUIS_INFRA_ETAT/known_hosts"
  tmp="$(mktemp)"
  trap 'rm -f "$tmp"' RETURN
  ssh-keyscan -t ed25519 "$hote" 2>/dev/null > "$tmp"
  [ -s "$tmp" ] || die "aucune clé ed25519 lue sur $hote"
  vue="$(ssh-keygen -lf "$tmp" | awk '{print $2}')"
  if [ "$vue" != "$attendue" ]; then
    die "empreinte vue ($vue) différente de celle de la console : NE PAS se connecter"
  fi
  touch "$connus"
  chmod 600 "$connus"
  ssh-keygen -R "$hote" -f "$connus" >/dev/null 2>&1 || true
  cat "$tmp" >> "$connus"
  rm -f "$connus.old"
  echo "Clé d'hôte vérifiée et enregistrée pour $machine ($hote)." >&2
}

# ── SSH ──────────────────────────────────────────────────────────────────────
#
# Une connexion maîtresse par machine, ouverte une fois (clé, puis code TOTP
# tapé par toi), que les commandes suivantes et Ansible réutilisent : Ansible
# ne sait pas répondre à une invite TOTP, et il n'a pas à le savoir.

preparer_ssh() {
  charger_env
  exiger ssh
  [ -f "$SLUIS_INFRA_ETAT/known_hosts" ] || die "clé d'hôte non vérifiée : lancer d'abord $0 empreinte"
  lire_machines
  SSH_OPTS=(
    -i "$SLUIS_CLE_SSH"
    -o IdentitiesOnly=yes
    -o "UserKnownHostsFile=$SLUIS_INFRA_ETAT/known_hosts"
    -o StrictHostKeyChecking=yes
    -o ControlMaster=auto
    -o "ControlPath=$SLUIS_INFRA_ETAT/ssh-%C"
    -o ControlPersist=15m
  )
}

ouvrir() {
  local machine="$1" hote
  hote="$(adresse "$machine")"
  if ! ssh "${SSH_OPTS[@]}" -O check "$SLUIS_ADMIN@$hote" 2>/dev/null; then
    echo "Connexion à $machine ($hote) : clé, puis code TOTP s'il est déjà imposé." >&2
    ssh "${SSH_OPTS[@]}" -o ControlMaster=yes -fN "$SLUIS_ADMIN@$hote"
  fi
  HOTE="$hote"
}

# Commande distante dans un vrai terminal : les invites (TOTP, phrase LUKS)
# s'affichent chez toi, et rien de ce que tu tapes ne passe par ce script.
distant() { ssh "${SSH_OPTS[@]}" -t "$SLUIS_ADMIN@$HOTE" "$@"; }

# Le secret TOTP naît sur la machine et s'affiche dans TON terminal, jamais
# dans un fichier du poste ni dans une sortie qu'un agent lirait (ADR-037).
# Codes de secours : à noter et ranger en zone 3.
totp() {
  exiger_machine "${1:-}"
  preparer_ssh
  ouvrir "$1"
  distant 'test ! -e ~/.google_authenticator || { echo "TOTP déjà enrôlé : rien à faire."; exit 0; }
    command -v google-authenticator >/dev/null || { sudo apt-get update -qq && sudo apt-get install -y -qq libpam-google-authenticator; }
    google-authenticator --time-based --disallow-reuse --force --rate-limit=3 --rate-time=30 --window-size=3'
  echo "Enrôlé sur $1. Il sera imposé au prochain : $0 configurer" >&2
}

# Taille du volume chiffré : le disque d'un d2-2 fait 25 Go, celui d'un d2-8
# 50 Go, et le système en garde une part.
taille_chiffre() {
  local taille
  case "$1" in
    sluis) taille="${SLUIS_TAILLE_CHIFFRE_SLUIS:-12G}" ;;
    k3s) taille="${SLUIS_TAILLE_CHIFFRE_K3S:-35G}" ;;
  esac
  [[ "$taille" =~ ^[0-9]+G$ ]] || die "taille du volume chiffré au format <n>G"
  printf '%s\n' "$taille"
}

# Création du volume chiffré, une seule fois par machine. cryptsetup demande
# la phrase deux fois dans ton terminal ; elle n'est écrite nulle part sur la
# machine (aucune entrée dans crypttab). Sa perte rend les données
# illisibles : la ranger en zone 3 avant de taper la seconde fois.
chiffrer() {
  exiger_machine "${1:-}"
  preparer_ssh
  local taille
  taille="$(taille_chiffre "$1")"
  ouvrir "$1"
  distant "set -e
    if sudo test -e /srv/chiffre.luks; then echo 'Volume déjà créé : rien à faire.'; exit 0; fi
    command -v cryptsetup >/dev/null || { sudo apt-get update -qq && sudo apt-get install -y -qq cryptsetup; }
    sudo install -d -m 0755 /srv
    sudo fallocate -l $taille /srv/chiffre.luks
    sudo chmod 600 /srv/chiffre.luks
    sudo cryptsetup luksFormat --type luks2 --cipher aes-xts-plain64 --key-size 512 --pbkdf argon2id --verify-passphrase /srv/chiffre.luks
    sudo cryptsetup open /srv/chiffre.luks sluis_chiffre
    sudo mkfs.ext4 -q -L sluis_chiffre /dev/mapper/sluis_chiffre
    sudo install -d -m 0755 /srv/chiffre
    sudo mount /dev/mapper/sluis_chiffre /srv/chiffre
    echo 'Volume chiffré créé et monté.'"
}

# Après tout redémarrage d'une machine : Docker (sluis) ou K3s (k3s) attend ce
# montage et reste arrêté jusque-là. C'est le prix d'une clé qui ne vit pas
# sur la machine.
deverrouiller() {
  exiger_machine "${1:-}"
  preparer_ssh
  local service=docker
  [ "$1" = "k3s" ] && service=k3s
  ouvrir "$1"
  distant "set -e
    if ! findmnt -n /srv/chiffre >/dev/null; then
      sudo test -e /dev/mapper/sluis_chiffre || sudo cryptsetup open /srv/chiffre.luks sluis_chiffre
      sudo mount /dev/mapper/sluis_chiffre /srv/chiffre
    fi
    if systemctl list-unit-files $service.service >/dev/null 2>&1; then sudo systemctl start $service; fi
    echo 'Volume ouvert.'"
}

# ── Ansible ──────────────────────────────────────────────────────────────────

preparer_ansible() {
  preparer_ssh
  exiger ansible-playbook ansible-galaxy sops jq
  [ -f "$SLUIS_INVENTAIRE" ] || die "$SLUIS_INVENTAIRE absent : copier ansible/inventaire.exemple.yml"
  [ -n "$SLUIS_SECRETS_SOPS" ] && [ -f "$SLUIS_SECRETS_SOPS" ] \
    || die "SLUIS_SECRETS_SOPS ne désigne aucun fichier (secrets du serveur, dépôt privé)"
  local machine
  for machine in sluis k3s; do
    ouvrir "$machine"
  done
  lire_machines
  SLUIS_HOTE_SLUIS="$(adresse sluis)"
  SLUIS_HOTE_K3S="$(adresse k3s)"
  SLUIS_VLAN_K3S="$(jq -er '.k3s.vlan' <<<"$MACHINES_JSON")"
  export SLUIS_HOTE_SLUIS SLUIS_HOTE_K3S SLUIS_VLAN_K3S
  export ANSIBLE_CONFIG="$ANSIBLE_DIR/ansible.cfg"
  ANSIBLE_SSH_ARGS="${SSH_OPTS[*]}"
  export ANSIBLE_SSH_ARGS
  export ANSIBLE_HOST_KEY_CHECKING=True
  ansible-galaxy collection install -r "$ANSIBLE_DIR/requirements.yml" >/dev/null
}

# Sorties de l'état durable dont les rôles ont besoin, identifiants S3
# compris. Ansible les lit dans l'environnement de son processus
# (SLUIS_INFRA_JSON) : rien ne touche le disque du poste ni la ligne de
# commande. Un descripteur de fichier ne convient pas, Ansible le refuse.
variables_infra() {
  jq -n \
    --argjson endpoint "$(sortie_json durable s3_endpoint)" \
    --argjson compartiments "$(sortie_json durable compartiments)" \
    --argjson identifiants "$(sortie_json durable identifiants_s3)" \
    --argjson kms "$(sortie_json durable kms)" \
    '{infra_s3_endpoint: $endpoint, infra_region_s3: "eu-west-par", infra_compartiments: $compartiments,
      infra_identifiants_s3: $identifiants, infra_kms: $kms}'
}

# Nombre de tâches modifiées, lu dans la sortie JSON d'Ansible et non dans
# son texte : c'est le code de sortie de jq, puis ce nombre, qui font foi.
changements() { jq -e '[.stats[].changed] | add' "$1"; }
echecs() { jq -e '[.stats[] | .failures + .unreachable] | add' "$1"; }

lancer_ansible() {
  local journal="$1"; shift
  local code=0 infra
  infra="$(variables_infra)" || die "sorties de l'état durable illisibles"
  SLUIS_INFRA_JSON="$infra" ANSIBLE_STDOUT_CALLBACK=json ansible-playbook -i "$SLUIS_INVENTAIRE" \
    "$ANSIBLE_DIR/site.yml" "$@" > "$journal" || code=$?
  local nb_echecs
  nb_echecs="$(echecs "$journal")" || die "sortie d'Ansible illisible ($journal)"
  if [ "$code" -ne 0 ] || [ "$nb_echecs" -ne 0 ]; then
    die "Ansible en échec (code $code, $nb_echecs échec(s)) : voir $journal"
  fi
}

configurer() {
  preparer_ansible
  local journal="$SLUIS_INFRA_ETAT/ansible-configurer.json"
  lancer_ansible "$journal"
  echo "Passage terminé : $(changements "$journal") changement(s). Preuve de convergence : $0 verifier" >&2
}

# Contrôle de dérive (ADR-032, règles 3 et 5) : un passage en --check qui
# trouve un écart sort en erreur. Zéro changement est la preuve de
# convergence ; ce journal daté est celle qu'on archive.
verifier() {
  preparer_ansible
  local journal
  journal="$SLUIS_INFRA_ETAT/ansible-verifier-$(date -u +%Y%m%dT%H%M%SZ).json"
  lancer_ansible "$journal" --check --diff
  local nb
  nb="$(changements "$journal")" || die "sortie d'Ansible illisible ($journal)"
  if [ "$nb" -ne 0 ]; then
    die "DÉRIVE : $nb tâche(s) changeraient les machines, voir $journal"
  fi
  echo "Convergé : changed=0 ($journal)." >&2
}

# ── KMS d'OpenBao ────────────────────────────────────────────────────────────
#
# La clé privée du client du KMS naît sur le nœud K3s et n'en sort pas. Seule
# sa demande de certificat, publique, revient au poste ; OpenTofu la fait
# signer par le KMS, et le passage Ansible suivant pose le certificat.

kms() {
  preparer_ssh
  ouvrir k3s
  local csr="$SLUIS_INFRA_ETAT/openbao.csr"
  ssh "${SSH_OPTS[@]}" "$SLUIS_ADMIN@$HOTE" 'sudo cat /srv/chiffre/okms/client.csr' > "$csr.tmp" \
    || { rm -f "$csr.tmp"; die "demande de certificat absente du nœud : lancer d'abord $0 configurer"; }
  grep -q 'BEGIN CERTIFICATE REQUEST' "$csr.tmp" || { rm -f "$csr.tmp"; die "ce que le nœud a rendu n'est pas une demande de certificat"; }
  mv "$csr.tmp" "$csr"
  echo "Demande de certificat dans $csr. Ensuite :" >&2
  echo "  $0 plan durable && $0 appliquer durable && $0 configurer" >&2
}

# ── API de K3s ───────────────────────────────────────────────────────────────
#
# L'API n'est jamais ouverte sur Internet. Le fichier kubeconfig du nœud
# (/etc/rancher/k3s/k3s.yaml) est un secret d'administration : tu le
# rapatries toi-même, ce script ne le lit pas.

tunnel() {
  preparer_ssh
  ouvrir k3s
  echo "API de K3s sur https://127.0.0.1:6443 tant que cette commande tourne (Ctrl-C pour fermer)." >&2
  ssh "${SSH_OPTS[@]}" -N -L 127.0.0.1:6443:127.0.0.1:6443 "$SLUIS_ADMIN@$HOTE"
}

# ── Sauvegardes à Paris ──────────────────────────────────────────────────────
#
# L'identifiant du superviseur relit tous les compartiments et dépose dans
# celui des états. Il vit dans l'état durable, lui-même sauvegardé là-bas :
# en garder une copie hors ligne, sans quoi la perte du poste ferme la porte.

rclone_paris() {
  exiger rclone
  local id
  id="$(sortie_json durable identifiant_superviseur)" || die "identifiant du superviseur illisible"
  RCLONE_CONFIG=/dev/null \
  RCLONE_S3_PROVIDER=Other \
  RCLONE_S3_ENDPOINT="$(sortie_json durable s3_endpoint | jq -er .)" \
  RCLONE_S3_REGION=eu-west-par \
  RCLONE_S3_NO_CHECK_BUCKET=true \
  RCLONE_S3_ACCESS_KEY_ID="$(jq -er .access_key_id <<<"$id")" \
  RCLONE_S3_SECRET_ACCESS_KEY="$(jq -er .secret_access_key <<<"$id")" \
    rclone "$@"
}

compartiment() {
  sortie_json durable compartiments | jq -er --arg c "$1" '.[$c]' \
    || die "compartiment inconnu : $1 (wal-zone-1, wal-zone-2, openbao, sluis, etat, console)"
}

# Ce qu'il faut pour reconstruire si le poste disparaît aussi : états
# (chiffrés par OpenTofu), variables, inventaire, fichier SOPS (chiffré),
# clés d'hôte. La phrase de l'état et la clé SOPS n'y sont pas : zone 3.
sauvegarder_etat() {
  charger_env
  phrase_etat
  local cible horodatage f
  cible="$(compartiment etat)"
  horodatage="$(date -u +%Y%m%dT%H%M%SZ)"
  for f in "$SLUIS_INFRA_ETAT"/*.tfstate "$SLUIS_INFRA_ETAT/known_hosts" "$SLUIS_INFRA_ETAT/openbao.csr" \
           "$SLUIS_INFRA_CONFIG"/*.tfvars "$SLUIS_INVENTAIRE" "$SLUIS_SECRETS_SOPS"; do
    [ -f "$f" ] || continue
    rclone_paris copyto "$f" ":s3:$cible/$horodatage/$(basename "$f")"
  done
  echo "États et inventaires déposés dans $cible/$horodatage/." >&2
}

sauvegardes() {
  [ -n "${1:-}" ] || die "usage : $0 sauvegardes <compartiment> [préfixe]"
  charger_env
  phrase_etat
  rclone_paris lsf ":s3:$(compartiment "$1")/${2:-}"
}

rapatrier() {
  [ -n "${1:-}" ] && [ -n "${2:-}" ] && [ -n "${3:-}" ] || die "usage : $0 rapatrier <compartiment> <préfixe> <dossier>"
  charger_env
  phrase_etat
  mkdir -p "$3"
  chmod 700 "$3"
  rclone_paris copy ":s3:$(compartiment "$1")/$2" "$3"
  echo "Rapatrié dans $3." >&2
}

identifiant_superviseur() {
  [ -t 1 ] || die "cette commande n'écrit que dans un terminal : jamais dans un fichier ni un tube"
  charger_env
  phrase_etat
  echo "À ranger hors ligne (zone 3), avec la phrase de l'état :" >&2
  sortie_json durable identifiant_superviseur | jq -r '"access_key_id     \(.access_key_id)\nsecret_access_key \(.secret_access_key)"'
  sortie_json durable s3_endpoint | jq -r '"endpoint          \(.)"'
}

# Identifiant d'écriture seule que poste/sauvegarder-console.ps1 demande à
# son initialisation, côté Windows.
identifiant_console() {
  [ -t 1 ] || die "cette commande n'écrit que dans un terminal : jamais dans un fichier ni un tube"
  charger_env
  sortie_json durable identifiants_s3 | jq -r '.console | "access_key_id     \(.access_key_id)\nsecret_access_key \(.secret_access_key)"'
  sortie_json durable compartiments | jq -r '"compartiment      \(.console)"'
  sortie_json durable s3_endpoint | jq -r '"endpoint          \(.)"'
}

# ── Règles d'accès des jetons ────────────────────────────────────────────────
#
# Elles s'utilisent telles quelles dans POST /auth/credential. Ce sont des
# préfixes : une règle sur /vrack/* ne vaut pas pour /me/*.

regles_infra() {
  cat <<'JSON'
{
  "accessRules": [
    { "method": "GET",  "path": "/me" },
    { "method": "GET",  "path": "/me/payment/method" },
    { "method": "POST", "path": "/order/cart" },
    { "method": "GET",  "path": "/order/cart/*" },
    { "method": "POST", "path": "/order/cart/*" },
    { "method": "GET",  "path": "/me/order/*" },
    { "method": "POST", "path": "/me/order/*/pay" },
    { "method": "GET",  "path": "/cloud/project" },
    { "method": "GET",  "path": "/cloud/project/*" },
    { "method": "POST", "path": "/cloud/project/*" },
    { "method": "PUT",  "path": "/cloud/project/*" },
    { "method": "GET",  "path": "/v2/publicCloud/project/*" },
    { "method": "POST", "path": "/v2/publicCloud/project/*" },
    { "method": "PUT",  "path": "/v2/publicCloud/project/*" },
    { "method": "GET",  "path": "/vrack" },
    { "method": "GET",  "path": "/vrack/*" },
    { "method": "POST", "path": "/vrack/*" },
    { "method": "PUT",  "path": "/vrack/*" },
    { "method": "GET",  "path": "/v2/okms/*" },
    { "method": "POST", "path": "/v2/okms/*" },
    { "method": "PUT",  "path": "/v2/okms/*" },
    { "method": "GET",  "path": "/me/api/oauth2/client" },
    { "method": "GET",  "path": "/me/api/oauth2/client/*" },
    { "method": "POST", "path": "/me/api/oauth2/client" },
    { "method": "PUT",  "path": "/me/api/oauth2/client/*" },
    { "method": "GET",  "path": "/v2/iam/*" },
    { "method": "POST", "path": "/v2/iam/policy" },
    { "method": "PUT",  "path": "/v2/iam/policy/*" }
  ]
}
JSON
  cat >&2 <<'NOTE'

Aucune règle DELETE : ce jeton ne rend ni machine, ni compartiment, ni
réseau, ni clé. Un plan qui remplace une ressource (sous-réseau, groupe de
sécurité) échouera donc à la suppression : c'est voulu, l'ajouter le temps du
geste, jamais à demeure.

Ces routes sont DÉDUITES de la documentation du fournisseur (v2.25), pas
mesurées. Si un `appliquer` échoue sur « This call has not been granted », le
message nomme la route manquante : l'ajouter seule, jamais /* entier.
NOTE
}

regles_dns() {
  local zone="${1:-}"
  [[ "$zone" =~ ^[a-z0-9.-]+$ ]] || die "usage : $0 regles dns <zone>"
  cat <<JSON
{
  "accessRules": [
    { "method": "GET",    "path": "/domain/zone/$zone" },
    { "method": "GET",    "path": "/domain/zone/$zone/record" },
    { "method": "GET",    "path": "/domain/zone/$zone/record/*" },
    { "method": "POST",   "path": "/domain/zone/$zone/record" },
    { "method": "PUT",    "path": "/domain/zone/$zone/record/*" },
    { "method": "DELETE", "path": "/domain/zone/$zone/record/*" },
    { "method": "POST",   "path": "/domain/zone/$zone/refresh" }
  ]
}
JSON
}

case "${1:-}" in
  regles)
    case "${2:-}" in
      infra) regles_infra ;;
      dns) regles_dns "${3:-}" ;;
      *) die "usage : $0 regles infra | $0 regles dns <zone>" ;;
    esac ;;
  plan) faire_plan "${2:-}" ;;
  appliquer) appliquer "${2:-}" ;;
  etat) etat "${2:-}" ;;
  empreinte) empreinte "${2:-}" "${3:-}" ;;
  totp) totp "${2:-}" ;;
  chiffrer) chiffrer "${2:-}" ;;
  deverrouiller) deverrouiller "${2:-}" ;;
  configurer) configurer ;;
  kms) kms ;;
  verifier) verifier ;;
  tunnel) tunnel ;;
  sauvegarder-etat) sauvegarder_etat ;;
  sauvegardes) sauvegardes "${2:-}" "${3:-}" ;;
  rapatrier) rapatrier "${2:-}" "${3:-}" "${4:-}" ;;
  identifiant-superviseur) identifiant_superviseur ;;
  identifiant-console) identifiant_console ;;
  *) sed -n '2,33p' "$0" | sed 's/^# \{0,1\}//' ; exit 2 ;;
esac
