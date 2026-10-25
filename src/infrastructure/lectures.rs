//! Registre des fichiers lus pendant un appel d'outil (ADR-034, point 5).
//!
//! Les lecteurs (`FsModuleTofu`, `FsCharge`, `FsInventaire`) consignent ce
//! qu'ils lisent ; le serveur MCP recueille le registre après l'appel et
//! l'attache à l'entrée du journal. Le passer en paramètre à travers chaque
//! port et chaque moteur aurait touché toutes les signatures pour un besoin
//! d'audit.
//!
//! **Un registre par fil d'exécution.** Un appel d'outil s'exécute de bout en
//! bout sur un fil : deux appels concurrents ne mélangent pas leurs lectures.
//! **Limite assumée** : une lecture faite sur un autre fil que celui de
//! l'appel n'est pas consignée. Elle est perdue, jamais attribuée à un autre
//! appel.

use std::cell::RefCell;
use std::path::Path;

use crate::domain::Lecture;

thread_local! {
    static LECTURES: RefCell<Vec<Lecture>> = const { RefCell::new(Vec::new()) };
}

/// Consigne la lecture de `contenu` depuis `chemin`.
pub fn consigner(chemin: &Path, contenu: &str) {
    let lecture = Lecture::new(chemin.display().to_string(), contenu.as_bytes())
        .avec_commit(crate::infrastructure::depot_git::commit_courant(chemin));
    LECTURES.with(|lectures| lectures.borrow_mut().push(lecture));
}

/// Rend les lectures consignées sur ce fil depuis le dernier recueil, et
/// vide le registre.
pub fn recueillir() -> Vec<Lecture> {
    LECTURES.with(|lectures| std::mem::take(&mut *lectures.borrow_mut()))
}
