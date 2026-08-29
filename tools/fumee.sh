#!/usr/bin/env bash
# Test de fumée d'un Sluis déployé.
#
#   ./tools/fumee.sh https://sluis.ecosolva.org
#
# Trois vérifications, dans cet ordre : le service répond, il s'annonce sous
# la bonne identité, et il refuse un appel non authentifié. La troisième est
# la seule qui compte vraiment : un /mcp ouvert est une porte ouverte.
set -euo pipefail

BASE="${1:-https://sluis.ecosolva.org}"
BASE="${BASE%/}"
echec=0

verifier() {
  if [ "$2" = "$3" ]; then
    printf '  ✓ %s\n' "$1"
  else
    printf '  ✗ %s : attendu « %s », obtenu « %s »\n' "$1" "$3" "$2"
    echec=1
  fi
}

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
