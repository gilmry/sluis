# ADR-040 : Gestion des clés matérielles, trois YubiKeys, une copie papier, une station hors ligne

- **Statut** : Proposé le 2026-10-22, non contresigné.
- **Décision du superviseur du 2026-10-23**, intégrée au point 2 : le destinataire SOPS humain est la sous-clé de chiffrement GPG (amende ADR-016 sur ce point).
- **Amende ADR-015** : trois YubiKeys au lieu de deux (point 1 d'ADR-015, « deux YubiKeys : une d'usage, une de secours »). Le reste d'ADR-015 est inchangé.
- **Irréversible** : en partie. **Oui** pour ce que les clés ancrent : la clé GPG maîtresse, la racine de step-ca et les identités qui déchiffrent les secrets et les sauvegardes, déjà qualifiées d'irréversibles par ADR-015, ADR-016 et ADR-035 (proposé) ; une copie papier perdue ou un partage à seuil distribué ne se rattrape pas, il se remplace par une cérémonie complète. **Non** pour le nombre de clés, leurs lieux et la cadence de vérification, qui se révisent sans re-signer quoi que ce soit.
- **S'articule avec** : ADR-013 (clé SSH de signature `ed25519-sk` des approbations, `allowed_signers` de chaque cible), ADR-014 (les YubiKeys et leur copie de secours vivent en zone 3), ADR-015 (qu'il amende), ADR-016 (destinataires SOPS et age), ADR-017 (parts de secours d'OpenBao, en zone 3), ADR-025 (signal de succès des tâches récurrentes), ADR-033 (étape 1) ; ADR-035 (proposé) (sauvegardes de la zone 2 chiffrées vers les seules identités humaines) ; ADR-037 (proposé) (cérémonie hors ligne, phase 1 de l'amorce, ancres en zone 3).
- **Sert la vision (ADR-000)** : principe n°2, parce que chaque clé rangée est vérifiée à date fixe et qu'une vérification manquée alerte ; principe n°3, parce qu'une clé de secours ne vaut que par un essai réussi, pas par sa présence dans un tiroir ; principe n°4, parce que le partage à seuil reste une option tant que son coût n'est pas justifié ; règle « aucune brique ne détient tout », parce qu'aucun lieu ne réunit une clé et ce qui l'ouvre.

## Contexte

ADR-015 donne au superviseur deux YubiKeys, une d'usage et une de secours. Deux
objets couvrent la perte d'un seul. Ils ne couvrent pas la période qui suit :
dès qu'une clé est perdue, la seconde devient l'unique, et elle doit à la fois
servir tous les jours et rester à l'abri, deux exigences contraires, jusqu'à ce
qu'une remplaçante soit achetée et chargée. Ils ne couvrent pas non plus un
sinistre qui touche les deux lieux à la fois, ni la perte du support qui porte
la clé maîtresse.

ADR-015 laisse aussi ouverts la cérémonie de génération et la procédure de
perte. ADR-037 (proposé) fixe l'ordre de l'amorce et place la cérémonie hors
ligne en phase 1, sans dire sur quel poste ni ce qui en sort. ADR-035 (proposé)
chiffre les sauvegardes de la zone 2 vers les seules identités humaines : la
perte de toutes ces identités rend chaque sauvegarde inutilisable.

Dans sa lettre de mission du 2026-10-22, le superviseur demande trois YubiKeys
(une portée, une au coffre bancaire, une de secours), une copie papier scellée
chez une personne de confiance, une station hors ligne sur système live pour la
clé maîtresse GPG et la racine de step-ca, l'étude d'un partage de secret à
seuil, et les alternatives.

Une contrainte technique structure le reste : **tout ne se copie pas**. Les
sous-clés GPG sont générées une fois, hors ligne, puis chargées sur chaque
YubiKey. Une clé résidente FIDO2 (`ed25519-sk`, ADR-013) naît sur la YubiKey et
n'en sort pas : chaque YubiKey a donc la sienne. Il en va de même d'une identité
`age-plugin-yubikey`, générée dans la YubiKey.

