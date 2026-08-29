//! Port de la passerelle d'approbation.

use crate::domain::{AppError, Empreinte, PlanChangement};

/// Où en est une demande d'approbation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EtatApprobation {
    /// Soumise, en attente d'un relecteur.
    EnAttente {
        /// Identifiant du run, pour le suivi.
        run: String,
        /// URL où l'humain approuve.
        url: String,
    },
    /// Approuvée et exécutée.
    Approuvee {
        /// Qui a approuvé.
        approbateur: String,
        /// Identifiant du run.
        run: String,
    },
    /// Refusée par un relecteur, ou annulée.
    Refusee {
        /// Motif rapporté.
        motif: String,
    },
    /// Exécutée mais en échec.
    Echouee {
        /// Détail de l'échec.
        detail: String,
    },
}

/// Soumet une mutation à validation humaine.
///
/// **Sluis ne détient jamais les secrets de mutation de production.** Cette
/// propriété n'est pas une règle de conduite, elle est structurelle : la
/// passerelle ne fait que déclencher un travail qui, lui, détient les
/// identifiants. Un Sluis compromis ne peut donc que demander, jamais muter.
pub trait PasserelleApprobation: Send + Sync {
    /// Soumet un plan et rend l'état initial.
    fn soumettre(&self, plan: &PlanChangement) -> Result<EtatApprobation, AppError>;

    /// Interroge l'état d'une demande.
    fn interroger(&self, empreinte: &Empreinte) -> Result<EtatApprobation, AppError>;
}
