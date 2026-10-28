# ADR-010 : Deux déploiements, et OpenTofu comme moteur

- **Statut** : contresigné le 2026-10-18.
- **Sorti de l'index le 2026-10-21** (revue des ADR, décision 7) : texte déplacé depuis `README.md` sans modification de fond.
- **Amendé le 2026-10-28** (sluis#8) : k6 remplace wrk comme moteur de charge de l'image exécutante. Amendement proposé, contresigné le 2026-10-28.

**Amendée le 2026-10-21** (revue des ADR, décision 2) : le Sluis exécutant
tourne sur un VPS séparé, hors de la machine de confiance. Amendement proposé, contresigné le 2026-10-28 ; le reste de l'ADR reste contresigné.

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

**Amendement du 2026-10-21 : où tourne l'exécutant.** Le Sluis exécutant tourne
sur **un VPS séparé**, hors de la machine de confiance d'ADR-014 (step-ca,
OpenBao, supervision). Motif : son image est exemptée du gate de scan pour le
binaire `tofu` (ADR-011) ; une image qui porte des vulnérabilités tolérées ne
cohabite pas avec le gardien de la clé maîtresse. « Ce qui détient n'est pas
exposé » reste vrai : le VPS de l'exécutant n'a pas d'entrée depuis Internet.
**Conséquence** : un VPS de plus à louer, tenir et superviser (ADR-014,
ADR-033 étape 2). Proposé, contresigné le 2026-10-28.

**Amendement du 2026-10-28 : k6 remplace wrk** (sluis#8, décision du
superviseur). Le moteur de charge de l'image exécutante devient **k6**, embarqué
comme `tofu` : binaire de release téléchargé à une version épinglée, **vérifié
par empreinte** dans le Dockerfile, et nommé `k6` dans l'allowlist, le
diagnostic et l'adaptateur. La compilation de `wrk` depuis ses sources
disparaît, et avec elle les bibliothèques copiées sans `dpkg` (ADR-011).

*Licence.* k6 est sous **AGPL-3.0**. L'image d'un dépôt MIT peut le
redistribuer **sans modification**, comme un programme distinct agrégé à côté
des binaires de Sluis : Sluis ne le lie pas, il le lance comme processus. Deux
conditions, vérifiées par le Dockerfile : le texte de la licence d'AGPL est
copié dans l'image à côté du binaire, et un fichier y indique la version et
l'adresse de ses sources. Modifier k6 ou le lier à Sluis ferait tomber cette
lecture ; ni l'un ni l'autre n'est prévu.

*Gains.* Un résumé JSON (`--summary-export`, avec `summaryTrendStats` pour le
p99) remplace l'analyse du texte de `wrk --latency`. *Coût.* Le corpus de charge
de KoproGo, écrit en wrk et Lua, est à réécrire en scénarios k6 : l'hypothèse
H5 du PRD (« corpus réutilisable tel quel ») tombe.

*Calendrier.* Le remplacement dans le code vient après la gate review du jalon
zéro : il dépasse la taille XS. D'ici là, `wrk` reste dans l'image.
Proposé, contresigné le 2026-10-28.
