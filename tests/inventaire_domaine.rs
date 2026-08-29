//! Story 1.1 — Types du domaine d'inventaire.

use std::str::FromStr;

use sluis::domain::{Environnement, ModuleTerraform, ProfilCluster, Topologie};

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_les_topologies_se_lisent_depuis_un_nom_de_dossier() {
    assert_eq!(Topologie::from_str("vps").unwrap(), Topologie::Vps);
    assert_eq!(Topologie::from_str("k3s").unwrap(), Topologie::K3s);
    assert_eq!(Topologie::from_str("k8s").unwrap(), Topologie::K8s);
}

#[test]
fn happy_la_promotion_suit_l_ordre_des_etages() {
    assert_eq!(
        Environnement::Dev
            .promouvoir_vers(Environnement::Integration)
            .unwrap(),
        Environnement::Integration
    );
    assert_eq!(
        Environnement::Staging
            .promouvoir_vers(Environnement::Production)
            .unwrap(),
        Environnement::Production
    );
}

// ── @negative ────────────────────────────────────────────────

#[test]
fn negative_une_topologie_inconnue_produit_une_erreur_typee() {
    let erreur = Topologie::from_str("openshift").unwrap_err();
    assert!(erreur.to_string().contains("openshift"));
}

#[test]
fn negative_un_environnement_inconnu_produit_une_erreur_typee() {
    let erreur = Environnement::from_str("preprod").unwrap_err();
    assert!(erreur.to_string().contains("preprod"));
}

#[test]
fn negative_un_saut_d_etage_est_refuse() {
    let erreur = Environnement::Integration
        .promouvoir_vers(Environnement::Production)
        .unwrap_err();
    assert!(
        erreur.to_string().contains("staging"),
        "l'erreur doit nommer l'étage sauté, obtenu : {erreur}"
    );
}

#[test]
fn negative_un_profil_sans_nom_est_refuse() {
    assert!(ProfilCluster::new("   ".to_string(), None, None, None, None, None).is_err());
}

#[test]
fn negative_un_module_sans_nom_est_refuse() {
    assert!(ModuleTerraform::new(String::new()).is_err());
}

// ── @edge ────────────────────────────────────────────────────

#[test]
fn edge_production_ne_peut_etre_promue_nulle_part() {
    let erreur = Environnement::Production
        .promouvoir_vers(Environnement::Dev)
        .unwrap_err();
    assert!(erreur.to_string().contains("dernier étage"));
}

#[test]
fn edge_la_casse_et_les_espaces_sont_tolerés_a_la_lecture() {
    assert_eq!(Topologie::from_str("  K3S  ").unwrap(), Topologie::K3s);
    assert_eq!(
        Environnement::from_str("\tProduction\n").unwrap(),
        Environnement::Production
    );
}

#[test]
fn edge_un_profil_qui_ne_surcharge_rien_reste_valide() {
    let profil = ProfilCluster::new("minimal".to_string(), None, None, None, None, None).unwrap();
    assert_eq!(profil.nom(), "minimal");
    assert_eq!(profil.classe_stockage(), None);
}

// ── @security ────────────────────────────────────────────────

#[test]
fn security_l_enumeration_est_fermee_aucun_nom_forge_ne_cree_de_variante() {
    // Un nom d'environnement venu d'un chemin, d'un argument d'outil ou d'un
    // JSON ne doit jamais produire une variante inconnue qui contournerait
    // ensuite les vérifications de tier.
    for forge in [
        "production ",
        "prod",
        "PRODUCTION_",
        "../production",
        "production\0",
        "",
    ] {
        let lu = Environnement::from_str(forge);
        if let Ok(environnement) = lu {
            assert!(
                Environnement::TOUS.contains(&environnement),
                "« {forge} » a produit une variante hors énumération"
            );
        }
    }
}

#[test]
fn security_la_promotion_ne_peut_jamais_atteindre_production_en_un_saut() {
    // Le chemin le plus dangereux du dispositif : atteindre production sans
    // passer par les étages. Aucun environnement de départ ne doit le permettre,
    // hormis staging.
    for depart in Environnement::TOUS {
        let atteint = depart.promouvoir_vers(Environnement::Production).is_ok();
        assert_eq!(
            atteint,
            depart == Environnement::Staging,
            "{depart} ne devrait pas pouvoir atteindre production directement"
        );
    }
}
