//! The dimension commands of the product command catalog: a dimension made
//! from what it measures and where it goes (docs/adr/0255; plan/cad.md
//! CAD-CMD-05), and an associative dimension's points bound to new places
//! on objects, or freed, as one undo step “Ölçüyü yeniden bağla” (docs/adr/0249 §3).
//!
//! Their inputs name places and edges, not coordinates: the commands read
//! each point from its place on the object now (`ops::subref::point`) and
//! each edge from the object (the edge a click there picks), so a tool,
//! Python and an agent asking the same get the same dimension.
//! Coordinates are metres on the wire (x east (Y), y north (X)).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::cad_region::NewLook;
use crate::entity::{Entity, Vec2};
use crate::subref::SubPlace;

/// Makes a dimension from what it measures.
pub const CAD_DIMENSIONS_CREATE: &str = "cad.dimensions.create";
pub const CAD_DIMENSIONS_CREATE_VERSION: u32 = 1;

/// A measured point: coordinates, or a place on an object.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum MeasuredPoint {
    /// Coordinates: the point follows nothing.
    At {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        at: Vec2,
    },
    /// A place on an object: the point is the place's now. Hizalı's,
    /// Doğrusal's, Koordinat's, Semt's and Eğim's points follow their places
    /// from then on (docs/adr/0249 §3); the others' are read from them.
    Place {
        /// The object's persistent id (lowercase UUID text with hyphens).
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        uid: String,
        place: SubPlace,
        /// When `uid` is an insert, the piece inside it the place is on (docs/adr/0263 §3).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[cfg_attr(feature = "ts", ts(as = "Option<Vec<u32>>", optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        path: Vec<u32>,
    },
}

/// An object's edge as a click picks it: of the object's edges of the kind
/// the measure takes (straight, an arc, a circle), the one nearest `near`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EdgeNear {
    /// The object's persistent id (lowercase UUID text with hyphens).
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    /// Where it was clicked: its nearest edge is the one measured. An angle's
    /// arms reach as far as their edges were clicked.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub near: Vec2,
}

/// What a dimension measures: Ölçülendirme's ten kinds (docs/adr/0061, 0147), Zincir ölçü and Baz ölçü (docs/adr/0320).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DimensionMeasure {
    /// Hizalı: the distance from `a` to `b`.
    Aligned { a: MeasuredPoint, b: MeasuredPoint },
    /// Doğrusal: the distance from `a` to `b` along a direction, degrees
    /// (0 ΔY, 90 ΔX, any other); absent: ΔY or ΔX as `at` lies (AutoCAD's rule).
    Linear {
        a: MeasuredPoint,
        b: MeasuredPoint,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
        angle: Option<f64>,
    },
    /// Koordinat: `a`'s Y (angle 0) or X (90), its leader ending at `at`;
    /// absent: the axis `at` lies further along.
    Ordinate {
        a: MeasuredPoint,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
        angle: Option<f64>,
    },
    /// Semt: the direction from `a` to `b`, from north clockwise.
    Azimuth { a: MeasuredPoint, b: MeasuredPoint },
    /// Eğim: the slope from `a` at `za` to `b` at `zb`, metres; an
    /// elevation absent: its place's vertex's (docs/adr/0142).
    Slope {
        a: MeasuredPoint,
        b: MeasuredPoint,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
        za: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
        zb: Option<f64>,
    },
    /// Açı between two straight edges: their lines' crossing the vertex, the
    /// sector the one `at` is in.
    Angle { first: EdgeNear, second: EdgeNear },
    /// Açı from its vertex `c` and a point on each arm (Köşeden; a circle's
    /// centre and two points on it, Daireden): the arc from `a` to `b` the
    /// way `at` lies.
    AngleVertex {
        c: MeasuredPoint,
        a: MeasuredPoint,
        b: MeasuredPoint,
    },
    /// Açı of an arc (Yaydan): its own angle about its centre.
    AngleArc { of: EdgeNear },
    /// Yarıçap of a circle, an arc or a path's arc segment, toward `at`.
    Radius { of: EdgeNear },
    /// Çap of a circle, an arc or a path's arc segment, toward `at`.
    Diameter { of: EdgeNear },
    /// Yay uzunluğu of an arc or a path's arc segment; with `between`, the
    /// part between two points put on its circle (Kısmi).
    ArcLength {
        of: EdgeNear,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        between: Option<[Vec2; 2]>,
    },
    /// Kırıklı yarıçap of a circle, an arc or a path's arc segment: from the
    /// centre shown to the circle toward `on`, its jog where `at` is.
    Jogged {
        of: EdgeNear,
        #[serde(rename = "shownCentre")]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        shown_centre: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        on: Vec2,
    },
    /// Zincir ölçü (docs/adr/0320 §3): from the base dimension's second point to `to`, along the base's direction,
    /// its line on the base's line; its first point follows what the base's second follows. The base is an aligned
    /// or a linear dimension; `at` is not read.
    Continue {
        /// The base dimension's persistent id (lowercase UUID text with hyphens).
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        base: String,
        to: MeasuredPoint,
    },
    /// Baz ölçü (docs/adr/0320 §3): from the base dimension's first point to `to`, along the base's direction, its
    /// line one level (three times the base's value height) further out than the base's; its first point follows
    /// what the base's first follows. The base is an aligned or a linear dimension; `at` is not read.
    Baseline {
        /// The base dimension's persistent id (lowercase UUID text with hyphens).
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        base: String,
        to: MeasuredPoint,
    },
    /// Merkez işareti (docs/adr/0322) of a circle, an arc or a path's arc segment: the cross at its centre, `height`
    /// across (absent: the value height), upright; its axis lines reach `offset` (absent: `height`) past the curve.
    /// It follows the curve; `at` is not read.
    CenterMark { of: EdgeNear },
    /// Eksen çizgisi (docs/adr/0322) between two straight edges: from the middle of their starts to the middle of their
    /// ends (the second turned to run the first's way), `offset` (absent: `height`) past each end. It follows the
    /// edges; `at` is not read.
    CenterLine { first: EdgeNear, second: EdgeNear },
    /// Kot (docs/adr/0322): the elevation of `a`, `z` metres (absent: its place's vertex's, docs/adr/0142), its mark on
    /// `a` and its value on the shoulder at `at`. A bound point's elevation follows its vertex's.
    Elevation {
        a: MeasuredPoint,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "elevation")))]
        z: Option<f64>,
    },
}

