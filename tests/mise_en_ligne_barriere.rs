//! Story 7.2 (sluis#96) — la barrière s'évalue après le plancher et avant
//! toute soumission ; vide, elle n'est admise que datée et mentionnée
//! (ADR-021, décision du superviseur du 2026-10-29).

use std::sync::{Arc, Mutex};

use sluis::application::mise_en_ligne::{Barriere, IssueMiseEnLigne, MiseEnLigne};
use sluis::application::ports::{
    EtatApprobation, EtatGate, EvaluateurBarriere, GatePlancher, HorlogeFigee,
    PasserelleApprobation, VerificateurGates,
};
use sluis::domain::{
    AppError, ControleBarriere, Empreinte, Environnement, ExceptionBarriereVide, Horodatage,
    PlanChangement, VerdictControle,
};

/// 2027-01-01T00:00:00Z.
const NOUVEL_AN_2027: i64 = 1_798_761_600;

struct Gates {
    verte: bool,
}

impl VerificateurGates for Gates {
    fn etat_plancher(&self, _reference: &str) -> Result<Vec<EtatGate>, AppError> {
        Ok(vec![EtatGate {
            gate: GatePlancher::Secrets,
            verte: self.verte,
            detail: String::new(),
        }])
    }
}

#[derive(Default)]
struct Passerelle {
    plans: Mutex<Vec<PlanChangement>>,
}

impl PasserelleApprobation for Passerelle {
    fn soumettre(&self, plan: &PlanChangement) -> Result<EtatApprobation, AppError> {
        self.plans.lock().expect("verrou").push(plan.clone());
        Ok(EtatApprobation::EnAttente {
            run: "1".to_string(),
            url: String::new(),
            commande: None,
        })
    }
    fn interroger(&self, _empreinte: &Empreinte) -> Result<EtatApprobation, AppError> {
        unreachable!()
    }
}

/// Évaluateur doublé : rend vert sauf pour les contrôles nommés, et compte
/// ses appels.
#[derive(Default)]
struct Evaluateur {
    rouges: Vec<&'static str>,
    appels: Mutex<u32>,
}

impl EvaluateurBarriere for Evaluateur {
    fn evaluer(&self, controles: &[ControleBarriere]) -> Vec<VerdictControle> {
        *self.appels.lock().expect("verrou") += 1;
        controles
            .iter()
            .map(|c| {
                let rouge = self.rouges.contains(&c.nom());
                VerdictControle {
                    nom: c.nom().to_string(),
                    vert: !rouge,
                    motif: if rouge { "code 1" } else { "code 0" }.to_string(),
                    rapport: String::new(),
                }
            })
            .collect()
    }
}

fn controle(nom: &str) -> ControleBarriere {
    ControleBarriere::new(
        nom.to_string(),
        format!("/opt/controles/{nom}"),
        Vec::new(),
        None,
    )
    .expect("contrôle")
}

struct Cas {
    mise_en_ligne: MiseEnLigne,
    passerelle: Arc<Passerelle>,
    evaluateur: Arc<Evaluateur>,
}

fn cas(
    plancher_vert: bool,
    controles: Vec<ControleBarriere>,
    rouges: Vec<&'static str>,
    exception: Option<&str>,
    maintenant: i64,
) -> Cas {
    let passerelle = Arc::new(Passerelle::default());
    let evaluateur = Arc::new(Evaluateur {
        rouges,
        appels: Mutex::new(0),
    });
    let mise_en_ligne = MiseEnLigne::new(
        Arc::new(Gates {
            verte: plancher_vert,
        }),
        passerelle.clone(),
        Barriere {
            evaluateur: evaluateur.clone(),
            controles,
            exception_vide: exception.map(|d| ExceptionBarriereVide::depuis_date(d).expect("date")),
            horloge: Arc::new(HorlogeFigee::a(Horodatage::new(maintenant))),
        },
        journal_muet(),
    );
    Cas {
        mise_en_ligne,
        passerelle,
        evaluateur,
    }
}

fn demander(cas: &Cas) -> IssueMiseEnLigne {
    cas.mise_en_ligne
        .demander("koprogo", Environnement::Production, "v1.0.0")
        .expect("demande")
}

fn soumissions(cas: &Cas) -> usize {
    cas.passerelle.plans.lock().expect("verrou").len()
}

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_une_barriere_verte_figure_dans_le_plan_soumis() {
    let cas = cas(
        true,
        vec![controle("supervision"), controle("sauvegarde")],
        vec![],
        None,
        0,
    );
    let IssueMiseEnLigne::Soumise { plan, .. } = demander(&cas) else {
        panic!("attendu soumise");
    };
    let detail = plan.diff();
    assert!(
        detail.contains("barrière supervision : vert (code 0)"),
        "{detail}"
    );
    assert!(
        detail.contains("barrière sauvegarde : vert (code 0)"),
        "{detail}"
    );
    assert!(plan
        .description()
        .contains("2 contrôle(s) de la barrière au vert"));
}

// ── @negative ────────────────────────────────────────────────

#[test]
fn negative_un_controle_rouge_refuse_avant_toute_soumission() {
    let cas = cas(
        true,
        vec![controle("supervision"), controle("sauvegarde")],
        vec!["sauvegarde"],
        Some("2999-12-31"),
        0,
    );
    let IssueMiseEnLigne::RefuseeParLaBarriere { controles_rouges } = demander(&cas) else {
        panic!("attendu refusée par la barrière");
    };
    assert_eq!(controles_rouges.len(), 1);
    assert_eq!(controles_rouges[0].nom, "sauvegarde");
    assert_eq!(controles_rouges[0].motif, "code 1");
    assert_eq!(soumissions(&cas), 0, "la passerelle ne doit rien recevoir");
}

