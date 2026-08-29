//! Story 0.4 — Erreurs typées et rédaction des secrets.
//!
//! Les quatre classes obligatoires. La classe `@security` est ici le cœur :
//! elle prouve qu'**aucun** chemin de sortie ne révèle un secret, en énumérant
//! les traits qui pourraient le faire fuiter plutôt qu'en échantillonnant.

use sluis::domain::{AppError, Redacted};

// ─────────────────────────────────────────────────────────────
// @happy — chemin nominal
// ─────────────────────────────────────────────────────────────

#[test]
fn happy_apperror_porte_un_message_utilisateur() {
    let err = AppError::EngineMissing {
        binaire: "terraform".to_string(),
    };
    let message = err.to_string();
    assert!(
        message.contains("terraform"),
        "l'erreur doit nommer le binaire manquant, obtenu : {message}"
    );
}

#[test]
fn happy_redacted_rend_la_valeur_a_qui_la_demande_explicitement() {
    let secret = Redacted::new("cle-application-ovh".to_string());
    assert_eq!(secret.exposer(), "cle-application-ovh");
}

// ─────────────────────────────────────────────────────────────
// @negative — défaillance correcte, jamais de panique
// ─────────────────────────────────────────────────────────────

#[test]
fn negative_chaque_variante_porte_un_message_non_vide() {
    let variantes = vec![
        AppError::EngineMissing {
            binaire: "helm".to_string(),
        },
        AppError::TierViolation {
            raison: "plan Tier 2 visant production".to_string(),
        },
        AppError::ProjetNonAutorise {
            projet: "prj-inconnu".to_string(),
        },
        AppError::CheminHorsRacine {
            chemin: "../../etc".to_string(),
        },
        AppError::Configuration {
            detail: "TTL absent".to_string(),
        },
    ];
    for variante in variantes {
        let message = variante.to_string();
        assert!(
            !message.trim().is_empty(),
            "toute variante doit porter un message utilisateur"
        );
    }
}

#[test]
fn negative_redacted_vide_reste_masque() {
    let vide = Redacted::new(String::new());
    assert_eq!(format!("{vide}"), "«redacted»");
}

// ─────────────────────────────────────────────────────────────
// @edge — bornes
// ─────────────────────────────────────────────────────────────

#[test]
fn edge_redacted_tres_long_ne_fuit_pas_par_sa_longueur() {
    let long = Redacted::new("a".repeat(100_000));
    let rendu = format!("{long}");
    assert_eq!(
        rendu, "«redacted»",
        "le rendu ne doit pas varier avec la taille du secret"
    );
    assert_eq!(
        rendu.len(),
        format!("{}", Redacted::new("x".to_string())).len(),
        "un secret long et un secret court doivent rendre exactement la même chose, \
         sinon la longueur elle-même devient un canal de fuite"
    );
}

#[test]
fn edge_redacted_tolere_les_caracteres_de_controle() {
    let bizarre = Redacted::new("cle\navec\0des\tcontroles".to_string());
    assert_eq!(format!("{bizarre}"), "«redacted»");
    assert_eq!(format!("{bizarre:?}"), "«redacted»");
}

// ─────────────────────────────────────────────────────────────
// @security — étanchéité prouvée sur tous les chemins de sortie
// ─────────────────────────────────────────────────────────────

const SECRET: &str = "SECRET-QUI-NE-DOIT-JAMAIS-SORTIR";

#[test]
fn security_display_ne_revele_pas_le_secret() {
    let secret = Redacted::new(SECRET.to_string());
    let rendu = format!("{secret}");
    assert!(!rendu.contains(SECRET), "Display a fuité : {rendu}");
    assert_eq!(rendu, "«redacted»");
}

#[test]
fn security_debug_ne_revele_pas_le_secret() {
    let secret = Redacted::new(SECRET.to_string());
    let rendu = format!("{secret:?}");
    assert!(!rendu.contains(SECRET), "Debug a fuité : {rendu}");
}

#[test]
fn security_debug_dans_une_structure_englobante_ne_revele_pas_le_secret() {
    // Le piège réel : le secret ne fuit pas seul, il fuit dans le `{:?}` d'une
    // structure qui le contient, typiquement une ligne de journal.
    #[derive(Debug)]
    #[allow(dead_code)]
    struct Config {
        endpoint: String,
        cle: Redacted<String>,
    }
    let config = Config {
        endpoint: "https://eu.api.ovh.com/1.0".to_string(),
        cle: Redacted::new(SECRET.to_string()),
    };
    let rendu = format!("{config:?}");
    assert!(
        !rendu.contains(SECRET),
        "le Debug dérivé de la structure englobante a fuité : {rendu}"
    );
    assert!(
        rendu.contains("eu.api.ovh.com"),
        "le masquage ne doit pas cacher ce qui n'est pas secret"
    );
}

#[test]
fn security_serialize_ne_revele_pas_le_secret() {
    let secret = Redacted::new(SECRET.to_string());
    let json = serde_json::to_string(&secret).expect("la sérialisation ne doit pas échouer");
    assert!(!json.contains(SECRET), "Serialize a fuité : {json}");
    assert_eq!(json, "\"«redacted»\"");
}

#[test]
fn security_une_erreur_ne_transporte_jamais_un_secret_en_clair() {
    // Une variante d'erreur qui accepterait un secret le ferait fuiter par
    // Display. Le seul type autorisé à porter un secret est Redacted.
    let err = AppError::Authentification {
        secret: Redacted::new(SECRET.to_string()),
    };
    let message = err.to_string();
    assert!(!message.contains(SECRET), "AppError a fuité : {message}");
    let debug = format!("{err:?}");
    assert!(!debug.contains(SECRET), "AppError Debug a fuité : {debug}");
}