## Décision

**1. Trois YubiKeys, trois rôles, trois lieux.** Toutes trois sont en zone 3
(ADR-014) : la clé privée ne quitte jamais son support, même quand la clé
portée est branchée sur le poste du superviseur.

| Clé | Rôle | Lieu | Branchée |
|---|---|---|---|
| **A, portée** | Usage courant : signature des commits, approbations (ADR-013), déchiffrement SOPS, accès d'urgence | Sur le superviseur | Tous les jours, sur son poste |
| **B, coffre** | Remplaçante immédiate de A ; sert aux cérémonies sur la station hors ligne | Un coffre bancaire | Aux vérifications et aux cérémonies seulement |
| **C, secours** | Survit à la perte simultanée de A et de B | Un troisième lieu, distinct du coffre et de la personne de confiance | Aux vérifications seulement |

Le choix des lieux, de la banque et de la personne de confiance appartient au
superviseur et n'est écrit dans aucun dépôt.

**2. Ce que porte chaque clé.**

| Élément | A | B | C | Se copie ? | Conséquence |
|---|---|---|---|---|---|
| Sous-clés GPG de signature, de chiffrement et d'authentification (ADR-015) | oui | oui | oui | oui, depuis la station hors ligne | Les trois clés sont interchangeables pour GPG ; la perte de l'une compromet les sous-clés des trois |
| Clé SSH de signature `ed25519-sk` (ADR-013) | la sienne | la sienne | la sienne | **non**, générée sur chaque YubiKey | Trois lignes dans chaque `allowed_signers` ; une perte ne retire qu'une ligne |
| Identité age, si `age-plugin-yubikey` est retenu (ADR-015, ADR-016) | la sienne | la sienne | la sienne | **non** | Trois destinataires dans chaque `.sops.yaml` et pour les sauvegardes d'ADR-035 (proposé) |
| Clé GPG maîtresse, racine de step-ca | non | non | non | sans objet | Elles ne vivent sur aucune YubiKey (point 3) |

ADR-015 laissait ouvert le choix entre la sous-clé de chiffrement GPG et une
identité `age-plugin-yubikey` comme destinataire SOPS. **Le superviseur l'a
tranché le 2026-10-23 : la sous-clé de chiffrement GPG**, parce que la clé
maîtresse restaurée suffit alors à tout relire, même après la perte des trois
YubiKeys (Conséquences). Cet ADR amende donc aussi ADR-016 sur ce point ; la
ligne `age-plugin-yubikey` du tableau ci-dessus ne s'applique pas.

**3. Une station hors ligne sur système live.** Les opérations sur les ancres
se font sur un poste démarré depuis un système live en lecture seule, sans
réseau, qui ne conserve rien entre deux démarrages :

- génération de la clé GPG maîtresse et de ses sous-clés, chargement des
  sous-clés sur A, B et C, prolongation de leur échéance, révocation ;
- génération de la racine de step-ca et signature de l'intermédiaire
  (ADR-015, ADR-037 (proposé), phase 4) ;
- impression de la copie papier.

La clé maîtresse GPG et la racine de step-ca vivent, chiffrées par phrase de
passe, sur des supports amovibles qui ne se branchent qu'à cette station, et
qui sont rangés au coffre avec B. La phrase de passe n'est jamais rangée avec
eux. La clé `ed25519-sk` et une identité `age-plugin-yubikey` se génèrent sur la
YubiKey elle-même, depuis la station ou depuis le poste du superviseur : elles
ne passent par aucun disque.

**4. Une copie papier scellée chez une personne de confiance.** Elle permet de
reconstruire la clé maîtresse si les supports du coffre sont perdus.

- **Elle contient** : la sortie de `paperkey` pour la clé maîtresse GPG (selon
  sa documentation, elle n'extrait que les octets secrets et reste chiffrée par
  la phrase de passe de la clé) ; le certificat de révocation de la clé
  maîtresse ; les empreintes de la clé GPG, des trois clés `ed25519-sk`, des
  identités age et de la racine de step-ca, pour vérifier une restauration ; la
  date et la signature du superviseur sur le scellé.
