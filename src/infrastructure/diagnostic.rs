//! Diagnostic de l'environnement d'exécution.
//!
//! Cherche les six moteurs dans le `PATH` et signale la présence des
//! identifiants, sans jamais en révéler la valeur.
//!
//! Le `PATH` et les variables sont **injectés** plutôt que lus depuis le
//! processus. C'est ce qui rend les tests indépendants de la machine qui les
//! joue, comme l'exige NFR-06 : la suite doit se comporter pareil sur un poste
//! équipé et sur un agent de CI nu.

use std::path::{Path, PathBuf};

use crate::application::ports::Diagnostic;
use crate::domain::{AppError, EtatBinaire, Identifiant, Moteur, RapportDiagnostic};

/// Les moteurs que Sluis pilote, et ce à quoi ils servent.
pub const MOTEURS_ATTENDUS: &[(&str, &str)] = &[
    ("terraform", "provisionnement d'infrastructure"),
    ("ansible-playbook", "configuration et durcissement"),
    ("helm", "déploiement applicatif Kubernetes"),
    ("kubectl", "interrogation d'un cluster"),
    ("kustomize", "rendu des surcouches d'environnement"),
    ("argocd", "état de la réconciliation GitOps"),
];

/// Les identifiants attendus pour parler à OVH.
pub const IDENTIFIANTS_ATTENDUS: &[&str] = &[
    "OVH_APPLICATION_KEY",
    "OVH_APPLICATION_SECRET",
    "OVH_CONSUMER_KEY",
    "OVH_ENDPOINT",
];

/// Diagnostic système, paramétré par un `PATH` et un jeu de variables.
#[derive(Debug, Clone)]
pub struct DiagnosticSysteme {
    dossiers: Vec<PathBuf>,
    variables_presentes: Vec<String>,
}

impl DiagnosticSysteme {
    /// Construit le diagnostic depuis l'environnement réel du processus.
    pub fn depuis_environnement() -> Self {
        let path = std::env::var("PATH").unwrap_or_default();
        let variables_presentes = IDENTIFIANTS_ATTENDUS
            .iter()
            .filter(|nom| {
                std::env::var(**nom)
                    .map(|v| !v.trim().is_empty())
                    .unwrap_or(false)
            })
            .map(|nom| (*nom).to_string())
            .collect();
        Self::avec(&path, variables_presentes)
    }

    /// Construit le diagnostic à partir d'un `PATH` et d'une liste de variables
    /// présentes. C'est le constructeur qu'utilisent les tests.
    pub fn avec(path: &str, variables_presentes: Vec<String>) -> Self {
        let dossiers = path
            .split(':')
            .filter(|d| !d.is_empty())
            .map(PathBuf::from)
            .collect();
        Self {
            dossiers,
            variables_presentes,
        }
    }

    fn chercher(&self, binaire: &str) -> EtatBinaire {
        for dossier in &self.dossiers {
            let candidat = dossier.join(binaire);
            let Ok(metadonnees) = std::fs::metadata(&candidat) else {
                continue;
            };
            if !metadonnees.is_file() {
                continue;
            }
            return if est_executable(&candidat) {
                EtatBinaire::Disponible {
                    chemin: candidat.display().to_string(),
                }
            } else {
                // Distinct de l'absence : un binaire présent sans bit
                // d'exécution est une erreur de configuration, et le confondre
                // avec une machine nue rendrait le diagnostic trompeur.
                EtatBinaire::NonExecutable {
                    chemin: candidat.display().to_string(),
                }
            };
        }
        EtatBinaire::Absent
    }
}

#[cfg(unix)]
fn est_executable(chemin: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(chemin)
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn est_executable(_chemin: &Path) -> bool {
    true
}

impl Diagnostic for DiagnosticSysteme {
    fn etablir(&self) -> Result<RapportDiagnostic, AppError> {
        // Aucune absence ne produit d'erreur : sur une machine nue, les six
        // moteurs manquent, et c'est le résultat attendu, pas une panne.
        let moteurs = MOTEURS_ATTENDUS
            .iter()
            .map(|(nom, role)| Moteur {
                nom: (*nom).to_string(),
                role: (*role).to_string(),
                etat: self.chercher(nom),
            })
            .collect();

        let identifiants = IDENTIFIANTS_ATTENDUS
            .iter()
            .map(|nom| Identifiant {
                variable: (*nom).to_string(),
                // Présence, et rien d'autre : ni longueur, ni préfixe, ni
                // empreinte. Un diagnostic qui en dirait plus serait un oracle.
                present: self.variables_presentes.iter().any(|v| v == nom),
            })
            .collect();

        Ok(RapportDiagnostic::new(
            env!("CARGO_PKG_VERSION").to_string(),
            moteurs,
            identifiants,
        ))
    }
}
