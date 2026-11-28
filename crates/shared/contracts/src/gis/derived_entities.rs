//! Shared preview delta: identities on additions remain provisional.
use crate::Entity;
use serde::{Deserialize, Serialize};

/// Derived preview geometry only. New entities' ids/references are provisional;
/// only the executing command assigns real persistent ids.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DerivedEntitiesPlan {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added: Vec<Entity>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub updated: Vec<Entity>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed: Vec<String>,
}
impl DerivedEntitiesPlan {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.updated.is_empty() && self.removed.is_empty()
    }
}
