//! Journal d'audit sur fichier JSONL.
//!
//! Une entrée, une ligne de JSON. Le format est choisi pour rester lisible par
//! `grep`, `jq` et un humain pressé pendant un incident, sans outil dédié.
//!
//! Deux garanties d'implémentation portent le reste :
//!
//! - Le fichier est ouvert en `append`, jamais en troncature. Une réouverture
//!   n'efface rien, ce que prouve un test `@security` dédié.
//! - L'écriture est sérialisée par un mutex et faite en un seul appel. Sans
//!   cela, deux écritures concurrentes s'entrelacent et produisent des lignes
//!   de JSON invalides, ce qui rend le journal inexploitable exactement au
//!   moment où l'on en a besoin.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

use crate::application::ports::AuditLog;
use crate::domain::{AppError, AuditEntry};

/// Journal append-only sur fichier JSONL.
///
/// `Debug` ne révèle que le chemin et le descripteur : aucune entrée, donc
/// aucun secret, ne transite par ce trait.
#[derive(Debug)]
pub struct JsonlAuditLog {
    fichier: Mutex<std::fs::File>,
    chemin: String,
}

impl JsonlAuditLog {
    /// Ouvre ou crée le journal.
    ///
    /// Échoue si le fichier n'est pas inscriptible. C'est voulu : un appel qui
    /// ne peut pas être tracé ne doit pas s'exécuter, sans quoi le journal
    /// donne une image fausse de ce qui s'est produit.
    pub fn new(chemin: &Path) -> Result<Self, AppError> {
        let fichier = OpenOptions::new()
            .create(true)
            .append(true)
            .open(chemin)
            .map_err(|e| AppError::EntreeSortie {
                chemin: chemin.display().to_string(),
                detail: e.to_string(),
            })?;
        Ok(Self {
            fichier: Mutex::new(fichier),
            chemin: chemin.display().to_string(),
        })
    }

    /// Chemin du journal, pour le diagnostic.
    pub fn chemin(&self) -> &str {
        &self.chemin
    }
}

impl AuditLog for JsonlAuditLog {
    fn append(&self, entree: &AuditEntry) -> Result<(), AppError> {
        // `serde_json::to_string` échappe les sauts de ligne du contenu : une
        // entrée reste donc sur une ligne, et ne peut pas se faire passer pour
        // plusieurs en injectant un `\n` dans un message d'erreur.
        let mut ligne = serde_json::to_string(entree).map_err(|e| AppError::Analyse {
            quoi: "entrée de journal".to_string(),
            detail: e.to_string(),
        })?;
        ligne.push('\n');

        let mut fichier = self.fichier.lock().map_err(|_| AppError::EntreeSortie {
            chemin: self.chemin.clone(),
            detail: "verrou du journal empoisonné".to_string(),
        })?;

        // Une seule écriture, sous verrou : c'est ce qui empêche l'entrelacement.
        fichier
            .write_all(ligne.as_bytes())
            .map_err(|e| AppError::EntreeSortie {
                chemin: self.chemin.clone(),
                detail: e.to_string(),
            })?;
        fichier.flush().map_err(|e| AppError::EntreeSortie {
            chemin: self.chemin.clone(),
            detail: e.to_string(),
        })
    }
}
