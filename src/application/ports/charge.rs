//! Port du moteur de charge.

use crate::domain::{AppError, MesureCapacite, Palier};

/// Réglages d'un palier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReglagePalier {
    /// Palier concerné.
    pub palier: Palier,
    /// Nombre de connexions simultanées.
    pub connexions: u32,
    /// Nombre de fils.
    pub fils: u32,
    /// Durée, en secondes.
    pub duree_secondes: u32,
}

/// Joue un palier de charge et rend des mesures.
pub trait MoteurCharge: Send + Sync {
    /// Joue un palier contre `cible`.
    ///
    /// Rend `AppError::EngineMissing` si le moteur est absent — et ce refus
    /// doit avoir lieu **à l'admission de la campagne**, pas au milieu de
    /// l'escalier, sous peine de laisser une infrastructure provisionnée et
    /// des mesures inexploitables.
    fn jouer(&self, cible: &str, reglage: &ReglagePalier) -> Result<Vec<MesureCapacite>, AppError>;

    /// Vrai si le moteur est utilisable.
    fn disponible(&self) -> bool;
}
