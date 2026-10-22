# ADR-037 : Le problème de l'amorce

- **Statut** : Proposé le 2026-10-22, non contresigné.
- **Irréversible** : en partie. L'ordre et les invariants se révisent comme tout ADR. Mais l'amorce produit les ancres d'ADR-015 et d'ADR-017 (clé GPG maîtresse, racine de step-ca, parts de secours d'OpenBao), dont le remplacement est déjà qualifié d'irréversible par ces ADR ; et un secret exposé pendant l'amorce est compromis pour toujours, comme un secret poussé en clair (ADR-016).
- **S'articule avec** : ADR-014 (la zone de confiance qu'il faut mettre debout, la zone 3 où vivent les ancres), ADR-015 (YubiKeys, racine et intermédiaire de step-ca), ADR-016 (SOPS et age, destination de la plupart des secrets d'amorce), ADR-017 (OpenBao, déverrouillage par le KMS d'OVHcloud, parts de secours), ADR-010 (OpenTofu loue les machines), ADR-013 (clé SSH de signature des approbations), ADR-023 (supervision et témoin externe), ADR-024 (l'agent IA n'a aucun droit), ADR-032 (Ansible, certificats SSH courts), ADR-033 (étapes 1 et 2). Renvoie à ADR-035 (reprise après perte totale) et ADR-036 (exercices de reprise).
- **Sert la vision (ADR-000)** : principe n°1 à rebours, parce qu'un geste rare et risqué reste humain, et l'amorce est le plus rare et le plus risqué de tous ; principe n°3, parce que la fin de l'amorce se prouve par des refus constatés, pas par une liste cochée ; règle « aucune brique ne détient tout », parce qu'aucune machine ni aucun agent ne détient les secrets d'amorce, et qu'aucun ne survit à la mise en place de la zone de confiance.

## Contexte

Les ADR 015 à 017 décrivent où vivent les secrets une fois la plateforme
debout : les identités humaines sur YubiKey, les identités machines chez
step-ca, les secrets de déploiement dans Git chiffrés par SOPS, la clé
maîtresse dans OpenBao. Aucun ne dit comment on arrive à cet état. Or chacune
de ces briques a besoin, pour naître, d'un secret qu'elle ne peut pas encore
protéger : il faut un jeton de l'hébergeur pour louer la machine de confiance,
un premier accès SSH avant que step-ca émette des certificats, un identifiant
du KMS avant qu'OpenBao se déverrouille seul.

Laissée implicite, l'amorce se fait au fil de l'eau : un jeton large créé « pour
tester », une clé SSH posée « en attendant ». Ce sont les secrets oubliés
qu'ADR-015 veut faire disparaître, installés à la fondation. Et une session
d'agent IA qui voit un secret d'amorce en fait une donnée qui sort du poste.

## Décision

**1. Définition.** Est un secret d'amorce tout secret nécessaire avant que la
zone de confiance soit debout, c'est-à-dire avant la fin de l'étape 2
d'ADR-033 (step-ca, OpenBao, supervision, témoin externe). Les secrets créés
ensuite suivent le régime ordinaire d'ADR-016 ou d'ADR-017.

**2. Trois classes, trois sorts.**

- **Ancres** : générées hors ligne ou sur support matériel, conservées en zone 3,
  jamais dans OpenBao ni en clair sur un disque connecté. Elles ne tournent
  qu'à leur échéance propre ou sur compromission.
- **Secrets de démarrage** : ceux dont une machine a besoin pour démarrer sans
  geste humain, et qui ne peuvent donc pas vivre dans la brique qu'ils font
  démarrer. Ils vivent sur leur machine, et leur source dans SOPS.
- **Secrets temporaires** : larges, créés pour l'amorce seule. Ils sont révoqués
  ou détruits à la clôture, et leur refus est constaté.

**3. Recensement.**

