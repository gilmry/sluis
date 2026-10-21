# Architecture Decision Records

Les ADR de Sluis vivent dans le livrable de l'Architecte,
[`../bmad/03-architecture.md`](../bmad/03-architecture.md), section *ADR*.
Ils n'y sont pas dupliqués ici : une décision recopiée à deux endroits
finit par diverger.

À partir du 2026-10-21, chaque nouvel ADR vit dans **son propre fichier**
(`ADR-0NN-titre.md`, ce dossier), et ce tableau en reste l'index. ADR-000 pose la
vision dont chacun se réclame. ADR-014 à ADR-033 décrivent l'architecture de
plateforme et de sécurité visée (alignement CyFun 2025 Essential) ; ADR-033 en
donne l'ordre de construction. Aucun ADR antérieur n'est supprimé ni remplacé :
ceux qui sont amendés le disent dans leur ligne.

| # | Décision | Irréversible | Statut |
|---|---|---|---|
| [ADR-000](ADR-000-vision.md) | Vision, l'humain lit les alertes et valide l'irréversible | non | proposée le 2026-10-21, non contresignée |
| ADR-001 | Hexagonale + SOLID | non | acceptée |
| ADR-002 | TDD + BDD + Documentation Vivante | non | acceptée |
| ADR-003 | DDD ubiquitous language → mapping code | non | acceptée |
| ADR-004 | Rust plutôt que le défaut BMAD Python + FastAPI | non | acceptée, écart méthodologique assumé |
| ADR-005 | Le contrat MCP est matérialisé, pas décrit | **oui** | acceptée |
| ADR-006 | Persistance CQRS SQL pur, pas d'ORM | **oui** | contresignée le 2026-08-29 · **amendée à la fabrication** : dépôt fichier, amendement contresigné le 2026-10-18 |
| ADR-007 | Dérogation Tier 2 bornée **et à durée limitée** pour les bacs à sable | **oui** | **contresignée le 2026-08-29** · fenêtre ouverte jusqu'au **2026-11-27** |
| ADR-008 | Passerelle d'approbation = environnement GitHub protégé | **oui** | acceptée · **amendée** pour les dépôts privés par ADR-013 et ADR-031 (proposées) |
| ADR-009 | Mapping branche → environnement | **oui** | acceptée |
| ADR-010 | Deux déploiements, et OpenTofu comme moteur | **oui** | **contresignée le 2026-10-18** |
| ADR-011 | Gate de scan différencié par image | non | **contresignée le 2026-10-18** |
| ADR-012 | La mise en ligne se demande par un outil MCP du Sluis public · amende ADR-010 | **oui** | **contresignée le 2026-10-19** · relance d'un run approuvé à vérifier au premier essai (#4, 4.5) |
| ADR-013 | Pour un dépôt privé, l'approbation est une étiquette signée par le superviseur, vérifiée par le tireur GitOps de la cible · amende ADR-008 | **oui** | **proposée le 2026-10-21, non contresignée** |
| [ADR-014](ADR-014-trois-zones.md) | Trois zones, la production, la confiance, le hors-ligne | **oui** | proposée le 2026-10-21, non contresignée |
| [ADR-015](ADR-015-identites.md) | Identités, des YubiKeys pour les humains, step-ca pour les machines | **oui** | proposée le 2026-10-21, non contresignée |
| [ADR-016](ADR-016-secrets-de-deploiement.md) | Secrets de déploiement chiffrés dans Git avec SOPS et age | non | proposée le 2026-10-21, non contresignée |
| [ADR-017](ADR-017-openbao-hors-cluster.md) | OpenBao hors du cluster, gardien de la seule clé maîtresse | **oui** | proposée le 2026-10-21, non contresignée |
| [ADR-018](ADR-018-chiffrement-applicatif.md) | Chiffrement applicatif par enveloppe, une clé par personne | **oui** | proposée le 2026-10-21, non contresignée |
| [ADR-019](ADR-019-base-de-cles-et-effacement.md) | Base de clés séparée et effacement cryptographique | **oui** | proposée le 2026-10-21, non contresignée |
| [ADR-020](ADR-020-staging-nocturne.md) | Staging restauré chaque nuit depuis la sauvegarde hors site, pseudonymisé | non | proposée le 2026-10-21, non contresignée |
| [ADR-021](ADR-021-barriere-de-deploiement.md) | Barrière de déploiement, des contrôles déterministes qui bloquent | non | proposée le 2026-10-21, non contresignée |
| [ADR-022](ADR-022-preuves-cyfun.md) | Preuves CyFun, un rapport daté par déploiement, conservé hors du serveur | non | proposée le 2026-10-21, non contresignée |
| [ADR-023](ADR-023-supervision.md) | Supervision sur la machine de confiance, surveillée de l'extérieur | non | proposée le 2026-10-21, non contresignée |
| [ADR-024](ADR-024-agent-ia.md) | L'agent IA d'exploitation collecte, propose, et n'agit jamais seul | non | proposée le 2026-10-21, non contresignée |
| [ADR-025](ADR-025-automatisation-exploitation.md) | Toute tâche d'exploitation récurrente est automatisée et signale son succès | non | proposée le 2026-10-21, non contresignée |
| [ADR-026](ADR-026-base-de-donnees-cloudnativepg.md) | Les bases des SaaS tournent sous CloudNativePG, archivées en continu hors région | **oui** | proposée le 2026-10-21, non contresignée |
| [ADR-027](ADR-027-stockage-des-fichiers.md) | Les fichiers vivent dans un fork communautaire de MinIO, chiffrés avant envoi | **oui** | proposée le 2026-10-21, non contresignée |
| [ADR-028](ADR-028-front-end-statique.md) | Le front-end est un site statique sur un stockage objet européen | non | proposée le 2026-10-21, non contresignée |
| [ADR-029](ADR-029-recettes-vps-et-k3s.md) | Deux recettes, VPS puis K3s, derrière les mêmes outils MCP | **oui** | proposée le 2026-10-21, non contresignée |
| [ADR-030](ADR-030-serveur-mcp.md) | Le serveur MCP lit ou demande, il n'exécute jamais | **oui** | proposée le 2026-10-21, non contresignée |
| [ADR-031](ADR-031-pipeline-foyer-production.md) | Du staging à la production, par l'une des deux passerelles d'approbation | **oui** | proposée le 2026-10-21, non contresignée |
| [ADR-032](ADR-032-outil-de-convergence-ansible.md) | Ansible configure le socle, OpenTofu loue, Flux réconcilie le cluster | **oui** | proposée le 2026-10-21, non contresignée |
| [ADR-033](ADR-033-feuille-de-route.md) | Feuille de route, huit étapes utilisables seules | non | proposée le 2026-10-21, non contresignée |

