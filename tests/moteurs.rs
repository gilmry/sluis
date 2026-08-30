//! Stories 3.4 et 3.5 — runners sans shell, plans et statuts.
//!
//! Aucun test ne lance de binaire réel : les six moteurs sont absents de la
//! machine de développement, et NFR-06 impose que la suite n'en dépende pas.
//! L'exécuteur est doublé, ce qui rend aussi les cas d'échec reproductibles.

use std::sync::Mutex;

use sluis::application::ports::{
    Executeur, MoteurArgocd, MoteurHelm, MoteurKustomize, MoteurTerraform, SortieProcessus,
};
use sluis::domain::{AppError, ValeurSure};
use sluis::infrastructure::process::{
    analyser_resume_plan, masquer_secrets, Argocd, ExecuteurSysteme, Helm, Kustomize, Terraform,
};

/// Le module que porte un bail de test.
fn module_du_bail() -> sluis::domain::ValeurSure {
    sluis::domain::ValeurSure::new("depots/projet/infra/bac-a-sable").expect("module")
}

/// Exécuteur doublé : consigne les appels, rend une sortie décidée d'avance.
struct ExecuteurDouble {
    reponse: Result<SortieProcessus, AppError>,
    appels: Mutex<Vec<(String, Vec<String>)>>,
}

impl ExecuteurDouble {
    fn rendant(code: i32, sortie: &str) -> Self {
        Self {
            reponse: Ok(SortieProcessus {
                code,
                sortie: sortie.to_string(),
                erreur: String::new(),
            }),
            appels: Mutex::new(Vec::new()),
        }
    }

    fn absent(binaire: &str) -> Self {
        Self {
            reponse: Err(AppError::EngineMissing {
                binaire: binaire.to_string(),
            }),
            appels: Mutex::new(Vec::new()),
        }
    }

    fn dernier_appel(&self) -> Option<(String, Vec<String>)> {
        self.appels.lock().ok()?.last().cloned()
    }
}

impl Executeur for ExecuteurDouble {
    fn executer(
        &self,
        programme: &str,
        arguments: &[String],
        _dossier: Option<&str>,
    ) -> Result<SortieProcessus, AppError> {
        if let Ok(mut appels) = self.appels.lock() {
            appels.push((programme.to_string(), arguments.to_vec()));
        }
        match &self.reponse {
            Ok(s) => Ok(s.clone()),
            Err(AppError::EngineMissing { binaire }) => Err(AppError::EngineMissing {
                binaire: binaire.clone(),
            }),
            Err(_) => Err(AppError::Configuration {
                detail: "double".to_string(),
            }),
        }
    }
}

fn sure(valeur: &str) -> ValeurSure {
    ValeurSure::new(valeur).expect("valeur sûre")
}

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_un_plan_terraform_est_resume_correctement() {
    let double = ExecuteurDouble::rendant(
        0,
        "Terraform will perform the following actions:\n\n\
         Plan: 3 to add, 1 to change, 2 to destroy.\n",
    );
    let plan = Terraform::new(double)
        .plan(&sure("infra/prod"))
        .expect("plan");
    assert_eq!(plan.creations, 3);
    assert_eq!(plan.modifications, 1);
    assert_eq!(plan.destructions, 2);
    assert!(!plan.sans_changement());
}

