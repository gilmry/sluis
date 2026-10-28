//! Story 8 — k6 remplace wrk comme moteur de charge (sluis#8).
//!
//! Aucun test ne lance k6 : NFR-06 impose que la suite n'en dépende pas.
//! L'exécuteur est doublé ; la sortie qu'il rend est celle d'un vrai run de
//! l'image exécutante, capturée dans `fixtures/k6-palier.txt`.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use sluis::application::ports::{
    EnvironnementIsole, Executeur, MoteurCharge, ReglagePalier, SortieProcessus,
};
use sluis::domain::{AppError, Palier};
use sluis::infrastructure::charge::{analyser_sortie, ecrire_script, K6, SCRIPT_K6};

const SORTIE_K6: &str = include_str!("fixtures/k6-palier.txt");

/// Exécuteur doublé : consigne les arguments et le script tel qu'il existe
/// au moment de l'appel, puis rend une sortie décidée d'avance.
struct ExecuteurDouble {
    code: i32,
    sortie: String,
    appels: Mutex<Vec<Appel>>,
}

#[derive(Clone)]
struct Appel {
    programme: String,
    arguments: Vec<String>,
    script: Option<String>,
    environnement: EnvironnementIsole,
}

impl ExecuteurDouble {
    fn rendant(code: i32, sortie: &str) -> Self {
        Self {
            code,
            sortie: sortie.to_string(),
            appels: Mutex::new(Vec::new()),
        }
    }

    fn appel_run(&self) -> Appel {
        self.appels
            .lock()
            .expect("verrou")
            .iter()
            .find(|appel| appel.programme == "k6" && appel.arguments[0] == "run")
            .expect("un appel à k6 run")
            .clone()
    }
}

/// Le double, partagé entre le pilote et le test qui lit ses appels.
#[derive(Clone)]
struct Partage(Arc<ExecuteurDouble>);

impl Executeur for Partage {
    fn executer(
        &self,
        programme: &str,
        arguments: &[String],
        dossier: Option<&str>,
    ) -> Result<SortieProcessus, AppError> {
        self.0.executer(programme, arguments, dossier)
    }

    fn executer_isole(
        &self,
        programme: &str,
        arguments: &[String],
        dossier: &str,
        environnement: &EnvironnementIsole,
    ) -> Result<SortieProcessus, AppError> {
        self.0
            .executer_isole(programme, arguments, dossier, environnement)
    }
}

impl Executeur for ExecuteurDouble {
    fn executer(
        &self,
        _programme: &str,
        _arguments: &[String],
        _dossier: Option<&str>,
    ) -> Result<SortieProcessus, AppError> {
        unreachable!("k6 est toujours lancé isolé")
    }

    fn executer_isole(
        &self,
        programme: &str,
        arguments: &[String],
        _dossier: &str,
        environnement: &EnvironnementIsole,
    ) -> Result<SortieProcessus, AppError> {
        let script = arguments
            .last()
            .and_then(|chemin| std::fs::read_to_string(chemin).ok());
        self.appels.lock().expect("verrou").push(Appel {
            programme: programme.to_string(),
            arguments: arguments.to_vec(),
            script,
            environnement: environnement.clone(),
        });
        Ok(SortieProcessus {
            code: self.code,
            sortie: self.sortie.clone(),
            erreur: "k6 a échoué".to_string(),
        })
    }
}

