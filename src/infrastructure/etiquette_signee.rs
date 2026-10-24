//! Passerelle d'approbation par étiquette git signée (ADR-013).
//!
//! Sur un dépôt privé hors offre Enterprise, GitHub n'offre pas de relecteurs
//! requis : la passerelle d'ADR-008 ne peut pas y garder le Tier 1. L'acte
//! d'approbation devient une étiquette `approuve/<empreinte courte>`, signée
//! par la clé du superviseur sur **son poste**, posée sur le commit à mettre
//! en ligne, et dont le message nomme l'environnement et l'empreinte complète.
//!
//! **Sluis prépare et lit, il ne signe jamais.** `soumettre` résout la
//! référence en commit et rend la commande exacte à signer ; il ne déclenche
//! aucun workflow. `interroger` lit l'étiquette et rapporte l'état. Le jeton
//! n'est employé qu'en **lecture** (refs, objets git, commits) ; aucune
//! étiquette n'est poussée d'ici, aucune clé de signature n'est lue ni
//! configurée.
//!
//! **L'état rendu est informatif.** La vérification qui fait foi est celle du
//! tireur GitOps de la cible, contre un `allowed_signers` conservé hors du
//! dépôt. Ce que Sluis lit ici, c'est la vérification de GitHub, utile pour
//! dire au superviseur où il en est, jamais pour déployer.
//!
//! **Le registre des plans soumis vit en mémoire.** Pour juger une étiquette,
//! il faut savoir sur quel commit et pour quel environnement le plan a été
//! soumis ; l'empreinte seule ne le dit pas. Un processus redémarré ne connaît
//! plus les plans d'avant : il le dit, et le plan se resoumet (les gates sont
//! rejouées, l'empreinte reste la même pour les mêmes entrées).

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use crate::application::ports::{EtatApprobation, PasserelleApprobation};
use crate::domain::{
    commande_signature, nom_etiquette_approbation, Action, AppError, Empreinte, Environnement,
    MessageEtiquette, PlanChangement, Redacted,
};
use crate::infrastructure::gates_github::verifier_reference;

/// Ce que la passerelle retient d'un plan soumis.
#[derive(Debug, Clone)]
struct PlanSoumis {
    environnement: Environnement,
    commit: String,
}

/// Passerelle par étiquette signée.
pub struct PasserelleEtiquetteSignee {
    proprietaire: String,
    depot: String,
    jeton: Redacted<String>,
    api: String,
    agent: ureq::Agent,
    soumis: Mutex<HashMap<Empreinte, PlanSoumis>>,
}

impl std::fmt::Debug for PasserelleEtiquetteSignee {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PasserelleEtiquetteSignee")
            .field("depot", &format!("{}/{}", self.proprietaire, self.depot))
            .finish_non_exhaustive()
    }
}