#[test]
fn happy_un_statut_helm_est_lu() {
    let double = ExecuteurDouble::rendant(0, r#"{"info":{"status":"deployed"},"version":7}"#);
    let statut = Helm::new(double)
        .statut(&sure("koprogo"), &sure("production"))
        .expect("statut");
    assert_eq!(statut.statut, "deployed");
    assert_eq!(statut.revision, 7);
}

#[test]
fn happy_un_statut_argocd_est_lu() {
    let double = ExecuteurDouble::rendant(
        0,
        r#"{"status":{"sync":{"status":"Synced"},"health":{"status":"Healthy"}}}"#,
    );
    let statut = Argocd::new(double)
        .statut_application(&sure("koprogo-prod"))
        .expect("statut");
    assert_eq!(statut.synchronisation, "Synced");
    assert_eq!(statut.sante, "Healthy");
}

// ── @negative ────────────────────────────────────────────────

#[test]
fn negative_un_binaire_absent_nomme_le_binaire() {
    let erreur = Terraform::new(ExecuteurDouble::absent("terraform"))
        .plan(&sure("infra"))
        .unwrap_err();
    assert!(
        erreur.to_string().contains("terraform"),
        "obtenu : {erreur}"
    );
    assert!(
        !erreur.to_string().contains("panic"),
        "l'absence est un état normal, pas une panne"
    );
}

#[test]
fn negative_un_code_de_retour_non_nul_produit_une_erreur() {
    let erreur = Terraform::new(ExecuteurDouble::rendant(1, ""))
        .plan(&sure("infra"))
        .unwrap_err();
    assert!(erreur.to_string().contains("terraform"));
}

#[test]
fn negative_une_release_inconnue_produit_une_erreur_typee() {
    let erreur = Helm::new(ExecuteurDouble::rendant(1, ""))
        .statut(&sure("absente"), &sure("defaut"))
        .unwrap_err();
    assert!(erreur.to_string().contains("absente"));
}

#[test]
fn negative_une_sortie_json_illisible_produit_une_erreur_d_analyse() {
    let erreur = Helm::new(ExecuteurDouble::rendant(0, "ceci n'est pas du json"))
        .statut(&sure("r"), &sure("n"))
        .unwrap_err();
    assert!(erreur.to_string().contains("statut helm"));
}

// ── @edge ────────────────────────────────────────────────────

#[test]
fn edge_un_plan_sans_changement_prouve_la_convergence() {
    let plan = analyser_resume_plan("No changes. Your infrastructure matches the configuration.\n");
    assert!(
        plan.sans_changement(),
        "un ré-apply sans écart est la preuve de convergence de convergence-iac.md"
    );
}

#[test]
fn edge_un_plan_a_plusieurs_centaines_de_ressources_est_lu() {
    let plan = analyser_resume_plan("Plan: 412 to add, 0 to change, 0 to destroy.\n");
    assert_eq!(plan.creations, 412);
}

#[test]
fn edge_une_sortie_sans_ligne_de_resume_rend_un_plan_a_zero_avec_le_brut() {
    let plan = analyser_resume_plan("sortie tronquée par le moteur");
    assert!(plan.sans_changement());
    assert!(
        plan.brut.contains("tronquée"),
        "la sortie brute est conservée pour l'audit, même illisible"
    );
}

#[test]
fn edge_un_historique_vide_rend_une_liste_vide() {
    let historique = Helm::new(ExecuteurDouble::rendant(0, "[]"))
        .historique(&sure("r"), &sure("n"))
        .expect("historique");
    assert!(historique.is_empty());
}

// ── @security ────────────────────────────────────────────────

#[test]
fn security_une_valeur_contenant_un_point_virgule_est_refusee_a_l_admission() {
    // Le test emblématique de la story : le refus a lieu à l'admission, et
    // aucun processus n'est lancé. Un échappement, lui, supposerait de connaître
    // l'analyseur des six moteurs.
    let erreur = ValeurSure::new("module; rm -rf /").unwrap_err();
    assert!(erreur.to_string().contains("caractère"));
}

#[test]
fn security_tous_les_metacaracteres_sont_refuses() {
    for tentative in [
        "a;b", "a|b", "a&b", "a$b", "a`b", "a>b", "a<b", "a(b", "a{b", "a*b", "a?b", "a'b", "a\"b",
        "a\\b", "a\nb", "a\0b",
    ] {
        assert!(
            ValeurSure::new(tentative).is_err(),
            "« {} » aurait dû être refusé",
            tentative.escape_default()
        );
    }
}

#[test]
fn security_une_remontee_de_chemin_est_refusee() {
    let erreur = ValeurSure::new("../../etc/shadow").unwrap_err();
    assert!(erreur.to_string().contains("racine"));
}

#[test]
fn security_aucun_argument_n_est_interpole_les_arguments_partent_en_tableau() {
    let double = ExecuteurDouble::rendant(0, "Plan: 0 to add, 0 to change, 0 to destroy.");
    let terraform = Terraform::new(double);
    let _ = terraform.plan(&sure("infra/prod"));
    // Impossible d'observer le double après l'avoir déplacé ; on vérifie plutôt
    // que la construction d'arguments ne concatène jamais, via un second double.
    let observable = ExecuteurDouble::rendant(0, r#"{"info":{"status":"deployed"},"version":1}"#);
    {
        let helm = Helm::new(&observable);
        let _ = helm.statut(&sure("rel"), &sure("esp"));
    }
    let (programme, arguments) = observable.dernier_appel().expect("un appel");
    assert_eq!(programme, "helm");
    assert!(
        arguments.iter().all(|a| !a.contains(' ') || a == "-o"),
        "aucun argument ne doit être une ligne de commande concaténée : {arguments:?}"
    );
    assert!(arguments.contains(&"rel".to_string()));
    assert!(arguments.contains(&"esp".to_string()));
}

#[test]
fn security_un_executable_hors_allowlist_est_refuse() {
    let erreur = ExecuteurSysteme
        .executer("sh", &["-c".to_string(), "echo".to_string()], None)
        .unwrap_err();
    assert!(
        erreur.to_string().contains("allowlist"),
        "obtenu : {erreur}"
    );
}

#[test]
fn security_le_rendu_kustomize_masque_les_valeurs_de_secret() {
    let rendu = "apiVersion: v1\n\
                 kind: Secret\n\
                 metadata:\n  name: db\n\
                 data:\n  \
                   motdepasse: c3VwZXJzZWNyZXQ=\n  \
                   utilisateur: YWRtaW4=\n\
                 ---\n\
                 apiVersion: v1\n\
                 kind: ConfigMap\n\
                 data:\n  niveau: debug\n";
    let masque = masquer_secrets(rendu);
    assert!(
        !masque.contains("c3VwZXJzZWNyZXQ="),
        "une valeur de Secret a fuité : {masque}"
    );
    assert!(
        masque.contains("niveau: debug"),
        "le masquage ne doit pas toucher aux ConfigMap : {masque}"
    );
}

#[test]
fn security_le_rendu_masque_est_bien_applique_par_le_moteur() {
    let double = ExecuteurDouble::rendant(0, "kind: Secret\ndata:\n  cle: dmFsZXVy\n");
    let rendu = Kustomize::new(double)
        .rendre(&sure("overlays/prod"))
        .expect("rendu");
    assert!(!rendu.contains("dmFsZXVy"));
}

impl Executeur for &ExecuteurDouble {
    fn executer(
        &self,
        programme: &str,
        arguments: &[String],
        dossier: Option<&str>,
    ) -> Result<SortieProcessus, AppError> {
        (*self).executer(programme, arguments, dossier)
    }
}

// ── Story 5.1 — apply, destroy et sorties ────────────────────
//
// Le bac à sable est le seul endroit où Sluis mute lui-même : ailleurs, une
// mutation passe par la passerelle d'ADR-008. `appliquer` exige donc un bail
// en paramètre, non pour s'en servir, mais pour qu'aucun chemin d'appel ne
// puisse exister sans qu'un bail ait été loué, donc sans dérogation valide,
// sans TTL et sans plafond. `detruire` n'exige rien, volontairement : le chien
// de garde doit pouvoir nettoyer un bail déjà échu.

use sluis::domain::{
    Action, BailBacASable, DemandeBail, Duree, Environnement, FenetreDerogation, Horodatage,
    JetonChangement, JetonConsomme, ListeAutorisation, PlafondDepense, PlanChangement, Tier,
};
use sluis::infrastructure::process::analyser_resume_mutation;

const MAINTENANT: Horodatage = Horodatage::new(1_000_000);

/// Approbation de Tier 1 réelle, obtenue par tout le chemin.
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

/// Un bail vivant, pour les appels qui doivent en présenter un.
fn bail_de_test() -> BailBacASable {
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
        Duree::secondes(21_600).expect("ttl max"),
        MAINTENANT,
    )
    .expect("bail")
}

fn double_en_echec(stderr: &str) -> ExecuteurDouble {
    ExecuteurDouble {
        reponse: Ok(SortieProcessus {
            code: 1,
            sortie: String::new(),
            erreur: stderr.to_string(),
        }),
        appels: Mutex::new(Vec::new()),
    }
}

#[test]
fn happy_un_apply_terraform_est_resume_correctement() {
    let double = ExecuteurDouble::rendant(
        0,
        "openstack_compute_instance_v2.koprogo_vps: Creation complete\n\n\
         Apply complete! Resources: 3 added, 1 changed, 0 destroyed.\n",
    );
    let mutation = Terraform::new(&double)
        .appliquer(&sure("infra/bac-a-sable"), &bail_de_test())
        .expect("apply");

    assert_eq!(mutation.creations, 3);
    assert_eq!(mutation.modifications, 1);
    assert_eq!(mutation.destructions, 0);
}

#[test]
fn happy_un_destroy_terraform_est_resume_correctement() {
    let double = ExecuteurDouble::rendant(0, "Destroy complete! Resources: 3 destroyed.\n");
    let mutation = Terraform::new(&double)
        .detruire(&sure("infra/bac-a-sable"))
        .expect("destroy");

    assert_eq!(mutation.destructions, 3);
    assert_eq!(mutation.creations, 0);
}

#[test]
fn happy_les_sorties_terraform_sont_lues_en_json() {
    let double = ExecuteurDouble::rendant(
        0,
        r#"{"vps_ip":{"sensitive":false,"type":"string","value":"57.128.0.1"},
            "ssh_command":{"sensitive":false,"type":"string","value":"ssh ubuntu@57.128.0.1"}}"#,
    );
    let sorties = Terraform::new(&double)
        .sorties(&sure("infra/bac-a-sable"))
        .expect("sorties");

    assert_eq!(
        sorties
            .iter()
            .find(|(nom, _)| nom == "vps_ip")
            .map(|(_, valeur)| valeur.as_str()),
        Some("57.128.0.1")
    );
}

