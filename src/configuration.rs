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
    /// Section OpenTofu : ce qu'un module doit respecter pour être lancé.
    #[serde(default)]
    pub tofu: SectionTofu,
    /// Section GitHub : la passerelle d'approbation (ADR-008, ADR-012).
    ///
    /// **Absente par défaut** : sans elle, aucune passerelle n'est composée et
    /// le serveur ne peut rien soumettre.
    #[serde(default)]
    pub github: Option<SectionGithub>,
}

/// Réglages de la passerelle GitHub.
///
/// Le jeton n'y figure pas : il vient de `SLUIS_GITHUB_TOKEN`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SectionGithub {
    /// Propriétaire du dépôt cible.
    pub proprietaire: String,
    /// Dépôt cible, celui qui porte le workflow de Tier 1.
    pub depot: String,
    /// Passerelle d'approbation : `environnement-protege` (ADR-008, le
    /// défaut) ou `etiquette-signee` (ADR-013, pour un dépôt privé).
    #[serde(default)]
    pub passerelle: TypePasserelle,
    /// Fichier du workflow de Tier 1, par exemple `tier1-apply.yml`.
    ///
    /// Requis par la passerelle `environnement-protege`, ignoré par
    /// `etiquette-signee`, qui ne déclenche aucun workflow.
    #[serde(default)]
    pub workflow: Option<String>,
    /// Branche sur laquelle le workflow est déclenché.
    #[serde(default = "branche_par_defaut")]
    pub branche: String,
    /// Point d'entrée de l'API.
    #[serde(default = "api_github_par_defaut")]
    pub api: String,
    /// Noms des check-runs qui portent les gates du plancher.
    ///
    /// Absent : aucun vérificateur n'est composé, donc aucune mise en ligne
    /// ne peut être demandée.
    #[serde(default)]
    pub gates: Option<SectionGatesGithub>,
}

/// Les deux passerelles d'approbation de Tier 1.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TypePasserelle {
    /// Environnement GitHub protégé à relecteurs requis (ADR-008). Le défaut.
    #[default]
    EnvironnementProtege,
    /// Étiquette git signée par le superviseur (ADR-013). Le jeton n'y sert
    /// qu'à lire : aucun workflow déclenché, aucune étiquette poussée, aucune
    /// clé de signature lue ni configurée dans Sluis.
    EtiquetteSignee,
}

/// Les gates du plancher qu'exige ce déploiement.
///
/// Les **noms** des check-runs ne se configurent pas : ils sont figés par le
/// contrat du dépôt cible (ADR-034, `contrat_depot::check_runs`). Seul
/// l'archétype se déclare, et il est **requis** : un oubli ne doit pas choisir
/// à la place du superviseur l'absence de la gate de retour de migration.
///
/// L'ancienne forme, qui nommait chaque check-run (`secrets = "Secrets"`), est
/// refusée au chargement plutôt que silencieusement ignorée.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SectionGatesGithub {
    /// `stateless` : les trois gates communes. `stateful` : plus le retour de
    /// migration.
    pub archetype: ArchetypeGates,
}

/// L'archétype du dépôt cible, qui fixe les gates exigées.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArchetypeGates {
    /// Sans état : secrets, sbom, scan-conteneur.
    Stateless,
    /// Avec état : les trois, plus retour-migration.
    Stateful,
}

/// Les clés de l'ancienne forme de `[github.gates]`, qui nommait les check-runs.
const ANCIENNES_CLES_GATES: &[&str] = &["secrets", "sbom", "scan_conteneur", "retour_migration"];

/// Refuse l'ancienne forme de `[github.gates]` avec un message qui dit quoi
/// faire, plutôt que l'erreur générique d'un champ inconnu. Un fichier qui ne
/// se lit pas comme du TOML est laissé à l'analyse typée, qui le dira.
fn refuser_noms_de_check_runs(contenu: &str) -> Result<(), AppError> {
    let Ok(table) = contenu.parse::<toml::Table>() else {
        return Ok(());
    };
    let gates = table
        .get("github")
        .and_then(|github| github.get("gates"))
        .and_then(toml::Value::as_table);
    let Some(gates) = gates else {
        return Ok(());
    };
    if let Some(cle) = ANCIENNES_CLES_GATES
        .iter()
        .find(|c| gates.contains_key(**c))
    {
        return Err(AppError::Configuration {
            detail: format!(
                "[github.gates] : « {cle} » ne se configure plus, les noms des check-runs sont \
                 figés par le contrat du dépôt cible (ADR-034, contrat/depot-cible.v1.json) ; \
                 déclarer seulement archetype = \"stateless\" ou \"stateful\""
            ),
        });
    }
    Ok(())
}

