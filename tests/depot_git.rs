//! Story 34.5b — le commit du dépôt d'un fichier lu, sans lancer `git`.

use std::path::{Path, PathBuf};

use sluis::infrastructure::depot_git::commit_courant;
use sluis::infrastructure::lectures;

const COMMIT_A: &str = "0123456789abcdef0123456789abcdef01234567";
const COMMIT_B: &str = "89abcdef0123456789abcdef0123456789abcdef";

/// Un dépôt factice : un `.git` écrit à la main et un fichier suivi.
struct Depot(PathBuf);

impl Depot {
    fn nouveau(nom: &str) -> Self {
        let racine = std::env::temp_dir().join(format!("sluis-git-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&racine);
        std::fs::create_dir_all(racine.join(".git/refs/heads")).expect("création");
        std::fs::create_dir_all(racine.join("infrastructure/_shared")).expect("création");
        std::fs::write(racine.join("infrastructure/_shared/charge.yaml"), "a: b\n")
            .expect("écriture");
        Self(racine)
    }
    fn ecrire(&self, relatif: &str, contenu: &str) {
        let chemin = self.0.join(relatif);
        std::fs::create_dir_all(chemin.parent().expect("parent")).expect("création");
        std::fs::write(chemin, contenu).expect("écriture");
    }
    fn fichier(&self) -> PathBuf {
        self.0.join("infrastructure/_shared/charge.yaml")
    }
}

impl Drop for Depot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_head_sur_une_branche_libre_donne_son_commit() {
    let depot = Depot::nouveau("libre");
    depot.ecrire(".git/HEAD", "ref: refs/heads/main\n");
    depot.ecrire(".git/refs/heads/main", &format!("{COMMIT_A}\n"));

    assert_eq!(commit_courant(&depot.fichier()).as_deref(), Some(COMMIT_A));
}

#[test]
fn happy_une_reference_empaquetee_est_resolue() {
    let depot = Depot::nouveau("empaquetee");
    depot.ecrire(".git/HEAD", "ref: refs/heads/main\n");
    depot.ecrire(
        ".git/packed-refs",
        &format!(
            "# pack-refs with: peeled fully-peeled sorted\n\
             {COMMIT_B} refs/heads/autre\n{COMMIT_A} refs/heads/main\n^{COMMIT_B}\n"
        ),
    );

    assert_eq!(commit_courant(&depot.fichier()).as_deref(), Some(COMMIT_A));
}

#[test]
fn happy_la_lecture_consignee_porte_le_commit() {
    let depot = Depot::nouveau("consignee");
    depot.ecrire(".git/HEAD", "ref: refs/heads/main\n");
    depot.ecrire(".git/refs/heads/main", COMMIT_A);
    let _ = lectures::recueillir();

    lectures::consigner(&depot.fichier(), "a: b\n");

    let lues = lectures::recueillir();
    assert_eq!(lues[0].commit(), Some(COMMIT_A));
}

// ── @edge ────────────────────────────────────────────────────

#[test]
fn edge_un_head_detache_est_lu_tel_quel() {
    let depot = Depot::nouveau("detache");
    depot.ecrire(".git/HEAD", &format!("{COMMIT_B}\n"));

    assert_eq!(commit_courant(&depot.fichier()).as_deref(), Some(COMMIT_B));
}

#[test]
fn edge_un_worktree_suit_gitdir_puis_commondir() {
    let principal = Depot::nouveau("principal");
    principal.ecrire(".git/refs/heads/branche", COMMIT_B);
    principal.ecrire(".git/worktrees/arbre/HEAD", "ref: refs/heads/branche\n");
    principal.ecrire(".git/worktrees/arbre/commondir", "../..\n");

    let arbre = Depot::nouveau("arbre");
    let _ = std::fs::remove_dir_all(arbre.0.join(".git"));
    let gitdir = principal.0.join(".git/worktrees/arbre");
    arbre.ecrire(".git", &format!("gitdir: {}\n", gitdir.display()));

    assert_eq!(commit_courant(&arbre.fichier()).as_deref(), Some(COMMIT_B));
}

// ── @negative ────────────────────────────────────────────────

#[test]
fn negative_un_fichier_hors_de_tout_depot_n_a_pas_de_commit() {
    let dossier = std::env::temp_dir().join(format!("sluis-sans-git-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dossier);
    std::fs::create_dir_all(&dossier).expect("création");
    let fichier = dossier.join("f.yaml");
    std::fs::write(&fichier, "a: b\n").expect("écriture");

    // /tmp n'est dans aucun dépôt sur une machine de CI ; si c'était le cas,
    // le test ne prouverait rien, d'où la vérification préalable.
    if Path::new("/tmp/.git").exists() || Path::new("/.git").exists() {
        return;
    }
    assert_eq!(commit_courant(&fichier), None);
    let _ = std::fs::remove_dir_all(&dossier);
}

// ── @security ────────────────────────────────────────────────

#[test]
fn security_une_reference_qui_sort_du_dossier_git_est_refusee() {
    let depot = Depot::nouveau("evasion");
    // Un fichier hors du .git qui contient un identifiant valide : s'il était
    // lu, le commit rendu serait celui-ci.
    depot.ecrire("piege", COMMIT_A);
    depot.ecrire(".git/HEAD", "ref: refs/../../piege\n");

    assert_eq!(commit_courant(&depot.fichier()), None);
}

#[test]
fn security_un_head_arbitraire_donne_un_commit_inconnu() {
    let depot = Depot::nouveau("arbitraire");
    for contenu in [
        "n'importe quoi\n",
        "ref: /etc/passwd\n",
        "0123456789ABCDEF0123456789ABCDEF01234567\n",
        "",
    ] {
        depot.ecrire(".git/HEAD", contenu);
        assert_eq!(commit_courant(&depot.fichier()), None, "HEAD = {contenu:?}");
    }
}
