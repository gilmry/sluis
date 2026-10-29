# ADR-021 : Barrière de déploiement, des contrôles déterministes qui bloquent

- **Statut** : Proposé le 2026-10-21, contresigné le 2026-10-28.
- **Amendé le 2026-10-21** (revue des ADR, décision 7) : identifiants CyFun 2025 complétés. Amendement proposé, contresigné le 2026-10-28.
- **Amendé le 2026-10-21** (revue des ADR, décision 5) : la barrière est rejouée juste avant l'exécution, après l'approbation humaine ; un contrôle qui a changé arrête le déploiement et annule l'approbation. Amendement proposé, contresigné le 2026-10-28.
- **Amendé le 2026-10-29** (sluis#50, Q1, décision du superviseur) : **exception datée pour une barrière vide**, admise avec mention dans le plan soumis, bornée à l'étape 6. Amendement proposé, contresigné le 2026-10-29.
- **Amendé le 2026-10-29** (sluis#50, Q2 et Q3, décisions du superviseur) : la barrière se **rejoue dans le job de la passerelle approuvée**, avant qu'il n'écrive l'état voulu ; les images sont signées par **cosign avec clé**, jamais sans clé. Amendement proposé, non contresigné.
- **Irréversible** : non. Une gate s'ajoute ou se retire par configuration ; mais retirer une gate de la barrière relâche un garde-fou et se trace en ADR.
- **S'articule avec** : FR-024 et les gates du plancher déjà câblées (lecture des check-runs GitHub, sluis#16, story 4.2 de #4) ; Epic 9 stories 9.2 (gates de conformité) et 9.5 (dérive) ; ADR-022 (chaque contrôle produit sa preuve) ; ADR-023 (supervision vivante) ; ADR-031 (la barrière précède toute demande d'approbation) ; `foyer/skills/gates.md`.
- **Sert la vision (ADR-000)** : principe n°3, chaque contrôle croise une seconde source au lieu de croire un voyant ; principe n°5, chaque contrôle produit un verdict machine ; principe n°2, une barrière qui ne peut pas s'évaluer bloque au lieu de laisser passer.

## Contexte

Sluis vérifie déjà quatre gates du plancher avant toute mise en ligne (FR-024) :
il lit les check-runs de la référence, et une gate absente compte rouge. Ces
gates portent sur le **logiciel** : tests, secrets, SBOM, scan d'image. Elles ne
disent rien de l'**état de la plateforme** au moment de déployer : la production
a-t-elle dérivé de Git, la supervision tourne-t-elle, la dernière sauvegarde
est-elle récente ?

Déployer sur une plateforme dont la supervision est morte, ou dont la dernière
sauvegarde date de trois jours, c'est prendre un risque sans le voir. Les pannes
passées montrent que ces situations existent sans que personne ne le sache.

## Décision

1. **La barrière est un ensemble de contrôles déterministes oui/non**, évalués
   avant toute demande d'approbation. **Un seul échec bloque.** Pas de score,
   pas de seuil, pas de « presque vert ».
2. **Chaque contrôle est une gate Foyer** : un exécutable qui rend un **code
   de sortie** (0 passe, autre chose bloque) et écrit un rapport. Le verdict
   est le code de sortie, jamais une analyse de la sortie texte.
3. **Contrôles de la première version**, chacun croisant deux sources :
   - **dérive nulle** : l'état réel de la cible ne diffère pas de Git (contrôle
     de dérive d'ADR-032 pour le socle, réconciliateur pour le cluster) ;
   - **détection cohérente** : l'écart entre ce que détecte le détecteur
     d'intrusion et ce que bannit le bouncer est sous un seuil, sans quoi la
     protection périmétrique est aveugle ;
   - **supervision vivante** : la supervision a collecté récemment **et** le
     témoin externe la voit (ADR-023) ; l'un sans l'autre ne suffit pas ;
   - **sauvegarde récente** : la dernière sauvegarde a moins de N heures **et**
     la dernière restauration nocturne a réussi (ADR-020) ;
   - **images signées** : chaque image à déployer porte une signature cosign
     vérifiée contre la clé attendue (ADR-015). **Signature avec clé, jamais
     sans clé** (décision du 2026-10-29, sluis#50 Q3) : la signature sans clé
     de Sigstore inscrit l'identité du workflow, donc le nom des dépôts
     privés, dans le journal public Rekor ; une clé ne publie rien et ne
     dépend d'aucun service tiers. Une seule voie de vérification pour tous
     les dépôts, publics comme privés.
4. **Indétermination = échec** (fail-closed, comme ADR-007) : un contrôle qui
   ne peut pas s'exécuter, faute de réseau ou de source, bloque.
5. **La barrière est exécutée par Sluis**, côté exécutant pour ce qui exige un
   accès à la cible, et lue en Tier 2 : elle ne mute rien, elle conditionne la
   demande d'approbation (ADR-030, ADR-031).
6. **La barrière est rejouée après l'approbation** (amendement du
   2026-10-21). Juste avant l'exécution, après l'approbation humaine, elle
   relance les mêmes contrôles : **dérive, supervision vivante, sauvegarde
   récente, images signées**.
   - **Si un contrôle a changé depuis l'approbation, le déploiement s'arrête
     et l'approbation est annulée.** Elle ne vaut que pour l'état qu'elle a
     vu.
   - **Le superviseur reçoit une alerte** (ADR-023) qui indique quel contrôle
     a changé et pourquoi. Il doit réapprouver en connaissance de cause.
   - **Aucune nouvelle tentative automatique.** Une barrière rouge après
     approbation ne se rejoue pas seule jusqu'à ce qu'elle repasse au vert.
   - **Où elle se rejoue** (décision du 2026-10-29, sluis#50 Q2) : dans le
     job de la passerelle approuvée (ADR-008), qui demande à l'exécutant de
     rejouer la barrière **avant** d'écrire l'état voulu que Flux réconcilie
     (ADR-029). Ni le tireur GitOps actuel, qui disparaît à l'étape 3, ni un
     contrôleur d'admission dans le cluster. La fenêtre résiduelle, entre
     l'écriture et la réconciliation, relève du contrôle de dérive (ADR-025).
   - **Comment l'approbation est annulée** : par la passerelle elle-même. Un
     run GitHub dont la barrière rejouée est rouge échoue sans rien écrire, et
     son approbation ne vaut que pour lui ; une étiquette signée (ADR-013)
     voit son empreinte consommée, elle ne redéploie plus.

7. **Exception datée pour une barrière vide** (amendement du 2026-10-29).
   Tant que la plateforme des étapes 2 à 6 n'existe pas, aucun contrôle n'a
   rien à vérifier. Une barrière **sans contrôle** refuse par défaut ; elle
   n'est admise que si la configuration déclare
   `[barriere] vide_admise_jusqu_au = "AAAA-MM-JJ"`, et seulement jusqu'à
   cette date incluse (UTC). Chaque plan soumis porte alors **« BARRIÈRE
   VIDE »** dans sa description et le dit dans son détail : le superviseur
   approuve en sachant qu'aucun contrôle de plateforme n'a tourné. La date est
   la date de réexamen ; elle ne va pas au-delà de l'étape 6. L'exception ne
   rattrape jamais un contrôle déclaré et rouge.

## Alternatives écartées

- **Contrôles consultatifs** (un avertissement qu'on peut ignorer) : un
  avertissement ignoré une fois l'est toujours ; le principe n°2 les assimile à
  une automatisation muette.
- **Score global de santé** : agrège et masque. Une sauvegarde absente ne se
  compense pas par une supervision parfaite.
- **Contrôles dans le workflow de déploiement de chaque projet** : chaque projet
  les réécrirait et les ferait dériver ; ils relèvent de la couche plateforme,
  portée une fois par le déployeur (`foyer/skills/conformite.md` §2bis).
- **Analyse de journaux par un modèle** : non déterministe, et exposée au texte
  libre (ADR-024).

## Conséquences

- **Les mises en ligne seront bloquées plus souvent au début.** C'est voulu :
  chaque blocage révèle un défaut de plateforme qui existait déjà sans être vu.
  Risque : la tentation de désactiver la gate gênante. Toute désactivation se
  trace en ADR avec une date de réexamen.
- **Entretien** : cinq gates à maintenir et à tester par témoin (défaut remis,
  la gate doit rougir), sans quoi une gate toujours verte est une décoration.
- **Coût d'une signature d'images** : la CI doit signer (cosign), et la clé ou
  l'identité de signature devient un secret de la chaîne de build.
- **Limite** : la barrière constate un état à un instant. Rejouée après
  l'approbation (point 6), elle réduit la fenêtre au temps qui sépare son
  second passage de l'exécution, sans l'annuler ; le contrôle de dérive
  quotidien (ADR-025) rattrape après coup ce qui s'y glisse.
- **Une approbation peut être perdue** : si l'état change entre l'approbation
  et l'exécution, le superviseur réapprouve. C'est voulu : il approuve un
  état, pas une intention.

## Mesures CyFun 2025 couvertes

Story 9.2 de l'Epic 9 : PR.DS-11.1 ★, PR.PS-04.3, PR.AA-03.2 ★, PR.IR-01.1 ★.
Story 9.5 : PR.PS-01.1 ★, DE.CM-09.1. Les gates vertes avant mise en production
répondent à PR.PS-06.2 I (`foyer/skills/conformite.md` §2ter). La vérification
de signature des images relève de PR.DS-01.1 I (intégrité des logiciels), avec
PR.PS-05.2 I, et PR.DS-01.2 E pour l'alerte automatique.

## Points ouverts

- **Seuils** : âge maximal de la sauvegarde (proposition : 26 h), fraîcheur de
  la supervision (proposition : 10 min), écart détecteur/bannissement toléré.
- **Dérogation datée** : une gate rouge bloque-t-elle toujours, ou existe-t-il
  une dérogation datée sur le modèle de la story 5.4 ? Question déjà ouverte
  dans sluis#10, à trancher par RFC.
- **Contrôle « détection cohérente »** : la décision du 2026-10-21 nomme quatre
  contrôles à rejouer après l'approbation ; le cinquième (écart entre
  détecteur et bouncer) l'est-il aussi ?
- ~~**Annulation selon la passerelle**~~ : tranché le 2026-10-29 (point 6,
  « Comment l'approbation est annulée »).
- **Où vit la clé de signature des images** : ni dans la CI GitHub, ni sur
  une machine où tourne un agent. Soit la signature se fait sur la machine de
  confiance de l'étape 2, avec une clé détenue par OpenBao (cosign sait
  signer par son moteur *transit*, la clé ne sort pas) ; soit sur le poste du
  superviseur, à l'approbation. Recommandation : OpenBao sur la machine de
  confiance.
