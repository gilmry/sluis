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
| `sluis_doctor` | état des sept moteurs d'infrastructure et présence des identifiants OVH, jamais leur valeur |
| `sluis_inventory` | matrice topologies x environnements d'un dépôt, découverte sans saisie |
| `sluis_cluster_profiles` | profils de cluster déclarés dans le dépôt |

La campagne de charge et la passerelle Tier 1 sont écrites et testées, mais
**ce déploiement-ci ne les expose pas**. Un agent connecté à `sluis.ecosolva.org`
ne peut donc rien muter, et c'est une propriété de l'image autant que de la
configuration : l'image `server` ne contient ni `tofu` ni `wrk`, elle ne
pourrait pas provisionner même si on lui donnait des clés OVH.

C'est le second déploiement, décrit plus bas, qui porte les moteurs.

---

## Prérequis

- Docker avec le plugin `compose`
- le réseau Traefik partagé : `docker network create ecosolva-web` s'il n'existe pas
- un enregistrement DNS pointant vers le serveur, par défaut `sluis.ecosolva.org`
- rien d'autre. Pas de base de données, pas de tofu, pas de kubectl

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

**Qui exécute le cron.** Sur une machine où Docker exige `sudo`, lancez
`sudo ./deploy.sh` : le cron s'installe alors pour root, seul compte capable de
parler au démon. Deux conséquences à traiter d'un coup, sans quoi la boucle
échoue toutes les cinq minutes en silence dans `deploy.log` :

```sh
# root n'a pas la clé SSH du dépôt. Le dépôt étant public, le fetch passe en
# HTTPS ; le push reste en SSH pour l'humain qui travaille sur la machine.
git remote set-url origin https://github.com/gilmry/sluis.git
git remote set-url --push origin git@github.com:gilmry/sluis.git

# root refuse d'opérer sur un dépôt qui ne lui appartient pas
sudo git config --global --add safe.directory "$PWD"
```

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

Pour l'exécutant, il faut ajouter sa surcouche à chaque commande, sans quoi
Compose ne connaît pas le service :

```sh
alias dcx='docker compose -f docker-compose.yml -f docker-compose.executant.yml'
dcx logs -f sluis-executant
dcx run --rm --entrypoint /usr/local/bin/sluis sluis-executant doctor
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

## Le second déploiement : le Sluis exécutant

Le Sluis public ne détient rien, et c'est pour cela qu'il peut être exposé.
Celui qui détient les clés n'est joignable de nulle part : pas de label
Traefik, pas de port publié, pas de place sur `ecosolva-web`. Les deux font
tourner le **même commit du même binaire**, épinglé sur le même tag ; ce qui
les distingue est l'image (`server` contre `executant`) et la configuration.

Fusionner les deux rendrait à un service public le pouvoir de muter. C'est le
choix d'architecture du Sprint 5, et il ne se négocie pas.

**Il ne se déploie que si `sluis.executant.toml` existe à la racine.** C'est
le seul interrupteur, et il est à la main : tant que H1 n'est pas levée, il
n'y a ni projet OVH de bac à sable, ni dépôt à mesurer.

```sh
# 1. Le projet OVH dédié aux bacs à sable (H1), et lui seul, dans le fichier
cp config/sluis.executant.toml sluis.executant.toml
$EDITOR sluis.executant.toml     # y mettre l'identifiant réel du projet

# 2. Les identifiants OVH restreints à ce projet, et une signature à part
$EDITOR .env                     # SLUIS_SECRET_SIGNATURE_EXECUTANT, OVH_BAC_A_SABLE_*

# 3. Les dépôts à mesurer, montés en lecture-écriture sous /depots
mkdir -p /home/ubuntu/depots
git clone git@github.com:gilmry/koprogo.git /home/ubuntu/depots/koprogo

# 4. Le déploiement les prend tous les deux, sans autre commande
sudo ./deploy.sh
sudo ./deploy.sh --run
```

`deploy.sh` refuse de continuer si le fichier porte encore l'identifiant de
projet du gabarit : un `sluis_campagne` armé contre un projet qui n'existe pas
serait pire qu'un outil absent.

Vérification, qui ne passe pas par HTTP puisque rien n'est publié :

```sh
./tools/fumee.sh --executant
```

```
Fumée sur le Sluis exécutant
  ✓ tofu présent
  ✓ wrk présent
  ✓ identifiants OVH présents
