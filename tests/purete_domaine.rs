//! Story 0.2 — Gate mécanique de pureté du domaine.
//!
//! L'architecture hexagonale pose que `src/domain/` n'importe aucun crate
//! d'infrastructure. Tant que cette règle vit dans un document, elle dépend de
//! la vigilance ; ici elle échoue en CI.
//!
//! **Choix d'implémentation** : le livrable 04 parlait d'un « job
//! purete-domaine ». C'est un test d'intégration plutôt qu'un script externe,
//! pour trois raisons. Il tourne à l'identique en local et en CI, ce qui est la
//! redondance DRY que `gates.md` demande. Il ne dépend d'aucun outillage absent
//! de la machine. Et il ne peut pas être contourné par `--no-verify`, puisqu'il
//! n'est pas un hook mais une condition de `cargo test`.

use std::fs;
use std::path::{Path, PathBuf};

/// Ce que le domaine n'a pas le droit de connaître.
///
/// Les crates d'infrastructure, d'abord. Mais aussi les portes d'entrée vers le
/// monde extérieur de la bibliothèque standard : un domaine qui lit un fichier
/// ou ouvre une socket n'est plus testable sans ce fichier ni cette socket.
///
/// `SystemTime::now` et `Instant::now` sont dans la liste pour une raison
/// précise : le domaine doit recevoir son horloge par le port `Clock`. Sans
/// cela, l'expiration d'un jeton ou d'une fenêtre de dérogation devient
/// intestable, or ce sont exactement les invariants les plus sensibles.
const INTERDITS: &[(&str, &str)] = &[
    ("reqwest", "client HTTP — passe par un adaptateur"),
    ("sqlx", "accès base — passe par un port de dépôt"),
    ("actix_web", "serveur HTTP — appartient aux adaptateurs"),
    (
        "tokio",
        "runtime asynchrone — le domaine reste synchrone et pur",
    ),
    ("hyper", "client/serveur HTTP — passe par un adaptateur"),
    ("std::fs", "système de fichiers — passe par un port"),
    ("std::net", "réseau — passe par un port"),
    (
        "std::process",
        "exécution de processus — passe par un runner",
    ),
    ("std::env", "environnement — la configuration est injectée"),
    (
        "SystemTime::now",
        "horloge — le domaine reçoit son temps par le port Clock",
    ),
    (
        "Instant::now",
        "horloge — le domaine reçoit son temps par le port Clock",
    ),
];

fn fichiers_rust(racine: &Path) -> Vec<PathBuf> {
    let mut trouves = Vec::new();
    let Ok(entrees) = fs::read_dir(racine) else {
        return trouves;
    };
    for entree in entrees.flatten() {
        let chemin = entree.path();
        if chemin.is_dir() {
            trouves.extend(fichiers_rust(&chemin));
        } else if chemin.extension().is_some_and(|e| e == "rs") {
            trouves.push(chemin);
        }
    }
    trouves
}

/// Retire les commentaires, de ligne comme de bloc.
///
/// Sans cela la gate se déclenche sur sa propre documentation : le
/// commentaire de `src/domain/mod.rs` cite nommément les crates interdits pour
/// expliquer la règle. Une gate qui échoue sur une phrase de prose est une
/// gate qu'on finit par désactiver, ce que `gates.md` décrit précisément comme
/// le mécanisme par lequel un plancher perd sa crédibilité.
fn sans_commentaires(source: &str) -> String {
    // Sur des `char` et non des octets : le domaine est commenté en français,
    // et découper « — » au milieu fait paniquer le slicing.
    let caracteres: Vec<char> = source.chars().collect();
    let mut sortie = String::with_capacity(source.len());
    let mut i = 0;
    let (mut dans_chaine, mut dans_ligne, mut dans_bloc) = (false, false, false);

    while i < caracteres.len() {
        let courant = caracteres[i];
        let suivant = caracteres.get(i + 1).copied();

        if dans_ligne {
            if courant == '\n' {
                dans_ligne = false;
                sortie.push('\n');
            }
            i += 1;
        } else if dans_bloc {
            if courant == '*' && suivant == Some('/') {
                dans_bloc = false;
                i += 2;
            } else {
                if courant == '\n' {
                    sortie.push('\n');
                }
                i += 1;
            }
        } else if dans_chaine {
            if courant == '\\' {
                i += 2;
                continue;
            }
            if courant == '"' {
                dans_chaine = false;
            }
            sortie.push(courant);
            i += 1;
        } else if courant == '/' && suivant == Some('/') {
            dans_ligne = true;
            i += 2;
        } else if courant == '/' && suivant == Some('*') {
            dans_bloc = true;
            i += 2;
        } else {
            if courant == '"' {
                dans_chaine = true;
            }
            sortie.push(courant);
            i += 1;
        }
    }
    sortie
}

/// Retire les blocs `#[cfg(test)] mod … { … }`.
///
/// Un test unitaire du domaine a le droit d'utiliser ce que le domaine
/// s'interdit : il n'est pas livré, il n'entre pas dans le binaire de
/// production. La tolérance est délibérée et le test `edge_` plus bas la prouve.
fn sans_blocs_de_test(source: &str) -> String {
    let mut sortie = String::with_capacity(source.len());
    let mut reste = source;

    while let Some(position) = reste.find("#[cfg(test)]") {
        sortie.push_str(&reste[..position]);
        let apres = &reste[position..];
        // Se placer sur l'accolade ouvrante du bloc, puis l'apparier.
        let Some(debut_bloc) = apres.find('{') else {
            break;
        };
        let mut profondeur = 0_i32;
        let mut fin = None;
        for (index, caractere) in apres[debut_bloc..].char_indices() {
            match caractere {
                '{' => profondeur += 1,
                '}' => {
                    profondeur -= 1;
                    if profondeur == 0 {
                        fin = Some(debut_bloc + index + 1);
                        break;
                    }
                }
                _ => {}
            }
        }
        match fin {
            Some(fin) => reste = &apres[fin..],
            None => break,
        }
    }
    sortie.push_str(reste);
    sortie
}

