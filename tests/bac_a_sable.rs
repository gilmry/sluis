//! Stories 5.1, 5.2 et 5.4 — baux bornés, destruction garantie, fenêtre.
//!
//! Les sept conditions d'ADR-007 sont vérifiées **chacune** par un test dédié.
//! Le plus important du fichier est
//! `security_le_chien_de_garde_survit_a_la_disparition_du_demandeur` : c'est
//! lui qui distingue une dérogation encadrée d'une facture ouverte.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use sluis::domain::{
    Action, AppError, BailBacASable, Duree, Empreinte, Environnement, FenetreDerogation,
    Horodatage, JetonChangement, JetonConsomme, ListeAutorisation, PlafondDepense, PlanChangement,
    Tier,
};
use sluis::infrastructure::bac_a_sable::{ChienDeGarde, DestructeurBail, GardeBail, RegistreBaux};

const MAINTENANT: Horodatage = Horodatage::new(1_000_000);

/// Fabrique une approbation de Tier 1 réelle, en passant par tout le chemin.
fn approbation(approbateur: &str) -> JetonConsomme {
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
        approbateur.to_string(),
        MAINTENANT,
        Duree::secondes(600).expect("durée"),
    )
    .expect("émission")
    .consommer(&plan, MAINTENANT)
    .expect("consommation")
}

fn fenetre_ouverte(jours: i64) -> FenetreDerogation {
    FenetreDerogation::ouvrir(
        &approbation("Gilles Maury"),
        MAINTENANT,
        Duree::jours(jours).expect("durée"),
    )
    .expect("ouverture")
}

fn liste() -> ListeAutorisation {
    ListeAutorisation::new(vec!["prj-prod".to_string()], vec!["prj-bac".to_string()])
        .expect("liste")
}

/// Destructeur doublé, qui compte et peut échouer.
struct DestructeurDouble {
    appels: AtomicUsize,
    echoue: bool,
    projets: Mutex<Vec<String>>,
}

impl DestructeurDouble {
    fn nouveau(echoue: bool) -> Arc<Self> {
        Arc::new(Self {
            appels: AtomicUsize::new(0),
            echoue,
            projets: Mutex::new(Vec::new()),
        })
    }
    fn appels(&self) -> usize {
        self.appels.load(Ordering::SeqCst)
    }
}

impl DestructeurBail for DestructeurDouble {
    fn detruire(&self, bail: &BailBacASable) -> Result<(), AppError> {
        self.appels.fetch_add(1, Ordering::SeqCst);
        if let Ok(mut p) = self.projets.lock() {
            p.push(bail.projet().to_string());
        }
        if self.echoue {
            Err(AppError::ServiceTiers {
                service: "OVH".to_string(),
                detail: "quota".to_string(),
            })
        } else {
            Ok(())
        }
    }
}

fn louer(fenetre: &FenetreDerogation, ttl_secondes: i64) -> Result<BailBacASable, AppError> {
    let derogation = fenetre.valider(MAINTENANT)?;
    BailBacASable::louer(
        &derogation,
        liste().projet_bac_a_sable("prj-bac")?,
        Duree::secondes(ttl_secondes)?,
        PlafondDepense::new(20.0)?,
        5.0,
        Duree::secondes(21_600)?,
        MAINTENANT,
    )
}

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_un_bail_nominal_se_loue_dans_une_fenetre_ouverte() {
    let bail = louer(&fenetre_ouverte(90), 3_600).expect("bail");
    assert_eq!(bail.projet().identifiant(), "prj-bac");
    assert_eq!(bail.expire_le(), Horodatage::new(1_003_600));
    assert!(!bail.expire(MAINTENANT));
}

#[test]
fn happy_une_fenetre_ouverte_rend_une_preuve_de_validite() {
    let fenetre = fenetre_ouverte(90);
    assert_eq!(fenetre.approbateur(), "Gilles Maury");
    assert!(fenetre.valider(MAINTENANT).is_ok());
}

// ── @negative ────────────────────────────────────────────────

#[test]
fn negative_un_ttl_au_dela_du_maximum_est_refuse() {
    let erreur = louer(&fenetre_ouverte(90), 100_000).unwrap_err();
    assert!(erreur.to_string().contains("maximum"), "obtenu : {erreur}");
}

