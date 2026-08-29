//! Le contrat d'un outil MCP, matérialisé.
//!
//! L'archétype de Sluis est **API-first** : le contrat est le produit, et
//! `contrat-api.md` exige qu'il soit *matérialisé* et non *décrit*. Le skill
//! est né d'un incident réel où un contrat décrit en prose, sans mécanisme,
//! a produit un NO-GO en production.
//!
//! Matérialiser signifie ici quatre choses, toutes présentes :
//!
//! 1. les types d'entrée sont annotés ;
//! 2. le schéma JSON est **généré depuis le type**, jamais écrit à la main ;
//! 3. le type porte `deny_unknown_fields` ;
//! 4. un contract test prouve que le schéma annoncé et la désérialisation
//!    effective acceptent et refusent les mêmes charges utiles.
//!
//! Le point 4 est le seul qui distingue un contrat matérialisé d'un contrat
//! bien intentionné : sans lui, le schéma dérive du code sans que rien ne le
//! signale.

use std::marker::PhantomData;

use serde::de::DeserializeOwned;

/// Ce qu'un outil MCP expose de son contrat, indépendamment de son type
/// d'arguments. C'est l'objet-trait qui rend le registre énumérable, donc les
/// contract tests exhaustifs plutôt qu'échantillonnés.
pub trait ContratOutil: Send + Sync {
    /// Nom de l'outil, tel qu'annoncé par `tools/list`.
    fn nom(&self) -> &'static str;

    /// Description destinée au client MCP.
    fn description(&self) -> &'static str;

    /// Schéma JSON des arguments, généré depuis le type.
    fn schema(&self) -> serde_json::Value;

    /// Tente la désérialisation réelle des arguments.
    ///
    /// C'est cette fonction que le contract test confronte au schéma : elle
    /// emprunte exactement le chemin de `tools/call`, sans réimplémentation.
    fn desérialiser(&self, arguments: &serde_json::Value) -> Result<(), String>;
}

/// Contrat d'un outil, dérivé de son type d'arguments.
///
/// Le schéma et la désérialisation proviennent du **même type**, ce qui rend
/// la dérive structurellement impossible plutôt que seulement détectable.
pub struct Contrat<T> {
    nom: &'static str,
    description: &'static str,
    _arguments: PhantomData<fn() -> T>,
}

impl<T> Contrat<T> {
    /// Déclare le contrat d'un outil.
    pub const fn new(nom: &'static str, description: &'static str) -> Self {
        Self {
            nom,
            description,
            _arguments: PhantomData,
        }
    }
}

impl<T> ContratOutil for Contrat<T>
where
    T: schemars::JsonSchema + DeserializeOwned + Send + Sync,
{
    fn nom(&self) -> &'static str {
        self.nom
    }

    fn description(&self) -> &'static str {
        self.description
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(T))
            .unwrap_or_else(|_| serde_json::json!({"type": "object"}))
    }

    fn desérialiser(&self, arguments: &serde_json::Value) -> Result<(), String> {
        serde_json::from_value::<T>(arguments.clone())
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}
