//! Le rapport de preuve d'une demande de mise en ligne (ADR-022).
//!
//! Assemblé par Sluis à partir de ce que le plancher et la barrière ont vu,
//! **jamais rédigé à la main**, et écrit au journal append-only avant toute
//! soumission. Les identifiants CyFun y sont **référencés**, pas recopiés :
//! leur sens vit dans `foyer/skills/conformite.md`.
//!
//! **Un refus est aussi une preuve** (conformite.md §2ter, règle 3) : une
//! demande refusée laisse son rapport comme une demande admise.

use serde::Serialize;

use crate::domain::{AppError, Horodatage};

/// Version du schéma du rapport.
pub const SCHEMA_RAPPORT_PREUVE: &str = "sluis/rapport-preuve/v1";

/// Ce qu'un contrôle, du plancher ou de la barrière, a rendu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ElementPreuve {
    /// `plancher` ou `barriere`.
    pub source: &'static str,
    /// Nom de la gate ou du contrôle.
    pub nom: String,
    /// Verdict.
    pub vert: bool,
    /// Ce qui a décidé.
    pub motif: String,
    /// Sortie du contrôle, tronquée, telle qu'il l'a laissée.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub rapport: String,
    /// Identifiants CyFun auxquels ce contrôle répond.
    pub cyfun: Vec<String>,
}

/// Le rapport d'une demande.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RapportPreuve {
    /// Schéma, pour que le rapport se lise sans le code qui l'a écrit.
    pub schema: &'static str,
    /// Instant de la demande.
    pub horodatage: Horodatage,
    /// Projet.
    pub projet: String,
    /// Environnement visé.
    pub environnement: String,
    /// Commit ou étiquette demandés.
    pub reference: String,
    /// Version de Sluis qui a produit le rapport.
    pub version_sluis: String,
    /// `admise`, `refusee_par_les_gates`, `refusee_par_la_barriere` ou
    /// `refusee_barriere_vide`.
    pub issue: &'static str,
    /// Empreinte complète du plan, quand il est soumis.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub empreinte_plan: Option<String>,
    /// Date de l'exception de barrière vide, quand elle a été appliquée ou
    /// trouvée échue.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub barriere_vide_jusqu_au: Option<String>,
    /// Identifiants couverts par la demande elle-même.
    pub cyfun: Vec<String>,
    /// Un élément par gate et par contrôle évalués.
    pub controles: Vec<ElementPreuve>,
}

/// Vérifie la forme d'un identifiant CyFun 2025, par exemple `PR.PS-01.1`.
///
/// La forme seulement : la liste des identifiants vit dans Foyer, et Sluis
/// ne la recopie pas.
pub fn verifier_identifiant_cyfun(identifiant: &str) -> Result<(), AppError> {
    let refus = || AppError::Configuration {
        detail: format!(
            "identifiant CyFun « {identifiant} » refusé : la forme attendue est « PR.PS-01.1 »"
        ),
    };
    let (fonction, reste) = identifiant.split_once('.').ok_or_else(refus)?;
    let (categorie, reste) = reste.split_once('-').ok_or_else(refus)?;
    let (numero, mesure) = reste.split_once('.').ok_or_else(refus)?;
    let lettres = |s: &str| s.len() == 2 && s.chars().all(|c| c.is_ascii_uppercase());
    let chiffres =
        |s: &str, n: usize| !s.is_empty() && s.len() <= n && s.chars().all(|c| c.is_ascii_digit());
    if lettres(fonction)
        && lettres(categorie)
        && chiffres(numero, 2)
        && numero.len() == 2
        && chiffres(mesure, 2)
    {
        Ok(())
    } else {
        Err(refus())
    }
}
