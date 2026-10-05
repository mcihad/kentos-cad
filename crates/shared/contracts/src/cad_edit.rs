//! The edit command of the product command catalog (docs/adr/0047): objects
//! named by their persistent ids given a new geometry, replaced by pieces,
//! followed by new objects made from them, or deleted, as one undo step
//! named after the modify tool. Ötele, Buda, Uzat, Köşe yuvarla, Pah, Kır,
//! Birleştir, Patlat, Uzat-kısalt, Köşe ekle/sil and Esnet write through
//! `cad.entities.edit` on the web (`apps/web/src/product`) and on the
//! desktop (`crates/native/application`); so do Öznitelikler's geometry rows
//! and the in-place text editor (operation `properties`) and the area tools
//! (Alan birleştir, kesiştir, çıkar, böl, Alana çevir, Çizgiye çevir; docs/adr/0065).
//! Both pass the shared cases in `fixtures/commands/v1`.
//!
//! The geometry is given, not computed here: the tools compute it with the
//! shared geometry core, from what they picked and what the view shows (the
//! visible edges a trim cuts against, the corner under the cursor), and the
//! command writes what the preview showed. What every tool had to do alike
//! is the command's: which object keeps its place and persistent id, what a
//! piece inherits, the locked layer, one undo step.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::entity::{
    AreaPart, DimensionStyle, Entity, HatchPattern, LeaderArrow, PointPart, RingGeometry,
    TextAlign, Vec2,
};
use crate::identity::BlockId;

/// Reshapes, splits, joins and explodes objects in one undo step.
pub const CAD_ENTITIES_EDIT: &str = "cad.entities.edit";
pub const CAD_ENTITIES_EDIT_VERSION: u32 = 1;

/// The modify tool an edit comes from; it names the undo step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum EditOperation {
    /// Ötele: a parallel copy.
    Offset,
    /// Buda: the part between two boundaries goes.
    Trim,
    /// Uzat: an end grows to a boundary.
    Extend,
    /// Köşe yuvarla: a corner rounded by an arc.
    Fillet,
    /// Pah: a corner cut by a line.
    Chamfer,
    /// Kır: the part between two points goes, or the object splits at one.
    Break,
    /// Birleştir: chains of objects meeting end to end become one each.
    Join,
    /// Patlat: an object comes apart into simple ones.
    Explode,
    /// Uzat-kısalt: a new total length, changed at one end.
    Lengthen,
    /// Köşe ekle: a vertex added on an edge.
    VertexAdd,
    /// Köşe sil: a vertex removed.
    VertexRemove,
    /// Esnet: the vertices in a window move, the rest stay.
    Stretch,
    /// Öznitelikler: a value typed into a geometry row (a point's Y and X; a
    /// text's content, height and angle; a dimension's offset, text height
    /// and text; a hatch's pattern, angle and spacing), or a text or a
    /// dimension's text edited in place. The step is “Değiştir”.
    Properties,
    /// Alan birleştir: the areas go, their union comes from the first.
    AreaUnion,
    /// Alan kesiştir: the areas' overlap, a new area (the areas go with it when erased).
    AreaIntersect,
    /// Alan çıkar: areas with others cut out of them; the pieces left come from each.
    AreaSubtract,
    /// Alan böl: areas split along a line; the first piece is the area itself.
    AreaSplit,
    /// Alana çevir: closed objects become areas in their places; the regions line work closes are new areas.
    ToArea,
    /// Çizgiye çevir: an area's outer ring becomes a closed polyline in its place; its holes are new polylines.
    ToPolyline,
    /// Tutamaçla düzenle: a selected object's grip moved (a vertex, an end,
    /// a centre, a radius, an edge's middle), the object updated in its place.
    Grip,
    /// Düz kenar yap: an arc edge of a polyline or an area made straight
    /// (its bulge 0), from the grip menu of the edge's middle.
    StraightEdge,
    /// Yaya dönüştür: a straight edge of a polyline or an area made an arc,
    /// from the grip menu of the edge's middle.
    ArcEdge,
    /// Parçala (docs/adr/0140): objects cut into separate pieces where they
    /// cross, into equal parts or by length; the first piece keeps the
    /// object's place and id, every piece its data.
    Split,
    /// Yönü çevir: an object drawn the other way round, its outline the same.
    Reverse,
    /// Sadeleştir: a path's vertices within a tolerance of its outline dropped.
    Simplify,
    /// Çizimi temizle: objects repeated on their layer and empty ones
    /// deleted, vertices repeated in a row dropped.
    Cleanup,
    /// Kot ver (docs/adr/0142): objects' vertices given elevations as the
    /// geometry's `zs` says: the same value, the old ones raised, or none.
    Elevation,
    /// Parçaları birleştir (docs/adr/0143): areas become one multi-part area
    /// in the first one's place, overlapping ones merged into one part.
    PartsJoin,
    /// Parçalara ayır (docs/adr/0143): a multi-part area becomes an area a
    /// part; the first part keeps its place, the others are new, with its data.
    PartsSplit,
    /// Okunur yap (docs/adr/0145): texts that read upside down turned half
    /// round, each box where it was.
    Readable,
    /// Bul ve değiştir (docs/adr/0145 §6): texts given new words, the step
    /// named after the window; otherwise as `Properties`.
    ReplaceText,
    /// Topolojik temizlik (docs/adr/0148): line work and area outlines put
    /// right within a tolerance, objects updated in place, each geometry with
    /// its elevations as the cleanup carried them.
    Topology,
    /// Kenar eşle (docs/adr/0159): the line ends of two sheets put together
    /// across their common edge, objects updated in place (Parça ekle makes
    /// a line a polyline), each geometry with its elevations as the core's
    /// `ops::edgematch` gave them.
    Edgematch,
    /// Biçim değiştir (docs/adr/0173 §2–§3): an area cut or grown, or a
    /// path's stretch redrawn, by a sketched line; the object updated in
    /// place (a line becomes a polyline).
    Reshape,
    /// Sürdür (docs/adr/0173 §4): a line or a polyline continued from one
    /// of its ends, updated in place.
    Continue,
    /// Delik ekle (docs/adr/0173 §5): a hole added to an area.
    HoleAdd,
    /// Deliği sil: a hole of an area removed.
    HoleRemove,
    /// Deliği doldur: a new area filling a hole, made from the area.
    HoleFill,
}