- **Elle ne contient jamais** : aucune phrase de passe, aucun PIN ni code de
  déblocage des YubiKeys, aucune part de secours d'OpenBao, aucun identifiant
  d'accès au KMS (ADR-017), aucun secret en clair. Une copie papier et ce qui
  l'ouvre ne sont jamais au même endroit.
- **La personne de confiance ne peut rien en faire seule** : sans la phrase de
  passe, la copie ne livre rien. Elle garde un objet scellé et prévient le
  superviseur si le scellé est rompu ou l'enveloppe perdue.

**5. Perte ou vol.** Une clé perdue est traitée comme volée : on ne sait pas
qui la trouvera. Le nombre d'essais du PIN est borné, mais il protège mal
contre un PIN observé.

| Ce qui manque | Geste immédiat | Ensuite |
|---|---|---|
| **A** | Retirer la ligne `ed25519-sk` de A de l'`allowed_signers` de chaque cible ; B devient la clé portée | À la station : révoquer les sous-clés GPG, en générer de nouvelles, les charger sur B, C et une nouvelle clé ; si `age-plugin-yubikey`, retirer le destinataire de A, re-chiffrer (`sops updatekeys`) et changer les secrets qu'il pouvait lire (ADR-016) |
| **B** | Retirer la ligne de B ; vérifier l'état des supports de la clé maîtresse rangés avec elle | Mêmes gestes à la station ; si les supports ont disparu avec B, la clé maîtresse est réputée exposée sous forme chiffrée (point ouvert) |
| **C** | Retirer la ligne de C | Mêmes gestes à la station, avec A et B |
| **Copie papier** (perte, vol ou scellé rompu) | Aucun secret n'est en clair ; rien ne presse dans l'heure | Remplacer la clé maîtresse par une cérémonie complète, planifiée : changer la phrase de passe ne suffit pas, le papier garde l'ancienne protection |
| **A, B et C ensemble** | Accès par la console de l'hébergeur | Restaurer la clé maîtresse depuis le coffre ou le papier, charger de nouvelles sous-clés sur trois clés neuves, générer trois nouvelles `ed25519-sk` et les installer sur chaque cible (voir Conséquences pour les identités age) |

Chaque perte et chaque remplacement sont consignés, sans aucune valeur
secrète, dans le journal que tient la procédure.

**6. Vérification périodique.** Une clé rangée qui ne fonctionne plus est une
clé absente. Chaque vérification d'une clé consiste à : déverrouiller la carte
avec son PIN et relever le compteur d'essais ; signer un message et vérifier la
signature contre la clé publique ; déchiffrer un fichier SOPS témoin, chiffré
vers tous les destinataires humains ; signer avec sa clé `ed25519-sk` et la
vérifier contre l'`allowed_signers` d'une cible ; relever la date d'échéance
des sous-clés.

- **A** est vérifiée par l'usage, et par le même essai à chaque vérification
  des autres.
- **B et C** sont vérifiées au moins une fois par an, comme le prévoyait
  ADR-015 pour la clé de secours, et à chaque cérémonie.
- **La copie papier** est restaurée une fois à sa création, sur la station, et
  l'empreinte obtenue comparée à celle de la clé ; ensuite, seul le scellé se
  vérifie.
- **Les supports de la clé maîtresse** sont relus à la station lors de chaque
  prolongation des sous-clés.

Chaque vérification produit une trace datée, signée par la clé vérifiée. Une
tâche planifiée (ADR-025) attend cette trace : son absence à l'échéance alerte.

**7. Partage de secret à seuil : une option documentée, non décidée.** Voir
Alternatives et Points ouverts. Cet ADR ne le met pas en œuvre.

**8. Les gestes sont réservés au superviseur.** L'achat des clés, leur
configuration, la cérémonie, le dépôt au coffre, le choix des lieux et de la
personne de confiance sont faits par lui, en personne (ADR-037 (proposé),
invariants 1 et 2). Cet ADR décide ; la procédure, dans un dépôt privé sans
aucun secret, exécute.

