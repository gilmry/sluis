//! Port de découverte de la déclaration de charge d'un dépôt.

use crate::domain::{AppError, DeclarationCharge};

/// Lit ce qu'un projet déclare pour être mesurable sous charge.
///
/// L'absence de déclaration est une erreur, pas un défaut silencieux : un
/// dépôt qui ne déclare rien n'est pas mesurable, et l'agent doit l'apprendre
/// plutôt que de recevoir des valeurs devinées.
pub trait DepotCharge: Send + Sync {
    /// Lit la déclaration à la racine d'infrastructure donnée.
    fn lire(&self, racine: &str) -> Result<DeclarationCharge, AppError>;
}
