# Manifestes du cluster K3s (palier 2)

Lus par Flux depuis ce dossier seulement (ADR-029). Ansible ne pose que la
source et la racine (`ansible/roles/flux`) ; tout le reste se relit ici.

| Dossier | Contenu | Zone |
|---|---|---|
| `flux/` | Sources et ordre d'application (Kustomizations Flux) | |
| `commun/controleurs/` | cert-manager, CloudNativePG, greffon barman-cloud | |
| `commun/configuration/` | Émetteur ACME, Traefik | |
| `zone-1/` | Keycloak des SaaS, sa base, ses sauvegardes, son entrée publique | 1 |
| `zone-2/` | Keycloak de plateforme, sa base, OpenBao, leurs sauvegardes | 2 |

La zone 2 vit sur le même nœud pendant la dérogation d'ADR-043 : elle en
sortira à l'éclatement, sans changer de dossier.

Ce que le dépôt public ne porte pas (domaines, noms des compartiments,
coordonnées du KMS) arrive par le ConfigMap `flux-system/parametres-plateforme`,
substitué par Flux (`${...}`). Les secrets n'y passent jamais : Ansible les
écrit directement dans le cluster.
