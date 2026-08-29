//! Port de découverte d'inventaire.

use crate::domain::{AppError, MatriceInfrastructure, ProfilCluster};

/// Découvre ce qu'un dépôt déclare comme infrastructure.
///
/// Le port ne sait rien du système de fichiers : il pourrait tout aussi bien
/// être servi par une API Git ou un index en cache. C'est ce qui permet de
/// tester les cas d'usage sans arborescence sur disque.
pub trait DepotInventaire: Send + Sync {
    /// Découvre la matrice topologies × environnements, plus profils et modules.
    fn decouvrir_matrice(&self, racine: &str) -> Result<MatriceInfrastructure, AppError>;

    /// Lit les seuls profils de cluster.
    fn lire_profils(&self, racine: &str) -> Result<Vec<ProfilCluster>, AppError>;
}
