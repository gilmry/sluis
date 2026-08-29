//! Story 0.3 — Harnais de contract testing.
//!
//! Sous-story **non optionnelle** du Sprint 0 : `contrat-api.md` documente
//! qu'un contrat décrit mais non matérialisé a déjà coûté un NO-GO en
//! production. Reléguer ce harnais serait refaire l'erreur exactement.
//!
//! Le harnais prouve, pour chaque outil du registre, que le schéma annoncé par
//! `tools/list` et la désérialisation effective de `tools/call` acceptent et
//! refusent les **mêmes** charges utiles.

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use sluis::domain::{AppError, Tier};
use sluis::infrastructure::mcp::{contrat::Contrat, ContratOutil, Outil, RegistreOutils};

/// Enveloppe un contrat pour en faire un outil enregistrable. L'exécution est
/// sans effet : le harnais ne teste que le contrat, pas le métier.
struct OutilDeContrat<T>(Contrat<T>);

impl<T> ContratOutil for OutilDeContrat<T>
where
    T: schemars::JsonSchema + serde::de::DeserializeOwned + Send + Sync,
{
    fn nom(&self) -> &'static str {
        self.0.nom()
    }
    fn description(&self) -> &'static str {
        self.0.description()
    }
    fn schema(&self) -> serde_json::Value {
        self.0.schema()
    }
    fn desérialiser(&self, arguments: &serde_json::Value) -> Result<(), String> {
        self.0.desérialiser(arguments)
    }
}

impl<T> Outil for OutilDeContrat<T>
where
    T: schemars::JsonSchema + serde::de::DeserializeOwned + Send + Sync,
{
    fn tier(&self) -> Tier {
        Tier::Two
    }
    fn appeler(&self, _arguments: &serde_json::Value) -> Result<serde_json::Value, AppError> {
        Ok(json!({}))
    }
}

// ─────────────────────────────────────────────────────────────
// Types de démonstration du harnais
// ─────────────────────────────────────────────────────────────

/// Un type d'arguments conforme : annoté, et fermé aux champs inconnus.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // les champs existent pour le schéma, pas pour être lus ici
struct ArgumentsConformes {
    /// Chemin du dépôt à inspecter.
    chemin: String,
    /// Profondeur maximale de parcours.
    #[serde(default)]
    profondeur: Option<u8>,
}

/// Un type volontairement défectueux : il accepte les champs inconnus.
///
/// Il n'est jamais enregistré. Il sert à prouver que le harnais **détecte**
/// la faute, sans quoi une suite verte ne dirait rien.
#[derive(Debug, Deserialize, JsonSchema)]
struct ArgumentsOuverts {
    #[allow(dead_code)]
    chemin: String,
}

fn registre_de_demonstration() -> RegistreOutils {
    let mut registre = RegistreOutils::new();
    registre
        .enregistrer(Box::new(OutilDeContrat(
            Contrat::<ArgumentsConformes>::new(
                "outil_de_demonstration",
                "Outil de démonstration du harnais de contrat.",
            ),
        )))
        .expect("enregistrement");
    registre
}

/// Le cœur du harnais : confronte schéma annoncé et désérialisation effective.
///
/// Rendu public au fichier pour que chaque outil réel, quand il arrivera, passe
/// par exactement cette fonction plutôt que par une variante réécrite.
fn verifier_contrat(outil: &dyn ContratOutil) -> Result<(), String> {
    let schema = outil.schema();

    // 1. Le schéma décrit bien un objet d'arguments.
    let type_declare = schema.get("type").and_then(|t| t.as_str());
    if type_declare != Some("object") {
        return Err(format!(
            "outil « {} » : le schéma ne décrit pas un objet ({type_declare:?})",
            outil.nom()
        ));
    }

    // 2. Les champs requis par le schéma sont réellement requis par le type.
    //    On retire chaque champ requis et on exige un échec de désérialisation.
    let requis: Vec<String> = schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    let complet = charge_utile_minimale(&schema);
    if outil.desérialiser(&complet).is_err() {
        return Err(format!(
            "outil « {} » : la charge utile dérivée du schéma est refusée par le type",
            outil.nom()
        ));
    }
    for champ in &requis {
        let mut ampute = complet.clone();
        ampute
            .as_object_mut()
            .expect("objet")
            .remove(champ.as_str());
        if outil.desérialiser(&ampute).is_ok() {
            return Err(format!(
                "outil « {} » : le schéma déclare « {champ} » requis, mais le type l'accepte absent",
                outil.nom()
            ));
        }
    }

    // 3. deny_unknown_fields est effectif : un champ inconnu doit être rejeté.
    let mut avec_intrus = complet.clone();
    avec_intrus
        .as_object_mut()
        .expect("objet")
        .insert("champ_totalement_inconnu".to_string(), json!(1));
    if outil.desérialiser(&avec_intrus).is_ok() {
        return Err(format!(
            "outil « {} » : un champ inconnu est accepté, deny_unknown_fields manque",
            outil.nom()
        ));
    }

    Ok(())
}

/// Fabrique une charge utile minimale à partir du schéma, pour chaque champ
/// requis. Volontairement limitée aux types scalaires : un outil MCP dont les
/// arguments seraient plus profonds mériterait d'être découpé.
fn charge_utile_minimale(schema: &serde_json::Value) -> serde_json::Value {
    let mut objet = serde_json::Map::new();
    let proprietes = schema.get("properties").and_then(|p| p.as_object());
    let requis: Vec<&str> = schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    if let Some(proprietes) = proprietes {
        for (nom, description) in proprietes {
            if !requis.contains(&nom.as_str()) {
                continue;
            }
            let type_champ = description.get("type").and_then(|t| t.as_str());
            let valeur = match type_champ {
                Some("string") => json!("valeur"),
                Some("integer") => json!(1),
                Some("number") => json!(1.0),
                Some("boolean") => json!(true),
                Some("array") => json!([]),
                Some("object") => json!({}),
                _ => json!("valeur"),
            };
            objet.insert(nom.clone(), valeur);
        }
    }
    serde_json::Value::Object(objet)
}

