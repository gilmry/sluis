# ADR-038 : Double registre d'images, référencées par empreinte et signées

- **Statut** : Proposé le 2026-10-22, non contresigné.
- **Décision du superviseur du 2026-10-23** : en recette VPS, le double registre reste sous Coolify (images par empreinte, repli manuel par la passerelle d'approbation) ; `registries.yaml` et Kyverno n'arrivent qu'avec K3s.
- **Irréversible** : non, pour l'essentiel. Une empreinte désigne un contenu, pas un emplacement : changer de registre ne change pas les références, et une image se recopie. Deux choix s'en approchent sans l'être tout à fait : l'**identité de signature**, dont le changement oblige à re-signer les images en service et à redéclarer l'identité attendue partout où elle est vérifiée ; et l'**ouverture du compte payant** chez OVHcloud, un engagement contractuel réservé au superviseur.
- **S'articule avec** : ADR-010 (le dépôt publie déjà ses images sur un registre public) ; ADR-011 (gate de scan avant tout envoi vers un registre) ; ADR-014 (trois zones) ; ADR-015 (identités machines, dont relèverait l'identité de signature) ; ADR-016 (les jetons de registre sont des secrets de déploiement) ; ADR-021 (contrôle « images signées » de la barrière) ; ADR-022 (preuves) ; ADR-027 (images tierces épinglées) ; ADR-028 (même artefact empreinté, promu sans reconstruction) ; ADR-029 (Coolify puis K3s) ; ADR-031 (l'image porte le hash du commit) ; ADR-033 (feuille de route, étapes 3 et 8) ; ADR-034 (contrat du dépôt cible).
- **Sert la vision (ADR-000)** : règle « aucune brique ne détient tout », parce que le registre stocke les images sans pouvoir les rendre acceptables, la signature venant d'ailleurs ; principe n°3, parce que la barrière croise l'empreinte demandée et la signature au lieu de croire le registre ; principe n°2, parce qu'une image non vérifiable bloque au lieu de passer ; principe n°4, parce que le registre principal est un service géré, pas une brique de plus à opérer.

## Contexte

Aujourd'hui, les images de ce dépôt partent sur un seul registre, GHCR, sous
deux étiquettes : le hash du commit et `latest` (`.github/workflows/docker-publish.yml`).
Les fichiers Compose du dépôt tirent `latest` par défaut. Une étiquette est
mobile : `latest` désigne demain une autre image qu'aujourd'hui, et même une
étiquette de commit peut être réécrite par qui détient le droit de pousser.
Ce qui tourne n'est alors prouvé par rien d'autre que la confiance dans le
registre.

Un registre unique est aussi un point de défaillance silencieux : tant qu'aucun
conteneur ne redémarre, sa panne ne se voit pas ; le jour d'un redémarrage,
d'un retour arrière ou d'un nouveau nœud, l'image ne se tire plus. La
production future vise OVHcloud ; un registre chez le même fournisseur partage
ses pannes et une éventuelle suspension du compte, limite déjà relevée par
ADR-017 pour le déverrouillage d'OpenBao.

La barrière d'ADR-021 prévoit déjà un contrôle « images signées » par cosign,
sans dire où vivent les images ni comment elles sont référencées. Le
superviseur a demandé, par lettre de mission du 2026-10-22, de le fixer.

## Décision

1. **Deux registres.** Le registre **principal** est le registre privé géré
   d'OVHcloud (*Managed Private Registry*, fondé sur Harbor). Le registre de
   **secours** est GHCR. Une image destinée à la production est présente dans
   les deux, sous **la même empreinte**.
2. **Références par empreinte, jamais par étiquette mobile.** Tout ce qui
   désigne une image à déployer (manifeste, configuration de la recette,
   plan de mise en ligne) la nomme par son empreinte `sha256`. Une étiquette
   peut accompagner l'empreinte pour la lecture humaine ; elle ne fait jamais
   foi. Le repli vers le registre de secours n'a de sens que parce que
   l'empreinte est la même des deux côtés : c'est elle qui garantit que
   l'image de secours est bien celle qui a été approuvée.
3. **Signature cosign** de chaque image destinée à la production, par la
   chaîne de build, sur son empreinte.
4. **Vérification à la barrière** (ADR-021, contrôle « images signées »),
   avant la demande d'approbation et à nouveau après elle (ADR-021, point 6).
   Une signature absente, invalide, ou émise par une autre identité que
   l'identité attendue bloque. Une signature introuvable parce que le registre
   ne répond pas est une indétermination, donc un échec (ADR-021, point 4).
5. **En recette K3s**, deux mécanismes s'ajoutent : la **configuration miroir
   de K3s** (`registries.yaml`), qui fait tirer les images depuis le registre
   principal puis, à défaut, depuis le secours, sans changer la référence ; et
   une **vérification à l'admission par Kyverno**, qui refuse dans le cluster
   une image non signée ou non référencée par empreinte.
6. **Place dans la feuille de route** : cet ADR ne bloque pas les étapes 1 à 3
   d'ADR-033 et se construit après elles. Son étape exacte reste ouverte.

**En recette VPS, sous Coolify** (décision du superviseur du 2026-10-23). Le
double registre reste sous Coolify jusqu'à l'étape 8 : chaque application
Coolify désigne son image par empreinte (`image@sha256:…`, forme prise en
charge selon la documentation de Coolify, vérifiée le 2026-10-23). Le repli
vers le registre de secours n'est **pas automatique** : il consiste à changer
le registre désigné, à empreinte égale, et ce changement de configuration de
la cible passe par la passerelle d'approbation (ADR-031). C'est la limite
acceptée avant K3s.

**Ce qui vaut quelle que soit la recette** : les points 1 à 4, soit
l'empreinte, la signature vérifiée par la barrière, et la présence de l'image
dans les deux registres. **Ce qui ne vaut qu'en K3s** : le point 5,
`registries.yaml` et Kyverno n'existant qu'avec un cluster.

## Alternatives écartées

- **Harbor sur la machine de supervision (zone 2).** La zone de confiance n'est
  pas joignable depuis Internet (ADR-014) ; or la CI doit y pousser et la
  production y tirer. Il faudrait lui ouvrir une entrée, et la production y
  lirait ce qu'elle exécute, ce que la première règle de flux d'ADR-014
  n'admet pas (la production ne parle à la zone de confiance que pour
  demander). Surtout, la machine qui détient déjà l'autorité de certification,
  la clé maîtresse et les journaux détiendrait aussi le contenu de ce qui
  tourne : sa compromission permettrait de substituer une image et d'effacer
  la trace de la substitution. C'est contraire à « aucune brique ne détient
  tout ». Enfin, un Harbor auto-hébergé est une brique de plus à mettre à jour
  et à sauvegarder sur une machine qu'ADR-014 veut modeste (principe n°4).
