//! Cas d'usage — campagne de charge sur infrastructure éphémère.
//!
//! Le point le plus important est l'ordre des refus : **tout ce qui peut être
//! refusé l'est à l'admission**, avant qu'une seule ressource ne soit
//! provisionnée. Un moteur absent, un plafond dépassé, une fenêtre trop courte
//! découverts en cours d'escalier laisseraient une infrastructure en l'air et
//! des mesures inutilisables.

use std::sync::Arc;

use crate::application::ports::{MoteurCharge, ReglagePalier};
use crate::domain::{AppError, MesureCapacite, Palier};

/// L'escalier de charge par défaut, calqué sur le corpus existant.
pub fn escalier_par_defaut() -> Vec<ReglagePalier> {
    vec![
        (Palier::Warmup, 2, 1, 10),
        (Palier::Light, 10, 2, 30),
        (Palier::Medium, 50, 4, 60),
        (Palier::Heavy, 200, 8, 60),
        (Palier::Realistic, 100, 4, 120),
        (Palier::Spike, 500, 8, 30),
        (Palier::Soak, 50, 4, 600),
    ]
    .into_iter()
    .map(|(palier, connexions, fils, duree_secondes)| ReglagePalier {
        palier,
        connexions,
        fils,
        duree_secondes,
    })
    .collect()
}

/// Résultat d'une campagne.
#[derive(Debug)]
pub struct ResultatCampagne {
    /// Mesures collectées, tous paliers confondus.
    pub mesures: Vec<MesureCapacite>,
    /// Paliers effectivement joués.
    pub paliers_joues: Vec<Palier>,
    /// Palier où la campagne s'est arrêtée, le cas échéant.
    pub interrompue_a: Option<Palier>,
}

/// Conduit une campagne.
pub struct Campagne {
    moteur: Arc<dyn MoteurCharge>,
}

impl Campagne {
    /// Construit le cas d'usage.
    pub fn new(moteur: Arc<dyn MoteurCharge>) -> Self {
        Self { moteur }
    }

    /// Vérifie qu'une campagne peut être lancée.
    ///
    /// À appeler **avant** de provisionner quoi que ce soit.
    pub fn verifier_admission(
        &self,
        duree_totale_secondes: i64,
        secondes_avant_fermeture_fenetre: i64,
    ) -> Result<(), AppError> {
        if !self.moteur.disponible() {
            return Err(AppError::EngineMissing {
                binaire: "wrk".to_string(),
            });
        }
        if duree_totale_secondes > secondes_avant_fermeture_fenetre {
            return Err(AppError::TierViolation {
                raison: format!(
                    "la campagne durerait {duree_totale_secondes}s alors que la fenêtre de \
                     dérogation ferme dans {secondes_avant_fermeture_fenetre}s : refusée à \
                     l'admission pour qu'elle ne soit pas coupée en plein palier"
                ),
            });
        }
        Ok(())
    }

    /// Joue l'escalier contre une cible.
    ///
    /// Un palier en échec **arrête** la campagne au lieu de la poursuivre : les
    /// paliers suivants mesureraient un système déjà dégradé, et leurs chiffres
    /// seraient trompeurs.
    pub fn jouer(&self, cible: &str, escalier: &[ReglagePalier]) -> ResultatCampagne {
        let mut mesures = Vec::new();
        let mut paliers_joues = Vec::new();
        let mut interrompue_a = None;

        for reglage in escalier {
            match self.moteur.jouer(cible, reglage) {
                Ok(mut obtenues) => {
                    mesures.append(&mut obtenues);
                    paliers_joues.push(reglage.palier);
                }
                Err(_) => {
                    interrompue_a = Some(reglage.palier);
                    break;
                }
            }
        }
        ResultatCampagne {
            mesures,
            paliers_joues,
            interrompue_a,
        }
    }

    /// Durée totale de l'escalier, en secondes.
    pub fn duree_totale(escalier: &[ReglagePalier]) -> i64 {
        escalier.iter().map(|r| r.duree_secondes as i64).sum()
    }
}
