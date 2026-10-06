//! Drawing objects (`Entity` in `apps/web/src/model/entities.ts`), version 1.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// A point in world units: x = east (Y, sağa), y = north (X, yukarı).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

/// Fields every object has. `id` is the object's local id inside a v1 file.
/// v1 keeps no persistent id: it is derived from the file's content when the
/// file is opened (`identity`, docs/adr/0014); the server's is a UUID in
/// PostgreSQL (`feature.id`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntityBase {
    pub id: u32,
    pub layer_id: String,
    /// Colour override; absent = the layer's colour ("katmana göre").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    /// GIS attributes; text in v1 (typed attributes: CLAUDE.md §15).
    pub attrs: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
    /// Library symbol overriding the layer's style.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub symbol: Option<String>,
    /// Its own line weight, paper millimetres as the layer's
    /// (`LayerStyle.line_weight`), 0 the thinnest line; absent = the layer's
    /// ("katmana göre"). What a DXF's group 370 and an NCZ's pen give an
    /// object (docs/adr/0139).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 0.0, max = 100.0)))]
    pub line_weight: Option<f64>,
}

/// The heaviest line weight an object may have, mm: DXF's heaviest is 2.11,
/// a Netcad pen reaches 100 (docs/adr/0139).
pub const MAX_LINE_WEIGHT: f64 = 100.0;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    pub p: Vec2,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub z: Option<f64>,
    /// A multi-point object's points past its first, whose own are the
    /// fields above; absent for one point (docs/adr/0174).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub parts: Option<Vec<PointPart>>,
}

/// A point of a multi-point object past its first (docs/adr/0174): its
/// place and elevation, as the point's own fields hold the first's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointPart {
    pub p: Vec2,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub z: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LineEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    pub a: Vec2,
    pub b: Vec2,
    /// The start's elevation, m; absent = none (docs/adr/0142).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub za: Option<f64>,
    /// The end's elevation, m; absent = none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub zb: Option<f64>,
}

/// A closed ring in vertex + bulge form (a polygon hole).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RingGeometry {
    pub pts: Vec<Vec2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub bulges: Option<Vec<f64>>,
    /// Each vertex's elevation, m, as many as `pts`; `null` for a vertex
    /// without one (not 0); absent when no vertex has one (docs/adr/0142).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub zs: Option<Vec<Option<f64>>>,
}

/// A part of a multi-part area past its first (docs/adr/0143): its outer
/// ring in vertex + bulge form, its holes and its vertices' elevations, as
/// the area's own fields hold the first part's. A multi-part polyline's
/// part is the same, open and without holes (docs/adr/0174).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AreaPart {
    pub pts: Vec<Vec2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub bulges: Option<Vec<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub holes: Option<Vec<RingGeometry>>,
    /// Each vertex's elevation, as the area's own `zs` (docs/adr/0142).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub zs: Option<Vec<Option<f64>>>,
}

/// Polyline or polygon: vertices, DXF bulges (tan(θ/4), CCW positive) and, for polygons, holes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PathEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    pub pts: Vec<Vec2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub bulges: Option<Vec<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub holes: Option<Vec<RingGeometry>>,
    /// Each vertex's elevation, m, as many as `pts`; `null` for a vertex
    /// without one (not 0); absent when no vertex has one (docs/adr/0142).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub zs: Option<Vec<Option<f64>>>,
    /// A polygon's or a polyline's parts past its first, whose own are the
    /// fields above; absent for one part (docs/adr/0143, 0174). A
    /// polyline's parts are open, of two vertices or more, without holes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub parts: Option<Vec<AreaPart>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CircleEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    pub c: Vec2,
    pub r: f64,
}

/// Arc from `a0` to `a1` (radians), always counter-clockwise.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ArcEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    pub c: Vec2,
    pub r: f64,
    pub a0: f64,
    pub a1: f64,
}

/// DXF ELLIPSE: centre, major axis vector, minor/major ratio, parameters t0 → t1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EllipseEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    pub c: Vec2,
    pub major: Vec2,
    pub ratio: f64,
    pub t0: f64,
    pub t1: f64,
}

