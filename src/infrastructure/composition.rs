//! Composition des outils, à partir de la configuration.
//!
//! Un seul endroit décide de ce qu'un déploiement expose. C'est ce qui permet
//! aux deux Sluis de partager le même binaire : le public n'a pas de module
//! de bac à sable dans sa configuration, donc il ne peut pas enregistrer
//! l'outil de campagne, quand bien même il en aurait les identifiants.

use std::sync::Arc;

use crate::application::campagne::Campagne;
use crate::application::ports::Horloge;
use crate::configuration::Configuration;
use crate::domain::{AppError, Duree, PlafondDepense};
use crate::infrastructure::bac_a_sable::BacASableIac;
use crate::infrastructure::charge::Wrk;
use crate::infrastructure::derogation_depot::DepotDerogationFichier;
use crate::infrastructure::fs_charge::FsCharge;
use crate::infrastructure::github::PasserelleGithub;
use crate::infrastructure::mcp::outil_campagne::{OutilCampagne, ReglagesCampagne};
use crate::infrastructure::mcp::Outil;
use crate::infrastructure::process::{ExecuteurSysteme, Tofu};

/// Construit `sluis_campagne`, **si et seulement si** la configuration le
/// prévoit.
///
/// Trois conditions cumulatives, et l'absence de l'une n'est pas une panne :
/// au moins un dépôt autorisé, un projet de bac à sable déclaré, un secret de
/// signature pour sceller la fenêtre de dérogation. Un déploiement de lecture
/// n'en remplit aucune, et n'expose donc rien qui mute.
pub fn outil_campagne_si_configure(
    configuration: &Configuration,
    secret_signature: &[u8],
    horloge: Arc<dyn Horloge>,
) -> Result<Option<Box<dyn Outil>>, AppError> {
    if configuration
        .fichier
        .bac_a_sable
        .depots_autorises
        .is_empty()
    {
        return Ok(None);
    }
    let Some(nom_projet) = configuration
        .fichier
        .ovh
        .projets_bac_a_sable
        .first()
        .cloned()
    else {
        return Ok(None);
    };
    if secret_signature.is_empty() {
        return Ok(None);
    }

    let projet = configuration.autorisation.projet_bac_a_sable(&nom_projet)?;
    let bac = Arc::new(BacASableIac::new(Tofu::new(ExecuteurSysteme)));

    Ok(Some(Box::new(OutilCampagne::new(
        Arc::new(Campagne::new(Arc::new(Wrk::new(ExecuteurSysteme)))),
        Arc::new(FsCharge::new()),
        Arc::new(DepotDerogationFichier::nouveau(
            &configuration.fichier.bac_a_sable.depot_derogation,
            secret_signature,
        )),
        bac.clone(),
        bac,
        horloge,
        ReglagesCampagne {
            projet,
            ttl_maximal: Duree::secondes(configuration.fichier.bac_a_sable.ttl_maximal_secondes)?,
            plafond: PlafondDepense::new(configuration.fichier.bac_a_sable.plafond_depense)?,
            racines_autorisees: configuration.fichier.bac_a_sable.depots_autorises.clone(),
        },
    ))))
}

/// Ce que la composition a pu faire de la passerelle GitHub.
#[derive(Debug)]
pub enum EtatPasserelle {
    /// La section et le jeton sont présents : la passerelle est composée.
    Composee(PasserelleGithub),
    /// Pas de section `[github]` : le défaut sûr, ce n'est pas une panne.
    NonConfiguree,
    /// Une section `[github]` sans jeton : une intention de soumettre que le
    /// déploiement ne peut pas honorer, à dire plutôt qu'à taire.
    JetonManquant,
}

impl EtatPasserelle {
    /// Ligne de diagnostic, destinée au démarrage du serveur.
    pub fn diagnostic(&self) -> String {
        match self {
            EtatPasserelle::Composee(passerelle) => format!("passerelle GitHub : {passerelle:?}"),
            EtatPasserelle::NonConfiguree => "passerelle GitHub : non configurée".to_string(),
            EtatPasserelle::JetonManquant => {
                "passerelle GitHub : section [github] présente mais SLUIS_GITHUB_TOKEN absent \
                 ou vide, aucune soumission possible"
                    .to_string()
            }
        }
    }
}

/// Compose la passerelle d'approbation GitHub (ADR-008, ADR-012), **si et
/// seulement si** la configuration porte une section `[github]` et que le
/// jeton est présent.
pub fn passerelle_github_si_configuree(configuration: &Configuration) -> EtatPasserelle {
    let Some(section) = configuration.fichier.github.as_ref() else {
        return EtatPasserelle::NonConfiguree;
    };
    let Some(jeton) = configuration.jeton_github.clone() else {
        return EtatPasserelle::JetonManquant;
    };
    EtatPasserelle::Composee(PasserelleGithub::new(
        section.proprietaire.clone(),
        section.depot.clone(),
        section.workflow.clone(),
        section.branche.clone(),
        jeton,
        section.api.clone(),
    ))
}
