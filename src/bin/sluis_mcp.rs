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
use sluis::infrastructure::composition::{
    outil_campagne_si_configure, outil_ouverture_derogation_si_configure,
    outils_mise_en_ligne_si_configures, passerelle_github_si_configuree,
};
use sluis::infrastructure::diagnostic::DiagnosticSysteme;
use sluis::infrastructure::fs_inventaire::FsInventaire;
use sluis::infrastructure::mcp::outils_lecture::{
    OutilDoctor, OutilInventaire, OutilProfils, OutilVerifierDepot,
};
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
    registre.enregistrer(Box::new(OutilVerifierDepot))?;

    // En stdio comme en HTTP, la campagne n'apparaît que si le bac à sable est
    // configuré et si un secret de signature existe pour sceller la fenêtre de
    // dérogation. Son absence n'est pas une panne.
    let secret_signature = std::env::var("SLUIS_SECRET_SIGNATURE").unwrap_or_default();
    if let Some(campagne) = outil_campagne_si_configure(
        &configuration,
        secret_signature.as_bytes(),
        Arc::new(HorlogeSysteme),
    )? {
        registre.enregistrer(campagne)?;
    }

    // La passerelle s'expose par des outils de Tier 2 qui ne font que
    // demander ; la mise en ligne exige en plus que le plancher soit
    // vérifiable (ADR-012). Son état est dit au démarrage.
    let passerelle = passerelle_github_si_configuree(&configuration);
    eprintln!("sluis-mcp : {}", passerelle.diagnostic());
    for outil in
        outils_mise_en_ligne_si_configures(&configuration, passerelle, Arc::new(HorlogeSysteme))
    {
        eprintln!("sluis-mcp : « {} » exposé", outil.nom());
        registre.enregistrer(outil)?;
    }
    if let Some(ouverture) = outil_ouverture_derogation_si_configure(
        &configuration,
        secret_signature.as_bytes(),
        passerelle_github_si_configuree(&configuration),
        Arc::new(HorlogeSysteme),
    )? {
        eprintln!("sluis-mcp : « {} » exposé", ouverture.nom());
        registre.enregistrer(ouverture)?;
    }

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
