//! Vérification des gates du plancher sur les check-runs GitHub.
//!
//! Une mise en ligne n'est soumise que si chaque gate du plancher est verte sur
//! la référence demandée (`MiseEnLigne`). Ce vérificateur lit cet état là où la
//! CI du dépôt cible l'a laissé : les check-runs du commit.
//!
//! **Fail-closed partout.** Une gate dont le check est absent, en cours,
//! ignoré, neutre ou annulé est rouge. Seul `completed` + `success` est vert.
//! Un plancher sans gate configurée ne rend pas « tout vert » : il est refusé,
//! sans quoi l'absence de configuration vaudrait autorisation.

use std::time::Duration;

use crate::application::ports::{EtatGate, GatePlancher, VerificateurGates};
use crate::domain::{AppError, Redacted};

/// Longueur maximale d'une référence acceptée.
const LONGUEUR_MAX_REFERENCE: usize = 100;

/// Vérificateur des gates sur l'API GitHub.
pub struct VerificateurGatesGithub {
    proprietaire: String,
    depot: String,
    api: String,
    jeton: Redacted<String>,
    correspondance: Vec<(GatePlancher, String)>,
    agent: ureq::Agent,
}

impl std::fmt::Debug for VerificateurGatesGithub {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VerificateurGatesGithub")
            .field("depot", &format!("{}/{}", self.proprietaire, self.depot))
            .field("correspondance", &self.correspondance)
            .finish_non_exhaustive()
    }
}

impl VerificateurGatesGithub {
    /// Construit le vérificateur.
    ///
    /// `correspondance` associe chaque gate du plancher au nom du check-run
    /// qui la porte dans la CI du dépôt cible.
    pub fn new(
        proprietaire: String,
        depot: String,
        api: String,
        jeton: Redacted<String>,
        correspondance: Vec<(GatePlancher, String)>,
    ) -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(30)))
            .build()
            .into();
        Self {
            proprietaire,
            depot,
            api,
            jeton,
            correspondance,
            agent,
        }
    }

    fn erreur(&self, detail: String) -> AppError {
        AppError::ServiceTiers {
            service: "GitHub".to_string(),
            detail,
        }
    }
}

/// Une référence n'est acceptée que si elle ne peut rien faire d'autre que
/// nommer un commit : elle finit dans un chemin d'URL.
fn verifier_reference(reference: &str) -> Result<(), AppError> {
    let valide = !reference.is_empty()
        && reference.len() <= LONGUEUR_MAX_REFERENCE
        && !reference.starts_with('.')
        && !reference.contains("..")
        && reference
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    if valide {
        Ok(())
    } else {
        Err(AppError::Configuration {
            detail: format!(
                "référence refusée : seuls un commit ou une étiquette simples sont acceptés \
                 ({} caractères au plus, lettres, chiffres, « . », « _ », « - »)",
                LONGUEUR_MAX_REFERENCE
            ),
        })
    }
}

impl VerificateurGates for VerificateurGatesGithub {
    fn etat_plancher(&self, reference: &str) -> Result<Vec<EtatGate>, AppError> {
        if self.correspondance.is_empty() {
            return Err(AppError::Configuration {
                detail: "aucune gate du plancher configurée : un plancher vide ne vaut pas \
                         un plancher vert"
                    .to_string(),
            });
        }
        verifier_reference(reference)?;

        let url = format!(
            "{}/repos/{}/{}/commits/{reference}/check-runs?per_page=100",
            self.api, self.proprietaire, self.depot
        );
        let corps: serde_json::Value = match self
            .agent
            .get(&url)
            .header("Authorization", &format!("Bearer {}", self.jeton.exposer()))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .call()
        {
            Ok(mut r) => r
                .body_mut()
                .read_json()
                .map_err(|e| self.erreur(e.to_string()))?,
            Err(ureq::Error::StatusCode(401)) | Err(ureq::Error::StatusCode(403)) => {
                return Err(AppError::Authentification {
                    secret: Redacted::new("jeton GitHub".to_string()),
                })
            }
            Err(ureq::Error::StatusCode(404)) | Err(ureq::Error::StatusCode(422)) => {
                return Err(AppError::Introuvable {
                    quoi: format!(
                        "référence « {reference} » dans {}/{}",
                        self.proprietaire, self.depot
                    ),
                })
            }
            Err(e) => return Err(self.erreur(e.to_string())),
        };

        let runs = corps
            .get("check_runs")
            .and_then(|r| r.as_array())
            .cloned()
            .unwrap_or_default();

        Ok(self
            .correspondance
            .iter()
            .map(|(gate, nom)| etat_d_une_gate(*gate, nom, &runs))
            .collect())
    }
}

/// Une gate est verte si **tous** les check-runs qui portent son nom sont
/// terminés en succès, et s'il en existe au moins un.
fn etat_d_une_gate(gate: GatePlancher, nom: &str, runs: &[serde_json::Value]) -> EtatGate {
    let siens: Vec<&serde_json::Value> = runs
        .iter()
        .filter(|run| run.get("name").and_then(|n| n.as_str()) == Some(nom))
        .collect();

    if siens.is_empty() {
        return EtatGate {
            gate,
            verte: false,
            detail: format!("check « {nom} » absent de la référence"),
        };
    }

    for run in &siens {
        let statut = run.get("status").and_then(|s| s.as_str()).unwrap_or("");
        let conclusion = run.get("conclusion").and_then(|c| c.as_str()).unwrap_or("");
        if statut != "completed" {
            return EtatGate {
                gate,
                verte: false,
                detail: format!("check « {nom} » pas terminé ({statut})"),
            };
        }
        if conclusion != "success" {
            return EtatGate {
                gate,
                verte: false,
                detail: format!("check « {nom} » conclu « {conclusion} »"),
            };
        }
    }

    EtatGate {
        gate,
        verte: true,
        detail: format!("check « {nom} » en succès"),
    }
}
