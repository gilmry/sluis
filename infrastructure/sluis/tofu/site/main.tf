# Les deux machines du palier 2 (ADR-044, point 3 ; ADR-043, point 1), deux
# instances Public Cloud rattachées au vRack, sur le VLAN de la zone 1 :
#
#   sluis  Sluis public et, plus tard, l'exécutant (zone 1 ; l'exécutant
#          rejoindra sa propre machine à l'éclatement, ADR-010)
#   k3s    le nœud K3s : SaaS, Keycloak des SaaS, et, pendant la dérogation
#          d'ADR-043, Keycloak de plateforme et OpenBao
#
# La supervision reste sur le serveur actuel (ADR-043, point 2) : elle n'est
# pas louée ici. Le vRack et ses réseaux, qui ne dépendent d'aucun centre de
# données, vivent dans ../durable ; le VLAN de la zone 2 y attend la machine
# de confiance.

locals {
  machines = {
    sluis = {
      gabarit = var.gabarit_sluis
      ip      = cidrhost(var.plan_zone_1, 4)
    }
    k3s = {
      gabarit = var.gabarit_k3s
      ip      = cidrhost(var.plan_zone_1, 5)
    }
  }
}

# Gabarit et image résolus par leur nom dans le catalogue de la région : un
# identifiant change d'une région à l'autre, et une reconstruction à
# Strasbourg ne doit rien demander de plus que `region_cloud`.
data "ovh_cloud_instance_flavors" "catalogue" {
  service_name = var.projet
  region       = var.region_cloud
}

data "ovh_cloud_instance_images" "catalogue" {
  service_name = var.projet
  region       = var.region_cloud
}

locals {
  gabarits = {
    for cle, m in local.machines : cle => one([
      for g in data.ovh_cloud_instance_flavors.catalogue.flavors : g.id if g.name == m.gabarit
    ])
  }
  image = one([
    for i in data.ovh_cloud_instance_images.catalogue.images : i.id
    if i.name == var.image && lower(i.status) == "active"
  ])
}

# Le sous-réseau de la zone 1 dans la région de ce site. Le réseau (VLAN) est
# durable ; son sous-réseau suit le site, pour qu'une reconstruction dans une
# autre région ait le sien.
resource "ovh_cloud_network_private_vrack_subnet" "zone_1" {
  service_name = var.projet
  network_id   = var.reseau_zone_1
  region       = var.region_cloud
  name         = "${var.site}-zone-1"
  cidr         = var.plan_zone_1
  dhcp_enabled = true

  # Aucune passerelle : la sortie vers Internet passe par l'interface
  # publique, le VLAN ne sert qu'aux échanges internes. Les adresses fixes
  # des machines (.4, .5) restent hors de la plage distribuée.
  allocation_pools = [
    {
      start = cidrhost(var.plan_zone_1, 10)
      end   = cidrhost(var.plan_zone_1, 200)
    },
  ]
}

resource "ovh_cloud_ssh_key" "administration" {
  service_name = var.projet
  name         = "${var.site}-administration"
  public_key   = var.cle_ssh_publique
}

# Premier filtre, devant nftables : il ne remplace pas le pare-feu des
# machines (rôle `pare_feu`), il borne ce qu'un pare-feu mal rendu laisserait
# passer. Rien d'entrant depuis Internet hors SSH (sources admises), HTTP (défi ACME et
# redirection) et HTTPS ; tout depuis le VLAN, où WireGuard et l'API de K3s
# circulent. La sortie reste ouverte (règle par défaut d'OpenStack).
resource "ovh_cloud_security_group" "machines" {
  service_name = var.projet
  region       = var.region_cloud
  name         = "${var.site}-machines"
  description  = "Entrées admises sur les machines du site ${var.site}"

  rule = concat(
    # SSH depuis les seules sources admises, comme dans nftables : ouvrir SSH
    # à tout Internet est une décision, pas un oubli (point ouvert d'ADR-029).
    [
      for src in var.ssh_sources_admises : {
        direction        = "INGRESS"
        ethernet_type    = strcontains(src, ":") ? "IPV6" : "IPV4"
        protocol         = "TCP"
        port_range_min   = 22
        port_range_max   = 22
        remote_ip_prefix = src
        description      = "SSH d'administration"
      }
    ],
    flatten([
      for v in ["IPV4", "IPV6"] : [
        for port in [80, 443] : {
          direction        = "INGRESS"
          ethernet_type    = v
          protocol         = "TCP"
          port_range_min   = port
          port_range_max   = port
          remote_ip_prefix = v == "IPV4" ? "0.0.0.0/0" : "::/0"
          description      = "TCP ${port}"
        }
      ]
    ]),
    [
      {
        direction        = "INGRESS"
        ethernet_type    = "IPV4"
        protocol         = "ICMP"
        port_range_min   = null
        port_range_max   = null
        remote_ip_prefix = "0.0.0.0/0"
        description      = "ICMP"
      },
      {
        direction        = "INGRESS"
        ethernet_type    = "IPV4"
        protocol         = null
        port_range_min   = null
        port_range_max   = null
        remote_ip_prefix = var.plan_zone_1
        description      = "VLAN de la zone 1"
      },
    ],
  )
}

resource "ovh_cloud_instance" "machine" {
  for_each = local.machines

  service_name       = var.projet
  region             = var.region_cloud
  name               = "${var.site}-${each.key}"
  flavor_id          = local.gabarits[each.key]
  image_id           = local.image
  ssh_key_name       = ovh_cloud_ssh_key.administration.name
  security_group_ids = [ovh_cloud_security_group.machines.id]

  networks = [
    { auto_assign_public_ip = true },
    {
      network_id = var.reseau_zone_1
      subnet_id  = ovh_cloud_network_private_vrack_subnet.zone_1.id
      ip         = each.value.ip
    },
  ]

  # Changer l'image, la clé ou la région reconstruit la machine et efface
  # son disque : journal d'audit, dépôt OAuth, bases. C'est un geste
  # irréversible, réservé au superviseur : il passe par la suppression
  # délibérée de ces lignes, dans un commit relu. Le gabarit, lui, se change
  # en place.
  lifecycle {
    prevent_destroy = true
    ignore_changes  = [image_id]
  }
}