/// A drawing object's geometry alone: its kind and the fields that place and
/// shape it, without the fields every object has (layer, colour, attributes,
/// label, symbol). Coordinates are x east (Y), y north (X), in the project's
/// units (m), float64; angles in radians unless said otherwise.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum EntityGeometry {
    Point {
        p: Vec2,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        z: Option<f64>,
        /// A multi-point object's points past its first, each with its
        /// elevation (docs/adr/0174). The geometry is the whole object:
        /// absent, it has one point, so an edit of a multi-point object
        /// writes every point.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        parts: Option<Vec<PointPart>>,
    },
    Line {
        a: Vec2,
        b: Vec2,
        /// The vertices' elevations as written (docs/adr/0142): its two ends, `null` for one without; all `null`: none. Absent:
        /// each vertex takes one from the objects the edit names.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        zs: Option<Vec<Option<f64>>>,
    },
    /// An open path: vertices and DXF bulges (tan(θ/4), CCW positive), the
    /// one at vertex i bending the edge to vertex i + 1.
    Polyline {
        pts: Vec<Vec2>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        bulges: Option<Vec<f64>>,
        /// The vertices' elevations as written (docs/adr/0142): as many as
        /// the vertices, `null` for one without; all `null`: none. Absent:
        /// each vertex takes one from the objects the edit names.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        zs: Option<Vec<Option<f64>>>,
        /// A multi-part polyline's parts past its first, each its vertices,
        /// arcs and elevations, without holes (docs/adr/0174). The geometry
        /// is the whole polyline: absent, it has one part, so an edit of a
        /// multi-part polyline writes every part.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        parts: Option<Vec<AreaPart>>,
    },
    /// A closed area: its ring and, when it has any, its holes.
    Polygon {
        pts: Vec<Vec2>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        bulges: Option<Vec<f64>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        holes: Option<Vec<RingGeometry>>,
        /// The vertices' elevations as written (docs/adr/0142): as many as
        /// the vertices, `null` for one without; all `null`: none. The holes' come with them (`RingGeometry.zs`). Absent:
        /// each vertex takes one from the objects the edit names.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        zs: Option<Vec<Option<f64>>>,
        /// A multi-part area's parts past its first, each its ring, arcs,
        /// holes and elevations (docs/adr/0143). The geometry is the whole
        /// area: absent, it has one part, so an edit of a multi-part area
        /// writes every part.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        parts: Option<Vec<AreaPart>>,
    },
    Circle {
        c: Vec2,
        r: f64,
    },
    /// Counter-clockwise from `a0` to `a1`.
    Arc {
        c: Vec2,
        r: f64,
        a0: f64,
        a1: f64,
    },
    /// Centre, major axis vector, minor/major ratio, parameters `t0` → `t1`.
    Ellipse {
        c: Vec2,
        major: Vec2,
        ratio: f64,
        t0: f64,
        t1: f64,
    },
    Spline {
        pts: Vec<Vec2>,
        closed: bool,
    },
    /// A construction line through `p` along the unit direction `dir`.
    Xline {
        p: Vec2,
        dir: Vec2,
    },
    /// A ray from `p` along the unit direction `dir`.
    Ray {
        p: Vec2,
        dir: Vec2,
    },
    /// Text at `p`; `height` in metres, `rotation` in degrees counter-clockwise from east.
    Text {
        p: Vec2,
        text: String,
        height: f64,
        rotation: f64,
        /// Where `p` is on the text; absent: the left of its baseline (docs/adr/0145).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        align: Option<TextAlign>,
        /// The letters' width times this, the height kept; absent: 1. Over 0, at most 100.
        #[serde(default, rename = "widthFactor", skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(range(min = 0.0, max = 100.0)))]
        width_factor: Option<f64>,
        /// Its box filled with the drawing area's colour before it is drawn.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
        mask: bool,
    },
    /// A dimension from `a` to `b` (an angular one's vertex is `c`), its line
    /// `offset` metres away, its text `height` metres high; `text` replaces
    /// the measured value, `angle` is a linear one's measured direction in
    /// degrees counter-clockwise from east (0 = ΔY, 90 = ΔX). What the fields
    /// mean for the styles of docs/adr/0147 (an ordinate's `angle`, an arc
    /// length's and a jogged one's `c`, a slope's `za` and `zb`) is that ADR's
    /// table.
    Dimension {
        a: Vec2,
        b: Vec2,
        offset: f64,
        height: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        text: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        style: Option<DimensionStyle>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        angle: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        c: Option<Vec2>,
        /// The value over the drawing's background (docs/adr/0147).
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
        mask: bool,
        /// A slope's two elevations, metres (docs/adr/0147).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        za: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        zb: Option<f64>,
    },
    /// A hatched area: its ring, its holes when it has any, and its pattern.
    Hatch {
        ring: Vec<Vec2>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        holes: Option<Vec<Vec<Vec2>>>,
        pattern: HatchPattern,
    },
    /// A block placed (docs/adr/0144): the definition's base point goes to
    /// `p`, its objects are mirrored in the definition's x axis when
    /// `mirror`, scaled by `scale` (above 0) and turned by `rotation`
    /// (radians, counter-clockwise) about `p`. The block is the drawing's.
    Insert {
        block: BlockId,
        p: Vec2,
        scale: f64,
        rotation: f64,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
        mirror: bool,
    },
    /// A leader (docs/adr/0146): an arrowhead at `pts[0]`, a line through
    /// `pts` and, with a note, a landing from the last vertex and the note
    /// past it; `height` (metres, over 0) measures the note, the arrowhead
    /// and the landing, `rotation` (degrees counter-clockwise from east)
    /// turns the note and the landing.
    Leader {
        /// At least two: the arrow's tip first.
        pts: Vec<Vec2>,
        /// The note, one line; absent: the arrow alone. Never empty.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        text: Option<String>,
        height: f64,
        rotation: f64,
        /// The arrowhead; absent: a filled arrow.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        arrow: Option<LeaderArrow>,
        /// The note's box filled with the drawing area's colour before it is drawn.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
        mask: bool,
    },
}

