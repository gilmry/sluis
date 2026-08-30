//! Dépôt de la fenêtre de dérogation, sur fichier JSON scellé.
//!
//! Le fichier ne fait pas foi par lui-même : il porte un sceau, calculable
//! seulement avec le secret de signature du serveur. Sans cela, s'octroyer
//! une dérogation de Tier 2 se réduirait à écrire trois nombres dans un
//! fichier, ce qui viderait ADR-007 de son objet.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::application::ports::DepotDerogation;
use crate::domain::{AppError, FenetreDerogation, Horodatage};

/// Forme persistée d'une fenêtre.
#[derive(Debug, Serialize, Deserialize)]
struct FenetreEnregistree {
    ouverte_le: i64,
    close_le: i64,
    approbateur: String,
    sceau: String,
}

/// Dépôt sur fichier.
pub struct DepotDerogationFichier {
    chemin: PathBuf,
    secret: Vec<u8>,
}

impl DepotDerogationFichier {
    /// Construit le dépôt.
    pub fn nouveau(chemin: impl Into<PathBuf>, secret: &[u8]) -> Self {
        Self {
            chemin: chemin.into(),
            secret: secret.to_vec(),
        }
    }

    fn erreur_io(&self, detail: String) -> AppError {
        AppError::EntreeSortie {
            chemin: self.chemin.display().to_string(),
            detail,
        }
    }
}

impl DepotDerogation for DepotDerogationFichier {
    fn courante(&self) -> Result<Option<FenetreDerogation>, AppError> {
        if !Path::new(&self.chemin).exists() {
            // Fenêtre absente vaut fenêtre fermée : ce n'est pas une panne,
            // c'est le défaut sûr.
            return Ok(None);
        }
        let contenu =
            std::fs::read_to_string(&self.chemin).map_err(|e| self.erreur_io(e.to_string()))?;

        // Une corruption ne doit pas se lire comme une absence : elle mènerait
        // à un renouvellement de Tier 1 qui masquerait le problème.
        let enregistree: FenetreEnregistree =
            serde_json::from_str(&contenu).map_err(|e| AppError::Analyse {
                quoi: "fenêtre de dérogation".to_string(),
                detail: e.to_string(),
            })?;

        FenetreDerogation::restaurer(
            Horodatage::new(enregistree.ouverte_le),
            Horodatage::new(enregistree.close_le),
            enregistree.approbateur,
            &enregistree.sceau,
        )
        .ouvrir(&self.secret)
        .map(Some)
    }

    fn enregistrer(&self, fenetre: &FenetreDerogation) -> Result<(), AppError> {
        let enregistree = FenetreEnregistree {
            ouverte_le: fenetre.ouverte_le().secondes(),
            close_le: fenetre.close_le().secondes(),
            approbateur: fenetre.approbateur().to_string(),
            sceau: fenetre.sceau(&self.secret),
        };
        let contenu =
            serde_json::to_string_pretty(&enregistree).map_err(|e| AppError::Analyse {
                quoi: "fenêtre de dérogation".to_string(),
                detail: e.to_string(),
            })?;
        std::fs::write(&self.chemin, contenu).map_err(|e| self.erreur_io(e.to_string()))
    }
}
