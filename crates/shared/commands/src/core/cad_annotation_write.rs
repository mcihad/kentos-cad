//! Written annotations as one product command (docs/adr/0622 §9, TODOS.md `UX-14`): Koordinat yaz, Köşelere
//! koordinat yaz, Km yaz, Eğri boyunca yazı, Hızlı ölçü and Etiketleri yazıya çevir. The caller gives the places or the objects by persistent id and the options the user
//! sets; the command lays the texts, leaders, ticks, sections and points out with the shared core
//! (`ops::coordinate_labels`, `ops::stationing`) and writes them as `cad.entities.create` writes, one undo step named
//! after the tool. The tools write through it; Python and an agent ask the same.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::REVISION_TEXT;
use crate::entity::Vec2;

/// Writes annotations.
pub const CAD_ANNOTATIONS_WRITE: &str = "cad.annotations.write";
pub const CAD_ANNOTATIONS_WRITE_VERSION: u32 = 1;

/// Where a coordinate label leans (Yön).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LabelLean {
    /// Away from the middle of its object's box; north-east without one.
    #[default]
    Auto,
    NorthEast,
    NorthWest,
    SouthWest,
    SouthEast,
}

/// The side the km text goes on (Yazı), or none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum StationText {
    Left,
    Right,
    None,
}

/// What is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum AnnotationWork {
    /// Koordinat yaz and Köşelere koordinat yaz (docs/adr/0185): a label at each place `at`, its elevation and name the
    /// visible objects' there, or at every vertex of the objects `ids`; `template`'s lines (`{Y}`, `{X}`, `{Z}`,
    /// `{ad}`; the project type's first when none), `decimals` (the project's when none), a leader (`leader`) and
    /// the way it leans (`lean`).
    #[serde(rename_all = "camelCase")]
    Coordinates {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        at: Option<Vec<Vec2>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        ids: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        template: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        decimals: Option<u8>,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
        leader: bool,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<LabelLean>"))]
        lean: LabelLean,
    },
    /// Km yaz (docs/adr/0189): the route `id`'s stations at the multiples of `interval` of its km (from `start` at
    /// its first point, or its last with `reverse`; its measures with `measures` when it has them), its ends too with
    /// `ends`; at each the km text on `text`'s side, a tick (`tick`), a cross-section of half width `section` and a
    /// point `point` off the way (the right positive), each as asked.
    #[serde(rename_all = "camelCase")]
    Stations {
        id: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        interval: f64,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<f64>"))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        start: f64,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
        reverse: bool,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
        measures: bool,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
        ends: bool,
        text: StationText,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
        tick: bool,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<f64>"))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        section: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        point: Option<f64>,
    },
    /// Eğri boyunca yazı (docs/adr/0196 §4): `text` on the curve `id` (a line, an arc, a circle, a polyline's part, an
    /// area's ring or hole, an ellipse, a spline: the path nearest `at`), on the piece its letters take, `at` being
    /// what `align`'s share along says of it (its start, middle or end) and its share up where the letters stand
    /// (over, on or under the curve; bottom centre when none); the piece turned so that the text reads.
    Along {
        id: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        at: Vec2,
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        align: Option<crate::TextAlign>,
    },
    /// Hızlı ölçü (docs/adr/0147 §7): the straight edges of the lines, polylines and areas `ids` measured at once,
    /// an aligned dimension each, an arc length each arc; out of an area, into a hole, on `at`'s side of an open path,
    /// as far from the edges as `at` is (or `distance`); an edge two objects share once. The dimension style `style`
    /// by name (Standart when none) gives the look and, without `height`, the value's height; `mask` its background.
    Dimensions {
        ids: Vec<String>,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        at: Vec2,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        distance: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        style: Option<String>,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
        mask: bool,
    },
    /// Etiketleri yazıya çevir (docs/adr/0175, 0212 §4): the labels of the objects `ids` (none: every object on a
    /// visible layer that labels, past texts, dimensions and leaders) written where the label engine places them at
    /// 1:`scale` (the project's when none): a one-line label a text, a stacked one a multi-line text, a curved one a
    /// text along a curve, a callout a line; with `every` the labels with no free place too, with `mask` the texts
    /// masked, with `linked` each text tied to its object (`labelOf`, `labelScale`).
    Labels {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        ids: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
        scale: Option<f64>,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
        every: bool,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
        mask: bool,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
        linked: bool,
    },
}

/// `cad.annotations.write`'s input: the work, the layer and the texts' look.
///
/// Refusals, in this order: `not_finite`, `invalid_uid`, `entity_not_found`, then the work's own (`invalid_input`,
/// `not_a_path`, `no_route`, `invalid_distance`, `nothing_derived` …), then `cad.entities.create`'s.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AnnotationsWrite {
    /// The layer written to; a standard layer the drawing lacks (`yazi` …) is opened in the same step.
    pub layer_id: String,
    pub work: AnnotationWork,
    /// The texts' paper height, mm (the project's for the kind when none); in the drawing it is this times the scale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub face: Option<crate::TextFace>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub width_factor: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// What `cad.annotations.write` wrote.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AnnotationsWritten {
    /// The new objects' slots and persistent ids, in order.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub ids: Vec<u32>,
    pub uids: Vec<String>,
    /// The labels or the stations written.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub count: u32,
    /// Köşelere koordinat yaz: the places the template left no line.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub skipped: u32,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
