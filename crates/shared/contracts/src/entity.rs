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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub line_weight: Option<f64>,
    /// Its labels moved, turned, pinned or hidden by hand (docs/adr/0212
    /// §3.7), one a class at most.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "ts",
        ts(as = "Option<Vec<crate::labels::LabelPin>>", optional)
    )]
    pub label_pins: Vec<crate::labels::LabelPin>,
    /// The photos and documents it carries, in their order (docs/adr/0225
    /// §1); none on a block's objects.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "ts",
        ts(as = "Option<Vec<crate::attachment::Attachment>>", optional)
    )]
    pub attachments: Vec<crate::attachment::Attachment>,
    /// Its line type, line type scale, transparency, drawing order and, on a
    /// block's object, the parts that follow its own layer (docs/adr/0260).
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub look: crate::look::ObjectLook,
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub p: Vec2,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub p: Vec2,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub a: Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub b: Vec2,
    /// The start's elevation, m; absent = none (docs/adr/0142).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub za: Option<f64>,
    /// The end's elevation, m; absent = none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub zb: Option<f64>,
    /// The start's measure (M) on a route, in the route's own unit; absent =
    /// none (docs/adr/0224 §1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "measure")))]
    pub ma: Option<f64>,
    /// The end's measure; absent = none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "measure")))]
    pub mb: Option<f64>,
}

/// A closed ring in vertex + bulge form (a polygon hole).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RingGeometry {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub pts: Vec<Vec2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub bulges: Option<Vec<f64>>,
    /// Each vertex's elevation, m, as many as `pts`; `null` for a vertex
    /// without one (not 0); absent when no vertex has one (docs/adr/0142).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub pts: Vec<Vec2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub bulges: Option<Vec<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub holes: Option<Vec<RingGeometry>>,
    /// Each vertex's elevation, as the area's own `zs` (docs/adr/0142).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub zs: Option<Vec<Option<f64>>>,
    /// A polyline's part's measures, as the polyline's own `ms`
    /// (docs/adr/0224 §1); an area's part has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "measure")))]
    pub ms: Option<Vec<Option<f64>>>,
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub pts: Vec<Vec2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub bulges: Option<Vec<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub holes: Option<Vec<RingGeometry>>,
    /// Each vertex's elevation, m, as many as `pts`; `null` for a vertex
    /// without one (not 0); absent when no vertex has one (docs/adr/0142).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub zs: Option<Vec<Option<f64>>>,
    /// A polyline's vertices' measures (M) on a route, as many as `pts`;
    /// `null` for a vertex without one; absent when none has one
    /// (docs/adr/0224 §1). A polygon has none: a route is a line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "measure")))]
    pub ms: Option<Vec<Option<f64>>>,
    /// A polygon's or a polyline's parts past its first, whose own are the
    /// fields above; absent for one part (docs/adr/0143, 0174). A
    /// polyline's parts are open, of two vertices or more, without holes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub parts: Option<Vec<AreaPart>>,
    /// A polyline whose last vertex joins its first: a closed line, not an
    /// area (docs/adr/0258 §1); its closing edge's bulge is the last of
    /// `bulges`. One part only. A polygon is a ring without saying so.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub closed: bool,
    /// A polyline's widths: the start and the end of the edge from each
    /// vertex, as many as `pts` (the last is the closing edge's, `[0, 0]`
    /// on an open polyline), in the drawing's length, not the paper's line
    /// weight (docs/adr/0258 §2). Absent: none, the line is drawn with its
    /// line weight. One part only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub widths: Option<Vec<[f64; 2]>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CircleEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub c: Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub c: Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub r: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub a0: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub c: Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub major: Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub ratio: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub t0: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub p: Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub p: Vec2,
    pub text: String,
    /// Metres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub height: f64,
    /// Degrees, counter-clockwise from east.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "scale")))]
    pub label_scale: Option<f64>,
    /// A multi-line text's box, line spacing and letter formats (docs/adr/0182).
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub paragraph: Paragraph,
    /// The style it follows and the face it is drawn in (docs/adr/0183 §2).
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub face: crate::TextFace,
    /// The curve its letters stand on (Eğri boyunca yazı, docs/adr/0196 §1),
    /// in its own frame; absent: a straight text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub path: Option<TextPath>,
    /// Its scales: drawn at the paper height it was given at every scale it takes part in, hidden
    /// at others (docs/adr/0323); absent: one height.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub annotative: Option<Box<crate::Annotative>>,
}

/// The curve a text's letters stand on (docs/adr/0196 §1). Its vertices are
/// in the text's own frame: `p` its start, x along the text's `rotation`, y
/// square to it on the left, metres. The first vertex is `p` itself and is
/// not written; `pts` are the ones after it. The frame carries the curve with
/// the text when it moves, turns or scales.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TextPath {
    /// The vertices after `p`, at least one.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub pts: Vec<Vec2>,
    /// Edge i's bulge, from vertex i to vertex i + 1 (vertex 0 is `p`):
    /// DXF's tan(θ/4), counter-clockwise positive; as many as `pts`. Absent:
    /// every edge straight.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub bulges: Option<Vec<f64>>,
}

