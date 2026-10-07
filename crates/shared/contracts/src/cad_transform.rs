//! The modify commands of the product command catalog (docs/adr/0037,
//! 0047): objects named by their persistent ids moved, rotated, scaled,
//! mirrored or aligned, in place or as copies. The move, copy, rotate,
//! scale, mirror and align tools write through `cad.entities.transform` on
//! the web (`apps/web/src/product`) and on the desktop
//! (`crates/native/application`); both pass the shared cases in
//! `fixtures/commands/v1`.
//!
//! The transform is typed by what the tools ask for (a displacement, a
//! centre and an angle, a centre and a factor, the two points of an axis,
//! source and target points), not a matrix: the shared geometry core builds
//! the matrix from it on both platforms and moves every kind of object with
//! it (arcs stay counter-clockwise, mirrored text stays readable).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::entity::{Entity, Vec2};

/// Moves, rotates, scales or mirrors objects, in place or as copies.
pub const CAD_ENTITIES_TRANSFORM: &str = "cad.entities.transform";
pub const CAD_ENTITIES_TRANSFORM_VERSION: u32 = 1;

/// One transform of the plane: a similarity as the modify tools ask for it
/// (docs/adr/0037, 0047), Vektör oturtma's similarity, affine or projective
/// transform in centred form (docs/adr/0156 §6), or Kauçuk levha's links
/// (docs/adr/0158). Coordinates are x east (Y), y north (X), in the
/// project's units (m), float64.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum Transform {
    /// Move (taşı) by a displacement: `dx` east, `dy` north.
    Move { dx: f64, dy: f64 },
    /// Rotate (döndür) about `center` by `angle` radians, counter-clockwise.
    Rotate { center: Vec2, angle: f64 },
    /// Scale (ölçekle) about `center` by `factor`, above zero.
    Scale { center: Vec2, factor: f64 },
    /// Mirror (aynala) across the line through `a` and `b`, two different points.
    Mirror { a: Vec2, b: Vec2 },
    /// Align (hizala, AutoCAD's ALIGN): `source` goes onto `target`. With a
    /// second pair (`source2` and `target2`, given together) the objects
    /// also turn about `source` so that the direction from `source` to
    /// `source2` lies along the one from `target` to `target2`, and with
    /// `scale` they are scaled about it so that the one length becomes the
    /// other. The second pair's points must lie apart from the first's;
    /// `scale` means nothing without a second pair.
    Align {
        source: Vec2,
        target: Vec2,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        source2: Option<Vec2>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        target2: Option<Vec2>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        scale: Option<bool>,
    },
    /// Oturt (docs/adr/0156): a similarity between centred frames, `from`
    /// the source centre, `to` the target's: x′ = to.x + a·x̄ − b·ȳ,
    /// y′ = to.y + b·x̄ + a·ȳ, x̄ = x − from.x, ȳ = y − from.y. Every kind
    /// moves as the modify tools move it.
    Similarity { from: Vec2, to: Vec2, a: f64, b: f64 },
    /// Oturt: an affine transform between centred frames, `m` = [a, b, c, d]:
    /// x′ = to.x + a·x̄ + c·ȳ, y′ = to.y + b·x̄ + d·ȳ. Not a similarity: a
    /// circle, an arc or an ellipse becomes the ellipse of its image (an
    /// object may change its kind), a path's arc segments straight vertices
    /// within 0.1 mm; texts, notes, blocks, dimensions and hatch patterns
    /// keep their shape at their anchor (docs/adr/0156 §4, §5).
    Affine { from: Vec2, to: Vec2, m: [f64; 4] },
    /// Oturt: a projective transform between centred frames, `h` = [a1, a2,
    /// a3, b1, b2, b3, c1, c2]: w = c1·x̄ + c2·ȳ + 1, x′ = to.x + (a1·x̄ +
    /// a2·ȳ + a3)/w, y′ = to.y + (b1·x̄ + b2·ȳ + b3)/w. Lines stay straight;
    /// curves become straight vertices within 0.1 mm; a point where w is
    /// 1e-9 or less lies beyond the horizon.
    Projective { from: Vec2, to: Vec2, h: [f64; 8] },
    /// Kauçuk levha (docs/adr/0158): the thin plate spline of the links'
    /// displacements, through every link exactly (a link whose `to` is its
    /// `from` holds its point), following their affine trend far from them.
    /// Only vertices move: straight edges stay straight, an arc segment keeps
    /// its bulge; a circle, an arc and an ellipse move by the nearest
    /// similarity at their centre; texts, notes, blocks, dimensions and hatch
    /// patterns keep their shape at their anchor. 3 to 1000 links, their
    /// sources apart and not all on one line.
    Rubbersheet { links: Vec<RubberLink> },
    /// Hizala ve dağıt (docs/adr/0194): each object moves on its own along
    /// one axis. The six alignments put its box's west side (`left`),
    /// middle (`center`) or east side (`right`) on the easting `at`, its
    /// north side (`top`), middle (`middle`) or south side (`bottom`) on
    /// the northing `at`; the two spreads (`horizontal`, `vertical`) keep
    /// the first and the last box by their middles and space the others
    /// at equal gaps between them, `at` not given. A box is the object's
    /// extent as the drawing measures it (an insert with its block's
    /// pieces, a text in the drawing's typeface).
    Arrange {
        mode: ArrangeMode,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        at: Option<f64>,
    },
}

