//! Story 5.7 — ce qu'un projet Foyer déclare pour être mesuré sous charge.
//!
//! Deux tests portent le fichier. Le premier est
//! `security_un_projet_ne_peut_pas_s_octroyer_plus_que_le_serveur` : la
//! déclaration vit dans le dépôt mesuré, donc quiconque écrit dans ce dépôt
//! l'écrit. Si elle pouvait relever les bornes, elle deviendrait un moyen de
//! contourner ADR-007 par une pull request.
//!
//! Le second est `edge_un_palier_de_reference_non_joue_rend_un_verdict_indetermine` :
//! l'abaque distingue le mesuré du supposé, et un verdict rendu sur un palier
//! qui n'a pas tourné serait du supposé déguisé en mesuré.

use sluis::domain::{
    CibleCapacite, DeclarationCharge, Duree, MesureCapacite, Palier, PlafondDepense, Verdict,
};

fn declaration(ttl: i64, plafond: f64) -> DeclarationCharge {
    DeclarationCharge::new(
        "vps".to_string(),
        "monosite/vps/bac-a-sable/terraform".to_string(),
        "vps_ip".to_string(),
        "/api/sante".to_string(),
        CibleCapacite::new(200.0, 300.0).expect("cible"),
        Duree::secondes(ttl).expect("ttl"),
        PlafondDepense::new(plafond).expect("plafond"),
    )
    .expect("déclaration")
}

fn mesure(grandeur: &str, valeur: f64, unite: &str, palier: Palier) -> MesureCapacite {
    MesureCapacite::mesuree(
        grandeur.to_string(),
        valeur,
        unite.to_string(),
        palier,
        10_000,
        "campagne de test".to_string(),
    )
    .expect("mesure")
}

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_une_declaration_complete_est_admise() {
    let declaration = declaration(3_600, 20.0);

    assert_eq!(declaration.topologie(), "vps");
    assert_eq!(declaration.chemin(), "/api/sante");
    assert_eq!(declaration.sortie_adresse(), "vps_ip");
}

#[test]
fn happy_une_cible_atteinte_rend_un_verdict_favorable() {
    let declaration = declaration(3_600, 20.0);
    let mesures = vec![
        mesure("debit", 240.0, "req/s", Palier::Realistic),
        mesure("latence_p99", 210.0, "ms", Palier::Realistic),
    ];

    assert!(matches!(
        declaration.verdict(&mesures),
        Verdict::Tient { .. }
    ));
}

// ── @negative ────────────────────────────────────────────────

#[test]
fn negative_une_cible_manquee_nomme_chacun_de_ses_motifs() {
    let declaration = declaration(3_600, 20.0);
    let mesures = vec![
        mesure("debit", 90.0, "req/s", Palier::Realistic),
        mesure("latence_p99", 480.0, "ms", Palier::Realistic),
    ];

    let Verdict::NeTientPas { motifs, .. } = declaration.verdict(&mesures) else {
        panic!("la cible est manquée sur les deux grandeurs");
    };
    assert_eq!(
        motifs.len(),
        2,
        "un motif par grandeur manquée : {motifs:?}"
    );
    assert!(motifs.iter().any(|m| m.contains("débit")));
    assert!(motifs.iter().any(|m| m.contains("p99")));
}

#[test]
fn negative_un_module_vide_est_refuse_a_la_construction() {
    let erreur = DeclarationCharge::new(
        "vps".to_string(),
        String::new(),
        "vps_ip".to_string(),
        "/api/sante".to_string(),
        CibleCapacite::new(200.0, 300.0).expect("cible"),
        Duree::secondes(3_600).expect("ttl"),
        PlafondDepense::new(20.0).expect("plafond"),
    )
    .unwrap_err();

    assert!(erreur.to_string().contains("module"));
}

#[test]
fn negative_une_cible_nulle_est_refusee() {
    // Une cible à zéro serait toujours atteinte : elle rendrait le verdict
    // décoratif, ce qui est pire que pas de verdict du tout.
    assert!(CibleCapacite::new(0.0, 300.0).is_err());
    assert!(CibleCapacite::new(200.0, 0.0).is_err());
}

// ── @edge ────────────────────────────────────────────────────

#[test]
fn edge_un_palier_de_reference_non_joue_rend_un_verdict_indetermine() {
    let declaration = declaration(3_600, 20.0);
    // La campagne s'est arrêtée avant le palier réaliste : le débit maximal
    // observé ne dit rien de la tenue en conditions.
    let mesures = vec![
        mesure("debit", 240.0, "req/s", Palier::Light),
        mesure("latence_p99", 120.0, "ms", Palier::Light),
    ];

    let verdict = declaration.verdict(&mesures);

    assert!(
        matches!(verdict, Verdict::Indetermine { .. }),
        "sans palier de référence, aucun verdict ne doit être affirmé : {verdict:?}"
    );
}

#[test]
fn edge_aucune_mesure_rend_un_verdict_indetermine() {
    assert!(matches!(
        declaration(3_600, 20.0).verdict(&[]),
        Verdict::Indetermine { .. }
    ));
}

// ── @security ────────────────────────────────────────────────

#[test]
fn security_un_projet_ne_peut_pas_s_octroyer_plus_que_le_serveur() {
    // Le dépôt mesuré demande six heures et deux cents euros.
    let gourmande = declaration(21_600, 200.0);
    let ttl_serveur = Duree::secondes(3_600).expect("ttl serveur");
    let plafond_serveur = PlafondDepense::new(20.0).expect("plafond serveur");

    assert_eq!(gourmande.ttl_borne(ttl_serveur), ttl_serveur);
    assert_eq!(
        gourmande.plafond_borne(plafond_serveur).montant(),
        plafond_serveur.montant()
    );
}

#[test]
fn security_un_projet_plus_modeste_que_le_serveur_garde_sa_modestie() {
    // Le bornage retient le minimum des deux, pas celui du serveur : un projet
    // qui se sait peu coûteux ne doit pas se voir accorder six heures.
    let modeste = declaration(600, 2.0);

    assert_eq!(
        modeste.ttl_borne(Duree::secondes(21_600).expect("max")),
        Duree::secondes(600).expect("demandé")
    );
    assert_eq!(
        modeste
            .plafond_borne(PlafondDepense::new(20.0).expect("plafond"))
            .montant(),
        2.0
    );
}
