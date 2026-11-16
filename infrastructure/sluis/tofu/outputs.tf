# Lues par ../deployer.sh pour écrire l'inventaire Ansible, hors du dépôt.

output "service" {
  description = "Nom de service OVHcloud du VPS (sert aussi d'adresse DNS)."
  value       = ovh_vps.sluis.name
}

output "etat" {
  description = "État du VPS chez l'hébergeur."
  value       = ovh_vps.sluis.state
}

output "nom_dns" {
  description = "Nom complet publié dans la zone technique, vide sans zone."
  value       = local.dns_actif ? "${var.sous_domaine}.${var.zone_dns_technique}" : ""
}
