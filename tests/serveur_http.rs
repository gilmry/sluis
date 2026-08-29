//! Stories 8.1 et 8.2 — flow OAuth complet et transport Streamable HTTP.
//!
//! Rejoue la procédure de vérification en sept étapes du skill
//! `mcp-oauth-maison`, sans réseau : le routage est appelé directement, ce qui
//! rend le flow reproductible et rapide.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::Value;
use sluis::application::ports::{AuditLog, Horloge};
use sluis::domain::{
    base64url_sans_remplissage, jeton_acces, AppError, AuditEntry, Duree, Horodatage, Portee,
    Redacted, Revendications,
};
use sluis::infrastructure::mcp::{RegistreOutils, ServeurMcp};
use sluis::infrastructure::oauth_depot::DepotOAuthFichier;
use sluis::infrastructure::serveur_http::{
    analyser_parametres, echapper_html, Alea, ReglagesHttp, ServeurHttp, VerificateurIdentifiants,
};

struct HorlogeFixe;
impl Horloge for HorlogeFixe {
    fn maintenant(&self) -> Horodatage {
        Horodatage::new(1_000_000)
    }
}

#[derive(Default)]
struct JournalMuet;
impl AuditLog for JournalMuet {
    fn append(&self, _entree: &AuditEntry) -> Result<(), AppError> {
        Ok(())
    }
}

/// Aléa déterministe, pour que le flow soit rejouable.
struct AleaCompte(AtomicUsize);
impl Alea for AleaCompte {
    fn valeur(&self) -> String {
        format!("alea-{:04}", self.0.fetch_add(1, Ordering::SeqCst))
    }
}

struct Superviseur;
impl VerificateurIdentifiants for Superviseur {
    fn verifier(&self, identifiant: &str, mot_de_passe: &str) -> Option<String> {
        (identifiant == "gilmry" && mot_de_passe == "bon-mot-de-passe")
            .then(|| "gilmry".to_string())
    }
}

const SECRET: &str = "secret-de-signature-de-test";

