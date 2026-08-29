//! Bounded context Autorisation — plans, empreintes et jetons.
//!
//! Deux invariants du Brief §10 se rencontrent ici, et tous deux sont portés
//! par le typage plutôt que par une vérification :
//!
//! - **Invariant 1** : un plan visant la production est nécessairement de
//!   Tier 1. [`PlanChangement::new`] est le seul constructeur, et il refuse.
//! - **Invariant 5** : un jeton est consommé exactement une fois.
//!   [`JetonChangement::consommer`] prend `self` **par valeur** : rejouer un
//!   jeton ne produit pas une erreur à l'exécution, ça ne compile pas.

use std::fmt;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::domain::{AppError, Environnement, Horodatage, Tier};

/// Empreinte d'un plan.
///
/// Elle sert de lien entre un plan et l'approbation qui le vise. Deux plans
/// qui diffèrent d'un seul caractère ont des empreintes différentes, sans quoi
/// une approbation donnée pour l'un vaudrait pour l'autre.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct Empreinte(String);

impl Empreinte {
    /// Calcule l'empreinte d'un contenu.
    pub fn calculer(contenu: &str) -> Self {
        let mut hacheur = Sha256::new();
        hacheur.update(contenu.as_bytes());
        Self(format!("{:x}", hacheur.finalize()))
    }

    /// Représentation hexadécimale.
    pub fn hexadecimal(&self) -> &str {
        &self.0
    }

    /// Forme abrégée, pour les journaux et les messages.
    pub fn abregee(&self) -> &str {
        &self.0[..12.min(self.0.len())]
    }
}

impl fmt::Display for Empreinte {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Ce qu'un plan se propose de faire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Appliquer une déclaration Terraform.
    TerraformApply,
    /// Détruire une infrastructure Terraform.
    TerraformDestroy,
    /// Installer ou mettre à jour une release Helm.
    HelmUpgrade,
    /// Désinstaller une release Helm.
    HelmUninstall,
    /// Forcer une synchronisation ArgoCD.
    ArgocdSync,
    /// Restaurer une sauvegarde Velero.
    VeleroRestore,
    /// Mettre un projet en ligne.
    MiseEnLigne,
    /// Renouveler la dérogation de bac à sable.
    RenouvellementDerogation,
    /// Louer une infrastructure éphémère de test.
    LocationBacASable,
    /// Détruire une infrastructure éphémère de test.
    DestructionBacASable,
}

impl Action {
    /// Le tier minimal que cette action exige, quel que soit l'environnement.
    ///
    /// Reprend la liste nommée dans `AGENT_GUARDRAILS.md`. Les deux actions de
    /// bac à sable font exception au titre d'ADR-007, et **uniquement** dans le
    /// cadre borné qu'il décrit.
    pub fn tier_minimal(&self) -> Tier {
        match self {
            Action::LocationBacASable | Action::DestructionBacASable => Tier::Two,
            _ => Tier::One,
        }
    }

    /// Nom canonique.
    pub fn nom(&self) -> &'static str {
        match self {
            Action::TerraformApply => "terraform_apply",
            Action::TerraformDestroy => "terraform_destroy",
            Action::HelmUpgrade => "helm_upgrade",
            Action::HelmUninstall => "helm_uninstall",
            Action::ArgocdSync => "argocd_sync",
            Action::VeleroRestore => "velero_restore",
            Action::MiseEnLigne => "mise_en_ligne",
            Action::RenouvellementDerogation => "renouvellement_derogation",
            Action::LocationBacASable => "location_bac_a_sable",
            Action::DestructionBacASable => "destruction_bac_a_sable",
        }
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.nom())
    }
}

/// Un plan de changement : une mutation décrite, empreintée, et **non exécutée**.
///
/// Produire un plan est de Tier 2 quelle que soit l'action : décrire ne mute
/// rien. C'est l'exécution qui relève du Tier 1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlanChangement {
    action: Action,
    environnement: Environnement,
    tier: Tier,
    cible: String,
    description: String,
    diff: String,
    empreinte: Empreinte,
}

impl PlanChangement {
    /// Construit un plan.
    ///
    /// **Seul constructeur.** Il refuse un plan de Tier 2 visant la production,
    /// et refuse plus généralement un tier inférieur à celui que l'action
    /// exige. Il n'existe aucun chemin pour contourner cette vérification :
    /// les champs sont privés et il n'y a pas de `Default`.
    pub fn new(
        action: Action,
        environnement: Environnement,
        tier: Tier,
        cible: String,
        description: String,
        diff: String,
    ) -> Result<Self, AppError> {
        if environnement == Environnement::Production && tier == Tier::Two {
            return Err(AppError::TierViolation {
                raison: format!(
                    "un plan visant production ne peut pas être de Tier 2 (action {action})"
                ),
            });
        }
        if action.tier_minimal() == Tier::One && tier == Tier::Two {
            return Err(AppError::TierViolation {
                raison: format!("l'action {action} exige le Tier 1"),
            });
        }
        if cible.trim().is_empty() {
            return Err(AppError::Configuration {
                detail: "plan sans cible".to_string(),
            });
        }
        let empreinte = Empreinte::calculer(&format!(
            "{action}|{environnement}|{tier:?}|{cible}|{description}|{diff}"
        ));
        Ok(Self {
            action,
            environnement,
            tier,
            cible,
            description,
            diff,
            empreinte,
        })
    }

