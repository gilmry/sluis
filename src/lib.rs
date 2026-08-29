//! Sluis — l'écluse.
//!
//! Orchestrateur d'infrastructure OVH exposé comme serveur MCP, sous le modèle
//! d'autorisation Tier 1 / Tier 2 défini dans `AGENT_GUARDRAILS.md` de KoproGo.
//!
//! Architecture hexagonale stricte, dépendances vers l'intérieur uniquement :
//!
//! - [`domain`] : entités et invariants. **Zéro dépendance d'infrastructure.**
//!   Cette pureté n'est pas une convention, c'est un job de CI qui échoue
//!   (voir `scripts/verifier-purete-domaine.sh`).
//! - [`application`] : ports (traits) et use cases. Orchestre, ne décide pas.
//! - [`infrastructure`] : adaptateurs. Décide encore moins.

pub mod application;
pub mod domain;
pub mod infrastructure;