#[test]
fn negative_une_estimation_au_dessus_du_plafond_est_refusee_a_l_admission() {
    let fenetre = fenetre_ouverte(90);
    let derogation = fenetre.valider(MAINTENANT).expect("valide");
    let erreur = BailBacASable::louer(
        &derogation,
        liste().projet_bac_a_sable("prj-bac").expect("projet"),
        Duree::secondes(3_600).expect("ttl"),
        PlafondDepense::new(20.0).expect("plafond"),
        35.0,
        Duree::secondes(21_600).expect("max"),
        MAINTENANT,
    )
    .unwrap_err();
    assert!(
        erreur.to_string().contains("facture"),
        "le refus doit dire pourquoi il vaut mieux qu'une découverte sur la facture : {erreur}"
    );
}

#[test]
fn negative_un_plafond_nul_ou_negatif_est_refuse() {
    assert!(PlafondDepense::new(0.0).is_err());
    assert!(PlafondDepense::new(-1.0).is_err());
    assert!(PlafondDepense::new(f64::NAN).is_err());
}

#[test]
fn negative_une_fenetre_expiree_refuse_toute_location() {
    let fenetre = fenetre_ouverte(1);
    let bien_plus_tard = Horodatage::new(MAINTENANT.secondes() + 86_400 * 2);
    let erreur = fenetre.valider(bien_plus_tard).unwrap_err();
    assert!(
        erreur.to_string().contains("renouvellement de Tier 1"),
        "le message doit dire quoi faire, obtenu : {erreur}"
    );
}

// ── @edge ────────────────────────────────────────────────────

#[test]
fn edge_une_campagne_qui_survivrait_a_la_fenetre_est_refusee_a_l_admission() {
    // Le cas qui compte : la fenêtre ferme dans deux heures, la campagne en
    // demande six. Refuser maintenant vaut mieux que couper à mi-parcours.
    let fenetre = FenetreDerogation::ouvrir(
        &approbation("a"),
        MAINTENANT,
        Duree::secondes(7_200).expect("durée"),
    )
    .expect("fenêtre");
    let derogation = fenetre.valider(MAINTENANT).expect("valide");
    let erreur = BailBacASable::louer(
        &derogation,
        liste().projet_bac_a_sable("prj-bac").expect("projet"),
        Duree::secondes(21_600).expect("ttl"),
        PlafondDepense::new(20.0).expect("plafond"),
        1.0,
        Duree::secondes(21_600).expect("max"),
        MAINTENANT,
    )
    .unwrap_err();
    assert!(
        erreur.to_string().contains("interrompue"),
        "obtenu : {erreur}"
    );
}

#[test]
fn edge_la_validite_a_la_seconde_exacte_de_fermeture_est_encore_acquise() {
    let fenetre = FenetreDerogation::ouvrir(
        &approbation("a"),
        MAINTENANT,
        Duree::secondes(100).expect("durée"),
    )
    .expect("fenêtre");
    assert!(fenetre.valider(fenetre.close_le()).is_ok());
    assert!(fenetre
        .valider(Horodatage::new(fenetre.close_le().secondes() + 1))
        .is_err());
}

#[test]
fn edge_une_fenetre_pas_encore_ouverte_refuse_aussi() {
    let fenetre = fenetre_ouverte(90);
    let avant = Horodatage::new(MAINTENANT.secondes() - 1);
    assert!(fenetre.valider(avant).is_err());
}

#[test]
fn edge_une_double_destruction_reste_idempotente() {
    let destructeur = DestructeurDouble::nouveau(false);
    let bail = louer(&fenetre_ouverte(90), 3_600).expect("bail");
    let garde = GardeBail::nouvelle(bail, destructeur.clone());
    garde.detruire_maintenant().expect("destruction");
    garde.detruire_maintenant().expect("seconde destruction");
    drop(garde);
    assert_eq!(
        destructeur.appels(),
        1,
        "la garde ne doit détruire qu'une fois, même appelée plusieurs fois"
    );
}

// ── @security — les sept conditions d'ADR-007 ────────────────

#[test]
fn security_condition_1_les_deux_listes_de_projets_sont_disjointes() {
    assert!(
        ListeAutorisation::new(vec!["commun".to_string()], vec!["commun".to_string()]).is_err()
    );
    // Et un projet de production ne peut pas servir de bac à sable.
    assert!(liste().projet_bac_a_sable("prj-prod").is_err());
}

#[test]
fn security_condition_2_un_bail_a_toujours_un_ttl() {
    // Porté par la signature : Duree n'est pas optionnel, et Duree::secondes
    // refuse zéro. Un bail sans échéance n'est pas exprimable.
    assert!(Duree::secondes(0).is_err());
    let bail = louer(&fenetre_ouverte(90), 60).expect("bail");
    assert!(bail.expire_le().apres(bail.ouvert_le()));
}

