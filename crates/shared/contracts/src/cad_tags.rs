//! The semantic commands of tags and schedules (docs/adr/0344 §4, plan/cad.md `CAD-ARCH-20`–`22`, `39`): tags put on
//! objects (`cad.tags.create`), the tags and numbers checked (`cad.tags.check`, read-only), a schedule placed
//! (`cad.schedules.create`) and edited (`cad.schedules.edit`: a cell written to its objects, or its spec changed). Each
//! that writes is one undo step.
//!
//! Lengths are metres on the wire, whatever unit the project shows (docs/adr/0165 §2); a schedule's cell value is
//! words as the cell shows them, a length in the project's unit.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::entity::{Entity, Vec2};
use crate::{ScheduleColumn, ScheduleFilter, ScheduleOf, ScheduleSort, TagShape};

pub const CAD_TAGS_CREATE: &str = "cad.tags.create";
pub const CAD_TAGS_CREATE_VERSION: u32 = 1;
pub const CAD_TAGS_CHECK: &str = "cad.tags.check";
pub const CAD_TAGS_CHECK_VERSION: u32 = 1;
pub const CAD_TAGS_EDIT: &str = "cad.tags.edit";
pub const CAD_TAGS_EDIT_VERSION: u32 = 1;
pub const CAD_SCHEDULES_CREATE: &str = "cad.schedules.create";
pub const CAD_SCHEDULES_CREATE_VERSION: u32 = 1;
pub const CAD_SCHEDULES_EDIT: &str = "cad.schedules.edit";
pub const CAD_SCHEDULES_EDIT_VERSION: u32 = 1;
/// The most tags one call writes.
pub const MAX_TAGS: usize = 10_000;

/// The kinds of object a tag is put on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TaggedKind {
    Opening,
    Space,
    Column,
    Grid,
    Wall,
    Stair,
    Ramp,
    Insert,
}

impl TaggedKind {
    pub const ALL: [TaggedKind; 8] = [
        TaggedKind::Opening,
        TaggedKind::Space,
        TaggedKind::Column,
        TaggedKind::Grid,
        TaggedKind::Wall,
        TaggedKind::Stair,
        TaggedKind::Ramp,
        TaggedKind::Insert,
    ];

    /// The object kind, as the contract names it.
    pub fn kind(self) -> &'static str {
        match self {
            TaggedKind::Opening => "opening",
            TaggedKind::Space => "space",
            TaggedKind::Column => "column",
            TaggedKind::Grid => "grid",
            TaggedKind::Wall => "wall",
            TaggedKind::Stair => "stair",
            TaggedKind::Ramp => "ramp",
            TaggedKind::Insert => "insert",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            TaggedKind::Opening => "Açıklıklar",
            TaggedKind::Space => "Mahaller",
            TaggedKind::Column => "Kolonlar",
            TaggedKind::Grid => "Akslar",
            TaggedKind::Wall => "Duvarlar",
            TaggedKind::Stair => "Merdivenler",
            TaggedKind::Ramp => "Rampalar",
            TaggedKind::Insert => "Bloklar",
        }
    }

    pub fn of_kind(kind: &str) -> Option<TaggedKind> {
        TaggedKind::ALL.into_iter().find(|k| k.kind() == kind)
    }
}

/// One tag to put.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NewTag {
    /// The object it tags, by its persistent id.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub of: String,
    /// Its middle; absent: its object's point (a space's), else 4 times its words' height above it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Option<Vec2>,
    /// Its template; absent: its object's kind's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub content: Option<String>,
    /// Its frame; absent: its object's kind's (an opening's hexagon, a space's rectangle, else a circle).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub shape: Option<TagShape>,
    /// Its words' height on paper, millimetres; absent: 2.5.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub size: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub leader: Option<bool>,
}

/// Tags on every shown object of the kinds given that has none, each at its object's point as a `NewTag` without `at`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TagAll {
    pub kinds: Vec<TaggedKind>,
    /// Their layers' ids; absent: every layer shown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layers: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub shape: Option<TagShape>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub size: Option<f64>,
}

/// Input of `cad.tags.create` v1 (“Etiket”): `tags`, then those of `all`, on a named layer, as one undo step.
///
/// Refusals, in this order: `no_objects`, `invalid_uid`, `not_finite`, `entity_not_found`, `not_taggable`,
/// `invalid_tag`, `invalid_line_weight`, `invalid_revision`, `revision_conflict`, `layer_not_found`, `not_a_layer`,
/// `layer_locked`; on the desktop also `slots_exhausted`. Warning: `layer_hidden`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TagsCreate {
    pub layer_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<NewTag>>", optional))]
    pub tags: Vec<NewTag>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub all: Option<TagAll>,
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

