//! Un outil MCP : son contrat, et son exécution.
//!
//! [`Outil`] étend [`ContratOutil`] : tout outil exécutable expose donc
//! forcément son schéma, et le registre ne peut pas contenir un outil dont le
//! contrat serait absent.
//!
//! Un outil est un **adaptateur mince** vers un cas d'usage déjà testé. Aucune
//! logique métier ne vit ici : le skill `mcp-oauth-maison` note qu'écrire du
//! métier dans un `tools/call` crée un chemin d'exécution parallèle et non
//! testé, ce qui est exactement l'erreur à éviter.

use crate::domain::{AppError, Tier};
use crate::infrastructure::mcp::ContratOutil;

/// Un outil exposé par `tools/list` et exécutable par `tools/call`.
pub trait Outil: ContratOutil {
    /// Niveau d'autorisation exigé.
    fn tier(&self) -> Tier;

    /// Exécute l'outil.
    ///
    /// Les arguments arrivent bruts : c'est l'implémentation qui les
    /// désérialise, par le **même** type que celui dont le schéma est dérivé.
    fn appeler(&self, arguments: &serde_json::Value) -> Result<serde_json::Value, AppError>;
}
