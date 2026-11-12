//! Patlat's semantic command of the product command catalog (docs/adr/0302; AutoCAD's EXPLODE): objects named by
//! their persistent ids broken one level into simpler ones, the pieces computed by the command rather than given.
//!
//! The pieces are the shared core's (`ops::explode`, a block's `Blocks::explode`): a path its lines and arcs, an
//! area its edges (holes and parts too), a dimension its lines, arrowheads and text, a patterned hatch its lines, a
//! leader its line, arrowhead and note, a table its lines and words, a block's insert its definition's objects. An
//! item of an associative array (docs/adr/0300) explodes its whole array: every item's insert, and the array is
//! gone (§2). A curve (circle, arc, ellipse, elliptical arc, spline, NURBS curve) becomes a polyline whose vertices lie
//! on it, as dense as `density` asks (docs/adr/0308).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};

/// Explodes objects one level.
pub const CAD_ENTITIES_EXPLODE: &str = "cad.entities.explode";
pub const CAD_ENTITIES_EXPLODE_VERSION: u32 = 1;

/// A circle's, an arc's or an ellipse's chords' largest distance from it when no density is given, metres
/// (docs/adr/0308 §2).
pub const EXPLODE_DEVIATION: f64 = 0.01;
/// The most vertices one curve explodes into (docs/adr/0308 §2).
pub const MAX_EXPLODE_VERTICES: u32 = 100_000;

/// How dense an exploded curve's vertices are (docs/adr/0308): its pieces equal in length along it, as many as
/// asked, none longer than a length, or as few as keep every chord within a distance of the curve.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ExplodeDensity {
    /// This many equal pieces: an open curve `segments + 1` vertices, a closed one (a circle, an ellipse, a closed
    /// curve) `segments`, at least 3.
    Count {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
        segments: u32,
    },
    /// Equal pieces none longer than `length`, metres.
    Step {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        length: f64,
    },
    /// Equal pieces whose chords lie within `tolerance` of the curve, metres.
    Deviation {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        tolerance: f64,
    },
}

/// Input of `cad.entities.explode` v1: the objects `uids` exploded one level, as one undo step “Patlat”. Each object
/// is replaced by its pieces: a piece keeps the object's layer, look and data, a block's object its own layer when
/// the drawing has it, colour, line weight, data and label (the insert's when it has none). An object on a locked
/// layer is left out with a warning (`layer_locked`); one that does not explode (a line, a point, a text, a solid
/// hatch …) too (`not_explodable`), with the first one's words. A curve becomes a polyline (docs/adr/0308).
///
/// Refusals (`CommandError.code`), checked in this order: `no_entities`, `invalid_uid`, `invalid_density` (a
/// count below 1 or above [`MAX_EXPLODE_VERTICES`], a length or a tolerance not above zero or not finite),
/// `invalid_revision`,
/// `revision_conflict` (status `conflict`), `entity_not_found`, `layer_locked` (every object locked),
/// `not_explodable` (none explodes), `slots_exhausted`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesExplode {
    /// The objects' persistent ids (lowercase UUID text with hyphens), at least one.
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    /// How dense an exploded curve's vertices are; absent: a circle's, an arc's and an ellipse's chords within
    /// [`EXPLODE_DEVIATION`], a spline's and a NURBS curve's within 0.1 mm (docs/adr/0308).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub density: Option<ExplodeDensity>,
    /// The document revision the input was prepared against, as decimal text. When given and the document is no
    /// longer at it, nothing is written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}
