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

impl LineType {
    /// Every line type, in the contract's order.
    pub const ALL: [LineType; 4] = [
        LineType::Continuous,
        LineType::Dashed,
        LineType::Dashdot,
        LineType::Dotted,
    ];

    /// Its name in the contract and the file (`dashdot`).
    pub fn name(self) -> &'static str {
        match self {
            LineType::Continuous => "continuous",
            LineType::Dashed => "dashed",
            LineType::Dashdot => "dashdot",
            LineType::Dotted => "dotted",
        }
    }

    /// The line type of a name; none for any other.
    pub fn from_name(name: &str) -> Option<LineType> {
        Self::ALL.into_iter().find(|t| t.name() == name)
    }

    /// Its name as the interface says it.
    pub fn label(self) -> &'static str {
        match self {
            LineType::Continuous => "Sürekli",
            LineType::Dashed => "Kesikli",
            LineType::Dashdot => "Noktalı kesik",
            LineType::Dotted => "Noktalı",
        }
    }
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
    /// The text as an expression (İfadeyle seç's language, docs/adr/0212 §2): instead of the template.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub text: Option<String>,
    /// The letters' colour (hex or a theme name): instead of `ink`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    /// Leaning letters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub italic: Option<bool>,
    /// How a label's lines line up; absent, centred.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub align: Option<crate::labels::LabelAlign>,
    /// Where a point's label goes; absent, by `placement`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub point: Option<crate::labels::PointLabelMode>,
    /// Where a line's label goes; absent, by `placement`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub line: Option<crate::labels::LineLabelMode>,
    /// Where an area's label goes; absent, by `placement`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub area: Option<crate::labels::AreaLabelMode>,
    /// A line label's side of its line (on an outline, `above` inside).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub position: Option<crate::labels::LabelPosition>,
    /// CSS px between a label and its object; absent, 2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub distance: Option<f64>,
    /// CSS px between a line's repeated labels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub repeat: Option<f64>,
    /// Degrees two neighbouring letters of a curved label may turn; absent, 25.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub max_angle: Option<f64>,
    /// An outline's label (perimeter, boundary) letter by letter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub curved: Option<bool>,
    /// Lines meeting end to end with the same text labelled once.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub merge_lines: Option<bool>,
    /// An area's label only where it fits inside.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub inside: Option<bool>,
    /// An area's label that fits nowhere inside, outside with a callout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub outside: Option<bool>,
    /// The halo round the letters; absent, 1.5 px of the drawing area's colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub halo: Option<crate::labels::LabelHalo>,
    /// A shape behind the label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub background: Option<crate::labels::LabelBackground>,
    /// The label's shadow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub shadow: Option<crate::labels::LabelShadow>,
    /// The line to the object from a label placed away from it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub callout: Option<crate::labels::LabelCallout>,
    /// Cutting the label into lines.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub stack: Option<crate::labels::LabelStack>,
    /// Shortening its words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub abbreviate: Option<crate::labels::LabelAbbreviate>,
    /// The smallest size a label is made when it does not fit, a factor 0.5–1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub shrink: Option<f64>,
    /// 0 to 10; absent, 5: higher first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub priority: Option<u8>,
    /// Whether it may cover another label; absent, never.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub overlap: Option<crate::labels::LabelOverlap>,
    /// CSS px within which the same text is labelled once.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub duplicates: Option<f64>,
}

impl Default for LabelStyle {
    /// Centred, 10 px, nothing else.
    fn default() -> Self {
        LabelStyle {
            placement: LabelPlacement::Center,
            size: 10.0,
            grow: None,
            max_size: None,
            weight: None,
            template: None,
            min_feature_px: None,
            min_scale: None,
            max_scale: None,
            ink: None,
            text: None,
            color: None,
            italic: None,
            align: None,
            point: None,
            line: None,
            area: None,
            position: None,
            distance: None,
            repeat: None,
            max_angle: None,
            curved: None,
            merge_lines: None,
            inside: None,
            outside: None,
            halo: None,
            background: None,
            shadow: None,
            callout: None,
            stack: None,
            abbreviate: None,
            shrink: None,
            priority: None,
            overlap: None,
            duplicates: None,
        }
    }
}