#[test]
fn security_condition_3_un_bail_a_toujours_un_plafond() {
    assert!(PlafondDepense::new(0.0).is_err());
    let bail = louer(&fenetre_ouverte(90), 60).expect("bail");
    assert!(bail.plafond().montant() > 0.0);
}

#[test]
fn security_condition_7_hors_fenetre_la_location_est_inexprimable() {
    // Il n'existe aucun chemin vers BailBacASable::louer sans une
    // DerogationValide, et celle-ci ne sort que de FenetreDerogation::valider.
    // Hors fenêtre, valider échoue, donc la preuve n'existe pas.
    let fenetre = fenetre_ouverte(1);
    let trop_tard = Horodatage::new(MAINTENANT.secondes() + 86_400 * 3);
    assert!(fenetre.valider(trop_tard).is_err());
}

#[test]
fn security_une_fenetre_ne_s_ouvre_que_sur_preuve_d_approbation_tier1() {
    // FenetreDerogation::ouvrir exige un JetonConsomme, qui ne s'obtient qu'en
    // consommant un jeton émis pour un plan de Tier 1. L'autorité de déléguer
    // n'est donc jamais elle-même déléguée.
    let jeton_anonyme = JetonChangement::emettre(
        Empreinte::calculer("x"),
        "  ".to_string(),
        MAINTENANT,
        Duree::secondes(60).expect("durée"),
    );
    assert!(
        jeton_anonyme.is_err(),
        "une approbation anonyme ne doit pas exister"
    );
}

#[test]
fn security_la_garde_detruit_le_bail_meme_apres_une_panique() {
    let destructeur = DestructeurDouble::nouveau(false);
    let bail = louer(&fenetre_ouverte(90), 3_600).expect("bail");
    let destructeur_dans_fil = destructeur.clone();

    let resultat = std::thread::spawn(move || {
        let _garde = GardeBail::nouvelle(bail, destructeur_dans_fil);
        panic!("la campagne panique en plein palier");
    })
    .join();

    assert!(resultat.is_err(), "le fil doit bien avoir paniqué");
    assert_eq!(
        destructeur.appels(),
        1,
        "le bail doit être détruit malgré la panique"
    );
}

#[test]
fn security_le_chien_de_garde_survit_a_la_disparition_du_demandeur() {
    // LE test du bounded context. Le demandeur n'existe plus — on simule sa
    // disparition en n'utilisant aucune garde RAII — et le bail doit être
    // détruit quand même, à l'échéance.
    let registre = Arc::new(RegistreBaux::new());
    let destructeur = DestructeurDouble::nouveau(false);
    let bail = louer(&fenetre_ouverte(90), 60).expect("bail");
    registre.inscrire(bail).expect("inscription");

    let chien = ChienDeGarde::new(registre.clone(), destructeur.clone());

    // Avant l'échéance : rien.
    assert_eq!(chien.ronde(MAINTENANT), 0);
    assert_eq!(registre.vivants(), 1);

    // Après l'échéance : détruit, sans aucune intervention du demandeur.
    let apres = Horodatage::new(MAINTENANT.secondes() + 61);
    assert_eq!(chien.ronde(apres), 1);
    assert_eq!(destructeur.appels(), 1);
    assert_eq!(registre.vivants(), 0);
    assert_eq!(chien.detruits(), 1);
}

#[test]
fn security_un_echec_de_destruction_est_retente_et_jamais_abandonne() {
    // Un bail qu'on croit détruit et qui facture encore est le pire cas.
    let registre = Arc::new(RegistreBaux::new());
    let destructeur = DestructeurDouble::nouveau(true);
    let bail = louer(&fenetre_ouverte(90), 60).expect("bail");
    registre.inscrire(bail).expect("inscription");

    let chien = ChienDeGarde::new(registre.clone(), destructeur.clone());
    let apres = Horodatage::new(MAINTENANT.secondes() + 61);

    assert_eq!(
        chien.ronde(apres),
        0,
        "aucun bail traité, la destruction échoue"
    );
    assert_eq!(chien.echecs(), 1);
    assert_eq!(
        registre.vivants(),
        1,
        "le bail est réinscrit : l'abandonner reviendrait à le perdre de vue"
    );

    // Deuxième ronde : retenté.
    chien.ronde(apres);
    assert_eq!(destructeur.appels(), 2);
}
