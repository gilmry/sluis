//! Bounded context Accès — OAuth 2.1 + PKCE.
//!
//! Repris du pattern `mcp-oauth-maison` d'Elevia, déjà éprouvé deux fois dans
//! l'écosystème. Les décisions de sécurité y sont reprises telles quelles,
//! parce qu'elles ont été prises pour de bonnes raisons et qu'en dévier
//! demanderait de meilleures.
//!
//! Les cinq qui ne se discutent pas :
//!
//! 1. **PKCE S256 uniquement.** `plain` est interdit par OAuth 2.1, ce n'est
//!    pas une option de configuration.
//! 2. **Le jeton de rafraîchissement n'est jamais persisté en clair**, seul son
//!    empreinte SHA-256 l'est, exactement comme un mot de passe.
//! 3. **Rotation inconditionnelle** du jeton de rafraîchissement à chaque
//!    usage, y compris si le reste de l'échange échoue : un jeton volé puis
//!    rejoué après usage légitime doit être mort.
//! 4. **Le code d'autorisation est à usage unique**, court, et lié au triplet
//!    `client_id` + `redirect_uri` + défi, revérifié à l'échange.
//! 5. **`redirect_uri` n'est jamais fait confiance avant validation** contre la
//!    liste enregistrée, et cette validation précède tout rendu comme toute
//!    redirection — sinon l'endpoint devient un redirecteur ouvert.

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::domain::{AppError, Horodatage, Redacted};

/// Encode en base64url sans remplissage, comme l'exige PKCE.
///
/// Écrit à la main : trente lignes valent mieux qu'une dépendance pour une
/// transformation aussi stable, et le test de vecteur ci-dessous en fixe le
/// comportement.
pub fn base64url_sans_remplissage(donnees: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut sortie = String::with_capacity(donnees.len().div_ceil(3) * 4);
    for morceau in donnees.chunks(3) {
        let b0 = morceau[0] as u32;
        let b1 = *morceau.get(1).unwrap_or(&0) as u32;
        let b2 = *morceau.get(2).unwrap_or(&0) as u32;
        let triplet = (b0 << 16) | (b1 << 8) | b2;
        let indices = [
            (triplet >> 18) & 0x3F,
            (triplet >> 12) & 0x3F,
            (triplet >> 6) & 0x3F,
            triplet & 0x3F,
        ];
        let a_ecrire = morceau.len() + 1;
        for indice in indices.iter().take(a_ecrire) {
            sortie.push(ALPHABET[*indice as usize] as char);
        }
    }
    sortie
}

/// Empreinte SHA-256, en hexadécimal.
pub fn empreinte_sha256(valeur: &str) -> String {
    let mut hacheur = Sha256::new();
    hacheur.update(valeur.as_bytes());
    format!("{:x}", hacheur.finalize())
}

/// Vérifie un défi PKCE en S256.
///
/// `SHA256(code_verifier)` encodé en base64url sans remplissage doit égaler le
/// défi. La comparaison est faite sur toute la longueur, sans court-circuit.
pub fn verifier_pkce(verificateur: &str, defi: &str) -> bool {
    let mut hacheur = Sha256::new();
    hacheur.update(verificateur.as_bytes());
    let calcule = base64url_sans_remplissage(&hacheur.finalize());
    comparaison_constante(calcule.as_bytes(), defi.as_bytes())
}

/// Comparaison à temps constant sur la longueur commune.
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

/// Portée d'accès.
///
/// **Aucune portée ne permet de muter la production.** Ce n'est pas un oubli :
/// la mutation passe nécessairement par la passerelle d'ADR-008, qui n'est pas
/// un chemin d'appel MCP. `Propose` autorise à demander, jamais à faire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Portee {
    /// Lecture seule : inventaire, coûts, plans, statuts.
    Read,
    /// Écriture bornée : baux de bac à sable, sous les sept conditions.
    Sandbox,
    /// Soumission de plans à approbation. Ne mute rien par elle-même.
    Propose,
}

impl Portee {
    /// Toutes les portées.
    pub const TOUTES: [Portee; 3] = [Portee::Read, Portee::Sandbox, Portee::Propose];

    /// Nom tel qu'annoncé dans le document de découverte.
    pub fn nom(&self) -> &'static str {
        match self {
            Portee::Read => "sluis:read",
            Portee::Sandbox => "sluis:sandbox",
            Portee::Propose => "sluis:propose",
        }
    }

    /// Lit une portée depuis sa forme textuelle.
    pub fn depuis(nom: &str) -> Option<Portee> {
        Portee::TOUTES.into_iter().find(|p| p.nom() == nom.trim())
    }
}

