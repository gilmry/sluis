---
livrable: PRD
persona: Product Manager
phase_togaf: B-C — Business + Systèmes d'information
projet: Sluis
version: 0.1.0-draft
genere_le: 2026-08-29
depend_de: 01-product-brief.md
signature_humaine:
  nom:
  role:
  date:
  verdict:
---

# PRD — Sluis

*Livrable du Product Manager · TOGAF Phases B-C. Chaque exigence se rattache à une capacité du Brief (traçabilité).*

## 1. Résumé exécutif

Sluis est un serveur MCP qui expose l'orchestration d'infrastructure OVH comme un domaine typé et gouverné. Il répond à trois questions qu'aucun outil existant ne sait traiter ensemble : *qu'est-ce qui est déclaré et qu'est-ce qui tourne vraiment*, *combien ça tient et combien ça coûte réellement*, et *qui a le droit de changer quoi, avec quelle trace*.

L'archétype est **stateful × API-first**. En conséquence le contrat d'outils MCP est le produit : il est écrit avant le code, matérialisé, et prouvé par des contract tests. La couche Frontend est hors scope, à l'exception du formulaire de connexion technique de l'endpoint d'autorisation OAuth.

Le périmètre est complet et non découpé. Les treize premières exigences sont intégralement de Tier 2 en lecture : elles ne mutent rien et se démontrent en transport stdio local. Les onze suivantes introduisent l'écriture bornée puis le Tier 1, chacune sous les garanties décrites au §7.

## 2. Objectifs produit (mesurables)

| # | Objectif | Mesure | Cible |
|---|---|---|---|
| O1 | Rendre l'infrastructure lisible par un agent sans saisie manuelle | Découverte sur `koprogo/infrastructure` | 3 topologies, 4 environnements, 3 profils, 4 modules |
| O2 | Ne jamais planter sur un environnement incomplet | Binaires absents détectés et rapportés | 6/6 sans panique |
| O3 | Rendre le contrat vérifiable | Schémas `tools/list` prouvés conformes à la désérialisation | 100 % |
| O4 | Remplacer des priors par du mesuré | Constantes `[caler]` de l'abaque calibrées | ≥ 3 |
| O5 | Ne jamais pouvoir muter la production depuis Sluis | Secrets de mutation détenus par Sluis | 0 |
| O6 | Tracer l'intégralité des actions | Appels d'outils journalisés avec empreinte | 100 % |
| O7 | Garantir l'expiration des baux | Baux survivant à leur TTL sur 30 jours | 0 |

## 3. Périmètre — complet / Hors scope

**Périmètre complet, décision du superviseur du 2026-08-29 : pas de découpage MVP.** Les six bounded contexts et les 24 exigences FR-001 à FR-024 sont dans le périmètre de cette gate et sont spécifiés au format long.

L'ordre des sprints reste un ordre de fabrication, pas un découpage de périmètre : rien n'est reporté, rien n'est conditionnel.

**Hors scope, explicitement** :

- Toute interface utilisateur autre que le formulaire de connexion OAuth.
- La multi-location : Sluis sert un superviseur et ses projets, pas des organisations tierces isolées.
- Le remplacement de Terraform, Ansible, Helm ou ArgoCD. Sluis les pilote, ne les réimplémente pas.
- La détention de secrets de mutation de production. C'est une exclusion structurelle, pas un choix de périmètre.
- Un second fournisseur d'infrastructure. Le domaine est agnostique par construction, l'adaptateur viendra si le besoin réel l'exige (YAGNI).

## 4. Glossaire DDD (repris et figé depuis le Brief)

Le glossaire du Brief §8 est **figé** ici et fait loi dans le code : Sluis, Topologie, Environnement, Profil de cluster, Déploiement, Tier, Plan de changement, Jeton de changement, Bail de bac à sable, Chien de garde, Campagne de charge, Mesure de capacité, Prior, Convergence, Gate du plancher, Passerelle d'approbation.

Le mapping terme → type Rust est un livrable de l'Architecte.

## 5. Bounded contexts → modules

| BC | Module Rust | Dépendances autorisées |
|---|---|---|
| BC1 Inventaire | `domain::inventory` + `application::use_cases::inventory` | aucune vers les autres BC |
| BC2 Autorisation | `domain::authorization` | aucune |
| BC3 Bac à sable | `domain::sandbox` | BC2 (pour le tier) |
| BC4 Capacité | `domain::capacity` | BC3 (une campagne vit dans un bail) |
| BC5 Exécution | `application::use_cases::execution` | BC1, BC2 |
| BC6 Accès | `domain::access` | aucune |

## 6. Exigences fonctionnelles

### Module BC1 — Inventaire

#### FR-001 — Diagnostiquer l'environnement d'exécution
- **En tant que** agent exécutant **je veux** connaître les binaires et identifiants disponibles **afin de** savoir quels outils je peux réellement utiliser avant d'échouer.
- **Critères d'acceptation (Gherkin)** :
  - `Étant donné une machine sans terraform ni helm, Quand j'appelle sluis_doctor, Alors je reçois un rapport listant chaque binaire avec son statut présent ou absent, et le processus ne panique pas.`
  - `Étant donné des identifiants OVH absents, Quand j'appelle sluis_doctor, Alors le rapport indique l'absence sans jamais révéler de valeur partielle.`