#[test]
fn negative_un_apply_en_echec_nomme_le_moteur_et_son_erreur() {
    let double = double_en_echec("Error: quota exceeded for instances");
    let erreur = Terraform::new(&double)
        .appliquer(&sure("infra/bac-a-sable"), &bail_de_test())
        .expect_err("l'apply doit échouer");

    assert!(erreur.to_string().contains("terraform"));
    assert!(erreur.to_string().contains("quota exceeded"));
}

#[test]
fn negative_une_destruction_en_echec_ne_se_tait_pas() {
    // Le seul échec du lot qui coûte de l'argent tant qu'il n'est pas vu : une
    // destruction ratée laisse une infrastructure qui facture.
    let double = double_en_echec("Error: instance still locked");
    let erreur = Terraform::new(&double)
        .detruire(&sure("infra/bac-a-sable"))
        .expect_err("la destruction doit échouer bruyamment");

    assert!(erreur.to_string().contains("instance still locked"));
}

#[test]
fn edge_un_apply_sans_ligne_de_resume_rend_zero_plutot_que_de_deviner() {
    let double = ExecuteurDouble::rendant(0, "Apply complete!\n");
    let mutation = Terraform::new(&double)
        .appliquer(&sure("infra/bac-a-sable"), &bail_de_test())
        .expect("apply");

    assert_eq!(mutation.creations, 0);
    assert!(mutation.brut.contains("Apply complete!"));
}

