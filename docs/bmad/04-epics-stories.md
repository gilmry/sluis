---
livrable: Epics & User Stories
persona: Scrum Master
phase_togaf: E — Opportunités et solutions
projet: Sluis
version: 0.1.0-draft
genere_le: 2026-08-29
depend_de: 03-architecture.md
signature_humaine:
  nom:
  role:
  date:
  verdict:
---

# Epics & User Stories — Sluis

*Livrable du Scrum Master · TOGAF Phase E. Chaque story est un tour du cercle, dimensionné sur les deux axes.*

**Convention de lecture.** Une story est exécutable par un agent sans le superviseur si et seulement si elle porte ses critères Gherkin, ses quatre classes de tests, sa couche, sa taille et sa Definition of Done. Toute story qui n'en remplit pas une est réputée non prête, et ne peut pas entrer en sprint.

**Definition of Done, commune à toutes les stories** : tests des 4 classes verts · `make ci` vert (dont `purete-domaine`, gitleaks, SBOM) · contract test si la story touche le contrat · aucun `unwrap()` ni `expect()` hors tests · doc vivante à jour.

---

## Sprint 0 — Fondations *(la story habilitante — précède tout)*

Livre « la capacité de boucler ». Aucune story fonctionnelle ne démarre avant que ce sprint soit vert.

### Story 0.1 — Squelette hexagonal et CI de plancher
- **En tant que** agent exécutant **je veux** un dépôt qui compile et une CI qui tranche **afin de** obtenir un signal objectif à chaque tour.
- **Critères Gherkin** :
  - `Étant donné le dépôt fraîchement cloné, Quand je lance make ci, Alors fmt, clippy en -D warnings, les tests, gitleaks et le SBOM passent tous.`
  - `Étant donné une machine sans terraform ni helm, Quand je lance make ci, Alors la CI est verte malgré tout.`
- **Classes de tests** :
  - `@happy` — `make ci` vert sur dépôt sain
  - `@negative` — un `warning` clippy introduit fait échouer la CI
  - `@edge` — CI sur machine nue, sans aucun binaire d'infrastructure
  - `@security` — un secret factice commité fait échouer gitleaks, sur le diff **et** sur l'historique
- **Couche** : IaC / CI
- **Taille** : L (1 j) · **Tours** : 3
- **Bloque** : toutes les autres stories

### Story 0.2 — Gate mécanique de pureté du domaine
- **En tant que** superviseur **je veux** que la CI refuse tout import d'infrastructure dans le domaine **afin de** rendre l'invariant hexagonal objectivable plutôt que déclaratif.
- **Critères Gherkin** :
  - `Étant donné un fichier de src/domain/ important reqwest, Quand la CI tourne, Alors le job purete-domaine échoue en nommant le fichier et le crate fautif.`
- **Classes de tests** :
  - `@happy` — domaine pur, job vert
  - `@negative` — import de `sqlx`, `reqwest`, `actix_web` ou `tokio` : échec explicite à chaque fois
  - `@edge` — import transitif via un `pub use`, import derrière un `#[cfg(test)]` (toléré, et le test le prouve)
  - `@security` — le job ne peut pas être contourné par `--no-verify`, la CI le rejoue côté serveur
- **Couche** : IaC / CI
- **Taille** : M (0,75 j) · **Tours** : 2

### Story 0.3 — Harnais de contract testing *(sous-story non optionnelle, `contrat-api.md`)*
- **En tant que** agent exécutant **je veux** que tout schéma d'outil déclaré soit prouvé conforme à sa désérialisation **afin de** que le contrat soit matérialisé et non décrit.
- **Critères Gherkin** :
  - `Étant donné un outil dont le schéma déclare un champ absent du type Rust, Quand les contract tests tournent, Alors ils échouent en nommant l'outil et le champ.`
  - `Étant donné une charge utile portant un champ inconnu, Quand tools/call la désérialise, Alors elle est rejetée par deny_unknown_fields.`
- **Classes de tests** :
  - `@happy` — schéma et type équivalents pour tous les outils enregistrés
  - `@negative` — champ en trop, champ manquant, type divergent : trois échecs distincts et nommés
  - `@edge` — outil sans argument, schéma imbriqué, champ optionnel
  - `@security` — `deny_unknown_fields` effectif sur **tous** les types d'entrée, prouvé par énumération du registre, pas par échantillon
