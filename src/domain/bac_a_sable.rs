//! Bounded context Bac à sable — baux éphémères et fenêtre de dérogation.
//!
//! Ce module porte la dérogation d'ADR-007, et il la porte **par le typage**.
//!
//! Les six premières conditions sont vérifiées à la construction d'un bail. La
//! septième, la fenêtre de validité, est portée différemment : [`BailBacASable::louer`]
//! exige une [`DerogationValide`], et ce type ne s'obtient qu'en confrontant
//! une [`FenetreDerogation`] à une horloge. **Hors fenêtre, louer un bail n'est
//! pas refusé : c'est inexprimable.**
//!
//! Et la fenêtre elle-même ne s'ouvre qu'avec un [`JetonConsomme`], c'est-à-dire
//! la preuve qu'une approbation de Tier 1 a eu lieu. L'autorité de déléguer
//! n'est donc jamais elle-même déléguée.

use serde::Serialize;

use crate::domain::{AppError, Duree, Horodatage, JetonConsomme, ProjetBacASable};

/// Plafond de dépense d'une campagne.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize)]
pub struct PlafondDepense(f64);

impl PlafondDepense {
    /// Construit un plafond strictement positif.
    ///
    /// Un plafond nul ou négatif n'est pas « pas de limite » : c'est une erreur
    /// de configuration qui autoriserait une dépense illimitée sous couvert
    /// d'une valeur qui a l'air d'être une limite.
    pub fn new(montant: f64) -> Result<Self, AppError> {
        if !montant.is_finite() || montant <= 0.0 {
            return Err(AppError::Configuration {
                detail: format!("plafond de dépense invalide : {montant}"),
            });
        }
        Ok(Self(montant))
    }

    /// Montant du plafond.
    pub fn montant(&self) -> f64 {
        self.0
    }
}

/// La fenêtre pendant laquelle la dérogation de Tier 2 produit ses effets.
///
/// **Expirée par défaut.** Il n'existe aucun constructeur qui ne parte pas d'un
/// [`JetonConsomme`], donc aucune façon d'ouvrir une fenêtre sans qu'une
/// approbation de Tier 1 ait eu lieu et soit tracée.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FenetreDerogation {
    ouverte_le: Horodatage,
    close_le: Horodatage,
    approbateur: String,
}

/// Preuve qu'une dérogation est valide **à un instant donné**.
///
/// Ne peut pas être construite directement : elle sort de
/// [`FenetreDerogation::valider`]. Sa durée de vie est liée à celle de la
/// fenêtre, ce qui empêche de la conserver au-delà de son objet.
#[derive(Debug)]
pub struct DerogationValide<'a> {
    fenetre: &'a FenetreDerogation,
    #[allow(dead_code)]
    constatee_a: Horodatage,
}

impl<'a> DerogationValide<'a> {
    /// Instant de fermeture de la fenêtre qui porte cette dérogation.
    pub fn close_le(&self) -> Horodatage {
        self.fenetre.close_le
    }
}

impl FenetreDerogation {
    /// Ouvre une fenêtre, **sur preuve d'approbation de Tier 1**.
    ///
    /// Le jeton consommé est la preuve. Sa présence dans la signature est ce
    /// qui rend impossible d'ouvrir une fenêtre par un chemin de Tier 2, y
    /// compris depuis un outil MCP.
    pub fn ouvrir(
        approbation: &JetonConsomme,
        ouverte_le: Horodatage,
        duree: Duree,
    ) -> Result<Self, AppError> {
        if approbation.approbateur().trim().is_empty() {
            return Err(AppError::Configuration {
                detail: "approbation sans approbateur".to_string(),
            });
        }
        Ok(Self {
            ouverte_le,
            close_le: ouverte_le.plus(duree),
            approbateur: approbation.approbateur().to_string(),
        })
    }

    /// Confronte la fenêtre à l'horloge.
    ///
    /// **Fail-closed** : hors fenêtre, aucune preuve n'est rendue, donc aucun
    /// bail ne peut être loué. Il n'existe pas de troisième issue, et
    /// l'indétermination est traitée en amont, par l'absence de fenêtre.
    pub fn valider(&self, maintenant: Horodatage) -> Result<DerogationValide<'_>, AppError> {
        if maintenant < self.ouverte_le {
            return Err(AppError::TierViolation {
                raison: format!(
                    "la fenêtre de dérogation n'ouvre qu'à {} (nous sommes à {maintenant})",
                    self.ouverte_le
                ),
            });
        }
        if maintenant.apres(self.close_le) {
            return Err(AppError::TierViolation {
                raison: format!(
                    "la fenêtre de dérogation a expiré à {} : un renouvellement de Tier 1 \
                     est requis avant de pouvoir louer un bail",
                    self.close_le
                ),
            });
        }
        Ok(DerogationValide {
            fenetre: self,
            constatee_a: maintenant,
        })
    }

    /// Instant d'ouverture.
    pub fn ouverte_le(&self) -> Horodatage {
        self.ouverte_le
    }
    /// Instant de fermeture.
    pub fn close_le(&self) -> Horodatage {
        self.close_le
    }
    /// Qui a approuvé l'ouverture.
    pub fn approbateur(&self) -> &str {
        &self.approbateur
    }
}

