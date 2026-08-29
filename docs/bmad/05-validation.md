---
livrable: Rapport de Validation croisée
persona: Validateur
phase_togaf: F — Planification de la migration
projet: Sluis
version: 0.1.0-draft
genere_le: 2026-08-29
depend_de: [01-product-brief.md, 02-prd.md, 03-architecture.md, 04-epics-stories.md]
signature_humaine:
  nom:
  role:
  date:
  verdict:
---

# Rapport de Validation croisée — Sluis

*Livrable du Validateur · TOGAF Phase F. Verdict objectivé : on ne sort de BMAD qu'au PASS.*

## Statut global : **PASS**

**Fidelity score : 100/100.** Au-dessus du seuil de 80, et toutes les cases obligatoires sont désormais cochées.

> **Deuxième passage, 2026-08-29.** Le premier passage rendait CONCERNS à 88/100. Trois remédiations ont été appliquées : I-1 par rebouclage vers l'Architecte, I-2 et I-3 par décision du superviseur (périmètre complet, pas de MVP). Le détail des constats initiaux est conservé au §10 pour la traçabilité.

### Détail du calcul

| Terme | Numérateur / dénominateur | Points |
|---|---|---|
| Capacités du Brief couvertes en PRD | 10 / 10 | 25,00 |
| FR couvertes en Architecture | 24 / 24 | 25,00 |
| FR couvertes en Stories | 24 / 24 | 25,00 |
| Matrice 4×N présente **dans le PRD** | 96 / 96 | 25,00 |
| **Total** | | **100,00** |

Aucun terme n'est en défaut.

## 1. Cohérence DDD

- Glossaire (ubiquitous language) cohérent Brief→PRD→Archi : ☑ — les 16 termes du Brief §8 sont figés au PRD §4 et mappés vers des types à l'Architecture. Aucune dérive détectée.
- Bounded contexts stables : ☑ — les 6 BC du Brief §9 sont repris à l'identique au PRD §5 et à l'Architecture Couche 1, avec leurs dépendances autorisées explicitées.
- Invariants métier présents et codables : ☑ — les 10 invariants du Brief §10 ont chacun un mécanisme de codage nommé à l'Architecture Couche 1. Sept sur dix sont rendus *inconstructibles* plutôt que vérifiés, ce qui est plus fort que ce que la méthode exige.

**Point saillant.** Le choix `consume(self) -> ConsumedToken` déplace l'invariant d'usage unique vers la compilation. `SandboxProjectId` distinct de `ProductionProjectId` fait de même pour la confusion de projet. C'est la meilleure application du principe « invariants codés dans les constructeurs » observée dans les livrables Maury à ce jour.

## 2. Couverture SOLID

- **SRP** ☑ — un port par préoccupation, un use case par outil.
- **OCP** ☑ — l'ajout d'un moteur passe par un adaptateur, sans toucher au domaine.
- **LSP** ☑ — les doublures de test se substituent aux adaptateurs réels sans contrat dégradé.
- **ISP** ☑ — 12 ports étroits plutôt qu'une interface d'orchestration unique. `AuditLog` n'expose qu'`append`, ce qui porte l'invariant d'immuabilité dans la forme de l'interface.
- **DIP** ☑ — vérifié **mécaniquement** par le job `purete-domaine`, pas seulement déclaré. C'est l'application correcte de la distinction guidance/enforcement.

## 3. Traçabilité

- Brief → PRD (toute capacité a ses exigences) : ☑ — 10/10, table de correspondance vérifiée capacité par capacité.
- PRD → Architecture (toute exigence a sa couche) : ☑ — 24/24 après remédiation de I-1.
- PRD → Stories (toute exigence a ses stories) : ☑ — 24/24, chaque FR est rattachée à au moins une story qui la nomme.

## 4. Architecture hexagonale

- Backend, dépendances vers l'intérieur : ☑ — et rendu objectivable par un job de CI dédié, ce qui est le bon registre.
- Frontend (hexagonale light) : **sans objet** — archétype API-first, couche hors scope. L'exception du formulaire OAuth est tracée et justifiée par deux précédents.

