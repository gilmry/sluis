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
set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRON_SCHEDULE="${CRON_SCHEDULE:-*/5 * * * *}"
CRON_MARKER="sluis-auto-deploy"
LOG_FILE="$REPO_DIR/deploy.log"
LOCK_FILE="$REPO_DIR/.deploy.lock"
DEPLOYED_REV_FILE="$REPO_DIR/.deployed_rev"

log() { echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] $*" >> "$LOG_FILE"; }

run_deploy() {
  cd "$REPO_DIR"
  # Verrou non bloquant : deux tics de cron qui se chevauchent ne doivent pas
  # déployer en parallèle.
  exec 9>"$LOCK_FILE"
  flock -n 9 || exit 0

  git fetch origin main --quiet

  local_rev="$(git rev-parse main)"
  remote_rev="$(git rev-parse origin/main)"
  running="$(docker compose --profile prod ps --status running -q 2>/dev/null || true)"
  deployed_rev="$(cat "$DEPLOYED_REV_FILE" 2>/dev/null || true)"

  if [ "$deployed_rev" = "$remote_rev" ] && [ -n "$running" ]; then
    exit 0
  fi

  # Fast-forward uniquement : un historique réécrit côté distant doit faire
  # échouer le déploiement, pas être avalé silencieusement.
  if ! git checkout main --quiet || ! git merge --ff-only origin/main --quiet; then
    log "échec du fast-forward vers origin/main, déploiement annulé"
    exit 1
  fi

  if ! docker compose --profile prod pull >> "$LOG_FILE" 2>&1; then
    log "échec du pull GHCR ($remote_rev), nouvelle tentative au prochain tic"
    exit 1
  fi

  if docker compose --profile prod up -d --remove-orphans >> "$LOG_FILE" 2>&1; then
    log "déploiement réussi ($remote_rev)"
    echo "$remote_rev" > "$DEPLOYED_REV_FILE"
    docker image prune -f >> "$LOG_FILE" 2>&1
  else
    log "échec du démarrage ($remote_rev)"
    exit 1
  fi
}

install_cron() {
  command -v docker >/dev/null || { echo "docker requis" >&2; exit 1; }
  docker network inspect ecosolva-web >/dev/null 2>&1 \
    || { echo "réseau ecosolva-web absent : le créer d'abord" >&2; exit 1; }
  [ -f "$REPO_DIR/sluis.toml" ] \
    || cp "$REPO_DIR/config/sluis.example.toml" "$REPO_DIR/sluis.toml"
  ( crontab -l 2>/dev/null | grep -v "$CRON_MARKER" || true
    echo "$CRON_SCHEDULE cd $REPO_DIR && ./deploy.sh --run # $CRON_MARKER"
  ) | crontab -
  echo "cron programmé : $CRON_SCHEDULE"
}

case "${1:-}" in
  --run) run_deploy ;;
  *)     install_cron ;;
esac
