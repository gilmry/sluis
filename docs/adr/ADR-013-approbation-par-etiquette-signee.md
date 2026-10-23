# ADR-013 : Pour un dépôt privé, l'approbation est une étiquette signée

- **Sorti de l'index le 2026-10-21** (revue des ADR, décision 7) : texte déplacé depuis `README.md` sans modification de fond.

**Statut : proposée le 2026-10-21, contresignée le 2026-10-21.** Amende ADR-008 pour les
dépôts privés ; ADR-008 reste la règle pour les dépôts publics.
**Amendée le 2026-10-21** (revue des ADR, décision 3) : le message signé nomme
l'environnement cible, et chaque tireur refuse une étiquette qui ne nomme pas
le sien. Amendement contresigné le 2026-10-21.

**Contexte.** ADR-008 fait reposer l'approbation de Tier 1 sur un environnement
GitHub protégé à **relecteurs requis**. Or, selon la documentation de GitHub
(vérifiée le 2026-10-19) : « If you are on a GitHub Free, GitHub Pro, or GitHub
Team plan, required reviewers are only available for public repositories. »
Le premier projet témoin du jalon zéro est un dépôt **privé**. Sur lui, Sluis
refuse à juste titre de soumettre (#2 : environnement non protégé), et le
critère de sortie n°2 n'est pas atteignable par ADR-008. Le superviseur a écarté
le 2026-10-21 le plan Enterprise, le dépôt de déploiement public séparé et le
renoncement au relecteur, et retenu **une approbation hors GitHub**, sous la
forme d'une étiquette git signée par sa clé.

**Décision.**

1. **L'acte d'approbation est une étiquette git signée par la clé du
   superviseur**, posée sur le commit à mettre en ligne, au nom réservé
   `approuve/<empreinte courte>`. Son message porte l'empreinte complète du plan
   de changement **et l'environnement cible** (amendement du 2026-10-21). La clé de signature (SSH de préférence, GPG accepté) ne quitte
   jamais le poste du superviseur, et ne réside sur aucun serveur.
2. **La signature est vérifiée par le tireur GitOps de la cible, pas dans un
   workflow.** Un workflow vit dans le dépôt : quiconque peut y écrire pourrait
   le modifier pour sauter la vérification. Le tireur (le script que le cron de
   la cible lance déjà) ne déploie qu'un commit porteur d'une étiquette
   `approuve/*` dont la signature se vérifie contre un fichier
   `allowed_signers` **conservé sur la cible, hors du dépôt, en lecture seule
   pour le compte qui déploie**. Une étiquette non signée, signée par une autre
   clé, ou qui ne désigne pas le commit attendu, ne déploie rien. **Chaque
   tireur refuse une étiquette qui ne nomme pas son propre environnement**, et
   journalise ce refus en erreur : une approbation donnée pour le staging ne
   vaut pas pour la production.
3. **Sluis prépare, ne signe jamais.** Pour un dépôt privé, `sluis_mise_en_ligne`
   vérifie les gates du plancher comme aujourd'hui, calcule l'empreinte, et rend
   au superviseur la commande exacte à signer. `sluis_etat_approbation` lit les
   étiquettes avec un jeton **en lecture seule** et rapporte l'état. Ni le Sluis
   public ni l'exécutant ne détiennent de clé capable de produire une
   approbation.

**Garde-fous.**

1. **Usage unique.** Le tireur consigne chaque empreinte déployée avant de
   déployer ; une étiquette déjà consommée ne redéploie pas.
2. **Fraîcheur.** Une étiquette de plus de 72 h (date de la signature) est
   refusée, comme la demande de renouvellement d'ADR-012.
3. **Le dépôt ne fait pas foi pour les clés.** `allowed_signers` n'est jamais
   lu depuis le dépôt ; le modifier exige un accès au serveur.
4. **Journal.** Le tireur écrit l'empreinte, le commit, l'identité de la clé et
   l'horodatage de chaque déploiement : c'est la preuve datée qu'attend la
   conformité (Epic 9, PR.PS-04.1).
5. **Échec bruyant.** Une étiquette `approuve/*` qui ne se vérifie pas, ou qui
   nomme un autre environnement que celui du tireur, est journalisée en erreur,
   pas ignorée en silence.
6. **Témoin d'environnement.** Une étiquette valide pour le staging, posée sur
   un commit que le tireur de production surveille, ne déploie rien en
   production et produit l'erreur du garde-fou 5. Ce témoin se rejoue à chaque
   modification du tireur.

**Ce que la décision ne couvre pas, et qu'il faut savoir.**

- **La clé du superviseur devient le secret critique.** Sa perte empêche toute
  mise en ligne ; son vol permet d'approuver. Une clé sur support matériel est
  recommandée ; la révocation consiste à la retirer d'`allowed_signers`.
- **Un compromis de la cible reste un compromis de la cible.** Le tireur détient
  déjà de quoi déployer ; ADR-013 ne change rien à ce périmètre.
- **Il n'y a plus de relecteur distinct de l'auteur.** C'est le cas du porteur
  seul : la mesure compensatoire est la signature personnelle et le journal
  (même raisonnement que les mesures compensatoires du porteur seul de Foyer).

**Mode opératoire du superviseur.** À faire sur **son poste**, jamais sur un
serveur : la clé de signature ne doit résider nulle part ailleurs.

1. *Créer une clé de signature dédiée*, distincte de la clé de connexion SSH,
   protégée par une phrase de passe :

   ```bash
   # support matériel FIDO2, recommandé
   ssh-keygen -t ed25519-sk -C "signature git" -f ~/.ssh/signature_git
   # à défaut
   ssh-keygen -t ed25519 -C "signature git" -f ~/.ssh/signature_git
   ```

2. *Configurer git pour signer en SSH* :

   ```bash
   git config --global gpg.format ssh
   git config --global user.signingkey ~/.ssh/signature_git.pub
   git config --global tag.gpgsign true      # optionnel : commit.gpgsign aussi
   ```

3. *Déclarer la clé à GitHub* comme **Signing Key** (*Settings → SSH and GPG
   keys*), pour que l'état rapporté par Sluis et le badge « Verified »
   reflètent la signature. Ce badge est informatif : il ne fait pas foi.

4. *Approuver un plan* : Sluis rend la commande exacte, de la forme

   ```bash
   git tag -s approuve/<empreinte courte> <commit> -m "<environnement> <empreinte complète>"
   git push origin approuve/<empreinte courte>
   ```

5. *Vérifier localement* avant de pousser, avec le même mécanisme que le
   tireur :

   ```bash
   echo "<courriel> $(cat ~/.ssh/signature_git.pub)" > ~/.ssh/allowed_signers
   git config --global gpg.ssh.allowedSignersFile ~/.ssh/allowed_signers
   git tag -v approuve/<empreinte courte>     # attendu : Good "git" signature
   ```

6. *Installer la clé publique sur la cible* : le même format de ligne, dans un
   `allowed_signers` hors du dépôt, propriété de `root`, en lecture seule pour
   le compte qui déploie, et ne contenant **que** la clé du superviseur.

**Révocation et perte.** Retirer la ligne de la clé dans l'`allowed_signers` de
chaque cible suffit à révoquer : plus aucune étiquette signée par elle ne
déploie. Une clé perdue se remplace par une nouvelle clé, installée de la même
façon ; les étiquettes déjà consommées restent valides dans le journal, qui
garde l'identité de la clé.

**Ce qu'un agent signe, et ce qu'il ne signe pas.** Les commits produits par un
agent ne portent pas la signature du superviseur et ne doivent jamais la
porter. Un commit « Unverified » d'agent à côté d'une étiquette « Verified » du
superviseur est la distinction voulue entre ce qui est **proposé** et ce qui
est **approuvé**.

**Alternatives écartées.** GitHub Enterprise (coût récurrent pour un projet
expérimental) ; dépôt de déploiement public séparé (journaux de runs publics,
deux dépôts à tenir) ; environnement sans relecteur (l'invariant « Sluis ne
peut que demander » tomberait) ; vérification dans un workflow GitHub
(contournable par qui peut écrire dans le dépôt) ; mutation par l'exécutant de
Sluis (il n'est pas déployé, et ce serait plus de code pour la même garantie).

**Format du message signé (décidé par le superviseur le 2026-10-23).**
`<environnement> <empreinte complète>` : deux champs séparés par une seule
espace, l'environnement pris dans la liste connue (ADR-009), l'empreinte en
64 caractères hexadécimaux minuscules. Tout autre message est refusé, jamais
deviné. Le format vit dans une seule constante, que Sluis construit et que le
tireur analyse (#24, #25).

**Irréversible ?** Oui : c'est une seconde passerelle d'approbation.
Contresignée par le superviseur le 2026-10-21, en session (règle de
contresignature d'avant la première release, voir l'index).
