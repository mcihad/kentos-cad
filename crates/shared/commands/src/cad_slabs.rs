//! The slab commands of the product command catalog (docs/adr/0722 §4): slabs made with a drawn outline or found from
//! the walls round a point (`cad.slabs.create`), and changed, their edges moved, an area cut out of them or added to
//! them (`cad.slabs.edit`). Lengths are metres on the wire, whatever unit the project shows (docs/adr/0165 §2).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::entity::{Entity, Vec2};
use crate::{ArchInfo, ArchPhase, SlabEdgeKind, SlabRole, SpacePart};

pub const CAD_SLABS_CREATE: &str = "cad.slabs.create";
pub const CAD_SLABS_CREATE_VERSION: u32 = 1;
pub const CAD_SLABS_EDIT: &str = "cad.slabs.edit";
pub const CAD_SLABS_EDIT_VERSION: u32 = 1;
/// The most slabs one call writes.
pub const MAX_SLABS: usize = 10_000;
/// The farthest an edge offset moves a slab's edges, metres, either way.
pub const MAX_SLAB_EDGE_OFFSET: f64 = 10.0;

/// Which of the walls' faces a slab found from them runs along.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SlabBoundary {
    /// Dış yüzler: the outline of the walls round the point, to their outer faces (a storey's whole floor).
    #[default]
    Outline,
    /// Oda: the room round the point, to the walls' inner faces (a wet room's lowered slab, a balcony between walls).
    Room,
}

impl SlabBoundary {
    pub const ALL: [SlabBoundary; 2] = [SlabBoundary::Outline, SlabBoundary::Room];

    pub fn label(self) -> &'static str {
        match self {
            SlabBoundary::Outline => "Dış yüzler",
            SlabBoundary::Room => "Oda",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            SlabBoundary::Outline => "outline",
            SlabBoundary::Room => "room",
        }
    }
}

/// The edge a slab is given: its kind and its width (absent: the kind's, 5 cm a railing's, 15 cm a parapet's).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NewSlabEdge {
    pub kind: SlabEdgeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: Option<f64>,
}

/// One slab to make: with its outline `parts` drawn, or found from the walls round `seed`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NewSlab {
    /// Its outline drawn: one part or more, each an outer ring and its holes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub parts: Option<Vec<SpacePart>>,
    /// A point: its outline found from the walls round it, those of its level (every wall when it has none).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub seed: Option<Vec2>,
    /// Which faces of the walls round `seed` it runs along; absent: their outer faces.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub boundary: Option<SlabBoundary>,
    /// Every edge of its outline moved by this much, out of it when positive (a slab under the walls, a cantilever),
    /// in when negative; at most 10 m either way.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub edge_offset: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub role: Option<SlabRole>,
    /// Absent: 15 cm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub thickness: Option<f64>,
    /// Its top above its level's elevation (negative: below); absent: 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub offset: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub arch: Option<ArchInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub material: Option<String>,
    /// A railing or a parapet along its free sides; the document draws it along the sides no wall runs along.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub edge: Option<NewSlabEdge>,
    /// Left out of the quantities' totals.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub excluded: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attrs: Option<BTreeMap<String, String>>,
    /// Kaynağa bağlı (docs/adr/0771 §4): with `seed`, the slab keeps its walls: its outline found again round the
    /// point, by `boundary` and moved by `edgeOffset`, whenever they change. Absent or false: its outline is its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub bound: Option<bool>,
    /// A space by its persistent id: the slab's outline is the space's rings (moved by `edgeOffset`), bound to it; not
    /// with `parts` or `seed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub space: Option<String>,
}

/// Input of `cad.slabs.create` v1: slabs on a named layer, written as one undo step “Döşeme”.
///
/// Refusals, in this order: `no_objects`, `not_finite`, `invalid_slab`, `invalid_uid`, `entity_not_found`,
/// `not_a_space`, `level_not_found`, `slab_not_enclosed`, `slab_offset_failed`, `invalid_line_weight`, `invalid_revision`, `revision_conflict`, `layer_not_found`,
/// `not_a_layer`, `layer_locked`, `invalid_attribute`, `attribute_required`; on the desktop also `slots_exhausted`.
/// Warning: `layer_hidden`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SlabsCreate {
    pub layer_id: String,
    pub slabs: Vec<NewSlab>,
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

/// Output of `cad.slabs.create` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SlabsCreated {
    pub uids: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub ids: Vec<u32>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.slabs.create` would write (plan mode): the slabs with their outlines and their edges' runs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SlabsPlan {
    pub slabs: Vec<Entity>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// A slab's own fields to change; absent ones stay, a nullable one's null takes it away.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SlabChange {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub role: Option<SlabRole>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub thickness: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub offset: Option<f64>,
    #[serde(default, with = "crate::cad_properties::nullable", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<String>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub level: Option<Option<String>>,
    #[serde(default, with = "crate::cad_properties::nullable", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<ArchPhase>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<ArchPhase>"))]
    pub phase: Option<Option<ArchPhase>>,
    #[serde(default, with = "crate::cad_properties::nullable", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<String>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub material: Option<Option<String>>,
    /// Its edge's kind and width; null: no edge.
    #[serde(default, with = "crate::cad_properties::nullable", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<NewSlabEdge>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<NewSlabEdge>"))]
    pub edge: Option<Option<NewSlabEdge>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub excluded: Option<bool>,
}

/// What `cad.slabs.edit` does to the slabs (docs/adr/0722 §4).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SlabsOperation {
    /// Değiştir: the fields given, on every slab named.
    Set(SlabChange),
    /// Kenarı ötele: every edge of every ring of each slab moved by `distance`, out when positive.
    Offset {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        distance: f64,
    },
    /// Kes: the area drawn (`parts`) or found from the walls round `seed` (by `boundary`; absent: the room, to the
    /// walls' inner faces, docs/adr/0798 §3) taken out of each slab: a hole, a notch, a stairwell.
    Subtract {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        parts: Option<Vec<SpacePart>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        seed: Option<Vec2>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        boundary: Option<SlabBoundary>,
    },
    /// Kaynağa bağla (docs/adr/0771 §4): each slab's outline bound to the walls round `seed` (by `boundary`) or to the
    /// space `space` (its persistent id), moved by `offset`, and found now; a bound slab bound again.
    Bind {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        seed: Option<Vec2>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        boundary: Option<SlabBoundary>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        space: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        offset: Option<f64>,
    },
    /// Ayır: each bound slab's outline kept as it is now, its own (Bağımsız kontur).
    Detach,
    /// Ekle: the area drawn or found added to each slab.
    Add {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        parts: Option<Vec<SpacePart>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        seed: Option<Vec2>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        boundary: Option<SlabBoundary>,
    },
}

/// Input of `cad.slabs.edit` v1: slabs named by their persistent ids changed as one undo step “Döşemeyi düzenle”, all
/// or none.
///
/// Refusals: `no_entities`, `invalid_uid`, `entity_not_found`, `not_a_slab`, `nothing_to_set`, `not_finite`,
/// `invalid_slab`, `level_not_found`, `slab_not_enclosed`, `slab_offset_failed`, `slab_cut_missed`, `slab_bound`
/// (subtract or add on a bound slab), `slab_not_bound` (detach), `not_a_space`, `invalid_revision`,
/// `revision_conflict`, `layer_locked`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SlabsEdit {
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    pub operation: SlabsOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.slabs.edit` v1 (its plan's too: the slabs as they would be, nothing written).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SlabsEdited {
    pub slabs: Vec<Entity>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
