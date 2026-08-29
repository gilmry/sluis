//! Stories 2.1 et 2.2 — plans de changement, empreintes et jetons.

use sluis::domain::{
    Action, Duree, Empreinte, Environnement, Horodatage, JetonChangement, PlanChangement, Tier,
};

fn plan_de_test(
    environnement: Environnement,
    tier: Tier,
) -> Result<PlanChangement, sluis::domain::AppError> {
    PlanChangement::new(
        Action::TerraformApply,
        environnement,
        tier,
        "projet/topologie".to_string(),
        "applique la déclaration".to_string(),
        "+ 1 instance".to_string(),
    )
}

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_un_plan_tier1_sur_production_se_construit() {
    let plan = plan_de_test(Environnement::Production, Tier::One).expect("plan");
    assert_eq!(plan.tier(), Tier::One);
    assert_eq!(plan.environnement(), Environnement::Production);
    assert_eq!(plan.empreinte().hexadecimal().len(), 64);
}

#[test]
fn happy_un_jeton_valide_se_consomme_et_rend_une_preuve() {
    let plan = plan_de_test(Environnement::Production, Tier::One).expect("plan");
    let maintenant = Horodatage::new(1_000);
    let jeton = JetonChangement::emettre(
        plan.empreinte().clone(),
        "Gilles Maury".to_string(),
        maintenant,
        Duree::secondes(600).unwrap(),
    )
    .expect("émission");

    let consomme = jeton
        .consommer(&plan, Horodatage::new(1_100))
        .expect("consommation");
    assert_eq!(consomme.approbateur(), "Gilles Maury");
    assert_eq!(consomme.empreinte(), plan.empreinte());
}

// ── @negative ────────────────────────────────────────────────

#[test]
fn negative_un_plan_tier2_visant_production_est_refuse() {
    let erreur = plan_de_test(Environnement::Production, Tier::Two).unwrap_err();
    assert!(
        erreur.to_string().contains("production"),
        "obtenu : {erreur}"
    );
}

#[test]
fn negative_une_action_dangereuse_ne_peut_pas_etre_classee_tier2() {
    // Même sur dev : `terraform apply` est nommément Tier 1 dans
    // AGENT_GUARDRAILS.md, l'environnement n'y change rien.
    let erreur = plan_de_test(Environnement::Dev, Tier::Two).unwrap_err();
    assert!(erreur.to_string().contains("Tier 1"), "obtenu : {erreur}");
}

#[test]
fn negative_un_plan_sans_cible_est_refuse() {
    let erreur = PlanChangement::new(
        Action::MiseEnLigne,
        Environnement::Staging,
        Tier::One,
        "  ".to_string(),
        "d".to_string(),
        "".to_string(),
    )
    .unwrap_err();
    assert!(erreur.to_string().contains("cible"));
}

#[test]
fn negative_un_jeton_sans_approbateur_est_refuse() {
    let erreur = JetonChangement::emettre(
        Empreinte::calculer("x"),
        "   ".to_string(),
        Horodatage::new(0),
        Duree::secondes(60).unwrap(),
    )
    .unwrap_err();
    assert!(
        erreur.to_string().contains("imputable"),
        "une approbation anonyme n'est pas une approbation, obtenu : {erreur}"
    );
}

#[test]
fn negative_un_jeton_pour_une_autre_empreinte_est_rejete() {
    let plan = plan_de_test(Environnement::Staging, Tier::One).expect("plan");
    let autre = PlanChangement::new(
        Action::TerraformDestroy,
        Environnement::Staging,
        Tier::One,
        "autre".to_string(),
        "détruit".to_string(),
        "- 1 instance".to_string(),
    )
    .expect("autre plan");

    let jeton = JetonChangement::emettre(
        autre.empreinte().clone(),
        "approbateur".to_string(),
        Horodatage::new(0),
        Duree::secondes(600).unwrap(),
    )
    .expect("émission");

    let erreur = jeton.consommer(&plan, Horodatage::new(10)).unwrap_err();
    assert!(
        erreur.to_string().contains("empreinte"),
        "obtenu : {erreur}"
    );
}

#[test]
fn negative_une_duree_nulle_ou_negative_est_refusee() {
    assert!(Duree::secondes(0).is_err());
    assert!(Duree::secondes(-1).is_err());
}

// ── @edge ────────────────────────────────────────────────────

#[test]
fn edge_un_jeton_expire_a_la_seconde_pres_est_rejete() {
    let plan = plan_de_test(Environnement::Staging, Tier::One).expect("plan");
    let jeton = JetonChangement::emettre(
        plan.empreinte().clone(),
        "a".to_string(),
        Horodatage::new(1_000),
        Duree::secondes(60).unwrap(),
    )
    .expect("émission");
    assert_eq!(jeton.expire_le(), Horodatage::new(1_060));

    // À l'instant exact d'expiration, le jeton vaut encore : c'est « après »
    // qui invalide, et cette frontière doit être choisie, pas subie.
    let a_la_seconde = JetonChangement::emettre(
        plan.empreinte().clone(),
        "a".to_string(),
        Horodatage::new(1_000),
        Duree::secondes(60).unwrap(),
    )
    .expect("émission");
    assert!(a_la_seconde
        .consommer(&plan, Horodatage::new(1_060))
        .is_ok());

    let apres = JetonChangement::emettre(
        plan.empreinte().clone(),
        "a".to_string(),
        Horodatage::new(1_000),
        Duree::secondes(60).unwrap(),
    )
    .expect("émission");
    assert!(apres.consommer(&plan, Horodatage::new(1_061)).is_err());
}

