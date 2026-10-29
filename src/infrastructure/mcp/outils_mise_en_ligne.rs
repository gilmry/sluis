//! Les outils de mise en ligne (ADR-012).
//!
//! `sluis_mise_en_ligne` **ne met rien en ligne** : il vérifie le plancher,
//! produit un plan de Tier 1 et le soumet à la passerelle. La mutation n'a lieu
//! qu'après approbation humaine dans un environnement GitHub protégé, par un
//! workflow qui détient des secrets que Sluis n'a pas. C'est pourquoi l'outil
//! lui-même est de Tier 2 : son acte est une demande refusable.
//!
//! **Aucun texte libre ne traverse.** Le projet vient de la configuration,
//! l'environnement d'une liste fermée, la référence d'un format qui ne peut
//! que nommer un commit ou une étiquette. La description du plan est écrite
//! par Sluis. Ce qui a des droits ne lit pas de texte libre.

use std::str::FromStr;
use std::sync::Arc;

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use crate::application::mise_en_ligne::{IssueMiseEnLigne, MiseEnLigne};
use crate::application::ports::{EtatApprobation, Horloge, PasserelleApprobation};
use crate::domain::{plan_renouvellement, AppError, Empreinte, Environnement, Tier};
use crate::infrastructure::gates_github::verifier_reference;
use crate::infrastructure::mcp::contrat::Contrat;
use crate::infrastructure::mcp::{ContratOutil, Outil};

/// `sluis_mise_en_ligne` — arguments.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArgumentsMiseEnLigne {
    /// Environnement cible : `integration`, `staging` ou `production`.
    pub environnement: String,
    /// Référence à mettre en ligne : un commit ou une étiquette, par exemple
    /// `v0.1.0`. Lettres, chiffres, « . », « _ » et « - » seulement.
    pub reference: String,
}

/// `sluis_etat_approbation` — arguments.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArgumentsEtatApprobation {
    /// Empreinte complète rendue par `sluis_mise_en_ligne` (64 caractères).
    pub empreinte: String,
}

const CONTRAT_MISE_EN_LIGNE: Contrat<ArgumentsMiseEnLigne> = Contrat::new(
    "sluis_mise_en_ligne",
    "Demande la mise en ligne d'une référence du projet configuré. Vérifie les \
     gates du plancher puis soumet un plan de Tier 1 à approbation humaine : \
     environnement GitHub protégé, ou, sur un dépôt privé, étiquette git signée \
     par le superviseur (la commande exacte est rendue). Ne met rien en ligne \
     par lui-même : rend l'empreinte du plan, à suivre avec \
     sluis_etat_approbation.",
);

/// `sluis_renouvellement_derogation` — aucun argument.
///
/// Rien n'est laissé à l'appelant : le plan est entièrement écrit par Sluis,
/// daté par son horloge.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArgumentsRenouvellement {}

const CONTRAT_RENOUVELLEMENT: Contrat<ArgumentsRenouvellement> = Contrat::new(
    "sluis_renouvellement_derogation",
    "Demande le renouvellement de la fenêtre de dérogation d'ADR-007 : soumet un \
     plan de Tier 1 daté à approbation humaine dans l'environnement production. \
     N'ouvre rien : l'exécutant ouvre la fenêtre, sur preuve lue chez GitHub, avec \
     l'instant de demande rendu ici.",
);

const CONTRAT_ETAT: Contrat<ArgumentsEtatApprobation> = Contrat::new(
    "sluis_etat_approbation",
    "Rend l'état d'une demande soumise par sluis_mise_en_ligne : en attente \
     (avec, sur un dépôt privé, la commande de signature à exécuter), approuvée \
     (avec l'approbateur rapporté par GitHub), refusée ou échouée.",
);

fn analyser<T: serde::de::DeserializeOwned>(
    outil: &str,
    arguments: &serde_json::Value,
) -> Result<T, AppError> {
    serde_json::from_value(arguments.clone()).map_err(|e| AppError::Analyse {
        quoi: format!("arguments de {outil}"),
        detail: e.to_string(),
    })
}

fn etat_en_json(etat: &EtatApprobation) -> serde_json::Value {
    match etat {
        EtatApprobation::EnAttente {
            run,
            url,
            commande: None,
        } => json!({ "etat": "en_attente", "run": run, "url": url }),
        EtatApprobation::EnAttente {
            run,
            url,
            commande: Some(commande),
        } => json!({ "etat": "en_attente", "run": run, "url": url, "commande": commande }),
        EtatApprobation::Approuvee { approbateur, run } => {
            json!({ "etat": "approuvee", "approbateur": approbateur, "run": run })
        }
        EtatApprobation::Refusee { motif } => json!({ "etat": "refusee", "motif": motif }),
        EtatApprobation::Echouee { detail } => json!({ "etat": "echouee", "detail": detail }),
    }
}

/// Demande la mise en ligne d'une référence.
pub struct OutilMiseEnLigne {
    cas: MiseEnLigne,
    projet: String,
}

impl OutilMiseEnLigne {
    /// Construit l'outil pour le projet configuré.
    pub fn new(cas: MiseEnLigne, projet: String) -> Self {
        Self { cas, projet }
    }
}

