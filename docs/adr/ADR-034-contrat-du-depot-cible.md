# ADR-034 : Le contrat du dépôt cible

- **Statut** : Proposé le 2026-10-21 (revue des ADR, décision 8), contresigné le 2026-10-21.
- **Irréversible** : à qualifier par le superviseur. Proposition : **oui** pour la forme du contrat, comme ADR-005 : une fois des dépôts générés selon lui, en changer sans version oblige à reprendre chaque dépôt. Le versionnement du schéma est ce qui rend ses évolutions possibles sans casser l'existant.
- **S'articule avec** : ADR-005 (le contrat MCP est matérialisé, pas décrit : même modèle ici), ADR-009 (mapping branche → environnement), ADR-010 (OpenTofu, moteur de location), ADR-011 (image exécutante), ADR-012 et ADR-013 (le workflow de Tier 1 et sa vérification d'empreinte), ADR-016 (`.sops.yaml`), ADR-022 (journal et preuves), ADR-026 (CloudNativePG), ADR-029 et ADR-032 (playbooks Ansible, manifestes Flux), ADR-030 (famille lecture des outils MCP), ADR-033 (étape 3).
- **Sert la vision (ADR-000)** : règle « aucune brique ne détient tout », parce que la skill Foyer qui écrit les fichiers ne les exécute pas, et que Sluis qui les exécute ne les écrit pas ; règle « ce qui a des droits ne lit pas de texte libre », parce que Sluis n'exécute que ce que le contrat admet ; principe n°3, parce que la skill et Sluis vérifient le même dépôt avec le même schéma, chacun de son côté ; principe n°5, parce que la conformité d'un dépôt est un verdict d'outil.

## Contexte

Sluis lit déjà des fichiers dans les dépôts qu'il sert : `_shared/charge.yaml`,
les profils de cluster, les modules OpenTofu de `monosite/` et `multisite/`. Ce
qu'il en attend n'est écrit nulle part comme un tout : chaque lecteur a ses
règles, et un dépôt n'apprend qu'il n'est pas conforme qu'à l'exécution.

La méthode Foyer veut qu'une skill génère les fichiers d'un projet et que Sluis
les valide puis les exécute. Entre les deux, il faut une frontière que l'un et
l'autre lisent pareil. Une description en prose dériverait du code au premier
changement ; ADR-005 a déjà tranché ce cas pour le contrat MCP.

Une partie de ces fichiers n'est pas de la donnée. Un fichier `.tf` est un
programme : un fournisseur arbitraire, un `local-exec` ou un `data "external"`
exécutent ce qu'ils veulent sur la machine de l'exécutant, avec ses secrets.

## Décision

**Une skill Foyer génère les fichiers d'un projet ; Sluis les valide, puis les
exécute.** Le contrat qui les sépare a les propriétés suivantes.

1. **Un schéma versionné, qui vit dans Sluis, seule source de vérité**, sur le
   modèle d'ADR-005 : contrat matérialisé, pas décrit. La skill le lit, elle ne
   le recopie pas.
2. **Ce que le contrat fixe :**
   - l'arborescence `infrastructure/` :
     - `_shared/charge.yaml` ;
     - `_shared/cluster-profiles/*.yaml` ;
     - `_shared/terraform/modules/<nom>/` ;
     - `{monosite|multisite}/<topologie>/<environnement>/` ;
   - le **sous-ensemble YAML admis** (celui que lit déjà l'analyseur restreint
     de Sluis) ;
   - le workflow **`.github/workflows/tier1-apply.yml`, copié du gabarit** de
     Sluis ;
   - les **noms des check-runs des gates** que Sluis lit avant toute mise en
     ligne.
3. **Les fichiers `.tf` sont des programmes, pas des données.** Le contrat
   impose :
   - une **liste blanche de fournisseurs**, avec **`.terraform.lock.hcl`
     obligatoire** ;
   - l'**interdiction de `local-exec`, de `remote-exec` et de
     `data "external"`**.

   **Sluis le vérifie avant `tofu init`.** Un module qui y manque n'est pas
   initialisé.