- **Harbor sur l'exécutant Sluis.** Le VPS de l'exécutant n'a pas d'entrée
  depuis Internet (ADR-010) ; un registre lui en imposerait une. Son image est
  exemptée du gate de scan pour `tofu` (ADR-011) : un hôte qui porte des
  vulnérabilités tolérées deviendrait la source de tout ce que la production
  exécute. Et celui qui exécute les déploiements fournirait aussi ce qui est
  déployé : la séparation entre demander, exécuter et fournir disparaîtrait,
  contre « aucune brique ne détient tout ».
- **Le registre d'OVHcloud seul.** Plus simple, mais la production, le
  déverrouillage d'OpenBao (ADR-017) et la source des images dépendraient
  tous du même fournisseur et du même compte. Une panne ou une suspension
  empêcherait à la fois de servir et de redémarrer ou de revenir en arrière.
  GHCR est déjà en place, chez un autre fournisseur : il donne un second
  chemin qui ne partage pas cette panne.
- **GHCR seul, comme aujourd'hui.** Écarté par la demande du superviseur. Le
  registre principal d'une production européenne est mis chez l'hébergeur
  européen, GHCR restant le secours.

## Conséquences

- **Coût** : un abonnement au registre géré d'OVHcloud, sur un compte payant
  que seul le superviseur ouvre ; GHCR ne coûte rien pour les images
  publiques, et le stockage des images privées dépend du forfait GitHub.
- **La CI pousse deux fois et signe** : un jeton d'écriture de plus, pour le
  registre d'OVHcloud, devient un secret de la chaîne de build ; l'identité de
  signature en est un autre (déjà relevé par ADR-021).
- **Les signatures doivent suivre les images** : cosign range la signature à
  côté de l'image, dans le registre. Une image recopiée sans sa signature
  serait tirable mais refusée par la barrière ; le repli ne marche que si les
  deux registres portent l'une et l'autre.
