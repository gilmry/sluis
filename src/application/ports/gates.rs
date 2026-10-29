//! Port de vérification des gates du plancher.

use crate::domain::AppError;
use serde::Serialize;

/// Les gates du plancher, au sens de `gates.md` : objectivables **et**
/// irréversibles. Elles cassent le build, elles ne produisent pas un rapport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GatePlancher {
    /// Aucun secret dans le diff ni l'historique.
    ///
    /// Irréversible : un secret poussé est fuité, et la rotation ne défait pas
    /// la fuite.
    Secrets,
    /// Nomenclature CycloneDX produite à chaque build.
    ///
    /// Irréversible : l'arbre de dépendances d'un build passé ne se reconstitue
    /// pas après coup.
    Sbom,
    /// Image scannée avant push registre.
    ScanConteneur,
    /// Fichier de retour de migration présent.
    ///
    /// Ne concerne que l'archétype stateful, et protège un état plutôt qu'un
    /// artefact : une migration sans retour jouée en production ne se défait pas.
    RetourMigration,
}

impl GatePlancher {
    /// Nom lisible.
    pub fn nom(&self) -> &'static str {
        match self {
            GatePlancher::Secrets => "secrets",
            GatePlancher::Sbom => "sbom",
            GatePlancher::ScanConteneur => "scan-conteneur",
            GatePlancher::RetourMigration => "retour-migration",
        }
    }

    /// Identifiants CyFun dont la gate produit la preuve, d'après
    /// `foyer/skills/conformite.md` §2ter. Référencés, pas recopiés : le sens
    /// de chaque identifiant reste dans Foyer.
    ///
    /// Le retour de migration n'y est pas mappé : il ne revendique rien
    /// plutôt qu'un identifiant choisi ici.
    pub fn cyfun(&self) -> &'static [&'static str] {
        match self {
            GatePlancher::Secrets => &["PR.AA-01.1"],
            GatePlancher::Sbom => &["ID.AM-02.1", "ID.AM-02.2"],
            GatePlancher::ScanConteneur => &["ID.RA-01.1", "ID.RA-01.2", "ID.RA-01.6"],
            GatePlancher::RetourMigration => &[],
        }
    }
}

/// État d'une gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EtatGate {
    /// La gate concernée.
    pub gate: GatePlancher,
    /// Vrai si elle est au vert.
    pub verte: bool,
    /// Détail, notamment en cas d'échec.
    pub detail: String,
}

/// Rend l'état des gates du plancher pour une référence donnée.
pub trait VerificateurGates: Send + Sync {
    /// État de toutes les gates applicables.
    fn etat_plancher(&self, reference: &str) -> Result<Vec<EtatGate>, AppError>;
}