/// Un identifiant de commit git : 40 (SHA-1) ou 64 (SHA-256) chiffres
/// hexadécimaux minuscules.
fn est_un_commit(texte: &str) -> bool {
    matches!(texte.len(), 40 | 64)
        && texte
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

/// Le message de l'étiquette tel que git l'a écrit, ramené à ce que le
/// superviseur a passé à `-m`.
///
/// Deux artefacts du transport git, et eux seuls, sont retirés : le bloc de
/// signature qu'une étiquette signée porte à la suite de son message, et le
/// saut de ligne final que git ajoute. Tout le reste passe tel quel à
/// l'analyse stricte du domaine.
fn message_signe(brut: &str) -> &str {
    let sans_signature = brut
        .find("\n-----BEGIN ")
        .map_or(brut, |position| &brut[..=position]);
    sans_signature.trim_end_matches('\n')
}

impl PasserelleEtiquetteSignee {
    /// Construit la passerelle.
    pub fn new(proprietaire: String, depot: String, jeton: Redacted<String>, api: String) -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(30)))
            .build()
            .into();
        Self {
            proprietaire,
            depot,
            jeton,
            api,
            agent,
            soumis: Mutex::new(HashMap::new()),
        }
    }

    fn url(&self, suffixe: &str) -> String {
        format!(
            "{}/repos/{}/{}{suffixe}",
            self.api, self.proprietaire, self.depot
        )
    }

    fn url_etiquettes(&self) -> String {
        format!(
            "https://github.com/{}/{}/tags",
            self.proprietaire, self.depot
        )
    }

    fn erreur(&self, detail: String) -> AppError {
        AppError::ServiceTiers {
            service: "GitHub".to_string(),
            detail,
        }
    }

    /// Lecture JSON authentifiée. `Ok(None)` pour un 404 : une étiquette pas
    /// encore posée n'est pas une panne.
    ///
    /// C'est **le seul appel réseau** de cette passerelle, et c'est un `GET`.
    fn lire(&self, url: &str) -> Result<Option<serde_json::Value>, AppError> {
        let reponse = self
            .agent
            .get(url)
            .header("Authorization", &format!("Bearer {}", self.jeton.exposer()))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .call();
        match reponse {
            Ok(mut r) => r
                .body_mut()
                .read_json()
                .map(Some)
                .map_err(|e| self.erreur(e.to_string())),
            Err(ureq::Error::StatusCode(404)) => Ok(None),
            Err(ureq::Error::StatusCode(401)) | Err(ureq::Error::StatusCode(403)) => {
                Err(AppError::Authentification {
                    secret: Redacted::new("jeton GitHub".to_string()),
                })
            }
            Err(e) => Err(self.erreur(e.to_string())),
        }
    }

    fn verrou(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, HashMap<Empreinte, PlanSoumis>>, AppError> {
        self.soumis.lock().map_err(|_| AppError::Configuration {
            detail: "registre des plans soumis inutilisable (verrou empoisonné)".to_string(),
        })
    }

    fn en_attente(&self, empreinte: &Empreinte, plan: &PlanSoumis) -> EtatApprobation {
        let message = MessageEtiquette::new(plan.environnement, empreinte.clone());
        EtatApprobation::EnAttente {
            run: nom_etiquette_approbation(empreinte),
            url: self.url_etiquettes(),
            commande: Some(commande_signature(&message, &plan.commit)),
        }
    }

    /// Juge une étiquette déjà posée.
    fn juger(
        &self,
        empreinte: &Empreinte,
        plan: &PlanSoumis,
        reference: &serde_json::Value,
    ) -> Result<EtatApprobation, AppError> {
        let refus = |motif: String| Ok(EtatApprobation::Refusee { motif });
        let nom = nom_etiquette_approbation(empreinte);

        // Une étiquette légère pointe directement le commit : elle ne porte
        // ni message ni signature, elle ne vaut jamais approbation.
        let type_objet = reference
            .pointer("/object/type")
            .and_then(|t| t.as_str())
            .unwrap_or_default();
        if type_objet != "tag" {
            return refus(format!(
                "étiquette {nom} non annotée, donc non signée : `git tag -s` attendu"
            ));
        }
        let sha_etiquette = reference
            .pointer("/object/sha")
            .and_then(|s| s.as_str())
            .filter(|s| est_un_commit(s))
            .ok_or_else(|| self.erreur("référence d'étiquette sans objet".to_string()))?;

        let etiquette = self
            .lire(&self.url(&format!("/git/tags/{sha_etiquette}")))?
            .ok_or_else(|| self.erreur(format!("objet d'étiquette {sha_etiquette} illisible")))?;

        // 1. Le commit désigné.
        let cible = etiquette
            .pointer("/object/sha")
            .and_then(|s| s.as_str())
            .unwrap_or_default();
        let type_cible = etiquette
            .pointer("/object/type")
            .and_then(|t| t.as_str())
            .unwrap_or_default();
        if type_cible != "commit" || cible != plan.commit {
            return refus(format!(
                "étiquette sur un autre commit : {nom} désigne {cible}, le plan vise {}",
                plan.commit
            ));
        }

        // 2. Le message : analysé strictement, puis confronté au plan.
        let brut = etiquette
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or_default();
        let message = match MessageEtiquette::analyser(message_signe(brut)) {
            Ok(message) => message,
            Err(e) => return refus(format!("message d'étiquette mal formé : {e}")),
        };
        if message.environnement() != plan.environnement {
            return refus(format!(
                "étiquette pour un autre environnement : le message nomme {}, le plan vise {}",
                message.environnement(),
                plan.environnement
            ));
        }
        if message.empreinte() != empreinte {
            return refus(format!(
                "étiquette pour une autre empreinte : le message porte {}, le plan est {}",
                message.empreinte().abregee(),
                empreinte.abregee()
            ));
        }

        // 3. La signature, telle que GitHub l'a vérifiée. L'approbateur est
        // l'identité signée, jamais celle du jeton qui lit.
        let verifiee = etiquette
            .pointer("/verification/verified")
            .and_then(|v| v.as_bool())
            == Some(true);
        if !verifiee {
            let raison = etiquette
                .pointer("/verification/reason")
                .and_then(|r| r.as_str())
                .unwrap_or("vérification absente");
            return refus(format!(
                "signature de {nom} non vérifiée par GitHub ({raison})"
            ));
        }
        let Some(approbateur) = etiquette
            .pointer("/tagger/email")
            .and_then(|e| e.as_str())
            .filter(|e| !e.trim().is_empty())
        else {
            return refus(format!("étiquette {nom} sans identité de signataire"));
        };

        Ok(EtatApprobation::Approuvee {
            approbateur: approbateur.to_string(),
            run: nom,
        })
    }
}

