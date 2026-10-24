#!/usr/bin/env bash
# Tests du tireur GitOps d'ADR-013 (story 13.2, #25).
#
#   make test-tireur          dans un conteneur jetable, en local comme en CI
#
# Le harnais doit tourner en root, et c'est pourquoi `make` le lance dans un
# conteneur : la règle « allowed_signers propriété de root, en lecture seule
# pour le compte qui déploie » ne se teste qu'avec deux comptes. Le harnais
# prépare les fichiers en root, puis lance le tireur sous `nobody`, le compte
# qui déploie. Chaque test part d'un dépôt git jetable et de clés SSH générées
# pour l'occasion : aucune clé réelle, aucun réseau.
#
# Ce qui est vérifié, c'est le CODE DE SORTIE du tireur, puis la présence de
# l'erreur et de son motif dans le journal, puis l'absence de tout appel au
# déploiement. TIREUR_SCRIPT permet de viser une copie modifiée du tireur, pour
# les témoins (défaut remis, le test doit échouer).
set -euo pipefail

RACINE="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TIREUR_SCRIPT="${TIREUR_SCRIPT:-$RACINE/tools/tireur-etiquette-signee.sh.example}"

if [ "$(id -u)" -ne 0 ]; then
  echo "tests/tireur : à lancer en root, dans un conteneur (make test-tireur)." >&2
  exit 2
fi
for outil in git ssh-keygen setpriv flock; do
  command -v "$outil" >/dev/null || { echo "outil absent : $outil" >&2; exit 2; }
done

DEPLOYEUR=nobody
DEPLOYEUR_GID="$(id -g "$DEPLOYEUR")"
ENVIRONNEMENT=production
EMPREINTE="$(printf 'plan de changement' | sha256sum | cut -d' ' -f1)"
COURT="${EMPREINTE:0:12}"

reussis=0
echecs=0
echecs_noms=()

# ── Montage d'un monde jetable ─────────────────────────────────────────────

nouveau_monde() {
  T="$(mktemp -d)"
  chmod 755 "$T"
  mkdir -p "$T/conf" "$T/etat" "$T/cles" "$T/home" "$T/bin"
  chmod 700 "$T/cles"

  ssh-keygen -q -t ed25519 -N "" -C "superviseur" -f "$T/cles/superviseur"
  ssh-keygen -q -t ed25519 -N "" -C "intrus" -f "$T/cles/intrus"
  printf 'superviseur@exemple.org %s\n' "$(cat "$T/cles/superviseur.pub")" > "$T/conf/allowed_signers"
  chmod 644 "$T/conf/allowed_signers"

  git init -q "$T/depot"
  g commit -q --allow-empty -m "premier"
  COMMIT_A="$(g rev-parse HEAD)"
  g commit -q --allow-empty -m "second"
  COMMIT_B="$(g rev-parse HEAD)"

  # Le déploiement factice : il note qu'il a été appelé, et si l'empreinte
  # était déjà consignée à ce moment-là.
  cat > "$T/bin/deployer" <<EOF
#!/usr/bin/env bash
if awk -v e="\$2" '\$1 == e { t = 1 } END { exit t ? 0 : 1 }' "$T/etat/registre"; then
  ordre=consigne_avant
else
  ordre=consigne_apres
fi
printf '%s %s %s %s\n' "\$1" "\$2" "\$3" "\$ordre" >> "$T/etat/deploiements"
EOF
  chmod 755 "$T/bin/deployer"
  chown "$DEPLOYEUR:$DEPLOYEUR_GID" "$T/etat" "$T/home"

  ALLOWED="$T/conf/allowed_signers"
  RENDRE_A_ROOT=()
}

# git en root sur le dépôt jetable, identité de test.
g() {
  git -C "$T/depot" -c safe.directory='*' -c user.name=test \
    -c user.email=test@exemple.org "$@"
}

# etiqueter <clé|non> <nom> <commit> <message> [âge en secondes]
etiqueter() {
  local cle="$1" nom="$2" commit="$3" message="$4" age="${5:-0}" date
  date="@$(($(date +%s) - age)) +0000"
  if [ "$cle" = non ]; then
    GIT_COMMITTER_DATE="$date" g tag -a "$nom" "$commit" -m "$message"
  else
    GIT_COMMITTER_DATE="$date" g -c gpg.format=ssh \
      -c user.signingkey="$T/cles/$cle" tag -s "$nom" "$commit" -m "$message"
  fi
}

