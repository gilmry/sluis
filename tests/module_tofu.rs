//! Story 34.1 (ADR-034, point 3) : vérifier un module OpenTofu avant de le
//! lancer.
//!
//! Un fichier `.tf` est un programme. Ces tests prouvent que Sluis refuse de
//! lancer `tofu` sur un module qui sort du contrat, et que le refus précède
//! toujours le processus : l'exécuteur doublé compte ses appels.

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use sluis::application::ports::{Executeur, LecteurModule, MoteurIac, SortieProcessus};
use sluis::domain::{
    verifier_module, Action, AppError, BailBacASable, Construction, DemandeBail, Duree,
    Environnement, FenetreDerogation, FichierModule, Horodatage, JetonChangement,
    ListeAutorisation, ModuleSoumis, PlafondDepense, PlanChangement, Tier, ValeurSure, Violation,
};
use sluis::infrastructure::fs_module_tofu::FsModuleTofu;
use sluis::infrastructure::process::Tofu;

const AWS: &str = "registry.opentofu.org/hashicorp/aws";
const NULL: &str = "registry.opentofu.org/hashicorp/null";
const RACINE: &str = "monosite/vps/dev";

// ── Doublures ────────────────────────────────────────────────

/// Exécuteur doublé : compte ses appels et rend un succès.
#[derive(Default)]
struct ExecuteurCompteur {
    appels: Mutex<Vec<Vec<String>>>,
}

impl ExecuteurCompteur {
    fn nombre(&self) -> usize {
        self.appels.lock().map(|a| a.len()).unwrap_or(usize::MAX)
    }
}

impl Executeur for &ExecuteurCompteur {
    fn executer(
        &self,
        _programme: &str,
        arguments: &[String],
        _dossier: Option<&str>,
    ) -> Result<SortieProcessus, AppError> {
        if let Ok(mut appels) = self.appels.lock() {
            appels.push(arguments.to_vec());
        }
        Ok(SortieProcessus {
            code: 0,
            sortie: "{}".to_string(),
            erreur: String::new(),
        })
    }
}

/// Lecteur doublé : rend le module qu'on lui donne.
struct LecteurFixe(ModuleSoumis);

impl LecteurModule for LecteurFixe {
    fn lire(&self, _module: &ValeurSure) -> Result<ModuleSoumis, AppError> {
        Ok(self.0.clone())
    }
}

// ── Fabriques ────────────────────────────────────────────────

fn fichier(chemin: &str, contenu: &str) -> FichierModule {
    FichierModule {
        chemin: chemin.to_string(),
        contenu: contenu.to_string(),
    }
}

fn verrou(fournisseurs: &[&str]) -> FichierModule {
    let contenu: String = fournisseurs
        .iter()
        .map(|f| {
            format!(
                "provider \"{f}\" {{\n  version     = \"5.0.0\"\n  hashes = [\"h1:abc=\"]\n}}\n\n"
            )
        })
        .collect();
    fichier(&format!("{RACINE}/.terraform.lock.hcl"), &contenu)
}

const MAIN_CONFORME: &str = r#"terraform {
  required_providers {
    aws = { source = "hashicorp/aws" }
  }
}

module "vps" {
  source = "../../../_shared/terraform/modules/vps"
  taille = var.taille
}

output "adresse" {
  value = module.vps.adresse
}
"#;

/// Un module conforme : verrou, fournisseur autorisé, module partagé local.
fn module_conforme() -> ModuleSoumis {
    ModuleSoumis {
        chemin: RACINE.to_string(),
        fichiers: vec![
            verrou(&[AWS]),
            fichier(&format!("{RACINE}/main.tf"), MAIN_CONFORME),
            fichier(
                "_shared/terraform/modules/vps/main.tf",
                "resource \"aws_instance\" \"vps\" {\n  ami = \"ami-1\"\n}\n",
            ),
        ],
    }
}

/// Le module conforme, augmenté d'un fichier.
fn module_avec(chemin: &str, contenu: &str) -> ModuleSoumis {
    let mut module = module_conforme();
    module.fichiers.push(fichier(chemin, contenu));
    module
}

fn autorises() -> Vec<String> {
    vec![AWS.to_string()]
}

fn verifier(module: &ModuleSoumis) -> Vec<Violation> {
    verifier_module(module, &autorises())
}

fn constructions(module: &ModuleSoumis) -> Vec<Construction> {
    verifier(module)
        .into_iter()
        .map(|v| v.construction)
        .collect()
}

