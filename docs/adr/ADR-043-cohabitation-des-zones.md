# ADR-043 : Cohabitation des zones au démarrage, éclatement dès les premiers revenus

- **Statut** : Proposé le 2026-11-13, contresigné le 2026-11-14.
- **Décisions du superviseur du 2026-11-13**, à l'origine de cet ADR : « pour des raisons de coût, tout mettre sur le même cluster, avec des mesures conservatoires pour l'éclatement en plusieurs zones » ; « supervision sur ce serveur, et Sluis planifiant et exécutant sur un même petit VPS au début » ; « dès que l'argent rentre, on fait les migrations et on éclate les zones et les serveurs ».
- **Précision du superviseur du 2026-11-13** : ce qui compte, ce sont l'architecture et les mesures conservatoires, et **avant chaque migration, une restauration déchiffrée jouée** pour vérifier qu'un retour arrière malheureux est possible (point 6).
- **Dérogation bornée**, sur le modèle d'ADR-007 : elle prend fin à son déclencheur (point 4), et rien ne la reconduit en silence.
- **Amende, pour la durée de la dérogation** : ADR-014 (zones 1 et 2 sur un même cluster), ADR-017 (OpenBao dans ce cluster, hors de la machine de confiance), ADR-010 (Sluis public et exécutant sur un même VPS). ADR-042 (sluis#101) est concerné : la haute disponibilité de Keycloak n'y devient effective qu'à l'éclatement.
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
barrière rejouée, preuve au journal ; et aucune ne part sans la restauration
déchiffrée du point 6.

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

**6. Avant chaque migration, une restauration déchiffrée jouée.** Un retour
arrière n'est possible que s'il a été joué. Revenir à l'image d'avant ne défait
pas une migration : sans restauration éprouvée, le retour arrière aggrave la
panne.

1. **Une donnée témoin** est enveloppée par OpenBao avant l'instantané, à côté
   des données réelles existantes.
2. **L'instantané** de ce qui migre est pris : stockage d'OpenBao
   (`bao operator raft snapshot save`), base du Keycloak de plateforme
   (CloudNativePG), secrets SOPS de la zone concernée.
3. **Il est restauré sur un environnement jetable**, isolé du réseau de la
   production, comme les exercices d'ADR-036, avec le même déverrouillage (KMS
   d'OVHcloud).
4. **Le contrôle est un déchiffrement, pas un démarrage** : la donnée témoin
   **et** une enveloppe réelle existante se désenveloppent ; le Keycloak
   restauré émet un jeton dont un client vérifie la signature, avec le même
   `iss` ; une connexion FIDO2 aboutit.
5. **Fail-closed** : le contrôle est un exécutable dont seul le code de sortie
   fait foi (ADR-021). Sans restauration verte de moins de 24 heures, la
   barrière refuse la migration. Le résultat entre au rapport de preuve
   (ADR-022).
6. **Le chemin de retour reste intact** jusqu'à la fin de la validation : la
   source n'est pas détruite, elle passe en lecture seule. Revenir en arrière,
   c'est rejouer sur la source l'instantané qui vient d'être éprouvé.
7. **Gel pendant la fenêtre**, de l'instantané à la fin de la validation : pas
   de rotation de la clé maîtresse ni des clés de signature du realm, pas de
   nouvel enrôlement. Une version de clé ou une passkey créée après
   l'instantané serait perdue au retour arrière, et ce qu'elle a chiffré
   deviendrait illisible.

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
- **Durée de la fenêtre de validation** (point 6.6 et 6.7) avant de détruire la
  source : à fixer par migration.
- **Étendre le point 6** à toute migration de schéma des SaaS, au-delà de
  l'éclatement ? La seconde production éphémère (ADR-020) restaure déjà chaque
  nuit ; il resterait à en faire une condition de la barrière.
