//! Application — ports et cas d'usage.
//!
//! Les ports sont des traits volontairement étroits : un par préoccupation,
//! conformément au principe de ségrégation des interfaces. La forme d'un port
//! porte parfois un invariant à elle seule, comme [`ports::AuditLog`] qui
//! n'expose qu'`append` et rend donc l'altération inexprimable.

pub mod campagne;
pub mod mise_en_ligne;
pub mod ports;
