# Le vRack et un réseau privé par zone (ADR-014 ; ADR-043, mesure
# conservatoire 2). Un VLAN ne dépend d'aucun centre de données : il survit à
# la perte de Gravelines, et une reconstruction à Strasbourg ou à Roubaix y
# rattache ses machines.
#
# Le vRack isole, il ne chiffre pas : WireGuard reste le transport entre zones
# (ADR-042, point 5), et aucune requête n'est crue sur sa provenance réseau
# (ADR-042, point 4).
#
#   zone-1  production : Sluis public, nœud K3s
#   zone-2  confiance : vide jusqu'à l'éclatement (ADR-043, point 4), où le
#           Keycloak de plateforme et OpenBao quittent le nœud K3s

data "ovh_order_cart_product_plan" "vrack" {
  cart_id        = data.ovh_order_cart.panier.id
  price_capacity = "renew"
  product        = "vrack"
  plan_code      = "vrack"
}

resource "ovh_vrack" "plateforme" {
  ovh_subsidiary = data.ovh_order_cart.panier.ovh_subsidiary
  name           = "${var.prefixe}-plateforme"
  description    = "Zones de la plateforme Sluis"

  plan {
    duration     = data.ovh_order_cart_product_plan.vrack.selected_price[0].duration
    plan_code    = data.ovh_order_cart_product_plan.vrack.plan_code
    pricing_mode = data.ovh_order_cart_product_plan.vrack.selected_price[0].pricing_mode
  }

  lifecycle {
    prevent_destroy = true
  }
}

resource "ovh_vrack_cloudproject" "plateforme" {
  service_name = ovh_vrack.plateforme.service_name
  project_id   = ovh_cloud_project.plateforme.project_id
}

locals {
  zones = {
    "zone-1" = 101
    "zone-2" = 102
  }

  # Un réseau par zone et par région : le VLAN est commun aux régions, c'est
  # lui qui fait la zone ; le réseau OpenStack, lui, est régional.
  reseaux = {
    for paire in setproduct(keys(local.zones), var.regions_cloud) :
    "${paire[0]}/${paire[1]}" => { zone = paire[0], region = paire[1], vlan = local.zones[paire[0]] }
  }
}

resource "ovh_cloud_network_private_vrack" "zone" {
  for_each = local.reseaux

  service_name = ovh_cloud_project.plateforme.project_id
  name         = "${var.prefixe}-${each.value.zone}"
  region       = each.value.region
  description  = "Zone ${each.value.zone} de la plateforme Sluis"
  vlan_id      = each.value.vlan

  depends_on = [ovh_vrack_cloudproject.plateforme]
}
