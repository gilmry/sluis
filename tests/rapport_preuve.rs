//! Story 7.3 (sluis#98) — chaque demande de mise en ligne laisse un rapport
//! de preuve daté au journal, avant toute soumission (ADR-022).

use std::sync::{Arc, Mutex};

use serde_json::Value;
use sluis::application::mise_en_ligne::{Barriere, IssueMiseEnLigne, MiseEnLigne, OUTIL_PREUVE};
use sluis::application::ports::{
    AuditLog, EtatApprobation, EtatGate, EvaluateurBarriere, GatePlancher, HorlogeFigee,
    PasserelleApprobation, VerificateurGates,
};
use sluis::domain::{
    verifier_identifiant_cyfun, AppError, AuditEntry, ControleBarriere, Empreinte, Environnement,
    ExceptionBarriereVide, Horodatage, PlanChangement, VerdictControle,
};

/// 2027-01-01T00:00:00Z.
const NOUVEL_AN_2027: i64 = 1_798_761_600;

/// Ordre des événements, partagé par le journal et la passerelle.
type Fil = Arc<Mutex<Vec<String>>>;

struct Journal {
    fil: Fil,
    entrees: Mutex<Vec<Value>>,
    refuse: bool,
}

impl AuditLog for Journal {
    fn append(&self, entree: &AuditEntry) -> Result<(), AppError> {
        if self.refuse {
            return Err(AppError::EntreeSortie {
                chemin: "journal".to_string(),
                detail: "disque plein".to_string(),
            });
        }
        self.fil.lock().expect("verrou").push("preuve".to_string());
        self.entrees
            .lock()
            .expect("verrou")
            .push(serde_json::to_value(entree).expect("json"));
        Ok(())
    }
}

struct Passerelle {
    fil: Fil,
}

impl PasserelleApprobation for Passerelle {
    fn soumettre(&self, _plan: &PlanChangement) -> Result<EtatApprobation, AppError> {
        self.fil
            .lock()
            .expect("verrou")
            .push("soumission".to_string());
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

struct Gates {
    verte: bool,
}

impl VerificateurGates for Gates {
    fn etat_plancher(&self, _reference: &str) -> Result<Vec<EtatGate>, AppError> {
        Ok(vec![
            EtatGate {
                gate: GatePlancher::Secrets,
                verte: self.verte,
                detail: if self.verte { "" } else { "fuite" }.to_string(),
            },
            EtatGate {
                gate: GatePlancher::Sbom,
                verte: true,
                detail: String::new(),
            },
        ])
    }
}

struct Evaluateur {
    vert: bool,
}

impl EvaluateurBarriere for Evaluateur {
    fn evaluer(&self, controles: &[ControleBarriere]) -> Vec<VerdictControle> {
        controles
            .iter()
            .map(|c| VerdictControle {
                nom: c.nom().to_string(),
                vert: self.vert,
                motif: if self.vert { "code 0" } else { "code 2" }.to_string(),
                rapport: "dernière collecte il y a 3 s".to_string(),
            })
            .collect()
    }
}

struct Scenario {
    plancher_vert: bool,
    controles_verts: Option<bool>,
    exception: Option<&'static str>,
    maintenant: i64,
    journal_refuse: bool,
}

impl Default for Scenario {
    fn default() -> Self {
        Self {
            plancher_vert: true,
            controles_verts: Some(true),
            exception: None,
            maintenant: 1_000,
            journal_refuse: false,
        }
    }
}

struct Resultat {
    issue: Result<IssueMiseEnLigne, AppError>,
    preuves: Vec<Value>,
    fil: Vec<String>,
}

fn jouer(scenario: Scenario) -> Resultat {
    let fil: Fil = Arc::new(Mutex::new(Vec::new()));
    let journal = Arc::new(Journal {
        fil: fil.clone(),
        entrees: Mutex::new(Vec::new()),
        refuse: scenario.journal_refuse,
    });
    let controles = match scenario.controles_verts {
        None => Vec::new(),
        Some(_) => vec![ControleBarriere::new(
            "supervision".to_string(),
            "/opt/controles/supervision".to_string(),
            Vec::new(),
            None,
        )
        .and_then(|c| c.avec_cyfun(vec!["PR.PS-01.1".to_string()]))
        .expect("contrôle")],
    };
    let issue = MiseEnLigne::new(
        Arc::new(Gates {
            verte: scenario.plancher_vert,
        }),
        Arc::new(Passerelle { fil: fil.clone() }),
        Barriere {
            evaluateur: Arc::new(Evaluateur {
                vert: scenario.controles_verts.unwrap_or(true),
            }),
            controles,
            exception_vide: scenario
                .exception
                .map(|d| ExceptionBarriereVide::depuis_date(d).expect("date")),
            horloge: Arc::new(HorlogeFigee::a(Horodatage::new(scenario.maintenant))),
        },
        journal.clone(),
    )
    .demander("koprogo", Environnement::Production, "v1.4.0");
    let preuves = journal.entrees.lock().expect("verrou").clone();
    let fil = fil.lock().expect("verrou").clone();
    Resultat {
        issue,
        preuves,
        fil,
    }
}

/// Le rapport de l'unique entrée de preuve.
fn preuve(resultat: &Resultat) -> &Value {
    assert_eq!(resultat.preuves.len(), 1, "{:#?}", resultat.preuves);
    assert_eq!(resultat.preuves[0]["outil"], OUTIL_PREUVE);
    &resultat.preuves[0]["preuve"]
}

fn controle<'a>(rapport: &'a Value, nom: &str) -> &'a Value {
    rapport["controles"]
        .as_array()
        .expect("contrôles")
        .iter()
        .find(|c| c["nom"] == nom)
        .unwrap_or_else(|| panic!("contrôle « {nom} » absent : {rapport:#}"))
}

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_une_demande_admise_laisse_un_rapport_rattache() {
    let resultat = jouer(Scenario::default());
    let Ok(IssueMiseEnLigne::Soumise { plan, .. }) = &resultat.issue else {
        panic!("attendu soumise : {:?}", resultat.issue.as_ref().err());
    };
    let rapport = preuve(&resultat);
    assert_eq!(rapport["schema"], "sluis/rapport-preuve/v1");
    assert_eq!(rapport["issue"], "admise");
    assert_eq!(rapport["reference"], "v1.4.0");
    assert_eq!(rapport["environnement"], "production");
    assert_eq!(rapport["horodatage"], 1_000);
    assert_eq!(rapport["version_sluis"], env!("CARGO_PKG_VERSION"));
    assert_eq!(rapport["empreinte_plan"], plan.empreinte().hexadecimal());
    assert_eq!(rapport["cyfun"], serde_json::json!(["PR.PS-06.2"]));
    assert_eq!(
        controle(rapport, "secrets")["cyfun"],
        serde_json::json!(["PR.AA-01.1"])
    );
    let supervision = controle(rapport, "supervision");
    assert_eq!(supervision["source"], "barriere");
    assert_eq!(supervision["cyfun"], serde_json::json!(["PR.PS-01.1"]));
    assert_eq!(supervision["rapport"], "dernière collecte il y a 3 s");
}

