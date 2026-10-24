//! Story 34.2 (ADR-034, point 4) : lancer `tofu` dans un environnement vidé,
//! sur un chemin canonisé.
//!
//! Deux niveaux de preuve. Le pilote est vérifié avec un exécuteur doublé, qui
//! reçoit l'environnement et le dossier : `tofu` n'est jamais lancé. La
//! commande réelle est vérifiée sans `tofu` non plus : `construire_commande`
//! la rend sans la lancer, et le test `@security` lance à sa place `env`, qui
//! affiche ce que le processus reçoit vraiment.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use sluis::application::ports::{
    EnvironnementIsole, Executeur, LecteurModule, MoteurHelm, MoteurIac, SortieProcessus,
};
use sluis::domain::{AppError, FichierModule, ModuleSoumis, ValeurSure};
use sluis::infrastructure::fs_module_tofu::FsModuleTofu;
use sluis::infrastructure::process::{
    construire_commande, environnement_tofu, Helm, Tofu, PATH_TOFU,
};

const NULL: &str = "registry.opentofu.org/hashicorp/null";
const RACINE: &str = "monosite/vps/dev";
const JETON: &str = "ghp_jeton_de_sluis_qui_ne_doit_pas_sortir";

// ── Doublures ────────────────────────────────────────────────

/// Un appel vu par l'exécuteur doublé.
#[derive(Debug, Clone)]
struct Appel {
    programme: String,
    isole: bool,
    dossier: Option<String>,
    environnement: Option<EnvironnementIsole>,
}

/// Exécuteur doublé : consigne chaque appel, rend le code décidé d'avance.
#[derive(Default)]
struct ExecuteurEspion {
    code: i32,
    appels: Mutex<Vec<Appel>>,
}

impl ExecuteurEspion {
    fn echouant() -> Self {
        Self {
            code: 1,
            appels: Mutex::new(Vec::new()),
        }
    }

    fn appels(&self) -> Vec<Appel> {
        self.appels.lock().map(|a| a.clone()).unwrap_or_default()
    }

    fn seul_appel(&self) -> Appel {
        let appels = self.appels();
        assert_eq!(appels.len(), 1, "{appels:?}");
        appels[0].clone()
    }

    fn consigner(&self, appel: Appel) -> SortieProcessus {
        if let Ok(mut appels) = self.appels.lock() {
            appels.push(appel);
        }
        SortieProcessus {
            code: self.code,
            sortie: "{\"info\":{\"status\":\"deployed\"},\"version\":1}".to_string(),
            erreur: "authentification refusée".to_string(),
        }
    }
}

impl Executeur for &ExecuteurEspion {
    fn executer(
        &self,
        programme: &str,
        _arguments: &[String],
        dossier: Option<&str>,
    ) -> Result<SortieProcessus, AppError> {
        Ok(self.consigner(Appel {
            programme: programme.to_string(),
            isole: false,
            dossier: dossier.map(str::to_string),
            environnement: None,
        }))
    }

    fn executer_isole(
        &self,
        programme: &str,
        _arguments: &[String],
        dossier: &str,
        environnement: &EnvironnementIsole,
    ) -> Result<SortieProcessus, AppError> {
        Ok(self.consigner(Appel {
            programme: programme.to_string(),
            isole: true,
            dossier: Some(dossier.to_string()),
            environnement: Some(environnement.clone()),
        }))
    }
}

/// Lecteur doublé : un module conforme, dont le dossier canonique est donné.
struct LecteurFixe;

const DOSSIER_FIXE: &str = "/depot/monosite/vps/dev";

impl LecteurModule for LecteurFixe {
    fn lire(&self, _module: &ValeurSure) -> Result<ModuleSoumis, AppError> {
        Ok(ModuleSoumis {
            chemin: RACINE.to_string(),
            dossier: DOSSIER_FIXE.to_string(),
            fichiers: vec![
                FichierModule {
                    chemin: format!("{RACINE}/.terraform.lock.hcl"),
                    contenu: format!("provider \"{NULL}\" {{\n  version = \"3.2.0\"\n}}\n"),
                },
                FichierModule {
                    chemin: format!("{RACINE}/main.tf"),
                    contenu: "resource \"null_resource\" \"x\" {}\n".to_string(),
                },
            ],
        })
    }
}

