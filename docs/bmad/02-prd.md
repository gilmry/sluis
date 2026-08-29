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

Le MVP (FR-001 à FR-013) est intégralement de Tier 2 en lecture : il ne mute rien, ne nécessite aucun identifiant de mutation, et se démontre en transport stdio local.

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

## 3. Périmètre — MVP / Hors scope

**MVP** : BC1 Inventaire, BC5 Exécution en lecture seule, BC6 Accès réduit au journal d'audit. Transport stdio. FR-001 à FR-013.

**Post-MVP** : BC2 Autorisation, BC3 Bac à sable, BC4 Capacité, BC6 complet avec OAuth. FR-014 à FR-024.

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

### Module BC5 — Exécution (lecture seule au MVP)

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
  - `@security` — une zone de production n'est jamais modifiable depuis le MVP, seulement lisible
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

### Post-MVP — résumé des exigences

| FR | Titre | BC | Tier | Capacité |
|---|---|---|---|---|
| FR-014 | Louer un bac à sable borné (TTL + plafond obligatoires) | BC3 | 2 borné | C4 |
| FR-015 | Détruire un bail, y compris après panique du demandeur | BC3 | 2 borné | C4 |
| FR-016 | Prouver la convergence par ré-application sans écart | BC5 | 2 borné | C4 |
| FR-017 | Conduire une campagne de charge en paliers | BC4 | 2 borné | C5 |
| FR-018 | Collecter les mesures de capacité avec leur provenance | BC4 | 2 | C5 |
| FR-019 | Produire un rapport de recalage des priors | BC4 | 2 | C6 |
| FR-020 | Produire un plan de changement empreinté | BC2 | 2 | C7 |
| FR-021 | Soumettre un plan à la passerelle d'approbation | BC2 | 1 | C8 |
| FR-022 | Consommer un jeton d'approbation exactement une fois | BC2 | 1 | C8 |
| FR-023 | Authentifier un client MCP distant (OAuth 2.1 + PKCE) | BC6 | 1 | C8 |
| FR-024 | Mettre un projet en ligne après vérification des gates | BC5 | 1 | C9 |

Chacune sera détaillée au format complet (Gherkin + 4 classes) avant son sprint. Leurs classes `@security` minimales sont déjà fixées au Brief §16 et reprises au §7 ci-dessous.

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

**Face HTTP** (post-MVP, FR-023). Cinq endpoints, conformes RFC 8414 et RFC 7591 :

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

1. Découverte complète : `initialize` → `tools/list` → `sluis_inventory` sur un dépôt réel, sans saisie manuelle. C'est le critère d'acceptation du MVP.
2. Dégradation propre : machine sans aucun binaire d'infrastructure, tous les outils répondent une erreur typée, aucune panique.
3. Cycle de vie complet d'un bail : location, campagne, destruction, y compris sous panique.
4. Cycle d'approbation : plan → dispatch → blocage → approbation → exécution → compte rendu.
5. Flow OAuth complet, dont le rejet d'un mauvais `code_verifier` et d'un refresh token rejoué.

**Alignement BDD ↔ contract tests** : chaque flux critique est un scénario Gherkin, et chaque outil du contrat a son contract test. Les deux vivent dans `tests/features/` et `tests/contract/`.

## 10. Modèle de données (entités DDD → tables PostgreSQL) · *stateful*

Le MVP n'a **aucune base de données** : le journal d'audit est un fichier JSONL append-only, l'inventaire est dérivé du système de fichiers et de l'API OVH. PostgreSQL n'apparaît qu'avec FR-023.

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

## 13. Critères de succès MVP

Le MVP est atteint quand, sur une machine dépourvue de tout binaire d'infrastructure :

1. `make ci` est vert, y compris clippy en `-D warnings`, gitleaks et le SBOM CycloneDX ;
2. Sluis déclaré dans un `.mcp.json` répond à `initialize` puis `tools/list` ;
3. `sluis_inventory` pointé sur `koprogo/infrastructure` ressort **3 topologies, 4 environnements, 3 profils de cluster et 4 modules Terraform, sans aucune saisie manuelle** ;
4. `sluis_doctor` rapporte les 6 binaires absents sans panique ;
5. les contract tests prouvent la conformité de chaque schéma déclaré à sa désérialisation ;
6. tous les tests `@security` du MVP sont au vert, dont le refus d'un projet hors liste et l'absence de tout secret en sortie ;
7. le journal d'audit contient une entrée par appel, succès comme échec.

---

*Dérivé du Manifeste Maury (CC BY-SA 4.0). Source : [`github.com/gilmry/manifest`](https://github.com/gilmry/manifest).*
