//! Preuve de convergence.
//!
//! `convergence-iac.md` définit l'évaluation de l'infrastructure ainsi : on
//! applique, on ré-applique, et **l'absence d'écart prouve l'idempotence**. La
//! condition de sortie n'est pas « ça a l'air d'avoir marché », c'est un
//! ré-apply sans diff.

use serde::Serialize;

use crate::domain::{AppError, PlanTerraform};

/// Résultat d'une tentative de convergence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PreuveConvergence {
    /// Nombre de tours joués.
    pub tours: u32,
    /// Vrai si un ré-apply n'a produit aucun écart.
    pub convergee: bool,
    /// Écart restant au dernier tour, s'il en reste.
    pub ecart_restant: Option<String>,
}

/// Établit la convergence à partir des plans successifs.
///
/// Échoue explicitement au-delà de `tours_maximum` plutôt que de boucler : une
/// infrastructure qui ne converge pas doit se signaler, pas occuper la machine
/// indéfiniment.
pub fn etablir(plans: &[PlanTerraform], tours_maximum: u32) -> Result<PreuveConvergence, AppError> {
    if plans.is_empty() {
        return Err(AppError::Configuration {
            detail: "aucun plan fourni : la convergence ne se déduit pas du vide".to_string(),
        });
    }
    if plans.len() as u32 > tours_maximum {
        return Err(AppError::Configuration {
            detail: format!(
                "convergence non atteinte après {tours_maximum} tour(s) : \
                 l'écart persiste, il faut corriger la déclaration"
            ),
        });
    }
    let dernier = &plans[plans.len() - 1];
    Ok(PreuveConvergence {
        tours: plans.len() as u32,
        convergee: dernier.sans_changement(),
        ecart_restant: if dernier.sans_changement() {
            None
        } else {
            Some(format!(
                "{} à créer, {} à modifier, {} à détruire",
                dernier.creations, dernier.modifications, dernier.destructions
            ))
        },
    })
}
