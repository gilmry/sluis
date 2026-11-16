# Location du petit VPS Sluis (ADR-043, point 1). OpenTofu loue, il ne
# configure pas (ADR-010, ADR-032) : tout ce qui se passe sur la machine est
# l'affaire d'Ansible, dans ../ansible.
#
# Lancé uniquement depuis le poste du superviseur, par ../deployer.sh
# (ADR-037, invariant 1). Aucun agent IA ne lance ce module : le jeton OVH qui
# le pilote est un secret d'amorce.

terraform {
  # 1.8 au minimum : le chiffrement de l'état lit une variable (évaluation
  # précoce), ce que 1.7 refuse.
  required_version = ">= 1.8.0"

  required_providers {
    ovh = {
      source  = "ovh/ovh"
      version = "~> 2.25"
    }
  }

  # L'état reste sur le poste, hors du dépôt, et il est chiffré. Il contient
  # le nom de service et l'adresse de la machine : ni secret au sens strict, ni
  # rien qui ait sa place dans un dépôt public. Point ouvert d'ADR-037 (« état
  # distant d'OpenTofu ») tranché à titre provisoire, voir ../README.md.
  backend "local" {
    path = var.chemin_etat
  }

  # Chiffrement natif d'OpenTofu, obligatoire (`enforced`) : un état ou un plan
  # en clair fait échouer la commande au lieu d'être écrit.
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

# Les identifiants viennent de l'environnement (OVH_APPLICATION_KEY,
# OVH_APPLICATION_SECRET, OVH_CONSUMER_KEY), jamais d'un fichier.
provider "ovh" {
  endpoint = "ovh-eu"
}
