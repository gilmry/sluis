//! Bounded context Capacité — campagnes de charge et recalage des priors.
//!
//! La finalité de ce module n'est pas de mesurer pour mesurer. L'abaque
//! coût/capacité de la Méthode Foyer porte une douzaine de constantes marquées
//! `[caler]`, c'est-à-dire supposées, et son §9 exige qu'elles soient
//! remplacées par du mesuré dès qu'un projet a tourné. Ces constantes
//! gouvernent des arbitrages de palier d'architecture, donc des décisions
//! coûteuses et parfois irréversibles.
//!
//! D'où l'invariant central : **une mesure porte toujours sa provenance**.
//! Confondre observé et déduit ferait exactement ce que la discipline de
//! lucidité interdit — présenter une extrapolation comme un fait.

use std::fmt;

use serde::Serialize;

use crate::domain::AppError;

/// D'où vient une valeur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Provenance {
    /// Observé sur un système réel, avec un échantillon.
    Mesure,
    /// Déduit, extrapolé, ou hérité d'un prior.
    Supposition,
}

impl fmt::Display for Provenance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Provenance::Mesure => "mesuré",
            Provenance::Supposition => "supposé",
        })
    }
}

/// Les paliers d'un escalier de charge.
///
/// Repris du corpus `wrk` existant de KoproGo, dans l'ordre où ils doivent être
/// joués : chauffer avant de charger, et finir par le maintien long qui révèle
/// les fuites que les pointes ne montrent pas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Palier {
    /// Chauffe, pour que les caches et le JIT ne faussent pas la suite.
    Warmup,
    /// Charge légère.
    Light,
    /// Charge moyenne.
    Medium,
    /// Charge lourde.
    Heavy,
    /// Profil réaliste, mélangé.
    Realistic,
    /// Pointe brutale.
    Spike,
    /// Maintien long.
    Soak,
}

impl Palier {
    /// Tous les paliers, dans l'ordre d'exécution.
    pub const ESCALIER: [Palier; 7] = [
        Palier::Warmup,
        Palier::Light,
        Palier::Medium,
        Palier::Heavy,
        Palier::Realistic,
        Palier::Spike,
        Palier::Soak,
    ];

    /// Nom canonique, celui des scripts du corpus existant.
    pub fn nom(&self) -> &'static str {
        match self {
            Palier::Warmup => "warmup",
            Palier::Light => "light",
            Palier::Medium => "medium",
            Palier::Heavy => "heavy",
            Palier::Realistic => "realistic",
            Palier::Spike => "spike",
            Palier::Soak => "soak",
        }
    }
}

impl fmt::Display for Palier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.nom())
    }
}

/// Une mesure de capacité.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MesureCapacite {
    grandeur: String,
    valeur: f64,
    unite: String,
    provenance: Provenance,
    palier: Option<Palier>,
    /// Taille de l'échantillon. Nulle pour une supposition.
    echantillon: u64,
    /// Conditions d'obtention, sans lesquelles une mesure ne se compare pas.
    conditions: String,
}

impl MesureCapacite {
    /// Consigne une valeur observée.
    ///
    /// Refuse un échantillon vide : une « mesure » sans observation est une
    /// supposition qui se fait passer pour un fait.
    pub fn mesuree(
        grandeur: String,
        valeur: f64,
        unite: String,
        palier: Palier,
        echantillon: u64,
        conditions: String,
    ) -> Result<Self, AppError> {
        if !valeur.is_finite() {
            return Err(AppError::Configuration {
                detail: format!("mesure « {grandeur} » non finie"),
            });
        }
        if echantillon == 0 {
            return Err(AppError::Configuration {
                detail: format!(
                    "mesure « {grandeur} » sans échantillon : ce serait une supposition \
                     présentée comme un fait"
                ),
            });
        }
        if conditions.trim().is_empty() {
            return Err(AppError::Configuration {
                detail: format!(
                    "mesure « {grandeur} » sans conditions : elle ne serait comparable à rien"
                ),
            });
        }
        Ok(Self {
            grandeur,
            valeur,
            unite,
            provenance: Provenance::Mesure,
            palier: Some(palier),
            echantillon,
            conditions,
        })
    }

