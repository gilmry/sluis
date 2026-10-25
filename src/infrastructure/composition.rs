//! Composition des outils, à partir de la configuration.
//!
//! Un seul endroit décide de ce qu'un déploiement expose. C'est ce qui permet
//! aux deux Sluis de partager le même binaire : le public n'a pas de module
//! de bac à sable dans sa configuration, donc il ne peut pas enregistrer
//! l'outil de campagne, quand bien même il en aurait les identifiants.

use std::sync::Arc;

use crate::application::campagne::Campagne;
use crate::application::mise_en_ligne::MiseEnLigne;
use crate::application::ouverture_derogation::OuvertureDerogation;
use crate::application::ports::{GatePlancher, Horloge};
use crate::configuration::{ArchetypeGates, Configuration, TypePasserelle};
use crate::domain::contrat_depot::check_runs;
use crate::domain::{AppError, Duree, PlafondDepense};
use crate::infrastructure::bac_a_sable::BacASableIac;
use crate::infrastructure::charge::Wrk;
use crate::infrastructure::derogation_depot::DepotDerogationFichier;
use crate::infrastructure::etiquette_signee::PasserelleEtiquetteSignee;
use crate::infrastructure::fs_charge::FsCharge;
use crate::infrastructure::fs_module_tofu::FsModuleTofu;
use crate::infrastructure::gates_github::VerificateurGatesGithub;
use crate::infrastructure::github::PasserelleGithub;
use crate::infrastructure::mcp::outil_campagne::{OutilCampagne, ReglagesCampagne};
use crate::infrastructure::mcp::outil_ouverture_derogation::OutilOuvertureDerogation;
use crate::infrastructure::mcp::outils_mise_en_ligne::{
    OutilEtatApprobation, OutilMiseEnLigne, OutilRenouvellementDerogation,
};
use crate::infrastructure::mcp::Outil;
use crate::infrastructure::process::{ExecuteurSysteme, Tofu};
use crate::infrastructure::renouvellements_depot::RegistreRenouvellementsFichier;

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
    // Le lecteur de modules ne connaît que les dépôts autorisés : un module
    // qui n'en descend pas est refusé avant d'être lu.
    let bac = Arc::new(BacASableIac::new(
        Tofu::new(
            ExecuteurSysteme,
            FsModuleTofu::new(configuration.fichier.bac_a_sable.depots_autorises.clone()),
            configuration.fichier.tofu.fournisseurs_autorises.clone(),
        )
        .avec_variables_transmises(configuration.fichier.tofu.variables_transmises.clone()),
    ));

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
    /// La section et le jeton sont présents : la passerelle d'ADR-008 est
    /// composée.
    Composee(PasserelleGithub),
    /// La section demande `passerelle = "etiquette-signee"` et le jeton est
    /// présent : la passerelle d'ADR-013 est composée. Elle ne porte que la
    /// mise en ligne.
    ComposeeEtiquette(PasserelleEtiquetteSignee),
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
            EtatPasserelle::ComposeeEtiquette(passerelle) => {
                format!("passerelle GitHub par étiquette signée : {passerelle:?}")
            }
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
///
/// `passerelle = "etiquette-signee"` choisit celle d'ADR-013 ; sans ce
/// réglage, c'est celle d'ADR-008, inchangée.
pub fn passerelle_github_si_configuree(configuration: &Configuration) -> EtatPasserelle {
    let Some(section) = configuration.fichier.github.as_ref() else {
        return EtatPasserelle::NonConfiguree;
    };
    let Some(jeton) = configuration.jeton_github.clone() else {
        return EtatPasserelle::JetonManquant;
    };
    if section.passerelle == TypePasserelle::EtiquetteSignee {
        return EtatPasserelle::ComposeeEtiquette(PasserelleEtiquetteSignee::new(
            section.proprietaire.clone(),
            section.depot.clone(),
            jeton,
            section.api.clone(),
        ));
    }
    EtatPasserelle::Composee(PasserelleGithub::new(
        section.proprietaire.clone(),
        section.depot.clone(),
        // Garanti au chargement pour cette passerelle.
        section.workflow.clone().unwrap_or_default(),
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

    // Les noms viennent du contrat du dépôt cible, jamais de la configuration.
    let mut correspondance = vec![
        (GatePlancher::Secrets, check_runs::SECRETS.to_string()),
        (GatePlancher::Sbom, check_runs::SBOM.to_string()),
        (
            GatePlancher::ScanConteneur,
            check_runs::SCAN_CONTENEUR.to_string(),
        ),
    ];
    if gates.archetype == ArchetypeGates::Stateful {
        correspondance.push((
            GatePlancher::RetourMigration,
            check_runs::RETOUR_MIGRATION.to_string(),
        ));
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
    // L'exécutant ne demande pas, il ouvre (ADR-012, partie 3) : un serveur
    // qui arme un bac à sable n'expose aucun outil de soumission.
    if !configuration
        .fichier
        .bac_a_sable
        .depots_autorises
        .is_empty()
    {
        return Vec::new();
    }
    // Le renouvellement de la dérogation n'emprunte que la passerelle
    // d'ADR-008 : l'exécutant en lit la preuve dans les approbations d'un run.
    let (passerelle, renouvelable): (
        Arc<dyn crate::application::ports::PasserelleApprobation>,
        bool,
    ) = match passerelle {
        EtatPasserelle::Composee(passerelle) => (Arc::new(passerelle), true),
        EtatPasserelle::ComposeeEtiquette(passerelle) => (Arc::new(passerelle), false),
        EtatPasserelle::NonConfiguree | EtatPasserelle::JetonManquant => return Vec::new(),
    };

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
    if renouvelable {
        outils.push(Box::new(OutilRenouvellementDerogation::new(
            passerelle, horloge,
        )));
    }
    outils
}

/// Construit `sluis_ouvrir_derogation` (ADR-012, partie 3), **si et seulement
/// si** le serveur arme un bac à sable (dépôt autorisé, projet dédié, secret de
/// signature) **et** dispose d'une passerelle pour lire l'approbation chez
/// GitHub. C'est donc un outil de l'exécutant, jamais du Sluis public.
pub fn outil_ouverture_derogation_si_configure(
    configuration: &Configuration,
    secret_signature: &[u8],
    passerelle: EtatPasserelle,
    horloge: Arc<dyn Horloge>,
) -> Result<Option<Box<dyn Outil>>, AppError> {
    let section = &configuration.fichier.bac_a_sable;
    if section.depots_autorises.is_empty()
        || configuration.fichier.ovh.projets_bac_a_sable.is_empty()
        || secret_signature.is_empty()
    {
        return Ok(None);
    }
    let EtatPasserelle::Composee(passerelle) = passerelle else {
        return Ok(None);
    };

    let cas = OuvertureDerogation::new(
        Arc::new(passerelle),
        Arc::new(RegistreRenouvellementsFichier::nouveau(
            &section.registre_renouvellements,
        )),
        Arc::new(DepotDerogationFichier::nouveau(
            &section.depot_derogation,
            secret_signature,
        )),
        horloge,
        configuration.fenetre_derogation()?,
    );
    Ok(Some(Box::new(OutilOuvertureDerogation::new(Arc::new(cas)))))
}
