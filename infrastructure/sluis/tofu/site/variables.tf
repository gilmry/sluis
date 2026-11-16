variable "chemin_etat" {
  description = "Chemin de l'état sur le poste du superviseur, hors du dépôt."
  type        = string

  validation {
    condition     = !strcontains(var.chemin_etat, "infrastructure/sluis")
    error_message = "L'état ne vit pas dans le dépôt : choisir un chemin hors de l'arbre de travail."
  }
}

variable "phrase_etat" {
  description = "Phrase de chiffrement de l'état et des plans. Jamais dans un fichier versionné."
  type        = string
  sensitive   = true

  validation {
    condition     = length(var.phrase_etat) >= 16
    error_message = "La phrase de chiffrement de l'état fait au moins 16 caractères (exigence de pbkdf2 dans OpenTofu)."
  }
}

variable "site" {
  description = "Nom court du site (par exemple gra1). Il préfixe les machines et a son propre état : reconstruire ailleurs, c'est un site neuf."
  type        = string

  validation {
    condition     = can(regex("^[a-z][a-z0-9]{1,11}$", var.site))
    error_message = "Le site est un nom court : minuscules et chiffres, 2 à 12 caractères."
  }
}

variable "projet" {
  description = "Projet Public Cloud de la plateforme (sortie `projet` de ../durable)."
  type        = string
}

variable "reseau_zone_1" {
  description = "Réseau privé de la zone 1 dans la région du site (sortie `reseaux[\"zone-1/<région>\"].id` de ../durable)."
  type        = string
}

variable "plan_zone_1" {
  description = "Plage d'adresses de la zone 1 dans le vRack."
  type        = string
  default     = "10.101.0.0/24"
}

variable "region_cloud" {
  description = "Région Public Cloud des machines : celle de Gravelines en production, Strasbourg ou Roubaix pour reconstruire. Jamais Paris."
  type        = string
  default     = "GRA11"

  validation {
    condition     = can(regex("^(GRA|SBG|RBX)[0-9]+$", var.region_cloud))
    error_message = "Région Public Cloud à Gravelines, Strasbourg ou Roubaix seulement : jamais Paris, qui porte les sauvegardes."
  }
}

# Gabarits d2 seulement, au plus petit qui tient la charge mesurée des briques
# qu'une machine porte. Un gabarit se change en place (redimensionnement), sans
# reconstruire la machine.
variable "gabarit_sluis" {
  description = "Gabarit de l'instance Sluis : deux Sluis et un Traefik tiennent dans 2 Go."
  type        = string
  default     = "d2-2"

  validation {
    condition     = contains(["d2-2", "d2-4", "d2-8"], var.gabarit_sluis)
    error_message = "Gabarit d2-2, d2-4 ou d2-8 seulement."
  }
}

variable "gabarit_k3s" {
  description = "Gabarit du nœud K3s : deux Keycloak, OpenBao, deux bases CloudNativePG, Flux et cert-manager demandent 8 Go."
  type        = string
  default     = "d2-8"

  validation {
    condition     = contains(["d2-2", "d2-4", "d2-8"], var.gabarit_k3s)
    error_message = "Gabarit d2-2, d2-4 ou d2-8 seulement."
  }
}

variable "image" {
  description = "Nom de l'image du catalogue de la région, résolu en identifiant au plan."
  type        = string
  default     = "Debian 13"
}

variable "cle_ssh_publique" {
  description = "Clé publique d'administration injectée à la livraison : ed25519, logicielle (avec TOTP) ou matérielle sk-ssh-ed25519 (ADR-045)."
  type        = string

  validation {
    condition     = startswith(var.cle_ssh_publique, "ssh-ed25519 ") || startswith(var.cle_ssh_publique, "sk-ssh-ed25519@openssh.com ")
    error_message = "La clé d'administration est une ed25519 (ssh-ed25519 ou sk-ssh-ed25519@openssh.com) : ni RSA, ni ECDSA."
  }
}

# ── DNS technique (dns.tf) ──────────────────────────────────────────────────

variable "ssh_sources_admises" {
  description = "Sources admises en SSH, en CIDR, les mêmes que dans l'inventaire Ansible. Sans valeur par défaut."
  type        = list(string)

  validation {
    condition     = length(var.ssh_sources_admises) > 0 && alltrue([for c in var.ssh_sources_admises : can(cidrhost(c, 0)) && !contains(["0.0.0.0/0", "::/0"], c)])
    error_message = "Au moins une source, en CIDR, et jamais tout Internet."
  }
}

variable "zone_dns_technique" {
  description = "Zone DNS gérée chez OVHcloud où publier le nom du VPS. Vide : aucun enregistrement."
  type        = string
  default     = ""
}

variable "sous_domaine" {
  description = "Sous-domaine du VPS Sluis dans la zone technique. Le nom complet est l'issuer OAuth de Sluis."
  type        = string
  default     = "sluis"
}

variable "noms_k3s" {
  description = "Sous-domaines servis par le nœud K3s : Keycloak des SaaS, Keycloak de plateforme. Stables d'un site à l'autre (ADR-043, mesure 1)."
  type        = list(string)
  default     = ["auth", "plateforme"]
}

variable "ttl_dns" {
  description = "TTL des enregistrements, en secondes (60 au minimum chez OVHcloud)."
  type        = number
  default     = 3600

  validation {
    condition     = var.ttl_dns >= 60
    error_message = "OVHcloud refuse un TTL inférieur à 60 secondes."
  }
}

# Jeton DNS, distinct du jeton de location : TF_VAR_dns_* dans le .env du
# poste, jamais dans un fichier de variables.
variable "dns_application_key" {
  description = "Clé d'application du jeton DNS."
  type        = string
  default     = ""
  sensitive   = true
}

variable "dns_application_secret" {
  description = "Secret d'application du jeton DNS."
  type        = string
  default     = ""
  sensitive   = true
}

variable "dns_consumer_key" {
  description = "Clé de consommateur du jeton DNS, bornée à la zone technique."
  type        = string
  default     = ""
  sensitive   = true
}
