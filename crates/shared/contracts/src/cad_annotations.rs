//! The annotative scales command of the product command catalog (docs/adr/0323
//! §4): texts, dimensions and leaders made annotative, their scales added and
//! removed, a scale's own place, direction, line or value set and reset, for
//! objects named by their persistent ids, as one undo step.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::{TextShift, Vec2};

/// Makes annotations annotative and edits their scales.
pub const CAD_ANNOTATIONS_SCALES: &str = "cad.annotations.scales";
pub const CAD_ANNOTATIONS_SCALES_VERSION: u32 = 1;

/// What the command does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AnnotativeOperation {
    /// Ölçekli yap: the objects' own geometry is their representation at the project's plot scale;
    /// `scales` are added. One annotative already only takes the scales. “Ölçekli yap”.
    Make,
    /// Ölçekliliği kaldır: one height again, the object's own; its other scales are dropped. “Ölçekliliği kaldır”.
    Unmake,
    /// Ölçek ekle: the objects take part in `scales` too, as their own scale draws them, scaled. “Ölçek ekle”.
    Add,
    /// Ölçek kaldır: the objects are not drawn at `scales` any more; their own scale stays (`own_scale`). “Ölçek kaldır”.
    Remove,
    /// Ölçekteki yeri: at `scale`, the given values (`point`, `rotation`; a dimension's `offset`,
    /// `textShift`) set apart from the object's own. “Ölçekteki yeri”.
    Set,
    /// Ölçeği sıfırla: at `scales` (none: every other scale) nothing set apart. “Ölçeği sıfırla”.
    Reset,
}

/// Input of `cad.annotations.scales` v1 (docs/adr/0323 §4). The command reads
/// no selection (TODOS.md CMD-07): the Properties panel and the tool give the ids.
///
/// Objects that are not texts, dimensions or leaders (or are texts along a
/// curve, or linked to an object) are left out with the warning
/// `not_annotative`, as are those not annotative for every other operation
/// but `make`; when none is left, it is the refusal. What is on a locked layer
/// is left out with the warning `layer_locked` (docs/adr/0278). An object that
/// would not change is not written.
///
/// Refusals (`CommandError.code`), checked in this order: `no_entities`,
/// `invalid_uid`, `no_scales` (`add` and `remove` without `scales`), `no_scale`
/// (`set` without `scale`), `invalid_scale` (a scale not over 0, not finite, or
/// past 1:1 000 000), `no_value` (`set` without a value), `invalid_revision`,
/// `revision_conflict` (status `conflict`), `entity_not_found`, `layer_locked`,
/// `not_annotative`, `own_scale` (removing or setting an object's own scale),
/// `scale_not_taken` (`set` at a scale an object does not take part in),
/// `invalid_annotative` (a value its kind does not have: a text's offset, a
/// dimension's point; more than 32 scales).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AnnotationsScales {
    pub operation: AnnotativeOperation,
    /// The objects' persistent ids, at least one; a repeated one counts once.
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    /// `make`, `add`, `remove`, `reset`: the scales, 1:N's N.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
    pub scales: Vec<f64>,
    /// `set`: the scale whose values are given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
    pub scale: Option<f64>,
    /// `set`: a text's point, a leader's landing start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub point: Option<Vec2>,
    /// `set`: a text's or a note's direction, degrees counter-clockwise from east.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub rotation: Option<f64>,
    /// `set`: a dimension's line offset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub offset: Option<f64>,
    /// `set`: a dimension's value moved from its place.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub text_shift: Option<TextShift>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.annotations.scales` v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AnnotationsScaled {
    /// The objects written, in the input's order.
    pub changed: Vec<String>,
    /// Their slots in the open document (`Entity.id`), in the same order.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub ids: Vec<u32>,
    /// The document's revision after the write (the same when nothing changed; in a plan, now).
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.annotations.scales` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AnnotationsScalesPlan {
    /// The objects that would change, each as execute would write it, with its own slot (`id`).
    pub changed: Vec<crate::entity::Entity>,
    /// The document revision the plan was made against.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
