//! Issue #2 — refuser le dispatch Tier 1 quand l'environnement cible est
//! absent ou non protégé (faille S1).
//!
//! **Aucun appel réseau réel** (NFR-07). Le serveur d'essai répond à la lecture
//! de l'environnement selon le scénario, accepte tout dispatch, et rend compte
//! des requêtes reçues : c'est ce qui permet de prouver qu'un refus a lieu
//! **avant** le dispatch, et pas seulement qu'une erreur remonte.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;

use sluis::application::ports::{EtatApprobation, PasserelleApprobation};
use sluis::domain::{Action, AppError, Environnement, PlanChangement, Redacted, Tier};
use sluis::infrastructure::github::PasserelleGithub;

struct ServeurEssai {
    adresse: String,
    requetes: mpsc::Receiver<String>,
}

impl ServeurEssai {
    /// Les requêtes reçues, sous la forme « MÉTHODE chemin ».
    fn recues(&self) -> Vec<String> {
        self.requetes.try_iter().collect()
    }

    fn a_dispatche(&self) -> bool {
        self.recues()
            .iter()
            .any(|r| r.starts_with("POST ") && r.ends_with("/dispatches"))
    }
}

/// `environnement` : `None` → 404 (environnement absent) ; `Some(json)` → 200.
fn lancer_serveur(environnement: Option<&'static str>) -> ServeurEssai {
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
            let mut morceaux = ligne.split_whitespace();
            let methode = morceaux.next().unwrap_or("").to_string();
            let chemin = morceaux.next().unwrap_or("/").to_string();

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
            let mut corps_recu = vec![0u8; longueur];
            let _ = lecteur.read_exact(&mut corps_recu);
            let _ = emetteur.send(format!("{methode} {chemin}"));

            let (code, corps) = if methode == "POST" && chemin.ends_with("/dispatches") {
                (204, String::new())
            } else if chemin.contains("/environments/") {
                match environnement {
                    Some(json) => (200, json.to_string()),
                    None => (404, String::new()),
                }
            } else {
                (404, String::new())
            };
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

fn passerelle(adresse: &str) -> PasserelleGithub {
    PasserelleGithub::new(
        "gilmry".to_string(),
        "cible".to_string(),
        "tier1.yml".to_string(),
        "main".to_string(),
        Redacted::new("jeton-essai".to_string()),
        adresse.to_string(),
    )
}

fn plan_production() -> PlanChangement {
    PlanChangement::new(
        Action::TerraformApply,
        Environnement::Production,
        Tier::One,
        "projet/topologie".to_string(),
        "applique la déclaration".to_string(),
        "+ 1 instance".to_string(),
    )
    .expect("plan")
}

const PROTEGE: &str = r#"{"name":"production","protection_rules":[
    {"type":"wait_timer","wait_timer":0},
    {"type":"required_reviewers","prevent_self_review":true,
     "reviewers":[{"type":"User","reviewer":{"login":"relecteur-a"}}]}]}"#;

const SANS_REGLE: &str = r#"{"name":"production","protection_rules":[]}"#;

const AUTO_APPROBATION: &str = r#"{"name":"production","protection_rules":[
    {"type":"required_reviewers","prevent_self_review":false,
     "reviewers":[{"type":"User","reviewer":{"login":"relecteur-a"}}]}]}"#;

const SANS_RELECTEUR: &str = r#"{"name":"production","protection_rules":[
    {"type":"required_reviewers","prevent_self_review":true,"reviewers":[]}]}"#;

fn refus_attendu(resultat: Result<EtatApprobation, AppError>) -> String {
    match resultat {
        Err(e @ AppError::EnvironnementNonProtege { .. }) => e.to_string(),
        autre => panic!("refus EnvironnementNonProtege attendu, obtenu {autre:?}"),
    }
}

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_un_environnement_protege_laisse_partir_le_dispatch() {
    let serveur = lancer_serveur(Some(PROTEGE));
    let etat = passerelle(&serveur.adresse)
        .soumettre(&plan_production())
        .expect("soumission");
    assert!(matches!(etat, EtatApprobation::EnAttente { .. }));
    let recues = serveur.recues();
    assert!(recues
        .iter()
        .any(|r| r == "GET /repos/gilmry/cible/environments/production"));
    assert!(recues
        .iter()
        .any(|r| r == "POST /repos/gilmry/cible/actions/workflows/tier1.yml/dispatches"));
}

// ── @negative ────────────────────────────────────────────────

#[test]
fn negative_un_environnement_absent_bloque_le_dispatch_et_est_nomme() {
    let serveur = lancer_serveur(None);
    let message = refus_attendu(passerelle(&serveur.adresse).soumettre(&plan_production()));
    assert!(message.contains("production"), "{message}");
    assert!(message.contains("n'existe pas"), "{message}");
    assert!(!serveur.a_dispatche(), "aucun dispatch ne doit partir");
}

// ── @edge ────────────────────────────────────────────────────

#[test]
fn edge_un_environnement_sans_regle_est_refuse_comme_absent() {
    let serveur = lancer_serveur(Some(SANS_REGLE));
    refus_attendu(passerelle(&serveur.adresse).soumettre(&plan_production()));
    assert!(!serveur.a_dispatche());
}

#[test]
fn edge_une_regle_de_relecture_sans_relecteur_est_refusee() {
    let serveur = lancer_serveur(Some(SANS_RELECTEUR));
    refus_attendu(passerelle(&serveur.adresse).soumettre(&plan_production()));
    assert!(!serveur.a_dispatche());
}

// ── @security ────────────────────────────────────────────────

#[test]
fn security_l_auto_approbation_permise_refuse_le_dispatch() {
    let serveur = lancer_serveur(Some(AUTO_APPROBATION));
    let message = refus_attendu(passerelle(&serveur.adresse).soumettre(&plan_production()));
    assert!(message.contains("prevent_self_review"), "{message}");
    assert!(!serveur.a_dispatche());
}