# Lance le tireur sous le compte qui déploie ; rend son code de sortie.
tirer() {
  chown -R "$DEPLOYEUR:$DEPLOYEUR_GID" "$T/depot"
  if [ "${#RENDRE_A_ROOT[@]}" -gt 0 ]; then chown root:root "${RENDRE_A_ROOT[@]}"; fi
  local commande=(env -i PATH="/usr/local/bin:/usr/bin:/bin" HOME="$T/home"
    TIREUR_ENVIRONNEMENT="$ENVIRONNEMENT"
    TIREUR_DEPOT="$T/depot"
    TIREUR_ALLOWED_SIGNERS="$ALLOWED"
    TIREUR_REGISTRE="$T/etat/registre"
    TIREUR_JOURNAL="$T/etat/journal"
    TIREUR_DEPLOIEMENT="$T/bin/deployer"
    bash "$TIREUR_SCRIPT" "$1")
  setpriv --reuid="$DEPLOYEUR" --regid="$DEPLOYEUR_GID" --clear-groups \
    "${commande[@]}" 2>"$T/sortie_erreur"
}

# ── Assertions ─────────────────────────────────────────────────────────────

ECHEC_COURANT=""
echoue() { ECHEC_COURANT="${ECHEC_COURANT}    - $*"$'\n'; }

attendre_code() {
  [ "$1" -eq "$2" ] || echoue "code de sortie $1, attendu $2"
}

attendre_aucun_deploiement() {
  if [ -s "$T/etat/deploiements" ]; then
    echoue "le déploiement a été appelé : $(cat "$T/etat/deploiements")"
  fi
}

# L'erreur doit être dans la sortie d'erreur, et dans le journal quand il est
# ouvert ; c'est le contraire du silence.
attendre_erreur() {
  local motif="$1" ou
  for ou in "$T/sortie_erreur" ${2:+"$T/etat/journal"}; do
    if ! grep -Eq " ERREUR tireur .*motif=${motif}( |\$)" "$ou" 2>/dev/null; then
      echoue "aucune ERREUR motif=$motif dans $(basename "$ou")"
    fi
  done
}

refus_attendu() {
  # $1 code obtenu, $2 code attendu, $3 motif
  attendre_code "$1" "$2"
  attendre_erreur "$3" journal
  attendre_aucun_deploiement
}

scenario() {
  local nom="$1"; shift
  ECHEC_COURANT=""
  nouveau_monde
  "$@"
  if [ -z "$ECHEC_COURANT" ]; then
    reussis=$((reussis + 1)); printf '  ok    %s\n' "$nom"
  else
    echecs=$((echecs + 1)); echecs_noms+=("$nom")
    printf '  ÉCHEC %s\n%s' "$nom" "$ECHEC_COURANT"
    sed 's/^/        | /' "$T/sortie_erreur" 2>/dev/null || true
  fi
  rm -rf "$T"
}

# ── @happy ─────────────────────────────────────────────────────────────────

t_happy() {
  etiqueter superviseur "approuve/$COURT" "$COMMIT_B" "$ENVIRONNEMENT $EMPREINTE"
  local code=0; tirer "$COMMIT_B" || code=$?
  attendre_code "$code" 0
  grep -qx "$COMMIT_B $EMPREINTE $ENVIRONNEMENT consigne_avant" "$T/etat/deploiements" 2>/dev/null \
    || echoue "déploiement absent, ou appelé avant la consignation de l'empreinte"
  awk -v e="$EMPREINTE" -v c="$COMMIT_B" '$1 == e && $2 == c { t = 1 } END { exit t ? 0 : 1 }' \
    "$T/etat/registre" || echoue "empreinte non consignée dans le registre"
  grep -Eq " INFO tireur .*deploiement empreinte=$EMPREINTE commit=$COMMIT_B cle=SHA256:[^ ]+ principal=superviseur@exemple.org date_signature=[0-9]+ horodatage=[0-9T:Z-]+\$" \
    "$T/etat/journal" || echoue "journal sans empreinte, commit, clé et horodatage"
  if grep -q ' ERREUR ' "$T/etat/journal"; then echoue "erreur journalisée sur le chemin nominal"; fi
}

# ── @negative ──────────────────────────────────────────────────────────────

t_non_signee() {
  etiqueter non "approuve/$COURT" "$COMMIT_B" "$ENVIRONNEMENT $EMPREINTE"
  local code=0; tirer "$COMMIT_B" || code=$?
  refus_attendu "$code" 1 non_signee
}

t_etiquette_legere() {
  g tag "approuve/$COURT" "$COMMIT_B"
  local code=0; tirer "$COMMIT_B" || code=$?
  refus_attendu "$code" 1 non_signee
}

