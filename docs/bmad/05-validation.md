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

## Statut global : **CONCERNS**

**Fidelity score : 88/100.** Au-dessus du seuil de 80, donc éligible au PASS sur le score seul. Mais une case obligatoire du §7bis échoue, ce qui interdit le PASS en l'état.

### Détail du calcul

| Terme | Numérateur / dénominateur | Points |
|---|---|---|
| Capacités du Brief couvertes en PRD | 10 / 10 | 25,00 |
| FR couvertes en Architecture | 24 / 24 | 25,00 |
| FR couvertes en Stories | 24 / 24 | 25,00 |
| Matrice 4×N présente **dans le PRD** | 52 / 96 | 13,54 |
| **Total** | | **88,54 → 88** |

Le terme faible est le dernier, et il est la cause du verdict.

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

- Matrice 4×N par FR présente dans le PRD : ☐ **ÉCHEC** — 13 FR sur 24 la portent (FR-001 à FR-013). Les 11 FR post-MVP (FR-014 à FR-024) sont présentées en table résumée, avec la mention « chacune sera détaillée au format complet avant son sprint ».
- 4 classes listées dans chaque story : ☑ — 36 stories sur 36, sans exception.
- Aucune FR orpheline d'une classe : ☐ **ÉCHEC au niveau PRD**, ☑ au niveau Stories.

**C'est la cause du verdict CONCERNS.** La situation n'est pas grave sur le fond, puisque les 4 classes existent bien pour ces 11 FR, mais dans le livrable 04 et non dans le 02. Le risque réel n'est pas l'absence de couverture, c'est que la spécification d'une exigence vive dans le livrable du Scrum Master plutôt que dans celui du Product Manager, ce qui inverse la responsabilité et fragilise la relecture.

## 8. Readiness organisationnelle

- Scrum : ☑ — 5 sprints, story habilitante en Sprint 0, réserve d'émergence de 20 %.
- Nexus : sans objet à cette échelle.
- SAFe : sans objet.
- ITIL (pré-prod) : ☑ — story transverse prévue, avec runbook du chien de garde et procédure de révocation de jeton.

## 9. « Agent IA Ready »

- **Specs assez précises pour qu'un agent boucle sans deviner** : ☑ — chaque story porte Gherkin, 4 classes, couche, taille, tours et DoD. La DoD commune est factorisée en tête, ce qui évite la répétition sans perdre l'exigence.
- **Critères d'acceptation testables** : ☑ — les critères sont formulés en observables, pas en intentions. Le critère d'acceptation du MVP est chiffré (« 3 topologies, 4 environnements, 3 profils, 4 modules ») et vérifiable automatiquement contre un dépôt réel.
- **Points irréversibles identifiés et marqués « validation humaine »** : ☑ — 5 points irréversibles tracés en ADR (005 contrat, 006 persistance, 007 dérogation bac à sable, 008 passerelle, 009 mapping branche), chacun avec alternatives écartées et conséquences.

**Verdict sur ce critère, qui est l'objectif déclaré du superviseur** : la backlog est effectivement exécutable sans supervision continue. Deux réserves à porter au débat :

- La story 0.3 (harnais de contract testing) est un prérequis dur : tant qu'elle n'est pas verte, aucune story touchant le contrat n'est réellement vérifiable. L'ordre n'est donc pas négociable.
- Les 6 stories d'émergence ne sont, par nature, pas spécifiées. Elles représentent 17 % du total et redeviendront un point de supervision.

## 10. Incohérences détectées

