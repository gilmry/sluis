# Sous-traitants ultérieurs : <nom du SaaS>

Modèle F4 (sluis#112), annexe 3 de chaque contrat. RGPD, article 28.2 : aucun
sous-traitant ultérieur sans autorisation écrite du client ; article 28.4 : les
mêmes obligations lui sont imposées ; chapitre V : tout transfert hors UE est
encadré.

**Règle** : la liste est **complète**. Le courriel, la supervision, les alertes,
la sauvegarde et le registre d'images y figurent au même titre que
l'hébergement. Un service absent de cette liste ne reçoit aucune donnée
personnelle.

**Version** : `<n>` du `<date>`. Tout ajout ou remplacement est annoncé au
client `<délai fixé au contrat>` à l'avance, avec possibilité d'objection.

| Sous-traitant | Prestation | Données personnelles reçues | Pays de traitement | Garantie de transfert | Autorisé le |
|---|---|---|---|---|---|
| `<hébergeur>` | serveurs de production | toutes | France | sans objet (UE) | `<date>` |
| `<hébergeur>` | stockage objet des sauvegardes chiffrées | toutes, chiffrées côté client | France, autre région | sans objet (UE) | `<date>` |
| `<prestataire de courriel>` | envoi des courriels | adresse, contenu des messages | `<pays>` | `<UE, décision d'adéquation, ou clauses types>` | `<date>` |
| `<service d'alerte>` | notifications d'exploitation | **aucune** (alertes sans donnée personnelle) | `<pays>` | sans objet | `<date>` |

## Services exclus

Services utilisés par l'équipe mais qui **ne reçoivent aucune donnée
personnelle** des clients. Les nommer évite qu'ils y entrent par accident.

| Service | Usage | Pourquoi il ne reçoit rien |
|---|---|---|
| `<…>` | `<…>` | `<…>` |
