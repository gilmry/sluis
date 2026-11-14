# ADR-042 : Identités sans PKI, Keycloak pour les humains (FIDO2) et pour les machines (assertions fédérées)

- **Statut** : Proposé le 2026-11-13, contresigné le 2026-11-14.
- **Décision du superviseur du 2026-11-13**, à l'origine de cet ADR : « il n'y aura pas de PKI mais du PKI-less, avec les protocoles qu'il faut pour le machine à machine, et l'authentification humaine avec Keycloak » ; « ce sera FIDO2, ou passwordless WebAuthn » ; « pour le machine à machine, des protocoles PKI-less gérés par Keycloak, pour faire du zero trust ».
- **Décision du superviseur du 2026-11-13 (suite)** : « on retire step-ca et on met Keycloak » dans la zone de confiance. Le point ouvert A est tranché : un Keycloak de plateforme en zone 2, distinct de celui des SaaS. **OpenBao est maintenu** (ADR-017) : il porte la clé maîtresse du chiffrement par personne, ce que Keycloak ne fait pas ; seule change son authentification (point 6).
- **Décisions du superviseur du 2026-11-13 (fin)** : point B tranché, voie B1 (Ansible en SSH avec opkssh et un jeton machine) ; **le Keycloak de plateforme est en haute disponibilité, OpenBao ne l'est pas** (une instance, déverrouillage automatique et cache court côté backend, ADR-017 et ADR-018).
- **Amende ADR-015** : son point 3 (step-ca, certificats X.509 pour le mTLS et certificats SSH courts) est **remplacé** ; son point 1 est complété (les sous-clés GPG restent, l'accès courant passe par Keycloak). Le point 2 (clé SSH `ed25519-sk` des approbations, ADR-013) est **inchangé**.
- **Amende aussi**, chacun sur ce qu'il dit de step-ca ou du mTLS : ADR-014, ADR-017, ADR-025, ADR-029, ADR-032, ADR-033, ADR-035, ADR-036, ADR-037, ADR-040, ADR-041 (voir « Ce qui change ailleurs »).
- **Irréversible** : oui, à mesure que des identités y naissent ; **réversible à faible coût aujourd'hui**. Aucune racine step-ca n'a été générée : les étapes 1 et 2 d'ADR-033 ne sont pas commencées (epics #44 et #45 ouverts, constat du 2026-11-13). Une fois les machines et les humains enrôlés dans Keycloak, en sortir est une migration de toutes les identités.
- **S'articule avec** : ADR-013 (approbations signées en SSH, inchangé), ADR-014 (zones), ADR-016 (SOPS et age, inchangés), ADR-017 (OpenBao), ADR-023 (supervision), ADR-025 (renouvellements), ADR-029 (tunnel privé, K3s), ADR-030 (serveur OAuth 2.1 du Sluis public), ADR-032 (Ansible), ADR-035 à ADR-037 (reprise, amorce), ADR-040 (YubiKeys), ADR-041 (Keycloak des SaaS).
- **Sert la vision (ADR-000)** : principe n°4, parce qu'une seule brique d'identité, déjà retenue pour les SaaS (ADR-041), remplace une autorité de certification à opérer ; principe n°1, parce que les identités machines sont des jetons de quelques minutes, renouvelés sans cérémonie ; règle « aucune brique ne détient tout », parce que le Keycloak de la plateforme ne détient ni la clé maîtresse (ADR-017) ni les secrets de déploiement (ADR-016), et qu'il est distinct de celui des SaaS.

## Contexte

ADR-015 prévoyait **step-ca** : une racine hors ligne, une intermédiaire sur la
machine de confiance, des certificats X.509 pour le mTLS entre composants et
des certificats SSH courts pour l'administration. C'est une PKI complète à
opérer : cérémonie de la racine (ADR-037, ADR-040), phrase de passe de
l'intermédiaire, sauvegarde de sa base (ADR-035), exercices de reprise
(ADR-036), distribution des chaînes de confiance, révocation. Pour une équipe
d'une personne, c'est une brique de plus, critique, rarement manipulée, donc
mal connue le jour où elle casse.

Keycloak est déjà retenu pour les utilisateurs des SaaS (ADR-041). Les versions
récentes savent authentifier **les machines sans secret ni certificat**, par des
jetons signés émis par un tiers de confiance déjà présent (Kubernetes, un autre
fournisseur OIDC), et **les humains sans mot de passe**, par FIDO2/WebAuthn.

**Ce que « sans PKI » veut dire ici.** Nous n'opérons **aucune autorité de
certification** : ni racine, ni intermédiaire, ni chaîne à distribuer, ni liste
de révocation. La confiance repose sur des **clés publiques nues publiées en
JWKS** (Keycloak, émetteur des comptes de service de K3s) et sur des clés
déclarées une à une (WireGuard, `allowed_signers`). Deux PKI subsistent, que
nous **consommons sans les opérer** : la PKI du Web pour le TLS public
(Let's Encrypt, à l'entrée HTTP des applications) et l'autorité interne que K3s
génère seule pour son plan de contrôle. Elles sont hors du périmètre de cette
décision.

### Vérifié sur keycloak.org le 2026-11-13

Dernière version publiée : **26.8.2**.

| Fonction | Version, statut | Source |
|---|---|---|
| Authentification fédérée des clients (assertion émise par un fournisseur **OIDC** ou par **Kubernetes**, envoyée en `client_assertion` à la place d'un secret) | preview en 26.4 et 26.5, **supportée depuis 26.6** | notes de version ; billet « Federated client authentication, no more secrets » (janvier 2026) |
| Même mécanisme avec **SPIFFE** (JWT-SVID) | **preview**, tant que le brouillon IETF n'est pas finalisé | même billet |
| **DPoP** (RFC 9449, jeton lié à une clé du client) | **supporté depuis 26.4** | notes de version |
| **Token exchange** standard (RFC 8693) | **supporté depuis 26.2** | notes de version |
| **JWT Authorization Grant** (RFC 7523) | preview en 26.5, **supporté depuis 26.6** | notes de version |
| **Passkeys**, WebAuthn sans mot de passe | **supportées depuis 26.4** ; nouvelles options de politique WebAuthn en 26.8 | notes de version ; billet « Passkeys support in upcoming Keycloak release (26.4) » |

Limites relevées dans le billet sur l'authentification fédérée : la confiance
envers l'émetteur externe est totale (un émetteur compromis forge des jetons
pour tout client qui s'y fie) ; **pas de protection contre le rejeu** pour les
jetons Kubernetes et SPIFFE ; durée de vie recommandée de **10 minutes au
plus** ; si Keycloak est **hors du cluster**, le point de découverte de
l'émetteur Kubernetes (`/.well-known/openid-configuration` et ses clés) doit
lui être joignable **sans authentification**.

## Décision

**1. Aucune autorité de certification opérée.** step-ca n'est pas déployé. Pas
de racine, pas d'intermédiaire, pas de certificat X.509 émis par nous, pas de
certificat SSH.

**2. Un Keycloak de plateforme, distinct de celui des SaaS.**
*Tranché par le superviseur le 2026-11-13.*

- Une instance dédiée en **zone 2** (machine de confiance), non exposée à
  Internet, joignable par le tunnel privé seulement (point 5). Un realm
  `plateforme` pour les humains d'administration et les machines.
- Le Keycloak des SaaS (ADR-041) reste en zone 1, un realm par SaaS. Aucun
  compte d'administration ni aucune identité machine de la plateforme n'y vit.
- Raison : ADR-015 refusait que l'administration dépende d'un service exposé en
  production. Un Keycloak commun ferait de la page de connexion publique des
  SaaS la porte de l'infrastructure.

**2bis. Haute disponibilité : Keycloak oui, OpenBao non.**

- Le Keycloak de plateforme est en **haute disponibilité** : avec des jetons
  machines de 10 minutes, une panne plus longue arrête tous les échanges entre
  machines et l'accès d'Ansible. Sa topologie est un point ouvert (D).
- OpenBao reste à **une instance** : il redémarre seul (déverrouillage par le
  KMS d'OVHcloud, ADR-017), et le cache court des clés de données côté backend
  (ADR-018) couvre un redémarrage. Une panne plus longue empêche d'ouvrir de
  nouvelles données chiffrées : risque assumé, déjà écrit dans ADR-017.

**3. Humains : FIDO2, WebAuthn sans mot de passe.**

- Les comptes du realm `plateforme` s'authentifient par **passkey résidente sur
  YubiKey** (WebAuthn sans mot de passe, vérification de l'utilisateur par PIN
  exigée). Aucun mot de passe n'existe sur ces comptes ; aucun second facteur
  par SMS ni par code.
- Chaque YubiKey d'ADR-040 est enrôlée comme passkey du même compte nominatif ;
  la perte d'une clé se traite par retrait de sa passkey.
- Politique WebAuthn : attestation exigée et limitée aux modèles de YubiKey
  retenus (identifiants AAGUID déclarés), à vérifier sur 26.8.
- **Accès SSH humain : jeton OIDC de Keycloak, sans certificat**, par
  **opkssh** (OpenPubkey, Apache 2.0) : le poste obtient un jeton du realm
  `plateforme` après connexion FIDO2, le serveur SSH vérifie sa signature contre
  les clés publiées par Keycloak et la correspondance avec l'identité autorisée.
  *Compatibilité de la version retenue d'opkssh avec Keycloak à vérifier en
  recette.*
- Consoles d'administration (OpenBao, supervision, Keycloak lui-même, API du
  cluster) : OIDC contre le realm `plateforme`.
- **Accès d'urgence, inchangé** : la sous-clé d'authentification GPG sur
  YubiKey (ADR-015, point 1), déclarée sur chaque machine, à ne pas utiliser au
  quotidien. Elle couvre la panne du Keycloak de plateforme.

**4. Machines : jetons courts, assertions fédérées, aucun secret statique.**
C'est le zero trust de cette plateforme : **aucune requête n'est crue sur sa
provenance réseau** ; chacune porte un jeton court, émis pour une seule
audience, vérifié par le destinataire à chaque appel.

- **Charges dans K3s** (backend des SaaS, Flux, agents de supervision) :
  **jeton projeté de compte de service Kubernetes** (audience dédiée, 10 minutes
  au plus) présenté à Keycloak en `client_assertion` (authentification fédérée
  des clients, fournisseur de type Kubernetes). Aucun secret de client.
- **Machines hors K3s** (Sluis exécutant, machine de confiance, témoin
  externe) : **clé privée propre à la machine, publique déclarée dans Keycloak**
  (`private_key_jwt`, assertion RFC 7523). La clé est générée sur la machine et
  n'en sort pas ; sa rotation est une redéclaration de clé publique, pas
  l'émission d'un certificat. *Possibilité de la garder dans un TPM à évaluer.*
- **Jeton lié par DPoP** (RFC 9449) partout où l'appelé le sait vérifier : un
  jeton volé ne sert pas sans la clé de l'appelant.
- **Token exchange** (RFC 8693) pour réduire un jeton large en un jeton d'une
  seule audience avant un appel en aval, quand un composant en appelle un autre
  au nom d'un premier.
- **Aucun secret de client** dans le realm `plateforme`. Un client à secret est
  un refus de la barrière (ADR-021), vérifié sur la configuration déclarée.
- SPIFFE n'est pas retenu tant qu'il est en preview chez Keycloak.

**5. Transport entre zones : WireGuard.** Le tunnel privé laissé ouvert par
ADR-029 est **WireGuard** : clés publiques Curve25519 déclarées pair par pair,
sans autorité. Il chiffre et restreint qui joint quoi ; **il n'autorise rien** :
l'autorisation reste au jeton (point 4). Les clés WireGuard sont déclarées dans
Git et posées par Ansible ; leur rotation est planifiée (ADR-025).

**6. OpenBao (amende ADR-017, point 3).** Le backend ne s'authentifie plus par
mTLS mais par la **méthode JWT d'OpenBao** : il présente un jeton court (son
jeton de compte de service, ou un jeton Keycloak d'audience `openbao`), dont
OpenBao vérifie la signature et les revendications (`iss`, `aud`, `sub`) ; la
politique ne permet toujours que d'envelopper et de désenvelopper, pour son seul
SaaS. L'administration d'OpenBao passe par OIDC contre le realm `plateforme`,
donc par FIDO2. *Choix entre jeton Kubernetes direct et jeton Keycloak : point
ouvert C.*

**7. Ansible et l'administration des machines (amende ADR-032, point 1).**
*Tranché le 2026-11-13 : B1.* Keycloak authentifie, Ansible fait le travail :
il installe et converge sur chaque machine le Keycloak de plateforme (la zone 2
n'est pas un cluster, il n'y a donc pas d'opérateur), opkssh et la
configuration de `sshd`, WireGuard et OpenBao. Les realms, les clients et les
politiques WebAuthn se déclarent par le fournisseur OpenTofu de Keycloak, comme
pour ADR-041. **Amorce** : le premier passage d'Ansible, avant que Keycloak
existe, se fait avec la sous-clé d'authentification GPG sur YubiKey (ADR-037),
qui est retirée des chemins courants une fois opkssh en place. Les deux voies
examinées :

- **B1, retenue** : Ansible exécuté par le Sluis exécutant, en SSH, avec
  opkssh et un jeton machine du realm `plateforme`. Reste sans agent, comme
  ADR-032. *Que opkssh accepte un jeton obtenu sans navigateur est à vérifier.*
- **B2, écartée** : bascule en tirage (`ansible-pull`) : chaque machine tire le dépôt et
  vérifie la signature du commit contre `allowed_signers`, comme Flux
  (ADR-029). Plus aucun accès SSH entrant d'automate, mais un agent sur chaque
  machine, contraire au choix d'ADR-032.

**8. Ce qui ne change pas.** Les approbations restent signées en SSH
(`ed25519-sk`, ADR-013) et vérifiées contre `allowed_signers` : c'est déjà sans
PKI. GPG reste l'identité de chiffrement et de signature des commits, et le
destinataire SOPS humain (ADR-016, ADR-040). Le serveur OAuth 2.1 du Sluis
public (ADR-030) reste distinct.

## Ce qui change ailleurs

Chaque ADR amendé porte en tête une ligne qui renvoie ici ; son corps n'est pas
réécrit, cet ADR prévaut sur ce qu'il dit de step-ca.

| ADR | Ce qui disait step-ca ou mTLS | Devient |
|---|---|---|
| 014 | step-ca en zone 2 ; « l'autorité de certification » parmi ce qui ne doit pas vivre en zone 1 | Keycloak de plateforme en zone 2 ; lire « les clés de signature du Keycloak de plateforme » |
| 015 | point 3, step-ca | points 1 à 4 ci-dessus |
| 017 | point 3, mTLS par certificat step-ca | point 6, méthode JWT |
| 025 | renouvellement des certificats machines | rotation des clés `private_key_jwt`, des clés de signature du realm et des clés WireGuard |
| 029 | tunnel à choisir ; certificats courts pour l'API du cluster | WireGuard ; OIDC du realm `plateforme` pour l'API du cluster |
| 032 | certificats SSH courts | point 7, voie B à trancher |
| 033 | étape 2 : step-ca, « certificats SSH courts » | étape 2 : Keycloak de plateforme ; critère « une connexion SSH sans jeton valide est refusée ; un client à secret est refusé » |
| 035, 036 | restauration de step-ca, de son intermédiaire et de sa base | restauration de la base du Keycloak de plateforme (comptes, passkeys, clés publiques des machines, clés de signature du realm) |
| 037 | racine et intermédiaire step-ca à l'amorce | rien à générer hors ligne pour les machines ; l'amorce crée le realm `plateforme` et enrôle les passkeys |
| 040 | racine step-ca sur la copie papier et la station hors ligne | sans objet ; la station hors ligne ne garde que la clé GPG maîtresse |
| 041 | « le fournisseur d'identité ne remplace ni les YubiKeys ni step-ca » | le Keycloak des SaaS reste séparé ; le Keycloak de plateforme remplace step-ca, pas les YubiKeys |

## Alternatives écartées

- **step-ca (ADR-015).** Fonctionne, mais c'est une PKI à opérer seul, avec une
  racine hors ligne et une intermédiaire dont la perte coupe tous les accès.
  Écarté par le superviseur.
- **SPIFFE/SPIRE.** Identités de charge standard, mais SPIRE est lui-même une
  autorité de certification (X.509-SVID), et le support des JWT-SVID par
  Keycloak est en preview.
- **mTLS avec clés publiques nues** (RFC 7250). Sans autorité, mais peu outillé
  dans les briques visées (OpenBao, supervision, Traefik).
- **Un seul Keycloak pour les SaaS et la plateforme.** Une instance de moins,
  mais l'administration dépendrait de la production exposée ; écarté par le
  superviseur le 2026-11-13 (point 2).
- **Secrets de client partagés.** Simples, mais statiques : exactement ce
  qu'ADR-015 voulait supprimer.

## Conséquences

- **Moins de cérémonie** : plus de racine ni d'intermédiaire à générer, ranger,
  sauvegarder et exercer. ADR-037 et ADR-040 perdent une ancre chacun.
- **Nouvelle ancre** : les clés de signature du realm `plateforme` et sa base
  deviennent ce que la racine step-ca était. Leur perte coupe tous les accès
  sauf l'accès d'urgence GPG ; leur vol permet de forger toute identité. Elles
  se sauvegardent chiffrées vers les seules identités humaines (ADR-035).
- **Point de défaillance**, réduit par la haute disponibilité (point 2bis) : si
  le Keycloak de plateforme tombe entièrement, plus aucun jeton ne s'émet ; les jetons en cours expirent en minutes. Les composants qui
  vérifient localement les JWKS continuent tant qu'ils ont des jetons. Le témoin
  externe doit sonder Keycloak (ADR-023).
- **Coût** : une seconde instance Keycloak et sa base sur la machine de
  confiance (de l'ordre de 1250 Mo d'après ADR-041), à confronter à sa taille
  (ADR-014, point ouvert).
- **Rejeu** : un jeton Kubernetes intercepté reste utilisable jusqu'à son
  expiration ; d'où la durée courte, DPoP et le transport WireGuard.
- **kit-actix** : l'adaptateur OIDC prévu par ADR-041 sert aussi à vérifier les
  jetons machines (audience, DPoP).

## Mesures CyFun 2025 couvertes

Renvoi à `foyer/skills/conformite.md` §2bis :

- **PR.AA-03.2 ★** : authentification forte, ici FIDO2 sans mot de passe.
- **PR.AA-02.2** et **PR.AA-01.4** : identifiant unique par humain et par
  processus.
- **PR.AA-01.1 ★** : aucun moyen d'authentification de service dans le code ni
  en secret partagé.
- **PR.AA-05.3 ★** : moindre privilège par jeton d'une seule audience.
- **PR.AA-01.2 I** : identités machines à durée de vie courte, renouvelées
  automatiquement.

## Points ouverts

- ~~**A. Une ou deux instances Keycloak**~~ : tranché le 2026-11-13, deux
  instances, celle de la plateforme en zone 2 (point 2).
- ~~**B. Ansible**~~ : tranché le 2026-11-13, B1 (point 7).
- **D. Topologie de la haute disponibilité de Keycloak** : la haute
  disponibilité suppose au moins deux nœuds et une base PostgreSQL elle-même
  hautement disponible, sinon la base reste le point unique. La zone 2 n'est
  aujourd'hui qu'une machine (ADR-014). Voies : plusieurs machines de confiance
  avec PostgreSQL répliqué (Patroni), ou un petit cluster dédié en zone 2 avec
  l'opérateur Keycloak et CloudNativePG. Coût à mesurer.
- **C. OpenBao** : jeton Kubernetes direct (pas de dépendance à Keycloak sur le
  chemin du chiffrement) ou jeton Keycloak (un seul émetteur à surveiller).
  Recommandation : jeton Kubernetes direct, pour que la panne de Keycloak
  n'arrête pas le déchiffrement des données.
- **Découverte de l'émetteur K3s** : le Keycloak de plateforme est hors du
  cluster ; exposer la découverte sans authentification par le tunnel, ou
  déclarer les clés de l'émetteur à la main (à rejouer à chaque rotation).
- **Durées de vie exactes** des jetons machines, des sessions humaines et des
  clés opkssh.
- **Rotation des clés de signature du realm** et sa cadence.
- **opkssh** : version retenue, fonctionnement avec Keycloak et sans
  navigateur, à vérifier en recette.
- **Attestation WebAuthn** : faisabilité de restreindre aux AAGUID des YubiKeys
  sur 26.8.
- **Exercice de reprise** (ADR-036) : remplacer le scénario step-ca par la
  restauration du Keycloak de plateforme.
