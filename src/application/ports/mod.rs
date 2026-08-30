//! Ports — les frontières que le domaine offre au monde extérieur.

mod audit_log;
mod charge;
mod derogation;
mod diagnostic;
mod gates;
mod horloge;
mod inventaire;
mod moteurs;
mod oauth;
mod ovh;
mod passerelle;
mod provisionnement;

pub use audit_log::AuditLog;
pub use charge::{MoteurCharge, ReglagePalier};
pub use derogation::DepotDerogation;
pub use diagnostic::Diagnostic;
pub use gates::{EtatGate, GatePlancher, VerificateurGates};
pub use horloge::{Horloge, HorlogeFigee};
pub use inventaire::DepotInventaire;
pub use moteurs::{
    Executeur, MoteurArgocd, MoteurHelm, MoteurKustomize, MoteurTerraform, SortieProcessus,
};
pub use oauth::DepotOAuth;
pub use ovh::FournisseurOvh;
pub use passerelle::{EtatApprobation, PasserelleApprobation};
pub use provisionnement::{DestructeurBail, Provisionneur};
