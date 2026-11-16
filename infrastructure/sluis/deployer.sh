#!/usr/bin/env bash
# Louer et configurer le petit VPS Sluis (ADR-043), depuis le poste du
# superviseur, et de là seulement (ADR-037, invariant 1).
#
#   ./deployer.sh regles location        règles d'accès du jeton de location
#   ./deployer.sh regles dns <zone>      règles d'accès du jeton DNS
#   ./deployer.sh plan                   plan OpenTofu, chiffré, rien n'est loué
#   ./deployer.sh louer                  applique le dernier plan (COMMANDE PAYANTE)
#   ./deployer.sh empreinte <SHA256:…>   vérifie la clé d'hôte lue dans la console
#   ./deployer.sh totp                   enrôle le TOTP de l'administrateur, sur la machine
#   ./deployer.sh chiffrer               crée le volume chiffré (phrase tapée par toi)
#   ./deployer.sh deverrouiller          rouvre le volume après un redémarrage
#   ./deployer.sh configurer             passage Ansible complet
#   ./deployer.sh verifier               dérive : --check --diff, changed=0 exigé
#   ./deployer.sh etat                   sorties OpenTofu (nom du VPS, état, DNS)
#
# Le script ne conserve rien d'autre que ce qu'OpenTofu et Ansible écrivent
# dans SLUIS_INFRA_ETAT, hors du dépôt. Il n'affiche jamais un secret. Il ne
# rend jamais le VPS : `prevent_destroy` l'interdit, et le jeton de location
# n'a pas les routes de résiliation.
set -euo pipefail

ICI="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TOFU_DIR="$ICI/tofu"
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
    [[ "$cle" =~ ^[A-Z_][A-Z0-9_]*$ ]] || die "ligne illisible dans $FICHIER_ENV (clé « $cle »)"
    # L'environnement du shell l'emporte sur le fichier.
    if [ -z "${!cle:-}" ]; then
      export "$cle=$valeur"
    fi
  done < "$FICHIER_ENV"

  SLUIS_INFRA_ETAT="$(developper "${SLUIS_INFRA_ETAT:-$HOME/.local/state/sluis-infra}")"
  SLUIS_INFRA_TFVARS="$(developper "${SLUIS_INFRA_TFVARS:-$HOME/.config/sluis-infra/sluis.tfvars}")"
  SLUIS_INVENTAIRE="$(developper "${SLUIS_INVENTAIRE:-$HOME/.config/sluis-infra/inventaire.yml}")"
  SLUIS_CLE_SSH="$(developper "${SLUIS_CLE_SSH:-$HOME/.ssh/id_ed25519_sk}")"
  SLUIS_SECRETS_SOPS="$(developper "${SLUIS_SECRETS_SOPS:-}")"
  SLUIS_ADMIN="${SLUIS_ADMIN:-debian}"
  SLUIS_TAILLE_CHIFFRE="${SLUIS_TAILLE_CHIFFRE:-20G}"
  export SLUIS_CLE_SSH SLUIS_SECRETS_SOPS

  case "$SLUIS_INFRA_ETAT" in
    "$ICI"*|"$(cd "$ICI/../.." && pwd)"*) die "SLUIS_INFRA_ETAT pointe dans le dépôt : l'état n'y a pas sa place" ;;
  esac
  mkdir -p "$SLUIS_INFRA_ETAT"
  chmod 700 "$SLUIS_INFRA_ETAT"
  export TF_VAR_chemin_etat="$SLUIS_INFRA_ETAT/sluis.tfstate"
  export TF_DATA_DIR="$SLUIS_INFRA_ETAT/tofu"
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

exiger_jeton_location() {
  local v
  for v in OVH_APPLICATION_KEY OVH_APPLICATION_SECRET OVH_CONSUMER_KEY; do
    [ -n "${!v:-}" ] || die "$v vide : jeton de location absent (voir .env.exemple, section 1)"
  done
}

