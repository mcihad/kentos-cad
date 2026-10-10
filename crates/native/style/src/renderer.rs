//! A layer's renderer as the layer style window edits it (the web's
//! `LayerRenderer`, `model/style.ts`): one symbol set for every object, a
//! set per value of an expression, per numeric class, or per rule. The
//! layer's style keeps it as JSON (`LayerStyle::renderer`); the style core
//! reads that JSON as it is. A field this version does not know stays where
//! it was, so a project edited here keeps what a newer KentOS wrote.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// The geometry classes the style engine draws: every object is an area, a
/// set of lines or a point (the web's `GeometryClass`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GeometryClass {
    Fill,
    Line,
    Marker,
}

impl GeometryClass {
    pub const ALL: [GeometryClass; 3] = [
        GeometryClass::Fill,
        GeometryClass::Line,
        GeometryClass::Marker,
    ];

    /// The key in a symbol set and in a symbol's `type`.
    pub fn key(self) -> &'static str {
        match self {
            GeometryClass::Fill => "fill",
            GeometryClass::Line => "line",
            GeometryClass::Marker => "marker",
        }
    }

    /// What the window calls it (`CLASS_LABEL`).
    pub fn label(self) -> &'static str {
        match self {
            GeometryClass::Fill => "Alan",
            GeometryClass::Line => "Çizgi",
            GeometryClass::Marker => "Nokta",
        }
    }

    pub fn from_key(key: &str) -> Option<GeometryClass> {
        GeometryClass::ALL.into_iter().find(|c| c.key() == key)
    }
}

/// What a layer (mixed geometry) draws for each geometry class: a library
/// symbol (`{"ref": id}`) or a symbol written into the style.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SymbolSet {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker: Option<Value>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl SymbolSet {
    pub fn get(&self, class: GeometryClass) -> Option<&Value> {
        match class {
            GeometryClass::Fill => self.fill.as_ref(),
            GeometryClass::Line => self.line.as_ref(),
            GeometryClass::Marker => self.marker.as_ref(),
        }
    }

    pub fn set(&mut self, class: GeometryClass, symbol: Option<Value>) {
        match class {
            GeometryClass::Fill => self.fill = symbol,
            GeometryClass::Line => self.line = symbol,
            GeometryClass::Marker => self.marker = symbol,
        }
    }

    /// Only the classes given (`pick`): a new set shows what the layer has.
    pub fn only(&self, classes: &[GeometryClass]) -> SymbolSet {
        let mut out = SymbolSet::default();
        for c in classes {
            out.set(*c, self.get(*c).cloned());
        }
        out
    }

    /// Reads a set from the style's JSON; anything else is an empty set.
    pub fn from_value(v: &Value) -> SymbolSet {
        serde_json::from_value(v.clone()).unwrap_or_default()
    }

    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }
}

/// A library symbol's id, when the reference is one (`{"ref": id}`).
pub fn ref_id(symbol: &Value) -> Option<&str> {
    symbol.get("ref").and_then(Value::as_str)
}

