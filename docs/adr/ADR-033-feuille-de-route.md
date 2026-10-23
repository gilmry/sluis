# ADR-033 : Feuille de route, huit étapes utilisables seules

- **Statut** : Proposé le 2026-10-21, non contresigné.
- **Amendé le 2026-10-21** (revue des ADR, décision 6) : l'étape 6 devient la seconde production éphémère restaurée chaque nuit. Amendement proposé, non contresigné.
- **Amendé le 2026-10-21** (revue des ADR, décision 2) : l'étape 2 compte un VPS de plus pour le Sluis exécutant. Amendement proposé, non contresigné.
- **Décision du superviseur du 2026-10-23** : le miroir des dépôts publics (3a) passe avant l'étape 3 ; les dépôts privés vont vers un Forgejo auto-hébergé (ADR-039).
- **Amendé le 2026-10-22** (lettre de mission du 2026-10-22, chantier 2) : quatre étapes ajoutées après les trois premières, sans renuméroter les existantes (3a à 3d) ; l'étape 1 compte trois YubiKeys ADR-040 étant retenu. Amendement contresigné le 2026-10-23.
- **Irréversible** : non pour l'ordre, qui se révise en gate review. Chaque étape porte ses propres points irréversibles, décidés dans son ADR.
- **S'articule avec** : ADR-000 à ADR-032 ; l'Epic 9 de Sluis, conformité de plateforme (sluis#10, `docs/bmad/04-epics-stories.md`) ; la gouvernance du jalon zéro du porteur ; la future production chez OVH.
- **Sert la vision (ADR-000)** : principe 4, aucune étape n'est construite avant d'être utile ; principe 1, chaque étape automatise d'abord ce que l'étape précédente laissait à la main.

## Contexte

Les ADR 014 à 032 décrivent une plateforme complète. Construite d'un bloc, elle
serait l'inverse de la vision : des mois sans rien d'utilisable, et des briques
dont personne ne vérifie le fonctionnement. Le porteur travaille par créneaux
courts : tant que la règle de capacité du jalon zéro s'applique (tâches XS
uniquement, arrêt à 16 h, aucune mise en production), chaque étape ci-dessous se
découpe en stories XS, et l'ordre lui-même se réévalue à chaque gate review.

## Décision

**Huit étapes, dans cet ordre. Chacune est utilisable seule, même si la suivante n'arrive jamais.**

| # | Étape | ADR | Ce qu'elle apporte seule | Critère de fin vérifiable | Taille |
|---|---|---|---|---|---|
| 1 | **Identités humaines et clés hors ligne** : deux YubiKeys, GPG, `age` | 015, 016 | Les secrets de déploiement sont chiffrés dans Git ; la perte d'une clé ne bloque rien (la seconde YubiKey) | Un fichier SOPS se déchiffre avec chacune des deux YubiKeys, et avec aucune autre clé | S |
| 2 | **Machine de confiance** : step-ca, OpenBao, supervision, témoin externe ; **Sluis exécutant sur un VPS séparé** (un VPS de plus, ADR-014) | 014, 015, 017, 023 | Certificats SSH courts au lieu de clés statiques ; supervision hors des machines surveillées ; un témoin chez un autre hébergeur qui alerte si la supervision se tait ; l'image exemptée du gate de scan ne côtoie pas la clé maîtresse | Une connexion SSH avec un certificat expiré est refusée ; arrêter la supervision déclenche une alerte reçue par le témoin externe | M |
| 3 | **Recette VPS** : Ansible et Coolify | 029, 032, 031 | Un SaaS déployable sur une machine, socle rejouable, dérive détectée chaque jour | Second passage Ansible à `changed=0` ; une mise en ligne passe par une approbation (ADR-031) et rien d'autre | M |
| 4 | **CloudNativePG et archivage des WAL** | 026 | Une base restaurable à la seconde près depuis une autre région | Restauration à un instant donné réussie et chronométrée depuis le stockage distant | M |
| 5 | **Backend chiffré** dans kit-actix | 018, 019 | Données personnelles chiffrées par personne ; effacement par destruction de clé | Le test d'effacement cryptographique passe : après destruction de la clé, la donnée n'est plus lisible, même depuis une sauvegarde | L |
| 6 | **Seconde production éphémère restaurée chaque nuit** | 020, 027 | La restauration est prouvée chaque nuit, base de clés comprise, et chaque release est répétée sur les données réelles avant la production | La copie de la nuit tourne sur la sauvegarde de la veille avec sa base de clés, ne peut joindre aucun tiers, est détruite au matin, et sa durée est consignée | M |
| 7 | **Barrière et preuves** | 021, 022, 025 | Une mise en ligne est refusée si un contrôle échoue ; chaque déploiement laisse un rapport daté | Un contrôle volontairement rompu (témoin) bloque le déploiement ; le rapport du déploiement suivant est lisible hors du serveur | M |
| 8 | **Recette K3s, trois nœuds** | 029, 026, 027, 032 | Disponibilité et croissance, sans changer la skill Foyer | Bascule d'un nœud sans interruption du SaaS de référence ; mêmes outils MCP qu'en recette VPS | L |

**Étapes ajoutées le 2026-10-22** (amendement contresigné le 2026-10-23). Elles
se placent **après les trois premières** et ne les bloquent pas. Elles ne
renumérotent pas les étapes 4 à 8, que d'autres ADR citent ; leur ordre par
rapport à ces étapes s'arbitre en gate review.