Fumée verte : les moteurs et les identifiants sont là.
```

**Le moteur d'infrastructure est OpenTofu**, pas Terraform. Terraform est sous
BSL 1.1 depuis la 1.6 ; ce dépôt est MIT et publie une image publique sur GHCR,
où redistribuer du BSL serait contradictoire. OpenTofu (MPL-2.0) lit les mêmes
fichiers `.tf` et rend les mêmes résumés. Le binaire s'appelle `tofu`, et c'est
sous ce nom que l'allowlist et `sluis_doctor` le connaissent.

### Les modules OpenTofu que l'exécutant accepte de lancer

Un fichier `.tf` est un programme : un `provisioner "local-exec"` ou un
`data "external"` exécute une commande sur la machine qui lance `tofu`, avec
ses secrets, et `data "external"` le fait dès le `plan`. **Avant chaque
`init`, `plan`, `apply`, `destroy` et `output`**, Sluis lit le module et le
refuse, sans rien lancer, s'il sort du contrat d'ADR-034 :

- `provisioner "local-exec"`, `provisioner "remote-exec"` et `data "external"`
  sont interdits, quelle que soit leur mise en forme. Une construction écrite
  **en commentaire** est refusée aussi : faux positif assumé ;
- les fichiers `.tf.json` et `.tofu.json` sont refusés ;
- le module porte un `.terraform.lock.hcl`, et chaque fournisseur qu'il
  verrouille figure dans `[tofu] fournisseurs_autorises`. `tofu init` est
  lancé avec `-lockfile=readonly` : le verrou vérifié est celui qui sert ;
- **un module appelé est copié dans le dépôt**, sous
  `_shared/terraform/modules/<nom>/`, et appelé par un chemin local
  (`source = "../../../_shared/terraform/modules/<nom>"`). Toute autre
  `source` est refusée : `git::`, `https://`, registre, chemin absolu, ou
  chemin local qui sort de la racine du dépôt ou vise ailleurs. Les modules
  partagés sont lus et vérifiés avec le module qui les appelle.

L'erreur nomme le fichier, la ligne et la construction refusée.

La liste blanche des fournisseurs se déclare dans `sluis.executant.toml`,
par leur adresse complète, telle qu'elle figure dans le verrou :

```toml
[tofu]
fournisseurs_autorises = [
  "registry.opentofu.org/terraform-provider-openstack/openstack",
]
```

**Elle est vide par défaut, et une liste vide refuse tout module** : son
contenu initial est un point ouvert d'ADR-034, à trancher par le superviseur,
pas une valeur que le code choisit. Tant qu'elle est vide, `sluis_campagne`
échoue avant tout appel à `tofu`.

### Ce que `tofu` reçoit de l'environnement

`tofu` ne voit **pas** l'environnement de Sluis (ADR-034, point 4). Il est
lancé avec un environnement **vidé**, puis rempli de deux choses seulement :

- un `PATH` fixe, `/usr/local/bin:/usr/bin:/bin`, qui ne vient pas de
  l'environnement de Sluis ;
- les variables nommées dans `[tofu] variables_transmises`, avec la valeur
  qu'elles ont dans l'environnement de Sluis.

```toml
[tofu]
variables_transmises = ["OVH_ENDPOINT", "OS_AUTH_URL"]
```

**La liste est vide par défaut** : `tofu` ne reçoit alors que `PATH`.
`SLUIS_GITHUB_TOKEN`, et toute variable non listée, n'atteignent jamais le
processus, donc jamais un fournisseur ni un module. Chaque identifiant dont
un fournisseur a besoin se nomme ici, un par un ; `HOME` aussi, si un
fournisseur en dépend. Une variable `PATH` listée est ignorée, le `PATH` fixe
l'emporte.

Une variable listée mais **absente** de l'environnement de Sluis n'est pas
transmise. Sluis la nomme sur sa sortie d'erreur à chaque lancement, et dans
l'erreur rendue si `tofu` échoue : une authentification refusée se lit alors
avec sa cause.

Les autres moteurs (`helm`, `kubectl`, `kustomize`, `argocd`, `wrk`,
`ansible-playbook`) gardent l'environnement hérité.

**Le chemin du module est canonisé** avant tout lancement : il doit, une fois
les liens symboliques résolus, descendre d'un `[bac_a_sable] depots_autorises`
lui-même canonisé (une racine déclarée par un lien est donc acceptée). Un
module atteint par un lien qui sort de la racine est refusé, et l'erreur
donne le chemin canonique refusé. Un chemin contenant `..` est refusé dès
l'admission. `tofu` est lancé dans le chemin canonique, pas dans le chemin
soumis.

## Ce que ce déploiement ne fait pas encore

- **Pas de mutation de production, d'aucune sorte.** Le Tier 1 reste derrière
  la passerelle d'ADR-008, et le jeton `SLUIS_GITHUB_TOKEN` du `.env` n'est lu
  par aucun outil tant que H2 n'est pas levée
- **Aucun projet ne déclare encore sa charge.** L'exécutant sait mesurer, mais
  `infrastructure/_shared/charge.yaml` n'existe dans aucun dépôt : voir
  `config/charge.exemple.yaml` pour ce qu'un projet doit déposer
- **Un seul utilisateur.** Sluis sert un superviseur, il n'a pas de table
  d'utilisateurs et n'en aura pas tant qu'il en sert un
