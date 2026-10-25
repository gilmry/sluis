//! Vérification de structure d'un dépôt cible (ADR-034, point 7).
//!
//! Chaque règle passe par **le lecteur que Sluis utilise à l'exécution** :
//! `FsCharge`, `FsInventaire`, la comparaison de section d'ADR-034 point 6.
//! Un verdict qui réimplémenterait les règles pourrait dire conforme un dépôt
//! que Sluis refusera.
//!
//! Les modules OpenTofu passent par `FsModuleTofu` et `verifier_module`, le
//! lecteur et le juge qui précèdent `tofu init` (ADR-034, point 3).

use std::path::Path;

use crate::application::ports::{DepotCharge, DepotInventaire, LecteurModule};
use crate::domain::contrat_depot::{RACINE_INFRASTRUCTURE, WORKFLOW_TIER1};
use crate::domain::module_tofu::{
    EXTENSIONS_JSON_REFUSEES, EXTENSIONS_PROGRAMME, MODULES_PARTAGES,
};
use crate::domain::section_empreinte::{verifier_section, GABARIT_TIER1};
use crate::domain::{verifier_module, AppError, Construction, Ecart, ValeurSure, VerdictDepot};
use crate::infrastructure::fs_charge::FsCharge;
use crate::infrastructure::fs_inventaire::FsInventaire;
use crate::infrastructure::fs_module_tofu::FsModuleTofu;

/// Profondeur maximale de la recherche des modules racines sous
/// `infrastructure/` : `<regroupement>/<topologie>/<environnement>/<sous-dossier>`
/// en demande quatre, la marge couvre les conventions voisines.
const PROFONDEUR_MODULES: usize = 6;

/// Taille maximale du workflow lu.
const TAILLE_MAXIMALE_WORKFLOW: u64 = 1024 * 1024;

/// Établit le verdict du dépôt dont `racine` est la racine.
///
/// Rend une erreur seulement si la racine elle-même est illisible ; tout le
/// reste est un écart du verdict.
///
/// `fournisseurs_autorises` est la liste blanche du Sluis interrogé. Vide, les
/// fournisseurs ne sont pas jugés et une remarque le dit : un écart « liste
/// blanche vide » par module ne dirait rien du dépôt.
pub fn verifier_depot(
    racine: &str,
    fournisseurs_autorises: &[String],
) -> Result<VerdictDepot, AppError> {
    let racine = FsInventaire::canoniser_racine(racine)?;
    let mut ecarts = Vec::new();
    let mut remarques = Vec::new();

    verifier_workflow(&racine, &mut ecarts);

    let infrastructure = racine.join(RACINE_INFRASTRUCTURE);
    if !infrastructure.is_dir() || !FsInventaire::dans_racine(&racine, &infrastructure) {
        ecarts.push(Ecart {
            regle: "arborescence",
            chemin: format!("{RACINE_INFRASTRUCTURE}/"),
            detail: "dossier absent".to_string(),
        });
        return Ok(VerdictDepot::new(ecarts, remarques));
    }
    let infra = infrastructure.to_string_lossy().to_string();

    if let Err(erreur) = FsCharge::new().lire(&infra) {
        ecarts.push(Ecart {
            regle: "charge",
            chemin: format!("{RACINE_INFRASTRUCTURE}/_shared/charge.yaml"),
            detail: erreur.to_string(),
        });
    }

    match FsInventaire::new().decouvrir_matrice(&infra) {
        Ok(matrice) => {
            if matrice.cellules.is_empty() {
                ecarts.push(Ecart {
                    regle: "cellules",
                    chemin: format!("{RACINE_INFRASTRUCTURE}/"),
                    detail: "aucune cellule <regroupement>/<topologie>/<environnement>/ reconnue"
                        .to_string(),
                });
            }
            remarques.extend(
                matrice
                    .ignores
                    .into_iter()
                    .map(|nom| format!("dossier non reconnu par l'inventaire : {nom}")),
            );
        }
        Err(erreur) => ecarts.push(Ecart {
            regle: "profils",
            chemin: format!("{RACINE_INFRASTRUCTURE}/_shared/cluster-profiles/"),
            detail: erreur.to_string(),
        }),
    }

    verifier_modules(
        &racine,
        &infrastructure,
        fournisseurs_autorises,
        &mut ecarts,
        &mut remarques,
    );

    Ok(VerdictDepot::new(ecarts, remarques))
}

