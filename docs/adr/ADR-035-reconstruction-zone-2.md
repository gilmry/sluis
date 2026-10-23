# ADR-035 : Sauvegarde et reconstruction des briques communes

- **Statut** : Proposé le 2026-10-22, non contresigné.
- **Décision du superviseur du 2026-10-23**, intégrée au point 4 : les sauvegardes sont chiffrées vers la sous-clé de chiffrement GPG (ADR-040), et non par `age`.
- **Irréversible** : en partie. **Oui** pour le stockage verrouillé en écriture (ce qui y entre ne s'efface pas avant la fin de sa rétention, comme pour ADR-022) et pour le choix des identités qui déchiffrent (les perdre toutes rend chaque sauvegarde inutilisable). **Non** pour l'ordre de reconstruction et la procédure, révisés après chaque exercice.
- **S'articule avec** : ADR-014 (zone 2 à reconstruire, zone 3 en ajout seul), ADR-015 (racine step-ca hors ligne, YubiKeys, accès d'urgence), ADR-016 (secrets de déploiement lisibles depuis Git), ADR-017 (OpenBao, KMS d'OVHcloud, parts de secours, rotation), ADR-019 (perte du stockage d'OpenBao = perte des données), ADR-022 et ADR-027 (stockage verrouillé dans une autre région), ADR-023 (supervision, témoin externe), ADR-025 (signal de succès), ADR-026 (sauvegardes de la zone 1, à ne pas mélanger), ADR-032 (socle rejoué depuis Git) ; **ADR-036** (exercices de reprise trimestriels) pour la preuve ; **ADR-037** (problème de l'amorce) pour les secrets nécessaires avant que le coffre existe.
- **Sert la vision (ADR-000)** : principe n°1, la reconstruction se rejoue depuis Git au lieu de se refaire de mémoire ; principe n°2, une sauvegarde muette alerte ; principe n°3, une sauvegarde ne vaut que par sa restauration (ADR-036) ; règle « aucune brique ne détient tout », aucune sauvegarde n'est rangée avec ce qui l'ouvre.

## Contexte

La zone 2 (ADR-014) porte les briques communes à tous les SaaS : OpenBao et la
clé maîtresse (ADR-017), l'autorité de certification step-ca (ADR-015), la
supervision et ses journaux (ADR-023). Les deux Sluis en dépendent pour leurs
certificats et leurs journaux, l'exécutant tournant sur un VPS séparé
(ADR-010).

Les ADR existants disent ce que fait chaque brique, pas comment elle survit à
la perte de sa machine. ADR-017 renvoie la sauvegarde d'OpenBao à un point
ouvert, alors qu'ADR-019 rappelle que la perte de son stockage est la perte de
toutes les données chiffrées. Le déverrouillage automatique par le KMS
d'OVHcloud ajoute une question : OpenBao émet alors des clés de récupération
qui, seules, ne déverrouillent pas.

Suspension du compte, disque perdu, erreur d'automatisation, compromission qui
oblige à tout jeter : il faut savoir, avant, ce qui revient de Git, ce qui
revient d'une sauvegarde, et dans quel ordre.

## Décision

**1. Deux familles.**

| Famille | Contenu | Comment elle revient |
|---|---|---|
| **Se reconstruit depuis Git** | La machine (OpenTofu) et le socle (Ansible, ADR-032) ; la configuration de step-ca hors clés ; la configuration déclarative d'OpenBao (politiques, authentification, destinations d'audit) ; la supervision entière (règles, routes, tableaux de bord) ; la configuration des deux Sluis ; les secrets SOPS (ADR-016) | Rejouée. Seul le dépôt doit exister hors de la zone 2 |
| **Se restaure, état unique** | Le stockage chiffré d'OpenBao (clé maîtresse et ses versions) ; la clé intermédiaire step-ca, sa base et ses clés de signature SSH ; les journaux de référence (audit d'OpenBao en ajout seul, zone 3) | Restaurée depuis une sauvegarde, ou relue sur place en zone 3 pour les journaux. Jamais régénérée en silence : une clé maîtresse neuve n'ouvre rien de l'existant |

Les métriques et journaux de diagnostic de la supervision ne font pas foi :
leur perte coûte de l'historique, pas de preuve.

