# ADR-014 : Trois zones, la production, la confiance, le hors-ligne

- **Statut** : Proposé le 2026-10-21, contresigné le 2026-10-28.
- **Amendé le 2026-10-21** (revue des ADR, décision 7) : identifiants CyFun 2025 complétés. Amendement proposé, contresigné le 2026-10-28.
- **Amendé le 2026-10-21** (revue des ADR, décision 2) : le Sluis exécutant quitte la machine de confiance pour un VPS séparé. Amendement proposé, contresigné le 2026-10-28.
- **Irréversible** : oui. Le découpage fixe où vivent les clés, les journaux et les sauvegardes ; le déplacer après coup oblige à re-créer des identités et à migrer des données.
- **S'articule avec** : ADR-010 (deux déploiements de Sluis : ce qui est exposé ne détient rien, ce qui détient n'est pas exposé ; l'exécutant sur un VPS séparé), ADR-011 (image exécutante exemptée du gate de scan pour `tofu`), ADR-008, ADR-012 et ADR-013 (Sluis ne peut que demander), ADR-007 (bacs à sable éphémères, hors des trois zones). Prolongé par ADR-015 à ADR-017, ADR-023 et ADR-033.
- **Sert la vision (ADR-000)** : principe n°3, parce que la zone de confiance surveille la production depuis l'extérieur et qu'un témoin hors ligne surveille la zone de confiance ; règle « aucune brique ne détient tout », parce que chaque zone tient une part des pouvoirs et aucune ne les tient tous.

## Contexte

Aujourd'hui, une seule machine porte tout : les démos, Sluis, sa supervision,
ses journaux et les clés qui permettent de déployer. La panne de septembre l'a
montré : une supervision qui vit sur la machine qu'elle surveille meurt avec
elle, ou pire, survit à moitié et donne le change. Une compromission de cette
machine donnerait à la fois les données, les clés et les preuves, y compris
celles qui devraient établir ce qui s'est passé.

CyFun Essential demande, pour la couche plateforme (`foyer/skills/conformite.md`,
§2bis) : sauvegardes immuables et testées, journaux hors hôte, séparation
production / test, accès à privilèges maîtrisés. Ces exigences supposent qu'il
existe au moins un endroit que la production ne peut pas réécrire.

## Décision

L'infrastructure de tout SaaS déployé par Sluis se découpe en **trois zones**,
chacune avec un rôle, une exposition et un ensemble de pouvoirs distincts.

| Zone | Contient | Exposée à Internet | Ne contient jamais |
|---|---|---|---|
| **1. Production** | Le cluster ou le VPS des SaaS, leurs bases, leurs fichiers, le réconciliateur (Flux, ADR-029) | Oui, par l'entrée HTTP des applications seulement | La clé maîtresse applicative, l'autorité de certification, les journaux de référence |
| **2. Confiance et supervision** | La machine de confiance : step-ca (ADR-015), OpenBao (ADR-017), la supervision et le stockage de ses journaux (ADR-023) | Non : accès par tunnel privé et double authentification | Les données des utilisateurs ; le Sluis exécutant (ADR-010), dont l'image est exemptée du gate de scan pour `tofu` (ADR-011) |
| **3. Hors ligne et externe** | Les YubiKeys et leur copie de secours (ADR-015), les sauvegardes hors site dans une autre région (ADR-026, ADR-027), le témoin externe chez un autre hébergeur (ADR-023), l'archive des preuves (ADR-022) | Le témoin seul, qui sonde de l'extérieur | Aucun accès en écriture depuis les zones 1 et 2 vers ce qui fait foi |

Trois règles de flux tiennent le découpage :

1. **La production ne parle à la zone de confiance que pour demander** : un
   certificat, une clé de données chiffrée, l'envoi d'une métrique ou d'un
   journal. Elle n'y lit rien qui la rende autonome.
2. **La zone de confiance n'écrit dans la production que par la voie
   approuvée** : le réconciliateur GitOps, après approbation (ADR-031). Elle n'a
   pas de session d'administration permanente sur le cluster.
3. **Ce qui fait foi en zone 3 est en ajout seul** : sauvegardes avec
   verrouillage en écriture, journal des preuves, témoin. Une compromission des
   zones 1 et 2 ensemble ne peut pas réécrire l'histoire.

Le Sluis **public** (serveur MCP, ADR-030) reste à part : il est exposé, il ne
détient rien qui mute sans un humain (ADR-012), et il peut vivre en zone 1 comme
en zone 2 selon la recette.