/// Construction line or ray: base point and unit direction.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ConstructionEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    pub p: Vec2,
    pub dir: Vec2,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SplineEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    pub pts: Vec<Vec2>,
    pub closed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TextEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    pub p: Vec2,
    pub text: String,
    /// Metres.
    pub height: f64,
    /// Degrees, counter-clockwise from east.
    pub rotation: f64,
    /// Where `p` is on the text; absent: the left of the baseline (docs/adr/0145).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub align: Option<TextAlign>,
    /// The letters' width times this, the height kept (Netcad's “sıkışma”,
    /// DXF's group 41); absent: 1. Finite, over 0, at most `MAX_WIDTH_FACTOR`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 0.0, max = 100.0)))]
    pub width_factor: Option<f64>,
    /// The text's box is filled with the drawing area's colour before the
    /// text is drawn (Netcad's “fon”): what lies under it does not show.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub mask: bool,
    /// The object whose label this text writes (Etiketleri yazıya çevir's
    /// “Nesneye bağlı”, docs/adr/0175 §4): its persistent id. The text
    /// follows the object as its label at `label_scale`; absent: a text of
    /// its own. Given with `label_scale` or not at all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label_of: Option<crate::identity::EntityId>,
    /// The scale's denominator (1:N) the linked label is written at; finite, over 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label_scale: Option<f64>,
    /// A multi-line text's box, line spacing and letter formats (docs/adr/0182).
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub paragraph: Paragraph,
    /// The style it follows and the face it is drawn in (docs/adr/0183 §2).
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub face: crate::TextFace,
}

/// A multi-line text's own fields (docs/adr/0182 §1): the width its lines
/// wrap to, their spacing and its letters' formats. All absent, the text is
/// as it always was: its lines end only at its line breaks (`\n`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Paragraph {
    /// Metres, finite and over 0: the lines wrap word by word to this width.
    /// Absent: they end only at line breaks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub box_width: Option<f64>,
    /// The distance between the lines' baselines in 5/3 of the height (DXF's
    /// group 44); absent: 1. From `MIN_LINE_SPACING` to `MAX_LINE_SPACING`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 0.25, max = 4.0)))]
    pub line_spacing: Option<f64>,
    /// The letters' formats: ranges of the text's Unicode scalar values (not
    /// UTF-16 units), in order, not overlapping, each with a format; a run
    /// with no format is not written, and touching runs of one format are one.
    /// Absent: none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<TextRun>>", optional))]
    pub runs: Vec<TextRun>,
}

impl Paragraph {
    /// No box, no spacing, no formats: a text as it always was.
    pub fn is_plain(&self) -> bool {
        self.box_width.is_none() && self.line_spacing.is_none() && self.runs.is_empty()
    }

    /// What is wrong with these fields for `text` (docs/adr/0182 §1): the
    /// field (`boxWidth`, `lineSpacing`, `runs`) and the refusal's words. A
    /// box width not finite or not over 0, a line spacing out of its bounds,
    /// a run out of the text, not after the one before it, with no format, an
    /// empty colour, or touching one of the same format (the two are one run).
    /// None when they may be written.
    pub fn problem(&self, text: &str) -> Option<(&'static str, String)> {
        if let Some(w) = self.box_width
            && !(w.is_finite() && w > 0.0)
        {
            return Some((
                "boxWidth",
                format!(
                    "Çok satırlı yazının kutu genişliği sıfırdan büyük olmalı; {w} verildi. Genişliği metre olarak, pozitif verin ya da alanı kaldırın (satırlar yalnız satır sonlarında biter)."
                ),
            ));
        }
        if let Some(s) = self.line_spacing
            && !(MIN_LINE_SPACING..=MAX_LINE_SPACING).contains(&s)
        {
            return Some((
                "lineSpacing",
                format!(
                    "Çok satırlı yazının satır aralığı {MIN_LINE_SPACING} ile {MAX_LINE_SPACING} arasında olmalı; {s} verildi. Aralığı bu sınırlarda verin ya da alanı kaldırın (1)."
                ),
            ));
        }
        self.runs_problem(text).map(|words| ("runs", words))
    }

