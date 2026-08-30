//! `sluis_campagne` — Tier 2, écriture bornée.
//!
//! L'outil ne décide de rien. Il lit la fenêtre de dérogation, la confronte à
//! l'horloge, loue un bail et confie le reste au cas d'usage. Chaque refus
//! qu'il rend vient d'un invariant du domaine : réécrire ces règles ici
//! créerait un second chemin d'exécution, non testé, exactement ce que
//! l'archétype interdit.

use std::sync::Arc;

use schemars::JsonSchema;
use serde::Deserialize;

use crate::application::campagne::{escalier_par_defaut, Campagne};
use crate::application::ports::{
    DepotDerogation, DestructeurBail, Horloge, Provisionneur, ReglagePalier,
};
use crate::domain::{AppError, Duree, PlafondDepense, ProjetBacASable, Tier};
use crate::infrastructure::bac_a_sable::GardeBail;
use crate::infrastructure::mcp::{ContratOutil, Outil};

/// `sluis_campagne` — arguments.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArgumentsCampagne {
    /// Durée de vie du bail, en secondes. Bornée par la configuration.
    pub ttl_secondes: i64,
    /// Dépense projetée pour la campagne, dans la devise du compte.
    pub estimation_depense: f64,
    /// Nombre de paliers de l'escalier à jouer. Tous par défaut.
    #[serde(default)]
    pub paliers: Option<usize>,
}

/// Ce que la configuration fixe, et que l'appelant ne peut pas déborder.
pub struct ReglagesCampagne {
    /// Projet OVH dédié aux bacs à sable.
    pub projet: ProjetBacASable,
    /// TTL maximal d'un bail.
    pub ttl_maximal: Duree,
    /// Plafond de dépense par campagne.
    pub plafond: PlafondDepense,
}

/// Loue une infrastructure éphémère, y déroule un escalier de charge, la
/// détruit, et rend les mesures.
pub struct OutilCampagne {
    campagne: Arc<Campagne>,
    derogations: Arc<dyn DepotDerogation>,
    provisionneur: Arc<dyn Provisionneur>,
    destructeur: Arc<dyn DestructeurBail>,
    horloge: Arc<dyn Horloge>,
    reglages: ReglagesCampagne,
}

impl OutilCampagne {
    /// Construit l'outil.
    pub fn new(
        campagne: Arc<Campagne>,
        derogations: Arc<dyn DepotDerogation>,
        provisionneur: Arc<dyn Provisionneur>,
        destructeur: Arc<dyn DestructeurBail>,
        horloge: Arc<dyn Horloge>,
        reglages: ReglagesCampagne,
    ) -> Self {
        Self {
            campagne,
            derogations,
            provisionneur,
            destructeur,
            horloge,
            reglages,
        }
    }

    fn escalier(&self, paliers: Option<usize>) -> Vec<ReglagePalier> {
        let complet = escalier_par_defaut();
        match paliers {
            Some(combien) if combien < complet.len() => complet.into_iter().take(combien).collect(),
            _ => complet,
        }
    }
}

impl ContratOutil for OutilCampagne {
    fn nom(&self) -> &'static str {
        "sluis_campagne"
    }
    fn description(&self) -> &'static str {
        "Loue une infrastructure éphémère dans le projet bac à sable, y déroule \
         un escalier de charge, la détruit inconditionnellement, et rend les \
         mesures. Refusé si la fenêtre de dérogation est fermée, si le TTL \
         dépasse le maximum configuré, si la dépense projetée dépasse le \
         plafond, ou si la campagne finirait après la fermeture de la fenêtre."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(ArgumentsCampagne))
            .unwrap_or_else(|_| serde_json::json!({"type": "object"}))
    }
    fn desérialiser(&self, arguments: &serde_json::Value) -> Result<(), String> {
        serde_json::from_value::<ArgumentsCampagne>(arguments.clone())
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

impl Outil for OutilCampagne {
    fn tier(&self) -> Tier {
        // Écriture bornée : infrastructure éphémère, à TTL et plafond, dans une
        // fenêtre de dérogation. Le Tier 1 reste ce qui touche la production.
        Tier::Two
    }

    fn appeler(&self, arguments: &serde_json::Value) -> Result<serde_json::Value, AppError> {
        let arguments: ArgumentsCampagne =
            serde_json::from_value(arguments.clone()).map_err(|e| AppError::Analyse {
                quoi: "arguments de sluis_campagne".to_string(),
                detail: e.to_string(),
            })?;

        let maintenant = self.horloge.maintenant();
        let fenetre = self
            .derogations
            .courante()?
            .ok_or_else(|| AppError::TierViolation {
                raison: "aucune fenêtre de dérogation en vigueur : une fenêtre absente vaut \
                         fenêtre fermée, et son ouverture est un acte de Tier 1"
                    .to_string(),
            })?;
        let derogation = fenetre.valider(maintenant)?;

        let bail = crate::domain::BailBacASable::louer(
            &derogation,
            self.reglages.projet.clone(),
            Duree::secondes(arguments.ttl_secondes)?,
            self.reglages.plafond,
            arguments.estimation_depense,
            self.reglages.ttl_maximal,
            maintenant,
        )?;

        let escalier = self.escalier(arguments.paliers);
        let secondes_avant_fermeture = derogation.close_le().secondes() - maintenant.secondes();

        // La garde double le cas d'usage : lui gère l'ordre des opérations,
        // elle couvre la panique, que nul chemin de sortie ne couvre.
        let garde = GardeBail::nouvelle(bail.clone(), self.destructeur.clone());

        let resultat = self.campagne.conduire(
            &bail,
            self.provisionneur.as_ref(),
            self.destructeur.as_ref(),
            &escalier,
            secondes_avant_fermeture,
        )?;

        // Le cas d'usage a détruit. Désarmer évite un second `terraform
        // destroy`, qui serait sans danger mais coûterait des minutes. Si la
        // destruction a échoué, on ne désarme pas : la garde retentera à la
        // sortie de portée, et c'est précisément là qu'on la veut.
        if resultat.echec_destruction.is_none() {
            garde.desarmer();
        }

        Ok(serde_json::json!({
            "cible": resultat.cible.as_ref().map(|c| c.adresse()),
            "sorties": resultat
                .cible
                .as_ref()
                .map(|c| c.sorties().to_vec())
                .unwrap_or_default(),
            "paliers_joues": resultat
                .paliers_joues
                .iter()
                .map(|p| p.nom())
                .collect::<Vec<_>>(),
            "interrompue_a": resultat.interrompue_a.map(|p| p.nom()),
            "mesures": resultat.mesures,
            "echec_destruction": resultat.echec_destruction,
            "bail": {
                "projet": bail.projet().to_string(),
                "ouvert_le": bail.ouvert_le().secondes(),
                "expire_le": bail.expire_le().secondes(),
            },
        }))
    }
}
