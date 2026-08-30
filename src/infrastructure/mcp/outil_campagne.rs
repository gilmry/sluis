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

use crate::application::campagne::{escalier_par_defaut, Campagne, PlanCampagne};
use crate::application::ports::{
    DepotCharge, DepotDerogation, DestructeurBail, Horloge, Provisionneur, ReglagePalier,
};
use crate::domain::{
    AppError, BailBacASable, DemandeBail, Duree, PlafondDepense, ProjetBacASable, Tier, ValeurSure,
};
use crate::infrastructure::bac_a_sable::GardeBail;
use crate::infrastructure::mcp::{ContratOutil, Outil};

/// `sluis_campagne` — arguments.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArgumentsCampagne {
    /// Racine d'infrastructure du dépôt à mesurer, par exemple
    /// `/depots/koprogo/infrastructure`. Doit figurer parmi les racines
    /// autorisées du serveur.
    pub depot: String,
    /// Dépense projetée pour la campagne, dans la devise du compte.
    pub estimation_depense: f64,
    /// Nombre de paliers de l'escalier à jouer. Tous par défaut.
    #[serde(default)]
    pub paliers: Option<usize>,
}

/// Ce que la configuration du serveur fixe, et que nul appelant ne déborde.
///
/// La déclaration du dépôt mesuré demande un TTL et un plafond ; ces
/// valeurs-ci les bornent. Le minimum des deux l'emporte toujours, sans quoi
/// écrire dans le dépôt mesuré suffirait à s'octroyer six heures et deux cents
/// euros, et ADR-007 se contournerait par une pull request.
pub struct ReglagesCampagne {
    /// Projet OVH dédié aux bacs à sable.
    pub projet: ProjetBacASable,
    /// TTL maximal d'un bail, quelle que soit la demande.
    pub ttl_maximal: Duree,
    /// Plafond de dépense, quelle que soit la demande.
    pub plafond: PlafondDepense,
    /// Racines d'infrastructure que ce serveur a le droit de mesurer.
    ///
    /// Liste blanche : sans elle, un appelant ferait tourner terraform sur
    /// n'importe quel chemin de la machine.
    pub racines_autorisees: Vec<String>,
}

/// Loue une infrastructure éphémère, y déroule un escalier de charge, la
/// détruit, et rend les mesures.
pub struct OutilCampagne {
    campagne: Arc<Campagne>,
    charges: Arc<dyn DepotCharge>,
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
        charges: Arc<dyn DepotCharge>,
        derogations: Arc<dyn DepotDerogation>,
        provisionneur: Arc<dyn Provisionneur>,
        destructeur: Arc<dyn DestructeurBail>,
        horloge: Arc<dyn Horloge>,
        reglages: ReglagesCampagne,
    ) -> Self {
        Self {
            campagne,
            charges,
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

        // Liste blanche d'abord : rien n'est lu d'un dépôt non autorisé, pas
        // même sa déclaration.
        if !self
            .reglages
            .racines_autorisees
            .iter()
            .any(|racine| racine == &arguments.depot)
        {
            return Err(AppError::TierViolation {
                raison: format!(
                    "« {} » ne figure pas parmi les dépôts que ce serveur a le droit de \
                     mesurer",
                    arguments.depot
                ),
            });
        }

        let declaration = self.charges.lire(&arguments.depot)?;

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

        // Le module est résolu depuis la racine du dépôt : le bail le portera,
        // pour que le chien de garde sache quoi détruire sans rien demander.
        let module = ValeurSure::new(format!(
            "{}/{}",
            arguments.depot.trim_end_matches('/'),
            declaration.module().trim_start_matches('/')
        ))?;

        let bail = BailBacASable::louer(
            &derogation,
            DemandeBail {
                projet: self.reglages.projet.clone(),
                module,
                // Les bornes se cumulent : le minimum du demandé et de
                // l'autorisé, jamais ce que le dépôt mesuré réclame seul.
                ttl: declaration.ttl_borne(self.reglages.ttl_maximal),
                plafond: declaration.plafond_borne(self.reglages.plafond),
                estimation_depense: arguments.estimation_depense,
            },
            self.reglages.ttl_maximal,
            maintenant,
        )?;

        let escalier = self.escalier(arguments.paliers);
        let plan = PlanCampagne {
            escalier: &escalier,
            sortie_adresse: declaration.sortie_adresse(),
            chemin: declaration.chemin(),
            secondes_avant_fermeture_fenetre: derogation.close_le().secondes()
                - maintenant.secondes(),
        };

        // La garde double le cas d'usage : lui gère l'ordre des opérations,
        // elle couvre la panique, que nul chemin de sortie ne couvre.
        let garde = GardeBail::nouvelle(bail.clone(), self.destructeur.clone());

        let resultat = self.campagne.conduire(
            &bail,
            self.provisionneur.as_ref(),
            self.destructeur.as_ref(),
            &plan,
        )?;

        // Le cas d'usage a détruit. Désarmer évite un second `terraform
        // destroy`, qui serait sans danger mais coûterait des minutes. Si la
        // destruction a échoué, on ne désarme pas : la garde retentera à la
        // sortie de portée, et c'est précisément là qu'on la veut.
        if resultat.echec_destruction.is_none() {
            garde.desarmer();
        }

        let verdict = declaration.verdict(&resultat.mesures);

        Ok(serde_json::json!({
            "depot": arguments.depot,
            "cible": resultat.cible.as_ref().map(|c| c.adresse()),
            "chemin": declaration.chemin(),
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
            "verdict": verdict,
            "cible_declaree": declaration.cible(),
            "echec_destruction": resultat.echec_destruction,
            "bail": {
                "projet": bail.projet().to_string(),
                "module": bail.module().valeur(),
                "ouvert_le": bail.ouvert_le().secondes(),
                "expire_le": bail.expire_le().secondes(),
            },
        }))
    }
}