fn sure(valeur: &str) -> ValeurSure {
    ValeurSure::new(valeur).expect("valeur sûre")
}

const MAINTENANT: Horodatage = Horodatage::new(1_000_000);

/// Un bail vivant, obtenu par tout le chemin d'approbation de Tier 1.
fn bail() -> BailBacASable {
    let plan = PlanChangement::new(
        Action::RenouvellementDerogation,
        Environnement::Production,
        Tier::One,
        "derogation-bac-a-sable".to_string(),
        "ouvre une fenêtre de 90 jours".to_string(),
        String::new(),
    )
    .expect("plan");
    let approbation = JetonChangement::emettre(
        plan.empreinte().clone(),
        "superviseur".to_string(),
        MAINTENANT,
        Duree::secondes(600).expect("durée"),
    )
    .expect("émission")
    .consommer(&plan, MAINTENANT)
    .expect("consommation");
    let fenetre =
        FenetreDerogation::ouvrir(&approbation, MAINTENANT, Duree::jours(90).expect("durée"))
            .expect("fenêtre");
    let derogation = fenetre.valider(MAINTENANT).expect("dérogation");
    let liste = ListeAutorisation::new(Vec::new(), vec!["bac".to_string()]).expect("liste");
    BailBacASable::louer(
        &derogation,
        DemandeBail {
            projet: liste.projet_bac_a_sable("bac").expect("projet"),
            module: sure("depots/projet/infra"),
            ttl: Duree::secondes(3_600).expect("ttl"),
            plafond: PlafondDepense::new(20.0).expect("plafond"),
            estimation_depense: 4.0,
        },
        Duree::secondes(21_600).expect("ttl max"),
        MAINTENANT,
    )
    .expect("bail")
}

/// Les cinq opérations de l'adaptateur, chacune réduite à son verdict.
fn cinq_operations<E: Executeur, L: LecteurModule>(
    tofu: &Tofu<E, L>,
) -> Vec<(&'static str, Result<(), AppError>)> {
    let module = sure("depots/projet/infra");
    vec![
        ("init", tofu.initialiser(&module)),
        ("plan", tofu.plan(&module).map(|_| ())),
        ("apply", tofu.appliquer(&module, &bail()).map(|_| ())),
        ("destroy", tofu.detruire(&module).map(|_| ())),
        ("output", tofu.sorties(&module).map(|_| ())),
    ]
}

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_un_module_conforme_n_a_aucune_violation() {
    assert_eq!(verifier(&module_conforme()), Vec::new());
}

#[test]
fn happy_un_module_conforme_lance_tofu_comme_aujourd_hui() {
    let executeur = ExecuteurCompteur::default();
    let tofu = Tofu::new(&executeur, LecteurFixe(module_conforme()), autorises());
    tofu.plan(&sure("depots/projet/infra")).expect("plan");

    let appels = executeur.appels.lock().expect("verrou").clone();
    assert_eq!(appels.len(), 1);
    assert_eq!(appels[0].first().map(String::as_str), Some("plan"));
}

#[test]
fn happy_les_cinq_operations_passent_sur_un_module_conforme() {
    let executeur = ExecuteurCompteur::default();
    let tofu = Tofu::new(&executeur, LecteurFixe(module_conforme()), autorises());
    for (operation, verdict) in cinq_operations(&tofu) {
        assert!(verdict.is_ok(), "{operation} : {verdict:?}");
    }
    assert_eq!(executeur.nombre(), 5);
}

#[test]
fn happy_init_garde_le_verrou_en_lecture_seule() {
    let executeur = ExecuteurCompteur::default();
    let tofu = Tofu::new(&executeur, LecteurFixe(module_conforme()), autorises());
    tofu.initialiser(&sure("depots/projet/infra"))
        .expect("init");
    let appels = executeur.appels.lock().expect("verrou").clone();
    assert!(appels[0].contains(&"-lockfile=readonly".to_string()));
}

#[test]
fn happy_le_lecteur_de_fichiers_rend_le_module_et_les_modules_partages() {
    let racine = depot_temporaire("lecture");
    let module = racine.join(RACINE);
    let soumis = FsModuleTofu::new(vec![racine.display().to_string()])
        .lire(&sure(&module.display().to_string()))
        .expect("lecture");

    assert_eq!(soumis.chemin, RACINE);
    let chemins: Vec<&str> = soumis.fichiers.iter().map(|f| f.chemin.as_str()).collect();
    assert!(chemins.contains(&"monosite/vps/dev/main.tf"), "{chemins:?}");
    assert!(chemins.contains(&"monosite/vps/dev/.terraform.lock.hcl"));
    assert!(chemins.contains(&"_shared/terraform/modules/vps/main.tf"));
    assert!(
        !chemins.iter().any(|c| c.contains(".terraform/")),
        "le cache de tofu n'est pas lu : {chemins:?}"
    );
    assert_eq!(verifier(&soumis), Vec::new());
    let _ = fs::remove_dir_all(racine);
}

