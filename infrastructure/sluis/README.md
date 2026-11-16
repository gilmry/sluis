# Infrastructure de Sluis : plateforme du palier 2

Ce dossier loue et configure toute la plateforme du palier 2 d'ADR-044 : la
machine Sluis, le nœud K3s et ce qu'il porte, leurs sauvegardes, et de quoi
tout reconstruire si le centre de données disparaît.

| Où | Quoi |
|---|---|
| Gravelines (production) | Deux instances Public Cloud dans un vRack : `sluis` (d2-2) et `k3s` (d2-8) |
| Paris, trois zones (sauvegardes) | Compartiments S3 versionnés et verrouillés 30 jours |
| Roubaix (clé) | KMS qui déverrouille OpenBao : ni avec les machines, ni avec les sauvegardes |

Le partage des rôles est celui d'ADR-032 :

| Couche | Outil | Ici |
|---|---|---|
| Location | OpenTofu (ADR-010) | `tofu/durable/` et `tofu/site/` |
| Socle | Ansible | `ansible/` : SSH, pare-feu, volume chiffré, Docker ou K3s, Flux |
| Cluster | Flux (ADR-029) | `cluster/` : Keycloak ×2, CloudNativePG, OpenBao |
| Application Sluis | `deploy.sh` (boucle GitOps existante) | posé par le rôle `sluis` |
| Orchestration | `deployer.sh` | enchaîne le tout, depuis le poste du superviseur |

Tout se lance **depuis le poste du superviseur, par lui seul** (ADR-037,
invariant 1). Les jetons OVHcloud, la phrase de l'état, les secrets TOTP et
les phrases des volumes chiffrés sont des secrets d'amorce : aucun agent IA
ne les voit.

## Deux états OpenTofu

- **`tofu/durable/`** : ce qui survit à la perte d'un centre de données.
  Projet Public Cloud, vRack, un réseau (VLAN) par zone et par région,
  compartiments de Paris et leurs identifiants, KMS.
- **`tofu/site/`** : les machines d'un centre de données et leurs noms DNS.
  Un état par site (`gra1`). Reconstruire, c'est un site neuf (`sbg1`).

Gabarits d2 seulement, au plus petit qui tient : `sluis` en d2-2 (deux Sluis
et un Traefik), `k3s` en d2-8 (deux Keycloak, OpenBao, deux bases, Flux,
cert-manager). Un gabarit se change en place.

Le vRack sépare les zones (VLAN 101 pour la zone 1, 102 pour la zone 2). Il
isole, il ne chiffre pas : rien n'est cru sur sa provenance réseau (ADR-042).
Le VLAN de la zone 2 reste vide jusqu'à l'éclatement (ADR-043, point 4) :
d'ici là, la séparation des zones sur le nœud K3s tient aux espaces de noms,
aux NetworkPolicy et aux identifiants, pas au réseau.

## Ce qui reste hors du dépôt

| Quoi | Où | Gabarit |
|---|---|---|
| Jetons, chemins | `~/.config/sluis-infra/env` (0600) | `.env.exemple` |
| Variables OpenTofu | `~/.config/sluis-infra/durable.tfvars`, `site-gra1.tfvars` | `tofu/*/*.tfvars.example` |
| États et plans, chiffrés | `~/.local/state/sluis-infra/` | aucun |
| Inventaire Ansible | `~/.config/sluis-infra/inventaire.yml` | `ansible/inventaire.exemple.yml` |
| Secrets des machines, chiffrés par SOPS | dépôt privé | `ansible/secrets.exemple.yml` |
| Signataires admis par Flux | `~/.config/sluis-infra/signataires-flux.pub` | aucun |

Les secrets du cluster (identifiants S3, client du KMS) ne passent pas par
Git, même chiffrés : le dépôt est public. Ansible les écrit directement, et
K3s les chiffre au repos.

## Ordre de l'amorce

```
./deployer.sh regles infra            # règles du jeton, à créer chez OVHcloud
./deployer.sh plan durable            # rien n'est loué
./deployer.sh appliquer durable       # COMMANDE : projet, vRack, compartiments, KMS
./deployer.sh identifiant-superviseur # à ranger hors ligne
./deployer.sh plan site
./deployer.sh appliquer site          # les deux instances, facturées à l'usage

# pour chaque machine (sluis, k3s) :
./deployer.sh empreinte <machine> SHA256:...   # lue dans la console, jamais au premier contact
./deployer.sh totp <machine>          # le secret naît sur la machine, dans ton terminal
./deployer.sh chiffrer <machine>      # phrase LUKS tapée par toi, rangée hors ligne

./deployer.sh configurer              # socle, Docker et Sluis, K3s et Flux
./deployer.sh kms                     # demande de certificat d'OpenBao vers le poste
./deployer.sh plan durable && ./deployer.sh appliquer durable   # le KMS la signe
./deployer.sh configurer              # pose le certificat, OpenBao démarre
./deployer.sh verifier                # --check --diff : changed=0 exigé, journal daté
./deployer.sh sauvegarder-etat        # états et inventaires à Paris
```

