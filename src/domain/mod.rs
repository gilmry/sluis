//! Domaine — entités et invariants métier.
//!
//! **Règle de pureté, vérifiée mécaniquement en CI** : aucun module de ce
//! dossier n'importe `reqwest`, `sqlx`, `actix_web` ni `tokio`. Une violation
//! fait échouer le job `purete-domaine`, elle n'est pas laissée à la vigilance.

pub mod audit;
pub mod autorisation;
pub mod bac_a_sable;
pub mod diagnostic;
pub mod error;
pub mod inventaire;
pub mod ovh;
pub mod redacted;
pub mod temps;
pub mod valeur_sure;

pub use audit::{AuditEntry, Tier};
pub use autorisation::{Action, Empreinte, JetonChangement, JetonConsomme, PlanChangement};
pub use bac_a_sable::{BailBacASable, DerogationValide, FenetreDerogation, PlafondDepense};
pub use diagnostic::{EtatBinaire, Identifiant, Moteur, RapportDiagnostic};
pub use error::AppError;
pub use inventaire::{
    Cellule, Environnement, MatriceInfrastructure, ModuleTerraform, ProfilCluster, Topologie,
};
pub use ovh::{
    CoutCourant, EnregistrementDns, InstanceOvh, ListeAutorisation, ProjetBacASable, ProjetOvh,
    ProjetProduction,
};
pub use redacted::Redacted;
pub use temps::{Duree, Horodatage};
pub use valeur_sure::{PlanTerraform, StatutArgocd, StatutHelm, ValeurSure};
