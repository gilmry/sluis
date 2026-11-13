# ADR-043 : Cohabitation des zones au démarrage, éclatement dès les premiers revenus

- **Statut** : Proposé le 2026-11-13, **non contresigné**.
- **Décisions du superviseur du 2026-11-13**, à l'origine de cet ADR : « pour des raisons de coût, tout mettre sur le même cluster, avec des mesures conservatoires pour l'éclatement en plusieurs zones » ; « supervision sur ce serveur, et Sluis planifiant et exécutant sur un même petit VPS au début » ; « dès que l'argent rentre, on fait les migrations et on éclate les zones et les serveurs ».
- **Dérogation bornée**, sur le modèle d'ADR-007 : elle prend fin à son déclencheur (point 4), et rien ne la reconduit en silence.
- **Amende, pour la durée de la dérogation** : ADR-014 (zones 1 et 2 sur un même cluster), ADR-017 (OpenBao dans ce cluster, hors de la machine de confiance), ADR-010 (Sluis public et exécutant sur un même VPS). ADR-042 (proposé, sluis#101) est concerné : la haute disponibilité de Keycloak n'y devient effective qu'à l'éclatement.
- **Irréversible** : non. C'est son objet : chaque mesure conservatoire existe pour que l'éclatement soit un déménagement. Un secret exposé pendant la cohabitation reste toutefois compromis pour toujours (ADR-016).
- **S'articule avec** : ADR-007 (modèle de dérogation datée), ADR-010, ADR-014, ADR-016 (SOPS), ADR-017, ADR-023 (supervision, témoin externe), ADR-026 (CloudNativePG), ADR-029 (K3s à un nœud, Flux), ADR-033 (feuille de route), ADR-036 (exercices de reprise), ADR-042 (identités sans PKI).
- **Sert la vision (ADR-000)** : principe n°4, parce qu'on ne paie pas trois machines avant d'avoir un revenu qui les justifie ; principe n°2, parce que la dérogation a un déclencheur écrit et que l'éclatement est répété avant d'être nécessaire. Elle va **contre** la règle « aucune brique ne détient tout » pendant sa durée : c'est le prix assumé, borné par le point 4.

## Contexte

ADR-014 sépare trois zones : la production (zone 1), la confiance et la
supervision (zone 2), le hors ligne (zone 3). ADR-010 sépare le Sluis public,
exposé à Internet, du Sluis exécutant, qui détient les clés OVH. Pris à la
lettre, c'est au moins trois machines louées en permanence, plus le témoin
externe, avant le premier client.

Aucun SaaS n'a aujourd'hui de revenu ni de donnée personnelle réelle chiffrée
par OpenBao en production. Le superviseur choisit de regrouper au démarrage,
et d'éclater dès que l'argent rentre.

## Décision

**1. Topologie de démarrage.**

| Machine | Ce qu'elle porte | Zone visée à terme |
|---|---|---|
| **Cluster K3s, un nœud** (ADR-029) | Les SaaS et leurs bases ; le Keycloak des SaaS (ADR-041) ; **le Keycloak de plateforme et OpenBao**, dans des espaces de noms à eux | zone 1 et zone 2, à séparer |
| **Petit VPS Sluis** | Le Sluis public **et** le Sluis exécutant, isolés l'un de l'autre (point 3) | Sluis public en zone 1, exécutant sur son VPS (ADR-010) |
| **Serveur actuel** | La supervision et le stockage de ses journaux (ADR-023) | zone 2 |
| **Témoin externe**, chez un autre hébergeur | Il sonde la supervision et le cluster | inchangé |
| **Zone 3** | Hors ligne | inchangé |

**2. Ce qui ne cohabite jamais, même pour économiser.**

- **La supervision n'est pas dans le cluster** qu'elle surveille : une
  supervision qui tombe avec ce qu'elle observe ne prévient personne.
- **Le Sluis exécutant n'est pas dans le cluster** où vit OpenBao : son image
  est exemptée du gate de scan (ADR-011), et ADR-010 l'a éloigné de la clé
  maîtresse.
- **La zone 3 reste hors ligne.**

**3. Sluis public et exécutant sur un même VPS : isolement minimal.** C'est
exactement ce qu'ADR-010 voulait éviter (« le service exposé à Internet serait
celui qui détient les clés OVH »). Pendant la dérogation :

- deux conteneurs, deux utilisateurs système, aucun volume partagé ; les
  secrets de l'exécutant (clés OVH, accès Ansible) ne sont lisibles que par
  lui ;
- l'exécutant **n'écoute sur aucun port** ; il ne fait que tirer le travail
  approuvé ;
- il ne lance rien sans la signature d'approbation vérifiée contre
  `allowed_signers` (ADR-013) : c'est cette vérification, pas l'isolement, qui
  reste la vraie barrière ;
