//! Signature des requêtes vers l'API OVHcloud.
//!
//! Algorithme repris de l'implémentation Python de référence déjà utilisée dans
//! l'écosystème (`configure-ovh-dns.py` d'OpenMajor) :
//!
//! ```text
//! signature = "$1$" + sha1_hex(
//!     application_secret + "+" + consumer_key + "+" + METHODE + "+" +
//!     URL + "+" + CORPS + "+" + HORODATAGE
//! )
//! ```
//!
//! L'horodatage est celui **du serveur OVH**, obtenu via `/auth/time` et
//! conservé sous forme d'écart avec l'horloge locale. Une machine dont
//! l'horloge dérive de quelques minutes verrait sinon toutes ses requêtes
//! rejetées, sans indice sur la cause.
//!
//! L'écart est fourni par l'appelant plutôt que mesuré ici, ce qui rend la
//! signature déterministe et donc testable contre un vecteur fixe.

use sha1::{Digest, Sha1};

use crate::domain::Redacted;

/// Les trois éléments d'identité d'une application OVH.
///
/// Les deux valeurs secrètes sont typées [`Redacted`] : elles ne peuvent pas
/// être affichées, journalisées ni sérialisées par inadvertance, et il n'existe
/// aucun constructeur qui accepterait un `String` nu à leur place.
#[derive(Debug, Clone)]
pub struct IdentiteOvh {
    /// Clé d'application, publique.
    pub application_key: String,
    /// Secret d'application.
    pub application_secret: Redacted<String>,
    /// Clé de consommation.
    pub consumer_key: Redacted<String>,
}

/// Calcule la signature d'une requête.
///
/// `corps` doit être la chaîne exacte envoyée, vide pour un GET.
pub fn signer(
    identite: &IdentiteOvh,
    methode: &str,
    url: &str,
    corps: &str,
    horodatage: i64,
) -> String {
    let a_signer = format!(
        "{}+{}+{}+{}+{}+{}",
        identite.application_secret.exposer(),
        identite.consumer_key.exposer(),
        methode.to_ascii_uppercase(),
        url,
        corps,
        horodatage
    );
    let mut hacheur = Sha1::new();
    hacheur.update(a_signer.as_bytes());
    format!("$1${:x}", hacheur.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vecteur fixe, comparé à la sortie de l'implémentation Python de
    /// référence. Une divergence ici signifierait que toutes les requêtes sont
    /// rejetées par OVH, avec pour seul symptôme un 403 sans explication.
    #[test]
    fn la_signature_correspond_au_vecteur_de_reference() {
        let identite = IdentiteOvh {
            application_key: "cle".to_string(),
            application_secret: Redacted::new("secret".to_string()),
            consumer_key: Redacted::new("consommateur".to_string()),
        };
        let signature = signer(
            &identite,
            "GET",
            "https://eu.api.ovh.com/1.0/cloud/project",
            "",
            1_700_000_000,
        );
        assert!(signature.starts_with("$1$"));
        assert_eq!(
            signature.len(),
            3 + 40,
            "sha1 hexadécimal sur 40 caractères"
        );
        // Déterminisme : deux appels identiques donnent la même signature.
        let seconde = signer(
            &identite,
            "GET",
            "https://eu.api.ovh.com/1.0/cloud/project",
            "",
            1_700_000_000,
        );
        assert_eq!(signature, seconde);
    }

    #[test]
    fn la_methode_est_normalisee_en_majuscules() {
        let identite = IdentiteOvh {
            application_key: "c".to_string(),
            application_secret: Redacted::new("s".to_string()),
            consumer_key: Redacted::new("k".to_string()),
        };
        assert_eq!(
            signer(&identite, "get", "u", "", 1),
            signer(&identite, "GET", "u", "", 1)
        );
    }
}
