//! Explicit, zero-thickness architectural room boundaries (ADR 0650).
use serde::{Deserialize,Serialize};
use std::collections::BTreeMap;
#[cfg(feature="ts")]
use ts_rs::TS;
use crate::{ArchInfo,Entity,Vec2};
pub const CAD_SPACES_SEPARATORS:&str="cad.spaces.separators";
pub const CAD_SPACES_SEPARATORS_VERSION:u32=1;
pub const MAX_SEPARATOR_PATHS:usize=10_000;
pub const MAX_SEPARATOR_VERTICES:usize=100_000;

#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
#[cfg_attr(feature="ts",derive(TS))]
#[cfg_attr(feature="schema",derive(schemars::JsonSchema))]
#[cfg_attr(feature="ts",ts(export))]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct NewSpaceSeparator {
    #[cfg_attr(feature="schema",schemars(extend("x-unit"="length")))]
    pub pts:Vec<Vec2>,
    #[serde(default,skip_serializing_if="Option::is_none")]
    #[cfg_attr(feature="ts",ts(optional))]
    #[cfg_attr(feature="schema",schemars(extend("x-unit"="1")))]
    pub bulges:Option<Vec<f64>>,
    #[serde(default)]
    pub closed:bool,
    #[serde(default,skip_serializing_if="Option::is_none")]
    #[cfg_attr(feature="ts",ts(optional))]
    pub arch:Option<ArchInfo>,
}

#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
#[cfg_attr(feature="ts",derive(TS))]
#[cfg_attr(feature="schema",derive(schemars::JsonSchema))]
#[cfg_attr(feature="ts",ts(export))]
#[serde(tag="kind",rename_all="camelCase",deny_unknown_fields)]
pub enum SpaceSeparatorOperation {
    #[serde(rename_all="camelCase")]
    Create {
        layer_id:String, paths:Vec<NewSpaceSeparator>,
        #[serde(default,skip_serializing_if="Option::is_none")]
        #[cfg_attr(feature="ts",ts(optional))]
        color:Option<String>,
        #[serde(default,skip_serializing_if="Option::is_none")]
        #[cfg_attr(feature="ts",ts(optional))]
        #[cfg_attr(feature="schema",schemars(extend("x-unit"="paper-mm")))]
        line_weight:Option<f64>,
        #[serde(default,skip_serializing_if="Option::is_none")]
        #[cfg_attr(feature="ts",ts(optional))]
        line_type:Option<crate::layer::LineType>,
        #[serde(default,skip_serializing_if="Option::is_none")]
        #[cfg_attr(feature="ts",ts(optional))]
        attrs:Option<BTreeMap<String,String>>,
    },
    /// Assign the exact role information to existing suitable polylines.
    Set {
        uids:Vec<String>,
        #[serde(default,skip_serializing_if="Option::is_none")]
        #[cfg_attr(feature="ts",ts(optional))]
        arch:Option<ArchInfo>,
    },
    /// Keep geometry and appearance, removing only the explicit role.
    Clear {uids:Vec<String>},
}

#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
#[cfg_attr(feature="ts",derive(TS))]
#[cfg_attr(feature="schema",derive(schemars::JsonSchema))]
#[cfg_attr(feature="ts",ts(export))]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct SpacesSeparators {
    pub operation:SpaceSeparatorOperation,
    #[serde(default,skip_serializing_if="Option::is_none")]
    #[cfg_attr(feature="ts",ts(optional))]
    pub expected_revision:Option<String>,
}

#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
#[cfg_attr(feature="ts",derive(TS))]
#[cfg_attr(feature="schema",derive(schemars::JsonSchema))]
#[cfg_attr(feature="ts",ts(export))]
#[serde(rename_all="camelCase")]
pub struct SpaceSeparatorsPlan {pub entities:Vec<Entity>,pub revision:String}

#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[cfg_attr(feature="ts",derive(TS))]
#[cfg_attr(feature="schema",derive(schemars::JsonSchema))]
#[cfg_attr(feature="ts",ts(export))]
#[serde(rename_all="camelCase")]
pub struct SpaceSeparatorsChanged {
    pub uids:Vec<String>,
    #[cfg_attr(feature="schema",schemars(extend("x-unit"="id")))]
    pub ids:Vec<u32>,
    pub revision:String,
}
