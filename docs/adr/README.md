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
| ADR-006 | Persistance CQRS SQL pur, pas d'ORM | **oui** | contresignée le 2026-08-29 · **amendée à la fabrication** : dépôt fichier, voir ci-dessous |
| ADR-007 | Dérogation Tier 2 bornée **et à durée limitée** pour les bacs à sable | **oui** | **contresignée le 2026-08-29** · fenêtre ouverte jusqu'au **2026-11-27** |
| ADR-008 | Passerelle d'approbation = environnement GitHub protégé | **oui** | acceptée |
| ADR-009 | Mapping branche → environnement | **oui** | acceptée |

Tout point marqué irréversible exige une validation humaine explicite
avant d'être mis en œuvre. ADR-007 est le seul du lot qui assouplit un
garde-fou existant : il mérite une signature écrite, pas un accord oral.

C'est aussi le seul qui **s'auto-limite dans le temps**. Sa septième
condition fait expirer la dérogation par défaut et rend son
renouvellement dépendant du gate Tier 1 qu'elle relâche.

> ⏳ **Première fenêtre : 2026-08-29 → 2026-11-27 (90 jours).**
> Passé cette date et sans renouvellement Tier 1, tout bail de bac à
> sable est refusé. C'est voulu : le silence ne reconduit rien.


## Amendement d'ADR-006, 2026-08-29

L'implémentation persiste le dépôt OAuth **dans un fichier JSON** plutôt que
dans PostgreSQL. Deux raisons :

- Sluis sert un superviseur et ses projets, pas des organisations tierces. Le
  volume est de quelques clients et quelques jetons.
- `sqlx` en vérification à la compilation exige une base vivante au moment du
  build, ce qui contredirait NFR-06 : aucun test ne doit dépendre d'un service
  absent.

Ce que l'amendement **ne relâche pas** : la révocation reste durable, écrite
avant tout retour, et la consommation d'un code reste atomique. Le port
`DepotOAuth` rend la bascule vers PostgreSQL sans effet sur le reste du code, le
jour où le volume l'exigera. Cet amendement est à contresigner.
