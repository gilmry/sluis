# ADR-006, amendement du 2026-08-29 : le dépôt OAuth persiste dans un fichier JSON

- **Statut** : amendement contresigné le 2026-10-18. ADR-006 lui-même vit dans [`../bmad/03-architecture.md`](../bmad/03-architecture.md).
- **Sorti de l'index le 2026-10-21** (revue des ADR, décision 7) : texte déplacé depuis `README.md` sans modification de fond.

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
jour où le volume l'exigera. Amendement contresigné le 2026-10-18.
