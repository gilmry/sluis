//! Ports des moteurs d'exécution externes.

use crate::domain::{AppError, PlanTerraform, StatutArgocd, StatutHelm, ValeurSure};

/// Sortie brute d'un processus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortieProcessus {
    /// Code de retour.
    pub code: i32,
    /// Sortie standard.
    pub sortie: String,
    /// Sortie d'erreur.
    pub erreur: String,
}

impl SortieProcessus {
    /// Vrai si le processus a réussi.
    pub fn reussi(&self) -> bool {
        self.code == 0
    }
}

/// Exécute un programme externe.
///
/// **Ne prend jamais une ligne de commande, toujours un programme et un
/// tableau d'arguments.** Il n'existe donc aucun point du code où un shell
/// pourrait interpréter une valeur venue d'un appelant.
pub trait Executeur: Send + Sync {
    /// Lance `programme` avec `arguments`.
    ///
    /// Rend `AppError::EngineMissing` si le binaire est absent, ce qui est
    /// l'état normal d'une machine de développement et non une panne.
    fn executer(
        &self,
        programme: &str,
        arguments: &[String],
        dossier: Option<&str>,
    ) -> Result<SortieProcessus, AppError>;
}

/// Pilote Terraform.
pub trait MoteurTerraform: Send + Sync {
    /// Produit un plan, **sans rien appliquer**.
    fn plan(&self, module: &ValeurSure) -> Result<PlanTerraform, AppError>;
}

/// Pilote Helm.
pub trait MoteurHelm: Send + Sync {
    /// Statut d'une release.
    fn statut(&self, release: &ValeurSure, espace: &ValeurSure) -> Result<StatutHelm, AppError>;
    /// Historique des révisions.
    fn historique(
        &self,
        release: &ValeurSure,
        espace: &ValeurSure,
    ) -> Result<Vec<StatutHelm>, AppError>;
}

/// Pilote Kustomize.
pub trait MoteurKustomize: Send + Sync {
    /// Rend une surcouche. Les valeurs de `Secret` sont masquées.
    fn rendre(&self, chemin: &ValeurSure) -> Result<String, AppError>;
}

/// Pilote ArgoCD.
pub trait MoteurArgocd: Send + Sync {
    /// Statut d'une application.
    fn statut_application(&self, application: &ValeurSure) -> Result<StatutArgocd, AppError>;
}
