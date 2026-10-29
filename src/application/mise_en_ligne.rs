//! Cas d'usage — mise en ligne d'un projet.
//!
//! L'ordre des étapes porte l'essentiel de la valeur.
//!
//! Les gates du plancher sont vérifiées **avant** toute soumission à
//! approbation. Une gate rouge n'est donc pas découverte au déploiement, ni
//! soumise au jugement d'un relecteur qui devrait se souvenir de la vérifier :
//! elle bloque en amont. C'est la distinction que `gates.md` pose entre le
//! plancher mécanique, qui ne se discute pas, et l'exigence de jalon, qui se
//! trie.
//!
//! La **barrière de déploiement** (ADR-021) vient ensuite, toujours avant la
//! soumission : le plancher dit si le logiciel est prêt, la barrière si la
//! plateforme l'est. Le plan soumis dit ce qu'elle a vu, y compris qu'elle
//! était vide : le superviseur approuve un état, pas une intention.

use std::sync::Arc;

use crate::application::ports::{
    EtatApprobation, EvaluateurBarriere, GatePlancher, Horloge, PasserelleApprobation,
    VerificateurGates,
};
use crate::domain::{
    Action, AppError, ControleBarriere, Environnement, ExceptionBarriereVide, PlanChangement, Tier,
    VerdictControle,
};

/// Résultat d'une demande de mise en ligne.
#[derive(Debug)]
pub enum IssueMiseEnLigne {
    /// Refusée avant soumission : au moins une gate du plancher est rouge.
    RefuseeParLesGates {
        /// Les gates fautives.
        gates_rouges: Vec<GatePlancher>,
    },
    /// Refusée avant soumission : au moins un contrôle de la barrière est rouge.
    RefuseeParLaBarriere {
        /// Les contrôles rouges, avec leur motif.
        controles_rouges: Vec<VerdictControle>,
    },
    /// Refusée avant soumission : la barrière est vide, sans exception
    /// datée en cours (ADR-021).
    RefuseeBarriereVide {
        /// Pourquoi l'exception ne s'applique pas.
        motif: String,
    },
    /// Soumise à approbation humaine.
    Soumise {
        /// Le plan soumis.
        plan: Box<PlanChangement>,
        /// L'état initial rendu par la passerelle.
        etat: EtatApprobation,
    },
}

/// La barrière de déploiement telle que la mise en ligne l'évalue.
pub struct Barriere {
    /// Évalue les contrôles.
    pub evaluateur: Arc<dyn EvaluateurBarriere>,
    /// Contrôles déclarés, dans l'ordre.
    pub controles: Vec<ControleBarriere>,
    /// Exception datée qui admet une barrière vide.
    pub exception_vide: Option<ExceptionBarriereVide>,
    /// Horloge, pour l'échéance de l'exception.
    pub horloge: Arc<dyn Horloge>,
}

/// Ce que la barrière a laissé passer, à écrire dans le plan.
enum Passage {
    Verte(Vec<VerdictControle>),
    VideAdmise(String),
}

impl Barriere {
    fn franchir(&self) -> Result<Passage, IssueMiseEnLigne> {
        if self.controles.is_empty() {
            return match &self.exception_vide {
                Some(exception) if exception.admet(self.horloge.maintenant()) => {
                    Ok(Passage::VideAdmise(exception.jusqu_au().to_string()))
                }
                Some(exception) => Err(IssueMiseEnLigne::RefuseeBarriereVide {
                    motif: format!(
                        "barrière vide : l'exception d'ADR-021 a expiré le {}",
                        exception.jusqu_au()
                    ),
                }),
                None => Err(IssueMiseEnLigne::RefuseeBarriereVide {
                    motif: "barrière vide, sans exception datée : aucun contrôle de plateforme \
                            ne pourrait être vu par l'approbation"
                        .to_string(),
                }),
            };
        }
        let verdicts = self.evaluateur.evaluer(&self.controles);
        let mut rouges: Vec<VerdictControle> =
            verdicts.iter().filter(|v| !v.vert).cloned().collect();
        // Un verdict manquant compte rouge : un évaluateur qui en perdrait
        // un ne doit pas ouvrir la barrière.
        if verdicts.len() != self.controles.len() {
            rouges.push(VerdictControle {
                nom: "barriere".to_string(),
                vert: false,
                motif: format!(
                    "{} verdict(s) pour {} contrôle(s)",
                    verdicts.len(),
                    self.controles.len()
                ),
                rapport: String::new(),
            });
        }
        if !rouges.is_empty() {
            return Err(IssueMiseEnLigne::RefuseeParLaBarriere {
                controles_rouges: rouges,
            });
        }
        Ok(Passage::Verte(verdicts))
    }
}

/// Orchestre une mise en ligne.
pub struct MiseEnLigne {
    gates: Arc<dyn VerificateurGates>,
    passerelle: Arc<dyn PasserelleApprobation>,
    barriere: Barriere,
}

impl MiseEnLigne {
    /// Construit le cas d'usage.
    pub fn new(
        gates: Arc<dyn VerificateurGates>,
        passerelle: Arc<dyn PasserelleApprobation>,
        barriere: Barriere,
    ) -> Self {
        Self {
            gates,
            passerelle,
            barriere,
        }
    }

    /// Demande la mise en ligne d'une référence.
    ///
    /// Ne met rien en ligne : produit un plan de Tier 1 et le soumet. C'est
    /// l'humain, via un environnement GitHub protégé, qui décide, et c'est le
    /// travail GitHub qui exécute avec des secrets que Sluis n'a pas.
    pub fn demander(
        &self,
        projet: &str,
        environnement: Environnement,
        reference: &str,
    ) -> Result<IssueMiseEnLigne, AppError> {
        let etats = self.gates.etat_plancher(reference)?;
        let rouges: Vec<GatePlancher> = etats.iter().filter(|e| !e.verte).map(|e| e.gate).collect();
        if !rouges.is_empty() {
            return Ok(IssueMiseEnLigne::RefuseeParLesGates {
                gates_rouges: rouges,
            });
        }

        let passage = match self.barriere.franchir() {
            Ok(passage) => passage,
            Err(refus) => return Ok(refus),
        };
        let (barriere_resume, barriere_detail) = match &passage {
            Passage::Verte(verdicts) => (
                format!("{} contrôle(s) de la barrière au vert", verdicts.len()),
                verdicts
                    .iter()
                    .map(|v| format!("barrière {} : vert ({})", v.nom, v.motif))
                    .collect::<Vec<_>>(),
            ),
            Passage::VideAdmise(jusqu_au) => (
                format!("BARRIÈRE VIDE, exception d'ADR-021 jusqu'au {jusqu_au}"),
                vec![format!(
                    "barrière vide : aucun contrôle de plateforme n'a tourné \
                     (exception d'ADR-021 jusqu'au {jusqu_au})"
                )],
            ),
        };

        let plan = PlanChangement::new(
            Action::MiseEnLigne,
            environnement,
            Tier::One,
            format!("{projet}@{reference}"),
            format!(
                "met en ligne {projet} sur {environnement}, {} gate(s) du plancher au vert, \
                 {barriere_resume}",
                etats.len()
            ),
            etats
                .iter()
                .map(|e| format!("{} : vert", e.gate.nom()))
                .chain(barriere_detail)
                .collect::<Vec<_>>()
                .join("\n"),
        )?;

        let etat = self.passerelle.soumettre(&plan)?;
        Ok(IssueMiseEnLigne::Soumise {
            plan: Box::new(plan),
            etat,
        })
    }
}