Tout point marqué irréversible exige une validation humaine explicite
avant d'être mis en œuvre. ADR-007 est le seul du lot qui assouplit un
garde-fou existant : il mérite une signature écrite, pas un accord oral.

C'est aussi le seul qui **s'auto-limite dans le temps**. Sa septième
condition fait expirer la dérogation par défaut et rend son
renouvellement dépendant du gate Tier 1 qu'elle relâche.

> ⏳ **Première fenêtre : 2026-08-29 → 2026-11-27 (90 jours).**
> Passé cette date et sans renouvellement Tier 1, tout bail de bac à
> sable est refusé. C'est voulu : le silence ne reconduit rien.


## Amendement d'ADR-006, 2026-08-29

L'implémentation persiste le dépôt OAuth **dans un fichier JSON** plutôt que
dans PostgreSQL. Deux raisons :

- Sluis sert un superviseur et ses projets, pas des organisations tierces. Le
  volume est de quelques clients et quelques jetons.
- `sqlx` en vérification à la compilation exige une base vivante au moment du
  build, ce qui contredirait NFR-06 : aucun test ne doit dépendre d'un service
  absent.

Ce que l'amendement **ne relâche pas** : la révocation reste durable, écrite
avant tout retour, et la consommation d'un code reste atomique. Le port
`DepotOAuth` rend la bascule vers PostgreSQL sans effet sur le reste du code, le
jour où le volume l'exigera. Amendement contresigné le 2026-10-18.


## ADR-010 — Deux déploiements, et OpenTofu comme moteur

**Contexte.** Le Sprint 5 a livré la campagne de charge, donc du code capable de
lancer `apply` et `destroy`. Un déploiement unique aurait fait du service exposé
à Internet celui qui détient les clés OVH.

**Décision, en deux parties.**

