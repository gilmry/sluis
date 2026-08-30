//! Ports des moteurs d'exécution externes.

use crate::domain::{
    AppError, BailBacASable, MutationTerraform, PlanTerraform, StatutArgocd, StatutHelm, ValeurSure,
};

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

    /// Prépare le module : fournisseurs et état distant.
    fn initialiser(&self, module: &ValeurSure) -> Result<(), AppError>;

    /// Applique.
    ///
    /// **Le bail n'est pas utilisé par l'implémentation, il est exigé par le
    /// type.** C'est la garantie centrale de cette signature : il n'existe
    /// aucun chemin d'appel qui applique sans qu'un [`BailBacASable`] ait été
    /// loué, donc sans dérogation valide, sans TTL et sans plafond. Une
    /// vérification équivalente écrite dans le corps de la méthode serait
    /// contournable par un second appelant ; celle-ci ne l'est pas.
    ///
    /// Hors bac à sable, Sluis ne mute pas : il demande, par la passerelle
    /// d'ADR-008.
    fn appliquer(
        &self,
        module: &ValeurSure,
        bail: &BailBacASable,
    ) -> Result<MutationTerraform, AppError>;

    /// Détruit, **sans rien exiger**.
    ///
    /// L'asymétrie avec `appliquer` est délibérée. Le chien de garde détruit
    /// des baux échus, donc dont la dérogation peut avoir expiré : réclamer
    /// une preuve de validité ici rendrait le nettoyage impossible au moment
    /// précis où il devient obligatoire.
    fn detruire(&self, module: &ValeurSure) -> Result<MutationTerraform, AppError>;

    /// Lit les sorties déclarées du module, par exemple l'adresse de la cible.
    fn sorties(&self, module: &ValeurSure) -> Result<Vec<(String, String)>, AppError>;
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
