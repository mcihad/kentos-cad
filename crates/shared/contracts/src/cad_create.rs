//! The create command of the product command catalog (docs/adr/0057): new
//! objects of any kind on a named layer, as one undo step. The drawing tools
//! whose objects have no command of their own write through
//! `cad.entities.create` on the web (`apps/web/src/product`) and on the
//! desktop (`crates/native/application`): Elips, Eğri, Yardımcı çizgi, Işın,
//! Halka, Paralel çizgi, Dik in, Dik çık and Böl; so do the Hesap windows'
//! “Çizime ekle” (Poligon hesabı, Kutupsal alım, Önden and Geriden kestirme).
//! Both pass the shared cases in `fixtures/commands/v1`.
//!
//! The geometry is given, not computed here: the tools compute it with the
//! shared geometry core from what was clicked and typed (an ellipse from its
//! axis, the sides of a parallel line, the points along an object), and the
//! command writes what the preview showed. A geometry is typed as
//! `cad.entities.edit` types it ([`EntityGeometry`]) and checked by the same
//! rules, so a geometry one command takes the other takes too.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::cad_edit::EntityGeometry;
use crate::entity::Entity;

/// Writes new objects on a named layer in one undo step.
pub const CAD_ENTITIES_CREATE: &str = "cad.entities.create";
pub const CAD_ENTITIES_CREATE_VERSION: u32 = 1;

/// The drawing tool or Hesap window whose step has its own name; without
/// one the step is “Ekle”, as for every object a drawing tool adds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CreateOperation {
    /// Paralel çizgi: the lines beside an axis, the corridor between them, the axis.
    Parallel,
    /// Dik in: a perpendicular from a point down to a reference line.
    PerpendicularIn,
    /// Dik çık: a perpendicular up from a point of a reference line.
    PerpendicularOut,
    /// Böl: points along an object.
    Divide,
    /// Tarama: a hatch filling the region clicked inside.
    Hatch,
    /// İçine tıklayarak alan: the region clicked inside, as an area. The step is “Alan oluştur”.
    Boundary,
    /// Poligon hesabı: the traverse's new points (Hesap menu).
    Traverse,
    /// Kutupsal alım: the points measured from a station (Hesap menu).
    PolarSurvey,
    /// Önden kestirme: the point found from two known points (Hesap menu).
    ForwardIntersection,
    /// Geriden kestirme: the station found from three known points (Hesap menu).
    Resection,
    /// Ara nokta: points on the line between two points (docs/adr/0140).
    PointsBetween,
    /// Kesişim noktası: the point where two distances, two bearings or two lines meet.
    IntersectPoint,
    /// Zincir ölçü: the next dimension of a chain.
    DimensionChain,
    /// Baz ölçü: a dimension measured from the base dimension's first point.
    DimensionBaseline,
    /// Metin dosyası yerleştir (docs/adr/0145 §6): a text file's lines as texts.
    TextFile,
    /// Kılavuz (docs/adr/0146 §6): a leader drawn by its tool.
    Leader,
    /// Toplu alan (docs/adr/0151): the regions line work closes, as areas.
    Polygonize,
    /// Köşelere nokta (docs/adr/0152): named points at the vertices of lines and areas.
    VertexPoints,
    /// Bitişik alan (docs/adr/0162 §3): the region a drawn path closes with
    /// the neighbouring areas, as one area (its parts and holes as they are).
    Adjoin,
    /// Etiketleri yazıya çevir (docs/adr/0175 §2): the layers' labels as
    /// texts, placed and sized as a sheet at a scale writes them.
    Labels,
    /// Tablo (docs/adr/0184 §3): a table placed by its tool.
    Table,
    /// Koordinat yaz (docs/adr/0185): coordinate labels (texts and their
    /// leaders) at clicked points or at the selection's vertices.
    Coordinates,
    /// Km yaz (docs/adr/0189): a route's stations: their ticks, km texts,
    /// cross-sections and points.
    Stations,
    /// Orta hat (docs/adr/0190): the axis between two sides, a polyline.
    Centerline,
    /// Resim ekle (docs/adr/0192): a picture placed.
    Image,
    /// Eğri boyunca yazı (docs/adr/0196 §4): a text along a curve.
    TextAlong,
    /// İki daireye teğet (docs/adr/0197 §1): a common tangent of two circles or arcs, a line.
    TangentLine,
    /// Dördüncü köşe (docs/adr/0197 §2): a parallelogram's fourth corner, a point or the four as an area.
    FourthCorner,
    /// Menzil halkaları (docs/adr/0197 §3): rings round a centre and rays to the outer one.
    RangeRings,
    /// Plan yolu (docs/adr/0198 §2): a road's areas from its axis, and the axis.
    PlanRoad,
    /// Yatay ağ dengelemesi (docs/adr/0203 §8): the network's new points the drawing has not.
    NetworkAdjust,
    /// Raster ekle (docs/adr/0204 §8): a raster placed.
    Raster,
    /// Nokta bulutu ekle (docs/adr/0207 §9): a point cloud added.
    PointCloud,
}

