//! Story 34.3 — le schéma versionné du contrat du dépôt cible (ADR-034).
//!
//! Un schéma publié ne vaut que s'il dit ce que les lecteurs font. Ce harnais
//! le prouve dans les deux sens, sur le modèle d'ADR-005 : le fichier publié
//! est celui que produit le code, et chaque clé qu'il annonce est réellement
//! exigée par le lecteur.

use sluis::application::ports::DepotCharge;
use sluis::domain::contrat_depot::cles_charge;
use sluis::domain::{ContratDepot, Environnement, Topologie, VERSION_CONTRAT_DEPOT};
use sluis::infrastructure::fs_charge::FsCharge;

const SCHEMA_PUBLIE: &str = include_str!("../contrat/depot-cible.v1.json");

/// Une valeur valide pour chaque clé de `charge.yaml`.
fn valeur(cle: &str) -> &'static str {
    match cle {
        cles_charge::TOPOLOGIE => "vps",
        cles_charge::MODULE => "monosite/vps/bac-a-sable/terraform",
        cles_charge::SORTIE_ADRESSE => "vps_ip",
        cles_charge::CHEMIN => "/api/sante",
        cles_charge::CIBLE_RPS => "200",
        cles_charge::CIBLE_P99 => "300",
        cles_charge::BORNE_TTL => "3600",
        cles_charge::BORNE_PLAFOND => "20",
        autre => panic!("clé du contrat sans valeur de test : {autre}"),
    }
}

/// Écrit une `charge.yaml` à partir des seules clés du schéma, sauf `omise`.
///
/// Les clés pointées deviennent des blocs indentés : c'est la forme que la
/// skill générera, lue par l'analyseur restreint.
fn charge_depuis_le_schema(omise: Option<&str>) -> String {
    let mut yaml = String::new();
    let mut bloc_ouvert: Option<&str> = None;
    for cle in ContratDepot::courant().cles_charge {
        if Some(cle) == omise {
            continue;
        }
        match cle.split_once('.') {
            Some((parent, enfant)) => {
                if bloc_ouvert != Some(parent) {
                    yaml.push_str(&format!("{parent}:\n"));
                    bloc_ouvert = Some(parent);
                }
                yaml.push_str(&format!("  {enfant}: {}\n", valeur(cle)));
            }
            None => {
                bloc_ouvert = None;
                yaml.push_str(&format!("{cle}: {}\n", valeur(cle)));
            }
        }
    }
    yaml
}

/// Dépôt temporaire, supprimé à la sortie de portée.
struct DepotTemporaire(std::path::PathBuf);

impl DepotTemporaire {
    fn avec_charge(nom: &str, contenu: &str) -> Self {
        let racine = std::env::temp_dir().join(format!("sluis-contrat-{nom}"));
        let _ = std::fs::remove_dir_all(&racine);
        let charge = ContratDepot::courant().arborescence.charge;
        let chemin = racine.join(&charge);
        std::fs::create_dir_all(chemin.parent().expect("parent")).expect("création");
        std::fs::write(chemin, contenu).expect("écriture");
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

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_le_schema_publie_est_celui_du_code() {
    let genere = serde_json::to_string_pretty(&ContratDepot::courant()).expect("sérialisation");
    assert_eq!(
        SCHEMA_PUBLIE.trim_end(),
        genere,
        "contrat/depot-cible.v1.json a dérivé du code : le régénérer avec `make contrat`"
    );
}

#[test]
fn happy_le_fichier_publie_porte_la_version_du_code() {
    let publie: serde_json::Value = serde_json::from_str(SCHEMA_PUBLIE).expect("JSON valide");
    assert_eq!(publie["version"], VERSION_CONTRAT_DEPOT);
}

#[test]
fn happy_une_charge_ecrite_depuis_le_schema_est_lue() {
    let depot = DepotTemporaire::avec_charge("complete", &charge_depuis_le_schema(None));

    let declaration = FsCharge::new().lire(depot.racine()).expect("lecture");

    assert_eq!(declaration.topologie(), "vps");
    assert_eq!(declaration.cible().requetes_par_seconde(), 200.0);
}

// ── @negative ────────────────────────────────────────────────

#[test]
fn negative_chaque_cle_annoncee_est_exigee_par_le_lecteur() {
    for cle in ContratDepot::courant().cles_charge {
        let depot = DepotTemporaire::avec_charge(
            &format!("sans-{}", cle.replace('.', "-")),
            &charge_depuis_le_schema(Some(cle)),
        );

        let erreur = FsCharge::new().lire(depot.racine()).expect_err(&format!(
            "le schéma annonce « {cle} » comme requise, le lecteur l'a ignorée"
        ));

        assert!(
            erreur.to_string().contains(cle),
            "l'erreur doit nommer la clé manquante « {cle} » : {erreur}"
        );
    }
}

// ── @edge ────────────────────────────────────────────────────

#[test]
fn edge_topologies_et_environnements_sont_exactement_ceux_du_domaine() {
    let arborescence = ContratDepot::courant().arborescence;

    let topologies: Vec<&str> = Topologie::TOUTES.iter().map(Topologie::nom).collect();
    let environnements: Vec<&str> = Environnement::TOUS.iter().map(Environnement::nom).collect();

    assert_eq!(arborescence.topologies, topologies);
    assert_eq!(arborescence.environnements, environnements);
}

// ── @security ────────────────────────────────────────────────

#[test]
fn security_le_schema_reprend_les_interdictions_verifiees_avant_tofu_init() {
    let tofu = ContratDepot::courant().tofu;

    for attendue in [
        "provisioner \"local-exec\"",
        "provisioner \"remote-exec\"",
        "data \"external\"",
    ] {
        assert!(
            tofu.constructions_interdites.iter().any(|c| c == attendue),
            "interdiction absente du schéma : {attendue}"
        );
    }
    assert_eq!(tofu.verrou_obligatoire, ".terraform.lock.hcl");
    assert!(tofu.extensions_refusees.contains(&".tf.json"));
}

#[test]
fn security_le_schema_ne_porte_aucune_valeur_de_deploiement() {
    // Le schéma est public et partagé par tous les dépôts : ni secret, ni
    // fournisseur autorisé, ni nom de dépôt. Ces valeurs vivent dans la
    // configuration de chaque déploiement de Sluis.
    for interdit in ["token", "password", "gilmry", "registry.terraform.io"] {
        assert!(
            !SCHEMA_PUBLIE.to_lowercase().contains(interdit),
            "le schéma publié contient « {interdit} »"
        );
    }
}
