//! What changing a look's source would reach (docs/adr/0314 §4; plan/cad.md CAD-STYLE-15): the read-only
//! `cad.styles.impact` names, for a layer, a text or dimension style or a line type, the objects that take it
//! and those that keep values of their own (and which), those a locked layer keeps as they are, the block pieces that
//! take it and the inserts that show them, the layer states that keep a look of their own for it, the sheet maps
//! that draw the layer otherwise, and the external references (none: KentOS keeps no xref). The Katmanlar, Yazı ve
//! ölçü stilleri and Çizgi tipleri windows say it before a change.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;

pub const CAD_STYLES_IMPACT: &str = "cad.styles.impact";
pub const CAD_STYLES_IMPACT_VERSION: u32 = 1;

/// The most objects named by id in an answer; the counts are whole.
pub const IMPACT_UIDS: usize = 1000;

/// A look's source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ImpactTarget {
    /// A layer by its id: its colour, line type, pattern, weight and transparency.
    Layer,
    /// A text style by its id.
    TextStyle,
    /// A dimension style by its id.
    DimensionStyle,
    /// A project line type by its name (docs/adr/0313).
    LineType,
}

/// Input of `cad.styles.impact` v1. Refusals: `empty_name` (no `id`), `layer_not_found`,
/// `style_not_found`, `line_type_not_found`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StylesImpact {
    pub target: ImpactTarget,
    /// The layer's or the style's id, the line type's name.
    pub id: String,
}

/// Objects keeping one property of their own.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ImpactOwn {
    /// The property, as the contract names it (`color`, `lineType`, `font` …).
    pub property: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub count: u32,
}

/// Output of `cad.styles.impact` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StyleImpact {
    /// The source's name as people see it.
    pub name: String,
    /// The drawing's objects that take at least one of its values.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub follow: u32,
    /// Their persistent ids, at most [`IMPACT_UIDS`], in the drawing's order.
    pub uids: Vec<String>,
    /// Of the objects drawn with it, those keeping a value of their own, by property.
    pub own: Vec<ImpactOwn>,
    /// Of the followers, those on locked layers: a style's change leaves them as they are (a layer's look still
    /// reaches them).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub locked: u32,
    /// The layers drawn with it (a line type), by id.
    pub layers: Vec<String>,
    /// The block definitions with pieces that take it, by id.
    pub blocks: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub block_pieces: u32,
    /// The drawing's inserts that show those definitions, nested ones too.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub inserts: u32,
    /// The layer states that keep a look of their own for it (applying them brings that back), by name.
    pub layer_states: Vec<String>,
    /// The sheet maps that draw it otherwise (docs/adr/0314 §1), by item id.
    pub sheet_maps: Vec<String>,
    /// External references holding it: KentOS keeps none, always 0.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub external_references: u32,
    /// What it means, Turkish sentences.
    pub notes: Vec<String>,
    /// The document's revision it was read at, as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
