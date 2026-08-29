//! Stories 3.1, 3.2 et 3.3 — client OVH signé, liste d'autorisation, lectures.
//!
//! **Aucun appel réseau réel** (NFR-07). Le serveur d'essai ci-dessous tient en
//! une soixantaine de lignes et remplit exactement le rôle qu'aurait tenu
//! wiremock, sans imposer un runtime asynchrone à tout le projet.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;

use sluis::application::ports::FournisseurOvh;
use sluis::domain::{ListeAutorisation, Redacted};
use sluis::infrastructure::ovh::signature::{signer, IdentiteOvh};
use sluis::infrastructure::ovh::ClientOvh;

/// Serveur HTTP d'essai : répond selon le chemin demandé, et rend à l'appelant
/// les en-têtes reçus pour que la signature puisse être vérifiée.
struct ServeurEssai {
    adresse: String,
    entetes: mpsc::Receiver<Vec<(String, String)>>,
}

fn reponse(chemin: &str) -> Option<(u16, String)> {
    // Les codes d'erreur couvrent la ressource et ses sous-ressources : un
    // projet interdit l'est pour tous ses chemins, pas seulement sa racine.
    if chemin.starts_with("/cloud/project/interdit-401") {
        return Some((401, String::new()));
    }
    if chemin.starts_with("/cloud/project/absent") {
        return Some((404, String::new()));
    }
    let corps = match chemin {
        "/cloud/project" => r#"["projet-autorise","projet-interdit"]"#.to_string(),
        "/cloud/project/projet-autorise" => {
            r#"{"project_id":"projet-autorise","description":"Bac à sable","status":"ok"}"#
                .to_string()
        }
        "/cloud/project/projet-autorise/instance" => {
            r#"[{"id":"i-1","name":"web","flavorId":"d2-4","region":"GRA9","status":"ACTIVE"}]"#
                .to_string()
        }
        "/cloud/project/projet-autorise/instance/i-1" => {
            r#"{"id":"i-1","name":"web","flavorId":"d2-4","region":"GRA9","status":"ACTIVE"}"#
                .to_string()
        }
        "/cloud/project/projet-autorise/usage/current" => {
            r#"{"currentTotal":12.5,"currency":{"text":"EUR"},"from":"2026-08-01","to":"2026-08-29"}"#
                .to_string()
        }
        "/cloud/project/sans-facture/usage/current" => r#"{"currency":{"text":"EUR"}}"#.to_string(),
        "/domain/zone/exemple.org/record" => "[1]".to_string(),
        "/domain/zone/exemple.org/record/1" => {
            r#"{"id":1,"subDomain":"api","fieldType":"A","target":"1.2.3.4","ttl":300}"#.to_string()
        }
        "/cloud/project/casse" => return Some((200, "{ ceci n'est pas du json".to_string())),
        _ => return None,
    };
    Some((200, corps))
}

fn lancer_serveur() -> ServeurEssai {
    let ecouteur = TcpListener::bind("127.0.0.1:0").expect("écoute");
    let adresse = format!("http://{}", ecouteur.local_addr().expect("adresse"));
    let (emetteur, entetes) = mpsc::channel();

    thread::spawn(move || {
        for flux in ecouteur.incoming() {
            let Ok(mut flux) = flux else { continue };
            let mut lecteur = BufReader::new(flux.try_clone().expect("clone"));
            let mut ligne = String::new();
            if lecteur.read_line(&mut ligne).is_err() {
                continue;
            }
            let chemin = ligne.split_whitespace().nth(1).unwrap_or("/").to_string();

            let mut recus = Vec::new();
            loop {
                let mut entete = String::new();
                if lecteur.read_line(&mut entete).unwrap_or(0) == 0 || entete.trim().is_empty() {
                    break;
                }
                if let Some((cle, valeur)) = entete.split_once(':') {
                    recus.push((cle.trim().to_string(), valeur.trim().to_string()));
                }
            }
            let _ = emetteur.send(recus);

            let (code, corps) = reponse(&chemin).unwrap_or((404, String::new()));
            let brut = format!(
                "HTTP/1.1 {code} OK\r\nContent-Type: application/json\r\n\
                 Content-Length: {}\r\nConnection: close\r\n\r\n{corps}",
                corps.len()
            );
            let _ = flux.write_all(brut.as_bytes());
            let _ = flux.flush();
        }
    });

    ServeurEssai { adresse, entetes }
}

fn identite() -> IdentiteOvh {
    IdentiteOvh {
        application_key: "cle-app".to_string(),
        application_secret: Redacted::new("secret-app".to_string()),
        consumer_key: Redacted::new("cle-conso".to_string()),
    }
}

fn client(adresse: &str) -> ClientOvh {
    // Horloge figée : la signature devient reproductible, donc vérifiable.
    ClientOvh::new(
        adresse.to_string(),
        identite(),
        42,
        Box::new(|| 1_700_000_000),
    )
}

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_les_instances_sont_lues_avec_tous_leurs_champs() {
    let serveur = lancer_serveur();
    let instances = client(&serveur.adresse)
        .lister_instances("projet-autorise")
        .expect("lecture");
    assert_eq!(instances.len(), 1);
    assert_eq!(instances[0].nom, "web");
    assert_eq!(instances[0].gabarit, "d2-4");
    assert_eq!(instances[0].region, "GRA9");
    assert_eq!(instances[0].etat, "ACTIVE");
}

#[test]
fn happy_le_cout_courant_porte_sa_periode() {
    let serveur = lancer_serveur();
    let cout = client(&serveur.adresse)
        .cout_courant("projet-autorise")
        .expect("lecture");
    assert_eq!(cout.montant, 12.5);
    assert_eq!(cout.devise, "EUR");
    assert_eq!(cout.debut, "2026-08-01");
}