- **Les étiquettes mobiles cessent de faire foi.** Les fichiers Compose de ce
  dépôt, qui tirent `latest` par défaut, et l'épinglage par étiquette datée
  d'ADR-027 sont en écart avec le point 2 s'ils entrent dans son périmètre.
- **Limite** : la signature prouve que l'image vient de la chaîne de build et
  n'a pas été modifiée depuis, pas que son contenu est sain. Le scan avant
  envoi (ADR-011) reste la seule garantie sur le contenu.
- **Limite, en recette VPS** : sans `registries.yaml`, le repli vers le
  secours n'est pas transparent. Il suppose de changer le registre désigné,
  à empreinte égale, ce qui est un changement de configuration de la cible.
- **Limite** : une clé ou une identité de signature compromise permet de
  signer une image malveillante que la barrière acceptera. Sa protection
  conditionne la valeur de tout le dispositif.

## Mesures CyFun 2025 couvertes

Mapping dans `foyer/skills/conformite.md` §2bis, couche plateforme.

- Intégrité des logiciels exécutés, par empreinte et signature vérifiée :
  **PR.DS-01.1 I**, avec **PR.DS-01.2 E** pour l'alerte quand la barrière
  bloque.
- Exécution de logiciels non autorisés empêchée (barrière, puis Kyverno en
  K3s) : **PR.PS-05.2 I**.
- Ce qui part en production est ce qui a été testé, prouvé par l'empreinte :
  **PR.PS-06.2 I**.
- Disponibilité des images par le registre de secours : **PR.IR-03.1 E** et
  **PR.IR-04.1 I**.
- Dépendance envers deux fournisseurs de registre : catégorie GV.SC,
  identifiant à confirmer.

## Points ouverts

- **Repli manuel en recette VPS** : délai acceptable pour basculer vers le
  secours, et qui le déclenche (point tranché le 2026-10-23 : repli par la
  passerelle d'approbation, non automatique).
- **Kyverno ou la barrière, en K3s.** La lettre de mission dit « à la barrière
  ou par Kyverno ». En K3s, Kyverno remplace-t-il la vérification de la
  barrière ou s'y ajoute-t-il ? Flux sait aussi vérifier des signatures
  (ADR-029, ADR-031) : Kyverno se justifie-t-il comme brique de plus
  (principe n°4) ?
- **Qui pousse vers les deux registres.** La CI, a priori : envoi direct vers
  les deux, ou envoi vers l'un puis copie vers l'autre ? Où vit le jeton
  d'écriture du registre d'OVHcloud, et avec quels droits ?
- **Synchronisation et cohérence des empreintes.** Une image reconstruite
  deux fois peut changer d'empreinte : comment garantir que les deux
  registres portent exactement le même manifeste et sa signature ? Quel
  contrôle détecte un écart, et la barrière doit-elle vérifier la présence de
  l'empreinte dans les deux registres ?
- **Identité de signature.** Clé cosign gérée (où, avec quelle rotation, en
  lien avec ADR-015 et OpenBao) ou signature sans clé (*keyless*), qui
  s'appuie sur l'identité du workflow et sur des services publics de
  transparence ? ADR-021 parle de « la clé ou l'identité attendue » sans
  choisir.
- **Coût du registre d'OVHcloud.** Offre et volume retenus, sur le compte
  payant réservé au superviseur.
- **Étape dans la feuille de route.** Après les étapes 1 à 3, sans les
  bloquer ; avec l'étape 7 (barrière et preuves), qui porte déjà le contrôle
  « images signées », ou avec l'étape 8 ? Un amendement d'ADR-033 sera
  nécessaire.
- **Périmètre.** L'ADR vise les images des SaaS déployés en production.
  S'applique-t-il aux images de Sluis lui-même, publiées sur GHCR et tirées
  par `latest` dans les fichiers Compose du dépôt, et aux images tierces
  épinglées par étiquette datée (ADR-027) ?
- **Réécriture des chemins.** Les deux registres n'ont pas la même
  arborescence de dépôts ; la configuration miroir de K3s devra la
  réconcilier (réécriture de chemins), à vérifier sur la version retenue.
- **Contrat du dépôt cible.** La référence par empreinte entre-t-elle dans le
  contrat d'ADR-034, vérifiée par `sluis_verifier_depot` ?
