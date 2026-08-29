//! Le temps, comme valeur du domaine.
//!
//! Le domaine ne lit jamais l'horloge : `SystemTime::now` figure dans la liste
//! des interdits de la gate de pureté. Il reçoit un [`Horodatage`], fourni par
//! le port `Horloge`.
//!
//! Ce n'est pas de la cérémonie. Les invariants les plus sensibles du
//! dispositif sont des expirations — celle d'un jeton d'approbation, celle
//! d'un bail de bac à sable, celle de la fenêtre de dérogation. Une expiration
//! qui dépend de l'horloge du processus est intestable à la seconde près, or
//! c'est exactement à la seconde près qu'il faut la prouver.

use std::fmt;

use serde::Serialize;

/// Un instant, en secondes depuis l'époque Unix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, serde::Deserialize)]
pub struct Horodatage(i64);

impl Horodatage {
    /// Construit un horodatage.
    pub const fn new(secondes: i64) -> Self {
        Self(secondes)
    }

    /// Secondes depuis l'époque.
    pub const fn secondes(&self) -> i64 {
        self.0
    }

    /// Ajoute une durée.
    ///
    /// Sature plutôt que de déborder : un dépassement d'entier produirait une
    /// date dans le passé, donc une expiration immédiate ou, pire, une
    /// expiration jamais atteinte.
    pub const fn plus(&self, duree: Duree) -> Self {
        Self(self.0.saturating_add(duree.0))
    }

    /// Vrai si l'instant est strictement postérieur à `autre`.
    pub const fn apres(&self, autre: Horodatage) -> bool {
        self.0 > autre.0
    }
}

impl fmt::Display for Horodatage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}s", self.0)
    }
}

/// Une durée, en secondes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Duree(i64);

impl Duree {
    /// Construit une durée strictement positive.
    ///
    /// Une durée nulle ou négative est refusée : un TTL de zéro seconde
    /// donnerait un bail déjà expiré à sa création, ce qui est une erreur de
    /// configuration et non une intention.
    pub fn secondes(valeur: i64) -> Result<Self, crate::domain::AppError> {
        if valeur <= 0 {
            return Err(crate::domain::AppError::Configuration {
                detail: format!(
                    "durée invalide : {valeur} seconde(s), attendu strictement positif"
                ),
            });
        }
        Ok(Self(valeur))
    }

    /// Construit une durée exprimée en jours.
    pub fn jours(valeur: i64) -> Result<Self, crate::domain::AppError> {
        Self::secondes(valeur.saturating_mul(86_400))
    }

    /// Valeur en secondes.
    pub const fn en_secondes(&self) -> i64 {
        self.0
    }
}

impl fmt::Display for Duree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}s", self.0)
    }
}
