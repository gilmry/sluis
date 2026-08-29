//! Serveur MCP — JSON-RPC 2.0 sur stdio.
//!
//! **Pas de SDK.** Le protocole tient ici en quelques centaines de lignes, et
//! deux projets de l'écosystème l'ont déjà écrit à la main. Une dépendance
//! externe évolutive coûterait plus qu'elle ne rapporte. Contrepartie assumée :
//! suivre les évolutions du protocole à la main (ADR-005).
//!
//! **`tools/list` gouverne la découvrabilité, jamais l'autorisation.**
//! `tools/call` revérifie systématiquement le tier de l'outil, indépendamment
//! de ce que la liste a montré. C'est l'anti-pattern que le skill
//! `mcp-oauth-maison` désigne nommément.
//!
//! **Le filtrage des secrets est appliqué à la frontière du transport**, pas
//! dans les cas d'usage. C'est ce qui garantit qu'un nouvel outil, écrit plus
//! tard par quelqu'un d'autre, ne puisse pas contourner la protection en
//! oubliant de l'appeler.

use std::io::{BufRead, Write};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::application::ports::{AuditLog, Horloge};
use crate::domain::{AppError, AuditEntry, Empreinte, Redacted, Tier};
use crate::infrastructure::mcp::RegistreOutils;

/// Version du protocole MCP annoncée.
pub const VERSION_PROTOCOLE: &str = "2024-11-05";

/// Codes d'erreur JSON-RPC 2.0.
pub mod codes {
    /// JSON illisible.
    pub const ERREUR_ANALYSE: i32 = -32700;
    /// Requête non conforme.
    pub const REQUETE_INVALIDE: i32 = -32600;
    /// Méthode inconnue.
    pub const METHODE_INCONNUE: i32 = -32601;
    /// Paramètres invalides.
    pub const PARAMETRES_INVALIDES: i32 = -32602;
    /// Erreur interne.
    pub const ERREUR_INTERNE: i32 = -32603;
}

#[derive(Debug, Deserialize)]
struct RequeteJsonRpc {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct ReponseJsonRpc {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ErreurJsonRpc>,
}

#[derive(Debug, Serialize)]
struct ErreurJsonRpc {
    code: i32,
    message: String,
}

/// Serveur MCP.
pub struct ServeurMcp {
    registre: RegistreOutils,
    journal: Arc<dyn AuditLog>,
    horloge: Arc<dyn Horloge>,
    /// Valeurs à effacer de toute sortie, quelle qu'en soit l'origine.
    secrets_connus: Vec<Redacted<String>>,
}

impl ServeurMcp {
    /// Construit le serveur.
    pub fn new(
        registre: RegistreOutils,
        journal: Arc<dyn AuditLog>,
        horloge: Arc<dyn Horloge>,
        secrets_connus: Vec<Redacted<String>>,
    ) -> Self {
        Self {
            registre,
            journal,
            horloge,
            secrets_connus,
        }
    }

    /// Traite une requête et rend la réponse à écrire.
    ///
    /// Rend `None` pour une notification (requête sans `id`), conformément à
    /// JSON-RPC : y répondre serait une faute de protocole.
    pub fn traiter(&self, brut: &str) -> Option<String> {
        let requete: RequeteJsonRpc = match serde_json::from_str(brut) {
            Ok(r) => r,
            Err(_) => {
                return Some(self.rendre(ReponseJsonRpc {
                    jsonrpc: "2.0",
                    id: Value::Null,
                    result: None,
                    error: Some(ErreurJsonRpc {
                        code: codes::ERREUR_ANALYSE,
                        message: "JSON illisible".to_string(),
                    }),
                }))
            }
        };

        let id = requete.id.clone();
        let resultat = match requete.method.as_str() {
            "initialize" => Ok(self.initialize()),
            "notifications/initialized" => Ok(Value::Null),
            "tools/list" => Ok(self.tools_list()),
            "tools/call" => self.tools_call(requete.params.unwrap_or(Value::Null)),
            autre => Err((
                codes::METHODE_INCONNUE,
                format!("méthode inconnue : {autre}"),
            )),
        };

        // Une notification n'attend pas de réponse.
        let id = id?;

        Some(match resultat {
            Ok(valeur) => self.rendre(ReponseJsonRpc {
                jsonrpc: "2.0",
                id,
                result: Some(self.filtrer(valeur)),
                error: None,
            }),
            Err((code, message)) => self.rendre(ReponseJsonRpc {
                jsonrpc: "2.0",
                id,
                result: None,
                error: Some(ErreurJsonRpc {
                    code,
                    message: self.filtrer_texte(&message),
                }),
            }),
        })
    }

