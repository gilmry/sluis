# ADR-022 : Preuves CyFun, un rapport daté par déploiement, conservé hors du serveur

- **Statut** : Proposé le 2026-10-21, non contresigné.
- **Irréversible** : non pour le format ; **oui** pour le stockage choisi, si c'est un stockage à verrouillage en écriture : ce qui y entre ne se corrige plus, ce qui est le but.
- **S'articule avec** : Epic 9 story 9.3 (preuve de conformité par déploiement) et story 9.1 (profil de conformité déclaré) ; journal append-only de la story 0.5 ; ADR-021 (chaque gate de la barrière produit un rapport) ; ADR-020 (relevé de restauration) ; ADR-027 (stockage verrouillé en écriture dans une autre région) ; `foyer/skills/conformite.md` §2ter et §2quater.
- **Sert la vision (ADR-000)** : principe n°5 au premier chef, la preuve est produite par l'outil qui a fait le contrôle ; principe n°3, une preuve conservée hors du serveur ne dépend pas de la santé de ce qu'elle atteste.

## Contexte

Le seuil de maturité 3/5 de CyFun demande qu'une activité soit appliquée **et**
documentée, avec une preuve par réponse de l'autoévaluation
(`foyer/skills/conformite.md` §2quater). Les gates de Foyer produisent déjà des
rapports à chaque build ; la barrière de déploiement (ADR-021) en produit à
chaque mise en ligne. Mais un rapport non rattaché, ou qui expire avec la CI au
bout de quatre-vingt-dix jours, ne vaut rien le jour de l'autoévaluation.

Un rapport conservé sur le serveur qu'il atteste a un second défaut : le
porteur, ou un attaquant présent sur ce serveur, peut le réécrire. Pour un
porteur seul, c'est aussi la mesure compensatoire de la séparation des tâches :
des journaux qu'il ne peut pas réécrire.

## Décision

1. **Chaque déploiement produit un rapport de conformité daté**, assemblé par
   Sluis à partir des rapports des gates (FR-024 et ADR-021), jamais rédigé à
   la main.
2. **Chaque contrôle du rapport est relié à son identifiant CyFun**, au niveau
   déclaré par le dépôt (story 9.1). Le mapping vit dans
   `foyer/skills/conformite.md` ; le rapport le **référence** par identifiant,
   il ne le recopie pas.
3. **Le rapport respecte les trois règles de preuve** de conformite.md §2ter :
   rattaché (commit, date, outil et version), conservé au-delà de la CI, et un
   rouge trié compte comme preuve au même titre qu'un vert.
4. **Le rapport est écrit dans le journal append-only** (story 0.5) **et**
   copié hors du serveur, dans un stockage à versioning et verrouillage en
   écriture situé dans une autre région (ADR-027). La copie hors serveur fait
   foi ; le journal local sert au diagnostic.
5. **Une absence de rapport alerte** : un déploiement sans rapport archivé
   dans l'heure est signalé (ADR-023). Un rapport manquant est une preuve
   manquante, pas un oubli.
6. **Export pour l'outil d'autoévaluation du CCB** : le rapport peut être lu
   par identifiant, pour remplir la colonne de preuve. L'autoévaluation
   remplie, elle, ne vit **jamais** dans un dépôt public (conformite.md
   §2quater).

## Alternatives écartées

- **Preuves rédigées au moment de l'autoévaluation** : contredit le principe
  n°5, et reconstruit après coup ce qui aurait dû être capturé au moment du
  contrôle.
- **Artefacts de CI seuls** : expirent, et ne couvrent pas l'état de la
  plateforme au déploiement.
- **Stockage sur le serveur déployé** : réécrivable par qui contrôle ce
  serveur, et perdu avec lui.
- **Un outil de GRC du marché** : une brique de plus à opérer et à payer, pour
  un porteur seul, sans gain sur la qualité des preuves (principe n°4).

## Conséquences

- **Coût** : un bucket verrouillé en écriture dans une autre région, quelques
  kilo-octets par déploiement. Négligeable en stockage ; le verrouillage interdit
  de purger une erreur, ce qui est assumé.
- **Entretien** : le mapping contrôle vers identifiant suit les révisions de
  CyFun (CyFun 2023 et 2025 coexistent jusqu'au 18 avril 2027). Il vit dans
  Foyer, une seule fois, pour tous les projets.
- **Limite** : un rapport prouve que le contrôle a tourné et ce qu'il a rendu,
  pas que le contrôle est pertinent. Un mapping n'est pas une conformité
  (conformite.md §4) ; seule l'évaluation par un tiers dira « conforme ».
- **Limite** : les rapports sont des cartes de faiblesses. Ils ne vont pas dans
  le dépôt public de Sluis ni dans celui d'un projet ; seul leur verdict agrégé
  peut y apparaître.

## Mesures CyFun 2025 couvertes

Story 9.3 de l'Epic 9 : PR.PS-04.1 ★ (journaux produits et conservés),
RS.AN-06.1. Les preuves rassemblées couvrent celles listées par
`foyer/skills/conformite.md` §2ter (ID.AM-02.x, ID.RA-01.x, PR.AA-01.1 ★,
PR.PS-06.x) ; le stockage hors serveur non réécrivable est la mesure
compensatoire de PR.AA-05.6 I décrite en §2bis.

## Points ouverts

- **Durée de conservation** des rapports (proposition : cinq ans, à aligner sur
  les obligations de journalisation applicables).
- **Signature des rapports** : signés par l'identité machine de l'exécutant
  (ADR-015), pour qu'une copie altérée hors du bucket se détecte ?
- **Format** : JSON maison minimal, ou un format existant (OSCAL) qui coûterait
  plus mais se lirait ailleurs ?