tofu_init() {
  exiger tofu
  phrase_etat
  [ -f "$SLUIS_INFRA_TFVARS" ] || die "$SLUIS_INFRA_TFVARS absent : copier tofu/sluis.tfvars.example"
  tofu -chdir="$TOFU_DIR" init -input=false -lockfile=readonly >/dev/null
}

sortie() { tofu -chdir="$TOFU_DIR" output -raw "$1"; }

# ── Règles d'accès des jetons ────────────────────────────────────────────────
#
# Elles s'utilisent telles quelles dans POST /auth/credential. Ce sont des
# préfixes : une règle sur /vps/* ne vaut pas pour /me/*.

regles_location() {
  cat <<'EOF'
{
  "accessRules": [
    { "method": "GET",  "path": "/me" },
    { "method": "GET",  "path": "/me/payment/method" },
    { "method": "POST", "path": "/order/cart" },
    { "method": "POST", "path": "/order/cart/*/assign" },
    { "method": "POST", "path": "/order/cart/*/vps" },
    { "method": "POST", "path": "/order/cart/*/item/*/configuration" },
    { "method": "GET",  "path": "/order/cart/*/checkout" },
    { "method": "POST", "path": "/order/cart/*/checkout" },
    { "method": "POST", "path": "/me/order/*/pay" },
    { "method": "GET",  "path": "/me/order/*" },
    { "method": "GET",  "path": "/vps/*" },
    { "method": "PUT",  "path": "/vps/*" },
    { "method": "POST", "path": "/vps/*/rebuild" }
  ]
}
EOF
  cat >&2 <<'EOF'

Aucune route de résiliation (/vps/*/terminate, confirmTermination) ni de
lecture des courriels (/me/notification/*) : ce jeton ne peut pas rendre le
VPS. Si le premier `louer` échoue sur « This call has not been granted »,
le message nomme la route manquante : l'ajouter seule, jamais /* entier.
EOF
}

regles_dns() {
  local zone="${1:-}"
  [[ "$zone" =~ ^[a-z0-9.-]+$ ]] || die "usage : $0 regles dns <zone>"
  cat <<EOF
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
EOF
}

# ── OpenTofu ─────────────────────────────────────────────────────────────────

faire_plan() {
  charger_env
  exiger_jeton_location
  tofu_init
  tofu -chdir="$TOFU_DIR" plan -input=false \
    -var-file="$SLUIS_INFRA_TFVARS" -out="$SLUIS_INFRA_ETAT/sluis.tfplan"
  echo "Plan chiffré dans $SLUIS_INFRA_ETAT/sluis.tfplan. Le relire, puis : $0 louer" >&2
}

louer() {
  charger_env
  exiger_jeton_location
  tofu_init
  local plan="$SLUIS_INFRA_ETAT/sluis.tfplan"
  [ -f "$plan" ] || die "aucun plan : lancer d'abord $0 plan"
  echo "Appliquer ce plan PASSE UNE COMMANDE PAYÉE par le moyen de paiement du compte." >&2
  local reponse
  read -r -p "Taper « louer » pour confirmer : " reponse
  [ "$reponse" = "louer" ] || die "abandon, rien n'a été commandé"
  tofu -chdir="$TOFU_DIR" apply -input=false "$plan"
  rm -f "$plan"
  echo >&2
  echo "Étape suivante : relever l'empreinte de la clé d'hôte ED25519 dans la" >&2
  echo "console KVM de l'espace client (ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub)," >&2
  echo "puis : $0 empreinte SHA256:..." >&2
}

etat() {
  charger_env
  tofu_init
  tofu -chdir="$TOFU_DIR" output
}

# ── Clé d'hôte ───────────────────────────────────────────────────────────────
#
# Jamais de confiance au premier contact (ADR-037, phase 2) : l'empreinte vue
# par le réseau doit être celle lue dans la console de l'hébergeur.

empreinte() {
  local attendue="${1:-}"
  [[ "$attendue" =~ ^SHA256:[A-Za-z0-9+/]{43}$ ]] || die "usage : $0 empreinte SHA256:<43 caractères>"
  charger_env
  tofu_init
  exiger ssh-keyscan ssh-keygen
  local hote vue tmp
  hote="$(sortie service)"
  tmp="$(mktemp)"
  trap 'rm -f "$tmp"' RETURN
  ssh-keyscan -t ed25519 "$hote" 2>/dev/null > "$tmp"
  [ -s "$tmp" ] || die "aucune clé ed25519 lue sur $hote"
  vue="$(ssh-keygen -lf "$tmp" | awk '{print $2}')"
  if [ "$vue" != "$attendue" ]; then
    die "empreinte vue ($vue) différente de celle de la console : NE PAS se connecter"
  fi
  install -m 600 "$tmp" "$SLUIS_INFRA_ETAT/known_hosts"
  echo "Clé d'hôte vérifiée et enregistrée pour $hote." >&2
}

# ── SSH ──────────────────────────────────────────────────────────────────────
#
# Une connexion maîtresse, ouverte une fois (clé, puis code TOTP tapé par
# toi), que les commandes suivantes et Ansible réutilisent : Ansible ne sait
# pas répondre à une invite TOTP, et il n'a pas à le savoir.

preparer_ssh() {
  charger_env
  tofu_init
  exiger ssh
  [ -f "$SLUIS_INFRA_ETAT/known_hosts" ] || die "clé d'hôte non vérifiée : lancer d'abord $0 empreinte"
  SLUIS_HOTE="$(sortie service)"
  export SLUIS_HOTE
  SSH_OPTS=(
    -i "$SLUIS_CLE_SSH"
    -o IdentitiesOnly=yes
    -o "UserKnownHostsFile=$SLUIS_INFRA_ETAT/known_hosts"
    -o StrictHostKeyChecking=yes
    -o ControlMaster=auto
    -o "ControlPath=$SLUIS_INFRA_ETAT/ssh-%C"
    -o ControlPersist=15m
  )
  if ! ssh "${SSH_OPTS[@]}" -O check "$SLUIS_ADMIN@$SLUIS_HOTE" 2>/dev/null; then
    echo "Connexion à $SLUIS_HOTE (clé, puis code TOTP s'il est déjà imposé)." >&2
    ssh "${SSH_OPTS[@]}" -o ControlMaster=yes -fN "$SLUIS_ADMIN@$SLUIS_HOTE"
  fi
}

# Commande distante dans un vrai terminal : les invites (TOTP, phrase LUKS)
# s'affichent chez toi, et rien de ce que tu tapes ne passe par ce script.
distant() { ssh "${SSH_OPTS[@]}" -t "$SLUIS_ADMIN@$SLUIS_HOTE" "$@"; }

# Le secret TOTP naît sur la machine et s'affiche dans TON terminal, jamais
# dans un fichier du poste ni dans une sortie qu'un agent lirait (ADR-037).
# Codes de secours : à noter et ranger en zone 3.
totp() {
  preparer_ssh
  distant 'test ! -e ~/.google_authenticator || { echo "TOTP déjà enrôlé : rien à faire."; exit 0; }
    google-authenticator --time-based --disallow-reuse --force --rate-limit=3 --rate-time=30 --window-size=3'
  echo "Enrôlé. Il sera imposé au prochain : $0 configurer" >&2
}

# Création du volume chiffré, une seule fois. cryptsetup demande la phrase
# deux fois dans ton terminal ; elle n'est écrite nulle part sur la machine
# (aucune entrée dans crypttab). Sa perte rend les données illisibles : la
# ranger en zone 3 avant de taper la seconde fois.
chiffrer() {
  preparer_ssh
  [[ "$SLUIS_TAILLE_CHIFFRE" =~ ^[0-9]+G$ ]] || die "SLUIS_TAILLE_CHIFFRE au format <n>G"
  distant "set -e
    if sudo test -e /srv/chiffre.luks; then echo 'Volume déjà créé : rien à faire.'; exit 0; fi
    sudo install -d -m 0700 /srv
    sudo fallocate -l $SLUIS_TAILLE_CHIFFRE /srv/chiffre.luks
    sudo chmod 600 /srv/chiffre.luks
    sudo cryptsetup luksFormat --type luks2 --cipher aes-xts-plain64 --key-size 512 --pbkdf argon2id --verify-passphrase /srv/chiffre.luks
    sudo cryptsetup open /srv/chiffre.luks sluis_chiffre
    sudo mkfs.ext4 -q -L sluis_chiffre /dev/mapper/sluis_chiffre
    sudo install -d -m 0755 /srv/chiffre
    sudo mount /dev/mapper/sluis_chiffre /srv/chiffre
    echo 'Volume chiffré créé et monté.'"
}

# Après tout redémarrage du VPS : Docker attend ce montage et Sluis reste
# arrêté jusque-là. C'est le prix d'une clé qui ne vit pas sur la machine.
deverrouiller() {
  preparer_ssh
  distant "set -e
    if findmnt -n /srv/chiffre >/dev/null; then echo 'Déjà ouvert.'; exit 0; fi
    sudo test -e /dev/mapper/sluis_chiffre || sudo cryptsetup open /srv/chiffre.luks sluis_chiffre
    sudo mount /dev/mapper/sluis_chiffre /srv/chiffre
    sudo systemctl start docker
    echo 'Volume ouvert, Docker démarré.'"
}

# ── Ansible ──────────────────────────────────────────────────────────────────

preparer_ansible() {
  preparer_ssh
  exiger ansible-playbook ansible-galaxy sops jq
  [ -f "$SLUIS_INVENTAIRE" ] || die "$SLUIS_INVENTAIRE absent : copier ansible/inventaire.exemple.yml"
  [ -n "$SLUIS_SECRETS_SOPS" ] && [ -f "$SLUIS_SECRETS_SOPS" ] \
    || die "SLUIS_SECRETS_SOPS ne désigne aucun fichier (secrets du serveur, dépôt privé)"
  export ANSIBLE_CONFIG="$ANSIBLE_DIR/ansible.cfg"
  ANSIBLE_SSH_ARGS="${SSH_OPTS[*]}"
  export ANSIBLE_SSH_ARGS
  export ANSIBLE_HOST_KEY_CHECKING=True
  ansible-galaxy collection install -r "$ANSIBLE_DIR/requirements.yml" >/dev/null
}

# Nombre de tâches modifiées, lu dans la sortie JSON d'Ansible et non dans
# son texte : c'est le code de sortie de jq, puis ce nombre, qui font foi.
changements() { jq -e '[.stats[].changed] | add' "$1"; }
echecs() { jq -e '[.stats[] | .failures + .unreachable] | add' "$1"; }

lancer_ansible() {
  local journal="$1"; shift
  local code=0
  ANSIBLE_STDOUT_CALLBACK=json ansible-playbook -i "$SLUIS_INVENTAIRE" \
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
    die "DÉRIVE : $nb tâche(s) changeraient la machine, voir $journal"
  fi
  echo "Convergé : changed=0 ($journal)." >&2
}

case "${1:-}" in
  regles)
    case "${2:-}" in
      location) regles_location ;;
      dns) regles_dns "${3:-}" ;;
      *) die "usage : $0 regles location | $0 regles dns <zone>" ;;
    esac ;;
  plan) faire_plan ;;
  louer) louer ;;
  empreinte) empreinte "${2:-}" ;;
  totp) totp ;;
  chiffrer) chiffrer ;;
  deverrouiller) deverrouiller ;;
  configurer) configurer ;;
  verifier) verifier ;;
  etat) etat ;;
  *) sed -n '2,18p' "$0" | sed 's/^# \{0,1\}//' ; exit 2 ;;
esac
