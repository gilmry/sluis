//! Story 5.3 — la campagne de bout en bout.
//!
//! Le fil conducteur du fichier est la destruction : elle doit avoir lieu sur
//! **tous** les chemins de sortie, y compris celui que personne ne teste
//! d'habitude, l'échec du provisionnement. Un apply qui échoue à mi-course
//! laisse des ressources créées, et c'est le seul cas du lot qui continue à
//! facturer tant que personne ne regarde.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use sluis::application::campagne::{escalier_par_defaut, Campagne};
use sluis::application::ports::{DestructeurBail, MoteurCharge, Provisionneur, ReglagePalier};
use sluis::domain::{
    Action, AppError, BailBacASable, CibleEphemere, Duree, Environnement, FenetreDerogation,
    Horodatage, JetonChangement, JetonConsomme, ListeAutorisation, MesureCapacite, Palier,
    PlafondDepense, PlanChangement, Tier,
};

const MAINTENANT: Horodatage = Horodatage::new(1_000_000);

fn approbation() -> JetonConsomme {
    let plan = PlanChangement::new(
        Action::RenouvellementDerogation,
        Environnement::Production,
        Tier::One,
        "derogation-bac-a-sable".to_string(),
        "ouvre une fenêtre de 90 jours".to_string(),
        String::new(),
    )
    .expect("plan");
    JetonChangement::emettre(
        plan.empreinte().clone(),
        "Gilles Maury".to_string(),
        MAINTENANT,
        Duree::secondes(600).expect("durée"),
    )
    .expect("émission")
    .consommer(&plan, MAINTENANT)
    .expect("consommation")
}

fn bail() -> BailBacASable {
    let fenetre =
        FenetreDerogation::ouvrir(&approbation(), MAINTENANT, Duree::jours(90).expect("durée"))
            .expect("fenêtre");
    let derogation = fenetre.valider(MAINTENANT).expect("dérogation");
    let liste = ListeAutorisation::new(Vec::new(), vec!["bac-koprogo".to_string()]).expect("liste");
    BailBacASable::louer(
        &derogation,
        liste.projet_bac_a_sable("bac-koprogo").expect("projet"),
        Duree::secondes(3_600).expect("ttl"),
        PlafondDepense::new(20.0).expect("plafond"),
        4.0,
        Duree::secondes(21_600).expect("max"),
        MAINTENANT,
    )
    .expect("bail")
}

/// Journal partagé : qui a été appelé, dans quel ordre.
#[derive(Default)]
struct Journal(Mutex<Vec<String>>);

impl Journal {
    fn consigner(&self, quoi: &str) {
        self.0.lock().expect("verrou").push(quoi.to_string());
    }
    fn lignes(&self) -> Vec<String> {
        self.0.lock().expect("verrou").clone()
    }
}

struct ProvisionneurDouble {
    journal: Arc<Journal>,
    echoue: bool,
}

impl Provisionneur for ProvisionneurDouble {
    fn provisionner(&self, _bail: &BailBacASable) -> Result<CibleEphemere, AppError> {
        self.journal.consigner("provisionner");
        if self.echoue {
            return Err(AppError::ServiceTiers {
                service: "terraform".to_string(),
                detail: "quota exceeded".to_string(),
            });
        }
        CibleEphemere::new(
            "57.128.0.1",
            vec![("vps_ip".to_string(), "57.128.0.1".to_string())],
        )
    }
}

struct DestructeurDouble {
    journal: Arc<Journal>,
    appels: AtomicUsize,
    echoue: bool,
}

impl DestructeurBail for DestructeurDouble {
    fn detruire(&self, _bail: &BailBacASable) -> Result<(), AppError> {
        self.journal.consigner("detruire");
        self.appels.fetch_add(1, Ordering::SeqCst);
        if self.echoue {
            return Err(AppError::ServiceTiers {
                service: "terraform".to_string(),
                detail: "instance still locked".to_string(),
            });
        }
        Ok(())
    }
}

struct MoteurDouble {
    journal: Arc<Journal>,
    disponible: bool,
    echoue_a: Option<Palier>,
}

