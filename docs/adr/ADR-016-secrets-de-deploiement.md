# ADR-016 : Secrets de déploiement chiffrés dans Git avec SOPS et age

- **Statut** : Proposé le 2026-10-21, non contresigné.
- **Amendé le 2026-10-21** (revue des ADR, décision 7) : identifiants CyFun 2025 complétés. Amendement proposé, non contresigné.
- **Amendé le 2026-10-23 par ADR-040** (contresigné le 2026-10-23) : le destinataire humain des secrets SOPS est la sous-clé de chiffrement GPG du superviseur, et non une identité `age-plugin-yubikey`.
- **Irréversible** : non pour l'outil, oui pour les secrets eux-mêmes : un secret poussé une fois en clair dans un dépôt est compromis pour toujours, quoi qu'on réécrive ensuite.
- **S'articule avec** : ADR-008 (les secrets de mutation vivent hors de Sluis), ADR-015 (destinataires age et GPG sur YubiKey), ADR-017 (les clés applicatives, elles, vivent dans OpenBao), ADR-029 et ADR-032 (Flux et Ansible les consomment).
- **Sert la vision (ADR-000)** : principe n°1, parce qu'un secret versionné ne dépend plus de la mémoire de qui l'a posé sur le serveur ; principe n°2, parce qu'un déchiffrement qui échoue fait échouer le déploiement au lieu de démarrer une pile aux secrets vides ; principe n°5, parce que l'historique Git date chaque changement de secret.

## Contexte

Aujourd'hui, les secrets de déploiement vivent dans des fichiers `.env` posés à
la main sur le serveur, hors de tout historique. On ne sait pas quand un secret a
changé ni par qui, une machine neuve ne se reconstruit pas sans eux, et leur
sauvegarde dépend de celle du serveur qu'ils protègent. L'incident du
2026-10-21 l'a rappelé : un déploiement sans `.env` aurait démarré avec des
secrets vides si une garde ne l'avait pas refusé.

Il faut distinguer deux familles de secrets :

- **les secrets de déploiement** : mots de passe de base, jetons de registre,
  identifiants S3, clés d'API de tiers. Ils changent rarement et doivent suivre
  la configuration qu'ils accompagnent ;
- **les clés applicatives** : la clé maîtresse du chiffrement des données des
  utilisateurs. Elles relèvent d'ADR-017 et ne passent jamais par Git.

## Décision

1. **Les secrets de déploiement sont versionnés dans Git, chiffrés par SOPS avec
   age.** Seules les valeurs sont chiffrées ; les clés des fichiers restent
   lisibles, ce qui permet de relire un diff sans rien déchiffrer.
2. **Destinataires par zone et par usage.** Chaque fichier chiffré déclare dans
   `.sops.yaml` qui peut le lire : l'identité du superviseur sur YubiKey (ADR-015)
   et l'identité age de la machine qui déploie. Aucune machine ne lit les secrets
   d'une autre zone.
3. **Deux consommateurs, un seul format.**
   - **Ansible** déchiffre au moment de l'application par la collection
     `community.sops` (ADR-032) ;
   - **Flux** déchiffre dans le cluster par son intégration SOPS native
     (ADR-029, à un nœud comme à trois), avec une clé age détenue par le seul
     contrôleur. (Amendé le 2026-10-28, sluis#6 : Coolify, troisième
     consommateur de la première version, est abandonné.)
4. **Au repos dans K3s, `secrets-encryption` est activé.** Les objets `Secret`
   sont chiffrés dans la base du cluster, pour qu'une copie de cette base ne
   livre pas les secrets en clair.
5. **Gate secrets au poste et en CI.** Le plancher de Foyer (gitleaks, anneau 3 de
   `foyer/skills/enforcement.md`) refuse tout fichier qui porterait une valeur en
   clair là où SOPS est attendu.

## Alternatives écartées

- **Fichiers `.env` posés à la main.** État actuel : pas d'historique, pas de
  reconstruction, pas de preuve.
- **Secrets dans GitHub Actions.** Retenu par ADR-008 pour les secrets de
  mutation que porte le workflow ; insuffisant pour la configuration d'une pile
  qui tourne hors de GitHub, et inutilisable par Ansible ou Flux.
- **Tout dans OpenBao.** Possible, mais chaque déploiement dépendrait alors d'un
  service vivant ; SOPS dans Git reste lisible sur une machine neuve, avec une
  YubiKey, même si la zone de confiance est tombée.
- **Sealed Secrets.** Lié à un cluster précis et à sa clé ; inutilisable pour la
  recette VPS et pour Ansible.
- **GPG seul comme format SOPS.** Accepté pour l'humain, mais age est plus
  simple à donner à une machine.

## Conséquences

- **Coût** : nul en argent ; une discipline de dépôt (tout secret nouveau passe
  par `sops`) et un fichier `.sops.yaml` par dépôt de configuration.
- **Entretien** : la rotation d'un secret est un commit ; le retrait d'un
  destinataire impose de re-chiffrer (`sops updatekeys`) **et** de changer le
  secret, car l'ancien destinataire a pu le lire.
- **Limite** : un secret déchiffré existe en clair dans la mémoire de la machine
  qui l'utilise, et dans le cluster sous forme d'objet `Secret`. SOPS protège le
  stockage et le transport, pas l'exécution.
- **Limite** : la perte de toutes les identités qui peuvent déchiffrer rend les
  secrets irrécupérables ; d'où les deux YubiKeys d'ADR-015.

## Mesures CyFun 2025 couvertes

Renvoi à `foyer/skills/conformite.md` §2ter :

- **PR.AA-01.1 ★** : aucun moyen d'authentification dans le code en clair ; la
  détection de secrets du plancher en produit la preuve.
- Protection des données au repos (secrets) : **PR.DS-01.x** et **GV.PO-01.2**,
  cités par conformite.md pour la couche application et appliqués ici à la
  plateforme.
- Traçabilité des changements de configuration : pas de mesure dédiée dans
  CyFun 2025. Au plus près, **PR.PS-06.2 I** (changement validé avant mise en
  œuvre, prouvé par l'historique des PR) et **PR.PS-01.5 E** (changements non
  autorisés détectés).

## Points ouverts

- **Dépôt de configuration** : un dépôt par SaaS, ou un dépôt de plateforme
  unique et privé ? Le second simplifie Flux ; le premier suit la règle
  « chaque projet possède sa convention » (ADR-009).
- **Rotation planifiée** des secrets de déploiement : quelle cadence, et quel
  signal prouve qu'elle a eu lieu (ADR-025) ?
