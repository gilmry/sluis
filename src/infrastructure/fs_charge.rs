//! Découverte de `_shared/charge.yaml` dans un dépôt.
//!
//! Le fichier est lu par le même analyseur YAML restreint que les profils de
//! cluster : le contrat est court et connu, une bibliothèque généraliste
//! apporterait un arbre de dépendances hors de proportion.

use std::path::Path;

use crate::application::ports::DepotCharge;
use crate::domain::contrat_depot::{cles_charge as cle, FICHIER_CHARGE};
use crate::domain::{AppError, CibleCapacite, DeclarationCharge, Duree, PlafondDepense};

/// Adaptateur de système de fichiers.
#[derive(Debug, Default, Clone, Copy)]
pub struct FsCharge;

impl FsCharge {
    /// Construit l'adaptateur.
    pub fn new() -> Self {
        Self
    }
}

fn requise(
    cles: &std::collections::BTreeMap<String, String>,
    nom: &str,
) -> Result<String, AppError> {
    cles.get(nom)
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| AppError::Configuration {
            detail: format!(
                "déclaration de charge incomplète : « {nom} » est absent de _shared/charge.yaml"
            ),
        })
}

fn nombre(cles: &std::collections::BTreeMap<String, String>, nom: &str) -> Result<f64, AppError> {
    requise(cles, nom)?
        .parse::<f64>()
        .map_err(|_| AppError::Analyse {
            quoi: format!("_shared/charge.yaml, clé « {nom} »"),
            detail: "valeur non numérique".to_string(),
        })
}

impl DepotCharge for FsCharge {
    fn lire(&self, racine: &str) -> Result<DeclarationCharge, AppError> {
        let racine = Path::new(racine)
            .canonicalize()
            .map_err(|e| AppError::EntreeSortie {
                chemin: racine.to_string(),
                detail: e.to_string(),
            })?;
        let chemin = FICHIER_CHARGE
            .iter()
            .fold(racine, |chemin, segment| chemin.join(segment));
        if !chemin.is_file() {
            return Err(AppError::Configuration {
                detail: format!(
                    "ce dépôt ne déclare pas de charge : « {} » est absent, donc le projet \
                     n'est pas mesurable en l'état",
                    chemin.display()
                ),
            });
        }

        let contenu = std::fs::read_to_string(&chemin).map_err(|e| AppError::EntreeSortie {
            chemin: chemin.display().to_string(),
            detail: e.to_string(),
        })?;
        crate::infrastructure::lectures::consigner(&chemin, &contenu);
        let cles = crate::infrastructure::yaml_plat::aplatir(&contenu).map_err(|detail| {
            AppError::Analyse {
                quoi: chemin.display().to_string(),
                detail,
            }
        })?;

        DeclarationCharge::new(
            requise(&cles, cle::TOPOLOGIE)?,
            requise(&cles, cle::MODULE)?,
            requise(&cles, cle::SORTIE_ADRESSE)?,
            requise(&cles, cle::CHEMIN)?,
            CibleCapacite::new(
                nombre(&cles, cle::CIBLE_RPS)?,
                nombre(&cles, cle::CIBLE_P99)?,
            )?,
            Duree::secondes(nombre(&cles, cle::BORNE_TTL)? as i64)?,
            PlafondDepense::new(nombre(&cles, cle::BORNE_PLAFOND)?)?,
        )
    }
}