/// L'environnement de Sluis, tel que le voient ces tests : un jeton GitHub,
/// une variable quelconque, et deux variables de fournisseur dont une seule
/// est présente.
fn environnement_de_sluis(nom: &str) -> Option<String> {
    match nom {
        "SLUIS_GITHUB_TOKEN" => Some(JETON.to_string()),
        "VARIABLE_QUELCONQUE" => Some("quelconque".to_string()),
        "OVH_ENDPOINT" => Some("ovh-eu".to_string()),
        "PATH" => Some("/home/sluis/bin:/usr/bin".to_string()),
        _ => None,
    }
}

fn tofu<'a>(
    executeur: &'a ExecuteurEspion,
    variables: &[&str],
) -> Tofu<&'a ExecuteurEspion, LecteurFixe> {
    Tofu::new(executeur, LecteurFixe, vec![NULL.to_string()])
        .avec_variables_transmises(variables.iter().map(|v| v.to_string()).collect())
        .avec_lecture_environnement(environnement_de_sluis)
}

fn sure(valeur: &str) -> ValeurSure {
    ValeurSure::new(valeur).expect("valeur sûre")
}

fn noms(environnement: &EnvironnementIsole) -> Vec<String> {
    environnement
        .noms()
        .into_iter()
        .map(str::to_string)
        .collect()
}

/// Les variables posées sur une commande construite, et leur valeur.
fn variables_posees(commande: &std::process::Command) -> BTreeMap<String, Option<String>> {
    commande
        .get_envs()
        .map(|(nom, valeur)| {
            (
                nom.to_string_lossy().to_string(),
                valeur.map(|v| v.to_string_lossy().to_string()),
            )
        })
        .collect()
}

// ── Dépôt sur disque ─────────────────────────────────────────

/// Un dossier de travail propre à un test.
fn dossier_temporaire(nom: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!("sluis-env-{nom}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).expect("création");
    base
}

/// Écrit un module conforme dans `dossier`.
fn ecrire_module(dossier: &Path) {
    fs::create_dir_all(dossier).expect("création");
    fs::write(
        dossier.join(".terraform.lock.hcl"),
        format!("provider \"{NULL}\" {{\n  version = \"3.2.0\"\n}}\n"),
    )
    .expect("écriture");
    fs::write(
        dossier.join("main.tf"),
        "resource \"null_resource\" \"x\" {}\n",
    )
    .expect("écriture");
}

fn tofu_sur_disque<'a>(
    executeur: &'a ExecuteurEspion,
    racine: &Path,
) -> Tofu<&'a ExecuteurEspion, FsModuleTofu> {
    Tofu::new(
        executeur,
        FsModuleTofu::new(vec![racine.display().to_string()]),
        vec![NULL.to_string()],
    )
    .avec_lecture_environnement(environnement_de_sluis)
}

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_tofu_recoit_exactement_path_et_les_variables_listees() {
    let executeur = ExecuteurEspion::default();
    tofu(&executeur, &["OVH_ENDPOINT"])
        .plan(&sure("depot/monosite/vps/dev"))
        .expect("plan");

    let appel = executeur.seul_appel();
    assert_eq!(appel.programme, "tofu");
    assert!(appel.isole, "tofu doit être lancé isolé");
    let environnement = appel.environnement.expect("environnement");
    assert_eq!(
        environnement.variables,
        vec![
            ("PATH".to_string(), PATH_TOFU.to_string()),
            ("OVH_ENDPOINT".to_string(), "ovh-eu".to_string()),
        ]
    );
}

#[test]
fn happy_init_plan_output_destroy_lancent_tofu_isole_dans_le_dossier_canonique() {
    let executeur = ExecuteurEspion::default();
    let pilote = tofu(&executeur, &["OVH_ENDPOINT"]);
    let module = sure("depot/monosite/vps/dev");
    pilote.initialiser(&module).expect("init");
    pilote.plan(&module).expect("plan");
    pilote.sorties(&module).expect("output");
    pilote.detruire(&module).expect("destroy");

    let appels = executeur.appels();
    assert_eq!(appels.len(), 4);
    for appel in appels {
        assert!(appel.isole, "{appel:?}");
        assert_eq!(appel.dossier.as_deref(), Some(DOSSIER_FIXE));
        let environnement = appel.environnement.expect("environnement");
        assert_eq!(noms(&environnement), vec!["PATH", "OVH_ENDPOINT"]);
    }
}