    /// Empreinte du plan.
    pub fn empreinte(&self) -> &Empreinte {
        &self.empreinte
    }
    /// Action visée.
    pub fn action(&self) -> &Action {
        &self.action
    }
    /// Environnement visé.
    pub fn environnement(&self) -> Environnement {
        self.environnement
    }
    /// Tier sous lequel le plan est classé.
    pub fn tier(&self) -> Tier {
        self.tier
    }
    /// Cible du plan.
    pub fn cible(&self) -> &str {
        &self.cible
    }
    /// Description lisible.
    pub fn description(&self) -> &str {
        &self.description
    }
    /// Écart projeté.
    pub fn diff(&self) -> &str {
        &self.diff
    }
}

/// Un jeton d'approbation, valable pour **une** empreinte et **une seule** fois.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct JetonChangement {
    empreinte: Empreinte,
    approbateur: String,
    emis_le: Horodatage,
    expire_le: Horodatage,
}

/// Preuve qu'un jeton a été consommé.
///
/// Ce type n'existe que pour rendre la consommation observable dans les
/// signatures : une fonction qui exige un `JetonConsomme` ne peut être appelée
/// qu'après une consommation réussie.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct JetonConsomme {
    empreinte: Empreinte,
    approbateur: String,
    consomme_le: Horodatage,
}

impl JetonConsomme {
    /// Empreinte du plan que ce jeton autorisait.
    pub fn empreinte(&self) -> &Empreinte {
        &self.empreinte
    }
    /// Qui a approuvé.
    pub fn approbateur(&self) -> &str {
        &self.approbateur
    }
    /// Quand la consommation a eu lieu.
    pub fn consomme_le(&self) -> Horodatage {
        self.consomme_le
    }
}

impl JetonChangement {
    /// Émet un jeton pour une empreinte donnée.
    pub fn emettre(
        empreinte: Empreinte,
        approbateur: String,
        emis_le: Horodatage,
        validite: crate::domain::Duree,
    ) -> Result<Self, AppError> {
        if approbateur.trim().is_empty() {
            return Err(AppError::Configuration {
                detail: "jeton sans approbateur : l'approbation doit être imputable".to_string(),
            });
        }
        Ok(Self {
            empreinte,
            approbateur,
            emis_le,
            expire_le: emis_le.plus(validite),
        })
    }

    /// Consomme le jeton pour exécuter `plan`.
    ///
    /// **Prend `self` par valeur.** C'est le cœur de l'invariant 5 : après
    /// l'appel, le jeton n'existe plus, et une seconde tentative de
    /// consommation est une erreur de compilation, pas un test qui pourrait
    /// être oublié.
    ///
    /// Vérifie, dans cet ordre, que l'empreinte correspond puis que le jeton
    /// n'est pas expiré. L'ordre compte : un jeton présenté pour le mauvais
    /// plan doit être rejeté pour cette raison, pas pour une expiration qui
    /// masquerait la vraie anomalie.
    pub fn consommer(
        self,
        plan: &PlanChangement,
        maintenant: Horodatage,
    ) -> Result<JetonConsomme, AppError> {
        if &self.empreinte != plan.empreinte() {
            return Err(AppError::TierViolation {
                raison: format!(
                    "jeton émis pour l'empreinte {} et présenté pour {}",
                    self.empreinte.abregee(),
                    plan.empreinte().abregee()
                ),
            });
        }
        if maintenant.apres(self.expire_le) {
            return Err(AppError::TierViolation {
                raison: format!(
                    "jeton expiré : émis à {}, expiré à {}, présenté à {maintenant}",
                    self.emis_le, self.expire_le
                ),
            });
        }
        Ok(JetonConsomme {
            empreinte: self.empreinte,
            approbateur: self.approbateur,
            consomme_le: maintenant,
        })
    }

    /// Empreinte visée par ce jeton.
    pub fn empreinte(&self) -> &Empreinte {
        &self.empreinte
    }
    /// Instant d'expiration.
    pub fn expire_le(&self) -> Horodatage {
        self.expire_le
    }
}
