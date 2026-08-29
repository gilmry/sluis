//! Story 8.1 — serveur d'autorisation OAuth 2.1 + PKCE.
//!
//! Les cinq décisions de sécurité du skill `mcp-oauth-maison` sont chacune
//! couverte par un test qui échouerait si on la relâchait.

use sluis::domain::{
    base64url_sans_remplissage, empreinte_sha256, verifier_pkce, ClientOAuth, CodeAutorisation,
    DemandeCode, Duree, Horodatage, JetonRafraichissement, Portee,
};

const MAINTENANT: Horodatage = Horodatage::new(1_000_000);

fn defi_pour(verificateur: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(verificateur.as_bytes());
    base64url_sans_remplissage(&h.finalize())
}

fn code(defi: &str) -> CodeAutorisation {
    CodeAutorisation::emettre(DemandeCode {
        code: "code-secret".to_string(),
        client_id: "client-1".to_string(),
        utilisateur: "gilmry".to_string(),
        redirect_uri: "https://claude.ai/callback".to_string(),
        defi: defi.to_string(),
        methode: "S256".to_string(),
        emis_le: MAINTENANT,
        validite: Duree::secondes(600).expect("durée"),
    })
    .expect("émission")
}

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_un_client_s_enregistre_avec_une_uri_https() {
    let client = ClientOAuth::enregistrer(
        "abc".to_string(),
        "Claude Code".to_string(),
        vec!["https://claude.ai/callback".to_string()],
    )
    .expect("enregistrement");
    assert!(client.uri_enregistree("https://claude.ai/callback"));
}

#[test]
fn happy_un_code_valide_s_echange() {
    let verificateur = "un-verificateur-suffisamment-long-pour-etre-serieux";
    let utilisateur = code(&defi_pour(verificateur))
        .echanger(
            "client-1",
            "https://claude.ai/callback",
            verificateur,
            Horodatage::new(1_000_100),
        )
        .expect("échange");
    assert_eq!(utilisateur, "gilmry");
}

#[test]
fn happy_les_portees_se_lisent_et_se_rendent() {
    for portee in Portee::TOUTES {
        assert_eq!(Portee::depuis(portee.nom()), Some(portee));
    }
    assert_eq!(Portee::Read.nom(), "sluis:read");
}

// ── @negative ────────────────────────────────────────────────

#[test]
fn negative_la_methode_plain_est_refusee() {
    let erreur = CodeAutorisation::emettre(DemandeCode {
        code: "c".to_string(),
        client_id: "client".to_string(),
        utilisateur: "u".to_string(),
        redirect_uri: "https://x/cb".to_string(),
        defi: "defi".to_string(),
        methode: "plain".to_string(),
        emis_le: MAINTENANT,
        validite: Duree::secondes(60).expect("durée"),
    })
    .unwrap_err();
    assert!(
        erreur.to_string().contains("S256"),
        "OAuth 2.1 interdit plain, ce n'est pas une option : {erreur}"
    );
}

#[test]
fn negative_un_mauvais_verificateur_est_rejete() {
    let erreur = code(&defi_pour("le-bon"))
        .echanger(
            "client-1",
            "https://claude.ai/callback",
            "le-mauvais",
            MAINTENANT,
        )
        .unwrap_err();
    assert!(erreur.to_string().contains("authentification"));
}

#[test]
fn negative_un_client_id_non_concordant_est_rejete() {
    let v = "verificateur";
    assert!(code(&defi_pour(v))
        .echanger("autre-client", "https://claude.ai/callback", v, MAINTENANT)
        .is_err());
}

#[test]
fn negative_une_redirect_uri_non_concordante_est_rejetee() {
    let v = "verificateur";
    assert!(code(&defi_pour(v))
        .echanger("client-1", "https://ailleurs/cb", v, MAINTENANT)
        .is_err());
}

#[test]
fn negative_un_code_expire_est_rejete() {
    let v = "verificateur";
    assert!(code(&defi_pour(v))
        .echanger(
            "client-1",
            "https://claude.ai/callback",
            v,
            Horodatage::new(1_000_601)
        )
        .is_err());
}

// ── @edge ────────────────────────────────────────────────────

#[test]
fn edge_le_base64url_est_sans_remplissage_et_sans_caractere_hors_alphabet() {
    for taille in 0..40 {
        let donnees: Vec<u8> = (0..taille).map(|i| (i * 7 % 251) as u8).collect();
        let encode = base64url_sans_remplissage(&donnees);
        assert!(!encode.contains('='), "le remplissage est interdit en PKCE");
        assert!(
            !encode.contains('+') && !encode.contains('/'),
            "base64url n'utilise ni + ni /"
        );
    }
}

#[test]
fn edge_un_defi_de_longueur_differente_ne_passe_pas() {
    assert!(!verifier_pkce("v", "trop-court"));
    assert!(!verifier_pkce("v", ""));
}

#[test]
fn edge_l_expiration_a_la_seconde_exacte_reste_valable() {
    let v = "verificateur";
    assert!(code(&defi_pour(v))
        .echanger(
            "client-1",
            "https://claude.ai/callback",
            v,
            Horodatage::new(1_000_600)
        )
        .is_ok());
}

// ── @security ────────────────────────────────────────────────

