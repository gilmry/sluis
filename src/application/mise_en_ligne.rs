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
    AuditLog, EtatApprobation, EvaluateurBarriere, GatePlancher, Horloge, PasserelleApprobation,
    VerificateurGates,
};
use crate::domain::{
    Action, AppError, AuditEntry, ControleBarriere, ElementPreuve, Environnement,
    ExceptionBarriereVide, PlanChangement, RapportPreuve, Tier, VerdictControle,
    SCHEMA_RAPPORT_PREUVE,
};

/// Nom sous lequel le rapport de preuve est consigné au journal.
pub const OUTIL_PREUVE: &str = "sluis_mise_en_ligne/preuve";

/// Identifiant couvert quand le plancher est vert avant la demande
/// (« changements testés avant la production », conformite.md §2ter).
const CYFUN_PLANCHER_VERT: &str = "PR.PS-06.2";

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

/// Ce que la barrière a vu, qu'elle laisse passer ou non : le rapport de
/// preuve en a besoin dans les deux cas.
struct Franchissement {
    verdicts: Vec<VerdictControle>,
    exception: Option<String>,
    issue: Result<Passage, IssueMiseEnLigne>,
}

impl Barriere {
    fn franchir(&self) -> Franchissement {
        if self.controles.is_empty() {
            let exception = self
                .exception_vide
                .as_ref()
                .map(|e| e.jusqu_au().to_string());
            let issue = match &self.exception_vide {
                Some(e) if e.admet(self.horloge.maintenant()) => {
                    Ok(Passage::VideAdmise(e.jusqu_au().to_string()))
                }
                Some(e) => Err(IssueMiseEnLigne::RefuseeBarriereVide {
                    motif: format!(
                        "barrière vide : l'exception d'ADR-021 a expiré le {}",
                        e.jusqu_au()
                    ),
                }),
                None => Err(IssueMiseEnLigne::RefuseeBarriereVide {
                    motif: "barrière vide, sans exception datée : aucun contrôle de plateforme \
                            ne pourrait être vu par l'approbation"
                        .to_string(),
                }),
            };
            return Franchissement {
                verdicts: Vec::new(),
                exception,
                issue,
            };
        }
        let mut verdicts = self.evaluateur.evaluer(&self.controles);
        // Un verdict manquant compte rouge : un évaluateur qui en perdrait
        // un ne doit pas ouvrir la barrière.
        if verdicts.len() != self.controles.len() {
            let motif = format!(
                "{} verdict(s) pour {} contrôle(s)",
                verdicts.len(),
                self.controles.len()
            );
            verdicts.push(VerdictControle {
                nom: "barriere".to_string(),
                vert: false,
                motif,
                rapport: String::new(),
            });
        }
        let rouges: Vec<VerdictControle> = verdicts.iter().filter(|v| !v.vert).cloned().collect();
        let issue = if rouges.is_empty() {
            Ok(Passage::Verte(verdicts.clone()))
        } else {
            Err(IssueMiseEnLigne::RefuseeParLaBarriere {
                controles_rouges: rouges,
            })
        };
        Franchissement {
            verdicts,
            exception: None,
            issue,
        }
    }

    fn cyfun_de(&self, nom: &str) -> Vec<String> {
        self.controles
            .iter()
            .find(|c| c.nom() == nom)
            .map(|c| c.cyfun().to_vec())
            .unwrap_or_default()
    }
}

/// Orchestre une mise en ligne.
pub struct MiseEnLigne {
    gates: Arc<dyn VerificateurGates>,
    passerelle: Arc<dyn PasserelleApprobation>,
    barriere: Barriere,
    journal: Arc<dyn AuditLog>,
}

impl MiseEnLigne {
    /// Construit le cas d'usage.
    ///
    /// `journal` reçoit le rapport de preuve de chaque demande (ADR-022).
    pub fn new(
        gates: Arc<dyn VerificateurGates>,
        passerelle: Arc<dyn PasserelleApprobation>,
        barriere: Barriere,
        journal: Arc<dyn AuditLog>,
    ) -> Self {
        Self {
            gates,
            passerelle,
            barriere,
            journal,
        }
    }

