//! Story 34.6a — `sluis_verifier_depot`, verdict de structure (ADR-034, point 7).

use std::path::PathBuf;

use serde_json::{json, Value};
use sluis::infrastructure::mcp::outils_lecture::OutilVerifierDepot;
use sluis::infrastructure::mcp::Outil;

const GABARIT: &str = include_str!("../.github/workflows/tier1-apply.yml.example");

const CHARGE: &str = "topologie: vps\nmodule: monosite/vps/bac-a-sable/terraform\n\
sortie_adresse: vps_ip\nchemin: /api/sante\ncible:\n  requetes_par_seconde: 200\n  \
p99_millisecondes: 300\nbornes:\n  ttl_secondes: 3600\n  plafond_depense: 20\n";

/// Un dépôt cible conforme, que chaque scénario abîme à sa façon.
struct Depot(PathBuf);

impl Depot {
    fn conforme(nom: &str) -> Self {
        let racine = std::env::temp_dir().join(format!("sluis-verif-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&racine);
        let depot = Self(racine);
        depot.ecrire("infrastructure/_shared/charge.yaml", CHARGE);
        depot.ecrire("infrastructure/monosite/vps/staging/main.tf", "");
        depot.ecrire(".github/workflows/tier1-apply.yml", GABARIT);
        depot
    }
    fn ecrire(&self, relatif: &str, contenu: &str) {
        let chemin = self.0.join(relatif);
        std::fs::create_dir_all(chemin.parent().expect("parent")).expect("création");
        std::fs::write(chemin, contenu).expect("écriture");
    }
    fn retirer(&self, relatif: &str) {
        std::fs::remove_file(self.0.join(relatif)).expect("suppression");
    }
    fn verdict(&self) -> Value {
        OutilVerifierDepot
            .appeler(&json!({ "racine": self.0.to_str().expect("chemin") }))
            .expect("verdict")
    }
}

impl Drop for Depot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn regles(verdict: &Value) -> Vec<String> {
    verdict["ecarts"]
        .as_array()
        .expect("écarts")
        .iter()
        .map(|e| e["regle"].as_str().expect("règle").to_string())
        .collect()
}

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_un_depot_conforme_rend_un_verdict_conforme() {
    let depot = Depot::conforme("happy");

    let verdict = depot.verdict();

    assert_eq!(verdict["conforme"], true, "{verdict}");
    assert_eq!(verdict["ecarts"], json!([]));
    assert_eq!(verdict["version_contrat"], 1);
}

// ── @negative ────────────────────────────────────────────────

#[test]
fn negative_chaque_manquement_est_un_ecart_nomme() {
    let depot = Depot::conforme("negative");
    depot.retirer(".github/workflows/tier1-apply.yml");
    depot.ecrire(
        "infrastructure/_shared/charge.yaml",
        &CHARGE.replace("chemin: /api/sante\n", ""),
    );

    let verdict = depot.verdict();

    assert_eq!(verdict["conforme"], false);
    let regles = regles(&verdict);
    assert!(regles.contains(&"workflow-tier1".to_string()), "{verdict}");
    assert!(regles.contains(&"charge".to_string()), "{verdict}");
    assert!(
        verdict.to_string().contains("chemin"),
        "la clé doit être nommée"
    );
}

#[test]
fn negative_une_section_modifiee_est_un_ecart() {
    let depot = Depot::conforme("section");
    depot.ecrire(
        ".github/workflows/tier1-apply.yml",
        &GABARIT.replace("exit 1", "exit 0"),
    );

    let verdict = depot.verdict();

    assert_eq!(
        regles(&verdict),
        vec!["workflow-tier1".to_string()],
        "{verdict}"
    );
}

#[test]
fn negative_sans_infrastructure_ni_cellule() {
    let depot = Depot::conforme("vide");
    let _ = std::fs::remove_dir_all(depot.0.join("infrastructure/monosite"));
    assert!(regles(&depot.verdict()).contains(&"cellules".to_string()));

    let _ = std::fs::remove_dir_all(depot.0.join("infrastructure"));
    assert_eq!(regles(&depot.verdict()), vec!["arborescence".to_string()]);
}

// ── @edge ────────────────────────────────────────────────────

#[test]
fn edge_un_dossier_inconnu_est_une_remarque_pas_un_ecart() {
    let depot = Depot::conforme("edge");
    depot.ecrire("infrastructure/monosite/vps/local/main.tf", "");

    let verdict = depot.verdict();

    assert_eq!(verdict["conforme"], true, "{verdict}");
    assert!(
        verdict["remarques"].to_string().contains("vps/local"),
        "{verdict}"
    );
}

// ── @security ────────────────────────────────────────────────

#[cfg(unix)]
#[test]
fn security_une_charge_qui_pointe_hors_du_depot_est_refusee_sans_etre_lue() {
    let depot = Depot::conforme("lien");
    let exterieur = std::env::temp_dir().join(format!("sluis-exterieur-{}", std::process::id()));
    std::fs::write(&exterieur, "secret_exterieur: valeur-a-ne-pas-citer\n").expect("écriture");
    depot.retirer("infrastructure/_shared/charge.yaml");
    std::os::unix::fs::symlink(
        &exterieur,
        depot.0.join("infrastructure/_shared/charge.yaml"),
    )
    .expect("lien");

    let verdict = depot.verdict();
    let _ = std::fs::remove_file(&exterieur);

    assert!(
        regles(&verdict).contains(&"charge".to_string()),
        "{verdict}"
    );
    assert!(
        !verdict.to_string().contains("valeur-a-ne-pas-citer"),
        "le contenu extérieur ne doit pas apparaître : {verdict}"
    );
}
