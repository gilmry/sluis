//! Client REST de l'API OVHcloud.
//!
//! **Pourquoi `ureq` et pas un client asynchrone.** Sluis n'a aucun besoin de
//! concurrence pour interroger OVH : les appels sont séquentiels, peu nombreux,
//! et le coût dominant est l'attente réseau d'un humain qui lit un rapport. Un
//! runtime asynchrone apporterait ici un arbre de dépendances considérable
//! pour un gain nul, ce que la sobriété de la méthode déconseille.
//!
//! **Pourquoi pas `wiremock` en test.** Même raison : wiremock impose tokio. Un
//! serveur d'essai sur `TcpListener`, écrit une fois, remplit la même
//! exigence — aucun appel réseau réel en CI (NFR-07) — sans rien ajouter.

use std::time::Duration;

use serde::Deserialize;

use crate::application::ports::FournisseurOvh;
use crate::domain::{AppError, CoutCourant, EnregistrementDns, InstanceOvh, ProjetOvh, Redacted};
use crate::infrastructure::ovh::signature::{signer, IdentiteOvh};

/// Point d'entrée par défaut, Europe.
pub const ENDPOINT_EU: &str = "https://eu.api.ovh.com/1.0";

/// Client de l'API OVHcloud.
pub struct ClientOvh {
    endpoint: String,
    identite: IdentiteOvh,
    /// Écart entre l'horloge du serveur OVH et l'horloge locale, en secondes.
    ecart_horloge: i64,
    /// Instant courant local, injecté pour rendre la signature déterministe.
    horodatage_local: Box<dyn Fn() -> i64 + Send + Sync>,
    agent: ureq::Agent,
}

impl std::fmt::Debug for ClientOvh {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Ni l'identité ni l'agent ne sont affichés : le premier porte des
        // secrets, le second n'apprend rien.
        f.debug_struct("ClientOvh")
            .field("endpoint", &self.endpoint)
            .field("ecart_horloge", &self.ecart_horloge)
            .finish_non_exhaustive()
    }
}

impl ClientOvh {
    /// Construit un client.
    ///
    /// `horodatage_local` est injecté : c'est ce qui permet de rejouer une
    /// signature à l'identique dans un test.
    pub fn new(
        endpoint: String,
        identite: IdentiteOvh,
        ecart_horloge: i64,
        horodatage_local: Box<dyn Fn() -> i64 + Send + Sync>,
    ) -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(30)))
            .build()
            .into();
        Self {
            endpoint,
            identite,
            ecart_horloge,
            horodatage_local,
            agent,
        }
    }

    /// Construit un client en interrogeant `/auth/time` pour l'écart d'horloge.
    ///
    /// Un échec de cet appel n'est pas fatal : l'écart vaut alors zéro, et les
    /// requêtes échoueront avec un message d'authentification explicite plutôt
    /// que d'empêcher le client d'exister.
    pub fn avec_synchronisation(endpoint: String, identite: IdentiteOvh) -> Self {
        let maintenant = || {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0)
        };
        let ecart = ureq::get(format!("{endpoint}/auth/time"))
            .call()
            .ok()
            .and_then(|mut r| r.body_mut().read_to_string().ok())
            .and_then(|t| t.trim().parse::<i64>().ok())
            .map(|serveur| serveur - maintenant())
            .unwrap_or(0);
        Self::new(endpoint, identite, ecart, Box::new(maintenant))
    }

    fn horodatage_signature(&self) -> i64 {
        (self.horodatage_local)() + self.ecart_horloge
    }

    fn obtenir(&self, chemin: &str) -> Result<String, AppError> {
        let url = format!("{}{chemin}", self.endpoint);
        let horodatage = self.horodatage_signature();
        let signature = signer(&self.identite, "GET", &url, "", horodatage);

        let reponse = self
            .agent
            .get(&url)
            .header("X-Ovh-Application", &self.identite.application_key)
            .header("X-Ovh-Consumer", self.identite.consumer_key.exposer())
            .header("X-Ovh-Timestamp", horodatage.to_string())
            .header("X-Ovh-Signature", &signature)
            .call();

        match reponse {
            Ok(mut r) => r
                .body_mut()
                .read_to_string()
                .map_err(|e| AppError::ServiceTiers {
                    service: "OVH".to_string(),
                    detail: format!("lecture du corps impossible : {e}"),
                }),
            Err(ureq::Error::StatusCode(401)) | Err(ureq::Error::StatusCode(403)) => {
                // Le secret n'est pas perdu pour le diagnostic, mais il est
                // typé Redacted : il ne peut pas ressortir en clair.
                Err(AppError::Authentification {
                    secret: Redacted::new(self.identite.application_key.clone()),
                })
            }
            Err(ureq::Error::StatusCode(404)) => Err(AppError::Introuvable {
                quoi: chemin.to_string(),
            }),
            Err(e) => Err(AppError::ServiceTiers {
                service: "OVH".to_string(),
                detail: e.to_string(),
            }),
        }
    }

    fn analyser<T: serde::de::DeserializeOwned>(
        &self,
        corps: &str,
        quoi: &str,
    ) -> Result<T, AppError> {
        serde_json::from_str(corps).map_err(|e| AppError::Analyse {
            quoi: quoi.to_string(),
            detail: e.to_string(),
        })
    }
}

