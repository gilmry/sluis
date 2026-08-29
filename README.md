# Sluis

> L'écluse. Un orchestrateur d'infrastructure OVH exposé comme serveur MCP, qui laisse un agent IA mesurer, proposer et déployer sans jamais pouvoir nuire.

**État : fabrication terminée.** Gate BMAD franchie le 2026-08-29, les cinq livrables sont signés. Sprints 0 à 2 livrés, Sprints 3 et 4 en cours.

```
Sprint 0  Fondations                    ██████████  5/5
Sprint 1  Inventaire · Autorisation     ██████████  6/6
Sprint 2  Exécution · Transport MCP     ██████████  8/8
Sprint 3  Bac à sable · Capacité        ██████████  7/7
Sprint 4  Tier 1 · Accès distant        ██████████  5/5
```

**31 stories livrées, 187 tests verts.** Restent deux hypothèses à lever avant
de pouvoir prouver le dispositif contre du réel : un projet OVH dédié aux bacs
à sable (H1), et un environnement GitHub protégé sur les dépôts cibles (H2).

---

## Le nom

Une écluse fait passer un bateau d'un niveau à l'autre **sans jamais ouvrir les deux portes en même temps**. C'est le pipeline `dev → integration → staging → production`, et c'est le mécanisme d'approbation en deux temps qui gouverne toute mutation dangereuse.

C'est aussi une technologie sobre : mue par la gravité, sans énergie, réparable, en service depuis des siècles.

## Le problème

Piloter une infrastructure avec des agents IA laisse aujourd'hui deux postures, toutes deux mauvaises. Soit on interdit à l'agent toute mutation, et le travail avance à la vitesse de la disponibilité humaine. Soit on lui ouvre les droits, et on désarme les protections qu'on a soi-même posées.

## La réponse

Un point de passage unique, authentifié et journalisé, qui classe chaque action selon le modèle d'autorisation **Tier 1 / Tier 2** déjà en vigueur dans l'écosystème Maury, et qui refuse structurellement ce qu'il n'a pas le droit de faire.

La décision d'architecture centrale : **Sluis ne détient aucun secret de mutation de production.** Ils vivent dans GitHub Actions, derrière un environnement protégé à relecteurs requis. Une action dangereuse suit ce chemin :

```
agent → plan empreinté → workflow_dispatch → job bloqué
                                                 ↓
                                    approbation humaine sur GitHub
                                                 ↓
                                 exécution avec les secrets du job
                                                 ↓
                                        compte rendu à l'agent
```

Conséquence : même compromis, Sluis ne peut pas muter la production. Il ne peut que demander.

## Les trois niveaux d'autorisation

| Niveau | Portée | Confirmation |
|---|---|---|
| Tier 2 lecture | inventaire, coûts, plans, statuts, diagnostics | aucune |
| Tier 2 écriture bornée | infrastructure éphémère de test, à TTL et plafond | confirmation d'appel côté client MCP, **dans une fenêtre de dérogation ouverte** |
| Tier 1 | toute mutation de production, mise en ligne | environnement GitHub protégé |

La dérogation du niveau intermédiaire est encadrée par **sept conditions cumulatives**, codées comme invariants de domaine et non comme vérifications de surface.

La septième mérite d'être lue à part : **la dérogation elle-même expire.** Elle est accordée pour une fenêtre limitée, elle est réputée expirée par défaut, et son renouvellement est un acte de Tier 1 qui emprunte la même passerelle qu'un `terraform apply` de production. Autrement dit, l'autorité de déléguer n'est jamais elle-même déléguée, et une dérogation ne peut pas devenir permanente par simple inertie. Voir ADR-007.

## Les deux scénarios cibles

**Campagnes de charge sur infrastructure éphémère.** Louer une infra jetable, prouver sa convergence, dérouler un escalier de charge, collecter des mesures, détruire inconditionnellement, puis produire un rapport qui remplace les constantes supposées du modèle de coût par du mesuré.

**Mise en ligne.** Vérifier les gates du plancher, produire un plan, obtenir l'approbation humaine, déployer, et revenir en arrière si les tests post-déploiement échouent. La sortie n'est jamais un état non vérifié.

## Conception

Les livrables suivent la pipeline BMAD du framework Foyer. Chacun porte un frontmatter de signature humaine : **un livrable non relu est une faute au sens *répondre-de***.

| # | Livrable | Persona | État |
|---|---|---|---|
| [01](docs/bmad/01-product-brief.md) | Product Brief | Analyste | à signer |
| [02](docs/bmad/02-prd.md) | PRD | Product Manager | à signer |
| [03](docs/bmad/03-architecture.md) | Architecture + ADR | Architecte | à signer |
| [04](docs/bmad/04-epics-stories.md) | Epics & Stories | Scrum Master | à signer |
| [05](docs/bmad/05-validation.md) | Validation | Validateur | **PASS**, 100/100 |

Les cinq sont **signés au 2026-08-29**, ADR-006 et ADR-007 contresignés.

Le premier passage rendait CONCERNS à 88/100. Les trois incohérences sont closes : deux par rebouclage vers les personas concernés, une par décision du superviseur. Le second passage rend **PASS à 100/100**, avec deux réserves qui n'empêchent pas la sortie : une marge de 7 % sur la target de coût, et deux hypothèses à lever avant les Sprints 3 et 4.

**Périmètre complet, pas de découpage MVP** (décision du superviseur, 2026-08-29) : les 25 exigences et les 11 capacités sont livrées. Les sprints ordonnent la fabrication, ils ne priorisent pas un abandon.

## Utilisation

```sh
# Serveur MCP distant (OAuth 2.1 + PKCE, transport Streamable HTTP)
cargo run --bin sluis-server

# Diagnostic : ce que la machine sait faire, sans rien révéler des identifiants
cargo run --bin sluis -- doctor

# Matrice d'infrastructure d'un dépôt, découverte sans aucune saisie
cargo run --bin sluis -- inventory /chemin/vers/infrastructure

# Serveur MCP, à déclarer dans un .mcp.json
cargo run --bin sluis-mcp
```

Sur une machine à petite racine, passer par un conteneur :
`make ci CARGO=kcargo`.

## Architecture

Rust 2021, hexagonale stricte, archétype **stateful × API-first**.

```
domain/          entités et invariants — zéro dépendance infrastructure
application/     ports (traits) et use cases — un use case par outil MCP
infrastructure/  ovh · process · fs_inventory · github · loadtest · mcp · oauth · audit
```

La pureté du domaine n'est pas une convention mais un job de CI qui échoue.

## Déploiement prévu

Deux modes. En local, un binaire en transport stdio déclaré dans un `.mcp.json`. Sur serveur, un service en transport Streamable HTTP derrière Traefik, avec OAuth 2.1 + PKCE, image construite en CI et tirée depuis GHCR, déploiement GitOps idempotent.

## Généalogie

Sluis ne part pas de zéro. Il assemble ce qui existe déjà :

- le modèle d'autorisation Tier 1 / Tier 2 de **KoproGo**, repris sans le redéfinir ;
- son corpus de tests de charge et ses modules Terraform OVH ;
- le serveur d'autorisation OAuth 2.1 + PKCE d'**Elevia**, packagé en skill ;
- son extension en écriture par **derniere-chance** ;
- la pipeline BMAD, l'abaque coût/capacité et les gates du framework **Foyer**.

## Licence

MIT. Voir [LICENSE](LICENSE).
