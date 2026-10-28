# ADR-028 : Le front-end est un site statique sur un stockage objet européen

- **Statut** : Proposé le 2026-10-21, contresigné le 2026-10-28.
- **Irréversible** : non. Le site statique se redéploie ailleurs en une commande ; seule la bascule DNS du domaine public est un point à valider.
- **S'articule avec** : ADR-020 (staging), ADR-021 (barrière de déploiement), ADR-031 (même artefact promu du staging à la production), `foyer/skills/convergence-iac.md` (archétype full-stack : pages Astro statiques + backend conteneurisé), `foyer/KITS.md` (frontend îlots-first, client d'API généré).
- **Sert la vision (ADR-000)** : principe 4, un site statique est la brique la plus simple qui suffit ; principe 3, l'artefact testé en staging est exactement celui qui part en production.

## Contexte

Les kits Foyer produisent un frontend Astro avec des îlots Svelte : en sortie de
build, ce sont des fichiers statiques. Les servir depuis un conteneur web dans
le cluster ajoute une image à scanner, un processus à superviser et une surface
d'attaque, pour un contenu qui ne change qu'au déploiement.

Constaté sur un projet témoin privé : l'URL de l'API est **figée au build**
(argument de construction). Le même artefact ne peut donc pas servir le staging
et la production ; chaque environnement a son build, et ce qui part en
production n'est pas ce qui a été testé.

## Décision

1. **Le site est servi en statique depuis un stockage objet européen**,
   OVHcloud Object Storage, dans la région de la production.
2. **Fichiers pré-compressés** (Brotli et gzip) au build, et **en-têtes de cache
   réglés** : longue durée et immuables pour les fichiers dont le nom porte une
   empreinte, courte durée pour le point d'entrée HTML.
3. **La configuration de l'API est chargée à l'exécution** : un petit fichier de
   configuration, propre à chaque environnement, est lu au démarrage de la
   page. Le **même artefact** passe du staging à la production sans
   reconstruction.
4. **L'artefact est construit une fois, empreinté, et promu.** La barrière de
   déploiement (ADR-021) vérifie que l'empreinte promue est celle qui a passé le
   staging.
5. **Cloudflare est écarté** pour la souveraineté : un CDN extra-européen verrait
   passer l'intégralité du trafic, déchiffré à sa frontière. Un CDN européen
   reste possible plus tard si la latence le justifie.

## Alternatives écartées

- **Conteneur web dans le cluster.** C'est l'état actuel ; une image et un
  processus de plus, sans gain pour un contenu statique.
- **URL d'API figée au build.** Simple, mais casse le principe « ce qui part en
  production est ce qui a été testé ».
- **Cloudflare ou un CDN extra-européen.** Performant et gratuit au départ, mais
  terminaison TLS chez un tiers hors UE : contraire à l'objectif de
  souveraineté et à la cartographie des flux attendue par Foyer.
- **Rendu côté serveur.** Utile pour des pages dynamiques ; les kits Foyer
  rendent l'interactivité par îlots, sans besoin de serveur de rendu.

## Conséquences

- **Coût** : faible, stockage et sortie de trafic facturés au volume.
- **Adaptation du code** : un frontend qui fige son URL d'API au build, cas
  actuel du projet témoin, doit lire sa configuration à l'exécution avant
  d'adopter cet ADR. C'est une modification des kits, à porter dans Foyer.
- **En-têtes de sécurité** : le stockage objet doit pouvoir servir la politique
  de sécurité du contenu et les en-têtes HTTP attendus ; à vérifier sur
  l'offre retenue, sinon un proxy léger reste nécessaire.
- **Limite** : la disponibilité du site dépend d'une seule région ; le
  retour arrière consiste à republier l'artefact précédent, conservé avec son
  empreinte.

## Mesures CyFun 2025 couvertes

Mapping dans `foyer/skills/conformite.md` (§2bis). Changements testés avant la
production : PR.PS-06.2 (Important), prouvé par la promotion du même artefact.
Séparation des environnements : PR.IR-01.2 (key measure, Epic 9, story 9.6).
Cartographie des flux et chiffrement en transit : ID.AM-03.3 (key measure,
Essential) et PR.DS-02.2 (Essential), côté application.

## Points ouverts

- L'offre de stockage objet retenue sert-elle les en-têtes de sécurité et un
  domaine personnalisé en TLS sans proxy ? À vérifier avant adoption.
- Le fichier de configuration chargé à l'exécution est public : il ne doit
  contenir aucune valeur sensible, seulement des URL publiques.
