# Déploiement de Sluis lui-même sur son petit VPS

Ce dossier loue et configure le **petit VPS Sluis** d'ADR-043 (point 1) : le
Sluis public et, plus tard, le Sluis exécutant sur une même machine, isolés
l'un de l'autre. Le partage des rôles est celui d'ADR-032 :

| Couche | Outil | Ici |
|---|---|---|
| Location | OpenTofu (ADR-010) | `tofu/` : commande du VPS, enregistrements DNS facultatifs |
| Socle | Ansible | `ansible/` : SSH, pare-feu, mises à jour, volume chiffré, Docker, Traefik |
| Application | `deploy.sh` (boucle GitOps existante) | posé et programmé par le rôle `sluis` |
| Orchestration | `deployer.sh` | enchaîne le tout, depuis le poste du superviseur |

Tout se lance **depuis le poste du superviseur, par lui seul** (ADR-037,
invariant 1). Les jetons OVHcloud, la phrase de l'état, le secret TOTP et la
phrase du volume chiffré sont des secrets d'amorce : aucun agent IA ne les voit.

## Ce qui reste hors du dépôt

| Quoi | Où | Gabarit |
|---|---|---|
| Jetons de location et DNS, chemins | `~/.config/sluis-infra/env` (0600) | `.env.exemple` |
| Variables OpenTofu | `~/.config/sluis-infra/sluis.tfvars` | `tofu/sluis.tfvars.example` |
| État et plans OpenTofu, chiffrés | `~/.local/state/sluis-infra/` | aucun |
| Inventaire Ansible | `~/.config/sluis-infra/inventaire.yml` | `ansible/inventaire.exemple.yml` |
| Secrets du serveur, chiffrés par SOPS | dépôt privé | `ansible/secrets.exemple.yml` |

## Trois jetons OVHcloud, trois périmètres

1. **Location** (poste) : commander et payer le VPS. Aucune route de
   résiliation : il ne peut pas rendre la machine. `./deployer.sh regles location`.
2. **DNS** (poste) : écrire les enregistrements de la seule zone technique.
   Le serveur n'en a jamais de copie, puisque le certificat s'obtient par le
   défi HTTP. `./deployer.sh regles dns <zone>`.
3. **Bac à sable** (serveur, exécutant seulement) : l'API OVHcloud et
   l'utilisateur OpenStack du projet bac à sable (H1). Dans le fichier SOPS,
   puis dans le `.env` du serveur ; tofu ne les reçoit que s'ils sont nommés
   dans `[tofu] variables_transmises` (ADR-034, point 4).

## Ordre

```
./deployer.sh plan            # rien n'est loué
./deployer.sh louer           # COMMANDE PAYANTE, confirmation tapée
./deployer.sh empreinte SHA256:...   # lue dans la console KVM, jamais au premier contact
./deployer.sh totp            # le secret naît sur la machine, dans ton terminal
./deployer.sh chiffrer        # phrase LUKS tapée par toi, rangée en zone 3
./deployer.sh configurer      # socle, Docker, Traefik, Sluis
./deployer.sh verifier        # --check --diff : changed=0 exigé, journal daté
```

Après un redémarrage du VPS : `./deployer.sh deverrouiller`. Docker attend le
volume chiffré, et Sluis reste arrêté jusque-là.

## SSH : TOTP d'abord, clé matérielle ensuite

`socle_ssh_methode` (ADR-045, point 4) :

- **`totp`, par défaut** : clé publique ed25519, puis code TOTP par PAM. Jamais
  de mot de passe Unix, jamais de clé RSA ou ECDSA.
- **`cle_materielle`, deuxième méthode** : clé `sk-ssh-ed25519` sur YubiKey
  seule (ADR-042). Le rôle refuse de basculer tant qu'une clé non matérielle
  reste dans `authorized_keys`, pour ne pas couper l'accès.

Ansible ne sait pas répondre à une invite TOTP : `deployer.sh` ouvre une
connexion maîtresse (clé puis code), qu'Ansible réutilise pendant 15 minutes.

## Chiffrement des disques

Un volume **LUKS2** (aes-xts, argon2id), dans un fichier sur le disque du VPS,
porte tout ce que la machine sait : données de Docker (images, volumes, journal
d'audit, dépôt OAuth), certificats TLS, dépôt de déploiement et son `.env`.
Aucune entrée dans `crypttab` : la phrase n'est jamais sur la machine.

## Décisions provisoires, à contresigner

- **État d'OpenTofu** (point ouvert d'ADR-037) : local, sur le poste, chiffré
  par OpenTofu (pbkdf2 et AES-GCM, `enforced`). Pas d'état distant tant qu'un
  seul porteur lance ces commandes.
- **Déverrouillage du volume** (point ouvert d'ADR-032) : manuel et distant,
  par le superviseur. Le prix : après un redémarrage, Sluis est indisponible
  jusqu'au déverrouillage.
- **Volume dans un fichier plutôt que disque entier** : chiffrer la racine
  d'un VPS exige le mode secours et un déverrouillage dans l'initramfs. Le
  système, ses paquets et le journal système restent en clair.

## Ce qui manque, dit tel quel

- **Rien n'a tourné sur une vraie machine.** Validé : `tofu validate`,
  `ansible-lint` (profil production), `--syntax-check`, `shellcheck`, et
  `sshd -t` et `nft -c` sur les gabarits rendus, plus la création LUKS en
  conteneur. Pas validé : une commande réelle, un passage Ansible complet, la
  preuve `changed=0` au second passage.
- **Routes du jeton de location** : déduites de la documentation du
  fournisseur (v2.25). La réinstallation avec clé publique peut en demander
  une autre ; le premier `louer` le dira.
- **ADR-043, point 3, incomplet** : les deux Sluis lisent le même `.env` et
  tournent sous le même utilisateur de conteneur. « Deux utilisateurs
  système, secrets lisibles par l'exécutant seul » reste à faire, avant de
  passer `sluis_executant` à vrai.
- **Traefik lit le socket Docker** : accès équivalent à root sur l'hôte, à
  remplacer par un mandataire filtré.
- **Palier** : ADR-044 place le petit VPS Sluis au palier 2 (premier revenu).
  Le louer avant est une décision du superviseur.
- **Rendre le VPS** : impossible par ces outils, et voulu (`prevent_destroy`,
  jeton sans route de résiliation). C'est un geste irréversible : le disque et
  le journal d'audit disparaissent.
