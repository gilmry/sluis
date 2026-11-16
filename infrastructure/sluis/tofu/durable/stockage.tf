# Un compartiment par usage, un identifiant par consommateur. Chaque
# compartiment est versionné et verrouillé en écriture : un serveur compromis
# peut ajouter des objets, il ne peut ni écraser ni effacer une version
# verrouillée (ADR-045, point 5.2 ; ADR-027, point 4).
#
# Les WAL et les sauvegardes de base sont rangés par zone (ADR-043, mesure
# conservatoire 4) : les identifiants de la zone 1 n'ouvrent pas ceux de la
# zone 2.

locals {
  compartiments = {
    # Archivage continu des WAL et sauvegardes de base (CloudNativePG, ADR-026).
    "wal-zone-1" = "Bases de la zone 1 : Keycloak des SaaS, puis les SaaS"
    "wal-zone-2" = "Base du Keycloak de plateforme"
    # Instantanés du stockage d'OpenBao (ADR-035), jamais avec l'identifiant
    # d'accès au KMS.
    "openbao" = "Instantanés d'OpenBao"
    # Données du VPS Sluis : journal d'audit, dépôt OAuth.
    "sluis" = "Données du VPS Sluis, chiffrées par age avant l'envoi"
    # États OpenTofu chiffrés, inventaires et fichier SOPS : ce qu'il faut pour
    # reconstruire, si le poste du superviseur disparaît aussi.
    "etat" = "États OpenTofu chiffrés"
  }

  s3_endpoint = "https://s3.${lower(var.region_sauvegardes)}.io.cloud.ovh.net"

  # Droits de chaque consommateur. Aucun n'a s3:BypassGovernanceRetention :
  # personne, depuis une machine, ne lève un verrou.
  consommateurs = {
    # barman-cloud efface les objets sortis de la rétention : la suppression
    # est permise, le verrou la refuse tant que l'objet a moins de 30 jours.
    cnpg-zone-1 = {
      compartiment = "wal-zone-1"
      actions      = ["s3:GetObject", "s3:PutObject", "s3:DeleteObject", "s3:ListBucket", "s3:GetBucketLocation", "s3:ListMultipartUploadParts", "s3:ListBucketMultipartUploads", "s3:AbortMultipartUpload"]
    }
    cnpg-zone-2 = {
      compartiment = "wal-zone-2"
      actions      = ["s3:GetObject", "s3:PutObject", "s3:DeleteObject", "s3:ListBucket", "s3:GetBucketLocation", "s3:ListMultipartUploadParts", "s3:ListBucketMultipartUploads", "s3:AbortMultipartUpload"]
    }
    # Écriture seule : les machines déposent, elles ne relisent ni n'effacent.
    openbao = {
      compartiment = "openbao"
      actions      = ["s3:PutObject", "s3:GetBucketLocation", "s3:ListMultipartUploadParts", "s3:AbortMultipartUpload"]
    }
    sluis = {
      compartiment = "sluis"
      actions      = ["s3:PutObject", "s3:GetBucketLocation", "s3:ListMultipartUploadParts", "s3:AbortMultipartUpload"]
    }
  }
}

resource "ovh_cloud_storage_object_bucket" "compartiment" {
  for_each = local.compartiments

  service_name = ovh_cloud_project.plateforme.project_id
  region       = var.region_sauvegardes
  name         = "${var.prefixe}-${each.key}"

  versioning = {
    status = "ENABLED"
  }

  # Le verrouillage ne s'arme qu'à la création du compartiment : il ne
  # s'ajoute pas après coup. Mode et durée se changent en place.
  object_lock = {
    mode           = upper(var.mode_verrou)
    retention_days = var.retention_verrou_jours
  }

  encryption = {
    algorithm = "AES256"
  }

  tags = {
    usage = each.value
  }

  lifecycle {
    prevent_destroy = true
  }
}

resource "ovh_cloud_project_user" "s3" {
  for_each = local.consommateurs

  service_name = ovh_cloud_project.plateforme.project_id
  description  = "${var.prefixe} ${each.key}"
  role_names   = ["objectstore_operator"]
}

resource "ovh_cloud_project_user_s3_credential" "s3" {
  for_each = local.consommateurs

  service_name = ovh_cloud_project.plateforme.project_id
  user_id      = ovh_cloud_project_user.s3[each.key].id
}

resource "ovh_cloud_project_user_s3_policy" "s3" {
  for_each = local.consommateurs

  service_name = ovh_cloud_project.plateforme.project_id
  user_id      = ovh_cloud_project_user.s3[each.key].id
  policy = jsonencode({
    Statement = [{
      Sid    = replace(each.key, "-", "")
      Effect = "Allow"
      Action = each.value.actions
      Resource = [
        "arn:aws:s3:::${ovh_cloud_storage_object_bucket.compartiment[each.value.compartiment].name}",
        "arn:aws:s3:::${ovh_cloud_storage_object_bucket.compartiment[each.value.compartiment].name}/*",
      ]
    }]
  })
}

# L'identifiant du superviseur : relire tous les compartiments pour
# restaurer, déposer les états chiffrés. Il ne sort pas du poste.
resource "ovh_cloud_project_user" "superviseur" {
  service_name = ovh_cloud_project.plateforme.project_id
  description  = "${var.prefixe} superviseur (poste)"
  role_names   = ["objectstore_operator"]
}

resource "ovh_cloud_project_user_s3_credential" "superviseur" {
  service_name = ovh_cloud_project.plateforme.project_id
  user_id      = ovh_cloud_project_user.superviseur.id
}

resource "ovh_cloud_project_user_s3_policy" "superviseur" {
  service_name = ovh_cloud_project.plateforme.project_id
  user_id      = ovh_cloud_project_user.superviseur.id
  policy = jsonencode({
    Statement = [
      {
        Sid      = "relire"
        Effect   = "Allow"
        Action   = ["s3:GetObject", "s3:GetObjectVersion", "s3:ListBucket", "s3:ListBucketVersions", "s3:GetBucketLocation"]
        Resource = flatten([for c in ovh_cloud_storage_object_bucket.compartiment : ["arn:aws:s3:::${c.name}", "arn:aws:s3:::${c.name}/*"]])
      },
      {
        Sid      = "deposeretat"
        Effect   = "Allow"
        Action   = ["s3:PutObject", "s3:ListMultipartUploadParts", "s3:AbortMultipartUpload"]
        Resource = ["arn:aws:s3:::${ovh_cloud_storage_object_bucket.compartiment["etat"].name}/*"]
      },
    ]
  })
}