## 5. Glossaire → code (mapping présent) : ☑

16 termes sur 16 mappés vers des types nommés et localisés à l'Architecture, après remédiation de I-1.

## 6. BDD + Documentation Vivante (flux critiques couverts) : ☑

Les 5 flux critiques du PRD §9 sont identifiés, et chacun a une story qui le porte. Le flux 2 (dégradation propre sur machine nue) est particulièrement bien traité : il apparaît en NFR-06, en story 0.1 `@edge`, et en story 1.4 `@edge`.

## 7. TDD (stratégie de tests + couverture cible) : ☑

Cibles différenciées par couche, 95 % sur le domaine, et une contrainte structurante correctement posée : aucun test ne dépend d'un binaire d'infrastructure. RED-first affirmé au Brief §16 et à ADR-002.

## 7bis. Classes de tests 4×N (mécanisé)

- Matrice 4×N par FR présente dans le PRD : ☑ — **24 FR sur 24**, chacune avec ses critères Gherkin et ses quatre classes, après remédiation de I-2.
- 4 classes listées dans chaque story : ☑ — 30 stories sur 30, sans exception.
- Aucune FR orpheline d'une classe : ☑ aux deux niveaux, PRD et Stories.

La décision de périmètre complet a rendu la seconde option de remédiation (déclarer les 11 FR hors périmètre de la gate) sans objet : elles ont donc été spécifiées au format long. La responsabilité de la spécification est revenue au Product Manager, où elle devait être.

## 8. Readiness organisationnelle

- Scrum : ☑ — 5 sprints, story habilitante en Sprint 0, réserve d'émergence de 20 %.
- Nexus : sans objet à cette échelle.
- SAFe : sans objet.
- ITIL (pré-prod) : ☑ — story transverse prévue, avec runbook du chien de garde et procédure de révocation de jeton.

## 9. « Agent IA Ready »

- **Specs assez précises pour qu'un agent boucle sans deviner** : ☑ — chaque story porte Gherkin, 4 classes, couche, taille, tours et DoD. La DoD commune est factorisée en tête, ce qui évite la répétition sans perdre l'exigence.
- **Critères d'acceptation testables** : ☑ — les critères sont formulés en observables, pas en intentions. Le critère du jalon de fabrication est chiffré (« 3 topologies, 4 environnements, 3 profils, 4 modules ») et vérifiable automatiquement contre un dépôt réel.
- **Points irréversibles identifiés et marqués « validation humaine »** : ☑ — 5 points irréversibles tracés en ADR (005 contrat, 006 persistance, 007 dérogation bac à sable, 008 passerelle, 009 mapping branche), chacun avec alternatives écartées et conséquences.

**Verdict sur ce critère, qui est l'objectif déclaré du superviseur** : la backlog est effectivement exécutable sans supervision continue. Deux réserves à porter au débat :

- La story 0.3 (harnais de contract testing) est un prérequis dur : tant qu'elle n'est pas verte, aucune story touchant le contrat n'est réellement vérifiable. L'ordre n'est donc pas négociable.
- Les 6 stories d'émergence ne sont, par nature, pas spécifiées. Elles représentent 17 % du total et redeviendront un point de supervision.

## 10. Incohérences détectées