/// The label style of an object whose layer has none, by its kind: the
/// web's `DEFAULT_LABELS` (apps/web/src/model/labelDefaults.ts); the desktop
/// app's tests hold the two to each other (labels.rs). The drawing's labels,
/// Etiketleri yazıya çevir and the linked texts the documents keep with
/// their objects (docs/adr/0175 §4) read it.
pub fn default_label(kind: &str) -> Option<LabelStyle> {
    let style = |placement, size| LabelStyle {
        placement,
        size,
        ..LabelStyle::default()
    };
    Some(match kind {
        "polygon" => LabelStyle {
            grow: Some(1.0),
            max_size: Some(14.0),
            min_feature_px: Some(26.0),
            ..style(LabelPlacement::Center, 10.0)
        },
        "circle" => LabelStyle {
            min_feature_px: Some(26.0),
            ..style(LabelPlacement::Center, 10.0)
        },
        "point" => LabelStyle {
            min_scale: Some(2.0),
            ..style(LabelPlacement::Beside, 10.5)
        },
        "polyline" | "line" => LabelStyle {
            min_scale: Some(1.6),
            ..style(LabelPlacement::Along, 10.0)
        },
        _ => return None,
    })
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
    /// How the layer is labelled (docs/adr/0212 §2): rule-based classes,
    /// none, its objects as obstacles; absent, by `label` (one label).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub labels: Option<crate::labels::LayerLabels>,
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
    /// A layer's fields (docs/adr/0199 §1): the schema of its objects'
    /// attributes, in the order the table and the form show them; empty, no
    /// schema (and nothing written). Only a layer has them, never a group.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "ts",
        ts(as = "Option<Vec<crate::fields::LayerField>>", optional)
    )]
    pub fields: Vec<crate::fields::LayerField>,
    /// A layer drawn from a map service (docs/adr/0208 §2): it holds no
    /// objects. Only a layer has it, never a group.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub service: Option<crate::service::ServiceLayer>,
    /// Where the layer's objects were taken from, to take them again
    /// (docs/adr/0208 §10). Only a layer has it, never a group.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub feed: Option<crate::service::FeatureFeed>,
    /// A temporal layer's time setting (docs/adr/0210 §2): which attributes
    /// hold its objects' start and end. Only a layer has it, never a group.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub time: Option<crate::temporal::LayerTime>,
    /// A group made a scenario (docs/adr/0210 §9). Only a group has it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub scenario: Option<crate::temporal::ScenarioInfo>,
    /// A scenario's layer: the base layer it stands for (docs/adr/0210 §9).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub replaces: Option<String>,
    /// The layer's filter (docs/adr/0211 §2): only the objects that pass it
    /// are shown, picked and given to its tools. Only a layer has it, never a
    /// group or a layer drawn from a service.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub filter: Option<crate::layer_filter::LayerFilter>,
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

/// A named layer state (docs/adr/0177 §4; QGIS's map themes, AutoCAD's
/// layer states): the tree's nodes as they were when it was saved, their
/// visibility and, when it was saved with them, their locks and the layers'
/// styles. The project keeps them (`ProjectSettings::layer_states`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayerState {
    /// One of its kind among the project's states.
    pub id: String,
    /// Its name as the menu lists it: not empty, one of its kind (as written,
    /// spaces at its ends aside).
    pub name: String,
    /// The nodes, in the tree's order when it was saved.
    pub nodes: Vec<LayerStateNode>,
}

/// A node of a layer state: a layer's or a group's id and what was kept of it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayerStateNode {
    /// The node's id; a node the tree no longer has is passed over.
    pub node: String,
    pub visible: bool,
    /// Its own lock, when the state keeps locks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub locked: Option<bool>,
    /// A layer's style, when the state keeps styles; a group has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub style: Option<LayerStyle>,
}

/// What is wrong with a project's layer states, in the words the project
/// file and the settings say it: an empty or a repeated id or name; in a
/// state, an empty or a repeated node.
pub fn layer_states_problem(states: &[LayerState]) -> Option<String> {
    for (i, s) in states.iter().enumerate() {
        if s.id.is_empty() {
            return Some("katman durumunun kimliği boş".to_owned());
        }
        if states[..i].iter().any(|t| t.id == s.id) {
            return Some(format!("“{}” kimlikli katman durumu iki kez var", s.id));
        }
        let name = s.name.trim();
        if name.is_empty() {
            return Some("katman durumunun adı boş".to_owned());
        }
        if states[..i].iter().any(|t| t.name.trim() == name) {
            return Some(format!("“{name}” adlı katman durumu iki kez var"));
        }
        for (j, n) in s.nodes.iter().enumerate() {
            if n.node.is_empty() {
                return Some(format!("“{name}” durumunda düğüm kimliği boş"));
            }
            if s.nodes[..j].iter().any(|m| m.node == n.node) {
                return Some(format!(
                    "“{name}” durumunda “{}” düğümü iki kez var",
                    n.node
                ));
            }
        }
    }
    None
}

/// The layer states as a project keeps them: of those with the same id or
/// name the first, none with an empty one; in each, of the same node the
/// first, none with an empty id.
pub fn sanitized_layer_states(states: Vec<LayerState>) -> Vec<LayerState> {
    let mut out: Vec<LayerState> = Vec::with_capacity(states.len());
    for mut s in states {
        let name = s.name.trim().to_owned();
        if s.id.is_empty()
            || name.is_empty()
            || out.iter().any(|t| t.id == s.id || t.name.trim() == name)
        {
            continue;
        }
        let mut nodes: Vec<LayerStateNode> = Vec::with_capacity(s.nodes.len());
        for n in s.nodes {
            if !n.node.is_empty() && !nodes.iter().any(|m| m.node == n.node) {
                nodes.push(n);
            }
        }
        s.nodes = nodes;
        out.push(s);
    }
    out
}
