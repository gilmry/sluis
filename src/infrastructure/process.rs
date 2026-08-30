//! Runners des moteurs externes.
//!
//! Trois décisions structurent ce module.
//!
//! **Jamais de shell.** [`ExecuteurSysteme`] passe un programme et un tableau
//! d'arguments à `std::process::Command`. Il n'existe aucun point où une chaîne
//! serait interprétée, donc aucune surface d'injection par métacaractère.
//!
//! **Allowlist d'exécutables.** Seuls les six moteurs attendus peuvent être
//! lancés. Un appelant qui demanderait `sh` ou `curl` est refusé, même si son
//! argument est par ailleurs valide.
//!
//! **L'absence n'est pas une panne.** Un binaire manquant produit
//! `AppError::EngineMissing` nommant le binaire, ce qui est l'état normal d'une
//! machine de développement nue et doit se lire comme tel.

use std::process::Command;

use crate::application::ports::{
    Executeur, MoteurArgocd, MoteurHelm, MoteurKustomize, MoteurTerraform, SortieProcessus,
};
use crate::domain::{
    AppError, BailBacASable, MutationTerraform, PlanTerraform, StatutArgocd, StatutHelm, ValeurSure,
};

/// Les seuls exécutables que Sluis a le droit de lancer.
pub const EXECUTABLES_AUTORISES: &[&str] = &[
    "terraform",
    "ansible-playbook",
    "helm",
    "kubectl",
    "kustomize",
    "argocd",
    "wrk",
];

/// Exécuteur réel, sur `std::process::Command`.
#[derive(Debug, Default, Clone, Copy)]
pub struct ExecuteurSysteme;