fn branche_par_defaut() -> String {
    "main".to_string()
}
fn api_github_par_defaut() -> String {
    "https://api.github.com".to_string()
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

    /// Racines d'infrastructure que ce serveur a le droit de mesurer.
    ///
    /// **Vide par défaut, et c'est ce qui fait la différence entre les deux
    /// déploiements** : sans dépôt autorisé, `sluis_campagne` n'est pas
    /// enregistré, donc le service n'expose que de la lecture. Le Sluis public
    /// n'en déclare aucun ; le Sluis exécutant, sur le réseau interne, en
    /// déclare.
    ///
    /// Le module Terraform, lui, ne se configure pas : il vient de la
    /// déclaration de charge du dépôt mesuré, `_shared/charge.yaml`.
    #[serde(default)]
    pub depots_autorises: Vec<String>,

    /// Fichier où la fenêtre de dérogation est conservée, scellée.
    #[serde(default = "depot_derogation_par_defaut")]
    pub depot_derogation: String,

    /// Fichier des empreintes de renouvellement déjà consommées : une
    /// approbation ne rouvre la fenêtre qu'une fois.
    #[serde(default = "registre_renouvellements_par_defaut")]
    pub registre_renouvellements: String,
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
fn depot_derogation_par_defaut() -> String {
    "sluis-derogation.json".to_string()
}
fn registre_renouvellements_par_defaut() -> String {
    "sluis-renouvellements.json".to_string()
}

impl Default for SectionBacASable {
    fn default() -> Self {
        Self {
            ttl_maximal_secondes: ttl_max_par_defaut(),
            plafond_depense: plafond_par_defaut(),
            fenetre_derogation_jours: fenetre_par_defaut(),
            depots_autorises: Vec::new(),
            depot_derogation: depot_derogation_par_defaut(),
            registre_renouvellements: registre_renouvellements_par_defaut(),
        }
    }
}

/// Réglages d'OpenTofu (ADR-034, point 3).
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SectionTofu {
    /// Fournisseurs qu'un module a le droit de verrouiller, adresse complète
    /// telle qu'elle figure dans `.terraform.lock.hcl`, par exemple
    /// `registry.opentofu.org/terraform-provider-openstack/openstack`.
    ///
    /// **Vide par défaut, donc tout module est refusé.** Son contenu initial
    /// est un point ouvert d'ADR-034, pas une valeur que le code choisit.
    #[serde(default)]
    pub fournisseurs_autorises: Vec<String>,

    /// Variables de l'environnement de Sluis transmises à `tofu` (ADR-034,
    /// point 4), par exemple `OVH_ENDPOINT`.
    ///
    /// `tofu` est lancé avec un environnement **vidé**, puis rempli d'un
    /// `PATH` fixe et des seules variables listées ici. **Vide par défaut** :
    /// aucun secret de Sluis, `SLUIS_GITHUB_TOKEN` compris, n'atteint un
    /// fournisseur sans avoir été nommé.
    #[serde(default)]
    pub variables_transmises: Vec<String>,
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
    /// Jeton GitHub de la passerelle, si `SLUIS_GITHUB_TOKEN` est non vide.
    pub jeton_github: Option<Redacted<String>>,
}

impl Configuration {
    /// Charge la configuration.
    ///
    /// L'absence de fichier n'est pas une erreur : Sluis démarre alors sans
    /// aucun projet autorisé, donc capable de tout lire localement et de ne
    /// rien voir chez OVH. C'est le défaut le plus sûr.
    pub fn charger(chemin: Option<&Path>) -> Result<Self, AppError> {
        Self::charger_avec(chemin, |nom| std::env::var(nom).ok())
    }

    /// Charge la configuration en lisant les secrets par `lire_env`.
    ///
    /// Sépare la lecture de l'environnement du reste, pour que les tests
    /// n'aient pas à modifier les variables du processus, partagées entre
    /// tests parallèles.
    pub fn charger_avec(
        chemin: Option<&Path>,
        lire_env: impl Fn(&str) -> Option<String>,
    ) -> Result<Self, AppError> {
        let fichier = match chemin {
            Some(chemin) if chemin.exists() => {
                let contenu =
                    std::fs::read_to_string(chemin).map_err(|e| AppError::EntreeSortie {
                        chemin: chemin.display().to_string(),
                        detail: e.to_string(),
                    })?;
                refuser_noms_de_check_runs(&contenu)?;
                toml::from_str(&contenu).map_err(|e| AppError::Analyse {
                    quoi: chemin.display().to_string(),
                    detail: e.to_string(),
                })?
            }
            _ => FichierConfiguration::default(),
        };

        if let Some(github) = &fichier.github {
            if github.passerelle == TypePasserelle::EnvironnementProtege
                && github
                    .workflow
                    .as_deref()
                    .is_none_or(|w| w.trim().is_empty())
            {
                return Err(AppError::Configuration {
                    detail: "[github] : la passerelle environnement-protege exige `workflow`"
                        .to_string(),
                });
            }
        }

        let autorisation = ListeAutorisation::new(
            fichier.ovh.projets_production.clone(),
            fichier.ovh.projets_bac_a_sable.clone(),
        )?;

        let identite = match (
            lire_env("OVH_APPLICATION_KEY"),
            lire_env("OVH_APPLICATION_SECRET"),
            lire_env("OVH_CONSUMER_KEY"),
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

        let jeton_github = lire_env("SLUIS_GITHUB_TOKEN")
            .filter(|jeton| !jeton.trim().is_empty())
            .map(Redacted::new);

        Ok(Self {
            fichier,
            autorisation,
            identite,
            jeton_github,
        })
    }

    /// Durée de la fenêtre de dérogation.
    pub fn fenetre_derogation(&self) -> Result<Duree, AppError> {
        Duree::jours(self.fichier.bac_a_sable.fenetre_derogation_jours)
    }

    /// Les valeurs secrètes connues, à effacer de toute sortie.
    pub fn secrets_connus(&self) -> Vec<Redacted<String>> {
        let mut secrets: Vec<Redacted<String>> = self
            .identite
            .as_ref()
            .map(|i| vec![i.application_secret.clone(), i.consumer_key.clone()])
            .unwrap_or_default();
        secrets.extend(self.jeton_github.clone());
        secrets
    }
}
