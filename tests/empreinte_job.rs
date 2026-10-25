//! C3 — le job de Tier 1 recalcule l'empreinte avant toute mutation (faille S2).
//!
//! Le test confronte **deux codes qui ne se voient pas** : la passerelle Rust,
//! qui compose les entrées du dispatch, et le script du gabarit
//! `tier1-apply.yml.example`, qui les relit dans le dépôt cible. Le script est
//! extrait du gabarit et exécuté tel quel, avec les entrées réellement
//! envoyées : si l'un dérive, le test casse ici plutôt qu'en production.
//!
//! **Aucun appel réseau réel** (NFR-07).

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::sync::mpsc;
use std::thread;

use sha2::{Digest, Sha256};
use sluis::application::ports::PasserelleApprobation;
use sluis::domain::{
    plan_renouvellement, Action, Environnement, Horodatage, PlanChangement, Redacted, Tier,
};
use sluis::infrastructure::github::PasserelleGithub;

const GABARIT: &str = include_str!("../.github/workflows/tier1-apply.yml.example");

/// Le script entre les marqueurs du gabarit, désindenté.
fn script_du_gabarit() -> String {
    let debut = GABARIT
        .find("# >>> verification-empreinte")
        .expect("marqueur de début absent du gabarit");
    let fin = GABARIT
        .find("# <<< verification-empreinte")
        .expect("marqueur de fin absent du gabarit");
    GABARIT[debut..fin]
        .lines()
        .map(str::trim_start)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Les entrées réellement envoyées au dispatch pour `plan`.
fn entrees_envoyees(plan: &PlanChangement) -> serde_json::Map<String, serde_json::Value> {
    let ecouteur = TcpListener::bind("127.0.0.1:0").expect("écoute");
    let adresse = format!("http://{}", ecouteur.local_addr().expect("adresse"));
    let (emetteur, corps) = mpsc::channel::<String>();

    thread::spawn(move || {
        for flux in ecouteur.incoming() {
            let Ok(mut flux) = flux else { continue };
            let mut lecteur = BufReader::new(flux.try_clone().expect("clone"));
            let mut ligne = String::new();
            if lecteur.read_line(&mut ligne).is_err() {
                continue;
            }
            let mut longueur = 0usize;
            loop {
                let mut entete = String::new();
                if lecteur.read_line(&mut entete).unwrap_or(0) == 0 || entete.trim().is_empty() {
                    break;
                }
                if let Some((cle, valeur)) = entete.split_once(':') {
                    if cle.trim().eq_ignore_ascii_case("content-length") {
                        longueur = valeur.trim().parse().unwrap_or(0);
                    }
                }
            }
            let mut recu = vec![0u8; longueur];
            let _ = lecteur.read_exact(&mut recu);

            let (code, reponse) = if ligne.starts_with("POST ") {
                let _ = emetteur.send(String::from_utf8_lossy(&recu).to_string());
                (204, String::new())
            } else if ligne.contains("/contents/") {
                // Le workflow de la cible est le gabarit lui-même (ADR-034, point 6).
                (200, GABARIT.to_string())
            } else {
                (
                    200,
                    r#"{"protection_rules":[{"type":"required_reviewers","prevent_self_review":true,
                       "reviewers":[{"type":"User","reviewer":{"login":"relecteur-a"}}]}]}"#
                        .to_string(),
                )
            };
            let brut = format!(
                "HTTP/1.1 {code} OK\r\nContent-Type: application/json\r\n\
                 Content-Length: {}\r\nConnection: close\r\n\r\n{reponse}",
                reponse.len()
            );
            let _ = flux.write_all(brut.as_bytes());
        }
    });

    PasserelleGithub::new(
        "gilmry".to_string(),
        "koprogo".to_string(),
        "tier1-apply.yml".to_string(),
        "main".to_string(),
        Redacted::new("jeton-essai".to_string()),
        adresse,
    )
    .soumettre(plan)
    .expect("soumission");

    let corps: serde_json::Value =
        serde_json::from_str(&corps.recv().expect("dispatch reçu")).expect("json");
    corps["inputs"].as_object().expect("inputs").clone()
}

