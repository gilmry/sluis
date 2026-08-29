//! Serveur HTTP — endpoints OAuth 2.1 et transport MCP Streamable HTTP.
//!
//! Six endpoints, conformes RFC 8414 et RFC 7591. L'ordre des vérifications
//! porte l'essentiel de la sécurité, et il est commenté à chaque endroit où il
//! compte.
//!
//! `tiny_http` plutôt qu'un serveur asynchrone : Sluis sert un superviseur et
//! ses agents, pas une foule. Un runtime asynchrone apporterait ici un arbre de
//! dépendances sans contrepartie.

use std::sync::Arc;

use serde_json::json;

use crate::application::ports::{DepotOAuth, Horloge};
use crate::domain::{
    empreinte_sha256, jeton_acces, AppError, ClientOAuth, CodeAutorisation, DemandeCode, Duree,
    Portee, Redacted, Revendications,
};
use crate::infrastructure::mcp::ServeurMcp;

/// Vérifie un couple identifiant / mot de passe.
///
/// Délégué à l'application hôte : le serveur d'autorisation n'invente pas de
/// second magasin d'utilisateurs, il s'appuie sur celui qui existe déjà.
pub trait VerificateurIdentifiants: Send + Sync {
    /// Rend l'identifiant de l'utilisateur si le couple est valide.
    fn verifier(&self, identifiant: &str, mot_de_passe: &str) -> Option<String>;
}

/// Fabrique des valeurs aléatoires.
pub trait Alea: Send + Sync {
    /// Une valeur imprévisible, en hexadécimal.
    fn valeur(&self) -> String;
}

/// Aléa fondé sur l'entropie du système.
#[derive(Debug, Default)]
pub struct AleaSysteme;

impl Alea for AleaSysteme {
    fn valeur(&self) -> String {
        // `getrandom` via /dev/urandom : pas de générateur maison pour ce qui
        // doit être imprévisible.
        let mut octets = [0u8; 32];
        if std::fs::File::open("/dev/urandom")
            .and_then(|mut f| std::io::Read::read_exact(&mut f, &mut octets))
            .is_err()
        {
            // Repli explicite plutôt que silencieux : sans entropie, on refuse
            // d'émettre une valeur prévisible.
            return String::new();
        }
        octets.iter().map(|o| format!("{o:02x}")).collect()
    }
}

/// Réglages du serveur.
pub struct ReglagesHttp {
    /// URL publique, telle qu'annoncée dans le document de découverte.
    pub base_url: String,
    /// Secret de signature des jetons d'accès.
    pub secret_signature: Redacted<String>,
    /// Durée de vie d'un jeton d'accès.
    pub validite_acces: Duree,
    /// Durée de vie d'un jeton de rafraîchissement.
    pub validite_rafraichissement: Duree,
    /// Durée de vie d'un code d'autorisation.
    pub validite_code: Duree,
}

/// Serveur HTTP.
pub struct ServeurHttp {
    depot: Arc<dyn DepotOAuth>,
    mcp: Arc<ServeurMcp>,
    horloge: Arc<dyn Horloge>,
    identifiants: Arc<dyn VerificateurIdentifiants>,
    alea: Arc<dyn Alea>,
    reglages: ReglagesHttp,
}

/// Une réponse HTTP à rendre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReponseHttp {
    /// Code de statut.
    pub code: u16,
    /// Type de contenu.
    pub type_contenu: String,
    /// Corps.
    pub corps: String,
    /// En-têtes supplémentaires, notamment `Location`.
    pub entetes: Vec<(String, String)>,
}

impl ReponseHttp {
    fn json(code: u16, valeur: serde_json::Value) -> Self {
        Self {
            code,
            type_contenu: "application/json".to_string(),
            corps: valeur.to_string(),
            entetes: Vec::new(),
        }
    }
    fn html(code: u16, corps: String) -> Self {
        Self {
            code,
            type_contenu: "text/html; charset=utf-8".to_string(),
            corps,
            entetes: Vec::new(),
        }
    }
    fn redirection(vers: String) -> Self {
        Self {
            code: 302,
            type_contenu: "text/plain".to_string(),
            corps: String::new(),
            entetes: vec![("Location".to_string(), vers)],
        }
    }
    fn erreur(code: u16, message: &str) -> Self {
        Self::json(code, json!({ "error": message }))
    }
}

impl ServeurHttp {
    /// Construit le serveur.
    pub fn new(
        depot: Arc<dyn DepotOAuth>,
        mcp: Arc<ServeurMcp>,
        horloge: Arc<dyn Horloge>,
        identifiants: Arc<dyn VerificateurIdentifiants>,
        alea: Arc<dyn Alea>,
        reglages: ReglagesHttp,
    ) -> Self {
        Self {
            depot,
            mcp,
            horloge,
            identifiants,
            alea,
            reglages,
        }
    }