    /// What is wrong with the runs for `text`, as `problem` says it.
    pub fn runs_problem(&self, text: &str) -> Option<String> {
        let letters = text.chars().count() as u64;
        let mut before: Option<&TextRun> = None;
        for (i, r) in self.runs.iter().enumerate() {
            let n = i + 1;
            if r.start >= r.end || u64::from(r.end) > letters {
                return Some(format!(
                    "{n}. biçim dilimi {}–{}: başı sonundan önce olmalı, sonu yazının harf sayısını ({letters}) aşmamalı. Dilimi yazının harfleri içinde verin.",
                    r.start, r.end
                ));
            }
            if !r.has_format() {
                return Some(format!(
                    "{n}. biçim diliminin biçimi yok; biçimsiz dilim yazılmaz. Dilime bir biçim verin ya da dilimi çıkarın."
                ));
            }
            if r.color.as_deref().is_some_and(str::is_empty) {
                return Some(format!(
                    "{n}. biçim diliminin rengi boş. Bir renk verin ya da rengi kaldırın."
                ));
            }
            if let Some(b) = before {
                if r.start < b.end {
                    return Some(format!(
                        "{n}. biçim dilimi öncekiyle örtüşüyor ya da ondan önce başlıyor; dilimler sıralı ve ayrı olmalı. Dilimleri sırayla, örtüşmeden verin."
                    ));
                }
                if r.start == b.end && r.same_format(b) {
                    return Some(format!(
                        "{n}. biçim dilimi aynı biçimdeki öncekine bitişik; ikisi tek dilimdir. İki dilimi birleştirin."
                    ));
                }
            }
            before = Some(r);
        }
        None
    }
}

impl TextRun {
    /// Whether it has any format at all.
    pub fn has_format(&self) -> bool {
        self.bold || self.italic || self.underline || self.script.is_some() || self.color.is_some()
    }

    /// Whether two runs' letters look the same.
    pub fn same_format(&self, other: &TextRun) -> bool {
        self.bold == other.bold
            && self.italic == other.italic
            && self.underline == other.underline
            && self.script == other.script
            && self.color == other.color
    }
}

/// The least and the most a multi-line text's line spacing may be (AutoCAD's own bounds).
pub const MIN_LINE_SPACING: f64 = 0.25;
pub const MAX_LINE_SPACING: f64 = 4.0;

/// A range of a text's letters and their format (docs/adr/0182 §1).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TextRun {
    /// Its first letter and the one past its last, counting Unicode scalar values.
    pub start: u32,
    pub end: u32,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub bold: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub italic: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub underline: bool,
    /// Raised or lowered, at 0.6 of the height; absent: on the line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub script: Option<TextScript>,
    /// Its own colour, as an object's (`#E5484D`, or a theme name); absent: the text's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
}

/// A run raised over the line (superscript) or lowered under it (subscript).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TextScript {
    Super,
    Sub,
}

/// Whether `n` may be a linked text's scale (docs/adr/0175 §4): finite, over 0.
pub fn label_scale_ok(n: f64) -> bool {
    n.is_finite() && n > 0.0
}

/// The widest a text's letters may be drawn, times their width (DXF's own bound).
pub const MAX_WIDTH_FACTOR: f64 = 100.0;

/// Whether `f` may be a text's width factor: finite, over 0, at most `MAX_WIDTH_FACTOR`.
pub fn width_factor_ok(f: f64) -> bool {
    f > 0.0 && f <= MAX_WIDTH_FACTOR
}

/// Which point of a text its `p` is (docs/adr/0145 §1): horizontally its
/// left, centre or right; vertically on its baseline, at its bottom (0.2 of
/// its height under the baseline), its middle (half its height over it) or
/// its top (its height over it). The left of the baseline, where a text
/// always stood, is no value but the field's absence: a text has one
/// spelling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TextAlign {
    BaselineCenter,
    BaselineRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
    MiddleLeft,
    MiddleCenter,
    MiddleRight,
    TopLeft,
    TopCenter,
    TopRight,
}

impl TextAlign {
    /// Every alignment, in the order of the enum.
    pub const ALL: [TextAlign; 11] = [
        TextAlign::BaselineCenter,
        TextAlign::BaselineRight,
        TextAlign::BottomLeft,
        TextAlign::BottomCenter,
        TextAlign::BottomRight,
        TextAlign::MiddleLeft,
        TextAlign::MiddleCenter,
        TextAlign::MiddleRight,
        TextAlign::TopLeft,
        TextAlign::TopCenter,
        TextAlign::TopRight,
    ];

