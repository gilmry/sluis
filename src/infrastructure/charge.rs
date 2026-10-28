//! Moteur de charge `k6`.
//!
//! k6 remplace wrk depuis l'amendement d'ADR-010 du 2026-10-28 (sluis#8) :
//! binaire de release vérifié par empreinte, et un résumé structuré au lieu
//! d'un texte à analyser. Sluis le lance comme processus, sans le lier ni le
//! modifier, ce qui permet de redistribuer ce binaire AGPL dans une image MIT.

use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Deserialize;

use crate::application::ports::{EnvironnementIsole, Executeur, MoteurCharge, ReglagePalier};
use crate::domain::{AppError, MesureCapacite, Palier};
use crate::infrastructure::process::PATH_TOFU;

/// Le script joué à chaque palier, embarqué dans le binaire.
pub const SCRIPT_K6: &str = include_str!("charge.k6.js");

/// Préfixe de la ligne de résumé écrite par le `handleSummary` du script.
pub const MARQUE_RESUME: &str = "SLUIS_RESUME ";

/// Pilote `k6`.
pub struct K6<E: Executeur> {
    executeur: E,
    dossier_travail: PathBuf,
}

impl<E: Executeur> K6<E> {
    /// L'environnement de k6 : `PATH` fixe et `HOME` dans le dossier de
    /// travail, **rien d'autre**.
    ///
    /// L'exécutant détient les clés OVH et le jeton GitHub, et k6 est un
    /// binaire tiers que le gate bloquant ne scanne pas (ADR-011). Hérité,
    /// l'environnement lui serait entièrement lisible, et k6 l'expose par
    /// défaut au script dans `__ENV`. Lancé isolé, il n'a aucun secret à
    /// perdre.
    pub fn environnement(&self) -> EnvironnementIsole {
        EnvironnementIsole {
            variables: vec![
                ("PATH".to_string(), PATH_TOFU.to_string()),
                (
                    "HOME".to_string(),
                    self.dossier_travail.to_string_lossy().to_string(),
                ),
            ],
        }
    }

    fn lancer(
        &self,
        arguments: &[String],
    ) -> Result<crate::application::ports::SortieProcessus, AppError> {
        self.executeur.executer_isole(
            "k6",
            arguments,
            &self.dossier_travail.to_string_lossy(),
            &self.environnement(),
        )
    }

    /// Construit le pilote ; le script est écrit dans le dossier temporaire
    /// du système, un tmpfs dans l'image exécutante.
    pub fn new(executeur: E) -> Self {
        Self::avec_dossier(executeur, std::env::temp_dir())
    }

    /// Construit le pilote avec un dossier de travail donné.
    pub fn avec_dossier(executeur: E, dossier_travail: PathBuf) -> Self {
        Self {
            executeur,
            dossier_travail,
        }
    }
}

/// Les arguments de `k6` pour un palier.
///
/// `--no-usage-report` n'est pas cosmétique : sans lui, k6 envoie à Grafana
/// un rapport d'usage à chaque run, depuis la machine qui détient les clés
/// OVH. L'exécutant ne parle qu'à sa cible.
pub fn arguments_k6(cible: &str, reglage: &ReglagePalier, script: &Path) -> Vec<String> {
    vec![
        "run".to_string(),
        "--quiet".to_string(),
        "--no-color".to_string(),
        "--no-usage-report".to_string(),
        "--vus".to_string(),
        reglage.connexions.to_string(),
        "--duration".to_string(),
        format!("{}s", reglage.duree_secondes),
        "--summary-trend-stats".to_string(),
        "p(50),p(99)".to_string(),
        "-e".to_string(),
        format!("SLUIS_CIBLE={cible}"),
        script.to_string_lossy().to_string(),
    ]
}

