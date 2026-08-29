//! Passerelle d'approbation par environnement GitHub protégé.
//!
//! C'est ADR-008, et c'est la décision la plus structurante du projet.
//!
//! Une action de Tier 1 ne s'exécute pas ici. Sluis déclenche un
//! `workflow_dispatch` en passant l'empreinte du plan ; le travail est lié à un
//! **environnement GitHub protégé** avec relecteurs requis, donc GitHub le
//! bloque et notifie l'humain. À l'approbation seulement, le travail s'exécute,
//! **avec des secrets qui vivent dans GitHub Actions et non dans Sluis**.
//!
//! Propriété obtenue : même compromis, Sluis ne peut pas muter la production,
//! parce qu'il n'en a pas les clés. C'est aussi ce qui rend acceptable son
//! co-hébergement avec d'autres services. Si Sluis venait un jour à détenir ces
//! clés, ADR-008 tomberait.
//!
//! Le jeton utilisé ici n'ouvre qu'un droit : déclencher un workflow. Il ne
//! donne aucun accès à l'infrastructure.

use std::time::Duration;

use crate::application::ports::{EtatApprobation, PasserelleApprobation};
use crate::domain::{AppError, Empreinte, PlanChangement, Redacted};

/// Passerelle GitHub Actions.
pub struct PasserelleGithub {
    proprietaire: String,
    depot: String,
    workflow: String,
    reference: String,
    jeton: Redacted<String>,
    api: String,
    agent: ureq::Agent,
}

impl std::fmt::Debug for PasserelleGithub {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PasserelleGithub")
            .field("depot", &format!("{}/{}", self.proprietaire, self.depot))
            .field("workflow", &self.workflow)
            .finish_non_exhaustive()
    }
}

impl PasserelleGithub {
    /// Construit la passerelle.
    pub fn new(
        proprietaire: String,
        depot: String,
        workflow: String,
        reference: String,
        jeton: Redacted<String>,
        api: String,
    ) -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(30)))
            .build()
            .into();
        Self {
            proprietaire,
            depot,
            workflow,
            reference,
            jeton,
            api,
            agent,
        }
    }

    fn url(&self, suffixe: &str) -> String {
        format!(
            "{}/repos/{}/{}{suffixe}",
            self.api, self.proprietaire, self.depot
        )
    }

    fn erreur(&self, detail: String) -> AppError {
        AppError::ServiceTiers {
            service: "GitHub".to_string(),
            detail,
        }
    }
}

impl PasserelleApprobation for PasserelleGithub {
    fn soumettre(&self, plan: &PlanChangement) -> Result<EtatApprobation, AppError> {
        // L'empreinte voyage en entrée du workflow : c'est elle qui lie
        // l'approbation humaine à un plan précis, et pas à un autre.
        let corps = serde_json::json!({
            "ref": self.reference,
            "inputs": {
                "empreinte": plan.empreinte().hexadecimal(),
                "action": plan.action().nom(),
                "environnement": plan.environnement().nom(),
                "cible": plan.cible(),
            }
        });

        let reponse = self
            .agent
            .post(self.url(&format!("/actions/workflows/{}/dispatches", self.workflow)))
            .header("Authorization", &format!("Bearer {}", self.jeton.exposer()))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send_json(&corps);

        match reponse {
            Ok(_) => Ok(EtatApprobation::EnAttente {
                run: plan.empreinte().abregee().to_string(),
                url: format!(
                    "https://github.com/{}/{}/actions/workflows/{}",
                    self.proprietaire, self.depot, self.workflow
                ),
            }),
            Err(ureq::Error::StatusCode(404)) => Err(self.erreur(format!(
                "workflow « {} » introuvable, ou dépôt sans environnement protégé : \
                 la soumission est refusée plutôt qu'exécutée sans garde",
                self.workflow
            ))),
            Err(ureq::Error::StatusCode(401)) | Err(ureq::Error::StatusCode(403)) => {
                Err(AppError::Authentification {
                    secret: Redacted::new("jeton GitHub".to_string()),
                })
            }
            Err(e) => Err(self.erreur(e.to_string())),
        }
    }

    fn interroger(&self, empreinte: &Empreinte) -> Result<EtatApprobation, AppError> {
        let mut reponse = self
            .agent
            .get(self.url("/actions/runs?event=workflow_dispatch&per_page=20"))
            .header("Authorization", &format!("Bearer {}", self.jeton.exposer()))
            .header("Accept", "application/vnd.github+json")
            .call()
            .map_err(|e| self.erreur(e.to_string()))?;

        let corps: serde_json::Value = reponse
            .body_mut()
            .read_json()
            .map_err(|e| self.erreur(e.to_string()))?;

        let runs = corps
            .get("workflow_runs")
            .and_then(|r| r.as_array())
            .ok_or_else(|| self.erreur("réponse sans workflow_runs".to_string()))?;

        for run in runs {
            let nom = run.get("name").and_then(|n| n.as_str()).unwrap_or_default();
            let titre = run
                .get("display_title")
                .and_then(|n| n.as_str())
                .unwrap_or_default();
            if !nom.contains(empreinte.abregee()) && !titre.contains(empreinte.abregee()) {
                continue;
            }
            let statut = run.get("status").and_then(|s| s.as_str()).unwrap_or("");
            let conclusion = run.get("conclusion").and_then(|c| c.as_str()).unwrap_or("");
            let identifiant = run
                .get("id")
                .map(|i| i.to_string())
                .unwrap_or_else(|| empreinte.abregee().to_string());

            return Ok(match (statut, conclusion) {
                ("completed", "success") => EtatApprobation::Approuvee {
                    approbateur: run
                        .pointer("/triggering_actor/login")
                        .and_then(|a| a.as_str())
                        .unwrap_or("inconnu")
                        .to_string(),
                    run: identifiant,
                },
                ("completed", "cancelled") => EtatApprobation::Refusee {
                    motif: "approbation refusée ou exécution annulée".to_string(),
                },
                ("completed", autre) => EtatApprobation::Echouee {
                    detail: format!("conclusion « {autre} »"),
                },
                _ => EtatApprobation::EnAttente {
                    run: identifiant,
                    url: run
                        .get("html_url")
                        .and_then(|u| u.as_str())
                        .unwrap_or_default()
                        .to_string(),
                },
            });
        }
        Err(AppError::Introuvable {
            quoi: format!("aucun run pour l'empreinte {}", empreinte.abregee()),
        })
    }
}
