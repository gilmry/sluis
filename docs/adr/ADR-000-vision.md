# ADR-000 : Vision, l'humain lit les alertes et valide l'irréversible

- **Statut** : Proposé le 2026-10-21, contresigné le 2026-10-28.
- **Irréversible** : non. C'est une boussole, pas un mécanisme ; la changer ne casse rien, mais rend caduque la justification des ADR qui s'y réfèrent.
- **S'articule avec** : ADR-008 et ADR-012 (« Sluis ne peut que demander »), ADR-007 (une délégation expire), `foyer/skills/enforcement.md` (les trois anneaux), `foyer/skills/conformite.md` (conforme par construction), `foyer/skills/convergence-iac.md` (la convergence se prouve par l'absence de diff).
- **Numérotée 000** parce qu'elle précède les autres : chaque ADR à partir de la 014 dit comment elle la sert.

## Contexte

Sluis orchestre la création, le déploiement, le retour arrière, la restauration
et le staging des SaaS produits avec la méthode Foyer, et expose toute cette
orchestration par un serveur MCP piloté par une skill Foyer. La cible est
l'alignement **CyFun® 2025 Essential** dès l'architecture.

Le tout est tenu par **une seule personne**, qui dispose d'environ seize heures
de projets par semaine. Les pannes déjà vécues sur l'infrastructure actuelle
donnent la mesure du risque : une supervision morte onze jours pendant que son
alerteur, debout, donnait le change ; des déploiements bloqués sans que personne
ne soit prévenu ; une tâche planifiée qui exécutait du code non commité ; un
build qui ne se faisait plus nulle part, découvert seulement au redéploiement
suivant. Aucune de ces pannes ne venait d'une attaque. Toutes venaient de ce
qu'un humain seul ne peut pas tout surveiller à la main.

## Décision

**Le travail humain quotidien se limite à lire les alertes et à valider les
actions irréversibles. Tout le reste est automatisé.**

Cinq principes en découlent, et chaque ADR suivant les cite :

1. **Automatiser d'abord ce qui est récurrent ou facile à oublier.** Un geste
   rare et risqué reste humain ; un geste fréquent ou oubliable devient une
   tâche planifiée, versionnée, qui ne dépend de la mémoire de personne.
2. **Toute automatisation signale son propre échec ; une automatisation muette
   est cassée.** Chaque tâche émet un signal de succès, et c'est l'**absence**
   de ce signal qui alerte. Le silence n'est jamais une preuve de bonne santé.
3. **Un voyant vert ne prouve rien : croiser deux sources indépendantes.** Un
   système qui se déclare sain peut se tromper sur lui-même. La supervision est
   elle-même surveillée de l'extérieur ; un rapport de sauvegarde se confirme
   par une restauration.
4. **N'ajouter de la complexité que quand elle sert ; chaque brique justifie son
   coût.** Le coût dominant est le temps du superviseur, pas l'infrastructure.
   Une brique qui demande plus d'entretien qu'elle n'en épargne est refusée,
   même si elle est à la mode, même si elle est « CNCF ».
5. **Les preuves de conformité sont produites par les outils, jamais rédigées à
   la main.** Une preuve rédigée prouve qu'on a écrit ; une preuve produite par
   l'outil qui a fait le contrôle prouve que le contrôle a eu lieu, sur quel
   commit, à quelle date.

Deux règles transversales, reprises de Foyer, encadrent toute la plateforme :

- **Aucune brique ne détient tout.** La séparation des responsabilités est
  structurelle : celui qui demande n'exécute pas, celui qui exécute n'est pas
  exposé, celui qui détient la clé maîtresse ne détient pas les données.
- **Ce qui a des droits ne lit pas de texte libre ; ce qui lit du texte libre
  n'a aucun droit** (`foyer/skills/enforcement.md`, §Le contenu externe).

## Alternatives écartées

- **Un humain qui surveille les tableaux de bord.** Il ne tient pas à seize
  heures par semaine, et les pannes passées montrent qu'il rate justement ce qui
  ne fait pas de bruit.
- **Tout automatiser, y compris l'irréversible.** Une migration de schéma, une
  rotation de clé maîtresse, une mise en production ou une destruction de
  données doivent rester des actes dont un humain répond (`foyer/skills/convergence-iac.md`,
  points irréversibles). Les déléguer à une machine supprimerait le *répondre-de*.
- **Une plateforme complète d'emblée.** Chaque brique ajoutée sans besoin est de
  l'entretien sans contrepartie. La feuille de route (ADR-033) construit par
  étapes utilisables seules.

## Conséquences

- **Le critère d'acceptation d'une brique change.** On ne demande pas seulement
  « fonctionne-t-elle ? » mais aussi « dit-elle quand elle ne fonctionne plus,
  et qui le voit ? ». Une brique sans signal de succès n'est pas terminée.
- **L'automatisation coûte d'abord du temps de conception.** Il faut écrire le
  signal, le témoin externe, le croisement des sources. Ce coût initial est
  assumé : il remplace une surveillance humaine qui ne tiendrait pas.
- **La validation humaine devient rare et lourde de sens.** Elle porte sur peu
  d'actes, toujours irréversibles, toujours accompagnés de leurs preuves
  brutes. Une demande d'approbation qu'on valide sans la lire est le risque à
  surveiller.
- **Limite : un seul humain reste un seul humain.** La séparation des tâches et
  l'évaluation indépendante ne passent que par des mesures compensatoires
  (`foyer/skills/conformite.md`, §2bis), documentées comme telles.

## Mesures CyFun 2025 couvertes

Aucune directement : la vision est la raison des mesures, pas une mesure. Elle
fixe la **méthode de preuve** attendue par le seuil de maturité 3/5 (appliqué
**et** documenté), que `foyer/skills/conformite.md` §2ter décrit : rapports
produits par les outils, rattachés à un commit, conservés au-delà de la CI.

## Points ouverts

- Comment mesurer que la vision tient ? Proposition : compter chaque mois les
  interventions humaines non irréversibles. Chacune est soit une automatisation
  manquante, soit une alerte mal calibrée.
