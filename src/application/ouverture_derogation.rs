//! Cas d'usage — ouvrir la fenêtre d'ADR-007 sur preuve d'approbation.
//!
//! C'est la troisième partie d'ADR-012 : le Sluis public **demande** le
//! renouvellement, l'exécutant l'**ouvre**, et seulement sur une preuve qu'il
//! lit lui-même chez GitHub. Il ne reçoit qu'un instant : il reconstruit le
//! plan daté, donc son empreinte, et ne peut pas être dirigé vers
//! l'approbation d'un autre plan.
//!
//! L'ordre des contrôles compte, du moins coûteux au plus engageant :
//! fraîcheur de la demande, empreinte jamais consommée, approbation lue chez
//! GitHub, consignation de l'empreinte, puis seulement ouverture.

use std::sync::Arc;

use crate::application::ports::{
    DepotDerogation, EtatApprobation, Horloge, PasserelleApprobation, RegistreRenouvellements,
};
use crate::domain::{
    plan_renouvellement, AppError, Duree, FenetreDerogation, Horodatage, JetonChangement,
};

/// Âge maximal d'une demande de renouvellement, en secondes : 72 heures.
///
/// Laisse au relecteur le temps d'un week-end. Borne aussi ce que vaudrait une
/// approbation dont l'empreinte aurait été effacée du registre.
pub const FRAICHEUR_MAXIMALE_SECONDES: i64 = 72 * 3600;

/// Ouvre la fenêtre de dérogation.
pub struct OuvertureDerogation {
    passerelle: Arc<dyn PasserelleApprobation>,
    registre: Arc<dyn RegistreRenouvellements>,
    depot: Arc<dyn DepotDerogation>,
    horloge: Arc<dyn Horloge>,
    duree: Duree,
}

impl OuvertureDerogation {
    /// Construit le cas d'usage.
    pub fn new(
        passerelle: Arc<dyn PasserelleApprobation>,
        registre: Arc<dyn RegistreRenouvellements>,
        depot: Arc<dyn DepotDerogation>,
        horloge: Arc<dyn Horloge>,
        duree: Duree,
    ) -> Self {
        Self {
            passerelle,
            registre,
            depot,
            horloge,
            duree,
        }
    }

    /// Ouvre la fenêtre pour la demande faite à l'instant `demande_le`.
    pub fn ouvrir(&self, demande_le: Horodatage) -> Result<FenetreDerogation, AppError> {
        let refus = |raison: String| AppError::TierViolation { raison };
        let maintenant = self.horloge.maintenant();

        if demande_le.apres(maintenant) {
            return Err(refus(format!(
                "demande de renouvellement datée du futur ({demande_le}, il est {maintenant})"
            )));
        }
        if maintenant.secondes() - demande_le.secondes() > FRAICHEUR_MAXIMALE_SECONDES {
            return Err(refus(format!(
                "demande de renouvellement trop ancienne ({demande_le}) : au-delà de 72 h, \
                 il faut en soumettre une nouvelle"
            )));
        }

        let plan = plan_renouvellement(demande_le)?;
        if self.registre.deja_consommee(plan.empreinte())? {
            return Err(refus(format!(
                "l'approbation {} a déjà ouvert une fenêtre : elle ne sert qu'une fois",
                plan.empreinte().abregee()
            )));
        }

        let approbateur = match self.passerelle.interroger(plan.empreinte())? {
            EtatApprobation::Approuvee { approbateur, .. } => approbateur,
            EtatApprobation::EnAttente { .. } => {
                return Err(refus(
                    "renouvellement pas encore approuvé dans l'environnement protégé".to_string(),
                ))
            }
            EtatApprobation::Refusee { motif } => {
                return Err(refus(format!("renouvellement refusé : {motif}")))
            }
            EtatApprobation::Echouee { detail } => {
                return Err(refus(format!("renouvellement en échec : {detail}")))
            }
        };

        // Consignée avant l'ouverture : si l'écriture de la fenêtre échoue
        // ensuite, le renouvellement est perdu, mais il ne peut pas être rejoué.
        self.registre.consigner(plan.empreinte())?;

        let jeton = JetonChangement::emettre(
            plan.empreinte().clone(),
            approbateur,
            maintenant,
            Duree::secondes(60)?,
        )?
        .consommer(&plan, maintenant)?;
        let fenetre = FenetreDerogation::ouvrir(&jeton, maintenant, self.duree)?;
        self.depot.enregistrer(&fenetre)?;
        Ok(fenetre)
    }
}