    /// Its name in the contract and the file (`middleCenter`).
    pub fn name(self) -> &'static str {
        match self {
            TextAlign::BaselineCenter => "baselineCenter",
            TextAlign::BaselineRight => "baselineRight",
            TextAlign::BottomLeft => "bottomLeft",
            TextAlign::BottomCenter => "bottomCenter",
            TextAlign::BottomRight => "bottomRight",
            TextAlign::MiddleLeft => "middleLeft",
            TextAlign::MiddleCenter => "middleCenter",
            TextAlign::MiddleRight => "middleRight",
            TextAlign::TopLeft => "topLeft",
            TextAlign::TopCenter => "topCenter",
            TextAlign::TopRight => "topRight",
        }
    }

    /// The alignment named `name`; none for any other name.
    pub fn from_name(name: &str) -> Option<TextAlign> {
        TextAlign::ALL.into_iter().find(|a| a.name() == name)
    }

    /// Where `p` is along the text, as a share of its width: 0 left, ½ centre, 1 right.
    pub fn along(self) -> f64 {
        match self {
            TextAlign::BottomLeft | TextAlign::MiddleLeft | TextAlign::TopLeft => 0.0,
            TextAlign::BaselineCenter
            | TextAlign::BottomCenter
            | TextAlign::MiddleCenter
            | TextAlign::TopCenter => 0.5,
            TextAlign::BaselineRight
            | TextAlign::BottomRight
            | TextAlign::MiddleRight
            | TextAlign::TopRight => 1.0,
        }
    }

    /// Where `p` is over the baseline, as a share of the text's height: 0 the
    /// baseline, −0.2 the bottom, ½ the middle, 1 the top.
    pub fn up(self) -> f64 {
        match self {
            TextAlign::BaselineCenter | TextAlign::BaselineRight => 0.0,
            TextAlign::BottomLeft | TextAlign::BottomCenter | TextAlign::BottomRight => -0.2,
            TextAlign::MiddleLeft | TextAlign::MiddleCenter | TextAlign::MiddleRight => 0.5,
            TextAlign::TopLeft | TextAlign::TopCenter | TextAlign::TopRight => 1.0,
        }
    }
}

/// A block placed in the drawing (docs/adr/0144): its definition drawn
/// moved from the definition's base point to `p`, mirrored in the
/// definition's x axis when `mirror`, then scaled and turned about `p`. The
/// transform is a similarity: shapes keep their kind. The insert's own
/// attributes carry the values of the definition's attribute definitions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct InsertEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    /// The definition's id.
    pub block: crate::identity::BlockId,
    /// Where the definition's base point goes.
    pub p: Vec2,
    /// Positive and finite; 1 is the definition's size.
    pub scale: f64,
    /// Radians, counter-clockwise from east.
    pub rotation: f64,
    /// Mirrored in the definition's x axis, before the turn.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub mirror: bool,
}

/// A block definition (docs/adr/0144): objects drawn once, placed many
/// times. Its objects' ids are local to it; their layer is kept, but an
/// insert draws them on its own layer (the DXF's layer 0), each with its own
/// colour or line weight when it has one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BlockDefinition {
    pub id: crate::identity::BlockId,
    /// Unique in the drawing, Turkish case folded.
    pub name: String,
    /// The point placed at an insert's `p`, in the definition's coordinates.
    pub base: Vec2,
    pub entities: Vec<Entity>,
    /// The texts an insert shows from its attributes (ATTDEF).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<AttributeDefinition>>", optional))]
    pub attributes: Vec<AttributeDefinition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub description: Option<String>,
}

/// One attribute an insert shows as text (docs/adr/0144 §7): the insert's
/// attribute `tag`, else `value`, written at `p` in the definition's
/// coordinates, moved with the insert.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AttributeDefinition {
    pub tag: String,
    /// What Blok ekle asks for it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub prompt: Option<String>,
    /// The default value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub value: Option<String>,
    pub p: Vec2,
    /// Metres, in the definition's size.
    pub height: f64,
    /// Degrees, counter-clockwise from east, as a text's.
    pub rotation: f64,
    /// Where `p` is on the text, as a text's (docs/adr/0145).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub align: Option<TextAlign>,
    /// The letters' width times this, as a text's (docs/adr/0145).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 0.0, max = 100.0)))]
    pub width_factor: Option<f64>,
}

/// The deepest nesting of blocks: a definition holding inserts of
/// definitions holding inserts, and so on (docs/adr/0144).
pub const MAX_BLOCK_DEPTH: usize = 16;

/// A dimension's kind (none: aligned). The last five came with KCAD schema 9
/// (docs/adr/0147): what `a`, `b`, `c`, `offset`, `angle`, `za` and `zb`
/// mean for each is the ADR's table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DimensionStyle {
    Aligned,
    Linear,
    Angular,
    Radius,
    Diameter,
    /// Koordinat: the point's Y (`angle` 0) or X (`angle` 90).
    Ordinate,
    /// Yay uzunluğu: the arc about `c` from `a` to `b`, counter-clockwise.
    ArcLength,
    /// Kırıklı yarıçap: the radius about `a` through `b`, drawn from `c`.
    Jogged,
    /// Semt: the direction from `a` to `b`, from north clockwise.
    Azimuth,
    /// Eğim: the slope between `a` at `za` and `b` at `zb`.
    Slope,
}