| Secret | Classe | Créé par | Pendant l'amorce | Après | Fin du secret d'amorce |
|---|---|---|---|---|---|
| Comptes humains chez les fournisseurs (hébergeur, second hébergeur du témoin, forge) et leurs codes de secours | ancre | superviseur | double authentification par YubiKey ; codes de secours en zone 3 | inchangé, jamais dans un coffre | tournés si un code de secours a servi |
| Clé GPG maîtresse, sous-clés, identité age du superviseur (ADR-015, ADR-016) | ancre | superviseur, cérémonie hors ligne | maîtresse hors ligne, sous-clés sur les deux YubiKeys | inchangé | à l'échéance des sous-clés |
| Clé SSH de signature des approbations, `ed25519-sk` (ADR-013, ADR-015) | ancre | superviseur, sur YubiKey | résidente sur les deux YubiKeys | inchangé | sur perte ou compromission |
| Racine de step-ca (ADR-015) | ancre | superviseur, cérémonie hors ligne | hors ligne, zone 3 | inchangé | à son échéance |
| Jeton d'API de l'hébergeur pour la location (ADR-010) | temporaire | superviseur | poste du superviseur, chiffré par SOPS | jetons bornés par usage, dans SOPS (point ouvert sur leur détenteur) | révoqué à la clôture |
| État d'OpenTofu des machines amorcées | temporaire | OpenTofu, lancé par le superviseur | local, sur le poste, chiffré | état distant (lieu en point ouvert) | copie locale détruite après migration |
| Mot de passe initial que l'hébergeur fournit à la livraison d'un VPS | temporaire | hébergeur | jamais utilisé | connexion par mot de passe désactivée | désactivé par le premier passage Ansible |
| Premier accès SSH aux machines | temporaire | superviseur | clé publique de la sous-clé d'authentification GPG, sur YubiKey, injectée à la location | certificats SSH courts de step-ca (ADR-032) | retrait de la clé injectée : voir point ouvert |
| Identité age de chaque machine (machine de confiance, exécutant, témoin) | démarrage | la machine elle-même | naît sur la machine, la clé publique seule en sort | inchangé, déclarée dans `.sops.yaml` | sur reconstruction de la machine |
| Clé et phrase de passe de l'intermédiaire de step-ca | démarrage | superviseur, clé générée sur la machine de confiance et signée hors ligne par la racine | machine de confiance | phrase de passe dans SOPS, jamais dans OpenBao (OpenBao dépend de step-ca) | à l'échéance de l'intermédiaire |
| Jeton racine initial d'OpenBao | temporaire | OpenBao, à l'initialisation | utilisé une fois pour poser l'administration par YubiKey | aucun | révoqué à la clôture |
| Clés de récupération ou parts de secours d'OpenBao (ADR-017) | ancre | OpenBao, à l'initialisation | remises au superviseur | zone 3, hors ligne | selon le mécanisme encore ouvert dans ADR-017 |
| Identifiant d'accès au KMS d'OVHcloud (ADR-017, point 7) | démarrage | superviseur | machine de confiance | machine de confiance, source dans SOPS ; jamais dans OpenBao, qu'il déverrouille | tourné à la clôture s'il a existé en clair hors de sa machine |
| Jeton de la forge pour le premier déploiement | temporaire | superviseur | poste, durée de vie courte | accès en lecture seule par dépôt, dans SOPS (ADR-013) | révoqué à la clôture |
| Jetons de notification de la supervision et du témoin externe (ADR-023) | démarrage | superviseur | machine qui les utilise, source dans SOPS | inchangé ; celui du témoin ne dépend jamais de la machine de confiance | tournés à la clôture s'ils ont servi aux essais |

**4. Ordre.** L'amorce suit un ordre fixe ; chaque phase laisse un état stable
et peut se clore seule.