**2. Ce qui est sauvegardé.** OpenBao : un instantané chaque jour **et après
chaque rotation de la clé maîtresse** ou changement de politique, car une
version absente des sauvegardes rend irrécupérables les enveloppes qu'elle a
produites. step-ca : la clé intermédiaire (chiffrée par son mot de passe), sa
base et ses clés SSH, chaque jour ; la racine est déjà hors ligne (ADR-015).
Les journaux de référence s'écrivent en zone 3 au fil de l'eau : rien à
sauvegarder, seulement un flux à rétablir. Chaque sauvegarde émet un **signal
de succès** (ADR-025) dont l'absence alerte, et que le témoin externe voit
manquer si la supervision est tombée.

**3. Où.** En zone 3, dans un stockage objet **d'une autre région**, avec
**versioning et verrouillage en écriture**. La zone 2 y écrit avec un
identifiant qui ne peut ni lire, ni écraser, ni supprimer.

**4. Qui déchiffre.** Chaque sauvegarde est chiffrée avant l'envoi **vers la
seule sous-clé de chiffrement GPG du superviseur** (décision du 2026-10-23,
ADR-040), chargée sur ses YubiKeys. La machine de confiance ne peut pas relire
ses propres sauvegardes. Restaurer exige une YubiKey, ou à défaut la clé
maîtresse GPG restaurée hors ligne : un acte humain, rare et irréversible
(ADR-000).

**5. Jamais au même endroit que ce qu'il protège.**

- Le stockage d'OpenBao, jamais avec l'identifiant d'accès au KMS, les clés de
  récupération ou les parts de secours : réunis, ils livrent la clé maîtresse.
- La clé intermédiaire step-ca, jamais avec son mot de passe.
- Les sauvegardes de la zone 2, jamais dans le même compte ni avec les mêmes
  identifiants que celles des données et de la base de clés de la zone 1
  (ADR-019, ADR-026, ADR-027) : la règle d'ADR-017 vaut aussi pour les copies.
- Les identités qui déchiffrent, jamais dans le stockage des sauvegardes ni
  sur la machine de confiance.

**6. Ordre de reconstruction.** On ne passe à l'étape suivante qu'une fois son
critère vérifié.

0. **Préalables, tous hors de la zone 2** : une YubiKey, l'accès aux dépôts
   Git et aux sauvegardes, le KMS ou les parts de secours, les secrets
   d'amorce (ADR-037).
1. **Machine et socle**, par la clé GPG d'urgence (ADR-015) puisque les
   certificats SSH courts n'existent pas encore. *Critère* : second passage
   Ansible à `changed=0`.
2. **step-ca restaurée.** *Critère* : un certificat SSH court émis est
   accepté par une machine existante.
3. **Supervision reconstruite**, avant OpenBao, qui doit trouver une
   destination d'audit vivante avant de servir. *Critère* : toutes les cibles
   collectées, le témoin externe reçoit de nouveau l'alerte de vie.
4. **OpenBao restauré et déverrouillé**, audit rétabli vers la zone 3.
   *Critère* : un échantillon de clés de données se désenveloppe, comme chaque
   nuit dans la seconde production éphémère (ADR-020).
5. **Raccordements** : certificats mTLS du backend et des deux Sluis, envoi
   des journaux. *Critère* : un déploiement de test passe la barrière
   (ADR-021).
6. **Clôture** : première sauvegarde de la zone reconstruite reçue, durée de
   chaque étape consignée.

Si la zone 2 est **soupçonnée compromise**, l'étape 2 réémet une
intermédiaire depuis la racine hors ligne au lieu de restaurer l'ancienne, et
l'identifiant d'accès au KMS est remplacé avant l'étape 4.

**7. Délai de reprise : à mesurer.** Il n'est pas fixé ici ; le premier
exercice d'ADR-036 donne la durée réelle de chaque étape, et l'objectif
s'annonce sur cette mesure. Une borne existe déjà : pendant la perte, les SaaS
tournent (ADR-014) mais n'ouvrent plus de nouvelles données au-delà du cache
de clés (ADR-017), et leurs certificats mTLS expirent en quelques jours
(ADR-015).

**8. L'ADR décide, la procédure exécute.** La procédure pas à pas vit dans un
dépôt privé, **sans aucun secret**, avec une version imprimable lisible quand
plus aucune machine ne l'est. Chaque exercice d'ADR-036 la corrige.

## Alternatives écartées

