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
depots_autorises = ["/depots/koprogo/infrastructure"]
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
fn security_un_depot_autorise_sans_projet_de_bac_a_sable_n_expose_rien() {
    let configuration = configuration(
        r#"
[bac_a_sable]
depots_autorises = ["/depots/koprogo/infrastructure"]
"#,
        "sans-projet",
    );

    // Sans projet déclaré, la campagne n'aurait aucun projet OVH où louer :
    // mieux vaut ne rien exposer que d'exposer un outil qui muterait ailleurs.
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
depots_autorises = ["/depots/koprogo/infrastructure"]
"#,
        "sans-secret",
    );

    // Sans secret, la fenêtre de dérogation ne peut pas être scellée, donc
    // n'importe quelle écriture de fichier vaudrait approbation de Tier 1.
    assert!(outil_campagne_si_configure(&configuration, b"", horloge())
        .expect("composition")
        .is_none());
}

// ── Story 4.1 (#4) — passerelle GitHub composée par la configuration ──

mod passerelle {
    use sluis::configuration::Configuration;
    use sluis::infrastructure::composition::{passerelle_github_si_configuree, EtatPasserelle};

    const SECTION: &str = r#"
[github]
proprietaire = "gilmry"
depot = "koprogo"
workflow = "tier1-apply.yml"
"#;

    const JETON: &str = "ghp_jeton-d-essai-tres-reconnaissable";

    /// Charge une configuration sans toucher aux variables du processus.
    fn charger(contenu: &str, nom: &str, jeton: Option<&'static str>) -> Configuration {
        let chemin = std::env::temp_dir().join(format!("sluis-passerelle-{nom}.toml"));
        std::fs::write(&chemin, contenu).expect("écriture");
        let configuration = Configuration::charger_avec(Some(&chemin), move |variable| {
            (variable == "SLUIS_GITHUB_TOKEN")
                .then_some(jeton)
                .flatten()
                .map(str::to_string)
        })
        .expect("chargement");
        let _ = std::fs::remove_file(&chemin);
        configuration
    }

    #[test]
    fn happy_une_section_et_un_jeton_composent_la_passerelle() {
        let configuration = charger(SECTION, "complete", Some(JETON));
        match passerelle_github_si_configuree(&configuration) {
            EtatPasserelle::Composee(passerelle) => {
                let vue = format!("{passerelle:?}");
                assert!(vue.contains("gilmry/koprogo"), "{vue}");
                assert!(vue.contains("tier1-apply.yml"), "{vue}");
                assert!(
                    !vue.contains(JETON),
                    "le Debug ne doit pas montrer le jeton"
                );
            }
            autre => panic!("passerelle composée attendue, obtenu {autre:?}"),
        }
        let section = configuration.fichier.github.as_ref().expect("section");
        assert_eq!(section.branche, "main");
        assert_eq!(section.api, "https://api.github.com");
    }

    #[test]
    fn negative_sans_section_aucune_passerelle_et_ce_n_est_pas_une_erreur() {
        let configuration = charger(
            "[ovh]\nprojets_production = [\"prj\"]\n",
            "absente",
            Some(JETON),
        );
        assert!(matches!(
            passerelle_github_si_configuree(&configuration),
            EtatPasserelle::NonConfiguree
        ));
    }

    #[test]
    fn edge_une_section_sans_jeton_ne_compose_rien_et_le_dit() {
        for (nom, jeton) in [("sans-jeton", None), ("jeton-vide", Some("   "))] {
            let configuration = charger(SECTION, nom, jeton);
            let etat = passerelle_github_si_configuree(&configuration);
            assert!(
                matches!(etat, EtatPasserelle::JetonManquant),
                "{nom} : {etat:?}"
            );
            assert!(etat.diagnostic().contains("SLUIS_GITHUB_TOKEN"), "{nom}");
        }
    }

    #[test]
    fn edge_une_section_incomplete_est_une_erreur_de_chargement() {
        let chemin = std::env::temp_dir().join("sluis-passerelle-incomplete.toml");
        std::fs::write(&chemin, "[github]\nproprietaire = \"gilmry\"\n").expect("écriture");
        let resultat = Configuration::charger_avec(Some(&chemin), |_| None);
        let _ = std::fs::remove_file(&chemin);
        assert!(resultat.is_err(), "dépôt et workflow sont requis");
    }

