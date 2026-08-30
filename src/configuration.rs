//! Configuration de Sluis.
//!
//! Trois sources, dans cet ordre de priorité décroissante : les variables
//! d'environnement, le fichier de configuration, les valeurs par défaut.
//!
//! **Les secrets ne viennent que de l'environnement.** Un secret dans un
//! fichier de configuration finit dans un dépôt Git, c'est une question de
//! temps. Le fichier ne porte que des listes et des seuils.

use std::path::Path;

use serde::Deserialize;

use crate::domain::{AppError, Duree, ListeAutorisation, Redacted};
use crate::infrastructure::ovh::signature::IdentiteOvh;

/// Fichier de configuration, tel qu'il est lu sur disque.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FichierConfiguration {
    /// Section OVH.
    #[serde(default)]
    pub ovh: SectionOvh,
    /// Section bac à sable.
    #[serde(default)]
    pub bac_a_sable: SectionBacASable,
    /// Section journal.
    #[serde(default)]
    pub journal: SectionJournal,
}

/// Réglages OVH.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SectionOvh {
    /// Point d'entrée de l'API.
    #[serde(default = "endpoint_par_defaut")]
    pub endpoint: String,
    /// Projets portant de la production.
    #[serde(default)]
    pub projets_production: Vec<String>,
    /// Projets dédiés aux bacs à sable.
    #[serde(default)]
    pub projets_bac_a_sable: Vec<String>,
}

fn endpoint_par_defaut() -> String {
    crate::infrastructure::ovh::client::ENDPOINT_EU.to_string()
}

impl Default for SectionOvh {
    fn default() -> Self {
        Self {
            endpoint: endpoint_par_defaut(),
            projets_production: Vec::new(),
            projets_bac_a_sable: Vec::new(),
        }
    }
}

/// Réglages du bac à sable.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SectionBacASable {
    /// Durée de vie maximale d'un bail, en secondes.
    #[serde(default = "ttl_max_par_defaut")]
    pub ttl_maximal_secondes: i64,
    /// Plafond de dépense par campagne, dans la devise du compte.
    #[serde(default = "plafond_par_defaut")]
    pub plafond_depense: f64,
    /// Durée de la fenêtre de dérogation, en jours.
    ///
    /// Quatre-vingt-dix jours par décision du superviseur du 2026-08-29
    /// (ADR-007). Cette valeur est **paramétrable, non supprimable** : une
    /// fenêtre absente vaut fenêtre fermée.
    #[serde(default = "fenetre_par_defaut")]
    pub fenetre_derogation_jours: i64,

    /// Module Terraform jetable que loue une campagne.
    ///
    /// **Absent par défaut, et c'est ce qui fait la différence entre les deux
    /// déploiements** : sans lui, `sluis_campagne` n'est pas enregistré, donc
    /// le service n'expose que de la lecture. Le Sluis public n'a pas à le
    /// renseigner ; le Sluis exécutant, sur le réseau interne, l'a.
    #[serde(default)]
    pub module_terraform: Option<String>,

    /// Nom de la sortie du module qui porte l'adresse à charger.
    #[serde(default = "sortie_adresse_par_defaut")]
    pub sortie_adresse: String,

    /// Fichier où la fenêtre de dérogation est conservée, scellée.
    #[serde(default = "depot_derogation_par_defaut")]
    pub depot_derogation: String,
}

fn ttl_max_par_defaut() -> i64 {
    6 * 3600
}
fn plafond_par_defaut() -> f64 {
    20.0
}
fn fenetre_par_defaut() -> i64 {
    90
}
fn sortie_adresse_par_defaut() -> String {
    "vps_ip".to_string()
}
fn depot_derogation_par_defaut() -> String {
    "sluis-derogation.json".to_string()
}

impl Default for SectionBacASable {
    fn default() -> Self {
        Self {
            ttl_maximal_secondes: ttl_max_par_defaut(),
            plafond_depense: plafond_par_defaut(),
            fenetre_derogation_jours: fenetre_par_defaut(),
            module_terraform: None,
            sortie_adresse: sortie_adresse_par_defaut(),
            depot_derogation: depot_derogation_par_defaut(),
        }
    }
}

/// Réglages du journal d'audit.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SectionJournal {
    /// Chemin du fichier JSONL.
    #[serde(default = "journal_par_defaut")]
    pub chemin: String,
}

fn journal_par_defaut() -> String {
    "sluis-audit.jsonl".to_string()
}

impl Default for SectionJournal {
    fn default() -> Self {
        Self {
            chemin: journal_par_defaut(),
        }
    }
}

/// Configuration résolue, prête à l'emploi.
pub struct Configuration {
    /// Fichier lu.
    pub fichier: FichierConfiguration,
    /// Liste d'autorisation, disjonction vérifiée.
    pub autorisation: ListeAutorisation,
    /// Identité OVH, si les trois secrets sont présents.
    pub identite: Option<IdentiteOvh>,
}

impl Configuration {
    /// Charge la configuration.
    ///
    /// L'absence de fichier n'est pas une erreur : Sluis démarre alors sans
    /// aucun projet autorisé, donc capable de tout lire localement et de ne
    /// rien voir chez OVH. C'est le défaut le plus sûr.
    pub fn charger(chemin: Option<&Path>) -> Result<Self, AppError> {
        let fichier = match chemin {
            Some(chemin) if chemin.exists() => {
                let contenu =
                    std::fs::read_to_string(chemin).map_err(|e| AppError::EntreeSortie {
                        chemin: chemin.display().to_string(),
                        detail: e.to_string(),
                    })?;
                toml::from_str(&contenu).map_err(|e| AppError::Analyse {
                    quoi: chemin.display().to_string(),
                    detail: e.to_string(),
                })?
            }
            _ => FichierConfiguration::default(),
        };

        let autorisation = ListeAutorisation::new(
            fichier.ovh.projets_production.clone(),
            fichier.ovh.projets_bac_a_sable.clone(),
        )?;

        let identite = match (
            std::env::var("OVH_APPLICATION_KEY").ok(),
            std::env::var("OVH_APPLICATION_SECRET").ok(),
            std::env::var("OVH_CONSUMER_KEY").ok(),
        ) {
            (Some(cle), Some(secret), Some(consommateur))
                if !cle.is_empty() && !secret.is_empty() && !consommateur.is_empty() =>
            {
                Some(IdentiteOvh {
                    application_key: cle,
                    application_secret: Redacted::new(secret),
                    consumer_key: Redacted::new(consommateur),
                })
            }
            _ => None,
        };

        Ok(Self {
            fichier,
            autorisation,
            identite,
        })
    }

    /// Durée de la fenêtre de dérogation.
    pub fn fenetre_derogation(&self) -> Result<Duree, AppError> {
        Duree::jours(self.fichier.bac_a_sable.fenetre_derogation_jours)
    }

    /// Les valeurs secrètes connues, à effacer de toute sortie.
    pub fn secrets_connus(&self) -> Vec<Redacted<String>> {
        self.identite
            .as_ref()
            .map(|i| vec![i.application_secret.clone(), i.consumer_key.clone()])
            .unwrap_or_default()
    }
}
