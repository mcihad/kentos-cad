//! The space commands of the product command catalog (docs/adr/0342 §4, plan/cad.md `CAD-ARCH-23`…`29`): spaces made
//! from the walls round a point or from a drawn boundary (`cad.spaces.create`), changed, merged, split and found again
//! (`cad.spaces.edit`), and the drawing's quantities (`cad.spaces.quantities`, read-only). Lengths are metres on the
//! wire, whatever unit the project shows (docs/adr/0165 §2).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::entity::{Entity, Vec2};
use crate::{ArchInfo, ArchPhase, SpaceColumns, SpaceFields, SpaceFinishes, SpaceMethod, SpacePart, SpaceScope};

pub const CAD_SPACES_CREATE: &str = "cad.spaces.create";
pub const CAD_SPACES_CREATE_VERSION: u32 = 1;
pub const CAD_SPACES_DISCOVER: &str = "cad.spaces.discover";
pub const CAD_SPACES_DISCOVER_VERSION: u32 = 1;
pub const CAD_SPACES_EDIT: &str = "cad.spaces.edit";
pub const CAD_SPACES_EDIT_VERSION: u32 = 1;
pub const CAD_SPACES_QUANTITIES: &str = "cad.spaces.quantities";
pub const CAD_SPACES_QUANTITIES_VERSION: u32 = 1;
/// The most spaces one call writes.
pub const MAX_SPACES: usize = 10_000;

/// Find bounded free rooms of the whole wall network or one semantic level.
/// Bounds filter each room's interior representative point, never cut walls.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SpacesDiscover {
    /// Explicit wall/column UUIDs; absent uses all sources eligible for the scope/level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sources: Option<Vec<String>>,

    pub layer_id: String,
    /// Source scope: project follows level (absent: all); unassigned requires absent level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub scope: Option<SpaceScope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub level: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub method: Option<SpaceMethod>,
    /// Physical column deduction; absent keeps legacy wall-only areas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub columns: Option<SpaceColumns>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub bounds: Option<[Vec2; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attrs: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Existing rooms are preserved. A partial manual overlap blocks the whole
/// write; the plan still shows the conflicting proposals and their sources.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SpaceProposal {
    pub space: SpaceFields,
    pub existing: Vec<String>,
    pub conflicts: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SpacesDiscovered {
    pub proposals: Vec<SpaceProposal>,
    pub revision: String,
}

/// One space to make: from the walls round `seed`, or with the boundary `parts` drawn.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NewSpace {
    /// Explicit wall/column UUIDs; absent uses all sources eligible for the scope/level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sources: Option<Vec<String>>,

    /// Persistent source scope for a seeded room; drawn rooms use project only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub scope: Option<SpaceScope>,
    /// A point inside it: its boundary is found from the walls round it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub seed: Option<Vec2>,
    /// Its boundary drawn, when no seed is given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub parts: Option<Vec<SpacePart>>,
    /// What its rings stand for; absent: the project's (`settings.arch.spaceMethod`, else net).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub method: Option<SpaceMethod>,
    /// Physical column deduction; absent keeps legacy wall-only areas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub columns: Option<SpaceColumns>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub usage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub arch: Option<ArchInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub finishes: Option<SpaceFinishes>,
    /// A void: a shaft, a lift shaft, an opening in the floor (docs/adr/0708); absent: a room.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub void: Option<bool>,
    /// The building program's group it joins, by its id (docs/adr/0773); absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attrs: Option<BTreeMap<String, String>>,
}

/// Input of `cad.spaces.create` v1: spaces on a named layer, written as one undo step “Mahal”.
///
/// Refusals, in this order: `no_objects`, `not_finite`, `space_not_enclosed`, `invalid_space`, `level_not_found`,
/// `invalid_line_weight`, `invalid_revision`, `revision_conflict`, `layer_not_found`, `not_a_layer`, `layer_locked`,
/// `invalid_attribute`, `attribute_required`; on the desktop also `slots_exhausted`. Warning: `layer_hidden`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SpacesCreate {
    pub layer_id: String,
    pub spaces: Vec<NewSpace>,
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