- **Couche** : Application / CI
- **Taille** : L (1 j) · **Tours** : 4
- **Note** : jamais reléguée en GO-forward. C'est la condition de l'archétype API-first.

### Story 0.4 — Erreurs typées et rédaction des secrets
- **En tant que** superviseur **je veux** un type d'erreur unique et un type qui rend les secrets inaffichables **afin de** que la fuite devienne une impossibilité de compilation plutôt qu'une vigilance.
- **Critères Gherkin** :
  - `Étant donné un identifiant OVH chargé dans Redacted, Quand je le formate ou le sérialise, Alors j'obtiens «redacted» et jamais la valeur.`
- **Classes de tests** :
  - `@happy` — `AppError` couvre les familles d'erreur, `Redacted` masque
  - `@negative` — chaque variante d'erreur porte un message utilisateur correct, aucune panique
  - `@edge` — `Redacted` vide, très long, contenant des caractères de contrôle
  - `@security` — **aucun chemin** (`Debug`, `Display`, `Serialize`, journal) ne révèle la valeur ; prouvé sur les trois traits
- **Couche** : Domain
- **Taille** : M (0,75 j) · **Tours** : 2

### Story 0.5 — Journal d'audit append-only *(FR-013)*
- **En tant que** platform engineer **je veux** une trace inaltérable de chaque appel **afin de** pouvoir auditer sans avoir suivi en temps réel.
- **Critères Gherkin** :
  - `Étant donné un appel d'outil qui échoue, Quand il se termine, Alors une entrée est tout de même ajoutée au journal.`
  - `Étant donné un journal non inscriptible, Quand un outil est appelé, Alors l'appel échoue plutôt que de s'exécuter sans trace.`
- **Classes de tests** :
  - `@happy` — succès et échec journalisés à l'identique
  - `@negative` — journal non inscriptible : refus d'exécution, erreur typée
  - `@edge` — écritures concurrentes depuis plusieurs tâches, entrée volumineuse, rotation
  - `@security` — aucun secret dans le journal ; **aucune méthode de modification ou de suppression n'est exposée par le port**
- **Couche** : Domain + Infrastructure
- **Taille** : M (0,75 j) · **Tours** : 3

---

## Sprint 1 — Core Domain (TDD)

### Epic 1 — BC1 Inventaire · Priorité **Must**

#### Story 1.1 — Types du domaine d'inventaire
- **En tant que** agent **je veux** topologies, environnements et profils typés **afin de** ne plus raisonner sur des chaînes de caractères.
- **Critères Gherkin** :
  - `Étant donné l'environnement integration, Quand je demande sa promotion vers production, Alors elle est refusée car staging est sauté.`
- **Classes de tests** :
  - `@happy` — parsing et ordre de promotion nominal
  - `@negative` — topologie inconnue, environnement inconnu : `AppError` typée
  - `@edge` — promotion depuis `production` (aucune cible), casse et espaces dans les noms
  - `@security` — un nom d'environnement forgé ne crée jamais de variante ; l'énumération est fermée
- **Couche** : Domain · **Taille** : M · **Tours** : 2 · **FR** : FR-002

#### Story 1.2 — Découverte de la matrice depuis le système de fichiers *(FR-002)*
- **En tant que** agent **je veux** obtenir la matrice d'un dépôt **afin de** connaître l'existant sans qu'on me le décrive.
- **Critères Gherkin** :
  - `Étant donné le dépôt koprogo, Quand j'appelle la découverte sur son dossier infrastructure, Alors j'obtiens 3 topologies, 4 environnements et 3 profils.`
  - `Étant donné un chemin hors de la racine autorisée, Quand je lance la découverte, Alors elle est refusée avant toute lecture.`
- **Classes de tests** :
  - `@happy` — arborescence KoproGo réelle, matrice complète
  - `@negative` — chemin inexistant, chemin vers un fichier, dossier illisible
  - `@edge` — topologie sans environnement, arborescence partielle, environnement inconnu ignoré avec avertissement
  - `@security` — remontée `../` refusée, lien symbolique sortant de la racine non suivi