- **Classes de tests** :
  - `@happy` — tous les binaires présents, rapport complet et exact
  - `@negative` — binaire au `PATH` mais non exécutable, rapporté comme inutilisable et non comme présent
  - `@edge` — `PATH` vide, aucun binaire, rapport intégralement négatif sans panique
  - `@security` — aucun fragment d'identifiant, ni longueur, ni préfixe, ne figure dans la sortie
- **Capacité du Brief** : §7 C1

#### FR-002 — Découvrir la matrice d'infrastructure déclarée
- **En tant que** agent exécutant **je veux** obtenir topologies, environnements et profils d'un dépôt **afin de** raisonner sur l'existant sans qu'on me le décrive.
- **Critères d'acceptation (Gherkin)** :
  - `Étant donné le dépôt koprogo, Quand j'appelle sluis_inventory sur son dossier infrastructure, Alors j'obtiens 3 topologies, 4 environnements et 3 profils de cluster.`
  - `Étant donné un dépôt sans dossier infrastructure, Quand j'appelle sluis_inventory, Alors j'obtiens une erreur typée nommant le chemin attendu, pas une liste vide.`
- **Classes de tests** :
  - `@happy` — arborescence conforme, matrice complète et correctement croisée
  - `@negative` — chemin inexistant, chemin vers un fichier, dossier illisible : `AppError` typée distincte à chaque fois
  - `@edge` — topologie déclarée sans aucun environnement, environnement inconnu du domaine, arborescence partielle
  - `@security` — un chemin remontant hors de la racine autorisée est refusé ; aucun lien symbolique n'est suivi hors racine
- **Capacité du Brief** : §7 C1

#### FR-003 — Décrire les profils de cluster
- **En tant que** agent exécutant **je veux** le détail du contrat Day 1 / Day 2 de chaque profil **afin de** savoir quelles valeurs seront injectées à un déploiement.
- **Critères d'acceptation (Gherkin)** :
  - `Étant donné le profil k3s-self-hosted, Quand j'appelle sluis_cluster_profiles, Alors j'obtiens sa classe de stockage local-path, son ingress traefik, son backend de secrets sealed-secrets et son préréglage de ressources.`
- **Classes de tests** :
  - `@happy` — les 3 profils décrits avec leurs champs de contrat
  - `@negative` — YAML malformé, clé de contrat manquante : erreur typée nommant le fichier et la clé
  - `@edge` — profil déclarant un champ inconnu, profil vide, deux profils de même nom
  - `@security` — une valeur ressemblant à un secret dans un profil n'est jamais rendue en clair
- **Capacité du Brief** : §7 C1

#### FR-004 — Croiser déclaré et réel
- **En tant que** superviseur **je veux** voir l'écart entre l'infrastructure déclarée et celle qui tourne **afin de** décider en connaissance de cause.
- **Critères d'acceptation (Gherkin)** :
  - `Étant donné une topologie déclarant deux instances et un projet OVH n'en portant qu'une, Quand j'appelle sluis_drift, Alors l'instance manquante est rapportée comme un écart typé.`
- **Classes de tests** :
  - `@happy` — déclaré et réel identiques, écart vide
  - `@negative` — API OVH indisponible : écart non calculable, erreur explicite, jamais un écart vide trompeur
  - `@edge` — ressource présente en réel et absente en déclaré, et l'inverse ; ressource en cours de création
  - `@security` — l'écart ne divulgue aucune ressource d'un projet hors liste d'autorisation
- **Capacité du Brief** : §7 C3

### Module BC5 — Exécution (lecture seule)

#### FR-005 — Lister les projets OVH autorisés
- **En tant que** agent exécutant **je veux** la liste des projets OVH visibles **afin de** cibler mes lectures.
- **Critères d'acceptation (Gherkin)** :
  - `Étant donné une liste d'autorisation de deux projets et un compte OVH en portant cinq, Quand j'appelle ovh_projects_list, Alors j'obtiens exactement les deux projets autorisés.`
- **Classes de tests** :
  - `@happy` — projets autorisés retournés avec identifiant et description
  - `@negative` — identifiants invalides : erreur d'authentification typée, jamais une liste vide
  - `@edge` — liste d'autorisation vide (retour vide légitime), projet autorisé mais inexistant chez OVH
  - `@security` — **un projet hors liste n'apparaît jamais**, y compris si l'appelant en fournit l'identifiant explicitement
- **Capacité du Brief** : §7 C2

#### FR-006 — Lister et décrire les instances
- **En tant que** agent exécutant **je veux** les instances d'un projet avec leur état **afin de** connaître l'infrastructure réelle.
- **Critères d'acceptation (Gherkin)** :
  - `Étant donné un projet autorisé portant trois instances, Quand j'appelle ovh_instances_list, Alors j'obtiens trois instances avec identifiant, nom, flavor, région et état.`
