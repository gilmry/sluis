//! Infrastructure — adaptateurs.
//!
//! Tout ce qui touche au monde extérieur vit ici : système de fichiers, réseau,
//! processus. Le domaine n'en connaît rien, et un test de CI le prouve.

pub mod audit;
pub mod mcp;
