#!/usr/bin/env bash
# Déploiement GitOps de Sluis sur le serveur ecosolva.
#
# Sur le serveur cible, une fois :  ./deploy.sh
#   -> programme un cron qui rappelle ce script avec --run.
# Le cron appelle ensuite :        ./deploy.sh --run
#   -> tire et redéploie si origin/main a bougé, sinon ne fait rien.
#
# Repris de derniere-chance : le serveur ne construit rien, les images sont
# produites en CI et poussées sur GHCR. Idempotent dans les deux modes : une
# seconde exécution sans nouveau commit n'a aucun effet, ce qu'un test vérifie.
#
# Deux déploiements possibles, du même commit et donc du même binaire : le
# Sluis public, toujours, et le Sluis exécutant si `sluis.executant.toml`
# existe à la racine. Les deux sont épinglés sur le MÊME tag, et le
# déploiement attend que les deux images soient publiées avant de bouger.
set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRON_SCHEDULE="${CRON_SCHEDULE:-*/5 * * * *}"
CRON_MARKER="sluis-auto-deploy"
LOG_FILE="$REPO_DIR/deploy.log"
LOCK_FILE="$REPO_DIR/.deploy.lock"
DEPLOYED_REV_FILE="$REPO_DIR/.deployed_rev"

# Image GHCR à épingler. La CI la publie sous le SHA complet du commit en plus
# de `latest` (voir .github/workflows/docker-publish.yml).
IMAGE_REPO="${IMAGE_REPO:-ghcr.io/gilmry/sluis}"

# Le Sluis exécutant ne se déploie QUE si sa configuration existe. C'est le
# seul interrupteur, et il est délibérément à la main : tant que H1 n'est pas
# levée, il n'y a ni projet OVH de bac à sable ni dépôt à mesurer, et déployer
# un second Sluis en lecture ne servirait à rien. Voir config/sluis.executant.toml.
#
# Le choix passe par un FICHIER et non par un profil : Compose interpole les
# variables de tous les services avant de filtrer par profil, si bien qu'un
# `${VAR:?}` sur un service hors profil ferait quand même échouer la commande.
# Un serveur qui ne veut pas de l'exécutant n'a pas ses variables, et le
# déploiement du Sluis public en serait bloqué.
CONFIG_EXECUTANT="$REPO_DIR/sluis.executant.toml"
COMPOSE=(docker compose -f docker-compose.yml)
IMAGE_SERVICES=(server)
if [ -f "$CONFIG_EXECUTANT" ]; then
  COMPOSE+=(-f docker-compose.executant.yml)
  IMAGE_SERVICES+=(executant)
fi
COMPOSE+=(--profile prod)

log() { echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] $*" >> "$LOG_FILE"; }

# Vraie ou fausse selon que le tag existe dans le registre, sans le tirer.
image_published() {
  docker manifest inspect "$1" >/dev/null 2>&1
}

# Vraie seulement si TOUTES les images du déploiement portent ce tag.
#
# Une seule des deux publiée ne suffit pas : déployer le Sluis public d'un
# commit et l'exécutant du précédent, c'est faire tourner deux versions du même
# binaire en se croyant sur une seule.
images_publiees() {
  local tag="$1" service
  for service in "${IMAGE_SERVICES[@]}"; do
    image_published "$IMAGE_REPO/$service:$tag" || return 1
  done
}

# Épingle les images sur un tag précis, pour que `docker compose` résolve des
# images déterminées et non un `latest` mouvant.
pin_images() {
  export SLUIS_IMAGE="$IMAGE_REPO/server:$1"
  export SLUIS_IMAGE_EXECUTANT="$IMAGE_REPO/executant:$1"
}

