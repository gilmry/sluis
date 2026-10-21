# ADR-019 : Base de clés séparée et effacement cryptographique

- **Statut** : Proposé le 2026-10-21, non contresigné.
- **Amendé le 2026-10-21** (revue des ADR, décision 4) : la restauration de la base de clés est testée chaque nuit par le déchiffrement d'un échantillon (ADR-020). Amendement proposé, non contresigné.
- **Amendé le 2026-10-21** (revue des ADR, décision 6) : la base de clés est restaurée chaque nuit avec la seconde production éphémère (ADR-020), ce qui remplace la règle « la base de clés ne vient jamais en staging ». Amendement proposé, non contresigné.
- **Irréversible** : oui. Une fois des données chiffrées sous ce schéma, en sortir exige de tout rechiffrer ; et une clé effacée l'est pour de bon, c'est le but.
- **S'articule avec** : ADR-017 (OpenBao hors cluster, détient la clé maîtresse), ADR-018 (enveloppe applicative, une clé par personne), ADR-020 (la seconde production éphémère restaure la base de clés), ADR-026 (sauvegardes des données par archivage WAL).
- **Sert la vision (ADR-000)** : principe n°1, l'effacement d'une personne devient un geste unique et automatisable au lieu d'une chasse dans toutes les sauvegardes ; principe n°5, la preuve d'effacement est l'absence de la clé, constatable par un outil.

## Contexte

Le chiffrement applicatif (ADR-018) donne à chaque personne sa propre clé de
données, elle-même chiffrée par la clé maîtresse que détient OpenBao (ADR-017).
Reste une question que le RGPD rend concrète : **comment effacer une personne**
quand ses données vivent aussi dans des sauvegardes conservées des semaines,
répliquées dans une autre région, et que la base de données relationnelle ne
permet pas de réécrire le passé archivé ?

Effacer ligne par ligne dans chaque sauvegarde est impraticable et invérifiable.
Ne pas effacer les sauvegardes revient à garder les données d'une personne qui
a demandé leur suppression.

## Décision

1. **Les clés de données vivent dans une base séparée de celle des données.**
   Deux bases, deux cycles de vie : la **base de données** (le contenu chiffré)
   et la **base de clés** (les clés par personne, chiffrées par la clé
   maîtresse). Aucune requête applicative ne joint les deux : le backend lit la
   clé, puis la donnée, par deux ports distincts.
2. **Effacer une personne, c'est détruire sa clé.** Sans elle, ses données
   chiffrées deviennent illisibles partout où elles subsistent : base en
   service, réplique, archives WAL, sauvegardes hors site. C'est l'**effacement
   cryptographique**.
3. **Les sauvegardes de la base de clés sont conservées moins longtemps que
   celles des données.** Quand la dernière sauvegarde de clés contenant la clé
   détruite expire, l'effacement est complet dans tout le système, même si des
   sauvegardes de données plus anciennes subsistent. La durée de rétention des
   clés fixe donc le **délai maximal d'effacement**, annoncé tel quel.
4. **Chaque effacement est journalisé sans donnée personnelle** : identifiant
   technique de la clé, date de destruction, date de fin de rétention de la
   dernière sauvegarde qui la contient. Ce journal est la preuve, produite par
   l'outil, que l'effacement est en cours puis complet.
5. **La base de clés ne sort de la production que vers la seconde production
   éphémère** (ADR-020, amendement du 2026-10-21), qui la restaure chaque nuit
   depuis la sauvegarde hors site, l'utilise sous une règle OpenBao dédiée qui
   journalise chaque déchiffrement, puis la détruit avec elle. Un bac à sable
   (ADR-007) ne la reçoit jamais.
6. **Ce que la copie éphémère fait à l'effacement.** Elle restaure la base de
   clés de la dernière sauvegarde, donc **des clés détruites depuis cette
   sauvegarde**. Une personne effacée reste ainsi lisible dans la copie
   éphémère pendant le délai d'effacement annoncé (point 3), comme elle l'est
   déjà dans les sauvegardes. Le délai annoncé ne change pas ; la copie
   éphémère y est comptée, et disparaît chaque matin.

