# ADR-044 : Le noyau de Sluis et la plateforme commune, atteinte par paliers du moins cher au plus complet

- **Statut** : Proposé le 2026-11-14, contresigné le 2026-11-14.
- **Décisions du superviseur du 2026-11-13 et du 2026-11-14**, à l'origine de cet ADR : les mécanismes de sécurité « handicapent les déploiements des SaaS » et sont « trop contraignants pour le moment », et il n'y a « pas les fonds pour une infrastructure aussi importante » ; « maintenant qu'on a l'image cible, il faut planifier une évolution du moins cher au plus complet » ; « l'idée de Sluis à la base, c'est de permettre à Claude Code d'utiliser un serveur MCP pour faire un test de montée en charge avec une skill Foyer, et après de demander la mise en production. C'est tout. » ; séparation du noyau et de la plateforme validée le 2026-11-14.
- **Amende ADR-033** : l'ordre des huit étapes et de 3a à 3d est remplacé par les paliers du point 3. Les étapes, leurs critères de fin et leurs ADR restent la définition de la cible ; seul leur déclenchement change.
- **Ne change pas** la cible : ADR-014 à ADR-043 décrivent toujours la plateforme complète. ADR-042 (sluis#101) et ADR-043 (sluis#102), contresignés le 2026-11-14, y prennent leur place au palier indiqué.
- **Irréversible** : non. Un palier se révise en gate review. Les mesures conservatoires du point 4 sont justement là pour qu'aucun palier ne force à défaire le précédent.
- **S'articule avec** : ADR-000 (principe n°4), ADR-007 (bacs à sable), ADR-010 et ADR-012 (« Sluis ne peut que demander »), ADR-018 (chiffrement par personne, conditionné au point 5), ADR-021 et ADR-022 (barrière, preuve), ADR-033 (qu'il amende), ADR-041 (Keycloak), ADR-042, ADR-043.
- **Sert la vision (ADR-000)** : principe n°4 avant tout, « chaque brique justifie son coût » : une garantie n'arrive que quand un événement réel la justifie ; principe n°3, parce que chaque palier garde une restauration éprouvée, même au plus bas.

## Contexte

Sluis est né pour une chose : permettre à un agent (Claude Code et une skill
Foyer) de lancer, par un serveur MCP, un test de montée en charge sur un SaaS,
puis de **demander** sa mise en production. Ce noyau est en grande partie
livré : campagne k6 et verdict avec taux d'échec, plancher, barrière, rapport de
preuve, demande de mise en ligne.

Une seconde ambition s'y est greffée : une **plateforme commune** dont tous les
SaaS hériteraient les mécanismes de sécurité (zones, identités, coffre de clés,
chiffrement par personne, seconde production éphémère, reprise). ADR-014 à
ADR-043 la décrivent. ADR-033 l'ordonne en étapes, mais toutes se suivent sans
condition d'argent ni de besoin : la plateforme complète devient le préalable
implicite de tout déploiement.

Résultat mesuré par le superviseur : les déploiements des SaaS en cours sont
freinés, et l'infrastructure visée (au moins trois machines louées en
permanence, plus un témoin externe) n'est pas finançable avant le premier
revenu. Le coût du chiffrement à la volée n'a jamais été mesuré.

## Décision

**1. Deux choses distinctes.**

| | Noyau de Sluis | Plateforme commune |
|---|---|---|
| **Ce que c'est** | Le serveur MCP : test de charge, verdict, demande de mise en production | Le socle dont les SaaS héritent la sécurité |
| **ADR** | 005, 006, 007, 008, 010, 011, 012, 021, 022, 030, 031, 034 | 014 à 020, 023 à 029, 032, 035 à 043 |
| **Dépend de** | Rien de la plateforme | Paliers du point 3 |

**Le noyau ne dépend d'aucun palier.** Une campagne de charge et une demande de
mise en production marchent au palier 0. La plateforme, elle, est une cible
atteinte palier par palier.

**2. Règle des paliers.** Un palier s'ouvre sur un **événement réel**, jamais
sur une date. Il n'ajoute que les garanties que cet événement justifie. Chaque
palier tient seul : si le suivant n'arrive jamais, celui-ci reste défendable.

**3. Les paliers.**

| Palier | Déclencheur | Machines | Ce qu'il apporte | Étapes d'ADR-033 |
|---|---|---|---|---|
| **0. Tenir proprement ce qui tourne** | maintenant | aucune de plus | images référencées par empreinte ; secrets SOPS + age ; sauvegardes chiffrées hors hôte, **restauration testée chaque semaine** ; supervision actuelle et chien de garde ; noyau de Sluis utilisable | 1 en partie (age, YubiKeys quand elles sont là), 3a public |
| **1. Comptes utilisateurs réels** | le premier SaaS qui ouvre des comptes | aucune de plus (environ 1,2 Go de mémoire, ADR-041) | un Keycloak unique, un realm par SaaS, passkeys ; SSH d'administration par clé `ed25519-sk` sur YubiKey, sans PKI | 3d sous forme réduite |
| **2. Premier revenu** | un premier paiement encaissé | un nœud K3s et un petit VPS Sluis | K3s, Flux et Ansible ; CloudNativePG et archivage des WAL ; Keycloak de plateforme et OpenBao dans le même cluster (ADR-043) ; chiffrement par personne pour les nouvelles données, **si le point 5 le permet** ; approbation signée, barrière et preuves obligatoires | 3, 4, 5, 7 ; ADR-042 sans haute disponibilité |
| **3. Revenu qui paie les machines** | des clients engagés, dont une ESN | une machine de confiance et le VPS de l'exécutant | éclatement des zones, chaque migration précédée d'une restauration déchiffrée (ADR-043, point 6) ; seconde production éphémère ; double registre, miroir privé, exercices de reprise | 2, 6, 3a privé, 3b, 3c |
| **4. Croissance** | une charge ou un contrat qui l'exige | trois nœuds | Keycloak en haute disponibilité ; CyFun 2025 Essential complet | 8 |

