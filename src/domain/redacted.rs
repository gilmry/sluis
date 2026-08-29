//! `Redacted<T>` — un secret qu'aucun chemin de sortie ne peut révéler.
//!
//! Le problème que ce type résout n'est pas « penser à masquer les secrets »,
//! c'est « ne plus avoir à y penser ». Un secret stocké dans un `String` fuit
//! au premier `{:?}` d'une structure englobante, au premier `tracing::debug!`,
//! à la première sérialisation d'une erreur. Ces fuites ne se voient pas en
//! revue : elles se produisent dans du code dérivé automatiquement.
//!
//! `Redacted<T>` déplace la garantie de la vigilance vers le typage. Les trois
//! traits qui rendent une valeur observable — `Display`, `Debug`, `Serialize` —
//! sont écrits à la main pour rendre un marqueur constant. Le seul accès à la
//! valeur passe par [`Redacted::exposer`], dont le nom rend l'intention
//! explicite en revue.
//!
//! Ce qui est délibérément **absent** : `Deref`, `AsRef`, `Into<T>`. Chacun
//! rouvrirait un chemin implicite vers la valeur.

use std::fmt;

/// Marqueur rendu à la place de la valeur, quel que soit le chemin de sortie.
///
/// Sa longueur est constante : deux secrets de tailles différentes rendent
/// exactement la même chose, sinon la longueur devient elle-même un canal.
pub const MARQUEUR: &str = "«redacted»";

/// Enveloppe un secret pour qu'il ne puisse être ni affiché, ni journalisé,
/// ni sérialisé par inadvertance.
///
/// ```
/// use sluis::domain::Redacted;
///
/// let cle = Redacted::new("valeur-sensible".to_string());
/// assert_eq!(format!("{cle}"), "«redacted»");
/// assert_eq!(cle.exposer(), "valeur-sensible");
/// ```
#[derive(Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(transparent)]
pub struct Redacted<T> {
    valeur: T,
}

impl<T> Redacted<T> {
    /// Enveloppe une valeur sensible.
    pub fn new(valeur: T) -> Self {
        Self { valeur }
    }

    /// Rend la valeur en clair.
    ///
    /// Le nom est volontairement explicite : un appel à `exposer()` doit
    /// sauter aux yeux en revue de code. C'est le seul chemin d'accès.
    pub fn exposer(&self) -> &T {
        &self.valeur
    }

    /// Consomme l'enveloppe et rend la valeur en clair.
    pub fn exposer_possede(self) -> T {
        self.valeur
    }
}

impl<T> fmt::Display for Redacted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(MARQUEUR)
    }
}

// Écrit à la main, jamais dérivé : un `#[derive(Debug)]` afficherait la valeur,
// et c'est précisément par le Debug d'une structure englobante que les secrets
// fuient en pratique.
impl<T> fmt::Debug for Redacted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(MARQUEUR)
    }
}

impl<T> serde::Serialize for Redacted<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(MARQUEUR)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_marqueur_est_de_longueur_constante() {
        let court = Redacted::new("a".to_string());
        let long = Redacted::new("a".repeat(10_000));
        assert_eq!(format!("{court}").len(), format!("{long}").len());
    }

    #[test]
    fn exposer_possede_rend_bien_la_valeur() {
        let secret = Redacted::new(42_u32);
        assert_eq!(secret.exposer_possede(), 42);
    }
}
