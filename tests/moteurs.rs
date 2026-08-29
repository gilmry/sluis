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
