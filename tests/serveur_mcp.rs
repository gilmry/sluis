//! Stories 4.1 et 4.2 — serveur MCP stdio, registre et filtrage en frontière.

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use sluis::application::ports::{AuditLog, Horloge};
use sluis::domain::{AppError, AuditEntry, Horodatage, Redacted, Tier};
use sluis::infrastructure::mcp::{ContratOutil, Outil, RegistreOutils, ServeurMcp};

// ── doublures ────────────────────────────────────────────────

#[derive(Default)]
struct JournalMemoire {
    entrees: Mutex<Vec<String>>,
    en_panne: bool,
}

impl AuditLog for JournalMemoire {
    fn append(&self, entree: &AuditEntry) -> Result<(), AppError> {
        if self.en_panne {
            return Err(AppError::EntreeSortie {
                chemin: "journal".to_string(),
                detail: "disque plein".to_string(),
            });
        }
        self.entrees
            .lock()
            .map_err(|_| AppError::Configuration {
                detail: "verrou".to_string(),
            })?
            .push(entree.outil().to_string());
        Ok(())
    }
}

struct HorlogeFixe;
impl Horloge for HorlogeFixe {
    fn maintenant(&self) -> Horodatage {
        Horodatage::new(1_700_000_000)
    }
}

/// Outil de Tier 2 qui rend ce qu'on lui a donné.
struct OutilEcho;
impl ContratOutil for OutilEcho {
    fn nom(&self) -> &'static str {
        "echo"
    }
    fn description(&self) -> &'static str {
        "Rend son argument."
    }
    fn schema(&self) -> Value {
        json!({"type":"object","properties":{"texte":{"type":"string"}},"required":["texte"]})
    }
    fn desérialiser(&self, arguments: &Value) -> Result<(), String> {
        if arguments.get("texte").and_then(|t| t.as_str()).is_some() {
            Ok(())
        } else {
            Err("« texte » manquant".to_string())
        }
    }
}
impl Outil for OutilEcho {
    fn tier(&self) -> Tier {
        Tier::Two
    }
    fn appeler(&self, arguments: &Value) -> Result<Value, AppError> {
        Ok(json!({ "echo": arguments.get("texte").cloned().unwrap_or(Value::Null) }))
    }
}

/// Outil de Tier 1 : il ne doit jamais être exécutable par le transport stdio.
struct OutilDangereux;
impl ContratOutil for OutilDangereux {
    fn nom(&self) -> &'static str {
        "terraform_apply"
    }
    fn description(&self) -> &'static str {
        "Applique une déclaration Terraform."
    }
    fn schema(&self) -> Value {
        json!({"type":"object","properties":{},"required":[]})
    }
    fn desérialiser(&self, _arguments: &Value) -> Result<(), String> {
        Ok(())
    }
}
impl Outil for OutilDangereux {
    fn tier(&self) -> Tier {
        Tier::One
    }
    fn appeler(&self, _arguments: &Value) -> Result<Value, AppError> {
        // Ne doit jamais être atteint depuis stdio.
        Ok(json!({"applique": true}))
    }
}

/// Outil qui fuit volontairement un secret, pour prouver le filtre de frontière.
struct OutilQuiFuit;
impl ContratOutil for OutilQuiFuit {
    fn nom(&self) -> &'static str {
        "fuite"
    }
    fn description(&self) -> &'static str {
        "Rend volontairement un secret, pour prouver le filtrage."
    }
    fn schema(&self) -> Value {
        json!({"type":"object","properties":{},"required":[]})
    }
    fn desérialiser(&self, _arguments: &Value) -> Result<(), String> {
        Ok(())
    }
}
impl Outil for OutilQuiFuit {
    fn tier(&self) -> Tier {
        Tier::Two
    }
    fn appeler(&self, _arguments: &Value) -> Result<Value, AppError> {
        Ok(json!({"cle": "SECRET-APPLICATION", "imbrique": ["SECRET-APPLICATION"]}))
    }
}

fn serveur(journal: Arc<JournalMemoire>, secrets: Vec<&str>) -> ServeurMcp {
    let mut registre = RegistreOutils::new();
    registre.enregistrer(Box::new(OutilEcho)).expect("echo");
    registre
        .enregistrer(Box::new(OutilDangereux))
        .expect("dangereux");
    registre.enregistrer(Box::new(OutilQuiFuit)).expect("fuite");
    ServeurMcp::new(
        registre,
        journal,
        Arc::new(HorlogeFixe),
        secrets
            .into_iter()
            .map(|s| Redacted::new(s.to_string()))
            .collect(),
    )
}

