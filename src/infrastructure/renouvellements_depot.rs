//! Registre des renouvellements consommés, sur fichier JSON.
//!
//! Une liste d'empreintes, réécrite d'un bloc par renommage : un arrêt au
//! milieu d'une écriture laisse l'ancienne liste, jamais une liste tronquée.
//! Un fichier illisible est une erreur, pas une liste vide : le lire comme
//! vide rouvrirait la porte à chaque approbation déjà servie.

use std::path::PathBuf;

use crate::application::ports::RegistreRenouvellements;
use crate::domain::{AppError, Empreinte};

/// Registre sur fichier.
pub struct RegistreRenouvellementsFichier {
    chemin: PathBuf,
}

impl RegistreRenouvellementsFichier {
    /// Construit le registre.
    pub fn nouveau(chemin: impl Into<PathBuf>) -> Self {
        Self {
            chemin: chemin.into(),
        }
    }

    fn erreur_io(&self, detail: String) -> AppError {
        AppError::EntreeSortie {
            chemin: self.chemin.display().to_string(),
            detail,
        }
    }

    fn lire(&self) -> Result<Vec<String>, AppError> {
        if !self.chemin.exists() {
            return Ok(Vec::new());
        }
        let contenu =
            std::fs::read_to_string(&self.chemin).map_err(|e| self.erreur_io(e.to_string()))?;
        serde_json::from_str(&contenu).map_err(|e| AppError::Analyse {
            quoi: "registre des renouvellements".to_string(),
            detail: e.to_string(),
        })
    }
}

impl RegistreRenouvellements for RegistreRenouvellementsFichier {
    fn deja_consommee(&self, empreinte: &Empreinte) -> Result<bool, AppError> {
        Ok(self.lire()?.iter().any(|e| e == empreinte.hexadecimal()))
    }

    fn consigner(&self, empreinte: &Empreinte) -> Result<(), AppError> {
        let mut empreintes = self.lire()?;
        if !empreintes.iter().any(|e| e == empreinte.hexadecimal()) {
            empreintes.push(empreinte.hexadecimal().to_string());
        }
        let contenu = serde_json::to_string_pretty(&empreintes).map_err(|e| AppError::Analyse {
            quoi: "registre des renouvellements".to_string(),
            detail: e.to_string(),
        })?;
        let provisoire = self.chemin.with_extension("json.provisoire");
        std::fs::write(&provisoire, contenu).map_err(|e| self.erreur_io(e.to_string()))?;
        std::fs::rename(&provisoire, &self.chemin).map_err(|e| self.erreur_io(e.to_string()))
    }
}
