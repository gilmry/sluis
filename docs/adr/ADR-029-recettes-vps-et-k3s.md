# ADR-029 : Deux recettes, VPS puis K3s, derrière les mêmes outils MCP

- **Statut** : Proposé le 2026-10-21, non contresigné.
- **Amendé le 2026-10-21** (revue des ADR, décision 7) : identifiants CyFun 2025 complétés. Amendement proposé, non contresigné.
- **Irréversible** : oui pour la recette VPS en production : les données d'un SaaS y naissent, et en sortir est une migration (ADR-026, ADR-027). Le passage VPS → K3s est un point irréversible à valider le moment venu.
- **S'articule avec** : ADR-010 (OpenTofu loue, il ne configure pas), ADR-014 (trois zones), ADR-016 (secrets SOPS), ADR-030 (serveur MCP), ADR-031 (pipeline), ADR-032 (Ansible) ; sluis#6 (ordre des deux portes, tranché le 2026-10-17 : elles se cumulent) ; sluis#7 (réserve H1-Recette) ; `foyer/skills/convergence-iac.md`.
- **Sert la vision (ADR-000)** : principe 4, on commence petit (un VPS) et on ne paie la complexité d'un cluster que quand elle sert ; principe 1, le déploiement applicatif est confié à un réconciliateur, pas à un humain.

## Contexte

Un SaaS Foyer doit pouvoir démarrer sur une seule machine louée à bas coût, puis
grandir vers un cluster sans que la méthode, la skill Foyer ni l'agent changent
de gestes. Le dossier « Recette VPS » prévoit Coolify sur un VPS central ; la
production future vise OVH, en K3s au printemps-été 2027. Deux questions restaient
ouvertes : l'ordre entre l'approbation GitHub et Coolify (sluis#6), et si Sluis
peut détenir un jeton Coolify limité au test (H1-Recette, sluis#7).

## Décision

**Recette VPS (démarrage).**

- **Ansible** pose le socle de la machine : système, durcissement, LUKS, pare-feu, agents de supervision (ADR-032).
- **Coolify** déploie les applications. Son interface n'est joignable que par un **tunnel privé**, avec **double authentification**.
- **L'agent n'a jamais accès à Coolify** : ni jeton, ni compte, ni réseau. Cela tranche H1-Recette par la négative : le risque d'un jeton Coolify mal borné est supprimé plutôt que mesuré.
- Coolify n'est déclenché que **par la passerelle d'approbation** (ADR-031). Les deux portes se cumulent : approbation d'abord, Coolify ensuite.

**Recette K3s (croissance).**

- **Ansible** prépare les nœuds et installe K3s.
- **Flux** remplace Coolify : il réconcilie le cluster avec le dépôt Git.

**Les deux recettes exposent les mêmes outils MCP** (ADR-030). La skill Foyer
demande « mettre en ligne », « état », « restaurer », « rapport » ; seule
l'implémentation derrière le port change. C'est le principe des **adaptateurs
enfichables** de `foyer/KITS.md` appliqué à la plateforme : le port est l'outil
MCP, Coolify et Flux en sont deux adaptateurs.

**Place de Coolify dans `convergence-iac.md`.** La skill distingue deux moteurs :
la configuration et le durcissement (salt-ssh dans la skill, Ansible ici, voir
ADR-032), et le **déploiement applicatif GitOps**. Coolify occupe le second rôle
sur la recette VPS : il est le **réconciliateur** qui applique la cible déclarée
par la branche. Il n'est pas un moteur de configuration système et ne doit pas le
devenir. La preuve de convergence reste celle de la skill : l'état déployé
correspond au commit approuvé, vérifié par la barrière (ADR-021) et par un
contrôle de dérive (ADR-025).

## Alternatives écartées

- **K3s dès le départ.** Trois nœuds, un plan de contrôle, des sauvegardes etcd : trop d'entretien pour un SaaS sans utilisateurs. Contraire au principe 4.
- **Docker Compose et un cron GitOps maison**, comme aujourd'hui sur la machine de travail. Suffisant pour une démonstration, mais sans interface de retour arrière, sans historique de déploiement exploitable et sans séparation de l'outil qui détient les secrets.
- **Donner à l'agent un jeton Coolify limité au test.** Coolify ne garantissait pas, à la lecture, une limitation par projet assez fine (réserve H1-Recette) ; même bornée, toute porte vers l'outil qui détient la production est une porte de trop.
- **Coolify en recette K3s.** Deux réconciliateurs sur un même cluster se contredisent ; Flux est natif, déclaratif et sait vérifier des signatures (ADR-031).

## Conséquences

- **Coolify détient de larges droits** : clés SSH des machines, accès au registre, variables d'environnement des applications. C'est une surface d'attaque à surveiller : interface derrière tunnel et MFA, mises à jour suivies (ADR-025), journal d'accès envoyé hors machine (ADR-023), compte d'administration nominatif.
- **Sur un VPS, l'hébergeur garde un accès à la mémoire.** LUKS protège surtout contre le vol ou la réaffectation d'un disque, pas contre l'hébergeur lui-même. Les clés applicatives restent hors du VPS (ADR-017) et le chiffrement applicatif (ADR-018) limite ce qu'une lecture de disque révèle, sans annuler ce risque résiduel.
- **Migration VPS → K3s** : base de données (ADR-026) et fichiers (ADR-027) changent de support ; la recette K3s doit fournir sa procédure de bascule, testée sur le staging (ADR-020).
- **Coût** : un VPS et Coolify (libre, Apache 2.0) au départ ; trois nœuds au moins en K3s (jamais deux, ADR-026).

## Mesures CyFun 2025 couvertes

Mapping dans `foyer/skills/conformite.md` §2bis, couche **plateforme**.

- Séparation production / test : PR.IR-01.2 ★ et PR.IR-01.7 (Epic 9, story 9.6), avec ADR-014 et ADR-020.
- Pare-feu réseau actif : PR.IR-01.1 ★ (Epic 9, story 9.2).
- Accès à privilèges sur Coolify (tunnel, MFA) : PR.AA-03.2 ★B, PR.AA-05.7 I, PR.AA-03.4 E et PR.AA-05.9 E.

## Points ouverts

- Vérifier dans la documentation de Coolify la façon la plus étroite de le déclencher depuis la passerelle (webhook signé ou jeton de déploiement par application), et ce que ce jeton permet d'autre.
- Choisir le tunnel privé (WireGuard ou équivalent) et l'endroit où il aboutit (machine de confiance, ADR-014).
- Fixer le critère qui déclenche la bascule vers K3s (charge mesurée par Sluis, nombre de SaaS, exigence de disponibilité).
