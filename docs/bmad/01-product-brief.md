---
livrable: Product Brief
persona: Analyste
phase_togaf: A — Vision
projet: Sluis
version: 0.1.0-draft
genere_le: 2026-08-29
signature_humaine:
  nom:
  role:
  date:
  verdict:
---

# Product Brief — Sluis

*Livrable de l'Analyste · TOGAF Phase A (Vision). Les champs non renseignés par le superviseur sont marqués `[hypothèse à valider]`.*

## 1. Vision

Pour un superviseur qui pilote plusieurs projets Maury avec une armée d'agents IA, Sluis transforme l'infrastructure OVH d'un ensemble de conventions de dossiers exécutables à la main en un **domaine typé, interrogeable et gouverné**, pour qu'un agent puisse mesurer, proposer et déployer sans jamais pouvoir nuire.

## 2. Stakeholders

| Rôle | Qui | Intérêt |
|---|---|---|
| Superviseur / décideur | Gilles Maury | Doit pouvoir donner un goal et s'absenter, tout en gardant le *répondre-de* |
| Co-conceptrice de la méthode | Farah Maury | Cohérence Sluis ↔ Méthode Foyer, transmissibilité |
| Agent exécutant | Claude Opus en runtime | Consommateur principal du contrat MCP |
| Projets consommateurs | KoproGo, BANKO, OpenMajor, Elevia, derniere-chance | Bénéficient de l'orchestration sans la réimplémenter |
| Organisation pilote Foyer | `[hypothèse à valider]` | Reprend le dispositif chez elle |
| Subit sans décider | L'hébergeur OVH, les utilisateurs finaux des apps déployées | Continuité de service |

## 3. Drivers business

- **Le premier objectif prioritaire est l'adoption de la Méthode Foyer par une organisation pilote.** Une méthode qui ne sait pas déployer reste un document. Sluis est le chaînon qui rend la boucle Foyer démontrable de bout en bout.
- **Le poste de coût dominant est le superviseur, pas le modèle** (`abaque-cout-capacite.md` §1). Tout ce qui retire une interruption humaine sans retirer le *répondre-de* déplace directement la rentabilité.
- **Les constantes du modèle de coût sont des priors non calibrés.** L'abaque marque `[caler]` une douzaine de valeurs qui gouvernent les décisions d'architecture. Sans mesure, les arbitrages de palier se prennent à l'intuition.
- **Le garde-fou actuel est binaire.** Le `deny` de `koprogo/.claude/settings.json` interdit aux agents toute mutation d'infrastructure. Le choix se réduit à « l'agent ne déploie pas » ou « on ouvre une brèche ».

## 4. Problème

Le superviseur qui veut faire avancer un projet Maury jusqu'en production doit aujourd'hui être physiquement présent à chaque étape qui compte. Il connaît l'état de son infrastructure par mémoire et par lecture de dossiers ; il lance les tests de charge à la main et n'en réinjecte jamais les résultats dans son modèle de coût ; et au moment de déployer, il doit soit tout faire lui-même, soit désarmer les protections qu'il a lui-même posées.

Le résultat est que le travail avance à la vitesse de sa disponibilité, que ses décisions d'architecture reposent sur des ordres de grandeur devinés, et que le seul moyen d'aller plus vite serait de dégrader la sécurité.

## 5. Proposition de valeur

Un point de passage unique qui rend l'infrastructure **lisible par un agent**, **mesurable pour de vrai**, et **mutable sous condition**, là où l'existant offre soit l'opacité, soit l'ouverture totale.

Ce que l'existant n'apporte pas :

- Terraform et Ansible savent muter, mais ne savent rien dire de l'état d'ensemble ni refuser selon un tier.
- Le Makefile de KoproGo offre une surface uniforme, mais à un humain, et sans journal ni autorisation.
- Les serveurs MCP d'Elevia et de derniere-chance savent authentifier et écrire, mais sur un domaine métier, sans notion de danger gradué.
- Aucun ne ferme la boucle entre une mesure de charge et le modèle de coût qui gouverne les décisions.

## 6. Personas

**Le superviseur absent.** Rôle : donne un goal, part travailler ailleurs. Objectifs : que le travail avance, garder le *répondre-de*, n'être interrompu que pour ce qui l'exige vraiment. Frustrations : être sollicité pour des décisions sans enjeu, découvrir après coup qu'un agent a été bloqué six heures sur une permission.

