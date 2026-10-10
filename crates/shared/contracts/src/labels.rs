//! The label engine's settings (docs/adr/0212 §2): how a label class places
//! and draws its labels (the new fields of `LabelStyle`), a layer's
//! labelling (one label, rule-based classes, none; its objects as obstacles)
//! and the labels moved, turned, pinned or hidden by hand (an object's
//! `labelPins`). The rules here are the readers', the commands' and the
//! server's; whether an expression compiles is the commands' (a file's
//! reader does not know the language).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::layer::{LabelStyle, LayerNode, LayerNodeType};

/// The most classes a layer's labelling has.
pub const LABEL_CLASSES_MAX: usize = 64;
/// The longest expression (a class's text or condition), in characters.
pub const LABEL_EXPRESSION_MAX: usize = 10_000;
/// The longest name (a class's), in characters.
pub const LABEL_NAME_MAX: usize = 100;
/// The most words an abbreviation dictionary holds.
pub const LABEL_WORDS_MAX: usize = 500;
/// The most pins an object holds.
pub const LABEL_PINS_MAX: usize = 64;

/// Where a point's label goes (docs/adr/0212 §3.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum PointLabelMode {
    /// Around it, QGIS's cartographic order (top right first).
    Around,
    /// On it.
    Center,
}

/// Where a line's label goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LineLabelMode {
    /// Straight, along the line.
    Parallel,
    /// Letter by letter, following the line.
    Curved,
    /// Level, on the line.
    Horizontal,
    /// A contour's height: curved on the line, its top uphill.
    Contour,
}

/// Where an area's label goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AreaLabelMode {
    /// Level, inside, away from the edges.
    Horizontal,
    /// Level when it fits, else along the area's long side.
    Free,
    /// Along its outline.
    Perimeter,
    /// Along its outline, inside, repeated (Maplex's boundary labels).
    Boundary,
    /// Inside only: level, then along the long side, then fitted (Maplex's land parcels).
    Parcel,
    /// By its box's top left corner (a sheet's frame).
    Corner,
}

/// A line label's side of its line; on an area's outline `above` is inside
/// and `below` outside.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LabelPosition {
    On,
    Above,
    Below,
    /// Above, else below.
    Sides,
}

/// How a label's lines line up.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LabelAlign {
    Left,
    Center,
    Right,
}

/// Whether a label may cover another (QGIS's three).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LabelOverlap {
    /// Never: a label with no free place is not drawn.
    Never,
    /// When no free place is left, at its best one.
    IfNeeded,
    /// Always at its best place.
    Always,
}

/// When a label is cut into lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum StackMode {
    /// Only when it does not fit on one line.
    IfNeeded,
    Always,
}

/// A label's background's shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LabelShape {
    Rect,
    /// A rectangle with round corners.
    Round,
    Ellipse,
}

/// A callout's line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CalloutKind {
    Straight,
    /// Level, then upright.
    Manhattan,
}

/// What of an obstacle area a label avoids.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ObstacleKind {
    /// Its inside: a label covering any of it.
    Interior,
    /// Its outline: a label crossing an edge.
    Boundary,
}

/// How a layer is labelled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LabelsMode {
    /// One label: the style's `label`, else its kind's default.
    Single,
    /// The classes, in order: each whose condition holds labels an object.
    Rules,
    /// None (its objects may still be obstacles).
    Off,
}

/// The halo round a label's letters.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LabelHalo {
    /// CSS px; 0 none.
    pub width: f64,
    /// Hex or a theme name; absent, the drawing area's colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
}

/// A shape behind a label.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LabelBackground {
    pub shape: LabelShape,
    /// Hex or a theme name (`paper`: the drawing area's colour, a mask); absent, not filled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fill: Option<String>,
    /// Its outline's colour; absent, none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub stroke: Option<String>,
    /// CSS px round the letters; absent, 2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub padding: Option<f64>,
}

/// A label's shadow: the label again under it, moved.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LabelShadow {
    /// CSS px right.
    pub dx: f64,
    /// CSS px up.
    pub dy: f64,
    /// Absent, black.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    /// 0 to 1; absent, 0.5.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub opacity: Option<f64>,
}

/// The line from a label placed away from its object to the object.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LabelCallout {
    pub kind: CalloutKind,
    /// Absent, the label's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    /// CSS px; absent, 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub width: Option<f64>,
    /// A shorter one is not drawn, CSS px; absent, 6.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub min_length: Option<f64>,
}