Le Sluis **exécutant** (ADR-010) tourne sur **un VPS séparé, hors de la
machine de confiance** (amendement du 2026-10-21). Son image est exemptée du
gate de scan pour le binaire `tofu` (ADR-011) : elle porte des vulnérabilités
connues et tolérées, et ne doit donc pas cohabiter avec le gardien de la clé
maîtresse. Une compromission de l'exécutant atteint les clés du projet bac à
sable qu'il détient, pas OpenBao ni l'autorité de certification. Les bacs à sable d'ADR-007 sont **hors zones** : un
projet de test dédié, sans donnée de production.

## Alternatives écartées

- **Une seule machine, durcie.** C'est l'état actuel. Le durcissement réduit la
  probabilité d'une compromission, pas son étendue : le jour où elle arrive, tout
  est perdu ensemble, preuves comprises.
- **Deux zones (production et le reste).** Les sauvegardes, l'archive des
  preuves et le témoin externe partageraient alors le sort de la machine de
  confiance : une compromission de celle-ci atteindrait à la fois les clés, la
  supervision et ce qui doit établir ce qui s'est passé, et il n'y aurait plus
  de lieu indépendant pour croiser les sources. (La supervision, elle, partage
  bien la machine des clés dans la décision retenue ; c'est le témoin externe
  et l'audit envoyé hors machine qui compensent, ADR-023.)
- **Tout dans le cluster, séparé par espaces de noms.** Un espace de noms isole
  des charges, pas des administrateurs : quiconque administre le cluster atteint
  tous ses espaces. La clé maîtresse et l'autorité de certification doivent vivre
  hors de ce périmètre (ADR-017).

## Conséquences

- **Coût** : deux machines de plus (la machine de confiance, modeste, et le VPS
  séparé du Sluis exécutant), un stockage objet dans une autre région et un
  témoin chez un autre hébergeur. L'ordre de
  grandeur est de quelques dizaines d'euros par mois ; ADR-033 le fait arriver
  par étapes.
- **Entretien** : trois machines à tenir au lieu d'une, mais par les mêmes
  playbooks (ADR-032), avec les mêmes correctifs automatiques (ADR-025).
- **Point de défaillance** : la machine de confiance devient critique pour les
  déploiements et les renouvellements de certificats, pas pour le service rendu.
  Si elle tombe, les SaaS continuent de tourner ; on ne déploie plus et les
  certificats courts finissent par expirer. Le témoin externe doit le voir.
- **Limite, sur VPS** : l'hébergeur garde un accès à la mémoire des machines. Le
  découpage protège contre une compromission applicative ou un vol de disque, pas
  contre l'hébergeur lui-même. C'est une raison de plus de ne pas mettre la clé
  maîtresse et les données sur la même machine.
- **Limite, porteur seul** : la même personne administre les trois zones. La
  séparation est technique (clés distinctes, accès distincts, journaux en ajout
  seul), pas humaine ; c'est une mesure compensatoire au sens de
  `foyer/skills/conformite.md`, §2bis.

## Mesures CyFun 2025 couvertes

Le mapping vit dans `foyer/skills/conformite.md` (§2bis, couche plateforme) et
dans l'Epic 9 de Sluis. Ce découpage en est le support :

- **PR.IR-01.1 ★, PR.IR-01.2 ★, PR.IR-01.7** (Epic 9, stories 9.2 et 9.6) :
  séparation des réseaux et des environnements.
- **PR.DS-11.1 ★, PR.DS-11.2, PR.DS-11.4** (Epic 9, story 9.4) : sauvegardes hors
  de la production, protégées et éprouvées.
- **PR.AA-05.3 ★, PR.AA-05.6** (conformite.md) : moindre privilège et séparation,
  ici portés par l'infrastructure.
- Journaux hors hôte : **PR.PS-04.3 I** (données d'audit transférées vers un
  système distinct), avec **PR.PS-04.1 ★B** et **DE.AE-03.1 ★B**.

## Points ouverts

- **Où vit le Sluis public** sur la recette VPS : sur la machine de production,
  comme aujourd'hui, ou sur la machine de confiance derrière son propre accès ?
- **Hébergeur et région de la zone 3** : à choisir hors de l'hébergeur principal
  pour que le témoin et les sauvegardes ne partagent pas sa panne.
- **Taille minimale de la machine de confiance** : à mesurer une fois step-ca,
  OpenBao et la supervision installés ensemble (ADR-033, étape 2).
- **Zone et flux du VPS de l'exécutant** : à quelle zone le rattacher, et quels
  flux il ouvre vers la machine de confiance (certificat step-ca, envoi de
  journaux et de métriques) et vers la production.
