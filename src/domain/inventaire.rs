//! Bounded context Inventaire — ce qui est déclaré, ce qui tourne, et l'écart.
//!
//! Les conventions de dossiers d'une infrastructure sont du sens implicite :
//! `monosite/k3s/staging` dit une topologie et un environnement, mais seulement
//! à qui connaît la convention. Ce module la rend explicite et typée, pour
//! qu'un agent n'ait plus à la deviner.

use std::fmt;
use std::str::FromStr;

use serde::Serialize;

use crate::domain::AppError;

/// Forme de déploiement d'un projet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Topologie {
    /// Machine unique, Docker Compose.
    Vps,
    /// Kubernetes léger, nœud unique.
    K3s,
    /// Kubernetes multi-nœuds.
    K8s,
}

impl Topologie {
    /// Toutes les topologies connues.
    pub const TOUTES: [Topologie; 3] = [Topologie::Vps, Topologie::K3s, Topologie::K8s];

    /// Nom canonique, celui qui apparaît dans les chemins.
    pub fn nom(&self) -> &'static str {
        match self {
            Topologie::Vps => "vps",
            Topologie::K3s => "k3s",
            Topologie::K8s => "k8s",
        }
    }
}

impl FromStr for Topologie {
    type Err = AppError;

    fn from_str(entree: &str) -> Result<Self, Self::Err> {
        match entree.trim().to_ascii_lowercase().as_str() {
            "vps" => Ok(Topologie::Vps),
            "k3s" => Ok(Topologie::K3s),
            "k8s" => Ok(Topologie::K8s),
            autre => Err(AppError::Analyse {
                quoi: "topologie".to_string(),
                detail: format!("« {autre} » n'est pas une topologie connue (vps, k3s, k8s)"),
            }),
        }
    }
}

impl fmt::Display for Topologie {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.nom())
    }
}

/// Étage de promotion.
///
/// L'ordre de déclaration **est** l'ordre de promotion, et `Ord` en dérive.
/// Ce n'est pas un détail d'implémentation : c'est ce qui rend
/// [`Environnement::promouvoir_vers`] capable de refuser un saut sans qu'aucune
/// table de correspondance n'ait à être tenue à jour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Environnement {
    Dev,
    Integration,
    Staging,
    Production,
}

impl Environnement {
    /// Tous les environnements, dans l'ordre de promotion.
    pub const TOUS: [Environnement; 4] = [
        Environnement::Dev,
        Environnement::Integration,
        Environnement::Staging,
        Environnement::Production,
    ];

    /// Nom canonique, celui qui apparaît dans les chemins.
    pub fn nom(&self) -> &'static str {
        match self {
            Environnement::Dev => "dev",
            Environnement::Integration => "integration",
            Environnement::Staging => "staging",
            Environnement::Production => "production",
        }
    }

    /// L'étage suivant, s'il existe.
    pub fn suivant(&self) -> Option<Environnement> {
        match self {
            Environnement::Dev => Some(Environnement::Integration),
            Environnement::Integration => Some(Environnement::Staging),
            Environnement::Staging => Some(Environnement::Production),
            Environnement::Production => None,
        }
    }

    /// Promeut vers l'étage visé.
    ///
    /// Refuse tout saut d'étage. C'est l'invariant 6 du Brief : l'ordre de
    /// promotion est total et non contournable. Promouvoir `integration` vers
    /// `production` en sautant `staging` reviendrait à mettre en production
    /// quelque chose que personne n'a vu tourner en pré-production.
    pub fn promouvoir_vers(&self, cible: Environnement) -> Result<Environnement, AppError> {
        match self.suivant() {
            Some(attendu) if attendu == cible => Ok(cible),
            Some(attendu) => Err(AppError::TierViolation {
                raison: format!(
                    "promotion de {self} vers {cible} interdite : l'étage suivant est {attendu}"
                ),
            }),
            None => Err(AppError::TierViolation {
                raison: format!("{self} est le dernier étage, il n'y a rien après"),
            }),
        }
    }
}

impl FromStr for Environnement {
    type Err = AppError;

    fn from_str(entree: &str) -> Result<Self, Self::Err> {
        match entree.trim().to_ascii_lowercase().as_str() {
            "dev" => Ok(Environnement::Dev),
            "integration" => Ok(Environnement::Integration),
            "staging" => Ok(Environnement::Staging),
            "production" => Ok(Environnement::Production),
            autre => Err(AppError::Analyse {
                quoi: "environnement".to_string(),
                detail: format!(
                    "« {autre} » n'est pas un environnement connu \
                     (dev, integration, staging, production)"
                ),
            }),
        }
    }
}

impl fmt::Display for Environnement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.nom())
    }
}