/// Exécute le script du gabarit comme le ferait le runner GitHub.
fn job_accepte(entrees: &serde_json::Map<String, serde_json::Value>) -> bool {
    let valeur = |cle: &str| {
        entrees
            .get(cle)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    Command::new("bash")
        .arg("-e")
        .arg("-c")
        .arg(script_du_gabarit())
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("EMPREINTE", valeur("empreinte"))
        .env("ACTION", valeur("action"))
        .env("ENVIRONNEMENT", valeur("environnement"))
        .env("TIER", valeur("tier"))
        .env("CIBLE", valeur("cible"))
        .env("DESCRIPTION", valeur("description"))
        .env("DIFF", valeur("diff"))
        .output()
        .expect("bash")
        .status
        .success()
}

fn plan_mise_en_ligne() -> PlanChangement {
    PlanChangement::new(
        Action::MiseEnLigne,
        Environnement::Production,
        Tier::One,
        "koprogo@v0.1.0".to_string(),
        "met en ligne koprogo sur production, 3 gate(s) du plancher au vert".to_string(),
        "secrets : vert\nsbom : vert\nscan-conteneur : vert".to_string(),
    )
    .expect("plan")
}

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_le_job_accepte_les_entrees_d_une_mise_en_ligne() {
    let plan = plan_mise_en_ligne();
    let entrees = entrees_envoyees(&plan);
    assert_eq!(entrees["empreinte"], plan.empreinte().hexadecimal());
    assert!(
        job_accepte(&entrees),
        "le job doit accepter un dispatch fidèle"
    );
}

#[test]
fn happy_le_job_accepte_un_renouvellement_au_diff_vide() {
    let plan = plan_renouvellement(Horodatage::new(1_790_000_000)).expect("plan");
    let entrees = entrees_envoyees(&plan);
    assert_eq!(entrees["diff"], "");
    assert!(job_accepte(&entrees));
}

// ── @negative ────────────────────────────────────────────────

#[test]
fn negative_une_entree_changee_apres_coup_arrete_le_job() {
    let entrees = entrees_envoyees(&plan_mise_en_ligne());
    for (cle, autre) in [
        ("action", "terraform_destroy"),
        ("cible", "koprogo@v9.9.9"),
        ("environnement", "staging"),
        ("diff", "secrets : vert"),
    ] {
        let mut falsifiees = entrees.clone();
        falsifiees.insert(
            cle.to_string(),
            serde_json::Value::String(autre.to_string()),
        );
        assert!(
            !job_accepte(&falsifiees),
            "{cle} modifié aurait dû arrêter le job"
        );
    }
}

// ── @edge ────────────────────────────────────────────────────

#[test]
fn edge_la_forme_canonique_est_bien_la_source_de_l_empreinte() {
    let plan = plan_mise_en_ligne();
    let calculee = format!("{:x}", Sha256::digest(plan.forme_canonique().as_bytes()));
    assert_eq!(calculee, plan.empreinte().hexadecimal());
}

// ── @security ────────────────────────────────────────────────

#[test]
fn security_une_entree_piegee_n_est_ni_executee_ni_acceptee_si_falsifiee() {
    let temoin = std::env::temp_dir().join("sluis-c3-injection");
    let _ = std::fs::remove_file(&temoin);
    let piege = format!("$(touch {}) `touch {}`", temoin.display(), temoin.display());
    let plan = PlanChangement::new(
        Action::MiseEnLigne,
        Environnement::Production,
        Tier::One,
        "koprogo@v0.1.0".to_string(),
        piege,
        String::new(),
    )
    .expect("plan");
    let entrees = entrees_envoyees(&plan);

    assert!(
        job_accepte(&entrees),
        "un plan fidèle passe, même avec des métacaractères"
    );
    assert!(
        !temoin.exists(),
        "une entrée ne doit jamais être exécutée par le script"
    );
}

#[test]
fn security_toutes_les_composantes_de_l_empreinte_sont_envoyees() {
    let entrees = entrees_envoyees(&plan_mise_en_ligne());
    for cle in [
        "empreinte",
        "action",
        "environnement",
        "tier",
        "cible",
        "description",
        "diff",
    ] {
        assert!(
            entrees.contains_key(cle),
            "entrée « {cle} » absente du dispatch"
        );
    }
    assert_eq!(entrees["tier"], "One");
}
