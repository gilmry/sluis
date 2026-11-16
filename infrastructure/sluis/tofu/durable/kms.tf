# Déverrouillage automatique d'OpenBao par le KMS d'OVHcloud (ADR-017,
# point 7), par le greffon `ovhcloud` d'openbao-plugins.
#
# - Un domaine KMS dédié, qui ne porte que la clé de déverrouillage : la
#   politique d'accès ne peut donc rien ouvrir d'autre.
# - Une clé symétrique, bornée à chiffrer et déchiffrer.
# - Un compte de service sans secret, et un certificat dont la clé privée naît
#   sur le nœud K3s et n'en sort pas : tofu ne reçoit que la demande de
#   certificat (csr_openbao).
# - Le KMS n'est ni à Gravelines ni à Paris (variable region_kms).

resource "ovh_okms" "openbao" {
  ovh_subsidiary = data.ovh_me.compte.ovh_subsidiary
  region         = var.region_kms
  display_name   = "${var.prefixe} déverrouillage d'OpenBao"

  lifecycle {
    prevent_destroy = true
  }
}

resource "ovh_okms_service_key" "deverrouillage" {
  okms_id    = ovh_okms.openbao.id
  name       = "openbao-deverrouillage"
  type       = "oct"
  size       = 256
  operations = ["encrypt", "decrypt"]

  # Perdre cette clé rend illisible le stockage d'OpenBao, sauvegardes
  # comprises, hors parts de secours (ADR-017, ADR-035).
  lifecycle {
    prevent_destroy = true
  }
}

resource "ovh_me_api_oauth2_client" "openbao" {
  name                  = "${var.prefixe}-openbao"
  description           = "OpenBao : déverrouillage par le KMS, certificat seulement"
  flow                  = "CLIENT_CREDENTIALS"
  discard_client_secret = true
}

resource "ovh_iam_policy" "openbao" {
  name        = "${var.prefixe}-openbao-deverrouillage"
  description = "OpenBao ne fait que chiffrer et déchiffrer avec sa clé de déverrouillage"
  identities  = [ovh_me_api_oauth2_client.openbao.identity]
  resources   = [ovh_okms.openbao.iam.urn]
  allow = [
    "okms:apikms:serviceKey/get",
    "okms:apikms:serviceKey/encrypt",
    "okms:apikms:serviceKey/decrypt",
  ]
}

# Validité de 365 jours au plus chez OVHcloud : renouvelée par une nouvelle
# demande (deployer.sh kms), alertée avant l'échéance (ADR-025).
resource "ovh_okms_credential" "openbao" {
  count = var.csr_openbao == "" ? 0 : 1

  okms_id       = ovh_okms.openbao.id
  name          = "openbao"
  description   = "Certificat d'OpenBao, clé privée née sur le nœud K3s"
  identity_urns = [ovh_me_api_oauth2_client.openbao.identity]
  csr           = file(var.csr_openbao)
  validity      = 365
}