// ─────────────────────────────────────────────────────────────
// @happy — le domaine est pur
// ─────────────────────────────────────────────────────────────

#[test]
fn happy_le_domaine_n_importe_aucun_crate_d_infrastructure() {
    let racine = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/domain");
    let fichiers = fichiers_rust(&racine);
    assert!(
        !fichiers.is_empty(),
        "aucun fichier trouvé sous src/domain : la gate se croirait verte en \
         ne vérifiant rien"
    );

    let mut violations = Vec::new();
    for fichier in &fichiers {
        let source = fs::read_to_string(fichier).expect("fichier du domaine illisible");
        let code = sans_blocs_de_test(&sans_commentaires(&source));
        for (interdit, pourquoi) in INTERDITS {
            if code.contains(interdit) {
                violations.push(format!(
                    "  {} importe « {} » ({})",
                    fichier
                        .strip_prefix(env!("CARGO_MANIFEST_DIR"))
                        .unwrap_or(fichier)
                        .display(),
                    interdit,
                    pourquoi
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "pureté du domaine violée — l'architecture hexagonale impose que \
         src/domain/ ne dépende de rien :\n{}",
        violations.join("\n")
    );
}

// ─────────────────────────────────────────────────────────────
// @negative — la gate détecte réellement une violation
// ─────────────────────────────────────────────────────────────

#[test]
fn negative_la_gate_detecte_chaque_crate_interdit() {
    // Sans ce test, une gate qui ne détecterait rien passerait au vert pour de
    // mauvaises raisons — l'« assurance fausse » que gates.md dit pire qu'une
    // absence de gate.
    for (interdit, _) in INTERDITS {
        let faux_source = format!("use {interdit}::quelque_chose;\npub struct Entite;\n");
        let code = sans_blocs_de_test(&sans_commentaires(&faux_source));
        assert!(
            code.contains(interdit),
            "la gate ne détecterait pas « {interdit} »"
        );
    }
}

// ─────────────────────────────────────────────────────────────
// @edge — la tolérance aux blocs de test est bien bornée
// ─────────────────────────────────────────────────────────────

#[test]
fn edge_un_import_sous_cfg_test_est_tolere() {
    let source = "pub struct Entite;\n\
                  #[cfg(test)]\n\
                  mod tests {\n    use tokio::runtime;\n}\n";
    assert!(
        !sans_blocs_de_test(source).contains("tokio"),
        "un import réservé aux tests ne doit pas faire échouer la gate"
    );
}

#[test]
fn edge_un_import_hors_cfg_test_reste_detecte_meme_avec_un_bloc_de_test_present() {
    // Le piège : le retrait du bloc de test ne doit pas avaler le code qui suit.
    let source = "#[cfg(test)]\n\
                  mod tests {\n    fn interne() { let _ = 1; }\n}\n\
                  use reqwest::Client;\n";
    assert!(
        sans_blocs_de_test(source).contains("reqwest"),
        "le retrait du bloc de test a avalé du code de production"
    );
}

#[test]
fn edge_une_mention_en_commentaire_ne_declenche_pas_la_gate() {
    // Cas rencontré pour de vrai au premier passage : la documentation du
    // domaine cite les crates interdits pour expliquer la règle.
    let source = "//! Ce module n'importe jamais reqwest ni sqlx.\n\
                  /* ni tokio, ni actix_web */\n\
                  pub struct Entite;\n";
    let code = sans_blocs_de_test(&sans_commentaires(source));
    for interdit in ["reqwest", "sqlx", "tokio", "actix_web"] {
        assert!(
            !code.contains(interdit),
            "une mention en commentaire ne doit pas faire échouer la gate : {interdit}"
        );
    }
}

#[test]
fn edge_un_import_reel_apres_un_commentaire_reste_detecte() {
    let source = "//! On ne parle surtout pas de reqwest ici.\n\
                  use sqlx::Pool;\n";
    let code = sans_blocs_de_test(&sans_commentaires(source));
    assert!(
        code.contains("sqlx"),
        "le retrait des commentaires a avalé du code de production"
    );
}

#[test]
fn edge_blocs_de_test_imbriques_sont_correctement_apparies() {
    let source = "#[cfg(test)]\n\
                  mod tests {\n    mod interne { fn f() {} }\n}\n\
                  use sqlx::Pool;\n";
    assert!(
        sans_blocs_de_test(source).contains("sqlx"),
        "l'appariement des accolades imbriquées est incorrect"
    );
}

// ─────────────────────────────────────────────────────────────
// @security — la gate ne peut pas être contournée
// ─────────────────────────────────────────────────────────────

#[test]
fn security_la_gate_est_un_test_donc_rejouee_cote_serveur() {
    // Un hook Git local se contourne par `--no-verify`. Ce test-ci ne se
    // contourne pas : il est une condition de `cargo test`, que la CI rejoue.
    // On vérifie ici que la liste d'interdits n'a pas été vidée, seule façon
    // discrète de neutraliser la gate tout en la laissant verte.
    assert!(
        INTERDITS.len() >= 11,
        "la liste d'interdits a été réduite : la gate serait verte sans rien vérifier"
    );
    for (interdit, pourquoi) in INTERDITS {
        assert!(!interdit.is_empty(), "un interdit vide matcherait tout");
        assert!(
            !pourquoi.trim().is_empty(),
            "chaque interdit doit dire pourquoi il l'est, sinon il se fera retirer"
        );
    }
}