impl TextPath {
    /// What is wrong with the curve alone (docs/adr/0196 §1): the refusal's
    /// words; none when it may be written.
    pub fn problem(&self) -> Option<String> {
        if self.pts.is_empty() {
            return Some(
                "Eğri boyunca yazının eğrisinde köşe yok. En az bir köşe verin ya da eğriyi kaldırın (düz yazı)."
                    .into(),
            );
        }
        let finite = self.pts.iter().all(|p| p.x.is_finite() && p.y.is_finite())
            && self.bulges.iter().flatten().all(|b| b.is_finite());
        if !finite {
            return Some(
                "Eğri boyunca yazının eğrisinde sonlu olmayan bir sayı var. Köşeleri ve kavisleri sonlu sayılarla verin."
                    .into(),
            );
        }
        if let Some(b) = &self.bulges
            && b.len() != self.pts.len()
        {
            return Some(format!(
                "Eğri boyunca yazının kavis sayısı ({}) köşe sayısından ({}) farklı. Her kenara bir kavis verin ya da kavisleri kaldırın (düz kenarlar).",
                b.len(),
                self.pts.len()
            ));
        }
        if self.pts.iter().all(|p| p.x == 0.0 && p.y == 0.0) {
            return Some(
                "Eğri boyunca yazının eğrisinin uzunluğu sıfır: bütün köşeleri yazının noktasında. Köşeleri yazının noktasından ayırın."
                    .into(),
            );
        }
        None
    }
}

/// What is wrong with a text's curve for the text (docs/adr/0196 §1): the
/// curve's own problem, then a text of more than one line (a line break, a
/// box width or a line spacing) or one linked to an object. None when it
/// may be written.
pub fn text_path_problem(
    path: &TextPath,
    text: &str,
    paragraph: &Paragraph,
    linked: bool,
) -> Option<String> {
    if let Some(why) = path.problem() {
        return Some(why);
    }
    if text.contains('\n') || paragraph.box_width.is_some() || paragraph.line_spacing.is_some() {
        return Some(
            "Eğri boyunca yazı tek satırdır: satır sonu, kutu genişliği ve satır aralığı olmaz. Yazıyı tek satır yapın ya da eğriyi kaldırın."
                .into(),
        );
    }
    if linked {
        return Some(
            "Nesneye bağlı yazının eğrisi olmaz. Önce bağı koparın ya da eğriyi kaldırın.".into(),
        );
    }
    None
}

/// What is wrong with a path's `closed` and `widths` (docs/adr/0258): an
/// area has neither (its ring is closed without saying so); a closed or a
/// wide polyline is of one part; a closed one has two vertices or more; its
/// widths are one pair for each vertex, finite and not below 0, the last pair
/// 0 on an open line (no edge runs from its last vertex) and not all 0 (a line
/// without widths has none). `None` when they may be written.
pub fn path_extras_problem(
    vertices: usize,
    closed: bool,
    widths: Option<&[[f64; 2]]>,
    parts: bool,
    area: bool,
) -> Option<String> {
    if area {
        if closed {
            return Some(
                "Kapalı alanın “closed” alanı olmaz: halkası zaten kapalıdır. Kapalı çizgi için çoklu çizgi kullanın."
                    .into(),
            );
        }
        if widths.is_some() {
            return Some(
                "Kapalı alanın genişliği olmaz; genişlik yalnız çoklu çizginindir.".into(),
            );
        }
        return None;
    }
    if (closed || widths.is_some()) && parts {
        return Some(
            "Kapalı ya da genişlikli çoklu çizgi tek parçalıdır. Önce Parçalara ayır ile parçalarına ayırın."
                .into(),
        );
    }
    if closed && vertices < 2 {
        return Some(format!(
            "Kapalı çoklu çizginin {vertices} köşesi var; en az iki olmalı."
        ));
    }
    if let Some(w) = widths {
        if w.len() != vertices {
            return Some(format!(
                "Genişlik sayısı ({}) köşe sayısından ({vertices}) farklı. Her köşeden çıkan kenara bir başlangıç ve bir bitiş genişliği verin.",
                w.len()
            ));
        }
        if let Some(i) = w
            .iter()
            .position(|[a, b]| !(a.is_finite() && b.is_finite() && *a >= 0.0 && *b >= 0.0))
        {
            return Some(format!(
                "{}. kenarın genişliği geçersiz: genişlik sonlu ve sıfır ya da daha büyük olmalı.",
                i + 1
            ));
        }
        if !closed && w.last().is_some_and(|[a, b]| *a != 0.0 || *b != 0.0) {
            return Some(
                "Açık çoklu çizginin son köşesinden kenar çıkmaz: son genişlik [0, 0] olmalı."
                    .into(),
            );
        }
        if w.iter().all(|[a, b]| *a == 0.0 && *b == 0.0) {
            return Some(
                "Bütün genişlikler sıfır: genişliksiz çizginin genişlik alanı yazılmaz.".into(),
            );
        }
    }
    None
}