- **Couche** : Domain + Infrastructure · **Taille** : L · **Tours** : 4 · **FR** : FR-002

#### Story 1.3 — Lecture des profils de cluster *(FR-003)*
- **Classes de tests** :
  - `@happy` — les 3 profils avec leur contrat Day 1 / Day 2
  - `@negative` — YAML malformé, clé de contrat manquante : erreur nommant fichier et clé
  - `@edge` — champ inconnu, profil vide, doublon de nom
  - `@security` — une valeur ressemblant à un secret n'est jamais rendue en clair
- **Couche** : Domain + Infrastructure · **Taille** : M · **Tours** : 3 · **FR** : FR-003

#### Story 1.4 — Diagnostic d'environnement *(FR-001)*
- **Classes de tests** :
  - `@happy` — rapport complet, binaires présents
  - `@negative` — binaire au `PATH` mais non exécutable : rapporté inutilisable, pas présent
  - `@edge` — `PATH` vide, six binaires absents, aucune panique
  - `@security` — aucun fragment d'identifiant, ni longueur ni préfixe, en sortie
- **Couche** : Application + Infrastructure · **Taille** : S · **Tours** : 2 · **FR** : FR-001

### Epic 2 — BC2 Autorisation · Priorité **Must**

#### Story 2.1 — Tier et plan de changement, invariants inconstructibles
- **En tant que** superviseur **je veux** qu'un plan visant la production ne puisse pas être de Tier 2 **afin de** que la règle ne dépende d'aucune vigilance.
- **Critères Gherkin** :
  - `Étant donné une action visant production, Quand je construis un plan en Tier 2, Alors la construction échoue avec TierViolation.`
- **Classes de tests** :
  - `@happy` — plan Tier 2 sur `dev`, plan Tier 1 sur `production`
  - `@negative` — Tier 2 sur `production` refusé ; aucun constructeur alternatif n'existe
  - `@edge` — action sans environnement cible, action sur plusieurs environnements
  - `@security` — l'empreinte du plan change si un seul champ change ; deux plans distincts n'ont jamais la même
- **Couche** : Domain · **Taille** : L · **Tours** : 3 · **FR** : FR-020

#### Story 2.2 — Jeton de changement à usage unique par le typage
- **En tant que** superviseur **je veux** qu'un jeton consommé ne puisse pas être rejoué **afin de** que le rejeu soit une erreur de compilation, pas un test.
- **Critères Gherkin** :
  - `Étant donné un jeton déjà consommé, Quand je tente de le consommer à nouveau, Alors le code ne compile pas (consume prend self par valeur).`
  - `Étant donné un jeton émis pour une empreinte A, Quand je le présente pour un plan d'empreinte B, Alors il est rejeté.`
- **Classes de tests** :
  - `@happy` — consommation nominale rendant un `ConsumedToken`
  - `@negative` — empreinte non concordante, jeton expiré : deux erreurs distinctes
  - `@edge` — expiration à la seconde près avec `Clock` figé, jeton émis dans le futur
  - `@security` — rejeu impossible par typage **et** vérifié côté persistance pour le cas distribué
- **Couche** : Domain · **Taille** : L · **Tours** : 4 · **FR** : FR-022

---

## Sprint 2 — Use cases + adaptateurs (TDD + BDD)

### Epic 3 — BC5 Exécution, lecture · Priorité **Must**

#### Story 3.1 — Client OVH signé
- **En tant que** agent **je veux** un client OVH authentifié **afin de** lire l'infrastructure réelle.
- **Critères Gherkin** :
  - `Étant donné une horloge figée et des identifiants connus, Quand je signe une requête, Alors la signature est identique à celle produite par l'implémentation Python de référence.`
- **Classes de tests** :
  - `@happy` — signature conforme au vecteur de référence, delta d'horloge appliqué
  - `@negative` — identifiants absents ou invalides, API injoignable : erreurs distinctes
  - `@edge` — horloge décalée de plusieurs minutes, corps vide, corps volumineux
  - `@security` — les identifiants ne transitent que dans l'en-tête de signature ; ils n'apparaissent ni en journal ni en erreur
- **Couche** : Infrastructure · **Taille** : L · **Tours** : 4 · **FR** : FR-005

