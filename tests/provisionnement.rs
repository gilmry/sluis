//! Story 5.2 — provisionner et détruire une infrastructure éphémère.
//!
//! Aucun test ne lance terraform : le moteur est doublé, ce qui rend les cas
//! d'échec reproductibles et respecte NFR-06.
//!
//! Le test qui compte le plus du fichier est
//! `security_une_destruction_est_idempotente` : la garde RAII et le chien de
//! garde peuvent tous deux réclamer la destruction du même bail, et si le
//! second appel échouait, un nettoyage réussi se lirait comme une panne.

use std::sync::Mutex;

use sluis::application::ports::MoteurTerraform;
use sluis::application::ports::Provisionneur;
use sluis::domain::{
    Action, AppError, BailBacASable, DemandeBail, Duree, Environnement, FenetreDerogation,
    Horodatage, JetonChangement, JetonConsomme, ListeAutorisation, MutationTerraform,
    PlafondDepense, PlanChangement, PlanTerraform, Tier, ValeurSure,
};
use sluis::infrastructure::bac_a_sable::{BacASableTerraform, DestructeurBail};

/// Le module que porte un bail de test.
fn module_du_bail() -> sluis::domain::ValeurSure {
    sluis::domain::ValeurSure::new("depots/projet/infra/bac-a-sable").expect("module")
}

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
        DemandeBail {
            projet: liste.projet_bac_a_sable("bac-koprogo").expect("projet"),
            module: module_du_bail(),
            ttl: Duree::secondes(3_600).expect("ttl"),
            plafond: PlafondDepense::new(20.0).expect("plafond"),
            estimation_depense: 4.0,
        },
        Duree::secondes(21_600).expect("max"),
        MAINTENANT,
    )
    .expect("bail")
}

/// Moteur Terraform doublé : consigne l'ordre des appels.
struct MoteurDouble {
    appels: Mutex<Vec<String>>,
    echec_apply: bool,
    echec_destroy: bool,
    sorties: Vec<(String, String)>,
}

impl MoteurDouble {
    fn nouveau() -> Self {
        Self {
            appels: Mutex::new(Vec::new()),
            echec_apply: false,
            echec_destroy: false,
            sorties: vec![
                ("vps_ip".to_string(), "57.128.0.1".to_string()),
                (
                    "ssh_command".to_string(),
                    "ssh ubuntu@57.128.0.1".to_string(),
                ),
            ],
        }
    }

    fn appels(&self) -> Vec<String> {
        self.appels.lock().expect("verrou").clone()
    }

    fn consigner(&self, quoi: &str) {
        self.appels.lock().expect("verrou").push(quoi.to_string());
    }
}

impl MoteurTerraform for MoteurDouble {
    fn plan(&self, _module: &ValeurSure) -> Result<PlanTerraform, AppError> {
        self.consigner("plan");
        Ok(PlanTerraform {
            creations: 0,
            modifications: 0,
            destructions: 0,
            brut: String::new(),
        })
    }

    fn initialiser(&self, _module: &ValeurSure) -> Result<(), AppError> {
        self.consigner("init");
        Ok(())
    }

    fn appliquer(
        &self,
        _module: &ValeurSure,
        _bail: &BailBacASable,
    ) -> Result<MutationTerraform, AppError> {
        self.consigner("apply");
        if self.echec_apply {
            return Err(AppError::ServiceTiers {
                service: "terraform".to_string(),
                detail: "quota exceeded".to_string(),
            });
        }
        Ok(MutationTerraform {
            creations: 3,
            modifications: 0,
            destructions: 0,
            brut: "Apply complete! Resources: 3 added, 0 changed, 0 destroyed.".to_string(),
        })
    }

    fn detruire(&self, _module: &ValeurSure) -> Result<MutationTerraform, AppError> {
        self.consigner("destroy");
        if self.echec_destroy {
            return Err(AppError::ServiceTiers {
                service: "terraform".to_string(),
                detail: "instance still locked".to_string(),
            });
        }
        Ok(MutationTerraform {
            creations: 0,
            modifications: 0,
            destructions: 3,
            brut: "Destroy complete! Resources: 3 destroyed.".to_string(),
        })
    }

    fn sorties(&self, _module: &ValeurSure) -> Result<Vec<(String, String)>, AppError> {
        self.consigner("output");
        Ok(self.sorties.clone())
    }
}