- les clés OVH ont le plus petit périmètre que l'API permet, et leur usage hors
  d'une mise en ligne approuvée déclenche une alerte (ADR-023).

**4. Déclencheur de l'éclatement : le premier revenu.** Dès qu'un SaaS
encaisse un premier paiement, les migrations sont planifiées et exécutées dans
cet ordre :

1. Keycloak de plateforme et OpenBao sortent du cluster de production, vers la
   zone 2 (machine ou cluster de confiance, ADR-042 point D) ;
2. le Sluis exécutant passe sur son propre VPS (ADR-010) ;
3. le cluster de production passe à trois nœuds (ADR-033, étape 8), ce qui rend
   effective la haute disponibilité de Keycloak.

Chaque migration est une demande de mise en ligne comme une autre : approuvée,
barrière rejouée, preuve au journal.

**5. Mesures conservatoires, à tenir dès le premier jour.** Elles rendent le
point 4 peu coûteux :

1. **Des émetteurs de jetons stables.** Le Keycloak de plateforme est servi sous
   un nom DNS qui lui est propre et qui ne change pas au déménagement : si le
   `iss` de ses jetons change, toute la confiance déclarée (OpenBao, opkssh,
   clients) est à refaire.
2. **Un espace de noms par future zone**, avec des NetworkPolicy fermées par
   défaut qui reproduisent les flux autorisés entre zones. Les appels entre zones
   passent par des noms DNS qui deviendront les points d'entrée WireGuard.
3. **Un répertoire Git et un Kustomization Flux par zone.** Éclater, c'est
   pointer un autre cluster sur le répertoire de la zone 2.
4. **Des bases séparées par zone** : un cluster CloudNativePG propre au Keycloak
   de plateforme, jamais partagé avec les SaaS ; des sauvegardes vers des
   destinations distinctes.
5. **Des destinataires SOPS distincts par zone** : les secrets de la zone 2 ne
   se déchiffrent pas avec la clé age de la zone 1.
6. **Des étiquettes et des *taints* de nœud dès maintenant**, pour qu'au
   deuxième nœud la zone 2 s'y range sans réécrire les manifestes.
7. **Un exercice de reprise (ADR-036) qui fait le déménagement** : sortir la
   zone 2 sur une machine neuve, puis déchiffrer une donnée existante. C'est la
   preuve que l'éclatement marche avant qu'il ne soit nécessaire.
8. **L'accès d'urgence GPG testé**, pas seulement prévu : si le cluster tombe,
   le Keycloak de plateforme tombe avec lui, et c'est par ce chemin qu'on le
   répare.

## Alternatives écartées

- **Les zones séparées dès le premier jour** (ADR-014 tel quel). Plus sûr, mais
  plusieurs machines payées sans revenu. Écarté par le superviseur.
- **Supervision dans le cluster.** Économise un serveur, mais rend la
  supervision aveugle à la panne qu'elle doit signaler. Écarté.
- **Éclatement à une date fixe.** Plus prévisible, mais déconnecté de ce qui
  paie les machines. Écarté par le superviseur au profit du premier revenu.

## Conséquences

- **Coût** : un cluster à un nœud et un petit VPS Sluis, en plus du serveur
  actuel et du témoin externe.
- **Risque assumé** : pendant la dérogation, quiconque obtient les droits
  d'administration du cluster (Flux les a, ADR-029) peut lire les identifiants
  du KMS et la mémoire d'OpenBao, donc atteindre la clé maîtresse. Les espaces
  de noms ne protègent pas contre un administrateur du cluster.
- **Haute disponibilité** : décidée pour Keycloak (ADR-042), elle reste nominale
  sur un nœud ; deux réplicas tombent ensemble.
- **Panne commune** : la chute du cluster coupe les SaaS, la connexion des
  utilisateurs et l'administration en même temps.
- **Dette datée** : l'éclatement est du travail certain, chiffré à l'avance par
  l'exercice du point 5.7.

## Mesures CyFun 2025 couvertes

Renvoi à `foyer/skills/conformite.md` §2bis. La dérogation **affaiblit** la
segmentation (PR.IR-01, à confirmer) pendant sa durée ; la mesure
compensatoire est le déclencheur écrit et l'exercice de déménagement.

## Points ouverts

- **Garde-fou sur les données** (recommandation) : faut-il aussi déclencher
  l'éclatement à la **première donnée personnelle réelle** chiffrée par OpenBao
  en production, si elle arrive avant le premier revenu ? Une démo gratuite
  avec de vrais utilisateurs remplirait OpenBao sans rien encaisser.
- **Taille du petit VPS Sluis** : à mesurer avec les deux conteneurs, k6 et
  `tofu` compris.
- **Ce que veut dire « premier revenu »** : premier paiement encaissé, premier
  contrat signé, ou seuil couvrant le coût des machines de l'éclatement ?
- **Contresignature** de cet ADR.