| # | Étape | ADR | Ce qu'elle apporte seule | Critère de fin vérifiable | Taille |
|---|---|---|---|---|---|
| 3a | **Miroir des dépôts** : publics vers Codeberg, privés vers un Forgejo auto-hébergé | 039 | Le code, l'historique et les étiquettes d'approbation survivent à la fermeture du compte GitHub | Un commit poussé sur GitHub apparaît sur le miroir dans le délai fixé ; un miroir arrêté déclenche une alerte | S |
| 3b | **Exercices de reprise, premier cycle partiel** | 035, 036, 037 | La procédure de reconstruction est jouée et chronométrée, au lieu d'être supposée juste | Rapport du scénario 2 en forme VPS archivé hors serveur, durée consignée, écarts corrigés par PR | S |
| 3c | **Double registre d'images** | 038 | Une image se tire encore si un registre tombe ; ce qui tourne est désigné par son empreinte | Une image se tire depuis le registre de secours à empreinte égale ; une référence par étiquette mobile est refusée | M |
| 3d | **Fédération des identités** | 041 | Les utilisateurs des SaaS se connectent par un fournisseur d'identité commun, y compris depuis l'annuaire d'une ESN | Une connexion OIDC fédérée réussit sur le SaaS de référence ; un second `plan` du fournisseur OpenTofu est sans changement | M |

**Place de 3a (décision du superviseur du 2026-10-23).** Le miroir des dépôts
publics vers Codeberg, petit et sans risque, se fait **avant l'étape 3**. Le
Forgejo des dépôts privés demande une machine et son exploitation : sa place
suit le choix de son hébergement (point ouvert d'ADR-039).

**Dépendances de ces étapes.** La vérification de signature d'ADR-038 est un
contrôle de la barrière, donc de l'étape 7 : avant elle, 3c se limite au double
registre et à l'empreinte. L'opérateur Keycloak d'ADR-041 suppose un cluster
(étape 8) : avant lui, le mode de déploiement de 3d est un point ouvert
d'ADR-041. Le premier cycle complet des exercices (trois scénarios) suit
l'étape 6.

**Étape 1 et ADR-040.** ADR-040 étant retenu, l'étape 1 compte trois
YubiKeys et son critère de fin se lit « avec chacune des trois YubiKeys ».

Tailles : S environ une semaine, M deux à trois semaines, L un mois ou plus, en
temps calendaire du porteur. Toutes sont des **bornes hautes de première passe**.

**Pourquoi cet ordre.**

- **Les clés d'abord** (1) : tout le reste en dépend, et c'est l'étape la moins coûteuse à défaire.
- **La machine de confiance avant la production** (2) : sans supervision externe ni certificats courts, la recette VPS reproduirait les pannes silencieuses du passé (ADR-000).
- **La base avant le chiffrement** (4 avant 5) : le chiffrement applicatif change le format des données ; il vaut mieux le poser sur une base déjà sauvegardée et restaurable.
- **La seconde production éphémère avant la barrière** (6 avant 7) : la barrière vérifie qu'une sauvegarde récente existe ; la copie nocturne prouve qu'elle se restaure.
- **K3s en dernier** (8) : c'est la seule étape qui ajoute de la complexité sans ajouter de garantie de sécurité ; elle ne se justifie que par la croissance.

**Lien avec l'Epic 9 (sluis#10).** Les stories 9.2 (gates de conformité), 9.3
(preuve par déploiement) et 9.5 (dérive) relèvent de l'étape 7 ; 9.4
(restauration éprouvée) de l'étape 6 ; 9.6 (séparation des environnements) des
étapes 2 et 3. L'Epic 9 reste le backlog ; cette feuille de route en fixe l'ordre.

**Lien avec la production chez OVH.** Les étapes 1 à 3 suffisent à une première
production sur VPS. Les étapes 4 à 7 la rendent défendable au niveau Essential.
L'étape 8 correspond à la croissance visée au printemps-été 2027.

## Alternatives écartées

- **Commencer par K3s**, parce que c'est la cible : des mois d'infrastructure avant le premier SaaS en ligne, et un cluster sans supervision externe ni restauration prouvée.
- **Commencer par le chiffrement applicatif**, parce que c'est le plus protecteur : il change le format des données d'un backend encore sans sauvegarde ni restauration éprouvée.
- **Un ordre par couche** (tout le réseau, puis tout le stockage) : aucune étape n'est utilisable avant la dernière.

## Conséquences

- Chaque étape ouvre ses stories au format Agent IA Ready quand elle devient la prochaine, pas avant.
- Une étape non terminée ne bloque pas l'usage des précédentes ; elle bloque seulement la suivante dans l'ordre recommandé, sauf arbitrage contraire en gate review.
- **Limite** : les tailles supposent la disponibilité actuelle du porteur ; une gate review peut allonger ou réordonner sans que ce soit un échec.

## Mesures CyFun 2025 couvertes

Aucune directement : la feuille de route ordonne les mesures des ADR 014 à 032.
Elle sert le passage au seuil de maturité 3/5 (`foyer/skills/conformite.md`,
§2bis et §2quater) : chaque étape terminée produit des preuves que
l'autoévaluation suivante peut citer.

## Points ouverts

- **Étapes 3b à 3d** (amendement du 2026-10-22) : leur ordre entre elles et par rapport aux étapes 4 à 8, à trancher en gate review. 3a est placée (voir plus haut).

- Faut-il une étape 0, l'inventaire de l'existant (machines, secrets, sauvegardes actuelles), pour mesurer le chemin restant ?
- Où placer la migration des projets déjà en ligne vers la recette VPS : après l'étape 3, ou après l'étape 7 ?
- Réévaluer l'ordre à chaque gate review, et noter dans cet ADR la date de chaque réordonnancement.
