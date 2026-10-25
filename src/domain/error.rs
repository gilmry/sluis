//! `AppError` — le type d'erreur unique du domaine.
//!
//! Deux règles héritées de `koprogo/CLAUDE.md`, non négociables :
//! `Result<_, AppError>` typé, jamais `Result<_, String>` ; et `unwrap()` /
//! `expect()` interdits hors tests.
//!
//! Une troisième règle est propre à Sluis : **aucune variante ne porte de
//! secret en clair**. Le champ qui devrait en contenir un est typé
//! [`Redacted`], sans quoi le secret fuirait par `Display` au premier
//! `?` remonté jusqu'à un journal.

use crate::domain::Redacted;

/// Erreur typée du domaine. Chaque variante porte un message utilisateur
/// correct : c'est la classe `@negative` de la discipline de test.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AppError {
    /// Un moteur d'exécution est absent de la machine.
    ///
    /// Cas nominal sur une machine de développement, pas une anomalie : les
    /// six binaires d'infrastructure peuvent manquer sans que rien ne panique.
    #[error("moteur absent : le binaire « {binaire} » est introuvable dans le PATH")]
    EngineMissing { binaire: String },

    /// Une action a été classée dans un tier qui ne lui convient pas.
    ///
    /// Typiquement : un plan de Tier 2 visant la production. Cette erreur ne
    /// devrait pratiquement jamais être construite, les invariants étant portés
    /// par le typage ; elle couvre les chemins que le typage ne peut pas fermer.
    #[error("violation de tier : {raison}")]
    TierViolation { raison: String },

    /// Un projet hors de la liste d'autorisation a été visé.
    ///
    /// Le refus a lieu **avant tout appel réseau**, et l'événement est
    /// journalisé comme événement de sécurité, pas comme simple erreur.
    #[error("projet non autorisé : « {projet} » n'est pas dans la liste d'autorisation")]
    ProjetNonAutorise { projet: String },

    /// Une lecture a tenté de sortir de la racine autorisée.
    #[error("chemin hors racine autorisée : « {chemin} »")]
    CheminHorsRacine { chemin: String },

    /// La configuration est absente, incomplète ou incohérente.
    #[error("configuration invalide : {detail}")]
    Configuration { detail: String },

    /// L'authentification auprès d'un service tiers a échoué.
    ///
    /// Le secret présenté est conservé pour le diagnostic mais **typé
    /// [`Redacted`]**, donc invisible dans `Display` comme dans `Debug`.
    #[error("échec d'authentification (secret masqué : {secret})")]
    Authentification { secret: Redacted<String> },

    /// Une ressource attendue est introuvable.
    #[error("introuvable : {quoi}")]
    Introuvable { quoi: String },

    /// L'environnement GitHub visé par un dispatch de Tier 1 n'offre pas la
    /// garde qu'ADR-008 suppose : absent, sans relecteur requis, ou qui laisse
    /// le titulaire du jeton approuver son propre dispatch.
    ///
    /// Le refus a lieu **avant le dispatch** : un environnement absent serait
    /// créé par GitHub sans protection au premier run, et le job passerait
    /// sans approbation humaine.
    #[error("dispatch Tier 1 refusé : l'environnement « {environnement} » {raison}")]
    EnvironnementNonProtege {
        environnement: String,
        raison: String,
    },

    /// Le workflow de Tier 1 du dépôt cible n'a pas, ou plus, la section
    /// `verification-empreinte` du gabarit (ADR-034, point 6).
    ///
    /// Le refus a lieu **avant le dispatch** : un workflow qui ne recalcule
    /// plus l'empreinte exécuterait, sur une approbation donnée, des entrées
    /// qui ne sont pas celles du plan approuvé.
    #[error("dispatch Tier 1 refusé : le workflow « {workflow} » {raison}")]
    WorkflowNonConforme { workflow: String, raison: String },

    /// Un module OpenTofu sort du contrat d'ADR-034 (point 3).
    ///
    /// Le refus a lieu **avant toute commande `tofu`** : un module qui y
    /// manque n'est ni initialisé, ni planifié, ni appliqué. Chaque motif
    /// nomme le fichier, la ligne et la construction refusée.
    #[error("module OpenTofu « {module} » refusé : {motifs}")]
    ModuleRefuse { module: String, motifs: String },

    /// Un service tiers a répondu une erreur.
    #[error("erreur du service {service} : {detail}")]
    ServiceTiers { service: String, detail: String },

    /// Une entrée n'a pas pu être analysée.
    #[error("analyse impossible de {quoi} : {detail}")]
    Analyse { quoi: String, detail: String },

    /// Une entrée-sortie a échoué.
    #[error("erreur d'entrée-sortie sur « {chemin} » : {detail}")]
    EntreeSortie { chemin: String, detail: String },
}

/// Alias de confort. Tout ce qui peut échouer dans Sluis rend ce type.
pub type Resultat<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_message_nomme_le_binaire_manquant() {
        let err = AppError::EngineMissing {
            binaire: "kustomize".to_string(),
        };
        assert!(err.to_string().contains("kustomize"));
    }

    #[test]
    fn aucune_variante_ne_rend_un_message_vide() {
        let err = AppError::Introuvable {
            quoi: "profil de cluster".to_string(),
        };
        assert!(!err.to_string().trim().is_empty());
    }
}
