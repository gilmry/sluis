# ADR-036 : Exercices de reprise trimestriels, la plateforme se reconstruit et quelqu'un d'autre sait le faire

- **Statut** : Proposé le 2026-10-22, non contresigné.
- **Irréversible** : non. Un exercice se suspend, se réordonne ou s'allège sans rien casser. Les scénarios 1 et 2 manipulent des données réelles et du matériel de clé dans un environnement jetable : ils reprennent le régime d'ADR-020 et n'ajoutent pas de point irréversible, mais la question de la clé maîtresse hors de la zone 2 reste ouverte (ADR-035).
- **S'articule avec** : ADR-020 (la nuit prouve que les données se restaurent ; le trimestre prouve que la plateforme se reconstruit), ADR-022 (chaque exercice produit un rapport archivé hors serveur), ADR-025 (l'exercice est une tâche récurrente dont l'absence alerte), ADR-023 (circuit d'alerte), ADR-014 (ce que chaque zone doit permettre de reconstruire), ADR-017 et ADR-019 (clé maîtresse et base de clés), ADR-026 et ADR-027 (sauvegardes hors région), ADR-032 et ADR-010 (Ansible et OpenTofu, la plateforme écrite dans Git), ADR-007 (**inchangé** : aucun exercice ne se joue dans un bac à sable avec des données réelles), ADR-033 (place dans la feuille de route, point ouvert). Prolongé par **ADR-035** (sauvegarde et reconstruction de la zone 2) et **ADR-037** (problème de l'amorce), rédigés en parallèle.
- **Sert la vision (ADR-000)** : principe n°3, une procédure déclarée complète ne prouve rien, sa reprise chronométrée si ; principe n°1, la reprise, geste rare entre tous, devient un rendez-vous planifié au lieu d'un souvenir ; principe n°5, la durée mesurée est le délai de reprise réel. Et l'objectif du porteur : bâtir des écosystèmes qui n'ont plus besoin de lui.

## Contexte

ADR-020 restaure chaque nuit la production depuis la sauvegarde hors site, avec
sa base de clés. Il prouve que les **données** reviennent. Il ne prouve pas que
la **plateforme** revient : la copie nocturne s'appuie sur une zone de confiance
debout, sur des outils déjà installés et sur un exécutant qui connaît le chemin.
Le jour où la machine de confiance disparaît, ou où toute la production est
perdue, ce sont précisément ces appuis qui manquent.

Reste un risque que ni la nuit ni la supervision ne mesurent : la procédure de
reprise n'existe peut-être que dans la tête du superviseur. Tenue par une seule
personne, une plateforme dont seul son auteur sait la remonter n'est pas
transmissible. Le superviseur l'a demandé le 2026-10-22 : des exercices de
reprise trimestriels, sur trois scénarios, dont l'un sans lui.

## Décision

1. **Chaque trimestre, trois scénarios de reprise sont joués.**

   | # | Scénario | Ce qu'il prouve | Où il se joue |
   |---|---|---|---|
   | 1 | **Destruction de la zone de confiance** | La zone 2 (step-ca, OpenBao, supervision) se reconstruit depuis Git, ses sauvegardes et ce que la zone 3 conserve (ADR-035) | Environnement jetable, isolé du réseau de la production |
   | 2 | **Reconstruction complète de la production** | La zone 1 se reconstruit depuis Git et les sauvegardes hors région seules : machines louées, socle, réconciliateur, applications, bases, fichiers, base de clés | Environnement jetable, régime d'ADR-020 (données réelles, protégé comme la production) |
   | 3 | **Procédure suivie par un tiers** | La procédure écrite suffit, sans le superviseur ni ce qu'il sait sans l'avoir écrit | Environnement jetable, données factices et secrets d'exercice |

2. **Jamais sur la vraie production.** Chaque exercice se joue dans un
   environnement créé pour lui, dans un projet distinct de celui de la
   production, détruit à la fin. Aucune entrée publique de la production n'est
   basculée vers lui ; il ne joint aucun tiers (même règle qu'ADR-020,
   point 4). La destruction est constatée et consignée dans le rapport.

3. **Scénario 1, critères de réussite vérifiables** :
   - OpenBao reconstruit se déverrouille et **déchiffre un échantillon**
     d'enveloppes tirées d'une sauvegarde de la base de clés (ADR-019) : la clé
     maîtresse est revenue, pas seulement le service ;
   - step-ca reconstruit émet un certificat validé par la chaîne attendue ;
   - la supervision reconstruite reçoit une métrique témoin et déclenche une
     alerte témoin ;
   - le second passage Ansible rend `changed=0` (ADR-032) ;
   - aucune étape n'a demandé un geste absent de la procédure (sinon, écart).

4. **Scénario 2, critères de réussite vérifiables** :
   - l'infrastructure est louée par OpenTofu (ADR-010) et configurée par
     Ansible depuis Git, second passage à `changed=0` ;
   - la base est restaurée depuis l'archive WAL hors région (ADR-026), les
     fichiers depuis la réplique (ADR-027), la base de clés à une date
     compatible (ADR-019) ;
   - le SaaS de référence passe ses tests de fumée sur l'entrée de
     l'environnement jetable, et un échantillon de données se déchiffre ;
   - le point de reprise atteint (horodatage de la dernière transaction
     restaurée) est relevé ;
   - un appel sortant vers un tiers échoue (témoin, comme ADR-020).

5. **Scénario 3, le tiers et le superviseur.** Le tiers rejoue la procédure du
   scénario 2 sur des données factices. Il n'a **ni accès privilégié permanent
   ni secret** de la production.
   - **Ce dont il a besoin** : la procédure, dans Git, lisible sans
     explication orale ; un poste à lui ; un accès à l'environnement jetable
     seul, émis pour l'exercice et expirant à sa fin (certificat court,
     ADR-015) ; des secrets d'exercice générés pour l'occasion et détruits
     avec l'environnement ; un moyen de noter ses questions et ses blocages,
     horodatés.
   - **Ce que fait le superviseur** : il émet les accès avant, les révoque
     après, et peut arrêter l'exercice en cas d'incident de sécurité.
   - **Ce qu'il ne fait pas** : il ne guide pas, ne répond pas aux questions
     de fond, ne touche pas l'environnement. Toute question posée est un écart
     consigné. Toute intervention du superviseur pour débloquer fait échouer
     l'exercice.
   - Ce que le scénario 3 ne prouve pas (la restauration des vraies données et
     des vraies clés), les scénarios 1 et 2 et la nuit le prouvent.

