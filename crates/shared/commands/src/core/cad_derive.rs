//! Objects derived from others as one product command (docs/adr/0622 §9, TODOS.md `UX-14`): Köşelere nokta, Orta hat
//! and Toplu alan. The caller gives the source objects by persistent id and the options the user sets; the command
//! finds the new objects with the shared core and writes them as `cad.entities.create` writes, one undo step named
//! after the tool. The tools write through it; Python and an agent ask the same.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

/// Writes objects derived from others.
pub const CAD_OBJECTS_DERIVE: &str = "cad.objects.derive";
pub const CAD_OBJECTS_DERIVE_VERSION: u32 = 1;

/// What is derived, from what.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DeriveWork {
    /// Köşelere nokta (docs/adr/0152 §5): a point at every vertex of the lines, polylines and areas, past the places
    /// two vertices share and those points already hold; named from `name` on (Yazı's Artır), each with the code
    /// `code` and the vertex's elevation.
    VertexPoints {
        ids: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        code: Option<String>,
    },
    /// Orta hat (docs/adr/0190): the axis between two sides, each one object or a chain of lines, arcs and polylines
    /// meeting end to end within `tolerance`; exact between matched sides, else sampled every `step`.
    Centerline {
        first: Vec<String>,
        second: Vec<String>,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        step: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        tolerance: Option<f64>,
    },
    /// Toplu alan (docs/adr/0151): every region the line work `lines` closes, an area with the one label (a text, a
    /// named point) of `labels` inside it as the attribute `attribute`; with `islands` the regions inside others too.
    Polygonize {
        lines: Vec<String>,
        #[serde(default)]
        labels: Vec<String>,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
        islands: bool,
        attribute: String,
    },
}

/// `cad.objects.derive`'s input: the work, the layer and the look of what it writes.
///
/// Refusals, in this order: `not_finite`, `invalid_uid`, `entity_not_found`, then the work's own (`not_a_path`,
/// `invalid_distance`, `no_route`, `not_joined`, `nothing_derived` …), then `cad.entities.create`'s.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ObjectsDerive {
    pub layer_id: String,
    pub work: DeriveWork,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 0.0, max = 100.0)))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub line_weight: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// What `cad.objects.derive` wrote.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ObjectsDerived {
    /// The new objects' slots and persistent ids, in order.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub ids: Vec<u32>,
    pub uids: Vec<String>,
    /// Köşelere nokta: the vertices passed over (a point already there). Toplu alan: the regions without a label.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub skipped: u32,
    /// Toplu alan: the regions with more than one label (their attribute left empty) and the regions an area
    /// already covers (not written again).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub many: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub existing: u32,
    /// Köşelere nokta: the name after the last one written, for the next run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub next: Option<String>,
    /// Orta hat: `matched` (side by side, exact) or `sampled` (every step).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub method: Option<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