impl PathEntity {
    /// [`path_extras_problem`] of this path, an area's (`area`) or a polyline's.
    pub fn extras_problem(&self, area: bool) -> Option<String> {
        path_extras_problem(
            self.pts.len(),
            self.closed,
            self.widths.as_deref(),
            self.parts.is_some(),
            area,
        )
    }
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub box_width: Option<f64>,
    /// The distance between the lines' baselines in 5/3 of the height (DXF's
    /// group 44); absent: 1. From `MIN_LINE_SPACING` to `MAX_LINE_SPACING`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 0.25, max = 4.0)))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub line_spacing: Option<f64>,
    /// The letters' formats: ranges of the text's Unicode scalar values (not
    /// UTF-16 units), in order, not overlapping, each with a format; a run
    /// with no format is not written, and touching runs of one format are one.
    /// Absent: none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<TextRun>>", optional))]
    pub runs: Vec<TextRun>,
    /// Its indents, tab stops and columns (docs/adr/0324 §1); absent: none, a tab every
    /// `DEFAULT_TAB` heights, one column.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layout: Option<Box<ParagraphLayout>>,
    /// Ranges of its letters bound to a source (docs/adr/0325 §1), in order, not overlapping; absent: none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<crate::TextField>>", optional))]
    pub fields: Vec<crate::TextField>,
}

/// A multi-line text's indents, tab stops and columns (docs/adr/0324 §1), in its height (MTEXT's `\p` codes):
/// every paragraph's first line starts `leftIndent + indent` heights from the box's left, its other lines
/// `leftIndent`; a tab goes to the next stop past where it is, a stop every `DEFAULT_TAB` heights after the last
/// given; columns take the lines in turn, each `columns.height` metres tall.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ParagraphLayout {
    /// A paragraph's first line from the others', heights; below 0 a hanging indent (a list's). Absent: 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub indent: Option<f64>,
    /// Every line but a paragraph's first from the box's left, heights. Absent: 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub left_indent: Option<f64>,
    /// The tab stops from the box's left, heights, ascending, each over 0.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<f64>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub tabs: Vec<f64>,
    /// Static columns; a text in columns has a box.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub columns: Option<TextColumns>,
}

/// A multi-line text's static columns (docs/adr/0324 §1): `count` columns of the box's width, `gap` heights
/// apart, each `height` metres tall; lines past the last column's height go on down in it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TextColumns {
    #[cfg_attr(feature = "schema", schemars(range(min = 2, max = 16)))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub count: u32,
    /// The space between two columns, heights.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub gap: f64,
    /// A column's height, metres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub height: f64,
}

/// A tab stop every this many heights past the last given one (MTEXT's own).
pub const DEFAULT_TAB: f64 = 4.0;
/// The largest indent, tab stop or column gap, heights.
pub const MAX_PARAGRAPH_HEIGHTS: f64 = 1000.0;
/// The most tab stops a text keeps.
pub const MAX_TABS: usize = 32;

impl ParagraphLayout {
    /// Nothing set: none at all.
    pub fn is_plain(&self) -> bool {
        self.indent.is_none_or(|i| i == 0.0)
            && self.left_indent.is_none_or(|i| i == 0.0)
            && self.tabs.is_empty()
            && self.columns.is_none()
    }

    /// What is wrong with it for a text with a box or none: the field and the words.
    pub fn problem(&self, boxed: bool) -> Option<(&'static str, String)> {
        let fits = |v: f64| v.is_finite() && v.abs() <= MAX_PARAGRAPH_HEIGHTS;
        let left = self.left_indent.unwrap_or(0.0);
        if !fits(left) || left < 0.0 {
            return Some((
                "layout.leftIndent",
                format!("Sol girinti 0 ile {MAX_PARAGRAPH_HEIGHTS} yükseklik arasında olmalı; {left} verildi."),
            ));
        }
        let first = self.indent.unwrap_or(0.0);
        if !fits(first) || left + first < 0.0 {
            return Some((
                "layout.indent",
                format!(
                    "İlk satırın girintisi sol girintiden fazla geri gidemez (ikisinin toplamı sıfırdan küçük olamaz); {first} verildi."
                ),
            ));
        }
        if self.indent == Some(0.0) || self.left_indent == Some(0.0) {
            return Some(("layout", "Sıfır girinti yazılmaz; alanı kaldırın.".to_owned()));
        }
        if self.tabs.len() > MAX_TABS {
            return Some(("layout.tabs", format!("En çok {MAX_TABS} sekme durağı olur; {} verildi.", self.tabs.len())));
        }
        let mut last = 0.0;
        for t in &self.tabs {
            if !fits(*t) || *t <= last {
                return Some((
                    "layout.tabs",
                    "Sekme durakları sıfırdan büyük, küçükten büyüğe ve birer kez olmalı.".to_owned(),
                ));
            }
            last = *t;
        }
        if let Some(c) = &self.columns {
            if !boxed {
                return Some((
                    "layout.columns",
                    "Sütunlar kutu genişliği ister: önce yazıya kutu genişliği verin.".to_owned(),
                ));
            }
            if !(2..=16).contains(&c.count) {
                return Some(("layout.columns.count", format!("Sütun sayısı 2 ile 16 arasında olmalı; {} verildi.", c.count)));
            }
            if !fits(c.gap) || c.gap < 0.0 {
                return Some(("layout.columns.gap", format!("Sütun aralığı sıfır ya da pozitif olmalı; {} verildi.", c.gap)));
            }
            if !(c.height.is_finite() && c.height > 0.0) {
                return Some((
                    "layout.columns.height",
                    format!("Sütun yüksekliği sıfırdan büyük olmalı; {} verildi.", c.height),
                ));
            }
        }
        None
    }
}

