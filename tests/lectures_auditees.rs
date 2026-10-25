//! Story 34.5a — le journal d'audit consigne l'empreinte des fichiers lus
//! (ADR-034, point 5 ; ADR-022).

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sluis::application::ports::{AuditLog, DepotCharge, Horloge};
use sluis::domain::{AppError, AuditEntry, Horodatage, Tier};
use sluis::infrastructure::fs_charge::FsCharge;
use sluis::infrastructure::mcp::{ContratOutil, Outil, RegistreOutils, ServeurMcp};

const CHARGE: &str = "topologie: vps\nmodule: monosite/vps/bac-a-sable/terraform\n\
sortie_adresse: vps_ip\nchemin: /api/sante\ncible:\n  requetes_par_seconde: 200\n  \
p99_millisecondes: 300\nbornes:\n  ttl_secondes: 3600\n  plafond_depense: 20\n";

/// Journal en mémoire : garde chaque entrée telle qu'elle serait écrite.
#[derive(Default)]
struct JournalMemoire(Mutex<Vec<Value>>);

impl AuditLog for JournalMemoire {
    fn append(&self, entree: &AuditEntry) -> Result<(), AppError> {
        let ligne = serde_json::to_value(entree).expect("sérialisation");
        self.0.lock().expect("verrou").push(ligne);
        Ok(())
    }
}

struct HorlogeFixe;
impl Horloge for HorlogeFixe {
    fn maintenant(&self) -> Horodatage {
        Horodatage::new(1_700_000_000)
    }
}

/// Outil qui lit la déclaration de charge du dépôt `racine` s'il en a un,
/// puis réussit ou échoue selon `echoue`.
struct OutilLecteur {
    nom: &'static str,
    racine: Option<String>,
    echoue: bool,
}

impl ContratOutil for OutilLecteur {
    fn nom(&self) -> &'static str {
        self.nom
    }
    fn description(&self) -> &'static str {
        "Lit une déclaration de charge."
    }
    fn schema(&self) -> Value {
        json!({"type":"object"})
    }
    fn desérialiser(&self, _arguments: &Value) -> Result<(), String> {
        Ok(())
    }
}

impl Outil for OutilLecteur {
    fn tier(&self) -> Tier {
        Tier::Two
    }
    fn appeler(&self, _arguments: &Value) -> Result<Value, AppError> {
        if let Some(racine) = &self.racine {
            FsCharge::new().lire(racine)?;
        }
        if self.echoue {
            return Err(AppError::Configuration {
                detail: "refus après lecture".to_string(),
            });
        }
        Ok(json!({}))
    }
}

struct Depot(std::path::PathBuf);
impl Depot {
    fn nouveau(nom: &str) -> Self {
        let racine = std::env::temp_dir().join(format!("sluis-lectures-{nom}"));
        let _ = std::fs::remove_dir_all(&racine);
        std::fs::create_dir_all(racine.join("_shared")).expect("création");
        std::fs::write(racine.join("_shared/charge.yaml"), CHARGE).expect("écriture");
        Self(racine)
    }
    fn racine(&self) -> String {
        self.0.to_str().expect("chemin").to_string()
    }
}
impl Drop for Depot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn serveur(outils: Vec<OutilLecteur>) -> (ServeurMcp, Arc<JournalMemoire>) {
    let mut registre = RegistreOutils::new();
    for outil in outils {
        registre
            .enregistrer(Box::new(outil))
            .expect("enregistrement");
    }
    let journal = Arc::new(JournalMemoire::default());
    let serveur = ServeurMcp::new(registre, journal.clone(), Arc::new(HorlogeFixe), Vec::new());
    (serveur, journal)
}

fn appeler(serveur: &ServeurMcp, nom: &str) {
    let requete = json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
                         "params":{"name":nom,"arguments":{}}});
    serveur.traiter(&requete.to_string()).expect("réponse");
}

fn sha256(texte: &str) -> String {
    format!("{:x}", Sha256::digest(texte.as_bytes()))
}

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_un_appel_qui_lit_charge_yaml_l_inscrit_au_journal() {
    let depot = Depot::nouveau("happy");
    let (serveur, journal) = serveur(vec![OutilLecteur {
        nom: "lire",
        racine: Some(depot.racine()),
        echoue: false,
    }]);

    appeler(&serveur, "lire");

    let entrees = journal.0.lock().expect("verrou");
    let lectures = entrees[0]["lectures"].as_array().expect("lectures");
    assert_eq!(lectures.len(), 1, "{lectures:?}");
    assert!(lectures[0]["chemin"]
        .as_str()
        .expect("chemin")
        .ends_with("_shared/charge.yaml"));
    assert_eq!(lectures[0]["sha256"], sha256(CHARGE));
}

// ── @negative ────────────────────────────────────────────────

#[test]
fn negative_un_appel_qui_ne_lit_rien_ne_porte_pas_de_champ_lectures() {
    let (serveur, journal) = serveur(vec![OutilLecteur {
        nom: "rien",
        racine: None,
        echoue: false,
    }]);

    appeler(&serveur, "rien");

    let entrees = journal.0.lock().expect("verrou");
    assert!(
        entrees[0].get("lectures").is_none(),
        "la forme d'avant doit être conservée : {}",
        entrees[0]
    );
}

// ── @edge ────────────────────────────────────────────────────

#[test]
fn edge_les_lectures_d_un_appel_ne_fuient_pas_dans_le_suivant() {
    let depot = Depot::nouveau("fuite");
    // Une lecture faite hors de tout appel, sur ce fil, ne doit pas être
    // attribuée à l'appel suivant.
    FsCharge::new()
        .lire(&depot.racine())
        .expect("lecture hors appel");
    let (serveur, journal) = serveur(vec![
        OutilLecteur {
            nom: "lire",
            racine: Some(depot.racine()),
            echoue: false,
        },
        OutilLecteur {
            nom: "rien",
            racine: None,
            echoue: false,
        },
    ]);

    appeler(&serveur, "rien");
    appeler(&serveur, "lire");
    appeler(&serveur, "rien");

    let entrees = journal.0.lock().expect("verrou");
    assert!(entrees[0].get("lectures").is_none(), "{}", entrees[0]);
    assert_eq!(entrees[1]["lectures"].as_array().map(Vec::len), Some(1));
    assert!(entrees[2].get("lectures").is_none(), "{}", entrees[2]);
}

// ── @security ────────────────────────────────────────────────

#[test]
fn security_un_appel_en_echec_consigne_ce_qu_il_a_lu() {
    let depot = Depot::nouveau("echec");
    let (serveur, journal) = serveur(vec![OutilLecteur {
        nom: "refus",
        racine: Some(depot.racine()),
        echoue: true,
    }]);

    appeler(&serveur, "refus");

    let entrees = journal.0.lock().expect("verrou");
    assert_eq!(entrees[0]["issue"], "echec", "{}", entrees[0]);
    assert_eq!(entrees[0]["lectures"][0]["sha256"], sha256(CHARGE));
}