impl Executeur for ExecuteurSysteme {
    fn executer(
        &self,
        programme: &str,
        arguments: &[String],
        dossier: Option<&str>,
    ) -> Result<SortieProcessus, AppError> {
        if !EXECUTABLES_AUTORISES.contains(&programme) {
            return Err(AppError::Configuration {
                detail: format!(
                    "exécutable « {programme} » hors allowlist : seuls {} sont autorisés",
                    EXECUTABLES_AUTORISES.join(", ")
                ),
            });
        }
        let mut commande = Command::new(programme);
        commande.args(arguments);
        if let Some(dossier) = dossier {
            commande.current_dir(dossier);
        }
        match commande.output() {
            Ok(sortie) => Ok(SortieProcessus {
                code: sortie.status.code().unwrap_or(-1),
                sortie: String::from_utf8_lossy(&sortie.stdout).to_string(),
                erreur: String::from_utf8_lossy(&sortie.stderr).to_string(),
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(AppError::EngineMissing {
                binaire: programme.to_string(),
            }),
            Err(e) => Err(AppError::ServiceTiers {
                service: programme.to_string(),
                detail: e.to_string(),
            }),
        }
    }
}

/// Pilote Terraform.
pub struct Terraform<E: Executeur> {
    executeur: E,
}

impl<E: Executeur> Terraform<E> {
    /// Construit le pilote.
    pub fn new(executeur: E) -> Self {
        Self { executeur }
    }

    /// Lance terraform dans le module et traduit un code non nul en erreur.
    ///
    /// Un seul endroit fait cette traduction : disséminée, elle finirait par
    /// diverger, et une commande finirait par avaler son propre échec.
    fn executer_terraform(
        &self,
        module: &ValeurSure,
        arguments: &[String],
    ) -> Result<String, AppError> {
        let sortie = self
            .executeur
            .executer("terraform", arguments, Some(module.valeur()))?;
        if !sortie.reussi() {
            return Err(AppError::ServiceTiers {
                service: "terraform".to_string(),
                detail: format!("code {} : {}", sortie.code, sortie.erreur.trim()),
            });
        }
        Ok(sortie.sortie)
    }
}

/// Analyse la ligne de résumé d'un plan Terraform.
///
/// Format visé : `Plan: 3 to add, 1 to change, 0 to destroy.`
/// Le format `No changes.` est traité comme un plan à zéro, qui est la preuve
/// de convergence au sens de `convergence-iac.md`.
pub fn analyser_resume_plan(sortie: &str) -> PlanTerraform {
    let mut creations = 0;
    let mut modifications = 0;
    let mut destructions = 0;

    for ligne in sortie.lines() {
        let ligne = ligne.trim();
        if let Some(reste) = ligne.strip_prefix("Plan:") {
            for morceau in reste.split(',') {
                let mots: Vec<&str> = morceau.split_whitespace().collect();
                let Some(nombre) = mots.first().and_then(|n| n.parse::<u32>().ok()) else {
                    continue;
                };
                if morceau.contains("add") {
                    creations = nombre;
                } else if morceau.contains("change") {
                    modifications = nombre;
                } else if morceau.contains("destroy") {
                    destructions = nombre;
                }
            }
        }
    }
    PlanTerraform {
        creations,
        modifications,
        destructions,
        brut: sortie.to_string(),
    }
}

/// Analyse la ligne de résumé d'une mutation Terraform.
///
/// Deux formats, un seul analyseur :
/// `Apply complete! Resources: 3 added, 1 changed, 0 destroyed.`
/// `Destroy complete! Resources: 3 destroyed.`
///
/// Chaque nombre est rattaché au verbe qui le suit, jamais à sa position : un
/// destroy ne rend qu'un seul chiffre, et le lire comme des créations
/// inverserait le sens du compte rendu.
pub fn analyser_resume_mutation(sortie: &str) -> MutationTerraform {
    let mut creations = 0;
    let mut modifications = 0;
    let mut destructions = 0;

    for ligne in sortie.lines() {
        let Some((_, reste)) = ligne.trim().split_once("Resources:") else {
            continue;
        };
        for morceau in reste.split(',') {
            let mots: Vec<&str> = morceau.split_whitespace().collect();
            let Some(nombre) = mots.first().and_then(|n| n.parse::<u32>().ok()) else {
                continue;
            };
            if morceau.contains("added") {
                creations = nombre;
            } else if morceau.contains("changed") {
                modifications = nombre;
            } else if morceau.contains("destroyed") {
                destructions = nombre;
            }
        }
    }
    MutationTerraform {
        creations,
        modifications,
        destructions,
        brut: sortie.to_string(),
    }
}

impl<E: Executeur> MoteurTerraform for Terraform<E> {
    fn plan(&self, module: &ValeurSure) -> Result<PlanTerraform, AppError> {
        // `-detailed-exitcode` est délibérément absent : il rend 2 quand des
        // changements existent, ce qui se lirait comme un échec alors que c'est
        // le résultat normal d'un plan.
        let sortie = self.executeur.executer(
            "terraform",
            &[
                "plan".to_string(),
                "-no-color".to_string(),
                "-input=false".to_string(),
                "-lock=false".to_string(),
            ],
            Some(module.valeur()),
        )?;
        if !sortie.reussi() {
            return Err(AppError::ServiceTiers {
                service: "terraform".to_string(),
                detail: format!("code {} : {}", sortie.code, sortie.erreur.trim()),
            });
        }
        Ok(analyser_resume_plan(&sortie.sortie))
    }

    fn initialiser(&self, module: &ValeurSure) -> Result<(), AppError> {
        self.executer_terraform(
            module,
            &[
                "init".to_string(),
                "-no-color".to_string(),
                "-input=false".to_string(),
            ],
        )
        .map(|_| ())
    }

    fn appliquer(
        &self,
        module: &ValeurSure,
        _bail: &BailBacASable,
    ) -> Result<MutationTerraform, AppError> {
        // Aucun `-lock=false` : deux apply concurrents sur le même état
        // produiraient une infrastructure que plus aucun état ne décrit.
        let sortie = self.executer_terraform(
            module,
            &[
                "apply".to_string(),
                "-auto-approve".to_string(),
                "-no-color".to_string(),
                "-input=false".to_string(),
            ],
        )?;
        Ok(analyser_resume_mutation(&sortie))
    }

    fn detruire(&self, module: &ValeurSure) -> Result<MutationTerraform, AppError> {
        let sortie = self.executer_terraform(
            module,
            &[
                "destroy".to_string(),
                "-auto-approve".to_string(),
                "-no-color".to_string(),
                "-input=false".to_string(),
            ],
        )?;
        Ok(analyser_resume_mutation(&sortie))
    }

    fn sorties(&self, module: &ValeurSure) -> Result<Vec<(String, String)>, AppError> {
        let sortie = self.executer_terraform(
            module,
            &[
                "output".to_string(),
                "-json".to_string(),
                "-no-color".to_string(),
            ],
        )?;
        let brut: serde_json::Value =
            serde_json::from_str(&sortie).map_err(|e| AppError::Analyse {
                quoi: "sorties terraform".to_string(),
                detail: e.to_string(),
            })?;
        let Some(objet) = brut.as_object() else {
            return Ok(Vec::new());
        };
        Ok(objet
            .iter()
            .map(|(nom, declaration)| {
                let valeur = declaration
                    .get("value")
                    .map(|v| match v {
                        serde_json::Value::String(texte) => texte.clone(),
                        autre => autre.to_string(),
                    })
                    .unwrap_or_default();
                (nom.clone(), valeur)
            })
            .collect())
    }
}

/// Pilote Helm.
pub struct Helm<E: Executeur> {
    executeur: E,
}

impl<E: Executeur> Helm<E> {
    /// Construit le pilote.
    pub fn new(executeur: E) -> Self {
        Self { executeur }
    }
}

impl<E: Executeur> MoteurHelm for Helm<E> {
    fn statut(&self, release: &ValeurSure, espace: &ValeurSure) -> Result<StatutHelm, AppError> {
        let sortie = self.executeur.executer(
            "helm",
            &[
                "status".to_string(),
                release.valeur().to_string(),
                "-n".to_string(),
                espace.valeur().to_string(),
                "-o".to_string(),
                "json".to_string(),
            ],
            None,
        )?;
        if !sortie.reussi() {
            return Err(AppError::Introuvable {
                quoi: format!("release helm « {release} » dans « {espace} »"),
            });
        }
        let valeur: serde_json::Value =
            serde_json::from_str(&sortie.sortie).map_err(|e| AppError::Analyse {
                quoi: "statut helm".to_string(),
                detail: e.to_string(),
            })?;
        Ok(StatutHelm {
            release: release.valeur().to_string(),
            statut: valeur
                .pointer("/info/status")
                .and_then(|v| v.as_str())
                .unwrap_or("inconnu")
                .to_string(),
            revision: valeur.get("version").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        })
    }

    fn historique(
        &self,
        release: &ValeurSure,
        espace: &ValeurSure,
    ) -> Result<Vec<StatutHelm>, AppError> {
        let sortie = self.executeur.executer(
            "helm",
            &[
                "history".to_string(),
                release.valeur().to_string(),
                "-n".to_string(),
                espace.valeur().to_string(),
                "-o".to_string(),
                "json".to_string(),
            ],
            None,
        )?;
        if !sortie.reussi() {
            return Err(AppError::Introuvable {
                quoi: format!("historique de « {release} »"),
            });
        }
        let entrees: Vec<serde_json::Value> =
            serde_json::from_str(&sortie.sortie).map_err(|e| AppError::Analyse {
                quoi: "historique helm".to_string(),
                detail: e.to_string(),
            })?;
        Ok(entrees
            .into_iter()
            .map(|e| StatutHelm {
                release: release.valeur().to_string(),
                statut: e
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("inconnu")
                    .to_string(),
                revision: e.get("revision").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            })
            .collect())
    }
}

/// Pilote Kustomize.
pub struct Kustomize<E: Executeur> {
    executeur: E,
}

impl<E: Executeur> Kustomize<E> {
    /// Construit le pilote.
    pub fn new(executeur: E) -> Self {
        Self { executeur }
    }
}

/// Masque les valeurs des objets `Secret` dans un rendu Kubernetes.
///
/// Le rendu d'une surcouche contient les `data:` des Secrets, encodés en base64
/// donc lisibles par quiconque. Les rendre tels quels ferait du plus banal des
/// outils de diagnostic une fuite.
pub fn masquer_secrets(rendu: &str) -> String {
    let mut sortie = String::with_capacity(rendu.len());
    let mut dans_secret = false;
    let mut dans_data = false;
    let mut indentation_data = 0usize;

    for ligne in rendu.lines() {
        let sans_indent = ligne.trim_start();
        let indentation = ligne.len() - sans_indent.len();

        if sans_indent.starts_with("---") {
            dans_secret = false;
            dans_data = false;
        }
        if sans_indent.starts_with("kind:") && sans_indent.contains("Secret") {
            dans_secret = true;
        }
        if dans_secret
            && (sans_indent.starts_with("data:") || sans_indent.starts_with("stringData:"))
        {
            dans_data = true;
            indentation_data = indentation;
            sortie.push_str(ligne);
            sortie.push('\n');
            continue;
        }
        if dans_data {
            if indentation > indentation_data && sans_indent.contains(':') {
                let cle = sans_indent.split(':').next().unwrap_or("");
                sortie.push_str(&format!(
                    "{}{cle}: {}\n",
                    " ".repeat(indentation),
                    crate::domain::redacted::MARQUEUR
                ));
                continue;
            }
            if indentation <= indentation_data {
                dans_data = false;
            }
        }
        sortie.push_str(ligne);
        sortie.push('\n');
    }
    sortie
}

impl<E: Executeur> MoteurKustomize for Kustomize<E> {
    fn rendre(&self, chemin: &ValeurSure) -> Result<String, AppError> {
        let sortie = self.executeur.executer(
            "kustomize",
            &["build".to_string(), chemin.valeur().to_string()],
            None,
        )?;
        if !sortie.reussi() {
            return Err(AppError::Analyse {
                quoi: format!("kustomization « {chemin} »"),
                detail: sortie.erreur.trim().to_string(),
            });
        }
        Ok(masquer_secrets(&sortie.sortie))
    }
}

/// Pilote ArgoCD.
pub struct Argocd<E: Executeur> {
    executeur: E,
}

impl<E: Executeur> Argocd<E> {
    /// Construit le pilote.
    pub fn new(executeur: E) -> Self {
        Self { executeur }
    }
}

impl<E: Executeur> MoteurArgocd for Argocd<E> {
    fn statut_application(&self, application: &ValeurSure) -> Result<StatutArgocd, AppError> {
        let sortie = self.executeur.executer(
            "argocd",
            &[
                "app".to_string(),
                "get".to_string(),
                application.valeur().to_string(),
                "-o".to_string(),
                "json".to_string(),
            ],
            None,
        )?;
        if !sortie.reussi() {
            return Err(AppError::Introuvable {
                quoi: format!("application argocd « {application} »"),
            });
        }
        let valeur: serde_json::Value =
            serde_json::from_str(&sortie.sortie).map_err(|e| AppError::Analyse {
                quoi: "statut argocd".to_string(),
                detail: e.to_string(),
            })?;
        Ok(StatutArgocd {
            application: application.valeur().to_string(),
            synchronisation: valeur
                .pointer("/status/sync/status")
                .and_then(|v| v.as_str())
                .unwrap_or("inconnu")
                .to_string(),
            sante: valeur
                .pointer("/status/health/status")
                .and_then(|v| v.as_str())
                .unwrap_or("inconnu")
                .to_string(),
        })
    }
}
