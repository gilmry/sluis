//! Domaine — entités et invariants métier.
//!
//! **Règle de pureté, vérifiée mécaniquement en CI** : aucun module de ce
//! dossier n'importe `reqwest`, `sqlx`, `actix_web` ni `tokio`. Une violation
//! fait échouer le job `purete-domaine`, elle n'est pas laissée à la vigilance.

pub mod audit;
pub mod autorisation;
pub mod bac_a_sable;
pub mod capacite;
pub mod charge_declaree;
pub mod contrat_depot;
pub mod convergence;
pub mod diagnostic;
pub mod error;
pub mod etiquette_signee;
pub mod inventaire;
pub mod jeton_acces;
pub mod module_tofu;
pub mod oauth;
pub mod ovh;
pub mod redacted;
pub mod section_empreinte;
pub mod temps;
pub mod valeur_sure;
pub mod verdict_depot;

pub use audit::{AuditEntry, Lecture, Tier};
pub use autorisation::{Action, Empreinte, JetonChangement, JetonConsomme, PlanChangement};
pub use bac_a_sable::{
    plan_renouvellement, BailBacASable, CibleEphemere, DemandeBail, DerogationValide,
    FenetreDerogation, FenetreScellee, PlafondDepense,
};
pub use capacite::{
    verifier_coherence_latences, MesureCapacite, Palier, Prior, Provenance, RapportRecalage,
    Recalage,
};
pub use charge_declaree::{CibleCapacite, DeclarationCharge, Verdict};
pub use contrat_depot::{ContratDepot, VERSION_CONTRAT_DEPOT};
pub use convergence::{etablir as etablir_convergence, PreuveConvergence};
pub use diagnostic::{EtatBinaire, Identifiant, Moteur, RapportDiagnostic};
pub use error::AppError;
pub use etiquette_signee::{
    commande_signature, nom_etiquette_approbation, MessageEtiquette, FORMAT_MESSAGE_ETIQUETTE,
    PREFIXE_ETIQUETTE_APPROBATION,
};
pub use inventaire::{
    Cellule, Environnement, MatriceInfrastructure, ModuleTerraform, ProfilCluster, Topologie,
};
pub use jeton_acces::{hmac_sha256, Revendications};
pub use module_tofu::{verifier_module, Construction, FichierModule, ModuleSoumis, Violation};
pub use oauth::{
    base64url_sans_remplissage, empreinte_sha256, verifier_pkce, ClientOAuth, CodeAutorisation,
    DemandeCode, JetonRafraichissement, Portee,
};
pub use ovh::{
    CoutCourant, EnregistrementDns, InstanceOvh, ListeAutorisation, ProjetBacASable, ProjetOvh,
    ProjetProduction,
};
pub use redacted::Redacted;
pub use temps::{Duree, Horodatage};
pub use valeur_sure::{MutationTerraform, PlanTerraform, StatutArgocd, StatutHelm, ValeurSure};
pub use verdict_depot::{Ecart, VerdictDepot};