/// Cutting a label into lines (yığma).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LabelStack {
    pub mode: StackMode,
    /// The most letters a line holds (a longer word stays whole).
    pub chars: u32,
    /// The letters a line may end at; absent, the space.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub at: Option<String>,
}

/// One word of an abbreviation dictionary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LabelWord {
    pub word: String,
    pub short: String,
}

/// Shortening a label's words (kısaltma).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LabelAbbreviate {
    /// Always; absent, only when the label does not fit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub always: Option<bool>,
    pub words: Vec<LabelWord>,
}

/// One class of a rule-based labelling: its name, the condition its objects
/// meet and its labels' style.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LabelClass {
    /// One of its kind on the layer; a pin names it.
    pub name: String,
    /// İfadeyle seç's language; absent, every object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub when: Option<String>,
    pub style: LabelStyle,
}

/// A layer's objects as obstacles to every label.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LabelObstacle {
    /// 1 to 10: a label of a lower priority cannot cover them; another may, at a cost.
    pub weight: u8,
    /// An area's inside (absent) or its outline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub kind: Option<ObstacleKind>,
}

/// How a layer is labelled (`LayerStyle.labels`, docs/adr/0212 §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayerLabels {
    pub mode: LabelsMode,
    /// The rules' classes, in order; only with `rules`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<LabelClass>>", optional))]
    pub classes: Vec<LabelClass>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub obstacle: Option<LabelObstacle>,
}

/// A label moved, turned, pinned or hidden by hand (an object's
/// `labelPins`, docs/adr/0212 §3.7).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LabelPin {
    /// The class's name; absent, the object's first label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub class: Option<String>,
    /// Where the label's middle is from the object's anchor, metres; absent, the engine places it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub at: Option<crate::Vec2>,
    /// Degrees counter-clockwise from east; only with `at`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rotation: Option<f64>,
    /// Not drawn; written only as `true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub hidden: Option<bool>,
}

/// Whether a colour is one a label may name: `#rrggbb`, `#rrggbbaa` or a
/// theme name (`fg`, `fg-dim`, `label`, `ink`, `paper`).
pub fn label_color_ok(c: &str) -> bool {
    if matches!(c, "fg" | "fg-dim" | "label" | "ink" | "paper") {
        return true;
    }
    let Some(hex) = c.strip_prefix('#') else {
        return false;
    };
    (hex.len() == 6 || hex.len() == 8) && hex.bytes().all(|b| b.is_ascii_hexdigit())
}

fn number(what: &str, x: f64, lo: f64, hi: f64) -> Option<String> {
    if !x.is_finite() {
        return Some(format!("{what} sayı değil"));
    }
    (!(lo..=hi).contains(&x)).then(|| format!("{what} {lo}–{hi} arasında olmalı"))
}

fn color(what: &str, c: &Option<String>) -> Option<String> {
    c.as_deref()
        .filter(|c| !label_color_ok(c))
        .map(|c| format!("{what} “{c}” renk değil (#rrggbb ya da fg, fg-dim, label, ink, paper)"))
}

fn expression(what: &str, e: &Option<String>) -> Option<String> {
    let e = e.as_deref()?;
    if e.trim() != e || e.is_empty() {
        return Some(format!("{what} boş ya da başında veya sonunda boşluk var"));
    }
    (e.chars().count() > LABEL_EXPRESSION_MAX)
        .then(|| format!("{what} {LABEL_EXPRESSION_MAX} karakterden uzun"))
}

