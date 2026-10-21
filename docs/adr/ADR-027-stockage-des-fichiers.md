# ADR-027 : Les fichiers vivent dans un fork communautaire de MinIO, chiffrés avant envoi

- **Statut** : Proposé le 2026-10-21, non contresigné.
- **Irréversible** : **oui** pour le format des données stockées et pour toute migration d'un volume existant vers une nouvelle version ; le choix de l'image reste réversible tant qu'aucun volume de production n'en dépend.
- **S'articule avec** : ADR-018 (chiffrement applicatif, s'applique aussi aux fichiers), ADR-020 (bucket de staging séparé), ADR-021 (images épinglées et signées), ADR-025 (montées de version), `foyer/KITS.md` (adaptateur de stockage enfichable).
- **Sert la vision (ADR-000)** : principe 4, une brique S3 déjà maîtrisée plutôt qu'une nouvelle ; principe 3, la réplication hors région est vérifiée par une seconde source, la restauration ; règle « aucune brique ne détient tout », le stockage ne voit que du chiffré.

## Contexte

Les SaaS Foyer stockent des fichiers (justificatifs, photos, pièces jointes)
dans un service compatible S3. MinIO était le choix par défaut. Constaté le
2026-10-21 : **les images officielles `minio/minio` et `minio/mc` ne sont plus
publiées sur Docker Hub** (« object not found »). Un projet témoin privé ne
tournait encore que parce que les images étaient en cache sur son hôte ; un
nettoyage, un hôte neuf ou un clone frais auraient fait échouer le déploiement.
Le même problème touche les autres projets qui utilisent MinIO.

## Décision

1. **Un fork communautaire de MinIO**, déployé dans le cluster, **épinglé** :
   par tag daté pour le serveur, par digest pour le client `mc` quand il n'a pas
   de tag versionné. Candidat retenu pour le projet témoin : `pgsty/minio`, seul
   fork évalué à la fois maintenu et épinglable par version (comparé à
   Chainguard, gratuit en `latest` seulement, et à Bitnami et Coollabs, figés).
2. **Garage est évalué** comme alternative de fond : projet européen,
   géo-réparti par conception, licence AGPL. L'évaluation compare la
   compatibilité S3 utile (versioning, verrouillage), l'entretien et la
   restauration.
3. **Les fichiers sont chiffrés par le backend avant l'envoi** (ADR-018). Le
   stockage ne voit que du chiffré ; un accès au bucket sans la clé ne livre
   rien de lisible.
4. **Réplication vers une autre région**, avec **versioning** et **verrouillage
   en écriture** (objet non modifiable pendant sa rétention). Un rançongiciel qui
   obtient les droits d'écriture ne peut ni écraser ni supprimer les versions
   verrouillées.
5. **Un bucket de staging séparé**, peuplé de fichiers factices ou de fichiers
   chiffrés sans leur clé : le staging (ADR-020) ne lit jamais un fichier
   réel en clair.
6. **L'adaptateur de stockage reste un port** côté application
   (`foyer/KITS.md`) : passer de MinIO à Garage ou à un stockage objet managé ne
   touche pas au métier.

## Alternatives écartées

- **Rester sur `minio/minio:latest` en cache.** Fonctionne jusqu'au prochain
  hôte neuf ; c'est une panne différée, et `latest` n'est pas épinglé.
- **Chainguard.** Image soignée et signée, mais sans version épinglable en
  gratuit : contraire à ADR-021.
- **Stockage objet de l'hébergeur seul.** Moins d'entretien, mais la réplication
  et le verrouillage dépendent alors d'un seul fournisseur ; reste la cible
  naturelle de la réplication hors région.
- **Système de fichiers local.** Ni versioning, ni réplication, ni verrouillage.

## Conséquences

- **Coût** : un second stockage hors région, facturé au volume, et la
  surconsommation du versioning.
- **Entretien** : un fork maintenu par un tiers peut s'arrêter. Le port
  d'adaptateur et l'évaluation de Garage sont la sortie prévue ; la veille
  passe par Renovate (ADR-025).
- **Migration** : passer un volume existant d'une version de MinIO à une autre,
  même fork d'une même lignée de code, est une migration de données : sauvegarde
  du volume avant, répétition sur une copie, accord du superviseur.
- **Limite** : le chiffrement côté backend protège le contenu, pas les
  métadonnées (nom d'objet, taille, date). Les noms d'objets ne portent donc
  aucune donnée personnelle.

## Mesures CyFun 2025 couvertes

Mapping dans `foyer/skills/conformite.md` (§2bis : données au repos, sauvegardes
immuables). Données au repos : PR.DS-01.x et GV.PO-01.2 (Important) côté
application. Sauvegarde hors hôte et restauration : PR.DS-11.1 (key measure),
PR.DS-11.2, PR.DS-11.4 (Epic 9). Inventaire logiciel des images épinglées :
ID.AM-02.1 et ID.AM-02.2 via le SBOM.

## Points ouverts

- Garage ou fork de MinIO comme cible durable ? À trancher après évaluation.
- Durée de verrouillage en écriture : elle doit rester compatible avec
  l'effacement cryptographique (ADR-019), qui détruit la clé et non l'objet.
- Le même retrait touche les autres projets qui utilisent MinIO : un suivi par
  dépôt est nécessaire.