/// Output of `cad.spaces.create` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SpacesCreated {
    pub uids: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub ids: Vec<u32>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.spaces.create` would write (plan mode).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SpacesPlan {
    pub spaces: Vec<Entity>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// A space's own fields to change; absent ones stay, a nullable one's null takes it away.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SpaceChange {
    /// Explicit UUIDs; null clears the list and restores scope/level search.
    #[serde(default, with = "crate::cad_properties::nullable", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<Vec<String>>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<Vec<String>>"))]
    pub sources: Option<Option<Vec<String>>>,
    /// Source scope to keep for later wall/column updates; unassigned requires clearing level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub scope: Option<SpaceScope>,
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<String>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub number: Option<Option<String>>,
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<String>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub name: Option<Option<String>>,
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<String>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub usage: Option<Option<String>>,
    /// The method its boundary is found again with (a space from the walls) or its rings stand for (a drawn one).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub method: Option<SpaceMethod>,
    /// Physical column deduction; absent keeps legacy wall-only areas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub columns: Option<SpaceColumns>,
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<String>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub level: Option<Option<String>>,
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<ArchPhase>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<ArchPhase>"))]
    pub phase: Option<Option<ArchPhase>>,
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<f64>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub height: Option<Option<f64>>,
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<String>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub floor: Option<Option<String>>,
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<String>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub wall: Option<Option<String>>,
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<String>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub ceiling: Option<Option<String>>,
    /// A void or a room again (docs/adr/0708).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub void: Option<bool>,
    /// The building program's group, by its id (docs/adr/0773); null: out of any.
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<String>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub unit: Option<Option<String>>,
}

/// What `cad.spaces.edit` does to the spaces (docs/adr/0342 §4).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SpacesOperation {
    /// Değiştir: the fields given, on every space named.
    Set(SpaceChange),
    /// Birleştir: the spaces named one, its boundary drawn; the id of `keep` goes on (absent: the first's), with its
    /// number unless `number` gives another (null: none), its finishes and its group. The others' numbers, finishes and
    /// tags go with them: the answer's `decision` says so before anything is written (docs/adr/0773 §4).
    Merge {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        keep: Option<String>,
        #[serde(
            default,
            with = "crate::cad_properties::nullable",
            skip_serializing_if = "Option::is_none"
        )]
        #[cfg_attr(feature = "ts", ts(as = "Option<Option<String>>", optional))]
        #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
        number: Option<Option<String>>,
    },
    /// Böl: each space named cut along the line `line` (two points or more). The part holding `keep` keeps its id
    /// (absent: the largest); the new parts take `numbers` in their order by size (absent: none, so no number is
    /// written twice), its group, and its finishes when `finishes` (absent: true). The answer's `decision` says each
    /// part's id, number, finishes and the fixtures standing in it (docs/adr/0773 §4).
    Split {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        line: Vec<Vec2>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        keep: Option<Vec2>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        numbers: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        finishes: Option<bool>,
    },
    /// Duvarlardan bul: each space's boundary found again from the walls round `seed` (absent: its own seed, else
    /// its pole of inaccessibility).
    Rebound {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        seed: Option<Vec2>,
    },
}

/// Input of `cad.spaces.edit` v1: spaces named by their persistent ids changed as one undo step “Mahali düzenle”, all
/// or none.
///
/// Refusals: `no_entities`, `invalid_uid`, `entity_not_found`, `not_a_space`, `nothing_to_set`, `not_finite`,
/// `invalid_space`, `level_not_found`, `too_few_objects`, `spaces_not_touching`, `split_missed`, `space_not_enclosed`,
/// `invalid_revision`, `revision_conflict`, `layer_locked`; on the desktop also `slots_exhausted`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SpacesEdit {
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    pub operation: SpacesOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.spaces.edit` v1 (its plan's too: the spaces as they would be, nothing written): the spaces kept
/// (their ids kept), the new ones, the ids gone.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SpacesEdited {
    pub spaces: Vec<Entity>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<Entity>>", optional))]
    pub added: Vec<Entity>,
    /// The ids kept, in their order.
    pub kept: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub removed: Vec<String>,
    /// A merge's or a split's decision, one per space named (docs/adr/0773 §4).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<SpaceDecision>>", optional))]
    pub decisions: Vec<SpaceDecision>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// How the space whose id goes on was chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SpaceKeptBy {
    /// Named (a merge's `keep`, a split's `keep` point).
    Given,
    /// A merge's first space.
    First,
    /// A split's largest part.
    Largest,
}

