//! Cas d'usage — mise en ligne d'un projet.
//!
//! L'ordre des étapes porte l'essentiel de la valeur.
//!
//! Les gates du plancher sont vérifiées **avant** toute soumission à
//! approbation. Une gate rouge n'est donc pas découverte au déploiement, ni
//! soumise au jugement d'un relecteur qui devrait se souvenir de la vérifier :
//! elle bloque en amont. C'est la distinction que `gates.md` pose entre le
//! plancher mécanique, qui ne se discute pas, et l'exigence de jalon, qui se
//! trie.

use std::sync::Arc;

use crate::application::ports::{
    EtatApprobation, GatePlancher, PasserelleApprobation, VerificateurGates,
};
use crate::domain::{Action, AppError, Environnement, PlanChangement, Tier};

/// Résultat d'une demande de mise en ligne.
#[derive(Debug)]
pub enum IssueMiseEnLigne {
    /// Refusée avant soumission : au moins une gate du plancher est rouge.
    RefuseeParLesGates {
        /// Les gates fautives.
        gates_rouges: Vec<GatePlancher>,
    },
    /// Soumise à approbation humaine.
    Soumise {
        /// Le plan soumis.
        plan: Box<PlanChangement>,
        /// L'état initial rendu par la passerelle.
        etat: EtatApprobation,
    },
}

/// Orchestre une mise en ligne.
pub struct MiseEnLigne {
    gates: Arc<dyn VerificateurGates>,
    passerelle: Arc<dyn PasserelleApprobation>,
}

impl MiseEnLigne {
    /// Construit le cas d'usage.
    pub fn new(
        gates: Arc<dyn VerificateurGates>,
        passerelle: Arc<dyn PasserelleApprobation>,
    ) -> Self {
        Self { gates, passerelle }
    }

    /// Demande la mise en ligne d'une référence.
    ///
    /// Ne met rien en ligne : produit un plan de Tier 1 et le soumet. C'est
    /// l'humain, via un environnement GitHub protégé, qui décide, et c'est le
    /// travail GitHub qui exécute avec des secrets que Sluis n'a pas.
    pub fn demander(
        &self,
        projet: &str,
        environnement: Environnement,
        reference: &str,
    ) -> Result<IssueMiseEnLigne, AppError> {
        let etats = self.gates.etat_plancher(reference)?;
        let rouges: Vec<GatePlancher> = etats.iter().filter(|e| !e.verte).map(|e| e.gate).collect();
        if !rouges.is_empty() {
            return Ok(IssueMiseEnLigne::RefuseeParLesGates {
                gates_rouges: rouges,
            });
        }

        let plan = PlanChangement::new(
            Action::MiseEnLigne,
            environnement,
            Tier::One,
            format!("{projet}@{reference}"),
            format!(
                "met en ligne {projet} sur {environnement}, {} gate(s) du plancher au vert",
                etats.len()
            ),
            etats
                .iter()
                .map(|e| format!("{} : vert", e.gate.nom()))
                .collect::<Vec<_>>()
                .join("\n"),
        )?;

        let etat = self.passerelle.soumettre(&plan)?;
        Ok(IssueMiseEnLigne::Soumise {
            plan: Box::new(plan),
            etat,
        })
    }
}
