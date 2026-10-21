# ADR-018 : Chiffrement applicatif par enveloppe, une clé par personne

- **Statut** : Proposé le 2026-10-21, non contresigné.
- **Amendé le 2026-10-21** (revue des ADR, décision 6) : mise en cohérence du renvoi à ADR-020, qui n'est plus un staging pseudonymisé. Amendement proposé, non contresigné.
- **Irréversible** : oui. Le format des données chiffrées s'inscrit dans chaque ligne et chaque fichier ; en changer exige de tout re-chiffrer, et une migration de schéma qui l'introduit est un point irréversible au sens de `foyer/skills/convergence-iac.md`.
- **S'articule avec** : ADR-017 (OpenBao enveloppe les clés de données), ADR-019 (base de clés séparée, effacement cryptographique), ADR-020 (seconde production éphémère, restaurée avec la base de clés), ADR-027 (fichiers chiffrés avant envoi), `foyer/KITS.md` (port et adaptateurs enfichables de kit-actix).
- **Sert la vision (ADR-000)** : règle « aucune brique ne détient tout », parce qu'une copie de la base sans OpenBao ne livre rien ; principe n°4, parce que le chiffrement vit dans un port unique du kit au lieu d'être réinventé par chaque SaaS ; principe n°5, parce que l'effacement d'une personne devient vérifiable par l'absence de sa clé.

## Contexte

Les SaaS de la méthode conservent des données de valeur : données personnelles,
documents, comptes. Le chiffrement du disque et de la base protège contre le vol
de support, pas contre une fuite de sauvegarde, une requête SQL abusive ou une
copie de base exfiltrée par l'application elle-même. CyFun Essential demande une
politique de chiffrement des données au repos (`foyer/skills/conformite.md`,
§2bis, menace « données au repos »).

Chiffrer dans le backend pose deux problèmes à résoudre en même temps : où vit la
clé (ADR-017), et comment chercher dans des données qu'on ne peut plus lire en
SQL.

## Décision

1. **Chiffrement par enveloppe dans le backend Rust, avec les bibliothèques
   RustCrypto** (AES-256-GCM ou XChaCha20-Poly1305 pour les données, aucune
   primitive maison). Chaque donnée sensible est chiffrée par une **clé de
   données**, elle-même chiffrée (enveloppée) par la clé maîtresse d'OpenBao.
2. **Une clé de données par personne.** Les champs sensibles et les fichiers
   d'une personne utilisent sa clé ; détruire cette clé rend ses données
   illisibles partout, sauvegardes comprises (ADR-019).
3. **Index aveugle pour les recherches.** Un champ qu'il faut retrouver
   (courriel, identifiant) porte, à côté de son chiffré, un HMAC calculé avec une
   clé d'index distincte. On cherche par égalité sur le HMAC, jamais sur le clair.
   Les recherches partielles ou par plage ne sont pas offertes sur ces champs.
4. **Les fichiers aussi.** Un fichier est chiffré par le backend avant son envoi
   au stockage objet (ADR-027), avec la clé de son propriétaire ; le stockage ne
   voit que du chiffré.
5. **Un port dans kit-actix.** Le domaine et l'application voient un port
   `Chiffreur` (envelopper, désenvelopper, indexer) ; **deux adaptateurs
   enfichables** au sens de `foyer/KITS.md` : un adaptateur **OpenBao** pour la
   production et un adaptateur **local**, à clé de test, pour les gates et le
   développement. La même suite de tests passe sur les deux.
6. **Ce qui est chiffré se déclare** dans la cartographie des données du livrable
   Architecture du SaaS (`foyer/bmad/livrables/architecture.template.md`), champ
   par champ.

## Alternatives écartées

- **Chiffrement au niveau de la base (TDE) ou du disque seul.** Protège le
  support, pas la donnée : quiconque obtient une session SQL lit tout.
- **Une seule clé pour toutes les données.** Plus simple, mais l'effacement d'une
  personne redevient une suppression physique, impossible dans les sauvegardes.
- **Chiffrement côté client (de bout en bout).** Le plus fort, mais il interdit au
  service de traiter les données qu'il doit traiter (calculs, notifications,
  documents générés) pour la plupart des SaaS visés.
- **Chiffrement homomorphe ou recherche sur chiffré avancée.** Hors de proportion
  pour un porteur seul (ADR-000, principe n°4).
- **Bibliothèque de chiffrement propre à chaque SaaS.** Contraire au kit : le
  savoir se fragmenterait et chaque erreur se paierait plusieurs fois.

## Conséquences

- **Coût** : un appel à OpenBao à chaque désenveloppement non mis en cache ; une
  colonne d'index par champ cherchable ; des requêtes SQL plus pauvres sur les
  champs chiffrés (pas de `LIKE`, pas de tri).
- **Entretien** : la rotation de la clé maîtresse re-enveloppe les clés de
  données sans re-chiffrer les données ; la rotation de la clé d'index impose de
  recalculer les index.
- **Limite, à écrire honnêtement** : **le backend réunit données et clés en
  mémoire.** Au moment de servir une requête, il détient la donnée en clair et la
  clé de données désenveloppée. Un backend compromis lit ce qu'il sert. Le
  chiffrement protège la base, les sauvegardes et le stockage, pas le processus
  en cours d'exécution. Risque résiduel assumé, borné par le cache court, les
  politiques d'OpenBao et l'alerte sur déchiffrements anormaux (ADR-017, ADR-023).
- **Limite** : un index aveugle révèle l'égalité. Deux lignes au même courriel
  ont le même HMAC ; c'est le prix de la recherche.
- **Limite, sur VPS** : l'hébergeur peut lire la mémoire du backend ; le
  chiffrement applicatif ne l'en empêche pas.

## Mesures CyFun 2025 couvertes

Renvoi à `foyer/skills/conformite.md` §2bis, ligne « Données au repos » :

- **PR.DS-01.x** et **GV.PO-01.2** : chiffrement des données au repos selon une
  politique écrite, ici matérialisée dans un port testé.
- **ID.AM-03.3 ★** et **PR.DS-02.2** : la cartographie des flux du livrable
  Architecture dit quelles données sont chiffrées et où elles circulent.
- Effacement des données personnelles : relève du RGPD plus que de CyFun ;
  catégorie **PR.DS** pour la destruction sûre, identifiant à confirmer dans le
  livret.

## Points ouverts

- **Algorithme par défaut** : AES-256-GCM (accéléré matériellement, nonces à
  gérer) ou XChaCha20-Poly1305 (nonces aléatoires sans risque de collision) ?
- **Granularité** : une clé par personne suffit-elle pour les données partagées
  entre plusieurs personnes (un document de copropriété, par exemple) ?
- **Migration des SaaS existants** : chiffrer des données déjà en clair est une
  migration de données, donc un point irréversible à valider projet par projet.