/// A part a split makes, or a space a merge takes in.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SpacePieceDecision {
    /// A merged space's id; a split's new part has none until written (the answer's `added`, in order).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub number: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    /// The finishes it takes (a split's part) or that go with it (a merged space's).
    #[serde(default, skip_serializing_if = "SpaceFinishes::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<SpaceFinishes>", optional))]
    pub finishes: SpaceFinishes,
    /// Its tags: a merged space's lose their space; none for a split's part.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub tags: Vec<String>,
    /// The fixtures (block inserts) standing in it, by their ids.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub fixtures: Vec<String>,
}

/// What a merge or a split does with a space's identity and its work (docs/adr/0773 §4), said before it is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SpaceDecision {
    /// The id that goes on.
    pub kept: String,
    pub by: SpaceKeptBy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub number: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    /// Its group, kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub unit: Option<String>,
    /// The fixtures standing in it after.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub fixtures: Vec<String>,
    /// A split's new parts, in the answer's `added` order; a merge's spaces taken in, in their order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<SpacePieceDecision>>", optional))]
    pub pieces: Vec<SpacePieceDecision>,
}

/// Input of `cad.spaces.quantities` v1 (read-only): the drawing's spaces, walls and openings measured, with its slabs,
/// ramps and stairs (docs/adr/0722 §5, 0723 §3, 0724 §5), those of the levels, phases, usages and layers given (each
/// absent: every one; a usage filter leaves out all but the spaces).
///
/// Refusals: `level_not_found`, `layer_not_found`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SpacesQuantities {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub levels: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub phases: Option<Vec<ArchPhase>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub usages: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layers: Option<Vec<String>>,
}

/// A space measured.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SpaceRow {
    /// Explicit wall/column UUIDs; absent uses all sources eligible for the scope/level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sources: Option<Vec<String>>,

    pub uid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub usage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub level: Option<String>,
    pub method: SpaceMethod,
    /// Persistent column policy; omitted for legacy/ignore rows.
    #[serde(default, skip_serializing_if = "SpaceColumns::is_ignore")]
    #[cfg_attr(feature = "ts", ts(as = "Option<SpaceColumns>", optional))]
    pub columns: SpaceColumns,
    #[serde(default, skip_serializing_if = "SpaceScope::is_project")]
    #[cfg_attr(feature = "ts", ts(as = "Option<SpaceScope>", optional))]
    pub scope: SpaceScope,
    /// A stale boundary's reason; area/perimeter/volume then describe the last
    /// valid geometry, included in totals. Absent for every valid boundary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub boundary_break: Option<crate::SpaceBreak>,
    /// A void (docs/adr/0708): its area is summed in `voidArea`, not in `spaceArea`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub void: bool,
    /// Its rings' area, m².
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub perimeter: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub volume: Option<f64>,
    /// A room's baseboard (süpürgelik, docs/adr/0722 §5): its rings' length less the widths of the passages (doors,
    /// gaps to the floor) its sides run along, metres; none for a void.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub baseboard: Option<f64>,
    /// A space from the walls: its area by each method, m².
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub by_method: Option<BTreeMap<String, f64>>,
    /// The rule its numbers come from.
    pub rule: String,
}

/// A wall measured.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WallRow {
    pub uid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub style: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub level: Option<String>,
    /// Along its base line, metres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub length: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub height: Option<f64>,
    /// length × height, m²; none without a height.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub gross_area: Option<f64>,
    /// Its openings' areas, m² (those with a height).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub openings_area: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub net_area: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub volume: Option<f64>,
    /// Its openings' ids.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub openings: Vec<String>,
    pub rule: String,
}

/// An opening measured.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct OpeningRow {
    pub uid: String,
    pub form: crate::OpeningForm,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub style: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub wall: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: f64,
    /// Head less sill (sill absent: 0); none without a head.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: Option<f64>,
    /// A niche's or flue's depth across its wall (docs/adr/0719 §4); none for one that goes through, which alone is
    /// taken off its wall's net face.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub depth: Option<f64>,
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

/// A slab measured (docs/adr/0722 §5).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SlabRow {
    pub uid: String,
    pub role: crate::SlabRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub level: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub material: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub thickness: f64,
    /// Its top above its level's elevation.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub offset: f64,
    /// Its top's elevation: its level's elevation and its offset; none when its level has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub top: Option<f64>,
    /// Its rings' area, its holes taken off, m².
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    /// The area the voids of its level (shafts, docs/adr/0708) take out of it, m².
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub void_area: f64,
    /// `area` less `voidArea`, m².
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub net_area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub perimeter: f64,
    /// `netArea` × thickness, m³.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub volume: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub edge: Option<crate::SlabEdgeKind>,
    /// Its edge's runs' length, metres.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub edge_length: f64,
    /// Listed, not summed.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub excluded: bool,
    pub rule: String,
}

