//! Composition des outils, à partir de la configuration.
//!
//! Un seul endroit décide de ce qu'un déploiement expose. C'est ce qui permet
//! aux deux Sluis de partager le même binaire : le public n'a pas de module
//! de bac à sable dans sa configuration, donc il ne peut pas enregistrer
//! l'outil de campagne, quand bien même il en aurait les identifiants.

use std::sync::Arc;

use crate::application::campagne::Campagne;
use crate::application::mise_en_ligne::MiseEnLigne;
use crate::application::ports::{GatePlancher, Horloge};
use crate::configuration::Configuration;
use crate::domain::{AppError, Duree, PlafondDepense};
use crate::infrastructure::bac_a_sable::BacASableIac;
use crate::infrastructure::charge::Wrk;
use crate::infrastructure::derogation_depot::DepotDerogationFichier;
use crate::infrastructure::fs_charge::FsCharge;
use crate::infrastructure::gates_github::VerificateurGatesGithub;
use crate::infrastructure::github::PasserelleGithub;
use crate::infrastructure::mcp::outil_campagne::{OutilCampagne, ReglagesCampagne};
use crate::infrastructure::mcp::outils_mise_en_ligne::{
    OutilEtatApprobation, OutilMiseEnLigne, OutilRenouvellementDerogation,
};
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

/// Compose le vérificateur des gates du plancher sur les check-runs GitHub,
/// **si et seulement si** la section `[github]`, sa sous-section `gates` et le
/// jeton sont présents. Sinon, aucune mise en ligne ne peut être demandée.
pub fn verificateur_gates_si_configure(
    configuration: &Configuration,
) -> Option<VerificateurGatesGithub> {
    let section = configuration.fichier.github.as_ref()?;
    let gates = section.gates.as_ref()?;
    let jeton = configuration.jeton_github.clone()?;

    let mut correspondance = vec![
        (GatePlancher::Secrets, gates.secrets.clone()),
        (GatePlancher::Sbom, gates.sbom.clone()),
        (GatePlancher::ScanConteneur, gates.scan_conteneur.clone()),
    ];
    if let Some(nom) = &gates.retour_migration {
        correspondance.push((GatePlancher::RetourMigration, nom.clone()));
    }

    Some(VerificateurGatesGithub::new(
        section.proprietaire.clone(),
        section.depot.clone(),
        section.api.clone(),
        jeton,
        correspondance,
    ))
}

/// Construit les outils qui consomment la passerelle (ADR-012).
///
/// - `sluis_etat_approbation` et `sluis_renouvellement_derogation` dès que la
///   passerelle est composée : ils ne dépendent d'aucune gate.
/// - `sluis_mise_en_ligne` **seulement si** le vérificateur des gates l'est
///   aussi : une demande dont le plancher ne serait pas vérifié ne doit pas
///   pouvoir être soumise.
pub fn outils_mise_en_ligne_si_configures(
    configuration: &Configuration,
    passerelle: EtatPasserelle,
    horloge: Arc<dyn Horloge>,
) -> Vec<Box<dyn Outil>> {
    let EtatPasserelle::Composee(passerelle) = passerelle else {
        return Vec::new();
    };
    let passerelle: Arc<dyn crate::application::ports::PasserelleApprobation> =
        Arc::new(passerelle);

    let mut outils: Vec<Box<dyn Outil>> = Vec::new();
    if let (Some(verificateur), Some(section)) = (
        verificateur_gates_si_configure(configuration),
        configuration.fichier.github.as_ref(),
    ) {
        outils.push(Box::new(OutilMiseEnLigne::new(
            MiseEnLigne::new(Arc::new(verificateur), passerelle.clone()),
            section.depot.clone(),
        )));
    }
    outils.push(Box::new(OutilEtatApprobation::new(passerelle.clone())));
    outils.push(Box::new(OutilRenouvellementDerogation::new(
        passerelle, horloge,
    )));
    outils
}
