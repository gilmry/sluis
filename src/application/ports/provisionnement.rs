//! Port du provisionnement d'infrastructure éphémère.

use crate::domain::{AppError, BailBacASable, CibleEphemere};

/// Provisionne l'infrastructure d'un bail.
///
/// Le bail est exigé, pas déduit : il porte le TTL, le plafond et la preuve
/// que la dérogation était valide. Un provisionnement qui s'en passerait
/// créerait une infrastructure que rien n'oblige à disparaître.
pub trait Provisionneur: Send + Sync {
    /// Crée l'infrastructure et rend la cible à charger.
    ///
    /// Doit être sûr à rejouer : un appel qui suit un échec partiel converge
    /// vers le même état plutôt que d'empiler des ressources.
    fn provisionner(
        &self,
        bail: &BailBacASable,
        sortie_adresse: &str,
    ) -> Result<CibleEphemere, AppError>;
}

/// Détruit une infrastructure éphémère.
///
/// Vit ici et non dans l'adaptateur : c'est une frontière de l'application,
/// et le cas d'usage de campagne s'en sert pour garantir la destruction quel
/// que soit le chemin de sortie.
pub trait DestructeurBail: Send + Sync {
    /// Détruit le bail.
    ///
    /// **Doit être idempotent** : la garde RAII, le chien de garde et le cas
    /// d'usage peuvent tous trois réclamer la destruction du même bail.
    fn detruire(&self, bail: &BailBacASable) -> Result<(), AppError>;
}