impl DimensionStyle {
    /// Every style, in the contract's order (the typed columns number them so).
    pub const ALL: [DimensionStyle; 10] = [
        DimensionStyle::Aligned,
        DimensionStyle::Linear,
        DimensionStyle::Angular,
        DimensionStyle::Radius,
        DimensionStyle::Diameter,
        DimensionStyle::Ordinate,
        DimensionStyle::ArcLength,
        DimensionStyle::Jogged,
        DimensionStyle::Azimuth,
        DimensionStyle::Slope,
    ];

    /// Its name in the contract and the file (`arcLength`).
    pub fn name(self) -> &'static str {
        match self {
            DimensionStyle::Aligned => "aligned",
            DimensionStyle::Linear => "linear",
            DimensionStyle::Angular => "angular",
            DimensionStyle::Radius => "radius",
            DimensionStyle::Diameter => "diameter",
            DimensionStyle::Ordinate => "ordinate",
            DimensionStyle::ArcLength => "arcLength",
            DimensionStyle::Jogged => "jogged",
            DimensionStyle::Azimuth => "azimuth",
            DimensionStyle::Slope => "slope",
        }
    }

    /// The style named `name`; none for any other name.
    pub fn from_name(name: &str) -> Option<DimensionStyle> {
        DimensionStyle::ALL.into_iter().find(|s| s.name() == name)
    }

    /// Whether it came with KCAD schema 9 (docs/adr/0147).
    pub fn is_schema_9(self) -> bool {
        matches!(
            self,
            DimensionStyle::Ordinate
                | DimensionStyle::ArcLength
                | DimensionStyle::Jogged
                | DimensionStyle::Azimuth
                | DimensionStyle::Slope
        )
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    pub a: Vec2,
    pub b: Vec2,
    pub offset: f64,
    pub height: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub style: Option<DimensionStyle>,
    /// Linear: measured direction in degrees (0 = ΔY, 90 = ΔX).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub angle: Option<f64>,
    /// Angular: the vertex. Arc length: the arc's centre. Jogged: the centre
    /// the line starts from (docs/adr/0147).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub c: Option<Vec2>,
    /// The value is drawn over the drawing's background (docs/adr/0147, as a
    /// text's mask, docs/adr/0145).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub mask: bool,
    /// Slope: the two points' elevations, metres (docs/adr/0147).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub za: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub zb: Option<f64>,
    /// The style it follows, its arrowheads, sizes and value's writing (docs/adr/0183 §3).
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub look: crate::DimensionLook,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum HatchPatternType {
    Solid,
    Lines,
    Cross,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HatchPattern {
    #[serde(rename = "type")]
    pub kind: HatchPatternType,
    /// Degrees, counter-clockwise from east.
    pub angle: f64,
    /// Metres.
    pub spacing: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HatchEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    pub ring: Vec<Vec2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub holes: Option<Vec<Vec<Vec2>>>,
    pub pattern: HatchPattern,
}

/// A leader (docs/adr/0146): an arrowhead at its first vertex, a line
/// through its vertices and, with a note, a landing from its last vertex
/// along the note's direction and the note past the landing's end; one
/// object. The arrowhead and the landing are measured by the note's height.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LeaderEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    /// At least two: the arrow's tip first, where the landing starts last.
    pub pts: Vec<Vec2>,
    /// The note, one line; absent: the arrow alone, no landing and no note.
    /// Never empty: no note is the field's absence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub text: Option<String>,
    /// The note's height, metres, finite and over 0; the arrowhead and the
    /// landing are measured by it.
    pub height: f64,
    /// The note's and the landing's direction, degrees counter-clockwise from east.
    pub rotation: f64,
    /// The arrowhead; absent: a filled arrow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub arrow: Option<LeaderArrow>,
    /// The note's box is filled with the drawing area's colour before the
    /// note is drawn, as a text's mask (docs/adr/0145).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub mask: bool,
}

/// A leader's arrowhead other than the filled arrow, which is no value but
/// the field's absence: a leader has one spelling (docs/adr/0146 §1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LeaderArrow {
    /// The filled arrow's two sides.
    Open,
    /// A filled dot.
    Dot,
    /// No arrowhead: the line ends at the tip.
    None,
}