- **Classes de tests** :
  - `@happy` — instances listées avec tous leurs champs
  - `@negative` — projet inexistant, instance inexistante : erreur typée distincte
  - `@edge` — projet sans instance, instance en cours de suppression, pagination de l'API OVH sur plus d'une page
  - `@security` — projet hors liste refusé avant tout appel réseau
- **Capacité du Brief** : §7 C2

#### FR-007 — Lire les coûts courants
- **En tant que** superviseur **je veux** la consommation en cours d'un projet **afin de** confronter le coût réel au modèle.
- **Critères d'acceptation (Gherkin)** :
  - `Étant donné un projet autorisé, Quand j'appelle ovh_costs_current, Alors j'obtiens le montant courant, la devise et la période couverte.`
- **Classes de tests** :
  - `@happy` — coût courant retourné avec sa période
  - `@negative` — projet sans données de facturation : absence explicite, jamais un zéro trompeur
  - `@edge` — période à cheval sur deux mois, montant nul légitime, devise inattendue
  - `@security` — projet hors liste refusé
- **Capacité du Brief** : §7 C2, C6

#### FR-008 — Lire les enregistrements DNS
- **En tant que** agent exécutant **je veux** les enregistrements DNS d'une zone **afin de** vérifier le routage d'un déploiement.
- **Classes de tests** :
  - `@happy` — enregistrements listés par type et sous-domaine
  - `@negative` — zone inexistante : erreur typée
  - `@edge` — zone vide, enregistrement avec TTL nul, caractères internationalisés
  - `@security` — une zone de production n'est jamais modifiable par les outils de lecture, seulement lisible
- **Capacité du Brief** : §7 C2

#### FR-009 — Produire un plan Terraform sans l'appliquer
- **En tant que** agent exécutant **je veux** le plan d'un module **afin de** connaître l'écart sans rien changer.
- **Critères d'acceptation (Gherkin)** :
  - `Étant donné un module Terraform valide, Quand j'appelle tf_plan, Alors j'obtiens la liste structurée des créations, modifications et destructions, et aucune ressource n'a été modifiée.`
  - `Étant donné que terraform est absent de la machine, Quand j'appelle tf_plan, Alors j'obtiens une erreur nommant le binaire manquant, pas une trace d'exécution.`
- **Classes de tests** :
  - `@happy` — plan sans changement, puis plan avec changements, tous deux correctement structurés
  - `@negative` — binaire absent, module invalide, état verrouillé : trois erreurs typées distinctes
  - `@edge` — plan vide, plan à plusieurs centaines de ressources, sortie tronquée par le moteur
  - `@security` — **aucun argument fourni par l'appelant n'atteint un shell** ; une tentative d'injection dans un nom de module est refusée à l'admission, pas échappée
- **Capacité du Brief** : §7 C3

#### FR-010 — Lire les statuts Helm
- **Classes de tests** :
  - `@happy` — statut et historique d'une release
  - `@negative` — binaire absent, release inconnue, kubeconfig absent
  - `@edge` — release en échec, historique vide, révision unique
  - `@security` — aucun argument interpolé dans un shell ; le contenu du kubeconfig n'est jamais rendu
- **Capacité du Brief** : §7 C3

#### FR-011 — Rendre une configuration Kustomize
- **Classes de tests** :
  - `@happy` — rendu d'une base et d'un overlay d'environnement
  - `@negative` — binaire absent, kustomization invalide
  - `@edge` — overlay sans patch, ressource dupliquée
  - `@security` — le rendu ne divulgue aucune valeur de Secret Kubernetes ; les champs sensibles sont masqués
- **Capacité du Brief** : §7 C3

#### FR-012 — Lire le statut ArgoCD
- **Classes de tests** :
  - `@happy` — statut de synchronisation et de santé d'une application
  - `@negative` — binaire absent, application inconnue, serveur injoignable
  - `@edge` — application en cours de synchronisation, application dégradée, ApplicationSet sans enfant
  - `@security` — le jeton ArgoCD n'apparaît jamais en sortie ni dans les journaux
- **Capacité du Brief** : §7 C3

### Module BC6 — Accès

#### FR-013 — Journaliser toute action
- **En tant que** platform engineer **je veux** un journal inaltérable de tous les appels **afin de** pouvoir auditer a posteriori.
- **Critères d'acceptation (Gherkin)** :
  - `Étant donné un appel d'outil quelconque, Quand il se termine, Alors une entrée horodatée est ajoutée au journal avec l'outil, le tier, le résultat et l'empreinte des arguments.`
  - `Étant donné un appel qui échoue, Quand il se termine, Alors une entrée est tout de même ajoutée.`
- **Classes de tests** :
  - `@happy` — succès et échec journalisés à l'identique
  - `@negative` — journal non inscriptible : l'appel échoue plutôt que de s'exécuter sans trace
  - `@edge` — écritures concurrentes, rotation de fichier, entrée volumineuse
  - `@security` — **le journal ne contient aucun secret** ; une modification ou une suppression n'est exposée par aucun chemin
