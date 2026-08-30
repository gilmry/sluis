//! Port de persistance de la fenêtre de dérogation.

use crate::domain::{AppError, FenetreDerogation};

/// Conserve la fenêtre de dérogation en vigueur.
///
/// **Une absence de fenêtre n'est pas une erreur** : c'est l'état par défaut,
/// et il interdit tout bail. C'est la septième condition d'ADR-007, celle qui
/// fait qu'une dérogation ne devient jamais permanente par simple inertie.
pub trait DepotDerogation: Send + Sync {
    /// Fenêtre en vigueur, s'il en existe une d'authentique.
    fn courante(&self) -> Result<Option<FenetreDerogation>, AppError>;

    /// Enregistre une fenêtre nouvellement ouverte.
    fn enregistrer(&self, fenetre: &FenetreDerogation) -> Result<(), AppError>;
}
