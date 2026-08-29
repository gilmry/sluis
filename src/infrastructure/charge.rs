//! Moteur de charge `wrk`.
//!
//! Réutilise l'outil du corpus de tests de KoproGo plutôt que d'en introduire
//! un nouveau : les scripts Lua, les paliers et les ordres de grandeur y sont
//! déjà calibrés sur la stack réelle.

use crate::application::ports::{Executeur, MoteurCharge, ReglagePalier};
use crate::domain::{AppError, MesureCapacite, Palier};

/// Pilote `wrk`.
pub struct Wrk<E: Executeur> {
    executeur: E,
}

impl<E: Executeur> Wrk<E> {
    /// Construit le pilote.
    pub fn new(executeur: E) -> Self {
        Self { executeur }
    }
}

/// Convertit une durée `wrk` (`12.34ms`, `1.20s`, `890us`) en millisecondes.
pub fn duree_en_millisecondes(brut: &str) -> Option<f64> {
    let brut = brut.trim();
    let (nombre, facteur) = if let Some(n) = brut.strip_suffix("ms") {
        (n, 1.0)
    } else if let Some(n) = brut.strip_suffix("us") {
        (n, 0.001)
    } else if let Some(n) = brut.strip_suffix('s') {
        (n, 1000.0)
    } else if let Some(n) = brut.strip_suffix('m') {
        (n, 60_000.0)
    } else {
        (brut, 1.0)
    };
    nombre.parse::<f64>().ok().map(|v| v * facteur)
}

/// Analyse la sortie de `wrk --latency`.
///
/// Conserve la sortie brute en conditions : une mesure sans ses conditions ne
/// se compare à rien, et l'abaque exige qu'elles soient consignées.
pub fn analyser_sortie(
    sortie: &str,
    palier: Palier,
    conditions: &str,
) -> Result<Vec<MesureCapacite>, AppError> {
    let mut mesures = Vec::new();
    let mut requetes = 0_u64;
    let mut p50 = None;
    let mut p99 = None;
    let mut debit = None;

    for ligne in sortie.lines() {
        let ligne = ligne.trim();
        if let Some(reste) = ligne.strip_prefix("Requests/sec:") {
            debit = reste.trim().parse::<f64>().ok();
        } else if ligne.starts_with("50%") {
            p50 = ligne
                .split_whitespace()
                .nth(1)
                .and_then(duree_en_millisecondes);
        } else if ligne.starts_with("99%") {
            p99 = ligne
                .split_whitespace()
                .nth(1)
                .and_then(duree_en_millisecondes);
        } else if let Some(position) = ligne.find(" requests in ") {
            requetes = ligne[..position].trim().parse::<u64>().unwrap_or(0);
        }
    }

    if requetes == 0 {
        return Err(AppError::Analyse {
            quoi: format!("sortie wrk du palier {palier}"),
            detail: "aucune requête comptée : la mesure serait sans échantillon".to_string(),
        });
    }
    if let (Some(p50), Some(p99)) = (p50, p99) {
        crate::domain::verifier_coherence_latences(p50, p99)?;
    }

    if let Some(debit) = debit {
        mesures.push(MesureCapacite::mesuree(
            "debit".to_string(),
            debit,
            "req/s".to_string(),
            palier,
            requetes,
            conditions.to_string(),
        )?);
    }
    if let Some(p50) = p50 {
        mesures.push(MesureCapacite::mesuree(
            "latence_p50".to_string(),
            p50,
            "ms".to_string(),
            palier,
            requetes,
            conditions.to_string(),
        )?);
    }
    if let Some(p99) = p99 {
        mesures.push(MesureCapacite::mesuree(
            "latence_p99".to_string(),
            p99,
            "ms".to_string(),
            palier,
            requetes,
            conditions.to_string(),
        )?);
    }
    Ok(mesures)
}

impl<E: Executeur> MoteurCharge for Wrk<E> {
    fn jouer(&self, cible: &str, reglage: &ReglagePalier) -> Result<Vec<MesureCapacite>, AppError> {
        let sortie = self.executeur.executer(
            "wrk",
            &[
                format!("-t{}", reglage.fils),
                format!("-c{}", reglage.connexions),
                format!("-d{}s", reglage.duree_secondes),
                "--latency".to_string(),
                cible.to_string(),
            ],
            None,
        )?;
        if !sortie.reussi() {
            return Err(AppError::ServiceTiers {
                service: "wrk".to_string(),
                detail: sortie.erreur.trim().to_string(),
            });
        }
        let conditions = format!(
            "palier {} · {} fils · {} connexions · {}s · cible {}",
            reglage.palier, reglage.fils, reglage.connexions, reglage.duree_secondes, cible
        );
        analyser_sortie(&sortie.sortie, reglage.palier, &conditions)
    }

    fn disponible(&self) -> bool {
        self.executeur
            .executer("wrk", &["--version".to_string()], None)
            .is_ok()
    }
}
