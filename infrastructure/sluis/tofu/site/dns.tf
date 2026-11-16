# Enregistrements des machines dans la zone DNS technique. Facultatif : sans
# zone déclarée, rien n'est créé et le jeton DNS n'est jamais lu.
#
# **Un jeton à part, borné à cette seule zone.** Le jeton d'infrastructure
# peut commander et payer ; celui-ci ne peut qu'écrire des enregistrements de
# la zone technique et la rafraîchir. Aucun ne quitte le poste du superviseur :
# les certificats TLS s'obtiennent par le défi HTTP, si bien qu'aucune machine
# exposée à Internet ne détient de droit sur le DNS.
#
# Les noms servis restent les mêmes d'un site à l'autre (ADR-043 et ADR-044,
# mesure conservatoire 1) : ce sont des émetteurs de jetons, et un émetteur qui
# change casse tous les clients. Reconstruire ailleurs, c'est faire pointer
# les mêmes noms sur les machines du site neuf (voir ../../README.md).

provider "ovh" {
  alias              = "dns"
  endpoint           = "ovh-eu"
  application_key    = var.dns_application_key
  application_secret = var.dns_application_secret
  consumer_key       = var.dns_consumer_key
}

locals {
  dns_actif = var.zone_dns_technique != ""

  # Adresses publiques de chaque instance : toutes sauf son adresse dans le
  # VLAN de la zone 1.
  # L'interface publique est la seule entrée sans identifiant de réseau.
  publiques = {
    for cle, m in ovh_cloud_instance.machine : cle => flatten([
      for n in try(m.current_state.networks, []) : n.addresses if n.id == null
    ])
  }
  ipv4 = { for cle, a in local.publiques : cle => [for x in a : x.ip if x.version == 4] }
  ipv6 = { for cle, a in local.publiques : cle => [for x in a : x.ip if x.version == 6] }

  # Sous-domaine → machine qui le sert.
  noms = merge(
    { (var.sous_domaine) = "sluis" },
    { for nom in var.noms_k3s : nom => "k3s" },
  )
}

resource "ovh_domain_zone_record" "a" {
  provider = ovh.dns
  for_each = local.dns_actif ? local.noms : {}

  zone      = var.zone_dns_technique
  subdomain = each.key
  fieldtype = "A"
  ttl       = var.ttl_dns
  target    = local.ipv4[each.value][0]

  lifecycle {
    precondition {
      condition     = length(local.ipv4[each.value]) == 1
      error_message = "Chaque machine doit porter exactement une adresse IPv4 pour que l'enregistrement A soit sans ambiguïté."
    }
  }
}

resource "ovh_domain_zone_record" "aaaa" {
  provider = ovh.dns
  for_each = local.dns_actif ? { for nom, machine in local.noms : nom => machine if length(local.ipv6[machine]) > 0 } : {}

  zone      = var.zone_dns_technique
  subdomain = each.key
  fieldtype = "AAAA"
  ttl       = var.ttl_dns
  target    = local.ipv6[each.value][0]
}