#[test]
fn edge_des_sorties_terraform_vides_ne_sont_pas_une_erreur() {
    let double = ExecuteurDouble::rendant(0, "{}");
    let sorties = Terraform::new(&double)
        .sorties(&sure("infra/bac-a-sable"))
        .expect("sorties");

    assert!(sorties.is_empty());
}

#[test]
fn security_un_apply_ne_demande_jamais_de_confirmation_interactive() {
    let double = ExecuteurDouble::rendant(
        0,
        "Apply complete! Resources: 1 added, 0 changed, 0 destroyed.",
    );
    let _ = Terraform::new(&double).appliquer(&sure("infra/bac-a-sable"), &bail_de_test());
    let (programme, arguments) = double.dernier_appel().expect("un appel");

    assert_eq!(programme, "terraform");
    // Sans `-input=false`, terraform attend une réponse sur une entrée standard
    // qui n'existe pas dans un service : le processus se bloque au lieu
    // d'échouer, et rien dans les journaux ne le dit.
    assert!(arguments.contains(&"-input=false".to_string()));
    assert!(arguments.contains(&"-auto-approve".to_string()));
}

#[test]
fn security_une_mutation_ne_desactive_jamais_le_verrou_d_etat() {
    let double = ExecuteurDouble::rendant(
        0,
        "Apply complete! Resources: 1 added, 0 changed, 0 destroyed.",
    );
    let _ = Terraform::new(&double).appliquer(&sure("infra/bac-a-sable"), &bail_de_test());
    let (_, arguments) = double.dernier_appel().expect("un appel");

    // `plan` passe `-lock=false` parce qu'il ne fait que lire. Une mutation qui
    // ferait de même autoriserait deux apply concurrents sur le même état, donc
    // une infrastructure orpheline que plus aucun état ne décrit.
    assert!(!arguments.contains(&"-lock=false".to_string()));
}

#[test]
fn security_l_analyse_d_un_resume_de_mutation_ne_confond_pas_les_verbes() {
    // « 0 added » et « 2 destroyed » dans la même ligne : un analyseur qui
    // prendrait le premier nombre venu rendrait 0 partout.
    let mutation =
        analyser_resume_mutation("Apply complete! Resources: 0 added, 0 changed, 2 destroyed.");
    assert_eq!(mutation.creations, 0);
    assert_eq!(mutation.destructions, 2);
}
