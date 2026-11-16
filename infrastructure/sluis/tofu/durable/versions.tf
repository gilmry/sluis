# Ce qui doit survivre à la perte du centre de données des machines :
#
#   - le projet Public Cloud, le vRack et un réseau privé (VLAN) par zone ;
#   - les compartiments de sauvegarde, à Paris (EU-WEST-PAR, trois zones de
#     disponibilité), versionnés et verrouillés en écriture ;
#   - un identifiant S3 par consommateur, borné à son compartiment ;
#   - le KMS qui déverrouille OpenBao (ADR-017, point 7), hors de Gravelines.
#
# Un état à part, qui ne suit pas les sites (../site) : reconstruire les
# machines ailleurs ne touche ni aux sauvegardes ni au KMS, et en a besoin.
#
# Lancé uniquement depuis le poste du superviseur, par ../../deployer.sh
# (ADR-037, invariant 1). Aucun agent IA ne lance ce module.

terraform {
  required_version = ">= 1.8.0"

  required_providers {
    ovh = {
      source  = "ovh/ovh"
      version = "~> 2.25"
    }
  }

  backend "local" {
    path = var.chemin_etat
  }

  # Mêmes règles que l'état d'un site : chiffré, obligatoire. Celui-ci porte
  # en plus les identifiants S3 : sa phrase est la même, sa copie à Paris
  # aussi (deployer.sh sauvegarder-etat).
  encryption {
    key_provider "pbkdf2" "phrase" {
      passphrase = var.phrase_etat
    }

    method "aes_gcm" "etat" {
      keys = key_provider.pbkdf2.phrase
    }

    state {
      method   = method.aes_gcm.etat
      enforced = true
    }

    plan {
      method   = method.aes_gcm.etat
      enforced = true
    }
  }
}

provider "ovh" {
  endpoint = "ovh-eu"
}