    /// Demande la mise en ligne d'une référence.
    ///
    /// Ne met rien en ligne : produit un plan de Tier 1 et le soumet. C'est
    /// l'humain, via un environnement GitHub protégé, qui décide, et c'est le
    /// travail GitHub qui exécute avec des secrets que Sluis n'a pas.
    ///
    /// Toute demande qui atteint le plancher laisse un rapport de preuve au
    /// journal, refus compris, **avant** la soumission : sans preuve écrite,
    /// rien n'est soumis.
    pub fn demander(
        &self,
        projet: &str,
        environnement: Environnement,
        reference: &str,
    ) -> Result<IssueMiseEnLigne, AppError> {
        let etats = self.gates.etat_plancher(reference)?;
        let mut rapport = RapportPreuve {
            schema: SCHEMA_RAPPORT_PREUVE,
            horodatage: self.barriere.horloge.maintenant(),
            projet: projet.to_string(),
            environnement: environnement.nom().to_string(),
            reference: reference.to_string(),
            version_sluis: env!("CARGO_PKG_VERSION").to_string(),
            issue: "refusee_par_les_gates",
            empreinte_plan: None,
            barriere_vide_jusqu_au: None,
            cyfun: Vec::new(),
            controles: etats
                .iter()
                .map(|e| ElementPreuve {
                    source: "plancher",
                    nom: e.gate.nom().to_string(),
                    vert: e.verte,
                    motif: if e.verte {
                        "vert".to_string()
                    } else {
                        format!("rouge : {}", e.detail)
                    },
                    rapport: String::new(),
                    cyfun: e.gate.cyfun().iter().map(|c| c.to_string()).collect(),
                })
                .collect(),
        };

        let rouges: Vec<GatePlancher> = etats.iter().filter(|e| !e.verte).map(|e| e.gate).collect();
        if !rouges.is_empty() {
            self.consigner(rapport)?;
            return Ok(IssueMiseEnLigne::RefuseeParLesGates {
                gates_rouges: rouges,
            });
        }
        rapport.cyfun.push(CYFUN_PLANCHER_VERT.to_string());

        let franchissement = self.barriere.franchir();
        rapport.barriere_vide_jusqu_au = franchissement.exception.clone();
        rapport
            .controles
            .extend(franchissement.verdicts.iter().map(|v| ElementPreuve {
                source: "barriere",
                nom: v.nom.clone(),
                vert: v.vert,
                motif: v.motif.clone(),
                rapport: v.rapport.clone(),
                cyfun: self.barriere.cyfun_de(&v.nom),
            }));
        let passage = match franchissement.issue {
            Ok(passage) => passage,
            Err(refus) => {
                rapport.issue = match refus {
                    IssueMiseEnLigne::RefuseeBarriereVide { .. } => "refusee_barriere_vide",
                    _ => "refusee_par_la_barriere",
                };
                self.consigner(rapport)?;
                return Ok(refus);
            }
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

        rapport.issue = "admise";
        rapport.empreinte_plan = Some(plan.empreinte().hexadecimal().to_string());
        // Écrit avant la soumission : une approbation demandée sans preuve
        // serait une preuve manquante, pas un oubli (ADR-022, point 5).
        self.consigner(rapport)?;

        let etat = self.passerelle.soumettre(&plan)?;
        Ok(IssueMiseEnLigne::Soumise {
            plan: Box::new(plan),
            etat,
        })
    }

    fn consigner(&self, rapport: RapportPreuve) -> Result<(), AppError> {
        let empreinte = rapport
            .empreinte_plan
            .as_deref()
            .map(|e| e.chars().take(12).collect())
            .unwrap_or_else(|| "-".to_string());
        let entree = AuditEntry::new(
            rapport.horodatage.to_string(),
            OUTIL_PREUVE.to_string(),
            Tier::Two,
            empreinte,
            Ok(()),
        )
        .avec_preuve(rapport);
        self.journal.append(&entree)
    }
}