- **Tout reconstruire, ne rien restaurer** : une clé maîtresse ou une autorité
  régénérée n'ouvre pas ce que l'ancienne a chiffré ou signé.
- **Image disque de la machine** : recopie ce qu'Ansible rejoue déjà, mêle
  état et configuration, et restaure aussi ce qu'une compromission a laissé.
- **Réplique chaude d'OpenBao dans une autre région** : une seconde machine de
  confiance à tenir, qui réplique aussi une suppression. Contraire au principe
  n°4 aujourd'hui ; à réévaluer avec la recette K3s.
- **Sauvegardes déchiffrables par la machine de confiance** : une restauration
  sans humain, mais une machine compromise relirait tout son historique.
- **Un seul stockage pour les zones 1 et 2** : un jeu d'identifiants donnerait
  les données chiffrées et la clé qui les ouvre.

## Conséquences

- **Coût** : quelques mégaoctets par jour dans un stockage verrouillé, sous un
  compte distinct de la zone 1 ; le coût dominant est celui des exercices.
- **Dépendance** : restaurer exige le superviseur et l'une de ses YubiKeys.
- **Perte bornée par la cadence** : un certificat ou une révocation émis
  depuis la dernière sauvegarde de step-ca est perdu, ce que la courte durée
  de vie rend tolérable ; l'instantané après rotation ferme la fenêtre la plus
  grave pour OpenBao.
- **Limite** : le verrouillage interdit de purger une sauvegarde ratée avant
  la fin de sa rétention.
- **Limite** : la perte simultanée de la zone 2, du KMS et des parts de
  secours équivaut à la perte des données, comme ADR-019 le dit de la base de
  clés.

## Mesures CyFun 2025 couvertes

Renvoi à `foyer/skills/conformite.md` §2bis, couche plateforme :

- **PR.DS-11.1 ★** : sauvegardes hors de la machine protégée ; **PR.DS-11.2**
  et **PR.DS-11.4** : protégées et éprouvées, preuve par ADR-036.
- **RC.RP-05.1** : intégrité vérifiée, critère par critère, avant le retour au
  service.
- **PR.PS-04.1 ★** et **PR.PS-04.3 I** : journaux de référence hors machine.
- **PR.AA-05.3 ★** : l'identifiant d'écriture ne peut ni lire ni supprimer.
- **GV.PO-01.2 I** : politique de cryptographie, appliquée aux copies de la
  clé maîtresse ; **PR.IR-04.1 I** : capacité de reprise.
- Plan de reprise ordonné et exécuté : catégorie RC.RP, identifiant à
  confirmer.

## Points ouverts

- **Voie de secours sans le KMS** (déjà ouverte par ADR-017) : tant que l'usage
  des parts de secours avec un déverrouillage automatique n'est pas établi et
  testé, la reconstruction sans OVHcloud n'est pas garantie.
- **Rotation et sauvegarde** : la nouvelle version de la clé maîtresse ne
  doit-elle servir qu'après l'archivage vérifié d'un instantané qui la
  contient ? Cela amenderait ADR-017, point 6.
- **Après compromission** : faut-il tourner la clé maîtresse et re-envelopper
  toutes les clés de données, et quel instantané reste digne de confiance ?
- **Cadence du test** : ADR-017 prévoit un test annuel de restauration,
  ADR-036 des exercices trimestriels. Amendement à prévoir, comme pour
  ajouter les sauvegardes de la zone 2 au tableau d'ADR-025.
- **Fournisseur de la zone 3** (ouvert par ADR-014 et ADR-026) : ni la panne ni
  le compte de l'hébergeur principal.
- **Rétention** des instantanés : sans clé par personne, ils ne retardent pas
  l'effacement (ADR-019) ; combien de versions garder ?
- **État propre de Sluis** : dépôt OAuth du Sluis public (amendement
  d'ADR-006), journaux d'audit des deux Sluis, états OpenTofu de l'exécutant
  (ADR-010). Ce n'est pas de la configuration, et leur sauvegarde n'est pas
  tranchée ici.
- **Journaux de référence** : les réinjecter dans la supervision reconstruite,
  ou les consulter en zone 3 quand une enquête l'exige ?
- **Accès à l'étape 1** : clé GPG d'urgence ou console de l'hébergeur, et
  comment cet accès est-il journalisé ?
