//! Registre des outils MCP.
//!
//! Le registre refuse au **démarrage** un outil sans schéma, et non à l'appel.
//! La différence compte : un refus à l'appel ne se découvre qu'en production,
//! sur l'appel qui échoue ; un refus au démarrage empêche le service de partir.

use crate::domain::AppError;
use crate::infrastructure::mcp::Outil;

/// Registre des outils exposés par `tools/list`.
///
/// L'énumérabilité est ce qui rend les contract tests exhaustifs : ils
/// parcourent le registre plutôt que d'échantillonner une liste écrite à la
/// main, qui vieillirait dès le premier outil ajouté.
#[derive(Default)]
pub struct RegistreOutils {
    outils: Vec<Box<dyn Outil>>,
}

impl RegistreOutils {
    /// Registre vide.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enregistre un outil.
    ///
    /// Refuse un nom vide, un schéma absent ou un doublon. Ces trois refus ont
    /// lieu au démarrage : un service qui part est un service dont le contrat
    /// tient.
    pub fn enregistrer(&mut self, outil: Box<dyn Outil>) -> Result<(), AppError> {
        if outil.nom().trim().is_empty() {
            return Err(AppError::Configuration {
                detail: "outil sans nom".to_string(),
            });
        }
        if self.outils.iter().any(|o| o.nom() == outil.nom()) {
            return Err(AppError::Configuration {
                detail: format!("outil « {} » déjà enregistré", outil.nom()),
            });
        }
        let schema = outil.schema();
        if !schema.is_object() {
            return Err(AppError::Configuration {
                detail: format!("outil « {} » sans schéma exploitable", outil.nom()),
            });
        }
        self.outils.push(outil);
        Ok(())
    }

    /// Tous les outils enregistrés, pour `tools/list` comme pour les tests.
    pub fn outils(&self) -> &[Box<dyn Outil>] {
        &self.outils
    }

    /// Retrouve un outil par son nom.
    pub fn trouver(&self, nom: &str) -> Option<&dyn Outil> {
        self.outils
            .iter()
            .find(|o| o.nom() == nom)
            .map(|o| o.as_ref())
    }

    /// Nombre d'outils enregistrés.
    pub fn len(&self) -> usize {
        self.outils.len()
    }

    /// Vrai si aucun outil n'est enregistré.
    pub fn is_empty(&self) -> bool {
        self.outils.is_empty()
    }
}
