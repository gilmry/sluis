//! La barrière de déploiement (ADR-021) : des contrôles déterministes oui/non.
//!
//! **Le verdict d'un contrôle est son code de sortie, jamais son texte.** Un
//! contrôle qui écrit « tout va bien » et rend 1 est rouge. Sa sortie est
//! conservée comme rapport, pour la preuve (ADR-022), sans être interprétée.
//!
//! **Indétermination = échec** (ADR-021, point 4) : un contrôle absent, non
//! exécutable, tué ou hors délai est rouge, et le motif le dit.

use serde::Serialize;

use crate::domain::AppError;

/// Délai d'un contrôle quand la configuration n'en donne pas, en secondes.
pub const DELAI_PAR_DEFAUT_SECONDES: u64 = 60;

/// Délai maximal d'un contrôle, en secondes.
///
/// Une barrière sert à décider vite d'un état ; un contrôle de plus de dix
/// minutes mesure autre chose, ou attend quelque chose qui ne viendra pas.
pub const DELAI_MAXIMAL_SECONDES: u64 = 600;

/// Un contrôle de la barrière, tel que la configuration le déclare.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ControleBarriere {
    nom: String,
    programme: String,
    arguments: Vec<String>,
    delai_secondes: u64,
}

impl ControleBarriere {
    /// Déclare un contrôle.
    ///
    /// `programme` doit être un **chemin absolu** : un nom résolu par `PATH`
    /// laisserait l'environnement de lancement choisir ce qui s'exécute.
    pub fn new(
        nom: String,
        programme: String,
        arguments: Vec<String>,
        delai_secondes: Option<u64>,
    ) -> Result<Self, AppError> {
        let refus = |detail: String| AppError::Configuration {
            detail: format!("[barriere] contrôle « {nom} » : {detail}"),
        };
        let nom_valide = !nom.is_empty()
            && nom.len() <= 64
            && nom
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if !nom_valide {
            return Err(refus(
                "nom refusé : minuscules, chiffres et « - », 64 caractères au plus".to_string(),
            ));
        }
        if !programme.starts_with('/') || programme.split('/').any(|p| p == "..") {
            return Err(refus(format!(
                "programme « {programme} » refusé : un chemin absolu, sans « .. », est exigé"
            )));
        }
        let delai_secondes = delai_secondes.unwrap_or(DELAI_PAR_DEFAUT_SECONDES);
        if delai_secondes == 0 || delai_secondes > DELAI_MAXIMAL_SECONDES {
            return Err(refus(format!(
                "délai de {delai_secondes} s refusé : entre 1 et {DELAI_MAXIMAL_SECONDES} s"
            )));
        }
        Ok(Self {
            nom,
            programme,
            arguments,
            delai_secondes,
        })
    }

    /// Nom du contrôle.
    pub fn nom(&self) -> &str {
        &self.nom
    }
    /// Chemin absolu du programme.
    pub fn programme(&self) -> &str {
        &self.programme
    }
    /// Arguments passés au programme, sans shell.
    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }
    /// Délai au-delà duquel le contrôle est tué et compté rouge.
    pub fn delai_secondes(&self) -> u64 {
        self.delai_secondes
    }
}

/// Refuse deux contrôles de même nom : leurs verdicts seraient indiscernables
/// dans le rapport de preuve.
pub fn verifier_noms_uniques(controles: &[ControleBarriere]) -> Result<(), AppError> {
    for (i, controle) in controles.iter().enumerate() {
        if controles[..i].iter().any(|c| c.nom == controle.nom) {
            return Err(AppError::Configuration {
                detail: format!("[barriere] contrôle « {} » déclaré deux fois", controle.nom),
            });
        }
    }
    Ok(())
}

/// Ce qu'un contrôle a rendu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VerdictControle {
    /// Nom du contrôle.
    pub nom: String,
    /// Vrai si, et seulement si, le contrôle a rendu 0 dans son délai.
    pub vert: bool,
    /// Ce qui a décidé : « code 0 », « code 1 », « délai dépassé »…
    pub motif: String,
    /// Sortie du contrôle, tronquée, conservée pour la preuve, jamais lue.
    pub rapport: String,
}
