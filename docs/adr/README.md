# Architecture Decision Records

ADR-001 à ADR-009 vivent dans le livrable de l'Architecte,
[`../bmad/03-architecture.md`](../bmad/03-architecture.md), section *ADR*.
Ils n'y sont pas dupliqués ici : une décision recopiée à deux endroits
finit par diverger.

Tous les autres vivent chacun dans **son propre fichier** (`ADR-0NN-titre.md`,
ce dossier), et ce tableau en reste l'index : depuis le 2026-10-21 pour les
nouveaux, et pour ADR-010 à ADR-013, sortis de cet index le même jour (revue
des ADR, décision 7), comme l'amendement d'ADR-006. ADR-000 pose la
vision dont chacun se réclame. ADR-014 à ADR-033 décrivent l'architecture de
plateforme et de sécurité visée (alignement CyFun 2025 Essential) ; ADR-033 en
donne l'ordre de construction. ADR-034 fixe le contrat entre la skill Foyer qui
génère un dépôt et Sluis qui l'exécute. Aucun ADR antérieur n'est supprimé ni remplacé :
ceux qui sont amendés le disent dans leur ligne.

| # | Décision | Irréversible | Statut |
|---|---|---|---|
| [ADR-000](ADR-000-vision.md) | Vision, l'humain lit les alertes et valide l'irréversible | non | proposée le 2026-10-21, non contresignée |
| ADR-001 | Hexagonale + SOLID | non | acceptée |
| ADR-002 | TDD + BDD + Documentation Vivante | non | acceptée |
| ADR-003 | DDD ubiquitous language → mapping code | non | acceptée |
| ADR-004 | Rust plutôt que le défaut BMAD Python + FastAPI | non | acceptée, écart méthodologique assumé |
| ADR-005 | Le contrat MCP est matérialisé, pas décrit | **oui** | acceptée |
| ADR-006 | Persistance CQRS SQL pur, pas d'ORM | **oui** | contresignée le 2026-08-29 · **amendée à la fabrication** : [dépôt fichier](ADR-006-amendement-depot-fichier.md), amendement contresigné le 2026-10-18 |
| ADR-007 | Dérogation Tier 2 bornée **et à durée limitée** pour les bacs à sable | **oui** | **contresignée le 2026-08-29** · fenêtre ouverte jusqu'au **2026-11-27** |
| ADR-008 | Passerelle d'approbation = environnement GitHub protégé | **oui** | acceptée · **amendée** pour les dépôts privés par ADR-013 et ADR-031 (proposées) |
| ADR-009 | Mapping branche → environnement | **oui** | acceptée |
| [ADR-010](ADR-010-deux-deploiements-opentofu.md) | Deux déploiements, et OpenTofu comme moteur | **oui** | **contresignée le 2026-10-18** · **amendée le 2026-10-21** : l'exécutant tourne sur un VPS séparé, hors de la machine de confiance (proposé, non contresigné) |
| [ADR-011](ADR-011-gate-de-scan-differencie.md) | Gate de scan différencié par image | non | **contresignée le 2026-10-18** |
| [ADR-012](ADR-012-mise-en-ligne-par-mcp.md) | La mise en ligne se demande par un outil MCP du Sluis public · amende ADR-010 | **oui** | **contresignée le 2026-10-19** · relance d'un run approuvé à vérifier au premier essai (#4, 4.5) |
| [ADR-013](ADR-013-approbation-par-etiquette-signee.md) | Pour un dépôt privé, l'approbation est une étiquette signée par le superviseur, vérifiée par le tireur GitOps de la cible · amende ADR-008 | **oui** | **contresignée le 2026-10-21** · **amendée le 2026-10-21** : l'étiquette nomme son environnement (contresigné le 2026-10-21) ; format du message signé fixé le 2026-10-23 |
| [ADR-014](ADR-014-trois-zones.md) | Trois zones, la production, la confiance, le hors-ligne | **oui** | proposée le 2026-10-21, non contresignée · **amendée le 2026-10-21** : le Sluis exécutant quitte la machine de confiance ; identifiants CyFun complétés (proposé, non contresigné) |
| [ADR-015](ADR-015-identites.md) | Identités, des YubiKeys pour les humains, step-ca pour les machines | **oui** | proposée le 2026-10-21, non contresignée · **amendée le 2026-10-21** : identifiants CyFun complétés (proposé, non contresigné) |
| [ADR-016](ADR-016-secrets-de-deploiement.md) | Secrets de déploiement chiffrés dans Git avec SOPS et age | non | proposée le 2026-10-21, non contresignée · **amendée le 2026-10-21** : identifiants CyFun complétés (proposé, non contresigné) |
| [ADR-017](ADR-017-openbao-hors-cluster.md) | OpenBao hors du cluster, gardien de la seule clé maîtresse | **oui** | proposée le 2026-10-21, non contresignée · **amendée le 2026-10-21** : déverrouillage automatique par le KMS d'OVHcloud ; identifiants CyFun complétés (proposé, non contresigné) |
| [ADR-018](ADR-018-chiffrement-applicatif.md) | Chiffrement applicatif par enveloppe, une clé par personne | **oui** | proposée le 2026-10-21, non contresignée · **amendée le 2026-10-21** : renvoi à ADR-020 mis en cohérence ; identifiants CyFun complétés (proposé, non contresigné) |
| [ADR-019](ADR-019-base-de-cles-et-effacement.md) | Base de clés séparée et effacement cryptographique | **oui** | proposée le 2026-10-21, non contresignée · **amendée le 2026-10-21** : la base de clés est restaurée avec la seconde production éphémère, et sa restauration testée chaque nuit ; identifiants CyFun complétés (proposé, non contresigné) |
| [ADR-020](ADR-020-staging-nocturne.md) | Une seconde production éphémère, restaurée chaque nuit depuis la sauvegarde hors site | non | proposée le 2026-10-21, non contresignée · **amendée le 2026-10-21** : remplace le staging pseudonymisé ; échantillon déchiffré chaque nuit (proposé, non contresigné) |
| [ADR-021](ADR-021-barriere-de-deploiement.md) | Barrière de déploiement, des contrôles déterministes qui bloquent | non | proposée le 2026-10-21, non contresignée · **amendée le 2026-10-21** : barrière rejouée après l'approbation ; identifiants CyFun complétés (proposé, non contresigné) |
| [ADR-022](ADR-022-preuves-cyfun.md) | Preuves CyFun, un rapport daté par déploiement, conservé hors du serveur | non | proposée le 2026-10-21, non contresignée |
| [ADR-023](ADR-023-supervision.md) | Supervision sur la machine de confiance, surveillée de l'extérieur | non | proposée le 2026-10-21, non contresignée · **amendée le 2026-10-21** : identifiants CyFun complétés (proposé, non contresigné) |
| [ADR-024](ADR-024-agent-ia.md) | L'agent IA d'exploitation collecte, propose, et n'agit jamais seul | non | proposée le 2026-10-21, non contresignée · **amendée le 2026-10-21** : identifiants CyFun complétés (proposé, non contresigné) |
| [ADR-025](ADR-025-automatisation-exploitation.md) | Toute tâche d'exploitation récurrente est automatisée et signale son succès | non | proposée le 2026-10-21, non contresignée · **amendée le 2026-10-21** : test de restauration de la base de clés (proposé, non contresigné) |
| [ADR-026](ADR-026-base-de-donnees-cloudnativepg.md) | Les bases des SaaS tournent sous CloudNativePG, archivées en continu hors région | **oui** | proposée le 2026-10-21, non contresignée · **amendée le 2026-10-21** : identifiants CyFun complétés (proposé, non contresigné) |
| [ADR-027](ADR-027-stockage-des-fichiers.md) | Les fichiers vivent dans un fork communautaire de MinIO, chiffrés avant envoi | **oui** | proposée le 2026-10-21, non contresignée · **amendée le 2026-10-21** : bucket de la seconde production éphémère (proposé, non contresigné) |
| [ADR-028](ADR-028-front-end-statique.md) | Le front-end est un site statique sur un stockage objet européen | non | proposée le 2026-10-21, non contresignée |
| [ADR-029](ADR-029-recettes-vps-et-k3s.md) | Deux recettes, VPS puis K3s, derrière les mêmes outils MCP | **oui** | proposée le 2026-10-21, non contresignée · **amendée le 2026-10-21** : identifiants CyFun complétés (proposé, non contresigné) |
| [ADR-030](ADR-030-serveur-mcp.md) | Le serveur MCP lit ou demande, il n'exécute jamais | **oui** | proposée le 2026-10-21, non contresignée · **amendée le 2026-10-21** : identifiants CyFun complétés (proposé, non contresigné) |
| [ADR-031](ADR-031-pipeline-foyer-production.md) | Du staging à la production, par l'une des deux passerelles d'approbation | **oui** | proposée le 2026-10-21, non contresignée · **amendée le 2026-10-21** : l'étiquette nomme son environnement ; répétition sur la seconde production éphémère ; barrière rejouée avant l'exécution (proposé, non contresigné) |
| [ADR-032](ADR-032-outil-de-convergence-ansible.md) | Ansible configure le socle, OpenTofu loue, Flux réconcilie le cluster | **oui** | proposée le 2026-10-21, non contresignée · **amendée le 2026-10-21** : identifiants CyFun complétés (proposé, non contresigné) |
| [ADR-033](ADR-033-feuille-de-route.md) | Feuille de route, huit étapes utilisables seules | non | proposée le 2026-10-21, non contresignée · **amendée le 2026-10-21** : un VPS de plus à l'étape 2 ; étape 6 renommée (proposé, non contresigné) · **amendée le 2026-10-22** : étapes 3a à 3d (miroir, exercices, double registre, identités) après les trois premières (proposé, non contresigné) |
| [ADR-034](ADR-034-contrat-du-depot-cible.md) | Le contrat du dépôt cible : une skill Foyer génère, Sluis valide puis exécute | **oui** | **contresignée le 2026-10-21** |

