//! Le verdict de conformité d'un dépôt cible (ADR-034, point 7).
//!
//! Rendu par l'outil `sluis_verifier_depot` : la skill Foyer vérifie son
//! travail avec les règles que Sluis appliquera lui-même. Un écart bloque
//! l'admission ; une remarque signale sans bloquer.

use serde::Serialize;

use crate::domain::VERSION_CONTRAT_DEPOT;

/// Un manquement au contrat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Ecart {
    /// Règle du contrat, par exemple `charge` ou `workflow-tier1`.
    pub regle: &'static str,
    /// Chemin concerné, relatif à la racine du dépôt.
    pub chemin: String,
    /// Ce qui manque ou diffère.
    pub detail: String,
}

/// Le verdict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VerdictDepot {
    /// Version du contrat appliquée.
    pub version_contrat: u32,
    /// Vrai si et seulement si aucun écart.
    pub conforme: bool,
    /// Manquements bloquants.
    pub ecarts: Vec<Ecart>,
    /// Signalements non bloquants.
    pub remarques: Vec<String>,
}

impl VerdictDepot {
    /// Établit le verdict. `conforme` est dérivé, jamais fourni : un verdict
    /// conforme avec des écarts est inexprimable.
    pub fn new(ecarts: Vec<Ecart>, remarques: Vec<String>) -> Self {
        Self {
            version_contrat: VERSION_CONTRAT_DEPOT,
            conforme: ecarts.is_empty(),
            ecarts,
            remarques,
        }
    }
}