## Alternatives écartées

- **Effacement ligne par ligne dans les sauvegardes** : impraticable sur des
  archives WAL et des copies hors site immuables (ADR-026, ADR-027), et
  invérifiable.
- **Clés et données dans la même base** : une seule sauvegarde contiendrait les
  deux, et l'effacement cryptographique redeviendrait impossible tant que cette
  sauvegarde existe.
- **Ne pas sauvegarder la base de clés** : la perte d'un disque rendrait toutes
  les données illisibles. Le risque d'effacement total accidentel est pire que
  celui qu'on cherche à éviter.
- **Même rétention pour clés et données** : l'effacement ne serait complet
  qu'à l'expiration de la plus ancienne sauvegarde de données, souvent bien
  plus tard.

## Conséquences

- **Coût** : une seconde base à opérer, sauvegarder, surveiller et restaurer.
  Elle est petite (une clé par personne), mais c'est une brique de plus à
  entretenir, justifiée par le principe n°4 : sans elle, le droit à
  l'effacement n'a pas de réponse honnête.
- **Restauration en deux temps** : restaurer les données exige de restaurer
  aussi la base de clés à une date compatible. Une base de clés plus récente
  que les données est sans danger ; une base de clés plus ancienne rend
  illisibles les personnes créées entre-temps. La restauration nocturne
  (ADR-020) restaure ce couple ensemble, sans quoi elle ne prouverait rien,
  et **déchiffre un échantillon** avec la base de clés restaurée : c'est le
  test automatique de restauration des clés, avec son signal de succès et
  son alerte d'absence (ADR-025).
- **Fenêtre d'effacement non nulle** : entre la destruction de la clé et
  l'expiration de sa dernière sauvegarde, la clé existe encore hors ligne. Le
  délai se publie dans la politique de confidentialité, il ne se cache pas.
- **Limite assumée** : le backend réunit données et clés en mémoire au moment
  du traitement (ADR-018). Un attaquant présent sur le backend à ce moment lit
  en clair. L'effacement cryptographique protège le passé archivé, pas le
  présent compromis.
- **Limite assumée** : la perte simultanée de la base de clés et de toutes ses
  sauvegardes équivaut à la perte de toutes les données. C'est le revers exact
  de la propriété recherchée.

## Mesures CyFun 2025 couvertes

Mapping dans `foyer/skills/conformite.md` §2bis, couche application, ligne
*Données au repos* (PR.DS-01.x, GV.PO-01.2 I) : la politique de chiffrement
applicatif reçoit ici son volet cycle de vie. Le journal d'effacement est une
preuve au sens de §2ter. Les exigences de sauvegarde (PR.DS-11.x, Epic 9) et la
protection des données en transit (PR.DS-02.2 E) restent portées par ADR-020,
ADR-026 et ADR-027. L'effacement sur demande relève du RGPD plus que de
CyFun : catégorie PR.DS, identifiant à confirmer dans le livret.

## Points ouverts

- **Durées de rétention** : clés (proposition : 30 jours) et données
  (proposition : 90 jours). Elles fixent le délai d'effacement annoncé ; à
  trancher par le superviseur.
- **Où vit la base de clés** : schéma dédié dans le même cluster CloudNativePG
  (ADR-026) ou instance distincte. Une instance distincte coûte plus mais
  sépare aussi les sauvegardes à la source.
- **Effacement rejoué dans la copie éphémère** : faut-il appliquer à la copie,
  avant de l'ouvrir, les destructions de clés journalisées depuis la dernière
  sauvegarde (point 4), pour qu'une personne effacée n'y soit plus lisible ?
- **Effacement d'une personne co-titulaire** de données partagées (un document
  commun à plusieurs personnes) : quelle clé chiffre le partagé ? À trancher
  avec le modèle de données d'ADR-018.