impl MoteurCharge for MoteurDouble {
    fn jouer(&self, cible: &str, reglage: &ReglagePalier) -> Result<Vec<MesureCapacite>, AppError> {
        self.journal
            .consigner(&format!("palier:{}", reglage.palier.nom()));
        if self.echoue_a == Some(reglage.palier) {
            return Err(AppError::ServiceTiers {
                service: "wrk".to_string(),
                detail: "connexion refusée".to_string(),
            });
        }
        Ok(vec![MesureCapacite::mesuree(
            "requetes_par_seconde".to_string(),
            120.0,
            "rps".to_string(),
            reglage.palier,
            1_000,
            format!("cible {cible}"),
        )
        .expect("mesure")])
    }

    fn disponible(&self) -> bool {
        self.disponible
    }
}

fn escalier_court() -> Vec<ReglagePalier> {
    escalier_par_defaut().into_iter().take(2).collect()
}

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_une_campagne_provisionne_joue_puis_detruit() {
    let journal = Arc::new(Journal::default());
    let campagne = Campagne::new(Arc::new(MoteurDouble {
        journal: journal.clone(),
        disponible: true,
        echoue_a: None,
    }));
    let provisionneur = ProvisionneurDouble {
        journal: journal.clone(),
        echoue: false,
    };
    let destructeur = DestructeurDouble {
        journal: journal.clone(),
        appels: AtomicUsize::new(0),
        echoue: false,
    };

    let resultat = campagne
        .conduire(
            &bail(),
            &provisionneur,
            &destructeur,
            &escalier_court(),
            100_000,
        )
        .expect("campagne");

    assert_eq!(resultat.paliers_joues.len(), 2);
    assert!(!resultat.mesures.is_empty());
    assert_eq!(
        journal.lignes(),
        vec!["provisionner", "palier:warmup", "palier:light", "detruire"]
    );
}

// ── @negative ────────────────────────────────────────────────

#[test]
fn negative_un_moteur_absent_refuse_avant_de_provisionner_quoi_que_ce_soit() {
    let journal = Arc::new(Journal::default());
    let campagne = Campagne::new(Arc::new(MoteurDouble {
        journal: journal.clone(),
        disponible: false,
        echoue_a: None,
    }));
    let provisionneur = ProvisionneurDouble {
        journal: journal.clone(),
        echoue: false,
    };
    let destructeur = DestructeurDouble {
        journal: journal.clone(),
        appels: AtomicUsize::new(0),
        echoue: false,
    };

    let erreur = campagne
        .conduire(
            &bail(),
            &provisionneur,
            &destructeur,
            &escalier_court(),
            100_000,
        )
        .expect_err("doit être refusée");

    assert!(erreur.to_string().contains("wrk"));
    // Rien n'a été créé, donc rien n'est à détruire : le journal est vide.
    assert!(journal.lignes().is_empty());
}

#[test]
fn negative_une_fenetre_trop_courte_refuse_avant_de_provisionner() {
    let journal = Arc::new(Journal::default());
    let campagne = Campagne::new(Arc::new(MoteurDouble {
        journal: journal.clone(),
        disponible: true,
        echoue_a: None,
    }));
    let provisionneur = ProvisionneurDouble {
        journal: journal.clone(),
        echoue: false,
    };
    let destructeur = DestructeurDouble {
        journal: journal.clone(),
        appels: AtomicUsize::new(0),
        echoue: false,
    };

    let erreur = campagne
        .conduire(&bail(), &provisionneur, &destructeur, &escalier_court(), 5)
        .expect_err("doit être refusée");

    assert!(erreur.to_string().contains("dérogation"));
    assert!(journal.lignes().is_empty());
}

// ── @edge ────────────────────────────────────────────────────