impl PasserelleApprobation for PasserelleEtiquetteSignee {
    fn soumettre(&self, plan: &PlanChangement) -> Result<EtatApprobation, AppError> {
        // Le renouvellement de la dérogation (ADR-012, partie 3) reste sur la
        // passerelle d'ADR-008 : sa preuve est lue par l'exécutant dans les
        // approbations d'un run, qu'aucune étiquette ne produit.
        if plan.action() != &Action::MiseEnLigne {
            return Err(AppError::Configuration {
                detail: format!(
                    "la passerelle etiquette-signee ne porte que la mise en ligne, pas {}",
                    plan.action()
                ),
            });
        }
        let reference = plan
            .cible()
            .rsplit_once('@')
            .map(|(_, reference)| reference)
            .ok_or_else(|| AppError::Configuration {
                detail: "plan de mise en ligne sans référence (`projet@référence`)".to_string(),
            })?;
        verifier_reference(reference)?;

        let commit = self
            .lire(&self.url(&format!("/commits/{reference}")))?
            .ok_or_else(|| AppError::Introuvable {
                quoi: format!("référence « {reference} » dans {}", self.depot),
            })?
            .get("sha")
            .and_then(|s| s.as_str())
            .filter(|s| est_un_commit(s))
            .map(str::to_string)
            .ok_or_else(|| self.erreur(format!("référence « {reference} » sans commit")))?;

        let soumis = PlanSoumis {
            environnement: plan.environnement(),
            commit,
        };
        let etat = self.en_attente(plan.empreinte(), &soumis);
        self.verrou()?.insert(plan.empreinte().clone(), soumis);
        Ok(etat)
    }

    fn interroger(&self, empreinte: &Empreinte) -> Result<EtatApprobation, AppError> {
        let Some(plan) = self.verrou()?.get(empreinte).cloned() else {
            return Err(AppError::Introuvable {
                quoi: format!(
                    "aucun plan soumis par ce processus pour l'empreinte {} \
                     (registre en mémoire : resoumettre la mise en ligne)",
                    empreinte.abregee()
                ),
            });
        };

        let nom = nom_etiquette_approbation(empreinte);
        match self.lire(&self.url(&format!("/git/ref/tags/{nom}")))? {
            None => Ok(self.en_attente(empreinte, &plan)),
            Some(reference) => self.juger(empreinte, &plan, &reference),
        }
    }
}