fn serveur(nom: &str) -> (ServeurHttp, Arc<DepotOAuthFichier>) {
    let chemin =
        std::env::temp_dir().join(format!("sluis-oauth-{nom}-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&chemin);
    let depot = Arc::new(DepotOAuthFichier::ouvrir(chemin).expect("dépôt"));
    let mcp = Arc::new(ServeurMcp::new(
        RegistreOutils::new(),
        Arc::new(JournalMuet),
        Arc::new(HorlogeFixe),
        Vec::new(),
    ));
    let serveur = ServeurHttp::new(
        depot.clone(),
        mcp,
        Arc::new(HorlogeFixe),
        Arc::new(Superviseur),
        Arc::new(AleaCompte(AtomicUsize::new(0))),
        ReglagesHttp {
            base_url: "https://sluis.exemple.org".to_string(),
            secret_signature: Redacted::new(SECRET.to_string()),
            validite_acces: Duree::secondes(3_600).expect("durée"),
            validite_rafraichissement: Duree::jours(30).expect("durée"),
            validite_code: Duree::secondes(600).expect("durée"),
        },
    );
    (serveur, depot)
}

fn defi(verificateur: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(verificateur.as_bytes());
    base64url_sans_remplissage(&h.finalize())
}

/// Déroule l'enregistrement, l'autorisation et l'échange. Rend
/// `(client_id, access_token, refresh_token)`.
fn flow_complet(s: &ServeurHttp, verificateur: &str) -> (String, String, String) {
    let inscription = s.acheminer(
        "POST",
        "/oauth/register",
        r#"{"redirect_uris":["https://claude.ai/cb"],"client_name":"Claude"}"#,
        &[],
    );
    assert_eq!(inscription.code, 201, "{}", inscription.corps);
    let client_id = serde_json::from_str::<Value>(&inscription.corps).expect("json")["client_id"]
        .as_str()
        .expect("client_id")
        .to_string();

    let formulaire = format!(
        "client_id={client_id}&redirect_uri=https%3A%2F%2Fclaude.ai%2Fcb&response_type=code\
         &state=xyz&code_challenge={}&code_challenge_method=S256\
         &identifiant=gilmry&mot_de_passe=bon-mot-de-passe",
        defi(verificateur)
    );
    let redirection = s.acheminer("POST", "/oauth/authorize", &formulaire, &[]);
    assert_eq!(redirection.code, 302, "{}", redirection.corps);
    let location = redirection
        .entetes
        .iter()
        .find(|(c, _)| c == "Location")
        .map(|(_, v)| v.clone())
        .expect("Location");
    let code = analyser_parametres(location.split_once('?').expect("requête").1)
        .get("code")
        .cloned()
        .expect("code");

    let echange = s.acheminer(
        "POST",
        "/oauth/token",
        &format!(
            "grant_type=authorization_code&code={code}&client_id={client_id}\
             &redirect_uri=https%3A%2F%2Fclaude.ai%2Fcb&code_verifier={verificateur}"
        ),
        &[],
    );
    assert_eq!(echange.code, 200, "{}", echange.corps);
    let jetons: Value = serde_json::from_str(&echange.corps).expect("json");
    (
        client_id,
        jetons["access_token"].as_str().expect("access").to_string(),
        jetons["refresh_token"]
            .as_str()
            .expect("refresh")
            .to_string(),
    )
}

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_le_document_de_decouverte_permet_l_auto_configuration() {
    let (s, _d) = serveur("decouverte");
    let reponse = s.acheminer("GET", "/.well-known/oauth-authorization-server", "", &[]);
    let doc: Value = serde_json::from_str(&reponse.corps).expect("json");
    assert_eq!(doc["issuer"], "https://sluis.exemple.org");
    assert_eq!(doc["code_challenge_methods_supported"][0], "S256");
    assert_eq!(doc["token_endpoint_auth_methods_supported"][0], "none");
    assert!(doc["registration_endpoint"].is_string());
}

#[test]
fn happy_le_flow_complet_rend_un_couple_de_jetons() {
    let (s, _d) = serveur("flow");
    let (_, acces, rafraichissement) = flow_complet(&s, "un-verificateur-bien-long");
    assert!(!acces.is_empty());
    assert!(!rafraichissement.is_empty());
    let revendications = jeton_acces::verifier(
        &acces,
        &Redacted::new(SECRET.to_string()),
        Horodatage::new(1_000_100),
    )
    .expect("jeton valide");
    assert_eq!(revendications.sujet, "gilmry");
}

#[test]
fn happy_mcp_repond_avec_un_jeton_porteur_valide() {
    let (s, _d) = serveur("mcp-ok");
    let (_, acces, _) = flow_complet(&s, "verificateur");
    let reponse = s.acheminer(
        "POST",
        "/mcp",
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#,
        &[("Authorization".to_string(), format!("Bearer {acces}"))],
    );
    assert_eq!(reponse.code, 200, "{}", reponse.corps);
    assert!(reponse.corps.contains("2024-11-05"));
}

// ── @negative ────────────────────────────────────────────────

#[test]
fn negative_un_mauvais_verificateur_est_rejete() {
    let (s, _d) = serveur("mauvais-v");
    let inscription = s.acheminer(
        "POST",
        "/oauth/register",
        r#"{"redirect_uris":["https://claude.ai/cb"],"client_name":"C"}"#,
        &[],
    );
    let client_id = serde_json::from_str::<Value>(&inscription.corps).expect("json")["client_id"]
        .as_str()
        .expect("id")
        .to_string();
    let formulaire = format!(
        "client_id={client_id}&redirect_uri=https%3A%2F%2Fclaude.ai%2Fcb&response_type=code\
         &state=x&code_challenge={}&code_challenge_method=S256\
         &identifiant=gilmry&mot_de_passe=bon-mot-de-passe",
        defi("le-bon")
    );
    let redirection = s.acheminer("POST", "/oauth/authorize", &formulaire, &[]);
    let location = redirection.entetes[0].1.clone();
    let code = analyser_parametres(location.split_once('?').expect("q").1)["code"].clone();

    let echange = s.acheminer(
        "POST",
        "/oauth/token",
        &format!(
            "grant_type=authorization_code&code={code}&client_id={client_id}\
             &redirect_uri=https%3A%2F%2Fclaude.ai%2Fcb&code_verifier=le-mauvais"
        ),
        &[],
    );
    assert_eq!(echange.code, 400, "PKCE doit rejeter : {}", echange.corps);
}

#[test]
fn negative_des_identifiants_incorrects_ne_redirigent_pas() {
    let (s, _d) = serveur("mauvais-id");
    let inscription = s.acheminer(
        "POST",
        "/oauth/register",
        r#"{"redirect_uris":["https://claude.ai/cb"],"client_name":"C"}"#,
        &[],
    );
    let client_id = serde_json::from_str::<Value>(&inscription.corps).expect("json")["client_id"]
        .as_str()
        .expect("id")
        .to_string();
    let reponse = s.acheminer(
        "POST",
        "/oauth/authorize",
        &format!(
            "client_id={client_id}&redirect_uri=https%3A%2F%2Fclaude.ai%2Fcb&response_type=code\
             &state=x&code_challenge={}&code_challenge_method=S256\
             &identifiant=gilmry&mot_de_passe=faux",
            defi("v")
        ),
        &[],
    );
    assert_eq!(reponse.code, 401);
    assert!(
        reponse.entetes.iter().all(|(c, _)| c != "Location"),
        "un échec d'authentification ne doit pas rediriger"
    );
}

#[test]
fn negative_mcp_sans_jeton_repond_401() {
    let (s, _d) = serveur("mcp-sans-jeton");
    let reponse = s.acheminer(
        "POST",
        "/mcp",
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#,
        &[],
    );
    assert_eq!(reponse.code, 401);
}

#[test]
fn negative_un_jeton_signe_avec_un_autre_secret_est_rejete() {
    let (s, _d) = serveur("mcp-faux-jeton");
    let faux = jeton_acces::emettre(
        &Revendications {
            sujet: "intrus".to_string(),
            client_id: "c".to_string(),
            portees: vec![Portee::Read],
            expire_le: Horodatage::new(9_999_999),
        },
        &Redacted::new("un-autre-secret".to_string()),
    );
    let reponse = s.acheminer(
        "POST",
        "/mcp",
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#,
        &[("Authorization".to_string(), format!("Bearer {faux}"))],
    );
    assert_eq!(reponse.code, 401);
}

// ── @edge ────────────────────────────────────────────────────

#[test]
fn edge_un_endpoint_inconnu_rend_404() {
    let (s, _d) = serveur("404");
    assert_eq!(s.acheminer("GET", "/inexistant", "", &[]).code, 404);
}

#[test]
fn edge_une_redirect_uri_avec_chaine_de_requete_recoit_le_bon_separateur() {
    let (s, _d) = serveur("separateur");
    let inscription = s.acheminer(
        "POST",
        "/oauth/register",
        r#"{"redirect_uris":["https://claude.ai/cb?deja=1"],"client_name":"C"}"#,
        &[],
    );
    let client_id = serde_json::from_str::<Value>(&inscription.corps).expect("json")["client_id"]
        .as_str()
        .expect("id")
        .to_string();
    let redirection = s.acheminer(
        "POST",
        "/oauth/authorize",
        &format!(
            "client_id={client_id}&redirect_uri=https%3A%2F%2Fclaude.ai%2Fcb%3Fdeja%3D1\
             &response_type=code&state=x&code_challenge={}&code_challenge_method=S256\
             &identifiant=gilmry&mot_de_passe=bon-mot-de-passe",
            defi("v")
        ),
        &[],
    );
    let location = &redirection.entetes[0].1;
    assert!(
        location.contains("?deja=1&code="),
        "séparateur incorrect : {location}"
    );
}

#[test]
fn edge_la_sante_repond_sans_authentification() {
    let (s, _d) = serveur("sante");
    assert_eq!(s.acheminer("GET", "/sante", "", &[]).code, 200);
}

// ── @security ────────────────────────────────────────────────

#[test]
fn security_la_methode_plain_est_refusee_avant_tout_affichage() {
    let (s, _d) = serveur("plain");
    let reponse = s.acheminer(
        "GET",
        "/oauth/authorize?client_id=x&redirect_uri=https%3A%2F%2Fx%2Fcb\
         &response_type=code&code_challenge=abc&code_challenge_method=plain",
        "",
        &[],
    );
    assert_eq!(reponse.code, 400);
    assert!(reponse.corps.contains("S256"));
}

#[test]
fn security_une_redirect_uri_non_enregistree_ne_produit_aucune_redirection() {
    // Le point le plus important de l'endpoint : tant que redirect_uri n'est
    // pas vérifiée, on n'y renvoie rien, pas même une erreur — ce serait un
    // redirecteur ouvert.
    let (s, _d) = serveur("uri-non-enregistree");
    let inscription = s.acheminer(
        "POST",
        "/oauth/register",
        r#"{"redirect_uris":["https://claude.ai/cb"],"client_name":"C"}"#,
        &[],
    );
    let client_id = serde_json::from_str::<Value>(&inscription.corps).expect("json")["client_id"]
        .as_str()
        .expect("id")
        .to_string();
    let reponse = s.acheminer(
        "GET",
        &format!(
            "/oauth/authorize?client_id={client_id}&redirect_uri=https%3A%2F%2Fattaquant%2Fcb\
             &response_type=code&code_challenge=abc&code_challenge_method=S256"
        ),
        "",
        &[],
    );
    assert_eq!(reponse.code, 400);
    assert!(
        reponse.entetes.iter().all(|(c, _)| c != "Location"),
        "aucune redirection ne doit avoir lieu vers une URI non vérifiée"
    );
    assert!(reponse.corps.contains("non enregistrée"));
}

#[test]
fn security_un_code_rejoue_est_rejete() {
    let (s, _d) = serveur("rejeu-code");
    let verificateur = "verificateur";
    let inscription = s.acheminer(
        "POST",
        "/oauth/register",
        r#"{"redirect_uris":["https://claude.ai/cb"],"client_name":"C"}"#,
        &[],
    );
    let client_id = serde_json::from_str::<Value>(&inscription.corps).expect("json")["client_id"]
        .as_str()
        .expect("id")
        .to_string();
    let redirection = s.acheminer(
        "POST",
        "/oauth/authorize",
        &format!(
            "client_id={client_id}&redirect_uri=https%3A%2F%2Fclaude.ai%2Fcb&response_type=code\
             &state=x&code_challenge={}&code_challenge_method=S256\
             &identifiant=gilmry&mot_de_passe=bon-mot-de-passe",
            defi(verificateur)
        ),
        &[],
    );
    let location = redirection.entetes[0].1.clone();
    let code = analyser_parametres(location.split_once('?').expect("q").1)["code"].clone();
    let corps = format!(
        "grant_type=authorization_code&code={code}&client_id={client_id}\
         &redirect_uri=https%3A%2F%2Fclaude.ai%2Fcb&code_verifier={verificateur}"
    );

    assert_eq!(s.acheminer("POST", "/oauth/token", &corps, &[]).code, 200);
    let rejeu = s.acheminer("POST", "/oauth/token", &corps, &[]);
    assert_eq!(rejeu.code, 400, "un code doit être à usage unique");
}

#[test]
fn security_un_refresh_token_rejoue_est_rejete() {
    let (s, _d) = serveur("rejeu-refresh");
    let (client_id, _, rafraichissement) = flow_complet(&s, "verificateur");
    let corps =
        format!("grant_type=refresh_token&refresh_token={rafraichissement}&client_id={client_id}");

    assert_eq!(
        s.acheminer("POST", "/oauth/token", &corps, &[]).code,
        200,
        "le premier rafraîchissement doit réussir"
    );
    let rejeu = s.acheminer("POST", "/oauth/token", &corps, &[]);
    assert_eq!(
        rejeu.code, 400,
        "la rotation doit rendre le jeton précédent inutilisable"
    );
}

#[test]
fn security_le_state_est_echappe_dans_le_formulaire() {
    // Les paramètres OAuth sont réaffichés en champs cachés : un `state` forgé
    // injecterait sinon du script dans la page de connexion.
    assert_eq!(
        echapper_html("\"><script>alert(1)</script>"),
        "&quot;&gt;&lt;script&gt;alert(1)&lt;/script&gt;"
    );

    let (s, _d) = serveur("xss");
    let inscription = s.acheminer(
        "POST",
        "/oauth/register",
        r#"{"redirect_uris":["https://claude.ai/cb"],"client_name":"C"}"#,
        &[],
    );
    let client_id = serde_json::from_str::<Value>(&inscription.corps).expect("json")["client_id"]
        .as_str()
        .expect("id")
        .to_string();
    let reponse = s.acheminer(
        "GET",
        &format!(
            "/oauth/authorize?client_id={client_id}&redirect_uri=https%3A%2F%2Fclaude.ai%2Fcb\
             &response_type=code&code_challenge=abc&code_challenge_method=S256\
             &state=%22%3E%3Cscript%3Ealert(1)%3C%2Fscript%3E"
        ),
        "",
        &[],
    );
    assert_eq!(reponse.code, 200);
    assert!(
        !reponse.corps.contains("<script>"),
        "le state n'a pas été échappé : {}",
        reponse.corps
    );
}

#[test]
fn security_le_depot_ne_conserve_aucun_code_en_clair() {
    let (s, depot) = serveur("depot-clair");
    let inscription = s.acheminer(
        "POST",
        "/oauth/register",
        r#"{"redirect_uris":["https://claude.ai/cb"],"client_name":"C"}"#,
        &[],
    );
    let client_id = serde_json::from_str::<Value>(&inscription.corps).expect("json")["client_id"]
        .as_str()
        .expect("id")
        .to_string();
    let redirection = s.acheminer(
        "POST",
        "/oauth/authorize",
        &format!(
            "client_id={client_id}&redirect_uri=https%3A%2F%2Fclaude.ai%2Fcb&response_type=code\
             &state=x&code_challenge={}&code_challenge_method=S256\
             &identifiant=gilmry&mot_de_passe=bon-mot-de-passe",
            defi("v")
        ),
        &[],
    );
    let location = redirection.entetes[0].1.clone();
    let code = analyser_parametres(location.split_once('?').expect("q").1)["code"].clone();
    let _ = depot;

    // Le code émis ne doit apparaître ni comme clé, ni comme valeur.
    let chemin = std::env::temp_dir().join(format!(
        "sluis-oauth-depot-clair-{}.json",
        std::process::id()
    ));
    let contenu = std::fs::read_to_string(&chemin).expect("dépôt lisible");
    assert!(
        !contenu.contains(&code),
        "le code en clair est dans le dépôt : un fichier lu livrerait un code utilisable"
    );
}

#[test]
fn security_les_appels_concurrents_au_meme_code_ne_reussissent_qu_une_fois() {
    let (s, _d) = serveur("concurrence-code");
    let verificateur = "verificateur";
    let inscription = s.acheminer(
        "POST",
        "/oauth/register",
        r#"{"redirect_uris":["https://claude.ai/cb"],"client_name":"C"}"#,
        &[],
    );
    let client_id = serde_json::from_str::<Value>(&inscription.corps).expect("json")["client_id"]
        .as_str()
        .expect("id")
        .to_string();
    let redirection = s.acheminer(
        "POST",
        "/oauth/authorize",
        &format!(
            "client_id={client_id}&redirect_uri=https%3A%2F%2Fclaude.ai%2Fcb&response_type=code\
             &state=x&code_challenge={}&code_challenge_method=S256\
             &identifiant=gilmry&mot_de_passe=bon-mot-de-passe",
            defi(verificateur)
        ),
        &[],
    );
    let location = redirection.entetes[0].1.clone();
    let code = analyser_parametres(location.split_once('?').expect("q").1)["code"].clone();
    let corps = Arc::new(format!(
        "grant_type=authorization_code&code={code}&client_id={client_id}\
         &redirect_uri=https%3A%2F%2Fclaude.ai%2Fcb&code_verifier={verificateur}"
    ));

    let s = Arc::new(s);
    let succes = Arc::new(Mutex::new(0usize));
    let mut fils = Vec::new();
    for _ in 0..8 {
        let (s, corps, succes) = (s.clone(), corps.clone(), succes.clone());
        fils.push(std::thread::spawn(move || {
            if s.acheminer("POST", "/oauth/token", &corps, &[]).code == 200 {
                *succes.lock().expect("verrou") += 1;
            }
        }));
    }
    for fil in fils {
        fil.join().expect("fil");
    }
    assert_eq!(
        *succes.lock().expect("verrou"),
        1,
        "le retrait du code doit être atomique"
    );
}
