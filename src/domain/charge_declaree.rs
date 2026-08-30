//! Ce qu'un projet Foyer déclare pour être mesurable sous charge.
//!
//! La déclaration vit dans le dépôt mesuré, `infrastructure/_shared/charge.yaml`,
//! là où vivent déjà les profils de cluster et les modules. Sluis la découvre
//! comme il découvre la matrice d'inventaire, sans aucune saisie.
//!
//! **Elle demande, elle n'accorde pas.** Quiconque écrit dans le dépôt mesuré
//! écrit la déclaration : si elle pouvait relever le TTL ou le plafond, elle
//! deviendrait un moyen de contourner ADR-007 par une pull request. Les bornes
//! se cumulent donc, elles ne se remplacent pas.

use serde::Serialize;

use crate::domain::{AppError, Duree, MesureCapacite, Palier, PlafondDepense};

/// Le palier auquel un verdict est rendu.
///
/// `Realistic` et non `Spike` : la question posée par l'abaque est la tenue en
/// conditions, pas la résistance à une pointe.
const PALIER_DE_REFERENCE: Palier = Palier::Realistic;

/// La cible de capacité, issue du brief.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct CibleCapacite {
    requetes_par_seconde: f64,
    p99_millisecondes: f64,
}

impl CibleCapacite {
    /// Déclare une cible.
    ///
    /// Refuse les valeurs nulles ou négatives : une cible à zéro serait
    /// toujours atteinte, donc décorative, ce qui est pire que pas de verdict.
    pub fn new(requetes_par_seconde: f64, p99_millisecondes: f64) -> Result<Self, AppError> {
        let invalide = |valeur: f64| !valeur.is_finite() || valeur <= 0.0;
        if invalide(requetes_par_seconde) || invalide(p99_millisecondes) {
            return Err(AppError::Configuration {
                detail: "cible de capacité invalide : le débit et la latence visés doivent être \
                         strictement positifs, faute de quoi le verdict est toujours favorable"
                    .to_string(),
            });
        }
        Ok(Self {
            requetes_par_seconde,
            p99_millisecondes,
        })
    }

    /// Débit visé, en requêtes par seconde.
    pub fn requetes_par_seconde(&self) -> f64 {
        self.requetes_par_seconde
    }

    /// Latence au 99e centile visée, en millisecondes.
    pub fn p99_millisecondes(&self) -> f64 {
        self.p99_millisecondes
    }
}

/// Le verdict d'une campagne, confronté à la cible déclarée.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Verdict {
    /// La cible est tenue au palier de référence.
    Tient {
        /// Débit constaté.
        debit: f64,
        /// Latence au 99e centile constatée.
        p99: f64,
    },
    /// La cible n'est pas tenue, avec un motif par grandeur manquée.
    NeTientPas {
        /// Ce qui manque, nommé grandeur par grandeur.
        motifs: Vec<String>,
        /// Débit constaté.
        debit: f64,
        /// Latence au 99e centile constatée.
        p99: f64,
    },
    /// Rien n'est affirmé, et la raison est dite.
    ///
    /// L'abaque distingue le mesuré du supposé : un verdict rendu sur un
    /// palier qui n'a pas tourné serait du supposé déguisé en mesuré.
    Indetermine {
        /// Pourquoi aucun verdict n'est rendu.
        motif: String,
    },
}

/// La déclaration de charge d'un projet.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DeclarationCharge {
    topologie: String,
    module: String,
    sortie_adresse: String,
    chemin: String,
    cible: CibleCapacite,
    ttl_demande: Duree,
    plafond_demande: PlafondDepense,
}

impl DeclarationCharge {
    /// Admet une déclaration.
    pub fn new(
        topologie: String,
        module: String,
        sortie_adresse: String,
        chemin: String,
        cible: CibleCapacite,
        ttl_demande: Duree,
        plafond_demande: PlafondDepense,
    ) -> Result<Self, AppError> {
        for (nom, valeur) in [
            ("topologie", &topologie),
            ("module", &module),
            ("sortie_adresse", &sortie_adresse),
            ("chemin", &chemin),
        ] {
            if valeur.trim().is_empty() {
                return Err(AppError::Configuration {
                    detail: format!(
                        "déclaration de charge incomplète : « {nom} » est vide, or Sluis ne \
                         devine pas ce qu'un projet n'a pas déclaré"
                    ),
                });
            }
        }
        Ok(Self {
            topologie,
            module,
            sortie_adresse,
            chemin,
            cible,
            ttl_demande,
            plafond_demande,
        })
    }

    /// Topologie à louer.
    pub fn topologie(&self) -> &str {
        &self.topologie
    }
    /// Module Terraform jetable, relatif à la racine d'infrastructure.
    pub fn module(&self) -> &str {
        &self.module
    }
    /// Sortie du module qui porte l'adresse.
    pub fn sortie_adresse(&self) -> &str {
        &self.sortie_adresse
    }
    /// Chemin HTTP que l'escalier frappe.
    pub fn chemin(&self) -> &str {
        &self.chemin
    }
    /// Cible de capacité déclarée.
    pub fn cible(&self) -> CibleCapacite {
        self.cible
    }

    /// TTL retenu : le plus petit du demandé et de l'autorisé.
    pub fn ttl_borne(&self, maximum_serveur: Duree) -> Duree {
        self.ttl_demande.min(maximum_serveur)
    }

    /// Plafond retenu : le plus petit du demandé et de l'autorisé.
    pub fn plafond_borne(&self, plafond_serveur: PlafondDepense) -> PlafondDepense {
        if self.plafond_demande.montant() <= plafond_serveur.montant() {
            self.plafond_demande
        } else {
            plafond_serveur
        }
    }

    /// Confronte les mesures à la cible.
    pub fn verdict(&self, mesures: &[MesureCapacite]) -> Verdict {
        let au_palier = |grandeur: &str| {
            mesures
                .iter()
                .find(|m| m.grandeur() == grandeur && m.palier() == Some(PALIER_DE_REFERENCE))
                .map(|m| m.valeur())
        };

        let (Some(debit), Some(p99)) = (au_palier("debit"), au_palier("latence_p99")) else {
            return Verdict::Indetermine {
                motif: format!(
                    "aucune mesure de débit et de latence au palier « {} » : la tenue en \
                     conditions ne se déduit pas des autres paliers",
                    PALIER_DE_REFERENCE.nom()
                ),
            };
        };

        let mut motifs = Vec::new();
        if debit < self.cible.requetes_par_seconde {
            motifs.push(format!(
                "débit mesuré {debit:.1} req/s sous la cible de {:.1}",
                self.cible.requetes_par_seconde
            ));
        }
        if p99 > self.cible.p99_millisecondes {
            motifs.push(format!(
                "latence p99 mesurée {p99:.1} ms au-dessus de la cible de {:.1}",
                self.cible.p99_millisecondes
            ));
        }

        if motifs.is_empty() {
            Verdict::Tient { debit, p99 }
        } else {
            Verdict::NeTientPas { motifs, debit, p99 }
        }
    }
}