    #[test]
    fn security_le_jeton_github_est_un_secret_connu_donc_efface_des_sorties() {
        let configuration = charger(SECTION, "secret", Some(JETON));
        assert!(
            configuration
                .secrets_connus()
                .iter()
                .any(|secret| secret.exposer() == JETON),
            "le jeton doit figurer parmi les valeurs effacées de toute sortie"
        );
    }
}

// ── Story 4.2 (#4) — vérificateur des gates composé par la configuration ──

mod gates {
    use sluis::configuration::Configuration;
    use sluis::infrastructure::composition::verificateur_gates_si_configure;

    fn charger(
        contenu: &str,
        nom: &str,
        jeton: Option<&'static str>,
    ) -> Result<Configuration, sluis::domain::AppError> {
        let chemin = std::env::temp_dir().join(format!("sluis-gates-{nom}.toml"));
        std::fs::write(&chemin, contenu).expect("écriture");
        let resultat = Configuration::charger_avec(Some(&chemin), move |variable| {
            (variable == "SLUIS_GITHUB_TOKEN")
                .then_some(jeton)
                .flatten()
                .map(str::to_string)
        });
        let _ = std::fs::remove_file(&chemin);
        resultat
    }

    const AVEC_GATES: &str = r#"
[github]
proprietaire = "gilmry"
depot = "koprogo"
workflow = "tier1-apply.yml"

[github.gates]
secrets = "Secrets"
sbom = "SBOM"
scan_conteneur = "Scan image"
retour_migration = "Migrations"
"#;

    #[test]
    fn happy_une_section_gates_et_un_jeton_composent_le_verificateur() {
        let configuration = charger(AVEC_GATES, "complete", Some("jeton")).expect("chargement");
        let verificateur = verificateur_gates_si_configure(&configuration).expect("composé");
        let vue = format!("{verificateur:?}");
        assert!(vue.contains("RetourMigration"), "{vue}");
        assert!(!vue.contains("jeton\""), "{vue}");
    }

    #[test]
    fn negative_sans_section_gates_aucun_verificateur() {
        let sans = AVEC_GATES.split("[github.gates]").next().expect("préfixe");
        let configuration = charger(sans, "sans-gates", Some("jeton")).expect("chargement");
        assert!(verificateur_gates_si_configure(&configuration).is_none());
    }

    #[test]
    fn edge_sans_jeton_aucun_verificateur() {
        let configuration = charger(AVEC_GATES, "sans-jeton", None).expect("chargement");
        assert!(verificateur_gates_si_configure(&configuration).is_none());
    }

    #[test]
    fn security_une_gate_commune_omise_empeche_le_chargement() {
        let incomplete = AVEC_GATES.replace("scan_conteneur = \"Scan image\"\n", "");
        assert!(
            charger(&incomplete, "incomplete", Some("jeton")).is_err(),
            "un plancher sans scan de conteneur ne doit pas se charger"
        );
    }
}

// ── Story 4.3 (#4) — les outils de mise en ligne n'existent qu'au complet ──

mod outils {
    use sluis::configuration::Configuration;
    use sluis::infrastructure::composition::{
        outils_mise_en_ligne_si_configures, passerelle_github_si_configuree,
    };

    const COMPLETE: &str = r#"
[github]
proprietaire = "gilmry"
depot = "koprogo"
workflow = "tier1-apply.yml"

[github.gates]
secrets = "Secrets"
sbom = "SBOM"
scan_conteneur = "Scan image"
"#;

