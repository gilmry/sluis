# ADR-015 : Identités, des YubiKeys pour les humains, step-ca pour les machines

- **Statut** : Proposé le 2026-10-21, non contresigné.
- **Amendé le 2026-10-21** (revue des ADR, décision 7) : identifiants CyFun 2025 complétés. Amendement proposé, non contresigné.
- **Irréversible** : oui. Les clés racines des humains et de l'autorité de certification ancrent toutes les autres identités ; les changer oblige à re-signer, re-déployer et re-déclarer partout.
- **S'articule avec** : ADR-013 (la clé SSH de signature des approbations, conservée à la demande du superviseur), ADR-014 (step-ca vit en zone de confiance, les YubiKeys en zone hors ligne), ADR-016 (age), ADR-032 (Ansible se connecte par certificat SSH court).
- **Sert la vision (ADR-000)** : principe n°1, parce que le renouvellement des certificats machines est automatique et que plus aucune clé statique n'attend d'être oubliée ; principe n°2, parce qu'un renouvellement qui échoue alerte avant l'expiration ; règle « aucune brique ne détient tout », parce que la clé privée humaine ne quitte jamais son support matériel.

## Contexte

Aujourd'hui, les accès reposent sur des clés SSH statiques, posées une fois et
jamais tournées, et sur des jetons d'API dont la durée de vie n'est suivie nulle
part. Une clé statique est le secret le plus facile à oublier : elle survit au
poste qui l'a créée, à la personne qui la détenait et à la raison qui l'a fait
naître. CyFun demande des identifiants uniques par utilisateur et par processus,
et une authentification forte des accès à privilèges.

Deux populations n'ont pas les mêmes besoins. Un **humain** est rare (une
personne aujourd'hui), agit peu mais sur l'irréversible, et doit pouvoir prouver
que c'est bien lui. Une **machine** est nombreuse, agit tout le temps, et doit
pouvoir être révoquée sans cérémonie.

## Décision

**1. Les humains : GPG sur deux YubiKeys.**

- Une clé GPG maîtresse, générée hors ligne, conservée en zone 3 (ADR-014) ; des
  sous-clés de signature, de chiffrement et d'authentification chargées sur
  **deux YubiKeys** : une d'usage, une de secours rangée ailleurs.
- La sous-clé de chiffrement sert aussi de destinataire pour les secrets de
  déploiement (ADR-016), soit en GPG, soit par une identité age dérivée sur
  YubiKey (`age-plugin-yubikey`).
- La sous-clé d'authentification sert l'accès SSH humain d'urgence ; l'accès
  courant passe par les certificats step-ca, comme pour les machines.

**2. Les approbations : la clé SSH de signature d'ADR-013, sur YubiKey aussi.**
Le superviseur a choisi de garder la signature SSH pour les étiquettes
`approuve/*` (ADR-013). Les deux mécanismes coexistent avec des rôles distincts :
GPG pour l'identité, le chiffrement et la signature des commits ; SSH pour
**l'acte d'approbation**, vérifié par le tireur GitOps contre son
`allowed_signers`. **Recommandation** : générer cette clé en `ed25519-sk`,
c'est-à-dire résidente sur la YubiKey, avec une seconde clé sur la YubiKey de
secours déclarée dans le même `allowed_signers`. Une approbation exige alors la
présence physique de la clé, ce qui ferme la voie d'une clé copiée.

**3. Les machines : step-ca, certificats courts, aucune clé statique.**

- **step-ca** en zone de confiance émet des certificats **X.509 pour le mTLS**
  entre composants (supervision, OpenBao, Sluis exécutant) et des **certificats
  SSH courts** pour les accès d'administration, Ansible compris (ADR-032).
- Durée de vie courte par défaut : de l'ordre de 24 h pour SSH, de quelques jours
  pour le mTLS ; renouvellement automatique (ADR-025). Un certificat expiré est
  un accès révoqué sans intervention.
- Chaque machine et chaque processus a **son** identité ; aucune n'est partagée.
- La racine de step-ca est hors ligne (zone 3) ; seule une autorité
  intermédiaire vit sur la machine de confiance.

## Alternatives écartées

- **Clés SSH statiques par machine.** Simples, mais jamais tournées en pratique,
  et une clé volée reste valide jusqu'à ce que quelqu'un s'en aperçoive.
- **Une seule YubiKey.** Sa perte bloquerait toute approbation et toute
  restauration de secret. La seconde est le prix d'une identité qui ne dépend pas
  d'un objet unique.
- **Tout en SSH (identité et approbation).** Écarté pour l'identité : GPG porte
  le chiffrement que SOPS consomme et une chaîne de confiance établie. Retenu pour
  l'approbation, par choix du superviseur (ADR-013).
- **Vault PKI ou OpenBao comme autorité de certification.** Possible, mais il
  concentrerait la clé maîtresse applicative et l'émission des identités dans la
  même brique, contre la règle « aucune brique ne détient tout ».
- **Fournisseur d'identité en nuage.** Dépendance externe pour l'accès à sa
  propre infrastructure, contraire à la souveraineté visée.

## Conséquences

- **Coût** : deux YubiKeys (de l'ordre de 100 à 150 euros), une cérémonie de
  génération hors ligne, step-ca sur la machine de confiance.
- **Entretien** : la rotation des certificats est automatique ; ce qui reste
  humain est la rotation des sous-clés GPG (à date d'expiration posée) et le
  test annuel de la YubiKey de secours.
- **Limite** : la machine de confiance devient indispensable aux accès. Si elle
  tombe, plus aucun certificat ne s'émet ; la clé d'authentification GPG sur
  YubiKey reste l'accès d'urgence, à ne pas utiliser au quotidien.
- **Limite** : deux formats de clé humaine (GPG et SSH) à tenir. C'est le prix du
  choix de garder ADR-013 tel quel ; les mettre sur les mêmes YubiKeys le réduit.
- **Limite, porteur seul** : il n'y a qu'un humain, donc qu'une identité qui
  approuve. Les deux YubiKeys protègent contre la perte, pas contre l'absence de
  second regard (mesure compensatoire, `foyer/skills/conformite.md` §2bis).

## Mesures CyFun 2025 couvertes

Renvoi à `foyer/skills/conformite.md` §2bis (couches) et §2ter (preuves) :

- **PR.AA-03.2 ★** : authentification forte, ici matérielle pour les humains.
- **PR.AA-02.2** et **PR.AA-01.4** : identifiant unique par utilisateur et par
  processus, MFA ; ici appliqués à la plateforme elle-même.
- **PR.AA-01.1 ★** : aucun moyen d'authentification de service dans le code.
- **PR.AA-05.3 ★** : moindre privilège par certificat dédié et court.
- Gestion du cycle de vie des clés : CyFun 2025 n'a pas de mesure dédiée. Elle
  est couverte indirectement par **GV.PO-01.2 I** (politique, dont l'usage de
  la cryptographie) et, pour des certificats courts renouvelés
  automatiquement, par **PR.AA-01.2 I**.

## Points ouverts

- **Durées de vie exactes** des certificats SSH et mTLS : à fixer après mesure du
  coût des renouvellements.
- **Cérémonie de génération** : où et comment la clé maîtresse GPG et la racine
  step-ca sont générées puis mises hors ligne, et qui en garde une trace.
- **Procédure de perte d'une YubiKey** : révocation des sous-clés, retrait de la
  clé SSH de l'`allowed_signers` de chaque cible (ADR-013), réémission.