**9. Place dans la feuille de route.** Étape 1 d'ADR-033, identités et clés.

## Alternatives écartées

- **Deux YubiKeys (ADR-015 actuel).** Moins chères, une cérémonie plus courte.
  Mais après une perte, la seule clé restante doit servir et rester à l'abri à
  la fois, et un sinistre sur deux lieux suffit à tout perdre. La troisième clé
  coûte de l'ordre de 50 à 75 euros et une ligne de plus par `allowed_signers`.
- **Clés logicielles chiffrées sur disque.** Gratuites et copiables à volonté.
  Mais la clé se copie aussi pour un attaquant : ADR-013 et ADR-015 demandent
  la présence physique d'un objet pour approuver, ce qu'un fichier ne prouve pas.
- **HSM.** La bonne réponse pour une autorité de certification à grande
  échelle, et l'objet que vise ADR-017 à terme. Pour un porteur seul, coût et
  entretien sans proportion (ADR-000, principe n°4) ; et il ne remplace pas une
  clé portée pour signer une approbation.
- **Gestionnaire de secrets en ligne.** Pratique pour retrouver un secret, mais
  il remet les ancres à un tiers et à un réseau, contre la zone 3 hors ligne
  d'ADR-014 et la souveraineté visée.
- **Autre marque de clé, dont Nitrokey (micrologiciel libre).** Cohérente avec
  la préférence pour le libre. Sa prise en charge d'OpenPGP, des clés résidentes
  `ed25519-sk` et d'`age` est **à évaluer** sur le modèle retenu avant tout
  achat ; ADR-015 a été écrit pour des YubiKeys. Écartée pour ce lot faute
  d'évaluation, pas sur le fond (point ouvert).
- **Partage de secret à seuil comme mécanisme principal, à la place de la
  copie papier.** Non décidé, présenté ici avec ses pour et contre :
  - *Ce qu'il partagerait* : la phrase de passe de la clé maîtresse et des
    supports du coffre (petit secret, le plus simple) ; ou la clé maîtresse
    elle-même (plus lourd) ; ou les parts de secours d'OpenBao, qu'OpenBao
    produit déjà en parts selon ADR-017.
  - *Qui détiendrait les parts* : en 2 sur 3, par exemple le superviseur, le
    coffre et la personne de confiance ; en 3 sur 5, deux personnes de plus.
  - *Pour* : la phrase de passe cesse d'exister dans une seule mémoire ; une
    reprise devient possible si le superviseur est indisponible ; aucune part
    seule ne livre rien.
  - *Contre* : plus de personnes à choisir, à informer et à revoir ; un risque
    de collusion qui n'existe pas aujourd'hui ; une reconstitution qui exige de
    réunir les détenteurs ; un outil de plus dont la pérennité engage des
    années ; une cérémonie de plus à exercer, rare donc oubliée.
  - *Outils* : `ssss` implémente le partage de Shamir (génération des parts et
    reconstitution), et il est empaqueté par les distributions courantes ; sa
    limite de taille de secret et son entretien sont **à évaluer**. `age` ne
    propose pas de partage à seuil : chaque destinataire déchiffre seul ; des
    greffons tiers sont **à évaluer**. `paperkey` n'en propose pas non plus.

## Conséquences

