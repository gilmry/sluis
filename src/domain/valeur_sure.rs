//! `ValeurSure` — une chaîne admissible comme argument d'un moteur externe.
//!
//! **Le refus a lieu à l'admission, pas par échappement.** C'est une décision,
//! pas une commodité : échapper suppose de connaître le langage de destination,
//! or Sluis passe ses arguments à six moteurs différents dont il ne maîtrise
//! aucun analyseur. Refuser une valeur douteuse est vérifiable ; l'échapper
//! correctement pour six cibles ne l'est pas.
//!
//! Les arguments ne transitent jamais par un shell — ils sont passés en
//! tableau — donc les métacaractères sont déjà inoffensifs. Cette validation
//! est une seconde barrière : ces mêmes valeurs servent aussi à construire des
//! chemins et des messages, où elles ne sont plus inoffensives.

use std::fmt;

use serde::Serialize;

use crate::domain::AppError;

/// Caractères refusés dans un argument.
///
/// Les métacaractères de shell d'abord, puis les séparateurs de chemin
/// remontants, puis tout caractère de contrôle.
const INTERDITS: &[char] = &[
    ';', '|', '&', '$', '`', '\n', '\r', '\0', '<', '>', '(', ')', '{', '}', '*', '?', '!', '\'',
    '"', '\\',
];

/// Une valeur validée, admissible comme argument d'un moteur.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct ValeurSure(String);

impl ValeurSure {
    /// Valide une valeur.
    ///
    /// Refuse le vide, les métacaractères, les caractères de contrôle et toute
    /// remontée de chemin `..`.
    pub fn new(valeur: impl Into<String>) -> Result<Self, AppError> {
        let valeur = valeur.into();
        if valeur.trim().is_empty() {
            return Err(AppError::Configuration {
                detail: "argument vide".to_string(),
            });
        }
        if let Some(interdit) = valeur
            .chars()
            .find(|c| INTERDITS.contains(c) || c.is_control())
        {
            return Err(AppError::Configuration {
                detail: format!(
                    "argument refusé : le caractère « {} » n'est pas admis. \
                     Sluis refuse à l'admission plutôt que d'échapper, car l'échappement \
                     supposerait de connaître l'analyseur de chacun des six moteurs",
                    interdit.escape_default()
                ),
            });
        }
        if valeur.contains("..") {
            return Err(AppError::CheminHorsRacine {
                chemin: valeur.clone(),
            });
        }
        Ok(Self(valeur))
    }

    /// Valeur brute.
    pub fn valeur(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ValeurSure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Résumé d'un plan Terraform.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlanTerraform {
    /// Ressources à créer.
    pub creations: u32,
    /// Ressources à modifier.
    pub modifications: u32,
    /// Ressources à détruire.
    pub destructions: u32,
    /// Sortie brute, conservée pour l'audit.
    pub brut: String,
}

impl PlanTerraform {
    /// Vrai si le plan ne change rien — donc si l'infrastructure a convergé.
    pub fn sans_changement(&self) -> bool {
        self.creations == 0 && self.modifications == 0 && self.destructions == 0
    }
}

/// Résultat d'une mutation Terraform réellement appliquée.
///
/// Distinct de [`PlanTerraform`] à dessein : un plan annonce, une mutation
/// constate. Les confondre ferait qu'un « 3 to add » jamais appliqué se lirait
/// comme trois ressources créées.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MutationTerraform {
    /// Ressources créées.
    pub creations: u32,
    /// Ressources modifiées.
    pub modifications: u32,
    /// Ressources détruites.
    pub destructions: u32,
    /// Sortie brute, conservée pour l'audit.
    pub brut: String,
}

impl MutationTerraform {
    /// Vrai si la mutation n'a touché aucune ressource.
    pub fn sans_effet(&self) -> bool {
        self.creations == 0 && self.modifications == 0 && self.destructions == 0
    }
}

/// Statut d'une release Helm.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatutHelm {
    /// Nom de la release.
    pub release: String,
    /// Statut rapporté.
    pub statut: String,
    /// Révision courante.
    pub revision: u32,
}

/// Statut d'une application ArgoCD.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatutArgocd {
    /// Nom de l'application.
    pub application: String,
    /// État de synchronisation.
    pub synchronisation: String,
    /// État de santé.
    pub sante: String,
}
