//! Rapport de diagnostic de l'environnement d'exécution.
//!
//! Sluis pilote six moteurs externes — terraform, ansible-playbook, helm,
//! kubectl, kustomize, argocd — dont **aucun n'est garanti présent**. Sur une
//! machine de développement nue, les six manquent, et c'est un état normal.
//!
//! Le rapport existe pour que l'agent sache ce qu'il peut tenter avant
//! d'échouer, plutôt que de découvrir l'absence au milieu d'une campagne.

use serde::Serialize;

/// Ce qu'on sait d'un binaire attendu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "etat")]
pub enum EtatBinaire {
    /// Présent et exécutable.
    Disponible { chemin: String },
    /// Présent dans le `PATH` mais non exécutable.
    ///
    /// Distingué de l'absence à dessein : un binaire trouvé mais sans bit
    /// d'exécution est une erreur de configuration, pas une machine nue, et le
    /// diagnostic serait trompeur s'il les confondait.
    NonExecutable { chemin: String },
    /// Absent du `PATH`.
    Absent,
}

impl EtatBinaire {
    /// Vrai si le binaire peut réellement être invoqué.
    pub fn utilisable(&self) -> bool {
        matches!(self, EtatBinaire::Disponible { .. })
    }
}

/// État d'un moteur attendu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Moteur {
    /// Nom du binaire.
    pub nom: String,
    /// Ce à quoi il sert, pour que le rapport soit lisible sans documentation.
    pub role: String,
    /// Son état sur cette machine.
    pub etat: EtatBinaire,
}

/// État d'un identifiant attendu.
///
/// **Ne porte jamais la valeur**, ni même sa longueur ou son préfixe : un
/// diagnostic qui dirait « clé présente, commence par `a1b2` » serait un oracle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Identifiant {
    /// Nom de la variable d'environnement.
    pub variable: String,
    /// Présence, et rien d'autre.
    pub present: bool,
}

/// Rapport complet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RapportDiagnostic {
    /// Version de Sluis.
    pub version: String,
    /// État des moteurs.
    pub moteurs: Vec<Moteur>,
    /// Présence des identifiants.
    pub identifiants: Vec<Identifiant>,
}

impl RapportDiagnostic {
    /// Construit un rapport.
    pub fn new(version: String, moteurs: Vec<Moteur>, identifiants: Vec<Identifiant>) -> Self {
        Self {
            version,
            moteurs,
            identifiants,
        }
    }

    /// Moteurs réellement utilisables.
    pub fn moteurs_utilisables(&self) -> Vec<&Moteur> {
        self.moteurs
            .iter()
            .filter(|m| m.etat.utilisable())
            .collect()
    }

    /// Moteurs manquants ou inutilisables.
    pub fn moteurs_indisponibles(&self) -> Vec<&Moteur> {
        self.moteurs
            .iter()
            .filter(|m| !m.etat.utilisable())
            .collect()
    }

    /// Vrai si tous les identifiants attendus sont présents.
    pub fn identifiants_complets(&self) -> bool {
        self.identifiants.iter().all(|i| i.present)
    }
}
