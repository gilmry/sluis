# ADR-041 : Fédération des identités, Keycloak fournisseur d'identité commun des utilisateurs des SaaS

- **Statut** : Proposé le 2026-10-22, non contresigné.
- **Décisions du superviseur du 2026-10-23**, intégrées au texte : déploiement par **Flux** (et non Argo CD) ; **un realm par SaaS**. Avant K3s : Coolify si un SaaS a de vrais utilisateurs avant l'étape 8, sinon Keycloak attend K3s (point 11).
- **Irréversible** : oui, à mesure que des comptes y naissent. Les comptes, leurs empreintes de mots de passe, leurs seconds facteurs et leurs liens de fédération vivent dans la base du fournisseur d'identité ; l'identifiant qu'il attribue à chaque personne s'inscrit dans les données de chaque SaaS. En changer est une migration de comptes : réinitialisation probable des mots de passe et des seconds facteurs, ré-association des identifiants, nouvelle déclaration de chaque fédération. Réversible à faible coût tant qu'aucun utilisateur réel n'y a de compte.
- **S'articule avec** : ADR-014 (zone de production, exposition), ADR-015 (identités d'administration et des machines, **inchangé**), ADR-016 (secrets du fournisseur de configuration), ADR-018 et ADR-019 (identifiant de personne, effacement), ADR-020 (seconde production éphémère), ADR-023 (supervision et journaux), ADR-026 (base sous CloudNativePG), ADR-029 (recettes VPS et K3s), ADR-030 (Sluis public et son propre serveur OAuth 2.1), ADR-032 (rôle d'OpenTofu, réconciliateur du cluster), ADR-033 (feuille de route), ADR-034 (liste blanche des fournisseurs OpenTofu).
- **Sert la vision (ADR-000)** : principe n°4, parce qu'une brique d'authentification construite une fois évite à chaque SaaS de réécrire comptes, mots de passe, second facteur et fédération ; principe n°1, parce que la configuration des realms et des clients se déclare dans Git et se rejoue ; règle « aucune brique ne détient tout », parce que le fournisseur d'identité authentifie les utilisateurs sans détenir les clés de leurs données (ADR-017, ADR-018). Le principe n°4 est aussi ce qui le met en tension : voir les points ouverts.

## Contexte

Les SaaS produits avec Foyer ont des utilisateurs : copropriétaires,
membres d'une association, salariés d'un client. Aujourd'hui, rien de commun ne
les authentifie. Le kit Rust de Foyer (kit-actix) ne contient aucun mécanisme
d'authentification : son exemple est mono-utilisateur. Chaque SaaS qui en a
besoin l'écrit donc lui-même, avec ses propres règles de mot de passe, de second
facteur et de récupération de compte. C'est exactement ce que la carte d'un SaaS
déployé avec Foyer et Sluis (« propre au SaaS, commun à tous ») range parmi ce
qui coûte cher à bien faire et qu'on fait mal quand on le refait à chaque projet.

Une seconde demande s'y ajoute : une entreprise de services numériques (ESN)
cliente voudra que ses salariés se connectent avec leur compte Microsoft
Entra ID, sans compte de plus à gérer. Cela suppose une fédération, en OIDC ou en
SAML, que chaque SaaS ne peut pas raisonnablement implémenter seul.

**Trois populations d'identités, à ne pas confondre.** Cet ADR ne traite que la
première.

| Population | Qui | Mécanisme | ADR |
|---|---|---|---|
| **Utilisateurs des SaaS** | Les personnes qui utilisent un SaaS en production | Fournisseur d'identité commun, objet de cet ADR | ADR-041 |
| **Humains d'administration** | Le superviseur | GPG et clé SSH de signature sur deux YubiKeys | ADR-015, ADR-013 |
| **Machines** | Composants, processus, Ansible | Certificats courts step-ca | ADR-015 |

Le fournisseur d'identité **ne remplace ni les YubiKeys ni step-ca**.
L'administration de l'infrastructure ne doit pas dépendre d'un service exposé
en production : ADR-015 a déjà écarté un fournisseur d'identité pour l'accès à
sa propre infrastructure. De même, le serveur d'autorisation OAuth 2.1 du Sluis
public (ADR-012, ADR-030), qui authentifie les clients MCP de l'agent et du
superviseur, n'est pas concerné par cette décision.

## Décision

Demande du superviseur (lettre de mission du 2026-10-22), proposée ici :

1. **Keycloak est le fournisseur d'identité des utilisateurs des SaaS.** Licence
   Apache 2.0 (vérifiée sur le dépôt du projet). C'est une brique **commune** au
   sens de la carte : une instance, héritée par tous les SaaS.
2. **Protocoles exposés aux SaaS : OIDC** (code d'autorisation avec PKCE en
   S256, comme le serveur OAuth 2.1 de Sluis). Un SaaS ne voit jamais un mot de
   passe ; il reçoit et vérifie des jetons.
3. **Déploiement par Flux avec l'opérateur Keycloak** en recette K3s. La
   lettre de mission parlait d'Argo CD ; le superviseur a retenu **Flux** le
   2026-10-23, par cohérence avec ADR-029 et ADR-032, qui refusent deux
   réconciliateurs sur un même cluster. L'opérateur fournit la ressource
   `Keycloak` ; il **ne déploie pas la base de données**, à fournir à part
   (documentation de l'opérateur, vérifiée).
4. **Configuration par le fournisseur OpenTofu de Keycloak**
   (`keycloak/keycloak`, maintenu dans l'organisation du projet Keycloak,
   Apache 2.0, vérifié). Realms, clients, fournisseurs d'identité fédérés,
   politiques de mot de passe et de second facteur se déclarent dans Git et se
   rejouent ; la preuve de convergence est celle de Foyer, un second `plan` sans
   changement. La ressource d'import de realm de l'opérateur ne crée que des
   realms nouveaux et ne les met pas à jour (documentation vérifiée) : elle ne
   suffit pas comme moteur de convergence.
5. **Place dans les zones d'ADR-014 : zone 1, production.** Les utilisateurs
   doivent joindre les pages de connexion : Keycloak est exposé par l'entrée
   HTTP des applications, et par elle seulement. **Sa console et son API
   d'administration ne sont pas exposées** à Internet : elles se joignent comme
   l'interface de Coolify, par tunnel privé (ADR-029). Le mécanisme exact de
   séparation des chemins d'administration reste à vérifier sur la version
   retenue. Keycloak n'entre pas en zone 2, qui n'est pas exposée.
6. **Sa base est une base PostgreSQL dédiée**, distincte de celles des SaaS et
   de la base de clés (ADR-019) : sous CloudNativePG, dans son propre espace de
   noms, en recette K3s (ADR-026) ; conteneur sauvegardé en recette VPS, comme
   les autres bases à ce stade.
7. **Ce qui est commun, ce qui est propre au SaaS.**
   - **Commun** : l'instance, sa base, son opérateur, ses mises à jour, sa
     supervision, un thème de base.
   - **Propre au SaaS** : son client OIDC, ses rôles, son thème dérivé du thème
     de base, et ses fédérations éventuelles.
   - **Un realm par SaaS** (décision du superviseur du 2026-10-23). Il isole
     les comptes, les politiques et les fédérations d'un SaaS de ceux des
     autres, et une ESN fédérée à un SaaS n'apparaît pas dans un autre. Le
     realm partagé, qui aurait donné une connexion unique entre SaaS, est
     écarté au prix de cette commodité.
8. **Fédération avec Entra ID pour les ESN.** Keycloak sait déléguer
   l'authentification à un fournisseur OIDC v1.0 ou SAML v2.0 (*identity
   brokering*, documentation vérifiée). **OIDC par défaut**, plus simple à
   déclarer ; SAML quand l'ESN l'exige. Chaque fédération se déclare par le
   fournisseur OpenTofu, dans le realm du SaaS concerné. Les attributs importés
   se limitent à ceux dont le SaaS a besoin. Entra ID est ici le choix du client
   pour ses propres salariés, pas une dépendance de la plateforme : l'argument
   de souveraineté d'ADR-015 porte sur l'accès à notre infrastructure, pas sur
   les comptes d'un client.
9. **Lien avec le chiffrement par personne (ADR-018, ADR-019).**
   - Le SaaS rattache la clé de données d'une personne à **son propre
     identifiant interne**, associé au sujet (`sub`) émis par Keycloak, jamais
     au courriel.
   - **Keycloak détient des données personnelles en clair** (courriel, nom,
     empreinte du mot de passe, liens de fédération) : le port `Chiffreur`
     d'ADR-018 ne s'y applique pas. Il ne garde que ce qu'exige
     l'authentification ; le reste vit dans le SaaS, chiffré.
   - **Effacer une personne**, c'est détruire sa clé (ADR-019) **et** supprimer
     son compte Keycloak. Les sauvegardes de la base de Keycloak conservent ce
     compte jusqu'à leur expiration : leur rétention compte dans le délai
     d'effacement annoncé.
10. **Sauvegarde.** La configuration se reconstruit depuis Git (point 4) ; les
    comptes, eux, n'existent que dans la base. Elle suit le régime des autres
    bases : archivage continu hors région en recette K3s (ADR-026), sauvegarde
    hors hôte en recette VPS, alerte sur l'âge de la dernière sauvegarde réussie
    (ADR-023). Les secrets du fournisseur OpenTofu (identifiant du client
    d'administration) sont chiffrés par SOPS (ADR-016).

11. **Avant K3s, règle de déploiement** (décision du superviseur du
    2026-10-23). **Si un SaaS doit accueillir de vrais utilisateurs avant
    l'étape 8**, Keycloak tourne d'ici là comme service Compose sous Coolify,
    configuré par le même fournisseur OpenTofu (qui parle à son API quel que
    soit le mode de déploiement), avec une base PostgreSQL sauvegardée hors
    hôte ; le passage à l'opérateur et à CloudNativePG au moment de K3s est
    alors une **migration de données**, planifiée et approuvée comme telle.
    **Sinon**, Keycloak n'arrive qu'avec K3s, directement dans sa forme cible,
    et aucune migration des comptes n'a lieu.

## Alternatives écartées

Comparaison sur ce qui a été vérifié le 2026-10-22 ; le reste est marqué
« à vérifier ».

| Critère | Keycloak | Authentik | Zitadel |
|---|---|---|---|
| Licence | Apache 2.0 | MIT pour le cœur ; répertoire `enterprise/` sous licence propre | AGPL-3.0-only ; quelques sous-ensembles en Apache 2.0 et MIT |
| Opérateur Kubernetes | Officiel (ressource `Keycloak`) | Chart Helm officiel ; opérateurs tiers seulement | Chart Helm officiel ; opérateur tiers seulement |
| Configuration déclarative | Fournisseur OpenTofu du projet (Apache 2.0) | Fournisseur du projet (`goauthentik/authentik`, GPL-3.0) | Fournisseur du projet (statut de maturité à vérifier) |
| Fédération Entra ID | OIDC et SAML (courtage générique) | OAuth documenté ; Entra en source SAML non trouvé, à vérifier | OIDC et SAML documentés |
| Base | Fournie à part (PostgreSQL pris en charge) | PostgreSQL ; autres dépendances à vérifier | PostgreSQL 14 à 18 |
| Ressources | Environ 1250 Mo par instance avec 10 000 sessions en cache (guide de dimensionnement du projet) | À vérifier | À vérifier |
| Maturité | Projet ancien, rattaché à la CNCF (niveau à vérifier) | À vérifier | À vérifier |

- **Authentik.** Licence ouverte pour le cœur, mais une part des fonctions est
  réservée à l'édition entreprise (périmètre exact à vérifier) ; pas d'opérateur
  officiel ; le fournisseur OpenTofu est sous GPL-3.0, sans conséquence pour un
  usage d'outil mais à noter. La fédération Entra ID en SAML n'a pas été trouvée
  dans sa documentation.
- **Zitadel.** Fédération Entra ID documentée dans les deux protocoles,
  fournisseur OpenTofu du projet. Mais pas d'opérateur officiel, et sa licence
  est passée à l'AGPL : compatible avec un usage sans modification, mais toute
  modification servie à des utilisateurs devrait être publiée, ce qui est à
  qualifier avant d'en faire une brique commune.
- **Une authentification écrite dans kit-actix.** Un port d'authentification
  propre au kit éviterait une brique lourde, mais chaque fonction (second
  facteur, récupération de compte, fédération SAML) serait à écrire, à tester
  et à maintenir : c'est le savoir fragmenté qu'ADR-018 refuse pour le
  chiffrement.
- **Un fournisseur d'identité en nuage.** Comptes des utilisateurs chez un
  tiers, contre la souveraineté visée (même raisonnement qu'ADR-015 et
  ADR-017).

## Conséquences

- **Coût** : une instance Keycloak et sa base, permanentes, en production.
  L'ordre de grandeur publié par le projet (environ 1250 Mo par instance) est
  à confronter à la taille des machines de la recette VPS.
- **Nouvelle surface exposée** : les pages de connexion sont publiques et
  attirent le bourrage d'identifiants. Débit limité, journaux de connexion
  envoyés hors machine (ADR-023), alerte sur échecs anormaux.
- **Point de défaillance commun** : si Keycloak tombe, plus aucun utilisateur ne
  se connecte à aucun SaaS ; les sessions ouvertes survivent jusqu'à
  l'expiration de leurs jetons. Le témoin externe doit sonder la page de
  connexion.
- **Entretien** : mises à jour de Keycloak et de son opérateur (Renovate,
  ADR-025), passées par la seconde production éphémère d'abord ; rotation des
  secrets des clients OIDC et des fédérations.
- **kit-actix** devra recevoir un adaptateur OIDC, vérifiant les jetons émis
  par Keycloak, et un adaptateur local pour les gates, sur le modèle du port
  `Chiffreur` (ADR-018). Il n'existe pas aujourd'hui.
- **Limite** : Keycloak détient en clair des données personnelles que le
  chiffrement par personne ne couvre pas. Sa base devient une cible à part
  entière, protégée par la production, pas par OpenBao.
- **Limite, porteur seul** : une brique de plus, de grande surface
  fonctionnelle, à connaître assez pour la configurer sans erreur.

## Mesures CyFun 2025 couvertes

Renvoi à `foyer/skills/conformite.md` §2bis. Ces mesures sont citées dans les
autres ADR pour la plateforme elle-même ; leur application aux comptes des
utilisateurs des SaaS est à confirmer.

- **PR.AA-02.2** et **PR.AA-01.4** : identifiant unique par utilisateur, second
  facteur, ici offerts à chaque SaaS par la brique commune.
- **PR.AA-03.2 ★** : authentification forte, par la politique de second facteur
  déclarée dans le realm.
- **PR.AA-01.5 E** : désactivation des moyens d'authentification à risque et
  détection des usages atypiques (échecs de connexion).
- **PR.AA-05.3 ★** : chaque SaaS n'obtient que son client et ses rôles.
- **PR.PS-04.1 ★B** et **DE.CM-09.1 I** : journalisation des connexions et
  surveillance.
- **PR.DS-11.1 ★, PR.DS-11.2, PR.DS-11.4** : sauvegarde hors hôte et
  restauration éprouvée de la base de Keycloak.
- Fédération avec l'annuaire d'un client : catégorie PR.AA, identifiant à
  confirmer.

## Points ouverts

- **Lecture du statut Argo CD** (FR-012 du PRD de Sluis) : Flux étant retenu,
  l'outil MCP correspondant devra lire le statut de Flux. À reprendre dans le
  PRD.
- **Seuil de la règle « avant K3s »** : qu'est-ce qui compte comme « vrais
  utilisateurs » (premier client, première inscription publique) ; la règle
  est tranchée (point 11), son déclencheur reste à préciser.
- **Place dans ADR-033.** Après les trois premières étapes, sans les bloquer :
  étape nouvelle, ou sous-étape de la recette VPS ou de l'étape 5 (backend
  chiffré, qui touche aussi l'identifiant de personne) ?
- **Coût en mémoire face au principe n°4.** Une instance d'au moins un gigaoctet
  permanente se justifie-t-elle avant le premier SaaS qui a des utilisateurs, ou
  la brique n'arrive-t-elle qu'avec lui ?
- **Rôle d'OpenTofu.** ADR-032 le borne à la location d'infrastructure ;
  configurer Keycloak en fait un moteur de configuration applicative. Amender
  ADR-032, et ajouter `keycloak/keycloak` à la liste blanche d'ADR-034 ? Quel
  exécutant détient l'identifiant d'administration de Keycloak ?
- **Connexion unique entre SaaS** : écartée par le choix d'un realm par SaaS ;
  à rouvrir seulement si un besoin réel apparaît.
- **Haute disponibilité** : une instance au démarrage, combien à terme, et sur
  quels nœuds ?
- **Seconde production éphémère (ADR-020)** : restaure-t-elle la base de
  Keycloak ? Elle ne peut joindre aucun tiers, donc aucune fédération Entra ID ;
  comment s'y connecte-t-on pour la répétition d'une release ?
- **Rétention des sauvegardes de Keycloak** face au délai d'effacement d'ADR-019.
- **Comptes d'administration de Keycloak** : authentification par YubiKey
  (WebAuthn, à vérifier), compte nominatif, et lien avec ADR-015.
- **Sluis public** : son serveur OAuth 2.1 reste-t-il distinct, ou délègue-t-il
  un jour à Keycloak ? Hors du périmètre de cet ADR.
- **Vérifications restantes** : maturité CNCF de Keycloak, fonctions réservées à
  l'édition entreprise d'Authentik, maturité du fournisseur OpenTofu de Zitadel,
  ressources d'Authentik et de Zitadel, compatibilité des trois fournisseurs avec
  le registre d'OpenTofu.