impl LeaderArrow {
    /// Every arrowhead, in the order of the enum.
    pub const ALL: [LeaderArrow; 3] = [LeaderArrow::Open, LeaderArrow::Dot, LeaderArrow::None];

    /// The name files and the wire use.
    pub fn name(self) -> &'static str {
        match self {
            LeaderArrow::Open => "open",
            LeaderArrow::Dot => "dot",
            LeaderArrow::None => "none",
        }
    }

    /// The arrowhead a name names; none for a name it does not know (the
    /// filled arrow has no name).
    pub fn from_name(name: &str) -> Option<LeaderArrow> {
        Self::ALL.into_iter().find(|a| a.name() == name)
    }
}

/// Any drawing object, tagged by `kind` as in the TypeScript model.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum Entity {
    Point(PointEntity),
    Line(LineEntity),
    Polyline(PathEntity),
    Polygon(PathEntity),
    Circle(CircleEntity),
    Arc(ArcEntity),
    Ellipse(EllipseEntity),
    Spline(SplineEntity),
    Xline(ConstructionEntity),
    Ray(ConstructionEntity),
    Text(TextEntity),
    Dimension(DimensionEntity),
    Hatch(HatchEntity),
    /// A block placed in the drawing (docs/adr/0144).
    Insert(InsertEntity),
    /// An arrow with a note (docs/adr/0146).
    Leader(LeaderEntity),
}

impl Entity {
    /// Whether the object is drawn with lines, so its own line weight shows
    /// and a new one takes the current weight: not a point, a text, a
    /// dimension or a hatch (docs/adr/0139; the web's `drawsLines`).
    pub fn draws_lines(&self) -> bool {
        !matches!(
            self,
            Entity::Point(_)
                | Entity::Text(_)
                | Entity::Dimension(_)
                | Entity::Hatch(_)
                | Entity::Insert(_)
        )
    }

    /// The fields every object has (id, layer, colour, line weight, attributes, label, symbol).
    pub fn base(&self) -> &EntityBase {
        match self {
            Entity::Point(e) => &e.base,
            Entity::Line(e) => &e.base,
            Entity::Polyline(e) | Entity::Polygon(e) => &e.base,
            Entity::Circle(e) => &e.base,
            Entity::Arc(e) => &e.base,
            Entity::Ellipse(e) => &e.base,
            Entity::Spline(e) => &e.base,
            Entity::Xline(e) | Entity::Ray(e) => &e.base,
            Entity::Text(e) => &e.base,
            Entity::Dimension(e) => &e.base,
            Entity::Hatch(e) => &e.base,
            Entity::Insert(e) => &e.base,
            Entity::Leader(e) => &e.base,
        }
    }

    /// The fields every object has, to change (a reader numbers the objects
    /// it reads: `id` is the slot, meaningless on the wire).
    pub fn base_mut(&mut self) -> &mut EntityBase {
        match self {
            Entity::Point(e) => &mut e.base,
            Entity::Line(e) => &mut e.base,
            Entity::Polyline(e) | Entity::Polygon(e) => &mut e.base,
            Entity::Circle(e) => &mut e.base,
            Entity::Arc(e) => &mut e.base,
            Entity::Ellipse(e) => &mut e.base,
            Entity::Spline(e) => &mut e.base,
            Entity::Xline(e) | Entity::Ray(e) => &mut e.base,
            Entity::Text(e) => &mut e.base,
            Entity::Dimension(e) => &mut e.base,
            Entity::Hatch(e) => &mut e.base,
            Entity::Insert(e) => &mut e.base,
            Entity::Leader(e) => &mut e.base,
        }
    }

    /// The `kind` tag as written in files and on the wire.
    pub fn kind(&self) -> &'static str {
        match self {
            Entity::Point(_) => "point",
            Entity::Line(_) => "line",
            Entity::Polyline(_) => "polyline",
            Entity::Polygon(_) => "polygon",
            Entity::Circle(_) => "circle",
            Entity::Arc(_) => "arc",
            Entity::Ellipse(_) => "ellipse",
            Entity::Spline(_) => "spline",
            Entity::Xline(_) => "xline",
            Entity::Ray(_) => "ray",
            Entity::Text(_) => "text",
            Entity::Dimension(_) => "dimension",
            Entity::Hatch(_) => "hatch",
            Entity::Insert(_) => "insert",
            Entity::Leader(_) => "leader",
        }
    }
}
