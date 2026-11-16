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

variable "prefixe" {
  description = "Préfixe des compartiments, unique chez OVHcloud (minuscules, chiffres, tirets)."
  type        = string

  validation {
    condition     = can(regex("^[a-z0-9][a-z0-9-]{2,30}$", var.prefixe))
    error_message = "Préfixe de 3 à 31 caractères : minuscules, chiffres et tirets."
  }
}

variable "region_sauvegardes" {
  description = "Région du stockage objet. EU-WEST-PAR : Paris, trois zones de disponibilité, loin de Gravelines."
  type        = string
  default     = "EU-WEST-PAR"

  # Les machines tournent à Gravelines et se reconstruisent à Strasbourg ou à
  # Roubaix : les sauvegardes ne partagent le sort d'aucun de ces sites.
  validation {
    condition     = var.region_sauvegardes == "EU-WEST-PAR"
    error_message = "Les sauvegardes vivent à Paris (EU-WEST-PAR), hors des centres où tournent ou se reconstruisent les machines."
  }
}

variable "retention_verrou_jours" {
  description = "Durée du verrouillage en écriture des objets, en jours (ADR-045, point 5 : 30 jours)."
  type        = number
  default     = 30

  validation {
    condition     = var.retention_verrou_jours >= 30
    error_message = "Trente jours au moins : en dessous, une compromission découverte tard a déjà effacé les sauvegardes."
  }
}

variable "mode_verrou" {
  description = "governance : un compte qui détient la permission de contournement peut lever le verrou ; compliance : personne, pas même le titulaire du compte."
  type        = string
  default     = "governance"

  validation {
    condition     = contains(["governance", "compliance"], var.mode_verrou)
    error_message = "Mode de verrouillage : governance ou compliance."
  }
}

variable "region_kms" {
  description = "Région du KMS qui déverrouille OpenBao. Ni Gravelines (les machines) ni Paris (les sauvegardes) : une seule perte ne doit jamais emporter à la fois le KMS et l'un des deux."
  type        = string
  default     = "eu-west-rbx"

  validation {
    condition     = contains(["eu-west-rbx", "eu-west-sbg"], var.region_kms)
    error_message = "KMS à Roubaix ou à Strasbourg seulement."
  }
}

variable "csr_openbao" {
  description = "Chemin de la demande de certificat produite SUR le nœud K3s (deployer.sh kms). Vide : pas encore d'identifiant d'accès au KMS."
  type        = string
  default     = ""
}

variable "regions_cloud" {
  description = "Régions Public Cloud où les réseaux des zones existent : Gravelines, et la région de reconstruction, prête d'avance."
  type        = list(string)
  default     = ["GRA11", "SBG5"]

  validation {
    condition     = alltrue([for r in var.regions_cloud : can(regex("^(GRA|SBG|RBX)[0-9]+$", r))])
    error_message = "Réseaux à Gravelines, Strasbourg ou Roubaix seulement : jamais Paris, qui porte les sauvegardes."
  }
}
