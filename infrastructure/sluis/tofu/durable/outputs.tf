# Lues par ../../deployer.sh et passées à Ansible par un descripteur de
# fichier, jamais écrites en clair sur le disque.

output "projet" {
  description = "Identifiant du projet Public Cloud."
  value       = ovh_cloud_project.plateforme.project_id
}

output "reseaux" {
  description = "Réseau privé de chaque zone, par région : identifiant OpenStack et VLAN."
  value = {
    for cle, r in ovh_cloud_network_private_vrack.zone : cle => {
      id   = r.id
      vlan = r.vlan_id
    }
  }
}

output "s3_endpoint" {
  description = "Point d'accès S3 des compartiments."
  value       = local.s3_endpoint
}

output "compartiments" {
  description = "Nom de chaque compartiment."
  value       = { for cle, c in ovh_cloud_storage_object_bucket.compartiment : cle => c.name }
}

output "identifiants_s3" {
  description = "Identifiant S3 de chaque consommateur, borné à son compartiment."
  sensitive   = true
  value = {
    for cle, c in ovh_cloud_project_user_s3_credential.s3 : cle => {
      access_key_id     = c.access_key_id
      secret_access_key = c.secret_access_key
    }
  }
}

output "identifiant_superviseur" {
  description = "Identifiant S3 du poste : relire et restaurer, déposer les états."
  sensitive   = true
  value = {
    access_key_id     = ovh_cloud_project_user_s3_credential.superviseur.access_key_id
    secret_access_key = ovh_cloud_project_user_s3_credential.superviseur.secret_access_key
  }
}

output "kms" {
  description = "Coordonnées du KMS pour le greffon de scellement d'OpenBao. Rien de secret."
  value = {
    endpoint   = ovh_okms.openbao.rest_endpoint
    kms_id     = ovh_okms.openbao.id
    key_id     = ovh_okms_service_key.deverrouillage.id
    ca         = ovh_okms.openbao.public_ca
    certificat = try(ovh_okms_credential.openbao[0].certificate_pem, "")
    expire_le  = try(ovh_okms_credential.openbao[0].expired_at, "")
  }
}
