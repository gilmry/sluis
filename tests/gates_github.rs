//! Story 4.2 (#4) — les gates du plancher lues sur les check-runs GitHub.
//!
//! **Aucun appel réseau réel** (NFR-07). Le serveur d'essai rend les
//! check-runs du scénario et rend compte des requêtes reçues, ce qui permet de
//! prouver qu'une référence refusée l'est **avant** toute requête.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;

use sluis::application::ports::{GatePlancher, VerificateurGates};
use sluis::domain::{AppError, Redacted};
use sluis::infrastructure::gates_github::VerificateurGatesGithub;

struct ServeurEssai {
    adresse: String,
    requetes: mpsc::Receiver<String>,
}

impl ServeurEssai {
    fn recues(&self) -> Vec<String> {
        self.requetes.try_iter().collect()
    }
}

/// Répond `code` et `corps` à toute requête.
fn lancer(code: u16, corps: String) -> ServeurEssai {
    let ecouteur = TcpListener::bind("127.0.0.1:0").expect("écoute");
    let adresse = format!("http://{}", ecouteur.local_addr().expect("adresse"));
    let (emetteur, requetes) = mpsc::channel();

    thread::spawn(move || {
        for flux in ecouteur.incoming() {
            let Ok(mut flux) = flux else { continue };
            let mut lecteur = BufReader::new(flux.try_clone().expect("clone"));
            let mut ligne = String::new();
            if lecteur.read_line(&mut ligne).is_err() {
                continue;
            }
            loop {
                let mut entete = String::new();
                if lecteur.read_line(&mut entete).unwrap_or(0) == 0 || entete.trim().is_empty() {
                    break;
                }
            }
            let mut morceaux = ligne.split_whitespace();
            let methode = morceaux.next().unwrap_or("");
            let chemin = morceaux.next().unwrap_or("/");
            let _ = emetteur.send(format!("{methode} {chemin}"));

            let brut = format!(
                "HTTP/1.1 {code} OK\r\nContent-Type: application/json\r\n\
                 Content-Length: {}\r\nConnection: close\r\n\r\n{corps}",
                corps.len()
            );
            let _ = flux.write_all(brut.as_bytes());
            let _ = flux.flush();
        }
    });

    ServeurEssai { adresse, requetes }
}

fn run(nom: &str, statut: &str, conclusion: Option<&str>) -> serde_json::Value {
    serde_json::json!({ "name": nom, "status": statut, "conclusion": conclusion })
}

fn check_runs(runs: Vec<serde_json::Value>) -> String {
    serde_json::json!({ "total_count": runs.len(), "check_runs": runs }).to_string()
}

fn plancher() -> Vec<(GatePlancher, String)> {
    vec![
        (GatePlancher::Secrets, "Secrets".to_string()),
        (GatePlancher::Sbom, "SBOM".to_string()),
        (GatePlancher::ScanConteneur, "Scan image".to_string()),
    ]
}

fn verificateur(
    adresse: &str,
    correspondance: Vec<(GatePlancher, String)>,
) -> VerificateurGatesGithub {
    VerificateurGatesGithub::new(
        "gilmry".to_string(),
        "koprogo".to_string(),
        adresse.to_string(),
        Redacted::new("jeton-essai".to_string()),
        correspondance,
    )
}

fn tout_vert() -> Vec<serde_json::Value> {
    vec![
        run("Secrets", "completed", Some("success")),
        run("SBOM", "completed", Some("success")),
        run("Scan image", "completed", Some("success")),
        run("Lint", "completed", Some("failure")),
    ]
}

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_trois_checks_en_succes_rendent_le_plancher_vert() {
    let serveur = lancer(200, check_runs(tout_vert()));
    let etats = verificateur(&serveur.adresse, plancher())
        .etat_plancher("v0.1.0")
        .expect("lecture");
    assert_eq!(etats.len(), 3);
    assert!(etats.iter().all(|e| e.verte), "{etats:?}");
    assert_eq!(
        serveur.recues(),
        vec!["GET /repos/gilmry/koprogo/commits/v0.1.0/check-runs?per_page=100"]
    );
}

// ── @negative ────────────────────────────────────────────────

