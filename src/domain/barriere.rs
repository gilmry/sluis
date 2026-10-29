//! La barrière de déploiement (ADR-021) : des contrôles déterministes oui/non.
//!
//! **Le verdict d'un contrôle est son code de sortie, jamais son texte.** Un
//! contrôle qui écrit « tout va bien » et rend 1 est rouge. Sa sortie est
//! conservée comme rapport, pour la preuve (ADR-022), sans être interprétée.
//!
//! **Indétermination = échec** (ADR-021, point 4) : un contrôle absent, non
//! exécutable, tué ou hors délai est rouge, et le motif le dit.

use serde::Serialize;

use crate::domain::{AppError, Horodatage};

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

/// Exception datée d'ADR-021 : une barrière **vide** est admise jusqu'à une
/// date, mentionnée dans chaque plan soumis.
///
/// Décision du superviseur du 2026-10-29 (sluis#50, Q1) : tant que la
/// plateforme des étapes 2 à 6 n'existe pas, aucun contrôle n'a rien à
/// vérifier. Sans date, une barrière vide refuse : l'exception se déclare et
/// expire, elle ne s'oublie pas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExceptionBarriereVide {
    jusqu_au: String,
    fin: Horodatage,
}

impl ExceptionBarriereVide {
    /// Lit une date `AAAA-MM-JJ`, jour inclus, en UTC.
    pub fn depuis_date(texte: &str) -> Result<Self, AppError> {
        let refus = || AppError::Configuration {
            detail: format!(
                "[barriere] vide_admise_jusqu_au « {texte} » : une date AAAA-MM-JJ valide est \
                 exigée"
            ),
        };
        let morceaux: Vec<&str> = texte.split('-').collect();
        let [annee, mois, jour] = morceaux.as_slice() else {
            return Err(refus());
        };
        if annee.len() != 4 || mois.len() != 2 || jour.len() != 2 {
            return Err(refus());
        }
        let (Ok(annee), Ok(mois), Ok(jour)) = (
            annee.parse::<i64>(),
            mois.parse::<u32>(),
            jour.parse::<u32>(),
        ) else {
            return Err(refus());
        };
        if !(1..=12).contains(&mois) || jour == 0 || jour > jours_du_mois(annee, mois) {
            return Err(refus());
        }
        // Jour inclus : l'exception tombe au premier instant du lendemain.
        let fin = (jours_depuis_epoque(annee, mois, jour) + 1) * 86_400;
        Ok(Self {
            jusqu_au: texte.to_string(),
            fin: Horodatage::new(fin),
        })
    }

    /// La date déclarée, telle qu'écrite.
    pub fn jusqu_au(&self) -> &str {
        &self.jusqu_au
    }

    /// Vrai si l'exception vaut encore à `maintenant`.
    pub fn admet(&self, maintenant: Horodatage) -> bool {
        self.fin.apres(maintenant)
    }
}

fn bissextile(annee: i64) -> bool {
    (annee % 4 == 0 && annee % 100 != 0) || annee % 400 == 0
}

fn jours_du_mois(annee: i64, mois: u32) -> u32 {
    match mois {
        2 if bissextile(annee) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Jours depuis le 1970-01-01 (algorithme « days from civil » de H. Hinnant).
fn jours_depuis_epoque(annee: i64, mois: u32, jour: u32) -> i64 {
    let annee = if mois <= 2 { annee - 1 } else { annee };
    let ere = annee.div_euclid(400);
    let annee_de_l_ere = annee - ere * 400;
    let mois = i64::from(mois);
    let jour_de_l_annee =
        (153 * (if mois > 2 { mois - 3 } else { mois + 9 }) + 2) / 5 + i64::from(jour) - 1;
    let jour_de_l_ere =
        annee_de_l_ere * 365 + annee_de_l_ere / 4 - annee_de_l_ere / 100 + jour_de_l_annee;
    ere * 146_097 + jour_de_l_ere - 719_468
}
