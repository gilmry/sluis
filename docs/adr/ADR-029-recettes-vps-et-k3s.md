# ADR-029 : Une seule pile, K3s et Flux, d'un nœud à trois, derrière les mêmes outils MCP

- **Statut** : Proposé le 2026-10-21, non contresigné.
- **Amendé le 2026-10-21** (revue des ADR, décision 7) : identifiants CyFun 2025 complétés. Amendement proposé, non contresigné.
- **Réécrit le 2026-10-28** (sluis#6, décision du superviseur) : **Flux partout, Coolify abandonné**. La recette VPS devient un K3s à un nœud réconcilié par Flux, la recette K3s le même cluster porté à trois nœuds. Réécriture proposée, non contresignée.
- **Irréversible** : oui pour la mise en production : les données d'un SaaS y naissent, et en sortir est une migration (ADR-026, ADR-027). Le passage d'un à trois nœuds n'est plus un changement de plateforme : c'est un ajout de nœuds, et la bascule de la base qui l'accompagne (ADR-026) reste un point à valider le moment venu.
- **S'articule avec** : ADR-010 (OpenTofu loue, il ne configure pas), ADR-014 (trois zones), ADR-016 (secrets SOPS), ADR-026 (un nœud au démarrage, trois à terme, jamais deux), ADR-030 (serveur MCP), ADR-031 (pipeline), ADR-032 (Ansible), ADR-038 (double registre), ADR-041 (Keycloak) ; sluis#6 (ordre des deux portes) ; sluis#7 (réserve H1-Recette, sans objet) ; `foyer/skills/convergence-iac.md`.
- **Sert la vision (ADR-000)** : principe 4, on commence petit (une machine) sans payer plus tard une migration de plateforme ; principe 1, le déploiement applicatif est confié à un réconciliateur, pas à un humain.

## Contexte

Un SaaS Foyer doit pouvoir démarrer sur une seule machine louée à bas coût, puis
grandir vers un cluster sans que la méthode, la skill Foyer ni l'agent changent
de gestes. La première version de cet ADR prévoyait deux réconciliateurs :
Coolify sur un VPS au démarrage, Flux sur K3s ensuite. Le passage de l'un à
l'autre était une migration de plateforme, base et fichiers compris, à l'étape 8
de la feuille de route (ADR-033).

Le superviseur a tranché le 2026-10-28 (sluis#6) : **un seul réconciliateur,
Flux, dès la première machine.** ADR-026 admettait déjà un démarrage à un nœud.

## Décision

**Recette VPS (démarrage) : K3s à un nœud.**

- **Ansible** pose le socle de la machine : système, durcissement, LUKS, pare-feu, agents de supervision, **et installe K3s** (ADR-032).
- **Flux** réconcilie le cluster avec le dépôt Git. Il déchiffre les secrets par son intégration SOPS (ADR-016).
- **L'API du cluster n'est joignable que par un tunnel privé.** Aucune entrée publique autre que celle des applications (ADR-014).
- **L'agent n'a jamais accès au cluster** : ni jeton, ni compte, ni réseau, ni accès en écriture à la branche que Flux suit. H1-Recette (sluis#7) est sans objet : il n'y a plus de jeton Coolify à borner.
- Flux n'applique que ce que **la passerelle d'approbation** (ADR-031) a laissé passer. Les deux portes se cumulent : approbation d'abord, réconciliation ensuite.

**Recette K3s (croissance) : le même cluster, à trois nœuds.**

- **Ansible** prépare deux nœuds de plus et les joint au cluster.
- **Flux** ne change pas, ni les manifestes : seuls les réplicas et la base (ADR-026, trois instances sur trois nœuds) suivent. Jamais deux nœuds (ADR-026).

**Les deux formes exposent les mêmes outils MCP** (ADR-030). La skill Foyer
demande « mettre en ligne », « état », « restaurer », « rapport » ; seule la
taille du cluster change. C'est le principe des **adaptateurs enfichables** de
`foyer/KITS.md` appliqué à la plateforme : le port est l'outil MCP, Flux en est
l'unique adaptateur.

**Place de Flux dans `convergence-iac.md`.** La skill distingue deux moteurs :
la configuration et le durcissement (salt-ssh dans la skill, Ansible ici, voir
ADR-032), et le **déploiement applicatif GitOps**. Flux occupe le second rôle :
il est le **réconciliateur** qui applique la cible déclarée par la branche. Il
n'est pas un moteur de configuration système et ne doit pas le devenir. La
preuve de convergence reste celle de la skill : l'état déployé correspond au
commit approuvé, vérifié par la barrière (ADR-021) et par un contrôle de dérive
(ADR-025).

## Alternatives écartées

- **Coolify sur VPS, puis Flux sur K3s** (première version de cet ADR). Démarrage plus léger, mais deux réconciliateurs à apprendre, sécuriser et superviser, et une migration de plateforme à l'étape 8, base et fichiers compris. Coolify détenait en outre de larges droits (clés SSH des machines, registre, variables des applications) derrière une interface web à protéger.
- **K3s à trois nœuds dès le départ.** Trois machines, des sauvegardes etcd et une base répliquée pour un SaaS sans utilisateurs : contraire au principe 4. Le nœud unique garde ce principe sans changer de pile.
- **Docker Compose et un cron GitOps maison**, comme aujourd'hui sur la machine de travail. Suffisant pour une démonstration, mais sans historique de déploiement exploitable, sans vérification de signatures et sans séparation de l'outil qui détient les secrets.
- **Donner à l'agent un accès limité au cluster de test.** Toute porte vers l'outil qui détient la production est une porte de trop ; l'agent demande par l'outil MCP, il n'applique rien.

## Conséquences

- **K3s est à entretenir dès le premier SaaS** : montées de version, sauvegarde de la base du cluster, certificats internes. Ansible les rend rejouables (ADR-032), la dérive quotidienne les surveille (ADR-025).
- **Mémoire** : K3s et Flux prennent environ 1 à 2 Go de RAM de plus qu'un Compose. La plus petite machine de la recette VPS se dimensionne en conséquence.
- **Flux détient de larges droits dans le cluster**, et la clé age qui déchiffre les secrets (ADR-016). C'est une surface à surveiller : API du cluster derrière tunnel, mises à jour suivies (ADR-025), journal d'audit de l'API envoyé hors machine (ADR-023).
- **Sur un VPS, l'hébergeur garde un accès à la mémoire.** LUKS protège surtout contre le vol ou la réaffectation d'un disque, pas contre l'hébergeur lui-même. Les clés applicatives restent hors du VPS (ADR-017) et le chiffrement applicatif (ADR-018) limite ce qu'une lecture de disque révèle, sans annuler ce risque résiduel.
- **À un nœud, une panne du nœud est une indisponibilité** (ADR-026). C'est la limite acceptée au démarrage ; trois nœuds la lèvent.
- **Plus de migration de plateforme** : le passage à trois nœuds garde les manifestes, la base reste sous CloudNativePG (ADR-026) et les fichiers sur leur stockage (ADR-027). Kyverno et la configuration miroir des registres (ADR-038) deviennent possibles dès la première machine.
- **Décisions du 2026-10-23 reprises** : ADR-038 (double registre sous Coolify en recette VPS) et ADR-041 (Keycloak en Compose sous Coolify avant l'étape 8) sont amendés en conséquence.
- **Coût** : une machine au départ, K3s et Flux libres (Apache 2.0) ; trois nœuds à terme (jamais deux, ADR-026).

## Mesures CyFun 2025 couvertes

Mapping dans `foyer/skills/conformite.md` §2bis, couche **plateforme**.

- Séparation production / test : PR.IR-01.2 ★ et PR.IR-01.7 (Epic 9, story 9.6), avec ADR-014 et ADR-020.
- Pare-feu réseau actif : PR.IR-01.1 ★ (Epic 9, story 9.2).
- Accès à privilèges sur l'API du cluster (tunnel, certificats courts d'ADR-015) : PR.AA-03.2 ★B, PR.AA-05.7 I, PR.AA-03.4 E et PR.AA-05.9 E.

## Points ouverts

- **Comment Flux n'applique que l'approuvé** : branche de déploiement que seule la passerelle fait avancer, ou vérification par Flux de l'étiquette signée (ADR-031, point ouvert sur les formats de signature). À trancher avant la première mise en ligne.
- Choisir le tunnel privé (WireGuard ou équivalent) et l'endroit où il aboutit (machine de confiance, ADR-014).
- Fixer le critère qui déclenche le passage à trois nœuds (charge mesurée par Sluis, nombre de SaaS, exigence de disponibilité).
- Base du cluster à un nœud : SQLite intégrée (défaut de K3s) ou etcd dès le départ, pour que le passage à trois nœuds ne change pas de magasin.