/// Input of `cad.dimensions.create` v1: the dimension `measure` names,
/// placed at `at` (the dimension line, its arc or its leader through it),
/// written on `layerId` by `cad.entities.create`'s rules as one undo step
/// “Ekle”. The dimension is the one Ölçülendirme draws for the same picks:
/// the tool writes it with this command. The points given as places are
/// bound: they follow their objects (docs/adr/0249); an angle by edges, a
/// radius, a diameter, a jogged radius, an arc length and an arc's angle
/// follow their edges or their curve too (docs/adr/0320 §1). Zincir ölçü and
/// Baz ölçü take the base's style, look and height unless a style is named.
///
/// Refusals (`CommandError.code`), checked in this order: `not_finite`
/// (`at`, `offset`, then the measure's numbers in their order),
/// `invalid_height`, `invalid_uid` (each place's and edge's object, in
/// order; a base's first), `invalid_offset` (an `offset` for Yarıçap, Çap,
/// Koordinat, Zincir, Baz or Kot, which take none; a negative one for Merkez
/// işareti or Eksen çizgisi), `invalid_revision`,
/// `revision_conflict` (status `conflict`), `entity_not_found`,
/// `no_such_place`, `not_measurable` (the object has no edge of the kind the
/// measure takes), `not_straight` (a base not an aligned or a linear dimension), `missing_elevation`
/// (Eğim's or Kot's point with no elevation given or kept), `unknown_style`,
/// `no_dimension` (no dimension forms there: the points meet, the edges are
/// parallel, the arc spans nothing; an axis's two edges are one), then the layer's and the look's as
/// `cad.entities.create` checks them (`layer_not_found`, `not_a_layer`,
/// `layer_locked`, `invalid_line_weight`); a hidden layer gives a
/// `layer_hidden` warning; on the desktop also `slots_exhausted`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionsCreate {
    pub measure: DimensionMeasure,
    /// Where the dimension goes: its line, its arc or its leader through it.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Vec2,
    /// Instead of `at`'s, the distance typed: Hizalı's, Semt's and Eğim's
    /// signed (left of a→b positive), Doğrusal's along its direction's
    /// normal, Açı's arc radius (its sign aside), Yay uzunluğu's from the arc
    /// (out positive), Kırıklı yarıçap's jog along the radius from the
    /// centre shown. Yarıçap, Çap and Koordinat take none: `at` places them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub offset: Option<f64>,
    /// The layer the dimension goes to.
    pub layer_id: String,
    /// The project's dimension style by its name (docs/adr/0183); absent or
    /// “Standart”: Standart.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub dimension_style: Option<String>,
    /// The value's height, metres; absent: the style's at the project's
    /// scale, Standart's the project's Ölçü height (docs/adr/0205).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub height: Option<f64>,
    /// The value drawn over the drawing's background (Zemin, docs/adr/0147).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub mask: Option<bool>,
    /// The dimension's colour, weight, attributes, label and symbol.
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub look: NewLook,
    /// The document revision the input was prepared against, as decimal
    /// text. When given and the document is no longer at it, nothing is
    /// written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Kesen çizgiyle iç ölçü zinciri (docs/adr/0711): the faces a cut crosses, measured as a chain.
