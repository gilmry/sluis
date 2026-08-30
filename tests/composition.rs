//! Story 5.6 — ce qu'un déploiement expose dépend de sa configuration.
//!
//! Le test qui porte tout le fichier est
//! `security_une_configuration_de_lecture_n_expose_aucune_mutation` : c'est
//! lui qui rend tenable de déployer le même binaire deux fois, un Sluis public
//! qui ne détient rien et un Sluis exécutant sur le réseau interne.

use std::sync::Arc;

use sluis::application::ports::{Horloge, HorlogeFigee};
use sluis::configuration::Configuration;
use sluis::domain::{Horodatage, Tier};
use sluis::infrastructure::composition::outil_campagne_si_configure;

const SECRET: &[u8] = b"secret-de-signature";

fn horloge() -> Arc<dyn Horloge> {
    Arc::new(HorlogeFigee::a(Horodatage::new(1_000_000)))
}

/// Écrit une configuration temporaire et la charge.
fn configuration(contenu: &str, nom: &str) -> Configuration {
    let chemin = std::env::temp_dir().join(format!("sluis-composition-{nom}.toml"));
    std::fs::write(&chemin, contenu).expect("écriture");
    let configuration = Configuration::charger(Some(&chemin)).expect("chargement");
    let _ = std::fs::remove_file(&chemin);
    configuration
}

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_une_configuration_complete_expose_la_campagne() {
    let configuration = configuration(
        r#"
[ovh]
projets_bac_a_sable = ["bac-koprogo"]

[bac_a_sable]
module_terraform = "infra/bac-a-sable"
"#,
        "complete",
    );

    let outil = outil_campagne_si_configure(&configuration, SECRET, horloge())
        .expect("composition")
        .expect("l'outil doit être exposé");

    assert_eq!(outil.nom(), "sluis_campagne");
    assert_eq!(outil.tier(), Tier::Two);
}

// ── @security ────────────────────────────────────────────────

#[test]
fn security_une_configuration_de_lecture_n_expose_aucune_mutation() {
    // La configuration du Sluis public, mot pour mot : des projets à lire,
    // aucun module de bac à sable.
    let configuration = configuration(
        r#"
[ovh]
projets_production = ["prj-prod"]
"#,
        "lecture",
    );

    assert!(
        outil_campagne_si_configure(&configuration, SECRET, horloge())
            .expect("composition")
            .is_none(),
        "un déploiement de lecture ne doit exposer aucun outil qui mute"
    );
}

#[test]
fn security_un_module_declare_sans_projet_de_bac_a_sable_n_expose_rien() {
    let configuration = configuration(
        r#"
[bac_a_sable]
module_terraform = "infra/bac-a-sable"
"#,
        "sans-projet",
    );

    // Sans projet déclaré, la campagne n'aurait aucune liste d'autorisation à
    // opposer à sa cible : mieux vaut ne rien exposer que d'exposer un outil
    // qui muterait n'importe où.
    assert!(
        outil_campagne_si_configure(&configuration, SECRET, horloge())
            .expect("composition")
            .is_none()
    );
}

#[test]
fn security_sans_secret_de_signature_la_campagne_n_est_pas_exposee() {
    let configuration = configuration(
        r#"
[ovh]
projets_bac_a_sable = ["bac-koprogo"]

[bac_a_sable]
module_terraform = "infra/bac-a-sable"
"#,
        "sans-secret",
    );

    // Sans secret, la fenêtre de dérogation ne peut pas être scellée, donc
    // n'importe quelle écriture de fichier vaudrait approbation de Tier 1.
    assert!(outil_campagne_si_configure(&configuration, b"", horloge())
        .expect("composition")
        .is_none());
}
