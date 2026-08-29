# Architecture Decision Records

Les ADR de Sluis vivent dans le livrable de l'Architecte,
[`../bmad/03-architecture.md`](../bmad/03-architecture.md), section *ADR*.
Ils n'y sont pas dupliqués ici : une décision recopiée à deux endroits
finit par diverger.

| # | Décision | Irréversible | Statut |
|---|---|---|---|
| ADR-001 | Hexagonale + SOLID | non | acceptée |
| ADR-002 | TDD + BDD + Documentation Vivante | non | acceptée |
| ADR-003 | DDD ubiquitous language → mapping code | non | acceptée |
| ADR-004 | Rust plutôt que le défaut BMAD Python + FastAPI | non | acceptée, écart méthodologique assumé |
| ADR-005 | Le contrat MCP est matérialisé, pas décrit | **oui** | acceptée |
| ADR-006 | Persistance CQRS SQL pur, pas d'ORM | **oui** | à contresigner |
| ADR-007 | Dérogation Tier 2 bornée **et à durée limitée** pour les bacs à sable | **oui** | **approuvée le 2026-08-29 avec la 7e condition, à contresigner** |
| ADR-008 | Passerelle d'approbation = environnement GitHub protégé | **oui** | acceptée |
| ADR-009 | Mapping branche → environnement | **oui** | acceptée |

Tout point marqué irréversible exige une validation humaine explicite
avant d'être mis en œuvre. ADR-007 est le seul du lot qui assouplit un
garde-fou existant : il mérite une signature écrite, pas un accord oral.

C'est aussi le seul qui **s'auto-limite dans le temps**. Sa septième
condition fait expirer la dérogation par défaut et rend son
renouvellement dépendant du gate Tier 1 qu'elle relâche. La signature
doit donc porter la **date d'ouverture de la première fenêtre**, puisque
c'est elle qui déclenche le décompte.
