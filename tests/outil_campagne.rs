//! Story 5.5 — `sluis_campagne`, l'outil qui loue, charge et détruit.
//!
//! L'outil est un adaptateur mince : tout ce qui est refusé ici l'est par le
//! domaine, jamais par une règle réécrite dans le `tools/call`. Les tests le
//! vérifient en observant qu'aucun provisionnement n'a lieu quand un invariant
//! est violé.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::json;

use sluis::application::campagne::Campagne;
use sluis::application::ports::{
    DepotCharge, DepotDerogation, DestructeurBail, Horloge, HorlogeFigee, MoteurCharge,
    Provisionneur, ReglagePalier,
};
use sluis::domain::{
    Action, AppError, BailBacASable, CibleCapacite, CibleEphemere, DeclarationCharge, Duree,
    Environnement, FenetreDerogation, Horodatage, JetonChangement, JetonConsomme,
    ListeAutorisation, MesureCapacite, PlafondDepense, PlanChangement, Tier,
};
use sluis::infrastructure::mcp::outil_campagne::{OutilCampagne, ReglagesCampagne};
use sluis::infrastructure::mcp::Outil;

const MAINTENANT: Horodatage = Horodatage::new(1_000_000);
const DEPOT: &str = "/depots/koprogo/infrastructure";

