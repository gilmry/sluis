//! `sluis-server` — serveur MCP distant, transport Streamable HTTP.
//!
//! Le pendant réseau de `sluis-mcp`. Mêmes outils, même registre, même journal
//! d'audit ; s'y ajoutent le serveur d'autorisation OAuth 2.1 et l'exigence
//! d'un jeton porteur sur `/mcp`.
//!
//! Se déploie derrière Traefik, qui termine le TLS. Le service n'écoute qu'en
//! clair sur le réseau interne : lui confier la terminaison TLS dupliquerait
//! une responsabilité déjà tenue, et deux configurations de certificats
//! finissent par diverger.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use sluis::application::ports::Horloge;
use sluis::configuration::Configuration;
use sluis::domain::{Duree, Horodatage, Redacted};
use sluis::infrastructure::audit::JsonlAuditLog;
use sluis::infrastructure::diagnostic::DiagnosticSysteme;
use sluis::infrastructure::fs_inventaire::FsInventaire;
use sluis::infrastructure::mcp::outils_lecture::{OutilDoctor, OutilInventaire, OutilProfils};
use sluis::infrastructure::mcp::{RegistreOutils, ServeurMcp};
use sluis::infrastructure::oauth_depot::DepotOAuthFichier;
use sluis::infrastructure::serveur_http::{
    AleaSysteme, ReglagesHttp, ServeurHttp, VerificateurIdentifiants,
};

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

/// Vérification par identifiants uniques, définis dans l'environnement.
///
/// Sluis n'a pas de table d'utilisateurs : il sert un superviseur. Le mot de
/// passe est comparé à son empreinte, jamais en clair, et la comparaison est à
/// temps constant.
struct SuperviseurUnique {
    identifiant: String,
    empreinte_mot_de_passe: String,
}

impl VerificateurIdentifiants for SuperviseurUnique {
    fn verifier(&self, identifiant: &str, mot_de_passe: &str) -> Option<String> {
        let empreinte = sluis::domain::empreinte_sha256(mot_de_passe);
        let identifiant_ok = sluis::domain::hmac_sha256(b"cmp", identifiant.as_bytes())
            == sluis::domain::hmac_sha256(b"cmp", self.identifiant.as_bytes());
        let mot_de_passe_ok = sluis::domain::hmac_sha256(b"cmp", empreinte.as_bytes())
            == sluis::domain::hmac_sha256(b"cmp", self.empreinte_mot_de_passe.as_bytes());
        // Les deux comparaisons sont évaluées avant le `&&` : pas de
        // court-circuit qui révélerait, par le temps de réponse, si
        // l'identifiant existe.
        if identifiant_ok && mot_de_passe_ok {
            Some(self.identifiant.clone())
        } else {
            None
        }
    }
}

fn main() -> ExitCode {
    match executer() {
        Ok(()) => ExitCode::SUCCESS,
        Err(erreur) => {
            eprintln!("sluis-server : {erreur}");
            ExitCode::FAILURE
        }
    }
}

fn variable_requise(nom: &str) -> Result<String, sluis::domain::AppError> {
    std::env::var(nom)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| sluis::domain::AppError::Configuration {
            detail: format!(
                "variable « {nom} » absente : le serveur refuse de démarrer plutôt que \
                 de tourner avec une valeur par défaut devinable"
            ),
        })
}

fn executer() -> Result<(), sluis::domain::AppError> {
    let chemin_config = std::env::var("SLUIS_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("sluis.toml"));
    let configuration = Configuration::charger(Some(&chemin_config))?;

    // Aucune valeur par défaut pour ce qui doit être imprévisible : un secret
    // de signature par défaut serait un secret public.
    let secret_signature = Redacted::new(variable_requise("SLUIS_SECRET_SIGNATURE")?);
    let identifiant = variable_requise("SLUIS_IDENTIFIANT")?;
    let empreinte_mot_de_passe = variable_requise("SLUIS_EMPREINTE_MOT_DE_PASSE")?;
    let adresse = std::env::var("SLUIS_ECOUTE").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let base_url = std::env::var("SLUIS_BASE_URL")
        .unwrap_or_else(|_| "https://sluis.ecosolva.org".to_string());

    let journal = Arc::new(JsonlAuditLog::new(std::path::Path::new(
        &configuration.fichier.journal.chemin,
    ))?);
    let depot_inventaire = Arc::new(FsInventaire::new());
    let diagnostic = Arc::new(DiagnosticSysteme::depuis_environnement());

    let mut registre = RegistreOutils::new();
    registre.enregistrer(Box::new(OutilDoctor::new(diagnostic)))?;
    registre.enregistrer(Box::new(OutilInventaire::new(depot_inventaire.clone())))?;
    registre.enregistrer(Box::new(OutilProfils::new(depot_inventaire)))?;

    let mcp = Arc::new(ServeurMcp::new(
        registre,
        journal,
        Arc::new(HorlogeSysteme),
        configuration.secrets_connus(),
    ));

    let depot_oauth = Arc::new(DepotOAuthFichier::ouvrir(PathBuf::from(
        std::env::var("SLUIS_DEPOT_OAUTH").unwrap_or_else(|_| "sluis-oauth.json".to_string()),
    ))?);

    let serveur = ServeurHttp::new(
        depot_oauth,
        mcp,
        Arc::new(HorlogeSysteme),
        Arc::new(SuperviseurUnique {
            identifiant,
            empreinte_mot_de_passe,
        }),
        Arc::new(AleaSysteme),
        ReglagesHttp {
            base_url,
            secret_signature,
            validite_acces: Duree::secondes(3_600)?,
            validite_rafraichissement: Duree::jours(30)?,
            validite_code: Duree::secondes(600)?,
        },
    );

    eprintln!(
        "sluis-server {} écoute sur {adresse}",
        env!("CARGO_PKG_VERSION")
    );
    serveur.ecouter(&adresse)
}