/// One new object: its geometry and what else it carries. The layer is the
/// input's; the persistent id and the slot are given when it is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NewObject {
    /// Its kind and the fields that place and shape it.
    pub geometry: EntityGeometry,
    /// Colour override (`EntityBase.color`). Absent: the layer's colour (katmana göre).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    /// Its own line weight, paper mm (`EntityBase.line_weight`, 0 the
    /// thinnest, at most 100; docs/adr/0139): what the tools give a new
    /// object from the current weight. Absent: the layer's (katmana göre).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 0.0, max = 100.0)))]
    pub line_weight: Option<f64>,
    /// GIS attributes, text in v1. Absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attrs: Option<BTreeMap<String, String>>,
    /// The text shown beside it (`EntityBase.label`). Absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label: Option<String>,
    /// Its own symbol, a library item's id (`EntityBase.symbol`), drawn
    /// instead of its layer's style: an object template's (docs/adr/0176).
    /// The id is not looked up: the libraries are the host's. Absent: the
    /// layer's style.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub symbol: Option<String>,
    /// The object whose label this new text writes (Etiketleri yazıya
    /// çevir's “Nesneye bağlı”, docs/adr/0175 §4): its persistent id
    /// (lowercase UUID text with hyphens), an object of the drawing. On a
    /// text only, given with `labelScale`; the text then follows the object
    /// (`TextEntity.label_of`). Absent: a text of its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub label_of: Option<String>,
    /// The scale's denominator (1:N) the linked label is written at:
    /// finite, over 0 (`TextEntity.label_scale`). Given with `labelOf`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub label_scale: Option<f64>,
}

/// Input of `cad.entities.create` v1: new objects on a named layer, written
/// as one undo step. Everything the command depends on is here (TODOS.md
/// CMD-07): the tool fills `layerId` from the active layer and each object's
/// `color` from the current colour; the command reads neither.
///
/// The objects are written in their order, each with a new persistent id.
/// The undo step is named after `operation`, or “Ekle”.
///
/// Refusals (`CommandError.code`), checked in this order: `no_objects`, then
/// each object's geometry in order: `too_few_points` (a polyline),
/// `too_few_corners` (a closed area's or a hatch's ring or hole, by
/// `cad.entities.edit`'s rule: a closed area's may have 2 corners when an
/// edge is an arc), `empty_text` (a text whose text is empty or only white space),
/// `invalid_elevations`, `not_finite`, `invalid_radius`, `invalid_scale` (an
/// insert's), `invalid_line_weight` (the object's weight not from 0 to 100
/// mm), `invalid_link` (`labelOf` or `labelScale` on an object that is not
/// a text, one without the other, an id that is not lowercase UUID text
/// with hyphens, a scale not finite or not over 0; docs/adr/0175 §4); then
/// `invalid_revision`, `revision_conflict` (status `conflict`),
/// `layer_not_found`, `not_a_layer`, `layer_locked`, `unknown_block` (each
/// insert's block, in order; docs/adr/0144), `link_not_found` (each linked
/// text's object, in order: no object of the drawing has that id), then
/// `invalid_attribute` and `attribute_required` (each object's attributes by
/// the layer's fields, objects in order, names in theirs, at
/// `objects[i].attrs.<name>`; docs/adr/0199 §2); on the desktop also
/// `slots_exhausted`. Warning: `layer_hidden` (they are written all the same).
///
/// A layer with fields (docs/adr/0199 §2): a value given to a field's key is
/// written in its canonical text; a value that does not keep the field's
/// rules is refused (`invalid_attribute`), an empty one of a required field
/// too (`attribute_required`); a field an object does not give takes its
/// default. A required field without a value or a default is not refused.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesCreate {
    /// The layer they go on: a layer's id (`LayerNode.id`), not a group's.
    pub layer_id: String,
    /// What is written, at least one, in this order.
    pub objects: Vec<NewObject>,
    /// The drawing tool or Hesap window the objects come from, when its step
    /// has its own name: Paralel çizgi, Dik in, Dik çık, Böl, Tarama, Alan
    /// oluştur, Poligon hesabı, Kutupsal alım, Önden kestirme, Geriden
    /// kestirme, Toplu alan, Köşelere nokta, Bitişik alan, Etiketleri yazıya
    /// çevir. Absent: “Ekle”.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub operation: Option<CreateOperation>,
    /// The document revision the input was prepared against, as decimal text
    /// (from a plan, or the document). When given and the document is no
    /// longer at it, nothing is written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.entities.create` v1: the objects written, in the input's order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesCreated {
    /// Their persistent ids (UUIDv7, docs/adr/0014): the names files, the
    /// cloud, Python and AI use.
    pub created: Vec<String>,
    /// Their slots in the open document (`Entity.id`); they mean nothing
    /// once the document is closed.
    pub ids: Vec<u32>,
    /// The document's revision after the write, as decimal text. Inside an
    /// open transaction or group the write joins it, and the revision
    /// changes when that ends.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.entities.create` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesCreatePlan {
    /// The objects as they would be stored, in the input's order: `id` 0,
    /// as their slots are given when they are written.
    pub entities: Vec<Entity>,
    /// The document revision the plan was made against. Give it as
    /// `expectedRevision` to write exactly this plan, or nothing.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