#[test]
fn happy_la_commande_construite_ne_pose_que_path_et_les_variables_listees() {
    let (environnement, absentes) =
        environnement_tofu(&["OVH_ENDPOINT".to_string()], environnement_de_sluis);
    assert!(absentes.is_empty());
    let commande = construire_commande(
        "tofu",
        &["plan".to_string()],
        Some("/depot/monosite/vps/dev"),
        Some(&environnement),
    );

    let attendu: BTreeMap<String, Option<String>> = [
        ("OVH_ENDPOINT".to_string(), Some("ovh-eu".to_string())),
        ("PATH".to_string(), Some(PATH_TOFU.to_string())),
    ]
    .into_iter()
    .collect();
    assert_eq!(variables_posees(&commande), attendu);
    assert_eq!(
        commande.get_current_dir(),
        Some(Path::new("/depot/monosite/vps/dev"))
    );
}

#[test]
fn happy_les_autres_moteurs_gardent_l_environnement_herite() {
    let executeur = ExecuteurEspion::default();
    Helm::new(&executeur)
        .statut(&sure("sluis"), &sure("defaut"))
        .expect("statut");
    let appel = executeur.seul_appel();
    assert_eq!(appel.programme, "helm");
    assert!(!appel.isole, "helm ne change pas de comportement");

    let commande = construire_commande("helm", &["version".to_string()], None, None);
    assert!(variables_posees(&commande).is_empty());
}

#[cfg(unix)]
#[test]
fn happy_un_module_atteint_par_un_lien_interne_est_lance_dans_son_chemin_canonique() {
    let racine = dossier_temporaire("lien-interne");
    ecrire_module(&racine.join(RACINE));
    std::os::unix::fs::symlink(racine.join(RACINE), racine.join("raccourci")).expect("lien");

    let executeur = ExecuteurEspion::default();
    tofu_sur_disque(&executeur, &racine)
        .plan(&sure(&racine.join("raccourci").display().to_string()))
        .expect("plan");

    let canonique = racine.join(RACINE).canonicalize().expect("canonique");
    assert_eq!(
        executeur.seul_appel().dossier,
        Some(canonique.display().to_string()),
        "tofu est lancé dans le chemin canonique, pas dans le lien"
    );
    let _ = fs::remove_dir_all(racine);
}

// ── @negative ────────────────────────────────────────────────

#[cfg(unix)]
#[test]
fn negative_un_lien_symbolique_qui_sort_de_la_racine_n_est_pas_lance() {
    let racine = dossier_temporaire("lien-sortant");
    let dehors = dossier_temporaire("lien-sortant-dehors");
    ecrire_module(&dehors.join("module"));
    fs::create_dir_all(racine.join("monosite/vps")).expect("création");
    std::os::unix::fs::symlink(dehors.join("module"), racine.join(RACINE)).expect("lien");

    let executeur = ExecuteurEspion::default();
    let erreur = tofu_sur_disque(&executeur, &racine)
        .plan(&sure(&racine.join(RACINE).display().to_string()))
        .expect_err("lien sortant");

    let canonique = dehors
        .join("module")
        .canonicalize()
        .expect("canonique")
        .display()
        .to_string();
    match &erreur {
        AppError::CheminHorsRacine { chemin } => assert_eq!(chemin, &canonique),
        autre => panic!("CheminHorsRacine attendu, obtenu : {autre}"),
    }
    assert!(executeur.appels().is_empty(), "rien ne doit être lancé");
    let _ = fs::remove_dir_all(racine);
    let _ = fs::remove_dir_all(dehors);
}

#[test]
fn negative_un_chemin_contenant_point_point_ne_devient_jamais_un_module() {
    // Le module n'atteint le pilote que sous forme de `ValeurSure`, et une
    // `ValeurSure` ne contient jamais `..` : il n'existe aucun appel à
    // construire, donc rien à lancer.
    for chemin in [
        "depot/../dehors",
        "depot/monosite/vps/dev/../../../../etc",
        "../depot",
    ] {
        assert!(ValeurSure::new(chemin).is_err(), "{chemin}");
    }
}