#[test]
fn negative_un_check_en_echec_rend_sa_gate_rouge_et_le_dit() {
    let mut runs = tout_vert();
    runs[1] = run("SBOM", "completed", Some("failure"));
    let serveur = lancer(200, check_runs(runs));
    let etats = verificateur(&serveur.adresse, plancher())
        .etat_plancher("abc123")
        .expect("lecture");
    let sbom = etats
        .iter()
        .find(|e| e.gate == GatePlancher::Sbom)
        .expect("sbom");
    assert!(!sbom.verte);
    assert!(sbom.detail.contains("failure"), "{}", sbom.detail);
    assert!(etats.iter().filter(|e| e.verte).count() == 2);
}

#[test]
fn negative_une_reference_inconnue_est_introuvable() {
    let serveur = lancer(404, String::new());
    let resultat = verificateur(&serveur.adresse, plancher()).etat_plancher("inconnue");
    assert!(
        matches!(resultat, Err(AppError::Introuvable { .. })),
        "{resultat:?}"
    );
}

// ── @edge ────────────────────────────────────────────────────

#[test]
fn edge_absent_en_cours_ignore_ou_neutre_valent_rouge() {
    let cas = [
        (
            "absent",
            vec![
                run("Secrets", "completed", Some("success")),
                run("SBOM", "completed", Some("success")),
            ],
        ),
        (
            "en cours",
            vec![
                run("Secrets", "completed", Some("success")),
                run("SBOM", "completed", Some("success")),
                run("Scan image", "in_progress", None),
            ],
        ),
        (
            "ignoré",
            vec![
                run("Secrets", "completed", Some("success")),
                run("SBOM", "completed", Some("success")),
                run("Scan image", "completed", Some("skipped")),
            ],
        ),
        (
            "neutre",
            vec![
                run("Secrets", "completed", Some("success")),
                run("SBOM", "completed", Some("success")),
                run("Scan image", "completed", Some("neutral")),
            ],
        ),
    ];
    for (nom, runs) in cas {
        let serveur = lancer(200, check_runs(runs));
        let etats = verificateur(&serveur.adresse, plancher())
            .etat_plancher("abc123")
            .expect("lecture");
        let scan = etats
            .iter()
            .find(|e| e.gate == GatePlancher::ScanConteneur)
            .expect("scan");
        assert!(!scan.verte, "{nom} : {scan:?}");
    }
}

#[test]
fn edge_un_seul_echec_parmi_des_runs_homonymes_suffit_a_rougir() {
    let mut runs = tout_vert();
    runs.push(run("Secrets", "completed", Some("failure")));
    let serveur = lancer(200, check_runs(runs));
    let etats = verificateur(&serveur.adresse, plancher())
        .etat_plancher("abc123")
        .expect("lecture");
    let secrets = etats
        .iter()
        .find(|e| e.gate == GatePlancher::Secrets)
        .expect("secrets");
    assert!(!secrets.verte);
}

#[test]
fn edge_un_plancher_vide_est_refuse_sans_requete() {
    let serveur = lancer(200, check_runs(tout_vert()));
    let resultat = verificateur(&serveur.adresse, Vec::new()).etat_plancher("abc123");
    assert!(
        matches!(resultat, Err(AppError::Configuration { .. })),
        "{resultat:?}"
    );
    assert!(serveur.recues().is_empty());
}

// ── @security ────────────────────────────────────────────────

#[test]
fn security_une_reference_qui_sort_du_chemin_est_refusee_avant_toute_requete() {
    let serveur = lancer(200, check_runs(tout_vert()));
    let v = verificateur(&serveur.adresse, plancher());
    for reference in [
        "",
        "../../orgs/gilmry",
        "main/../../x",
        "abc?per_page=1",
        "abc#x",
        "a b",
        "refs/heads/main",
        ".cache",
        &"a".repeat(101),
    ] {
        let resultat = v.etat_plancher(reference);
        assert!(
            matches!(resultat, Err(AppError::Configuration { .. })),
            "« {reference} » aurait dû être refusée : {resultat:?}"
        );
    }
    assert!(serveur.recues().is_empty(), "aucune requête ne doit partir");
}

#[test]
fn security_un_jeton_refuse_est_une_erreur_d_authentification_masquee() {
    let serveur = lancer(401, String::new());
    let resultat = verificateur(&serveur.adresse, plancher()).etat_plancher("abc123");
    match resultat {
        Err(e @ AppError::Authentification { .. }) => {
            assert!(!e.to_string().contains("jeton-essai"), "{e}");
        }
        autre => panic!("authentification attendue, obtenu {autre:?}"),
    }
}
