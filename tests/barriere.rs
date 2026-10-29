//! Story 7.1 (sluis#93) — un contrôle de barrière est un exécutable dont
//! seul le code de sortie fait foi (ADR-021, points 2 et 4).
//!
//! Les contrôles sont de petits scripts `sh` écrits à chaque test dans un
//! dossier temporaire : ils sont réels, mais ne dépendent d'aucun moteur
//! absent de la machine (NFR-06).

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use sluis::application::ports::EvaluateurBarriere;
use sluis::configuration::Configuration;
use sluis::domain::{ControleBarriere, VerdictControle};
use sluis::infrastructure::barriere_systeme::{EvaluateurBarriereSysteme, RAPPORT_MAXIMAL_OCTETS};

fn dossier(nom: &str) -> PathBuf {
    let chemin =
        std::env::temp_dir().join(format!("sluis-test-barriere-{nom}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&chemin);
    std::fs::create_dir_all(&chemin).expect("dossier");
    chemin
}

/// Écrit un script exécutable et rend son chemin absolu.
fn script(nom: &str, corps: &str) -> String {
    let chemin = dossier(nom).join("controle.sh");
    std::fs::write(&chemin, format!("#!/bin/sh\n{corps}\n")).expect("script");
    std::fs::set_permissions(&chemin, std::fs::Permissions::from_mode(0o755)).expect("mode");
    chemin.display().to_string()
}

fn controle(nom: &str, programme: String, delai: Option<u64>) -> ControleBarriere {
    ControleBarriere::new(nom.to_string(), programme, Vec::new(), delai).expect("contrôle")
}

fn evaluer(controle: ControleBarriere) -> VerdictControle {
    EvaluateurBarriereSysteme
        .evaluer(&[controle])
        .pop()
        .expect("un verdict")
}

fn charger(toml: &str, nom: &str) -> Result<Configuration, sluis::domain::AppError> {
    let chemin = dossier(nom).join("sluis.toml");
    std::fs::write(&chemin, toml).expect("configuration");
    Configuration::charger_avec(Some(&chemin), |_| None)
}

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_un_controle_qui_rend_zero_est_vert_avec_son_rapport() {
    let verdict = evaluer(controle("supervision", script("vert", "echo ok"), None));
    assert!(verdict.vert, "{verdict:?}");
    assert_eq!(verdict.motif, "code 0");
    assert!(verdict.rapport.contains("ok"), "{}", verdict.rapport);
}

#[test]
fn happy_les_controles_sont_evalues_dans_l_ordre_un_verdict_chacun() {
    let verdicts = EvaluateurBarriereSysteme.evaluer(&[
        controle("derive", script("ordre-a", "exit 0"), None),
        controle("sauvegarde", script("ordre-b", "exit 3"), None),
    ]);
    let noms: Vec<&str> = verdicts.iter().map(|v| v.nom.as_str()).collect();
    assert_eq!(noms, ["derive", "sauvegarde"]);
    assert!(verdicts[0].vert && !verdicts[1].vert);
}

#[test]
fn happy_une_configuration_declare_ses_controles() {
    let configuration = charger(
        "[[barriere.controles]]\nnom = \"supervision\"\nprogramme = \"/usr/local/bin/verifier-supervision\"\narguments = [\"--seuil\", \"600\"]\ndelai_secondes = 30\n",
        "config-ok",
    )
    .expect("configuration");
    assert_eq!(configuration.barriere.len(), 1);
    assert_eq!(configuration.barriere[0].arguments(), ["--seuil", "600"]);
    assert_eq!(configuration.barriere[0].delai_secondes(), 30);
}

// ── @negative ────────────────────────────────────────────────

#[test]
fn negative_un_code_non_nul_est_rouge_quel_que_soit_le_texte() {
    let verdict = evaluer(controle(
        "supervision",
        script("rouge", "echo \"tout va bien\"\nexit 1"),
        None,
    ));
    assert!(!verdict.vert, "{verdict:?}");
    assert_eq!(verdict.motif, "code 1");
    assert!(verdict.rapport.contains("tout va bien"));
}

#[test]
fn negative_un_programme_absent_est_rouge() {
    let verdict = evaluer(controle(
        "absent",
        "/chemin/qui/n-existe/pas".to_string(),
        None,
    ));
    assert!(!verdict.vert);
    assert_eq!(verdict.motif, "programme absent");
}

#[test]
fn negative_un_programme_non_executable_est_rouge() {
    let chemin = dossier("non-executable").join("controle.sh");
    std::fs::write(&chemin, "#!/bin/sh\nexit 0\n").expect("script");
    std::fs::set_permissions(&chemin, std::fs::Permissions::from_mode(0o644)).expect("mode");
    let verdict = evaluer(controle("lecture", chemin.display().to_string(), None));
    assert!(!verdict.vert);
    assert_eq!(verdict.motif, "programme non exécutable");
}

#[test]
fn negative_un_controle_tue_par_un_signal_est_rouge() {
    let verdict = evaluer(controle("signal", script("signal", "kill -TERM $$"), None));
    assert!(!verdict.vert);
    assert_eq!(verdict.motif, "tué par le signal 15");
}

#[test]
fn negative_une_configuration_mal_formee_arrete_le_chargement() {
    for (nom, toml) in [
        ("delai-nul", "[[barriere.controles]]\nnom = \"a\"\nprogramme = \"/bin/true\"\ndelai_secondes = 0\n"),
        ("delai-long", "[[barriere.controles]]\nnom = \"a\"\nprogramme = \"/bin/true\"\ndelai_secondes = 601\n"),
        ("nom", "[[barriere.controles]]\nnom = \"Avec Espace\"\nprogramme = \"/bin/true\"\n"),
        ("double", "[[barriere.controles]]\nnom = \"a\"\nprogramme = \"/bin/true\"\n[[barriere.controles]]\nnom = \"a\"\nprogramme = \"/bin/false\"\n"),
        ("inconnu", "[[barriere.controles]]\nnom = \"a\"\nprogramme = \"/bin/true\"\nshell = true\n"),
    ] {
        assert!(charger(toml, nom).is_err(), "« {nom} » aurait dû être refusé");
    }
}

// ── @edge ────────────────────────────────────────────────────

#[test]
fn edge_un_controle_hors_delai_est_tue_et_rouge_dans_les_temps() {
    let debut = Instant::now();
    let verdict = evaluer(controle("lent", script("lent", "sleep 30"), Some(1)));
    assert!(!verdict.vert);
    assert_eq!(verdict.motif, "délai dépassé (1 s)");
    // Le `sleep` est un enfant du shell : sans le groupe de processus, il
    // garderait les tubes ouverts et Sluis attendrait ses trente secondes.
    assert!(
        debut.elapsed() < Duration::from_secs(5),
        "évaluation de {:?}",
        debut.elapsed()
    );
}

#[test]
fn edge_un_enfant_laisse_en_arriere_plan_ne_retient_pas_la_barriere() {
    let debut = Instant::now();
    let verdict = evaluer(controle(
        "fuyard",
        script("fuyard", "sleep 30 &\necho fini"),
        Some(20),
    ));
    assert!(verdict.vert, "{verdict:?}");
    assert!(
        debut.elapsed() < Duration::from_secs(5),
        "{:?}",
        debut.elapsed()
    );
}

#[test]
fn edge_une_sortie_abondante_est_tronquee_sans_bloquer_le_controle() {
    let verdict = evaluer(controle(
        "bavard",
        script(
            "bavard",
            "i=0\nwhile [ $i -lt 20000 ]; do echo ligne-de-rapport-$i; i=$((i+1)); done",
        ),
        Some(30),
    ));
    assert!(verdict.vert, "{verdict:?}");
    assert!(
        verdict.rapport.len() <= 2 * RAPPORT_MAXIMAL_OCTETS + 64,
        "{} octets",
        verdict.rapport.len()
    );
}

// ── @security ────────────────────────────────────────────────

#[test]
fn security_un_controle_ne_voit_pas_l_environnement_de_sluis() {
    // Cargo pose cette variable dans l'environnement des tests : elle est là,
    // et le contrôle ne doit pas la voir, pas plus qu'un jeton.
    assert!(std::env::var("CARGO_MANIFEST_DIR").is_ok());
    let verdict = evaluer(controle("environnement", script("env", "env"), None));
    assert!(verdict.vert, "{verdict:?}");
    assert!(
        !verdict.rapport.contains("CARGO_MANIFEST_DIR"),
        "{}",
        verdict.rapport
    );
    assert!(verdict.rapport.contains("PATH="), "{}", verdict.rapport);
}

#[test]
fn security_un_programme_resolu_par_path_est_refuse() {
    for programme in ["verifier", "./verifier", "bin/verifier", "/opt/../bin/sh"] {
        assert!(
            ControleBarriere::new("a".to_string(), programme.to_string(), Vec::new(), None)
                .is_err(),
            "« {programme} » aurait dû être refusé"
        );
    }
}

#[test]
fn security_les_arguments_ne_passent_pas_par_un_shell() {
    let temoin = dossier("injection").join("temoin");
    let verdict = evaluer(
        ControleBarriere::new(
            "echo".to_string(),
            "/bin/echo".to_string(),
            vec![format!("; touch {}", temoin.display())],
            None,
        )
        .expect("contrôle"),
    );
    assert!(verdict.vert);
    assert!(
        !temoin.exists(),
        "un argument a été interprété par un shell"
    );
}