pub const CAD_DIMENSIONS_INTERIOR: &str = "cad.dimensions.interior";
pub const CAD_DIMENSIONS_INTERIOR_VERSION: u32 = 1;

/// The lines of a crossed wall the chain measures (docs/adr/0711 §3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum InteriorFaces {
    /// Its two faces.
    #[default]
    Outer,
    /// Every line: the faces and the boundaries between its components (Archicad's “All Skins”).
    Skins,
}

/// What of a crossed column the chain measures (docs/adr/0711 §3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum InteriorColumns {
    /// A rectangular column's sides the cut crosses; a round column's centre.
    #[default]
    Ends,
    /// Its centre.
    Centers,
    /// Nothing.
    None,
}

/// An interior dimension chain along a cut (docs/adr/0711 §4).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionsInterior {
    /// The cut's start.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub from: Vec2,
    /// The cut's end.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub to: Vec2,
    /// Where the chain's line goes: through this point, along the cut; absent: on the cut.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Option<Vec2>,
    /// A crossed wall's faces (`outer`, absent) or every line between its components (`skins`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub faces: Option<InteriorFaces>,
    /// A crossed column's sides (`ends`, absent), its centre (`centers`) or nothing (`none`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub columns: Option<InteriorColumns>,
    /// Also the overall dimension from the first to the last, one row further (8 paper millimetres), on `at`'s side.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub total: Option<bool>,
    /// Only the walls and columns of this architectural level (its id); absent: every one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub level: Option<String>,
    /// The layer the dimensions go to.
    pub layer_id: String,
    /// The project's dimension style by its name (docs/adr/0183); absent or “Standart”: Standart.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub dimension_style: Option<String>,
    /// The values' height; absent: the style's at the project's scale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub height: Option<f64>,
    /// The values drawn over the drawing's background (Zemin, docs/adr/0147).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub mask: Option<bool>,
    /// The dimensions' colour, weight, attributes, label and symbol.
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub look: NewLook,
    /// The document revision the input was prepared against, as decimal text. When given and the document is no
    /// longer at it, nothing is written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Binds a dimension's points to places on objects again.
pub const CAD_DIMENSIONS_REASSOCIATE: &str = "cad.dimensions.reassociate";
pub const CAD_DIMENSIONS_REASSOCIATE_VERSION: u32 = 1;

/// A dimension's measured point: `a`, `b` or `c` (its third point: an
/// angle's vertex, a radius's centre and the like).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DimensionPoint {
    A,
    B,
    C,
}

impl DimensionPoint {
    /// Its name as `DimensionRefs` and the wire say it.
    pub fn name(self) -> &'static str {
        match self {
            DimensionPoint::A => "a",
            DimensionPoint::B => "b",
            DimensionPoint::C => "c",
        }
    }
}

/// A place on an object named by the object's persistent id.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PlaceOn {
    /// The object's persistent id (lowercase UUID text with hyphens).
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    pub place: SubPlace,
    /// When `uid` is an insert, the piece inside it the place is on (docs/adr/0263 §3).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<u32>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub path: Vec<u32>,
}

/// One point's new binding.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PointBinding {
    pub point: DimensionPoint,
    /// The place the point follows from now on; null frees it (it keeps its
    /// coordinates and follows nothing).
    pub to: Option<PlaceOn>,
}

