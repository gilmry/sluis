//! Port de persistance de la fenêtre de dérogation.

use crate::domain::{AppError, Empreinte, FenetreDerogation};

/// Conserve la fenêtre de dérogation en vigueur.
///
/// **Une absence de fenêtre n'est pas une erreur** : c'est l'état par défaut,
/// et il interdit tout bail. C'est la septième condition d'ADR-007, celle qui
/// fait qu'une dérogation ne devient jamais permanente par simple inertie.
pub trait DepotDerogation: Send + Sync {
    /// Fenêtre en vigueur, s'il en existe une d'authentique.
    fn courante(&self) -> Result<Option<FenetreDerogation>, AppError>;

    /// Enregistre une fenêtre nouvellement ouverte.
    fn enregistrer(&self, fenetre: &FenetreDerogation) -> Result<(), AppError>;
}

/// Retient les empreintes de renouvellement déjà consommées.
///
/// Une approbation ne rouvre la fenêtre **qu'une fois** : sans ce registre, la
/// même demande approuvée, présentée deux fois, prolongerait la dérogation
/// sans qu'aucun humain ait eu à la redécider.
pub trait RegistreRenouvellements: Send + Sync {
    /// Vrai si l'empreinte a déjà ouvert une fenêtre.
    fn deja_consommee(&self, empreinte: &Empreinte) -> Result<bool, AppError>;

    /// Consigne une empreinte, **avant** l'ouverture de la fenêtre.
    fn consigner(&self, empreinte: &Empreinte) -> Result<(), AppError>;
}