/// Output of `cad.tags.create` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TagsCreated {
    pub uids: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub ids: Vec<u32>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.tags.create` would write (plan mode): the tags as they would be stored, their words written, `id` 0.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TagsPlan {
    pub tags: Vec<Entity>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// The fields of tags to change; absent ones stay, a nullable one's null takes it away (its kind's template, 2.5 mm).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TagChange {
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<String>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub content: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub shape: Option<TagShape>,
    #[serde(
        default,
        with = "crate::cad_properties::nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<f64>"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub size: Option<Option<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub leader: Option<bool>,
}

/// Input of `cad.tags.edit` v1 (“Etiketi düzenle”): the tags' template, frame, words' height or leader changed, their
/// words written again from their objects; one undo step, all or none.
///
/// Refusals, in this order: `no_entities`, `invalid_uid`, `entity_not_found`, `not_a_tag`, `nothing_to_set`,
/// `not_finite`, `invalid_tag`, `invalid_revision`, `revision_conflict`, `layer_locked`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TagsEdit {
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    pub change: TagChange,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.tags.edit` v1 (its plan's too: the tags as they would be, nothing written).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TagsEdited {
    pub tags: Vec<Entity>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Input of `cad.tags.check` v1 (“Etiketleri denetle”), read-only: the openings and spaces of the layers given (absent:
/// every layer) and every tag checked.
///
/// Refusals: `layer_not_found`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TagsCheck {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layers: Option<Vec<String>>,
}

/// One thing `cad.tags.check` found.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TagFinding {
    /// `tag_duplicate_number`, `tag_missing_type`, `tag_host_broken`, `tag_target_gone` or `tag_untagged`.
    pub code: String,
    pub message: String,
    /// The objects, in the drawing's order.
    pub uids: Vec<String>,
    /// Where to look: the first object's point.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Vec2,
}

/// Output of `cad.tags.check` v1 (its plan's too).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TagsChecked {
    /// By code (in the order above), then by their first object's place in the drawing.
    pub findings: Vec<TagFinding>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Input of `cad.schedules.create` v1 (“Çizelge”): a schedule of `of` placed with its top left corner at `at`, its
/// rows written from the drawing, as one undo step.
///
/// Refusals, in this order: `not_finite`, `invalid_schedule`, `level_not_found`, `invalid_line_weight`,
/// `invalid_revision`, `revision_conflict`, `layer_not_found`, `not_a_layer`, `layer_locked`; on the desktop also
/// `slots_exhausted`. Warning: `layer_hidden`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SchedulesCreate {
    pub layer_id: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Vec2,
    pub of: ScheduleOf,
    /// Absent: the kind's columns.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub columns: Option<Vec<ScheduleColumn>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub filter: Option<ScheduleFilter>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ScheduleSort>>", optional))]
    pub sort: Vec<ScheduleSort>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub group: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub total: Option<bool>,
    /// Its words' height, metres; absent: the project's table height at its plot scale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub height: Option<f64>,
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

/// Output of `cad.schedules.create` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SchedulesCreated {
    pub uid: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub id: u32,
    /// Its data rows (neither its heading nor its total).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rows: u32,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.schedules.create` would write (plan mode): the table as it would be stored, its `id` 0.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SchedulesPlan {
    pub table: Entity,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// A schedule's spec changed: each field given replaces its own (`filter: null` drops the filter).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ScheduleChange {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub columns: Option<Vec<ScheduleColumn>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::cad_properties::nullable"
    )]
    #[cfg_attr(feature = "ts", ts(as = "Option<Option<ScheduleFilter>>", optional))]
    #[cfg_attr(feature = "schema", schemars(with = "Option<ScheduleFilter>"))]
    pub filter: Option<Option<ScheduleFilter>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sort: Option<Vec<ScheduleSort>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub group: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub total: Option<bool>,
}

/// What `cad.schedules.edit` does.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ScheduleOperation {
    /// The cell of the data row `row` (the table's own row index: 0 is the heading) and the column `column`: `value`
    /// written to each of the row's objects. Empty words clear an optional field.
    Cell {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        row: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        column: u32,
        value: String,
    },
    /// Its columns, filter, sort, grouping or total changed; its rows written again.
    Set(Box<ScheduleChange>),
}

/// Input of `cad.schedules.edit` v1 (“Çizelgeyi düzenle”), one undo step.
///
/// Refusals, in this order: `invalid_uid`, `entity_not_found`, `not_schedule`, `cell_out_of_range`, `cell_computed`,
/// `schedule_group_number`, `invalid_cell_value`, `invalid_schedule`, `level_not_found`, `invalid_revision`,
/// `revision_conflict`, `layer_locked`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SchedulesEdit {
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    pub operation: ScheduleOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.schedules.edit` v1 (its plan's too: as it would be, nothing written).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SchedulesEdited {
    /// The schedule as written again.
    pub table: Entity,
    /// The objects a cell was written to, in the drawing's order.
    pub edited: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
