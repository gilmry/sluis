---
livrable: Architecture Technique
persona: Architecte
phase_togaf: C-D — Systèmes d'information + Technique
projet: Sluis
version: 0.1.0-draft
genere_le: 2026-08-29
depend_de: 02-prd.md
signature_humaine:
  nom:
  role:
  date:
  verdict:
---

# Architecture Technique — Sluis

*Livrable de l'Architecte · TOGAF Phases C-D. Organisé par les sept couches. Archétype **stateful × API-first** : la couche Frontend est hors scope, la persistance et le contrat sont en scope.*

## Vue d'ensemble

```
                        ┌───────────────────────────────────────┐
                        │   Clients MCP                          │
                        │   Claude Code (stdio) · claude.ai      │
                        └──────────────┬────────────────────────┘
                                       │ JSON-RPC 2.0
                    ┌──────────────────┴──────────────────┐
                    │  Couche 3 — Infrastructure           │
                    │  ┌────────────────────────────────┐  │
                    │  │  Couche 2 — Application         │  │
                    │  │  ports (traits) + use cases     │  │
                    │  │  ┌──────────────────────────┐   │  │
                    │  │  │  Couche 1 — Domain        │   │  │
                    │  │  │  entités + invariants     │   │  │
                    │  │  │  ZÉRO dépendance infra    │   │  │
                    │  │  └──────────────────────────┘   │  │
                    │  └────────────────────────────────┘  │
                    └──────┬───────────────────────┬───────┘
                           │                       │
              ┌────────────┴─────┐       ┌─────────┴──────────────┐
              │  API OVHcloud     │       │  GitHub Actions        │
              │  (lecture)        │       │  environnement protégé │
              │                   │       │  ← DÉTIENT LES CLÉS    │
              └───────────────────┘       └────────────────────────┘
                                                    │
                                          ┌─────────┴──────────┐
                                          │  terraform · helm  │
                                          │  kubectl · argocd  │
                                          └────────────────────┘

Dépendances strictement vers l'intérieur. Sluis n'a aucune flèche
directe vers les moteurs de mutation de production : elles partent
du job GitHub, après approbation humaine.
```

Ce diagramme porte la décision structurante du projet : **les moteurs de mutation ne sont pas atteignables depuis Sluis en production**. Sluis rend le plan et déclenche un job ; le job détient les identifiants.

## Couche 1 — Domain *(SOLID : SRP, DIP — racine : Ubuntu)*

Aucune dépendance vers `reqwest`, `sqlx`, `actix_web`, `tokio`. Vérifié mécaniquement (voir Couche 6).

### Entités et value objects par bounded context