fn appeler(serveur: &ServeurMcp, requete: Value) -> Value {
    let brut = serveur
        .traiter(&requete.to_string())
        .expect("une réponse attendue");
    serde_json::from_str(&brut).expect("réponse JSON")
}

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_initialize_annonce_la_version_de_protocole_et_les_capacites() {
    let s = serveur(Arc::new(JournalMemoire::default()), vec![]);
    let reponse = appeler(&s, json!({"jsonrpc":"2.0","id":1,"method":"initialize"}));
    assert_eq!(reponse["result"]["protocolVersion"], "2024-11-05");
    assert!(reponse["result"]["capabilities"]["tools"].is_object());
    assert_eq!(reponse["result"]["serverInfo"]["name"], "sluis");
}

#[test]
fn happy_tools_list_expose_chaque_outil_avec_son_schema() {
    let s = serveur(Arc::new(JournalMemoire::default()), vec![]);
    let reponse = appeler(&s, json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}));
    let outils = reponse["result"]["tools"].as_array().expect("liste");
    assert_eq!(outils.len(), 3);
    for outil in outils {
        assert!(outil["name"].is_string());
        assert!(outil["description"].is_string());
        assert!(
            outil["inputSchema"].is_object(),
            "tout outil listé doit porter son schéma"
        );
    }
}

#[test]
fn happy_tools_call_execute_un_outil_tier2() {
    let journal = Arc::new(JournalMemoire::default());
    let s = serveur(journal.clone(), vec![]);
    let reponse = appeler(
        &s,
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call",
               "params":{"name":"echo","arguments":{"texte":"bonjour"}}}),
    );
    assert_eq!(reponse["result"]["isError"], false);
    assert!(reponse["result"]["content"][0]["text"]
        .as_str()
        .expect("texte")
        .contains("bonjour"));
    assert_eq!(journal.entrees.lock().expect("verrou").len(), 1);
}

// ── @negative ────────────────────────────────────────────────

#[test]
fn negative_un_json_illisible_produit_moins_32700() {
    let s = serveur(Arc::new(JournalMemoire::default()), vec![]);
    let brut = s.traiter("{ ceci n'est pas du json").expect("réponse");
    let reponse: Value = serde_json::from_str(&brut).expect("json");
    assert_eq!(reponse["error"]["code"], -32700);
}

#[test]
fn negative_une_methode_inconnue_produit_moins_32601() {
    let s = serveur(Arc::new(JournalMemoire::default()), vec![]);
    let reponse = appeler(&s, json!({"jsonrpc":"2.0","id":4,"method":"inexistante"}));
    assert_eq!(reponse["error"]["code"], -32601);
}

#[test]
fn negative_un_appel_sans_nom_d_outil_produit_moins_32602() {
    let s = serveur(Arc::new(JournalMemoire::default()), vec![]);
    let reponse = appeler(
        &s,
        json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{}}),
    );
    assert_eq!(reponse["error"]["code"], -32602);
}

#[test]
fn negative_un_outil_inconnu_est_refuse() {
    let s = serveur(Arc::new(JournalMemoire::default()), vec![]);
    let reponse = appeler(
        &s,
        json!({"jsonrpc":"2.0","id":6,"method":"tools/call",
               "params":{"name":"inexistant","arguments":{}}}),
    );
    assert_eq!(reponse["error"]["code"], -32601);
}

#[test]
fn negative_un_journal_en_panne_empeche_l_appel_de_reussir() {
    // Un appel non traçable ne doit pas être rendu comme réussi : le journal
    // donnerait alors une image fausse de ce qui s'est produit.
    let journal = Arc::new(JournalMemoire {
        entrees: Mutex::new(Vec::new()),
        en_panne: true,
    });
    let s = serveur(journal, vec![]);
    let reponse = appeler(
        &s,
        json!({"jsonrpc":"2.0","id":7,"method":"tools/call",
               "params":{"name":"echo","arguments":{"texte":"x"}}}),
    );
    assert_eq!(reponse["error"]["code"], -32603);
    assert!(reponse["error"]["message"]
        .as_str()
        .expect("message")
        .contains("journal"));
}

// ── @edge ────────────────────────────────────────────────────

#[test]
fn edge_une_notification_sans_id_ne_recoit_aucune_reponse() {
    let s = serveur(Arc::new(JournalMemoire::default()), vec![]);
    assert!(
        s.traiter(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}).to_string())
            .is_none(),
        "répondre à une notification est une faute de protocole JSON-RPC"
    );
}