// ── @negative ────────────────────────────────────────────────

const INTERDITS: &[(&str, &str, &str)] = &[
    (
        "provisioner \"local-exec\"",
        "resource \"null_resource\" \"x\" {\n  provisioner \"local-exec\" {\n    command = \"id\"\n  }\n}\n",
        "local-exec",
    ),
    (
        "provisioner \"remote-exec\"",
        "resource \"null_resource\" \"x\" {\n  provisioner \"remote-exec\" {\n    inline = [\"id\"]\n  }\n}\n",
        "remote-exec",
    ),
    (
        "data \"external\"",
        "variable \"a\" {}\n\ndata \"external\" \"x\" {\n  program = [\"id\"]\n}\n",
        "external",
    ),
];

#[test]
fn negative_data_external_est_refuse_avec_fichier_et_ligne() {
    let violations = verifier(&module_avec("monosite/vps/dev/donnees.tf", INTERDITS[2].1));
    assert_eq!(
        violations,
        vec![Violation {
            fichier: "monosite/vps/dev/donnees.tf".to_string(),
            ligne: Some(3),
            construction: Construction::DataExternal,
        }]
    );
}

#[test]
fn negative_local_exec_et_remote_exec_sont_refuses_avec_fichier_et_ligne() {
    for (construction, contenu, _) in &INTERDITS[..2] {
        let violations = verifier(&module_avec("monosite/vps/dev/x.tf", contenu));
        assert_eq!(violations.len(), 1, "{construction} : {violations:?}");
        assert_eq!(violations[0].fichier, "monosite/vps/dev/x.tf");
        assert_eq!(violations[0].ligne, Some(2), "{construction}");
    }
}

#[test]
fn negative_aucune_des_cinq_operations_ne_lance_tofu_et_l_erreur_situe_la_construction() {
    for (construction, contenu, etiquette) in INTERDITS {
        let executeur = ExecuteurCompteur::default();
        let tofu = Tofu::new(
            &executeur,
            LecteurFixe(module_avec("monosite/vps/dev/piege.tf", contenu)),
            autorises(),
        );
        for (operation, verdict) in cinq_operations(&tofu) {
            let erreur = verdict.expect_err(operation).to_string();
            assert!(
                erreur.contains("monosite/vps/dev/piege.tf:"),
                "{construction} / {operation} : le fichier et la ligne manquent : {erreur}"
            );
            assert!(
                erreur.contains(etiquette),
                "{construction} / {operation} : la construction n'est pas nommée : {erreur}"
            );
        }
        assert_eq!(executeur.nombre(), 0, "{construction} : tofu a été lancé");
    }
}

// ── @edge ────────────────────────────────────────────────────

fn module_appelant(source: &str) -> ModuleSoumis {
    module_avec(
        "monosite/vps/dev/appel.tf",
        &format!("module \"autre\" {{\n  source  = \"{source}\"\n  version = \"1.0.0\"\n}}\n"),
    )
}

#[test]
fn edge_une_source_de_module_distante_est_refusee() {
    for source in [
        "git::https://example.org/modules/vps.git?ref=v1",
        "https://example.org/vps.zip",
        "hashicorp/consul/aws",
        "registry.opentofu.org/hashicorp/consul/aws",
        "github.com/exemple/vps",
        "s3::https://s3.amazonaws.com/bucket/vps.zip",
        "/opt/modules/vps",
    ] {
        assert_eq!(
            constructions(&module_appelant(source)),
            vec![Construction::SourceDistante(source.to_string())],
            "{source}"
        );
    }
}

#[test]
fn edge_une_source_locale_qui_sort_de_la_racine_est_refusee() {
    let source = "../../../../ailleurs/vps";
    assert_eq!(
        constructions(&module_appelant(source)),
        vec![Construction::SourceHorsRacine(source.to_string())]
    );
}

