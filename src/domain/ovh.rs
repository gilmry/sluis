//! Bounded context OVH — ce qui tourne réellement.
//!
//! Le type central n'est pas une ressource, c'est la [`ListeAutorisation`].
//! Elle porte l'invariant 3 du Brief, et sa forme dit une chose importante :
//! **la liste des projets de bac à sable et celle des projets de production
//! sont disjointes par construction**, pas par convention. Un identifiant ne
//! peut pas figurer dans les deux.

use std::collections::BTreeSet;
use std::fmt;

use serde::Serialize;

use crate::domain::AppError;

/// Identifiant d'un projet OVH portant de la production.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct ProjetProduction(String);

/// Identifiant d'un projet OVH dédié aux bacs à sable.
///
/// **Type distinct de [`ProjetProduction`] à dessein.** Une fonction qui loue
/// un bail n'accepte que ce type : passer un projet de production ne produit
/// pas une erreur à l'exécution, ça ne compile pas. C'est la deuxième garantie
/// structurelle d'ADR-007, après la fenêtre de validité.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct ProjetBacASable(String);

impl ProjetProduction {
    /// Identifiant brut.
    pub fn identifiant(&self) -> &str {
        &self.0
    }
}

impl ProjetBacASable {
    /// Identifiant brut.
    pub fn identifiant(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProjetProduction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for ProjetBacASable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// La liste des projets OVH que Sluis a le droit de voir.
///
/// Un projet absent de cette liste n'existe pas pour Sluis : il n'est ni
/// listé, ni interrogeable, **et le refus a lieu avant tout appel réseau**.
/// C'est ce qui rend l'erreur exploitable pour un opérateur (« ce projet n'est
/// pas autorisé ») plutôt qu'ambiguë (« projet introuvable »).
#[derive(Debug, Clone, Default)]
pub struct ListeAutorisation {
    production: BTreeSet<String>,
    bac_a_sable: BTreeSet<String>,
}

impl ListeAutorisation {
    /// Construit la liste.
    ///
    /// Refuse tout identifiant présent des deux côtés. C'est la disjonction
    /// exigée par la première condition d'ADR-007, vérifiée à la construction
    /// plutôt qu'à chaque usage : elle ne peut donc pas être oubliée.
    pub fn new(production: Vec<String>, bac_a_sable: Vec<String>) -> Result<Self, AppError> {
        let production: BTreeSet<String> = production
            .into_iter()
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .collect();
        let bac_a_sable: BTreeSet<String> = bac_a_sable
            .into_iter()
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .collect();

        let intersection: Vec<&String> = production.intersection(&bac_a_sable).collect();
        if !intersection.is_empty() {
            return Err(AppError::Configuration {
                detail: format!(
                    "projet(s) déclaré(s) à la fois en production et en bac à sable : {} — \
                     la première condition d'ADR-007 exige que les deux listes soient disjointes",
                    intersection
                        .iter()
                        .map(|p| p.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            });
        }
        Ok(Self {
            production,
            bac_a_sable,
        })
    }

    /// Résout un identifiant en projet de production autorisé.
    pub fn projet_production(&self, identifiant: &str) -> Result<ProjetProduction, AppError> {
        let identifiant = identifiant.trim();
        if self.production.contains(identifiant) {
            Ok(ProjetProduction(identifiant.to_string()))
        } else {
            Err(AppError::ProjetNonAutorise {
                projet: identifiant.to_string(),
            })
        }
    }

    /// Résout un identifiant en projet de bac à sable autorisé.
    pub fn projet_bac_a_sable(&self, identifiant: &str) -> Result<ProjetBacASable, AppError> {
        let identifiant = identifiant.trim();
        if self.bac_a_sable.contains(identifiant) {
            Ok(ProjetBacASable(identifiant.to_string()))
        } else {
            Err(AppError::ProjetNonAutorise {
                projet: identifiant.to_string(),
            })
        }
    }

    /// Vrai si l'identifiant est visible, quel que soit son usage.
    pub fn est_visible(&self, identifiant: &str) -> bool {
        let identifiant = identifiant.trim();
        self.production.contains(identifiant) || self.bac_a_sable.contains(identifiant)
    }

    /// Tous les identifiants visibles, triés.
    pub fn tous(&self) -> Vec<String> {
        let mut tous: Vec<String> = self
            .production
            .iter()
            .chain(self.bac_a_sable.iter())
            .cloned()
            .collect();
        tous.sort();
        tous
    }

    /// Identifiants de bacs à sable.
    pub fn bacs_a_sable(&self) -> Vec<String> {
        self.bac_a_sable.iter().cloned().collect()
    }
}

/// Un projet OVH tel que l'API le décrit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjetOvh {
    /// Identifiant du projet.
    pub identifiant: String,
    /// Nom lisible.
    pub nom: String,
    /// Statut rapporté par OVH.
    pub statut: String,
}

/// Une instance de calcul.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstanceOvh {
    /// Identifiant de l'instance.
    pub identifiant: String,
    /// Nom donné à l'instance.
    pub nom: String,
    /// Gabarit (flavor).
    pub gabarit: String,
    /// Région d'hébergement.
    pub region: String,
    /// État rapporté par OVH.
    pub etat: String,
}

/// La consommation courante d'un projet.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CoutCourant {
    /// Projet concerné.
    pub projet: String,
    /// Montant courant.
    pub montant: f64,
    /// Devise.
    pub devise: String,
    /// Début de la période couverte.
    pub debut: String,
    /// Fin de la période couverte.
    pub fin: String,
}

/// Un enregistrement DNS.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EnregistrementDns {
    /// Identifiant de l'enregistrement.
    pub identifiant: String,
    /// Sous-domaine.
    pub sous_domaine: String,
    /// Type d'enregistrement (A, CNAME, TXT…).
    pub type_enregistrement: String,
    /// Cible.
    pub cible: String,
    /// Durée de vie.
    pub ttl: u32,
}