t_cle_inconnue() {
  etiqueter intrus "approuve/$COURT" "$COMMIT_B" "$ENVIRONNEMENT $EMPREINTE"
  local code=0; tirer "$COMMIT_B" || code=$?
  refus_attendu "$code" 1 cle_inconnue
}

# ── @edge : un motif distinct pour chacun des cinq cas ─────────────────────

t_perimee() {
  etiqueter superviseur "approuve/$COURT" "$COMMIT_B" "$ENVIRONNEMENT $EMPREINTE" $((73 * 3600))
  local code=0; tirer "$COMMIT_B" || code=$?
  refus_attendu "$code" 1 perimee
}

t_juste_fraiche() {
  # Garde de la borne : 71 h passe.
  etiqueter superviseur "approuve/$COURT" "$COMMIT_B" "$ENVIRONNEMENT $EMPREINTE" $((71 * 3600))
  local code=0; tirer "$COMMIT_B" || code=$?
  attendre_code "$code" 0
}

t_deja_consommee() {
  etiqueter superviseur "approuve/$COURT" "$COMMIT_B" "$ENVIRONNEMENT $EMPREINTE"
  local code=0; tirer "$COMMIT_B" || code=$?
  attendre_code "$code" 0
  code=0; tirer "$COMMIT_B" || code=$?
  attendre_code "$code" 1
  attendre_erreur deja_consommee journal
  [ "$(wc -l < "$T/etat/deploiements")" -eq 1 ] || echoue "déployé plus d'une fois"
}

t_autre_commit() {
  etiqueter superviseur "approuve/$COURT" "$COMMIT_A" "$ENVIRONNEMENT $EMPREINTE"
  local code=0; tirer "$COMMIT_B" || code=$?
  refus_attendu "$code" 1 autre_commit
}

t_autre_environnement() {
  # Garde-fou 6 d'ADR-013 : une approbation de staging ne vaut pas en
  # production.
  etiqueter superviseur "approuve/$COURT" "$COMMIT_B" "staging $EMPREINTE"
  local code=0; tirer "$COMMIT_B" || code=$?
  refus_attendu "$code" 1 autre_environnement
}

t_message_mal_forme() {
  local messages=(
    "$ENVIRONNEMENT  $EMPREINTE"
    "$ENVIRONNEMENT ${EMPREINTE^^}"
    "$ENVIRONNEMENT ${EMPREINTE:0:63}"
    "$ENVIRONNEMENT ${EMPREINTE}0"
    "prod $EMPREINTE"
    "$EMPREINTE $ENVIRONNEMENT"
    "$ENVIRONNEMENT $EMPREINTE extra"
    "$ENVIRONNEMENT"
    $'production '"$EMPREINTE"$'\n\nseconde ligne'
  )
  local i=0 message code
  for message in "${messages[@]}"; do
    i=$((i + 1))
    rm -f "$T/etat/journal"
    g tag -d "approuve/$COURT" >/dev/null 2>&1 || true
    etiqueter superviseur "approuve/$COURT" "$COMMIT_B" "$message"
    code=0; tirer "$COMMIT_B" || code=$?
    attendre_code "$code" 1
    attendre_erreur message_mal_forme journal
  done
  attendre_aucun_deploiement
}

t_nom_incoherent() {
  etiqueter superviseur "approuve/0123456789ab" "$COMMIT_B" "$ENVIRONNEMENT $EMPREINTE"
  local code=0; tirer "$COMMIT_B" || code=$?
  refus_attendu "$code" 1 nom_incoherent
}

t_aucune_etiquette() {
  local code=0; tirer "$COMMIT_B" || code=$?
  refus_attendu "$code" 1 aucune_etiquette
}

# ── @security ──────────────────────────────────────────────────────────────

t_allowed_signers_dans_depot() {
  etiqueter superviseur "approuve/$COURT" "$COMMIT_B" "$ENVIRONNEMENT $EMPREINTE"
  # Mêmes droits qu'en bonne place (fichier et répertoire à root) : seule la
  # place change, pour que seul le motif « dans le dépôt » puisse jouer.
  mkdir "$T/depot/conf"
  cp -p "$ALLOWED" "$T/depot/conf/allowed_signers"
  ALLOWED="$T/depot/conf/allowed_signers"
  RENDRE_A_ROOT=("$T/depot/conf" "$ALLOWED")
  local code=0; tirer "$COMMIT_B" || code=$?
  refus_attendu "$code" 2 allowed_signers_dans_depot
}

