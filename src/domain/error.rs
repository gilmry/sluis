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