6. **Chaque exercice produit un rapport** archivé hors serveur, dans le
   stockage verrouillé en écriture d'ADR-022. Il consigne : scénario, commit de
   la procédure et des outils, **début**, **fin**, **durée mesurée** (le délai
   de reprise réel), étapes bloquantes, écarts à la procédure, verdict de
   chaque critère, point de reprise atteint (scénario 2), constat de
   destruction. Horodatages et verdicts sont produits par les outils
   (principe n°5) ; les écarts sont une observation humaine, datée et
   signalée comme telle. Le rapport est une carte de faiblesses : il ne va pas
   dans un dépôt public (ADR-022).

7. **Échec bruyant.** Un critère non atteint déclenche une alerte (ADR-023),
   ouvre une issue (sans détail d'infrastructure si le suivi est public) et
   appelle une correction de la procédure par PR. **L'exercice est rejoué**
   après correction, sans attendre le trimestre suivant. Un rouge trié reste
   une preuve (ADR-022).

8. **Absence de rapport à l'échéance = alerte** (ADR-025). L'exercice émet un
   signal de succès par scénario ; un scénario sans rapport à l'échéance
   trimestrielle alerte, comme l'absence du relevé du matin pour ADR-020.

## Alternatives écartées

- **S'en tenir à la restauration nocturne** : elle prouve les données, pas la
  plateforme, et s'appuie sur une zone 2 debout et un exécutant qui connaît le
  chemin.
- **Exercice sur la vraie production** (bascule réelle, zone 2 éteinte pour de
  bon) : le risque est celui d'une panne réelle, contraire à la règle de
  capacité du jalon zéro et à ADR-000.
- **Exercice dans un bac à sable d'ADR-007** : sa condition 4 interdit toute
  donnée de production, que les scénarios 1 et 2 manipulent.
- **Revue documentaire de la procédure, sans la jouer** : une procédure lue
  n'est pas une procédure exécutée ; seule la durée mesurée vaut délai de
  reprise.
- **Exercice annuel** : une procédure dérive avec chaque ADR amendé ; un an
  d'écart laisse s'accumuler des étapes fausses invisibles.
- **Tiers muni d'un accès permanent** : contraire à ADR-015 et à la règle
  « aucune brique ne détient tout » ; un accès d'exercice expirant suffit.

## Conséquences

- **Coût** : trois environnements jetables par trimestre, dont un de la taille
  de la production, plus le transfert depuis la région de sauvegarde ; le temps
  du superviseur pour émettre et révoquer les accès, lire les rapports et
  corriger la procédure ; le temps du tiers.
- **La procédure devient un livrable versionné**, au même titre que le code :
  un écart corrigé est une PR.
- **Surface des données réelles** : le scénario 2 ouvre, le temps de
  l'exercice, une copie de plus, protégée comme la production et détruite
  ensuite, avec les mêmes conséquences d'effacement qu'ADR-020 et ADR-019.
- **Limite, scénario 1** : une zone 2 reconstruite détient la vraie clé
  maîtresse et, si step-ca est restauré, la vraie autorité de certification.
  Tant qu'elle existe, elle peut signer ce que la production accepterait.
  L'isolement réseau, la journalisation de chaque usage et la destruction
  constatée en sont les seules bornes ; ADR-035 doit dire si c'est acceptable.
- **Limite, scénario 3** : il prouve que la procédure se lit, pas que le tiers
  pourrait reprendre la production réelle, qui exige des secrets qu'il n'a pas.
  Ce passage relève d'ADR-037.

## Mesures CyFun 2025 couvertes

Restauration éprouvée (Epic 9, story 9.4) : PR.DS-11.2, PR.DS-11.4,
RC.RP-05.1, ici étendue de la donnée à la plateforme. Séparation production /
test : PR.IR-01.2 ★ (story 9.6), les exercices ne touchent jamais la
production. Rapport conservé hors serveur : PR.PS-04.1 ★ (ADR-022). Le test
périodique des plans de réponse et de reprise relève de la catégorie
amélioration (ID.IM), identifiant à confirmer. Le rapport est une preuve au
sens de `foyer/skills/conformite.md` §2ter.

## Points ouverts

- **Qui est le tiers** : pair de la communauté, prestataire, autre personne de
  confiance ; avec quel engagement de confidentialité, et s'il change d'un
  trimestre à l'autre.
- **Coût d'un environnement jetable** par scénario, et plafond de dépense par
  exercice, sur le modèle des bornes d'ADR-007.
- **Calendrier** : date exacte de l'échéance trimestrielle et marge avant
  alerte (proposition : alerte à 92 jours après le dernier rapport du
  scénario) ; délai de rejeu après échec.
- **Les trois scénarios chaque trimestre, ou une rotation** : à trancher en
  gate review au vu de la durée mesurée du premier cycle.
- **Place dans la feuille de route (ADR-033)** : l'exercice ne bloque pas les
  étapes 1 à 3. Proposition : scénario 2 en forme VPS après l'étape 3,
  scénario 1 après l'étape 2 et la publication d'ADR-035, premier cycle
  complet après l'étape 6. ADR-033 n'est pas amendé par cet ADR.
- **Où se joue l'environnement jetable** : chez quel hébergeur, dans quelle
  région (celle de la sauvegarde, une troisième ?), et peut-il réutiliser
  l'outillage de la copie nocturne d'ADR-020 ?
- **Scénario 1, déverrouillage** : par le KMS d'OVHcloud ou par les parts de
  secours hors ligne (ADR-017) ? Le test annuel du déverrouillage manuel de
  secours peut-il s'y loger ? Usage de la vraie clé maîtresse hors de la zone
  2 : à trancher dans ADR-035.
- **Enchaîner les scénarios 1 et 2** (perte totale, zone 2 et zone 1
  ensemble) : variante à jouer, et à quelle cadence ?
- **Accès du tiers à la procédure** : elle vit dans un dépôt privé, sans aucun
  secret (lettre de mission du 2026-10-22, ADR-037). Le tiers reçoit-il un accès
  en lecture limité à la durée de l'exercice, ou une copie imprimée ?
- **Seuil d'alerte sur la durée** : à partir de quel écart avec le cycle
  précédent un délai de reprise mesuré doit-il alerter ?
- **Ce que l'amorce exige** (ADR-037) : quels secrets, comptes et objets
  physiques un tiers devrait recevoir pour reprendre la production réelle, et
  comment le tester sans les lui confier.
