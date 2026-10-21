# ADR-017 : OpenBao hors du cluster, gardien de la seule clé maîtresse

- **Statut** : Proposé le 2026-10-21, non contresigné.
- **Irréversible** : oui. La clé maîtresse chiffre toutes les clés de données (ADR-018) ; sa perte rend les données illisibles, et la migrer vers un autre gardien est une opération sur chaque enveloppe.
- **S'articule avec** : ADR-014 (zone de confiance), ADR-015 (accès par certificat step-ca et YubiKey), ADR-016 (les secrets de déploiement n'y vivent pas), ADR-018 (le backend lui demande d'envelopper et de désenvelopper), ADR-019 (la base de clés est séparée), ADR-023 (audit et alerte sur déchiffrements anormaux).
- **Sert la vision (ADR-000)** : règle « aucune brique ne détient tout », parce que la production détient les données chiffrées et la zone de confiance la clé qui les ouvre, jamais les deux ensemble ; principe n°3, parce que l'audit part hors machine et se croise avec les journaux applicatifs ; principe n°1, parce que la rotation est planifiée (ADR-025).

## Contexte

Le chiffrement applicatif (ADR-018) déplace la question sans la résoudre : il
faut bien une clé quelque part. Si elle vit dans le cluster qui porte les
données, un attaquant qui obtient l'un obtient l'autre. Si elle vit dans un
fichier de configuration, elle a le cycle de vie d'un secret de déploiement,
copiée partout.

OpenBao est le fork, sous licence MPL-2.0, de Vault, passé sous licence BSL :
même raisonnement de licence qu'ADR-010 pour OpenTofu.

## Décision

1. **OpenBao tourne sur la machine de confiance (zone 2), hors du cluster de
   production.** Il n'est pas joignable depuis Internet.
2. **Il ne détient que la clé maîtresse**, par son moteur de chiffrement en
   service (*transit*) : il chiffre et déchiffre des clés de données à la
   demande, il **ne stocke aucune donnée applicative** et ne laisse jamais sortir
   la clé maîtresse.
3. **Le backend s'authentifie par mTLS** avec un certificat step-ca qui lui est
   propre (ADR-015), avec une politique qui ne permet que d'envelopper et de
   désenvelopper, pour son seul SaaS.
4. **Accès d'administration distincts de ceux du cluster.** Administrer le
   cluster ne donne aucun droit sur OpenBao, et l'inverse. L'administration
   d'OpenBao exige une authentification par YubiKey ; son déverrouillage
   (*unseal*) repose sur des parts conservées hors ligne (zone 3).
5. **L'audit est envoyé hors machine.** Chaque opération est journalisée vers le
   stockage de journaux de la supervision et vers une copie en ajout seul en
   zone 3 ; un déchiffrement anormal (volume, horaire, appelant) déclenche une
   alerte (ADR-023).
6. **Rotation planifiée** de la clé maîtresse (ADR-025) : nouvelle version pour
   les nouvelles enveloppes, re-enveloppement progressif des anciennes.

## Alternatives écartées

- **OpenBao dans le cluster.** Plus simple à déployer, mais l'administrateur du
  cluster atteindrait la clé, et la séparation données / clé n'existerait plus.
- **Vault.** Licence BSL depuis 2023, même contradiction que Terraform (ADR-010).
- **KMS d'un hébergeur en nuage.** La clé de nos données chez un tiers, contre la
  souveraineté visée ; et une dépendance réseau externe pour chaque lecture.
- **Clé maîtresse dans un fichier chiffré par SOPS.** Elle serait déchiffrée et
  présente en mémoire de chaque instance du backend, sans audit des usages ni
  rotation simple.
- **Un HSM.** La bonne réponse à terme pour l'irréversible, mais un coût et une
  complexité sans proportion avec un porteur seul aujourd'hui (ADR-000,
  principe n°4). OpenBao peut l'adopter plus tard sans changer le port d'ADR-018.

## Conséquences

- **Coût** : un service de plus sur la machine de confiance ; une cérémonie de
  déverrouillage à chaque redémarrage, sauf déverrouillage automatique, qui
  déplace alors le secret.
- **Point de défaillance** : si OpenBao est injoignable, le backend ne peut plus
  ouvrir de nouvelles données chiffrées. Un cache court des clés de données
  désenveloppées en mémoire atténue la panne ; il augmente la fenêtre
  d'exposition (ADR-018). Le témoin externe doit voir OpenBao.
- **Entretien** : mises à jour d'OpenBao, rotation, test annuel de la procédure
  de déverrouillage et de restauration de son stockage.
- **Limite** : un backend compromis peut demander des déchiffrements tant que son
  certificat est valide. OpenBao borne et trace, il ne distingue pas une demande
  légitime d'une demande d'attaquant ; d'où l'alerte sur volume anormal.
- **Limite, sur VPS** : l'hébergeur garde un accès à la mémoire de la machine de
  confiance. OpenBao protège contre la compromission de la production, pas contre
  l'hébergeur.

## Mesures CyFun 2025 couvertes

Renvoi à `foyer/skills/conformite.md` §2bis :

- **PR.DS-01.x** et **GV.PO-01.2** : politique de chiffrement des données au
  repos, ici outillée et auditée.
- **PR.AA-05.3 ★** : la politique d'OpenBao limite chaque backend à son SaaS et à
  deux opérations.
- Gestion du cycle de vie des clés cryptographiques : catégorie **PR.DS**,
  identifiant à confirmer dans le livret.
- Journalisation et détection d'usage anormal : **DE.CM-09.1** (Epic 9, story
  9.5) pour la surveillance, catégorie **DE.AE** pour l'analyse, identifiant à
  confirmer dans le livret.

## Points ouverts

- **Déverrouillage** : manuel par parts (plus sûr, mais chaque redémarrage
  réclame le superviseur, contre ADR-000) ou automatique par une clé tierce (qui
  devient le nouveau secret critique) ?
- **Durée du cache** des clés de données côté backend : à fixer par mesure.
- **Stockage d'OpenBao lui-même** et sa sauvegarde : il porte la clé maîtresse,
  sa perte est la perte des données (ADR-019).
