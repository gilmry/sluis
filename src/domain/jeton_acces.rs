//! Jeton d'accès signé.
//!
//! **Jamais persisté côté serveur**, comme le JWT d'Elevia : il est signé, donc
//! vérifiable, et il expire vite. Il n'existe donc rien à voler dans le dépôt,
//! et rien à nettoyer.
//!
//! HMAC-SHA256 écrit à la main plutôt qu'une bibliothèque JWT : le format
//! complet de JWT apporte ici des algorithmes qu'on n'utilise pas et une
//! négociation qu'on ne veut pas — le fameux `alg: none` n'est un risque que
//! pour qui l'accepte. Le format ci-dessous n'accepte qu'un algorithme, parce
//! qu'il n'en connaît qu'un.

use sha2::{Digest, Sha256};

use crate::domain::{base64url_sans_remplissage, AppError, Horodatage, Portee, Redacted};

const BLOC: usize = 64;

/// HMAC-SHA256, écrit à la main (RFC 2104).
pub fn hmac_sha256(cle: &[u8], message: &[u8]) -> [u8; 32] {
    let mut cle_normalisee = [0u8; BLOC];
    if cle.len() > BLOC {
        let mut h = Sha256::new();
        h.update(cle);
        cle_normalisee[..32].copy_from_slice(&h.finalize());
    } else {
        cle_normalisee[..cle.len()].copy_from_slice(cle);
    }

    let mut interne = [0x36u8; BLOC];
    let mut externe = [0x5cu8; BLOC];
    for i in 0..BLOC {
        interne[i] ^= cle_normalisee[i];
        externe[i] ^= cle_normalisee[i];
    }

    let mut h = Sha256::new();
    h.update(interne);
    h.update(message);
    let intermediaire = h.finalize();

    let mut h = Sha256::new();
    h.update(externe);
    h.update(intermediaire);
    let mut sortie = [0u8; 32];
    sortie.copy_from_slice(&h.finalize());
    sortie
}

/// Ce qu'un jeton d'accès affirme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revendications {
    /// Utilisateur.
    pub sujet: String,
    /// Client MCP qui a obtenu le jeton.
    pub client_id: String,
    /// Portées accordées.
    pub portees: Vec<Portee>,
    /// Instant d'expiration.
    pub expire_le: Horodatage,
}

/// Émet un jeton d'accès signé.
///
/// Format : `charge_utile.signature`, tous deux en base64url sans remplissage.
/// La charge utile est lisible — ce n'est pas un secret, c'est une affirmation
/// signée — mais elle n'est pas modifiable sans la clé.
pub fn emettre(revendications: &Revendications, secret: &Redacted<String>) -> String {
    let charge = format!(
        "{}|{}|{}|{}",
        revendications.sujet,
        revendications.client_id,
        revendications
            .portees
            .iter()
            .map(|p| p.nom())
            .collect::<Vec<_>>()
            .join(","),
        revendications.expire_le.secondes()
    );
    let encodee = base64url_sans_remplissage(charge.as_bytes());
    let signature = hmac_sha256(secret.exposer().as_bytes(), encodee.as_bytes());
    format!("{encodee}.{}", base64url_sans_remplissage(&signature))
}

/// Vérifie un jeton et rend ses revendications.
///
/// Vérifie la signature **avant** de lire quoi que ce soit de la charge utile :
/// interpréter d'abord et vérifier ensuite reviendrait à faire confiance à une
/// entrée non authentifiée.
pub fn verifier(
    jeton: &str,
    secret: &Redacted<String>,
    maintenant: Horodatage,
) -> Result<Revendications, AppError> {
    let echec = || AppError::Authentification {
        secret: Redacted::new("jeton d'accès invalide".to_string()),
    };

    let (encodee, signature_recue) = jeton.split_once('.').ok_or_else(echec)?;
    let attendue = base64url_sans_remplissage(&hmac_sha256(
        secret.exposer().as_bytes(),
        encodee.as_bytes(),
    ));
    if !comparaison_constante(attendue.as_bytes(), signature_recue.as_bytes()) {
        return Err(echec());
    }

    let brut = decoder_base64url(encodee).ok_or_else(echec)?;
    let charge = String::from_utf8(brut).map_err(|_| echec())?;
    let morceaux: Vec<&str> = charge.split('|').collect();
    if morceaux.len() != 4 {
        return Err(echec());
    }
    let expire_le = Horodatage::new(morceaux[3].parse::<i64>().map_err(|_| echec())?);
    if maintenant.apres(expire_le) {
        return Err(AppError::Authentification {
            secret: Redacted::new("jeton d'accès expiré".to_string()),
        });
    }
    Ok(Revendications {
        sujet: morceaux[0].to_string(),
        client_id: morceaux[1].to_string(),
        portees: morceaux[2].split(',').filter_map(Portee::depuis).collect(),
        expire_le,
    })
}

fn comparaison_constante(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut difference = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        difference |= x ^ y;
    }
    difference == 0
}

/// Décode du base64url sans remplissage.
pub fn decoder_base64url(entree: &str) -> Option<Vec<u8>> {
    let valeur = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => (c - b'A') as u32,
            b'a'..=b'z' => (c - b'a') as u32 + 26,
            b'0'..=b'9' => (c - b'0') as u32 + 52,
            b'-' => 62,
            b'_' => 63,
            _ => return None,
        })
    };
    let octets = entree.as_bytes();
    let mut sortie = Vec::with_capacity(entree.len() * 3 / 4);
    for morceau in octets.chunks(4) {
        if morceau.len() < 2 {
            return None;
        }
        let mut accumulateur = 0u32;
        for (index, octet) in morceau.iter().enumerate() {
            accumulateur |= valeur(*octet)? << (18 - 6 * index);
        }
        sortie.push((accumulateur >> 16) as u8);
        if morceau.len() > 2 {
            sortie.push((accumulateur >> 8) as u8);
        }
        if morceau.len() > 3 {
            sortie.push(accumulateur as u8);
        }
    }
    Some(sortie)
}
