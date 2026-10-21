# ADR-020 : Staging restauré chaque nuit depuis la sauvegarde hors site, pseudonymisé

- **Statut** : Proposé le 2026-10-21, non contresigné.
- **Irréversible** : non pour le mécanisme ; **oui** pour la décision de copier des données de production, même pseudonymisées, dans un second environnement : elle engage la responsabilité RGPD du porteur.
- **S'articule avec** : ADR-007 (**inchangé** : il continue de régir les bacs à sable éphémères, sans aucune donnée de production ; le staging n'en relève pas), ADR-019 (la base de clés ne vient jamais en staging), ADR-026 (sauvegardes WAL), ADR-027 (bucket de staging séparé), ADR-031 (les tests de charge se jouent sur le staging), Epic 9 story 9.4 (restauration éprouvée).
- **Sert la vision (ADR-000)** : principe n°3, une sauvegarde déclarée réussie ne prouve rien, sa restauration nocturne si ; principe n°1, la restauration, geste rare et facile à oublier, devient quotidienne ; principe n°5, la durée mesurée est la preuve.

## Contexte

Une sauvegarde qui n'a jamais été restaurée est une hypothèse. Sur
l'infrastructure actuelle, plusieurs automatismes ont échoué en silence avant
qu'on s'en aperçoive ; rien ne garantit qu'une sauvegarde échappe à ce sort.
Par ailleurs, la méthode a besoin d'un environnement proche de la production
pour les tests de charge et la recette avant mise en ligne (ADR-031).

Les bacs à sable d'ADR-007 ne conviennent pas : ils sont éphémères, bornés à
six heures et vingt euros, et leur condition 4 interdit toute donnée de
production. Le superviseur a tranché le 2026-10-21 : **le staging est un
environnement distinct**, permanent, hors du champ d'ADR-007.

## Décision

1. **Chaque nuit, le staging est reconstruit à partir de la sauvegarde hors
   site**, pas d'une copie directe de la production : c'est la sauvegarde
   réelle qui est éprouvée, avec son chemin de récupération réel.
2. **Les données sont pseudonymisées pendant la restauration**, avant que le
   staging ne s'ouvre : identités remplacées, champs libres effacés, fichiers
   remplacés par des factices ou laissés chiffrés sans clé (ADR-027). La base
   de clés (ADR-019) ne vient jamais en staging, si bien que tout contenu
   chiffré y reste illisible.
3. **La durée de restauration est mesurée et consignée** : début, fin, volume,
   commit de l'outil, verdict d'intégrité. Ce relevé est la preuve de
   restauration, produite par l'outil (ADR-022), et la mesure du délai de
   reprise réel.
4. **Échec bruyant** : une restauration qui échoue, qui dépasse sa durée
   habituelle de façon anormale, ou qui **ne s'exécute pas** déclenche une
   alerte (ADR-023, ADR-025). L'absence du relevé du matin est elle-même une
   alerte.
5. **Le staging est cloisonné** : accès restreint au superviseur et aux
   automatismes, identités machine distinctes de la production (ADR-015),
   bucket séparé (ADR-027), aucune réplication vers la production.

## Alternatives écartées

- **Staging alimenté par des jeux synthétiques uniquement** : ne prouve pas que
  la sauvegarde se restaure, et les tests de charge sur des données irréalistes
  mesurent autre chose que la production.
- **Copie directe de la base de production** : ne teste pas la sauvegarde,
  qui est précisément ce qu'on veut éprouver.
- **Staging dans un bac à sable d'ADR-007** : contredit sa condition 4, et une
  base reconstruite toutes les nuits n'est pas éphémère au sens de l'ADR.
  Assouplir ADR-007, contresigné, pour un besoin qui peut vivre ailleurs, a été
  écarté par le superviseur.
- **Restauration hebdomadaire ou mensuelle** : une sauvegarde cassée resterait
  invisible des jours ; la quotidienne aligne la détection sur la cadence des
  sauvegardes.

## Conséquences

- **Coût** : un environnement permanent de plus (calcul, stockage, transfert
  depuis le site de sauvegarde chaque nuit). C'est le prix d'une restauration
  prouvée tous les jours plutôt que supposée.
- **Limite assumée : pseudonymisé n'est pas anonymisé.** Les données du
  staging restent des données personnelles au sens du RGPD : registre des
  traitements, base légale, durée de conservation, sécurité équivalente. La
  pseudonymisation réduit le risque, elle ne fait pas sortir du règlement.
- **Entretien de la pseudonymisation** : chaque nouvelle colonne contenant une
  donnée personnelle doit être déclarée. Une colonne oubliée passe en clair en
  staging. Proposition : une gate qui refuse une migration ajoutant une colonne
  non classée.
- **Dépendance au site de sauvegarde** : une panne de la région de sauvegarde
  bloque le staging. C'est acceptable, et c'est même un signal : la reprise
  aurait échoué de la même façon.

## Mesures CyFun 2025 couvertes

Story 9.4 de l'Epic 9 (`docs/bmad/04-epics-stories.md`) : PR.DS-11.2,
PR.DS-11.4, RC.RP-05.1. La séparation entre environnements de test et de
production recoupe la story 9.6 (PR.IR-01.2 ★). Le relevé de durée est une
preuve au sens de `foyer/skills/conformite.md` §2ter, à archiver hors serveur
(ADR-022). La couche plateforme qui porte ces mesures est décrite dans
conformite.md §2bis.

## Points ouverts

- **Règles de pseudonymisation** : liste des champs, méthode (substitution
  déterministe ou aléatoire), et qui la maintient. À écrire avec le modèle de
  données de chaque SaaS, dans son livrable Architecture.
- **Seuil d'anomalie de durée** : proposition, alerte au-delà de deux fois la
  médiane des sept dernières nuits.
- **Rétention des données de staging** : effacées à chaque reconstruction, ou
  conservées un jour de plus pour diagnostic ?
- **Base légale RGPD** du traitement de test, à documenter dans le registre du
  porteur (couche organisation).