/// Juge chaque module racine avec le lecteur et le juge d'exécution.
fn verifier_modules(
    racine: &Path,
    infrastructure: &Path,
    fournisseurs_autorises: &[String],
    ecarts: &mut Vec<Ecart>,
    remarques: &mut Vec<String>,
) {
    let liste_vide = fournisseurs_autorises.is_empty();
    if liste_vide {
        remarques.push(
            "fournisseurs OpenTofu non jugés : [tofu] fournisseurs_autorises est vide sur ce \
             Sluis ; l'exécutant refusera tout module tant qu'elle l'est"
                .to_string(),
        );
    }
    let lecteur = FsModuleTofu::new(vec![infrastructure.to_string_lossy().to_string()]);
    let mut modules = Vec::new();
    chercher_modules(infrastructure, infrastructure, 0, &mut modules);
    for module in modules {
        let relatif = module
            .strip_prefix(racine)
            .unwrap_or(&module)
            .display()
            .to_string();
        let ecart = |detail: String| Ecart {
            regle: "tofu",
            chemin: relatif.clone(),
            detail,
        };
        let soumis = ValeurSure::new(module.to_string_lossy().to_string())
            .and_then(|valeur| lecteur.lire(&valeur));
        let soumis = match soumis {
            Ok(soumis) => soumis,
            Err(erreur) => {
                ecarts.push(ecart(erreur.to_string()));
                continue;
            }
        };
        for violation in verifier_module(&soumis, fournisseurs_autorises) {
            let sur_les_fournisseurs = matches!(
                violation.construction,
                Construction::ListeBlancheVide | Construction::FournisseurHorsListe(_)
            );
            if liste_vide && sur_les_fournisseurs {
                continue;
            }
            ecarts.push(ecart(violation.to_string()));
        }
    }
}

/// Les dossiers sous `infrastructure/` qui contiennent directement un fichier
/// OpenTofu, hors `_shared/` (les modules partagés ne sont pas lancés seuls)
/// et hors dossiers cachés. Un lien qui sort du dépôt n'est pas suivi.
fn chercher_modules(
    infrastructure: &Path,
    dossier: &Path,
    profondeur: usize,
    modules: &mut Vec<std::path::PathBuf>,
) {
    if profondeur > PROFONDEUR_MODULES {
        return;
    }
    let Ok(entrees) = std::fs::read_dir(dossier) else {
        return;
    };
    let mut est_module = false;
    let mut sous_dossiers = Vec::new();
    for entree in entrees.flatten() {
        let chemin = entree.path();
        let Some(nom) = chemin.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if chemin.is_dir() {
            let partage = profondeur == 0 && nom == MODULES_PARTAGES[0];
            if !nom.starts_with('.')
                && !partage
                && FsInventaire::dans_racine(infrastructure, &chemin)
            {
                sous_dossiers.push(chemin);
            }
        } else if EXTENSIONS_PROGRAMME
            .iter()
            .chain(EXTENSIONS_JSON_REFUSEES)
            .any(|e| nom.ends_with(e))
        {
            est_module = true;
        }
    }
    if est_module {
        modules.push(dossier.to_path_buf());
    }
    sous_dossiers.sort();
    for sous_dossier in sous_dossiers {
        chercher_modules(infrastructure, &sous_dossier, profondeur + 1, modules);
    }
}

fn verifier_workflow(racine: &Path, ecarts: &mut Vec<Ecart>) {
    let chemin = racine.join(WORKFLOW_TIER1);
    let ecart = |detail: String| Ecart {
        regle: "workflow-tier1",
        chemin: WORKFLOW_TIER1.to_string(),
        detail,
    };
    if !chemin.is_file() {
        ecarts.push(ecart("absent".to_string()));
        return;
    }
    if !FsInventaire::dans_racine(racine, &chemin) {
        ecarts.push(ecart("lien qui sort du dépôt".to_string()));
        return;
    }
    match std::fs::metadata(&chemin) {
        Ok(meta) if meta.len() <= TAILLE_MAXIMALE_WORKFLOW => {}
        _ => {
            ecarts.push(ecart("illisible ou trop gros".to_string()));
            return;
        }
    }
    match std::fs::read_to_string(&chemin) {
        Ok(contenu) => {
            crate::infrastructure::lectures::consigner(&chemin, &contenu);
            if let Err(raison) = verifier_section(GABARIT_TIER1, &contenu) {
                ecarts.push(ecart(format!(
                    "section verification-empreinte non conforme au gabarit : {raison}"
                )));
            }
        }
        Err(e) => ecarts.push(ecart(format!("illisible : {e}"))),
    }
}