/// A category of a categorized renderer: the objects whose value is `value`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Category {
    pub value: String,
    pub label: String,
    #[serde(default)]
    pub symbols: SymbolSet,
    /// Only `false` is written: the category is kept but draws nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A class of a graduated renderer: min ≤ value < max; the last class includes its max.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraduatedClass {
    pub min: f64,
    pub max: f64,
    pub label: String,
    #[serde(default)]
    pub symbols: SymbolSet,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A rule of a rule-based renderer (QGIS “Kurala dayalı”).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    pub id: String,
    pub label: String,
    /// An expression; none: every object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    /// Draws what no sibling rule took.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_else: Option<bool>,
    /// The scale range as 1:N denominators: drawn when min ≤ N ≤ max.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_scale: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_scale: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbols: Option<SymbolSet>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<Rule>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Rule {
    pub fn new(id: String, label: &str, is_else: bool) -> Rule {
        Rule {
            id,
            label: label.to_owned(),
            filter: None,
            is_else: is_else.then_some(true),
            min_scale: None,
            max_scale: None,
            symbols: Some(SymbolSet::default()),
            children: None,
            enabled: None,
            extra: Map::new(),
        }
    }

    pub fn is_else(&self) -> bool {
        self.is_else == Some(true)
    }

    pub fn enabled(&self) -> bool {
        self.enabled != Some(false)
    }

    pub fn children(&self) -> &[Rule] {
        self.children.as_deref().unwrap_or_default()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Single {
    #[serde(default)]
    pub symbols: SymbolSet,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Categorized {
    #[serde(default)]
    pub expr: String,
    #[serde(default)]
    pub categories: Vec<Category>,
    /// “Diğer değerler”: what the objects no category takes draw with; none: nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub other: Option<SymbolSet>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Graduated {
    #[serde(default)]
    pub expr: String,
    #[serde(default)]
    pub classes: Vec<GraduatedClass>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Rules {
    #[serde(default)]
    pub rules: Vec<Rule>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

// ── Thematic renderers (docs/adr/0213 §2) ──────────────────────────────

/// A value of a dot density's or a chart's list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Field {
    #[serde(default)]
    pub expr: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default)]
    pub color: String,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// `{ color, width }`: a chart's outline, a displacement's circle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stroke {
    pub color: String,
    pub width: f64,
}

/// Sürekli renk.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Unclassed {
    #[serde(default)]
    pub expr: String,
    pub min: f64,
    pub max: f64,
    #[serde(default)]
    pub ramp: Vec<String>,
    #[serde(default)]
    pub symbols: SymbolSet,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub other: Option<SymbolSet>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Orantılı sembol.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Proportional {
    #[serde(default)]
    pub expr: String,
    pub min_value: f64,
    pub max_value: f64,
    pub min_size: f64,
    pub max_size: f64,
    /// `mm` (the default) or `px`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// `area` (the default), `radius` or `flannery`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scaling: Option<String>,
    #[serde(default)]
    pub symbols: SymbolSet,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub other: Option<SymbolSet>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// İki değişkenli renk: `colors[j · n + i]`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bivariate {
    #[serde(default)]
    pub expr_x: String,
    #[serde(default)]
    pub expr_y: String,
    #[serde(default)]
    pub breaks_x: Vec<f64>,
    #[serde(default)]
    pub breaks_y: Vec<f64>,
    #[serde(default)]
    pub colors: Vec<String>,
    #[serde(default)]
    pub symbols: SymbolSet,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub other: Option<SymbolSet>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Nokta yoğunluğu.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DotDensity {
    #[serde(default)]
    pub fields: Vec<Field>,
    pub dot_value: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dot_size: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbols: Option<SymbolSet>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A pie's diameter from its total.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SizeBy {
    pub min_value: f64,
    pub max_value: f64,
    pub min_size: f64,
    pub max_size: f64,
}

/// Grafik.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chart {
    /// `pie` (the default), `bar` or `stacked`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default)]
    pub fields: Vec<Field>,
    pub size: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_by: Option<SizeBy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bar_width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outline: Option<Stroke>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbols: Option<SymbolSet>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Isı haritası.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Heatmap {
    pub radius: f64,
    /// `px` (the default) or `m`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<String>,
    /// Fixed; none: dynamic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(default)]
    pub ramp: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f64>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Kümeleme; `renderer` its single points'.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cluster {
    pub distance: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grow: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub renderer: Option<Value>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Yayma; `renderer` its single points'.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Displacement {
    pub tolerance: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// `ring` (the default), `rings` or `grid`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spacing: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub center: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub circle: Option<Stroke>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub renderer: Option<Value>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Ters alan.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Inverted {
    #[serde(default)]
    pub symbols: SymbolSet,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merge: Option<bool>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A layer renderer; `LayerStyle::renderer` without one is the layer's simple look.
#[derive(Clone, Debug, PartialEq)]
pub enum Renderer {
    Single(Single),
    Categorized(Categorized),
    Graduated(Graduated),
    Rules(Rules),
    Unclassed(Unclassed),
    Proportional(Proportional),
    Bivariate(Bivariate),
    DotDensity(DotDensity),
    Chart(Chart),
    Heatmap(Heatmap),
    Cluster(Cluster),
    Displacement(Displacement),
    Inverted(Inverted),
}

impl Renderer {
    /// The renderer the style's JSON holds, or why it cannot be read (a kind
    /// this version does not know).
    pub fn from_value(v: &Value) -> Result<Renderer, String> {
        fn read<T: serde::de::DeserializeOwned>(v: &Value) -> Result<T, String> {
            let mut v = v.clone();
            if let Some(o) = v.as_object_mut() {
                o.remove("type");
            }
            serde_json::from_value(v).map_err(|e| e.to_string())
        }
        match v.get("type").and_then(Value::as_str) {
            Some("single") => read(v).map(Renderer::Single),
            Some("categorized") => read(v).map(Renderer::Categorized),
            Some("graduated") => read(v).map(Renderer::Graduated),
            Some("rules") => read(v).map(Renderer::Rules),
            Some("unclassed") => read(v).map(Renderer::Unclassed),
            Some("proportional") => read(v).map(Renderer::Proportional),
            Some("bivariate") => read(v).map(Renderer::Bivariate),
            Some("dotDensity") => read(v).map(Renderer::DotDensity),
            Some("chart") => read(v).map(Renderer::Chart),
            Some("heatmap") => read(v).map(Renderer::Heatmap),
            Some("cluster") => read(v).map(Renderer::Cluster),
            Some("displacement") => read(v).map(Renderer::Displacement),
            Some("inverted") => read(v).map(Renderer::Inverted),
            Some(other) => Err(format!("“{other}” türünde bir katman stili")),
            None => Err("türü yazılmamış bir katman stili".to_owned()),
        }
    }

    /// The JSON the layer's style keeps.
    pub fn to_value(&self) -> Value {
        let v = match self {
            Renderer::Single(r) => serde_json::to_value(r),
            Renderer::Categorized(r) => serde_json::to_value(r),
            Renderer::Graduated(r) => serde_json::to_value(r),
            Renderer::Rules(r) => serde_json::to_value(r),
            Renderer::Unclassed(r) => serde_json::to_value(r),
            Renderer::Proportional(r) => serde_json::to_value(r),
            Renderer::Bivariate(r) => serde_json::to_value(r),
            Renderer::DotDensity(r) => serde_json::to_value(r),
            Renderer::Chart(r) => serde_json::to_value(r),
            Renderer::Heatmap(r) => serde_json::to_value(r),
            Renderer::Cluster(r) => serde_json::to_value(r),
            Renderer::Displacement(r) => serde_json::to_value(r),
            Renderer::Inverted(r) => serde_json::to_value(r),
        };
        let mut v = v.unwrap_or(Value::Null);
        if let Some(o) = v.as_object_mut() {
            o.insert("type".into(), Value::from(self.kind()));
        }
        v
    }

    /// The `type` it is written with.
    pub fn kind(&self) -> &'static str {
        match self {
            Renderer::Single(_) => "single",
            Renderer::Categorized(_) => "categorized",
            Renderer::Graduated(_) => "graduated",
            Renderer::Rules(_) => "rules",
            Renderer::Unclassed(_) => "unclassed",
            Renderer::Proportional(_) => "proportional",
            Renderer::Bivariate(_) => "bivariate",
            Renderer::DotDensity(_) => "dotDensity",
            Renderer::Chart(_) => "chart",
            Renderer::Heatmap(_) => "heatmap",
            Renderer::Cluster(_) => "cluster",
            Renderer::Displacement(_) => "displacement",
            Renderer::Inverted(_) => "inverted",
        }
    }

    /// What its build depends on beyond the objects (docs/adr/0213 §3; the core's `Renderer::view_needs`).
    pub fn view_needs(&self) -> ViewNeeds {
        let px = |u: &Option<String>| u.as_deref() != Some("m");
        match self {
            Renderer::Heatmap(_) => ViewNeeds {
                scale: true,
                frame: Some(ViewFrameKind::Heat),
                whole: true,
            },
            Renderer::Cluster(c) => ViewNeeds {
                scale: px(&c.unit),
                frame: None,
                whole: true,
            },
            Renderer::Displacement(d) => ViewNeeds {
                scale: px(&d.unit),
                frame: None,
                whole: true,
            },
            Renderer::Chart(c) => ViewNeeds {
                scale: c.unit.as_deref() == Some("px"),
                frame: None,
                whole: false,
            },
            Renderer::Inverted(_) => ViewNeeds {
                scale: false,
                frame: Some(ViewFrameKind::Construction),
                whole: true,
            },
            // The bound on its dots is the layer's (docs/adr/0213 §2.4).
            Renderer::DotDensity(_) => ViewNeeds {
                whole: true,
                ..ViewNeeds::default()
            },
            _ => ViewNeeds::default(),
        }
    }
}

/// The box a view-dependent build takes (docs/adr/0213 §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewFrameKind {
    /// The view and half of it on every side.
    Heat,
    /// The construction lines' box.
    Construction,
}

/// What a renderer's build depends on beyond the objects.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ViewNeeds {
    /// The view's scale in quarter octaves.
    pub scale: bool,
    pub frame: Option<ViewFrameKind>,
    /// Built whole (never in parts).
    pub whole: bool,
}