/// Input of `cad.dimensions.reassociate` v1: a dimension's points bound to
/// places on objects, or freed, as one undo step “Ölçüyü yeniden bağla”. A
/// bound point moves to its place on the object now and follows it from
/// then on (docs/adr/0249 §3); a broken reference given a place is live
/// again. A binding that leaves a point and its reference as they are
/// changes nothing; when none changes, nothing is written (`changed` 0).
///
/// Refusals (`CommandError.code`), checked in this order: `invalid_uid` (the
/// dimension's id, then each place's object's, in order), `no_entities` (no
/// binding), `invalid_refs` (a point bound twice, a place's index too big),
/// `invalid_revision`, `revision_conflict` (status `conflict`),
/// `entity_not_found` (the dimension, then each place's object, at
/// `points[i].to.uid`), `not_a_dimension`, `layer_locked` (the dimension's
/// layer), `invalid_refs` (a point the dimension's kind does not measure:
/// `c` of a dimension without one; a place bound to the dimension itself),
/// `no_such_place` (a place its object does not have, at
/// `points[i].to.place`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionsReassociate {
    /// The dimension's persistent id.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    /// The points bound or freed, at least one, each once.
    pub points: Vec<PointBinding>,
    /// The document revision the input was prepared against, as decimal
    /// text. When given and the document is no longer at it, nothing is
    /// written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.dimensions.reassociate` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionsReassociated {
    /// The points whose binding or coordinates changed (an undo step was
    /// written when any did).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub changed: u32,
    /// The dimension's measured value now, in its unit (`unit`).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub value: f64,
    /// "length" (metres), "angle" (radians), "percent" or "coordinate".
    pub unit: String,
    /// The document's revision after the write, as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.dimensions.reassociate` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionsReassociatePlan {
    /// The dimension as execute would write it.
    pub dimension: Entity,
    /// The points whose binding or coordinates would change.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub changed: u32,
    /// Its measured value then, and the value's unit, as in the output.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub value: f64,
    pub unit: String,
    /// The document revision the plan was made against.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Edits dimensions: their spacing, breaks, moved value, written value and options.
pub const CAD_DIMENSIONS_EDIT: &str = "cad.dimensions.edit";
pub const CAD_DIMENSIONS_EDIT_VERSION: u32 = 1;

/// One edit of `cad.dimensions.edit` (docs/adr/0321 §4).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DimensionEdit {
    /// Ölçü aralığı (AutoCAD's DIMSPACE): the dimensions' lines put `spacing` apart from the base's, each side of it
    /// in its order of distance; 0 lines them up with the base's. Aligned and linear dimensions parallel to the base.
    Space {
        /// The base dimension's persistent id; it does not move.
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        base: String,
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
        /// Metres between two lines, 0 or more; absent: three times the base's value height (Baz ölçü's level).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        spacing: Option<f64>,
    },
    /// Ölçüyü kır (AutoCAD's DIMBREAK): a gap in each of the dimensions' lines where `objects` (absent: every shown
    /// object that is not a dimension) cross it; its breaks so far replaced. Aligned and linear dimensions.
    Break {
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        objects: Option<Vec<String>>,
    },
    /// Kırmayı kaldır: the dimensions' lines whole again.
    Unbreak {
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
    },
    /// Ölçü yazısını taşı: the value moved along and across its own direction from its place; 0 and 0 home again.
    MoveText {
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        uid: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        along: f64,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        across: f64,
    },
    /// Ölçü değerini yaz: the text written over the value, `<>` its measured value (AutoCAD's text override); null:
    /// the measured value again.
    Text {
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
        text: Option<String>,
    },
    /// Ölçü seçenekleri: the dimensions' options (docs/adr/0321 §1, §2); null: none.
    Options {
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
        options: Option<crate::DimensionOptions>,
    },
}