/// What is wrong with a label style, when anything is (docs/adr/0212 §2).
pub fn style_problem(s: &LabelStyle) -> Option<String> {
    let opt =
        |what: &str, x: Option<f64>, lo: f64, hi: f64| x.and_then(|x| number(what, x, lo, hi));
    number("boy", s.size, 1.0, 200.0)
        .or_else(|| opt("büyüme", s.grow, 0.0, 1000.0))
        .or_else(|| opt("en büyük boy", s.max_size, 1.0, 200.0))
        .or_else(|| {
            s.weight
                .filter(|w| !(100..=900).contains(w))
                .map(|w| format!("kalınlık {w}: 100–900 olmalı"))
        })
        .or_else(|| opt("en küçük nesne", s.min_feature_px, 0.0, 100_000.0))
        .or_else(|| opt("en küçük ölçek", s.min_scale, 0.0, 1e9))
        .or_else(|| opt("en büyük ölçek", s.max_scale, 0.0, 1e9))
        .or_else(|| expression("metnin ifadesi", &s.text))
        .or_else(|| color("yazının rengi", &s.color))
        .or_else(|| opt("uzaklık", s.distance, 0.0, 500.0))
        .or_else(|| opt("yineleme", s.repeat, 20.0, 100_000.0))
        .or_else(|| opt("harfler arası en büyük açı", s.max_angle, 5.0, 90.0))
        .or_else(|| opt("küçültme", s.shrink, 0.5, 1.0))
        .or_else(|| {
            s.priority
                .filter(|p| *p > 10)
                .map(|p| format!("öncelik {p}: 0–10 arasında olmalı"))
        })
        .or_else(|| opt("yinelenenlerin uzaklığı", s.duplicates, 1.0, 10_000.0))
        .or_else(|| {
            let h = s.halo.as_ref()?;
            number("halenin genişliği", h.width, 0.0, 10.0).or_else(|| color("halenin rengi", &h.color))
        })
        .or_else(|| {
            let b = s.background.as_ref()?;
            color("zeminin dolgusu", &b.fill)
                .or_else(|| color("zeminin çizgisi", &b.stroke))
                .or_else(|| opt("zeminin payı", b.padding, 0.0, 50.0))
        })
        .or_else(|| {
            let h = s.shadow.as_ref()?;
            number("gölgenin kayması", h.dx, -50.0, 50.0)
                .or_else(|| number("gölgenin kayması", h.dy, -50.0, 50.0))
                .or_else(|| color("gölgenin rengi", &h.color))
                .or_else(|| opt("gölgenin opaklığı", h.opacity, 0.0, 1.0))
        })
        .or_else(|| {
            let c = s.callout.as_ref()?;
            color("çağrı çizgisinin rengi", &c.color)
                .or_else(|| opt("çağrı çizgisinin kalınlığı", c.width, 0.1, 10.0))
                .or_else(|| opt("çağrı çizgisinin en kısası", c.min_length, 0.0, 1000.0))
        })
        .or_else(|| {
            let k = s.stack.as_ref()?;
            if !(2..=500).contains(&k.chars) {
                return Some(format!("yığmanın satırı {} harf: 2–500 olmalı", k.chars));
            }
            let at = k.at.as_deref()?;
            (at.is_empty() || at.chars().count() > 20)
                .then(|| "yığmanın bölme karakterleri 1–20 harf olmalı".to_owned())
        })
        .or_else(|| {
            let a = s.abbreviate.as_ref()?;
            if a.words.is_empty() || a.words.len() > LABEL_WORDS_MAX {
                return Some(format!("kısaltma sözlüğü 1–{LABEL_WORDS_MAX} sözcük olmalı"));
            }
            for (i, w) in a.words.iter().enumerate() {
                let bad = |t: &str| t.is_empty() || t.chars().count() > 100 || t.chars().any(char::is_whitespace);
                if bad(&w.word) || bad(&w.short) {
                    return Some(format!(
                        "kısaltma sözlüğünün {}. satırı: sözcük ve kısası 1–100 harf, boşluksuz olmalı",
                        i + 1
                    ));
                }
                if a.words[..i].iter().any(|v| v.word == w.word) {
                    return Some(format!("kısaltma sözlüğünde “{}” iki kez var", w.word));
                }
            }
            None
        })
}

/// What is wrong with a layer's labelling, when anything is.
pub fn layer_labels_problem(l: &LayerLabels) -> Option<String> {
    match l.mode {
        LabelsMode::Rules => {
            if l.classes.is_empty() || l.classes.len() > LABEL_CLASSES_MAX {
                return Some(format!(
                    "kurallı etiketlemede 1–{LABEL_CLASSES_MAX} sınıf olmalı"
                ));
            }
        }
        LabelsMode::Single | LabelsMode::Off => {
            if !l.classes.is_empty() {
                return Some("sınıflar yalnız kurallı etiketlemede olur".to_owned());
            }
        }
    }
    for (i, c) in l.classes.iter().enumerate() {
        let name = c.name.trim();
        if name.is_empty() || name != c.name || name.chars().count() > LABEL_NAME_MAX {
            return Some(format!(
                "{}. sınıfın adı boş, {LABEL_NAME_MAX} harften uzun ya da başında veya sonunda boşluk var",
                i + 1
            ));
        }
        if l.classes[..i].iter().any(|d| d.name == c.name) {
            return Some(format!("“{name}” adlı sınıf iki kez var"));
        }
        if let Some(p) = expression("koşul", &c.when).or_else(|| style_problem(&c.style)) {
            return Some(format!("“{name}” sınıfı: {p}"));
        }
    }
    l.obstacle
        .as_ref()
        .filter(|o| !(1..=10).contains(&o.weight))
        .map(|o| format!("engelin ağırlığı {}: 1–10 olmalı", o.weight))
}

