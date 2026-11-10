//! Kesit çıkar (docs/adr/8042; M42, ADV-19): a section mark's (docs/adr/0345) section drawn from what the model holds
//! into a block of its own, placed in the drawing (`cad.sections.edit`: derive, refresh), and the sections read
//! with whether they still are what the model would draw (`cad.sections.query`). The block's objects are on EK-4's
//! material layers (EK-4 4.14.3: the cut by its material, its hatch on `-T`, the seen by its depth `-1` … `-4`); what
//! the model does not hold (a roof, the natural ground, a ramp) is listed (`leftOut`), not drawn.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::{Entity, Vec2};

pub const CAD_SECTIONS_EDIT: &str = "cad.sections.edit";
pub const CAD_SECTIONS_EDIT_VERSION: u32 = 1;
pub const CAD_SECTIONS_QUERY: &str = "cad.sections.query";
pub const CAD_SECTIONS_QUERY_VERSION: u32 = 1;
/// The undo steps' names.
pub const SECTION_DERIVE_STEP: &str = "Kesit çıkar";
pub const SECTION_REFRESH_STEP: &str = "Kesiti yeniden çiz";

/// What `cad.sections.edit` does.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum BuildingSectionsOperation {
    /// Kesit çıkar: the section mark `mark`'s section (its first two points, looking where its arrows look) drawn into
    /// its block and placed with its line's left end at ±0.00 at `at` (absent: 5 m right of the drawing's objects, at
    /// their bottom).
    Derive {
        #[cfg_attr(feature = "schema", schemars(extend("x-format" = "uuid")))]
        mark: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        at: Option<Vec2>,
    },
    /// Yeniden çiz: the section's block drawn again from the model as it is now; every placement of it shows the new
    /// drawing where it is.
    Refresh {
        #[cfg_attr(feature = "schema", schemars(extend("x-format" = "uuid")))]
        mark: String,
    },
}

/// Input of `cad.sections.edit` v1.
///
/// Refusals: `not_finite`, `invalid_uid`, `entity_not_found`, `not_a_section_mark`,
/// `section_jogged`, `section_exists`, `section_not_found`, `layer_refused`, `nothing_to_set`, `invalid_revision`,
/// `revision_conflict`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BuildingSectionsEdit {
    pub operation: BuildingSectionsOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<String>,
}

/// Why something of the model is not in the section, or only in part.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum BuildingSectionLeft {
    /// The roof: the model has none yet (ADV-20).
    Roof,
    /// The natural ground: the model has no survey of it (EK-4 4.30); the graded ground is drawn at ±0.00.
    NaturalGround,
    /// A curved wall, slab or stair: drawn by its chords, a curved stair not at all.
    Curved,
    /// A ramp: not drawn yet.
    Ramp,
    /// A wall or a column with no height.
    NoHeight,
    /// An opening without a head: its gap reaches its wall's top.
    HeadUnknown,
    /// A stair with no slab to take its waist from: drawn solid down to its floor.
    StairWaist,
    /// A niche or a flue: inside its wall, not cut.
    Recess,
}

/// Something of the model the section leaves out or draws in part, and its object when one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BuildingSectionLeftOut {
    pub why: BuildingSectionLeft,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uid: Option<String>,
}

/// Output of `cad.sections.edit` (and its plan): the mark, the section's name ("A-A kesiti"), its block, its
/// placements' ids, the block's objects (after; a plan's are what it would write), whether a refresh changed them and
/// what was left out.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BuildingSectionsEdited {
    pub mark: String,
    pub name: String,
    pub block: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub inserts: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<Entity>>", optional))]
    pub drawn: Vec<Entity>,
    pub changed: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "ts",
        ts(as = "Option<Vec<BuildingSectionLeftOut>>", optional)
    )]
    pub left_out: Vec<BuildingSectionLeftOut>,
    pub revision: String,
}

/// Input of `cad.sections.query` v1 (read-only): every section mark's section, the one of `mark`, or the one whose
/// placement `uid` is.
///
/// Refusals: `invalid_uid`, `entity_not_found`, `not_a_section_mark`, `section_jogged`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BuildingSectionsQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub mark: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub uid: Option<String>,
}

/// Whether a section is what the model would draw now (BricsCAD marks a drawing out of sync red, one never made orange).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum BuildingSectionState {
    /// Güncel: drawn again it would be the same.
    Current,
    /// Eski: the model changed what it draws (a door moved, a wall's height); Yeniden çiz brings it up to date.
    Stale,
    /// Çıkarılmadı: the mark has no section yet.
    NotDrawn,
    /// Kaynağı yok: the block is the section of a mark the drawing no longer has.
    Gone,
}

/// A section as `cad.sections.query` reads it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BuildingSectionRow {
    /// Its mark's persistent id; none when the mark is gone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub mark: Option<String>,
    /// "A-A kesiti".
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub block: Option<String>,
    pub state: BuildingSectionState,
    /// Its placements in the drawing.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub inserts: u32,
}

/// Output of `cad.sections.query`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BuildingSectionsQueried {
    pub sections: Vec<BuildingSectionRow>,
    pub revision: String,
}