impl EntityGeometry {
    /// Whether an object of this geometry is drawn with lines, so it takes
    /// the current line weight: not a point, a text, a dimension, a hatch or
    /// an insert (docs/adr/0139; `Entity::draws_lines`).
    pub fn draws_lines(&self) -> bool {
        !matches!(
            self,
            EntityGeometry::Point { .. }
                | EntityGeometry::Text { .. }
                | EntityGeometry::Dimension { .. }
                | EntityGeometry::Hatch { .. }
                | EntityGeometry::Insert { .. }
        )
    }
}

/// One change of an edit. The objects are named by their persistent ids
/// (docs/adr/0014).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum EntityEdit {
    /// The object takes a new geometry and keeps everything else: its slot,
    /// persistent id, layer, colour, attributes, label and symbol (an end
    /// extended, a corner rounded, a vertex added).
    Update {
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        uid: String,
        geometry: EntityGeometry,
    },
    /// The object becomes another in its place: the new geometry in its slot,
    /// with its persistent id, layer and colour; its attributes and label only
    /// with `keepData`; not its symbol (the part a trim leaves, the first part
    /// of a break, a chain joined into its first object).
    Replace {
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        uid: String,
        geometry: EntityGeometry,
        /// True: the attributes and the label carry over.
        #[serde(rename = "keepData", default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        keep_data: Option<bool>,
    },
    /// A new object made from `from`: its layer, colour and line weight; its
    /// attributes and label only with `keepData` (an offset copy, a piece of
    /// a break or an explode, a fillet's arc). A field given here is the new
    /// object's own instead: a block's object exploded keeps its layer,
    /// colour, line weight, attributes and label (docs/adr/0144).
    Add {
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        from: String,
        geometry: EntityGeometry,
        /// True: the attributes and the label carry over.
        #[serde(rename = "keepData", default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        keep_data: Option<bool>,
        /// The layer it goes on: a layer's id (`LayerNode.id`), not a group's.
        #[serde(rename = "layerId", default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        layer_id: Option<String>,
        /// Its own colour (`EntityBase.color`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        color: Option<String>,
        /// Its own line weight, paper mm, 0 to 100 (`EntityBase.line_weight`).
        #[serde(
            rename = "lineWeight",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(range(min = 0.0, max = 100.0)))]
        line_weight: Option<f64>,
        /// Its attributes, whatever `keepData` says.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        attrs: Option<std::collections::BTreeMap<String, String>>,
        /// Its label, whatever `keepData` says.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        label: Option<String>,
    },
    /// The object is deleted.
    Remove {
        #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
        uid: String,
    },
}

