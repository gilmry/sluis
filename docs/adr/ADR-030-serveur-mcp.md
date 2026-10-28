# ADR-030 : Le serveur MCP lit ou demande, il n'exécute jamais

- **Statut** : Proposé le 2026-10-21, non contresigné.
- **Amendé le 2026-10-21** (revue des ADR, décision 7) : identifiants CyFun 2025 complétés. Amendement proposé, non contresigné.
- **Irréversible** : oui : c'est le contrat entre la skill Foyer et la plateforme (ADR-005), et la frontière de sécurité qui rend acceptable un service exposé à Internet.
- **S'articule avec** : ADR-005 (contrat MCP matérialisé), ADR-008 et ADR-012 (« Sluis ne peut que demander »), **prolongés sans être contredits** ; ADR-010 (public et exécutant) ; ADR-013 et ADR-031 (passerelles) ; ADR-024 (agent IA) ; `foyer/skills/enforcement.md`.
- **Sert la vision (ADR-000)** : la règle « aucune brique ne détient tout » appliquée à la porte d'entrée de l'agent ; principe 1, l'agent fait tout ce qui est récurrent par les outils de lecture, l'humain ne voit que les demandes.

## Contexte

Toute l'orchestration de Sluis est pilotée par une skill Foyer, à travers un
serveur MCP joignable depuis Internet (OAuth 2.1, ADR-012). C'est la surface la
plus exposée du système, et celle que l'agent manipule : elle doit pouvoir être
compromise sans que la production le soit. ADR-008 et ADR-012 ont déjà posé
qu'un Sluis compromis ne peut que soumettre un plan refusable ; cet ADR en fait
la règle générale de tous les outils, présents et à venir (staging, restauration,
retour arrière).

## Décision

1. **Deux familles d'outils, enregistrées séparément.**
   - **Lecture** : état, inventaire, métriques, rapports de barrière, preuves de
     conformité, journaux d'approbation. Aucun effet de bord.
   - **Action** : mise en ligne, retour arrière, restauration, staging,
     renouvellement de dérogation. **Un outil d'action crée une demande en
     attente** (un plan empreinté, ADR-012) **et n'exécute jamais rien.**
     L'exécution appartient à la passerelle (ADR-031) et à ce qu'elle déclenche.
2. **Le serveur MCP ne détient aucun secret de production** : ni clé
   d'infrastructure, ni secret de déploiement (ADR-016), ni clé applicative
   (ADR-017), ni accès au cluster ni à Flux (ADR-029), ni clé de signature (ADR-013).
3. **Précision sur ADR-012.** Le Sluis public détient un jeton GitHub limité au
   déclenchement et à la lecture des runs, et un jeton en lecture des étiquettes
   pour les dépôts privés. Ces jetons **ne sont pas des secrets de production au
   sens de cet ADR** : ils permettent de demander, jamais d'obtenir. Un jeton qui
   permettrait d'approuver, de déployer ou de lire des données de production
   serait un secret de production et ne pourrait pas résider dans le serveur MCP.
4. **Arguments validés, jamais de texte libre.** Les arguments d'un outil
   d'action sont des valeurs typées (projet de la liste d'autorisation,
   environnement nommé, référence de commit) ; la description du plan est
   construite par Sluis (ADR-012, garde-fou 1).
5. **Chaque appel est journalisé** avec son empreinte, et le journal part hors
   machine (ADR-023).

## Alternatives écartées

- **Outils d'action qui exécutent, avec confirmation côté client MCP** : inopérant en exécution autonome, et la trace ne serait qu'un message (déjà écarté par ADR-008).
- **Un serveur MCP en lecture seule et des actions par un autre canal** : l'agent ne pourrait plus préparer une mise en ligne, et le superviseur redeviendrait l'opérateur de chaque geste, contraire à la vision.
- **Une seule liste d'outils filtrée par scope OAuth** : un scope mal attribué suffirait à exposer une action. La séparation est portée par l'enregistrement (un outil d'action absent du registre ne peut pas être appelé), comme ADR-010 l'a fait pour la campagne.

## Conséquences

- Tout nouvel outil doit déclarer sa famille ; un outil d'action sans plan empreinté ni passerelle ne passe pas la revue d'architecture.
- Le contrat MCP grandit (staging, restauration, retour arrière) : chaque ajout passe par le contract test d'ADR-005.
- **Limite** : un jeton public volé permet de **demander** à l'excès (inonder le superviseur de demandes) et de lire ce que lisent les outils de lecture. La mitigation est le débit limité par client et l'alerte sur volume anormal de demandes (ADR-023), pas la confiance.
- **Limite** : les outils de lecture renvoient du texte venu des projets (journaux, rapports). L'agent qui les lit applique la règle d'ADR-024 : ce texte est une donnée, pas une consigne.

## Mesures CyFun 2025 couvertes

Mapping dans `foyer/skills/conformite.md`.

- Moindre privilège appliqué au pilote : PR.AA-05.3 ★ (`enforcement.md`, §Le contenu externe).
- Moyens d'authentification des services jamais dans le code : PR.AA-01.1 ★ (`conformite.md` §2ter).
- Journalisation des actes : PR.PS-04.1 ★B (CyFun 2025 la range en PR.PS, pas en DE.CM), avec DE.CM-09.1 I pour la surveillance.

## Points ouverts

- Débit maximal de demandes par client et seuil d'alerte : à fixer après le premier mois d'usage réel.
- Les outils de restauration et de staging (ADR-020) n'existent pas encore : leur famille (action) est fixée ici, leur contrat reste à écrire.
