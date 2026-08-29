//! Port d'horloge.

use crate::domain::Horodatage;

/// Rend l'instant courant.
///
/// Existe pour que le domaine n'ait jamais à lire l'horloge lui-même —
/// `SystemTime::now` figure d'ailleurs dans les interdits de la gate de pureté.
/// Les invariants les plus sensibles du dispositif étant des expirations, il
/// faut pouvoir les prouver à la seconde près, ce qu'une horloge réelle
/// interdit.
pub trait Horloge: Send + Sync {
    /// L'instant courant.
    fn maintenant(&self) -> Horodatage;
}

/// Horloge figée, pour les tests.
#[derive(Debug, Clone, Copy)]
pub struct HorlogeFigee(Horodatage);

impl HorlogeFigee {
    /// Fige l'horloge à un instant donné.
    pub const fn a(instant: Horodatage) -> Self {
        Self(instant)
    }
}

impl Horloge for HorlogeFigee {
    fn maintenant(&self) -> Horodatage {
        self.0
    }
}