/// Input of `cad.dimensions.edit` v1 (docs/adr/0321 §4): one edit of dimensions as one undo step named after it
/// (Ölçü aralığı, Ölçüyü kır, Kırmayı kaldır, Ölçü yazısını taşı, Ölçü değerini yaz, Ölçü seçenekleri). A dimension
/// the edit leaves as it was is not counted; when none changes, nothing is written.
///
/// Refusals (`CommandError.code`), checked in this order: `invalid_uid` (the base's, then each dimension's, then each
/// object's), `no_entities` (no dimension), `not_finite` (`spacing`, `along`, `across`), `invalid_offset` (a spacing
/// below 0), `empty_text` (a written value of spaces), `invalid_style` (options that break their rules),
/// `invalid_revision`, `revision_conflict` (status `conflict`), `entity_not_found` (the base, each dimension, each
/// object), `not_a_dimension`, `layer_locked` (a dimension's layer), `not_straight` (Space's and Break's dimensions:
/// aligned or linear), `not_parallel` (Space's dimension not parallel to its base). Break warns `no_crossing` for a
/// dimension no object crosses.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionsEdit {
    pub edit: DimensionEdit,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.dimensions.edit` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionsEdited {
    /// The dimensions that changed, by their persistent ids, in the input's order.
    pub changed: Vec<String>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.dimensions.edit` would write (plan mode).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionsEditPlan {
    /// The dimensions that would change, as execute would write them.
    pub dimensions: Vec<Entity>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// Reads dimensions: what each measures and what it shows.
pub const CAD_DIMENSIONS_QUERY: &str = "cad.dimensions.query";
pub const CAD_DIMENSIONS_QUERY_VERSION: u32 = 1;

/// Input of `cad.dimensions.query` v1 (docs/adr/0321 §3), read only: each dimension's measured value apart from the
/// value it shows (a written value is an override: what it says is not what the geometry measures), and what its
/// points follow. Refusals: `invalid_uid`, `entity_not_found`, `not_a_dimension`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionsQuery {
    /// The dimensions; absent: every dimension of the drawing, in its order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Option<Vec<String>>,
}

/// What a dimension's points follow.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DimensionBond {
    /// Nothing: its points are free.
    Free,
    /// Some of its points follow places, all live.
    Partial,
    /// Every point it measures follows a place (or its rule's), all live.
    Bound,
    /// A reference broke.
    Broken,
}

/// One dimension as `cad.dimensions.query` reads it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionRead {
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    /// `aligned`, `linear`, `angular`, `radius`, `diameter`, `ordinate`, `arcLength`, `jogged`, `azimuth`, `slope`.
    pub kind: String,
    /// What the geometry measures, in `unit`: metres, radians, per cent or a coordinate's metres.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "value")))]
    pub measured: f64,
    /// "length", "angle", "percent" or "coordinate".
    pub unit: String,
    /// What it shows, as written (its options; a written value with `<>` the measured one).
    pub shown: String,
    /// Whether it shows a written value without `<>`: not the measured one.
    pub overridden: bool,
    pub bond: DimensionBond,
}

/// Output of `cad.dimensions.query` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionsRead {
    pub dimensions: Vec<DimensionRead>,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_point_bound_and_a_point_freed_read() {
        let input: DimensionsReassociate = serde_json::from_value(json!({
            "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
            "points": [
                { "point": "a", "to": { "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e60",
                    "place": { "kind": "vertex", "part": 0, "ring": 0, "index": 1 } } },
                { "point": "b", "to": null }
            ]
        }))
        .expect("reads");
        assert_eq!(input.points[0].point, DimensionPoint::A);
        assert_eq!(input.points[1].to, None);
        assert_eq!(input.points[1].point.name(), "b");
    }

    #[test]
    fn a_measure_reads_its_points_and_edges_by_their_kind() {
        let input: DimensionsCreate = serde_json::from_value(json!({
            "measure": {
                "kind": "aligned",
                "a": { "kind": "place", "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f", "place": { "kind": "start" } },
                "b": { "kind": "at", "at": { "x": 10.0, "y": 0.0 } }
            },
            "at": { "x": 5.0, "y": 2.0 }, "layerId": "olcu", "color": "#E5484D"
        }))
        .expect("reads");
        assert!(matches!(
            input.measure,
            DimensionMeasure::Aligned {
                a: MeasuredPoint::Place {
                    place: SubPlace::Start,
                    ..
                },
                ..
            }
        ));
        assert_eq!(input.look.color.as_deref(), Some("#E5484D"));
        let arc: DimensionsCreate = serde_json::from_value(json!({
            "measure": { "kind": "arcLength", "of": { "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f", "near": { "x": 1.0, "y": 1.0 } },
                "between": [{ "x": 1.0, "y": 0.0 }, { "x": 0.0, "y": 1.0 }] },
            "at": { "x": 2.0, "y": 2.0 }, "layerId": "olcu"
        }))
        .expect("reads");
        assert!(matches!(
            arc.measure,
            DimensionMeasure::ArcLength {
                between: Some(_),
                ..
            }
        ));
        let jog = json!({ "kind": "jogged", "of": { "uid": "u", "near": { "x": 0.0, "y": 0.0 } },
            "shownCentre": { "x": 1.0, "y": 1.0 }, "on": { "x": 2.0, "y": 2.0 } });
        assert!(serde_json::from_value::<DimensionMeasure>(jog).is_ok());
    }
}
