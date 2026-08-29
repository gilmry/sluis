//! Transport MCP, registre et outils.

pub mod contrat;
pub mod outil;
pub mod outils_lecture;
pub mod registre;
pub mod serveur;

pub use contrat::ContratOutil;
pub use outil::Outil;
pub use registre::RegistreOutils;
pub use serveur::ServeurMcp;