#[test]
fn negative_un_module_sans_dossier_canonique_n_est_pas_lance() {
    struct LecteurSansDossier;
    impl LecteurModule for LecteurSansDossier {
        fn lire(&self, module: &ValeurSure) -> Result<ModuleSoumis, AppError> {
            let mut soumis = LecteurFixe.lire(module)?;
            soumis.dossier = String::new();
            Ok(soumis)
        }
    }
    let executeur = ExecuteurEspion::default();
    let pilote = Tofu::new(&executeur, LecteurSansDossier, vec![NULL.to_string()]);
    assert!(pilote.plan(&sure("depot/monosite/vps/dev")).is_err());
    assert!(executeur.appels().is_empty());
}

// ── @edge ────────────────────────────────────────────────────

#[test]
fn edge_une_variable_listee_mais_absente_n_est_pas_transmise_et_le_diagnostic_la_nomme() {
    let listees = [
        "OVH_ENDPOINT".to_string(),
        "OVH_APPLICATION_KEY".to_string(),
    ];
    let (environnement, absentes) = environnement_tofu(&listees, environnement_de_sluis);
    assert_eq!(noms(&environnement), vec!["PATH", "OVH_ENDPOINT"]);
    assert_eq!(absentes, vec!["OVH_APPLICATION_KEY".to_string()]);

    // Quand tofu échoue, l'erreur nomme la variable absente.
    let executeur = ExecuteurEspion::echouant();
    let erreur = tofu(&executeur, &["OVH_ENDPOINT", "OVH_APPLICATION_KEY"])
        .plan(&sure("depot/monosite/vps/dev"))
        .expect_err("tofu échoue");
    let message = erreur.to_string();
    assert!(message.contains("OVH_APPLICATION_KEY"), "{message}");
    assert!(message.contains("non transmise"), "{message}");
    let environnement = executeur.seul_appel().environnement.expect("env");
    assert_eq!(noms(&environnement), vec!["PATH", "OVH_ENDPOINT"]);
}

#[test]
fn edge_path_liste_doublons_et_noms_vides_sont_ecartes() {
    let listees = [
        "PATH".to_string(),
        " OVH_ENDPOINT ".to_string(),
        "OVH_ENDPOINT".to_string(),
        "   ".to_string(),
    ];
    let (environnement, absentes) = environnement_tofu(&listees, environnement_de_sluis);
    assert_eq!(
        environnement.variables,
        vec![
            ("PATH".to_string(), PATH_TOFU.to_string()),
            ("OVH_ENDPOINT".to_string(), "ovh-eu".to_string()),
        ],
        "le PATH de Sluis ne remplace jamais le PATH fixe"
    );
    assert!(absentes.is_empty());
}

#[test]
fn edge_sans_variable_listee_tofu_ne_recoit_que_path() {
    let (environnement, absentes) = environnement_tofu(&[], environnement_de_sluis);
    assert_eq!(noms(&environnement), vec!["PATH"]);
    assert!(absentes.is_empty());
}

#[cfg(unix)]
#[test]
fn edge_une_racine_lien_symbolique_est_canonisee_et_un_module_qui_en_descend_est_accepte() {
    let base = dossier_temporaire("racine-lien");
    let reelle = base.join("reelle");
    ecrire_module(&reelle.join(RACINE));
    let lien = base.join("racine");
    std::os::unix::fs::symlink(&reelle, &lien).expect("lien");
    let canonique = reelle
        .join(RACINE)
        .canonicalize()
        .expect("canonique")
        .display()
        .to_string();

    // Racine déclarée par son lien ; module donné par le lien, puis par le
    // chemin réel : les deux descendent de la racine une fois canonisée.
    for module in [lien.join(RACINE), reelle.join(RACINE)] {
        let executeur = ExecuteurEspion::default();
        tofu_sur_disque(&executeur, &lien)
            .plan(&sure(&module.display().to_string()))
            .expect("module sous la racine canonisée");
        assert_eq!(executeur.seul_appel().dossier.as_deref(), Some(&*canonique));
    }
    let _ = fs::remove_dir_all(base);
}

