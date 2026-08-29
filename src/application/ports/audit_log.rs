//! Port du journal d'audit.

use crate::domain::{AppError, AuditEntry};

/// Journal d'audit.
///
/// **Ce trait n'expose qu'`append`, et c'est le point.** L'invariant « une
/// entrée de journal est immuable » (Brief §10, invariant 7) n'est pas vérifié
/// à l'exécution : il est porté par la forme de l'interface. Il n'existe ni
/// `update`, ni `delete`, ni `truncate`, donc altérer le journal n'est pas
/// refusé, c'est inexprimable pour tout code écrit contre ce port.
///
/// La lecture est également absente, et délibérément : Sluis écrit son journal,
/// il ne le relit pas. L'audit se fait avec les outils du système de fichiers,
/// par un humain, hors du processus qui a produit les entrées.
pub trait AuditLog: Send + Sync {
    /// Ajoute une entrée. Échoue plutôt que de perdre la trace.
    fn append(&self, entree: &AuditEntry) -> Result<(), AppError>;
}