impl Paragraph {
    /// No box, no spacing, no formats, no layout: a text as it always was.
    pub fn is_plain(&self) -> bool {
        self.box_width.is_none()
            && self.line_spacing.is_none()
            && self.runs.is_empty()
            && self.layout.is_none()
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
        if let Some(l) = &self.layout {
            if l.is_plain() {
                return Some(("layout", "Boş paragraf düzeni yazılmaz; alanı kaldırın.".to_owned()));
            }
            if let Some(p) = l.problem(self.box_width.is_some()) {
                return Some(p);
            }
        }
        if let Some(words) = self.runs_problem(text) {
            return Some(("runs", words));
        }
        crate::text_fields_problem(&self.fields, text).map(|words| ("fields", words))
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
            // A stacked run (docs/adr/0324 §1): its letters hold the separator, `/`, `#` or `^`.
            if r.script == Some(TextScript::Stack) {
                let letters: Vec<char> = text.chars().collect();
                let inside = &letters[r.start as usize..(r.end as usize).min(letters.len())];
                if !inside.iter().any(|c| matches!(c, '/' | '#' | '^')) {
                    return Some(format!(
                        "{n}. biçim dilimi kesir, ama harflerinde ayıraç yok: pay ile paydayı /, # ya da ^ ile ayırın (1/2)."
                    ));
                }
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub start: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
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
    /// A stacked fraction (docs/adr/0324 §1): the run's letters before its first `/`, `#` or `^` raised over
    /// those after it; `/` a horizontal bar, `#` a slanted one beside them, `^` none (a tolerance).
    Stack,
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
/// transform is a similarity, shapes keeping their kind, unless the insert
/// has a y factor of its own (docs/adr/0315 §3): then an affine, a circle
/// becoming the exact ellipse of its image. The insert's own attributes
/// carry the values of the definition's attribute definitions.
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub p: Vec2,
    /// Positive and finite; 1 is the definition's size. With `yScale`, the factor along the definition's x axis.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub scale: f64,
    /// The factor along the definition's y axis when it is not `scale` (docs/adr/0315 §3), positive and finite;
    /// absent: `scale`. A negative factor of a DXF is a mirror and a turn here.
    #[serde(default, rename = "yScale", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, rename = "yScale"))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub y_scale: Option<f64>,
    /// Radians, counter-clockwise from east.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub rotation: f64,
    /// Mirrored in the definition's x axis, before the turn.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub mirror: bool,
    /// Where this insert shows a movable attribute (docs/adr/0317 §1): its tag's text at this point of the
    /// definition's coordinates rather than the definition's. Only a movable attribute's tag; absent: every text where
    /// the definition puts it.
    #[serde(
        default,
        rename = "attributePlaces",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    #[cfg_attr(
        feature = "ts",
        ts(
            as = "Option<BTreeMap<String, Vec2>>",
            optional,
            rename = "attributePlaces"
        )
    )]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub attribute_places: BTreeMap<String, Vec2>,
    /// Its dynamic block's parameter values that differ from their defaults (docs/adr/0318 §3), by parameter name;
    /// a lookup's is not kept (its row's values are).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(
        feature = "ts",
        ts(
            as = "Option<BTreeMap<String, crate::block_dynamic::ParamValue>>",
            optional
        )
    )]
    pub parameters: BTreeMap<String, crate::block_dynamic::ParamValue>,
}

/// A block definition (docs/adr/0144): objects drawn once, placed many
/// times. Its objects' ids are local to it, once each and kept by every edit
/// of it, so a reference to a piece follows it (docs/adr/0315 §1). An object
/// on the block's layer (`""`, the DXF's layer 0) is drawn on its insert's
/// layer; one on a layer of its own shows only while that layer does
/// (docs/adr/0315 §2). A part of its look it has is its own, a `byLayer` part
/// its layer's (its insert's layer on the block's layer), any other part its
/// insert's (BYBLOCK).
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub base: Vec2,
    pub entities: Vec<Entity>,
    /// The texts an insert shows from its attributes (ATTDEF).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<AttributeDefinition>>", optional))]
    pub attributes: Vec<AttributeDefinition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub description: Option<String>,
    /// The unit it was drawn in (docs/adr/0315 §4): a unit, its geometry being metres as the drawing's (an insert
    /// keeps its size anywhere); `unitless`, its numbers being drawing units (an insert is scaled by the drawing's
    /// unit). Absent: a real-size block of an unknown unit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub units: Option<BlockUnits>,
    /// Its library card (docs/adr/0317 §4): category, tags and version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub card: Option<BlockCard>,
    /// Its parameters and actions (docs/adr/0318): one definition at many sizes and looks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub dynamic: Option<crate::block_dynamic::DynamicBlock>,
    /// An external reference's source (docs/adr/0348 §1): its objects are that drawing's as last loaded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub xref: Option<crate::xref::XrefSource>,
}

/// A block's library card (docs/adr/0317 §4): what the Bloklar panel's cards and search show beside its name, its
/// description, its size and its attributes.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct BlockCard {
    /// A path of categories by `/` (`Mimari/Kapılar`); absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub category: Option<String>,
    /// Words it is found by, each once, Turkish case folded.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub tags: Vec<String>,
    /// How many times its definition has changed since it was made: 1 when made; each change of its objects, base,
    /// attributes or unit adds one.
    #[cfg_attr(feature = "schema", schemars(range(min = 1)))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub version: u32,
}

/// The unit a block was drawn in (docs/adr/0315 §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum BlockUnits {
    Mm,
    Cm,
    M,
    In,
    Ft,
    /// Its numbers are the drawing's units: a 1 is a millimetre in a millimetre drawing, a metre in a metre one.
    Unitless,
}