#[test]
fn edge_le_debug_de_l_environnement_ne_montre_pas_les_valeurs() {
    let (environnement, _) =
        environnement_tofu(&["SLUIS_GITHUB_TOKEN".to_string()], environnement_de_sluis);
    let trace = format!("{environnement:?}");
    assert!(trace.contains("SLUIS_GITHUB_TOKEN"), "{trace}");
    assert!(!trace.contains(JETON), "{trace}");
}

// ── @security ────────────────────────────────────────────────

#[test]
fn security_ni_le_jeton_ni_une_variable_non_listee_n_atteignent_tofu() {
    let executeur = ExecuteurEspion::default();
    tofu(&executeur, &["OVH_ENDPOINT"])
        .plan(&sure("depot/monosite/vps/dev"))
        .expect("plan");

    let environnement = executeur.seul_appel().environnement.expect("env");
    for interdite in ["SLUIS_GITHUB_TOKEN", "VARIABLE_QUELCONQUE"] {
        assert!(
            !environnement.noms().contains(&interdite),
            "{interdite} transmise : {environnement:?}"
        );
    }
    assert!(!environnement
        .variables
        .iter()
        .any(|(_, valeur)| valeur == JETON));
}

/// Preuve sur un vrai processus : `env` est lancé avec la commande que
/// construit `construire_commande`, et affiche l'environnement qu'il a reçu.
///
/// `cargo test` donne au processus de test des variables `CARGO_*` que
/// personne n'a listées : si l'effacement manque, elles traversent, et ce
/// test tombe. C'est le témoin de la story.
#[cfg(unix)]
#[test]
fn security_le_processus_lance_ne_recoit_que_l_environnement_construit() {
    assert!(
        std::env::var("CARGO_MANIFEST_DIR").is_ok(),
        "précondition : le processus de test hérite de variables non listées"
    );
    let (environnement, _) =
        environnement_tofu(&["OVH_ENDPOINT".to_string()], environnement_de_sluis);
    let sortie = construire_commande("env", &[], None, Some(&environnement))
        .output()
        .expect("env lancé");
    assert!(sortie.status.success());

    let mut recues: Vec<(String, String)> = String::from_utf8_lossy(&sortie.stdout)
        .lines()
        .filter_map(|ligne| ligne.split_once('='))
        .map(|(nom, valeur)| (nom.to_string(), valeur.to_string()))
        .collect();
    recues.sort();
    // En cas d'échec, seuls les noms sont affichés : les valeurs héritées
    // peuvent être des secrets de la machine qui joue les tests.
    let noms_recus: Vec<&str> = recues.iter().map(|(nom, _)| nom.as_str()).collect();
    assert_eq!(
        noms_recus,
        vec!["OVH_ENDPOINT", "PATH"],
        "le processus a reçu autre chose que l'environnement construit"
    );
    assert_eq!(
        recues,
        vec![
            ("OVH_ENDPOINT".to_string(), "ovh-eu".to_string()),
            ("PATH".to_string(), PATH_TOFU.to_string()),
        ]
    );
}

// ── Configuration ────────────────────────────────────────────

fn charger_configuration(contenu: &str, nom: &str) -> sluis::configuration::Configuration {
    let chemin = std::env::temp_dir().join(format!(
        "sluis-env-config-{nom}-{}.toml",
        std::process::id()
    ));
    fs::write(&chemin, contenu).expect("écriture");
    let configuration = sluis::configuration::Configuration::charger_avec(Some(&chemin), |_| None)
        .expect("chargement");
    let _ = fs::remove_file(&chemin);
    configuration
}

#[test]
fn happy_variables_transmises_se_lit_dans_la_section_tofu() {
    let configuration = charger_configuration(
        "[tofu]\nvariables_transmises = [\"OVH_ENDPOINT\"]\n",
        "liste",
    );
    assert_eq!(
        configuration.fichier.tofu.variables_transmises,
        vec!["OVH_ENDPOINT".to_string()]
    );
}

#[test]
fn edge_variables_transmises_est_vide_par_defaut_et_dans_l_executant() {
    let configuration = charger_configuration("[tofu]\n", "defaut");
    assert!(configuration.fichier.tofu.variables_transmises.is_empty());
    let executant =
        charger_configuration(include_str!("../config/sluis.executant.toml"), "executant");
    assert!(executant.fichier.tofu.variables_transmises.is_empty());
}
