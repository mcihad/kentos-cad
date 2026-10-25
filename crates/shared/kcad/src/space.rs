//! A space's fields beside its parts (docs/adr/0342 §1), as schema 78 writes them: one map, each field only when it
//! is not its default, so a file has one way to say a space.

use kentos_contracts::{
    ArchInfo, SpaceBreak, SpaceColumns, SpaceFields, SpaceFinishes, SpaceMethod, SpacePart, Vec2,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SpaceRest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<Vec2>,
    #[serde(default, skip_serializing_if = "is_net")]
    pub method: SpaceMethod,
    #[serde(default, skip_serializing_if = "SpaceColumns::is_ignore")]
    pub columns: SpaceColumns,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<String>,
    #[serde(default, skip_serializing_if = "ArchInfo::is_empty")]
    pub arch: ArchInfo,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
    #[serde(default, skip_serializing_if = "SpaceFinishes::is_empty")]
    pub finishes: SpaceFinishes,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub broken: Option<SpaceBreak>,
}

fn is_net(m: &SpaceMethod) -> bool {
    *m == SpaceMethod::Net
}

impl SpaceRest {
    pub(crate) fn of(f: &SpaceFields) -> SpaceRest {
        SpaceRest {
            seed: f.seed,
            method: f.method,
            columns: f.columns,
            number: f.number.clone(),
            name: f.name.clone(),
            usage: f.usage.clone(),
            arch: f.arch.clone(),
            height: f.height,
            finishes: f.finishes.clone(),
            broken: f.broken,
        }
    }

    pub(crate) fn with(self, parts: Vec<SpacePart>) -> SpaceFields {
        SpaceFields {
            parts,
            seed: self.seed,
            method: self.method,
            columns: self.columns,
            number: self.number,
            name: self.name,
            usage: self.usage,
            arch: self.arch,
            height: self.height,
            finishes: self.finishes,
            broken: self.broken,
        }
    }
}
