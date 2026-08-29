//! Entrée de journal d'audit et niveau d'autorisation.
//!
//! Le vocabulaire de [`Tier`] est repris tel quel d'`AGENT_GUARDRAILS.md` de
//! KoproGo, sans redéfinition. `Tier 2` est autonome et journalisé, `Tier 1`
//! exige une validation humaine. La règle d'or vaut aussi ici : au doute,
//! Tier 1.

use crate::domain::Redacted;
use serde::Serialize;

/// Niveau d'autorisation d'une action.
///
/// L'ordre de la déclaration ne porte aucune sémantique : `Tier 1` est le plus
/// contraint malgré son numéro plus bas. C'est le vocabulaire existant, et le
/// changer pour le rendre « logique » créerait deux langues pour une réalité.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Tier {
    /// Validation humaine obligatoire.
    #[serde(rename = "1")]
    One,
    /// Autonome, journalisé, auditable a posteriori.
    #[serde(rename = "2")]
    Two,
}

/// Une entrée de journal, immuable une fois construite.
///
/// Aucun setter, aucun champ public : la seule façon de la modifier serait d'en
/// construire une autre, et le port ne sait pas remplacer.
#[derive(Debug, Clone, Serialize)]
pub struct AuditEntry {
    horodatage: String,
    outil: String,
    tier: Tier,
    empreinte: String,
    #[serde(flatten)]
    issue: Issue,
    #[serde(skip_serializing_if = "Option::is_none")]
    secret: Option<Redacted<String>>,
}

/// Issue d'un appel. Le succès comme l'échec laissent une trace : un journal
/// qui ne consignerait que les succès donnerait une image fausse de l'activité.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "issue", rename_all = "lowercase")]
enum Issue {
    Succes,
    Echec { detail: String },
}

impl AuditEntry {
    /// Construit une entrée. `resultat` porte le détail de l'échec le cas
    /// échéant ; il n'est pas typé `AppError` pour que le journal reste capable
    /// de consigner ce qui vient d'ailleurs qu'du domaine.
    pub fn new(
        horodatage: String,
        outil: String,
        tier: Tier,
        empreinte: String,
        resultat: Result<(), String>,
    ) -> Self {
        Self {
            horodatage,
            outil,
            tier,
            empreinte,
            issue: match resultat {
                Ok(()) => Issue::Succes,
                Err(detail) => Issue::Echec { detail },
            },
            secret: None,
        }
    }

    /// Attache un secret à l'entrée, pour le diagnostic.
    ///
    /// Le type impose [`Redacted`] : il n'existe aucune signature acceptant un
    /// secret en clair, donc aucun moyen d'en écrire un par inadvertance.
    pub fn avec_secret(mut self, secret: Redacted<String>) -> Self {
        self.secret = Some(secret);
        self
    }

    /// Nom de l'outil appelé.
    pub fn outil(&self) -> &str {
        &self.outil
    }

    /// Niveau d'autorisation sous lequel l'appel a été fait.
    pub fn tier(&self) -> Tier {
        self.tier
    }
}