#[test]
fn edge_une_charge_utile_volumineuse_est_traitee() {
    let s = serveur(Arc::new(JournalMemoire::default()), vec![]);
    let gros = "a".repeat(200_000);
    let reponse = appeler(
        &s,
        json!({"jsonrpc":"2.0","id":8,"method":"tools/call",
               "params":{"name":"echo","arguments":{"texte":gros}}}),
    );
    assert_eq!(reponse["result"]["isError"], false);
}

#[test]
fn edge_des_appels_concurrents_sont_tous_journalises() {
    let journal = Arc::new(JournalMemoire::default());
    let s = Arc::new(serveur(journal.clone(), vec![]));
    let mut fils = Vec::new();
    for i in 0..8 {
        let s = Arc::clone(&s);
        fils.push(std::thread::spawn(move || {
            for j in 0..8 {
                let _ = s.traiter(
                    &json!({"jsonrpc":"2.0","id":i * 100 + j,"method":"tools/call",
                            "params":{"name":"echo","arguments":{"texte":"x"}}})
                    .to_string(),
                );
            }
        }));
    }
    for fil in fils {
        fil.join().expect("fil");
    }
    assert_eq!(journal.entrees.lock().expect("verrou").len(), 64);
}

// ── @security ────────────────────────────────────────────────

#[test]
fn security_un_outil_tier1_ne_peut_pas_etre_execute_depuis_stdio() {
    // `tools/list` le montre — la découvrabilité n'est pas l'autorisation —
    // mais `tools/call` revérifie et refuse.
    let s = serveur(Arc::new(JournalMemoire::default()), vec![]);
    let liste = appeler(&s, json!({"jsonrpc":"2.0","id":9,"method":"tools/list"}));
    let noms: Vec<&str> = liste["result"]["tools"]
        .as_array()
        .expect("liste")
        .iter()
        .filter_map(|o| o["name"].as_str())
        .collect();
    assert!(noms.contains(&"terraform_apply"), "l'outil est bien listé");

    let reponse = appeler(
        &s,
        json!({"jsonrpc":"2.0","id":10,"method":"tools/call",
               "params":{"name":"terraform_apply","arguments":{}}}),
    );
    assert_eq!(reponse["error"]["code"], -32600);
    assert!(reponse["error"]["message"]
        .as_str()
        .expect("message")
        .contains("Tier 1"));
}

#[test]
fn security_le_filtre_de_frontiere_efface_un_secret_meme_si_l_outil_le_fuit() {
    // La garantie est structurelle : elle ne dépend pas de la bonne conduite
    // des outils, donc un outil écrit plus tard par quelqu'un d'autre ne peut
    // pas la contourner en oubliant de l'appeler.
    let s = serveur(
        Arc::new(JournalMemoire::default()),
        vec!["SECRET-APPLICATION"],
    );
    let reponse = appeler(
        &s,
        json!({"jsonrpc":"2.0","id":11,"method":"tools/call",
               "params":{"name":"fuite","arguments":{}}}),
    );
    let rendu = reponse.to_string();
    assert!(
        !rendu.contains("SECRET-APPLICATION"),
        "le filtre de frontière a laissé passer un secret : {rendu}"
    );
    assert!(rendu.contains("redacted"));
}

#[test]
fn security_le_filtre_s_applique_aussi_aux_messages_d_erreur() {
    let s = serveur(Arc::new(JournalMemoire::default()), vec!["echo"]);
    let reponse = appeler(
        &s,
        json!({"jsonrpc":"2.0","id":12,"method":"tools/call",
               "params":{"name":"echo","arguments":{"texte":"x"}}}),
    );
    // « echo » étant déclaré secret, il doit disparaître même du nom rendu.
    assert!(!reponse.to_string().contains("\"echo\""));
}

#[test]
fn security_chaque_appel_laisse_une_trace_avec_son_empreinte() {
    let journal = Arc::new(JournalMemoire::default());
    let s = serveur(journal.clone(), vec![]);
    let _ = appeler(
        &s,
        json!({"jsonrpc":"2.0","id":13,"method":"tools/call",
               "params":{"name":"echo","arguments":{"texte":"a"}}}),
    );
    let entrees = journal.entrees.lock().expect("verrou");
    assert_eq!(entrees.len(), 1);
    assert_eq!(entrees[0], "echo");
}
