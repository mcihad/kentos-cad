//! The selection as one explicit query (docs/adr/0275; plan/cad.md CAD-INPUT-11): a window, a crossing box, a
//! fence, a polygon, a circle, the closed shapes around a point, a click's candidates, every object, objects given
//! (Önceki seçim is the list the caller kept), their inverse, the objects like given ones; with the selection
//! filter's kinds and a list of layers. Every selection tool asks it, and Python and an agent ask the same. It reads
//! no selection, view or setting (TODOS.md CMD-07) and writes nothing: the answer is the objects' persistent ids.
//!
//! Points, distances and radii are metres on the wire, x east (Y), y north (X), whatever unit the project shows
//! (docs/adr/0165 §2).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::entity::Vec2;
use crate::subref::SubPlace;

/// Selects objects by a region, a list or a likeness.
pub const CAD_ENTITIES_SELECT: &str = "cad.entities.select";
pub const CAD_ENTITIES_SELECT_VERSION: u32 = 1;

/// The most points a fence or a polygon takes, and the most objects a list.
pub const MAX_SELECT_ITEMS: usize = 100_000;

/// How a polygon selects (Çokgenle seç, docs/adr/0187 §2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum PolygonSelectMode {
    /// İçindekiler: wholly inside.
    Inside,
    /// Kesişenler: inside or touching.
    Crossing,
    /// Dışındakiler: every object it does not touch.
    Outside,
}

/// What Benzerini seç compares (docs/adr/0187 §4): an object is like an example when it equals it in every
/// criterion that is on; none on: every object.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SimilarCriteria {
    /// Tür: the kind, an insert's block too.
    pub kind: bool,
    /// Katman.
    pub layer: bool,
    /// Renk: the object's own colour (“by layer” is a value of its own).
    pub color: bool,
    /// Sembol: the object's own symbol.
    pub symbol: bool,
}

impl Default for SimilarCriteria {
    fn default() -> Self {
        Self {
            kind: true,
            layer: true,
            color: true,
            symbol: true,
        }
    }
}

/// Where the objects are, or which they are.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SelectRegion {
    /// Pencere: wholly inside the box from `a` to `b` (an infinite line never is).
    Window {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        a: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        b: Vec2,
    },
    /// Kesişen: inside the box or touching it (an edge across it, a point of it inside, the box's middle in its
    /// area).
    Crossing {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        a: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        b: Vec2,
    },
    /// Çitle seç: what the open path crosses; a point, `tolerance` metres from it.
    Fence {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        points: Vec<Vec2>,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        tolerance: f64,
    },
    /// Çokgenle seç: a simple ring's inside, crossing or outside.
    Polygon {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        points: Vec<Vec2>,
        mode: PolygonSelectMode,
    },
    /// Daireyle seç: wholly inside the circle, and with `crossing` touching it too.
    Circle {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        center: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        radius: f64,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
        crossing: bool,
    },
    /// İçeren alanı seç: the closed shapes around the point, the smallest first.
    Containing {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        at: Vec2,
    },
    /// A click: what a click at `at` selects within `tolerance` metres, the most specific first (Sıradakini seç's
    /// candidates, docs/adr/0187 §1).
    Point {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        at: Vec2,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        tolerance: f64,
    },
    /// Tümünü seç: every object shown.
    All,
    /// The objects given, those shown (Önceki seçim: the list the caller kept).
    Objects {
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
    },
    /// Seçimi ters çevir: every object shown but those given.
    Invert {
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
    },
    /// Benzerini seç: every object shown like one of those given.
    Similar {
        #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
        uids: Vec<String>,
        /// Absent: every criterion on.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        criteria: Option<SimilarCriteria>,
    },
}

/// A kind the selection filter holds (docs/adr/0187 §5): a NURBS curve is an Eğri (`spline`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SelectKind {
    Point,
    Line,
    Polyline,
    Polygon,
    Circle,
    Arc,
    Ellipse,
    Spline,
    Xline,
    Ray,
    Text,
    Dimension,
    Hatch,
    Insert,
    Leader,
    Table,
    Image,
}

impl SelectKind {
    /// Every kind, in the filter's menu's order.
    pub const ALL: [SelectKind; 17] = [
        SelectKind::Point,
        SelectKind::Line,
        SelectKind::Polyline,
        SelectKind::Polygon,
        SelectKind::Circle,
        SelectKind::Arc,
        SelectKind::Ellipse,
        SelectKind::Spline,
        SelectKind::Xline,
        SelectKind::Ray,
        SelectKind::Text,
        SelectKind::Dimension,
        SelectKind::Hatch,
        SelectKind::Insert,
        SelectKind::Leader,
        SelectKind::Table,
        SelectKind::Image,
    ];

    /// Its name on the wire, the entity kind's.
    pub fn name(self) -> &'static str {
        match self {
            SelectKind::Point => "point",
            SelectKind::Line => "line",
            SelectKind::Polyline => "polyline",
            SelectKind::Polygon => "polygon",
            SelectKind::Circle => "circle",
            SelectKind::Arc => "arc",
            SelectKind::Ellipse => "ellipse",
            SelectKind::Spline => "spline",
            SelectKind::Xline => "xline",
            SelectKind::Ray => "ray",
            SelectKind::Text => "text",
            SelectKind::Dimension => "dimension",
            SelectKind::Hatch => "hatch",
            SelectKind::Insert => "insert",
            SelectKind::Leader => "leader",
            SelectKind::Table => "table",
            SelectKind::Image => "image",
        }
    }
}