Tout point marqué irréversible exige une validation humaine explicite
avant d'être mis en œuvre. ADR-007 est le seul du lot qui assouplit un
garde-fou existant : il mérite une signature écrite, pas un accord oral.

C'est aussi le seul qui **s'auto-limite dans le temps**. Sa septième
condition fait expirer la dérogation par défaut et rend son
renouvellement dépendant du gate Tier 1 qu'elle relâche.

> ⏳ **Première fenêtre : 2026-08-29 → 2026-11-27 (90 jours).**
> Passé cette date et sans renouvellement Tier 1, tout bail de bac à
> sable est refusé. C'est voulu : le silence ne reconduit rien.


## Contresignature

- **Jusqu'à la première release en production**, les ADR sont acceptés en
  session par le superviseur, et la date est portée dans la colonne *Statut*.
- **À partir de la première release en production**, chaque contresignature
  est une **étiquette Git signée par la clé SSH de signature du superviseur**
  (celle d'ADR-013), posée sur le commit de l'ADR.
- **Le jour de cette release**, les ADR qui touchent la production sont
  contresignés par ce mécanisme **avant** la mise en ligne.
- **Rappel** : la fenêtre d'ADR-007 se ferme le **2026-11-27** ; son
  renouvellement passe par la voie Tier 1.

## Validation au fil de la feuille de route

Les ADR sont validés au fur et à mesure de la feuille de route (ADR-033),
étape par étape. **ADR-000 et ADR-033 se valident dès l'étape 1.**

| Étape | Contenu | ADR à valider | Construction (avec IA) | Observation |
|---|---|---|---|---|
| 1 | Identités et clés : deux YubiKeys, GPG, age | 015, 016 | 1 à 2 jours | aucune ; délai de livraison des YubiKeys |
| 2 | Machine de confiance : step-ca, OpenBao (déverrouillage par le KMS d'OVHcloud), supervision, témoin externe ; Sluis exécutant sur un VPS séparé | 014, 015, 017, 023 | 3 à 5 jours | 1 à 2 semaines |
| 3 | Recette VPS : Ansible et Coolify | 029, 031, 032, 034 | 2 à 4 jours | 1 semaine |
| 4 | CloudNativePG et archivage des WAL | 026 | 2 à 3 jours | 1 semaine |
| 5 | Backend chiffré, une clé par personne | 018, 019 | 1 à 2 semaines | migration validée projet par projet |
| 6 | Seconde production éphémère restaurée chaque nuit | 020, 027 | 2 à 3 jours | 1 semaine |
| 7 | Barrière rejouée après approbation, preuves CyFun | 021, 022, 025 | 2 à 3 jours | quelques déploiements |
| 8 | Recette K3s, trois nœuds | 026, 027, 029, 032 | 1 semaine | quand la croissance le justifie |

Les étapes 1 à 3 tiennent en 3 à 4 semaines ; l'observation d'une étape peut
chevaucher la construction de la suivante. Estimations de première passe, à
réviser en gate review. Les étapes 1 à 3 suffisent pour une première
production ; les étapes 4 à 7 la rendent défendable au niveau Essential.