- **Coût** : trois YubiKeys (de l'ordre de 150 à 225 euros), une location de
  coffre bancaire, un support live et des supports amovibles, une demi-journée
  de cérémonie à confirmer à la première passe, une visite au coffre par an.
- **Entretien** : une vérification annuelle de B et de C, couplée à la
  prolongation des sous-clés pour ne faire qu'une visite ; trois lignes par
  `allowed_signers` au lieu de deux.
- **La perte d'une seule clé coûte une cérémonie.** Les sous-clés GPG étant
  communes aux trois, une clé perdue oblige à les révoquer et à recharger les
  deux autres, donc à aller au coffre. C'est le prix de clés interchangeables.
- **Perte totale et identités age.** Si le destinataire SOPS est la sous-clé de
  chiffrement GPG, la clé maîtresse restaurée depuis le coffre ou le papier
  rend tout lisible à nouveau. Avec `age-plugin-yubikey`, les trois
  identités auraient disparu avec les trois clés, et avec elles les secrets et
  les sauvegardes d'ADR-035 (proposé). C'est la raison du choix de la sous-clé
  GPG (décision du 2026-10-23).
- **Limite, la phrase de passe.** Sans partage à seuil, la phrase de passe de la
  clé maîtresse ne vit que dans la mémoire du superviseur. Le papier et les
  supports du coffre ne servent qu'à lui.
- **Limite, porteur seul.** Trois clés protègent contre la perte, pas contre
  l'absence de second regard ; la personne de confiance garde un objet, elle ne
  contrôle rien (mesure compensatoire, `foyer/skills/conformite.md` §2bis).

## Mesures CyFun 2025 couvertes

Renvoi à `foyer/skills/conformite.md` §2bis et §2ter :

- **PR.AA-03.2 ★** : authentification forte, matérielle, maintenue même après
  la perte d'une clé.
- **PR.AA-01.5 E** : désactivation des moyens d'authentification à risque, ici
  le retrait d'une clé perdue de chaque `allowed_signers` et la révocation des
  sous-clés.
- **PR.AA-05.6 I** : séparation des tâches par mesure compensatoire du porteur
  seul.
- Cycle de vie des clés : pas de mesure dédiée dans CyFun 2025 ; couvert
  indirectement par **GV.PO-01.2 I**, comme dans ADR-015 et ADR-017.
- Copie de secours des ancres, hors ligne et vérifiée : rapprochée de
  **PR.DS-11.1 ★** et **PR.DS-11.2**, que les ADR citent pour les sauvegardes ;
  leur application aux clés est **à confirmer**.

## Points ouverts

- **Critère de fin de l'étape 1 d'ADR-033** : il dit « un fichier SOPS se
  déchiffre avec chacune des deux YubiKeys ». Il faut l'amender en « chacune des
  trois », et y ajouter la vérification des trois clés `ed25519-sk` contre un
  `allowed_signers`. Ce serait un amendement d'ADR-033 ; de même pour le tableau
  de validation de `README.md` (« deux YubiKeys »).
- **Mentions de « deux YubiKeys » ailleurs** : ADR-016 (Conséquences) et, s'ils
  sont acceptés, ADR-035 et ADR-037 (proposés) parlent de deux YubiKeys. À
  mettre en cohérence à leur prochaine révision.
- **Racine de step-ca sur la copie papier** : faut-il y ajouter sa clé chiffrée,
  pour qu'elle survive à la perte des supports du coffre ? Sans cela, une perte
  du coffre oblige à une nouvelle racine et au ré-enrôlement de toutes les
  identités machines.
- **Supports de la clé maîtresse disparus avec B** : faut-il remplacer la clé
  maîtresse d'office, comme pour la copie papier, ou se fier à la phrase de
  passe ?
- **Partage à seuil** : retenir ou non l'option, sur quel secret, avec quel
  seuil, quels détenteurs et quel outil. Décision du superviseur.
- **Lieu de C** par rapport à la personne de confiance : deux lieux distincts
  sont supposés ici ; à confirmer.
- **Autre marque de clé** : évaluer une Nitrokey sur OpenPGP, `ed25519-sk` et
  `age`, au moins pour l'une des trois clés, avant l'achat.
- **Modèle et micrologiciel des YubiKeys** : la prise en charge de `ed25519-sk`
  et d'`age-plugin-yubikey` dépend de la version ; à vérifier avant l'achat.
- **Cadence des vérifications** : annuelle ici, comme ADR-015 ; à aligner, le
  cas échéant, sur les exercices de reprise d'ADR-036 (proposé).
- **Choix du système live** de la station hors ligne et manière de prouver
  qu'il est sans réseau : à fixer dans la procédure privée.