#[derive(Deserialize)]
struct ProjetBrut {
    #[serde(rename = "project_id")]
    identifiant: Option<String>,
    description: Option<String>,
    #[serde(rename = "status")]
    statut: Option<String>,
}

#[derive(Deserialize)]
struct InstanceBrute {
    id: String,
    name: Option<String>,
    #[serde(rename = "flavorId")]
    gabarit: Option<String>,
    region: Option<String>,
    status: Option<String>,
}

#[derive(Deserialize)]
struct CoutBrut {
    #[serde(rename = "currentTotal")]
    total: Option<f64>,
    currency: Option<DeviseBrute>,
    #[serde(rename = "from")]
    debut: Option<String>,
    #[serde(rename = "to")]
    fin: Option<String>,
}

#[derive(Deserialize)]
struct DeviseBrute {
    #[serde(rename = "text")]
    texte: Option<String>,
}

#[derive(Deserialize)]
struct EnregistrementBrut {
    id: u64,
    #[serde(rename = "subDomain")]
    sous_domaine: Option<String>,
    #[serde(rename = "fieldType")]
    type_enregistrement: Option<String>,
    target: Option<String>,
    ttl: Option<u32>,
}

impl FournisseurOvh for ClientOvh {
    fn lister_projets(&self) -> Result<Vec<ProjetOvh>, AppError> {
        let identifiants: Vec<String> =
            self.analyser(&self.obtenir("/cloud/project")?, "liste des projets")?;
        let mut projets = Vec::new();
        for identifiant in identifiants {
            let corps = self.obtenir(&format!("/cloud/project/{identifiant}"))?;
            let brut: ProjetBrut = self.analyser(&corps, "projet")?;
            projets.push(ProjetOvh {
                identifiant: brut.identifiant.unwrap_or(identifiant),
                nom: brut.description.unwrap_or_default(),
                statut: brut.statut.unwrap_or_else(|| "inconnu".to_string()),
            });
        }
        Ok(projets)
    }

    fn lister_instances(&self, projet: &str) -> Result<Vec<InstanceOvh>, AppError> {
        let corps = self.obtenir(&format!("/cloud/project/{projet}/instance"))?;
        let brutes: Vec<InstanceBrute> = self.analyser(&corps, "liste des instances")?;
        Ok(brutes
            .into_iter()
            .map(|b| InstanceOvh {
                identifiant: b.id,
                nom: b.name.unwrap_or_default(),
                gabarit: b.gabarit.unwrap_or_default(),
                region: b.region.unwrap_or_default(),
                etat: b.status.unwrap_or_else(|| "inconnu".to_string()),
            })
            .collect())
    }

    fn instance(&self, projet: &str, instance: &str) -> Result<InstanceOvh, AppError> {
        let corps = self.obtenir(&format!("/cloud/project/{projet}/instance/{instance}"))?;
        let b: InstanceBrute = self.analyser(&corps, "instance")?;
        Ok(InstanceOvh {
            identifiant: b.id,
            nom: b.name.unwrap_or_default(),
            gabarit: b.gabarit.unwrap_or_default(),
            region: b.region.unwrap_or_default(),
            etat: b.status.unwrap_or_else(|| "inconnu".to_string()),
        })
    }

    fn cout_courant(&self, projet: &str) -> Result<CoutCourant, AppError> {
        let corps = self.obtenir(&format!("/cloud/project/{projet}/usage/current"))?;
        let b: CoutBrut = self.analyser(&corps, "consommation courante")?;
        // Un montant absent est rendu absent, pas ramené à zéro : un zéro
        // inventé se lirait comme « ce projet ne coûte rien ».
        let montant = b.total.ok_or_else(|| AppError::Introuvable {
            quoi: format!("données de facturation du projet {projet}"),
        })?;
        Ok(CoutCourant {
            projet: projet.to_string(),
            montant,
            devise: b
                .currency
                .and_then(|d| d.texte)
                .unwrap_or_else(|| "EUR".to_string()),
            debut: b.debut.unwrap_or_default(),
            fin: b.fin.unwrap_or_default(),
        })
    }

    fn enregistrements_dns(&self, zone: &str) -> Result<Vec<EnregistrementDns>, AppError> {
        let identifiants: Vec<u64> = self.analyser(
            &self.obtenir(&format!("/domain/zone/{zone}/record"))?,
            "liste des enregistrements DNS",
        )?;
        let mut enregistrements = Vec::new();
        for identifiant in identifiants {
            let corps = self.obtenir(&format!("/domain/zone/{zone}/record/{identifiant}"))?;
            let b: EnregistrementBrut = self.analyser(&corps, "enregistrement DNS")?;
            enregistrements.push(EnregistrementDns {
                identifiant: b.id.to_string(),
                sous_domaine: b.sous_domaine.unwrap_or_default(),
                type_enregistrement: b.type_enregistrement.unwrap_or_default(),
                cible: b.target.unwrap_or_default(),
                ttl: b.ttl.unwrap_or(0),
            });
        }
        Ok(enregistrements)
    }
}
