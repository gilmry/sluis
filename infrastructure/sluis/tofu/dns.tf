# Enregistrements du VPS dans la zone DNS technique. Facultatif : sans zone
# déclarée, rien n'est créé et le jeton DNS n'est jamais lu.
#
# **Un jeton à part, borné à cette seule zone.** Le jeton de location peut
# commander et payer ; celui-ci ne peut qu'écrire des enregistrements de la
# zone technique et la rafraîchir. Aucun des deux ne sert à l'autre usage, et
# aucun ne quitte le poste du superviseur : le certificat TLS du VPS s'obtient
# par le défi HTTP, si bien que le serveur, exposé à Internet, ne détient
# aucun droit sur le DNS.
#
# Le nom servi doit rester stable (ADR-044, mesure conservatoire 1) : c'est
# l'issuer OAuth de Sluis, et un issuer qui change casse tous les clients.

provider "ovh" {
  alias              = "dns"
  endpoint           = "ovh-eu"
  application_key    = var.dns_application_key
  application_secret = var.dns_application_secret
  consumer_key       = var.dns_consumer_key
}

data "ovh_vps" "sluis" {
  service_name = ovh_vps.sluis.name
}

locals {
  dns_actif = var.zone_dns_technique != ""
  ipv4      = [for ip in data.ovh_vps.sluis.ips : ip if can(regex("^[0-9.]+$", ip))]
  ipv6      = [for ip in data.ovh_vps.sluis.ips : ip if strcontains(ip, ":")]
}

resource "ovh_domain_zone_record" "sluis_a" {
  provider = ovh.dns
  count    = local.dns_actif ? 1 : 0

  zone      = var.zone_dns_technique
  subdomain = var.sous_domaine
  fieldtype = "A"
  ttl       = var.ttl_dns
  target    = local.ipv4[0]

  lifecycle {
    precondition {
      condition     = length(local.ipv4) == 1
      error_message = "Le VPS doit porter exactement une adresse IPv4 pour que l'enregistrement A soit sans ambiguïté."
    }
  }
}

resource "ovh_domain_zone_record" "sluis_aaaa" {
  provider = ovh.dns
  count    = local.dns_actif && length(local.ipv6) > 0 ? 1 : 0

  zone      = var.zone_dns_technique
  subdomain = var.sous_domaine
  fieldtype = "AAAA"
  ttl       = var.ttl_dns
  target    = local.ipv6[0]
}