/// Input of `cad.entities.select` v1: the objects the region gives that pass the kinds and the layers, in the
/// region's order (the drawing's, the smallest first for `containing`, the most specific first for `point`).
/// Only objects shown count: a hidden layer's, and what a layer's filter or the time window leaves out, never
/// (docs/adr/0211 §1, 0210); a locked layer's do (a lock forbids editing, not selecting).
///
/// Refusals (`CommandError.code`), the region's in its fields' order, then the layers': `not_finite` (a point, a
/// tolerance, a radius), `too_many_points` (more than 100 000 points or objects), `too_few_points` (a fence of
/// fewer than two points), `too_few_corners` (a polygon of fewer than three), `invalid_ring` (a polygon crossing
/// itself or without area), `invalid_radius` (a circle's not above zero), `invalid_distance` (a tolerance below
/// zero), `invalid_uid`, `entity_not_found` (an object given), `no_objects` (Benzerini seç without an example),
/// `layer_not_found`, `not_a_layer` (a group in `layers`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesSelect {
    pub region: SelectRegion,
    /// Seçim süzgeci: only these kinds; absent: every kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub kinds: Option<Vec<SelectKind>>,
    /// Only on these layers (their ids); absent: every layer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layers: Option<Vec<String>>,
}

/// What the kinds and the layers left out of the region's objects, and the objects given that are not shown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SelectLeftOut {
    /// By the kinds (the selection filter).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub kinds: u32,
    /// By the layers.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub layers: u32,
    /// Objects given (`objects`) that are not shown.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub hidden: u32,
}

/// Output of `cad.entities.select` v1, and its plan: the objects' persistent ids in the region's order, what was
/// left out, and the revision they were read at.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesSelected {
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    pub left_out: SelectLeftOut,
    /// The document's revision the objects were read at, as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// The objects a click falls on, each with its sub-object.
pub const CAD_ENTITIES_PICK: &str = "cad.entities.pick";
pub const CAD_ENTITIES_PICK_VERSION: u32 = 1;

/// Input of `cad.entities.pick` v1 (docs/adr/0276): what a click at `at` within `tolerance` metres can mean, the
/// most specific first (Sıradakini seç's candidates: points and edges by their distance, then the areas around it by
/// their size), each with the sub-object it falls on. Only objects shown count; `kinds` is the selection filter.
///
/// Refusals (`CommandError.code`), in this order: `not_finite` (`at`, the tolerance), `invalid_distance` (a
/// tolerance below zero).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesPick {
    /// Where the click is.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Vec2,
    /// How far from `at` an object may be, metres (a tool's is its pixels at the view's scale).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub tolerance: f64,
    /// Seçim süzgeci: only these kinds; absent: every kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub kinds: Option<Vec<SelectKind>>,
}

/// An object a click falls on and its sub-object.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PickedEntity {
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    /// The sub-object (docs/adr/0249, 0263): the object's nearest point within the tolerance (an end, a vertex, a
    /// point, an insertion point), else the nearest point of an edge or a curve (`along`); absent when the click is
    /// in the object's area or on a text's body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub place: Option<SubPlace>,
    /// When the object is an insert, the piece inside it the place is on.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<u32>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub path: Vec<u32>,
}

/// Output of `cad.entities.pick` v1, and its plan: the objects the most specific first, what the kinds left out, the
/// revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesPicked {
    pub picks: Vec<PickedEntity>,
    pub left_out: SelectLeftOut,
    /// The document's revision the objects were read at, as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// The standard relations between objects (docs/adr/0501).
pub const CAD_ENTITIES_RELATE: &str = "cad.entities.relate";
pub const CAD_ENTITIES_RELATE_VERSION: u32 = 1;

/// At most this many pairs of `a` and `b` (docs/adr/0501 §3).
pub const MAX_RELATE_PAIRS: usize = 1_000_000;

/// A predicate of the standard profile (OGC SFA 1.2.1 §6.1.15.3, as PostGIS reads it); not Konuma göre seç's
/// relations, which allow 1 mm.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum RelatePredicate {
    Intersects,
    Disjoint,
    Contains,
    Within,
    Covers,
    CoveredBy,
    Touches,
    Crosses,
    Overlaps,
    Equals,
}

/// Input of `cad.entities.relate` v1 (docs/adr/0501): how each object of `a` stands to each of `b` by the standard
/// DE-9IM profile, exact up to the floating point. With `predicate` or `pattern` (a nine-letter `ST_Relate` pattern of
/// `T`, `F`, `*`, `0`, `1`, `2`) only the pairs it holds for are answered; with neither, every pair and its matrix. An
/// object is read as a query reads it: its areas with their holes (arcs exact), its open paths or its points; Z is not
/// read. Hidden and locked objects are read as well: the ids are given.
///
/// Refusals (`CommandError.code`), in this order: `no_objects` (`a` or `b` empty), `invalid_input` (both `predicate`
/// and `pattern`), `invalid_pattern` (`pattern`), `entity_not_found` (an id the drawing lacks), `too_many_pairs` (more
/// than 1 000 000 pairs).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesRelate {
    /// The objects related, by their lasting ids.
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub a: Vec<String>,
    /// The objects they are related to.
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub b: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub predicate: Option<RelatePredicate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub pattern: Option<String>,
}

/// A pair and its matrix.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RelatedPair {
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub a: String,
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub b: String,
    /// The DE-9IM matrix as `ST_Relate` writes it (`F`, `0`, `1`, `2`; II, IB, IE, BI, BB, BE, EI, EB, EE).
    pub matrix: String,
}

/// Output of `cad.entities.relate` v1, and its plan: the pairs in `a`'s then `b`'s order, the objects the profile
/// does not read (a construction line, a ray), the revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesRelated {
    pub pairs: Vec<RelatedPair>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub unread: Vec<String>,
    /// The document's revision the objects were read at, as decimal text.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
