//! Entrée de journal d'audit et niveau d'autorisation.
//!
//! Le vocabulaire de [`Tier`] est repris tel quel d'`AGENT_GUARDRAILS.md` de
//! KoproGo, sans redéfinition. `Tier 2` est autonome et journalisé, `Tier 1`
//! exige une validation humaine. La règle d'or vaut aussi ici : au doute,
//! Tier 1.

use crate::domain::{RapportPreuve, Redacted};
use serde::Serialize;
use sha2::{Digest, Sha256};

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
    /// Fichiers du dépôt cible lus pendant l'appel (ADR-034, point 5). Omis
    /// quand l'appel n'a rien lu : les entrées d'avant gardent leur forme.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    lectures: Vec<Lecture>,
    /// Rapport de preuve d'une demande de mise en ligne (ADR-022). Omis
    /// ailleurs : les autres entrées gardent leur forme.
    #[serde(skip_serializing_if = "Option::is_none")]
    preuve: Option<RapportPreuve>,
}

/// Un fichier lu pendant un appel, et l'empreinte de ce qui a été lu.
///
/// L'empreinte prouve sur quoi Sluis a décidé, même si le fichier a changé
/// depuis : c'est le contenu lu qui est haché, pas celui du disque au moment
/// de l'audit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Lecture {
    chemin: String,
    sha256: String,
    /// Commit du dépôt d'où vient le fichier, omis quand il est inconnu.
    #[serde(skip_serializing_if = "Option::is_none")]
    commit: Option<String>,
}

impl Lecture {
    /// Consigne la lecture de `contenu` au `chemin` donné.
    pub fn new(chemin: String, contenu: &[u8]) -> Self {
        Self {
            chemin,
            sha256: format!("{:x}", Sha256::digest(contenu)),
            commit: None,
        }
    }

    /// Attache le commit du dépôt d'où vient le fichier.
    pub fn avec_commit(mut self, commit: Option<String>) -> Self {
        self.commit = commit;
        self
    }

    /// Commit du dépôt d'où vient le fichier, s'il est connu.
    pub fn commit(&self) -> Option<&str> {
        self.commit.as_deref()
    }

    /// Chemin du fichier lu.
    pub fn chemin(&self) -> &str {
        &self.chemin
    }

    /// SHA-256 du contenu lu, en hexadécimal minuscule.
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
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
            lectures: Vec::new(),
            preuve: None,
        }
    }

    /// Attache le rapport de preuve d'une demande de mise en ligne.
    pub fn avec_preuve(mut self, preuve: RapportPreuve) -> Self {
        self.preuve = Some(preuve);
        self
    }

    /// Rapport de preuve attaché, s'il y en a un.
    pub fn preuve(&self) -> Option<&RapportPreuve> {
        self.preuve.as_ref()
    }

    /// Attache les fichiers lus pendant l'appel.
    pub fn avec_lectures(mut self, lectures: Vec<Lecture>) -> Self {
        self.lectures = lectures;
        self
    }

    /// Fichiers lus pendant l'appel.
    pub fn lectures(&self) -> &[Lecture] {
        &self.lectures
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
