//! Denetle's semantic command of the product command catalog (docs/adr/0306; AutoCAD's AUDIT and PURGE): what the
//! drawing carries broken, found and on asking repaired, and the named resources nothing uses, reported and the
//! styles among them on asking removed.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

/// Audits the drawing.
pub const CAD_DRAWING_AUDIT: &str = "cad.drawing.audit";
pub const CAD_DRAWING_AUDIT_VERSION: u32 = 1;

/// What is broken.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AuditCode {
    /// A linked text whose object is gone.
    BrokenLink,
    /// An associative hatch one of whose boundary objects is gone.
    BrokenHatch,
    /// A hatch leaving open a text that is gone (docs/adr/0332 §2).
    MissingCutout,
    /// A hatch leaving open its own linked label: a cycle (docs/adr/0332 §2).
    HatchTextCycle,
    /// An associative dimension's live reference to an object that is gone.
    BrokenDimension,
    /// A text or a table naming a text style the project does not have.
    MissingTextStyle,
    /// A dimension naming a dimension style the project does not have.
    MissingDimensionStyle,
}

/// The named resources Denetle removes when nothing uses them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AuditPurge {
    TextStyles,
    DimensionStyles,
}

/// A named resource's kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ResourceKind {
    Layer,
    Group,
    Block,
    TextStyle,
    DimensionStyle,
}

/// Input of `cad.drawing.audit` v1. Finds what the drawing carries broken (`AuditCode`) and the named resources
/// nothing uses (empty layers and groups and unplaced block definitions as `cad.layers.edit`'s purge finds them, not
/// locked ones; the text and dimension styles no object of the drawing or of a block definition names). With `fix`
/// each broken object is repaired (a linked text loses its link, a hatch its association, a dimension's reference
/// breaks as `object_gone`, an unknown style becomes Standart, the look kept) as one undo step “Denetle”; an object on
/// a locked layer is left broken with a warning (`layer_locked`). `purge` removes the unused styles of the kinds named
/// (the styles' table is a project setting: no undo step). Nothing asked: nothing is written.
///
/// Refusals (`CommandError.code`): `invalid_revision`, `revision_conflict` (status `conflict`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DrawingAudit {
    /// Repair what is broken.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub fix: bool,
    /// The unused styles to remove.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<AuditPurge>>", optional))]
    pub purge: Vec<AuditPurge>,
    /// The document revision the input was prepared against, as decimal text. When given and the document is no
    /// longer at it, nothing is written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// A broken object.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AuditFinding {
    pub code: AuditCode,
    /// The object's persistent id.
    pub uid: String,
    /// What is broken and what the repair does, in Turkish.
    pub message: String,
    /// Whether it was repaired.
    pub fixed: bool,
}

/// A named resource nothing uses.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct UnusedResource {
    pub kind: ResourceKind,
    /// Its id (a layer's or group's node id, a definition's or a style's id).
    pub id: String,
    /// Its name; a layer's or group's with its groups (`Eski / Yollar`).
    pub name: String,
    /// Whether it was removed.
    pub purged: bool,
}

/// Output of `cad.drawing.audit` v1 (and its plan, what it would write).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DrawingAudited {
    /// What was found broken, object by object in the drawing's order.
    pub findings: Vec<AuditFinding>,
    /// The named resources nothing uses: layers, groups, blocks, text styles, dimension styles.
    pub unused: Vec<UnusedResource>,
    /// The objects repaired: their persistent ids, in the drawing's order.
    pub changed: Vec<String>,
    /// The document's revision after the write, as decimal text (the same when nothing was written).
    pub revision: String,
}