    /// Achemine une requête.
    pub fn acheminer(
        &self,
        methode: &str,
        chemin: &str,
        corps: &str,
        entetes: &[(String, String)],
    ) -> ReponseHttp {
        let (chemin_seul, requete) = chemin.split_once('?').unwrap_or((chemin, ""));
        match (methode, chemin_seul) {
            ("GET", "/.well-known/oauth-authorization-server") => self.decouverte(),
            ("POST", "/oauth/register") => self.enregistrer(corps),
            ("GET", "/oauth/authorize") => self.autoriser_get(requete),
            ("POST", "/oauth/authorize") => self.autoriser_post(corps),
            ("POST", "/oauth/token") => self.jeton(corps),
            ("POST", "/mcp") => self.mcp(corps, entetes),
            ("GET", "/sante") => ReponseHttp::json(200, json!({"statut": "ok"})),
            _ => ReponseHttp::erreur(404, "endpoint inconnu"),
        }
    }

    /// RFC 8414 — ce document permet à un client MCP de se configurer seul.
    fn decouverte(&self) -> ReponseHttp {
        let issuer = &self.reglages.base_url;
        ReponseHttp::json(
            200,
            json!({
                "issuer": issuer,
                "authorization_endpoint": format!("{issuer}/oauth/authorize"),
                "token_endpoint": format!("{issuer}/oauth/token"),
                "registration_endpoint": format!("{issuer}/oauth/register"),
                "scopes_supported": Portee::TOUTES.iter().map(|p| p.nom()).collect::<Vec<_>>(),
                "response_types_supported": ["code"],
                "grant_types_supported": ["authorization_code", "refresh_token"],
                // S256 seul : annoncer « plain » reviendrait à l'accepter.
                "code_challenge_methods_supported": ["S256"],
                "token_endpoint_auth_methods_supported": ["none"],
            }),
        )
    }