impl EntityEdit {
    /// A new object made from `from` with nothing of its own but its
    /// geometry: what the modify tools write (a block's objects exploded give
    /// their own layer, colour and data, docs/adr/0144).
    pub fn add(from: String, geometry: EntityGeometry, keep_data: Option<bool>) -> EntityEdit {
        EntityEdit::Add {
            from,
            geometry,
            keep_data,
            layer_id: None,
            color: None,
            line_weight: None,
            attrs: None,
            label: None,
        }
    }
}

/// Input of `cad.entities.edit` v1: changes to objects named by their
/// persistent ids, written as one undo step named after `operation`. The
/// modify tools give the objects they picked and the geometry the shared
/// core computed (TODOS.md CMD-07); the command reads no selection, layer or
/// view.
///
/// The changes are made in their order, against the document as it was
/// before them: an `add` may come from an object the same edit replaces or
/// deletes. An object is changed (`update`, `replace`, `remove`) by one
/// change at most.
///
/// An object on a locked layer (by itself or a group above it) is not
/// changed, and nothing is made from it: when one is named, nothing is
/// written and the answer is `layer_locked`. The pieces of a split object
/// belong together, so an edit is written whole or not at all.
///
/// Refusals (`CommandError.code`), checked in this order: `no_changes`,
/// `invalid_uid` (each change's id in order), then each change in order: its
/// geometry's `too_few_points` (a polyline), `too_few_corners` (a closed
/// area's ring or hole with fewer than 3 corners, or 2 whose two edges are
/// both straight, a bulge absent or 0; a hatch's ring or hole with fewer
/// than 3), `empty_text` (a text whose text is empty or only white space,
/// Unicode's `White_Space`), `invalid_elevations`, `not_finite`,
/// `invalid_radius`, `invalid_scale` (an insert's), and an `add`'s own
/// `invalid_line_weight`;
/// then `invalid_revision`, `revision_conflict` (status `conflict`),
/// `entity_not_found` (each id in order), `repeated_entity` (an object
/// changed twice), `layer_locked`; an `add`'s own layer: `layer_not_found`,
/// `not_a_layer`, `layer_locked`; `unknown_block` (each insert's block, in
/// order); on the desktop also `slots_exhausted`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesEdit {
    /// The modify tool the edit comes from; it names the undo step: Ötele,
    /// Buda, Uzat, Köşe yuvarla, Pah, Kır, Birleştir, Patlat, Uzat-kısalt,
    /// Köşe ekle, Köşe sil, Esnet; Değiştir for Öznitelikler; Alan birleştir,
    /// Alan kesiştir, Alan çıkar, Alan böl, Alana çevir, Çizgiye çevir,
    /// Parçaları birleştir, Parçalara ayır; Tutamaçla düzenle, Düz kenar
    /// yap, Yaya dönüştür for the grips.
    pub operation: EditOperation,
    /// What changes, at least one.
    pub changes: Vec<EntityEdit>,
    /// The document revision the input was prepared against, as decimal text
    /// (from a plan, or the document). When given and the document is no
    /// longer at it, nothing is written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.entities.edit` v1: what changed, what was made and what went.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesEdited {
    /// The objects updated or replaced in place, in the input's order.
    pub changed: Vec<String>,
    /// The new objects' persistent ids, in the order of their `add`s.
    pub created: Vec<String>,
    /// The objects deleted, in the input's order.
    pub removed: Vec<String>,
    /// The document's revision after the write, as decimal text. Inside an
    /// open transaction or group the write joins it, and the revision
    /// changes when that ends.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.entities.edit` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesEditPlan {
    /// The objects updated or replaced, each as execute would write it, with
    /// its own slot (`id`), in the input's order.
    pub changed: Vec<Entity>,
    /// The new objects as execute would write them, `id` 0 (their slots are
    /// given when they are written), in the order of their `add`s.
    pub created: Vec<Entity>,
    /// The ids of the objects execute would delete, in the input's order.
    pub removed: Vec<String>,
    /// The document revision the plan was made against. Give it as
    /// `expectedRevision` to write exactly this plan, or nothing.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
