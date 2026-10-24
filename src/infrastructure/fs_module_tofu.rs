//! Lecture d'un module OpenTofu sur le système de fichiers, pour vérification.
//!
//! L'adaptateur lit, le domaine juge ([`verifier_module`](crate::domain::verifier_module)).
//! Ce qu'il rend : les fichiers du module racine, et ceux de tous les modules
//! partagés du dépôt (`_shared/terraform/modules/`), seul endroit d'où un bloc
//! `module` peut tirer sa source. Un module appelé est donc toujours lu, donc
//! toujours jugé, avant que `tofu` ne le charge.
//!
//! **Sécurité du parcours**, comme pour l'inventaire : le module doit, une fois
//! canonisé, descendre d'une racine autorisée, et un fichier dont le chemin
//! canonique sort de cette racine (lien symbolique) fait refuser le module
//! entier plutôt que d'être ignoré, puisque `tofu`, lui, le suivrait.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use crate::application::ports::LecteurModule;
use crate::domain::module_tofu::{FICHIER_VERROU, MODULES_PARTAGES};
use crate::domain::{AppError, FichierModule, ModuleSoumis, ValeurSure};
use crate::infrastructure::fs_inventaire::FsInventaire;

/// Taille au-delà de laquelle un fichier n'est pas lu : un `.tf` de plus d'un
/// mégaoctet n'est pas un module écrit à la main.
const TAILLE_MAXIMALE: u64 = 1024 * 1024;

/// Lecteur de modules sur système de fichiers.
#[derive(Debug, Clone, Default)]
pub struct FsModuleTofu {
    racines: Vec<String>,
}

impl FsModuleTofu {
    /// Crée le lecteur. Les racines sont les dépôts autorisés : un module qui
    /// n'en descend pas est refusé.
    pub fn new(racines: Vec<String>) -> Self {
        Self { racines }
    }

    fn racine_du_module(&self, module: &Path) -> Option<PathBuf> {
        self.racines
            .iter()
            .filter_map(|racine| FsInventaire::canoniser_racine(racine).ok())
            .find(|racine| module.starts_with(racine))
    }
}

/// Vrai si ce nom de fichier intéresse la vérification.
fn a_lire(nom: &str) -> bool {
    nom == FICHIER_VERROU
        || nom.ends_with(".tf")
        || nom.ends_with(".tofu")
        || nom.ends_with(".tf.json")
        || nom.ends_with(".tofu.json")
}

/// Chemin relatif à la racine, séparé par `/`.
fn relatif(racine: &Path, chemin: &Path) -> String {
    chemin
        .strip_prefix(racine)
        .unwrap_or(chemin)
        .components()
        .filter_map(|c| match c {
            Component::Normal(nom) => nom.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn lire_fichier(
    racine: &Path,
    chemin: &Path,
    fichiers: &mut BTreeMap<String, String>,
) -> Result<(), AppError> {
    if !FsInventaire::dans_racine(racine, chemin) {
        return Err(AppError::CheminHorsRacine {
            chemin: chemin.display().to_string(),
        });
    }
    let erreur = |e: std::io::Error| AppError::EntreeSortie {
        chemin: chemin.display().to_string(),
        detail: e.to_string(),
    };
    let taille = std::fs::metadata(chemin).map_err(erreur)?.len();
    if taille > TAILLE_MAXIMALE {
        return Err(AppError::Analyse {
            quoi: chemin.display().to_string(),
            detail: format!("fichier de {taille} octets, au-delà de {TAILLE_MAXIMALE}"),
        });
    }
    let contenu = std::fs::read_to_string(chemin).map_err(erreur)?;
    fichiers.insert(relatif(racine, chemin), contenu);
    Ok(())
}

/// Lit les fichiers d'un dossier, et de ses sous-dossiers si `recursif`. Les
/// dossiers cachés (dont `.terraform/`, le cache de `tofu`) sont sautés.
fn lire_dossier(
    racine: &Path,
    dossier: &Path,
    recursif: bool,
    fichiers: &mut BTreeMap<String, String>,
) -> Result<(), AppError> {
    let entrees = std::fs::read_dir(dossier).map_err(|e| AppError::EntreeSortie {
        chemin: dossier.display().to_string(),
        detail: e.to_string(),
    })?;
    for entree in entrees.flatten() {
        let chemin = entree.path();
        let Some(nom) = chemin.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if chemin.is_dir() {
            if recursif && !nom.starts_with('.') {
                if !FsInventaire::dans_racine(racine, &chemin) {
                    return Err(AppError::CheminHorsRacine {
                        chemin: chemin.display().to_string(),
                    });
                }
                lire_dossier(racine, &chemin, true, fichiers)?;
            }
        } else if a_lire(nom) {
            lire_fichier(racine, &chemin, fichiers)?;
        }
    }
    Ok(())
}

impl LecteurModule for FsModuleTofu {
    fn lire(&self, module: &ValeurSure) -> Result<ModuleSoumis, AppError> {
        let canonique =
            Path::new(module.valeur())
                .canonicalize()
                .map_err(|e| AppError::EntreeSortie {
                    chemin: module.valeur().to_string(),
                    detail: e.to_string(),
                })?;
        let racine =
            self.racine_du_module(&canonique)
                .ok_or_else(|| AppError::CheminHorsRacine {
                    chemin: module.valeur().to_string(),
                })?;

        let mut fichiers = BTreeMap::new();
        lire_dossier(&racine, &canonique, false, &mut fichiers)?;
        let partages = MODULES_PARTAGES
            .iter()
            .fold(racine.clone(), |chemin, segment| chemin.join(segment));
        if partages.is_dir() {
            lire_dossier(&racine, &partages, true, &mut fichiers)?;
        }

        Ok(ModuleSoumis {
            chemin: relatif(&racine, &canonique),
            fichiers: fichiers
                .into_iter()
                .map(|(chemin, contenu)| FichierModule { chemin, contenu })
                .collect(),
        })
    }
}
