// Script k6 de Sluis : un palier de l'escalier de charge.
//
// Embarqué dans le binaire (`include_str!`) et écrit dans un fichier neuf à
// chaque palier : le script et l'adaptateur qui lit son résumé ne peuvent pas
// diverger. La cible arrive par `-e SLUIS_CIBLE`, le nombre d'utilisateurs et
// la durée par la ligne de commande.
//
// Le résumé tient en une ligne marquée `SLUIS_RESUME`, que l'adaptateur
// cherche dans la sortie : rien d'autre de k6 n'y est interprété.
import http from 'k6/http';

export const options = {
  // Comme wrk : on mesure le serveur, pas la mémoire du générateur.
  discardResponseBodies: true,
};

export default function () {
  http.get(__ENV.SLUIS_CIBLE);
}

export function handleSummary(data) {
  const requetes = data.metrics.http_reqs ? data.metrics.http_reqs.values : {};
  const durees = data.metrics.http_req_duration ? data.metrics.http_req_duration.values : {};
  const echecs = data.metrics.http_req_failed ? data.metrics.http_req_failed.values : {};
  const resume = {
    requetes: requetes.count || 0,
    debit: requetes.rate,
    p50: durees['p(50)'],
    p99: durees['p(99)'],
    // Part des requêtes en échec (statut hors 2xx-3xx ou erreur réseau).
    echecs: echecs.rate,
  };
  return { stdout: 'SLUIS_RESUME ' + JSON.stringify(resume) + '\n' };
}
