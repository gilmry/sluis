data "ovh_me" "compte" {}

# Le projet Public Cloud de la plateforme : les instances des sites, leurs
# réseaux dans le vRack, et le stockage objet des sauvegardes. Le bac à sable
# de l'exécutant (ADR-007) a son propre projet, hors de cet état : ce que
# l'exécutant loue pour une campagne ne côtoie jamais la production.
data "ovh_order_cart" "panier" {
  ovh_subsidiary = data.ovh_me.compte.ovh_subsidiary
}

data "ovh_order_cart_product_plan" "cloud" {
  cart_id        = data.ovh_order_cart.panier.id
  price_capacity = "renew"
  product        = "cloud"
  plan_code      = "project.2018"
}

resource "ovh_cloud_project" "plateforme" {
  ovh_subsidiary = data.ovh_order_cart.panier.ovh_subsidiary
  description    = "${var.prefixe} : plateforme Sluis"

  plan {
    duration     = data.ovh_order_cart_product_plan.cloud.selected_price[0].duration
    plan_code    = data.ovh_order_cart_product_plan.cloud.plan_code
    pricing_mode = data.ovh_order_cart_product_plan.cloud.selected_price[0].pricing_mode
  }

  # Rendre le projet efface toutes les sauvegardes : réservé au superviseur,
  # par la suppression délibérée de cette ligne.
  lifecycle {
    prevent_destroy = true
  }
}
