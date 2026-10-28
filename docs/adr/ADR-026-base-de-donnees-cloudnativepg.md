# ADR-026 : Les bases des SaaS tournent sous CloudNativePG, archivées en continu hors région

- **Statut** : Proposé le 2026-10-21, contresigné le 2026-10-28.
- **Amendé le 2026-10-21** (revue des ADR, décision 7) : identifiants CyFun 2025 complétés. Amendement proposé, contresigné le 2026-10-28.
- **Irréversible** : **oui**. Le choix de l'opérateur fixe le format des sauvegardes, des archives WAL et des procédures de restauration de toutes les bases ; en sortir est une migration de données.
- **S'articule avec** : ADR-006 et son amendement (**inchangés** : ils portent sur la persistance interne de Sluis, un fichier JSON, pas sur les bases des SaaS orchestrés), ADR-019 (base de clés séparée), ADR-020 (le staging restaure depuis l'archive), ADR-023 (circuit d'alerte), ADR-029 (recette K3s), `foyer/skills/convergence-iac.md` (archétype stateful, migrations irréversibles).
- **Sert la vision (ADR-000)** : principe 1, la bascule et l'archivage sont automatiques ; principe 2, chaque mécanisme a son alerte ; principe 4, pas de multi-maître tant qu'il ne sert pas.

## Contexte

Les SaaS produits avec Foyer sont majoritairement de l'archétype *stateful*
(`foyer/skills/convergence-iac.md`) sur PostgreSQL : leurs données sont la
valeur, et la cible CyFun Essential exige des sauvegardes immuables et une
restauration éprouvée. Aujourd'hui, une base tourne dans un conteneur à côté de
l'application, sauvegardée par un script. La recette K3s (ADR-029) a besoin
d'un mécanisme qui tienne la bascule, l'archivage et la restauration sans
intervention humaine.

## Décision

1. **CloudNativePG**, opérateur Kubernetes, gère chaque base dans un **espace de
   noms dédié**, séparé des applications.
2. **Pas de multi-maître.** Un principal et des répliques en lecture, sur des
   **nœuds distincts**. La bascule est automatique ; elle est alertée.
3. **Archivage continu des WAL** vers un stockage objet S3 **d'une autre
   région**, en plus des sauvegardes de base. La restauration à un instant donné
   est possible sans dépendre de la région de production.
4. **Démarrage à 1 nœud, cible 3 nœuds, jamais 2.** À deux, une partition rend
   la décision de bascule impossible sans risque de double principal ; à un, la
   disponibilité repose sur l'archive, ce qui est assumé au démarrage.
5. **Sluis pilote par des manifestes dans Git** : il propose le manifeste, le
   réconciliateur GitOps l'applique. Toute opération irréversible (montée de
   version majeure, restauration sur la production, suppression) passe par la
   passerelle d'approbation (ADR-031).
6. **Métriques et alertes dans le circuit standard** (ADR-023) : retard
   d'archivage des WAL, âge de la dernière sauvegarde réussie, retard de
   réplication, bascule survenue. L'absence de sauvegarde récente bloque aussi
   la barrière de déploiement (ADR-021).

**« CNCF » n'est pas une conformité.** L'appartenance à la CNCF dit qu'un projet
a une gouvernance ouverte ; elle ne dit rien de la configuration, du
chiffrement, ni de ce que la restauration a été testée. Seules les preuves
d'ADR-022 et la restauration nocturne d'ADR-020 en répondent.

## Alternatives écartées

- **PostgreSQL en conteneur simple avec un script de sauvegarde.** C'est l'état
  actuel ; il ne tient ni la bascule ni l'archivage continu, et le script est
  une automatisation de plus à surveiller.
- **Multi-maître (BDR, Citus distribué).** Complexité de résolution de conflits
  sans besoin démontré : contraire au principe 4.
- **Base managée de l'hébergeur.** Réduit l'entretien, mais lie la restauration
  et la souveraineté au fournisseur ; reste une option de sortie à évaluer.
- **Un autre opérateur (Zalando, Crunchy).** Comparables ; CloudNativePG est
  retenu pour son modèle sans Patroni et son archivage natif. Choix réversible
  tant qu'aucune base de production n'y vit.

## Conséquences

- **Coût** : un stockage objet dans une seconde région, facturé au volume des
  WAL ; trois nœuds à terme.
- **Entretien** : l'opérateur se met à jour (Renovate, ADR-025), et ses montées
  de version touchent toutes les bases ; elles passent par le staging d'abord.
- **Limite au démarrage** : à un nœud, une panne du nœud est une indisponibilité,
  pas une perte de données ; l'archive hors région borne la perte au dernier WAL
  archivé.
- **Limite** : l'opérateur dépend de Kubernetes, donc de la recette K3s ; sur la
  recette VPS (ADR-029), la base reste un conteneur sauvegardé, et cet ADR ne
  s'y applique pas encore.

## Mesures CyFun 2025 couvertes

Mapping dans `foyer/skills/conformite.md` (§2bis, couche plateforme : sauvegardes
immuables et testées). Sauvegarde hors hôte configurée : PR.DS-11.1 (key
measure) ; restauration éprouvée : PR.DS-11.2, PR.DS-11.4, RC.RP-05.1 (Epic 9,
stories 9.2 et 9.4). La disponibilité et la résilience relèvent de PR.IR-03.1 E
et PR.IR-04.1 I ; la redondance (trois nœuds), de GV.OC-04.3 E.

## Points ouverts

- Durée de rétention des archives WAL, à concilier avec l'effacement
  cryptographique d'ADR-019 (la base de clés est conservée moins longtemps).
- Quelle seconde région, et chez quel fournisseur, pour que l'archive ne
  partage aucune panne avec la production ?
- Seuil d'alerte sur le retard d'archivage : à mesurer sur le volume réel.