**1. Deux déploiements du même commit.** Un Sluis **public**, derrière Traefik,
qui ne détient rien ; un Sluis **exécutant**, sur un réseau sans entrée, qui
porte les moteurs et les clés du projet bac à sable. Ce qui est exposé ne détient
rien, ce qui détient n'est pas exposé.

La séparation est portée par **deux choses indépendantes**, et c'est délibéré :
la configuration (`depots_autorises` vide ⇒ `sluis_campagne` n'est pas
enregistré) **et** l'image (`--target public` n'embarque aucun moteur). Une
erreur de configuration sur le Sluis public ne suffirait donc pas à lui rendre
le pouvoir de provisionner : il n'aurait toujours pas de binaire à lancer.

**2. Le moteur d'infrastructure est OpenTofu, nommé `tofu`.** Terraform est sous
BSL 1.1 depuis la 1.6. Ce dépôt est MIT et publie une image sur un registre
public : y redistribuer du BSL serait contradictoire avec le choix de licence
comme avec le Manifeste. OpenTofu est le fork MPL-2.0, lit les mêmes fichiers
`.tf` et rend les mêmes résumés, que l'analyseur existant sait déjà lire.

**Ce qui est renommé, et ce qui ne l'est pas.** L'**exécutable** devient `tofu`,
dans l'allowlist, dans le diagnostic, dans l'adaptateur et dans les messages
d'erreur. Le **format** reste celui de Terraform : les répertoires `terraform/`
d'un dépôt mesuré s'appellent bien ainsi, les fichiers sont bien des `.tf`, et
les types qui les décrivent (`PlanTerraform`, `ModuleTerraform`) gardent donc ce
nom. Faire tourner OpenTofu sous le nom `terraform` aurait été le déguisement
que cette décision refuse.

**Alternatives écartées.** Terraform 1.5.7, dernière version MPL : redistribuable
mais figée depuis 2023, donc une dette à échéance connue. Terraform récent sous
BSL : contredit frontalement la licence du dépôt. Aucun moteur dans l'image, le
binaire venant de l'hôte : rend le déploiement dépendant d'une installation
manuelle, et l'image n'est plus autoportante.

**Conséquences.** Une seconde image à construire, scanner et publier à chaque
commit. Un second secret de signature, un second volume, un second journal
d'audit : les deux Sluis ne partagent rien, sans quoi leurs actes seraient
indiscernables. Et un moteur dont la compatibilité avec Terraform doit être
vérifiée sur les modules réels, ce que la première campagne dira.