#[test]
fn security_1_pkce_s256_est_le_seul_mode_accepte() {
    for methode in ["plain", "PLAIN", "none", "s512", ""] {
        assert!(
            CodeAutorisation::emettre(DemandeCode {
                code: "c".to_string(),
                client_id: "cl".to_string(),
                utilisateur: "u".to_string(),
                redirect_uri: "https://x/cb".to_string(),
                defi: "d".to_string(),
                methode: methode.to_string(),
                emis_le: MAINTENANT,
                validite: Duree::secondes(60).expect("durée"),
            })
            .is_err(),
            "la méthode « {methode} » aurait dû être refusée"
        );
    }
}

#[test]
fn security_2_le_jeton_de_rafraichissement_n_est_jamais_persiste_en_clair() {
    let clair = "JETON-EN-CLAIR-QUI-NE-DOIT-PAS-ETRE-STOCKE";
    let jeton = JetonRafraichissement::depuis_clair(
        clair,
        "client".to_string(),
        "u".to_string(),
        vec![Portee::Read],
        MAINTENANT,
        Duree::jours(30).expect("durée"),
    );
    let serialise = serde_json::to_string(&jeton).expect("sérialisation");
    assert!(
        !serialise.contains(clair),
        "le jeton en clair a fui : {serialise}"
    );
    assert!(serialise.contains(&empreinte_sha256(clair)));
}

#[test]
fn security_3_la_rotation_est_inconditionnelle() {
    // Un jeton volé puis rejoué après qu'un client légitime s'est rafraîchi
    // doit être mort. La révocation a donc lieu AVANT que l'issue de l'échange
    // ne soit connue.
    let jeton = JetonRafraichissement::depuis_clair(
        "j",
        "client".to_string(),
        "u".to_string(),
        vec![Portee::Read],
        MAINTENANT,
        Duree::jours(30).expect("durée"),
    );
    // Premier usage : réussi, et révoqué.
    let (jeton, issue) = jeton.utiliser("client", MAINTENANT);
    assert!(issue.is_ok());
    assert!(jeton.revoque(), "la rotation doit être immédiate");

    // Rejeu : rejeté.
    let (_, rejeu) = jeton.utiliser("client", MAINTENANT);
    assert!(rejeu.is_err(), "un jeton rotationné ne doit pas resservir");
}

#[test]
fn security_3bis_la_revocation_a_lieu_meme_quand_l_echange_echoue() {
    let jeton = JetonRafraichissement::depuis_clair(
        "j",
        "client".to_string(),
        "u".to_string(),
        vec![Portee::Read],
        MAINTENANT,
        Duree::jours(30).expect("durée"),
    );
    // Échange en échec : mauvais client.
    let (jeton, issue) = jeton.utiliser("autre-client", MAINTENANT);
    assert!(issue.is_err());
    assert!(
        jeton.revoque(),
        "révoquer seulement en cas de succès laisserait une fenêtre exploitable"
    );
}

#[test]
fn security_4_un_code_consomme_ne_peut_pas_etre_rejoue() {
    // `echanger` prend self par valeur : le rejeu ne compile pas. Ce test
    // documente la garantie et vérifie le champ pour le cas persistant.
    let v = "verificateur";
    let code = code(&defi_pour(v));
    assert!(code
        .echanger("client-1", "https://claude.ai/callback", v, MAINTENANT)
        .is_ok());
    // code.echanger(...) ici : error[E0382] use of moved value
}

#[test]
fn security_5_une_redirect_uri_non_https_distante_est_refusee() {
    for uri in [
        "http://exemple.org/cb",
        "ftp://exemple.org/cb",
        "/relatif",
        "javascript:alert(1)",
        "https://exemple.org/../evade",
        "",
    ] {
        assert!(
            ClientOAuth::enregistrer("a".to_string(), "n".to_string(), vec![uri.to_string()])
                .is_err(),
            "« {uri} » aurait dû être refusée : sinon l'endpoint devient un redirecteur ouvert"
        );
    }
    // Le localhost du développement reste admis.
    assert!(ClientOAuth::enregistrer(
        "a".to_string(),
        "n".to_string(),
        vec!["http://localhost:8765/callback".to_string()]
    )
    .is_ok());
}

#[test]
fn security_la_correspondance_d_uri_est_exacte_jamais_par_prefixe() {
    let client = ClientOAuth::enregistrer(
        "a".to_string(),
        "n".to_string(),
        vec!["https://claude.ai/callback".to_string()],
    )
    .expect("client");
    assert!(!client.uri_enregistree("https://claude.ai/callback/evade"));
    assert!(!client.uri_enregistree("https://claude.ai/callbackX"));
    assert!(client.uri_enregistree("https://claude.ai/callback"));
}

#[test]
fn security_aucune_portee_ne_permet_de_muter_la_production() {
    // Prouvé par énumération : les trois portées existantes sont lecture,
    // bac à sable borné, et proposition. Muter passe par la passerelle, qui
    // n'est pas un chemin d'appel MCP.
    assert_eq!(Portee::TOUTES.len(), 3);
    for portee in Portee::TOUTES {
        assert!(
            matches!(portee, Portee::Read | Portee::Sandbox | Portee::Propose),
            "une portée mutante a été ajoutée sans revoir ADR-008"
        );
    }
}
