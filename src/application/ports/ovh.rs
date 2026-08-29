//! Port du fournisseur OVH.

use crate::domain::{AppError, CoutCourant, EnregistrementDns, InstanceOvh, ProjetOvh};

/// Accès en lecture à l'infrastructure OVH réelle.
///
/// Le port ne connaît pas la liste d'autorisation : le filtrage est une
/// décision du domaine, appliquée par les cas d'usage **avant** d'appeler ce
/// port. Un adaptateur qui filtrerait lui-même dupliquerait la règle, et deux
/// copies d'une règle de sécurité finissent par diverger.
pub trait FournisseurOvh: Send + Sync {
    /// Tous les projets visibles du compte.
    fn lister_projets(&self) -> Result<Vec<ProjetOvh>, AppError>;

    /// Les instances d'un projet.
    fn lister_instances(&self, projet: &str) -> Result<Vec<InstanceOvh>, AppError>;

    /// Une instance précise.
    fn instance(&self, projet: &str, instance: &str) -> Result<InstanceOvh, AppError>;

    /// La consommation courante d'un projet.
    fn cout_courant(&self, projet: &str) -> Result<CoutCourant, AppError>;

    /// Les enregistrements DNS d'une zone.
    fn enregistrements_dns(&self, zone: &str) -> Result<Vec<EnregistrementDns>, AppError>;
}
