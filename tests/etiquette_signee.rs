//! Issue #24, ADR-013, story 13.1 : passerelle par étiquette signée.
//!
//! Sur un dépôt privé, l'approbation de Tier 1 est une étiquette
//! `approuve/<empreinte courte>` signée par le superviseur. Sluis rend la
//! commande à signer et lit l'étiquette ; il ne signe ni ne pousse rien.
//!
//! **Aucun appel réseau réel** (NFR-07). Le faux GitHub rend compte de chaque
//! requête reçue : c'est ce qui prouve que la passerelle ne fait que lire.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use sluis::application::mise_en_ligne::MiseEnLigne;
use sluis::application::ports::{
    EtatApprobation, EtatGate, GatePlancher, PasserelleApprobation, VerificateurGates,
};
use sluis::domain::{
    commande_signature, nom_etiquette_approbation, Action, AppError, Empreinte, Environnement,
    MessageEtiquette, PlanChangement, Redacted, Tier, FORMAT_MESSAGE_ETIQUETTE,
};
use sluis::infrastructure::etiquette_signee::PasserelleEtiquetteSignee;
use sluis::infrastructure::mcp::outils_mise_en_ligne::OutilMiseEnLigne;
use sluis::infrastructure::mcp::Outil;

const COMMIT: &str = "1111111111111111111111111111111111111111";
const AUTRE_COMMIT: &str = "2222222222222222222222222222222222222222";
const OBJET_ETIQUETTE: &str = "3333333333333333333333333333333333333333";
const SIGNATAIRE: &str = "superviseur@exemple.org";
const TITULAIRE_JETON: &str = "titulaire-jeton";

// ── Faux GitHub ──────────────────────────────────────────────

struct ServeurEssai {
    adresse: String,
    requetes: mpsc::Receiver<String>,
    /// L'objet d'étiquette servi ; `None` : aucune étiquette posée.
    etiquette: Arc<Mutex<Option<serde_json::Value>>>,
}

impl ServeurEssai {
    fn recues(&self) -> Vec<String> {
        self.requetes.try_iter().collect()
    }

    fn poser(&self, objet: serde_json::Value) {
        *self.etiquette.lock().expect("verrou") = Some(objet);
    }
}