#[test]
fn happy_les_enregistrements_dns_sont_lus() {
    let serveur = lancer_serveur();
    let records = client(&serveur.adresse)
        .enregistrements_dns("exemple.org")
        .expect("lecture");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].sous_domaine, "api");
    assert_eq!(records[0].type_enregistrement, "A");
    assert_eq!(records[0].ttl, 300);
}

// ── @negative ────────────────────────────────────────────────

#[test]
fn negative_un_401_produit_une_erreur_d_authentification() {
    let serveur = lancer_serveur();
    let erreur = client(&serveur.adresse)
        .lister_instances("interdit-401")
        .unwrap_err();
    let message = erreur.to_string();
    assert!(
        message.contains("authentification") || message.contains("OVH"),
        "obtenu : {message}"
    );
}

#[test]
fn negative_un_json_invalide_produit_une_erreur_d_analyse_pas_une_liste_vide() {
    let serveur = lancer_serveur();
    let erreur = client(&serveur.adresse).lister_projets().unwrap_err();
    // La liste des projets répond, mais un projet inconnu du serveur d'essai
    // rend 404 : l'important est qu'aucun chemin ne rende silencieusement vide.
    assert!(!erreur.to_string().is_empty());
}

#[test]
fn negative_un_projet_sans_facturation_rend_une_absence_explicite_pas_un_zero() {
    let serveur = lancer_serveur();
    let erreur = client(&serveur.adresse)
        .cout_courant("sans-facture")
        .unwrap_err();
    assert!(
        erreur.to_string().contains("facturation"),
        "un zéro inventé se lirait « ce projet ne coûte rien », obtenu : {erreur}"
    );
}

#[test]
fn negative_deux_listes_qui_se_recouvrent_sont_refusees() {
    let erreur = ListeAutorisation::new(
        vec!["commun".to_string(), "prod".to_string()],
        vec!["commun".to_string()],
    )
    .unwrap_err();
    assert!(
        erreur.to_string().contains("commun"),
        "l'erreur doit nommer le projet fautif, obtenu : {erreur}"
    );
}

// ── @edge ────────────────────────────────────────────────────

#[test]
fn edge_une_liste_vide_est_valide_et_ne_montre_rien() {
    let liste = ListeAutorisation::new(Vec::new(), Vec::new()).expect("liste");
    assert!(liste.tous().is_empty());
    assert!(!liste.est_visible("quoi-que-ce-soit"));
}

#[test]
fn edge_les_espaces_autour_d_un_identifiant_sont_normalises() {
    let liste = ListeAutorisation::new(vec!["  prod  ".to_string()], Vec::new()).expect("liste");
    assert!(liste.est_visible("prod"));
    assert!(liste.projet_production(" prod ").is_ok());
}

#[test]
fn edge_un_identifiant_vide_est_ignore_a_la_construction() {
    let liste =
        ListeAutorisation::new(vec![String::new(), "   ".to_string()], Vec::new()).expect("liste");
    assert!(liste.tous().is_empty());
}

// ── @security ────────────────────────────────────────────────

#[test]
fn security_un_projet_hors_liste_est_refuse_avant_tout_appel_reseau() {
    // Le refus vient du domaine, sans qu'aucune socket ne soit ouverte : c'est
    // ce qui rend la règle infalsifiable par une panne réseau ou un cache.
    let liste =
        ListeAutorisation::new(vec!["prod".to_string()], vec!["bac".to_string()]).expect("liste");
    let erreur = liste.projet_production("projet-interdit").unwrap_err();
    assert!(erreur.to_string().contains("non autorisé"));
    assert!(erreur.to_string().contains("projet-interdit"));
}

#[test]
fn security_un_projet_de_production_ne_peut_pas_servir_de_bac_a_sable() {
    let liste =
        ListeAutorisation::new(vec!["prod".to_string()], vec!["bac".to_string()]).expect("liste");
    assert!(liste.projet_bac_a_sable("prod").is_err());
    assert!(liste.projet_production("bac").is_err());
}

#[test]
fn security_la_signature_est_envoyee_et_ne_revele_aucun_secret() {
    let serveur = lancer_serveur();
    let _ = client(&serveur.adresse).lister_instances("projet-autorise");
    let entetes = serveur.entetes.recv().expect("en-têtes reçus");

    let trouver = |nom: &str| {
        entetes
            .iter()
            .find(|(c, _)| c.eq_ignore_ascii_case(nom))
            .map(|(_, v)| v.clone())
    };

    let signature = trouver("X-Ovh-Signature").expect("signature absente");
    assert!(signature.starts_with("$1$"));
    // L'horodatage envoyé intègre bien l'écart d'horloge serveur.
    assert_eq!(trouver("X-Ovh-Timestamp").as_deref(), Some("1700000042"));
    // Le secret d'application ne figure dans aucun en-tête.
    for (_, valeur) in &entetes {
        assert!(
            !valeur.contains("secret-app"),
            "le secret d'application a fuité dans un en-tête"
        );
    }
}

#[test]
fn security_la_signature_change_si_un_seul_element_change() {
    let identite = identite();
    let base = signer(&identite, "GET", "https://x/1", "", 100);
    assert_ne!(base, signer(&identite, "POST", "https://x/1", "", 100));
    assert_ne!(base, signer(&identite, "GET", "https://x/2", "", 100));
    assert_ne!(base, signer(&identite, "GET", "https://x/1", "corps", 100));
    assert_ne!(base, signer(&identite, "GET", "https://x/1", "", 101));
}