Restent à la main du superviseur, hors de ce script (ADR-037) :
l'initialisation d'OpenBao et la garde de ses parts de récupération, la
création du rôle `instantanes`, les royaumes Keycloak.

Après un redémarrage : `./deployer.sh deverrouiller <machine>`. Docker ou K3s
attend le volume chiffré et reste arrêté jusque-là.

## Sauvegardes

| Donnée | Vers | Comment | Droit de la machine |
|---|---|---|---|
| Bases des Keycloak | `wal-zone-1`, `wal-zone-2` | WAL en continu et base chaque nuit (CloudNativePG, barman-cloud) | lire, écrire |
| OpenBao | `openbao` | instantané Raft chaque nuit | écrire seulement |
| Données de Sluis (audit, OAuth) | `sluis` | archive chiffrée par age chaque nuit | écrire seulement |
| États, inventaires, fichier SOPS | `etat` | `deployer.sh sauvegarder-etat` | aucun (poste seul) |
| Console du superviseur (WSL, disque externe) | `console` | `poste/sauvegarder-console.ps1`, chiffrée par GPG | écrire seulement |

Aucune machine ne peut lever un verrou ni effacer une version verrouillée.
Les clés qui déchiffrent restent hors des machines : clé age et phrase de
l'état chez le superviseur, parts de récupération d'OpenBao hors ligne.

## Console du superviseur

La console est une distribution WSL dédiée, sauvegardée sur un disque
externe. `poste/sauvegarder-console.ps1` (PowerShell, côté Windows) exporte
la distribution, la chiffre par GPG au fil de l'eau, l'écrit sur le disque
externe, puis dépose à Paris cette archive et celle du reste du disque.

```
.\sauvegarder-console.ps1 -Initialiser    # configuration, identifiant S3 (deployer.sh identifiant-console)
.\sauvegarder-console.ps1                 # sauvegarde complète
.\sauvegarder-console.ps1 -Verifier <archive.tar.gpg>   # preuve de relecture, clé matérielle branchée
```

Deux invariants :

1. **Le poste ne détient que des clés publiques.** La clé privée qui
   déchiffre vit sur clé matérielle, avec un second destinataire de secours
   hors ligne. Elle n'est ni sur le poste ni dans la distribution : une
   sauvegarde qu'on ne relit qu'avec ce qu'elle contient ne sauve rien.
2. **Les ancres n'y sont pas** (ADR-037, invariant 3 ; ADR-040) : clé GPG
   maîtresse, racine de step-ca et parts de récupération d'OpenBao naissent
   et restent hors ligne. Ce script sauvegarde un poste d'exploitation, il
   ne remplace pas la zone 3.

## Reconstruire après la perte du centre de données

Ce qui survit : les compartiments de Paris, le KMS de Roubaix, le vRack et
ses réseaux, l'état durable, ce dépôt. Ce qui est perdu : les deux machines
et leurs disques. L'ordre, dont chaque étape suppose la précédente :

1. **Retrouver les états** si le poste a disparu aussi : avec l'identifiant
   du superviseur et la phrase de l'état, gardés hors ligne, relire le
   compartiment `etat` (`deployer.sh rapatrier etat <date> <dossier>`).
2. **Louer un site neuf** dans une autre région (Strasbourg ou Roubaix,
   jamais Paris) : `SLUIS_SITE=sbg1`, variables `site-sbg1.tfvars` avec
   `region_cloud`, puis `plan site`, `appliquer site`. Les noms DNS
   basculent vers les nouvelles adresses.
3. **Rejouer l'amorce des machines** : empreinte, TOTP, volume chiffré,
   `configurer`. Les secrets TOTP et les phrases LUKS sont neufs.
4. **Rendre un certificat à OpenBao** : `deployer.sh kms`, puis
   `appliquer durable`. La clé du client est neuve, celle du KMS n'a pas
   bougé.
5. **Restaurer OpenBao** depuis son dernier instantané
   (`rapatrier openbao ...`, puis `bao operator raft snapshot restore`).
   Le KMS le déverrouille.