/// Un dossier de travail propre à chaque test.
fn dossier(nom: &str) -> PathBuf {
    let chemin = std::env::temp_dir().join(format!("sluis-test-k6-{nom}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&chemin);
    std::fs::create_dir_all(&chemin).expect("dossier");
    chemin
}

fn reglage() -> ReglagePalier {
    ReglagePalier {
        palier: Palier::Medium,
        connexions: 50,
        duree_secondes: 60,
    }
}

fn contenu(dossier: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dossier)
        .expect("lecture")
        .map(|e| e.expect("entrée").path())
        .collect()
}

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_un_palier_joue_le_script_embarque_et_rend_les_trois_grandeurs() {
    let travail = dossier("happy");
    let executeur = ExecuteurDouble::rendant(0, SORTIE_K6);
    let k6 = K6::avec_dossier(executeur, travail.clone());

    let mesures = k6.jouer("http://cible/", &reglage()).expect("palier");

    let grandeurs: Vec<&str> = mesures.iter().map(|m| m.grandeur()).collect();
    assert_eq!(grandeurs, ["debit", "latence_p50", "latence_p99"]);
    assert!(mesures[0].conditions().contains("50 utilisateurs virtuels"));
}

#[test]
fn happy_la_commande_porte_la_cible_les_utilisateurs_et_la_duree() {
    let travail = dossier("commande");
    let double = Partage(Arc::new(ExecuteurDouble::rendant(0, SORTIE_K6)));
    K6::avec_dossier(double.clone(), travail)
        .jouer("http://cible/", &reglage())
        .expect("palier");
    let appel = double.0.appel_run();

    let suite = appel.arguments.join(" ");
    assert!(suite.contains("--vus 50"), "{suite}");
    assert!(suite.contains("--duration 60s"), "{suite}");
    assert!(suite.contains("-e SLUIS_CIBLE=http://cible/"), "{suite}");
    assert!(
        suite.contains("--summary-trend-stats p(50),p(99)"),
        "{suite}"
    );
    assert_eq!(
        appel.script.as_deref(),
        Some(SCRIPT_K6),
        "k6 joue le script embarqué"
    );
}

// ── @negative ────────────────────────────────────────────────

#[test]
fn negative_une_sortie_sans_resume_est_une_erreur_nommant_k6_et_le_palier() {
    let erreur = analyser_sortie("running (1m00.0s)\n", Palier::Heavy, "c").unwrap_err();
    let message = erreur.to_string();
    assert!(message.contains("k6"), "{message}");
    assert!(message.contains("heavy"), "{message}");
}

#[test]
fn negative_un_k6_en_echec_est_une_erreur_du_service_k6() {
    let k6 = K6::avec_dossier(ExecuteurDouble::rendant(107, ""), dossier("echec"));
    let erreur = k6.jouer("http://cible/", &reglage()).unwrap_err();
    assert!(
        matches!(&erreur, AppError::ServiceTiers { service, .. } if service == "k6"),
        "{erreur:?}"
    );
}

#[test]
fn negative_un_k6_qui_ne_repond_pas_n_est_pas_disponible() {
    let k6 = K6::avec_dossier(ExecuteurDouble::rendant(1, ""), dossier("indisponible"));
    assert!(!k6.disponible());
}

// ── @edge ────────────────────────────────────────────────────

#[test]
fn edge_un_resume_sans_requete_n_est_pas_une_mesure() {
    let sortie = "SLUIS_RESUME {\"requetes\":0}\n";
    let erreur = analyser_sortie(sortie, Palier::Light, "c").unwrap_err();
    assert!(erreur.to_string().contains("sans échantillon"), "{erreur}");
}

#[test]
fn edge_seule_la_ligne_marquee_est_lue_au_milieu_du_bruit() {
    let sortie = format!("avertissement de k6\n{SORTIE_K6}fin\n");
    assert_eq!(
        analyser_sortie(&sortie, Palier::Light, "c")
            .expect("analyse")
            .len(),
        3
    );
}

#[test]
fn edge_le_script_est_supprime_apres_le_palier_meme_en_echec() {
    let travail = dossier("menage");
    K6::avec_dossier(ExecuteurDouble::rendant(0, SORTIE_K6), travail.clone())
        .jouer("http://cible/", &reglage())
        .expect("palier");
    let _ = K6::avec_dossier(ExecuteurDouble::rendant(107, ""), travail.clone())
        .jouer("http://cible/", &reglage());
    assert!(contenu(&travail).is_empty(), "{:?}", contenu(&travail));
}

// ── @security ────────────────────────────────────────────────

#[test]
fn security_k6_ne_rapporte_pas_son_usage_a_grafana() {
    let double = Partage(Arc::new(ExecuteurDouble::rendant(0, SORTIE_K6)));
    K6::avec_dossier(double.clone(), dossier("usage"))
        .jouer("http://cible/", &reglage())
        .expect("palier");
    let arguments = double.0.appel_run().arguments;
    assert!(
        arguments.iter().any(|a| a == "--no-usage-report"),
        "sans ce drapeau, l'exécutant parlerait à Grafana : {arguments:?}"
    );
}

#[test]
fn security_k6_ne_recoit_ni_jeton_ni_cle_de_l_environnement_de_sluis() {
    let double = Partage(Arc::new(ExecuteurDouble::rendant(0, SORTIE_K6)));
    K6::avec_dossier(double.clone(), dossier("environnement"))
        .jouer("http://cible/", &reglage())
        .expect("palier");
    assert_eq!(
        double.0.appel_run().environnement.noms(),
        ["PATH", "HOME"],
        "k6 est un binaire tiers : il ne voit aucune variable de Sluis"
    );
}

#[test]
fn security_le_script_n_ecrase_jamais_un_fichier_existant() {
    let travail = dossier("existant");
    let chemin = travail.join("script.js");
    std::fs::write(&chemin, "export default function () {}").expect("piège");

    assert!(ecrire_script(&chemin).is_err());
    assert_eq!(
        std::fs::read_to_string(&chemin).expect("lecture"),
        "export default function () {}"
    );
}

#[test]
fn security_le_script_ne_suit_pas_un_lien_symbolique() {
    let travail = dossier("lien");
    let cible = travail.join("ailleurs.js");
    std::fs::write(&cible, "original").expect("cible");
    let lien = travail.join("script.js");
    std::os::unix::fs::symlink(&cible, &lien).expect("lien");

    assert!(ecrire_script(&lien).is_err());
    assert_eq!(
        std::fs::read_to_string(&cible).expect("lecture"),
        "original"
    );
}

#[test]
fn security_le_script_n_est_lisible_que_par_son_proprietaire() {
    use std::os::unix::fs::PermissionsExt;
    let chemin = dossier("mode").join("script.js");
    ecrire_script(&chemin).expect("écriture");
    let mode = std::fs::metadata(&chemin)
        .expect("méta")
        .permissions()
        .mode();
    assert_eq!(mode & 0o077, 0, "mode {mode:o}");
}