impl BlockUnits {
    /// The drawing unit it is; none when unitless.
    pub fn unit(self) -> Option<crate::DrawingUnit> {
        use crate::DrawingUnit as U;
        match self {
            Self::Mm => Some(U::Mm),
            Self::Cm => Some(U::Cm),
            Self::M => Some(U::M),
            Self::In => Some(U::In),
            Self::Ft => Some(U::Ft),
            Self::Unitless => None,
        }
    }

    /// A drawing unit as a block's.
    pub fn of(u: crate::DrawingUnit) -> Self {
        use crate::DrawingUnit as U;
        match u {
            U::Mm => Self::Mm,
            U::Cm => Self::Cm,
            U::M => Self::M,
            U::In => Self::In,
            U::Ft => Self::Ft,
        }
    }

    /// Its Turkish word: `mm`, `cm`, `m`, `in`, `ft` or `birimsiz`.
    pub fn word(self) -> &'static str {
        self.unit().map_or("birimsiz", |u| u.mark())
    }
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub p: Vec2,
    /// Metres, in the definition's size.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub height: f64,
    /// Degrees, counter-clockwise from east, as a text's.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub rotation: f64,
    /// Where `p` is on the text, as a text's (docs/adr/0145).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub align: Option<TextAlign>,
    /// The letters' width times this, as a text's (docs/adr/0145).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 0.0, max = 100.0)))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub width_factor: Option<f64>,
    /// Its options (docs/adr/0317 §1), each absent when off.
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub options: AttributeOptions,
}

/// An attribute definition's options (docs/adr/0317 §1; AutoCAD's ATTDEF modes, DXF group 70).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AttributeOptions {
    /// Not drawn: its value is kept, listed and scheduled (DXF 70 bit 1).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub invisible: bool,
    /// Its value is the definition's `value` in every insert; an insert holds none of its own (bit 2).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub constant: bool,
    /// Blok ekle asks for it twice, the two the same (bit 4).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub verify: bool,
    /// Blok ekle takes its default without asking (bit 8).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub preset: bool,
    /// An insert may move its text (`attributePlaces`); off: locked where the definition puts it (bit 16 set).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub movable: bool,
    /// Its value may hold line breaks, each a line; wrapped to `boxWidth` when given (an MTEXT attribute).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub multiline: bool,
    /// A multiline attribute's box width, metres in the definition's size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub box_width: Option<f64>,
    /// A field (docs/adr/0317 §1): a template whose `{ifade}` parts are filled for each insert (its attributes, its
    /// place, the project's `@` values); the commands that place, sync and update fields write it as the value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub field: Option<String>,
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
    /// Merkez işareti (KCAD schema 62, docs/adr/0322): the cross at the centre `a` of the curve through `b`, `height`
    /// across, turned `angle` degrees from `a`→`b`; its axis lines reach `offset` past the curve.
    CenterMark,
    /// Eksen çizgisi (docs/adr/0322): the axis between two straight edges, from `a` to `b` (the middles of their
    /// matched ends), `offset` past each end.
    CenterLine,
    /// Kot (docs/adr/0322): the elevation `za` of the point `a`, its mark on `a` and its value on the shoulder at `b`.
    Elevation,
}

