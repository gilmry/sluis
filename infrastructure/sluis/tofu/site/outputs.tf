# Lues par ../../deployer.sh pour écrire l'inventaire Ansible, hors du dépôt.

output "machines" {
  description = "Adresses de chaque machine : publique (IPv4) et dans le VLAN de la zone 1."
  value = {
    for cle, m in ovh_cloud_instance.machine : cle => {
      publique = try(local.ipv4[cle][0], "")
      vlan     = local.machines[cle].ip
      statut   = m.resource_status
    }
  }
}

output "noms_dns" {
  description = "Noms complets publiés dans la zone technique, vides sans zone."
  value       = local.dns_actif ? { for nom, machine in local.noms : "${nom}.${var.zone_dns_technique}" => machine } : {}
}