fn lancer() -> ServeurEssai {
    let ecouteur = TcpListener::bind("127.0.0.1:0").expect("écoute");
    let adresse = format!("http://{}", ecouteur.local_addr().expect("adresse"));
    let (emetteur, requetes) = mpsc::channel();
    let etiquette: Arc<Mutex<Option<serde_json::Value>>> = Arc::new(Mutex::new(None));
    let servie = etiquette.clone();
    let nom = nom_etiquette_approbation(plan().empreinte());

    thread::spawn(move || {
        for flux in ecouteur.incoming() {
            let Ok(mut flux) = flux else { continue };
            let mut lecteur = BufReader::new(flux.try_clone().expect("clone"));
            let mut ligne = String::new();
            if lecteur.read_line(&mut ligne).is_err() {
                continue;
            }
            let mut morceaux = ligne.split_whitespace();
            let methode = morceaux.next().unwrap_or("").to_string();
            let chemin = morceaux.next().unwrap_or("/").to_string();
            let mut longueur = 0usize;
            loop {
                let mut entete = String::new();
                if lecteur.read_line(&mut entete).unwrap_or(0) == 0 || entete.trim().is_empty() {
                    break;
                }
                if let Some((cle, valeur)) = entete.split_once(':') {
                    if cle.trim().eq_ignore_ascii_case("content-length") {
                        longueur = valeur.trim().parse().unwrap_or(0);
                    }
                }
            }
            let mut recu = vec![0u8; longueur];
            let _ = lecteur.read_exact(&mut recu);
            let _ = emetteur.send(format!("{methode} {chemin}"));

            let objet = servie.lock().expect("verrou").clone();
            let (code, corps) = match (methode.as_str(), chemin.as_str()) {
                ("GET", "/repos/gilmry/cible/commits/v0.1.0") => {
                    (200, format!(r#"{{"sha":"{COMMIT}"}}"#))
                }
                ("GET", "/user") => (200, format!(r#"{{"login":"{TITULAIRE_JETON}"}}"#)),
                ("GET", c) if c == format!("/repos/gilmry/cible/git/ref/tags/{nom}") => {
                    match &objet {
                        None => (404, String::new()),
                        Some(o) => {
                            let type_objet = if o.get("leger").is_some() {
                                "commit"
                            } else {
                                "tag"
                            };
                            (
                                200,
                                serde_json::json!({
                                    "ref": format!("refs/tags/{nom}"),
                                    "object": {"type": type_objet, "sha": OBJET_ETIQUETTE},
                                })
                                .to_string(),
                            )
                        }
                    }
                }
                ("GET", c) if c == format!("/repos/gilmry/cible/git/tags/{OBJET_ETIQUETTE}") => {
                    match &objet {
                        None => (404, String::new()),
                        Some(o) => (200, o.to_string()),
                    }
                }
                _ => (404, String::new()),
            };
            let brut = format!(
                "HTTP/1.1 {code} OK\r\nContent-Type: application/json\r\n\
                 Content-Length: {}\r\nConnection: close\r\n\r\n{corps}",
                corps.len()
            );
            let _ = flux.write_all(brut.as_bytes());
            let _ = flux.flush();
        }
    });

    ServeurEssai {
        adresse,
        requetes,
        etiquette,
    }
}

fn passerelle(adresse: &str) -> PasserelleEtiquetteSignee {
    PasserelleEtiquetteSignee::new(
        "gilmry".to_string(),
        "cible".to_string(),
        Redacted::new("jeton-lecture-essai".to_string()),
        adresse.to_string(),
    )
}

fn plan() -> PlanChangement {
    PlanChangement::new(
        Action::MiseEnLigne,
        Environnement::Staging,
        Tier::One,
        "cible@v0.1.0".to_string(),
        "met en ligne cible sur staging, 3 gate(s) du plancher au vert".to_string(),
        "secrets : vert\nsbom : vert\nscan-conteneur : vert".to_string(),
    )
    .expect("plan")
}

/// Un objet d'étiquette tel que `GET /git/tags/{sha}` le rend, signature
/// vérifiée par GitHub, message suivi du bloc de signature comme git l'écrit.
fn etiquette(message: &str, commit: &str, verifiee: bool) -> serde_json::Value {
    serde_json::json!({
        "tag": nom_etiquette_approbation(plan().empreinte()),
        "sha": OBJET_ETIQUETTE,
        "message": format!(
            "{message}\n-----BEGIN SSH SIGNATURE-----\nU1NIU0lH\n-----END SSH SIGNATURE-----\n"
        ),
        "tagger": {"name": "Superviseur", "email": SIGNATAIRE, "date": "2026-10-24T10:00:00Z"},
        "object": {"type": "commit", "sha": commit},
        "verification": {
            "verified": verifiee,
            "reason": if verifiee { "valid" } else { "unknown_key" },
        },
    })
}

fn message_du_plan() -> String {
    format!("staging {}", plan().empreinte().hexadecimal())
}

/// Soumet le plan, pose `objet` comme étiquette, puis interroge.
fn soumettre_puis_lire(objet: serde_json::Value) -> (EtatApprobation, ServeurEssai) {
    let serveur = lancer();
    let p = passerelle(&serveur.adresse);
    p.soumettre(&plan()).expect("soumission");
    serveur.poser(objet);
    let etat = p.interroger(plan().empreinte()).expect("état");
    (etat, serveur)
}

fn motif(etat: EtatApprobation) -> String {
    match etat {
        EtatApprobation::Refusee { motif } => motif,
        autre => panic!("Refusee attendu, obtenu {autre:?}"),
    }
}

// ── Le format, une seule constante ───────────────────────────

#[test]
fn format_la_constante_est_ce_que_sluis_construit() {
    let empreinte = plan().empreinte().clone();
    let message = MessageEtiquette::new(Environnement::Staging, empreinte.clone());
    let attendu = FORMAT_MESSAGE_ETIQUETTE
        .replace("<environnement>", "staging")
        .replace("<empreinte>", empreinte.hexadecimal());
    assert_eq!(message.to_string(), attendu);
    assert_eq!(
        MessageEtiquette::analyser(&attendu).expect("aller-retour"),
        message
    );
}

#[test]
fn format_l_analyse_refuse_tout_ecart() {
    let hex = plan().empreinte().hexadecimal().to_string();
    let ecarts = [
        String::new(),
        "staging".to_string(),
        hex.clone(),
        format!("staging  {hex}"),
        format!(" staging {hex}"),
        format!("staging {hex} "),
        format!("staging\t{hex}"),
        format!("staging {hex}\n"),
        format!("Staging {hex}"),
        format!("prod {hex}"),
        format!("staging {}", hex.to_uppercase()),
        format!("staging {}", &hex[..12]),
        format!("staging {hex} production"),
    ];
    for ecart in ecarts {
        assert!(
            MessageEtiquette::analyser(&ecart).is_err(),
            "« {ecart:?} » aurait dû être refusé"
        );
    }
}

/// Le tireur (story 13.2, #25) analyse ce que Sluis construit. Comme pour le
/// `run-name` du gabarit Tier 1, son gabarit doit citer le même format : si
/// l'un dérive, le test casse ici plutôt que chez la cible.
#[test]
fn format_le_gabarit_du_tireur_cite_le_meme_format() {
    let chemin = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tools/tireur-etiquette-signee.sh.example");
    let Ok(gabarit) = std::fs::read_to_string(&chemin) else {
        // Story 13.2 pas encore fusionnée : la constante reste vérifiée par
        // les deux tests précédents.
        assert_eq!(FORMAT_MESSAGE_ETIQUETTE, "<environnement> <empreinte>");
        return;
    };
    assert!(
        gabarit.contains(FORMAT_MESSAGE_ETIQUETTE),
        "le gabarit du tireur doit citer le format « {FORMAT_MESSAGE_ETIQUETTE} »"
    );
    assert!(
        gabarit.contains("[0-9a-f]{64}"),
        "le gabarit du tireur doit exiger 64 chiffres hexadécimaux minuscules"
    );
}

// ── @happy ───────────────────────────────────────────────────

#[test]
fn happy_soumettre_rend_la_commande_exacte_sans_rien_declencher() {
    let serveur = lancer();
    let etat = passerelle(&serveur.adresse)
        .soumettre(&plan())
        .expect("soumission");
    let court = plan().empreinte().abregee().to_string();
    let attendue = format!(
        "git tag -s approuve/{court} {COMMIT} -m \"staging {}\"",
        plan().empreinte().hexadecimal()
    );
    match etat {
        EtatApprobation::EnAttente { run, commande, .. } => {
            assert_eq!(run, format!("approuve/{court}"));
            assert_eq!(commande.as_deref(), Some(attendue.as_str()));
        }
        autre => panic!("EnAttente attendu, obtenu {autre:?}"),
    }
    assert!(
        !serveur.recues().iter().any(|r| r.contains("/actions/")),
        "aucun workflow ne doit être déclenché ni lu"
    );
}

struct GatesVertes;

impl VerificateurGates for GatesVertes {
    fn etat_plancher(&self, _reference: &str) -> Result<Vec<EtatGate>, AppError> {
        Ok([
            GatePlancher::Secrets,
            GatePlancher::Sbom,
            GatePlancher::ScanConteneur,
        ]
        .into_iter()
        .map(|gate| EtatGate {
            gate,
            verte: true,
            detail: String::new(),
        })
        .collect())
    }
}

#[test]
fn happy_sluis_mise_en_ligne_rend_la_commande_dans_sa_sortie() {
    let serveur = lancer();
    let outil = OutilMiseEnLigne::new(
        MiseEnLigne::new(
            Arc::new(GatesVertes),
            Arc::new(passerelle(&serveur.adresse)),
            barriere_admise(),
        ),
        "cible".to_string(),
    );
    let rendu = outil
        .appeler(&serde_json::json!({ "environnement": "staging", "reference": "v0.1.0" }))
        .expect("appel");
    assert_eq!(rendu["issue"], "soumise");
    assert_eq!(rendu["approbation"]["etat"], "en_attente");
    let empreinte = rendu["empreinte"].as_str().expect("empreinte");
    assert_eq!(
        rendu["approbation"]["commande"],
        format!(
            "git tag -s approuve/{} {COMMIT} -m \"staging {empreinte}\"",
            &empreinte[..12]
        )
    );
}

#[test]
fn happy_la_commande_vient_de_la_fonction_du_domaine() {
    let message = MessageEtiquette::new(Environnement::Staging, plan().empreinte().clone());
    assert_eq!(
        commande_signature(&message, COMMIT),
        format!(
            "git tag -s {} {COMMIT} -m \"{}\"",
            nom_etiquette_approbation(plan().empreinte()),
            message_du_plan()
        )
    );
}

// ── @negative ────────────────────────────────────────────────

#[test]
fn negative_une_etiquette_sur_un_autre_commit_est_refusee() {
    let (etat, _) = soumettre_puis_lire(etiquette(&message_du_plan(), AUTRE_COMMIT, true));
    let motif = motif(etat);
    assert!(motif.contains("étiquette sur un autre commit"), "{motif}");
}

#[test]
fn negative_une_etiquette_pour_un_autre_environnement_est_refusee() {
    let message = format!("production {}", plan().empreinte().hexadecimal());
    let (etat, _) = soumettre_puis_lire(etiquette(&message, COMMIT, true));
    let motif = motif(etat);
    assert!(motif.contains("autre environnement"), "{motif}");
    assert!(motif.contains("production"), "{motif}");
}

#[test]
fn negative_un_message_mal_forme_est_refuse_avec_un_motif_distinct() {
    let message = format!("staging  {}", plan().empreinte().hexadecimal());
    let (etat, _) = soumettre_puis_lire(etiquette(&message, COMMIT, true));
    let motif = motif(etat);
    assert!(motif.contains("mal formé"), "{motif}");
    assert!(!motif.contains("autre environnement"), "{motif}");
}

#[test]
fn negative_un_message_pour_une_autre_empreinte_est_refuse() {
    let message = format!(
        "staging {}",
        Empreinte::calculer("autre plan").hexadecimal()
    );
    let (etat, _) = soumettre_puis_lire(etiquette(&message, COMMIT, true));
    assert!(motif(etat).contains("autre empreinte"));
}

// ── @edge ────────────────────────────────────────────────────

#[test]
fn edge_en_attente_puis_approuvee_par_l_identite_signee_jamais_le_jeton() {
    let serveur = lancer();
    let p = passerelle(&serveur.adresse);
    p.soumettre(&plan()).expect("soumission");

    assert!(matches!(
        p.interroger(plan().empreinte()).expect("avant"),
        EtatApprobation::EnAttente {
            commande: Some(_),
            ..
        }
    ));

    serveur.poser(etiquette(&message_du_plan(), COMMIT, true));
    match p.interroger(plan().empreinte()).expect("après") {
        EtatApprobation::Approuvee { approbateur, run } => {
            assert_eq!(approbateur, SIGNATAIRE);
            assert_ne!(approbateur, TITULAIRE_JETON);
            assert_eq!(run, nom_etiquette_approbation(plan().empreinte()));
        }
        autre => panic!("Approuvee attendu, obtenu {autre:?}"),
    }
    assert!(
        !serveur.recues().iter().any(|r| r.ends_with(" /user")),
        "l'identité du jeton ne doit jamais être lue"
    );
}

#[test]
fn edge_un_plan_inconnu_du_processus_est_introuvable_pas_approuve() {
    let serveur = lancer();
    serveur.poser(etiquette(&message_du_plan(), COMMIT, true));
    assert!(matches!(
        passerelle(&serveur.adresse).interroger(plan().empreinte()),
        Err(AppError::Introuvable { .. })
    ));
}

// ── @security ────────────────────────────────────────────────

#[test]
fn security_la_passerelle_ne_fait_que_lire() {
    let (_, serveur) = soumettre_puis_lire(etiquette(&message_du_plan(), COMMIT, true));
    let recues = serveur.recues();
    assert!(!recues.is_empty());
    for requete in &recues {
        assert!(
            requete.starts_with("GET "),
            "requête qui écrit : {requete} (aucune étiquette ne se pousse d'ici)"
        );
    }
}

#[test]
fn security_une_signature_non_verifiee_n_approuve_jamais() {
    let (etat, _) = soumettre_puis_lire(etiquette(&message_du_plan(), COMMIT, false));
    let motif = motif(etat);
    assert!(motif.contains("non vérifiée"), "{motif}");
}

#[test]
fn security_une_etiquette_legere_n_approuve_jamais() {
    let mut objet = etiquette(&message_du_plan(), COMMIT, true);
    objet["leger"] = serde_json::json!(true);
    let (etat, _) = soumettre_puis_lire(objet);
    assert!(motif(etat).contains("non annotée"));
}

#[test]
fn security_le_renouvellement_n_emprunte_pas_cette_passerelle() {
    let serveur = lancer();
    let renouvellement =
        sluis::domain::plan_renouvellement(sluis::domain::Horodatage::new(1_790_000_000))
            .expect("plan");
    assert!(passerelle(&serveur.adresse)
        .soumettre(&renouvellement)
        .is_err());
    assert!(serveur.recues().is_empty());
}

#[test]
fn security_le_debug_ne_montre_pas_le_jeton() {
    let vue = format!("{:?}", passerelle("http://127.0.0.1:9"));
    assert!(vue.contains("gilmry/cible"), "{vue}");
    assert!(!vue.contains("jeton-lecture-essai"), "{vue}");
}

/// Barrière vide admise par une exception lointaine : ces tests portent sur
/// autre chose que la barrière (voir `tests/mise_en_ligne_barriere.rs`).
fn barriere_admise() -> sluis::application::mise_en_ligne::Barriere {
    sluis::application::mise_en_ligne::Barriere {
        evaluateur: std::sync::Arc::new(
            sluis::infrastructure::barriere_systeme::EvaluateurBarriereSysteme,
        ),
        controles: Vec::new(),
        exception_vide: Some(
            sluis::domain::ExceptionBarriereVide::depuis_date("2999-12-31").expect("date"),
        ),
        horloge: std::sync::Arc::new(sluis::application::ports::HorlogeFigee::a(
            sluis::domain::Horodatage::new(0),
        )),
    }
}
