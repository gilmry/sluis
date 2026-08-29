//! Story 0.5 — Journal d'audit append-only.
//!
//! Deux invariants du Brief §10 se rencontrent ici. Le septième, « une entrée
//! de journal est immuable », est porté par la **forme du port** : `AuditLog`
//! n'expose qu'`append`, donc modifier ou supprimer n'est pas refusé, c'est
//! inexprimable. Le neuvième, « un secret ne franchit jamais la sortie », est
//! prouvé sur le journal, qui est le chemin de fuite le plus probable.

use sluis::application::ports::AuditLog;
use sluis::domain::{AuditEntry, Redacted, Tier};
use sluis::infrastructure::audit::JsonlAuditLog;

fn dossier_temporaire(nom: &str) -> std::path::PathBuf {
    let base = std::env::temp_dir().join(format!("sluis-test-{nom}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).expect("création du dossier de test");
    base
}

// ─────────────────────────────────────────────────────────────
// @happy
// ─────────────────────────────────────────────────────────────

#[test]
fn happy_un_appel_reussi_est_journalise() {
    let dossier = dossier_temporaire("happy-succes");
    let chemin = dossier.join("audit.jsonl");
    let journal = JsonlAuditLog::new(&chemin).expect("ouverture du journal");

    let entree = AuditEntry::new(
        "2026-08-29T10:00:00Z".to_string(),
        "sluis_inventory".to_string(),
        Tier::Two,
        "a1b2c3".to_string(),
        Ok(()),
    );
    journal.append(&entree).expect("écriture");

    let contenu = std::fs::read_to_string(&chemin).expect("relecture");
    assert!(contenu.contains("sluis_inventory"));
    assert!(contenu.contains("a1b2c3"));
    assert_eq!(contenu.lines().count(), 1, "une entrée, une ligne");
}

#[test]
fn happy_un_appel_en_echec_est_journalise_aussi() {
    let dossier = dossier_temporaire("happy-echec");
    let chemin = dossier.join("audit.jsonl");
    let journal = JsonlAuditLog::new(&chemin).expect("ouverture");

    let entree = AuditEntry::new(
        "2026-08-29T10:00:01Z".to_string(),
        "tf_plan".to_string(),
        Tier::Two,
        "deadbeef".to_string(),
        Err("moteur absent".to_string()),
    );
    journal.append(&entree).expect("écriture");

    let contenu = std::fs::read_to_string(&chemin).expect("relecture");
    assert!(
        contenu.contains("moteur absent"),
        "un échec doit laisser une trace, sinon le journal ne montre que les succès"
    );
}

// ─────────────────────────────────────────────────────────────
// @negative
// ─────────────────────────────────────────────────────────────

#[test]
fn negative_un_journal_non_inscriptible_fait_echouer_l_ouverture() {
    // Le comportement voulu : refuser d'agir plutôt que d'agir sans trace.
    let inexistant = std::path::Path::new("/dossier-qui-n-existe-pas-du-tout/audit.jsonl");
    let resultat = JsonlAuditLog::new(inexistant);
    assert!(
        resultat.is_err(),
        "sans journal inscriptible, il faut refuser, pas continuer en silence"
    );
}

#[test]
fn negative_l_erreur_d_ouverture_nomme_le_chemin() {
    let inexistant = std::path::Path::new("/dossier-absent/audit.jsonl");
    let erreur = JsonlAuditLog::new(inexistant).unwrap_err();
    assert!(
        erreur.to_string().contains("audit.jsonl"),
        "l'erreur doit être diagnosticable, obtenu : {erreur}"
    );
}

// ─────────────────────────────────────────────────────────────
// @edge
// ─────────────────────────────────────────────────────────────

#[test]
fn edge_ecritures_concurrentes_ne_perdent_ni_n_entrelacent_aucune_ligne() {
    let dossier = dossier_temporaire("edge-concurrence");
    let chemin = dossier.join("audit.jsonl");
    let journal = std::sync::Arc::new(JsonlAuditLog::new(&chemin).expect("ouverture"));

    let mut fils = Vec::new();
    for i in 0..16 {
        let journal = std::sync::Arc::clone(&journal);
        fils.push(std::thread::spawn(move || {
            for j in 0..16 {
                let entree = AuditEntry::new(
                    "2026-08-29T10:00:02Z".to_string(),
                    format!("outil_{i}_{j}"),
                    Tier::Two,
                    format!("{i:02x}{j:02x}"),
                    Ok(()),
                );
                journal.append(&entree).expect("écriture concurrente");
            }
        }));
    }
    for fil in fils {
        fil.join().expect("fil d'exécution");
    }

    let contenu = std::fs::read_to_string(&chemin).expect("relecture");
    assert_eq!(
        contenu.lines().count(),
        256,
        "aucune entrée ne doit être perdue"
    );
    for ligne in contenu.lines() {
        serde_json::from_str::<serde_json::Value>(ligne)
            .expect("chaque ligne doit rester un JSON valide, donc non entrelacée");
    }
}

#[test]
fn edge_une_entree_volumineuse_reste_sur_une_seule_ligne() {
    let dossier = dossier_temporaire("edge-volumineuse");
    let chemin = dossier.join("audit.jsonl");
    let journal = JsonlAuditLog::new(&chemin).expect("ouverture");

    let entree = AuditEntry::new(
        "2026-08-29T10:00:03Z".to_string(),
        "outil".to_string(),
        Tier::One,
        "ff".to_string(),
        Err("détail\navec\ndes\nsauts\nde\nligne".repeat(1000)),
    );
    journal.append(&entree).expect("écriture");

    let contenu = std::fs::read_to_string(&chemin).expect("relecture");
    assert_eq!(
        contenu.lines().count(),
        1,
        "les sauts de ligne du contenu doivent être échappés, sinon une entrée \
         peut se faire passer pour plusieurs"
    );
}

// ─────────────────────────────────────────────────────────────
// @security
// ─────────────────────────────────────────────────────────────

#[test]
fn security_le_journal_ne_contient_aucun_secret() {
    const SECRET: &str = "SECRET-QUI-NE-DOIT-JAMAIS-SORTIR";
    let dossier = dossier_temporaire("security-secret");
    let chemin = dossier.join("audit.jsonl");
    let journal = JsonlAuditLog::new(&chemin).expect("ouverture");

    let entree = AuditEntry::new(
        "2026-08-29T10:00:04Z".to_string(),
        "ovh_projects_list".to_string(),
        Tier::Two,
        "0f0f".to_string(),
        Ok(()),
    )
    .avec_secret(Redacted::new(SECRET.to_string()));

    journal.append(&entree).expect("écriture");

    let contenu = std::fs::read_to_string(&chemin).expect("relecture");
    assert!(
        !contenu.contains(SECRET),
        "le journal est le chemin de fuite le plus probable : {contenu}"
    );
    assert!(contenu.contains("«redacted»"));
}

#[test]
fn security_le_journal_est_append_only_les_entrees_precedentes_survivent() {
    let dossier = dossier_temporaire("security-append");
    let chemin = dossier.join("audit.jsonl");

    {
        let journal = JsonlAuditLog::new(&chemin).expect("ouverture");
        journal
            .append(&AuditEntry::new(
                "2026-08-29T10:00:05Z".to_string(),
                "premier".to_string(),
                Tier::Two,
                "01".to_string(),
                Ok(()),
            ))
            .expect("écriture");
    }
    // Une réouverture ne doit pas tronquer : c'est le mode O_APPEND qui le
    // garantit, et une régression ici effacerait l'historique en silence.
    {
        let journal = JsonlAuditLog::new(&chemin).expect("réouverture");
        journal
            .append(&AuditEntry::new(
                "2026-08-29T10:00:06Z".to_string(),
                "second".to_string(),
                Tier::Two,
                "02".to_string(),
                Ok(()),
            ))
            .expect("écriture");
    }

    let contenu = std::fs::read_to_string(&chemin).expect("relecture");
    assert!(
        contenu.contains("premier"),
        "la réouverture a tronqué le journal"
    );
    assert!(contenu.contains("second"));
    assert_eq!(contenu.lines().count(), 2);
}

#[test]
fn security_le_port_n_expose_ni_modification_ni_suppression() {
    // Ce test ne peut pas échouer à l'exécution : il documente que l'immuabilité
    // est portée par la *forme* du trait. Si quelqu'un ajoutait `update` ou
    // `delete` à AuditLog, ce fichier resterait vert — mais le trait ne
    // compilerait plus tel qu'utilisé ici, et la revue verrait la méthode
    // apparaître. La garantie est structurelle, pas comportementale.
    fn accepte_tout_journal<J: AuditLog>(_journal: &J) {}
    let dossier = dossier_temporaire("security-forme");
    let journal = JsonlAuditLog::new(&dossier.join("audit.jsonl")).expect("ouverture");
    accepte_tout_journal(&journal);
}
