#!/usr/bin/env bash
# Test de fumée d'un Sluis déployé.
#
#   ./tools/fumee.sh https://sluis.ecosolva.org   le Sluis public
#   ./tools/fumee.sh --executant                  le Sluis exécutant
#
# Sur le public : quatre vérifications, dont la seule qui compte vraiment, que
# `/mcp` refuse un appel sans jeton. Un /mcp ouvert est une porte ouverte.
#
# Sur l'exécutant : on ne peut pas l'interroger en HTTP, il n'est publié nulle
# part, et c'est voulu. On le fait donc parler par sa CLI, qui rend le même
# diagnostic que `sluis_doctor`. C'est aussi le seul contrôle qui prouve ce
# pour quoi cette image existe : que tofu et k6 sont présents.
set -euo pipefail

echec=0

verifier() {
  if [ "$2" = "$3" ]; then
    printf '  ✓ %s\n' "$1"
  else
    printf '  ✗ %s : attendu « %s », obtenu « %s »\n' "$1" "$3" "$2"
    echec=1
  fi
}

# ── Sluis exécutant ────────────────────────────────────────────────────────
#
# `docker compose run --rm` plutôt qu'un `exec` dans le conteneur en service :
# l'image est distroless, elle n'a pas de shell, et `exec` n'aurait rien à
# lancer. Le `run` remplace l'entrypoint par la CLI, dans le même environnement
# et avec la même configuration, ce qui est exactement ce qu'on veut vérifier.
if [ "${1:-}" = "--executant" ]; then
  printf 'Fumée sur le Sluis exécutant\n'

  # Sans cette garde, `docker compose` CRÉERAIT un répertoire du nom du fichier
  # de configuration manquant, parce que c'est ce qu'il fait d'une source de
  # montage absente. Le déploiement s'en trouverait dans un état que ni
  # `[ -f ]` ni l'œil ne lisent correctement.
  if [ ! -f sluis.executant.toml ]; then
    printf '  ✗ sluis.executant.toml absent : aucun exécutant déployé ici\n'
    printf 'Voir docs/DEMARRAGE.md, section « Le second déploiement ».\n'
    exit 1
  fi

  rapport="$(docker compose -f docker-compose.yml -f docker-compose.executant.yml \
               run --rm --no-deps \
               --entrypoint /usr/local/bin/sluis sluis-executant doctor 2>&1 || true)"

  # Le diagnostic marque chaque moteur présent ou absent. Sur cette image, un
  # moteur absent n'est pas l'état normal d'une machine nue : c'est un défaut
  # de build, puisque l'image existe précisément pour les porter.
  for moteur in tofu k6; do
    verifier "$moteur présent" \
      "$(printf '%s' "$rapport" | grep -ci "$moteur.*absent" || true)" \
      "0"
  done

  # Sans identifiants, rien de ce qui touche OVH ne répond, et la campagne ne
  # peut rien louer. C'est l'état tant que H1 n'est pas levée : le contrôle le
  # dit, il ne le masque pas. Le diagnostic préfixe d'un ✗ chaque variable
  # absente ; on compte les ✗, pas les lignes.
  verifier "identifiants OVH présents" \
    "$(printf '%s' "$rapport" | grep -c '✗ OVH_' || true)" \
    "0"

  if [ "$echec" -eq 0 ]; then
    printf 'Fumée verte : les moteurs et les identifiants sont là.\n'
  else
    printf 'Fumée rouge. Rapport complet :\n%s\n' "$rapport"
    exit 1
  fi
  exit 0
fi

# ── Sluis public ───────────────────────────────────────────────────────────
BASE="${1:-https://sluis.ecosolva.org}"
BASE="${BASE%/}"

printf 'Fumée sur %s\n' "$BASE"

verifier "santé" \
  "$(curl -fsS --max-time 10 "$BASE/sante" 2>/dev/null || echo "pas de réponse")" \
  '{"statut":"ok"}'

decouverte="$(curl -fsS --max-time 10 "$BASE/.well-known/oauth-authorization-server" 2>/dev/null || echo '{}')"
# L'issuer doit être l'URL publique : un issuer qui ne correspond pas au
# domaine fait échouer la découverte côté client MCP, et le symptôme est
# illisible.
verifier "issuer OAuth" \
  "$(printf '%s' "$decouverte" | sed -n 's/.*"issuer":"\([^"]*\)".*/\1/p')" \
  "$BASE"

verifier "défi PKCE exigé en S256" \
  "$(printf '%s' "$decouverte" | grep -c 'S256' || true)" \
  "1"

verifier "/mcp refuse sans jeton" \
  "$(curl -s -o /dev/null -w '%{http_code}' --max-time 10 -X POST "$BASE/mcp" \
     -H 'content-type: application/json' \
     -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' 2>/dev/null || echo "000")" \
  "401"

if [ "$echec" -eq 0 ]; then
  printf 'Fumée verte.\n'
else
  printf 'Fumée rouge : ne pas annoncer le déploiement.\n'
  exit 1
fi