// ── @negative ────────────────────────────────────────────────

#[test]
fn negative_un_refus_de_la_barriere_est_aussi_une_preuve() {
    let resultat = jouer(Scenario {
        controles_verts: Some(false),
        ..Scenario::default()
    });
    let rapport = preuve(&resultat);
    assert_eq!(rapport["issue"], "refusee_par_la_barriere");
    assert_eq!(controle(rapport, "supervision")["motif"], "code 2");
    assert!(rapport.get("empreinte_plan").is_none());
    assert_eq!(resultat.fil, ["preuve"], "rien ne doit être soumis");
}

#[test]
fn negative_un_refus_du_plancher_est_aussi_une_preuve_sans_revendiquer_ps_06_2() {
    let resultat = jouer(Scenario {
        plancher_vert: false,
        ..Scenario::default()
    });
    let rapport = preuve(&resultat);
    assert_eq!(rapport["issue"], "refusee_par_les_gates");
    assert_eq!(controle(rapport, "secrets")["motif"], "rouge : fuite");
    assert_eq!(rapport["cyfun"], serde_json::json!([]));
}

#[test]
fn negative_un_identifiant_cyfun_mal_forme_est_refuse() {
    for identifiant in [
        "PR.PS-1.1",
        "pr.ps-01.1",
        "PR-PS.01.1",
        "PR.PS-01",
        "PR.PS-01.1x",
        "",
    ] {
        assert!(
            verifier_identifiant_cyfun(identifiant).is_err(),
            "« {identifiant} » aurait dû être refusé"
        );
    }
    for identifiant in ["PR.PS-01.1", "ID.AM-02.2", "DE.CM-09.1", "PR.AA-05.10"] {
        assert!(
            verifier_identifiant_cyfun(identifiant).is_ok(),
            "{identifiant}"
        );
    }
}

// ── @edge ────────────────────────────────────────────────────

#[test]
fn edge_une_barriere_vide_admise_le_dit_dans_la_preuve() {
    let resultat = jouer(Scenario {
        controles_verts: None,
        exception: Some("2026-12-31"),
        maintenant: NOUVEL_AN_2027 - 1,
        ..Scenario::default()
    });
    let rapport = preuve(&resultat);
    assert_eq!(rapport["issue"], "admise");
    assert_eq!(rapport["barriere_vide_jusqu_au"], "2026-12-31");
}

#[test]
fn edge_une_exception_echue_figure_dans_le_refus() {
    let resultat = jouer(Scenario {
        controles_verts: None,
        exception: Some("2026-12-31"),
        maintenant: NOUVEL_AN_2027,
        ..Scenario::default()
    });
    let rapport = preuve(&resultat);
    assert_eq!(rapport["issue"], "refusee_barriere_vide");
    assert_eq!(rapport["barriere_vide_jusqu_au"], "2026-12-31");
}

// ── @security ────────────────────────────────────────────────

#[test]
fn security_sans_preuve_ecrite_rien_n_est_soumis() {
    let resultat = jouer(Scenario {
        journal_refuse: true,
        ..Scenario::default()
    });
    assert!(resultat.issue.is_err(), "la demande doit échouer");
    assert!(resultat.fil.is_empty(), "{:?}", resultat.fil);
}

#[test]
fn security_la_preuve_est_ecrite_avant_la_soumission() {
    let resultat = jouer(Scenario::default());
    assert_eq!(resultat.fil, ["preuve", "soumission"]);
}

#[test]
fn security_un_controle_declare_avec_un_identifiant_invente_arrete_le_chargement() {
    let chemin = std::env::temp_dir().join(format!("sluis-preuve-{}.toml", std::process::id()));
    std::fs::write(
        &chemin,
        "[[barriere.controles]]\nnom = \"a\"\nprogramme = \"/bin/true\"\ncyfun = [\"PR.PS-1\"]\n",
    )
    .expect("configuration");
    let resultat = sluis::configuration::Configuration::charger_avec(Some(&chemin), |_| None);
    let _ = std::fs::remove_file(&chemin);
    assert!(resultat.is_err());
}
