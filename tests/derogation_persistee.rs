//! Story 5.4 — la fenêtre de dérogation survit à un redémarrage, sous sceau.
//!
//! Sans persistance, un redémarrage du service referme la fenêtre et exige un
//! nouvel acte de Tier 1 : le comportement serait sûr, mais inexploitable.
//!
//! Persister pose alors la question que ce fichier tranche : `ouvrir` exige un
//! jeton d'approbation consommé, or relire un fichier n'en produit aucun. Le
//! sceau tient ce rôle. Il n'est calculable qu'avec le secret de signature du
//! serveur, donc écrire dans le fichier ne suffit pas à s'octroyer une fenêtre,
//! et c'est exactement ce que vérifient les tests @security.

use sluis::application::ports::DepotDerogation;
use sluis::domain::{
    Action, Duree, Environnement, FenetreDerogation, Horodatage, JetonChangement, JetonConsomme,
    PlanChangement, Tier,
};
use sluis::infrastructure::derogation_depot::DepotDerogationFichier;

const MAINTENANT: Horodatage = Horodatage::new(1_000_000);
const SECRET: &[u8] = b"secret-de-signature-du-serveur";

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

fn fenetre() -> FenetreDerogation {
    FenetreDerogation::ouvrir(&approbation(), MAINTENANT, Duree::jours(90).expect("durée"))
        .expect("ouverture")
}

/// Chemin de fichier propre à chaque test, supprimé à la sortie de portée.
struct FichierTemporaire(std::path::PathBuf);

impl FichierTemporaire {
    fn nouveau(nom: &str) -> Self {
        let chemin = std::env::temp_dir().join(format!("sluis-derogation-{nom}.json"));
        let _ = std::fs::remove_file(&chemin);
        Self(chemin)
    }
    fn chemin(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for FichierTemporaire {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_une_fenetre_enregistree_est_relue_a_l_identique() {
    let fichier = FichierTemporaire::nouveau("relue");
    let depot = DepotDerogationFichier::nouveau(fichier.chemin(), SECRET);

    depot.enregistrer(&fenetre()).expect("enregistrement");
    let relue = depot.courante().expect("lecture").expect("une fenêtre");

    assert_eq!(relue, fenetre());
    assert_eq!(relue.approbateur(), "Gilles Maury");
}

#[test]
fn happy_une_fenetre_relue_valide_encore_pendant_sa_duree() {
    let fichier = FichierTemporaire::nouveau("valide");
    let depot = DepotDerogationFichier::nouveau(fichier.chemin(), SECRET);
    depot.enregistrer(&fenetre()).expect("enregistrement");

    let relue = depot.courante().expect("lecture").expect("une fenêtre");

    assert!(relue.valider(MAINTENANT).is_ok());
}

// ── @edge ────────────────────────────────────────────────────

#[test]
fn edge_l_absence_de_fichier_vaut_fenetre_fermee() {
    let fichier = FichierTemporaire::nouveau("absent");
    let depot = DepotDerogationFichier::nouveau(fichier.chemin(), SECRET);

    // Septième condition d'ADR-007 : une fenêtre absente vaut fenêtre fermée.
    // L'absence n'est donc pas une erreur, c'est l'état par défaut, et il
    // interdit tout bail.
    assert!(depot.courante().expect("lecture").is_none());
}

#[test]
fn edge_un_fichier_illisible_est_une_erreur_nommee_pas_un_silence() {
    let fichier = FichierTemporaire::nouveau("illisible");
    std::fs::write(fichier.chemin(), "{ ceci n'est pas du json").expect("écriture");
    let depot = DepotDerogationFichier::nouveau(fichier.chemin(), SECRET);

    let erreur = depot.courante().expect_err("doit échouer");

    // Rendre None ici ferait passer une corruption pour une absence, donc pour
    // un état normal, et le renouvellement de Tier 1 qui suivrait masquerait
    // le problème au lieu de le révéler.
    assert!(
        erreur.to_string().to_lowercase().contains("json")
            || erreur.to_string().contains("dérogation")
    );
}

// ── @security ────────────────────────────────────────────────

#[test]
fn security_une_fenetre_prolongee_a_la_main_est_refusee() {
    let fichier = FichierTemporaire::nouveau("prolongee");
    let depot = DepotDerogationFichier::nouveau(fichier.chemin(), SECRET);
    depot.enregistrer(&fenetre()).expect("enregistrement");

    // Le scénario réel : quelqu'un repousse la fermeture dans le fichier pour
    // s'épargner un renouvellement de Tier 1.
    let contenu = std::fs::read_to_string(fichier.chemin()).expect("lecture");
    let falsifie = contenu.replace(&fenetre().close_le().secondes().to_string(), "9000000000");
    std::fs::write(fichier.chemin(), falsifie).expect("écriture");

    let erreur = depot
        .courante()
        .expect_err("la falsification doit être refusée");
    assert!(
        erreur.to_string().contains("sceau"),
        "l'erreur doit nommer le sceau : {erreur}"
    );
}

#[test]
fn security_une_fenetre_scellee_avec_un_autre_secret_est_refusee() {
    let fichier = FichierTemporaire::nouveau("autre-secret");
    DepotDerogationFichier::nouveau(fichier.chemin(), b"un-autre-secret")
        .enregistrer(&fenetre())
        .expect("enregistrement");

    let depot = DepotDerogationFichier::nouveau(fichier.chemin(), SECRET);

    // Conséquence voulue : une rotation du secret de signature referme les
    // fenêtres ouvertes. Fail-closed, il faut un nouvel acte de Tier 1.
    assert!(depot.courante().is_err());
}

#[test]
fn security_un_sceau_absent_ne_vaut_pas_confiance() {
    let fichier = FichierTemporaire::nouveau("sans-sceau");
    std::fs::write(
        fichier.chemin(),
        r#"{"ouverte_le":1000000,"close_le":9000000000,"approbateur":"quelqu un"}"#,
    )
    .expect("écriture");
    let depot = DepotDerogationFichier::nouveau(fichier.chemin(), SECRET);

    assert!(depot.courante().is_err());
}