#### Story 3.2 — Liste d'autorisation des projets OVH *(FR-005)*
- **Critères Gherkin** :
  - `Étant donné une liste d'autorisation de deux projets et un compte en portant cinq, Quand je liste, Alors j'obtiens exactement les deux autorisés.`
  - `Étant donné un identifiant de projet hors liste fourni explicitement, Quand je l'interroge, Alors il est refusé avant tout appel réseau.`
- **Classes de tests** :
  - `@happy` — projets autorisés retournés
  - `@negative` — identifiants invalides : erreur d'authentification, jamais une liste vide
  - `@edge` — liste vide (retour vide légitime), projet autorisé mais inexistant
  - `@security` — **le refus a lieu avant l'appel réseau**, et l'événement est journalisé comme événement de sécurité
- **Couche** : Domain + Application · **Taille** : M · **Tours** : 3 · **FR** : FR-005

#### Story 3.3 — Instances, coûts, DNS *(FR-006, FR-007, FR-008)*
- **Classes de tests** :
  - `@happy` — instances avec état, coût courant avec période, enregistrements DNS par type
  - `@negative` — projet ou zone inexistants, absence de données de facturation rendue explicite
  - `@edge` — pagination sur plusieurs pages, instance en suppression, montant nul, période à cheval sur deux mois
  - `@security` — projet hors liste refusé sur les trois outils ; zone de production non modifiable
- **Couche** : Application + Infrastructure · **Taille** : L · **Tours** : 4 · **FR** : FR-006/007/008

#### Story 3.4 — Runners de processus sans shell
- **En tant que** superviseur **je veux** qu'aucun argument d'appelant n'atteigne un shell **afin de** rendre l'injection structurellement impossible.
- **Critères Gherkin** :
  - `Étant donné un nom de module contenant un point-virgule et une commande, Quand je lance un plan, Alors la demande est refusée à l'admission et rien n'est exécuté.`
  - `Étant donné que terraform est absent, Quand je lance un plan, Alors j'obtiens EngineMissing nommant le binaire.`
- **Classes de tests** :
  - `@happy` — invocation nominale, arguments passés en tableau
  - `@negative` — binaire absent, code de retour non nul, sortie illisible
  - `@edge` — sortie volumineuse, processus lent, terminaison par signal
  - `@security` — **refus à l'admission** et non échappement ; allowlist d'exécutables ; aucune interpolation
- **Couche** : Infrastructure · **Taille** : L · **Tours** : 4 · **FR** : FR-009

#### Story 3.5 — `tf_plan`, `helm_status`, `kustomize_build`, `argocd_app_status`
- **Classes de tests** :
  - `@happy` — plan structuré, statut et historique, rendu, statut de synchronisation
  - `@negative` — binaire absent, cible inconnue, état verrouillé, serveur injoignable
  - `@edge` — plan vide, plan à plusieurs centaines de ressources, release en échec, application en cours de synchronisation
  - `@security` — aucune valeur de Secret Kubernetes rendue ; kubeconfig et jeton ArgoCD jamais en sortie ; **aucune ressource n'est modifiée par un plan**
- **Couche** : Application + Infrastructure · **Taille** : L · **Tours** : 5 · **FR** : FR-009 à FR-012

#### Story 3.6 — Calcul d'écart entre déclaré et réel *(FR-004)*
- **Classes de tests** :
  - `@happy` — écart vide quand tout concorde ; écart typé quand une ressource manque
  - `@negative` — API indisponible : écart **non calculable**, jamais un écart vide trompeur
  - `@edge` — ressource en réel absente du déclaré et inversement, ressource en cours de création
  - `@security` — aucune ressource d'un projet hors liste ne transparaît dans l'écart
- **Couche** : Domain + Application · **Taille** : M · **Tours** : 3 · **FR** : FR-004

### Epic 4 — Transport MCP · Priorité **Must**

#### Story 4.1 — Serveur MCP stdio, JSON-RPC 2.0
- **Critères Gherkin** :
  - `Étant donné un client MCP, Quand il envoie initialize, Alors il reçoit la version de protocole et les capacités.`
  - `Étant donné une méthode inconnue, Quand elle est appelée, Alors le code d'erreur JSON-RPC est -32601.`
