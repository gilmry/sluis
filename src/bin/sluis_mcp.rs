//! `sluis-mcp` — serveur MCP en transport stdio.
//!
//! Une requête JSON-RPC par ligne sur l'entrée standard, une réponse par ligne
//! sur la sortie standard. C'est le mode de travail local, celui qu'on déclare
//! dans un `.mcp.json`.
//!
//! Toute trace de diagnostic part sur **la sortie d'erreur** : écrire sur la
//! sortie standard corromprait le flux JSON-RPC, et le symptôme serait un
//! client qui se déconnecte sans raison lisible.

use std::io::{self, BufReader};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use sluis::application::ports::Horloge;
use sluis::configuration::Configuration;
use sluis::domain::Horodatage;
use sluis::infrastructure::audit::JsonlAuditLog;
use sluis::infrastructure::diagnostic::DiagnosticSysteme;
use sluis::infrastructure::fs_inventaire::FsInventaire;
use sluis::infrastructure::mcp::outils_lecture::{OutilDoctor, OutilInventaire, OutilProfils};
use sluis::infrastructure::mcp::{RegistreOutils, ServeurMcp};

/// Horloge système.
struct HorlogeSysteme;

impl Horloge for HorlogeSysteme {
    fn maintenant(&self) -> Horodatage {
        Horodatage::new(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
        )
    }
}

fn main() -> ExitCode {
    match executer() {
        Ok(()) => ExitCode::SUCCESS,
        Err(erreur) => {
            eprintln!("sluis-mcp : {erreur}");
            ExitCode::FAILURE
        }
    }
}

fn executer() -> Result<(), sluis::domain::AppError> {
    let chemin_config = std::env::var("SLUIS_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("sluis.toml"));
    let configuration = Configuration::charger(Some(&chemin_config))?;

    let journal = Arc::new(JsonlAuditLog::new(std::path::Path::new(
        &configuration.fichier.journal.chemin,
    ))?);
    let depot = Arc::new(FsInventaire::new());
    let diagnostic = Arc::new(DiagnosticSysteme::depuis_environnement());

    let mut registre = RegistreOutils::new();
    registre.enregistrer(Box::new(OutilDoctor::new(diagnostic)))?;
    registre.enregistrer(Box::new(OutilInventaire::new(depot.clone())))?;
    registre.enregistrer(Box::new(OutilProfils::new(depot)))?;

    eprintln!(
        "sluis-mcp {} — {} outil(s), journal « {} »",
        env!("CARGO_PKG_VERSION"),
        registre.len(),
        configuration.fichier.journal.chemin
    );

    let serveur = ServeurMcp::new(
        registre,
        journal,
        Arc::new(HorlogeSysteme),
        configuration.secrets_connus(),
    );

    serveur
        .boucle(BufReader::new(io::stdin()), io::stdout())
        .map_err(|e| sluis::domain::AppError::EntreeSortie {
            chemin: "stdio".to_string(),
            detail: e.to_string(),
        })
}
