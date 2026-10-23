# ADR-039 : Dépendance à GitHub, et miroir automatique vers Codeberg et Forgejo

- **Statut** : Proposé le 2026-10-22, non contresigné.
- **Décision du superviseur du 2026-10-23**, intégrée aux points 1 et 4 : les dépôts privés sont copiés vers un **Forgejo auto-hébergé**, les dépôts publics vers Codeberg.
- **Irréversible** : probablement non. Un miroir s'ajoute et se retire sans toucher au dépôt d'origine ni à la production. Deux réserves : un dépôt privé poussé une fois sur un service tiers y a été copié, ce qui ne se défait pas ; et l'identité du miroir détient un jeton dont la fuite permettrait d'écrire sur le miroir.
- **S'articule avec** : ADR-008, ADR-012 et ADR-031 (la passerelle d'approbation est un environnement GitHub protégé) ; ADR-013 (l'étiquette signée transite par le dépôt d'origine) ; ADR-007 (son renouvellement passe par cette passerelle) ; ADR-016 (secrets dans Git, lisibles sans GitHub) ; ADR-021 et ADR-034 (gates lues sur les check-runs) ; ADR-022 (preuves conservées hors du serveur) ; ADR-025 (Renovate, prévu) ; ADR-033 (feuille de route) ; ADR-035 et ADR-036 (proposés, reconstruction depuis Git) ; ADR-038 (proposé en parallèle, double registre d'images).
- **Sert la vision (ADR-000)** : règle « aucune brique ne détient tout », parce qu'aujourd'hui un seul fournisseur détient à la fois le code, la CI, les gates, l'approbation, les secrets de mutation et les images ; principe n°3, parce qu'une copie tenue par un second fournisseur se compare à l'original ; principe n°2, parce que le miroir doit signaler son propre échec.

## Contexte

Sluis s'appuie sur GitHub bien au-delà de l'hébergement du code. Le superviseur
a demandé, par lettre de mission du 2026-10-22, un inventaire de cette
dépendance et une première protection. L'inventaire ci-dessous est mesuré dans
ce dépôt le 2026-10-22. Il ne couvre pas les dépôts cibles, qui ne sont pas ici.

Trois scénarios sont considérés : **indisponibilité** (panne, quelques heures
ou jours), **fermeture du compte** (suspension, perte d'accès, erreur de
modération), **changement de conditions** (fonction retirée ou rendue payante).
Le troisième n'est pas théorique : ADR-013 existe parce que les relecteurs
requis ne sont offerts, hors Enterprise, que sur les dépôts publics.

| Usage | Où, dans ce dépôt | Indisponibilité | Fermeture ou changement de conditions | Gravité |
|---|---|---|---|---|
| **Hébergement Git** (code, historique, étiquettes) | `origin` (`docs/DEMARRAGE.md`), `deploy.sh` (`git fetch origin main`) | Ni poussée ni tirage ; le déploiement GitOps ne voit plus rien | Perte du dépôt de référence ; les étiquettes `approuve/*` (ADR-013) absentes des clones locaux sont perdues | **Critique** en fermeture, élevée en panne |
| **Actions, CI du plancher** | `.github/workflows/ci.yml` (fmt, clippy, pureté du domaine, tests, gitleaks, SBOM CycloneDX, cargo audit) | Aucun verdict, donc aucune mise en ligne | Les commandes restent rejouables sur une machine nue (NFR-06, commentaire de `ci.yml`), mais plus aucun verdict n'est produit ni conservé | Élevée |
| **Check-runs comme gates** | `src/infrastructure/gates_github.rs` ; ADR-021, ADR-034 point 2 | Gates rouges par construction (fail-closed) : mise en ligne refusée, sans danger | À reconstruire contre une autre source de verdict | Élevée |
| **Environnements protégés, `workflow_dispatch`, approbations** | `src/infrastructure/github.rs`, `.github/workflows/tier1-apply.yml.example` ; ADR-008, ADR-012, ADR-031 | Aucune mutation de Tier 1 ; aucun renouvellement d'ADR-007 (`src/application/ouverture_derogation.rs` lit l'approbation chez GitHub) | La passerelle des dépôts publics disparaît ; ADR-013 seule ne la remplace pas sans un tireur par cible | **Critique** |
| **Secrets d'Actions** | `tier1-apply.yml.example` (`OS_PASSWORD`, `KUBECONFIG`), `GITHUB_TOKEN` des workflows | Inutilisables | Illisibles par construction : s'ils n'existent pas ailleurs, ils sont perdus | Moyenne si une copie existe ailleurs (ADR-016), critique sinon |
| **GHCR** | `.github/workflows/docker-publish.yml`, `docker-compose.yml`, `docker-compose.executant.yml`, `deploy.sh` | Ni publication ni tirage ; ce qui tourne continue de tourner, mais ne se redéploie pas | Plus d'images épinglées au commit ; renvoi à ADR-038 | Élevée |
| **Jeton de la passerelle** | `SLUIS_GITHUB_TOKEN` (`src/configuration.rs`, `src/infrastructure/composition.rs`) | Rien à faire avec | Révoqué avec le compte | Suit la passerelle |
| **Cache et artefacts d'Actions** | `cache-from: type=gha` ; `actions/upload-artifact@v4` (SBOM) | Build plus lent ; SBOM non publié | Le SBOM conservé par GitHub est perdu ; ADR-022 demande déjà un stockage hors serveur | Faible pour le cache, moyenne pour le SBOM |
| **Actions tierces et téléchargements au build** | `dtolnay/rust-toolchain`, `Swatinem/rust-cache`, `gitleaks/gitleaks-action`, `docker/*`, `aquasecurity/trivy-action` (épinglée par empreinte) ; `Dockerfile` (OpenTofu et wrk tirés de github.com) | L'image ne se construit plus | Dépend de chaque projet tiers, pas seulement de notre compte | Moyenne |
| **Issues comme backlog** | Epic 9 (sluis#10), #2, #3, #4, #16 cités par les ADR ; 25 issues au 2026-10-22 | Backlog illisible | Backlog et discussions perdus ; les renvois `#n` des ADR pointent dans le vide | Moyenne |
| **Clé de signature déclarée à GitHub** | ADR-013, mode opératoire, étape 3 | Badge « Verified » absent | Idem | Faible : le badge ne fait pas foi, `allowed_signers` sur la cible si |
| **Renovate** (prévu) | ADR-025 | Pas de PR de montée de version | À rebrancher sur une autre forge | Faible aujourd'hui, rien n'est câblé |
| **OAuth** | `src/domain/oauth.rs`, `src/infrastructure/serveur_http.rs`, `src/infrastructure/oauth_depot.rs` | Aucun effet | Aucun effet | **Nulle** : Sluis est son propre serveur OAuth 2.1, les identifiants sont vérifiés par l'application hôte, aucune connexion par compte GitHub |

Dependabot n'est pas configuré dans ce dépôt (aucun `.github/dependabot.yml`).

Ce que montre le tableau : en panne, le système **échoue fermé**. Rien ne se
met en ligne, rien ne mute, et c'est le comportement voulu par ADR-008. Le
danger est ailleurs : une fermeture de compte emporte en même temps le code de
référence, le backlog, les secrets de mutation et la passerelle, et rien n'en
garde aujourd'hui de copie tenue à jour hors de GitHub, à part les clones
locaux, dont la fraîcheur dépend de la mémoire de quelqu'un.

## Décision

1. **Chaque dépôt est copié automatiquement hors de GitHub**, branches et
   étiquettes comprises, dont les étiquettes `approuve/*` d'ADR-013 :
   - les **dépôts publics, sous licence libre, vers Codeberg** ;
   - les **dépôts privés vers un Forgejo auto-hébergé** (décision du
     superviseur du 2026-10-23), puisque les conditions de Codeberg n'admettent
     pas les dépôts privés non libres.
2. **Le miroir est en lecture seule.** On n'y pousse rien à la main, on n'y
   ouvre ni issue ni PR. GitHub reste la source de vérité ; le miroir est une
   copie, pas une seconde forge.
3. **Le miroir signale son échec** (ADR-000, principe n°2) : un miroir qui
   cesse de se mettre à jour doit alerter, et l'écart entre les deux têtes de
   branche se compare (principe n°3).
4. **Forgejo sert de miroir aux dépôts privés, pas encore de forge.** Il est
   en lecture seule comme Codeberg (point 2). En faire une forge complète
   (CI, issues, approbations) reste une évolution future : c'est un service de
   plus à tenir, sauvegarder et surveiller (principe n°4), à rouvrir si une
   bascule réelle hors de GitHub devient nécessaire.

### Ce que le miroir protège

- **Le code et son historique**, commits, branches, étiquettes, signatures
  comprises : une fermeture du compte GitHub ne les emporte plus.
- **La capacité de reconstruire depuis Git** visée par ADR-035 et ADR-036
  (proposés) : un clone du miroir suffit à reconstruire une image, à rejouer
  le plancher sur une machine nue (NFR-06) et à relire les secrets chiffrés
  par SOPS (ADR-016), qui ne dépendent d'aucune forge.
- **La preuve des approbations par étiquette** (ADR-013) : l'étiquette signée
  est un objet Git, copiée avec le reste et vérifiable hors de GitHub.

### Ce que le miroir ne protège pas

- **Actions et la CI** : le miroir ne rejoue aucun workflow.
- **Les approbations par environnement protégé** (ADR-008) : elles vivent dans
  la base de GitHub, pas dans Git. Ni l'approbation ni son auteur ne sont
  copiés.
- **Les check-runs** : un commit copié n'emporte pas ses verdicts.
- **Les secrets d'Actions** : illisibles, donc non copiables.
- **Les issues** et leurs discussions : un miroir Git ne les transporte pas.
- **GHCR et les images** : renvoi à ADR-038, double registre d'images.

Autrement dit, le miroir garantit qu'on peut **repartir**, pas qu'on peut
**continuer**. Une bascule hors de GitHub resterait un chantier.

## Alternatives écartées

- **Ne rien faire, compter sur les clones locaux.** Leur fraîcheur dépend d'un
  geste humain, et ils n'ont pas forcément les étiquettes poussées par
  d'autres. Contraire au principe n°1.
- **Forgejo auto-hébergé dès maintenant.** Plus complet, mais hors de
  proportion avec la règle de capacité actuelle ; noté comme évolution
  (décision, point 4).
- **Migrer hors de GitHub.** La passerelle d'ADR-008, câblée et testée (ADR-031),
  devrait être refaite. Le besoin exprimé est une assurance, pas un
  déménagement.
- **Sauvegarde périodique en archive** (`git bundle` vers un stockage objet).
  Protège aussi le code, mais reste invisible et non consultable ; un miroir se
  clone et se lit. Les deux ne s'excluent pas.

## Conséquences

- **Coût** : faible. Un compte ou une organisation chez Codeberg, une identité
  et un jeton dédiés, une tâche planifiée ou un réglage de miroir.
- **Un secret de plus** : le jeton du miroir. Sa portée doit se limiter à
  l'écriture des dépôts miroirs.
- **Un second fournisseur à suivre** (GV.SC, voir plus bas) : ses conditions,
  sa disponibilité, ses limites de ressources.
- **Limite** : un miroir copie aussi les erreurs. Un secret poussé en clair sur
  GitHub l'est aussi sur Codeberg ; la gate gitleaks reste la seule défense.
- **Limite** : un miroir mis à jour par poussée forcée reproduit une réécriture
  d'historique. Le miroir protège contre la perte du fournisseur, pas contre
  une erreur sur l'origine.
- **Limite** : la passerelle d'approbation et la CI restent entièrement chez
  GitHub. La dépendance la plus grave du tableau n'est pas réduite par cette
  décision.

## Mesures CyFun 2025 couvertes

Renvoi à `foyer/skills/conformite.md` :

- **Dépendance fournisseur** : catégorie **GV.SC**. Le seul identifiant déjà
  cité dans ces ADR est **GV.SC-07.3 E** (ADR-023, dépendance fournisseur à
  auditer) ; son rattachement à cet inventaire est **à confirmer**.
- **Inventaire des logiciels et services** : catégorie **ID.AM** (ADR-022 et
  ADR-027 citent ID.AM-02.1 et ID.AM-02.2 pour l'inventaire logiciel) ;
  identifiant exact pour un inventaire de services externes à confirmer.
- **Copie hors fournisseur du code** : catégorie **PR.DS-11** (sauvegardes) ;
  les identifiants cités ailleurs portent sur les données, leur application au
  code source est à confirmer.

## Points ouverts

- **Sens du miroir.** Poussée depuis GitHub (une tâche ou un workflow pousse
  vers Codeberg) ou tirage par Codeberg (miroir de dépôt réglé côté Codeberg).
  La poussée dépend d'Actions, donc de GitHub ; le tirage dépend de Codeberg et
  doit lire les dépôts privés avec un jeton GitHub. Où le signal d'échec est
  le plus simple à produire est un critère.
- **Identité et jeton du miroir, et leur détenteur.** Quel compte, sur quelle
  machine, avec quelle portée et quelle rotation. Ce ne peut être ni
  l'identité de la passerelle (ADR-012) ni celle d'un relecteur.
- **Où vit le Forgejo des dépôts privés.** Il contient le code privé : ni
  sur la machine de confiance (zone 2, non exposée, et aucune brique ne détient
  tout), ni chez le même hébergeur que la production si l'on veut survivre à la
  fermeture de ce compte. Machine, hébergeur, sauvegarde de Forgejo lui-même,
  supervision et coût sont à décider ; les conditions de Codeberg (lues le
  2026-10-22, § 1 (2) et § 2 (1) 2) motivent ce choix et restent à relire par
  le superviseur.
- **Export des issues.** Le miroir ne les couvre pas. Faut-il un export
  périodique (API GitHub vers fichiers versionnés), et où le conserver ?
- **Secrets d'Actions.** Vérifier que chaque secret de mutation existe aussi
  ailleurs, chiffré (ADR-016) ; sinon, la fermeture du compte les perd.
- **Place dans la feuille de route** (ADR-033). Proposition à arbitrer : après
  les trois premières étapes, puisqu'il ne conditionne pas une première
  production ; ou plus tôt, puisqu'il est petit et sans risque.
- **Tireurs GitOps et Flux.** Doivent-ils pouvoir tirer depuis le miroir en
  secours ? Cela changerait la source de vérité d'une cible, et touche
  ADR-013 et ADR-029.
- **Fenêtre d'ADR-007.** Son renouvellement passe par la passerelle GitHub ;
  une indisponibilité au moment de l'échéance fermerait la fenêtre (fail-closed
  voulu). À garder en tête, pas à corriger ici.