// ─────────────────────────────────────────────────────────────
// @happy
// ─────────────────────────────────────────────────────────────

#[test]
fn happy_tous_les_outils_du_registre_respectent_leur_contrat() {
    let registre = registre_de_demonstration();
    let mut fautes = Vec::new();
    for outil in registre.outils() {
        if let Err(faute) = verifier_contrat(outil.as_ref()) {
            fautes.push(faute);
        }
    }
    assert!(
        fautes.is_empty(),
        "contrats violés :\n{}",
        fautes.join("\n")
    );
}

#[test]
fn happy_le_schema_est_genere_depuis_le_type_et_nomme_ses_champs() {
    let registre = registre_de_demonstration();
    let outil = &registre.outils()[0];
    let schema = outil.schema();
    let proprietes = schema.get("properties").expect("schéma sans propriétés");
    assert!(proprietes.get("chemin").is_some());
    assert!(proprietes.get("profondeur").is_some());
}

// ─────────────────────────────────────────────────────────────
// @negative — le harnais détecte réellement les trois fautes
// ─────────────────────────────────────────────────────────────

#[test]
fn negative_un_champ_inconnu_est_rejete_par_un_type_conforme() {
    let outil = Contrat::<ArgumentsConformes>::new("t", "d");
    let resultat = outil.desérialiser(&json!({"chemin": "/x", "intrus": 1}));
    assert!(resultat.is_err(), "deny_unknown_fields inopérant");
}

#[test]
fn negative_un_champ_requis_manquant_est_rejete() {
    let outil = Contrat::<ArgumentsConformes>::new("t", "d");
    assert!(outil.desérialiser(&json!({})).is_err());
}

#[test]
fn negative_un_type_divergent_est_rejete() {
    let outil = Contrat::<ArgumentsConformes>::new("t", "d");
    assert!(outil.desérialiser(&json!({"chemin": 42})).is_err());
}

#[test]
fn negative_le_harnais_signale_un_type_sans_deny_unknown_fields() {
    // Sans ce test, un harnais qui ne détecterait rien passerait au vert pour
    // de mauvaises raisons : l'« assurance fausse » que gates.md juge pire
    // qu'une absence de gate.
    let defectueux = Contrat::<ArgumentsOuverts>::new("defectueux", "accepte tout");
    let faute = verifier_contrat(&defectueux).expect_err("le harnais aurait dû refuser");
    assert!(
        faute.contains("deny_unknown_fields"),
        "le harnais doit nommer la faute, obtenu : {faute}"
    );
}

// ─────────────────────────────────────────────────────────────
// @edge
// ─────────────────────────────────────────────────────────────

#[test]
fn edge_un_registre_vide_ne_fait_pas_croire_a_une_verification() {
    let registre = RegistreOutils::new();
    assert!(registre.is_empty());
    assert_eq!(registre.len(), 0);
}

#[test]
fn edge_un_outil_sans_nom_est_refuse_au_demarrage() {
    let mut registre = RegistreOutils::new();
    let resultat = registre.enregistrer(Box::new(OutilDeContrat(
        Contrat::<ArgumentsConformes>::new("", "d"),
    )));
    assert!(resultat.is_err(), "un outil sans nom doit être refusé");
}

#[test]
fn edge_un_doublon_de_nom_est_refuse_au_demarrage() {
    let mut registre = registre_de_demonstration();
    let resultat = registre.enregistrer(Box::new(OutilDeContrat(
        Contrat::<ArgumentsConformes>::new("outil_de_demonstration", "doublon"),
    )));
    assert!(resultat.is_err(), "deux outils de même nom sont ambigus");
}

#[test]
fn edge_un_champ_optionnel_absent_reste_accepte() {
    let outil = Contrat::<ArgumentsConformes>::new("t", "d");
    assert!(outil.desérialiser(&json!({"chemin": "/x"})).is_ok());
}

// ─────────────────────────────────────────────────────────────
// @security
// ─────────────────────────────────────────────────────────────

#[test]
fn security_deny_unknown_fields_est_prouve_par_enumeration_pas_par_echantillon() {
    // La différence est le tout du test. Une liste d'outils écrite à la main
    // vieillit dès le premier outil ajouté, et la suite reste verte en ayant
    // cessé de vérifier le nouveau. Ici on parcourt le registre lui-même.
    let registre = registre_de_demonstration();
    assert!(
        !registre.is_empty(),
        "un registre vide rendrait cette énumération vide, donc muette"
    );
    for outil in registre.outils() {
        let mut charge = charge_utile_minimale(&outil.schema());
        charge
            .as_object_mut()
            .expect("objet")
            .insert("injection".to_string(), json!("x"));
        assert!(
            outil.desérialiser(&charge).is_err(),
            "outil « {} » accepte un champ inconnu",
            outil.nom()
        );
    }
}

#[test]
fn security_tout_outil_enregistre_possede_un_schema_exploitable() {
    let registre = registre_de_demonstration();
    for outil in registre.outils() {
        let schema = outil.schema();
        assert!(
            schema.is_object(),
            "outil « {} » sans schéma : le contrat ne serait plus matérialisé",
            outil.nom()
        );
        assert!(
            !outil.description().trim().is_empty(),
            "outil « {} » sans description : illisible pour un client MCP",
            outil.nom()
        );
    }
}