/// Écrit le script dans un fichier **neuf**, lisible du seul processus.
///
/// `create_new` refuse un chemin existant, lien symbolique compris : un tiers
/// qui aurait préparé le nom dans un dossier partagé ne peut pas faire jouer
/// son propre script.
pub fn ecrire_script(chemin: &Path) -> Result<(), AppError> {
    let erreur = |e: std::io::Error| AppError::EntreeSortie {
        chemin: chemin.display().to_string(),
        detail: e.to_string(),
    };
    let mut fichier = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(chemin)
        .map_err(erreur)?;
    fichier.write_all(SCRIPT_K6.as_bytes()).map_err(erreur)
}

/// Supprime le script quand le palier est fini, quelle qu'en soit l'issue.
struct ScriptTemporaire(PathBuf);

impl Drop for ScriptTemporaire {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn chemin_neuf(dossier: &Path) -> PathBuf {
    static COMPTEUR: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    dossier.join(format!(
        "sluis-k6-{}-{nanos}-{}.js",
        std::process::id(),
        COMPTEUR.fetch_add(1, Ordering::Relaxed)
    ))
}

#[derive(Deserialize)]
struct Resume {
    requetes: u64,
    debit: Option<f64>,
    p50: Option<f64>,
    p99: Option<f64>,
}

/// Analyse la sortie de `k6 run` : la ligne `SLUIS_RESUME`, et elle seule.
///
/// Conserve les conditions avec chaque mesure : une mesure sans ses
/// conditions ne se compare à rien, et l'abaque exige qu'elles soient
/// consignées.
pub fn analyser_sortie(
    sortie: &str,
    palier: Palier,
    conditions: &str,
) -> Result<Vec<MesureCapacite>, AppError> {
    let erreur = |detail: String| AppError::Analyse {
        quoi: format!("sortie k6 du palier {palier}"),
        detail,
    };
    let json = sortie
        .lines()
        .rev()
        .find_map(|ligne| ligne.trim().strip_prefix(MARQUE_RESUME))
        .ok_or_else(|| erreur("ligne SLUIS_RESUME absente".to_string()))?;
    let resume: Resume = serde_json::from_str(json).map_err(|e| erreur(e.to_string()))?;

    if resume.requetes == 0 {
        return Err(erreur(
            "aucune requête comptée : la mesure serait sans échantillon".to_string(),
        ));
    }
    if let (Some(p50), Some(p99)) = (resume.p50, resume.p99) {
        crate::domain::verifier_coherence_latences(p50, p99)?;
    }

    let mut mesures = Vec::new();
    for (grandeur, valeur, unite) in [
        ("debit", resume.debit, "req/s"),
        ("latence_p50", resume.p50, "ms"),
        ("latence_p99", resume.p99, "ms"),
    ] {
        if let Some(valeur) = valeur {
            mesures.push(MesureCapacite::mesuree(
                grandeur.to_string(),
                valeur,
                unite.to_string(),
                palier,
                resume.requetes,
                conditions.to_string(),
            )?);
        }
    }
    Ok(mesures)
}

impl<E: Executeur> MoteurCharge for K6<E> {
    fn jouer(&self, cible: &str, reglage: &ReglagePalier) -> Result<Vec<MesureCapacite>, AppError> {
        // Le garde ne naît qu'une fois le fichier créé par nous : sur un nom
        // déjà pris, il supprimerait le fichier d'un autre.
        let chemin = chemin_neuf(&self.dossier_travail);
        ecrire_script(&chemin)?;
        let script = ScriptTemporaire(chemin);
        let sortie = self.lancer(&arguments_k6(cible, reglage, &script.0))?;
        if !sortie.reussi() {
            return Err(AppError::ServiceTiers {
                service: "k6".to_string(),
                detail: sortie.erreur.trim().to_string(),
            });
        }
        let conditions = format!(
            "palier {} · {} utilisateurs virtuels · {}s · cible {}",
            reglage.palier, reglage.connexions, reglage.duree_secondes, cible
        );
        analyser_sortie(&sortie.sortie, reglage.palier, &conditions)
    }

    fn disponible(&self) -> bool {
        self.lancer(&["version".to_string()])
            .is_ok_and(|s| s.reussi())
    }
}