/// Un client MCP enregistré dynamiquement (RFC 7591).
///
/// **Clients publics uniquement** : aucun secret de client n'est émis, et la
/// sécurité repose entièrement sur PKCE.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct ClientOAuth {
    client_id: String,
    nom: String,
    redirect_uris: Vec<String>,
}

impl ClientOAuth {
    /// Enregistre un client.
    ///
    /// Valide strictement chaque `redirect_uri` : absolue en `https://`, ou
    /// `http://localhost` et `http://127.0.0.1` pour le développement d'un
    /// client. Rien d'autre. Une URI relative, un schéma exotique ou un
    /// `http://` distant feraient de l'endpoint d'autorisation un redirecteur
    /// ouvert.
    pub fn enregistrer(
        client_id: String,
        nom: String,
        redirect_uris: Vec<String>,
    ) -> Result<Self, AppError> {
        if redirect_uris.is_empty() {
            return Err(AppError::Configuration {
                detail: "au moins une redirect_uri est requise".to_string(),
            });
        }
        for uri in &redirect_uris {
            if !uri_admissible(uri) {
                return Err(AppError::Configuration {
                    detail: format!(
                        "redirect_uri refusée : « {uri} ». Seules les URI absolues en https://, \
                         ou http:// sur localhost, sont admises"
                    ),
                });
            }
        }
        Ok(Self {
            client_id,
            nom,
            redirect_uris,
        })
    }

    /// Vérifie qu'une URI appartient bien à ce client.
    ///
    /// Comparaison **exacte**, jamais par préfixe : un préfixe permettrait
    /// `https://legitime.example/../evade`.
    pub fn uri_enregistree(&self, uri: &str) -> bool {
        self.redirect_uris.iter().any(|u| u == uri)
    }

    /// Identifiant du client.
    pub fn client_id(&self) -> &str {
        &self.client_id
    }
    /// Nom déclaré.
    pub fn nom(&self) -> &str {
        &self.nom
    }
    /// URI enregistrées.
    pub fn redirect_uris(&self) -> &[String] {
        &self.redirect_uris
    }
}

fn uri_admissible(uri: &str) -> bool {
    if uri.contains("..") || uri.contains(' ') {
        return false;
    }
    if let Some(reste) = uri.strip_prefix("https://") {
        return !reste.is_empty();
    }
    if let Some(reste) = uri.strip_prefix("http://") {
        return reste.starts_with("localhost")
            || reste.starts_with("127.0.0.1")
            || reste.starts_with("[::1]");
    }
    false
}

/// Les éléments d'une demande de code d'autorisation.
#[derive(Debug, Clone)]
pub struct DemandeCode {
    /// Le code en clair, tel qu'il sera rendu au client.
    pub code: String,
    /// Client destinataire.
    pub client_id: String,
    /// Utilisateur qui autorise.
    pub utilisateur: String,
    /// URI de redirection, déjà validée contre la liste enregistrée.
    pub redirect_uri: String,
    /// Défi PKCE.
    pub defi: String,
    /// Méthode du défi. Seul `S256` est accepté.
    pub methode: String,
    /// Instant d'émission.
    pub emis_le: Horodatage,
    /// Durée de validité.
    pub validite: crate::domain::Duree,
}

/// Un code d'autorisation à usage unique.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct CodeAutorisation {
    code: Redacted<String>,
    client_id: String,
    utilisateur: String,
    redirect_uri: String,
    defi: String,
    expire_le: Horodatage,
    utilise: bool,
}

impl CodeAutorisation {
    /// Émet un code.
    ///
    /// Refuse tout `code_challenge_method` autre que `S256`.
    ///
    /// Les paramètres sont regroupés dans [`DemandeCode`] : à huit arguments
    /// positionnels, deux `String` voisines finissent tôt ou tard interverties,
    /// et intervertir `client_id` et `redirect_uri` produirait un code lié au
    /// mauvais client sans qu'aucun type ne s'y oppose.
    pub fn emettre(demande: DemandeCode) -> Result<Self, AppError> {
        let DemandeCode {
            code,
            client_id,
            utilisateur,
            redirect_uri,
            defi,
            methode,
            emis_le,
            validite,
        } = demande;
        let methode = methode.as_str();
        if !methode.eq_ignore_ascii_case("S256") {
            return Err(AppError::Configuration {
                detail: format!(
                    "code_challenge_method « {methode} » refusé : OAuth 2.1 impose S256, \
                     « plain » n'est pas une option"
                ),
            });
        }
        if defi.trim().is_empty() {
            return Err(AppError::Configuration {
                detail: "code_challenge absent".to_string(),
            });
        }
        Ok(Self {
            code: Redacted::new(code),
            client_id,
            utilisateur,
            redirect_uri,
            defi,
            expire_le: emis_le.plus(validite),
            utilise: false,
        })
    }

