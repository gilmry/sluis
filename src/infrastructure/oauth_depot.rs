//! Dépôt OAuth persisté sur fichier.
//!
//! **Écart assumé par rapport à ADR-006**, qui prévoyait PostgreSQL via `sqlx`
//! en CQRS SQL pur. Deux raisons, à réévaluer si l'usage change :
//!
//! - Sluis sert un superviseur et ses projets, pas des organisations tierces.
//!   Le volume est de l'ordre de quelques clients et quelques jetons.
//! - `sqlx` avec vérification à la compilation exige une base vivante au build,
//!   ce qui contredirait NFR-06 : aucun test ne doit dépendre d'un service
//!   absent.
//!
//! Ce que ce choix **ne relâche pas** : la révocation reste durable, écrite
//! avant tout retour ; la consommation d'un code reste atomique, retrait et
//! lecture ayant lieu sous le même verrou. Le port `DepotOAuth` rend la bascule
//! vers PostgreSQL sans effet sur le reste du code, le jour où le volume
//! l'exigera.
//!
//! Les codes sont indexés par leur **empreinte** et non par leur valeur : un
//! fichier lu ne livre aucun code utilisable.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::application::ports::DepotOAuth;
use crate::domain::{
    empreinte_sha256, AppError, ClientOAuth, CodeAutorisation, Horodatage, JetonRafraichissement,
    Redacted,
};

#[derive(Default, Serialize, Deserialize)]
struct Contenu {
    clients: HashMap<String, ClientOAuth>,
    codes: HashMap<String, CodeAutorisation>,
    jetons: HashMap<String, JetonRafraichissement>,
}

/// Dépôt OAuth persisté dans un fichier JSON.
pub struct DepotOAuthFichier {
    chemin: PathBuf,
    contenu: Mutex<Contenu>,
}

impl std::fmt::Debug for DepotOAuthFichier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DepotOAuthFichier")
            .field("chemin", &self.chemin)
            .finish_non_exhaustive()
    }
}

impl DepotOAuthFichier {
    /// Ouvre ou crée le dépôt.
    pub fn ouvrir(chemin: PathBuf) -> Result<Self, AppError> {
        let contenu = if chemin.exists() {
            let brut = std::fs::read_to_string(&chemin).map_err(|e| AppError::EntreeSortie {
                chemin: chemin.display().to_string(),
                detail: e.to_string(),
            })?;
            serde_json::from_str(&brut).map_err(|e| AppError::Analyse {
                quoi: chemin.display().to_string(),
                detail: e.to_string(),
            })?
        } else {
            Contenu::default()
        };
        Ok(Self {
            chemin,
            contenu: Mutex::new(contenu),
        })
    }

    fn ecrire(&self, contenu: &Contenu) -> Result<(), AppError> {
        // Fichier temporaire puis renommage : une coupure au milieu ne doit pas
        // laisser un dépôt tronqué, ce qui reviendrait à perdre toutes les
        // révocations d'un coup.
        let temporaire = self.chemin.with_extension("tmp");
        let serialise = serde_json::to_string_pretty(contenu).map_err(|e| AppError::Analyse {
            quoi: "dépôt OAuth".to_string(),
            detail: e.to_string(),
        })?;
        std::fs::write(&temporaire, serialise).map_err(|e| AppError::EntreeSortie {
            chemin: temporaire.display().to_string(),
            detail: e.to_string(),
        })?;
        std::fs::rename(&temporaire, &self.chemin).map_err(|e| AppError::EntreeSortie {
            chemin: self.chemin.display().to_string(),
            detail: e.to_string(),
        })
    }

    fn verrou(&self) -> Result<std::sync::MutexGuard<'_, Contenu>, AppError> {
        self.contenu.lock().map_err(|_| AppError::Configuration {
            detail: "verrou du dépôt OAuth empoisonné".to_string(),
        })
    }
}

impl DepotOAuth for DepotOAuthFichier {
    fn enregistrer_client(&self, client: ClientOAuth) -> Result<(), AppError> {
        let mut contenu = self.verrou()?;
        contenu
            .clients
            .insert(client.client_id().to_string(), client);
        self.ecrire(&contenu)
    }

    fn client(&self, client_id: &str) -> Result<Option<ClientOAuth>, AppError> {
        Ok(self.verrou()?.clients.get(client_id).cloned())
    }

    fn deposer_code(&self, code_clair: &str, code: CodeAutorisation) -> Result<(), AppError> {
        let mut contenu = self.verrou()?;
        contenu.codes.insert(empreinte_sha256(code_clair), code);
        self.ecrire(&contenu)
    }

    fn consommer_code(&self, code_clair: &str) -> Result<Option<CodeAutorisation>, AppError> {
        let mut contenu = self.verrou()?;
        // Retrait et lecture sous le même verrou : deux échanges concurrents du
        // même code ne peuvent pas réussir tous les deux.
        let retire = contenu.codes.remove(&empreinte_sha256(code_clair));
        if retire.is_some() {
            self.ecrire(&contenu)?;
        }
        Ok(retire)
    }

    fn deposer_jeton(&self, jeton: JetonRafraichissement) -> Result<(), AppError> {
        let mut contenu = self.verrou()?;
        contenu.jetons.insert(jeton.empreinte().to_string(), jeton);
        self.ecrire(&contenu)
    }

    fn tourner_jeton(&self, empreinte: &str) -> Result<Option<JetonRafraichissement>, AppError> {
        let mut contenu = self.verrou()?;
        let Some(jeton) = contenu.jetons.remove(empreinte) else {
            return Ok(None);
        };
        if jeton.revoque() {
            // Réinscrit tel quel : un jeton déjà révoqué doit le rester, et sa
            // disparition du dépôt le rendrait indétectable au prochain rejeu.
            contenu.jetons.insert(empreinte.to_string(), jeton);
            self.ecrire(&contenu)?;
            return Err(AppError::Authentification {
                secret: Redacted::new("jeton de rafraîchissement déjà utilisé".to_string()),
            });
        }
        self.ecrire(&contenu)?;
        Ok(Some(jeton))
    }

    fn nettoyer(&self, maintenant: Horodatage) -> Result<usize, AppError> {
        let mut contenu = self.verrou()?;
        let avant = contenu.codes.len();
        contenu
            .codes
            .retain(|_, c| !maintenant.apres(c.expire_le()));
        let retires = avant - contenu.codes.len();
        if retires > 0 {
            self.ecrire(&contenu)?;
        }
        Ok(retires)
    }
}
