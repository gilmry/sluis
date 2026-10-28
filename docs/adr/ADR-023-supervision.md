# ADR-023 : Supervision sur la machine de confiance, surveillée de l'extérieur

- **Statut** : Proposé le 2026-10-21, contresigné le 2026-10-28.
- **Amendé le 2026-10-21** (revue des ADR, décision 7) : identifiants CyFun 2025 complétés. Amendement proposé, contresigné le 2026-10-28.
- **Irréversible** : non. Les composants se remplacent (les adaptateurs sont enfichables) ; les seuils et routes d'alerte se versionnent.
- **S'articule avec** : ADR-014 (zone de la machine de confiance, zone externe du témoin), ADR-017 (audit OpenBao envoyé hors machine), ADR-018 (déchiffrements), ADR-021 (« supervision vivante » est une gate de la barrière), ADR-025 (chaque tâche émet un signal de succès que la supervision attend), ADR-024 (l'agent IA lit les alertes sans droits), `foyer/skills/convergence-iac.md` (une alerte Alertmanager ré-entre dans le cercle et déclenche le platform engineer).
- **Sert la vision (ADR-000)** : c'est la brique qui rend la vision possible, puisque l'humain ne fait plus que lire les alertes. Principe n°2, l'absence de signal alerte ; principe n°3, le témoin externe croise la supervision avec une source indépendante.

## Contexte

Si le travail humain quotidien se limite à lire les alertes, une alerte qui
n'arrive pas équivaut à une panne invisible. C'est arrivé : sur l'infrastructure
actuelle, la collecte de métriques est restée morte **onze jours** pendant que
l'alerteur, toujours debout, donnait l'impression que tout allait bien. Une
supervision qui se surveille elle-même ne voit pas sa propre mort.

## Décision

1. **La supervision vit sur la machine de confiance** (ADR-014), pas dans le
   cluster qu'elle surveille : une panne du cluster ne l'emporte pas.
   Composants : **VictoriaMetrics ou Prometheus** pour les métriques (adaptateur
   enfichable, le choix se fait à l'usage), **Loki** pour les journaux,
   **Alertmanager** pour les alertes.
2. **Un témoin externe chez un autre hébergeur** sonde la supervision et les
   services publics, et alerte par un canal qui ne passe pas par la machine de
   confiance. C'est la seconde source du principe n°3 : si la supervision meurt,
   le témoin le dit.
3. **Alerte d'absence (« dead man's switch »)** : la supervision émet en
   permanence une alerte de vie ; le témoin externe alerte quand elle cesse.
   Chaque tâche automatisée (ADR-025) émet un signal de succès ; son absence
   au-delà de la cadence attendue alerte.
4. **Alerte sur déchiffrements anormaux** : le volume et la provenance des
   appels de déchiffrement à OpenBao (ADR-017, ADR-018) sont des métriques ;
   un pic, un appelant inconnu ou un déchiffrement hors des heures de
   traitement alertent. C'est le signal d'une exfiltration en cours.
5. **Alertmanager déclenche la persona platform engineer** : une alerte est un
   point d'entrée dans la boucle de Foyer (convergence-iac.md), pas une
   notification qu'on lit et qu'on oublie. L'analyse de l'alerte suit ADR-024
   (collecte en lecture, modèle sans outils, action par l'humain).
6. **Journaux hors hôte** : les journaux des machines et du cluster sont
   expédiés vers Loki dès leur production ; un journal resté sur la machine
   compromise ne prouve rien.

## Alternatives écartées

- **Supervision dans le cluster** : meurt avec lui, au moment où on en a le
  plus besoin.
- **Service de supervision SaaS** : sort des données de métriques et de
  journaux vers un tiers, et crée une dépendance fournisseur à auditer
  (GV.SC-07.3 E, non tenable pour un porteur seul). Le témoin externe, lui, ne
  voit que des sondes publiques.
- **Pas de témoin externe** : c'est exactement la configuration qui a laissé
  passer onze jours de panne.
- **Elasticsearch pour les journaux** : plus lourd à opérer que Loki pour le
  même usage (principe n°4).

## Conséquences

- **Coût** : la machine de confiance doit tenir la rétention des métriques et
  des journaux ; le témoin externe coûte un abonnement modeste ou une petite
  machine chez un autre hébergeur.
- **Entretien** : les règles d'alerte se versionnent dans Git et se testent par
  témoin (une alerte qu'on n'a jamais vue se déclencher n'est pas prouvée).
  Après chaque redémarrage, l'état des cibles collectées est vérifié.
- **Bruit** : une alerte trop sensible finit ignorée, ce qui la rend muette.
  Chaque alerte doit appeler une action ; sinon elle se retire.
- **Limite** : la machine de confiance concentre supervision, autorité de
  certification (ADR-015) et OpenBao (ADR-017). Sa compromission aveugle et
  désarme à la fois ; c'est pourquoi le témoin externe et l'audit d'OpenBao
  partent hors de cette machine.

## Mesures CyFun 2025 couvertes

Story 9.5 de l'Epic 9 : DE.CM-09.1 (surveillance des environnements) ; journaux
hors hôte de la couche plateforme (`foyer/skills/conformite.md` §2bis), qui sont
aussi la mesure compensatoire de PR.AA-05.6 I. La détection d'événements
anormaux relève de DE.AE-02.1 I, DE.AE-06.1 I (alerte transmise) et DE.AE-02.2 E
(analyse automatisée), avec DE.AE-03.1 ★B.
Le reporting d'incident déclenché par alerte reste un angle mort nommé par
conformite.md §5.

## Points ouverts

- **VictoriaMetrics ou Prometheus** : à trancher à l'usage, sur la mémoire
  consommée et la rétention tenue par la machine de confiance.
- **Canal d'alerte** : par où le superviseur reçoit-il les alertes (courriel,
  notification poussée) sans que ce canal dépende de la machine de confiance ?
- **Seuils de déchiffrement anormal** : à calibrer sur deux semaines de
  données réelles avant d'en faire une alerte bloquante.
- **Procédure de notification d'incident** (CRA, NIS2 : 24 h) déclenchée par
  alerte : relève de la couche organisation, à rattacher ici.