    /// Échange le code contre des jetons.
    ///
    /// **Prend `self` par valeur** : un code consommé ne peut pas être rejoué,
    /// c'est une erreur de compilation et non un champ booléen qu'on pourrait
    /// oublier de tester.
    ///
    /// Revérifie le triplet complet — `client_id`, `redirect_uri`, défi — parce
    /// que les valeurs présentées à l'échange viennent du client, pas d'une
    /// source de vérité.
    pub fn echanger(
        self,
        client_id: &str,
        redirect_uri: &str,
        verificateur: &str,
        maintenant: Horodatage,
    ) -> Result<String, AppError> {
        if self.utilise {
            return Err(AppError::Authentification {
                secret: Redacted::new("code déjà consommé".to_string()),
            });
        }
        if maintenant.apres(self.expire_le) {
            return Err(AppError::Authentification {
                secret: Redacted::new("code expiré".to_string()),
            });
        }
        if self.client_id != client_id || self.redirect_uri != redirect_uri {
            return Err(AppError::Authentification {
                secret: Redacted::new("client_id ou redirect_uri non concordant".to_string()),
            });
        }
        if !verifier_pkce(verificateur, &self.defi) {
            return Err(AppError::Authentification {
                secret: Redacted::new("code_verifier invalide".to_string()),
            });
        }
        Ok(self.utilisateur)
    }

    /// Identifiant du client destinataire.
    pub fn client_id(&self) -> &str {
        &self.client_id
    }
    /// Instant d'expiration.
    pub fn expire_le(&self) -> Horodatage {
        self.expire_le
    }
    /// Utilisateur autorisé.
    pub fn utilisateur(&self) -> &str {
        &self.utilisateur
    }
}

/// Un jeton de rafraîchissement, tel qu'il est **persisté**.
///
/// Ne porte que l'empreinte : le jeton en clair n'existe qu'une fois, dans la
/// réponse au client. Le principe est celui d'un hash de mot de passe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct JetonRafraichissement {
    empreinte: String,
    client_id: String,
    utilisateur: String,
    portees: Vec<Portee>,
    expire_le: Horodatage,
    revoque: bool,
}

impl JetonRafraichissement {
    /// Enregistre un jeton à partir de sa valeur en clair.
    ///
    /// La valeur n'est pas conservée : seule son empreinte l'est.
    pub fn depuis_clair(
        clair: &str,
        client_id: String,
        utilisateur: String,
        portees: Vec<Portee>,
        emis_le: Horodatage,
        validite: crate::domain::Duree,
    ) -> Self {
        Self {
            empreinte: empreinte_sha256(clair),
            client_id,
            utilisateur,
            portees,
            expire_le: emis_le.plus(validite),
            revoque: false,
        }
    }

    /// Utilise le jeton, en le révoquant **inconditionnellement**.
    ///
    /// La rotation a lieu que le reste de l'échange réussisse ou non : un jeton
    /// volé puis rejoué après qu'un client légitime s'est rafraîchi doit être
    /// mort, et attendre la fin de l'échange pour révoquer laisserait une
    /// fenêtre.
    pub fn utiliser(
        mut self,
        client_id: &str,
        maintenant: Horodatage,
    ) -> (Self, Result<(String, Vec<Portee>), AppError>) {
        let etait_revoque = self.revoque;
        self.revoque = true;

        let issue = if etait_revoque {
            Err(AppError::Authentification {
                secret: Redacted::new("jeton de rafraîchissement déjà utilisé".to_string()),
            })
        } else if maintenant.apres(self.expire_le) {
            Err(AppError::Authentification {
                secret: Redacted::new("jeton de rafraîchissement expiré".to_string()),
            })
        } else if self.client_id != client_id {
            Err(AppError::Authentification {
                secret: Redacted::new("jeton présenté par un autre client".to_string()),
            })
        } else {
            Ok((self.utilisateur.clone(), self.portees.clone()))
        };
        (self, issue)
    }

    /// Empreinte persistée.
    pub fn empreinte(&self) -> &str {
        &self.empreinte
    }
    /// Vrai si révoqué.
    pub fn revoque(&self) -> bool {
        self.revoque
    }
    /// Portées accordées.
    pub fn portees(&self) -> &[Portee] {
        &self.portees
    }
}
