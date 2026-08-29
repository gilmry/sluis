//! Stories 1.2, 1.3 et 1.4 — découverte de la matrice, profils, diagnostic.

use std::fs;
use std::path::{Path, PathBuf};

use sluis::application::ports::{DepotInventaire, Diagnostic};
use sluis::domain::{Environnement, Topologie};
use sluis::infrastructure::diagnostic::DiagnosticSysteme;
use sluis::infrastructure::fs_inventaire::FsInventaire;

/// Reconstruit une arborescence semblable à celle de KoproGo.
fn depot_de_reference(nom: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!("sluis-inv-{nom}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);

    for topologie in ["vps", "k3s"] {
        for environnement in ["dev", "integration", "staging", "production"] {
            fs::create_dir_all(base.join("monosite").join(topologie).join(environnement))
                .expect("création");
        }
    }
    // `local` n'est pas un environnement connu : il doit être signalé, pas tu.
    fs::create_dir_all(base.join("monosite").join("k3s").join("local")).expect("création");
    for environnement in ["dev", "integration", "staging", "production"] {
        fs::create_dir_all(base.join("multisite").join("k8s").join(environnement))
            .expect("création");
    }
    for module in ["networking", "ovh-k3s", "ovh-k8s", "ovh-vps"] {
        fs::create_dir_all(base.join("_shared/terraform/modules").join(module)).expect("création");
    }

    let profils = base.join("_shared/cluster-profiles");
    fs::create_dir_all(&profils).expect("création");
    fs::write(
        profils.join("k3s-self-hosted.yaml"),
        "# Profil k3s auto-hébergé\n\
         global:\n  \
           storageClassName: local-path      # défaut k3s\n  \
           ingressClassName: traefik\n  \
           secretsBackend: sealed-secrets\n  \
           tls:\n    \
             enabled: true\n\
         resources:\n  preset: medium\n",
    )
    .expect("écriture");
    fs::write(
        profils.join("k8s-managed.yaml"),
        "global:\n  storageClassName: csi-cinder-high-speed\n  \
         ingressClassName: nginx\n  secretsBackend: external-secrets-vault\n  \
         tls:\n    enabled: true\nresources:\n  preset: large\n",
    )
    .expect("écriture");
    fs::write(
        profils.join("docker-desktop.yaml"),
        "global:\n  storageClassName: hostpath\n  ingressClassName: nginx\n  \
         secretsBackend: raw\n  tls:\n    enabled: false\nresources:\n  preset: small\n",
    )
    .expect("écriture");
    fs::write(profils.join("README.md"), "# Profils\n").expect("écriture");
    base
}

fn chemin(base: &Path) -> String {
    base.to_str().expect("chemin").to_string()
}

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_la_matrice_de_reference_est_decouverte_sans_saisie() {
    // Le critère d'acceptation du PRD §13.1, joué sur une arborescence
    // reproduisant celle de KoproGo.
    let base = depot_de_reference("happy");
    let matrice = FsInventaire::new()
        .decouvrir_matrice(&chemin(&base))
        .expect("découverte");

    assert_eq!(matrice.topologies.len(), 3, "3 topologies attendues");
    assert_eq!(matrice.environnements.len(), 4, "4 environnements attendus");
    assert_eq!(matrice.profils.len(), 3, "3 profils attendus");
    assert_eq!(matrice.modules.len(), 4, "4 modules Terraform attendus");
    assert_eq!(
        matrice.cellules.len(),
        12,
        "3 topologies × 4 environnements"
    );
}

#[test]
fn happy_les_environnements_sortent_dans_l_ordre_de_promotion() {
    let base = depot_de_reference("ordre");
    let matrice = FsInventaire::new()
        .decouvrir_matrice(&chemin(&base))
        .expect("découverte");
    assert_eq!(
        matrice.environnements,
        vec![
            Environnement::Dev,
            Environnement::Integration,
            Environnement::Staging,
            Environnement::Production
        ]
    );
}

#[test]
fn happy_un_profil_rend_son_contrat_day1_day2() {
    let base = depot_de_reference("profil");
    let profils = FsInventaire::new()
        .lire_profils(&chemin(&base))
        .expect("lecture");
    let k3s = profils
        .iter()
        .find(|p| p.nom() == "k3s-self-hosted")
        .expect("profil k3s");
    assert_eq!(k3s.classe_stockage(), Some("local-path"));
    assert_eq!(k3s.classe_ingress(), Some("traefik"));
    assert_eq!(k3s.backend_secrets(), Some("sealed-secrets"));
    assert_eq!(k3s.tls_actif(), Some(true));
    assert_eq!(k3s.preset_ressources(), Some("medium"));
}

#[test]
fn happy_le_diagnostic_liste_les_six_moteurs() {
    let rapport = DiagnosticSysteme::avec("", Vec::new())
        .etablir()
        .expect("diagnostic");
    assert_eq!(rapport.moteurs.len(), 6);
    assert_eq!(rapport.identifiants.len(), 4);
}

// ── @negative ────────────────────────────────────────────────

#[test]
fn negative_un_chemin_inexistant_produit_une_erreur_nommant_le_chemin() {
    let erreur = FsInventaire::new()
        .decouvrir_matrice("/depot-qui-n-existe-pas")
        .unwrap_err();
    assert!(erreur.to_string().contains("depot-qui-n-existe-pas"));
}

#[test]
fn negative_un_chemin_vers_un_fichier_est_refuse() {
    let base = depot_de_reference("fichier");
    let fichier = base.join("_shared/cluster-profiles/README.md");
    let erreur = FsInventaire::new()
        .decouvrir_matrice(fichier.to_str().unwrap())
        .unwrap_err();
    assert!(erreur.to_string().contains("dossier"));
}

#[test]
fn negative_un_yaml_malforme_nomme_le_fichier() {
    let base = depot_de_reference("malforme");
    fs::write(
        base.join("_shared/cluster-profiles/casse.yaml"),
        "global:\n\tstorageClassName: x\n",
    )
    .expect("écriture");
    let erreur = FsInventaire::new()
        .lire_profils(&chemin(&base))
        .unwrap_err();
    let message = erreur.to_string();
    assert!(message.contains("casse.yaml"), "obtenu : {message}");
    assert!(message.contains("tabulation"), "obtenu : {message}");
}

#[test]
fn negative_un_binaire_non_executable_n_est_pas_rapporte_disponible() {
    let base = std::env::temp_dir().join(format!("sluis-bin-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).expect("création");
    let faux = base.join("terraform");
    fs::write(&faux, "#!/bin/sh\n").expect("écriture");
    // Volontairement sans bit d'exécution.

    let rapport = DiagnosticSysteme::avec(base.to_str().unwrap(), Vec::new())
        .etablir()
        .expect("diagnostic");
    let terraform = rapport
        .moteurs
        .iter()
        .find(|m| m.nom == "terraform")
        .expect("terraform");
    assert!(
        !terraform.etat.utilisable(),
        "un binaire sans bit d'exécution ne doit pas être dit disponible"
    );
    assert!(
        matches!(
            terraform.etat,
            sluis::domain::EtatBinaire::NonExecutable { .. }
        ),
        "il doit être distingué de l'absence, pour ne pas masquer une erreur \
         de configuration derrière un diagnostic de machine nue"
    );
}

// ── @edge ────────────────────────────────────────────────────

#[test]
fn edge_un_environnement_inconnu_est_signale_et_non_tu() {
    let base = depot_de_reference("ignores");
    let matrice = FsInventaire::new()
        .decouvrir_matrice(&chemin(&base))
        .expect("découverte");
    assert!(
        matrice.ignores.iter().any(|i| i.contains("local")),
        "« local » doit apparaître dans les ignorés : le taire donnerait \
         l'illusion d'un inventaire exhaustif, obtenu : {:?}",
        matrice.ignores
    );
}

#[test]
fn edge_une_topologie_sans_environnement_reste_declaree() {
    let base = std::env::temp_dir().join(format!("sluis-vide-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(base.join("monosite/vps")).expect("création");
    let matrice = FsInventaire::new()
        .decouvrir_matrice(&chemin(&base))
        .expect("découverte");
    assert_eq!(matrice.topologies, vec![Topologie::Vps]);
    assert!(matrice.cellules.is_empty());
}

#[test]
fn edge_un_depot_sans_profils_rend_une_liste_vide_pas_une_erreur() {
    let base = std::env::temp_dir().join(format!("sluis-sansprofil-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(base.join("monosite/k3s/dev")).expect("création");
    let profils = FsInventaire::new()
        .lire_profils(&chemin(&base))
        .expect("lecture");
    assert!(profils.is_empty());
}

#[test]
fn edge_un_path_vide_rend_les_six_moteurs_absents_sans_paniquer() {
    let rapport = DiagnosticSysteme::avec("", Vec::new())
        .etablir()
        .expect("diagnostic");
    assert_eq!(rapport.moteurs_indisponibles().len(), 6);
    assert!(rapport.moteurs_utilisables().is_empty());
}

// ── @security ────────────────────────────────────────────────

#[test]
fn security_un_lien_symbolique_sortant_de_la_racine_n_est_pas_suivi() {
    let base = std::env::temp_dir().join(format!("sluis-lien-{}", std::process::id()));
    let dehors = std::env::temp_dir().join(format!("sluis-dehors-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    let _ = fs::remove_dir_all(&dehors);
    fs::create_dir_all(base.join("monosite")).expect("création");
    fs::create_dir_all(dehors.join("k8s/production")).expect("création");

    #[cfg(unix)]
    std::os::unix::fs::symlink(&dehors, base.join("monosite/evade")).expect("lien");

    let matrice = FsInventaire::new()
        .decouvrir_matrice(&chemin(&base))
        .expect("découverte");
    assert!(
        matrice.cellules.is_empty(),
        "un lien sortant de la racine ne doit pas être suivi, sinon un dépôt \
         hostile fait lire n'importe quel dossier de la machine"
    );
}

#[test]
fn security_le_diagnostic_ne_revele_jamais_la_valeur_d_un_identifiant() {
    let rapport = DiagnosticSysteme::avec("", vec!["OVH_APPLICATION_KEY".to_string()])
        .etablir()
        .expect("diagnostic");
    let rendu = serde_json::to_string(&rapport).expect("sérialisation");
    // Le rapport dit la présence, jamais la valeur, ni sa longueur, ni un préfixe.
    assert!(rendu.contains("OVH_APPLICATION_KEY"));
    assert!(rendu.contains("\"present\":true"));
    for indice in ["longueur", "prefixe", "empreinte", "valeur"] {
        assert!(
            !rendu.contains(indice),
            "le rapport ne doit porter aucun indice sur la valeur : {indice}"
        );
    }
}

#[test]
fn security_le_diagnostic_ne_rend_jamais_present_un_identifiant_vide() {
    // Une variable définie mais vide n'est pas un identifiant utilisable ; la
    // dire présente ferait échouer plus loin, avec un diagnostic vert.
    let rapport = DiagnosticSysteme::avec("", Vec::new())
        .etablir()
        .expect("diagnostic");
    assert!(!rapport.identifiants_complets());
    assert!(rapport.identifiants.iter().all(|i| !i.present));
}