/// Le pilote prend son moteur par valeur ; les tests veulent garder le double
/// observable après coup, d'où cette délégation, calquée sur `tests/moteurs.rs`.
impl MoteurTerraform for &MoteurDouble {
    fn plan(&self, module: &ValeurSure) -> Result<PlanTerraform, AppError> {
        (*self).plan(module)
    }
    fn initialiser(&self, module: &ValeurSure) -> Result<(), AppError> {
        (*self).initialiser(module)
    }
    fn appliquer(
        &self,
        module: &ValeurSure,
        bail: &BailBacASable,
    ) -> Result<MutationTerraform, AppError> {
        (*self).appliquer(module, bail)
    }
    fn detruire(&self, module: &ValeurSure) -> Result<MutationTerraform, AppError> {
        (*self).detruire(module)
    }
    fn sorties(&self, module: &ValeurSure) -> Result<Vec<(String, String)>, AppError> {
        (*self).sorties(module)
    }
}

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_le_provisionnement_initialise_applique_puis_lit_les_sorties() {
    let moteur = MoteurDouble::nouveau();
    let bac = BacASableTerraform::new(&moteur);

    let cible = bac
        .provisionner(&bail(), "vps_ip")
        .expect("provisionnement");

    assert_eq!(cible.adresse(), "57.128.0.1");
    // L'ordre n'est pas cosmétique : un apply sans init échoue sur un module
    // dont les fournisseurs ne sont pas téléchargés, et lire les sorties avant
    // l'apply rendrait celles du tour précédent.
    assert_eq!(moteur.appels(), vec!["init", "apply", "output"]);
}

#[test]
fn happy_les_sorties_du_module_sont_conservees_avec_la_cible() {
    let moteur = MoteurDouble::nouveau();
    let bac = BacASableTerraform::new(&moteur);

    let cible = bac
        .provisionner(&bail(), "vps_ip")
        .expect("provisionnement");

    assert!(cible.sorties().iter().any(|(nom, _)| nom == "ssh_command"));
}

#[test]
fn happy_la_destruction_appelle_terraform_destroy() {
    let moteur = MoteurDouble::nouveau();
    let bac = BacASableTerraform::new(&moteur);

    bac.detruire(&bail()).expect("destruction");

    assert_eq!(moteur.appels(), vec!["destroy"]);
}

// ── @negative ────────────────────────────────────────────────

#[test]
fn negative_un_apply_en_echec_ne_rend_aucune_cible() {
    let mut moteur = MoteurDouble::nouveau();
    moteur.echec_apply = true;
    let bac = BacASableTerraform::new(&moteur);

    let erreur = bac
        .provisionner(&bail(), "vps_ip")
        .expect_err("doit échouer");

    assert!(erreur.to_string().contains("quota"));
    // Les sorties ne sont pas lues : elles décriraient un état partiel, et une
    // adresse tirée d'un apply raté enverrait la charge nulle part.
    assert_eq!(moteur.appels(), vec!["init", "apply"]);
}

#[test]
fn negative_une_sortie_d_adresse_absente_est_une_erreur_nommee() {
    let mut moteur = MoteurDouble::nouveau();
    moteur.sorties = vec![("autre_chose".to_string(), "x".to_string())];
    let bac = BacASableTerraform::new(&moteur);

    let erreur = bac
        .provisionner(&bail(), "vps_ip")
        .expect_err("doit échouer");

    assert!(
        erreur.to_string().contains("vps_ip"),
        "l'erreur doit nommer la sortie attendue : {erreur}"
    );
}

#[test]
fn negative_une_destruction_en_echec_remonte_l_erreur() {
    let mut moteur = MoteurDouble::nouveau();
    moteur.echec_destroy = true;
    let bac = BacASableTerraform::new(&moteur);

    let erreur = bac.detruire(&bail()).expect_err("doit échouer");

    assert!(erreur.to_string().contains("locked"));
}

// ── @edge ────────────────────────────────────────────────────

#[test]
fn edge_une_adresse_vide_est_refusee_plutot_que_transmise() {
    let mut moteur = MoteurDouble::nouveau();
    moteur.sorties = vec![("vps_ip".to_string(), String::new())];
    let bac = BacASableTerraform::new(&moteur);

    // Une campagne lancée contre une adresse vide mesurerait le vide et
    // rendrait des chiffres qu'on croirait valides.
    assert!(bac.provisionner(&bail(), "vps_ip").is_err());
}

// ── @security ────────────────────────────────────────────────

#[test]
fn security_une_destruction_est_idempotente() {
    let moteur = MoteurDouble::nouveau();
    let bac = BacASableTerraform::new(&moteur);

    bac.detruire(&bail()).expect("première destruction");
    bac.detruire(&bail()).expect("seconde destruction");

    // La garde RAII et le chien de garde réclament tous deux la destruction du
    // même bail. Si la seconde échouait, un nettoyage réussi se lirait comme
    // une panne et déclencherait une intervention inutile.
    assert_eq!(moteur.appels(), vec!["destroy", "destroy"]);
}
