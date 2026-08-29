//! Stories 5.3, 6.1, 6.2, 6.3, 7.1 et 7.2 — convergence, campagne, recalage,
//! passerelle d'approbation et mise en ligne.

use std::sync::{Arc, Mutex};

use sluis::application::campagne::{escalier_par_defaut, Campagne};
use sluis::application::mise_en_ligne::{IssueMiseEnLigne, MiseEnLigne};
use sluis::application::ports::{
    EtatApprobation, EtatGate, GatePlancher, MoteurCharge, PasserelleApprobation, ReglagePalier,
    VerificateurGates,
};
use sluis::domain::{
    etablir_convergence, AppError, Empreinte, Environnement, MesureCapacite, Palier, PlanTerraform,
    Prior, Provenance, RapportRecalage,
};
use sluis::infrastructure::charge::{analyser_sortie, duree_en_millisecondes};

fn plan(creations: u32) -> PlanTerraform {
    PlanTerraform {
        creations,
        modifications: 0,
        destructions: 0,
        brut: String::new(),
    }
}

const SORTIE_WRK: &str = "Running 30s test @ http://cible/\n\
  4 threads and 50 connections\n\
  Thread Stats   Avg      Stdev     Max   +/- Stdev\n\
    Latency    12.34ms    5.67ms  89.01ms   70.12%\n\
  Latency Distribution\n\
     50%   10.00ms\n\
     75%   15.00ms\n\
     90%   20.00ms\n\
     99%   50.00ms\n\
  123456 requests in 30.00s, 12.34MB read\n\
Requests/sec:   4115.20\n";

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_un_reapply_sans_ecart_prouve_la_convergence() {
    let preuve = etablir_convergence(&[plan(3), plan(0)], 5).expect("preuve");
    assert!(preuve.convergee);
    assert_eq!(preuve.tours, 2);
    assert!(preuve.ecart_restant.is_none());
}

#[test]
fn happy_la_sortie_wrk_donne_debit_et_latences() {
    let mesures =
        analyser_sortie(SORTIE_WRK, Palier::Medium, "conditions de test").expect("analyse");
    let debit = mesures
        .iter()
        .find(|m| m.grandeur() == "debit")
        .expect("débit");
    assert_eq!(debit.valeur(), 4115.20);
    assert_eq!(debit.echantillon(), 123_456);
    assert_eq!(debit.provenance(), Provenance::Mesure);
    let p99 = mesures
        .iter()
        .find(|m| m.grandeur() == "latence_p99")
        .expect("p99");
    assert_eq!(p99.valeur(), 50.0);
}

#[test]
fn happy_un_rapport_de_recalage_propose_une_valeur_mesuree() {
    let prior = Prior {
        grandeur: "control_plane_k3s".to_string(),
        valeur: 700.0,
        unite: "Mo".to_string(),
        origine: "abaque §6, marqué [caler]".to_string(),
    };
    let mesure = MesureCapacite::mesuree(
        "control_plane_k3s".to_string(),
        512.0,
        "Mo".to_string(),
        Palier::Soak,
        10_000,
        "k3s v1.29 sur d2-4, 30 min de maintien".to_string(),
    )
    .expect("mesure");

    let rapport = RapportRecalage::construire(vec![prior], &[mesure], 10.0);
    assert_eq!(rapport.calibrees(), 1);
    let recalage = &rapport.recalages[0];
    assert!((recalage.ecart_pourcent + 26.857).abs() < 0.01);
    assert!(recalage.notable, "un écart de 27 % mérite attention");
    assert!(
        recalage.mesure.conditions().contains("d2-4"),
        "les conditions doivent accompagner la mesure, sinon elle ne se compare à rien"
    );
}

// ── @negative ────────────────────────────────────────────────

#[test]
fn negative_une_mesure_sans_echantillon_est_refusee() {
    let erreur = MesureCapacite::mesuree(
        "debit".to_string(),
        100.0,
        "req/s".to_string(),
        Palier::Light,
        0,
        "conditions".to_string(),
    )
    .unwrap_err();
    assert!(
        erreur.to_string().contains("supposition"),
        "obtenu : {erreur}"
    );
}

#[test]
fn negative_une_mesure_sans_conditions_est_refusee() {
    assert!(MesureCapacite::mesuree(
        "debit".to_string(),
        1.0,
        "req/s".to_string(),
        Palier::Light,
        10,
        "  ".to_string()
    )
    .is_err());
}