#[test]
fn edge_une_source_locale_hors_des_modules_partages_est_refusee() {
    for source in [
        "./sous-module",
        "../../k3s/dev",
        "../../../_shared/terraform",
    ] {
        assert_eq!(
            constructions(&module_appelant(source)),
            vec![Construction::SourceHorsModulesPartages(source.to_string())],
            "{source}"
        );
    }
}

#[test]
fn edge_une_source_non_litterale_est_refusee() {
    let module = module_avec(
        "monosite/vps/dev/appel.tf",
        "module \"autre\" {\n  source = var.depot\n}\n",
    );
    assert_eq!(constructions(&module), vec![Construction::SourceIllisible]);
}

#[test]
fn edge_un_module_partage_qui_appelle_un_module_distant_est_refuse() {
    let module = module_avec(
        "_shared/terraform/modules/vps/reseau.tf",
        "module \"reseau\" { source = \"git::https://example.org/r.git\" }\n",
    );
    assert_eq!(
        constructions(&module),
        vec![Construction::SourceDistante(
            "git::https://example.org/r.git".to_string()
        )]
    );
}

#[test]
fn edge_un_module_sans_verrou_est_refuse() {
    let mut module = module_conforme();
    module
        .fichiers
        .retain(|f| !f.chemin.ends_with(".terraform.lock.hcl"));
    assert_eq!(
        verifier(&module),
        vec![Violation {
            fichier: "monosite/vps/dev/.terraform.lock.hcl".to_string(),
            ligne: None,
            construction: Construction::VerrouAbsent,
        }]
    );
}

#[test]
fn edge_un_verrou_ailleurs_qu_a_la_racine_du_module_ne_compte_pas() {
    let mut module = module_conforme();
    module
        .fichiers
        .retain(|f| !f.chemin.ends_with(".terraform.lock.hcl"));
    module.fichiers.push(fichier(
        "_shared/terraform/modules/vps/.terraform.lock.hcl",
        &format!("provider \"{AWS}\" {{}}\n"),
    ));
    assert_eq!(constructions(&module), vec![Construction::VerrouAbsent]);
}

#[test]
fn edge_un_fournisseur_hors_liste_blanche_est_refuse() {
    let mut module = module_conforme();
    module.fichiers[0] = verrou(&[AWS, NULL]);
    assert_eq!(
        verifier(&module),
        vec![Violation {
            fichier: "monosite/vps/dev/.terraform.lock.hcl".to_string(),
            ligne: Some(6),
            construction: Construction::FournisseurHorsListe(NULL.to_string()),
        }]
    );
}

#[test]
fn edge_une_liste_blanche_vide_refuse_tout() {
    let violations = verifier_module(&module_conforme(), &[]);
    let constructions: Vec<Construction> = violations.into_iter().map(|v| v.construction).collect();
    assert!(constructions.contains(&Construction::ListeBlancheVide));
    assert!(constructions.contains(&Construction::FournisseurHorsListe(AWS.to_string())));
}

#[test]
fn edge_une_liste_blanche_de_blancs_vaut_une_liste_vide() {
    let constructions: Vec<Construction> = verifier_module(&module_conforme(), &["  ".to_string()])
        .into_iter()
        .map(|v| v.construction)
        .collect();
    assert!(constructions.contains(&Construction::ListeBlancheVide));
}

#[test]
fn edge_une_construction_sur_plusieurs_lignes_avec_des_blancs_inhabituels_est_refusee() {
    for (contenu, attendue) in [
        (
            "resource \"null_resource\" \"x\" {\n  provisioner\n\n    \"local-exec\" {}\n}\n",
            Construction::ProvisionerLocalExec,
        ),
        (
            "data\t \u{a0}\r\n\t\"external\"   \"x\" {}\n",
            Construction::DataExternal,
        ),
        (
            "resource \"x\" \"y\" { provisioner /* glissé */ \"remote-exec\" {} }\n",
            Construction::ProvisionerRemoteExec,
        ),
        (
            "data # commentaire de fin de ligne\n \"external\" \"x\" {}\n",
            Construction::DataExternal,
        ),
        (
            "resource \"x\" \"y\" {\n  provisioner local-exec {}\n}\n",
            Construction::ProvisionerLocalExec,
        ),
    ] {
        assert_eq!(
            constructions(&module_avec("monosite/vps/dev/x.tf", contenu)),
            vec![attendue],
            "{}",
            contenu.escape_default()
        );
    }
}

