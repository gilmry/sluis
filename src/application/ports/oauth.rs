//! Port du dépôt OAuth.

use crate::domain::{AppError, ClientOAuth, CodeAutorisation, JetonRafraichissement};

/// Persiste les clients, codes et jetons.
///
/// La méthode qui compte est [`DepotOAuth::consommer_code`] : elle doit rendre
/// le code **et le retirer**, en une opération atomique. Un dépôt qui rendrait
/// le code puis attendrait un appel de suppression laisserait une fenêtre où
/// deux échanges concurrents réussiraient tous les deux.
pub trait DepotOAuth: Send + Sync {
    /// Enregistre un client.
    fn enregistrer_client(&self, client: ClientOAuth) -> Result<(), AppError>;
    /// Retrouve un client.
    fn client(&self, client_id: &str) -> Result<Option<ClientOAuth>, AppError>;

    /// Dépose un code d'autorisation.
    fn deposer_code(&self, code_clair: &str, code: CodeAutorisation) -> Result<(), AppError>;
    /// Retire et rend un code, **atomiquement**.
    fn consommer_code(&self, code_clair: &str) -> Result<Option<CodeAutorisation>, AppError>;

    /// Dépose un jeton de rafraîchissement.
    fn deposer_jeton(&self, jeton: JetonRafraichissement) -> Result<(), AppError>;
    /// Remplace un jeton par sa version révoquée, atomiquement, et rend
    /// l'ancienne. La rotation est ainsi indissociable de la lecture.
    fn tourner_jeton(&self, empreinte: &str) -> Result<Option<JetonRafraichissement>, AppError>;

    /// Retire les codes expirés.
    fn nettoyer(&self, maintenant: crate::domain::Horodatage) -> Result<usize, AppError>;
}
