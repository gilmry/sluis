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
//! Le jeton utilisé ici déclenche un workflow et lit l'environnement, les runs
//! et leurs approbations. Il ne donne aucun accès à l'infrastructure, et son
//! identité ne doit être celle d'aucun relecteur (ADR-012).

use std::time::Duration;

use crate::application::ports::{EtatApprobation, PasserelleApprobation};
use crate::domain::section_empreinte::{verifier_section, GABARIT_TIER1};
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

    /// Vérifie, avant tout dispatch, que l'environnement cible existe et qu'il
    /// impose une approbation humaine que le titulaire du jeton ne peut pas
    /// donner lui-même.
    ///
    /// Sans cette lecture, un environnement absent serait créé par GitHub
    /// **sans protection** au premier run, et le job s'exécuterait sans que
    /// personne l'approuve (faille S1).
    fn verifier_environnement(&self, environnement: &str) -> Result<(), AppError> {
        let refus = |raison: &str| AppError::EnvironnementNonProtege {
            environnement: environnement.to_string(),
            raison: raison.to_string(),
        };

        let reponse = self
            .agent
            .get(self.url(&format!("/environments/{environnement}")))
            .header("Authorization", &format!("Bearer {}", self.jeton.exposer()))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .call();

        let corps: serde_json::Value = match reponse {
            Ok(mut r) => r
                .body_mut()
                .read_json()
                .map_err(|e| self.erreur(e.to_string()))?,
            Err(ureq::Error::StatusCode(404)) => {
                return Err(refus(
                    "n'existe pas ; GitHub le créerait sans protection au premier run",
                ))
            }
            Err(ureq::Error::StatusCode(401)) | Err(ureq::Error::StatusCode(403)) => {
                return Err(AppError::Authentification {
                    secret: Redacted::new("jeton GitHub".to_string()),
                })
            }
            Err(e) => return Err(self.erreur(e.to_string())),
        };

        let relecteurs = corps
            .get("protection_rules")
            .and_then(|r| r.as_array())
            .and_then(|regles| {
                regles.iter().find(|regle| {
                    regle.get("type").and_then(|t| t.as_str()) == Some("required_reviewers")
                })
            });

        let Some(regle) = relecteurs else {
            return Err(refus("n'exige aucun relecteur"));
        };
        let nombre = regle
            .get("reviewers")
            .and_then(|r| r.as_array())
            .map_or(0, Vec::len);
        if nombre == 0 {
            return Err(refus("exige des relecteurs mais n'en désigne aucun"));
        }
        // Absent vaut inactif : on ne présume jamais une garde qu'on n'a pas lue.
        if regle.get("prevent_self_review").and_then(|p| p.as_bool()) != Some(true) {
            return Err(refus(
                "laisse le titulaire du jeton approuver son propre dispatch \
                 (prevent_self_review inactif)",
            ));
        }
        Ok(())
    }

    /// Vérifie, avant tout dispatch, que le workflow du dépôt cible porte la
    /// section `verification-empreinte` du gabarit, intacte (ADR-034, point 6).
    ///
    /// Lu sur la branche où le dispatch sera déclenché : c'est cette version
    /// du fichier que GitHub exécutera. Toute erreur de lecture refuse.
    fn verifier_workflow(&self) -> Result<(), AppError> {
        let refus = |raison: String| AppError::WorkflowNonConforme {
            workflow: self.workflow.clone(),
            raison,
        };

        let reponse = self
            .agent
            .get(self.url(&format!(
                "/contents/.github/workflows/{}?ref={}",
                self.workflow, self.reference
            )))
            .header("Authorization", &format!("Bearer {}", self.jeton.exposer()))
            .header("Accept", "application/vnd.github.raw+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .call();

        let contenu = match reponse {
            Ok(mut r) => r
                .body_mut()
                .read_to_string()
                .map_err(|e| refus(format!("est illisible : {e}")))?,
            Err(ureq::Error::StatusCode(401)) | Err(ureq::Error::StatusCode(403)) => {
                return Err(AppError::Authentification {
                    secret: Redacted::new("jeton GitHub".to_string()),
                })
            }
            Err(ureq::Error::StatusCode(404)) => {
                return Err(refus(format!(
                    "est introuvable sur la branche « {} »",
                    self.reference
                )))
            }
            Err(e) => return Err(refus(format!("n'a pas pu être lu : {e}"))),
        };

        verifier_section(GABARIT_TIER1, &contenu)
            .map_err(|raison| refus(format!("n'est pas conforme au gabarit : {raison}")))
    }

    /// Lecture JSON authentifiée.
    fn lire(&self, url: &str) -> Result<serde_json::Value, AppError> {
        let mut reponse = self
            .agent
            .get(url)
            .header("Authorization", &format!("Bearer {}", self.jeton.exposer()))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .call()
            .map_err(|e| match e {
                ureq::Error::StatusCode(401) | ureq::Error::StatusCode(403) => {
                    AppError::Authentification {
                        secret: Redacted::new("jeton GitHub".to_string()),
                    }
                }
                autre => self.erreur(autre.to_string()),
            })?;
        reponse
            .body_mut()
            .read_json()
            .map_err(|e| self.erreur(e.to_string()))
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
        self.verifier_environnement(plan.environnement().nom())?;
        self.verifier_workflow()?;

        // L'empreinte voyage en entrée du workflow avec **tout ce qui la
        // compose** : le job la recalcule avant toute mutation et s'arrête si
        // elle ne correspond pas. Sans cela, une approbation donnée pour une
        // empreinte vaudrait pour n'importe quelles entrées (faille S2).
        let corps = serde_json::json!({
            "ref": self.reference,
            "inputs": {
                "empreinte": plan.empreinte().hexadecimal(),
                "action": plan.action().nom(),
                "environnement": plan.environnement().nom(),
                "tier": format!("{:?}", plan.tier()),
                "cible": plan.cible(),
                "description": plan.description(),
                "diff": plan.diff(),
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
                commande: None,
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
        let corps = self.lire(&self.url("/actions/runs?event=workflow_dispatch&per_page=20"))?;

        let runs = corps
            .get("workflow_runs")
            .and_then(|r| r.as_array())
            .ok_or_else(|| self.erreur("réponse sans workflow_runs".to_string()))?;

        // L'empreinte complète, pas l'abrégée : le titre du run la porte en
        // entier (`run-name` du gabarit Tier 1), et un préfixe pourrait
        // désigner le run d'un autre plan.
        let Some(run) = runs.iter().find(|run| {
            run.get("display_title")
                .and_then(|t| t.as_str())
                .is_some_and(|titre| titre.contains(empreinte.hexadecimal()))
        }) else {
            return Err(AppError::Introuvable {
                quoi: format!("aucun run pour l'empreinte {}", empreinte.abregee()),
            });
        };

        let statut = run.get("status").and_then(|s| s.as_str()).unwrap_or("");
        let identifiant = run
            .get("id")
            .and_then(|i| i.as_u64())
            .ok_or_else(|| self.erreur("run sans identifiant".to_string()))?;

        if statut != "completed" {
            return Ok(EtatApprobation::EnAttente {
                run: identifiant.to_string(),
                url: run
                    .get("html_url")
                    .and_then(|u| u.as_str())
                    .unwrap_or_default()
                    .to_string(),
                commande: None,
            });
        }

        // Qui a approuvé se lit dans les approbations du run, jamais dans
        // `triggering_actor` : celui-là a lancé ou relancé le run, il n'a pas
        // forcément approuvé (failles H-b et S3).
        let approbations =
            self.lire(&self.url(&format!("/actions/runs/{identifiant}/approvals")))?;
        let approbations = approbations.as_array().cloned().unwrap_or_default();
        let par_etat = |etat: &str| {
            approbations
                .iter()
                .filter(|a| a.get("state").and_then(|s| s.as_str()) == Some(etat))
                .filter_map(|a| a.pointer("/user/login").and_then(|l| l.as_str()))
                .map(str::to_string)
                .collect::<Vec<_>>()
        };

        let rejets = par_etat("rejected");
        if let Some(relecteur) = rejets.first() {
            return Ok(EtatApprobation::Refusee {
                motif: format!("approbation rejetée par {relecteur}"),
            });
        }

        let conclusion = run.get("conclusion").and_then(|c| c.as_str()).unwrap_or("");
        Ok(match (conclusion, par_etat("approved").last()) {
            ("success", Some(approbateur)) => EtatApprobation::Approuvee {
                approbateur: approbateur.clone(),
                run: identifiant.to_string(),
            },
            // Exécuté sans approbation enregistrée : la garde n'a pas joué.
            // Ce n'est jamais une approbation.
            ("success", None) => EtatApprobation::Echouee {
                detail: format!(
                    "run {identifiant} exécuté sans aucune approbation humaine enregistrée"
                ),
            },
            ("cancelled", _) => EtatApprobation::Refusee {
                motif: "exécution annulée".to_string(),
            },
            (autre, _) => EtatApprobation::Echouee {
                detail: format!("conclusion « {autre} »"),
            },
        })
    }
}