t_allowed_signers_lien_vers_depot() {
  etiqueter superviseur "approuve/$COURT" "$COMMIT_B" "$ENVIRONNEMENT $EMPREINTE"
  # Un lien hors du dépôt qui pointe dedans : le chemin est résolu.
  cp -p "$ALLOWED" "$T/depot/.git/allowed_signers"
  ln -s "$T/depot/.git/allowed_signers" "$T/conf/lien"
  ALLOWED="$T/conf/lien"
  RENDRE_A_ROOT=("$T/depot/.git/allowed_signers")
  local code=0; tirer "$COMMIT_B" || code=$?
  refus_attendu "$code" 2 allowed_signers_dans_depot
}

t_allowed_signers_au_deployeur() {
  etiqueter superviseur "approuve/$COURT" "$COMMIT_B" "$ENVIRONNEMENT $EMPREINTE"
  chown "$DEPLOYEUR" "$ALLOWED"
  chmod 444 "$ALLOWED"
  local code=0; tirer "$COMMIT_B" || code=$?
  refus_attendu "$code" 2 allowed_signers_modifiable
}

t_allowed_signers_inscriptible() {
  etiqueter superviseur "approuve/$COURT" "$COMMIT_B" "$ENVIRONNEMENT $EMPREINTE"
  chmod 666 "$ALLOWED"
  local code=0; tirer "$COMMIT_B" || code=$?
  refus_attendu "$code" 2 allowed_signers_modifiable
}

t_repertoire_inscriptible() {
  etiqueter superviseur "approuve/$COURT" "$COMMIT_B" "$ENVIRONNEMENT $EMPREINTE"
  chown "$DEPLOYEUR" "$T/conf"
  local code=0; tirer "$COMMIT_B" || code=$?
  refus_attendu "$code" 2 allowed_signers_modifiable
}

t_compte_root() {
  etiqueter superviseur "approuve/$COURT" "$COMMIT_B" "$ENVIRONNEMENT $EMPREINTE"
  # Le dépôt reste à root, qui l'a créé : seul le compte est en cause.
  chown -R root:root "$T/etat"
  local code=0
  env -i PATH="/usr/local/bin:/usr/bin:/bin" HOME="$T/home" \
    TIREUR_ENVIRONNEMENT="$ENVIRONNEMENT" TIREUR_DEPOT="$T/depot" \
    TIREUR_ALLOWED_SIGNERS="$ALLOWED" TIREUR_REGISTRE="$T/etat/registre" \
    TIREUR_JOURNAL="$T/etat/journal" TIREUR_DEPLOIEMENT="$T/bin/deployer" \
    bash "$TIREUR_SCRIPT" "$COMMIT_B" 2>"$T/sortie_erreur" || code=$?
  refus_attendu "$code" 2 compte_root
}

t_environnement_inconnu() {
  etiqueter superviseur "approuve/$COURT" "$COMMIT_B" "$ENVIRONNEMENT $EMPREINTE"
  ENVIRONNEMENT=prod
  local code=0; tirer "$COMMIT_B" || code=$?
  ENVIRONNEMENT=production
  refus_attendu "$code" 2 environnement_inconnu
}

printf "Tireur de l'étiquette signée : %s\n" "$TIREUR_SCRIPT"
printf '@happy\n'
scenario "étiquette valide : consignée, déployée, journalisée" t_happy
printf '@negative\n'
scenario "étiquette annotée non signée" t_non_signee
scenario "étiquette légère" t_etiquette_legere
scenario "clé absente d'allowed_signers" t_cle_inconnue
printf '@edge\n'
scenario "plus de 72 h" t_perimee
scenario "71 h : encore fraîche" t_juste_fraiche
scenario "déjà consommée" t_deja_consommee
scenario "posée sur un autre commit" t_autre_commit
scenario "nomme un autre environnement" t_autre_environnement
scenario "message mal formé (9 variantes)" t_message_mal_forme
scenario "nom incohérent avec l'empreinte signée" t_nom_incoherent
scenario "aucune étiquette" t_aucune_etiquette
printf '@security\n'
scenario "allowed_signers dans le dépôt" t_allowed_signers_dans_depot
scenario "allowed_signers lié dans le dépôt" t_allowed_signers_lien_vers_depot
scenario "allowed_signers au compte qui déploie" t_allowed_signers_au_deployeur
scenario "allowed_signers inscriptible par tous" t_allowed_signers_inscriptible
scenario "répertoire d'allowed_signers au compte qui déploie" t_repertoire_inscriptible
scenario "tireur lancé en root" t_compte_root
scenario "environnement du tireur inconnu" t_environnement_inconnu

printf '\n%d réussis, %d en échec\n' "$reussis" "$echecs"
if [ "$echecs" -ne 0 ]; then
  printf '  en échec : %s\n' "${echecs_noms[@]}"
  exit 1
fi