#[test]
fn negative_des_latences_incoherentes_sont_rejetees() {
    let sortie = SORTIE_WRK.replace("99%   50.00ms", "99%   1.00ms");
    let erreur = analyser_sortie(&sortie, Palier::Medium, "c").unwrap_err();
    assert!(
        erreur.to_string().contains("impossible"),
        "un P99 sous la médiane signale un défaut de collecte : {erreur}"
    );
}

#[test]
fn negative_une_sortie_sans_requete_ne_produit_aucune_mesure() {
    let erreur = analyser_sortie("Requests/sec: 10\n", Palier::Light, "c").unwrap_err();
    assert!(erreur.to_string().contains("sans échantillon"));
}

#[test]
fn negative_une_convergence_non_atteinte_echoue_au_lieu_de_boucler() {
    let plans: Vec<PlanTerraform> = (0..10).map(|_| plan(1)).collect();
    let erreur = etablir_convergence(&plans, 3).unwrap_err();
    assert!(erreur.to_string().contains("non atteinte"));
}

// ── @edge ────────────────────────────────────────────────────

#[test]
fn edge_les_unites_de_duree_wrk_sont_toutes_converties() {
    assert_eq!(duree_en_millisecondes("12.5ms"), Some(12.5));
    assert_eq!(duree_en_millisecondes("2.00s"), Some(2000.0));
    assert_eq!(duree_en_millisecondes("500us"), Some(0.5));
    assert_eq!(duree_en_millisecondes("1.5m"), Some(90_000.0));
    assert_eq!(duree_en_millisecondes("n'importe quoi"), None);
}

#[test]
fn edge_un_prior_sans_mesure_est_signale_et_non_tu() {
    let prior = Prior {
        grandeur: "jamais_mesure".to_string(),
        valeur: 1.0,
        unite: "u".to_string(),
        origine: "abaque".to_string(),
    };
    let rapport = RapportRecalage::construire(vec![prior], &[], 10.0);
    assert_eq!(rapport.calibrees(), 0);
    assert_eq!(
        rapport.non_calibres.len(),
        1,
        "taire un prior non calibré laisserait croire que tout a été mesuré"
    );
}

#[test]
fn edge_une_supposition_ne_recale_jamais_un_prior() {
    // Remplacer une supposition par une autre n'apprend rien.
    let prior = Prior {
        grandeur: "x".to_string(),
        valeur: 1.0,
        unite: "u".to_string(),
        origine: "abaque".to_string(),
    };
    let supposition =
        MesureCapacite::supposee("x".to_string(), 2.0, "u".to_string(), "déduit".to_string());
    let rapport = RapportRecalage::construire(vec![prior], &[supposition], 10.0);
    assert_eq!(rapport.calibrees(), 0);
    assert_eq!(rapport.non_calibres.len(), 1);
}

#[test]
fn edge_l_escalier_par_defaut_suit_l_ordre_des_paliers() {
    let escalier = escalier_par_defaut();
    let paliers: Vec<Palier> = escalier.iter().map(|r| r.palier).collect();
    assert_eq!(paliers, Palier::ESCALIER.to_vec());
    assert_eq!(Campagne::duree_totale(&escalier), 910);
}

// ── @security ────────────────────────────────────────────────

/// Moteur de charge doublé.
struct MoteurDouble {
    disponible: bool,
    echoue_a: Option<Palier>,
}

impl MoteurCharge for MoteurDouble {
    fn jouer(
        &self,
        _cible: &str,
        reglage: &ReglagePalier,
    ) -> Result<Vec<MesureCapacite>, AppError> {
        if self.echoue_a == Some(reglage.palier) {
            return Err(AppError::ServiceTiers {
                service: "wrk".to_string(),
                detail: "cible saturée".to_string(),
            });
        }
        analyser_sortie(SORTIE_WRK, reglage.palier, "conditions")
    }
    fn disponible(&self) -> bool {
        self.disponible
    }
}

#[test]
fn security_une_campagne_est_refusee_a_l_admission_si_le_moteur_est_absent() {
    let campagne = Campagne::new(Arc::new(MoteurDouble {
        disponible: false,
        echoue_a: None,
    }));
    let erreur = campagne.verifier_admission(100, 10_000).unwrap_err();
    assert!(
        erreur.to_string().contains("wrk"),
        "le refus doit avoir lieu avant tout provisionnement : {erreur}"
    );
}

#[test]
fn security_une_campagne_plus_longue_que_la_fenetre_est_refusee_a_l_admission() {
    let campagne = Campagne::new(Arc::new(MoteurDouble {
        disponible: true,
        echoue_a: None,
    }));
    let erreur = campagne.verifier_admission(21_600, 7_200).unwrap_err();
    assert!(
        erreur.to_string().contains("coupée en plein palier"),
        "obtenu : {erreur}"
    );
}

