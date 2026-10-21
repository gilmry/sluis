# ADR-020 : Une seconde production éphémère, restaurée chaque nuit depuis la sauvegarde hors site

- **Statut** : Proposé le 2026-10-21, non contresigné.
- **Amendé le 2026-10-21** (revue des ADR, décision 6) : le staging pseudonymisé devient une seconde production éphémère, restaurée avec sa base de clés, protégée comme la production et détruite après usage ; la pseudonymisation est abandonnée ; tests de charge et recette fonctionnelle partent sur un environnement à part. Amendement proposé, non contresigné.
- **Irréversible** : non pour le mécanisme ; **oui** pour la décision de restaurer des données réelles, avec les clés qui les ouvrent, dans un second environnement : elle engage la responsabilité RGPD du porteur.
- **S'articule avec** : ADR-007 (**inchangé** : il continue de régir les bacs à sable éphémères, sans aucune donnée de production ; la seconde production n'en relève pas), ADR-017 (règle OpenBao dédiée), ADR-019 (la base de clés est restaurée avec les données, et ce que cela implique pour l'effacement), ADR-026 (sauvegardes WAL), ADR-027 (bucket propre à la copie éphémère), ADR-031 (répétition des releases et des migrations), Epic 9 story 9.4 (restauration éprouvée).
- **Sert la vision (ADR-000)** : principe n°3, une sauvegarde déclarée réussie ne prouve rien, sa restauration nocturne si ; principe n°1, la restauration, geste rare et facile à oublier, devient quotidienne ; principe n°5, la durée mesurée est la preuve.

## Contexte

Une sauvegarde qui n'a jamais été restaurée est une hypothèse. Sur
l'infrastructure actuelle, plusieurs automatismes ont échoué en silence avant
qu'on s'en aperçoive ; rien ne garantit qu'une sauvegarde échappe à ce sort.
Par ailleurs, une release ou une migration de schéma n'est vraiment répétée que
sur les données qu'elle va toucher.

Les bacs à sable d'ADR-007 ne conviennent pas : ils sont éphémères, bornés à
six heures et vingt euros, et leur condition 4 interdit toute donnée de
production. Le superviseur a tranché le 2026-10-21 : l'environnement de
restauration est distinct, hors du champ d'ADR-007.

La première version de cet ADR prévoyait un staging permanent et pseudonymisé,
privé de la base de clés. La revue des ADR du 2026-10-21 l'a remplacée : une
copie privée de ses clés ne prouve pas que les clés se restaurent, une
pseudonymisation s'entretient colonne par colonne et laisse passer ce qu'on
oublie de déclarer, et une migration répétée sur des données transformées ne
répète pas celle de la production.

## Décision

1. **Chaque nuit, la production est restaurée ailleurs, depuis la sauvegarde
   hors site, avec sa base de clés** (ADR-019). Pas d'une copie directe de la
   production : c'est la sauvegarde réelle qui est éprouvée, avec son chemin de
   récupération réel. Cette copie est à la fois **le test de restauration** et
   **l'environnement de répétition** des migrations et des releases. Le
   staging devient cette seconde production : dans les autres ADR, « le
   staging » la désigne.
2. **Une release ou une migration en attente est jouée sur cette copie avant
   la vraie production** (ADR-031).
3. **Données réelles, donc protégée comme la production** : mêmes accès, même
   journalisation, même traitement RGPD. Ce n'est pas un environnement de test
   ouvert aux essais.
4. **Aucune sortie réseau vers des tiers** (Instagram, TikTok…) : les vrais
   jetons restaurés avec la production ne peuvent rien appeler.
5. **Éphémère** : restaurée, testée, vérifiée, puis **détruite**. Rien n'y
   survit d'une nuit à l'autre.
6. **Une règle OpenBao dédiée** (ADR-017) donne à la copie l'accès aux clés ;
   chaque déchiffrement y est journalisé.
7. **La durée de restauration est mesurée et consignée** : début, fin, volume,
   commit de l'outil, verdict d'intégrité. Ce relevé est la preuve de
   restauration, produite par l'outil (ADR-022), et la mesure du délai de
   reprise réel.
8. **Échec bruyant** : une restauration qui échoue, qui dépasse sa durée
   habituelle de façon anormale, ou qui **ne s'exécute pas** déclenche une
   alerte (ADR-023, ADR-025). L'absence du relevé du matin est elle-même une
   alerte.
9. **Les tests de charge et la recette fonctionnelle se font ailleurs**, sur un
   environnement à part, avec des comptes de test et des données factices.
   Aucune donnée réelle n'y entre.

## Alternatives écartées

- **Staging permanent pseudonymisé, sans la base de clés** (première version de
  cet ADR) : ne prouve pas que les clés se restaurent, laisse passer en clair
  toute colonne oubliée, et ne répète pas une migration sur les données
  qu'elle touchera.
- **Environnement de restauration isolé, séparé de la répétition** : un
  environnement de plus pour un test que la seconde production couvre déjà.
- **Staging alimenté par des jeux synthétiques uniquement** : ne prouve pas que
  la sauvegarde se restaure. Retenu, en revanche, pour la charge et la recette
  fonctionnelle (point 9).
- **Copie directe de la base de production** : ne teste pas la sauvegarde,
  qui est précisément ce qu'on veut éprouver.
- **Restauration dans un bac à sable d'ADR-007** : contredit sa condition 4.
  Assouplir ADR-007, contresigné, pour un besoin qui peut vivre ailleurs, a été
  écarté par le superviseur.
- **Restauration hebdomadaire ou mensuelle** : une sauvegarde cassée resterait
  invisible des jours ; la quotidienne aligne la détection sur la cadence des
  sauvegardes.

## Conséquences

- **Coût** : un environnement de la taille de la production, chaque nuit le
  temps de la restauration et des vérifications, plus le transfert depuis le
  site de sauvegarde ; et un environnement à part, modeste, pour la charge et
  la recette.
- **La surface des données réelles double, la nuit.** C'est le prix assumé :
  la copie est protégée comme la production (point 3), sans sortie vers des
  tiers (point 4), et détruite au matin (point 5).
- **Effacement** : la copie restaure la base de clés de la dernière sauvegarde,
  donc des clés détruites depuis. Une personne effacée reste lisible dans la
  copie éphémère pendant le délai d'effacement annoncé (ADR-019).
- **Déchiffrements nocturnes** : la règle OpenBao dédiée les distingue de ceux
  de la production, pour que l'alerte sur déchiffrements anormaux (ADR-023) ne
  les confonde pas.
- **Dépendance au site de sauvegarde** : une panne de la région de sauvegarde
  bloque la restauration. C'est acceptable, et c'est même un signal : la
  reprise aurait échoué de la même façon.

## Mesures CyFun 2025 couvertes

Story 9.4 de l'Epic 9 (`docs/bmad/04-epics-stories.md`) : PR.DS-11.2,
PR.DS-11.4, RC.RP-05.1. La séparation entre environnements de test et de
production recoupe la story 9.6 (PR.IR-01.2 ★) : la charge et la recette vivent
à part, la copie éphémère est traitée comme la production. Le relevé de durée
est une preuve au sens de `foyer/skills/conformite.md` §2ter, à archiver hors
serveur (ADR-022). La couche plateforme qui porte ces mesures est décrite dans
conformite.md §2bis.

## Points ouverts

- **Où tourne la copie éphémère** : chez quel hébergeur, dans quelle zone
  (ADR-014), et par quel moyen l'absence de sortie réseau vers des tiers est
  vérifiée (témoin : un appel sortant doit échouer).
- **Seuil d'anomalie de durée** : proposition, alerte au-delà de deux fois la
  médiane des sept dernières nuits.
- **Destruction non faite** : une copie qui survit au matin doit-elle alerter,
  et au bout de combien de temps ?
- **Base légale RGPD** de ce traitement, à documenter dans le registre du
  porteur (couche organisation), au même titre que la production.
- **Environnement de charge et de recette** : sa forme (bac à sable d'ADR-007
  ou environnement permanent à données factices) reste à fixer.