impl DimensionStyle {
    /// Every style, in the contract's order (the typed columns number them so).
    pub const ALL: [DimensionStyle; 13] = [
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
        DimensionStyle::CenterMark,
        DimensionStyle::CenterLine,
        DimensionStyle::Elevation,
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
            DimensionStyle::CenterMark => "centerMark",
            DimensionStyle::CenterLine => "centerLine",
            DimensionStyle::Elevation => "elevation",
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

    /// Whether it came with KCAD schema 62 (docs/adr/0322).
    pub fn is_schema_62(self) -> bool {
        matches!(
            self,
            DimensionStyle::CenterMark | DimensionStyle::CenterLine | DimensionStyle::Elevation
        )
    }

    /// Whether it writes no value (docs/adr/0322): the centre mark and the axis.
    pub fn is_mark(self) -> bool {
        matches!(
            self,
            DimensionStyle::CenterMark | DimensionStyle::CenterLine
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub a: Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub b: Vec2,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub offset: f64,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub angle: Option<f64>,
    /// Angular: the vertex. Arc length: the arc's centre. Jogged: the centre
    /// the line starts from (docs/adr/0147).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub c: Option<Vec2>,
    /// The value is drawn over the drawing's background (docs/adr/0147, as a
    /// text's mask, docs/adr/0145).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub mask: bool,
    /// Slope: the two points' elevations, metres (docs/adr/0147).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub za: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
    pub zb: Option<f64>,
    /// The objects its points follow (docs/adr/0249): an associative dimension; none, free points.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub refs: Option<Box<crate::DimensionRefs>>,
    /// Its value moved from its place (docs/adr/0321 §1).
    #[serde(default, rename = "textShift", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub text_shift: Option<crate::TextShift>,
    /// The gaps in its lines where other objects cross them (docs/adr/0321 §1).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        feature = "ts",
        ts(as = "Option<Vec<crate::DimensionBreak>>", optional)
    )]
    pub breaks: Vec<crate::DimensionBreak>,
    /// The style it follows, its arrowheads, sizes and value's writing (docs/adr/0183 §3).
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub look: crate::DimensionLook,
    /// Its scales: drawn at the paper height it was given at every scale it takes part in, hidden
    /// at others (docs/adr/0323); absent: one height.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub annotative: Option<Box<crate::Annotative>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct HatchEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub ring: Vec<Vec2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub holes: Option<Vec<Vec<Vec2>>>,
    pub pattern: crate::HatchPattern,
    /// The objects its region follows (docs/adr/0186 §6); none: it stays as drawn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub assoc: Option<crate::HatchAssoc>,
    /// Its boundary as its source holds it (docs/adr/0328 §1): the outer loop, then the holes', lines, arcs,
    /// ellipses' arcs and NURBS curves exact; `ring` and `holes` are drawn from it (arcs and curves as chords). None:
    /// the ring and holes are all it has.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub boundary: Option<Vec<crate::HatchLoop>>,
    /// Its scales (docs/adr/0331 §3): an annotative hatch's pattern is drawn at each scale it takes part in as large
    /// on paper as at its own; absent: one pattern for every scale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub annotative: Option<Box<crate::Annotative>>,
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
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub pts: Vec<Vec2>,
    /// The note, one line; absent: the arrow alone, no landing and no note.
    /// Never empty: no note is the field's absence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub text: Option<String>,
    /// The note's height, metres, finite and over 0; the arrowhead and the
    /// landing are measured by it.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub height: f64,
    /// The note's and the landing's direction, degrees counter-clockwise from east.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub rotation: f64,
    /// The arrowhead; absent: a filled arrow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub arrow: Option<LeaderArrow>,
    /// The arrowhead's length, times the note's height (docs/adr/0205 §7);
    /// absent: 1. From `MIN_LEADER_ARROW` to `MAX_LEADER_ARROW`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub arrow_size: Option<f64>,
    /// The note's box is filled with the drawing area's colour before the
    /// note is drawn, as a text's mask (docs/adr/0145).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub mask: bool,
    /// Its scales: drawn at the paper height it was given at every scale it takes part in, hidden
    /// at others (docs/adr/0323); absent: one height.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub annotative: Option<Box<crate::Annotative>>,
    /// Its other arms (docs/adr/0326 §1): each from its own tip through its bends to the leader's last vertex, where
    /// the landing starts; the arrowhead, the note and the look the first arm's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<LeaderArm>>", optional))]
    pub arms: Vec<LeaderArm>,
    /// The place on an object its first arm's tip follows (docs/adr/0326 §3); absent: a free tip.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tip: Option<crate::SubRef>,
    /// Its content blocks (docs/adr/0326 §2): inserts standing in a row at its landing's end, by persistent id; a
    /// leader with blocks has no note.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<crate::identity::EntityId>>", optional))]
    pub blocks: Vec<crate::identity::EntityId>,
    /// The leader style it follows (docs/adr/0326 §4), by its id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub leader_style: Option<String>,
}

/// A leader's arm after its first (docs/adr/0326 §1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LeaderArm {
    /// At least one: its tip first, then its bends; it ends at the leader's last vertex.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub pts: Vec<Vec2>,
    /// The place on an object its tip follows; absent: a free tip.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub tip: Option<crate::SubRef>,
}

/// The most arms a leader has.
pub const MAX_LEADER_ARMS: usize = 64;

/// What is wrong with a leader's arms, blocks and note together (docs/adr/0326): the field and the refusal's words.
pub fn leader_problem(l: &LeaderEntity) -> Option<(&'static str, String)> {
    if l.arms.len() + 1 > MAX_LEADER_ARMS {
        return Some(("arms", format!("Bir kılavuzun en çok {MAX_LEADER_ARMS} kolu olur; {} verildi.", l.arms.len() + 1)));
    }
    if let Some(i) = l.arms.iter().position(|a| a.pts.is_empty()) {
        return Some(("arms", format!("{}. kolun ucu yok: her kolun en az bir noktası olmalı.", i + 2)));
    }
    if l.arms.iter().flat_map(|a| &a.pts).any(|p| !(p.x.is_finite() && p.y.is_finite())) {
        return Some(("arms", "Kolun noktaları sonlu olmalı.".to_owned()));
    }
    if !l.blocks.is_empty() && l.text.is_some() {
        return Some(("blocks", "Blok içerikli kılavuzun notu olmaz: notu ya da blokları kaldırın.".to_owned()));
    }
    let mut seen = std::collections::HashSet::new();
    if l.blocks.iter().any(|b| !seen.insert(*b)) {
        return Some(("blocks", "Kılavuzun bir bloğu iki kez verilmiş.".to_owned()));
    }
    None
}

/// The shortest and the longest a leader's arrowhead may be, times its
/// note's height (docs/adr/0205 §7).
pub const MIN_LEADER_ARROW: f64 = 0.1;
pub const MAX_LEADER_ARROW: f64 = 10.0;

/// Whether `size` may be a leader's arrowhead size.
pub fn leader_arrow_holds(size: f64) -> bool {
    size.is_finite() && (MIN_LEADER_ARROW..=MAX_LEADER_ARROW).contains(&size)
}

