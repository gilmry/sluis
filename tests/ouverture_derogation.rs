//! Story 4.4b (#4) — l'exécutant ouvre la fenêtre d'ADR-007 sur preuve.
//!
//! Le test qui porte le fichier est
//! `security_une_approbation_ne_rouvre_la_fenetre_qu_une_fois` : sans lui, une
//! seule approbation humaine prolongerait la dérogation indéfiniment.

use std::sync::{Arc, Mutex};

use sluis::application::ouverture_derogation::{OuvertureDerogation, FRAICHEUR_MAXIMALE_SECONDES};
use sluis::application::ports::{
    DepotDerogation, EtatApprobation, HorlogeFigee, PasserelleApprobation, RegistreRenouvellements,
};
use sluis::domain::{
    plan_renouvellement, AppError, Duree, Empreinte, FenetreDerogation, Horodatage, PlanChangement,
};
use sluis::infrastructure::renouvellements_depot::RegistreRenouvellementsFichier;

const MAINTENANT: i64 = 1_790_000_000;

struct PasserelleDouble {
    etat: EtatApprobation,
    interroges: Mutex<Vec<Empreinte>>,
}

impl PasserelleDouble {
    fn rendant(etat: EtatApprobation) -> Arc<Self> {
        Arc::new(Self {
            etat,
            interroges: Mutex::new(Vec::new()),
        })
    }
    fn approuvee() -> Arc<Self> {
        Self::rendant(EtatApprobation::Approuvee {
            approbateur: "relecteur-a".to_string(),
            run: "101".to_string(),
        })
    }
}

impl PasserelleApprobation for PasserelleDouble {
    fn soumettre(&self, _plan: &PlanChangement) -> Result<EtatApprobation, AppError> {
        unreachable!("l'exécutant ne soumet jamais")
    }
    fn interroger(&self, empreinte: &Empreinte) -> Result<EtatApprobation, AppError> {
        self.interroges
            .lock()
            .expect("verrou")
            .push(empreinte.clone());
        Ok(self.etat.clone())
    }
}

#[derive(Default)]
struct RegistreMemoire(Mutex<Vec<Empreinte>>);

impl RegistreRenouvellements for RegistreMemoire {
    fn deja_consommee(&self, empreinte: &Empreinte) -> Result<bool, AppError> {
        Ok(self.0.lock().expect("verrou").contains(empreinte))
    }
    fn consigner(&self, empreinte: &Empreinte) -> Result<(), AppError> {
        self.0.lock().expect("verrou").push(empreinte.clone());
        Ok(())
    }
}

#[derive(Default)]
struct DepotMemoire {
    fenetres: Mutex<Vec<FenetreDerogation>>,
    en_panne: bool,
}

impl DepotDerogation for DepotMemoire {
    fn courante(&self) -> Result<Option<FenetreDerogation>, AppError> {
        Ok(self.fenetres.lock().expect("verrou").last().cloned())
    }
    fn enregistrer(&self, fenetre: &FenetreDerogation) -> Result<(), AppError> {
        if self.en_panne {
            return Err(AppError::EntreeSortie {
                chemin: "fenêtre".to_string(),
                detail: "disque plein".to_string(),
            });
        }
        self.fenetres.lock().expect("verrou").push(fenetre.clone());
        Ok(())
    }
}

struct Banc {
    passerelle: Arc<PasserelleDouble>,
    registre: Arc<RegistreMemoire>,
    depot: Arc<DepotMemoire>,
    cas: OuvertureDerogation,
}

fn banc(passerelle: Arc<PasserelleDouble>, depot: DepotMemoire) -> Banc {
    let registre = Arc::new(RegistreMemoire::default());
    let depot = Arc::new(depot);
    let cas = OuvertureDerogation::new(
        passerelle.clone(),
        registre.clone(),
        depot.clone(),
        Arc::new(HorlogeFigee::a(Horodatage::new(MAINTENANT))),
        Duree::jours(90).expect("durée"),
    );
    Banc {
        passerelle,
        registre,
        depot,
        cas,
    }
}

fn il_y_a(secondes: i64) -> Horodatage {
    Horodatage::new(MAINTENANT - secondes)
}

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_une_demande_approuvee_ouvre_la_fenetre_pour_quatre_vingt_dix_jours() {
    let b = banc(PasserelleDouble::approuvee(), DepotMemoire::default());
    let demande = il_y_a(3_600);
    let fenetre = b.cas.ouvrir(demande).expect("ouverture");

    assert_eq!(fenetre.approbateur(), "relecteur-a");
    assert_eq!(fenetre.ouverte_le(), Horodatage::new(MAINTENANT));
    assert_eq!(
        fenetre.close_le(),
        Horodatage::new(MAINTENANT + 90 * 86_400)
    );
    assert_eq!(b.depot.fenetres.lock().expect("verrou").len(), 1);

    // L'empreinte interrogée est celle que l'exécutant a reconstruite.
    let attendue = plan_renouvellement(demande).expect("plan");
    assert_eq!(
        b.passerelle.interroges.lock().expect("verrou")[0],
        *attendue.empreinte()
    );
    assert!(b
        .registre
        .0
        .lock()
        .expect("verrou")
        .contains(attendue.empreinte()));
}