- **Capacité du Brief** : §7 C10

### Module BC3 — Bac à sable

#### FR-014 — Louer une infrastructure éphémère bornée
- **En tant que** agent exécutant **je veux** louer une infrastructure jetable **afin de** y conduire une campagne sans jamais toucher à un projet de production.
- **Critères d'acceptation (Gherkin)** :
  - `Étant donné une demande de bail sans TTL, Quand je la soumets, Alors elle est refusée : un bail sans échéance n'existe pas.`
  - `Étant donné un identifiant de projet portant de la production, Quand je le passe à une demande de bail, Alors elle est refusée avant tout appel réseau.`
  - `Étant donné un plafond de dépense de 20 euros et une estimation à 35, Quand je soumets la demande, Alors elle est refusée à l'admission.`
- **Classes de tests (4×N)** :
  - `@happy` — bail nominal avec TTL et plafond, ressources provisionnées dans le projet de bac à sable
  - `@negative` — TTL absent, plafond absent, estimation au-dessus du plafond, quota OVH atteint : quatre erreurs typées distinctes
  - `@edge` — TTL minimal, TTL maximal, plafond à l'euro près, deux baux concurrents sur le même projet
  - `@security` — **les six conditions d'ADR-007 vérifiées chacune par un test dédié** : projet sur allowlist de bac à sable, TTL présent, plafond présent, aucune donnée de production, aucun DNS de production, journalisation effective. La disjonction des deux listes de projets est prouvée.
- **Capacité du Brief rattachée** : §7 C4

#### FR-015 — Détruire un bail, y compris après disparition du demandeur
- **En tant que** superviseur **je veux** qu'un bail expiré soit détruit quoi qu'il arrive **afin de** ne jamais découvrir une facture ouverte.
- **Critères d'acceptation (Gherkin)** :
  - `Étant donné une campagne qui panique en cours de palier, Quand le processus se termine, Alors le bail est tout de même détruit.`
  - `Étant donné un processus principal tué par SIGKILL, Quand le TTL expire, Alors le chien de garde détruit le bail sans lui.`
  - `Étant donné une destruction qui échoue côté API, Quand le chien de garde réessaie, Alors il alerte plutôt que d'abandonner silencieusement.`
- **Classes de tests (4×N)** :
  - `@happy` — destruction à l'échéance nominale, ressources effectivement libérées
  - `@negative` — API en erreur, bail déjà détruit, ressource verrouillée : réessai puis alerte, jamais d'abandon
  - `@edge` — TTL expiré pendant une panne du chien de garde, destruction concurrente, bail détruit manuellement entre-temps
  - `@security` — **le chien de garde survit à l'arrêt du processus demandeur** ; c'est le test le plus important du bounded context
- **Capacité du Brief rattachée** : §7 C4

#### FR-016 — Prouver la convergence par ré-application
- **En tant que** agent exécutant **je veux** prouver qu'un ré-apply ne produit aucun écart **afin de** que l'idempotence soit démontrée et non supposée.
- **Critères d'acceptation (Gherkin)** :
  - `Étant donné une infrastructure fraîchement provisionnée, Quand je ré-applique la déclaration, Alors aucun écart n'est produit et la convergence est prouvée.`
  - `Étant donné un écart qui persiste après trois tours, Quand la boucle se termine, Alors elle échoue explicitement plutôt que de boucler indéfiniment.`
- **Classes de tests (4×N)** :
  - `@happy` — convergence au premier ré-apply, preuve horodatée
  - `@negative` — écart persistant, moteur absent, état verrouillé : erreurs typées distinctes, aucune boucle infinie
  - `@edge` — écart dû à une valeur générée (horodatage, identifiant), convergence au deuxième tour, déclaration vide
  - `@security` — la preuve de convergence ne divulgue aucune valeur d'état sensible ni aucun secret présent dans l'état Terraform
- **Capacité du Brief rattachée** : §7 C4

### Module BC4 — Capacité

#### FR-017 — Conduire une campagne de charge en paliers
- **En tant que** superviseur **je veux** dérouler un escalier de charge sur un déploiement **afin de** observer où il sature, pour de vrai.
- **Critères d'acceptation (Gherkin)** :
  - `Étant donné un bail actif et une cible déployée, Quand je lance une campagne, Alors les sept paliers s'exécutent dans l'ordre du warmup au soak.`
  - `Étant donné que le rate limiting est actif sur la cible, Quand je lance une campagne, Alors elle est refusée à l'admission car les résultats seraient faussés.`
  - `Étant donné une cible qui n'est pas dans un bail, Quand je lance une campagne, Alors elle est refusée.`
- **Classes de tests (4×N)** :
  - `@happy` — sept paliers dans l'ordre, résultats collectés à chacun
  - `@negative` — `wrk` absent, cible injoignable, bail expiré en cours de campagne : refus à l'admission quand c'est possible, arrêt propre sinon
  - `@edge` — palier échouant à mi-parcours, cible saturée dès le warmup, campagne interrompue par l'utilisateur, palier à zéro connexion
  - `@security` — refus si le rate limiting est actif, refus si la cible est hors bail, **aucune donnée de production dans les jeux d'essai**