    /// Consigne une valeur déduite ou héritée.
    pub fn supposee(grandeur: String, valeur: f64, unite: String, origine: String) -> Self {
        Self {
            grandeur,
            valeur,
            unite,
            provenance: Provenance::Supposition,
            palier: None,
            echantillon: 0,
            conditions: origine,
        }
    }

    /// Grandeur mesurée.
    pub fn grandeur(&self) -> &str {
        &self.grandeur
    }
    /// Valeur.
    pub fn valeur(&self) -> f64 {
        self.valeur
    }
    /// Unité.
    pub fn unite(&self) -> &str {
        &self.unite
    }
    /// Provenance.
    pub fn provenance(&self) -> Provenance {
        self.provenance
    }
    /// Palier d'obtention.
    pub fn palier(&self) -> Option<Palier> {
        self.palier
    }
    /// Taille de l'échantillon.
    pub fn echantillon(&self) -> u64 {
        self.echantillon
    }
    /// Conditions d'obtention.
    pub fn conditions(&self) -> &str {
        &self.conditions
    }
}

/// Vérifie la cohérence interne d'un jeu de latences.
///
/// Un P99 inférieur à la médiane est arithmétiquement impossible : le rapporter
/// signalerait un défaut de collecte, et le taire propagerait une mesure fausse
/// dans le modèle de coût.
pub fn verifier_coherence_latences(p50: f64, p99: f64) -> Result<(), AppError> {
    if p99 < p50 {
        return Err(AppError::Configuration {
            detail: format!(
                "latences incohérentes : P99 ({p99}) inférieur à la médiane ({p50}), \
                 ce qui est arithmétiquement impossible"
            ),
        });
    }
    Ok(())
}

/// Une constante supposée du modèle de coût, en attente d'être calibrée.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Prior {
    /// Nom de la constante, tel qu'il figure dans l'abaque.
    pub grandeur: String,
    /// Valeur supposée.
    pub valeur: f64,
    /// Unité.
    pub unite: String,
    /// D'où vient cette supposition.
    pub origine: String,
}

/// Ce qu'un recalage propose pour une constante.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Recalage {
    /// La constante concernée.
    pub prior: Prior,
    /// La mesure qui la remplace.
    pub mesure: MesureCapacite,
    /// Écart relatif, en pourcentage de la valeur supposée.
    pub ecart_pourcent: f64,
    /// Vrai si l'écart dépasse le seuil au-delà duquel il mérite attention.
    pub notable: bool,
}

/// Rapport de recalage : ce que la campagne apprend au modèle de coût.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RapportRecalage {
    /// Recalages proposés.
    pub recalages: Vec<Recalage>,
    /// Priors restés sans mesure correspondante, signalés plutôt que tus.
    pub non_calibres: Vec<Prior>,
}

impl RapportRecalage {
    /// Construit le rapport en confrontant priors et mesures.
    ///
    /// Un prior sans mesure n'est pas une erreur : c'est une constante que
    /// cette campagne n'éclaire pas. Le taire laisserait croire que tout a été
    /// calibré.
    pub fn construire(
        priors: Vec<Prior>,
        mesures: &[MesureCapacite],
        seuil_notable_pourcent: f64,
    ) -> Self {
        let mut recalages = Vec::new();
        let mut non_calibres = Vec::new();

        for prior in priors {
            // Seule une mesure, jamais une supposition, ne recale un prior :
            // remplacer une supposition par une autre n'apprend rien.
            let correspondante = mesures
                .iter()
                .find(|m| m.grandeur() == prior.grandeur && m.provenance() == Provenance::Mesure);
            match correspondante {
                Some(mesure) => {
                    let ecart = if prior.valeur.abs() > f64::EPSILON {
                        (mesure.valeur() - prior.valeur) / prior.valeur * 100.0
                    } else {
                        0.0
                    };
                    recalages.push(Recalage {
                        prior,
                        mesure: mesure.clone(),
                        ecart_pourcent: ecart,
                        notable: ecart.abs() >= seuil_notable_pourcent,
                    });
                }
                None => non_calibres.push(prior),
            }
        }
        Self {
            recalages,
            non_calibres,
        }
    }

    /// Nombre de constantes effectivement passées de supposé à mesuré.
    pub fn calibrees(&self) -> usize {
        self.recalages.len()
    }
}
