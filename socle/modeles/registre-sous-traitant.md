# Registre des activités de traitement du sous-traitant : <nom du SaaS>

Modèle F3 (sluis#112), RGPD, article 30, paragraphe 2. Tenu par écrit, sous forme
électronique, et communiqué à l'autorité de contrôle sur demande. Mis à jour à
chaque nouveau client, nouveau sous-traitant ultérieur ou changement de mesure
de sécurité.

**Dernière mise à jour** : `<date>`, par `<nom>`.

## 1. Le sous-traitant

| Champ | Valeur |
|---|---|
| Nom | `<raison sociale ou nom>` |
| Coordonnées | `<adresse, courriel, téléphone>` |
| Représentant (si hors UE) | sans objet / `<…>` |
| Délégué à la protection des données | `<nom et coordonnées, ou « non désigné » et pourquoi>` |

## 2. Les responsables du traitement pour le compte desquels il agit

Une ligne par client.

| Client | Coordonnées | Son représentant / DPO | Contrat signé le |
|---|---|---|---|
| `<nom>` | `<…>` | `<…>` | `<date>` |

## 3. Catégories de traitements effectués pour chaque client

| Client | Traitement | Personnes concernées | Catégories de données | Durée de conservation fixée par le client |
|---|---|---|---|---|
| `<nom>` | hébergement et exploitation de `<SaaS>` | `<…>` | `<…>` | `<…>` |
| `<nom>` | sauvegarde chiffrée hors serveur | idem | idem | 30 jours après chaque copie |

## 4. Transferts hors de l'Union européenne

| Destinataire | Pays | Données | Garantie (article 46 ou décision d'adéquation) |
|---|---|---|---|
| aucun / `<…>` | | | |

Un pays sous décision d'adéquation (par exemple la Suisse) se déclare ici
aussi, avec la décision qui le couvre.

## 5. Description générale des mesures de sécurité (article 32.1)

Renvoyer à la liste de contrôle « premier jour » du serveur (story F6) et à sa
dernière preuve datée, plutôt que de recopier l'état ici.

| Domaine | Mesure | Preuve |
|---|---|---|
| Chiffrement | sauvegardes chiffrées côté client, clé hors serveur ; TLS en transit | `<lien>` |
| Accès | clé SSH personnelle et second facteur ; liste nominative | `<lien>` |
| Disponibilité et restauration | sauvegarde quotidienne, restauration complète trimestrielle mesurée | `<lien>` |
| Journalisation | journaux hors hôte, en ajout seul, 12 mois | `<lien>` |
| Correctifs | correctifs critiques sous 5 jours ouvrés | `<lien>` |
| Évaluation régulière | contrôle de dérive quotidien, exercices | `<lien>` |
