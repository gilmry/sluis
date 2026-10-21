# ADR-012 : La mise en ligne se demande par un outil MCP du Sluis public

- **Statut** : contresigné le 2026-10-19 ; relance d'un run approuvé à vérifier au premier essai (#4, 4.5).
- **Sorti de l'index le 2026-10-21** (revue des ADR, décision 7) : texte déplacé depuis `README.md` sans modification de fond.

**Contexte.** Le critère de sortie n°1 du jalon zéro exige que l'approbation
GitHub soit câblée (#4). Aujourd'hui, `PasserelleGithub` et `MiseEnLigne` ne
sont composés dans aucun binaire, et la fenêtre d'ADR-007 n'a aucun chemin réel
de renouvellement avant son expiration du 2026-11-27. Deux questions étaient
ouvertes : quel Sluis détient le jeton qui déclenche le workflow de Tier 1, et
par quelle porte une demande arrive. Le superviseur a tranché le 2026-10-19 :
**le Sluis public, par un outil MCP**, pour qu'un agent puisse demander une mise
en ligne sans que le superviseur ait à ouvrir un terminal sur le serveur.

**Décision, en trois parties.**

**1. Soumettre un plan de Tier 1 est un acte de Tier 2.** L'outil
`sluis_mise_en_ligne` se déclare de Tier 2 : il ne met rien en ligne, il produit
un plan et le soumet. La mutation reste de Tier 1, exécutée par le workflow
GitHub après approbation humaine dans un environnement protégé, avec des
secrets que Sluis n'a pas. C'est la propriété qu'ADR-008 pose déjà : un Sluis
compromis ne peut que soumettre un plan refusable. La règle du serveur MCP
(« un outil de Tier 1 n'est jamais appelé directement ») est inchangée.

**2. ADR-010 est amendé : le Sluis public détient un jeton, et un seul.**
« Ce qui est exposé ne détient rien » devient « ce qui est exposé ne détient
rien qui mute sans un humain ». Le jeton GitHub est limité aux dépôts cibles et
aux droits que la passerelle exerce : déclencher le workflow, lire
l'environnement, lire les runs et leurs approbations. Son identité n'est jamais
celle d'un relecteur de l'environnement (faille S2) : Sluis refuse déjà de
dispatcher si `prevent_self_review` n'est pas actif (#2), ce qui interdit à
l'identité qui déclenche d'approuver son propre run.

**3. Le renouvellement d'ADR-007 se demande au public et s'ouvre chez
l'exécutant.** Le Sluis public soumet le plan `renouvellement_derogation`.
L'exécutant, qui seul détient le secret de signature de la fenêtre, ne l'ouvre
que sur une preuve qu'il lit **lui-même** chez GitHub, avec un jeton en lecture
seule : jamais sur la parole du Sluis public. Un public compromis peut donc
demander un renouvellement, pas l'obtenir.

**Garde-fous, sans lesquels la décision serait un relâchement.**

1. **Aucun texte libre ne traverse.** Les arguments de l'outil sont des valeurs
   validées : un projet de la liste d'autorisation, un environnement nommé, une
   référence au format d'un commit ou d'une étiquette. La description du plan
   est construite par Sluis, jamais recopiée de la demande. Ce qui a des droits
   ne lit pas de texte libre.
2. **Les gates du plancher sont vérifiées avant la soumission** (`MiseEnLigne`),
   et l'environnement est vérifié protégé avant le dispatch (#2).
3. **Chaque soumission est journalisée**, avec son empreinte, comme tout appel
   d'outil.
4. **L'approbateur enregistré est celui que GitHub rapporte** (#3), jamais
   l'identité qui a déclenché.

**Ce que la décision ne couvre pas, et qu'il faut savoir.** Un jeton fuité
permet d'inonder le relecteur de demandes, et le droit de déclencher un
workflow porte en général celui d'annuler ou de relancer un run. **À vérifier
avant contresignature** : qu'une relance d'un run déjà approuvé redemande
l'approbation de l'environnement protégé. Si ce n'est pas le cas, le jeton
public devient capable de rejouer une mise en ligne approuvée, et la partie 2
doit être revue.

**Alternatives écartées.** Le jeton chez l'exécutant, demandé par une commande
lancée sur le serveur : fidèle à la lettre d'ADR-010, mais la mise en ligne
reste alors un geste de terminal, ce qui contredit l'objectif d'autonomie
d'ADR-008. Un outil de Tier 1 approuvé côté client MCP : écarté par ADR-008,
inopérant en exécution autonome.

**Irréversible ?** Oui : le service exposé à Internet détient désormais un
secret. **Contresignée le 2026-10-19.** Le point sur la relance n'a pas pu être
tranché par la documentation de GitHub : il devient un test explicite du premier
essai bout en bout (#4, story 4.5), avant toute mise en ligne réelle.
