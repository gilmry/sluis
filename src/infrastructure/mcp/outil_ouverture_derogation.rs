//! `sluis_ouvrir_derogation` — l'exécutant ouvre la fenêtre d'ADR-007.
//!
//! N'existe que sur le Sluis exécutant, celui qui détient le secret de
//! signature de la fenêtre. Il ne reçoit qu'un instant de demande, rendu par
//! `sluis_renouvellement_derogation` sur le Sluis public : tout le reste, plan,
//! empreinte, approbation, il le reconstruit ou le lit lui-même.

use std::sync::Arc;

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use crate::application::ouverture_derogation::OuvertureDerogation;
use crate::domain::{AppError, Horodatage, Tier};
use crate::infrastructure::mcp::contrat::Contrat;
use crate::infrastructure::mcp::{ContratOutil, Outil};

/// `sluis_ouvrir_derogation` — arguments.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArgumentsOuverture {
    /// Instant de la demande, en secondes Unix, tel que rendu par
    /// `sluis_renouvellement_derogation`.
    pub demande_le: i64,
}

const CONTRAT: Contrat<ArgumentsOuverture> = Contrat::new(
    "sluis_ouvrir_derogation",
    "Ouvre la fenêtre de dérogation d'ADR-007 si la demande de renouvellement \
     faite à l'instant donné a été approuvée dans l'environnement protégé, il y a \
     moins de 72 h, et n'a jamais servi. Lit l'approbation chez GitHub ; ne croit \
     personne sur parole.",
);

/// Ouvre la fenêtre sur preuve.
pub struct OutilOuvertureDerogation {
    cas: Arc<OuvertureDerogation>,
}

impl OutilOuvertureDerogation {
    /// Construit l'outil.
    pub fn new(cas: Arc<OuvertureDerogation>) -> Self {
        Self { cas }
    }
}

impl ContratOutil for OutilOuvertureDerogation {
    fn nom(&self) -> &'static str {
        CONTRAT.nom()
    }
    fn description(&self) -> &'static str {
        CONTRAT.description()
    }
    fn schema(&self) -> serde_json::Value {
        CONTRAT.schema()
    }
    fn desérialiser(&self, arguments: &serde_json::Value) -> Result<(), String> {
        CONTRAT.desérialiser(arguments)
    }
}

impl Outil for OutilOuvertureDerogation {
    /// Tier 2 : l'outil n'ouvre rien sans la preuve d'une approbation de
    /// Tier 1, qu'il lit lui-même. L'autorité de déléguer reste humaine.
    fn tier(&self) -> Tier {
        Tier::Two
    }

    fn appeler(&self, arguments: &serde_json::Value) -> Result<serde_json::Value, AppError> {
        let arguments: ArgumentsOuverture =
            serde_json::from_value(arguments.clone()).map_err(|e| AppError::Analyse {
                quoi: "arguments de sluis_ouvrir_derogation".to_string(),
                detail: e.to_string(),
            })?;
        let fenetre = self.cas.ouvrir(Horodatage::new(arguments.demande_le))?;
        Ok(json!({
            "ouverte_le": fenetre.ouverte_le().secondes(),
            "close_le": fenetre.close_le().secondes(),
            "approbateur": fenetre.approbateur(),
        }))
    }
}
