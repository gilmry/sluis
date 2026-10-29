//! Port de la barrière de déploiement (ADR-021).

use crate::domain::{ControleBarriere, VerdictControle};

/// Évalue les contrôles de la barrière.
///
/// **Ne rend jamais d'erreur** : un contrôle qui ne peut pas s'évaluer est un
/// verdict rouge, pas une panne à remonter. Une erreur laisserait à l'appelant
/// le choix de l'ignorer ; un rouge ne lui laisse rien.
pub trait EvaluateurBarriere: Send + Sync {
    /// Évalue chaque contrôle, dans l'ordre, et rend un verdict par contrôle.
    fn evaluer(&self, controles: &[ControleBarriere]) -> Vec<VerdictControle>;
}