#[test]
fn edge_une_construction_en_commentaire_est_refusee_faux_positif_assume() {
    for contenu in [
        "# data \"external\" \"x\" {}\n",
        "// provisioner \"local-exec\" {}\n",
        "/*\n  provisioner \"remote-exec\" {}\n*/\n",
    ] {
        let violations = verifier(&module_avec("monosite/vps/dev/x.tf", contenu));
        assert_eq!(violations.len(), 1, "{}", contenu.escape_default());
    }
}

#[test]
fn edge_une_etiquette_avec_sequence_d_echappement_est_refusee() {
    let contenu = "resource \"x\" \"y\" {\n  provisioner \"local\\u002dexec\" {}\n}\n";
    assert_eq!(
        constructions(&module_avec("monosite/vps/dev/x.tf", contenu)),
        vec![Construction::EtiquetteNonLitterale(
            "local\\u002dexec".to_string()
        )]
    );
}

#[test]
fn edge_un_fichier_tf_json_est_refuse() {
    for chemin in [
        "monosite/vps/dev/main.tf.json",
        "monosite/vps/dev/x.tofu.json",
    ] {
        assert_eq!(
            verifier(&module_avec(chemin, "{}")),
            vec![Violation {
                fichier: chemin.to_string(),
                ligne: None,
                construction: Construction::FichierJson,
            }]
        );
    }
}

#[test]
fn edge_un_fichier_tofu_est_verifie_comme_un_fichier_tf() {
    assert_eq!(
        constructions(&module_avec(
            "monosite/vps/dev/main.tofu",
            "data \"external\" \"x\" {}\n"
        )),
        vec![Construction::DataExternal]
    );
}

#[test]
fn edge_une_reference_ou_un_mot_voisin_n_est_pas_une_construction() {
    let contenu = "locals {\n  a = data.external_bis.x\n  b = module.vps.adresse\n  \
                   metadata = \"external\"\n  provisioners = 2\n}\n\
                   data \"external_ip\" \"x\" {}\n";
    assert_eq!(
        verifier(&module_avec("monosite/vps/dev/x.tf", contenu)),
        Vec::new()
    );
}

#[test]
fn edge_chaque_cas_a_un_motif_distinct() {
    let cas = [
        Construction::ProvisionerLocalExec,
        Construction::ProvisionerRemoteExec,
        Construction::DataExternal,
        Construction::EtiquetteNonLitterale("x".to_string()),
        Construction::FichierJson,
        Construction::VerrouAbsent,
        Construction::FournisseurHorsListe("x".to_string()),
        Construction::FournisseurIllisible("x".to_string()),
        Construction::ListeBlancheVide,
        Construction::SourceDistante("x".to_string()),
        Construction::SourceHorsRacine("x".to_string()),
        Construction::SourceHorsModulesPartages("x".to_string()),
        Construction::SourceIllisible,
    ];
    let motifs: Vec<String> = cas.iter().map(ToString::to_string).collect();
    for (i, motif) in motifs.iter().enumerate() {
        assert!(!motif.trim().is_empty());
        assert!(
            !motifs[i + 1..].contains(motif),
            "motif en double : {motif}"
        );
    }
}

// ── @security ────────────────────────────────────────────────

#[test]
fn security_aucune_operation_n_appelle_l_executeur_sur_un_module_non_conforme() {
    let mut sans_verrou = module_conforme();
    sans_verrou
        .fichiers
        .retain(|f| !f.chemin.ends_with(".terraform.lock.hcl"));
    let non_conformes = [
        (
            "local-exec",
            module_avec("monosite/vps/dev/x.tf", INTERDITS[0].1),
        ),
        (
            "remote-exec",
            module_avec("monosite/vps/dev/x.tf", INTERDITS[1].1),
        ),
        (
            "data external",
            module_avec("monosite/vps/dev/x.tf", INTERDITS[2].1),
        ),
        (
            "source distante",
            module_appelant("git::https://example.org/m.git"),
        ),
        ("verrou absent", sans_verrou),
        (
            "fichier json",
            module_avec("monosite/vps/dev/main.tf.json", "{}"),
        ),
    ];
    for (cas, module) in non_conformes {
        let executeur = ExecuteurCompteur::default();
        let tofu = Tofu::new(&executeur, LecteurFixe(module), autorises());
        for (operation, verdict) in cinq_operations(&tofu) {
            assert!(
                matches!(verdict, Err(AppError::ModuleRefuse { .. })),
                "{cas} / {operation} : {verdict:?}"
            );
        }
        assert_eq!(executeur.nombre(), 0, "{cas} : l'exécuteur a été appelé");
    }
}