/// A leader's arrowhead other than the filled triangle (Dolu üçgen,
/// AutoCAD's Closed filled), which is no value but the field's absence: a
/// leader has one spelling (docs/adr/0146 §1). The first three are the
/// first leaders'; the others AutoCAD's arrowheads (docs/adr/0205 §7, `.kcad`
/// schema 30).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LeaderArrow {
    /// Açık ok: the filled triangle's two sides (Open).
    Open,
    /// Dolu nokta: a filled dot (Dot).
    Dot,
    /// Yok: no arrowhead, the line ends at the tip (None).
    None,
    /// Boş üçgen: the triangle's outline (Closed blank).
    Closed,
    /// İnce açık ok: two sides 15° off the line (Open 30).
    Open30,
    /// Dik açık ok: two sides square to each other (Open 90, Right angle).
    Open90,
    /// Küçük nokta (Dot small).
    DotSmall,
    /// Boş nokta: a ring (Dot blank).
    DotBlank,
    /// Eğik çizgi: a tick across the tip (Oblique).
    Oblique,
    /// Mimari çentik: a heavy tick (Architectural tick).
    ArchTick,
    /// Dolu kare (Box filled).
    BoxFilled,
    /// Boş kare (Box blank).
    BoxBlank,
    /// Dayanak üçgeni: a filled triangle standing on the tip (Datum triangle filled).
    DatumFilled,
}

impl LeaderArrow {
    /// Every arrowhead, in the order of the enum (the typed columns number them so).
    pub const ALL: [LeaderArrow; 13] = [
        LeaderArrow::Open,
        LeaderArrow::Dot,
        LeaderArrow::None,
        LeaderArrow::Closed,
        LeaderArrow::Open30,
        LeaderArrow::Open90,
        LeaderArrow::DotSmall,
        LeaderArrow::DotBlank,
        LeaderArrow::Oblique,
        LeaderArrow::ArchTick,
        LeaderArrow::BoxFilled,
        LeaderArrow::BoxBlank,
        LeaderArrow::DatumFilled,
    ];

    /// The name files and the wire use.
    pub fn name(self) -> &'static str {
        match self {
            LeaderArrow::Open => "open",
            LeaderArrow::Dot => "dot",
            LeaderArrow::None => "none",
            LeaderArrow::Closed => "closed",
            LeaderArrow::Open30 => "open30",
            LeaderArrow::Open90 => "open90",
            LeaderArrow::DotSmall => "dotSmall",
            LeaderArrow::DotBlank => "dotBlank",
            LeaderArrow::Oblique => "oblique",
            LeaderArrow::ArchTick => "archTick",
            LeaderArrow::BoxFilled => "boxFilled",
            LeaderArrow::BoxBlank => "boxBlank",
            LeaderArrow::DatumFilled => "datumFilled",
        }
    }

    /// Whether it is one of AutoCAD's arrowheads the first leaders did not
    /// have (`.kcad` schema 30).
    pub fn is_added(self) -> bool {
        !matches!(
            self,
            LeaderArrow::Open | LeaderArrow::Dot | LeaderArrow::None
        )
    }

    /// Its name as the interface says it.
    pub fn label(self) -> &'static str {
        match self {
            LeaderArrow::Open => "Açık ok",
            LeaderArrow::Dot => "Dolu nokta",
            LeaderArrow::None => "Yok",
            LeaderArrow::Closed => "Boş üçgen",
            LeaderArrow::Open30 => "İnce açık ok",
            LeaderArrow::Open90 => "Dik açık ok",
            LeaderArrow::DotSmall => "Küçük nokta",
            LeaderArrow::DotBlank => "Boş nokta",
            LeaderArrow::Oblique => "Eğik çizgi",
            LeaderArrow::ArchTick => "Mimari çentik",
            LeaderArrow::BoxFilled => "Dolu kare",
            LeaderArrow::BoxBlank => "Boş kare",
            LeaderArrow::DatumFilled => "Dayanak üçgeni",
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
    /// A general NURBS curve: degree, control points, weights and knots (docs/adr/0259).
    Nurbs(crate::nurbs::NurbsEntity),
    Xline(ConstructionEntity),
    Ray(ConstructionEntity),
    Text(TextEntity),
    Dimension(DimensionEntity),
    Hatch(HatchEntity),
    /// A block placed in the drawing (docs/adr/0144).
    Insert(InsertEntity),
    /// An arrow with a note (docs/adr/0146).
    Leader(LeaderEntity),
    /// Rows and columns of cells (docs/adr/0184).
    Table(crate::TableEntity),
    /// A picture (docs/adr/0192).
    Image(crate::ImageEntity),
    /// An orthophoto, a scanned sheet or an elevation model (docs/adr/0204).
    Raster(crate::RasterEntity),
    /// A point cloud: LAS, LAZ, COPC or text files (docs/adr/0207).
    PointCloud(crate::PointCloudEntity),
    /// A wall (docs/adr/0339): its path, width, justification and components; its faces drawn from them.
    Wall(crate::WallEntity),
    /// A door, a window or a bare opening in a wall (docs/adr/0341): its wall's id, its place along it and its
    /// jambs' points.
    Opening(crate::OpeningEntity),
    Space(crate::SpaceEntity),
    /// A grid (docs/adr/0343): its line, label and bubbles.
    Grid(crate::GridEntity),
    /// A column (docs/adr/0343): its centre, outline and the grids it stands on.
    Column(crate::ColumnEntity),
    /// A stair (docs/adr/0343): its walking line and its risers, treads and landings.
    Stair(crate::StairEntity),
    /// A ramp (docs/adr/0343): its centre line, width, rise and landings.
    Ramp(crate::RampEntity),
    /// A tag (docs/adr/0344): its object, its words and frame.
    Tag(crate::TagEntity),
    /// A view mark (docs/adr/0345): a section, elevation or detail mark pointing at a view.
    ViewMark(crate::ViewMarkEntity),
    /// A record without a geometry (docs/adr/0410): a table's row, with the base's attributes,
    /// attachments and label; not drawn, picked or snapped to.
    Record(RecordEntity),
}