// ── @negative ────────────────────────────────────────────────

#[test]
fn negative_une_demande_en_attente_ou_refusee_n_ouvre_rien() {
    for etat in [
        EtatApprobation::EnAttente {
            run: "1".to_string(),
            url: String::new(),
            commande: None,
        },
        EtatApprobation::Refusee {
            motif: "approbation rejetée par relecteur-a".to_string(),
        },
        EtatApprobation::Echouee {
            detail: "exécuté sans approbation".to_string(),
        },
    ] {
        let b = banc(
            PasserelleDouble::rendant(etat.clone()),
            DepotMemoire::default(),
        );
        assert!(b.cas.ouvrir(il_y_a(60)).is_err(), "{etat:?}");
        assert!(
            b.depot.fenetres.lock().expect("verrou").is_empty(),
            "{etat:?}"
        );
        assert!(b.registre.0.lock().expect("verrou").is_empty(), "{etat:?}");
    }
}

// ── @edge ────────────────────────────────────────────────────

#[test]
fn edge_une_demande_trop_ancienne_ou_future_est_refusee_sans_interroger_github() {
    for demande in [
        il_y_a(FRAICHEUR_MAXIMALE_SECONDES + 1),
        Horodatage::new(MAINTENANT + 1),
    ] {
        let b = banc(PasserelleDouble::approuvee(), DepotMemoire::default());
        assert!(b.cas.ouvrir(demande).is_err(), "{demande}");
        assert!(
            b.passerelle.interroges.lock().expect("verrou").is_empty(),
            "{demande}"
        );
    }
}

#[test]
fn edge_une_demande_d_exactement_soixante_douze_heures_est_encore_valable() {
    let b = banc(PasserelleDouble::approuvee(), DepotMemoire::default());
    assert!(b.cas.ouvrir(il_y_a(FRAICHEUR_MAXIMALE_SECONDES)).is_ok());
}

// ── @security ────────────────────────────────────────────────

#[test]
fn security_une_approbation_ne_rouvre_la_fenetre_qu_une_fois() {
    let b = banc(PasserelleDouble::approuvee(), DepotMemoire::default());
    let demande = il_y_a(600);
    b.cas.ouvrir(demande).expect("première ouverture");

    let erreur = b
        .cas
        .ouvrir(demande)
        .expect_err("le rejeu doit être refusé");
    assert!(erreur.to_string().contains("qu'une fois"), "{erreur}");
    assert_eq!(b.depot.fenetres.lock().expect("verrou").len(), 1);
}

#[test]
fn security_l_empreinte_est_consignee_avant_l_ouverture() {
    // Si l'écriture de la fenêtre échoue, le renouvellement est perdu, mais
    // l'approbation ne doit pas pouvoir resservir.
    let b = banc(
        PasserelleDouble::approuvee(),
        DepotMemoire {
            en_panne: true,
            ..DepotMemoire::default()
        },
    );
    let demande = il_y_a(600);
    assert!(b.cas.ouvrir(demande).is_err());
    assert!(b
        .registre
        .0
        .lock()
        .expect("verrou")
        .contains(plan_renouvellement(demande).expect("plan").empreinte()));
}

// ── Registre sur fichier ─────────────────────────────────────

fn chemin(nom: &str) -> std::path::PathBuf {
    let chemin = std::env::temp_dir().join(format!("sluis-renouvellements-{nom}.json"));
    let _ = std::fs::remove_file(&chemin);
    chemin
}

#[test]
fn happy_le_registre_fichier_se_souvient_d_un_redemarrage_a_l_autre() {
    let chemin = chemin("persistance");
    let empreinte = Empreinte::calculer("renouvellement");
    RegistreRenouvellementsFichier::nouveau(&chemin)
        .consigner(&empreinte)
        .expect("consignation");

    let relu = RegistreRenouvellementsFichier::nouveau(&chemin);
    assert!(relu.deja_consommee(&empreinte).expect("lecture"));
    assert!(!relu
        .deja_consommee(&Empreinte::calculer("autre"))
        .expect("lecture"));
    let _ = std::fs::remove_file(&chemin);
}

#[test]
fn security_un_registre_illisible_est_une_erreur_pas_une_liste_vide() {
    let chemin = chemin("corrompu");
    std::fs::write(&chemin, "pas du json").expect("écriture");
    let resultat =
        RegistreRenouvellementsFichier::nouveau(&chemin).deja_consommee(&Empreinte::calculer("x"));
    assert!(
        resultat.is_err(),
        "un registre corrompu lu comme vide rouvrirait la porte"
    );
    let _ = std::fs::remove_file(&chemin);
}