1. **Cérémonie hors ligne** (étape 1 d'ADR-033) : ancres du superviseur et
   racine de step-ca. Rien n'en sort sur un disque connecté.
2. **Location** : machine de confiance et VPS de l'exécutant, accès SSH par la
   YubiKey ; l'empreinte de la clé d'hôte se vérifie par la console de
   l'hébergeur, jamais par confiance au premier contact.
3. **Socle** : premier passage Ansible, identités age des machines.
4. **step-ca** : intermédiaire signé hors ligne ; Ansible passe aux certificats
   courts.
5. **OpenBao** : initialisation, parts de secours en zone 3, identifiant du KMS,
   administration par YubiKey.
6. **Supervision et témoin externe**, alerte d'absence éprouvée.
7. **Clôture** : révocations, migration de l'état, rotations, refus constatés,
   journal signé.

Les phases 2 à 7 forment l'étape 2 d'ADR-033. L'amorce n'ajoute ni étape ni
prérequis à la feuille de route : elle décrit comment les étapes 1 et 2 se
déroulent du point de vue des secrets.

**5. Invariants.**

1. **Unique et manuelle.** L'amorce est faite par le superviseur, en personne.
   Des scripts peuvent l'outiller, ils tournent sur son poste et ne conservent
   rien.
2. **Un agent IA ne voit jamais un secret d'amorce** : ni dans une session, ni
   dans une sortie de terminal qu'il lit, ni dans un fichier qu'il ouvre, ni
   dans un dépôt. Il peut rédiger la procédure et relire le journal, qui ne
   contiennent aucun secret ; il ne participe à aucun geste qui en manipule un
   (ADR-024).
3. **Une clé machine naît sur sa machine ; une ancre naît hors ligne.** Aucun
   secret ne transite en clair par un courriel, une messagerie, un presse-papiers
   partagé ou un dépôt.
4. **Est tourné à la clôture tout secret qui a existé en clair ailleurs que sur
   son lieu final.**
5. **Journal d'amorce** : chaque secret y figure par son nom et sa classe,
   jamais par sa valeur, avec sa date de création, son lieu et la date de sa
   révocation ou de sa rotation. Le superviseur le signe et l'archive en zone 3.
6. **La clôture se prouve par des refus.** Chaque jeton temporaire est essayé
   après révocation et doit être refusé ; une connexion par mot de passe doit
   être refusée ; une connexion SSH sans certificat valide doit être refusée.
7. **La procédure détaillée vit dans un dépôt privé sans aucun secret**, soumis
   à la détection de secrets du plancher. Ce dépôt public n'en porte que l'ordre
   et les invariants.

**6. Amorce initiale et amorce de reconstruction.** L'amorce initiale crée les
ancres. L'amorce de reconstruction, après perte totale des zones 1 et 2
(ADR-035), suit le même ordre et les mêmes invariants, à une différence près :
**elle restaure les ancres, elle ne les recrée pas.** Une racine step-ca
nouvelle obligerait à ré-enrôler toutes les identités ; un OpenBao initialisé à
neuf porterait une nouvelle clé maîtresse, et toutes les données chiffrées
deviendraient illisibles (ADR-017, ADR-019). La reconstruction part donc de la
zone 3 : YubiKey de secours, racine hors ligne, parts de secours et sauvegarde
du stockage d'OpenBao. Les secrets de démarrage et les secrets temporaires sont
toujours recréés. Si la perte est une compromission, les ancres touchées sont
remplacées par une cérémonie complète, et ce remplacement relève des points
irréversibles d'ADR-015 et d'ADR-017. La répétition de l'amorce de
reconstruction relève d'ADR-036.

## Alternatives écartées

- **Amorce au fil de l'eau, sans procédure.** C'est la voie par laquelle les clés
  statiques et les jetons sans échéance sont arrivés sur l'infrastructure
  actuelle (ADR-015, Contexte).
- **Amorce automatisée de bout en bout.** Un automate capable de créer les ancres
  et le premier accès détiendrait tout, au moins un instant, contre la règle
  « aucune brique ne détient tout » ; et l'amorce est trop rare pour qu'une
  automatisation reste juste sans être exercée (ADR-000, principe n°1).
- **Garder les secrets d'amorce comme accès de secours permanents.** Un jeton
  large conservé « au cas où » est la clé statique qu'on oublie. Le secours
  passe par les ancres en zone 3, pas par les secrets temporaires.
- **Tout ranger dans OpenBao une fois debout.** ADR-017 borne OpenBao à la clé
  maîtresse et ADR-016 met les secrets de déploiement dans SOPS ; et les secrets
  de démarrage ne peuvent pas vivre dans la brique qu'ils font démarrer.
- **Publier la procédure détaillée.** Utile à un attaquant, sans gain pour un
  lecteur que l'ordre et les invariants suffisent à éclairer.

## Conséquences

- **Coût** : une demi-journée à une journée de gestes du superviseur pour
  l'étape 2, hors cérémonie hors ligne, à confirmer à la première amorce ; un
  dépôt privé de plus à tenir.
- **Le poste du superviseur devient critique pendant l'amorce** : il porte le
  jeton large et l'état local, que seule une clôture rapide borne.
- **L'agent ne peut pas aider pendant les gestes sur les secrets.** C'est voulu,
  et cela allonge l'étape 2.
- **Limite, porteur seul** : aucun second regard sur la cérémonie. Le journal
  signé et les refus constatés sont la mesure compensatoire.
- **Limite** : l'hébergeur voit naître les machines et garde l'accès à leur
  mémoire (ADR-014, ADR-029). L'amorce ne protège pas contre lui.

## Mesures CyFun 2025 couvertes

Renvoi à `foyer/skills/conformite.md` §2bis et §2ter :

- **PR.AA-01.1 ★** : aucun secret d'amorce dans un dépôt, procédure comprise.
- **PR.AA-03.2 ★** : authentification matérielle des comptes fournisseurs et
  des accès d'administration dès la première connexion.
- **PR.AA-05.3 ★** : le jeton large de l'amorce est remplacé par des jetons
  bornés par usage.
- **PR.AA-01.5 E** : désactivation des moyens d'authentification à risque
  (secrets temporaires révoqués, mot de passe initial désactivé).
- Cycle de vie des clés : pas de mesure dédiée dans CyFun 2025 ; couvert
  indirectement par **GV.PO-01.2 I**, comme dans ADR-015 et ADR-017. La
  destruction sûre des secrets temporaires correspond, en partie, à
  **ID.AM-08.5 E**.
- Séparation des tâches, par mesure compensatoire du porteur seul :
  **PR.AA-05.6 I**.
- Amorce de reconstruction éprouvée : **RC.RP-05.1**, par ADR-036.

## Points ouverts

- **Clé SSH de la YubiKey après l'amorce.** ADR-015 garde la sous-clé
  d'authentification GPG comme accès SSH d'urgence ; ADR-032 exclut toute clé
  SSH statique d'accès aux machines. La clé injectée à la location reste-t-elle
  dans les clés autorisées comme accès d'urgence, ou en est-elle retirée ?
- **État distant d'OpenTofu** : aucun ADR ne fixe où il vit ni par quelle clé il
  est chiffré, alors qu'il contient des valeurs sensibles. Lieu, chiffrement et
  détenteur de la clé à décider avant la phase 2.
- **Détenteur des jetons de location après l'amorce** : ADR-010 donne à
  l'exécutant les clés du projet bac à sable seulement. Qui loue et détruit les
  machines de production, et avec quel jeton ?
- **Phrase de passe de l'intermédiaire de step-ca dans SOPS** : ce recensement
  la place dans SOPS faute d'autre lieu compatible avec un démarrage sans geste
  humain. ADR-016 vise les secrets de déploiement ; l'étendre aux secrets de
  démarrage de la zone de confiance est à confirmer.
- **Racine de step-ca à la cérémonie de l'étape 1** : la générer avec la clé GPG
  maîtresse économise une cérémonie, mais avance un élément de l'étape 2.
  Rejoint le point ouvert « Cérémonie de génération » d'ADR-015.
- **Lieu de l'identifiant du KMS sur la machine de confiance** et alerte sur son
  usage hors redémarrage connu : déjà ouverts dans ADR-017, ils conditionnent la
  phase 5.
- **Critère de fin de l'étape 2** : faut-il ajouter à ADR-033 la clôture de
  l'amorce (refus constatés, journal signé) comme critère de fin ? Ce serait un
  amendement d'ADR-033.
- **Moment unique et créneaux courts** : une phase entamée doit-elle se finir
  dans le même créneau ? Le découpage en phases stables le permet en principe ;
  la durée réelle de chaque phase est à mesurer.
- **Archive du journal d'amorce** : celle des preuves (ADR-022) ou une autre ?
- **Comptes chez le second hébergeur** (témoin, sauvegardes hors région) :
  ADR-014 n'a pas encore choisi l'hébergeur de la zone 3, dont le compte est une
  ancre de plus.
