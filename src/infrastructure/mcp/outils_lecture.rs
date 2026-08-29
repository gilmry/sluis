//! Les outils de lecture — Tier 2, sans confirmation.

use std::sync::Arc;

use schemars::JsonSchema;
use serde::Deserialize;

use crate::application::ports::{DepotInventaire, Diagnostic};
use crate::domain::{AppError, Tier};
use crate::infrastructure::mcp::{ContratOutil, Outil};

/// `sluis_doctor` — aucun argument.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArgumentsDoctor {}

/// Rend l'état des moteurs et des identifiants.
pub struct OutilDoctor {
    diagnostic: Arc<dyn Diagnostic>,
}

impl OutilDoctor {
    /// Construit l'outil.
    pub fn new(diagnostic: Arc<dyn Diagnostic>) -> Self {
        Self { diagnostic }
    }
}

impl ContratOutil for OutilDoctor {
    fn nom(&self) -> &'static str {
        "sluis_doctor"
    }
    fn description(&self) -> &'static str {
        "Rend l'état des six moteurs d'infrastructure et la présence des \
         identifiants OVH. Ne révèle jamais la valeur d'un identifiant. \
         Une absence est un état normal, pas une panne."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(ArgumentsDoctor))
            .unwrap_or_else(|_| serde_json::json!({"type": "object"}))
    }
    fn desérialiser(&self, arguments: &serde_json::Value) -> Result<(), String> {
        serde_json::from_value::<ArgumentsDoctor>(arguments.clone())
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

impl Outil for OutilDoctor {
    fn tier(&self) -> Tier {
        Tier::Two
    }
    fn appeler(&self, arguments: &serde_json::Value) -> Result<serde_json::Value, AppError> {
        serde_json::from_value::<ArgumentsDoctor>(arguments.clone()).map_err(|e| {
            AppError::Analyse {
                quoi: "arguments de sluis_doctor".to_string(),
                detail: e.to_string(),
            }
        })?;
        let rapport = self.diagnostic.etablir()?;
        serde_json::to_value(rapport).map_err(|e| AppError::Analyse {
            quoi: "rapport de diagnostic".to_string(),
            detail: e.to_string(),
        })
    }
}

/// `sluis_inventory` — un chemin de dépôt.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArgumentsInventaire {
    /// Chemin du dossier d'infrastructure à inspecter.
    pub racine: String,
}

/// Découvre la matrice d'infrastructure déclarée d'un dépôt.
pub struct OutilInventaire {
    depot: Arc<dyn DepotInventaire>,
}

impl OutilInventaire {
    /// Construit l'outil.
    pub fn new(depot: Arc<dyn DepotInventaire>) -> Self {
        Self { depot }
    }
}

impl ContratOutil for OutilInventaire {
    fn nom(&self) -> &'static str {
        "sluis_inventory"
    }
    fn description(&self) -> &'static str {
        "Découvre la matrice topologies × environnements d'un dépôt, ainsi que \
         ses profils de cluster et ses modules Terraform. Les noms non reconnus \
         sont signalés plutôt que tus."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(ArgumentsInventaire))
            .unwrap_or_else(|_| serde_json::json!({"type": "object"}))
    }
    fn desérialiser(&self, arguments: &serde_json::Value) -> Result<(), String> {
        serde_json::from_value::<ArgumentsInventaire>(arguments.clone())
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

impl Outil for OutilInventaire {
    fn tier(&self) -> Tier {
        Tier::Two
    }
    fn appeler(&self, arguments: &serde_json::Value) -> Result<serde_json::Value, AppError> {
        let args: ArgumentsInventaire =
            serde_json::from_value(arguments.clone()).map_err(|e| AppError::Analyse {
                quoi: "arguments de sluis_inventory".to_string(),
                detail: e.to_string(),
            })?;
        let matrice = self.depot.decouvrir_matrice(&args.racine)?;
        serde_json::to_value(matrice).map_err(|e| AppError::Analyse {
            quoi: "matrice".to_string(),
            detail: e.to_string(),
        })
    }
}

/// `sluis_cluster_profiles` — un chemin de dépôt.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArgumentsProfils {
    /// Chemin du dossier d'infrastructure.
    pub racine: String,
}

/// Décrit les profils de cluster et leur contrat Day 1 / Day 2.
pub struct OutilProfils {
    depot: Arc<dyn DepotInventaire>,
}

impl OutilProfils {
    /// Construit l'outil.
    pub fn new(depot: Arc<dyn DepotInventaire>) -> Self {
        Self { depot }
    }
}

impl ContratOutil for OutilProfils {
    fn nom(&self) -> &'static str {
        "sluis_cluster_profiles"
    }
    fn description(&self) -> &'static str {
        "Décrit les profils de cluster : classe de stockage, ingress, TLS, \
         backend de secrets, préréglage de ressources. C'est le contrat entre \
         provisionnement et déploiement."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(ArgumentsProfils))
            .unwrap_or_else(|_| serde_json::json!({"type": "object"}))
    }
    fn desérialiser(&self, arguments: &serde_json::Value) -> Result<(), String> {
        serde_json::from_value::<ArgumentsProfils>(arguments.clone())
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

impl Outil for OutilProfils {
    fn tier(&self) -> Tier {
        Tier::Two
    }
    fn appeler(&self, arguments: &serde_json::Value) -> Result<serde_json::Value, AppError> {
        let args: ArgumentsProfils =
            serde_json::from_value(arguments.clone()).map_err(|e| AppError::Analyse {
                quoi: "arguments de sluis_cluster_profiles".to_string(),
                detail: e.to_string(),
            })?;
        let profils = self.depot.lire_profils(&args.racine)?;
        serde_json::to_value(profils).map_err(|e| AppError::Analyse {
            quoi: "profils".to_string(),
            detail: e.to_string(),
        })
    }
}