/// A record without a geometry (docs/adr/0410 §1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RecordEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
}

impl Entity {
    /// Whether it has a geometry: every object but a record (docs/adr/0410).
    pub fn has_geometry(&self) -> bool {
        !matches!(self, Entity::Record(_))
    }

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
                | Entity::Image(_)
                | Entity::Raster(_)
                | Entity::PointCloud(_)
                | Entity::Record(_)
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
            Entity::Nurbs(e) => &e.base,
            Entity::Wall(e) => &e.base,
            Entity::Opening(e) => &e.base,
            Entity::Space(e) => &e.base,
            Entity::Grid(e) => &e.base,
            Entity::Column(e) => &e.base,
            Entity::Stair(e) => &e.base,
            Entity::Ramp(e) => &e.base,
            Entity::Tag(e) => &e.base,
            Entity::ViewMark(e) => &e.base,
            Entity::Xline(e) | Entity::Ray(e) => &e.base,
            Entity::Text(e) => &e.base,
            Entity::Dimension(e) => &e.base,
            Entity::Hatch(e) => &e.base,
            Entity::Insert(e) => &e.base,
            Entity::Leader(e) => &e.base,
            Entity::Table(e) => &e.base,
            Entity::Image(e) => &e.base,
            Entity::Raster(e) => &e.base,
            Entity::PointCloud(e) => &e.base,
            Entity::Record(e) => &e.base,
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
            Entity::Nurbs(e) => &mut e.base,
            Entity::Wall(e) => &mut e.base,
            Entity::Opening(e) => &mut e.base,
            Entity::Space(e) => &mut e.base,
            Entity::Grid(e) => &mut e.base,
            Entity::Column(e) => &mut e.base,
            Entity::Stair(e) => &mut e.base,
            Entity::Ramp(e) => &mut e.base,
            Entity::Tag(e) => &mut e.base,
            Entity::ViewMark(e) => &mut e.base,
            Entity::Xline(e) | Entity::Ray(e) => &mut e.base,
            Entity::Text(e) => &mut e.base,
            Entity::Dimension(e) => &mut e.base,
            Entity::Hatch(e) => &mut e.base,
            Entity::Insert(e) => &mut e.base,
            Entity::Leader(e) => &mut e.base,
            Entity::Table(e) => &mut e.base,
            Entity::Image(e) => &mut e.base,
            Entity::Raster(e) => &mut e.base,
            Entity::PointCloud(e) => &mut e.base,
            Entity::Record(e) => &mut e.base,
        }
    }

    /// The `kind` tag as written in files and on the wire.
    /// Its own phase (docs/adr/0346 §2): a wall's, a space's, a grid's, a column's, a stair's or a ramp's.
    pub fn own_phase(&self) -> Option<crate::ArchPhase> {
        match self {
            Entity::Wall(x) => x.wall.arch.phase,
            Entity::Space(x) => x.space.arch.phase,
            Entity::Grid(x) => x.grid.arch.phase,
            Entity::Column(x) => x.column.arch.phase,
            Entity::Stair(x) => x.stair.arch.phase,
            Entity::Ramp(x) => x.ramp.arch.phase,
            _ => None,
        }
    }

    /// Its phase as a phase view reads it (docs/adr/0346 §2): its own, an opening's its wall's, a tag's its object's
    /// (`by_uid` finds an object by its persistent id).
    pub fn phase<'a>(&'a self, by_uid: impl Fn(&str) -> Option<&'a Entity>) -> Option<crate::ArchPhase> {
        match self {
            Entity::Opening(o) => by_uid(&o.opening.host).and_then(|w| w.own_phase()),
            Entity::Tag(t) => by_uid(&t.tag.of).and_then(|e| match e {
                Entity::Opening(o) => by_uid(&o.opening.host).and_then(|w| w.own_phase()),
                other => other.own_phase(),
            }),
            other => other.own_phase(),
        }
    }

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
            Entity::Nurbs(_) => "nurbs",
            Entity::Wall(_) => "wall",
            Entity::Opening(_) => "opening",
            Entity::Space(_) => "space",
            Entity::Grid(_) => "grid",
            Entity::Column(_) => "column",
            Entity::Stair(_) => "stair",
            Entity::Ramp(_) => "ramp",
            Entity::Tag(_) => "tag",
            Entity::ViewMark(_) => "viewmark",
            Entity::Xline(_) => "xline",
            Entity::Ray(_) => "ray",
            Entity::Text(_) => "text",
            Entity::Dimension(_) => "dimension",
            Entity::Hatch(_) => "hatch",
            Entity::Insert(_) => "insert",
            Entity::Leader(_) => "leader",
            Entity::Table(_) => "table",
            Entity::Image(_) => "image",
            Entity::Raster(_) => "raster",
            Entity::PointCloud(_) => "pointcloud",
            Entity::Record(_) => "record",
        }
    }
}