**I-1 — FR-024 n'avait pas d'élément d'architecture dédié** *(→ Architecte)* — **RÉSOLUE**
La mise en ligne exige la vérification des gates du plancher (secrets, SBOM, scan d'image, fichier de retour de migration). Aucun port `GateChecker` n'existait dans la table des ports de la Couche 2, et *Gate du plancher* était absent du mapping glossaire → code, alors que la story 7.2 en dépend explicitement dans sa classe `@security`.
**Remédiation appliquée** : port `GateChecker` ajouté à la Couche 2, entrées `Gate du plancher` et `Prior` ajoutées au mapping glossaire → code.

**I-2 — Matrice 4×N incomplète dans le PRD** *(→ Product Manager)* — **RÉSOLUE**
11 FR sur 24 étaient présentées en table résumée, sans critères Gherkin ni classes de tests.
**Remédiation appliquée** : les 11 FR (FR-014 à FR-024) ont été détaillées au format long. La seconde option envisagée, les déclarer hors périmètre de la gate, est devenue sans objet du fait de la décision de périmètre complet.

**I-3 — Dépassement de la target MVP** *(→ superviseur)* — **CLOSE PAR DÉCISION**
Le Brief §18 fixait une target de 4 jours de superviseur pour un MVP ; la backlog en produisait 5,5, soit 37 % de plus. Le Scrum Master proposait trois options, toutes portant sur la réduction d'un périmètre MVP.
**Décision du superviseur, 2026-08-29 : périmètre complet, pas de découpage MVP.** Les trois options sont sans objet. La target a été remplacée par une target de périmètre complet à 12 jours de superviseur, que la backlog tient à 11,2. **Marge résiduelle : 0,8 jour, soit 7 %.** C'est mince pour 38 stories, et c'est le point de vigilance à porter au suivi.

**I-4 — Hypothèses bloquantes non levées** *(→ superviseur)*
H1 (projet OVH dédié aux bacs à sable) bloque FR-014, et H2 (environnements GitHub protégés sur les dépôts cibles) bloque FR-021. Aucune ne bloque le démarrage. Le périmètre étant complet, elles ne sont plus optionnelles : elles doivent être levées avant les Sprints 3 et 4 respectivement.

**I-5 — Ambiguïté résiduelle sur le nom d'hôte** *(mineure)*
Le PRD H3 note que le superviseur a mentionné `n8n.ecosolva com` alors que la convention existante est `.ecosolva.org`. Traité par une variable `SLUIS_DOMAIN`, donc sans impact structurel. À confirmer avant la story 8.3.

## 11. Recommandations

Avant passage en Phase 2 (chef de projet : WBS, Gantt, coût).

1. ~~Corriger I-1~~ — **fait** (port `GateChecker`).
2. ~~Trancher I-2~~ — **tranché** : les 11 FR sont spécifiées au format long, la décision de périmètre complet ayant écarté l'option « hors périmètre ».
3. ~~Trancher I-3~~ — **tranché** : périmètre complet. **Point de vigilance conservé** : la marge sur la target n'est plus que de 0,8 jour de superviseur. Le nombre de tours réellement consommés dans le Sprint 0 doit être mesuré et servir à recaler l'estimation de tous les sprints suivants, conformément au §9 de l'abaque.
4. **Lever H1 et H2 avant les Sprints 3 et 4.** Le périmètre étant complet, elles ne sont plus optionnelles : H1 (projet OVH dédié aux bacs à sable) conditionne FR-014, H2 (environnements GitHub protégés) conditionne FR-021. Aucune ne bloque le démarrage, les deux bloquent l'achèvement.
5. **Contresigner ADR-007** au frontmatter. La dérogation Tier 2 sur les bacs à sable a été approuvée oralement le 2026-08-29 ; elle doit porter une signature écrite, car c'est le seul assouplissement d'un garde-fou existant dans tout le dispositif.

---

**Verdict : PASS.** Les trois incohérences sont closes : I-1 par rebouclage vers l'Architecte, I-2 par rebouclage vers le Product Manager, I-3 par décision du superviseur.

Conformément à la condition de sortie de `BMAD-Conception.md`, la sortie validée déclenche la Phase 2 (chef de projet : WBS, Gantt, coût). Ce passage est une décision engageante : **il reste au superviseur à signer les cinq frontmatters et à contresigner ADR-007**, seule dérogation à un garde-fou existant du dispositif.

Deux réserves accompagnent ce PASS sans l'empêcher :

- **Marge de 7 % sur la target**, à surveiller dès le Sprint 0.
- **H1 et H2 non levées**, bloquantes pour l'achèvement des Sprints 3 et 4, pas pour le démarrage.

---

*Dérivé du Manifeste Maury (CC BY-SA 4.0). Source : [`github.com/gilmry/manifest`](https://github.com/gilmry/manifest).*
