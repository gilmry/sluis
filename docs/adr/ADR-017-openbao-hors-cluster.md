# ADR-017 : OpenBao hors du cluster, gardien de la seule clé maîtresse

- **Statut** : Proposé le 2026-10-21, non contresigné.
- **Amendé le 2026-10-21** (revue des ADR, décision 7) : identifiants CyFun 2025 complétés. Amendement proposé, non contresigné.
- **Amendé le 2026-10-21** (revue des ADR, décision 1) : déverrouillage automatique par le KMS d'OVHcloud, parts de secours hors ligne. Amendement proposé, non contresigné.
- **Irréversible** : oui. La clé maîtresse chiffre toutes les clés de données (ADR-018) ; sa perte rend les données illisibles, et la migrer vers un autre gardien est une opération sur chaque enveloppe.
- **S'articule avec** : ADR-014 (zone de confiance, parts de secours en zone 3), ADR-025 (redémarrages automatiques), ADR-015 (accès par certificat step-ca et YubiKey), ADR-016 (les secrets de déploiement n'y vivent pas), ADR-018 (le backend lui demande d'envelopper et de désenvelopper), ADR-019 (la base de clés est séparée), ADR-023 (audit et alerte sur déchiffrements anormaux).
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
   d'OpenBao exige une authentification par YubiKey.
5. **L'audit est envoyé hors machine.** Chaque opération est journalisée vers le
   stockage de journaux de la supervision et vers une copie en ajout seul en
   zone 3 ; un déchiffrement anormal (volume, horaire, appelant) déclenche une
   alerte (ADR-023).
6. **Rotation planifiée** de la clé maîtresse (ADR-025) : nouvelle version pour
   les nouvelles enveloppes, re-enveloppement progressif des anciennes.
7. **Déverrouillage automatique par le KMS d'OVHcloud** (amendement du
   2026-10-21). Au redémarrage, OpenBao se déverrouille (*auto-unseal*) en
   demandant au KMS d'OVHcloud d'ouvrir sa clé de déverrouillage. Aucun geste
   humain n'est requis, ce qui rend compatibles OpenBao et les redémarrages
   automatiques d'ADR-025.
   - **Le KMS d'OVHcloud ne détient que la clé de déverrouillage, jamais la
     clé maîtresse.** La clé maîtresse reste dans OpenBao et n'en sort pas
     (point 2). L'alternative « KMS d'un hébergeur en nuage », écartée plus
     bas, le reste pour la clé maîtresse.
   - **L'identifiant d'accès au KMS est un secret critique.** Ses droits se
     limitent à l'opération de déverrouillage ; chaque usage est journalisé
     hors machine (ADR-023).
   - **Des parts de déverrouillage de secours sont conservées hors ligne**
     (zone 3, ADR-014), pour un déverrouillage manuel si OVHcloud ou le compte
     est indisponible ou compromis.

## Alternatives écartées

- **OpenBao dans le cluster.** Plus simple à déployer, mais l'administrateur du
  cluster atteindrait la clé, et la séparation données / clé n'existerait plus.
- **Vault.** Licence BSL depuis 2023, même contradiction que Terraform (ADR-010).
- **KMS d'un hébergeur en nuage pour la clé maîtresse.** La clé de nos données
  chez un tiers, contre la souveraineté visée ; et une dépendance réseau
  externe pour chaque lecture. **Toujours écarté pour la clé maîtresse.** Le
  KMS d'OVHcloud n'est retenu que pour la clé de déverrouillage (point 7).
- **Déverrouillage manuel par parts à chaque redémarrage.** Chaque redémarrage,
  y compris ceux des correctifs automatiques (ADR-025), réclamerait le
  superviseur : contraire à ADR-000. Les parts restent, en secours seulement.
- **Clé maîtresse dans un fichier chiffré par SOPS.** Elle serait déchiffrée et
  présente en mémoire de chaque instance du backend, sans audit des usages ni
  rotation simple.
- **Un HSM.** La bonne réponse à terme pour l'irréversible, mais un coût et une
  complexité sans proportion avec un porteur seul aujourd'hui (ADR-000,
  principe n°4). OpenBao peut l'adopter plus tard sans changer le port d'ADR-018.

## Conséquences

- **Coût** : un service de plus sur la machine de confiance ; un KMS chez
  OVHcloud, sollicité à chaque redémarrage. La cérémonie de déverrouillage ne
  reste que pour le secours.
- **Le secret critique se déplace** des parts vers l'identifiant d'accès au
  KMS (point 7).
- **Point de défaillance** : si OpenBao est injoignable, le backend ne peut plus
  ouvrir de nouvelles données chiffrées. Un cache court des clés de données
  désenveloppées en mémoire atténue la panne ; il augmente la fenêtre
  d'exposition (ADR-018). Le témoin externe doit voir OpenBao.
- **Entretien** : mises à jour d'OpenBao, rotation, test annuel du
  déverrouillage manuel de secours et de la restauration de son stockage.
- **Limite** : un backend compromis peut demander des déchiffrements tant que son
  certificat est valide. OpenBao borne et trace, il ne distingue pas une demande
  légitime d'une demande d'attaquant ; d'où l'alerte sur volume anormal.
- **Limite, sur VPS** : l'hébergeur garde un accès à la mémoire de la machine de
  confiance. OpenBao protège contre la compromission de la production, pas contre
  l'hébergeur.
- **Limite : la production et le déverrouillage dépendent désormais du même
  fournisseur.** Une panne d'OVHcloud ou une suspension du compte peut à la fois
  arrêter la production et empêcher OpenBao de redémarrer seul. Les parts de
  secours hors ligne sont alors la seule sortie.

## Mesures CyFun 2025 couvertes

Renvoi à `foyer/skills/conformite.md` §2bis :

- **PR.DS-01.x** et **GV.PO-01.2** : politique de chiffrement des données au
  repos, ici outillée et auditée.
- **PR.AA-05.3 ★** : la politique d'OpenBao limite chaque backend à son SaaS et à
  deux opérations.
- Gestion du cycle de vie des clés cryptographiques : pas de mesure dédiée
  dans CyFun 2025 ; couverte indirectement par **GV.PO-01.2 I**, avec
  **PR.AA-01.5 E** pour la désactivation des moyens d'authentification à
  risque (révocation).
- Journalisation et détection d'usage anormal : **DE.CM-09.1 I** (Epic 9,
  story 9.5) pour la surveillance ; **PR.AA-01.5 E** (usages atypiques des
  moyens d'authentification) et **DE.AE-02.1 I** pour l'analyse.

## Points ouverts

- **Intégration avec le KMS d'OVHcloud** : par quel mécanisme de scellement
  OpenBao s'adresse-t-il à ce KMS (prise en charge native, KMIP, PKCS#11 ou
  intermédiaire) ? À vérifier sur la version retenue avant l'étape 2
  d'ADR-033.
- **Parts de secours et déverrouillage automatique** : avec un déverrouillage
  automatique, OpenBao (comme Vault) émet en principe des clés de récupération
  qui, seules, ne déverrouillent pas. Le mécanisme qui rend possible le
  déverrouillage manuel de secours (migration de scellement, second scellement)
  reste à établir et à tester.
- **Alerte sur l'usage de l'identifiant d'accès au KMS** hors d'un redémarrage
  connu, et lieu où cet identifiant est conservé sur la machine de confiance.
- **Durée du cache** des clés de données côté backend : à fixer par mesure.
- **Stockage d'OpenBao lui-même** et sa sauvegarde : il porte la clé maîtresse,
  sa perte est la perte des données (ADR-019).
