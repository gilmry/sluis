//! Le contrat du dépôt cible, matérialisé (ADR-034, points 1 et 2).
//!
//! Une skill Foyer écrit les fichiers d'un projet, Sluis les lit puis les
//! exécute. Ce module est la **seule source de vérité** de ce qui les sépare :
//! les lecteurs de Sluis prennent leurs chemins et leurs clés ici, et le
//! schéma publié (`contrat/depot-cible.v1.json`) est produit depuis ce module,
//! jamais écrit à la main. La skill lit ce fichier, elle ne le recopie pas.
//!
//! **Ce que le schéma ne prouve pas seul** : qu'un lecteur exige vraiment ce
//! qu'il annonce. C'est le rôle de `tests/contrat_depot.rs`, sur le modèle du
//! harnais d'ADR-005 : chaque clé annoncée est retirée tour à tour, et le
//! lecteur doit refuser.

use serde::Serialize;

use crate::domain::module_tofu::{
    constructions_interdites, EXTENSIONS_JSON_REFUSEES, EXTENSIONS_PROGRAMME, FICHIER_VERROU,
    MODULES_PARTAGES,
};
use crate::domain::{Environnement, Topologie};

/// Version du contrat. Une évolution incompatible en crée une nouvelle ; la
/// règle qui dit ce qui est incompatible reste un point ouvert d'ADR-034.
pub const VERSION_CONTRAT_DEPOT: u32 = 1;

/// Dossier du dépôt cible qui porte l'infrastructure. Les chemins suivants
/// sont relatifs à lui, sauf le workflow et les fichiers à la racine.
pub const RACINE_INFRASTRUCTURE: &str = "infrastructure";

/// La déclaration de charge.
pub const FICHIER_CHARGE: &[&str] = &["_shared", "charge.yaml"];

/// Le dossier des profils de cluster.
pub const DOSSIER_PROFILS: &[&str] = &["_shared", "cluster-profiles"];

/// Extensions d'un profil de cluster.
pub const EXTENSIONS_PROFIL: &[&str] = &["yaml", "yml"];

/// Les dossiers de regroupement des cellules `<topologie>/<environnement>`.
pub const REGROUPEMENTS: &[&str] = &["monosite", "multisite"];

/// Le workflow de Tier 1, copié du gabarit de Sluis (ADR-012, ADR-013).
pub const WORKFLOW_TIER1: &str = ".github/workflows/tier1-apply.yml";

/// Les clés de `charge.yaml`, toutes requises.
pub mod cles_charge {
    /// Topologie mesurée.
    pub const TOPOLOGIE: &str = "topologie";
    /// Module OpenTofu qui loue la cible.
    pub const MODULE: &str = "module";
    /// Sortie du module qui donne l'adresse de la cible.
    pub const SORTIE_ADRESSE: &str = "sortie_adresse";
    /// Chemin HTTP mesuré.
    pub const CHEMIN: &str = "chemin";
    /// Débit visé.
    pub const CIBLE_RPS: &str = "cible.requetes_par_seconde";
    /// Latence au 99e centile visée.
    pub const CIBLE_P99: &str = "cible.p99_millisecondes";
    /// Durée de vie demandée.
    pub const BORNE_TTL: &str = "bornes.ttl_secondes";
    /// Plafond de dépense demandé.
    pub const BORNE_PLAFOND: &str = "bornes.plafond_depense";

    /// Toutes, dans l'ordre du fichier.
    pub const TOUTES: &[&str] = &[
        TOPOLOGIE,
        MODULE,
        SORTIE_ADRESSE,
        CHEMIN,
        CIBLE_RPS,
        CIBLE_P99,
        BORNE_TTL,
        BORNE_PLAFOND,
    ];
}

/// Les clés d'un profil de cluster, toutes facultatives : une clé absente est
/// rendue absente, jamais devinée.
pub mod cles_profil {
    /// Classe de stockage.
    pub const STOCKAGE: &str = "global.storageClassName";
    /// Classe d'ingress.
    pub const INGRESS: &str = "global.ingressClassName";
    /// Dorsale des secrets.
    pub const SECRETS: &str = "global.secretsBackend";
    /// TLS activé (`true` ou autre chose).
    pub const TLS: &str = "global.tls.enabled";
    /// Préréglage de ressources.
    pub const RESSOURCES: &str = "resources.preset";

    /// Toutes.
    pub const TOUTES: &[&str] = &[STOCKAGE, INGRESS, SECRETS, TLS, RESSOURCES];
}

/// Le sous-ensemble YAML que lit l'analyseur restreint de Sluis.
const YAML_ADMIS: &[&str] = &[
    "clés imbriquées par indentation d'espaces",
    "valeurs scalaires, nues ou entre guillemets simples ou doubles",
    "commentaires introduits par # précédé d'un blanc",
];

/// Ce que l'analyseur ne lit pas. Une tabulation d'indentation est une erreur ;
/// le reste est ignoré, donc absent pour Sluis.
const YAML_HORS_SOUS_ENSEMBLE: &[&str] = &[
    "indentation par tabulation (refusée)",
    "listes",
    "ancres et références",
    "documents multiples",
    "blocs littéraux | et >",
];

