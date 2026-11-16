//! An IFC file read and described, nothing written (plan/ifc-uyum.md IFC-12, docs/adr/8061 §5 "Komutlar",
//! docs/adr/8073 §6): `cad.ifc.inspect` gives a file's schema, how many records and objects it holds and of which
//! classes, its project's units, its storeys and what each contains, what is wrong in it without stopping the read, and
//! with `object` one object's attributes and its property and quantity sets as the file writes them. The reading is
//! `kentos-ifc`'s reading face; the desktop's handler is `crates/native/application/src/arch/ifc_inspect.rs`; it and
//! the Python SDK pass the shared cases in `fixtures/commands/v1/cad.ifc.inspect.json`, written from an independent
//! reader's reading of the same files (`scripts/fixtures/ifc_inspect_cases.py`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_IFC_INSPECT: &str = "cad.ifc.inspect";
pub const CAD_IFC_INSPECT_VERSION: u32 = 1;

/// Input of `cad.ifc.inspect` v1: the file (absolute, ending `.ifc`) and, by its GlobalId, the object whose details are
/// asked for.
///
/// Refusals, in this order: `invalid_file_path` (empty, relative, not `.ifc`, not there), `ifc_unreadable` (not a STEP
/// file, a schema the reader does not know, no DATA section, past a limit; the message says which and where),
/// `ifc_object_not_found` (`object` names no object of the file). Warning: `ifc_read_findings` (damage that did not
/// stop the read; the findings list it).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct IfcInspect {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub object: Option<String>,
}

/// Output of `cad.ifc.inspect` v1, and its plan: the file as its header declares it, what it holds, and the object
/// asked for.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct IfcInspected {
    pub path: String,
    /// The schema as the header declares it (`IFC4X3_ADD2`).
    pub schema: String,
    /// The records of its DATA section.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub records: u64,
    /// Its objects (every `IfcObjectDefinition`: the project, the spatial structure, elements, types, groups).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u64,
    /// How many objects of each class, the class as the schema spells it.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub classes: BTreeMap<String, u64>,
    pub units: IfcInspectedUnits,
    /// Its building storeys in the file's order, each with the objects it contains directly.
    pub storeys: Vec<IfcInspectedStorey>,
    /// Damage that did not stop the read, in the file's order.
    pub findings: Vec<IfcInspectedFinding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub object: Option<IfcInspectedObject>,
}

/// The project's units for length, area, volume and plane angle, each its prefix and name as written
/// (`MILLIMETRE`, `METRE`, a conversion's `foot`); none where the file gives none.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct IfcInspectedUnits {
    pub length: Option<String>,
    pub area: Option<String>,
    pub volume: Option<String>,
    pub plane_angle: Option<String>,
}

/// A building storey: its GlobalId, its name and how many objects it contains directly
/// (`IfcRelContainedInSpatialStructure`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct IfcInspectedStorey {
    pub global_id: String,
    pub name: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub contains: u64,
}

/// Damage that did not stop the read: its code, `error` or `warning`, its record and line, what it says.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct IfcInspectedFinding {
    pub code: String,
    pub severity: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub step: Option<u32>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub line: Option<u64>,
    pub message: String,
}

/// An object: its GlobalId, record number, class and attributes as written, its type's and its storey's names, and its
/// sets: its own in the file's order, then its type's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct IfcInspectedObject {
    pub global_id: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub step: u32,
    pub class: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub object_type: Option<String>,
    pub long_name: Option<String>,
    pub tag: Option<String>,
    pub predefined_type: Option<String>,
    /// Its type object's name (`IfcRelDefinesByType`).
    pub type_name: Option<String>,
    /// The storey it stands on: the nearest up its parts and containers.
    pub storey: Option<String>,
    pub sets: Vec<IfcInspectedSet>,
}

/// A property set (`properties`), a quantity set (`quantities`) or another property definition (`other`, its name
/// only): its name, whether it came from the object's type, its values in the file's order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct IfcInspectedSet {
    pub name: String,
    pub kind: String,
    pub from_type: bool,
    pub values: Vec<IfcInspectedValue>,
}

/// One property or quantity: its name, its class (`IfcPropertySingleValue`, `IfcQuantityLength`), its value, and its
/// own unit (prefix and name as written) when it has one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct IfcInspectedValue {
    pub name: String,
    pub class: String,
    pub value: IfcValue,
    pub unit: Option<String>,
}

/// A value as the file writes it: a text; a real or an integer with the measure it is typed with (upper case as the
/// file writes it; a quantity's is its value attribute's, `IFCLENGTHMEASURE`); a logical (`null`: unknown); an
/// enumeration's items; a list's items (each in words, a number as the shortest decimal that reads back to it); a
/// bounded value's ends; the record a reference value names; nothing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum IfcValue {
    Text {
        text: String,
    },
    Number {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
        number: f64,
        measure: Option<String>,
    },
    Integer {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
        integer: i64,
        measure: Option<String>,
    },
    Logical {
        logical: Option<bool>,
    },
    Enumeration {
        items: Vec<String>,
    },
    List {
        items: Vec<String>,
    },
    Bounded {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
        lower: Option<f64>,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
        upper: Option<f64>,
        measure: Option<String>,
    },
    Reference {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
        step: u32,
    },
    Empty,
}