6. **Restaurer les bases** : un commit signé remplace `bootstrap.initdb` par
   `bootstrap.recovery` dans `cluster/zone-*/base.yaml`, avec la source
   barman-cloud du compartiment, sous un nom de serveur neuf pour les
   nouvelles archives. Les Keycloak repartent sur les bases restaurées.
7. **Restaurer les données de Sluis** : rapatrier la dernière archive,
   la déchiffrer sur le poste avec la clé age, la reposer dans le volume.
8. **Vérifier**, puis retirer l'ancien site de l'état (ses machines
   n'existent plus).

Perte maximale de données : celle du dernier WAL non archivé pour les
bases, jusqu'à 24 heures pour OpenBao et pour les données de Sluis.

## SSH : TOTP d'abord, clé matérielle ensuite

`socle_ssh_methode` (ADR-045, point 4) :

- **`totp`, par défaut** : clé publique ed25519, puis code TOTP par PAM.
- **`cle_materielle`, deuxième méthode** : clé `sk-ssh-ed25519` sur YubiKey
  seule (ADR-042). Le rôle refuse de basculer tant qu'une clé non matérielle
  reste dans `authorized_keys`.

SSH n'est ouvert qu'aux sources admises, deux fois : groupe de sécurité
d'OVHcloud, puis nftables. L'API de K3s n'est jamais ouverte sur Internet
(`deployer.sh tunnel`).

## Chiffrement des disques

Un volume **LUKS2** (aes-xts, argon2id) par machine, dans un fichier, porte
tout ce que la machine sait : données de Docker et de Sluis d'un côté, base
et secrets du cluster, volumes des pods de l'autre. Aucune entrée dans
`crypttab` : la phrase n'est jamais sur la machine.

## Décisions à contresigner

- **Instances Public Cloud et vRack, et non VPS** : ADR-045 (premier
  serveur en VPS) et ADR-043 (« petit VPS Sluis ») sont à amender.
- **Un seul projet Public Cloud** pour les machines et les sauvegardes. Le
  bac à sable garde le sien.
- **Mode du verrou** : `governance` par défaut. `compliance` rend le verrou
  irrévocable, même par le titulaire du compte.
- **Référence suivie par Flux** : la branche `main`, commits signés par une
  clé SSH admise (point ouvert d'ADR-029 : branche ou étiquette signée).
- **Keycloak de plateforme sans entrée publique**, atteint par tunnel.
- **État d'OpenTofu** local et chiffré, copié à Paris (point ouvert
  d'ADR-037).
- **Déverrouillage manuel** des volumes (point ouvert d'ADR-032) : après un
  redémarrage, la machine est indisponible jusqu'au geste du superviseur.
- **Louer le palier 2 maintenant** : ADR-044 le place au premier revenu.

## Ce qui manque, dit tel quel

- **Rien n'a tourné sur une vraie machine.** Validé : `tofu validate` et
  `fmt` sur les deux racines, `ansible-lint` (profil production) et
  `--syntax-check`, `shellcheck`, `kustomize build` et `kubeconform` (40
  objets), rendu des gabarits, `nft -c`. Pas validé : une commande réelle,
  un passage Ansible, Flux sur un cluster, une restauration.
- **La restauration n'est pas scriptée** au-delà de `rapatrier` : les étapes
  5 à 7 sont décrites, pas outillées, et aucune n'a été répétée.
- **Routes du jeton d'infrastructure** : déduites de la documentation du
  fournisseur (v2.25), pas mesurées. Le premier `appliquer` dira ce qui
  manque.
- **WAL et sauvegardes de base** : chiffrés côté serveur par OVHcloud
  seulement, pas côté client (ADR-045, point 5.1).
- **Certificat du client du KMS** : valable 365 jours, sans rotation
  automatique ni alerte d'échéance.
- **Alertes** : la métrique de la sauvegarde de Sluis est émise, les règles
  d'alerte vivent dans le dépôt privé de supervision et restent à écrire.
  Les sauvegardes du cluster n'émettent encore rien.
- **Profil de sécurité des pods** : `baseline` imposé, `restricted`
  seulement signalé.
- **OpenBao en clair dans le cluster**, derrière les NetworkPolicy : TLS
  interne à poser à l'éclatement.
- **ADR-043, point 3, incomplet** : les deux Sluis lisent le même `.env` et
  tournent sous le même utilisateur. À faire avant `sluis_executant: true`.
- **Traefik de la machine Sluis lit le socket Docker** : à remplacer par un
  mandataire filtré.
- **Rendre une machine ou un compartiment** : impossible par ces outils, et
  voulu.
