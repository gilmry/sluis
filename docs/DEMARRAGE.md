# Démarrage rapide

Déployer Sluis sur un serveur et vérifier qu'il tient. Vingt minutes, à
condition d'avoir un Docker, un Traefik et un enregistrement DNS.

Ce guide ne couvre que le **mode serveur**, transport Streamable HTTP derrière
Traefik. Le mode stdio existe, il se lance depuis un binaire local et n'a pas
sa place ici.

---

## Ce que vous déployez, exactement

Un serveur MCP distant qui expose **trois outils, tous en lecture** :

| Outil | Ce qu'il fait |
|---|---|
| `sluis_doctor` | état des six moteurs d'infrastructure et présence des identifiants OVH, jamais leur valeur |
| `sluis_inventory` | matrice topologies x environnements d'un dépôt, découverte sans saisie |
| `sluis_cluster_profiles` | profils de cluster déclarés dans le dépôt |

Le bac à sable, la campagne de charge et la passerelle Tier 1 sont écrits et
testés dans le domaine, mais **ne sont câblés à aucun registre MCP**. Un agent
connecté à ce déploiement ne peut donc rien muter. C'est l'état voulu tant que
les hypothèses H1 et H2 ne sont pas levées, et c'est ce qui rend ce
déploiement sans risque pour l'infrastructure existante.

---

## Prérequis

- Docker avec le plugin `compose`
- le réseau Traefik partagé : `docker network create ecosolva-web` s'il n'existe pas
- un enregistrement DNS pointant vers le serveur, par défaut `sluis.ecosolva.org`
- rien d'autre. Pas de base de données, pas de terraform, pas de kubectl

---

## Déploiement

```sh
git clone git@github.com:gilmry/sluis.git
cd sluis
cp .env.example .env
```

Remplissez les trois valeurs obligatoires du `.env`. Sans elles, le serveur
refuse de démarrer, et c'est voulu : une valeur par défaut devinable pour un
secret de signature est un secret public.

```sh
# Signature des jetons
openssl rand -base64 32

# EMPREINTE du mot de passe du superviseur, jamais le mot de passe
printf '%s' 'votre-mot-de-passe' | sha256sum | cut -d' ' -f1
```

```
SLUIS_DOMAIN=sluis.ecosolva.org
SLUIS_SECRET_SIGNATURE=<la sortie d'openssl>
SLUIS_IDENTIFIANT=<votre identifiant>
SLUIS_EMPREINTE_MOT_DE_PASSE=<la sortie de sha256sum>
```

Puis :

```sh
./deploy.sh          # vérifie la configuration, copie sluis.toml, programme le cron
./deploy.sh --run    # déploie tout de suite, sans attendre le tic suivant
```

`./deploy.sh` seul ne déploie pas : il installe une boucle GitOps qui, toutes
les cinq minutes, compare `main` à `origin/main` et ne fait rien tant que rien
n'a bougé. Le serveur ne construit jamais d'image, il tire celle que la CI a
poussée sur GHCR.

## Vérification

```sh
./tools/fumee.sh https://sluis.ecosolva.org
```

Quatre contrôles. Le dernier est le seul qui compte vraiment : un `/mcp` qui
répond sans jeton est une porte ouverte, et le déploiement doit être défait,
pas corrigé plus tard.

```
Fumée sur https://sluis.ecosolva.org
  ✓ santé
  ✓ issuer OAuth
  ✓ défi PKCE exigé en S256
  ✓ /mcp refuse sans jeton
Fumée verte.
```

## Connecter un client MCP

Le serveur est un serveur d'autorisation OAuth 2.1 complet. Un client qui
sait faire l'enregistrement dynamique n'a besoin que de l'URL :

```
https://sluis.ecosolva.org/mcp
```

Il découvre le reste sur `/.well-known/oauth-authorization-server`, s'enregistre
sur `/oauth/register`, et vous présente un formulaire de connexion. Vous y
entrez l'identifiant et le mot de passe du `.env`, en clair cette fois : c'est
son empreinte qui est stockée, jamais lui.

Contraintes non négociables, refusées à l'admission :

- `code_challenge_method` doit valoir `S256`. OAuth 2.1 interdit `plain`
- `redirect_uri` doit être enregistrée pour ce client, sinon rien n'est
  redirigé, pas même une erreur : ce serait un redirecteur ouvert
- trois portées existent, `sluis:read`, `sluis:sandbox`, `sluis:propose`. Seule
  la première mène à quelque chose aujourd'hui

## Exploitation

```sh
docker compose --profile prod logs -f sluis     # journal du service
docker compose --profile prod ps                # état
tail -f deploy.log                              # boucle GitOps
```

Le **journal d'audit** est une ligne de JSON par appel d'outil, dans le volume :

```sh
docker run --rm -v sluis_sluis_donnees:/d alpine tail -5 /d/sluis-audit.jsonl
```

```json
{"horodatage":"1788035667s","outil":"sluis_doctor","tier":"2","empreinte":"f92fca62392e","issue":"succes"}
```

Sluis refuse de démarrer s'il ne peut pas y écrire. Un appel non traçable ne
doit pas avoir lieu.

**Mettre à jour** : poussez sur `main`. La CI construit l'image, la scanne, et
la pousse seulement si le scan est vert. Le cron du serveur la tire dans les
cinq minutes. Rien à faire à la main.

**Changer le mot de passe du superviseur** : recalculez l'empreinte, modifiez
le `.env`, `docker compose --profile prod up -d`. Les jetons déjà émis restent
valides jusqu'à leur expiration, une heure pour un jeton d'accès.

## Quand ça ne marche pas

| Symptôme | Cause | Correctif |
|---|---|---|
| `required variable SLUIS_... is missing` | `.env` incomplet | remplir la variable nommée dans le message |
| conteneur en boucle, `EntreeSortie` sur le journal | `sluis.toml` copié depuis `config/sluis.example.toml` au lieu de `config/sluis.prod.toml` | le journal doit viser `/app/donnees/`, seul endroit inscriptible |
| `unauthorized` au `docker compose pull` | package GHCR repassé en privé | le remettre public, ou `docker login ghcr.io` avec un PAT `read:packages` |
| le client MCP échoue à la découverte | `SLUIS_DOMAIN` ne correspond pas au domaine servi | l'issuer OAuth doit être l'URL publique exacte |
| `exec /usr/local/bin/sluis-server: operation not permitted` | `no-new-privileges` sur un Docker installé par snap | ne pas le remettre dans le compose, AppArmor refuse la transition de profil |
| déploiement figé, `échec du fast-forward` dans `deploy.log` | historique réécrit sur `origin/main` | intervention humaine, jamais un `--force` automatique |

## Ce que ce déploiement ne fait pas encore

- **Pas de rollback automatique.** L'architecture le prévoit, `deploy.sh` ne
  l'implémente pas. Si `tools/fumee.sh` est rouge après une mise à jour, c'est
  à vous de revenir à l'image précédente
- **Pas de mutation, d'aucune sorte.** Ni bac à sable, ni Tier 1. Le jeton
  `SLUIS_GITHUB_TOKEN` du `.env` n'est lu par aucun outil
- **Un seul utilisateur.** Sluis sert un superviseur, il n'a pas de table
  d'utilisateurs et n'en aura pas tant qu'il en sert un
