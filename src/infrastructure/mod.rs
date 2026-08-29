//! Infrastructure — adaptateurs.
//!
//! Tout ce qui touche au monde extérieur vit ici : système de fichiers, réseau,
//! processus. Le domaine n'en connaît rien, et un test de CI le prouve.

pub mod audit;
pub mod bac_a_sable;
pub mod charge;
pub mod diagnostic;
pub mod fs_inventaire;
pub mod github;
pub mod mcp;
pub mod ovh;
pub mod process;
pub mod yaml_plat;
