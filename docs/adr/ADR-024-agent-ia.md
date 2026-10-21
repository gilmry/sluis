# ADR-024 : L'agent IA d'exploitation collecte, propose, et n'agit jamais seul

- **Statut** : Proposé le 2026-10-21, non contresigné.
- **Amendé le 2026-10-21** (revue des ADR, décision 7) : identifiants CyFun 2025 complétés. Amendement proposé, non contresigné.
- **Irréversible** : non. C'est une règle de placement des droits ; la relâcher plus tard serait possible, mais chaque relâchement exigerait un nouvel ADR.
- **S'articule avec** : ADR-008, ADR-012 et ADR-013 (l'action passe par une passerelle d'approbation), ADR-030 (outils MCP de lecture séparés des outils d'action), ADR-023 (l'alerte qui déclenche la persona platform engineer), `foyer/skills/enforcement.md` §Le contenu externe et anneau 3.
- **Sert la vision (ADR-000)** : principe 1, l'agent prend en charge le diagnostic récurrent ; principe 3, la proposition du modèle est toujours lue à côté des preuves brutes ; règle « ce qui a des droits ne lit pas de texte libre ».

## Contexte

Une alerte d'Alertmanager (ADR-023) déclenche la persona platform engineer.
Pour un porteur seul, l'intérêt d'un agent est évident : lire les journaux,
corréler, proposer une cause et un correctif pendant que l'humain fait autre
chose. Le danger l'est tout autant. Ce qu'il lit en exploitation n'a pas été
écrit par le superviseur : lignes de journaux, en-têtes HTTP, noms de fichiers
envoyés par des utilisateurs, charges d'alerte, messages d'erreur de
dépendances. Un attaquant qui sait qu'un agent lira ses requêtes peut y glisser
des instructions (injection de prompt indirecte, OWASP LLM #1). Si cet agent
détient un jeton capable d'agir, l'injection devient une action.

Foyer a déjà posé la règle pour le pilote de développement
(`foyer/skills/enforcement.md`, §Le contenu externe). Cet ADR l'applique à
l'exploitation, où les textes lus sont encore moins maîtrisés.

## Décision

L'agent d'exploitation est découpé en trois étapes, chacune avec ses droits,
et la séparation est portée par le **substrat** (anneau 3), pas par une
consigne :

| Étape | Qui | Droits | Lit |
|---|---|---|---|
| 1. Collecter | un script déterministe, versionné | lecture seule, jetons de lecture | métriques, journaux, état Git et cluster, sans les interpréter |
| 2. Diagnostiquer | un modèle | **aucun** : ni outil, ni jeton, ni egress réseau | le paquet produit par l'étape 1 |
| 3. Agir | l'humain, ou un outil MCP d'action | ceux de l'action | la proposition de l'étape 2 **et** les preuves brutes jointes |

1. **L'étape 2 tourne dans un conteneur sans réseau sortant ni jeton** dans son
   environnement. Un `deny` dans la configuration de l'agent ne suffit pas : il
   se contourne par délégation à un sous-agent (limite de l'anneau 2).
2. **Le paquet de l'étape 1 est daté et empreinté.** La proposition de l'étape 2
   le cite ; l'humain peut vérifier que le diagnostic porte bien sur ces
   données-là.
3. **Une action réversible et bornée** (relancer un pod, purger un cache) peut
   être exécutée par un outil MCP d'action, qui crée une demande et ne l'exécute
   pas lui-même (ADR-030). **Une action irréversible** (restauration, rotation,
   retour arrière avec migration, destruction) passe par la passerelle
   d'approbation (ADR-031) : demande, puis approbation humaine.
4. **Le modèle de l'étape 2 est substituable** (adaptateur enfichable) : modèle
   local ou API, sans changer les étapes 1 et 3.

## Alternatives écartées

- **Un agent outillé unique qui lit et agit.** C'est la configuration par défaut
  de la plupart des agents d'exploitation. Une seule ligne de journal piégée
  suffit à lui faire exécuter ce qu'elle dit, dans la limite de ses droits.
- **Filtrer le texte libre avant l'agent outillé.** Un filtre d'injection est un
  détecteur, donc contournable ; il réduit le risque sans le borner.
- **Pas d'agent du tout.** Tenable, mais contraire au principe 1 : le
  diagnostic de premier niveau est exactement le travail récurrent qu'un porteur
  seul ne peut pas tenir la nuit.

## Conséquences

- **Coût** : un conteneur d'analyse isolé et un script de collecte par source.
  Le script est le vrai travail : un collecteur qui oublie une source rend un
  diagnostic faux mais plausible.
- **Plus lent qu'un agent outillé.** Le modèle ne peut pas aller chercher une
  information qui manque ; il le dit, et le collecteur s'enrichit.
- **Limite assumée** : le modèle peut être manipulé et produire une mauvaise
  proposition. C'est accepté parce qu'il ne peut rien exécuter, et que
  l'étape 3 lit les preuves brutes à côté.
- **Limite assumée** : une approbation donnée sans lire les preuves annule la
  séparation. Le risque est humain ; la mesure est de joindre à chaque demande
  des preuves courtes et lisibles, pas un rapport de cinquante lignes.

## Mesures CyFun 2025 couvertes

Mapping dans `foyer/skills/conformite.md` (§2bis, couche plateforme). Moindre
privilège appliqué à l'agent : PR.AA-05.3 (key measure, Basic), déjà cité par
`foyer/skills/enforcement.md`. Analyse d'incident appuyée sur des données
datées : RS.AN-06.1 (Epic 9, story 9.3). Le reste : RS.AN-03.1 E (cause
établie), RS.AN-07.1 E (données d'incident collectées et protégées),
RS.AN-08.1 E (ampleur estimée), DE.AE-02.2 E et DE.AE-04.1 E.

## Points ouverts

- Quelles actions réversibles l'outil MCP d'action peut-il demander sans
  approbation ? Liste fermée à écrire, refusée par défaut.
- Modèle local ou API pour l'étape 2 ? Un modèle par API reçoit des journaux
  d'exploitation, donc potentiellement des données personnelles : le
  collecteur doit pseudonymiser avant envoi, ou l'étape 2 doit être locale.