#[test]
fn edge_un_palier_en_echec_arrete_la_campagne_sans_empecher_la_destruction() {
    let journal = Arc::new(Journal::default());
    let campagne = Campagne::new(Arc::new(MoteurDouble {
        journal: journal.clone(),
        disponible: true,
        echoue_a: Some(Palier::Light),
    }));
    let provisionneur = ProvisionneurDouble {
        journal: journal.clone(),
        echoue: false,
    };
    let destructeur = DestructeurDouble {
        journal: journal.clone(),
        appels: AtomicUsize::new(0),
        echoue: false,
    };

    let resultat = campagne
        .conduire(
            &bail(),
            &provisionneur,
            &destructeur,
            &escalier_court(),
            100_000,
        )
        .expect("la campagne rend son résultat partiel");

    assert_eq!(resultat.interrompue_a, Some(Palier::Light));
    assert!(journal.lignes().contains(&"detruire".to_string()));
}

#[test]
fn edge_une_destruction_en_echec_ne_fait_pas_perdre_les_mesures() {
    let journal = Arc::new(Journal::default());
    let campagne = Campagne::new(Arc::new(MoteurDouble {
        journal: journal.clone(),
        disponible: true,
        echoue_a: None,
    }));
    let provisionneur = ProvisionneurDouble {
        journal: journal.clone(),
        echoue: false,
    };
    let destructeur = DestructeurDouble {
        journal: journal.clone(),
        appels: AtomicUsize::new(0),
        echoue: true,
    };

    let resultat = campagne
        .conduire(
            &bail(),
            &provisionneur,
            &destructeur,
            &escalier_court(),
            100_000,
        )
        .expect("les mesures survivent à l'échec de destruction");

    assert!(!resultat.mesures.is_empty());
    assert!(
        resultat
            .echec_destruction
            .as_deref()
            .unwrap_or_default()
            .contains("locked"),
        "l'échec doit être porté par le résultat, pas tu : {:?}",
        resultat.echec_destruction
    );
}

// ── @security ────────────────────────────────────────────────

#[test]
fn security_un_provisionnement_en_echec_declenche_quand_meme_la_destruction() {
    let journal = Arc::new(Journal::default());
    let campagne = Campagne::new(Arc::new(MoteurDouble {
        journal: journal.clone(),
        disponible: true,
        echoue_a: None,
    }));
    let provisionneur = ProvisionneurDouble {
        journal: journal.clone(),
        echoue: true,
    };
    let destructeur = DestructeurDouble {
        journal: journal.clone(),
        appels: AtomicUsize::new(0),
        echoue: false,
    };

    let erreur = campagne
        .conduire(
            &bail(),
            &provisionneur,
            &destructeur,
            &escalier_court(),
            100_000,
        )
        .expect_err("le provisionnement échoue");

    assert!(erreur.to_string().contains("quota"));
    // Le cas qui coûte de l'argent : un apply interrompu a pu créer des
    // ressources avant d'échouer. Ne pas détruire ici, c'est facturer.
    assert_eq!(
        journal.lignes(),
        vec!["provisionner", "detruire"],
        "la destruction doit suivre un provisionnement raté"
    );
    assert_eq!(destructeur.appels.load(Ordering::SeqCst), 1);
}

#[test]
fn security_aucun_palier_n_est_joue_apres_la_destruction() {
    let journal = Arc::new(Journal::default());
    let campagne = Campagne::new(Arc::new(MoteurDouble {
        journal: journal.clone(),
        disponible: true,
        echoue_a: None,
    }));
    let provisionneur = ProvisionneurDouble {
        journal: journal.clone(),
        echoue: false,
    };
    let destructeur = DestructeurDouble {
        journal: journal.clone(),
        appels: AtomicUsize::new(0),
        echoue: false,
    };

    let _ = campagne.conduire(
        &bail(),
        &provisionneur,
        &destructeur,
        &escalier_court(),
        100_000,
    );

    let lignes = journal.lignes();
    let position_destruction = lignes
        .iter()
        .position(|l| l == "detruire")
        .expect("destruction");
    assert!(
        lignes[position_destruction..]
            .iter()
            .all(|l| !l.starts_with("palier:")),
        "charger une cible déjà détruite mesurerait le vide : {lignes:?}"
    );
}