run_deploy() {
  cd "$REPO_DIR"
  # Verrou non bloquant : deux tics de cron qui se chevauchent ne doivent pas
  # déployer en parallèle.
  exec 9>"$LOCK_FILE"
  flock -n 9 || exit 0

  git fetch origin main --quiet

  local_rev="$(git rev-parse main)"
  remote_rev="$(git rev-parse origin/main)"
  running="$("${COMPOSE[@]}" ps --status running -q 2>/dev/null || true)"
  deployed_rev="$(cat "$DEPLOYED_REV_FILE" 2>/dev/null || true)"

  if [ "$deployed_rev" = "$remote_rev" ] && [ -n "$running" ]; then
    exit 0
  fi

  # ATTENDRE que l'image du commit visé soit publiée.
  #
  # Sans ce contrôle, le déploiement tirait `latest`, qui n'existe qu'une fois
  # le build ET le scan Trivy terminés. Un tic de cron tombant avant tirait
  # l'image PRÉCÉDENTE, puis inscrivait le nouveau commit comme déployé : la
  # version d'avant tournait en se faisant passer pour la nouvelle, et plus
  # aucun tic ne rattrapait l'écart. Le même défaut a mordu derniere-chance
  # deux fois le 2026-08-30.
  #
  # On sort en 0 : la CI n'a pas fini, ce n'est pas une panne.
  local target_tag="$remote_rev"
  if ! images_publiees "$target_tag"; then
    if [ "$deployed_rev" != "$remote_rev" ]; then
      log "image $target_tag pas encore publiée, attente du prochain tic"
    fi
    exit 0
  fi

  # Fast-forward uniquement : un historique réécrit côté distant doit faire
  # échouer le déploiement, pas être avalé silencieusement.
  if ! git checkout main --quiet || ! git merge --ff-only origin/main --quiet; then
    log "échec du fast-forward vers origin/main, déploiement annulé"
    exit 1
  fi

  pin_images "$target_tag"

  if ! "${COMPOSE[@]}" pull >> "$LOG_FILE" 2>&1; then
    log "échec du pull de $target_tag, nouvelle tentative au prochain tic"
    exit 1
  fi

  if "${COMPOSE[@]}" up -d --remove-orphans >> "$LOG_FILE" 2>&1; then
    log "déploiement réussi ($remote_rev, image épinglée)"
    echo "$remote_rev" > "$DEPLOYED_REV_FILE"
    docker image prune -f >> "$LOG_FILE" 2>&1
    exit 0
  fi

  log "ÉCHEC du démarrage ($remote_rev)"
  rollback "$deployed_rev"
  exit 1
}

# Remet la version précédente en service. Sans cela, un démarrage à moitié
# appliqué laisse la prod dans un état incertain jusqu'à intervention
# manuelle.
rollback() {
  local previous_rev="$1"

  if [ -z "$previous_rev" ]; then
    log "ROLLBACK impossible : aucune version précédente connue, intervention manuelle requise"
    return
  fi

  if ! images_publiees "$previous_rev"; then
    log "ROLLBACK impossible : image $previous_rev absente du registre, intervention manuelle requise"
    return
  fi

  log "ROLLBACK vers $previous_rev"
  pin_images "$previous_rev"

  # `.deployed_rev` est remis à la version réellement en service pour que le
  # prochain tic retente le déploiement au lieu de croire le travail fait.
  if "${COMPOSE[@]}" up -d --remove-orphans >> "$LOG_FILE" 2>&1; then
    echo "$previous_rev" > "$DEPLOYED_REV_FILE"
    log "ROLLBACK réussi, la prod tourne sur $previous_rev"
  else
    log "ROLLBACK ÉCHOUÉ, la prod est dans un état incertain, intervention manuelle requise"
  fi
}

install_cron() {
  cd "$REPO_DIR"
  command -v docker >/dev/null || { echo "docker requis" >&2; exit 1; }
  docker network inspect ecosolva-web >/dev/null 2>&1 \
    || { echo "réseau ecosolva-web absent : le créer d'abord" >&2; exit 1; }
  # C'est le fichier de production qui est copié, pas l'exemple : ses chemins
  # d'écriture visent le volume, seul endroit inscriptible du conteneur.
  [ -f "$REPO_DIR/sluis.toml" ] \
    || cp "$REPO_DIR/config/sluis.prod.toml" "$REPO_DIR/sluis.toml"
  [ -f "$REPO_DIR/.env" ] \
    || { echo ".env absent : cp .env.example .env, puis le remplir" >&2; exit 1; }
  # Le fichier de l'exécutant n'est JAMAIS copié automatiquement : le gabarit
  # porte un identifiant de projet OVH à remplacer, et un Sluis qui démarrerait
  # dessus armerait `sluis_campagne` contre un projet qui n'existe pas. Un outil
  # qui a l'air armé et ne l'est pas est pire qu'un outil absent.
  if [ -f "$CONFIG_EXECUTANT" ] \
     && grep -q "a-remplacer-par-l-identifiant-du-projet-ovh" "$CONFIG_EXECUTANT"; then
    echo "sluis.executant.toml porte encore l'identifiant de projet du gabarit :" >&2
    echo "y mettre le projet OVH réel du bac à sable (H1), ou supprimer le fichier" >&2
    exit 1
  fi
  # Vérifie ici, une fois, ce que le cron ne pourrait que constater toutes les
  # cinq minutes : une variable requise absente fait échouer cette commande
  # avec le nom de la variable manquante.
  "${COMPOSE[@]}" config -q \
    || { echo "configuration incomplète : voir le message ci-dessus" >&2; exit 1; }
  ( crontab -l 2>/dev/null | grep -v "$CRON_MARKER" || true
    echo "$CRON_SCHEDULE cd $REPO_DIR && ./deploy.sh --run # $CRON_MARKER"
  ) | crontab -
  echo "cron programmé : $CRON_SCHEDULE"
}

case "${1:-}" in
  --run) run_deploy ;;
  *)     install_cron ;;
esac