- **Classes de tests** :
  - `@happy` — `initialize`, `tools/list`, `tools/call` sur un outil réel
  - `@negative` — JSON malformé (-32700), requête invalide (-32600), méthode inconnue (-32601), paramètres invalides (-32602)
  - `@edge` — requête sans identifiant (notification), charge utile volumineuse, appels concurrents
  - `@security` — **`tools/call` revérifie l'autorisation indépendamment de `tools/list`** ; un outil non listé mais appelé est refusé
- **Couche** : Infrastructure · **Taille** : L · **Tours** : 4

#### Story 4.2 — Registre d'outils et filtre de rédaction en frontière
- **Classes de tests** :
  - `@happy` — tous les outils enregistrés apparaissent avec leur schéma
  - `@negative` — outil enregistré sans schéma : refus au démarrage, pas à l'appel
  - `@edge` — registre vide, deux outils de même nom
  - `@security` — **le filtre de rédaction s'applique à la frontière du transport**, prouvé par un outil de test renvoyant volontairement un secret
- **Couche** : Infrastructure · **Taille** : M · **Tours** : 3

**Jalon de fabrication à la fin de la story 4.2** : la boucle est prouvée de bout en bout sur le socle de lecture. Ce n'est pas une livraison et rien ne s'arrête ici. Critère : PRD §13.1.

---

## Sprint 3 — Bac à sable et capacité

### Epic 5 — BC3 Bac à sable · Priorité **Should**

#### Story 5.1 — Bail borné, invariants inconstructibles *(FR-014)*
- **Critères Gherkin** :
  - `Étant donné une demande de bail sans TTL, Quand je la construis, Alors elle ne compile pas (TTL non optionnel).`
  - `Étant donné un identifiant de projet de production, Quand je le passe à un bail, Alors le code ne compile pas (type distinct).`
- **Classes de tests** :
  - `@happy` — bail nominal avec TTL et plafond
  - `@negative` — plafond dépassé à l'estimation : refus à l'admission
  - `@edge` — TTL minimal, TTL maximal, plafond à l'euro près
  - `@security` — les **sept** conditions d'ADR-007 vérifiées **chacune** par un test dédié, dont la disjonction des listes de projets et le refus hors fenêtre de dérogation
- **Couche** : Domain · **Taille** : L · **Tours** : 4 · **FR** : FR-014

#### Story 5.2 — Chien de garde et destruction garantie *(FR-015)*
- **En tant que** superviseur **je veux** qu'un bail expiré soit détruit même si le demandeur a disparu **afin de** ne jamais découvrir une facture ouverte.
- **Critères Gherkin** :
  - `Étant donné une campagne qui panique en cours de palier, Quand le processus se termine, Alors le bail est tout de même détruit.`
  - `Étant donné un processus principal tué brutalement, Quand le TTL expire, Alors le chien de garde détruit le bail.`
- **Classes de tests** :
  - `@happy` — destruction à l'échéance nominale
  - `@negative` — destruction impossible (API en erreur) : réessai et alerte, jamais un abandon silencieux
  - `@edge` — bail déjà détruit manuellement, TTL expiré pendant une panne du chien de garde
  - `@security` — **le chien de garde survit à l'arrêt du processus demandeur** ; c'est le test le plus important du sprint
- **Couche** : Application + Infrastructure · **Taille** : L · **Tours** : 5 · **FR** : FR-015

#### Story 5.3 — Preuve de convergence *(FR-016)*
- **Classes de tests** :
  - `@happy` — ré-application sans écart, convergence prouvée
  - `@negative` — écart persistant après N tours : échec explicite, pas de boucle infinie
  - `@edge` — écart dû à une valeur générée (horodatage), convergence au deuxième tour
  - `@security` — la preuve ne divulgue aucune valeur d'état sensible
- **Couche** : Domain + Application · **Taille** : M · **Tours** : 3 · **FR** : FR-016

#### Story 5.4 — Fenêtre de dérogation et renouvellement Tier 1 *(FR-025)*
- **En tant que** superviseur **je veux** que l'autorité déléguée expire d'elle-même **afin de** qu'elle soit reconduite par un acte et jamais par l'oubli.
- **Critères Gherkin** :
  - `Étant donné une fenêtre expirée, Quand l'agent demande un bail, Alors il est refusé et informé qu'un renouvellement Tier 1 est requis.`
  - `Étant donné un stockage de dérogation illisible, Quand la validité est évaluée, Alors elle est réputée expirée.`
  - `Étant donné un renouvellement approuvé, Quand la fenêtre s'ouvre, Alors l'événement est journalisé avec approbateur, date et durée.`
