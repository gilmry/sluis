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

use crate::application::ports::{MoteurTerraform, Provisionneur};

// Le port vit dans `application::ports` ; il reste visible ici parce que la
// garde RAII et le chien de garde, qui sont des mécanismes d'infrastructure,
// s'en servent, et parce que les appelants le nomment depuis ce module.
pub use crate::application::ports::DestructeurBail;
use crate::domain::{AppError, BailBacASable, CibleEphemere, Horodatage, ValeurSure};

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

    /// Désarme la garde : la destruction est **acquise par ailleurs**.
    ///
    /// À n'appeler qu'après une destruction réussie, sans quoi le filet est
    /// retiré alors que l'infrastructure existe encore. Le pendant est utile :
    /// si la destruction a échoué, ne pas désarmer laisse la garde retenter à
    /// la sortie de portée, ce qui donne une seconde chance sans rien coder de
    /// plus.
    pub fn desarmer(&self) {
        self.detruit.store(true, Ordering::SeqCst);
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

/// Provisionne et détruit une infrastructure éphémère avec Terraform.
///
/// Un seul module jetable, donné à la construction : le bac à sable n'a pas
/// vocation à décrire des topologies, il en loue une, la charge et l'efface.
pub struct BacASableTerraform<M: MoteurTerraform> {
    moteur: M,
    module: ValeurSure,
    sortie_adresse: String,
}

impl<M: MoteurTerraform> BacASableTerraform<M> {
    /// Construit l'adaptateur.
    ///
    /// `sortie_adresse` nomme la sortie du module qui porte l'adresse à
    /// charger, par exemple `vps_ip`.
    pub fn new(moteur: M, module: ValeurSure, sortie_adresse: impl Into<String>) -> Self {
        Self {
            moteur,
            module,
            sortie_adresse: sortie_adresse.into(),
        }
    }
}

impl<M: MoteurTerraform> Provisionneur for BacASableTerraform<M> {
    fn provisionner(&self, bail: &BailBacASable) -> Result<CibleEphemere, AppError> {
        // L'ordre n'est pas cosmétique : un apply sans init échoue sur un
        // module dont les fournisseurs ne sont pas téléchargés, et lire les
        // sorties avant l'apply rendrait celles du tour précédent.
        self.moteur.initialiser(&self.module)?;
        self.moteur.appliquer(&self.module, bail)?;
        let sorties = self.moteur.sorties(&self.module)?;

        let adresse = sorties
            .iter()
            .find(|(nom, _)| nom == &self.sortie_adresse)
            .map(|(_, valeur)| valeur.clone())
            .ok_or_else(|| AppError::Configuration {
                detail: format!(
                    "le module ne déclare aucune sortie « {} » : sans adresse, \
                     la campagne n'a pas de cible",
                    self.sortie_adresse
                ),
            })?;

        CibleEphemere::new(adresse, sorties)
    }
}

impl<M: MoteurTerraform> DestructeurBail for BacASableTerraform<M> {
    fn detruire(&self, _bail: &BailBacASable) -> Result<(), AppError> {
        // Idempotent par construction : `terraform destroy` sur un état déjà
        // vide rend « 0 destroyed » et réussit. La garde RAII et le chien de
        // garde peuvent donc réclamer la même destruction sans qu'un nettoyage
        // réussi se lise comme une panne.
        self.moteur.detruire(&self.module).map(|_| ())
    }
}
