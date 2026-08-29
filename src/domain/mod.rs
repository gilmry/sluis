//! Domaine — entités et invariants métier.
//!
//! **Règle de pureté, vérifiée mécaniquement en CI** : aucun module de ce
//! dossier n'importe `reqwest`, `sqlx`, `actix_web` ni `tokio`. Une violation
//! fait échouer le job `purete-domaine`, elle n'est pas laissée à la vigilance.

pub mod error;
pub mod redacted;

pub use error::AppError;
pub use redacted::Redacted;
