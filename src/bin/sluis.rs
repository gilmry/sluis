//! `sluis` — interface en ligne de commande.
//!
//! Le pendant humain du serveur MCP : les mêmes cas d'usage, rendus lisibles à
//! l'écran. Aucune logique métier n'est réécrite ici, seulement du formatage.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use sluis::application::ports::{DepotInventaire, Diagnostic};
use sluis::configuration::Configuration;
use sluis::domain::EtatBinaire;
use sluis::infrastructure::diagnostic::DiagnosticSysteme;
use sluis::infrastructure::fs_inventaire::FsInventaire;

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let commande = arguments.first().map(String::as_str).unwrap_or("aide");

    let resultat = match commande {
        "doctor" => doctor(),
        "inventory" => inventaire(arguments.get(1).map(String::as_str)),
        "profiles" => profils(arguments.get(1).map(String::as_str)),
        "aide" | "--help" | "-h" => {
            aide();
            Ok(())
        }
        autre => {
            eprintln!("commande inconnue : {autre}");
            aide();
            return ExitCode::FAILURE;
        }
    };

    match resultat {
        Ok(()) => ExitCode::SUCCESS,
        Err(erreur) => {
            eprintln!("sluis : {erreur}");
            ExitCode::FAILURE
        }
    }
}

fn aide() {
    println!(
        "sluis {} — l'écluse\n\n\
         Commandes :\n  \
           doctor              état des moteurs et des identifiants\n  \
           inventory <racine>  matrice topologies × environnements\n  \
           profiles <racine>   profils de cluster\n\n\
         Configuration : SLUIS_CONFIG (défaut : sluis.toml)",
        env!("CARGO_PKG_VERSION")
    );
}

fn doctor() -> Result<(), sluis::domain::AppError> {
    let rapport = DiagnosticSysteme::depuis_environnement().etablir()?;
    println!("sluis {} — diagnostic\n", rapport.version);

    println!("Moteurs :");
    for moteur in &rapport.moteurs {
        let (marque, detail) = match &moteur.etat {
            EtatBinaire::Disponible { chemin } => ("✓", chemin.clone()),
            EtatBinaire::NonExecutable { chemin } => {
                ("!", format!("{chemin} (présent, non exécutable)"))
            }
            EtatBinaire::Absent => ("✗", "absent".to_string()),
        };
        println!(
            "  {marque} {:<18} {:<38} {}",
            moteur.nom, detail, moteur.role
        );
    }

    println!("\nIdentifiants OVH :");
    for identifiant in &rapport.identifiants {
        println!(
            "  {} {}",
            if identifiant.present { "✓" } else { "✗" },
            identifiant.variable
        );
    }

    let indisponibles = rapport.moteurs_indisponibles().len();
    if indisponibles > 0 {
        println!(
            "\n{indisponibles} moteur(s) indisponible(s). Ce n'est pas une panne : les outils \
             qui en dépendent rendront une erreur nommant le binaire, les autres fonctionnent."
        );
    }
    Ok(())
}

fn racine_ou_defaut(argument: Option<&str>) -> Result<PathBuf, sluis::domain::AppError> {
    match argument {
        Some(chemin) => Ok(PathBuf::from(chemin)),
        None => Err(sluis::domain::AppError::Configuration {
            detail: "chemin de la racine d'infrastructure attendu".to_string(),
        }),
    }
}

fn inventaire(argument: Option<&str>) -> Result<(), sluis::domain::AppError> {
    let racine = racine_ou_defaut(argument)?;
    let matrice = FsInventaire::new().decouvrir_matrice(&racine.display().to_string())?;

    println!("Topologies    : {}", matrice.topologies.len());
    for topologie in &matrice.topologies {
        let environnements: Vec<String> = matrice
            .cellules
            .iter()
            .filter(|c| c.topologie == *topologie)
            .map(|c| c.environnement.to_string())
            .collect();
        println!("  {topologie:<5} → {}", environnements.join(", "));
    }
    println!("Environnements: {}", matrice.environnements.len());
    println!("Profils       : {}", matrice.profils.len());
    println!("Modules        : {}", matrice.modules.len());
    if !matrice.ignores.is_empty() {
        println!(
            "\nNon reconnus (signalés, pas tus) : {}",
            matrice.ignores.join(", ")
        );
    }
    Ok(())
}

fn profils(argument: Option<&str>) -> Result<(), sluis::domain::AppError> {
    let racine = racine_ou_defaut(argument)?;
    let profils = FsInventaire::new().lire_profils(&racine.display().to_string())?;
    let _ = Configuration::charger(Some(Path::new("sluis.toml")));
    for profil in &profils {
        println!(
            "{:<18} stockage={:<26} ingress={:<8} secrets={}",
            profil.nom(),
            profil.classe_stockage().unwrap_or("-"),
            profil.classe_ingress().unwrap_or("-"),
            profil.backend_secrets().unwrap_or("-")
        );
    }
    Ok(())
}
