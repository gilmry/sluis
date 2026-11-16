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

variable "nom_affiche" {
  description = "Nom affiché du VPS dans l'espace client."
  type        = string
  default     = "sluis"
}

variable "offre" {
  description = "Code d'offre du catalogue VPS d'OVHcloud (par exemple vps-2027-model1)."
  type        = string

  validation {
    condition     = startswith(var.offre, "vps-")
    error_message = "Le code d'offre est celui du catalogue VPS (préfixe vps-)."
  }
}

variable "duree" {
  description = "Durée d'engagement, au format ISO 8601. P1M : résiliable au mois."
  type        = string
  default     = "P1M"
}

variable "centre_de_donnees" {
  description = "Centre de données. Les données restent en France : seuls les centres français sont admis."
  type        = string
  default     = "GRA"

  # Valeurs lues dans le catalogue public OVHcloud (filiale FR) le 2026-11-16.
  validation {
    condition     = contains(["GRA", "SBG", "EU-WEST-RBX"], var.centre_de_donnees)
    error_message = "Centre de données hors de France : seuls GRA, SBG et EU-WEST-RBX sont admis."
  }
}

variable "systeme" {
  description = "Système installé à la livraison (valeur vps_os du catalogue)."
  type        = string
  default     = "Debian 13"

  validation {
    condition     = var.systeme == "Debian 13"
    error_message = "Les rôles Ansible ne sont écrits que pour Debian 13."
  }
}

variable "image_id" {
  description = "Identifiant de l'image Debian 13 (GET /vps/{service}/images/available). Exigé par le fournisseur dès qu'une clé publique est posée."
  type        = string
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

variable "zone_dns_technique" {
  description = "Zone DNS gérée chez OVHcloud où publier le nom du VPS. Vide : aucun enregistrement."
  type        = string
  default     = ""
}

variable "sous_domaine" {
  description = "Sous-domaine du VPS dans la zone technique. Le nom complet est l'issuer OAuth de Sluis."
  type        = string
  default     = "sluis"
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
