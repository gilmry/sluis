//! Story 5.8 — découvrir la déclaration de charge d'un dépôt.
//!
//! Même principe que l'inventaire : Sluis lit ce que le projet déclare, à
//! l'endroit où le cadre le prévoit, sans aucune saisie. Un dépôt qui ne
//! déclare rien n'est pas mesurable, et le dire vaut mieux que de deviner.

use sluis::application::ports::DepotCharge;
use sluis::infrastructure::fs_charge::FsCharge;

/// Écrit un dépôt d'infrastructure temporaire, supprimé à la sortie de portée.
struct DepotTemporaire(std::path::PathBuf);

impl DepotTemporaire {
    fn avec(nom: &str, charge: Option<&str>) -> Self {
        let racine = std::env::temp_dir().join(format!("sluis-charge-{nom}"));
        let _ = std::fs::remove_dir_all(&racine);
        std::fs::create_dir_all(racine.join("_shared")).expect("création");
        if let Some(contenu) = charge {
            std::fs::write(racine.join("_shared").join("charge.yaml"), contenu).expect("écriture");
        }
        Self(racine)
    }
    fn racine(&self) -> &str {
        self.0.to_str().expect("chemin")
    }
}

impl Drop for DepotTemporaire {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const DECLARATION: &str = r#"
# Ce qu'un projet Foyer déclare pour être mesurable sous charge.
topologie: vps
module: monosite/vps/bac-a-sable/terraform
sortie_adresse: vps_ip
chemin: /api/sante

cible:
  requetes_par_seconde: 200
  p99_millisecondes: 300

bornes:
  ttl_secondes: 3600
  plafond_depense: 20
"#;

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_une_declaration_est_lue_telle_qu_elle_est_ecrite() {
    let depot = DepotTemporaire::avec("complete", Some(DECLARATION));

    let declaration = FsCharge::new().lire(depot.racine()).expect("lecture");

    assert_eq!(declaration.topologie(), "vps");
    assert_eq!(declaration.module(), "monosite/vps/bac-a-sable/terraform");
    assert_eq!(declaration.chemin(), "/api/sante");
    assert_eq!(declaration.cible().requetes_par_seconde(), 200.0);
    assert_eq!(declaration.cible().p99_millisecondes(), 300.0);
}

// ── @negative ────────────────────────────────────────────────

#[test]
fn negative_un_depot_sans_declaration_le_dit_au_lieu_de_deviner() {
    let depot = DepotTemporaire::avec("sans", None);

    let erreur = FsCharge::new()
        .lire(depot.racine())
        .expect_err("aucune déclaration");

    assert!(
        erreur.to_string().contains("charge.yaml"),
        "l'erreur doit nommer le fichier attendu : {erreur}"
    );
}

#[test]
fn negative_une_cle_manquante_est_nommee() {
    let depot = DepotTemporaire::avec(
        "incomplete",
        Some("topologie: vps\nmodule: m\nsortie_adresse: ip\n"),
    );

    let erreur = FsCharge::new()
        .lire(depot.racine())
        .expect_err("déclaration incomplète");

    assert!(
        erreur.to_string().contains("chemin"),
        "la première clé manquante doit être nommée : {erreur}"
    );
}

#[test]
fn negative_une_valeur_non_numerique_est_refusee() {
    let depot = DepotTemporaire::avec(
        "non-numerique",
        Some(
            "topologie: vps\nmodule: m\nsortie_adresse: ip\nchemin: /x\n\
             cible:\n  requetes_par_seconde: beaucoup\n  p99_millisecondes: 300\n\
             bornes:\n  ttl_secondes: 600\n  plafond_depense: 5\n",
        ),
    );

    let erreur = FsCharge::new()
        .lire(depot.racine())
        .expect_err("valeur illisible");

    assert!(erreur.to_string().contains("requetes_par_seconde"));
}

// ── @edge ────────────────────────────────────────────────────

#[test]
fn edge_une_racine_inexistante_est_une_erreur_d_entree_sortie() {
    assert!(FsCharge::new().lire("/chemin/qui/n/existe/pas").is_err());
}

// ── @security ────────────────────────────────────────────────

#[test]
fn security_une_remontee_de_chemin_ne_lit_rien_hors_du_depot() {
    let depot = DepotTemporaire::avec("remontee", Some(DECLARATION));
    let hors_racine = format!("{}/_shared/../../..", depot.racine());

    // La racine remonte au-dessus du dépôt : la lecture doit échouer plutôt que
    // de chercher un charge.yaml ailleurs sur la machine.
    assert!(FsCharge::new().lire(&hors_racine).is_err());
}