#[test]
fn edge_l_ajout_de_duree_sature_au_lieu_de_deborder() {
    let tard = Horodatage::new(i64::MAX - 5);
    let encore_plus_tard = tard.plus(Duree::secondes(1_000).unwrap());
    assert!(
        encore_plus_tard.secondes() >= tard.secondes(),
        "un débordement produirait une date passée, donc une expiration immédiate"
    );
}

#[test]
fn edge_les_actions_de_bac_a_sable_sont_les_seules_admises_en_tier2() {
    let admises = [Action::LocationBacASable, Action::DestructionBacASable];
    for action in [
        Action::TerraformApply,
        Action::TerraformDestroy,
        Action::HelmUpgrade,
        Action::HelmUninstall,
        Action::ArgocdSync,
        Action::VeleroRestore,
        Action::MiseEnLigne,
        Action::RenouvellementDerogation,
        Action::LocationBacASable,
        Action::DestructionBacASable,
    ] {
        let attendu = if admises.contains(&action) {
            Tier::Two
        } else {
            Tier::One
        };
        assert_eq!(
            action.tier_minimal(),
            attendu,
            "tier minimal inattendu pour {action}"
        );
    }
}

// ── @security ────────────────────────────────────────────────

#[test]
fn security_deux_plans_qui_different_d_un_champ_ont_des_empreintes_differentes() {
    // Sinon une approbation donnée pour l'un vaudrait pour l'autre, ce qui est
    // exactement le scénario qu'un attaquant chercherait.
    let base = plan_de_test(Environnement::Staging, Tier::One).expect("plan");
    let variantes = [
        PlanChangement::new(
            Action::TerraformDestroy,
            Environnement::Staging,
            Tier::One,
            "projet/topologie".to_string(),
            "applique la déclaration".to_string(),
            "+ 1 instance".to_string(),
        ),
        PlanChangement::new(
            Action::TerraformApply,
            Environnement::Dev,
            Tier::One,
            "projet/topologie".to_string(),
            "applique la déclaration".to_string(),
            "+ 1 instance".to_string(),
        ),
        PlanChangement::new(
            Action::TerraformApply,
            Environnement::Staging,
            Tier::One,
            "projet/AUTRE".to_string(),
            "applique la déclaration".to_string(),
            "+ 1 instance".to_string(),
        ),
        PlanChangement::new(
            Action::TerraformApply,
            Environnement::Staging,
            Tier::One,
            "projet/topologie".to_string(),
            "applique la déclaration".to_string(),
            "+ 2 instances".to_string(),
        ),
    ];
    for variante in variantes {
        let variante = variante.expect("plan");
        assert_ne!(
            base.empreinte(),
            variante.empreinte(),
            "deux plans distincts partagent une empreinte"
        );
    }
}

#[test]
fn security_l_empreinte_est_stable_pour_un_plan_identique() {
    let a = plan_de_test(Environnement::Staging, Tier::One).expect("plan");
    let b = plan_de_test(Environnement::Staging, Tier::One).expect("plan");
    assert_eq!(a.empreinte(), b.empreinte());
}

#[test]
fn security_aucun_chemin_ne_produit_un_plan_tier2_sur_production() {
    // Énumération exhaustive plutôt qu'échantillon : chaque action, testée.
    for action in [
        Action::TerraformApply,
        Action::TerraformDestroy,
        Action::HelmUpgrade,
        Action::HelmUninstall,
        Action::ArgocdSync,
        Action::VeleroRestore,
        Action::MiseEnLigne,
        Action::RenouvellementDerogation,
        Action::LocationBacASable,
        Action::DestructionBacASable,
    ] {
        let resultat = PlanChangement::new(
            action.clone(),
            Environnement::Production,
            Tier::Two,
            "cible".to_string(),
            "d".to_string(),
            String::new(),
        );
        assert!(
            resultat.is_err(),
            "l'action {action} a produit un plan Tier 2 sur production"
        );
    }
}

#[test]
fn security_un_jeton_consomme_ne_peut_pas_etre_rejoue() {
    // Le rejeu n'est pas testé à l'exécution : `consommer` prend `self` par
    // valeur, donc une seconde consommation ne compile pas. Ce test documente
    // la garantie et vérifie la seule chose observable : la preuve rendue est
    // bien liée au plan consommé.
    let plan = plan_de_test(Environnement::Staging, Tier::One).expect("plan");
    let jeton = JetonChangement::emettre(
        plan.empreinte().clone(),
        "a".to_string(),
        Horodatage::new(0),
        Duree::secondes(60).unwrap(),
    )
    .expect("émission");
    let consomme = jeton.consommer(&plan, Horodatage::new(1)).expect("ok");
    assert_eq!(consomme.empreinte(), plan.empreinte());
    // jeton.consommer(...) ici serait : error[E0382]: use of moved value
}
