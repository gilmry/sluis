//! Approbation par étiquette git signée (ADR-013) : le format du message.
//!
//! Sur un dépôt privé, l'acte d'approbation de Tier 1 est une étiquette
//! `approuve/<empreinte courte>` signée par la clé du superviseur. Sluis
//! **construit** le message à signer ; le tireur GitOps de la cible
//! l'**analyse** et vérifie la signature. Les deux codes ne se voient pas :
//! le format vit donc ici, en un seul endroit, et un test le confronte au
//! gabarit du tireur.
//!
//! Format décidé par le superviseur le 2026-10-23 : `<environnement>
//! <empreinte>`, deux champs séparés par **une** espace, l'environnement pris
//! dans la liste connue, l'empreinte en 64 caractères hexadécimaux minuscules.
//! Tout autre message est refusé, jamais deviné.

use std::fmt;

use crate::domain::{AppError, Empreinte, Environnement};

/// Le format du message signé, tel qu'il est documenté et confronté au
/// gabarit du tireur. [`MessageEtiquette`] le construit et l'analyse.
pub const FORMAT_MESSAGE_ETIQUETTE: &str = "<environnement> <empreinte>";

/// Préfixe réservé des étiquettes d'approbation.
pub const PREFIXE_ETIQUETTE_APPROBATION: &str = "approuve/";

/// L'unique séparateur entre les deux champs.
const SEPARATEUR: char = ' ';

/// Le message d'une étiquette d'approbation : l'environnement cible et
/// l'empreinte complète du plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageEtiquette {
    environnement: Environnement,
    empreinte: Empreinte,
}

impl MessageEtiquette {
    /// Le message à signer pour un plan.
    pub fn new(environnement: Environnement, empreinte: Empreinte) -> Self {
        Self {
            environnement,
            empreinte,
        }
    }

    /// Analyse stricte d'un message.
    ///
    /// Exactement deux champs séparés par une seule espace ; l'environnement
    /// écrit exactement comme son nom canonique ; l'empreinte en 64 chiffres
    /// hexadécimaux minuscules. Aucune espace de tête ou de queue, aucun saut
    /// de ligne, aucune casse tolérée : un message qui s'écarte du format est
    /// refusé, il n'est pas interprété.
    pub fn analyser(texte: &str) -> Result<Self, AppError> {
        let refus = |detail: String| AppError::Analyse {
            quoi: "message d'étiquette".to_string(),
            detail: format!("{detail} ; format attendu « {FORMAT_MESSAGE_ETIQUETTE} »"),
        };

        let mut champs = texte.split(SEPARATEUR);
        let (Some(nom), Some(hexadecimal), None) = (champs.next(), champs.next(), champs.next())
        else {
            return Err(refus(
                "deux champs séparés par une seule espace attendus".to_string(),
            ));
        };

        let environnement = nom
            .parse::<Environnement>()
            .ok()
            .filter(|e| e.nom() == nom)
            .ok_or_else(|| refus(format!("environnement inconnu « {nom} »")))?;
        let empreinte = Empreinte::depuis_hexadecimal(hexadecimal).map_err(|_| {
            refus("empreinte : 64 chiffres hexadécimaux en minuscules attendus".to_string())
        })?;

        Ok(Self {
            environnement,
            empreinte,
        })
    }

    /// Environnement nommé par le message.
    pub fn environnement(&self) -> Environnement {
        self.environnement
    }

    /// Empreinte portée par le message.
    pub fn empreinte(&self) -> &Empreinte {
        &self.empreinte
    }
}

impl fmt::Display for MessageEtiquette {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{SEPARATEUR}{}",
            self.environnement.nom(),
            self.empreinte.hexadecimal()
        )
    }
}

/// Nom de l'étiquette d'approbation d'un plan : `approuve/<empreinte courte>`.
pub fn nom_etiquette_approbation(empreinte: &Empreinte) -> String {
    format!("{PREFIXE_ETIQUETTE_APPROBATION}{}", empreinte.abregee())
}

/// La commande exacte que le superviseur exécute sur son poste pour approuver.
///
/// Le message ne contient que des minuscules, des chiffres et une espace : il
/// se cite sans échappement.
pub fn commande_signature(message: &MessageEtiquette, commit: &str) -> String {
    format!(
        "git tag -s {} {commit} -m \"{message}\"",
        nom_etiquette_approbation(message.empreinte())
    )
}