/// What is wrong with an object's pins, when anything is.
pub fn pins_problem(pins: &[LabelPin]) -> Option<String> {
    if pins.len() > LABEL_PINS_MAX {
        return Some(format!(
            "nesnenin {} etiket iğnesi var; en çok {LABEL_PINS_MAX}",
            pins.len()
        ));
    }
    for (i, p) in pins.iter().enumerate() {
        if let Some(c) = &p.class
            && (c.trim() != c || c.is_empty() || c.chars().count() > LABEL_NAME_MAX)
        {
            return Some(format!(
                "etiket iğnesinin sınıfı “{c}” geçerli bir ad değil"
            ));
        }
        if pins[..i].iter().any(|q| q.class == p.class) {
            return Some("aynı sınıfın iki etiket iğnesi var".to_owned());
        }
        if p.at.is_none() && p.rotation.is_some() {
            return Some("etiket iğnesinin açısı yerle birlikte verilir".to_owned());
        }
        if p.at.is_none() && p.hidden != Some(true) {
            return Some("etiket iğnesi ya bir yer ya da gizli olur".to_owned());
        }
        if p.hidden == Some(false) {
            return Some("etiket iğnesinin hidden'ı yalnız true yazılır".to_owned());
        }
        if let Some(a) = p.at
            && (!a.x.is_finite() || !a.y.is_finite() || a.x.abs() > 1e7 || a.y.abs() > 1e7)
        {
            return Some("etiket iğnesinin yeri sonlu ve 10 000 km'den yakın olmalı".to_owned());
        }
        if let Some(r) = p.rotation
            && (!r.is_finite() || r.abs() > 360.0)
        {
            return Some("etiket iğnesinin açısı −360–360 derece olmalı".to_owned());
        }
    }
    None
}

impl LabelStyle {
    /// Whether it has any of the label engine's fields (docs/adr/0212 §2;
    /// `.kcad` schema 36): only such a style is checked whole (an old one
    /// keeps the old readers' leniency).
    pub fn has_engine_fields(&self) -> bool {
        self.text.is_some()
            || self.color.is_some()
            || self.italic.is_some()
            || self.align.is_some()
            || self.point.is_some()
            || self.line.is_some()
            || self.area.is_some()
            || self.position.is_some()
            || self.distance.is_some()
            || self.repeat.is_some()
            || self.max_angle.is_some()
            || self.curved.is_some()
            || self.merge_lines.is_some()
            || self.inside.is_some()
            || self.outside.is_some()
            || self.halo.is_some()
            || self.background.is_some()
            || self.shadow.is_some()
            || self.callout.is_some()
            || self.stack.is_some()
            || self.abbreviate.is_some()
            || self.shrink.is_some()
            || self.priority.is_some()
            || self.overlap.is_some()
            || self.duplicates.is_some()
    }
}

/// What is wrong with the layer tree's labelling, when anything is: a
/// labelling on a group, a broken one, or a broken label style with the
/// engine's fields (an old style is left as the old readers left it).
pub fn labels_problem(tree: &[LayerNode]) -> Option<String> {
    for n in tree {
        let s = &n.style;
        // A group labels nothing (an old group's label style is left as it is).
        if s.labels.is_some() && n.kind == LayerNodeType::Group {
            return Some(format!(
                "“{}” bir grup; grubun etiketlemesi olmaz, etiketleme katmanındır",
                n.name
            ));
        }
        if let Some(p) = s
            .label
            .as_ref()
            .filter(|l| l.has_engine_fields())
            .and_then(style_problem)
        {
            return Some(format!("“{}” katmanının etiketi: {p}", n.name));
        }
        if let Some(p) = s.labels.as_ref().and_then(layer_labels_problem) {
            return Some(format!("“{}” katmanının etiketlemesi: {p}", n.name));
        }
        if let Some(p) = labels_problem(&n.children) {
            return Some(p);
        }
    }
    None
}
