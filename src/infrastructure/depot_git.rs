//! Le commit courant d'un dépôt git, lu sans lancer `git` (ADR-034, point 5).
//!
//! Le journal d'audit attache à chaque fichier lu le commit du dépôt d'où il
//! vient. Lancer `git` pour cela ferait exécuter un programme configurable
//! par le dépôt lui-même (`core.fsmonitor`, hooks) : on lit donc `HEAD` et les
//! références comme des fichiers.
//!
//! **Ce qui est résolu** : HEAD détaché, référence libre, référence empaquetée
//! (`packed-refs`), `.git` fichier d'un worktree (`gitdir:` puis
//! `commondir`). **Tout le reste donne un commit inconnu**, jamais deviné :
//! un journal qui afficherait un commit faux serait pire qu'un journal qui
//! dit ne pas savoir.
//!
//! **Limite assumée** : le commit dit la version du dépôt, pas celle du
//! fichier si l'arbre de travail est modifié. C'est le SHA-256 de la lecture
//! qui prouve le contenu lu.

use std::path::{Path, PathBuf};

/// Taille maximale d'un fichier de références lu (`packed-refs` compris).
const TAILLE_MAXIMALE: u64 = 4 * 1024 * 1024;

/// Le commit de `HEAD` du dépôt qui contient `chemin`, ou `None`.
pub fn commit_courant(chemin: &Path) -> Option<String> {
    let (dossier_git, dossier_commun) = trouver_dossier_git(chemin)?;
    let head = lire_court(&dossier_git.join("HEAD"))?;
    let head = head.trim();
    match head.strip_prefix("ref:") {
        None => identifiant(head),
        Some(reference) => {
            let reference = reference.trim();
            if !reference_valide(reference) {
                return None;
            }
            resoudre(&dossier_git, &dossier_commun, reference)
        }
    }
}

/// Remonte depuis `chemin` jusqu'au premier `.git`. Rend le dossier git
/// propre (celui de `HEAD`) et le dossier commun (celui des références).
fn trouver_dossier_git(chemin: &Path) -> Option<(PathBuf, PathBuf)> {
    let depart = chemin.canonicalize().ok()?;
    for dossier in depart.ancestors() {
        let candidat = dossier.join(".git");
        if candidat.is_dir() {
            return Some((candidat.clone(), candidat));
        }
        if candidat.is_file() {
            let contenu = lire_court(&candidat)?;
            let cible = contenu.trim().strip_prefix("gitdir:")?.trim();
            let propre = dossier.join(cible).canonicalize().ok()?;
            let commun = match lire_court(&propre.join("commondir")) {
                Some(relatif) => propre.join(relatif.trim()).canonicalize().ok()?,
                None => propre.clone(),
            };
            return Some((propre, commun));
        }
    }
    None
}

/// Résout une référence : d'abord le fichier libre, dans le dossier propre
/// puis le commun, ensuite `packed-refs`.
fn resoudre(dossier_git: &Path, dossier_commun: &Path, reference: &str) -> Option<String> {
    for base in [dossier_git, dossier_commun] {
        if let Some(contenu) = lire_court(&base.join(reference)) {
            return identifiant(contenu.trim());
        }
    }
    let empaquetees = lire_court(&dossier_commun.join("packed-refs"))?;
    empaquetees
        .lines()
        .filter(|ligne| !ligne.starts_with('#') && !ligne.starts_with('^'))
        .find_map(|ligne| {
            let (id, nom) = ligne.split_once(' ')?;
            (nom.trim() == reference).then(|| identifiant(id)).flatten()
        })
}

/// Une référence lisible sans sortir du dossier git : `refs/…`, composants
/// simples, ni `..` ni chemin absolu.
fn reference_valide(reference: &str) -> bool {
    reference.starts_with("refs/")
        && !reference.contains("..")
        && !reference.contains("//")
        && reference
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '-' | '_' | '.'))
}

/// Un identifiant d'objet git : 40 (SHA-1) ou 64 (SHA-256) chiffres
/// hexadécimaux minuscules.
fn identifiant(texte: &str) -> Option<String> {
    let valide = matches!(texte.len(), 40 | 64)
        && texte
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c));
    valide.then(|| texte.to_string())
}

fn lire_court(chemin: &Path) -> Option<String> {
    let meta = std::fs::metadata(chemin).ok()?;
    if !meta.is_file() || meta.len() > TAILLE_MAXIMALE {
        return None;
    }
    std::fs::read_to_string(chemin).ok()
}
