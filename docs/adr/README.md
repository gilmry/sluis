# Architecture Decision Records

Les ADR de Sluis vivent dans le livrable de l'Architecte,
[`../bmad/03-architecture.md`](../bmad/03-architecture.md), section *ADR*.
Ils n'y sont pas dupliqués ici : une décision recopiée à deux endroits
finit par diverger.

| # | Décision | Irréversible | Statut |
|---|---|---|---|
| ADR-001 | Hexagonale + SOLID | non | acceptée |
| ADR-002 | TDD + BDD + Documentation Vivante | non | acceptée |
| ADR-003 | DDD ubiquitous language → mapping code | non | acceptée |
| ADR-004 | Rust plutôt que le défaut BMAD Python + FastAPI | non | acceptée, écart méthodologique assumé |
| ADR-005 | Le contrat MCP est matérialisé, pas décrit | **oui** | acceptée |
| ADR-006 | Persistance CQRS SQL pur, pas d'ORM | **oui** | contresignée le 2026-08-29 · **amendée à la fabrication** : dépôt fichier, voir ci-dessous |
| ADR-007 | Dérogation Tier 2 bornée **et à durée limitée** pour les bacs à sable | **oui** | **contresignée le 2026-08-29** · fenêtre ouverte jusqu'au **2026-11-27** |
| ADR-008 | Passerelle d'approbation = environnement GitHub protégé | **oui** | acceptée |
| ADR-009 | Mapping branche → environnement | **oui** | acceptée |
| ADR-010 | Deux déploiements, et OpenTofu comme moteur | **oui** | **à contresigner**, voir ci-dessous |
| ADR-011 | Gate de scan différencié par image | non | **à contresigner**, voir ci-dessous |

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
jour où le volume l'exigera. Cet amendement est à contresigner.


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
états écrits par OpenTofu. Cet ADR est à contresigner.


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
configuration. Il est à contresigner, parce qu'il relâche un garde-fou du
plancher.