/// What Hizala ve dağıt does (docs/adr/0194): six alignments and two spreads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ArrangeMode {
    Left,
    Center,
    Right,
    Top,
    Middle,
    Bottom,
    Horizontal,
    Vertical,
}

/// A link of Kauçuk levha: a point of the drawing and where it is to go.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RubberLink {
    pub from: Vec2,
    pub to: Vec2,
}

/// Input of `cad.entities.transform` v1: objects named by their persistent
/// ids (docs/adr/0014) moved by one transform as one undo step, in place or
/// as copies. The modify tools make the selection explicit here (TODOS.md
/// CMD-07): they give the selected objects' ids; the command reads no
/// selection, layer or view.
///
/// In place, each object keeps its slot, its persistent id and every other
/// field; only its geometry changes. As copies, each new object takes every
/// field of its original (layer, colour, attributes, label, symbol) and a new
/// persistent id; the originals stay. The undo step is the tool's name:
/// “Taşı” (a move), “Kopyala” (a move as copies), “Döndür”, “Ölçekle”,
/// “Aynala”, “Hizala”; of an arrangement the mode's name (“Sola hizala”,
/// “Ortala”, “Sağa hizala”, “Üste hizala”, “Ortaya hizala”, “Alta hizala”,
/// “Yatay dağıt”, “Dikey dağıt”).
///
/// Objects on a locked layer (by itself or a group above it) stay where they
/// are and are not copied: with others to transform they are named in the
/// output's `locked` with a `layer_locked` warning; when every one is locked
/// nothing is written and the answer is `layer_locked`. A repeated id counts
/// once.
///
/// Refusals (`CommandError.code`), checked in this order: `no_entities`,
/// `invalid_uid` (each id in order), `not_finite` (the transform's numbers,
/// in their order), `invalid_factor` (a scale not above zero),
/// `invalid_axis` (a mirror axis without a direction), `invalid_align` (an
/// alignment's second pair given by half, or its source or target points
/// within a nanometre of the first's), `invalid_transform` (an affine or
/// projective transform whose linear part squashes the plane: its
/// determinant under 1e-12 of its columns' lengths' product; an
/// arrangement's `at` missing for an alignment or given for a spread),
/// `invalid_links` (Kauçuk levha's links: fewer than 3 or more than 1000,
/// two from one point, all their sources on one line, or equations with no
/// single solution; path `transform.links`),
/// `invalid_revision`, `revision_conflict` (status `conflict`),
/// `entity_not_found` (each id in order), `layer_locked`, `too_few_objects`
/// (a spread with fewer than three objects off locked layers), `beyond_horizon`
/// (a point of an object beyond a projective transform's horizon), then
/// `not_finite` again (path `transform`) when the transform would carry a
/// coordinate past the largest float64; on the desktop also
/// `slots_exhausted` for copies.
///
/// The undo step of the similarity, affine and projective transforms is
/// “Oturt”, of the rubber sheet “Kauçuk levha”. When the transform is not a
/// similarity the output warns with `warp_curves` (objects whose curves
/// became straight vertices) and `warp_shapes` (texts, notes, blocks,
/// dimensions and hatch patterns that kept their shape), each with its
/// count; a rubber sheet with `rubber_bends` (objects whose kept edges or
/// curves lie over 0.1 mm from their true image, and the largest, mm).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesTransform {
    /// The objects' persistent ids (lowercase UUID text with hyphens), at least one.
    #[cfg_attr(feature = "schema", schemars(inner(regex(pattern = UID_TEXT))))]
    pub uids: Vec<String>,
    /// What happens to them.
    pub transform: Transform,
    /// True: copies are made and the originals stay (Kopyala, Kopya (K),
    /// Aynala keeping its source). Absent or false: the objects themselves change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub copy: Option<bool>,
    /// The document revision the input was prepared against, as decimal text
    /// (from a plan, or the document). When given and the document is no
    /// longer at it, nothing is written and the answer is `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.entities.transform` v1: what changed, what was made and
/// what stayed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesTransformed {
    /// In place: the ids of the objects transformed, in the input's order (a
    /// repeated one once). Empty for copies.
    pub changed: Vec<String>,
    /// As copies: the new objects' persistent ids, in the order of their
    /// originals in the input. Empty in place.
    pub created: Vec<String>,
    /// The ids left alone because their layer is locked, in the input's order.
    pub locked: Vec<String>,
    /// The document's revision after the write, as decimal text. Inside an
    /// open transaction or group the write joins it, and the revision
    /// changes when that ends.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}

/// What `cad.entities.transform` would write (plan mode); nothing is written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EntitiesTransformPlan {
    /// The ids of the objects execute would transform or copy, in the input's order.
    pub sources: Vec<String>,
    /// Each of them as execute would write it, in the same order: in place
    /// with its own slot (`id`); a copy with `id` 0, as its slot is given
    /// when it is written.
    pub entities: Vec<Entity>,
    /// The ids it would leave alone because their layer is locked.
    pub locked: Vec<String>,
    /// The document revision the plan was made against. Give it as
    /// `expectedRevision` to write exactly this plan, or nothing.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub revision: String,
}
