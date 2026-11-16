data "ovh_me" "compte" {}

# Une commande, pas une instance : le VPS est loué par le circuit de commande
# d'OVHcloud et rendu par son circuit de résiliation. Il faut un moyen de
# paiement par défaut sur le compte, et un jeton dont les règles d'accès
# couvrent les routes de commande en plus de /vps/*.
resource "ovh_vps" "sluis" {
  display_name   = var.nom_affiche
  ovh_subsidiary = data.ovh_me.compte.ovh_subsidiary

  image_id       = var.image_id
  public_ssh_key = var.cle_ssh_publique

  # Aucun mot de passe initial envoyé par courriel : la connexion se fait par
  # la clé matérielle, et ce mot de passe serait un secret d'amorce de plus
  # (ADR-037, point 3).
  do_not_send_password = true

  plan = [
    {
      duration     = var.duree
      plan_code    = var.offre
      pricing_mode = "default"

      configuration = [
        {
          label = "vps_datacenter"
          value = var.centre_de_donnees
        },
        {
          label = "vps_os"
          value = var.systeme
        }
      ]
    }
  ]

  # Rendre le VPS détruit son disque : journal d'audit, dépôt OAuth, fenêtre
  # de dérogation. C'est un geste irréversible, réservé au superviseur : il
  # passe par la suppression délibérée de cette ligne, dans un commit relu.
  lifecycle {
    prevent_destroy = true

    # Changer d'image réinstalle la machine. Après la livraison, l'image ne
    # sert plus qu'à cela : on ne la suit pas.
    ignore_changes = [image_id, public_ssh_key]
  }
}