**4. Mesures conservatoires, gratuites, dès le palier 0.** Elles ne coûtent
qu'un choix et évitent de défaire un palier au suivant :

1. **Noms DNS stables** pour les futurs émetteurs de jetons et pour chaque
   service : le `iss` d'un jeton ne doit jamais changer à un déménagement.
2. **Un répertoire Git par future zone**, même quand toutes les zones tournent
   sur une machine.
3. **Chaque SaaS parle OIDC dès qu'il a des comptes**, derrière un port du kit
   avec un adaptateur local pour les gates : l'arrivée ou le déménagement de
   Keycloak devient une affaire de configuration.
4. **Un identifiant interne par personne**, jamais le courriel (ADR-041,
   point 9) : le chiffrement par personne s'y branchera sans migrer les clés
   étrangères.
5. **Le port `Chiffreur` existe avant le chiffrement** (ADR-018) : au palier 0,
   un adaptateur sans effet ; le chiffrement devient un changement d'adaptateur.
6. **Aucun secret dans le code**, des images par empreinte, des sauvegardes
   chiffrées.
7. **La configuration de Keycloak dans Git** (fournisseur OpenTofu, ADR-041)
   dès le palier 1 : son passage d'une machine au cluster du palier 2 ne
   déplace plus que la base de comptes, précédé d'une restauration jouée.

**5. Le chiffrement par personne attend sa mesure.** Avant de l'inscrire au
palier 2, un spike mesure son coût (gilmry/foyer#28 pour le kit, sluis#104
pour la campagne) : le port `Chiffreur` dans kit-actix, quatre
variantes (en clair, chiffrement local, OpenBao sans cache, OpenBao avec
cache), une campagne de charge lancée **par le noyau de Sluis**. Fini quand un
tableau daté donne pour chacune la latence p95, le débit, le processeur, la
mémoire et le nombre de requêtes vers OpenBao, avec la taille de la machine et
le commit mesuré. C'est aussi le premier usage bout en bout du noyau. Le coût
fonctionnel (champs chiffrés qui ne se recherchent plus en SQL) est relevé à
côté.

**6. Risque assumé au palier 0.** Un SaaS déjà en production avec de vrais
utilisateurs n'a pas de chiffrement par personne. Sa protection repose sur des
sauvegardes chiffrées, une restauration testée et le fait de ne garder que les
données nécessaires. Ce risque est écrit ici pour qu'il soit levé au palier 2,
pas oublié.

## Alternatives écartées

- **Garder ADR-033 tel quel.** Toutes les étapes sans condition, avant le
  premier revenu : c'est ce qui freine aujourd'hui.
- **Abandonner la plateforme commune.** Chaque SaaS réécrirait sa sécurité ;
  la cible reste juste, c'est son calendrier qui ne l'était pas.
- **Des paliers à date fixe.** Plus prévisibles, mais déconnectés de ce qui
  paie les machines et de ce qui crée le risque.

## Conséquences

- **Les déploiements des SaaS ne sont plus suspendus à la plateforme.** Ils
  passent par le noyau, au palier où l'on est.
- **Les garanties arrivent plus tard** : pas de zones séparées ni de chiffrement
  par personne avant le premier revenu. C'est le risque du point 6.
- **Les ADR-014 à 043 restent la cible** ; chaque palier dit lesquels il active.
  Un ADR non encore activé n'oblige à rien.
- **Le jalon zéro** (gouvernance du porteur, hors de ce dépôt) est concerné :
  son critère « déployé via Sluis, Tier 1 réel » est à relire à la lumière du
  palier 0. Décision du superviseur, hors de cet ADR.

## Mesures CyFun 2025 couvertes

Renvoi à `foyer/skills/conformite.md` §2bis. Au palier 0, la couverture se
limite aux sauvegardes et à leur restauration (**PR.DS-11.1 ★**,
**PR.DS-11.2**), à la gestion des secrets (**PR.AA-01.1 ★**) et à la
supervision (**DE.CM-09.1 I**). Le niveau Essential n'est visé qu'au palier 4.

## Points ouverts

- **Échéances du 2026-11-27** : fenêtre d'ADR-007 et `vide_admise_jusqu_au` de
  la barrière. Prolonger de 90 jours, ou laisser se fermer ?
- **Palier 1 sans cluster** : Keycloak en Compose sur la machine actuelle, en
  attendant K3s. ADR-041 (amendé le 2026-10-28) voulait sa forme cible
  directement ; c'est à amender si le palier 1 arrive avant le palier 2.
- **Résultat du spike** (point 5) : s'il est trop coûteux, le chiffrement par
  personne glisse au palier 3 ou se limite aux champs les plus sensibles.
- **Restauration hebdomadaire du palier 0** : quelle base, quel contrôle de
  lecture, quel signal de succès (ADR-000, principe n°2).