**I-1 — FR-024 n'avait pas d'élément d'architecture dédié** *(→ Architecte)* — **RÉSOLUE**
La mise en ligne exige la vérification des gates du plancher (secrets, SBOM, scan d'image, fichier de retour de migration). Aucun port `GateChecker` n'existait dans la table des ports de la Couche 2, et *Gate du plancher* était absent du mapping glossaire → code, alors que la story 7.2 en dépend explicitement dans sa classe `@security`.
**Remédiation appliquée** : port `GateChecker` ajouté à la Couche 2, entrées `Gate du plancher` et `Prior` ajoutées au mapping glossaire → code.

**I-2 — Matrice 4×N incomplète dans le PRD** *(→ Product Manager)*
11 FR sur 24 sans matrice au livrable 02. Voir §7bis.
**Remédiation** : deux options. Soit compléter les 11 FR au format long (coût : ~1 h). Soit déclarer formellement au PRD §3 que FR-014 à FR-024 sont hors du périmètre de cette gate et feront l'objet d'un second passage BMAD avant leur sprint, ce qui est cohérent avec la progression YAGNI affirmée à l'Architecture Couche 5.

**I-3 — Dépassement de la target MVP de 42 %** *(→ superviseur, pas un persona)*
Le Brief §18 fixe une target de challenge à 4 jours de superviseur pour le MVP ; la backlog en produit 5,7. Ce n'est pas une incohérence entre livrables mais un écart assumé et signalé par le Scrum Master, avec trois options chiffrées. Il appartient au superviseur de trancher, c'est précisément l'objet d'une gate review.

**I-4 — Hypothèses bloquantes non levées** *(→ superviseur)*
H1 (projet OVH dédié aux bacs à sable) bloque FR-014, et H2 (environnements GitHub protégés sur les dépôts cibles) bloque FR-021. Aucune ne bloque le MVP. Elles doivent être levées avant les Sprints 3 et 4 respectivement, pas avant de commencer.

**I-5 — Ambiguïté résiduelle sur le nom d'hôte** *(mineure)*
Le PRD H3 note que le superviseur a mentionné `n8n.ecosolva com` alors que la convention existante est `.ecosolva.org`. Traité par une variable `SLUIS_DOMAIN`, donc sans impact structurel. À confirmer avant la story 8.3.

## 11. Recommandations

Avant passage en Phase 2 (chef de projet : WBS, Gantt, coût).

1. ~~Corriger I-1~~ — **fait**. C'était le seul défaut de traçabilité réel, et il relevait de l'Architecte, pas du superviseur.
2. **Trancher I-2** en gate review. Je recommande la seconde option : déclarer FR-014 à FR-024 hors périmètre de cette gate. Détailler maintenant onze exigences dont les trois premières dépendent d'hypothèses non levées produirait de la spécification qui vieillira avant d'être exécutée, ce que la progression YAGNI déconseille.
3. **Trancher I-3** en gate review. Je rejoins la recommandation du Scrum Master : réduire le périmètre du MVP plutôt que le Sprint 0. Sortir la story 3.6 et la moitié de la 3.5 ramène le MVP à ~4,5 jours sans compromettre le critère d'acceptation du PRD §13. Réduire le Sprint 0 reviendrait à reléguer le harnais de contract testing, exactement l'erreur que `contrat-api.md` documente comme ayant déjà coûté un NO-GO.
4. **Ne pas lever H1 et H2 maintenant.** Elles ne bloquent pas le MVP, et les lever prématurément consommerait du superviseur pour un besoin qui n'existe pas encore.
5. **Contresigner ADR-007** au frontmatter. La dérogation Tier 2 sur les bacs à sable a été approuvée oralement le 2026-08-29 ; elle doit porter une signature écrite, car c'est le seul assouplissement d'un garde-fou existant dans tout le dispositif.

---

**Verdict : CONCERNS.** I-1 est résolue par rebouclage vers l'Architecte. Restent **deux décisions qui appartiennent au superviseur** et à personne d'autre : I-2 (périmètre de la matrice 4×N) et I-3 (dépassement de la target MVP). Aucune ne se tranche par un calcul.

Une fois I-2 tranchée dans l'option « déclarer hors périmètre » (le dénominateur du terme 4×N devient 52/52), le score passe à **100/100** et le verdict à **PASS**. Dans l'option « détailler les 11 FR », il y passe aussi, au prix d'environ une heure.

---

*Dérivé du Manifeste Maury (CC BY-SA 4.0). Source : [`github.com/gilmry/manifest`](https://github.com/gilmry/manifest).*