- **Capacité du Brief rattachée** : §7 C5

#### FR-018 — Collecter les mesures avec leur provenance
- **En tant que** superviseur **je veux** que toute valeur porte sa provenance **afin de** ne jamais confondre ce qui est observé et ce qui est déduit.
- **Critères d'acceptation (Gherkin)** :
  - `Étant donné une valeur dérivée par calcul et non observée, Quand je la consigne, Alors sa provenance est Assumed et jamais Measured.`
  - `Étant donné un P99 inférieur à la médiane, Quand je consigne la mesure, Alors elle est rejetée comme incohérente.`
- **Classes de tests (4×N)** :
  - `@happy` — P99, débit, RSS, jeu chaud, pression, coût réel, tous datés et attribués à un palier
  - `@negative` — mesure incohérente, échantillon vide, unité manquante : rejet typé
  - `@edge` — mesure à zéro légitime, échantillon insuffisant (marqué comme tel), mesure sur un palier interrompu
  - `@security` — aucune donnée de production ni identifiant dans les mesures ; jeux synthétiques exclusivement
- **Capacité du Brief rattachée** : §7 C5

#### FR-019 — Produire un rapport de recalage des priors
- **En tant que** superviseur **je veux** un rapport qui propose de remplacer des `[caler]` par du mesuré **afin de** que mes arbitrages de palier cessent de reposer sur l'intuition.
- **Critères d'acceptation (Gherkin)** :
  - `Étant donné une campagne complète sur une topologie K3s, Quand je génère le rapport, Alors il propose une valeur mesurée pour le surcoût de control-plane et cite ses conditions de mesure.`
  - `Étant donné une campagne incomplète, Quand je demande le rapport, Alors il est refusé plutôt que produit partiel.`