    fn rendre(&self, reponse: ReponseJsonRpc) -> String {
        serde_json::to_string(&reponse).unwrap_or_else(|_| {
            // Ne peut pas échouer sur une structure aussi simple, mais la règle
            // « pas d'unwrap hors tests » vaut aussi pour l'impossible.
            r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":"erreur interne"}}"#
                .to_string()
        })
    }

    fn initialize(&self) -> Value {
        json!({
            "protocolVersion": VERSION_PROTOCOLE,
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": "sluis",
                "version": env!("CARGO_PKG_VERSION"),
            }
        })
    }

    fn tools_list(&self) -> Value {
        let outils: Vec<Value> = self
            .registre
            .outils()
            .iter()
            .map(|o| {
                json!({
                    "name": o.nom(),
                    "description": o.description(),
                    "inputSchema": o.schema(),
                })
            })
            .collect();
        json!({ "tools": outils })
    }

    fn tools_call(&self, params: Value) -> Result<Value, (i32, String)> {
        let nom = params
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or((codes::PARAMETRES_INVALIDES, "« name » manquant".to_string()))?;
        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));

        let outil = self
            .registre
            .trouver(nom)
            .ok_or((codes::METHODE_INCONNUE, format!("outil inconnu : {nom}")))?;

        // Revérification du tier, indépendante de ce que tools/list a montré.
        // Le transport stdio n'expose que le Tier 2 : le Tier 1 exige la
        // passerelle d'approbation, qui n'est pas un chemin d'appel direct.
        if outil.tier() == Tier::One {
            return Err((
                codes::REQUETE_INVALIDE,
                format!(
                    "l'outil « {nom} » est de Tier 1 : il exige une approbation humaine \
                     et ne peut pas être exécuté directement"
                ),
            ));
        }

        let empreinte = Empreinte::calculer(&format!("{nom}|{arguments}"));
        let resultat = outil.appeler(&arguments);

        let entree = AuditEntry::new(
            self.horloge.maintenant().to_string(),
            nom.to_string(),
            outil.tier(),
            empreinte.abregee().to_string(),
            resultat
                .as_ref()
                .map(|_| ())
                .map_err(|e| self.filtrer_texte(&e.to_string())),
        );
        // Un appel non traçable ne doit pas être rendu comme réussi.
        if let Err(erreur) = self.journal.append(&entree) {
            return Err((
                codes::ERREUR_INTERNE,
                format!("journal d'audit indisponible : {erreur}"),
            ));
        }

        match resultat {
            Ok(valeur) => Ok(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&valeur).unwrap_or_default(),
                }],
                "isError": false,
            })),
            Err(AppError::ProjetNonAutorise { projet }) => Err((
                codes::REQUETE_INVALIDE,
                format!("projet non autorisé : {projet}"),
            )),
            Err(erreur) => Ok(json!({
                "content": [{ "type": "text", "text": self.filtrer_texte(&erreur.to_string()) }],
                "isError": true,
            })),
        }
    }

    /// Efface de toute chaîne les valeurs secrètes connues.
    fn filtrer_texte(&self, texte: &str) -> String {
        let mut sortie = texte.to_string();
        for secret in &self.secrets_connus {
            let valeur = secret.exposer();
            if !valeur.is_empty() {
                sortie = sortie.replace(valeur.as_str(), crate::domain::redacted::MARQUEUR);
            }
        }
        sortie
    }

    /// Applique le filtrage récursivement à une valeur JSON.
    fn filtrer(&self, valeur: Value) -> Value {
        if self.secrets_connus.is_empty() {
            return valeur;
        }
        match valeur {
            Value::String(s) => Value::String(self.filtrer_texte(&s)),
            Value::Array(a) => Value::Array(a.into_iter().map(|v| self.filtrer(v)).collect()),
            Value::Object(o) => Value::Object(
                o.into_iter()
                    .map(|(cle, v)| (self.filtrer_texte(&cle), self.filtrer(v)))
                    .collect(),
            ),
            autre => autre,
        }
    }

    /// Boucle de lecture sur stdio : une requête par ligne.
    pub fn boucle(&self, entree: impl BufRead, mut sortie: impl Write) -> std::io::Result<()> {
        for ligne in entree.lines() {
            let ligne = ligne?;
            if ligne.trim().is_empty() {
                continue;
            }
            if let Some(reponse) = self.traiter(&ligne) {
                writeln!(sortie, "{reponse}")?;
                sortie.flush()?;
            }
        }
        Ok(())
    }
}