**L'agent exécutant.** Rôle : boucle sur une backlog. Objectifs : connaître l'état réel sans deviner, savoir ce qu'il a le droit de faire, obtenir une approbation sans humain au clavier. Frustrations : conventions implicites, commandes refusées sans alternative, résultats non structurés à reparser.

**Le platform engineer.** Rôle : répond des incidents. Objectifs : savoir qui a fait quoi, quand, sur quelle empreinte ; pouvoir rejouer et auditer. Frustrations : les mutations sans trace.

**L'organisation pilote.** Rôle : adopte Foyer. Objectifs : reprendre le dispositif sans dépendre de son auteur. Frustrations : un outillage non transmissible qui la rendrait captive. `[hypothèse à valider]`

## 7. Capacités métier requises

Ordonnées de la plus réversible à la moins réversible, comme le demande la doctrine « louer le réversible, posséder l'irréversible ».

| # | Capacité | Réversibilité | Tier |
|---|---|---|---|
| C1 | Décrire l'infrastructure déclarée d'un projet (topologies, environnements, profils, modules) | totale, lecture | 2 |
| C2 | Décrire l'infrastructure réelle chez OVH (projets, instances, coûts, DNS) | totale, lecture | 2 |
| C3 | Diagnostiquer l'écart entre déclaré et réel (`plan`, statuts Helm et ArgoCD) | totale, lecture | 2 |
| C4 | Louer une infrastructure éphémère bornée, prouver sa convergence, la détruire | forte, TTL garanti | 2 borné |
| C5 | Conduire une campagne de montée en charge et en produire des mesures | forte | 2 borné |
| C6 | Recaler le modèle de coût et capacité sur du mesuré | totale, documentaire | 2 |
| C7 | Proposer une mutation d'infrastructure sous forme de plan signé | totale, ne mute rien | 2 |
| C8 | Obtenir l'approbation humaine d'un plan et en suivre l'exécution | nulle, c'est le point irréversible | 1 |
| C9 | Mettre un projet en ligne après vérification des gates du plancher | nulle | 1 |
| C10 | Tenir un journal d'audit inaltérable de tout ce qui précède | append-only par construction | 2 |

## 8. Glossaire métier (ubiquitous language DDD)

| Terme | Définition |
|---|---|
| **Sluis** | L'écluse. Le service lui-même, et la métaphore : deux portes jamais ouvertes ensemble. |
| **Topologie** | Forme de déploiement d'un projet : `vps`, `k3s` ou `k8s`. |
| **Environnement** | Étage de promotion : `dev`, `integration`, `staging`, `production`. Strictement ordonné. |
| **Profil de cluster** | Contrat entre provisionnement (Day 1) et déploiement (Day 2) : classe de stockage, ingress, TLS, backend de secrets, préréglage de ressources. |
| **Déploiement** | Le quadruplet projet × topologie × environnement × profil, dans une région OVH. |
| **Tier** | Niveau d'autorisation d'une action. `Tier 2` autonome et journalisé, `Tier 1` validation humaine obligatoire. Vocabulaire repris d'`AGENT_GUARDRAILS.md`, jamais redéfini. |
| **Plan de changement** | Description immuable et empreintée d'une mutation envisagée. N'exécute rien. |
| **Jeton de changement** | Preuve d'approbation liée à une empreinte de plan, à durée de vie limitée, à usage unique. |
| **Bail de bac à sable** | Droit d'occuper une infrastructure éphémère : projet, TTL, plafond de dépense. Expire toujours. |
| **Chien de garde** | Processus indépendant qui détruit un bail expiré, même si le demandeur a disparu. |
| **Campagne de charge** | Suite ordonnée de paliers de charge appliquée à un déploiement, produisant des mesures. |
| **Mesure de capacité** | Fait observé et daté : P99, débit, empreinte mémoire, jeu chaud, pression, coût réel. |
| **Prior** | Constante supposée du modèle de coût, marquée `[caler]`, en attente d'être remplacée par une mesure. |
| **Convergence** | Propriété prouvée : ré-appliquer l'état déclaré ne produit aucun écart. |
| **Gate du plancher** | Vérification objectivable et irréversible qui casse le build : secrets, SBOM, scan d'image, retour de migration. |
| **Passerelle d'approbation** | Le mécanisme externe qui porte la décision humaine. Ici, un environnement GitHub protégé. |