/// Une place réservée par ADR-034, point 8, dont le format reste à définir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlaceReservee {
    /// Chemin relatif à la racine du dépôt.
    pub chemin: &'static str,
    /// ADR qui apporte le fichier.
    pub adr: &'static str,
}

const PLACES_RESERVEES: &[PlaceReservee] = &[
    PlaceReservee {
        chemin: ".sops.yaml",
        adr: "ADR-016",
    },
    PlaceReservee {
        chemin: "infrastructure/_shared/ansible/",
        adr: "ADR-032",
    },
    PlaceReservee {
        chemin: "infrastructure/_shared/flux/",
        adr: "ADR-029",
    },
    PlaceReservee {
        chemin: "infrastructure/_shared/cloudnativepg/",
        adr: "ADR-026",
    },
];

/// L'arborescence `infrastructure/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Arborescence {
    /// Dossier racine de l'infrastructure.
    pub racine: &'static str,
    /// Déclaration de charge.
    pub charge: String,
    /// Profils de cluster, `<dossier>/*.<extension>`.
    pub profils: String,
    /// Extensions admises d'un profil.
    pub extensions_profil: Vec<&'static str>,
    /// Modules partagés, `<dossier>/<nom>/`.
    pub modules_partages: String,
    /// Cellules, `<regroupement>/<topologie>/<environnement>/`.
    pub cellules: String,
    /// Regroupements admis.
    pub regroupements: Vec<&'static str>,
    /// Topologies admises.
    pub topologies: Vec<&'static str>,
    /// Environnements admis.
    pub environnements: Vec<&'static str>,
}

/// Le sous-ensemble YAML.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SousEnsembleYaml {
    /// Ce qui est lu.
    pub admis: Vec<&'static str>,
    /// Ce qui ne l'est pas.
    pub hors_sous_ensemble: Vec<&'static str>,
}

/// Les règles des programmes OpenTofu (ADR-034, point 3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReglesTofu {
    /// Fichier de verrouillage obligatoire à la racine de chaque module lancé.
    pub verrou_obligatoire: &'static str,
    /// Extensions lues.
    pub extensions_programme: Vec<&'static str>,
    /// Extensions refusées.
    pub extensions_refusees: Vec<&'static str>,
    /// Constructions refusées.
    pub constructions_interdites: Vec<String>,
    /// Seule source admise d'un bloc `module`.
    pub source_module: String,
    /// La liste blanche des fournisseurs est propre à chaque déploiement de
    /// Sluis (`[tofu] fournisseurs_autorises`), vide par défaut : tout refusé.
    pub fournisseurs: &'static str,
}

/// Le contrat du dépôt cible, tel que publié.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContratDepot {
    /// Version du contrat.
    pub version: u32,
    /// Arborescence `infrastructure/`.
    pub arborescence: Arborescence,
    /// Clés lues dans `charge.yaml`, toutes requises.
    pub cles_charge: Vec<&'static str>,
    /// Clés lues dans un profil de cluster, toutes facultatives.
    pub cles_profil: Vec<&'static str>,
    /// Sous-ensemble YAML admis.
    pub yaml: SousEnsembleYaml,
    /// Règles des programmes OpenTofu.
    pub tofu: ReglesTofu,
    /// Workflow de Tier 1, copié du gabarit.
    pub workflow_tier1: &'static str,
    /// Places réservées, format à définir.
    pub places_reservees: Vec<PlaceReservee>,
}

impl ContratDepot {
    /// Le contrat que ce binaire applique.
    pub fn courant() -> Self {
        Self {
            version: VERSION_CONTRAT_DEPOT,
            arborescence: Arborescence {
                racine: RACINE_INFRASTRUCTURE,
                charge: FICHIER_CHARGE.join("/"),
                profils: format!("{}/*", DOSSIER_PROFILS.join("/")),
                extensions_profil: EXTENSIONS_PROFIL.to_vec(),
                modules_partages: format!("{}/<nom>/", MODULES_PARTAGES.join("/")),
                cellules: "<regroupement>/<topologie>/<environnement>/".to_string(),
                regroupements: REGROUPEMENTS.to_vec(),
                topologies: Topologie::TOUTES.iter().map(Topologie::nom).collect(),
                environnements: Environnement::TOUS.iter().map(Environnement::nom).collect(),
            },
            cles_charge: cles_charge::TOUTES.to_vec(),
            cles_profil: cles_profil::TOUTES.to_vec(),
            yaml: SousEnsembleYaml {
                admis: YAML_ADMIS.to_vec(),
                hors_sous_ensemble: YAML_HORS_SOUS_ENSEMBLE.to_vec(),
            },
            tofu: ReglesTofu {
                verrou_obligatoire: FICHIER_VERROU,
                extensions_programme: EXTENSIONS_PROGRAMME.to_vec(),
                extensions_refusees: EXTENSIONS_JSON_REFUSEES.to_vec(),
                constructions_interdites: constructions_interdites(),
                source_module: format!("{}/<nom>/", MODULES_PARTAGES.join("/")),
                fournisseurs: "liste blanche du déploiement, vide par défaut",
            },
            workflow_tier1: WORKFLOW_TIER1,
            places_reservees: PLACES_RESERVEES.to_vec(),
        }
    }
}