/// Profil de cluster — le contrat entre provisionnement (Day 1) et déploiement
/// (Day 2).
///
/// Il ne porte que des préoccupations de **cluster** : classe de stockage,
/// ingress, TLS, backend de secrets, préréglage de ressources. La configuration
/// métier par environnement vit ailleurs, et les mélanger ferait qu'un même
/// cluster ne pourrait plus servir deux environnements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProfilCluster {
    nom: String,
    classe_stockage: Option<String>,
    classe_ingress: Option<String>,
    backend_secrets: Option<String>,
    tls_actif: Option<bool>,
    preset_ressources: Option<String>,
}

impl ProfilCluster {
    /// Construit un profil. Le nom est le seul champ obligatoire : un profil
    /// qui ne surcharge rien reste un profil valide, il hérite de tout.
    pub fn new(
        nom: String,
        classe_stockage: Option<String>,
        classe_ingress: Option<String>,
        backend_secrets: Option<String>,
        tls_actif: Option<bool>,
        preset_ressources: Option<String>,
    ) -> Result<Self, AppError> {
        if nom.trim().is_empty() {
            return Err(AppError::Configuration {
                detail: "profil de cluster sans nom".to_string(),
            });
        }
        Ok(Self {
            nom,
            classe_stockage,
            classe_ingress,
            backend_secrets,
            tls_actif,
            preset_ressources,
        })
    }

    /// Nom du profil, tel qu'il apparaît dans le nom de fichier.
    pub fn nom(&self) -> &str {
        &self.nom
    }
    /// Classe de stockage imposée par le cluster.
    pub fn classe_stockage(&self) -> Option<&str> {
        self.classe_stockage.as_deref()
    }
    /// Classe d'ingress imposée par le cluster.
    pub fn classe_ingress(&self) -> Option<&str> {
        self.classe_ingress.as_deref()
    }
    /// Backend de secrets attendu sur ce cluster.
    pub fn backend_secrets(&self) -> Option<&str> {
        self.backend_secrets.as_deref()
    }
    /// TLS actif sur ce cluster.
    pub fn tls_actif(&self) -> Option<bool> {
        self.tls_actif
    }
    /// Préréglage de ressources.
    pub fn preset_ressources(&self) -> Option<&str> {
        self.preset_ressources.as_deref()
    }
}

/// Un module Terraform réutilisable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModuleTerraform {
    nom: String,
}

impl ModuleTerraform {
    /// Déclare un module.
    pub fn new(nom: String) -> Result<Self, AppError> {
        if nom.trim().is_empty() {
            return Err(AppError::Configuration {
                detail: "module Terraform sans nom".to_string(),
            });
        }
        Ok(Self { nom })
    }

    /// Nom du module, tel qu'il apparaît dans le dossier.
    pub fn nom(&self) -> &str {
        &self.nom
    }
}

/// Une cellule de la matrice : une topologie déclinée dans un environnement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Cellule {
    /// Topologie de la cellule.
    pub topologie: Topologie,
    /// Environnement de la cellule.
    pub environnement: Environnement,
}

/// La matrice d'infrastructure déclarée d'un dépôt.
///
/// C'est la réponse à « qu'est-ce qui est déclaré ici », rendue sans qu'aucun
/// humain ait eu à la saisir.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct MatriceInfrastructure {
    /// Topologies effectivement déclarées, triées.
    pub topologies: Vec<Topologie>,
    /// Environnements effectivement déclarés, triés dans l'ordre de promotion.
    pub environnements: Vec<Environnement>,
    /// Croisements réellement présents.
    pub cellules: Vec<Cellule>,
    /// Profils de cluster disponibles.
    pub profils: Vec<ProfilCluster>,
    /// Modules Terraform disponibles.
    pub modules: Vec<ModuleTerraform>,
    /// Noms rencontrés mais non reconnus, conservés plutôt que tus.
    ///
    /// Un dossier `local/` ou `bac-a-sable/` n'est pas une erreur : c'est une
    /// convention que Sluis ne connaît pas. Le taire donnerait l'illusion d'un
    /// inventaire exhaustif ; le signaler laisse l'humain juger.
    pub ignores: Vec<String>,
}

impl MatriceInfrastructure {
    /// Vrai si rien n'a été découvert.
    pub fn est_vide(&self) -> bool {
        self.cellules.is_empty() && self.profils.is_empty() && self.modules.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l_ordre_de_promotion_est_total() {
        assert!(Environnement::Dev < Environnement::Integration);
        assert!(Environnement::Integration < Environnement::Staging);
        assert!(Environnement::Staging < Environnement::Production);
    }

    #[test]
    fn production_n_a_pas_de_suivant() {
        assert_eq!(Environnement::Production.suivant(), None);
    }
}