/// A ramp measured (docs/adr/0723 §3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RampRow {
    pub uid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub level: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: f64,
    /// How much higher its end is than its start (below zero going down).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub rise: f64,
    /// Its slope as drawn, per cent: its rise's size over its sloped length; none when no sloped run is left.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "percent")))]
    pub slope: Option<f64>,
    /// Its sloped runs' length: its centre line less its landings, metres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub run: f64,
    /// Its centre line's length, metres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub length: f64,
    #[serde(default, skip_serializing_if = "is_zero_count")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub landings: u32,
    /// Its band's plan area, m².
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    /// Its start's elevation and its end's.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub base: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub top: f64,
    /// Its start over its level's when bound to it (docs/adr/0723 §2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub base_offset: Option<f64>,
    /// Its railings' lines' length, metres.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub railing_length: f64,
    pub rule: String,
}

fn is_zero_count(n: &u32) -> bool {
    *n == 0
}

/// A stair measured (docs/adr/0724 §5).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StairRow {
    pub uid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub level: Option<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub risers: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub riser_height: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tread: f64,
    /// Its whole rise: its risers × their height.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub height: f64,
    /// Its walking line's length, its landings in, metres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub length: f64,
    #[serde(default, skip_serializing_if = "is_zero_count")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub landings: u32,
    /// Its band's plan area, m².
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub area: f64,
    /// Its railings' lines' length, metres.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub railing_length: f64,
    /// A curved stair's walking line's radius and its tread at its inner side.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub radius: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub inner_tread: Option<f64>,
    /// Its height follows its level's (docs/adr/0724 §1).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub height_from_level: bool,
    pub rule: String,
}

/// A floor finish's change at a passage (docs/adr/0722 §5): the rooms on its two sides, their floors.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TransitionRow {
    /// The passage's id.
    pub opening: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub number: Option<String>,
    /// The strip's length: the passage's width, metres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub width: f64,
    /// The rooms on its left and its right (along its wall's direction).
    pub spaces: [String; 2],
    /// Their floor finishes, in the same order.
    pub floors: [String; 2],
}

/// The rows' sums.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct QuantityTotals {
    /// The rooms' area, voids left out (docs/adr/0708).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub space_area: f64,
    /// The voids' area: shafts, lift shafts, openings in the floor.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub void_area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub wall_length: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub wall_net_area: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub openings: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub opening_area: f64,
    /// The rooms' baseboards, metres (docs/adr/0722 §5).
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub baseboard_length: f64,
    /// The slabs' net area and volume, those left out not summed.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub slab_area: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "volume")))]
    pub slab_volume: f64,
    /// The railings' and parapets' length along the slabs' free sides, metres.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub railing_length: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub parapet_length: f64,
    /// The ramps' plan area and their railings' length (docs/adr/0723 §3).
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub ramp_area: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub ramp_railing_length: f64,
    /// The stairs' plan area, their railings' length and their risers (docs/adr/0724 §5).
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "area")))]
    pub stair_area: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub stair_railing_length: f64,
    #[serde(default, skip_serializing_if = "is_zero_count")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub stair_risers: u32,
}

/// Output of `cad.spaces.quantities` v1 (its plan's too).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SpacesQuantified {
    pub spaces: Vec<SpaceRow>,
    pub walls: Vec<WallRow>,
    pub openings: Vec<OpeningRow>,
    /// The slabs (docs/adr/0722 §5).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<SlabRow>>", optional))]
    pub slabs: Vec<SlabRow>,
    /// The floor finishes' changes at the passages.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<TransitionRow>>", optional))]
    pub transitions: Vec<TransitionRow>,
    /// The ramps (docs/adr/0723 §3).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<RampRow>>", optional))]
    pub ramps: Vec<RampRow>,
    /// The stairs (docs/adr/0724 §5).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<StairRow>>", optional))]
    pub stairs: Vec<StairRow>,
    pub totals: QuantityTotals,
}