fn approbation() -> JetonConsomme {
    let plan = PlanChangement::new(
        Action::RenouvellementDerogation,
        Environnement::Production,
        Tier::One,
        "derogation-bac-a-sable".to_string(),
        "ouvre une fenêtre".to_string(),
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

/// Déclaration de charge doublée : ce qu'un `_shared/charge.yaml` fournirait.
struct ChargeDouble;

impl DepotCharge for ChargeDouble {
    fn lire(&self, _racine: &str) -> Result<DeclarationCharge, AppError> {
        DeclarationCharge::new(
            "vps".to_string(),
            "monosite/vps/bac-a-sable/terraform".to_string(),
            "vps_ip".to_string(),
            "/api/sante".to_string(),
            CibleCapacite::new(100.0, 300.0).expect("cible"),
            // Le dépôt demande six heures et deux cents euros ; les réglages du
            // serveur, plus bas, les bornent.
            Duree::secondes(21_600).expect("ttl"),
            PlafondDepense::new(200.0).expect("plafond"),
        )
    }
}

/// Dépôt doublé : rend la fenêtre qu'on lui a confiée, ou aucune.
struct DepotDouble(Option<FenetreDerogation>);

impl DepotDerogation for DepotDouble {
    fn courante(&self) -> Result<Option<FenetreDerogation>, AppError> {
        Ok(self.0.clone())
    }
    fn enregistrer(&self, _fenetre: &FenetreDerogation) -> Result<(), AppError> {
        Ok(())
    }
}

#[derive(Default)]
struct Compteurs {
    provisionnements: AtomicUsize,
    destructions: AtomicUsize,
    paliers: Mutex<Vec<String>>,
}

struct ProvisionneurDouble(Arc<Compteurs>);

impl Provisionneur for ProvisionneurDouble {
    fn provisionner(
        &self,
        _bail: &BailBacASable,
        _sortie_adresse: &str,
    ) -> Result<CibleEphemere, AppError> {
        self.0.provisionnements.fetch_add(1, Ordering::SeqCst);
        CibleEphemere::new(
            "57.128.0.1",
            vec![("vps_ip".to_string(), "57.128.0.1".to_string())],
        )
    }
}

struct DestructeurDouble {
    compteurs: Arc<Compteurs>,
    /// Échoue au premier appel seulement, comme un verrou qui se libère.
    echoue_une_fois: bool,
}

impl DestructeurBail for DestructeurDouble {
    fn detruire(&self, _bail: &BailBacASable) -> Result<(), AppError> {
        let rang = self.compteurs.destructions.fetch_add(1, Ordering::SeqCst);
        if self.echoue_une_fois && rang == 0 {
            return Err(AppError::ServiceTiers {
                service: "terraform".to_string(),
                detail: "instance still locked".to_string(),
            });
        }
        Ok(())
    }
}

struct MoteurDouble(Arc<Compteurs>);

impl MoteurCharge for MoteurDouble {
    fn jouer(&self, cible: &str, reglage: &ReglagePalier) -> Result<Vec<MesureCapacite>, AppError> {
        self.0
            .paliers
            .lock()
            .expect("verrou")
            .push(reglage.palier.nom().to_string());
        // Les mêmes grandeurs que celles qu'analyse le pilote wrk, sans quoi
        // le verdict porterait sur des noms que la production ne produit pas.
        Ok(vec![
            MesureCapacite::mesuree(
                "debit".to_string(),
                240.0,
                "req/s".to_string(),
                reglage.palier,
                1_000,
                format!("cible {cible}"),
            )
            .expect("mesure"),
            MesureCapacite::mesuree(
                "latence_p99".to_string(),
                210.0,
                "ms".to_string(),
                reglage.palier,
                1_000,
                format!("cible {cible}"),
            )
            .expect("mesure"),
        ])
    }
    fn disponible(&self) -> bool {
        true
    }
}

fn outil(fenetre: Option<FenetreDerogation>, compteurs: Arc<Compteurs>) -> OutilCampagne {
    outil_avec(fenetre, compteurs, false)
}

fn outil_avec(
    fenetre: Option<FenetreDerogation>,
    compteurs: Arc<Compteurs>,
    destruction_capricieuse: bool,
) -> OutilCampagne {
    let liste = ListeAutorisation::new(Vec::new(), vec!["bac-koprogo".to_string()]).expect("liste");
    OutilCampagne::new(
        Arc::new(Campagne::new(Arc::new(MoteurDouble(compteurs.clone())))),
        Arc::new(ChargeDouble),
        Arc::new(DepotDouble(fenetre)),
        Arc::new(ProvisionneurDouble(compteurs.clone())),
        Arc::new(DestructeurDouble {
            compteurs,
            echoue_une_fois: destruction_capricieuse,
        }),
        Arc::new(HorlogeFigee::a(MAINTENANT)) as Arc<dyn Horloge>,
        ReglagesCampagne {
            projet: liste.projet_bac_a_sable("bac-koprogo").expect("projet"),
            ttl_maximal: Duree::secondes(3_600).expect("ttl max"),
            plafond: PlafondDepense::new(20.0).expect("plafond"),
            racines_autorisees: vec![DEPOT.to_string()],
        },
    )
}

fn fenetre_de(jours: i64) -> FenetreDerogation {
    FenetreDerogation::ouvrir(
        &approbation(),
        MAINTENANT,
        Duree::jours(jours).expect("durée"),
    )
    .expect("fenêtre")
}

fn arguments(depense: f64, paliers: usize) -> serde_json::Value {
    json!({"depot": DEPOT, "estimation_depense": depense, "paliers": paliers})
}

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_une_campagne_rend_ses_mesures_et_l_adresse_chargee() {
    let compteurs = Arc::new(Compteurs::default());
    let outil = outil(Some(fenetre_de(90)), compteurs.clone());

    let rendu = outil
        .appeler(&arguments(4.0, 2))
        .expect("la campagne doit aboutir");

    assert_eq!(rendu["cible"], json!("57.128.0.1"));
    assert_eq!(compteurs.provisionnements.load(Ordering::SeqCst), 1);
    assert_eq!(compteurs.destructions.load(Ordering::SeqCst), 1);
    assert_eq!(rendu["paliers_joues"].as_array().map(Vec::len), Some(2));
    assert!(!rendu["mesures"].as_array().expect("mesures").is_empty());
}

#[test]
fn happy_l_outil_est_de_tier_deux() {
    let outil = outil(Some(fenetre_de(90)), Arc::new(Compteurs::default()));
    // Écriture bornée : infrastructure éphémère, à TTL et plafond. Le Tier 1
    // reste réservé à ce qui touche la production, et passe par GitHub.
    assert_eq!(outil.tier(), Tier::Two);
}

// ── @negative ────────────────────────────────────────────────

#[test]
fn negative_sans_fenetre_aucune_campagne_et_le_message_dit_quoi_faire() {
    let compteurs = Arc::new(Compteurs::default());
    let outil = outil(None, compteurs.clone());

    let erreur = outil
        .appeler(&arguments(4.0, 2))
        .expect_err("aucune fenêtre, aucune campagne");

    assert!(
        erreur.to_string().contains("Tier 1"),
        "le refus doit nommer le renouvellement attendu : {erreur}"
    );
    assert_eq!(compteurs.provisionnements.load(Ordering::SeqCst), 0);
}

#[test]
fn negative_un_champ_inconnu_est_refuse() {
    let outil = outil(Some(fenetre_de(90)), Arc::new(Compteurs::default()));

    assert!(outil
        .appeler(&json!({"depot": DEPOT, "estimation_depense": 4.0, "surprise": 1}))
        .is_err());
}

// ── @edge ────────────────────────────────────────────────────

#[test]
fn security_le_ttl_demande_par_le_depot_est_ecrete_par_le_serveur() {
    let compteurs = Arc::new(Compteurs::default());
    let outil = outil(Some(fenetre_de(90)), compteurs.clone());

    // La déclaration du dépôt demande six heures ; les réglages du serveur
    // n'en autorisent qu'une. Le bail rendu doit porter la plus courte, sinon
    // écrire dans le dépôt mesuré suffirait à s'octroyer six heures.
    let rendu = outil.appeler(&arguments(4.0, 2)).expect("campagne");

    let ouvert = rendu["bail"]["ouvert_le"].as_i64().expect("ouvert_le");
    let expire = rendu["bail"]["expire_le"].as_i64().expect("expire_le");
    assert_eq!(expire - ouvert, 3_600);
}

#[test]
fn negative_un_depot_non_autorise_n_est_meme_pas_lu() {
    let compteurs = Arc::new(Compteurs::default());
    let outil = outil(Some(fenetre_de(90)), compteurs.clone());

    let erreur = outil
        .appeler(&json!({"depot": "/tmp/depot-inconnu", "estimation_depense": 4.0}))
        .expect_err("dépôt hors liste blanche");

    assert!(erreur.to_string().contains("droit de mesurer"));
    assert_eq!(compteurs.provisionnements.load(Ordering::SeqCst), 0);
}

#[test]
fn happy_le_verdict_confronte_les_mesures_a_la_cible_declaree() {
    let compteurs = Arc::new(Compteurs::default());
    let outil = outil(Some(fenetre_de(90)), compteurs);

    // Escalier complet : le palier de référence est joué, donc un verdict est
    // rendu plutôt qu'un indéterminé.
    let rendu = outil.appeler(&arguments(4.0, 7)).expect("campagne");

    assert!(
        rendu["verdict"]["Tient"].is_object(),
        "240 req/s et 210 ms tiennent la cible déclarée : {}",
        rendu["verdict"]
    );
}

#[test]
fn edge_un_escalier_ecourte_rend_un_verdict_indetermine() {
    let compteurs = Arc::new(Compteurs::default());
    let outil = outil(Some(fenetre_de(90)), compteurs);

    let rendu = outil.appeler(&arguments(4.0, 2)).expect("campagne");

    // Deux paliers seulement : le palier réaliste n'a pas tourné, et un
    // verdict rendu sans lui serait du supposé présenté comme du mesuré.
    assert!(rendu["verdict"]["Indetermine"].is_object());
}

// ── @security ────────────────────────────────────────────────

#[test]
fn security_une_fenetre_expiree_interdit_la_campagne() {
    let compteurs = Arc::new(Compteurs::default());
    // Fenêtre ouverte pour un jour, horloge figée bien après : c'est le
    // scénario du 91e jour, celui qu'ADR-007 rend inévitable.
    let fenetre = FenetreDerogation::ouvrir(
        &approbation(),
        Horodatage::new(0),
        Duree::secondes(10).expect("durée"),
    )
    .expect("fenêtre");
    let outil = outil(Some(fenetre), compteurs.clone());

    let erreur = outil
        .appeler(&arguments(4.0, 2))
        .expect_err("fenêtre expirée");

    assert!(erreur.to_string().contains("expiré"));
    assert_eq!(compteurs.provisionnements.load(Ordering::SeqCst), 0);
}

#[test]
fn security_une_depense_au_dessus_du_plafond_est_refusee_avant_la_facture() {
    let compteurs = Arc::new(Compteurs::default());
    let outil = outil(Some(fenetre_de(90)), compteurs.clone());

    let erreur = outil
        .appeler(&arguments(35.0, 2))
        .expect_err("dépense au-dessus du plafond");

    assert!(erreur.to_string().contains("facture"));
    assert_eq!(compteurs.provisionnements.load(Ordering::SeqCst), 0);
}

#[test]
fn security_une_campagne_plus_longue_que_la_fenetre_est_refusee() {
    let compteurs = Arc::new(Compteurs::default());
    // Fenêtre de deux minutes : l'escalier complet dure bien plus, donc la
    // campagne serait coupée en plein palier.
    let fenetre = FenetreDerogation::ouvrir(
        &approbation(),
        MAINTENANT,
        Duree::secondes(120).expect("durée"),
    )
    .expect("fenêtre");
    let outil = outil(Some(fenetre), compteurs.clone());

    let erreur = outil
        .appeler(&arguments(4.0, 7))
        .expect_err("campagne trop longue pour la fenêtre");

    assert!(erreur.to_string().contains("admission"));
    assert_eq!(compteurs.provisionnements.load(Ordering::SeqCst), 0);
}

#[test]
fn security_une_destruction_ratee_est_retentee_par_la_garde() {
    let compteurs = Arc::new(Compteurs::default());
    let outil = outil_avec(Some(fenetre_de(90)), compteurs.clone(), true);

    let rendu = outil
        .appeler(&arguments(4.0, 2))
        .expect("les mesures survivent");

    assert!(rendu["echec_destruction"].is_string());
    // La garde n'est désarmée que sur destruction acquise. Ici la première a
    // échoué, donc la sortie de portée en tente une seconde : c'est ce qui
    // évite qu'une infrastructure continue de facturer parce qu'un verrou
    // terraform s'est libéré une seconde trop tard.
    assert_eq!(compteurs.destructions.load(Ordering::SeqCst), 2);
}
