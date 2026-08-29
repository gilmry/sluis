//! Cycle de vie d'un bail : garde RAII et chien de garde.
//!
//! Deux mécanismes indépendants garantissent la destruction, et cette
//! redondance est délibérée.
//!
//! [`GardeBail`] détruit le bail quand la portée se termine, y compris sur une
//! panique. Elle couvre le cas normal et le cas de l'erreur.
//!
//! [`ChienDeGarde`] tourne dans un fil séparé et détruit les baux échus **même
//! si le processus demandeur a disparu**. Il couvre le cas que la garde RAII ne
//! peut pas couvrir : un `SIGKILL`, une coupure de courant, un conteneur
//! évincé. C'est le seul mécanisme qui survit à la mort de son demandeur, et
//! c'est pour cela qu'il ne peut pas être supprimé au motif que la garde RAII
//! ferait déjà le travail.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::domain::{AppError, BailBacASable, Horodatage};

/// Détruit une infrastructure éphémère.
pub trait DestructeurBail: Send + Sync {
    /// Détruit le bail. Doit être idempotent : le chien de garde et la garde
    /// RAII peuvent tous deux l'appeler pour le même bail.
    fn detruire(&self, bail: &BailBacASable) -> Result<(), AppError>;
}

/// Garde RAII : détruit le bail à la sortie de portée, panique comprise.
pub struct GardeBail {
    bail: BailBacASable,
    destructeur: Arc<dyn DestructeurBail>,
    detruit: AtomicBool,
}

impl GardeBail {
    /// Prend en charge un bail.
    pub fn nouvelle(bail: BailBacASable, destructeur: Arc<dyn DestructeurBail>) -> Self {
        Self {
            bail,
            destructeur,
            detruit: AtomicBool::new(false),
        }
    }

    /// Le bail sous garde.
    pub fn bail(&self) -> &BailBacASable {
        &self.bail
    }

    /// Détruit explicitement, avant la fin de portée.
    pub fn detruire_maintenant(&self) -> Result<(), AppError> {
        if self.detruit.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        self.destructeur.detruire(&self.bail)
    }
}

impl Drop for GardeBail {
    fn drop(&mut self) {
        if self.detruit.swap(true, Ordering::SeqCst) {
            return;
        }
        // Une panique dans un `Drop` pendant un déroulement de pile avorte le
        // processus : on avale l'erreur ici et on la signale, plutôt que de
        // transformer un échec de destruction en arrêt brutal.
        if let Err(erreur) = self.destructeur.detruire(&self.bail) {
            eprintln!(
                "sluis : échec de destruction du bail {} : {erreur} — le chien de garde \
                 reprendra la main à l'échéance",
                self.bail.projet()
            );
        }
    }
}

/// Registre des baux vivants, partagé avec le chien de garde.
#[derive(Default)]
pub struct RegistreBaux {
    baux: Mutex<Vec<BailBacASable>>,
}

impl RegistreBaux {
    /// Registre vide.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inscrit un bail.
    pub fn inscrire(&self, bail: BailBacASable) -> Result<(), AppError> {
        self.baux
            .lock()
            .map_err(|_| AppError::Configuration {
                detail: "verrou du registre de baux empoisonné".to_string(),
            })?
            .push(bail);
        Ok(())
    }

    /// Retire et rend les baux échus.
    pub fn retirer_echus(&self, maintenant: Horodatage) -> Vec<BailBacASable> {
        let Ok(mut baux) = self.baux.lock() else {
            return Vec::new();
        };
        let (echus, vivants): (Vec<_>, Vec<_>) = baux.drain(..).partition(|b| b.expire(maintenant));
        *baux = vivants;
        echus
    }

    /// Nombre de baux vivants.
    pub fn vivants(&self) -> usize {
        self.baux.lock().map(|b| b.len()).unwrap_or(0)
    }
}

/// Chien de garde : détruit les baux échus, indépendamment du demandeur.
pub struct ChienDeGarde {
    registre: Arc<RegistreBaux>,
    destructeur: Arc<dyn DestructeurBail>,
    detruits: AtomicUsize,
    echecs: AtomicUsize,
}

impl ChienDeGarde {
    /// Construit le chien de garde.
    pub fn new(registre: Arc<RegistreBaux>, destructeur: Arc<dyn DestructeurBail>) -> Self {
        Self {
            registre,
            destructeur,
            detruits: AtomicUsize::new(0),
            echecs: AtomicUsize::new(0),
        }
    }

    /// Un tour de ronde.
    ///
    /// Un échec de destruction est **compté et signalé**, jamais avalé : un
    /// bail qu'on croit détruit et qui facture encore est le pire cas de ce
    /// bounded context.
    pub fn ronde(&self, maintenant: Horodatage) -> usize {
        let echus = self.registre.retirer_echus(maintenant);
        let mut traites = 0;
        for bail in echus {
            match self.destructeur.detruire(&bail) {
                Ok(()) => {
                    self.detruits.fetch_add(1, Ordering::SeqCst);
                    traites += 1;
                }
                Err(erreur) => {
                    self.echecs.fetch_add(1, Ordering::SeqCst);
                    eprintln!(
                        "sluis : ALERTE — le bail {} n'a pas pu être détruit : {erreur}. \
                         Il continue de facturer.",
                        bail.projet()
                    );
                    // Réinscrit pour être retenté à la ronde suivante : abandonner
                    // silencieusement reviendrait à perdre le bail de vue.
                    let _ = self.registre.inscrire(bail);
                }
            }
        }
        traites
    }

    /// Nombre de baux détruits depuis le démarrage.
    pub fn detruits(&self) -> usize {
        self.detruits.load(Ordering::SeqCst)
    }

    /// Nombre d'échecs de destruction.
    pub fn echecs(&self) -> usize {
        self.echecs.load(Ordering::SeqCst)
    }
}
