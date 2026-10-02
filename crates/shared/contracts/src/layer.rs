//! The layer tree and its simple look (`LayerNode`, `LayerStyle` in `apps/web/src/model/layers.ts`).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LineType {
    Continuous,
    Dashed,
    Dashdot,
    Dotted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum PointSymbol {
    Ring,
    Cross,
    Triangle,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointStyle {
    pub symbol: PointSymbol,
    /// Diameter in CSS px.
    pub size: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LabelPlacement {
    Center,
    Corner,
    Beside,
    Along,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LabelInk {
    #[serde(rename = "fg")]
    Fg,
    #[serde(rename = "fg-dim")]
    FgDim,
    #[serde(rename = "label")]
    Label,
}

/// How entity labels on a layer are drawn; sizes in CSS px.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LabelStyle {
    pub placement: LabelPlacement,
    pub size: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub grow: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub max_size: Option<f64>,
    /// 400, 500 or 600.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub weight: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub template: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub min_feature_px: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub min_scale: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub max_scale: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub ink: Option<LabelInk>,
}

/// A layer's look. `renderer` (the style engine, docs/STYLE.md) is opaque JSON in v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayerStyle {
    /// Hex colour or a theme token (fg, fg-dim, ink, paper).
    pub color: String,
    pub line_type: LineType,
    /// Plot line weight in mm.
    pub line_weight: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fill: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub point: Option<PointStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<LabelStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub pick_interior: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "unknown"))]
    pub renderer: Option<serde_json::Value>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LayerNodeType {
    Group,
    Layer,
}

/// A node of the layer tree: a group or a layer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayerNode {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: LayerNodeType,
    pub visible: bool,
    pub locked: bool,
    pub expanded: bool,
    pub style: LayerStyle,
    pub children: Vec<LayerNode>,
    /// A layer's own snapping (docs/adr/0163 §4): off, or only some kinds;
    /// absent, the general kinds. Only a layer has it, never a group.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub snap: Option<LayerSnap>,
}

/// The snap kinds a layer can keep to (the settings' `snap.<kind>` and the
/// Kenet menu's kinds; Uç nokta brings Çeyrek with it, as the settings do).
pub const LAYER_SNAP_KINDS: [&str; 12] = [
    "endpoint",
    "midpoint",
    "center",
    "node",
    "intersection",
    "perpendicular",
    "tangent",
    "nearest",
    "centroid",
    "extension",
    "parallel",
    "grid",
];

/// A layer's own snapping (docs/adr/0163 §4): `{ "off": true }`, no
/// snapping to its objects; or `{ "kinds": [...] }`, only these kinds (as
/// far as the general kinds take them). Exactly one of the two, and a list
/// that is not empty: the readers refuse anything else ([`LayerSnap::problem`]).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayerSnap {
    /// No snapping to the layer's objects; written only as `true`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub off: bool,
    /// Only these kinds ([`LAYER_SNAP_KINDS`]' names).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub kinds: Option<Vec<String>>,
}

impl LayerSnap {
    /// What is wrong with it, when anything is: both or neither of `off`
    /// and `kinds`; an empty list, a repeated or an unknown kind.
    pub fn problem(&self) -> Option<String> {
        let kinds = match (self.off, &self.kinds) {
            (true, None) => return None,
            (true, Some(_)) => {
                return Some("katmanın keneti ya kapalıdır (off) ya da türleri vardır (kinds), ikisi birden değil".to_owned());
            }
            (false, None) => {
                return Some(
                    "katmanın keneti off: true ya da kinds ister; ikisi de yoksa alan yazılmaz"
                        .to_owned(),
                );
            }
            (false, Some(kinds)) => kinds,
        };
        if kinds.is_empty() {
            return Some("kenet türleri boş olamaz; hiçbiri için off yazılır".to_owned());
        }
        for (i, k) in kinds.iter().enumerate() {
            if !LAYER_SNAP_KINDS.contains(&k.as_str()) {
                return Some(format!("bilinmeyen kenet türü: {k}"));
            }
            if kinds[..i].contains(k) {
                return Some(format!("kenet türü iki kez yazılmış: {k}"));
            }
        }
        None
    }
}