/// Un bail d'infrastructure éphémère.
///
/// Toujours borné : un TTL et un plafond, tous deux non optionnels. Il n'existe
/// ni `Default`, ni constructeur partiel.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BailBacASable {
    projet: ProjetBacASable,
    ttl: Duree,
    plafond: PlafondDepense,
    ouvert_le: Horodatage,
    expire_le: Horodatage,
}

impl BailBacASable {
    /// Loue une infrastructure éphémère.
    ///
    /// Vérifie les conditions d'ADR-007 dans un ordre choisi : la fenêtre
    /// d'abord — via le type même de `derogation` —, puis le TTL, puis le
    /// plafond, puis le débordement de fenêtre.
    ///
    /// Le dernier point mérite d'être souligné : une campagne dont la durée
    /// projetée dépasserait la fermeture de la fenêtre est refusée **à
    /// l'admission**. Une campagne n'est jamais interrompue à mi-parcours par
    /// une expiration, ce qui produirait une infrastructure orpheline et des
    /// mesures inexploitables.
    pub fn louer(
        derogation: &DerogationValide<'_>,
        projet: ProjetBacASable,
        ttl: Duree,
        plafond: PlafondDepense,
        estimation_depense: f64,
        ttl_maximal: Duree,
        maintenant: Horodatage,
    ) -> Result<Self, AppError> {
        if ttl > ttl_maximal {
            return Err(AppError::Configuration {
                detail: format!("TTL demandé {ttl} au-delà du maximum configuré {ttl_maximal}"),
            });
        }
        if estimation_depense > plafond.montant() {
            return Err(AppError::Configuration {
                detail: format!(
                    "dépense projetée {estimation_depense} au-dessus du plafond {} : \
                     refusée à l'admission plutôt que découverte sur la facture",
                    plafond.montant()
                ),
            });
        }
        let expire_le = maintenant.plus(ttl);
        if expire_le.apres(derogation.close_le()) {
            return Err(AppError::TierViolation {
                raison: format!(
                    "le bail expirerait à {expire_le}, après la fermeture de la fenêtre de \
                     dérogation à {} : refusé à l'admission pour qu'aucune campagne ne soit \
                     interrompue en cours de route",
                    derogation.close_le()
                ),
            });
        }
        Ok(Self {
            projet,
            ttl,
            plafond,
            ouvert_le: maintenant,
            expire_le,
        })
    }

    /// Vrai si le bail a dépassé son échéance.
    pub fn expire(&self, maintenant: Horodatage) -> bool {
        maintenant.apres(self.expire_le)
    }

    /// Projet loué.
    pub fn projet(&self) -> &ProjetBacASable {
        &self.projet
    }
    /// Échéance.
    pub fn expire_le(&self) -> Horodatage {
        self.expire_le
    }
    /// Instant d'ouverture.
    pub fn ouvert_le(&self) -> Horodatage {
        self.ouvert_le
    }
    /// Durée de vie.
    pub fn ttl(&self) -> Duree {
        self.ttl
    }
    /// Plafond de dépense.
    pub fn plafond(&self) -> PlafondDepense {
        self.plafond
    }
}

/// Une cible provisionnée, prête à recevoir de la charge.
///
/// Ne peut pas exister sans adresse : une campagne lancée contre une adresse
/// vide mesurerait le vide, et rendrait des chiffres qu'on croirait valides.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CibleEphemere {
    adresse: String,
    sorties: Vec<(String, String)>,
}

impl CibleEphemere {
    /// Construit la cible à partir des sorties du module.
    pub fn new(
        adresse: impl Into<String>,
        sorties: Vec<(String, String)>,
    ) -> Result<Self, AppError> {
        let adresse = adresse.into();
        if adresse.trim().is_empty() {
            return Err(AppError::Configuration {
                detail: "adresse de cible vide : une campagne sans cible mesurerait le vide"
                    .to_string(),
            });
        }
        Ok(Self { adresse, sorties })
    }

    /// Adresse de la cible.
    pub fn adresse(&self) -> &str {
        &self.adresse
    }

    /// Toutes les sorties du module, conservées pour le compte rendu.
    pub fn sorties(&self) -> &[(String, String)] {
        &self.sorties
    }
}
