//! Évaluation des contrôles de la barrière par des processus système.
//!
//! Chaque contrôle est lancé **isolé** (environnement vidé, `PATH` fixe, sans
//! entrée standard), **sans shell**, dans **son propre groupe de processus**.
//! Le groupe est ce qui permet de tenir le délai : un script qui lance un
//! `sleep` laisserait sinon un enfant vivant, qui garderait ses sorties
//! ouvertes et ferait attendre Sluis bien au-delà du délai.

use std::io::Read;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::application::ports::EvaluateurBarriere;
use crate::domain::{ControleBarriere, VerdictControle};
use crate::infrastructure::process::PATH_TOFU;

/// Taille maximale conservée de chaque sortie d'un contrôle, en octets.
///
/// Au-delà, la sortie est lue et jetée, pour que le contrôle ne se bloque pas
/// sur un tube plein.
pub const RAPPORT_MAXIMAL_OCTETS: usize = 8 * 1024;

/// Intervalle de surveillance d'un contrôle en cours.
const SONDAGE: Duration = Duration::from_millis(20);

/// Évaluateur sur `std::process`.
#[derive(Debug, Default, Clone, Copy)]
pub struct EvaluateurBarriereSysteme;

impl EvaluateurBarriere for EvaluateurBarriereSysteme {
    fn evaluer(&self, controles: &[ControleBarriere]) -> Vec<VerdictControle> {
        controles.iter().map(evaluer_un).collect()
    }
}

fn verdict(
    controle: &ControleBarriere,
    vert: bool,
    motif: String,
    rapport: String,
) -> VerdictControle {
    VerdictControle {
        nom: controle.nom().to_string(),
        vert,
        motif,
        rapport,
    }
}

fn evaluer_un(controle: &ControleBarriere) -> VerdictControle {
    let mut commande = Command::new(controle.programme());
    commande
        .args(controle.arguments())
        .env_clear()
        .env("PATH", PATH_TOFU)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);

    let mut enfant = match lancer(&mut commande) {
        Ok(enfant) => enfant,
        Err(e) => {
            let motif = match e.kind() {
                std::io::ErrorKind::NotFound => "programme absent".to_string(),
                std::io::ErrorKind::PermissionDenied => "programme non exécutable".to_string(),
                _ => format!("lancement impossible : {e}"),
            };
            return verdict(controle, false, motif, String::new());
        }
    };

    let sortie = lire_en_tache(enfant.stdout.take());
    let erreurs = lire_en_tache(enfant.stderr.take());
    let fin = attendre(&mut enfant, Duration::from_secs(controle.delai_secondes()));
    // Le chef du groupe est terminé ou tué : on tue ce qui reste du groupe,
    // pour que les tubes se ferment et que la lecture s'achève.
    tuer_groupe(&enfant);
    let rapport = format!(
        "sortie :\n{}\nerreurs :\n{}",
        recolter(sortie),
        recolter(erreurs)
    );

    match fin {
        None => verdict(
            controle,
            false,
            format!("délai dépassé ({} s)", controle.delai_secondes()),
            rapport,
        ),
        Some(statut) => match (statut.code(), statut.signal()) {
            (Some(0), _) => verdict(controle, true, "code 0".to_string(), rapport),
            (Some(code), _) => verdict(controle, false, format!("code {code}"), rapport),
            (None, Some(signal)) => verdict(
                controle,
                false,
                format!("tué par le signal {signal}"),
                rapport,
            ),
            (None, None) => verdict(controle, false, "fin inexpliquée".to_string(), rapport),
        },
    }
}

/// Tentatives de lancement quand le programme est « occupé » (`ETXTBSY`).
const TENTATIVES_ETXTBSY: u32 = 5;

/// Lance la commande, en retentant sur `ETXTBSY`.
///
/// Cette erreur est transitoire : le fichier est encore ouvert en écriture,
/// soit qu'on vient de l'installer, soit qu'un `fork` concurrent hérite un
/// instant de son descripteur avant son `exec`. Elle ne dit rien du
/// contrôle ; après quelques essais, elle compte rouge comme toute autre.
fn lancer(commande: &mut Command) -> std::io::Result<Child> {
    let mut essai = 1;
    loop {
        match commande.spawn() {
            Err(e) if e.raw_os_error() == Some(libc::ETXTBSY) && essai < TENTATIVES_ETXTBSY => {
                std::thread::sleep(SONDAGE * essai);
                essai += 1;
            }
            resultat => return resultat,
        }
    }
}

/// Attend la fin du processus ; au-delà du délai, tue son groupe et rend
/// `None`.
fn attendre(enfant: &mut Child, delai: Duration) -> Option<ExitStatus> {
    let debut = Instant::now();
    loop {
        match enfant.try_wait() {
            Ok(Some(statut)) => return Some(statut),
            Ok(None) if debut.elapsed() < delai => std::thread::sleep(SONDAGE),
            // Hors délai, ou état illisible : dans les deux cas le contrôle
            // n'a rien prouvé, il est tué et compté rouge.
            _ => {
                tuer_groupe(enfant);
                let _ = enfant.wait();
                return None;
            }
        }
    }
}

fn tuer_groupe(enfant: &Child) {
    let Ok(groupe) = i32::try_from(enfant.id()) else {
        return;
    };
    // SAFETY: `kill` ne lit ni n'écrit aucune mémoire du processus ; le groupe
    // est celui créé par `process_group(0)`, dont l'identifiant est celui de
    // l'enfant. Un groupe déjà vide rend ESRCH, sans effet.
    unsafe {
        libc::kill(-groupe, libc::SIGKILL);
    }
}

fn lire_en_tache<R: Read + Send + 'static>(flux: Option<R>) -> Option<JoinHandle<Vec<u8>>> {
    flux.map(|mut flux| {
        std::thread::spawn(move || {
            let mut garde = Vec::new();
            let mut tampon = [0_u8; 4096];
            while let Ok(lus) = flux.read(&mut tampon) {
                if lus == 0 {
                    break;
                }
                let place = RAPPORT_MAXIMAL_OCTETS.saturating_sub(garde.len());
                garde.extend_from_slice(&tampon[..lus.min(place)]);
            }
            garde
        })
    })
}

fn recolter(tache: Option<JoinHandle<Vec<u8>>>) -> String {
    tache
        .and_then(|t| t.join().ok())
        .map(|octets| String::from_utf8_lossy(&octets).trim_end().to_string())
        .unwrap_or_default()
}