#[test]
fn security_un_palier_en_echec_arrete_la_campagne() {
    // Poursuivre mesurerait un système déjà dégradé, et les chiffres suivants
    // seraient trompeurs.
    let campagne = Campagne::new(Arc::new(MoteurDouble {
        disponible: true,
        echoue_a: Some(Palier::Heavy),
    }));
    let resultat = campagne.jouer("http://cible", &escalier_par_defaut());
    assert_eq!(resultat.interrompue_a, Some(Palier::Heavy));
    assert_eq!(resultat.paliers_joues.len(), 3);
    assert!(!resultat.paliers_joues.contains(&Palier::Soak));
}

// ── Passerelle et mise en ligne ──────────────────────────────

struct PasserelleDouble {
    soumissions: Mutex<Vec<Empreinte>>,
}

impl PasserelleApprobation for PasserelleDouble {
    fn soumettre(&self, plan: &sluis::domain::PlanChangement) -> Result<EtatApprobation, AppError> {
        self.soumissions
            .lock()
            .map_err(|_| AppError::Configuration {
                detail: "verrou".to_string(),
            })?
            .push(plan.empreinte().clone());
        Ok(EtatApprobation::EnAttente {
            run: plan.empreinte().abregee().to_string(),
            url: "https://github.com/…".to_string(),
        })
    }
    fn interroger(&self, _empreinte: &Empreinte) -> Result<EtatApprobation, AppError> {
        Ok(EtatApprobation::Approuvee {
            approbateur: "gilmry".to_string(),
            run: "1".to_string(),
        })
    }
}

struct GatesDouble {
    rouges: Vec<GatePlancher>,
}

impl VerificateurGates for GatesDouble {
    fn etat_plancher(&self, _reference: &str) -> Result<Vec<EtatGate>, AppError> {
        Ok([
            GatePlancher::Secrets,
            GatePlancher::Sbom,
            GatePlancher::ScanConteneur,
            GatePlancher::RetourMigration,
        ]
        .into_iter()
        .map(|gate| EtatGate {
            gate,
            verte: !self.rouges.contains(&gate),
            detail: String::new(),
        })
        .collect())
    }
}

#[test]
fn happy_une_mise_en_ligne_aux_gates_vertes_est_soumise_a_approbation() {
    let passerelle = Arc::new(PasserelleDouble {
        soumissions: Mutex::new(Vec::new()),
    });
    let cas = MiseEnLigne::new(Arc::new(GatesDouble { rouges: vec![] }), passerelle.clone());
    let issue = cas
        .demander("koprogo", Environnement::Production, "v1.2.3")
        .expect("demande");
    match issue {
        IssueMiseEnLigne::Soumise { plan, .. } => {
            assert_eq!(plan.tier(), sluis::domain::Tier::One);
            assert_eq!(passerelle.soumissions.lock().expect("verrou").len(), 1);
        }
        autre => panic!("attendu soumise, obtenu {autre:?}"),
    }
}

#[test]
fn security_une_gate_rouge_bloque_avant_meme_la_soumission() {
    // C'est l'ordre qui compte : une gate rouge ne doit pas être soumise au
    // jugement d'un relecteur qui devrait penser à la vérifier.
    let passerelle = Arc::new(PasserelleDouble {
        soumissions: Mutex::new(Vec::new()),
    });
    let cas = MiseEnLigne::new(
        Arc::new(GatesDouble {
            rouges: vec![GatePlancher::Secrets],
        }),
        passerelle.clone(),
    );
    let issue = cas
        .demander("koprogo", Environnement::Production, "v1.2.3")
        .expect("demande");
    match issue {
        IssueMiseEnLigne::RefuseeParLesGates { gates_rouges } => {
            assert_eq!(gates_rouges, vec![GatePlancher::Secrets]);
            assert!(
                passerelle.soumissions.lock().expect("verrou").is_empty(),
                "rien ne doit être soumis quand une gate du plancher est rouge"
            );
        }
        autre => panic!("attendu refusée, obtenu {autre:?}"),
    }
}

#[test]
fn security_l_absence_de_fichier_de_retour_de_migration_bloque_aussi() {
    let cas = MiseEnLigne::new(
        Arc::new(GatesDouble {
            rouges: vec![GatePlancher::RetourMigration],
        }),
        Arc::new(PasserelleDouble {
            soumissions: Mutex::new(Vec::new()),
        }),
    );
    let issue = cas
        .demander("koprogo", Environnement::Production, "v1")
        .expect("demande");
    assert!(matches!(issue, IssueMiseEnLigne::RefuseeParLesGates { .. }));
}
