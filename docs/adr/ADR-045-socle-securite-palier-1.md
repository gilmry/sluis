# ADR-045 : Le socle sécurité commun entre au palier 1, en rôles Ansible publiés par Sluis

- **Statut** : Proposé le 2026-11-16, à contresigner.
- **Décisions du superviseur du 2026-11-16**, à l'origine de cet ADR : construire une seule fois dans Sluis le socle sécurité que chaque SaaS hérite (hôte, réseau, accès, sauvegardes, journaux, supervision, correctifs, procédures), plutôt que de le refaire par SaaS ; le premier serveur de production tourne en **Docker Compose sur un VPS**, K3s viendra plus tard ; le second facteur SSH du premier jour est un **TOTP**, dans tous les cas ; les écarts constatés sur les machines réelles se suivent **hors de ce dépôt public**.
- **Amende ADR-044** : le palier 1 (« comptes utilisateurs réels ») reçoit le socle du point 2. C'est la condition pour qu'une vraie donnée personnelle entre, avec ou sans comptes.
- **Amende ADR-040 et ADR-044 (palier 1)** : la clé `ed25519-sk` sur YubiKey reste la cible ; au premier jour, le second facteur SSH est un TOTP (point 4).
- **Répond au point ouvert d'ADR-026** (seconde région des archives) : une seconde région **en France**, chez OVHcloud (point 5).
- **Précise ADR-032** : où vivent les rôles et comment ils s'appliquent (point 3). Les règles 1 à 4 d'ADR-032 restent valables, sauf l'authentification, déjà remplacée par ADR-042, et le second facteur du point 4.
- **Ne change pas** le noyau de Sluis (ADR-044, point 1) : le socle appartient à la plateforme commune, au palier indiqué.
- **Irréversible** : en partie. Le durcissement d'un serveur de production et le choix du stockage des sauvegardes le sont (`foyer/skills/convergence-iac.md`). Le TOTP et la pile Compose sont provisoires par construction.
- **S'articule avec** : ADR-016 (SOPS et age), ADR-020 (restauration), ADR-022 (preuves datées), ADR-023 (supervision depuis un autre hôte), ADR-024 (agent IA), ADR-025 (signal de succès), ADR-026, ADR-029 (tunnel, point ouvert), ADR-032, ADR-040, ADR-042, ADR-044.
- **Sert la vision (ADR-000)** : principe n°2, chaque mesure se vérifie seule et alerte ; principe n°3, une sauvegarde ne compte que restaurée ; principe n°4, le socle n'arrive qu'avec la première vraie donnée, qui le justifie.

## Contexte

Le premier SaaS qui recevra de vraies données personnelles a des engagements
de sous-traitant RGPD (articles 28, 30, 32 et 33). Un second SaaS aura les mêmes
exigences. Les corriger dans chacun, c'est les écrire deux fois et les laisser
diverger.

ADR-044 place K3s, CloudNativePG et l'archivage des WAL au palier 2 (premier
revenu). Le premier serveur de production arrivera avant que ce palier soit
construit : il tourne en Docker Compose. Il lui faut pourtant, dès le premier
jour, des sauvegardes restaurables hors serveur, des journaux hors hôte, un
second facteur, une supervision depuis une autre machine et des procédures.

Le durcissement existe déjà sous forme de scripts sur la machine de
supervision, hors de ce dépôt. ADR-032 a choisi Ansible pour le socle, sans
dire où vivent les rôles ni comment un SaaS les reçoit.

## Décision

**1. Le socle est une condition d'entrée de la donnée réelle.** Une production
ne reçoit aucune vraie donnée personnelle tant qu'une case « premier jour » de
sa liste de contrôle reste ouverte. La liste est exécutable : chaque case a sa
vérification automatique et sa preuve datée, archivée hors du serveur
(ADR-022).

**2. Ce que le socle couvre.** Hôte et durcissement, pare-feu, accès,
sauvegardes, journaux et règles d'audit, supervision depuis un autre hôte,
correctifs, et les modèles de procédures (incident, registre du sous-traitant,
liste des sous-traitants, attestation de suppression). Il fournit aussi à chaque
SaaS un rôle de base sans superutilisateur, un emplacement de sauvegarde et une
cible de journaux. Le code des SaaS reste hors du socle.

**3. Forme : des rôles Ansible publiés par Sluis.**