**Irréversible ?** Oui pour la seconde partie : les modules mesurés porteront des
états écrits par OpenTofu. Contresigné le 2026-10-18. Le moteur de charge
`wrk` sera remplacé par k6 (#8), ce qui amendera cet ADR et ADR-011.


## ADR-011 — Gate de scan différencié par image

**Contexte, mesuré le 2026-09-02.** Le gate du plancher scanne chaque image en
`CRITICAL,HIGH` avec `ignore-unfixed`, et refuse de pousser si quoi que ce soit
sort. L'image exécutante en fait sortir **12**, toutes dans le binaire `tofu`,
c'est-à-dire dans le `go.mod` d'OpenTofu : `x/crypto`, `x/net`, `x/mod`,
`x/text`, `grpc`, `oras`. Aucune dans les paquets système, aucune dans wrk.

**Ce n'est pas un problème de moteur.** Mesuré à charge égale le même jour :
Terraform v1.16.1 en porte **9** dans son propre binaire, plus 2 dans sa base
Alpine. Changer de moteur déplace le chiffre, pas la nature du problème, qui est
d'embarquer un binaire Go tiers dans une image que l'on scanne.

**Décision.** Le gate devient **différencié, par image et non par sévérité** :

- **Sluis public** — gate plein, `CRITICAL,HIGH` bloquant sur tout. C'est
  l'image exposée à Internet, et tout ce qu'elle porte est de notre fait.
- **Sluis exécutant** — bloquant sur les paquets système et sur wrk, qui sont de
  notre ressort ; le binaire `tofu` est écarté du gate bloquant, et **de lui
  seul**, par `skip-files`.

Deux garde-fous, sans quoi la décision serait un simple relâchement :

1. **Ce qui est écarté est rapporté.** Un second scan, non bloquant, rejoue le
   scan complet et laisse les CVE de `tofu` lisibles dans le journal de chaque
   build. Une exception muette redevient un angle mort à la première montée de
   version.
2. **La version d'OpenTofu est épinglée dans le Dockerfile et se remonte.** La
   1.8.8 en portait 53, la 1.12.6 en porte 12 : le chiffre est un indicateur à
   suivre, pas une fatalité.

**Alternatives écartées.** Un `.trivyignore` daté listant les 12 CVE : il attrape
mieux les nouveautés, mais il est à refaire à chaque montée de version et pourrit
dès que personne ne le relit. Laisser le gate rouge et ne pas publier l'exécutant :
défendable aujourd'hui puisque H1 n'est pas levée, mais un job rouge en permanence
finit par ne plus être lu, et c'est alors tout le plancher qui est perdu.

**Ce que la décision ne couvre pas, et qu'il faut savoir.** Les bibliothèques
qu'`wrk` réclame sont copiées dans l'image comme des fichiers, sans leur
enregistrement `dpkg`. Trivy ne les rattache donc à aucun paquet et ne les
signale pas. Elles viennent de la même Debian 12 que la base et se remettent à
jour à chaque reconstruction, ce qui limite la portée du trou sans l'annuler.

Cet ADR n'est pas irréversible : rétablir le gate plein est une ligne de
configuration. Il a été contresigné le 2026-10-18, parce qu'il relâche un garde-fou du
plancher.


## ADR-012 — La mise en ligne se demande par un outil MCP du Sluis public

**Contexte.** Le critère de sortie n°1 du jalon zéro exige que l'approbation
GitHub soit câblée (#4). Aujourd'hui, `PasserelleGithub` et `MiseEnLigne` ne
sont composés dans aucun binaire, et la fenêtre d'ADR-007 n'a aucun chemin réel
de renouvellement avant son expiration du 2026-11-27. Deux questions étaient
ouvertes : quel Sluis détient le jeton qui déclenche le workflow de Tier 1, et
par quelle porte une demande arrive. Le superviseur a tranché le 2026-10-19 :
**le Sluis public, par un outil MCP**, pour qu'un agent puisse demander une mise
en ligne sans que le superviseur ait à ouvrir un terminal sur le serveur.

**Décision, en trois parties.**

**1. Soumettre un plan de Tier 1 est un acte de Tier 2.** L'outil
`sluis_mise_en_ligne` se déclare de Tier 2 : il ne met rien en ligne, il produit
un plan et le soumet. La mutation reste de Tier 1, exécutée par le workflow
GitHub après approbation humaine dans un environnement protégé, avec des
secrets que Sluis n'a pas. C'est la propriété qu'ADR-008 pose déjà : un Sluis
compromis ne peut que soumettre un plan refusable. La règle du serveur MCP
(« un outil de Tier 1 n'est jamais appelé directement ») est inchangée.

**2. ADR-010 est amendé : le Sluis public détient un jeton, et un seul.**
« Ce qui est exposé ne détient rien » devient « ce qui est exposé ne détient
rien qui mute sans un humain ». Le jeton GitHub est limité aux dépôts cibles et
aux droits que la passerelle exerce : déclencher le workflow, lire
l'environnement, lire les runs et leurs approbations. Son identité n'est jamais
celle d'un relecteur de l'environnement (faille S2) : Sluis refuse déjà de
dispatcher si `prevent_self_review` n'est pas actif (#2), ce qui interdit à
l'identité qui déclenche d'approuver son propre run.

**3. Le renouvellement d'ADR-007 se demande au public et s'ouvre chez
l'exécutant.** Le Sluis public soumet le plan `renouvellement_derogation`.
L'exécutant, qui seul détient le secret de signature de la fenêtre, ne l'ouvre
que sur une preuve qu'il lit **lui-même** chez GitHub, avec un jeton en lecture
seule : jamais sur la parole du Sluis public. Un public compromis peut donc
demander un renouvellement, pas l'obtenir.

**Garde-fous, sans lesquels la décision serait un relâchement.**

1. **Aucun texte libre ne traverse.** Les arguments de l'outil sont des valeurs
   validées : un projet de la liste d'autorisation, un environnement nommé, une
   référence au format d'un commit ou d'une étiquette. La description du plan
   est construite par Sluis, jamais recopiée de la demande. Ce qui a des droits
   ne lit pas de texte libre.
2. **Les gates du plancher sont vérifiées avant la soumission** (`MiseEnLigne`),
   et l'environnement est vérifié protégé avant le dispatch (#2).
3. **Chaque soumission est journalisée**, avec son empreinte, comme tout appel
   d'outil.
4. **L'approbateur enregistré est celui que GitHub rapporte** (#3), jamais
   l'identité qui a déclenché.

**Ce que la décision ne couvre pas, et qu'il faut savoir.** Un jeton fuité
permet d'inonder le relecteur de demandes, et le droit de déclencher un
workflow porte en général celui d'annuler ou de relancer un run. **À vérifier
avant contresignature** : qu'une relance d'un run déjà approuvé redemande
l'approbation de l'environnement protégé. Si ce n'est pas le cas, le jeton
public devient capable de rejouer une mise en ligne approuvée, et la partie 2
doit être revue.

**Alternatives écartées.** Le jeton chez l'exécutant, demandé par une commande
lancée sur le serveur : fidèle à la lettre d'ADR-010, mais la mise en ligne
reste alors un geste de terminal, ce qui contredit l'objectif d'autonomie
d'ADR-008. Un outil de Tier 1 approuvé côté client MCP : écarté par ADR-008,
inopérant en exécution autonome.

**Irréversible ?** Oui : le service exposé à Internet détient désormais un
secret. **Contresignée le 2026-10-19.** Le point sur la relance n'a pas pu être
tranché par la documentation de GitHub : il devient un test explicite du premier
essai bout en bout (#4, story 4.5), avant toute mise en ligne réelle.

## ADR-013 — Pour un dépôt privé, l'approbation est une étiquette signée

**Statut : proposée le 2026-10-21, non contresignée.** Amende ADR-008 pour les
dépôts privés ; ADR-008 reste la règle pour les dépôts publics.

**Contexte.** ADR-008 fait reposer l'approbation de Tier 1 sur un environnement
GitHub protégé à **relecteurs requis**. Or, selon la documentation de GitHub
(vérifiée le 2026-10-19) : « If you are on a GitHub Free, GitHub Pro, or GitHub
Team plan, required reviewers are only available for public repositories. »
Le premier projet témoin du jalon zéro est un dépôt **privé**. Sur lui, Sluis
refuse à juste titre de soumettre (#2 : environnement non protégé), et le
critère de sortie n°2 n'est pas atteignable par ADR-008. Le superviseur a écarté
le 2026-10-21 le plan Enterprise, le dépôt de déploiement public séparé et le
renoncement au relecteur, et retenu **une approbation hors GitHub**, sous la
forme d'une étiquette git signée par sa clé.

**Décision.**

1. **L'acte d'approbation est une étiquette git signée par la clé du
   superviseur**, posée sur le commit à mettre en ligne, au nom réservé
   `approuve/<empreinte courte>`. Son message porte l'empreinte complète du plan
   de changement. La clé de signature (SSH de préférence, GPG accepté) ne quitte
   jamais le poste du superviseur, et ne réside sur aucun serveur.
2. **La signature est vérifiée par le tireur GitOps de la cible, pas dans un
   workflow.** Un workflow vit dans le dépôt : quiconque peut y écrire pourrait
   le modifier pour sauter la vérification. Le tireur (le script que le cron de
   la cible lance déjà) ne déploie qu'un commit porteur d'une étiquette
   `approuve/*` dont la signature se vérifie contre un fichier
   `allowed_signers` **conservé sur la cible, hors du dépôt, en lecture seule
   pour le compte qui déploie**. Une étiquette non signée, signée par une autre
   clé, ou qui ne désigne pas le commit attendu, ne déploie rien.
3. **Sluis prépare, ne signe jamais.** Pour un dépôt privé, `sluis_mise_en_ligne`
   vérifie les gates du plancher comme aujourd'hui, calcule l'empreinte, et rend
   au superviseur la commande exacte à signer. `sluis_etat_approbation` lit les
   étiquettes avec un jeton **en lecture seule** et rapporte l'état. Ni le Sluis
   public ni l'exécutant ne détiennent de clé capable de produire une
   approbation.

**Garde-fous.**

1. **Usage unique.** Le tireur consigne chaque empreinte déployée avant de
   déployer ; une étiquette déjà consommée ne redéploie pas.
2. **Fraîcheur.** Une étiquette de plus de 72 h (date de la signature) est
   refusée, comme la demande de renouvellement d'ADR-012.
3. **Le dépôt ne fait pas foi pour les clés.** `allowed_signers` n'est jamais
   lu depuis le dépôt ; le modifier exige un accès au serveur.
4. **Journal.** Le tireur écrit l'empreinte, le commit, l'identité de la clé et
   l'horodatage de chaque déploiement : c'est la preuve datée qu'attend la
   conformité (Epic 9, PR.PS-04.1).
5. **Échec bruyant.** Une étiquette `approuve/*` qui ne se vérifie pas est
   journalisée en erreur, pas ignorée en silence.

**Ce que la décision ne couvre pas, et qu'il faut savoir.**

- **La clé du superviseur devient le secret critique.** Sa perte empêche toute
  mise en ligne ; son vol permet d'approuver. Une clé sur support matériel est
  recommandée ; la révocation consiste à la retirer d'`allowed_signers`.
- **Un compromis de la cible reste un compromis de la cible.** Le tireur détient
  déjà de quoi déployer ; ADR-013 ne change rien à ce périmètre.
- **Il n'y a plus de relecteur distinct de l'auteur.** C'est le cas du porteur
  seul : la mesure compensatoire est la signature personnelle et le journal
  (même raisonnement que les mesures compensatoires du porteur seul de Foyer).

**Mode opératoire du superviseur.** À faire sur **son poste**, jamais sur un
serveur : la clé de signature ne doit résider nulle part ailleurs.

1. *Créer une clé de signature dédiée*, distincte de la clé de connexion SSH,
   protégée par une phrase de passe :

   ```bash
   # support matériel FIDO2, recommandé
   ssh-keygen -t ed25519-sk -C "signature git" -f ~/.ssh/signature_git
   # à défaut
   ssh-keygen -t ed25519 -C "signature git" -f ~/.ssh/signature_git
   ```

2. *Configurer git pour signer en SSH* :

   ```bash
   git config --global gpg.format ssh
   git config --global user.signingkey ~/.ssh/signature_git.pub
   git config --global tag.gpgsign true      # optionnel : commit.gpgsign aussi
   ```

3. *Déclarer la clé à GitHub* comme **Signing Key** (*Settings → SSH and GPG
   keys*), pour que l'état rapporté par Sluis et le badge « Verified »
   reflètent la signature. Ce badge est informatif : il ne fait pas foi.

4. *Approuver un plan* : Sluis rend la commande exacte, de la forme

   ```bash
   git tag -s approuve/<empreinte courte> <commit> -m "<empreinte complète>"
   git push origin approuve/<empreinte courte>
   ```

5. *Vérifier localement* avant de pousser, avec le même mécanisme que le
   tireur :

   ```bash
   echo "<courriel> $(cat ~/.ssh/signature_git.pub)" > ~/.ssh/allowed_signers
   git config --global gpg.ssh.allowedSignersFile ~/.ssh/allowed_signers
   git tag -v approuve/<empreinte courte>     # attendu : Good "git" signature
   ```

6. *Installer la clé publique sur la cible* : le même format de ligne, dans un
   `allowed_signers` hors du dépôt, propriété de `root`, en lecture seule pour
   le compte qui déploie, et ne contenant **que** la clé du superviseur.

**Révocation et perte.** Retirer la ligne de la clé dans l'`allowed_signers` de
chaque cible suffit à révoquer : plus aucune étiquette signée par elle ne
déploie. Une clé perdue se remplace par une nouvelle clé, installée de la même
façon ; les étiquettes déjà consommées restent valides dans le journal, qui
garde l'identité de la clé.

**Ce qu'un agent signe, et ce qu'il ne signe pas.** Les commits produits par un
agent ne portent pas la signature du superviseur et ne doivent jamais la
porter. Un commit « Unverified » d'agent à côté d'une étiquette « Verified » du
superviseur est la distinction voulue entre ce qui est **proposé** et ce qui
est **approuvé**.

**Alternatives écartées.** GitHub Enterprise (coût récurrent pour un projet
expérimental) ; dépôt de déploiement public séparé (journaux de runs publics,
deux dépôts à tenir) ; environnement sans relecteur (l'invariant « Sluis ne
peut que demander » tomberait) ; vérification dans un workflow GitHub
(contournable par qui peut écrire dans le dépôt) ; mutation par l'exécutant de
Sluis (il n'est pas déployé, et ce serait plus de code pour la même garantie).

**Irréversible ?** Oui : c'est une seconde passerelle d'approbation. À
contresigner par le superviseur avant toute mise en œuvre.
