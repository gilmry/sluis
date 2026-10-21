# ADR-032 : Ansible configure le socle, OpenTofu loue, Flux réconcilie le cluster

- **Statut** : Proposé le 2026-10-21, non contresigné.
- **Irréversible** : oui : le durcissement serveur et le chiffrement des disques sont des points irréversibles (`foyer/skills/convergence-iac.md`), et les playbooks deviennent la mémoire de chaque machine.
- **S'articule avec** : ADR-010 (OpenTofu est le moteur de **location** d'infrastructure, inchangé) ; ADR-015 (certificats SSH courts) ; ADR-016 (SOPS) ; ADR-025 (dérive quotidienne) ; ADR-029 (recettes) ; **écart assumé avec `foyer/skills/convergence-iac.md`**, qui cite `salt-ssh`.
- **Sert la vision (ADR-000)** : principe 1, la configuration devient rejouable et non plus mémorisée ; principe 2, la dérive se détecte chaque jour et alerte ; principe 3, l'état réel est confronté à l'état déclaré, pas à ce qu'on croit avoir fait.

## Contexte

`foyer/skills/convergence-iac.md` prouve la convergence par l'absence de diff au
ré-apply, et nomme `salt-ssh` comme moteur de configuration. Sur la machine de
travail actuelle, une partie de la configuration a été faite à la main et
n'existe que dans des journaux de session : c'est exactement la mémoire
qu'aucune automatisation ne relit. Il faut un moteur de configuration sans agent
installé, joignable en SSH, avec un écosystème suffisant pour LUKS, K3s et SOPS.

## Décision

**Partage des rôles, sans recouvrement :**

| Couche | Outil | Rôle |
|---|---|---|
| Location | **OpenTofu** (ADR-010) | crée et détruit les machines, réseaux et stockages chez l'hébergeur |
| Socle | **Ansible** | système, durcissement, LUKS, pare-feu, agents de supervision, installation de K3s |
| Cluster | **Flux** (recette K3s) ou **Coolify** (recette VPS) | ce qui tourne sur la machine ou dans le cluster (ADR-029) |

**Règles d'Ansible :**

1. **Sans agent, via SSH**, avec des **certificats SSH courts émis par step-ca** (ADR-015) ; aucune clé SSH statique d'accès aux machines.
2. **Secrets** lus par `community.sops` (ADR-016) ; aucun secret en clair dans les playbooks ni dans l'inventaire.
3. **Contrôle de dérive quotidien** : exécution en `--check --diff` ; tout écart est une alerte dans le circuit standard (ADR-023, ADR-025), et sa réussite émet un signal de succès dont l'absence alerte aussi.
4. **Retour arrière** : les playbooks sont idempotents et versionnés ; revenir en arrière, c'est rejouer ceux d'un commit antérieur. Un rôle qui n'est pas idempotent ne passe pas la revue.
5. **Preuve de convergence** au sens de Foyer : un second passage sans changement (`changed=0`) est la preuve d'Évaluation.

## Alternatives écartées

- **salt-ssh**, cité par Foyer : fonctionne sans agent, mais écosystème plus étroit pour K3s et SOPS, et moins de rôles maintenus pour le durcissement. Écart assumé, et proposé à Foyer comme un second adaptateur plutôt qu'un remplacement (section suivante).
- **OpenTofu ou Pulumi pour la configuration système** : faits pour l'état des ressources chez un fournisseur, pas pour le contenu d'une machine ; leur état devient faux dès qu'un fichier change sur la machine. OpenTofu reste dans son rôle de location (ADR-010).
- **Images immuables reconstruites à chaque changement** (Packer et équivalents) : séduisant, mais un VPS loué au mois ne se remplace pas comme une instance ; à réévaluer en recette K3s.
- **Configuration à la main, documentée** : c'est la situation actuelle, et ce qu'ADR-000 refuse.

## Conséquences

- **Entretien** : rôles Ansible à tenir à jour (Renovate les suit, ADR-025) ; une exécution complète par jour en mode vérification.
- **Coût du premier passage** : décrire en playbooks ce qui existe déjà, machine par machine. C'est le prix de sortie de la configuration faite à la main.
- **Limite** : `--check` n'est pas parfait, certains modules ne savent pas simuler. Ces tâches se marquent explicitement, et leur dérive se mesure autrement (ADR-021, croisement de sources).
- **Limite** : le chiffrement LUKS sur un VPS demande un déverrouillage au démarrage ; sa méthode (déverrouillage distant ou par la machine de confiance) reste à fixer, et l'hébergeur garde l'accès à la mémoire (ADR-029).

## Mesures CyFun 2025 couvertes

Mapping dans `foyer/skills/conformite.md`, couche **plateforme**.

- Dérive de configuration détectée : PR.PS-01.1 ★ et DE.CM-09.1 (Epic 9, story 9.5).
- Durcissement et configuration de référence : catégorie PR.PS, identifiant à confirmer dans le livret.

## Proposition de PR sur Foyer

*Proposée, non ouverte.* Dans `skills/convergence-iac.md`, section « Deux moteurs », remplacer le paragraphe « Config + durcissement » par :

> **Config + durcissement** : un moteur **idempotent et sans agent**, adaptateur enfichable au sens de `../KITS.md`. Deux adaptateurs sont reconnus : **`salt-ssh`** et **Ansible** (via SSH, secrets par `community.sops`). Le choix se trace en ADR dans le projet ; la preuve d'Évaluation est la même pour les deux : un second passage sans changement. Cible : conteneur **ou** VM. Le moteur de **location** d'infrastructure (OpenTofu) est un troisième rôle, distinct : il crée les machines, il ne les configure pas.

et, dans « Le cercle pour l'infrastructure », remplacer « états salt idempotents » par « états idempotents (salt ou Ansible) » et « `salt-ssh` pour la config/durcissement » par « l'adaptateur de configuration (salt-ssh ou Ansible) ».

## Points ouverts

- Méthode de déverrouillage LUKS au démarrage d'un VPS.
- Rôles de durcissement retenus (rôles communautaires maintenus ou rôles écrits), et leur correspondance avec la référence de durcissement visée.
- Inventaire : dynamique depuis l'état d'OpenTofu, ou statique versionné.
