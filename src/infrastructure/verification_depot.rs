//! Vérification de structure d'un dépôt cible (ADR-034, point 7).
//!
//! Chaque règle passe par **le lecteur que Sluis utilise à l'exécution** :
//! `FsCharge`, `FsInventaire`, la comparaison de section d'ADR-034 point 6.
//! Un verdict qui réimplémenterait les règles pourrait dire conforme un dépôt
//! que Sluis refusera.
//!
//! Les modules OpenTofu ne sont pas encore jugés ici (story 34.6b).

use std::path::Path;

use crate::application::ports::{DepotCharge, DepotInventaire};
use crate::domain::contrat_depot::{RACINE_INFRASTRUCTURE, WORKFLOW_TIER1};
use crate::domain::section_empreinte::{verifier_section, GABARIT_TIER1};
use crate::domain::{AppError, Ecart, VerdictDepot};
use crate::infrastructure::fs_charge::FsCharge;
use crate::infrastructure::fs_inventaire::FsInventaire;

/// Taille maximale du workflow lu.
const TAILLE_MAXIMALE_WORKFLOW: u64 = 1024 * 1024;

/// Établit le verdict du dépôt dont `racine` est la racine.
///
/// Rend une erreur seulement si la racine elle-même est illisible ; tout le
/// reste est un écart du verdict.
pub fn verifier_depot(racine: &str) -> Result<VerdictDepot, AppError> {
    let racine = FsInventaire::canoniser_racine(racine)?;
    let mut ecarts = Vec::new();
    let mut remarques = Vec::new();

    verifier_workflow(&racine, &mut ecarts);

    let infrastructure = racine.join(RACINE_INFRASTRUCTURE);
    if !infrastructure.is_dir() || !FsInventaire::dans_racine(&racine, &infrastructure) {
        ecarts.push(Ecart {
            regle: "arborescence",
            chemin: format!("{RACINE_INFRASTRUCTURE}/"),
            detail: "dossier absent".to_string(),
        });
        return Ok(VerdictDepot::new(ecarts, remarques));
    }
    let infra = infrastructure.to_string_lossy().to_string();

    if let Err(erreur) = FsCharge::new().lire(&infra) {
        ecarts.push(Ecart {
            regle: "charge",
            chemin: format!("{RACINE_INFRASTRUCTURE}/_shared/charge.yaml"),
            detail: erreur.to_string(),
        });
    }

    match FsInventaire::new().decouvrir_matrice(&infra) {
        Ok(matrice) => {
            if matrice.cellules.is_empty() {
                ecarts.push(Ecart {
                    regle: "cellules",
                    chemin: format!("{RACINE_INFRASTRUCTURE}/"),
                    detail: "aucune cellule <regroupement>/<topologie>/<environnement>/ reconnue"
                        .to_string(),
                });
            }
            remarques.extend(
                matrice
                    .ignores
                    .into_iter()
                    .map(|nom| format!("dossier non reconnu par l'inventaire : {nom}")),
            );
        }
        Err(erreur) => ecarts.push(Ecart {
            regle: "profils",
            chemin: format!("{RACINE_INFRASTRUCTURE}/_shared/cluster-profiles/"),
            detail: erreur.to_string(),
        }),
    }

    Ok(VerdictDepot::new(ecarts, remarques))
}

fn verifier_workflow(racine: &Path, ecarts: &mut Vec<Ecart>) {
    let chemin = racine.join(WORKFLOW_TIER1);
    let ecart = |detail: String| Ecart {
        regle: "workflow-tier1",
        chemin: WORKFLOW_TIER1.to_string(),
        detail,
    };
    if !chemin.is_file() {
        ecarts.push(ecart("absent".to_string()));
        return;
    }
    if !FsInventaire::dans_racine(racine, &chemin) {
        ecarts.push(ecart("lien qui sort du dépôt".to_string()));
        return;
    }
    match std::fs::metadata(&chemin) {
        Ok(meta) if meta.len() <= TAILLE_MAXIMALE_WORKFLOW => {}
        _ => {
            ecarts.push(ecart("illisible ou trop gros".to_string()));
            return;
        }
    }
    match std::fs::read_to_string(&chemin) {
        Ok(contenu) => {
            crate::infrastructure::lectures::consigner(&chemin, &contenu);
            if let Err(raison) = verifier_section(GABARIT_TIER1, &contenu) {
                ecarts.push(ecart(format!(
                    "section verification-empreinte non conforme au gabarit : {raison}"
                )));
            }
        }
        Err(e) => ecarts.push(ecart(format!("illisible : {e}"))),
    }
}