4. **`tofu` est lancé avec un environnement vidé**, puis rempli des seules
   variables nécessaires. **Le chemin du module est canonisé** et doit
   descendre de la racine autorisée ; un lien symbolique qui en sort est
   refusé.
5. **Le journal d'audit consigne le commit et l'empreinte des fichiers lus**
   (ADR-022).
6. **Avant chaque dispatch**, Sluis vérifie que la section
   `verification-empreinte` du workflow du dépôt cible est **identique au
   gabarit**. Une section modifiée, même d'un caractère, refuse le dispatch.
7. **Un nouvel outil MCP de lecture, `sluis_verifier_depot`** (famille lecture,
   ADR-030). La skill vérifie son travail avant toute proposition ; **un dépôt
   non conforme est refusé à l'admission**.
8. **Le contrat réserve dès maintenant la place** des fichiers apportés par les
   ADR 014 à 033 : `.sops.yaml` (ADR-016), playbooks Ansible (ADR-032),
   manifestes Flux (ADR-029) et CloudNativePG (ADR-026). Leur format détaillé
   reste un point ouvert.
9. **Ordre de construction.** Le volet sécurité de l'exécution de `tofu`
   (points 3 et 4) est fait **avant la levée de H1** ; le contrat complet,
   **avant l'étape 3** de la feuille de route (ADR-033).

## Alternatives écartées

- **Un contrat décrit en prose** (documentation de la skill) : il dérive du
  code au premier changement, ce qu'ADR-005 a déjà écarté pour le contrat MCP.
- **Le contrat dans Foyer** : la skill deviendrait juge de ce que Sluis
  exécute. Celui qui exécute doit détenir la règle qu'il applique.
- **Valider seulement à l'exécution** : un dépôt apprendrait sa non-conformité
  au moment de déployer, et la skill ne pourrait pas vérifier son travail.
- **Traiter les `.tf` comme des données** : laisserait un fournisseur, un
  `local-exec` ou un `data "external"` exécuter du code arbitraire chez
  l'exécutant, avec ses secrets.

## Conséquences

- **Une frontière testable** : la skill et Sluis lisent le même schéma ; un
  test de contrat, sur le modèle d'ADR-005, prouve qu'ils s'accordent.
- **Des refus plus tôt** : à l'admission et avant `tofu init`, plutôt qu'au
  déploiement.
- **Entretien** : la liste blanche des fournisseurs et le gabarit du workflow
  se tiennent à jour ; un fournisseur nouveau est un changement du contrat,
  donc une nouvelle version.
- **Limite** : la liste blanche et les interdictions bornent ce que les `.tf`
  peuvent faire, pas ce que font les fournisseurs admis eux-mêmes. Un
  fournisseur admis reste du code tiers, épinglé par le fichier de
  verrouillage.
- **Limite** : la vérification de la section `verification-empreinte` prouve
  que ce bloc est intact, pas que le reste du workflow est sain.

## Mesures CyFun 2025 couvertes

Mapping dans `foyer/skills/conformite.md`. Intégrité des logiciels exécutés
(fournisseurs verrouillés, liste blanche) : PR.DS-01.1 I. Exécution de logiciels
non autorisés empêchée (interdictions, refus avant `tofu init`) : PR.PS-05.2 I.
Moindre privilège à l'exécution (environnement vidé) : PR.AA-05.3 ★B. Journal
d'audit des fichiers lus : PR.PS-04.1 ★B.

## Points ouverts

- **Format détaillé des fichiers réservés** : `.sops.yaml`, playbooks Ansible,
  manifestes Flux et CloudNativePG (point 8).
- **Contenu initial de la liste blanche** des fournisseurs, et qui la fait
  évoluer.
- **Règle de version** du schéma : quand un changement est-il incompatible, et
  combien de versions Sluis accepte-t-il à la fois ?
- **Irréversibilité** : l'ADR est contresigné, sa qualification (proposition : oui) reste à confirmer.