| | Où | Qui |
|---|---|---|
| Rôles du socle | ce dépôt, `socle/`, publiés par **étiquette** de Sluis | écrits ici, testés en CI sur une machine jetable |
| Inventaire et variables d'un serveur | le dépôt du SaaS, place réservée `infrastructure/_shared/ansible/` (ADR-034) | le dépôt cible, qui référence une étiquette de Sluis |
| Application | le workflow de Tier 1 du dépôt cible, sur approbation humaine | jamais le serveur MCP (ADR-030) |
| Contrôle de dérive | exécution quotidienne en `--check --diff` (ADR-032, règle 3) | alerte et signal de succès (ADR-025) |

Le durcissement existant est repris **sans écart** : mêmes outils, mêmes
réglages. Il est réécrit en rôles idempotents, pas recopié en scripts.

**4. Second facteur SSH au premier jour : un TOTP.** Clé SSH personnelle et
code TOTP pour chaque connexion, `PermitRootLogin no`, aucun sudo sans mot de
passe pour un compte joignable à distance. La bascule vers `ed25519-sk` sur
YubiKey (ADR-040, ADR-042) est une story séparée, faite à l'arrivée des clés ;
le TOTP est alors retiré, pas gardé en parallèle.

**5. Sauvegardes, données et alertes en France.**

1. Sauvegarde quotidienne de la base et des fichiers de chaque SaaS, chiffrée
   côté client avant envoi, vers un stockage objet OVHcloud en France, dans une
   **région différente** de la production. La clé de chiffrement reste hors du
   serveur.
2. Le serveur ne peut pas effacer ses propres copies : identifiant d'écriture
   seule ou verrouillage objet. La rétention de 30 jours se purge depuis un
   poste autorisé ou par une règle de cycle de vie, jamais depuis le serveur.
3. Alerte si la dernière sauvegarde réussie a plus de 26 heures.
4. **Restauration** : une vérification automatique chaque semaine (ADR-044,
   palier 0), et une restauration complète chaque trimestre depuis la copie
   distante, sur une machine vierge. Seule la durée mesurée de cette dernière
   peut figurer dans un contrat.
5. Au palier 2, l'archivage continu des WAL (ADR-026) remplace la sauvegarde
   quotidienne de la base. Celle des fichiers demeure.
6. Aucune donnée personnelle ne quitte l'Union européenne, alertes comprises :
   un message d'alerte ne contient ni adresse IP ni identifiant de personne.

**6. Ce dépôt ne porte aucun état réel.** Il décrit des rôles, des règles et des
modèles. L'état réel des défenses d'une machine, ses écarts, ses adresses et ses
ports se suivent dans un dépôt privé.

## Alternatives écartées

- **K3s et CloudNativePG dès le premier serveur** : c'est la cible, mais elle
  repousse la première vraie donnée de plusieurs semaines et loue un nœud de
  plus avant le revenu. Contraire au principe n°4 et à ADR-044.
- **Garder les scripts de la machine de supervision** : ils marchent, mais ne
  sont ni idempotents ni testés sur une machine jetable, et vivent dans un dépôt
  qu'un autre SaaS ne peut pas référencer.
- **Attendre les YubiKeys** : conforme à ADR-040, mais aucune vraie donnée tant
  qu'elles ne sont pas livrées.

## Conséquences

- **Le palier 1 coûte plus cher** : environ 64 heures de construction estimées
  au 2026-11-16, à réestimer par épique.
- **La sauvegarde de la base s'écrira deux fois** : dump quotidien maintenant,
  WAL au palier 2.
- **Le TOTP est plus faible que FIDO2** : son secret vit sur le serveur, et un
  code se laisse hameçonner. Risque assumé jusqu'à l'arrivée des clés.
- **Sluis grossit au-delà du noyau.** Le socle n'en dépend pas, et le noyau ne
  dépend pas du socle (ADR-044, point 1).

## Mesures CyFun 2025 couvertes

Renvoi à `foyer/skills/conformite.md` §2bis. Le socle touche les sauvegardes et
la restauration, l'authentification, les journaux, la supervision, les
correctifs et la réponse aux incidents. Les identifiants exacts seront
rattachés story par story, **vérifiés contre §2bis** : ils n'y sont pas encore
tous au 2026-11-16. Le niveau Essential complet reste visé au palier 4.

## Points ouverts

- **Tunnel d'administration** (ADR-029) : lequel au premier jour ?
- **Administrateur suppléant** : qui reprend la main si le superviseur est
  indisponible ?
- **Prestataire de courriel** : garder un prestataire suisse, déclaré comme
  sous-traitant, ou envoyer depuis OVHcloud ?
- **VPS ou Public Cloud avec vRack** (ADR-014) : la pile Compose tranche le
  premier serveur (un VPS) ; la question reste ouverte pour le palier 2.
