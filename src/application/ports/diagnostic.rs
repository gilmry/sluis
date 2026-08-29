//! Port de diagnostic de l'environnement d'exécution.

use crate::domain::{AppError, RapportDiagnostic};

/// Rend l'état réel de l'outillage et des identifiants disponibles.
///
/// Ce port existe pour que `sluis_doctor` reste testable sans dépendre du
/// `PATH` de la machine qui joue les tests, ce que NFR-06 impose.
pub trait Diagnostic: Send + Sync {
    /// Établit le rapport.
    ///
    /// Ne rend jamais d'erreur pour cause d'absence : une absence **est** le
    /// résultat attendu sur une machine nue, pas une anomalie.
    fn etablir(&self) -> Result<RapportDiagnostic, AppError>;
}