- **Classes de tests** :
  - `@happy` — fenêtre ouverte, expiration, renouvellement approuvé, nouvelle fenêtre active
  - `@negative` — renouvellement refusé, passerelle indisponible, durée au-delà du maximum configuré
  - `@edge` — renouvellement avant expiration (remplacement et non cumul), renouvellement à la seconde exacte, deux renouvellements concurrents
  - `@security` — **le renouvellement est de Tier 1 et jamais obtenable en Tier 2** ; aucun scope OAuth ne l'accorde ; une dérogation ne peut pas se renouveler elle-même ; **fail-closed prouvé** sur stockage indisponible et horloge incohérente
- **Couche** : Domain + Application · **Taille** : M · **Tours** : 3 · **FR** : FR-025
- **Note** : c'est la story qui empêche la dérogation de devenir permanente. Sans elle, ADR-007 est un trou permanent plutôt qu'une délégation encadrée.

### Epic 6 — BC4 Capacité · Priorité **Should**

#### Story 6.1 — Campagne en paliers *(FR-017)*
- **Classes de tests** :
  - `@happy` — les 7 paliers exécutés dans l'ordre, du warmup au soak
  - `@negative` — `wrk` absent : refus **à l'admission**, jamais en cours de route
  - `@edge` — palier échouant à mi-parcours, cible saturée dès le warmup, campagne interrompue
  - `@security` — refus si le rate limiting est actif (résultats faussés) ; refus si la cible n'est pas dans un bail
- **Couche** : Application + Infrastructure · **Taille** : L · **Tours** : 5 · **FR** : FR-017

#### Story 6.2 — Mesures avec provenance *(FR-018)*
- **Critères Gherkin** :
  - `Étant donné une valeur non observée mais dérivée, Quand je la consigne, Alors sa provenance est Assumed et jamais Measured.`
- **Classes de tests** :
  - `@happy` — P99, débit, RSS, jeu chaud, pression, coût réel, tous datés et attribués
  - `@negative` — mesure incohérente (P99 inférieur à la médiane) : rejet
  - `@edge` — mesure à zéro, mesure sur un échantillon insuffisant (marquée comme telle)
  - `@security` — aucune donnée de production dans les mesures ; jeux synthétiques uniquement
- **Couche** : Domain · **Taille** : M · **Tours** : 3 · **FR** : FR-018

#### Story 6.3 — Rapport de recalage des priors *(FR-019)*
- **En tant que** superviseur **je veux** un rapport qui propose de remplacer des `[caler]` par du mesuré **afin de** que mes arbitrages de palier cessent de reposer sur l'intuition.
- **Critères Gherkin** :
  - `Étant donné une campagne complète sur K3s, Quand je génère le rapport, Alors il propose une valeur mesurée pour le surcoût de control-plane et cite ses conditions de mesure.`
