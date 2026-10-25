//! Story 34.4 — comparaison pure de la section `verification-empreinte`.

use sluis::domain::section_empreinte::{extraire_section, verifier_section};

const GABARIT: &str = include_str!("../.github/workflows/tier1-apply.yml.example");

#[test]
fn happy_le_gabarit_est_conforme_a_lui_meme() {
    assert_eq!(verifier_section(GABARIT, GABARIT), Ok(()));
}

#[test]
fn negative_une_ligne_ajoutee_dans_la_section_est_refusee() {
    let ajoute = GABARIT.replace(
        "# <<< verification-empreinte",
        "exit 0\n          # <<< verification-empreinte",
    );
    assert!(verifier_section(GABARIT, &ajoute).is_err());
}

#[test]
fn negative_une_ligne_retiree_de_la_section_est_refusee() {
    let retire = GABARIT.replace("            exit 1\n", "");
    assert_ne!(retire, GABARIT, "la mutation doit porter");
    let erreur = verifier_section(GABARIT, &retire).expect_err("refusée");
    assert!(erreur.contains("ligne"), "{erreur}");
}

#[test]
fn edge_marqueurs_absents_ou_inverses() {
    assert!(extraire_section("name: rien").is_err());
    let inverse = "# <<< verification-empreinte\n# >>> verification-empreinte\n";
    assert!(extraire_section(inverse)
        .expect_err("inversés")
        .contains("précède"));
}

#[test]
fn security_un_espace_final_ajoute_est_refuse() {
    // Seule l'indentation de tête est ignorée : un caractère de plus, même
    // invisible, est un écart.
    let espace = GABARIT.replace("exit 1\n", "exit 1 \n");
    assert_ne!(espace, GABARIT, "la mutation doit porter");
    assert!(verifier_section(GABARIT, &espace).is_err());
}
