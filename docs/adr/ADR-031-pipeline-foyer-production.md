# ADR-031 : Du staging à la production, par l'une des deux passerelles d'approbation

- **Statut** : Proposé le 2026-10-21, non contresigné.
- **Amendé le 2026-10-21** (revue des ADR, décision 5) : la barrière est rejouée entre l'approbation et l'exécution. Amendement proposé, non contresigné.
- **Amendé le 2026-10-21** (revue des ADR, décision 6) : l'étape de staging devient la répétition sur la seconde production éphémère ; tests de charge et recette sur un environnement à part. Amendement proposé, non contresigné.
- **Amendé le 2026-10-21** (revue des ADR, décision 3) : l'étiquette d'approbation nomme son environnement cible. Amendement proposé, non contresigné.
- **Amendé le 2026-10-28** (sluis#6) : Flux est le seul réconciliateur, Coolify est abandonné (ADR-029). Amendement proposé, non contresigné.
- **Irréversible** : oui : la mise en production est le point irréversible par excellence (`foyer/skills/convergence-iac.md`).
- **S'articule avec** : **amende ADR-008** sans le remplacer (ADR-008 reste la passerelle des dépôts publics) ; ADR-012 (la demande passe par un outil MCP) ; **ADR-013** (étiquette signée, passerelle des dépôts privés) ; ADR-020 (seconde production éphémère) ; ADR-021 (barrière) ; ADR-022 (preuves) ; ADR-029 (Flux, d'un nœud à trois) ; ADR-030 (le MCP demande, n'exécute pas).
- **Sert la vision (ADR-000)** : le seul geste humain de la chaîne est la validation de l'irréversible ; principe 5, l'approbation est elle-même la preuve datée, produite par l'outil.

## Contexte

Un SaaS Foyer passe de la branche au staging, puis du staging à la production.
ADR-008 a fixé la passerelle : un environnement GitHub protégé à relecteur
obligatoire. Mais GitHub n'offre ce relecteur, hors Enterprise, que sur les
dépôts **publics** ; un projet témoin privé ne peut donc pas l'utiliser. Le
superviseur a tranché le 2026-10-21 : **deux passerelles**, choisies selon la
visibilité du dépôt, menant au même résultat.

## Décision

**La chaîne, identique pour tout SaaS :**

1. La branche est construite par la CI ; les gates du plancher sont vertes ; l'image porte le hash du commit.
2. **Recette et répétition**. Les **tests de charge** (campagne Sluis, moteur k6 visé par sluis#8) et la **recette fonctionnelle** se jouent sur un environnement à part, avec des comptes de test et des données factices. Puis la release, et sa migration s'il y en a une, est **jouée sur la seconde production éphémère** (ADR-020), restaurée de la nuit, avant la vraie production.
3. **Barrière de déploiement** (ADR-021) : contrôles déterministes oui/non ; un échec bloque la demande.
4. **Demande** : l'agent appelle `sluis_mise_en_ligne` (ADR-012, ADR-030), qui construit un plan empreinté.
5. **Approbation humaine**, par l'une des deux passerelles :

| Dépôt | Passerelle | Où l'humain approuve | Ce qui fait foi |
|---|---|---|---|
| **public** | environnement GitHub protégé à relecteur obligatoire (ADR-008) | l'interface GitHub du run | l'approbation enregistrée par GitHub, lue par Sluis (#3) |
| **privé** | étiquette `approuve/<empreinte>` signée par la clé SSH de signature du superviseur (ADR-013), dont le message porte l'empreinte complète du plan **et l'environnement cible** | son poste, `git tag -s` | la signature, vérifiée par le tireur de la cible contre un `allowed_signers` hors dépôt ; chaque tireur refuse, et journalise en erreur, une étiquette qui ne nomme pas son propre environnement |

6. **Barrière rejouée** (ADR-021, point 6) : juste avant l'exécution, les mêmes contrôles sont relancés. Si l'un d'eux a changé depuis l'approbation, le déploiement s'arrête, l'approbation est annulée, et le superviseur reçoit une alerte qui dit quel contrôle a changé et pourquoi. Il réapprouve ; rien n'est retenté automatiquement.
7. **Exécution** : l'approbation laisse passer la cible vers **Flux**, seul réconciliateur, à un nœud comme à trois (ADR-029). Ni Sluis ni l'agent ne font avancer ce que Flux applique autrement que par la passerelle.
8. **Preuve** : l'approbation (qui, quand, quelle empreinte, quel commit) entre dans le rapport daté du déploiement (ADR-022), stocké hors du serveur.

**Pourquoi deux et pas une.** La passerelle GitHub est câblée et testée
(ADR-012, #15 à #21) ; l'abandonner pour les dépôts publics retirerait un
garde-fou qui fonctionne. L'étiquette signée couvre le cas que GitHub ne couvre
pas sans payer. Les deux respectent la même propriété : ce qui demande ne peut
pas approuver.

## Alternatives écartées

- **GitHub seul**, avec un dépôt de déploiement public séparé pour chaque SaaS privé : deux dépôts à tenir et des journaux de runs publics.
- **GitHub Enterprise** : coût récurrent pour des projets sans revenu.
- **L'étiquette signée pour tous** : plus simple, mais abandonne ADR-008, câblé et testé.
- **Approbation dans le réconciliateur** : Flux (ou, dans la première version d'ADR-029, Coolify) deviendrait la passerelle ; il détient déjà les secrets de déploiement, et réunir l'approbation et l'exécution dans la même brique contredit « aucune brique ne détient tout ».

## Conséquences

- **Deux chemins à maintenir et à tester**, chacun avec son témoin (approbation absente ou mauvaise clé : rien ne se déploie). Pour l'étiquette signée, un témoin de plus : **une étiquette valide pour le staging ne déploie rien en production** (ADR-013, garde-fou 6).
- La skill Foyer ne voit pas la différence : `sluis_etat_approbation` rend le même état pour les deux passerelles (ADR-030).
- **Limite** : il n'y a pas de relecteur distinct de l'auteur chez un porteur seul. La signature personnelle, la barrière et le journal hors machine sont la mesure compensatoire (`foyer/skills/conformite.md`, §2bis).
- **Limite** : l'étiquette signée suppose la clé SSH de signature du superviseur, distincte de son identité GPG (ADR-015) ; sa perte bloque les mises en production des dépôts privés jusqu'au remplacement.

## Mesures CyFun 2025 couvertes

Mapping dans `foyer/skills/conformite.md`.

- Changements testés avant la production : PR.PS-06.2 (§2ter).
- Preuve de conformité par déploiement : PR.PS-04.1 ★ (Epic 9, story 9.3), avec ADR-022.
- Séparation des tâches, par mesure compensatoire : PR.AA-05.6 (§2bis).

## Points ouverts

- **Flux et les signatures** : Flux sait vérifier des signatures de commits et d'étiquettes, ce qui permettrait de faire de lui le tireur d'ADR-013. Vérifier quels formats sa version retenue prend en charge (GPG à coup sûr ; SSH à confirmer), sans quoi le cluster garde un tireur dédié.
- Le seuil de charge qui fait échouer l'étape 2 se fixe par SaaS, à partir des mesures de Sluis.