#[test]
fn negative_un_plancher_rouge_ne_lance_aucun_controle() {
    let cas = cas(false, vec![controle("supervision")], vec![], None, 0);
    assert!(matches!(
        demander(&cas),
        IssueMiseEnLigne::RefuseeParLesGates { .. }
    ));
    assert_eq!(*cas.evaluateur.appels.lock().expect("verrou"), 0);
}

#[test]
fn negative_une_exception_mal_ecrite_est_refusee() {
    for date in [
        "2026-02-29",
        "2026-13-01",
        "2026-12-32",
        "2026-1-05",
        "31/12/2026",
        "2026-12-31T00:00",
        "",
    ] {
        assert!(
            ExceptionBarriereVide::depuis_date(date).is_err(),
            "« {date} » aurait dû être refusée"
        );
    }
    assert!(ExceptionBarriereVide::depuis_date("2028-02-29").is_ok());
}

// ── @edge ────────────────────────────────────────────────────

#[test]
fn edge_une_barriere_vide_est_admise_jusqu_au_dernier_instant_du_jour_declare() {
    let veille = cas(true, vec![], vec![], Some("2026-12-31"), NOUVEL_AN_2027 - 1);
    let IssueMiseEnLigne::Soumise { plan, .. } = demander(&veille) else {
        panic!("le 2026-12-31 à 23h59:59 UTC, l'exception vaut encore");
    };
    assert!(
        plan.description().contains("BARRIÈRE VIDE"),
        "{}",
        plan.description()
    );
    assert!(plan
        .diff()
        .contains("aucun contrôle de plateforme n'a tourné"));

    let lendemain = cas(true, vec![], vec![], Some("2026-12-31"), NOUVEL_AN_2027);
    let IssueMiseEnLigne::RefuseeBarriereVide { motif } = demander(&lendemain) else {
        panic!("le 2027-01-01 à minuit UTC, l'exception a expiré");
    };
    assert!(motif.contains("2026-12-31"), "{motif}");
    assert_eq!(soumissions(&lendemain), 0);
}

#[test]
fn edge_les_dates_tombent_sur_minuit_utc() {
    let fin = |date: &str, avant: i64| {
        let e = ExceptionBarriereVide::depuis_date(date).expect("date");
        (
            e.admet(Horodatage::new(avant - 1)),
            e.admet(Horodatage::new(avant)),
        )
    };
    assert_eq!(fin("1970-01-01", 86_400), (true, false));
    assert_eq!(fin("2000-02-29", 951_868_800), (true, false));
    assert_eq!(fin("2028-02-29", 1_835_481_600), (true, false));
}

// ── @security ────────────────────────────────────────────────

#[test]
fn security_une_barriere_vide_sans_exception_refuse() {
    let cas = cas(true, vec![], vec![], None, 0);
    let IssueMiseEnLigne::RefuseeBarriereVide { motif } = demander(&cas) else {
        panic!("sans exception datée, une barrière vide doit refuser");
    };
    assert!(motif.contains("sans exception datée"), "{motif}");
    assert_eq!(soumissions(&cas), 0);
}

#[test]
fn security_une_exception_n_ouvre_pas_une_barriere_declaree_et_rouge() {
    // L'exception ne vaut que pour une barrière vide : elle ne rattrape pas
    // un contrôle rouge.
    let cas = cas(
        true,
        vec![controle("supervision")],
        vec!["supervision"],
        Some("2999-12-31"),
        0,
    );
    assert!(matches!(
        demander(&cas),
        IssueMiseEnLigne::RefuseeParLaBarriere { .. }
    ));
}

#[test]
fn security_un_verdict_perdu_par_l_evaluateur_compte_rouge() {
    struct Muet;
    impl EvaluateurBarriere for Muet {
        fn evaluer(&self, _controles: &[ControleBarriere]) -> Vec<VerdictControle> {
            Vec::new()
        }
    }
    let passerelle = Arc::new(Passerelle::default());
    let issue = MiseEnLigne::new(
        Arc::new(Gates { verte: true }),
        passerelle.clone(),
        Barriere {
            evaluateur: Arc::new(Muet),
            controles: vec![controle("supervision")],
            exception_vide: None,
            horloge: Arc::new(HorlogeFigee::a(Horodatage::new(0))),
        },
        journal_muet(),
    )
    .demander("koprogo", Environnement::Production, "v1.0.0")
    .expect("demande");
    assert!(matches!(
        issue,
        IssueMiseEnLigne::RefuseeParLaBarriere { .. }
    ));
    assert!(passerelle.plans.lock().expect("verrou").is_empty());
}

/// Journal qui accepte tout : ces tests ne portent pas sur le rapport de
/// preuve (voir `tests/rapport_preuve.rs`).
struct JournalMuet;

impl sluis::application::ports::AuditLog for JournalMuet {
    fn append(&self, _entree: &sluis::domain::AuditEntry) -> Result<(), sluis::domain::AppError> {
        Ok(())
    }
}

fn journal_muet() -> std::sync::Arc<dyn sluis::application::ports::AuditLog> {
    std::sync::Arc::new(JournalMuet)
}