## 9. Bounded contexts pressentis (DDD)

| BC | Responsabilité | Ce qu'il ne fait pas |
|---|---|---|
| **BC1 — Inventaire** | Connaître le déclaré et le réel, et l'écart entre les deux | Ne mute jamais rien |
| **BC2 — Autorisation** | Classer une action en tier, produire un plan, exiger et vérifier un jeton | N'exécute aucune mutation |
| **BC3 — Bac à sable** | Baux éphémères : TTL, plafond, destruction garantie | Ne touche à aucun projet de production |
| **BC4 — Capacité** | Campagnes de charge, mesures, recalage des priors | Ne décide d'aucune architecture |
| **BC5 — Exécution** | Piloter les moteurs (Terraform, config, Helm, Kustomize, ArgoCD) et la passerelle | Ne décide d'aucun tier |
| **BC6 — Accès** | Identité, OAuth 2.1, scopes, journal d'audit | N'interprète aucune sémantique métier |

## 10. Invariants métier critiques

Candidats à coder dans les constructeurs, donc à rendre inconstructibles s'ils sont violés.

1. **Un plan visant la production est nécessairement de Tier 1.** Aucun chemin ne permet de construire un plan Tier 2 sur `Environment::Production`.
2. **Un bail de bac à sable a toujours un TTL et un plafond de dépense.** Un bail sans l'un des deux n'existe pas.
3. **Un bail ne peut viser qu'un projet OVH de la liste d'autorisation des bacs à sable**, disjointe par construction de la liste des projets de production.
4. **Un jeton de changement n'est valide que pour l'empreinte exacte du plan qui l'a demandé**, non expiré, et non déjà consommé.
5. **Un jeton est consommé exactement une fois.** Un rejeu est un échec, jamais un succès silencieux.
6. **L'ordre de promotion des environnements est total et non contournable** : on ne promeut pas vers `staging` ce qui n'a pas passé `integration`.
7. **Une entrée de journal d'audit est immuable** : le journal n'expose ni modification ni suppression.
8. **Une mesure de capacité porte toujours sa provenance** : mesuré ou supposé, jamais ambigu (§9 de l'abaque).
9. **Un secret ne franchit jamais la frontière de sortie**, quel que soit le chemin d'appel.
10. **Une campagne ne se termine jamais sans destruction du bail**, y compris en cas de panique, d'arrêt du processus ou d'erreur du moteur.

## 10bis. Flux multi-acteurs

| Capacité | Initiateur | Validateur | Consommateur | Workflow |
|---|---|---|---|---|
| C4 Bac à sable | Agent | Client MCP (confirmation d'appel) | Agent | L'agent demande un bail borné, le client MCP confirme, Sluis loue, le chien de garde détruit à l'échéance |
| C5 Campagne | Agent | aucun (dans le bail) | Superviseur | Paliers successifs, collecte, rapport |
| C6 Recalage | Agent | Superviseur en gate review | Méthode Foyer | Le rapport propose des valeurs mesurées, le superviseur décide de mettre à jour l'abaque |
| C8 Mutation | Agent | **Superviseur via environnement GitHub protégé** | Projet cible | Plan → dispatch → blocage GitHub → approbation → exécution par le job → compte rendu |
| C9 Mise en ligne | Agent | **Superviseur**, après gates vertes | Utilisateurs finaux | Vérification des gates, plan Tier 1, approbation, déploiement, rollback si tests post-déploiement rouges |

## 11. Fonctionnalités du périmètre

**Décision du superviseur du 2026-08-29 : le périmètre est complet, sans découpage MVP.** Les dix capacités du §7 sont livrées. L'ordre ci-dessous est un ordre de fabrication, pas une priorisation qui autoriserait un abandon.

**Socle de lecture** — serveur MCP en transport stdio (`initialize`, `tools/list`, `tools/call`) · `sluis_doctor` · `sluis_inventory` · `sluis_cluster_profiles` · lecture OVH (projets, instances, coûts, DNS) · `tf_plan`, statuts Helm, rendu Kustomize, statut ArgoCD · écart déclaré/réel · journal d'audit append-only · contrat d'outils matérialisé et prouvé par des contract tests.

**Écriture bornée** — baux de bac à sable avec TTL et plafond · chien de garde indépendant · preuve de convergence · campagnes de charge en paliers · mesures avec provenance · rapport de recalage des priors de l'abaque.

**Tier 1** — plans de changement empreintés · jetons à usage unique · passerelle GitHub à environnement protégé · mise en ligne avec vérification des gates du plancher et rollback.

**Accès distant** — OAuth 2.1 + PKCE, transport Streamable HTTP, scopes, déploiement sur le serveur ecosolva.

## 12. Hors périmètre

Ce qui attend un besoin réel, au titre de la progression YAGNI.

- Support d'un second fournisseur d'infrastructure. Le domaine est agnostique par construction, seul l'adaptateur manquerait.
- Multi-location pour des organisations tierces isolées.
- Toute interface utilisateur au-delà du formulaire de connexion OAuth.

## 13. Dimensionnement projet

| Dimension | Valeur estimée |
|---|---|
| Bounded Contexts | 6 |
| Entités domain estimées | ~26 (≈4-5 par BC) |
| Outils MCP estimés (l'équivalent d'endpoints ici) | ~24, tous dans le périmètre |
| Endpoints HTTP hors MCP | 5 (discovery, register, authorize ×2, token) |
| Catégorie projet | **Moyen** (5-10 BC) |

## 14. Contraintes

- **Conformité** : RGPD, ISO 27001 (le mapping des contrôles vers l'IaC est un livrable de l'Architecte), NIS2.
- **Souveraineté** : OVH, EU-West. Aucune donnée de production ne quitte l'UE.
- **Licence** : MIT, alignée sur derniere-chance. Choix du superviseur, en écart assumé avec l'AGPL-3.0 de KoproGo.
- **Stack imposée** : Rust 2021 / Actix-web, en écart assumé avec le défaut BMAD (Python LTS + FastAPI) → ADR-001.
- **Archétype** : stateful × API-first. Conséquence directe : le contrat est un livrable de premier rang écrit avant le code, et le harnais de contract testing est une sous-story non optionnelle du Sprint 0.
- **Multilingue** : sans objet. Sluis n'a pas d'interface utilisateur finale, hormis un formulaire de connexion technique.
- **Environnement de développement** : `terraform`, `ansible-playbook`, `helm`, `kubectl`, `kustomize` et `argocd` sont absents de la machine de développement. Aucun test ne peut en dépendre.
- **Vocabulaire d'autorisation** : celui d'`AGENT_GUARDRAILS.md`, repris sans redéfinition.

## 15. Risques

| Risque | Probabilité | Impact | Mitigation |
|---|---|---|---|
| Sluis devient un contournement des garde-fous plutôt que leur exécuteur | Moyenne | **Critique** | Sluis ne détient aucun secret de mutation de production ; ils vivent dans GitHub Actions derrière un environnement protégé |
| La dérogation Tier 2 sur les bacs à sable s'élargit par glissement | Moyenne | Élevé | Les six conditions sont des invariants de domaine, pas des vérifications de surface ; toute extension exige un ADR |
| Un bail éphémère survit à son TTL et facture indéfiniment | Moyenne | Élevé | Chien de garde indépendant du processus demandeur, plus plafond de dépense contrôlé à l'admission |
| Le protocole MCP évolue et le JSON-RPC écrit à la main dérive | Élevée | Moyen | Surface volontairement minimale, contract tests, version de protocole déclarée explicitement |
| Le co-hébergement avec n8n élargit la surface d'attaque | Certaine | Moyen | Acceptable tant que Sluis ne détient pas les clés de mutation ; à revoir immédiatement si ce n'est plus vrai |
| Les mesures de charge sont prises pour des vérités générales | Élevée | Moyen | Toute mesure porte sa provenance et ses conditions ; le rapport distingue mesuré et supposé (§9 de l'abaque) |
| Les priors de l'abaque restent non calibrés faute de campagne réelle | Élevée | Élevé | C'est précisément la capacité C5-C6 ; la première campagne cible le prior le plus structurant, le control-plane K3s |
| L'écart Rust vs défaut BMAD Python fragilise la transmissibilité | Faible | Moyen | Trois projets Maury sont déjà en Rust ; tracé en ADR-001 |
| La backlog produite n'est pas réellement exécutable sans le superviseur | Moyenne | **Critique** | C'est le critère §9 du rapport de validation, avec un score de fidélité minimal de 80 |

## 16. Principes d'architecture

Les non-négociables.

1. **Hexagonale stricte**, DDD au centre. Le domaine n'importe aucun crate d'infrastructure.
2. **Le vocabulaire d'autorisation existant fait loi.** Tier 1 et Tier 2 au sens d'`AGENT_GUARDRAILS.md`.
3. **Sluis ne détient jamais les secrets de mutation de production.** Propriété structurelle, pas règle de conduite.
4. **Ce qui n'est pas objectivable ne bloque pas ; ce qui est objectivable et irréversible bloque toujours** (`gates.md`).
5. **Louer le réversible, posséder l'irréversible ; au doute, traiter comme irréversible** (`convergence-iac.md`).
6. **Le contrat précède le code** et est matérialisé, pas décrit (`contrat-api.md`).
7. **Aucune logique métier dans les handlers MCP.** Un outil est un adaptateur mince vers un use case testé.
8. **`tools/list` gouverne la découvrabilité, jamais l'autorisation.** `tools/call` revérifie systématiquement.
9. **RED-first**, quatre classes de tests obligatoires, `Result<_, AppError>` typé.
10. **Toute mesure porte sa provenance.** Mesuré et supposé ne se confondent jamais.

## 17. Métriques de succès

| Métrique | Cible | Comment on la mesure |
|---|---|---|
| Découverte sans saisie | `sluis_inventory` sur `koprogo/infrastructure` ressort 3 topologies, 4 environnements, 3 profils, 4 modules Terraform | Test d'acceptation automatisé |
| Priors calibrés | ≥ 3 constantes `[caler]` de l'abaque remplacées par du mesuré | Diff sur `abaque-cout-capacite.md` |
| Interruptions évitées | Une campagne complète de 7 paliers s'exécute sans sollicitation humaine | Journal d'audit |
| Sûreté prouvée | 100 % des tests `@security` de la liste minimale au vert | CI |
| Étanchéité des secrets | 0 secret dans les sorties d'outil | Test de non-régression dédié |
| Fiabilité du bac à sable | 0 bail survivant à son TTL sur 30 jours | Journal du chien de garde |
| Traçabilité | 100 % des actions Tier 1 avec empreinte, approbateur et horodatage | Journal d'audit |
| Transmissibilité | Un tiers déploie Sluis avec le seul README | `[hypothèse à valider]` |

## 18. Estimation budgétaire préliminaire (point 0)

- **Échelle visée** : Scrum.
- **Seed** : hérité à stack égale de KoproGo, Elevia et derniere-chance (Rust hexagonal, Actix, OAuth MCP déjà écrit deux fois). Ce n'est pas une estimation à froid, ce qui resserre la fourchette vers le bas.
- **Calibrage stories** : `total ≈ scénarios BDD ÷ 4`. Avec ~24 outils et 5 endpoints, à 4 classes chacun, l'ordre de grandeur est de **28 à 34 stories**.
- **Coût superviseur** : à 0,5 à 1 jour par story et un ratio de supervision de 3, l'ordre de grandeur est de **9 à 12 jours de superviseur**. C'est le poste dominant.
- **Coût modèle** : ~3 tours par story, ~30 stories, soit ~90 tours. À quelques centaines de milliers de tokens par tour, l'ordre de grandeur reste **sous les 50 €**, donc négligeable devant le poste superviseur, conformément au §1 de l'abaque.
- **Target de challenge** : **tenir le périmètre complet sous 12 jours de superviseur**, borne haute de la fourchette. Le périmètre n'étant pas négociable par décision du superviseur, c'est le ratio de supervision qui devient la variable d'ajustement, pas le contenu. L'écart à cette cible est le premier signal de dérive.

> Estimation = prior incertain, resserré story après story par le CSI. Les chiffres ci-dessus sont des `[caler]` au même titre que ceux de l'abaque : ils demandent à être remplacés par du mesuré dès le premier jalon.

---

*Dérivé du Manifeste Maury (CC BY-SA 4.0). Source : [`github.com/gilmry/manifest`](https://github.com/gilmry/manifest).*