**BC1 Inventaire** — `Topology` (`Vps | K3s | K8s`), `Environment` (`Dev | Integration | Staging | Production`, `Ord` implémenté selon l'ordre de promotion), `ClusterProfile`, `TerraformModule`, `InfrastructureMatrix`, `Drift`.

**BC2 Autorisation** — `Tier` (`Two | One`), `Action`, `ChangePlan`, `PlanFingerprint`, `ChangeToken`, `TokenState` (`Issued | Consumed | Expired`).

**BC3 Bac à sable** — `SandboxLease`, `Ttl`, `SpendCap`, `SandboxProjectId` (type distinct de `ProductionProjectId`).

**BC4 Capacité** — `LoadCampaign`, `LoadStep`, `CapacityMeasurement`, `Provenance` (`Measured | Assumed`), `Prior`.

**BC5 Exécution** — `EngineKind`, `EngineInvocation`, `ConvergenceProof`.

**BC6 Accès** — `Scope` (`Read | Sandbox | Propose`), `AuditEntry`, `OAuthClient`, `AuthorizationCode`, `RefreshTokenHash`.

### Invariants codés dans les constructeurs

Les dix invariants du Brief §10, rendus **inconstructibles** plutôt que vérifiés à l'exécution.

| Invariant | Mécanisme |
|---|---|
| Un plan sur `Production` est de Tier 1 | `ChangePlan::new` retourne `Err(AppError::TierViolation)` ; il n'existe aucun autre constructeur |
| Un bail a toujours TTL et plafond | `SandboxLease::new(ttl: Ttl, cap: SpendCap, …)` — les deux non optionnels, aucun `Default` |
| Un bail ne vise qu'un projet de bac à sable | `SandboxProjectId` est un **type distinct** ; la signature refuse un `ProductionProjectId` à la compilation |
| Un jeton n'est valide que pour son empreinte | `ChangeToken` porte la `PlanFingerprint` ; `consume(&plan)` compare avant tout effet |
| Un jeton est consommé une seule fois | `consume` prend `self` par valeur et rend `ConsumedToken` : le rejeu ne compile pas |
| L'ordre de promotion est total | `Environment` implémente `Ord` ; `promote_to` refuse un saut |
| Une entrée d'audit est immuable | Champs privés, aucun setter, port `AuditLog` sans `update` ni `delete` |
| Une mesure porte sa provenance | `Provenance` non optionnel dans `CapacityMeasurement::new` |
| Un secret ne franchit pas la sortie | `Redacted<T>` dont `Display` et `Serialize` rendent `«redacted»` ; les identifiants ne sont typés qu'ainsi |
| Une campagne détruit toujours son bail | Garde RAII `LeaseGuard` avec `Drop`, doublée du chien de garde externe |

Le choix `consume(self) -> ConsumedToken` mérite d'être souligné : il déplace l'invariant d'usage unique du domaine de l'exécution vers celui de la compilation. C'est la forme la plus forte de « codé dans le constructeur ».

## Couche 2 — Application *(ISP, OCP, DRY)*

Ports en traits, un par préoccupation, volontairement étroits (ISP).

| Port | Méthodes | Implémenté par |
|---|---|---|
| `InventoryRepository` | `discover_matrix`, `read_profiles` | `fs_inventory` |
| `OvhProvider` | `list_projects`, `list_instances`, `current_costs`, `dns_records` | `ovh::Client` |
| `TerraformRunner` | `plan`, `apply` (Tier 1) | `process::Terraform` |
| `ConfigRunner` | `converge`, `check` | `process::Ansible` (salt-ssh envisageable, d'où l'abstraction) |
| `HelmRunner` | `status`, `history`, `rollback` | `process::Helm` |
| `KustomizeRenderer` | `build` | `process::Kustomize` |
| `ArgoCdClient` | `app_status`, `sync` | `process::ArgoCd` |
| `LoadTestRunner` | `run_step` | `loadtest::Wrk` |
| `ApprovalGateway` | `submit`, `poll` | `github::Actions` |
| `GateChecker` | `floor_status` (secrets, SBOM, scan d'image, retour de migration) | `github::Checks` + `process::Scanner` |
| `AuditLog` | `append` (uniquement) | `audit::Jsonl` |
| `Clock` | `now` | système, ou figé en test |
| `OAuthRepository` | clients, codes, jetons | PostgreSQL |

Un use case par outil MCP, sans logique métier dans les handlers. Les use cases orchestrent, le domaine décide.

**Point DRY notable** : la vérification « ce projet est-il autorisé » n'existe qu'à un seul endroit, dans le domaine, et tout use case touchant OVH y passe. Elle n'est jamais réimplémentée dans un adaptateur.

## Couche 3 — Infrastructure backend *(LSP, DIP)* · *persistance : stateful*

| Module | Rôle | Décisions |
|---|---|---|
| `ovh/` | Client REST signé | Signature `$1$` + SHA-1 de `secret+consumerKey+METHOD+URL+body+timestamp`, delta d'horloge via `/auth/time`, `Clock` injecté pour la testabilité |
| `process/` | Runners CLI | **Allowlist d'exécutables, arguments passés en tableau, jamais de shell.** Un binaire absent produit `AppError::EngineMissing` nommant le binaire |
| `fs_inventory/` | Découverte | Racine autorisée, refus de toute remontée hors racine, liens symboliques non suivis hors racine |
| `github/` | Passerelle | `workflow_dispatch` + interrogation du run ; ne détient qu'un jeton de déclenchement, jamais les secrets d'infrastructure |
| `loadtest/` | Pilote `wrk` | Parsing structuré de la sortie, refus à l'admission si `wrk` est absent |
| `mcp/` | Transports | stdio et Streamable HTTP, dispatch JSON-RPC, registre d'outils |
| `oauth/` | Serveur d'autorisation | Porté des références du skill `mcp-oauth-maison` |
| `audit/` | Journal | JSONL append-only, ouverture en `O_APPEND`, verrou d'écriture |

**Persistance : CQRS SQL pur via `sqlx`, pas d'ORM.** Voir ADR-006.

Middleware dans les adaptateurs (extracteur `Bearer`, filtre de rédaction en sortie), jamais dans l'application.

## Couche 4 — Frontend

**Hors scope** (archétype API-first). Seule exception, tracée : le formulaire HTML de `GET /oauth/authorize`, détail de l'adaptateur d'authentification, sans page Astro ni îlot Svelte.

## Couche 5 — IaC

- **Conteneur** : image **distroless** en production, binaire unique statique.
- **Configuration et durcissement** : idempotents, derrière `ConfigRunner`. Ansible aujourd'hui, salt-ssh visé par `convergence-iac.md` : l'abstraction rend la bascule non structurante.
- **GitOps** : la branche `main` est la source de vérité, cron toutes les 5 minutes, `deploy.sh --run`, fast-forward only, verrou `flock`, révision déployée mémorisée. Le serveur ne construit rien, il tire depuis GHCR.
- **Progression YAGNI** : le socle de lecture n'a ni base de données ni serveur HTTP. PostgreSQL et Traefik n'apparaissent qu'avec FR-023.
- **Rollback** si les tests post-déploiement échouent. La sortie n'est jamais un état non vérifié.

### Mapping ISO 27001 → IaC

| Contrôle | Mise en œuvre |
|---|---|
| A.5 (politiques) | `SECURITY.md`, ADR-007 sur la dérogation bac à sable |
| A.8 (gestion des actifs) | `sluis_inventory` : l'inventaire déclaratif **est** une fonction du produit |
| A.9 (contrôle d'accès) | Scopes OAuth, liste d'autorisation des projets, environnement GitHub protégé à relecteurs requis |
| A.10 (cryptographie) | PKCE S256, jetons de rafraîchissement stockés en SHA-256, TLS Let's Encrypt |
| A.12 (sécurité d'exploitation) | Journal d'audit append-only, image distroless, SBOM CycloneDX à chaque build |
| A.12.6 (vulnérabilités techniques) | `cargo audit` en CI, scan d'image avant push registre (gate bloquante) |
| A.14 (acquisition et développement) | RED-first, 4 classes de tests, contract tests, revue de PR |
| A.16 (incidents) | Journal horodaté avec empreinte et approbateur, rejouable |
| A.17 (continuité) | Baux à TTL, chien de garde, destruction garantie |
| A.18 (conformité) | Souveraineté EU-West, aucune donnée personnelle hors identifiants |

## Couche 6 — CI/CD

Mêmes vérifications en local (hooks Git) et en CI, par DRY. Au plus tard en pre-push, la CI complète.

| Job | Contenu | Registre |
|---|---|---|
| `fmt` | `cargo fmt --check` | plancher |
| `clippy` | `cargo clippy -- -D warnings` | plancher |
| `test` | unitaires, intégration, BDD, **contract tests** | plancher |
| `purete-domaine` | **échoue si `src/domain/` importe un crate d'infrastructure** | plancher |
| `secrets` | gitleaks sur le diff et l'historique | **plancher, irréversible** |
| `sbom` | CycloneDX à chaque build | **plancher, irréversible** |
| `image` | scan avant push registre | **plancher, irréversible** |
| `audit-deps` | `cargo audit` | exigence de jalon (rapport) |
| `licences` | cohérence MIT et compatibilité des dépendances | exigence de jalon |

Le job `purete-domaine` est ce qui rend l'invariant hexagonal objectivable plutôt que déclaratif. Il satisfait le double critère du plancher : objectivable (l'import existe ou non) et irréversible (une fuite d'infrastructure dans le domaine contamine tout ce qui s'appuie dessus ensuite).

## Couche 7 — Monitoring & Sécurité

- **Observabilité** : `tracing` structuré vers stdout, agrégé par la stack existante. Métriques Prometheus sur le serveur distant.
- **Alertes comme déclencheur** : un bail approchant son TTL, un plafond de dépense à 80 %, un échec de chien de garde. Chacune ré-entre dans le cercle.
- **Détection** : toute tentative d'accès à un projet hors liste est journalisée comme événement de sécurité, pas comme simple erreur.
- **Rédaction** : filtre appliqué **à la frontière du transport**, pas dans les use cases, pour qu'aucun nouveau chemin d'appel ne puisse le contourner.

## Contrat API · *API-first : écrit avant le code, versionné*

Le contrat du PRD §9bis est la source de vérité. Sa **matérialisation** fait l'objet d'ADR-005 et repose sur quatre mécanismes, tous obligatoires :

1. annotation exhaustive des types d'entrée d'outils ;
2. génération du JSON Schema depuis ces types, jamais écrit à la main ;
3. `#[serde(deny_unknown_fields)]` sur tous ces types ;
4. contract tests prouvant l'équivalence schéma déclaré ↔ désérialisation effective.

Une rupture de contrat est un **point irréversible** : ADR et validation humaine.

## Glossaire DDD → mapping code

| Terme métier | Type Rust | Emplacement |
|---|---|---|
| Topologie | `Topology` | `domain::inventory` |
| Environnement | `Environment` | `domain::inventory` |
| Profil de cluster | `ClusterProfile` | `domain::inventory` |
| Déploiement | `Deployment` | `domain::inventory` |
| Tier | `Tier` | `domain::authorization` |
| Plan de changement | `ChangePlan` | `domain::authorization` |
| Jeton de changement | `ChangeToken` / `ConsumedToken` | `domain::authorization` |
| Bail de bac à sable | `SandboxLease` | `domain::sandbox` |
| Chien de garde | `LeaseWatchdog` | `infrastructure::sandbox` |
| Campagne de charge | `LoadCampaign` | `domain::capacity` |
| Mesure de capacité | `CapacityMeasurement` | `domain::capacity` |
| Prior | `Prior` | `domain::capacity` |
| Convergence | `ConvergenceProof` | `domain::execution` |
| Passerelle d'approbation | `ApprovalGateway` (port) | `application::ports` |
| Gate du plancher | `FloorGate` + port `GateChecker` | `domain::execution` / `application::ports` |
| Prior | `Prior` | `domain::capacity` |

## Stratégie de tests

Pyramide : beaucoup d'unitaires sur le domaine (pur, rapide, sans doublure), des tests d'application avec doublures de ports, peu de tests d'infrastructure, et des contract tests transverses.

| Couche | Couverture cible | Moyens |
|---|---|---|
| Domain | **≥ 95 %** | unitaires purs, property-based sur les invariants |
| Application | ≥ 85 % | doublures de ports (`mockall`) |
| Infrastructure | ≥ 60 % | `wiremock` pour OVH, doublures de processus, aucun binaire réel |
| Contrat | **100 % des outils** | contract tests |

**Aucun test ne dépend d'un binaire d'infrastructure** (NFR-06) : c'est une contrainte de conception des adaptateurs, pas une facilité de test.

## Classes de tests (4×N — hérité de `skills/cycle-dev.md`)

| Classe | BDD (Gherkin) | TDD (unitaire) | Conditionnement archétype |
|---|---|---|---|
| `@happy` | Chemin nominal d'un outil | Invariants de constructeur, use case nominal | Tous |
| `@negative` | Entrées invalides, moteur absent | `AppError` typée attendue, jamais de panique | Tous |
| `@edge` | Bornes : matrice vide, TTL au ras, pagination | **stateful** : concurrence sur le journal et les jetons, transitions d'état | stateful |
| `@security` | Abus de contrat, projet hors liste, rejeu de jeton | Étanchéité des secrets, refus d'injection d'arguments | **API-first** : abus de contrat, `deny_unknown_fields` |

## API · Sécurité & RGPD

- **RGPD** : Sluis ne traite aucune donnée personnelle, hormis l'adresse e-mail servant d'identifiant de connexion. Aucun profilage, aucune donnée de production ne transite. Les campagnes de charge utilisent exclusivement des jeux synthétiques (invariant de bac à sable).
- **Sécurité** : PKCE S256 exclusivement, jetons de rafraîchissement stockés en hash avec rotation inconditionnelle, codes d'autorisation à usage unique liés au triplet client / URI / défi, `redirect_uri` jamais fait confiance avant validation, `tools/call` revérifiant systématiquement l'autorisation indépendamment de `tools/list`.
- **Souveraineté** : EU-West exclusivement.

## ADR (Architecture Decision Records)

### ADR-001 — Hexagonale + SOLID *(racine : écologie des savoirs)*

**Décision** : trois couches, dépendances strictement vers l'intérieur, domaine sans dépendance d'infrastructure, vérifié mécaniquement en CI.
**Contexte** : trois projets Maury (KoproGo, Elevia, derniere-chance) partagent déjà cette structure ; un quatrième qui divergerait fragmenterait le savoir.
**Alternatives écartées** : architecture en couches classique (laisse fuiter la persistance dans le métier), monolithe transactionnel (rend les invariants non testables sans base).
**Conséquences** : plus de types et de traits à écrire ; en échange les invariants deviennent testables sans infrastructure, et les moteurs sont substituables.

### ADR-002 — TDD + BDD + Documentation Vivante *(racine : sept générations)*

**Décision** : RED-first, quatre classes de tests obligatoires pour tout livrable public, flux critiques en scénarios Gherkin.
**Contexte** : le code est majoritairement produit par un agent ; le test est le seul signal objectif qui distingue « ça compile » de « ça fait ce qui était demandé ».
**Alternatives écartées** : tests écrits après (mesurent ce que le code fait, pas ce qu'il devait faire), couverture comme seule cible (mesure l'exécution, pas l'intention).
**Conséquences** : coût d'écriture initial supérieur ; en échange une backlog réellement exécutable sans le superviseur.

### ADR-003 — DDD ubiquitous language → mapping code *(racine : Ubuntu)*

**Décision** : le glossaire du Brief §8 est figé au PRD §4 et mappé explicitement vers les types.
**Contexte** : le vocabulaire d'autorisation existe déjà dans `AGENT_GUARDRAILS.md`. En inventer un second créerait deux langues pour une seule réalité.
**Alternatives écartées** : un vocabulaire propre à Sluis (`palier 0/1/2` avait été envisagé puis abandonné, car il inversait le sens de Tier 1).
**Conséquences** : Sluis parle la langue de la méthode, au prix de ne pas pouvoir choisir ses propres mots.

### ADR-004 — Rust plutôt que le défaut BMAD Python + FastAPI

**Décision** : Rust 2021 / Actix-web.
**Contexte** : BMAD Étape 0 prescrit Python LTS + FastAPI par défaut. Trois projets Maury sont en Rust hexagonal, dont deux portent déjà un serveur MCP avec OAuth.
**Alternatives écartées** : Python + FastAPI (conforme au défaut, mais impose un runtime à déployer et perd la réutilisation directe de ~1400 lignes déjà écrites) ; Go (binaire unique aussi, mais aucun précédent dans l'écosystème Maury).
**Conséquences** : écart assumé au défaut méthodologique, à mentionner dans toute présentation de la méthode. En échange : binaire unique, image distroless, empreinte mémoire faible, et le typage nécessaire pour rendre les invariants inconstructibles.

### ADR-005 — Le contrat MCP est matérialisé, pas décrit *(point irréversible)*

**Décision** : schémas générés depuis les types, `deny_unknown_fields`, contract tests d'équivalence, version de protocole explicite.
**Contexte** : `contrat-api.md` est né d'un incident réel où un contrat décrit mais non matérialisé a produit un NO-GO en production.
**Alternatives écartées** : schémas écrits à la main (dérivent silencieusement du code) ; SDK MCP tiers (dépendance externe évolutive, contraire à la sobriété, et les deux précédents Maury écrivent le JSON-RPC à la main).
**Conséquences** : une rupture de contrat devient détectable en CI. Contrepartie assumée : suivre les évolutions du protocole MCP à la main.

### ADR-006 — Persistance : CQRS SQL pur, pas d'ORM *(point irréversible, validation humaine)*

**Décision** : `sqlx` en SQL explicite, vérifié à la compilation, migrations versionnées avec fichier de retour.
**Contexte** : le modèle est petit (7 tables) et fortement contraint par la sécurité (hash de jetons, unicité de consommation). Un ORM masquerait précisément ce qu'on veut voir.
**Alternatives écartées** : Diesel ou SeaORM (ajoutent une indirection sans bénéfice à cette taille) ; pas de base du tout (impossible dès qu'il faut révoquer un jeton).
**Conséquences** : SQL visible et auditable. Chaque migration est un point irréversible exigeant validation humaine et livrant son `*.down.sql` au titre du plancher.

### ADR-007 — Dérogation Tier 2 bornée pour les bacs à sable *(validation humaine requise)*

**Décision** : le provisionnement et la destruction d'infrastructure éphémère de test sont de Tier 2, sous six conditions cumulatives et non négociables.
**Contexte** : la règle d'or impose Tier 1 au doute, et un bail est bien un `terraform apply`. Mais soumettre chaque campagne à une approbation humaine détruit l'objectif d'autonomie.
**Alternatives écartées** : Tier 1 systématique (sûr, mais l'agent ne peut plus enchaîner de paliers sans interrompre le superviseur) ; Tier 2 non borné (inacceptable, ouvre la voie au glissement).
**Conséquences** : les six conditions sont des **invariants de domaine**, pas des vérifications de surface, et `SandboxProjectId` est un type distinct de `ProductionProjectId` pour que la confusion ne compile pas. Toute extension de cette dérogation exige un nouvel ADR.
**Statut** : approuvée par le superviseur le 2026-08-29, à contresigner au frontmatter.

### ADR-008 — La passerelle d'approbation est un environnement GitHub protégé *(point irréversible)*

**Décision** : une action Tier 1 passe par un `workflow_dispatch` dont le job est lié à un environnement GitHub protégé à relecteurs requis. Les secrets d'infrastructure vivent dans GitHub Actions.
**Contexte** : le skill `mcp-oauth-maison` pose que `/mcp` doit rester en lecture seule tant qu'il n'existe pas de mécanisme de confirmation fiable. La confirmation d'appel côté client, retenue par derniere-chance, suppose un humain au clavier, ce que l'objectif d'autonomie exclut.
**Alternatives écartées** : confirmation en conversation (Sluis détiendrait alors les clés, et la trace ne serait qu'un message) ; confirmation côté client MCP seule (inopérante en exécution autonome).
**Conséquences** : **propriété de sécurité structurelle** — un Sluis compromis ne peut pas muter la production, il ne peut que soumettre un plan refusable. C'est aussi ce qui rend acceptable le co-hébergement avec n8n. Si Sluis venait un jour à détenir ces clés, ADR-008 tomberait et le co-hébergement deviendrait à revoir.

### ADR-009 — Mapping branche → environnement *(point irréversible, validation humaine)*

**Décision** : `main` déploie le serveur Sluis lui-même. Les mappings des projets orchestrés restent la propriété de ces projets ; Sluis les lit, ne les impose pas.
**Contexte** : KoproGo a déjà son mapping (`integration`, `staging`, `production`). Le dupliquer créerait deux sources de vérité.
**Conséquences** : Sluis est un lecteur de la convention d'autrui, pas son auteur.

---

*Dérivé du Manifeste Maury (CC BY-SA 4.0). Source : [`github.com/gilmry/manifest`](https://github.com/gilmry/manifest).*