impl ContratOutil for OutilMiseEnLigne {
    fn nom(&self) -> &'static str {
        CONTRAT_MISE_EN_LIGNE.nom()
    }
    fn description(&self) -> &'static str {
        CONTRAT_MISE_EN_LIGNE.description()
    }
    fn schema(&self) -> serde_json::Value {
        CONTRAT_MISE_EN_LIGNE.schema()
    }
    fn desérialiser(&self, arguments: &serde_json::Value) -> Result<(), String> {
        CONTRAT_MISE_EN_LIGNE.desérialiser(arguments)
    }
}

impl Outil for OutilMiseEnLigne {
    fn tier(&self) -> Tier {
        Tier::Two
    }

    fn appeler(&self, arguments: &serde_json::Value) -> Result<serde_json::Value, AppError> {
        let arguments: ArgumentsMiseEnLigne = analyser(self.nom(), arguments)?;

        let environnement = Environnement::from_str(&arguments.environnement)?;
        if environnement == Environnement::Dev {
            return Err(AppError::Analyse {
                quoi: "environnement".to_string(),
                detail: "dev ne se met pas en ligne : integration, staging ou production"
                    .to_string(),
            });
        }
        // Vérifiée ici, avant même les gates : la référence finit dans le plan
        // soumis, donc dans les entrées du workflow.
        verifier_reference(&arguments.reference)?;

        match self
            .cas
            .demander(&self.projet, environnement, &arguments.reference)?
        {
            IssueMiseEnLigne::RefuseeParLesGates { gates_rouges } => Ok(json!({
                "issue": "refusee_par_les_gates",
                "gates_rouges": gates_rouges.iter().map(|g| g.nom()).collect::<Vec<_>>(),
            })),
            IssueMiseEnLigne::RefuseeParLaBarriere { controles_rouges } => Ok(json!({
                "issue": "refusee_par_la_barriere",
                "controles_rouges": controles_rouges
                    .iter()
                    .map(|v| json!({ "nom": v.nom, "motif": v.motif }))
                    .collect::<Vec<_>>(),
            })),
            IssueMiseEnLigne::RefuseeBarriereVide { motif } => Ok(json!({
                "issue": "refusee_barriere_vide",
                "motif": motif,
            })),
            IssueMiseEnLigne::Soumise { plan, etat } => Ok(json!({
                "issue": "soumise",
                "empreinte": plan.empreinte().hexadecimal(),
                "cible": plan.cible(),
                "environnement": plan.environnement().nom(),
                "approbation": etat_en_json(&etat),
            })),
        }
    }
}

/// Rend l'état d'une demande soumise.
pub struct OutilEtatApprobation {
    passerelle: Arc<dyn PasserelleApprobation>,
}

impl OutilEtatApprobation {
    /// Construit l'outil.
    pub fn new(passerelle: Arc<dyn PasserelleApprobation>) -> Self {
        Self { passerelle }
    }
}

impl ContratOutil for OutilEtatApprobation {
    fn nom(&self) -> &'static str {
        CONTRAT_ETAT.nom()
    }
    fn description(&self) -> &'static str {
        CONTRAT_ETAT.description()
    }
    fn schema(&self) -> serde_json::Value {
        CONTRAT_ETAT.schema()
    }
    fn desérialiser(&self, arguments: &serde_json::Value) -> Result<(), String> {
        CONTRAT_ETAT.desérialiser(arguments)
    }
}

impl Outil for OutilEtatApprobation {
    fn tier(&self) -> Tier {
        Tier::Two
    }

    fn appeler(&self, arguments: &serde_json::Value) -> Result<serde_json::Value, AppError> {
        let arguments: ArgumentsEtatApprobation = analyser(self.nom(), arguments)?;
        let empreinte = Empreinte::depuis_hexadecimal(&arguments.empreinte)?;
        let etat = self.passerelle.interroger(&empreinte)?;
        Ok(etat_en_json(&etat))
    }
}

/// Demande le renouvellement de la fenêtre d'ADR-007 (ADR-012, partie 3).
pub struct OutilRenouvellementDerogation {
    passerelle: Arc<dyn PasserelleApprobation>,
    horloge: Arc<dyn Horloge>,
}

impl OutilRenouvellementDerogation {
    /// Construit l'outil.
    pub fn new(passerelle: Arc<dyn PasserelleApprobation>, horloge: Arc<dyn Horloge>) -> Self {
        Self {
            passerelle,
            horloge,
        }
    }
}

impl ContratOutil for OutilRenouvellementDerogation {
    fn nom(&self) -> &'static str {
        CONTRAT_RENOUVELLEMENT.nom()
    }
    fn description(&self) -> &'static str {
        CONTRAT_RENOUVELLEMENT.description()
    }
    fn schema(&self) -> serde_json::Value {
        CONTRAT_RENOUVELLEMENT.schema()
    }
    fn desérialiser(&self, arguments: &serde_json::Value) -> Result<(), String> {
        CONTRAT_RENOUVELLEMENT.desérialiser(arguments)
    }
}

impl Outil for OutilRenouvellementDerogation {
    fn tier(&self) -> Tier {
        Tier::Two
    }

    fn appeler(&self, arguments: &serde_json::Value) -> Result<serde_json::Value, AppError> {
        let _: ArgumentsRenouvellement = analyser(self.nom(), arguments)?;
        let demande_le = self.horloge.maintenant();
        let plan = plan_renouvellement(demande_le)?;
        let etat = self.passerelle.soumettre(&plan)?;
        Ok(json!({
            "empreinte": plan.empreinte().hexadecimal(),
            "demande_le": demande_le.secondes(),
            "approbation": etat_en_json(&etat),
        }))
    }
}