- **Classes de tests** :
  - `@happy` — rapport citant prior, valeur mesurée, écart et conditions
  - `@negative` — campagne incomplète : rapport refusé plutôt que partiel et trompeur
  - `@edge` — mesure identique au prior, mesure aberrante signalée
  - `@security` — le rapport **distingue explicitement mesuré et supposé** (§9 de l'abaque) ; aucune extrapolation présentée comme mesure
- **Couche** : Domain + Application · **Taille** : L · **Tours** : 4 · **FR** : FR-019

---

## Sprint 4 — Approbation et mise en ligne

### Epic 7 — BC2/BC5 Tier 1 · Priorité **Must**

#### Story 7.1 — Passerelle GitHub à environnement protégé *(FR-021)*
- **Critères Gherkin** :
  - `Étant donné un plan Tier 1, Quand je le soumets, Alors un workflow_dispatch est déclenché et le job reste bloqué en attente de relecteur.`
  - `Étant donné un refus du relecteur, Quand j'interroge le run, Alors le plan est rapporté refusé et rien n'a été muté.`
- **Classes de tests** :
  - `@happy` — soumission, blocage, approbation, exécution, compte rendu
  - `@negative` — dépôt sans environnement protégé : **refus de soumettre**, plutôt qu'exécution non gardée
  - `@edge` — approbation après expiration du jeton, run annulé, relecteur indisponible
  - `@security` — **Sluis ne détient aucun secret d'infrastructure** ; vérifié par revue de configuration et par un test prouvant que la mutation échoue si tentée localement
- **Couche** : Application + Infrastructure · **Taille** : L · **Tours** : 5 · **FR** : FR-021

#### Story 7.2 — Mise en ligne après vérification des gates *(FR-024)*
- **Classes de tests** :
  - `@happy` — gates vertes, plan Tier 1, approbation, déploiement, tests post-déploiement verts
  - `@negative` — une gate du plancher rouge : **mise en ligne refusée avant même de soumettre**
  - `@edge` — tests post-déploiement rouges : rollback automatique, l'état final n'est jamais non vérifié
  - `@security` — secrets, SBOM, scan d'image et fichier de retour de migration vérifiés ; l'absence d'un `*.down.sql` bloque pour l'archétype stateful
- **Couche** : Application · **Taille** : L · **Tours** : 5 · **FR** : FR-024

### Epic 8 — BC6 Accès distant · Priorité **Should**

#### Story 8.1 — Serveur d'autorisation OAuth 2.1 + PKCE *(FR-023)*
- **Classes de tests** :
  - `@happy` — découverte, enregistrement dynamique, autorisation, échange de code, rafraîchissement
  - `@negative` — `code_verifier` erroné, code expiré, `client_id` inconnu
  - `@edge` — deux échanges concurrents du même code, rafraîchissement à l'expiration exacte
  - `@security` — **`plain` refusé**, code à usage unique (rejeu rejeté), **rotation inconditionnelle** du jeton de rafraîchissement, `redirect_uri` validé avant tout rendu ou redirection, jeton de rafraîchissement stocké en hash uniquement
- **Couche** : Domain + Application + Infrastructure · **Taille** : L · **Tours** : 6 · **FR** : FR-023
- **Réutilisation** : les 5 fichiers de `elevia/.claude/skills/mcp-oauth-maison/references/`, déjà éprouvés deux fois.

#### Story 8.2 — Transport Streamable HTTP et scopes
- **Classes de tests** :
  - `@happy` — `POST /mcp` avec `Bearer` valide, outils filtrés par scope
  - `@negative` — jeton absent, expiré, de scope insuffisant : trois refus distincts
  - `@edge` — jeton valide mais scope retiré entre deux appels
  - `@security` — **aucun scope ne permet de muter la production** ; prouvé par énumération des scopes contre le registre d'outils
- **Couche** : Infrastructure · **Taille** : M · **Tours** : 3

#### Story 8.3 — Déploiement GitOps sur le serveur ecosolva
- **Classes de tests** :
  - `@happy` — image poussée sur GHCR, `deploy.sh --run` déploie, service joignable en TLS
  - `@negative` — pull GHCR en échec : pas de déploiement partiel, réessai au tick suivant
  - `@edge` — deux exécutions concurrentes du cron (verrou `flock`), rien à déployer
  - `@security` — le réseau `ecosolva-web` n'est pas possédé par le dépôt ; aucun secret dans l'image ; **idempotence prouvée** (deuxième exécution sans effet)
- **Couche** : IaC · **Taille** : M · **Tours** : 3

---

## Stories transverses

- **Documentation Vivante** : les 5 flux critiques du PRD §9 en scénarios Gherkin, dont la découverte complète sans saisie et la dégradation propre sur machine nue. **Taille** : M · **Tours** : 3
- **i18n** : sans objet, Sluis n'a pas d'interface utilisateur finale.
- **Émergence** : réserve de ~20 % pour l'imprévu validé, soit **~6 stories** non spécifiées.
- **Scaling** : activées si le Gantt fait passer à Nexus. Non prévu à ce stade.
- **ITIL** : activées en pré-production, avec le runbook du chien de garde et la procédure de révocation de jeton. **Taille** : M · **Tours** : 2

---

## Estimation chiffrée du projet

Tally établi story par story à partir des tailles et tours déclarés ci-dessus.

| Sprint / Epic | Stories | Σ jours | Σ tours |
|---|---|---|---|
| Sprint 0 — Fondations | 5 | 4,25 | 14 |
| Epic 1 — BC1 Inventaire | 4 | 3,00 | 11 |
| Epic 2 — BC2 Autorisation | 2 | 2,00 | 7 |
| Epic 3 — BC5 Exécution | 6 | 5,50 | 23 |
| Epic 4 — Transport MCP | 2 | 1,75 | 7 |
| Epic 5 — BC3 Bac à sable | 4 | 3,50 | 15 |
| Epic 6 — BC4 Capacité | 3 | 2,75 | 12 |
| Epic 7 — Tier 1 | 2 | 2,00 | 10 |
| Epic 8 — Accès distant | 3 | 2,50 | 12 |
| Transverses (doc vivante, ITIL) | 2 | 1,50 | 5 |
| **Sous-total spécifié** | **33** | **28,75** | **116** |
| Émergence (~20 %, non spécifiée) | 6 | 5,75 | 23 |
| **Total** | **39** | **34,50** | **139** |

Répartition par couche dominante des 33 stories spécifiées (une story qui traverse plusieurs couches est comptée à sa couche la plus profonde) :

| Couche | Stories | Σ jours |
|---|---|---|
| Domain | 12 | 10,00 |
| Application | 7 | 6,25 |
| Infrastructure | 9 | 8,25 |
| Frontend | 0 | 0 |
| IaC / CI / Monitoring | 5 | 4,25 |

- **Coût superviseur** = 34,50 j ÷ ratio de supervision 3 ≈ **11,5 jours de superviseur**. C'est le poste dominant, conformément au §1 de l'abaque.
- **Coût modèle** = 139 tours. À quelques centaines de milliers de tokens par tour, **de l'ordre de quelques dizaines d'euros**. Négligeable devant le poste superviseur, ce qui confirme l'heuristique de l'abaque : optimiser les tokens ne déplace presque rien.

### Comparaison à la target du Brief §17-18

| | Brief (point 0) | Backlog détaillée | Écart |
|---|---|---|---|
| Stories | 28 à 34 | **39** | +15 % au-dessus de la borne haute |
| Jours superviseur | 9 à 12 | **11,5** | dans la fourchette, proche de la borne haute |
| Target de challenge | ≤ 12 j superviseur, périmètre complet | **11,5 j** | **tenue**, avec 4 % de marge |

**Décision du superviseur du 2026-08-29 : périmètre complet, pas de découpage MVP.**

Elle clôt l'arbitrage qui était ouvert dans la version précédente de ce livrable. Les trois options qui y étaient proposées portaient toutes sur la réduction d'un périmètre MVP ; elles sont sans objet. Deux conséquences directes :

1. **Le Sprint 0 n'est plus discutable.** Il l'était au motif qu'il pesait lourd devant une target MVP de 4 jours. Rapporté au périmètre complet, il représente 13 % de l'effort, ce qui est le prix normal d'un harnais.
2. **La variable d'ajustement devient le ratio de supervision, pas le contenu.** L'abaque rappelle que ce ratio est le plafond du *répondre-de* et non un levier budgétaire : le tenir à 3 est une contrainte, et le franchir demanderait un pair supplémentaire, pas seulement plus d'agents.

**Marge résiduelle : 0,5 jour de superviseur** sur une target de 12, soit 4 %. Elle était de 0,8 jour avant l'ajout de la fenêtre de dérogation (story 5.4), qui coûte 0,75 jour.

C'est très mince pour 39 stories, et il faut le dire franchement : à ce niveau, la target sera dépassée au premier imprévu qui ne rentre pas dans la réserve d'émergence. Deux réponses possibles, aucune ne consistant à rogner le contenu puisque le périmètre n'est plus négociable : relever la target en connaissance de cause, ou accepter que la réserve d'émergence de 20 % soit le vrai amortisseur et suivre sa consommation comme l'indicateur principal.

Le premier signal de dérive à surveiller reste le nombre de tours réellement consommés par story dans le Sprint 0, qui recalera l'estimation de tous les suivants.

---

*Dérivé du Manifeste Maury (CC BY-SA 4.0). Source : [`github.com/gilmry/manifest`](https://github.com/gilmry/manifest).*