/// What a layer style's renderer JSON needs (none without one, or one this version does not read).
pub fn needs_of(renderer: Option<&Value>) -> ViewNeeds {
    renderer
        .and_then(|v| Renderer::from_value(v).ok())
        .map_or_else(ViewNeeds::default, |r| r.view_needs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_and_writes_the_web_s_renderers_keeping_unknown_fields() {
        let cases = [
            json!({ "type": "single", "symbols": { "fill": { "ref": "sys.a" } } }),
            json!({
                "type": "categorized", "expr": "Nitelik",
                "categories": [
                    { "value": "Arsa", "label": "Arsa", "symbols": { "fill": { "ref": "x" } }, "enabled": false },
                    { "value": "Tarla", "label": "Tarla", "symbols": {}, "note": "newer field" },
                ],
                "other": { "line": { "type": "line", "layers": [] } },
                "sortBy": "value",
            }),
            json!({ "type": "graduated", "expr": "$alan", "classes": [
                { "min": 0.0, "max": 10.5, "label": "0 – 10.5", "symbols": { "marker": { "ref": "m" } } },
            ] }),
            json!({ "type": "rules", "rules": [
                { "id": "r1", "label": "Büyük", "filter": "$alan > 500", "minScale": 500.0, "maxScale": 5000.0,
                  "symbols": {}, "children": [ { "id": "r2", "label": "Diğerleri", "isElse": true } ] },
                { "id": "r3", "label": "Kapalı", "enabled": false },
            ] }),
        ];
        for v in cases {
            let r = Renderer::from_value(&v).expect("reads");
            assert_eq!(r.to_value(), v, "{}", v["type"]);
        }
        assert!(Renderer::from_value(&json!({ "type": "pie" })).is_err());
        assert!(Renderer::from_value(&json!({ "symbols": {} })).is_err());
    }
}