    /// RFC 7591 — enregistrement dynamique, clients publics uniquement.
    fn enregistrer(&self, corps: &str) -> ReponseHttp {
        let Ok(demande) = serde_json::from_str::<serde_json::Value>(corps) else {
            return ReponseHttp::erreur(400, "corps JSON illisible");
        };
        let uris: Vec<String> = demande
            .get("redirect_uris")
            .and_then(|u| u.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let nom = demande
            .get("client_name")
            .and_then(|n| n.as_str())
            .unwrap_or("client MCP")
            .to_string();

        let client_id = self.alea.valeur();
        if client_id.is_empty() {
            return ReponseHttp::erreur(503, "entropie indisponible");
        }
        match ClientOAuth::enregistrer(client_id.clone(), nom.clone(), uris.clone()) {
            Ok(client) => match self.depot.enregistrer_client(client) {
                Ok(()) => ReponseHttp::json(
                    201,
                    json!({
                        "client_id": client_id,
                        "client_name": nom,
                        "redirect_uris": uris,
                        // Client public : aucun secret n'est émis, PKCE porte
                        // toute la sécurité.
                        "token_endpoint_auth_method": "none",
                    }),
                ),
                Err(e) => ReponseHttp::erreur(500, &e.to_string()),
            },
            Err(e) => ReponseHttp::erreur(400, &e.to_string()),
        }
    }

    /// Valide **avant** d'afficher quoi que ce soit.
    ///
    /// Tant que `redirect_uri` n'est pas vérifiée contre la liste enregistrée
    /// du client, aucune redirection n'a lieu, pas même pour signaler une
    /// erreur : ce serait un redirecteur ouvert.
    fn autoriser_get(&self, requete: &str) -> ReponseHttp {
        let params = analyser_parametres(requete);
        match self.valider_demande(&params) {
            Ok(client) => ReponseHttp::html(200, formulaire(&client, &params)),
            Err(message) => ReponseHttp::html(
                400,
                format!(
                    "<!doctype html><meta charset=utf-8><title>Sluis</title>\
                     <h1>Demande d'autorisation refusée</h1><p>{}</p>",
                    echapper_html(&message)
                ),
            ),
        }
    }

    /// Revalide, vérifie les identifiants, émet un code.
    ///
    /// La revalidation n'est pas redondante : les champs cachés du formulaire
    /// sont fournis par le client, pas par une source de vérité, même s'ils
    /// sont invisibles à l'utilisateur.
    fn autoriser_post(&self, corps: &str) -> ReponseHttp {
        let params = analyser_parametres(corps);
        let client = match self.valider_demande(&params) {
            Ok(client) => client,
            Err(message) => {
                return ReponseHttp::html(
                    400,
                    format!(
                        "<!doctype html><meta charset=utf-8><p>{}</p>",
                        echapper_html(&message)
                    ),
                )
            }
        };

        let identifiant = params.get("identifiant").cloned().unwrap_or_default();
        let mot_de_passe = params.get("mot_de_passe").cloned().unwrap_or_default();
        let Some(utilisateur) = self.identifiants.verifier(&identifiant, &mot_de_passe) else {
            return ReponseHttp::html(
                401,
                formulaire_avec_erreur(&client, &params, "identifiants incorrects"),
            );
        };

        let code_clair = self.alea.valeur();
        if code_clair.is_empty() {
            return ReponseHttp::erreur(503, "entropie indisponible");
        }
        let redirect_uri = params.get("redirect_uri").cloned().unwrap_or_default();
        let code = match CodeAutorisation::emettre(DemandeCode {
            code: code_clair.clone(),
            client_id: client.client_id().to_string(),
            utilisateur,
            redirect_uri: redirect_uri.clone(),
            defi: params.get("code_challenge").cloned().unwrap_or_default(),
            methode: params
                .get("code_challenge_method")
                .cloned()
                .unwrap_or_default(),
            emis_le: self.horloge.maintenant(),
            validite: self.reglages.validite_code,
        }) {
            Ok(code) => code,
            Err(e) => return ReponseHttp::erreur(400, &e.to_string()),
        };
        if let Err(e) = self.depot.deposer_code(&code_clair, code) {
            return ReponseHttp::erreur(500, &e.to_string());
        }

        let separateur = if redirect_uri.contains('?') { '&' } else { '?' };
        ReponseHttp::redirection(format!(
            "{redirect_uri}{separateur}code={}&state={}",
            encoder_uri(&code_clair),
            encoder_uri(params.get("state").map(String::as_str).unwrap_or(""))
        ))
    }

    fn valider_demande(
        &self,
        params: &std::collections::HashMap<String, String>,
    ) -> Result<ClientOAuth, String> {
        let client_id = params.get("client_id").ok_or("client_id manquant")?;
        let redirect_uri = params.get("redirect_uri").ok_or("redirect_uri manquante")?;
        if params.get("response_type").map(String::as_str) != Some("code") {
            return Err("response_type doit valoir « code »".to_string());
        }
        if params.get("code_challenge_method").map(String::as_str) != Some("S256") {
            return Err(
                "code_challenge_method doit valoir « S256 » : OAuth 2.1 interdit « plain »"
                    .to_string(),
            );
        }
        let client = self
            .depot
            .client(client_id)
            .map_err(|e| e.to_string())?
            .ok_or("client_id inconnu")?;
        if !client.uri_enregistree(redirect_uri) {
            return Err("redirect_uri non enregistrée pour ce client".to_string());
        }
        Ok(client)
    }

    /// Échange un code, ou rafraîchit.
    fn jeton(&self, corps: &str) -> ReponseHttp {
        let params = analyser_parametres(corps);
        match params.get("grant_type").map(String::as_str) {
            Some("authorization_code") => self.echanger_code(&params),
            Some("refresh_token") => self.rafraichir(&params),
            _ => ReponseHttp::erreur(400, "grant_type non supporté"),
        }
    }

    fn echanger_code(&self, params: &std::collections::HashMap<String, String>) -> ReponseHttp {
        let (Some(code_clair), Some(client_id), Some(redirect_uri), Some(verificateur)) = (
            params.get("code"),
            params.get("client_id"),
            params.get("redirect_uri"),
            params.get("code_verifier"),
        ) else {
            return ReponseHttp::erreur(400, "paramètres manquants");
        };

        // Retrait atomique : un second échange du même code ne trouvera rien.
        let code = match self.depot.consommer_code(code_clair) {
            Ok(Some(code)) => code,
            Ok(None) => return ReponseHttp::erreur(400, "code inconnu ou déjà consommé"),
            Err(e) => return ReponseHttp::erreur(500, &e.to_string()),
        };

        let maintenant = self.horloge.maintenant();
        let utilisateur = match code.echanger(client_id, redirect_uri, verificateur, maintenant) {
            Ok(utilisateur) => utilisateur,
            Err(e) => return ReponseHttp::erreur(400, &e.to_string()),
        };
        self.emettre_jetons(utilisateur, client_id.clone(), Portee::TOUTES.to_vec())
    }

    fn rafraichir(&self, params: &std::collections::HashMap<String, String>) -> ReponseHttp {
        let (Some(clair), Some(client_id)) = (params.get("refresh_token"), params.get("client_id"))
        else {
            return ReponseHttp::erreur(400, "paramètres manquants");
        };

        // La rotation a lieu dans le dépôt, avant que l'issue ne soit connue.
        let jeton = match self.depot.tourner_jeton(&empreinte_sha256(clair)) {
            Ok(Some(jeton)) => jeton,
            Ok(None) => return ReponseHttp::erreur(400, "jeton inconnu"),
            Err(e) => return ReponseHttp::erreur(400, &e.to_string()),
        };
        let (revoque, issue) = jeton.utiliser(client_id, self.horloge.maintenant());
        if let Err(e) = self.depot.deposer_jeton(revoque) {
            return ReponseHttp::erreur(500, &e.to_string());
        }
        match issue {
            Ok((utilisateur, portees)) => {
                self.emettre_jetons(utilisateur, client_id.clone(), portees)
            }
            Err(e) => ReponseHttp::erreur(400, &e.to_string()),
        }
    }

    fn emettre_jetons(
        &self,
        utilisateur: String,
        client_id: String,
        portees: Vec<Portee>,
    ) -> ReponseHttp {
        let maintenant = self.horloge.maintenant();
        let acces = jeton_acces::emettre(
            &Revendications {
                sujet: utilisateur.clone(),
                client_id: client_id.clone(),
                portees: portees.clone(),
                expire_le: maintenant.plus(self.reglages.validite_acces),
            },
            &self.reglages.secret_signature,
        );

        let rafraichissement = self.alea.valeur();
        if rafraichissement.is_empty() {
            return ReponseHttp::erreur(503, "entropie indisponible");
        }
        let persiste = crate::domain::JetonRafraichissement::depuis_clair(
            &rafraichissement,
            client_id,
            utilisateur,
            portees.clone(),
            maintenant,
            self.reglages.validite_rafraichissement,
        );
        if let Err(e) = self.depot.deposer_jeton(persiste) {
            return ReponseHttp::erreur(500, &e.to_string());
        }

        ReponseHttp::json(
            200,
            json!({
                "access_token": acces,
                "token_type": "Bearer",
                "expires_in": self.reglages.validite_acces.en_secondes(),
                "refresh_token": rafraichissement,
                "scope": portees.iter().map(|p| p.nom()).collect::<Vec<_>>().join(" "),
            }),
        )
    }

    /// Transport MCP Streamable HTTP, **sans état**.
    ///
    /// Chaque requête se ré-authentifie par le même en-tête `Authorization`
    /// que le reste de l'API : il n'y a pas de session serveur à protéger
    /// séparément.
    fn mcp(&self, corps: &str, entetes: &[(String, String)]) -> ReponseHttp {
        let porteur = entetes
            .iter()
            .find(|(cle, _)| cle.eq_ignore_ascii_case("authorization"))
            .and_then(|(_, valeur)| valeur.strip_prefix("Bearer "))
            .unwrap_or("");
        if porteur.is_empty() {
            return ReponseHttp::erreur(401, "jeton d'accès requis");
        }
        match jeton_acces::verifier(
            porteur,
            &self.reglages.secret_signature,
            self.horloge.maintenant(),
        ) {
            Ok(revendications) => {
                // La portée `read` est le minimum. Les outils revérifient
                // ensuite leur propre tier, indépendamment de cette portée.
                if !revendications.portees.contains(&Portee::Read) {
                    return ReponseHttp::erreur(403, "portée sluis:read requise");
                }
                match self.mcp.traiter(corps) {
                    Some(reponse) => ReponseHttp {
                        code: 200,
                        type_contenu: "application/json".to_string(),
                        corps: reponse,
                        entetes: Vec::new(),
                    },
                    None => ReponseHttp {
                        code: 202,
                        type_contenu: "application/json".to_string(),
                        corps: String::new(),
                        entetes: Vec::new(),
                    },
                }
            }
            Err(e) => ReponseHttp::erreur(401, &e.to_string()),
        }
    }

    /// Boucle d'écoute.
    pub fn ecouter(&self, adresse: &str) -> Result<(), AppError> {
        let serveur = tiny_http::Server::http(adresse).map_err(|e| AppError::Configuration {
            detail: format!("écoute impossible sur {adresse} : {e}"),
        })?;
        for mut requete in serveur.incoming_requests() {
            let mut corps = String::new();
            let _ = std::io::Read::read_to_string(requete.as_reader(), &mut corps);
            let entetes: Vec<(String, String)> = requete
                .headers()
                .iter()
                .map(|h| (h.field.as_str().to_string(), h.value.as_str().to_string()))
                .collect();
            let reponse =
                self.acheminer(requete.method().as_str(), requete.url(), &corps, &entetes);

            let mut sortie =
                tiny_http::Response::from_string(reponse.corps).with_status_code(reponse.code);
            if let Ok(entete) =
                tiny_http::Header::from_bytes(&b"Content-Type"[..], reponse.type_contenu.as_bytes())
            {
                sortie = sortie.with_header(entete);
            }
            for (cle, valeur) in reponse.entetes {
                if let Ok(entete) = tiny_http::Header::from_bytes(cle.as_bytes(), valeur.as_bytes())
                {
                    sortie = sortie.with_header(entete);
                }
            }
            let _ = requete.respond(sortie);
        }
        Ok(())
    }
}

/// Analyse des paramètres `application/x-www-form-urlencoded`.
pub fn analyser_parametres(entree: &str) -> std::collections::HashMap<String, String> {
    entree
        .split('&')
        .filter(|p| !p.is_empty())
        .filter_map(|paire| {
            let (cle, valeur) = paire.split_once('=')?;
            Some((decoder_uri(cle), decoder_uri(valeur)))
        })
        .collect()
}

fn decoder_uri(entree: &str) -> String {
    let octets = entree.as_bytes();
    let mut sortie = Vec::with_capacity(octets.len());
    let mut i = 0;
    while i < octets.len() {
        match octets[i] {
            b'%' if i + 2 < octets.len() => {
                let hexa = std::str::from_utf8(&octets[i + 1..i + 3]).unwrap_or("");
                match u8::from_str_radix(hexa, 16) {
                    Ok(octet) => {
                        sortie.push(octet);
                        i += 3;
                    }
                    Err(_) => {
                        sortie.push(octets[i]);
                        i += 1;
                    }
                }
            }
            b'+' => {
                sortie.push(b' ');
                i += 1;
            }
            autre => {
                sortie.push(autre);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&sortie).to_string()
}

fn encoder_uri(entree: &str) -> String {
    entree
        .bytes()
        .map(|o| match o {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (o as char).to_string()
            }
            autre => format!("%{autre:02X}"),
        })
        .collect()
}

/// Échappe les caractères actifs en HTML.
///
/// Nécessaire parce que les paramètres OAuth sont réaffichés en champs cachés :
/// sans cela, un `state` forgé injecterait du script dans la page de connexion.
pub fn echapper_html(entree: &str) -> String {
    entree
        .chars()
        .map(|c| match c {
            '&' => "&amp;".to_string(),
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&#39;".to_string(),
            autre => autre.to_string(),
        })
        .collect()
}

fn formulaire(client: &ClientOAuth, params: &std::collections::HashMap<String, String>) -> String {
    formulaire_interne(client, params, None)
}

fn formulaire_avec_erreur(
    client: &ClientOAuth,
    params: &std::collections::HashMap<String, String>,
    erreur: &str,
) -> String {
    formulaire_interne(client, params, Some(erreur))
}

fn formulaire_interne(
    client: &ClientOAuth,
    params: &std::collections::HashMap<String, String>,
    erreur: Option<&str>,
) -> String {
    let caches: String = [
        "client_id",
        "redirect_uri",
        "response_type",
        "state",
        "code_challenge",
        "code_challenge_method",
    ]
    .iter()
    .map(|nom| {
        format!(
            "<input type=hidden name=\"{nom}\" value=\"{}\">",
            echapper_html(params.get(*nom).map(String::as_str).unwrap_or(""))
        )
    })
    .collect();

    format!(
        "<!doctype html><meta charset=utf-8><title>Sluis — connexion</title>\
         <style>body{{font-family:system-ui;max-width:26rem;margin:4rem auto;padding:0 1rem}}\
         input{{display:block;width:100%;margin:.5rem 0;padding:.5rem}}\
         .err{{color:#b00}}</style>\
         <h1>Sluis</h1>\
         <p><strong>{}</strong> demande l'accès à votre orchestrateur.</p>{}\
         <form method=post action=\"/oauth/authorize\">{caches}\
         <input name=identifiant placeholder=\"identifiant\" autocomplete=username required>\
         <input name=mot_de_passe type=password placeholder=\"mot de passe\" \
         autocomplete=current-password required>\
         <button type=submit>Autoriser</button></form>",
        echapper_html(client.nom()),
        erreur
            .map(|e| format!("<p class=err>{}</p>", echapper_html(e)))
            .unwrap_or_default()
    )
}
