//! Découverte de l'inventaire depuis le système de fichiers.
//!
//! Traduit une convention de dossiers en matrice typée. La convention visée est
//! celle de KoproGo : `{monosite,multisite}/<topologie>/<environnement>`, avec
//! `_shared/cluster-profiles/*.yaml` et `_shared/terraform/modules/*/`.
//!
//! **Sécurité du parcours** : la racine est canonisée une fois, et toute entrée
//! dont le chemin canonique n'en descend pas est ignorée. Un lien symbolique
//! sortant de la racine n'est donc pas suivi. Sans cela, un dépôt hostile
//! pourrait faire lire n'importe quel dossier de la machine par un simple lien.

use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::application::ports::DepotInventaire;
use crate::domain::{
    AppError, Cellule, Environnement, MatriceInfrastructure, ModuleTerraform, ProfilCluster,
    Topologie,
};

/// Adaptateur de découverte sur système de fichiers.
#[derive(Debug, Default)]
pub struct FsInventaire;

impl FsInventaire {
    /// Crée l'adaptateur.
    pub fn new() -> Self {
        Self
    }

    fn canoniser_racine(racine: &str) -> Result<PathBuf, AppError> {
        let chemin = Path::new(racine);
        let canonique = chemin.canonicalize().map_err(|e| AppError::EntreeSortie {
            chemin: racine.to_string(),
            detail: e.to_string(),
        })?;
        if !canonique.is_dir() {
            return Err(AppError::Configuration {
                detail: format!("« {racine} » n'est pas un dossier"),
            });
        }
        Ok(canonique)
    }

    /// Vrai si `chemin` descend bien de `racine` une fois canonisé.
    fn dans_racine(racine: &Path, chemin: &Path) -> bool {
        match chemin.canonicalize() {
            Ok(canonique) => canonique.starts_with(racine),
            Err(_) => false,
        }
    }

    fn sous_dossiers(racine: &Path, dossier: &Path) -> Vec<(String, PathBuf)> {
        let mut trouves = Vec::new();
        let Ok(entrees) = std::fs::read_dir(dossier) else {
            return trouves;
        };
        for entree in entrees.flatten() {
            let chemin = entree.path();
            if !chemin.is_dir() || !Self::dans_racine(racine, &chemin) {
                continue;
            }
            if let Some(nom) = chemin.file_name().and_then(|n| n.to_str()) {
                trouves.push((nom.to_string(), chemin.clone()));
            }
        }
        trouves.sort_by(|a, b| a.0.cmp(&b.0));
        trouves
    }
}

impl DepotInventaire for FsInventaire {
    fn decouvrir_matrice(&self, racine: &str) -> Result<MatriceInfrastructure, AppError> {
        let racine = Self::canoniser_racine(racine)?;
        let mut matrice = MatriceInfrastructure::default();

        // Les topologies se trouvent soit à la racine, soit sous un dossier de
        // regroupement (monosite, multisite). On accepte les deux plutôt que
        // d'imposer une profondeur, car la convention varie d'un dépôt à l'autre.
        let mut dossiers_a_examiner = vec![racine.clone()];
        for (_, chemin) in Self::sous_dossiers(&racine, &racine) {
            dossiers_a_examiner.push(chemin);
        }

        for dossier in &dossiers_a_examiner {
            for (nom, chemin_topologie) in Self::sous_dossiers(&racine, dossier) {
                let Ok(topologie) = Topologie::from_str(&nom) else {
                    continue;
                };
                if !matrice.topologies.contains(&topologie) {
                    matrice.topologies.push(topologie);
                }
                for (nom_env, _) in Self::sous_dossiers(&racine, &chemin_topologie) {
                    match Environnement::from_str(&nom_env) {
                        Ok(environnement) => {
                            if !matrice.environnements.contains(&environnement) {
                                matrice.environnements.push(environnement);
                            }
                            let cellule = Cellule {
                                topologie,
                                environnement,
                            };
                            if !matrice.cellules.contains(&cellule) {
                                matrice.cellules.push(cellule);
                            }
                        }
                        Err(_) => {
                            // Une convention que Sluis ne connaît pas, comme
                            // `local/`. La taire donnerait l'illusion d'un
                            // inventaire exhaustif.
                            let trace = format!("{nom}/{nom_env}");
                            if !matrice.ignores.contains(&trace) {
                                matrice.ignores.push(trace);
                            }
                        }
                    }
                }
            }
        }

        matrice.topologies.sort();
        matrice.environnements.sort();
        matrice
            .cellules
            .sort_by_key(|c| (c.topologie, c.environnement));
        matrice.ignores.sort();

        matrice.profils = self.lire_profils(racine.to_str().unwrap_or_default())?;
        matrice.modules = lire_modules(&racine);

        Ok(matrice)
    }

    fn lire_profils(&self, racine: &str) -> Result<Vec<ProfilCluster>, AppError> {
        let racine = Self::canoniser_racine(racine)?;
        let dossier = racine.join("_shared").join("cluster-profiles");
        if !dossier.is_dir() {
            return Ok(Vec::new());
        }
        let entrees = std::fs::read_dir(&dossier).map_err(|e| AppError::EntreeSortie {
            chemin: dossier.display().to_string(),
            detail: e.to_string(),
        })?;

        let mut profils = Vec::new();
        for entree in entrees.flatten() {
            let chemin = entree.path();
            let extension = chemin.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !matches!(extension, "yaml" | "yml") || !Self::dans_racine(&racine, &chemin) {
                continue;
            }
            let nom = chemin
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();
            let contenu = std::fs::read_to_string(&chemin).map_err(|e| AppError::EntreeSortie {
                chemin: chemin.display().to_string(),
                detail: e.to_string(),
            })?;
            let cles = crate::infrastructure::yaml_plat::aplatir(&contenu).map_err(|detail| {
                AppError::Analyse {
                    quoi: chemin.display().to_string(),
                    detail,
                }
            })?;
            profils.push(ProfilCluster::new(
                nom,
                cles.get("global.storageClassName").cloned(),
                cles.get("global.ingressClassName").cloned(),
                cles.get("global.secretsBackend").cloned(),
                cles.get("global.tls.enabled")
                    .map(|v| v.eq_ignore_ascii_case("true")),
                cles.get("resources.preset").cloned(),
            )?);
        }
        profils.sort_by(|a, b| a.nom().cmp(b.nom()));
        Ok(profils)
    }
}

fn lire_modules(racine: &Path) -> Vec<ModuleTerraform> {
    let dossier = racine.join("_shared").join("terraform").join("modules");
    let mut modules: Vec<ModuleTerraform> = FsInventaire::sous_dossiers(racine, &dossier)
        .into_iter()
        .filter_map(|(nom, _)| ModuleTerraform::new(nom).ok())
        .collect();
    modules.sort_by(|a, b| a.nom().cmp(b.nom()));
    modules
}