- **Classes de tests (4×N)** :
  - `@happy` — rapport citant prior, valeur mesurée, écart et conditions de mesure
  - `@negative` — campagne incomplète, mesures manquantes, prior inconnu : refus explicite
  - `@edge` — mesure identique au prior, mesure aberrante signalée, campagne sur une topologie sans prior existant
  - `@security` — le rapport **distingue explicitement mesuré et supposé** (§9 de l'abaque) ; aucune extrapolation n'est présentée comme une mesure
- **Capacité du Brief rattachée** : §7 C6

### Module BC2 — Autorisation

#### FR-020 — Produire un plan de changement empreinté
- **En tant que** agent exécutant **je veux** décrire une mutation sans l'exécuter **afin de** la soumettre à décision.
- **Critères d'acceptation (Gherkin)** :
  - `Étant donné une action visant production, Quand je construis un plan en Tier 2, Alors la construction échoue avec TierViolation.`
  - `Étant donné deux plans différant d'un seul champ, Quand je calcule leurs empreintes, Alors elles diffèrent.`
- **Classes de tests (4×N)** :
  - `@happy` — plan Tier 2 sur `dev`, plan Tier 1 sur `production`, empreinte stable et reproductible
  - `@negative` — Tier 2 sur `production` refusé ; aucun constructeur alternatif n'existe
  - `@edge` — action sans environnement cible, action portant sur plusieurs environnements, plan vide
  - `@security` — **aucune ressource n'est modifiée par la production d'un plan** ; l'empreinte ne fuit aucun secret présent dans le contenu du plan
- **Capacité du Brief rattachée** : §7 C7

#### FR-021 — Soumettre un plan à la passerelle d'approbation
- **En tant que** superviseur **je veux** décider dans une interface qui trace **afin de** que mon approbation soit un fait auditable, pas un message.
- **Critères d'acceptation (Gherkin)** :
  - `Étant donné un plan Tier 1, Quand je le soumets, Alors un workflow_dispatch est déclenché et le job reste bloqué en attente de relecteur.`
  - `Étant donné un dépôt sans environnement protégé, Quand je soumets un plan Tier 1, Alors la soumission est refusée plutôt qu'exécutée sans garde.`
  - `Étant donné un refus du relecteur, Quand j'interroge le run, Alors le plan est rapporté refusé et rien n'a été muté.`
- **Classes de tests (4×N)** :
  - `@happy` — soumission, blocage, approbation, exécution par le job, compte rendu à l'agent
  - `@negative` — dépôt sans environnement protégé, workflow absent, jeton de déclenchement invalide : trois refus distincts
  - `@edge` — approbation après expiration du jeton de changement, run annulé, relecteur indisponible, deux soumissions du même plan
  - `@security` — **Sluis ne détient aucun secret d'infrastructure** ; un test prouve qu'une tentative de mutation locale échoue faute d'identifiants
- **Capacité du Brief rattachée** : §7 C8

#### FR-022 — Consommer un jeton de changement exactement une fois
- **En tant que** superviseur **je veux** qu'une approbation ne serve qu'une fois **afin de** qu'un plan approuvé hier ne réexécute rien demain.
- **Critères d'acceptation (Gherkin)** :
  - `Étant donné un jeton déjà consommé, Quand je tente de le consommer à nouveau, Alors le code ne compile pas car consume prend self par valeur.`
  - `Étant donné un jeton émis pour une empreinte A, Quand je le présente pour un plan d'empreinte B, Alors il est rejeté.`
  - `Étant donné un jeton expiré, Quand je le présente, Alors il est rejeté avant tout effet.`
- **Classes de tests (4×N)** :
  - `@happy` — consommation nominale rendant un `ConsumedToken`
  - `@negative` — empreinte non concordante, jeton expiré, jeton inconnu : trois erreurs distinctes
  - `@edge` — expiration à la seconde près avec `Clock` figé, jeton émis dans le futur, consommation concurrente du même jeton
  - `@security` — rejeu impossible **par typage et vérifié côté persistance** pour le cas distribué ; la contrainte d'unicité est portée par la base, pas seulement par le code
- **Capacité du Brief rattachée** : §7 C8

### Module BC6 — Accès distant

#### FR-023 — Authentifier un client MCP distant (OAuth 2.1 + PKCE)
- **En tant que** superviseur **je veux** un vrai bouton Connect **afin de** ne pas coller à la main un jeton qui expire.
- **Critères d'acceptation (Gherkin)** :
  - `Étant donné un client MCP inconnu, Quand il lit le document de découverte puis s'enregistre, Alors il obtient un client_id sans intervention humaine.`
  - `Étant donné un code_challenge_method plain, Quand le client tente de l'utiliser, Alors la demande est refusée.`
  - `Étant donné un code d'autorisation déjà échangé, Quand il est présenté une seconde fois, Alors il est rejeté.`
  - `Étant donné un refresh token déjà utilisé une fois, Quand il est rejoué, Alors il est rejeté car la rotation l'a révoqué.`
- **Classes de tests (4×N)** :
  - `@happy` — découverte, enregistrement dynamique, autorisation, échange de code, rafraîchissement avec rotation
  - `@negative` — `code_verifier` erroné, code expiré, `client_id` inconnu, `redirect_uri` non enregistrée : quatre refus distincts
  - `@edge` — deux échanges concurrents du même code, rafraîchissement à l'expiration exacte, `redirect_uri` contenant déjà une chaîne de requête
  - `@security` — **`plain` refusé** (exigence OAuth 2.1), code à usage unique, **rotation inconditionnelle** du refresh token même si le reste de l'échange échoue, `redirect_uri` validée **avant tout rendu ou redirection**, refresh token persisté en hash SHA-256 uniquement
- **Capacité du Brief rattachée** : §7 C8

### Module BC5 — Mise en ligne

#### FR-024 — Mettre un projet en ligne après vérification des gates
- **En tant que** superviseur **je veux** qu'une mise en ligne ne parte jamais sur des gates rouges **afin de** que le déploiement ne soit pas le moment où l'on découvre un problème connu.
- **Critères d'acceptation (Gherkin)** :
  - `Étant donné une gate du plancher au rouge, Quand je demande une mise en ligne, Alors elle est refusée avant même d'être soumise à approbation.`
  - `Étant donné un archétype stateful et une migration sans fichier de retour, Quand je demande une mise en ligne, Alors elle est refusée.`
  - `Étant donné des tests post-déploiement au rouge, Quand le déploiement se termine, Alors un rollback automatique est déclenché.`
- **Classes de tests (4×N)** :
  - `@happy` — gates vertes, plan Tier 1, approbation, déploiement, tests post-déploiement verts
  - `@negative` — gate rouge, approbation refusée, déploiement en échec : refus ou rollback, jamais un état non vérifié
  - `@edge` — tests post-déploiement rouges (rollback), rollback lui-même en échec (alerte), déploiement partiel
  - `@security` — secrets, SBOM CycloneDX, scan d'image et fichier de retour de migration **tous les quatre** vérifiés ; l'absence d'un seul bloque
- **Capacité du Brief rattachée** : §7 C9

## 7. Exigences non-fonctionnelles

| # | Exigence | Mesure | Cible |
|---|---|---|---|
| NFR-01 | **Aucun secret de mutation de production détenu** | Revue de configuration et de code | 0, propriété structurelle |
| NFR-02 | **Aucun argument d'appelant n'atteint un shell** | Revue des adaptateurs de processus | 0 interpolation, allowlist d'exécutables |
| NFR-03 | Étanchéité des secrets en sortie | Test de non-régression sur tous les outils | 0 fuite |
| NFR-04 | Latence des outils de lecture locale (inventaire, profils) | P99 hors appel réseau | ≤ 200 ms |
| NFR-05 | Empreinte mémoire du serveur au repos | RSS | ≤ 64 Mo |
| NFR-06 | Aucun test de CI ne dépend d'un binaire d'infrastructure | Exécution de la CI sur machine nue | 100 % vert |
| NFR-07 | Aucun appel réseau réel en CI | Adaptateur OVH sous `wiremock` | 100 % |
| NFR-08 | Robustesse à l'absence d'outillage | `sluis_doctor` sur machine nue | 0 panique |
| NFR-09 | Disponibilité du serveur distant | Redéploiement GitOps idempotent toutes les 5 min | pas d'interruption sur no-op |
| NFR-10 | Conformité RGPD | Aucune donnée personnelle traitée | 0 donnée personnelle hors identifiants de compte |
| NFR-11 | Souveraineté | Région OVH | EU-West exclusivement |
| NFR-12 | Sobriété | Binaire unique, sans runtime | 1 artefact, image distroless |
| NFR-13 | Traçabilité | Actions Tier 1 avec empreinte, approbateur, horodatage | 100 % |
| NFR-14 | Expiration garantie des baux | Baux survivants à leur TTL | 0 |

## 8. Frontend UX

**Hors scope** au titre de l'archétype API-first, à une exception près.

L'endpoint `GET /oauth/authorize` rend un formulaire HTML de connexion (email et mot de passe, paramètres OAuth en champs cachés). Ce n'est pas une couche Frontend au sens des sept couches : c'est un détail de l'adaptateur d'authentification, traité comme tel dans Elevia et derniere-chance. Aucune page Astro, aucun îlot Svelte, aucun i18n.

## 9bis. Contrat API · *de premier rang, écrit avant le code*

**Le contrat est le produit.** Il a deux faces, toutes deux versionnées et testées.

**Face MCP.** La sortie de `tools/list` est la source de vérité : pour chaque outil, un nom, une description et un `inputSchema` en JSON Schema. Le contrat impose que :

1. tout outil exposé possède un schéma déclaré, sans exception ;
2. le type Rust de désérialisation de `tools/call` soit annoté `deny_unknown_fields` ;
3. un contract test prouve, pour chaque outil, que le schéma déclaré et le type de désérialisation acceptent et refusent exactement les mêmes charges utiles ;
4. la version de protocole MCP annoncée par `initialize` soit explicite et testée.

Le point 3 est le cœur : c'est ce qui distingue un contrat matérialisé d'un contrat décrit. `contrat-api.md` rappelle qu'un contrat non matérialisé a déjà coûté un NO-GO en production.

**Face HTTP** (FR-023). Cinq endpoints, conformes RFC 8414 et RFC 7591 :

| Méthode | Chemin | Rôle |
|---|---|---|
| GET | `/.well-known/oauth-authorization-server` | Découverte, `code_challenge_methods_supported: ["S256"]` |
| POST | `/oauth/register` | Enregistrement dynamique, clients publics uniquement |
| GET | `/oauth/authorize` | Formulaire, après validation stricte de `redirect_uri` |
| POST | `/oauth/authorize` | Revalidation, vérification des identifiants, code à usage unique |
| POST | `/oauth/token` | `authorization_code` avec vérification PKCE, et `refresh_token` avec rotation inconditionnelle |
| POST | `/mcp` | JSON-RPC 2.0, Streamable HTTP, `Authorization: Bearer` |

**Codes d'erreur** : JSON-RPC standard côté MCP (`-32700` parse, `-32600` requête invalide, `-32601` méthode inconnue, `-32602` paramètres invalides, `-32603` erreur interne). HTTP standard côté OAuth, avec refus d'affichage plutôt que redirection tant que `redirect_uri` n'est pas validé.

**Versioning** : une rupture de contrat est un **point irréversible** au sens de la méthode. Elle exige un ADR et une validation humaine.

**Scopes** : `sluis:read`, `sluis:sandbox`, `sluis:propose`. Aucun scope ne permet de muter la production, par construction.

## 9. Documentation Vivante · *API-first : contract tests*

**Flux critiques à couvrir** :

1. Découverte complète : `initialize` → `tools/list` → `sluis_inventory` sur un dépôt réel, sans saisie manuelle. C'est le critère du jalon de fabrication (§13.1).
2. Dégradation propre : machine sans aucun binaire d'infrastructure, tous les outils répondent une erreur typée, aucune panique.
3. Cycle de vie complet d'un bail : location, campagne, destruction, y compris sous panique.
4. Cycle d'approbation : plan → dispatch → blocage → approbation → exécution → compte rendu.
5. Flow OAuth complet, dont le rejet d'un mauvais `code_verifier` et d'un refresh token rejoué.

**Alignement BDD ↔ contract tests** : chaque flux critique est un scénario Gherkin, et chaque outil du contrat a son contract test. Les deux vivent dans `tests/features/` et `tests/contract/`.

## 10. Modèle de données (entités DDD → tables PostgreSQL) · *stateful*

Le socle de lecture n'a **aucune base de données** : le journal d'audit est un fichier JSONL append-only, l'inventaire est dérivé du système de fichiers et de l'API OVH. PostgreSQL n'apparaît qu'avec FR-023.

| Entité | Table | Notes |
|---|---|---|
| Client OAuth | `oauth_clients` | `client_id`, nom, `redirect_uris[]` — clients publics, jamais de secret |
| Code d'autorisation | `oauth_authorization_codes` | usage unique, 10 min, lié à `client_id` + `redirect_uri` + `code_challenge` |
| Jeton de rafraîchissement | `oauth_refresh_tokens` | **hash SHA-256 en clé primaire**, jamais le jeton, révocable |
| Bail de bac à sable | `sandbox_leases` | TTL et plafond **non nullables**, index sur l'échéance pour le chien de garde |
| Plan de changement | `change_plans` | empreinte, tier, état ; immuable après création |
| Jeton de changement | `change_tokens` | lié à une empreinte, expirable, **consommé exactement une fois** |
| Mesure de capacité | `capacity_measurements` | provenance `mesure` ou `supposition` **non nullable** |

Trois migrations sont des **points irréversibles** exigeant validation humaine, et chacune livre son fichier de retour `*.down.sql` au titre du plancher de `gates.md` pour l'archétype stateful.

## 11. Intégrations externes

| Système | Usage | Mode d'échec accepté |
|---|---|---|
| API OVHcloud (`eu.api.ovh.com/1.0`) | Projets, instances, coûts, DNS | Erreur typée, jamais de résultat vide trompeur |
| API OpenStack (via Terraform) | Compute, réseau, stockage | Uniquement à travers le moteur Terraform |
| GitHub Actions + environnements protégés | **Passerelle d'approbation Tier 1** | Sans elle, aucune action Tier 1 n'est possible : c'est voulu |
| GHCR | Distribution de l'image serveur | Le serveur ne construit rien |
| Traefik (`ecosolva-web`) | Terminaison TLS et routage | Réseau externe non possédé par le dépôt |
| `wrk` | Moteur de charge | Absent : campagne refusée à l'admission, pas en cours de route |
| Terraform, Ansible, Helm, Kustomize, ArgoCD | Moteurs d'exécution | Absents : erreur typée nommant le binaire |

## 12. Contraintes et hypothèses

**Contraintes** : celles du Brief §14, reprises sans altération.

**Hypothèses à valider** :

- H1 — Un projet OVH dédié aux bacs à sable peut être créé, distinct de tout projet portant de la production. **Bloquant pour FR-014**.
- H2 — Les dépôts cibles peuvent recevoir un workflow lié à un environnement GitHub protégé. **Bloquant pour FR-021**.
- H3 — Le serveur hébergeant n8n peut accueillir un service supplémentaire sur `ecosolva-web`, et son nom d'hôte reste à confirmer (`.org` par convention existante, `.com` mentionné par le superviseur).
- H4 — L'API OVH expose une granularité de coût suffisante par projet pour alimenter le recalage.
- H5 — Le corpus `wrk` de KoproGo est réutilisable tel quel sur un déploiement Sluis.

## 13. Critères de succès

### 13.1 — Jalon de fabrication intermédiaire (lecture seule)

Sur une machine dépourvue de tout binaire d'infrastructure :

1. `make ci` est vert, y compris clippy en `-D warnings`, gitleaks et le SBOM CycloneDX ;
2. Sluis déclaré dans un `.mcp.json` répond à `initialize` puis `tools/list` ;
3. `sluis_inventory` pointé sur `koprogo/infrastructure` ressort **3 topologies, 4 environnements, 3 profils de cluster et 4 modules Terraform, sans aucune saisie manuelle** ;
4. `sluis_doctor` rapporte les 6 binaires absents sans panique ;
5. les contract tests prouvent la conformité de chaque schéma déclaré à sa désérialisation ;
6. tous les tests `@security` du socle de lecture sont au vert, dont le refus d'un projet hors liste et l'absence de tout secret en sortie ;
7. le journal d'audit contient une entrée par appel, succès comme échec.

Ce jalon n'est pas une livraison : c'est le point où la boucle de fabrication est prouvée. Le périmètre n'est pas atteint tant que le §13.2 ne l'est pas.

### 13.2 — Périmètre complet

1. les 24 exigences FR-001 à FR-024 sont livrées, chacune avec ses quatre classes de tests au vert ;
2. une campagne de charge complète s'exécute de bout en bout sur une infrastructure éphémère, et son bail est détruit y compris sous panique ;
3. au moins trois constantes `[caler]` de l'abaque coût/capacité sont remplacées par du mesuré, provenance à l'appui ;
4. une action Tier 1 traverse la passerelle d'approbation de bout en bout : plan, blocage, approbation humaine, exécution, compte rendu ;
5. un client MCP distant se connecte par le flow OAuth complet, et les rejeux de code et de refresh token sont prouvés rejetés ;
6. une mise en ligne est refusée sur gate rouge, puis réussie sur gates vertes, avec rollback prouvé sur tests post-déploiement rouges ;
7. le service tourne sur le serveur ecosolva derrière Traefik, avec un redéploiement GitOps prouvé idempotent.

---

*Dérivé du Manifeste Maury (CC BY-SA 4.0). Source : [`github.com/gilmry/manifest`](https://github.com/gilmry/manifest).*