    fn noms(contenu: &str, nom: &str, jeton: Option<&'static str>) -> Vec<&'static str> {
        let chemin = std::env::temp_dir().join(format!("sluis-outils-{nom}.toml"));
        std::fs::write(&chemin, contenu).expect("écriture");
        let configuration = Configuration::charger_avec(Some(&chemin), move |variable| {
            (variable == "SLUIS_GITHUB_TOKEN")
                .then_some(jeton)
                .flatten()
                .map(str::to_string)
        })
        .expect("chargement");
        let _ = std::fs::remove_file(&chemin);
        let passerelle = passerelle_github_si_configuree(&configuration);
        outils_mise_en_ligne_si_configures(&configuration, passerelle, super::horloge())
            .iter()
            .map(|o| o.nom())
            .collect()
    }

    #[test]
    fn happy_une_configuration_complete_expose_les_deux_outils() {
        assert_eq!(
            noms(COMPLETE, "complete", Some("jeton")),
            vec![
                "sluis_mise_en_ligne",
                "sluis_etat_approbation",
                "sluis_renouvellement_derogation"
            ]
        );
    }

    #[test]
    fn negative_sans_section_github_rien_n_est_expose() {
        assert!(noms("", "vide", Some("jeton")).is_empty());
    }

    #[test]
    fn edge_sans_jeton_rien_n_est_expose() {
        assert!(noms(COMPLETE, "sans-jeton", None).is_empty());
    }

    #[test]
    fn security_sans_gates_la_mise_en_ligne_n_est_pas_exposee() {
        let sans_gates = COMPLETE.split("[github.gates]").next().expect("préfixe");
        let noms = noms(sans_gates, "sans-gates", Some("jeton"));
        assert!(
            !noms.contains(&"sluis_mise_en_ligne"),
            "une demande au plancher non vérifiable ne doit pas pouvoir être soumise : {noms:?}"
        );
        // Le renouvellement ne dépend d'aucune gate : il reste demandable.
        assert!(
            noms.contains(&"sluis_renouvellement_derogation"),
            "{noms:?}"
        );
    }
}

// ── Story 4.4b (#4) — l'exécutant ouvre, le public demande ───

mod ouverture {
    use sluis::configuration::Configuration;
    use sluis::infrastructure::composition::{
        outil_ouverture_derogation_si_configure, outils_mise_en_ligne_si_configures,
        passerelle_github_si_configuree,
    };

    const EXECUTANT: &str = r#"
[ovh]
projets_bac_a_sable = ["bac-koprogo"]

[bac_a_sable]
depots_autorises = ["/depots/koprogo/infrastructure"]

[github]
proprietaire = "gilmry"
depot = "koprogo"
workflow = "tier1-apply.yml"
"#;

    const PUBLIC: &str = r#"
[github]
proprietaire = "gilmry"
depot = "koprogo"
workflow = "tier1-apply.yml"
"#;

    fn charger(contenu: &str, nom: &str) -> Configuration {
        let chemin = std::env::temp_dir().join(format!("sluis-ouverture-{nom}.toml"));
        std::fs::write(&chemin, contenu).expect("écriture");
        let configuration = Configuration::charger_avec(Some(&chemin), |variable| {
            (variable == "SLUIS_GITHUB_TOKEN").then(|| "jeton".to_string())
        })
        .expect("chargement");
        let _ = std::fs::remove_file(&chemin);
        configuration
    }

    fn ouverture(configuration: &Configuration, secret: &[u8]) -> Option<&'static str> {
        outil_ouverture_derogation_si_configure(
            configuration,
            secret,
            passerelle_github_si_configuree(configuration),
            super::horloge(),
        )
        .expect("composition")
        .map(|o| o.nom())
    }

    #[test]
    fn happy_l_executant_expose_l_ouverture() {
        let configuration = charger(EXECUTANT, "executant");
        assert_eq!(
            ouverture(&configuration, super::SECRET),
            Some("sluis_ouvrir_derogation")
        );
    }

    #[test]
    fn negative_le_public_n_expose_pas_l_ouverture() {
        let configuration = charger(PUBLIC, "public");
        assert_eq!(ouverture(&configuration, super::SECRET), None);
    }

    #[test]
    fn edge_sans_secret_de_signature_l_executant_n_expose_pas_l_ouverture() {
        let configuration = charger(EXECUTANT, "sans-secret");
        assert_eq!(ouverture(&configuration, b""), None);
    }

    #[test]
    fn security_l_executant_n_expose_aucun_outil_de_soumission() {
        let configuration = charger(EXECUTANT, "sans-soumission");
        let outils = outils_mise_en_ligne_si_configures(
            &configuration,
            passerelle_github_si_configuree(&configuration),
            super::horloge(),
        );
        assert!(
            outils.is_empty(),
            "l'exécutant ouvre, il ne demande pas : {:?}",
            outils.iter().map(|o| o.nom()).collect::<Vec<_>>()
        );
    }
}
