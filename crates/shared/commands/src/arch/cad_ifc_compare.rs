//! Two revisions of an IFC model compared, nothing written (plan/ifc-uyum.md IFC-10, docs/adr/8103): `cad.ifc.compare`
//! matches objects by their GlobalIds alone and says what the newer file added, removed and changed: an object's class,
//! attributes, container, type, body (moved, resized, one side only) and its sets' values. The comparing is
//! `kentos-ifc`'s (`kentos_ifc::compare`); the desktop's handler is `crates/native/application/src/arch/ifc_compare.rs`;
//! it and the Python SDK pass the shared cases in `fixtures/commands/v1/cad.ifc.compare.json`, written from an
//! independent reader's view of files IfcOpenShell made (`scripts/fixtures/ifc_compare_cases.py`).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::IfcValue;

pub const CAD_IFC_COMPARE: &str = "cad.ifc.compare";
pub const CAD_IFC_COMPARE_VERSION: u32 = 1;
/// The tolerance when none is given, metres.
pub const IFC_COMPARE_TOLERANCE: f64 = 0.001;
/// The smallest tolerance, metres: above what a body's float32 corners may stray.
pub const MIN_IFC_COMPARE_TOLERANCE: f64 = 0.0001;
/// The largest tolerance, metres.
pub const MAX_IFC_COMPARE_TOLERANCE: f64 = 0.5;

/// Input of `cad.ifc.compare` v1: the older and the newer file (absolute, ending `.ifc`); a body whose box moves or
/// changes size by no more than `tolerance` metres is the same.
///
/// Refusals, in this order: `invalid_file_path` (empty, relative, not `.ifc`, not there; at `older` or `newer`),
/// `invalid_tolerance` (0.0001 to 0.5 m), `ifc_unreadable` (at the file's field). Warning: `ifc_repeated_global_id` (a
/// GlobalId a file holds more than once: those objects are left out and listed).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct IfcCompare {
    pub older: String,
    pub newer: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tolerance: Option<f64>,
}

/// A file compared: its path, its schema as the header declares it, how many objects it holds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct IfcComparedFile {
    pub path: String,
    pub schema: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u64,
}

/// An object as a list names it: its GlobalId, its class as the schema spells it and its name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct IfcComparedObject {
    pub global_id: String,
    pub class: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
}

/// One change of an object both files have. `class`: its class; `attribute`: one of Name, Description, ObjectType,
/// LongName, Tag, PredefinedType (absent where unset); `container`: the spatial structure containing it; `type`: its
/// type object; `body`: a body on one side only; `moved`: its box's middle moved (newer less older, metres) and how
/// far; `resized`: its box's size along x, y, z in each file; `property`: a value of one of its sets (the set's name,
/// whether the set is its type's, the value's name; absent on the side that has none).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum IfcChange {
    Class {
        old: String,
        new: String,
    },
    Attribute {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        old: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        new: Option<String>,
    },
    Container {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        old: Option<IfcComparedObject>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        new: Option<IfcComparedObject>,
    },
    Type {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        old: Option<IfcComparedObject>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        new: Option<IfcComparedObject>,
    },
    Body {
        old: bool,
        new: bool,
    },
    Moved {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        by: Vec<f64>,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        distance: f64,
    },
    Resized {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        old: Vec<f64>,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        new: Vec<f64>,
    },
    Property {
        set: String,
        #[serde(rename = "fromType")]
        #[cfg_attr(feature = "ts", ts(rename = "fromType"))]
        from_type: bool,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        old: Option<IfcValue>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        new: Option<IfcValue>,
    },
}

/// An object both files have with what changed on it: its GlobalId, its class and name in the newer file, its changes
/// (class, attributes, container, type, body, then its sets' values: the newer file's in its order, then the values only
/// the older has).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct IfcComparedChange {
    pub global_id: String,
    pub class: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    pub changes: Vec<IfcChange>,
}

/// A GlobalId a file holds more than once (`older` or `newer`): its objects are left out of the comparison.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct IfcRepeatedId {
    pub file: String,
    pub global_id: String,
}

/// Output of `cad.ifc.compare` v1, and its plan: the two files, what the newer added (in its order), removed (in the
/// older's order) and changed (in its order), how many objects are the same, the GlobalIds left out, the tolerance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct IfcCompared {
    pub older: IfcComparedFile,
    pub newer: IfcComparedFile,
    pub added: Vec<IfcComparedObject>,
    pub removed: Vec<IfcComparedObject>,
    pub changed: Vec<IfcComparedChange>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub unchanged: u64,
    pub repeated: Vec<IfcRepeatedId>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tolerance: f64,
}