#[test]
fn security_une_liste_blanche_vide_ne_lance_aucune_operation() {
    let executeur = ExecuteurCompteur::default();
    let tofu = Tofu::new(&executeur, LecteurFixe(module_conforme()), Vec::new());
    for (operation, verdict) in cinq_operations(&tofu) {
        assert!(verdict.is_err(), "{operation}");
    }
    assert_eq!(executeur.nombre(), 0);
}

#[test]
fn security_un_module_hors_des_racines_autorisees_n_est_ni_lu_ni_lance() {
    let racine = depot_temporaire("hors-racine");
    let autre = depot_temporaire("autre-racine");
    let executeur = ExecuteurCompteur::default();
    let tofu = Tofu::new(
        &executeur,
        FsModuleTofu::new(vec![autre.display().to_string()]),
        autorises(),
    );
    let erreur = tofu
        .plan(&sure(&racine.join(RACINE).display().to_string()))
        .expect_err("hors racine");
    assert!(
        matches!(erreur, AppError::CheminHorsRacine { .. }),
        "{erreur}"
    );
    assert_eq!(executeur.nombre(), 0);
    let _ = fs::remove_dir_all(racine);
    let _ = fs::remove_dir_all(autre);
}

#[cfg(unix)]
#[test]
fn security_un_lien_symbolique_qui_sort_de_la_racine_fait_refuser_le_module() {
    let racine = depot_temporaire("lien");
    let dehors = std::env::temp_dir().join(format!("sluis-tofu-dehors-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dehors);
    fs::create_dir_all(&dehors).expect("création");
    fs::write(dehors.join("piege.tf"), "data \"external\" \"x\" {}\n").expect("écriture");
    std::os::unix::fs::symlink(
        dehors.join("piege.tf"),
        racine.join(RACINE).join("piege.tf"),
    )
    .expect("lien");

    let executeur = ExecuteurCompteur::default();
    let tofu = Tofu::new(
        &executeur,
        FsModuleTofu::new(vec![racine.display().to_string()]),
        autorises(),
    );
    let erreur = tofu
        .plan(&sure(&racine.join(RACINE).display().to_string()))
        .expect_err("lien sortant");
    assert!(
        matches!(erreur, AppError::CheminHorsRacine { .. }),
        "{erreur}"
    );
    assert_eq!(executeur.nombre(), 0);
    let _ = fs::remove_dir_all(racine);
    let _ = fs::remove_dir_all(dehors);
}

#[test]
fn security_un_module_partage_piege_sur_disque_fait_refuser_le_module_racine() {
    let racine = depot_temporaire("partage-piege");
    fs::write(
        racine.join("_shared/terraform/modules/vps/piege.tf"),
        "data \"external\" \"x\" {}\n",
    )
    .expect("écriture");
    let executeur = ExecuteurCompteur::default();
    let tofu = Tofu::new(
        &executeur,
        FsModuleTofu::new(vec![racine.display().to_string()]),
        autorises(),
    );
    let erreur = tofu
        .plan(&sure(&racine.join(RACINE).display().to_string()))
        .expect_err("module partagé piégé");
    assert!(
        erreur
            .to_string()
            .contains("_shared/terraform/modules/vps/piege.tf:1"),
        "{erreur}"
    );
    assert_eq!(executeur.nombre(), 0);
    let _ = fs::remove_dir_all(racine);
}

// ── Dépôt sur disque ─────────────────────────────────────────

/// Un dépôt conforme sur disque, avec un cache `.terraform/` qui ne doit pas
/// être lu.
fn depot_temporaire(nom: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!("sluis-tofu-{nom}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    let module = base.join(RACINE);
    let partage = base.join("_shared/terraform/modules/vps");
    fs::create_dir_all(&module).expect("création");
    fs::create_dir_all(&partage).expect("création");
    fs::create_dir_all(module.join(".terraform/modules/x")).expect("création");
    fs::write(module.join("main.tf"), MAIN_CONFORME).expect("écriture");
    fs::write(
        module.join(".terraform.lock.hcl"),
        format!("provider \"{AWS}\" {{\n  version = \"5.0.0\"\n}}\n"),
    )
    .expect("écriture");
    fs::write(
        module.join(".terraform/modules/x/main.tf"),
        "data \"external\" \"cache\" {}\n",
    )
    .expect("écriture");
    fs::write(module.join("README.md"), "# module\n").expect("écriture");
    fs::write(
        partage.join("main.tf"),
        "resource \"aws_instance\" \"vps\" {}\n",
    )
    .expect("écriture");
    base
}
