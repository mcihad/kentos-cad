//! `cad.layers.classes` v1 (docs/adr/9105 §2): a layer's style classes as the layer tree lists them under the layer,
//! and as Python and the assistants read them: one per category and “Diğer değerler”, per graduated class, per rule
//! with its children after it. Each says whether it is drawn and its condition in İfadeyle seç's language; with
//! `count`, how many of the layer's objects it takes, by the drawing's own rule (a data layer's rows read from its
//! source, every row once, only the fields the classes read). Nothing is written: classes are switched on and off with
//! `cad.layers.set` (`classVisible`, `classInvert`).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CAD_LAYERS_CLASSES: &str = "cad.layers.classes";
pub const CAD_LAYERS_CLASSES_VERSION: u32 = 1;

/// Input of `cad.layers.classes` v1: the classes of `layer`'s renderer (kategorili, aralıklı, kurallı); a layer drawn
/// by its own look, tek sembol or a thematic renderer has none.
///
/// Refusals (`CommandError.code`), checked in this order: `no_layer` (`layer` empty), `layer_not_found`,
/// `not_a_layer` (a group), `service_layer` (a layer drawn from a map service), `source_unreachable` (`count` of a
/// data layer whose table could not be read; the message says why). A job stopped answers `cancelled`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayersClasses {
    /// The layer (its id).
    pub layer: String,
    /// Count the objects each class takes (Nesne sayısını göster); a class switched off still says what it takes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub count: Option<bool>,
}

/// A class of a layer's renderer.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayerClass {
    /// `c3` (a category), `other` (Diğer değerler), `g2` (a graduated class), `r0.1` (a rule by its path).
    pub id: String,
    pub label: String,
    /// Drawn; switched off, its objects are drawn by nothing.
    pub enabled: bool,
    /// A rule's depth under the top; 0 for every other class.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub depth: u32,
    /// Its condition in İfadeyle seç's language; none: every object of the layer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expression: Option<String>,
    /// `count`: the objects it takes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub count: Option<u64>,
    /// `count`: why it could not be counted (its expression cannot be read).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub count_error: Option<String>,
}

/// Output of `cad.layers.classes` v1.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayerClassesAnswer {
    pub layer: String,
    /// The renderer's kind: `categorized`, `graduated` or `rules`; none: the layer has no classes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub kind: Option<String>,
    /// In the tree's order: a rule's children after it.
    pub classes: Vec<LayerClass>,
    /// `count`: the objects of the layer, every class's and those none takes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub total: Option<u64>,
}
