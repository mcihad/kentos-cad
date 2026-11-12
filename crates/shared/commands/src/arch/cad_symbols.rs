//! Mimari işaretler (docs/adr/0721): `cad.north.create` (Kuzey oku), KentOS's library north arrow placed turned to the
//! plan's, the true or a given north; `cad.revisions.mark` (Revizyon işaretle), a revision cloud and its delta with
//! the revision's number in one step; `cad.legends.materials` (Malzeme lejandı), a legend of the materials the walls
//! are drawn with, each one's hatch beside its name, made again in its place when the walls change.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

#[cfg(feature = "schema")]
use crate::cad::{REVISION_TEXT, UID_TEXT};
use crate::{BlockDefinition, BlockId, Entity, Vec2};

/// The command's name.
pub const CAD_NORTH_CREATE: &str = "cad.north.create";
/// Its version.
pub const CAD_NORTH_CREATE_VERSION: u32 = 1;
/// The command's name.
pub const CAD_REVISIONS_MARK: &str = "cad.revisions.mark";
/// Its version.
pub const CAD_REVISIONS_MARK_VERSION: u32 = 1;
/// The command's name.
pub const CAD_LEGENDS_MATERIALS: &str = "cad.legends.materials";
/// Its version.
pub const CAD_LEGENDS_MATERIALS_VERSION: u32 = 1;

/// The block a north arrow is: the drawing's own of this name, else KentOS's library's (docs/adr/0721 §3).
pub const NORTH_BLOCK: &str = "Kuzey oku";
/// The block a material legend is made as (docs/adr/0721 §5).
pub const LEGEND_BLOCK: &str = "Malzeme lejandı";
/// The block a revision's delta is (docs/adr/0721 §4), its number the attribute [`REVISION_TAG`].
pub const REVISION_BLOCK: &str = "Revizyon";
/// The delta's attribute that shows the revision's number.
pub const REVISION_TAG: &str = "REV";
/// The delta's data its date and description are written as, the Revizyon listesi's further columns.
pub const REVISION_DATE: &str = "Tarih";
pub const REVISION_DESCRIPTION: &str = "Açıklama";
pub const REVISION_DATA: [&str; 2] = [REVISION_DATE, REVISION_DESCRIPTION];
/// A delta's size across on paper when none is given, millimetres.
pub const REVISION_SIZE: f64 = 6.0;
/// The most characters a revision's number, date and description may have.
pub const MAX_REVISION_NUMBER: usize = 16;
pub const MAX_REVISION_DATE: usize = 32;
pub const MAX_REVISION_DESCRIPTION: usize = 240;
/// A north arrow's size across on paper when none is given, millimetres.
pub const NORTH_SIZE: f64 = 15.0;
/// The most a north arrow may be across on paper, millimetres.
pub const MAX_NORTH_SIZE: f64 = 200.0;

/// Where a north arrow points (docs/adr/0721 §3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum NorthDirection {
    /// The plan's north: the drawing's up (+Y), grid north in a projected system (Revit's Project North).
    #[default]
    Plan,
    /// True north at the arrow: the meridian through it, from the project's coordinate system (a transverse
    /// Mercator's meridian convergence; a local system on one, turned by its plane).
    True,
    /// A north turned off the drawing's up by `angle`, counter-clockwise (a project drawn square to its site).
    Angle {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
        angle: f64,
    },
}

impl NorthDirection {
    /// Whether it is the plan's (written as no field).
    pub fn is_plan(&self) -> bool {
        *self == NorthDirection::Plan
    }
}

/// Input of `cad.north.create` v1 (“Kuzey oku”, docs/adr/0721 §3): the north arrow placed at `at` on `layerId`,
/// turned to `north`, `size` paper millimetres across (east to west, unturned) at the drawing's scale. It is the
/// drawing's own block “Kuzey oku” when it has one; else KentOS's library's, added in the same step. One undo step
/// “Kuzey oku”.
///
/// Refusals, in this order: `not_finite` (`at`, then the angle, then the size), `invalid_scale` (a size not above 0
/// or above 200 mm), `invalid_revision`, `revision_conflict` (status `conflict`), `no_true_north` (true north asked
/// in a project whose system has no meridian: a local project, or one the point is outside), then the layer's as
/// every create command's (`layer_not_found`, `not_a_layer`, `layer_locked`). Warnings: `layer_hidden`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NorthCreate {
    /// Where the arrow's base point goes: its circle's centre.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Vec2,
    /// The layer it goes on, a layer's id.
    pub layer_id: String,
    /// Where it points; absent: the plan's north.
    #[serde(default, skip_serializing_if = "NorthDirection::is_plan")]
    #[cfg_attr(feature = "ts", ts(as = "Option<NorthDirection>", optional))]
    pub north: NorthDirection,
    /// Its size across on paper, millimetres, above 0 and at most 200; absent: 15.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub size: Option<f64>,
    /// Its own colour (absent: its layer's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.north.create` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NorthCreated {
    /// The insert's persistent id.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub id: u32,
    /// The block placed.
    pub block: BlockId,
    /// The library's block was added in this step (the drawing had none of the name).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub added: bool,
    /// The arrow's turn, counter-clockwise from the drawing's up.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub rotation: f64,
    /// True north's angle from the plan's north at `at`, counter-clockwise (the meridian convergence); `true` only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub convergence: Option<f64>,
    pub revision: String,
}

/// Plan of `cad.north.create` v1: what execute would write.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NorthPlan {
    /// The insert, `id` 0.
    pub insert: Entity,
    /// The library's definition execute would add (absent: the drawing's own is placed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub block: Option<BlockDefinition>,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub rotation: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "rad")))]
    pub convergence: Option<f64>,
    pub revision: String,
}

/// Input of `cad.revisions.mark` v1 (“Revizyon işaretle”, docs/adr/0721 §4): a revision cloud round `frame` (as
/// `cad.revcloud.create` draws it, its arcs `arc` long) and its delta, KentOS's library block “Revizyon” (the
/// drawing's own when it has one), `size` paper millimetres across at the drawing's scale, at `at` (absent: off the
/// cloud's top right corner, out by half an arc and half the delta). The revision's `number` is the delta's `REV`
/// and the cloud's attribute `Revizyon`; `date` and `description`, when given, both objects' `Tarih` and
/// `Açıklama` (a Blok çizelgesi of “Revizyon” by `REV` lists the revisions). One undo step “Revizyon”.
///
/// Refusals, in this order: `invalid_mark` (the number blank, edged with spaces, with control characters or longer
/// than 16; a date longer than 32; a description longer than 240), `not_finite` (`at`, then the size), `invalid_scale`
/// (a size not above 0 or above 100 mm), then `cad.revcloud.create`'s for the frame and the arcs, the expected
/// revision and the layer. Warnings: `layer_hidden`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RevisionsMark {
    pub layer_id: String,
    pub frame: crate::RevcloudFrame,
    /// The arcs' chord, in the drawing's length (m), above zero.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub arc: f64,
    /// The revision's number or letter: 1, A, B2.
    pub number: String,
    /// Its date, as written (absent: none).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub date: Option<String>,
    /// What changed (absent: none).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub description: Option<String>,
    /// Where the delta's centre goes; absent: off the cloud's top right corner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Option<Vec2>,
    /// The delta's size across on paper, millimetres, above 0 and at most 100; absent: 6.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub size: Option<f64>,
    /// Both objects' own colour (absent: their layer's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub color: Option<String>,
    /// The cloud's own line weight, paper millimetres (absent: its layer's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(range(min = 0.0, max = 100.0)))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub line_weight: Option<f64>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// Output of `cad.revisions.mark` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RevisionMarked {
    /// The cloud's persistent id and slot.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub cloud: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub cloud_id: u32,
    /// The delta's persistent id and slot.
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub tag: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub tag_id: u32,
    /// The delta's block.
    pub block: BlockId,
    /// The library's block was added in this step.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub added: bool,
    pub revision: String,
}

/// Plan of `cad.revisions.mark` v1: what execute would write.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RevisionPlan {
    /// The cloud, `id` 0.
    pub cloud: Entity,
    /// The delta, `id` 0.
    pub tag: Entity,
    /// The library's definition execute would add (absent: the drawing's own is placed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub block: Option<BlockDefinition>,
    pub revision: String,
}

/// Input of `cad.legends.materials` v1 (“Malzeme lejandı”, docs/adr/0721 §5): a legend of the materials the walls are
/// drawn with: a row for each component of theirs (in the drawing's order, left face first) that differs from the
/// rows before in its name or its hatch, its swatch hatched as the component and its name (its material, else its
/// own name) beside. Made as the block “Malzeme lejandı”, `height` paper millimetres its text, at the drawing's
/// scale: a legend the drawing has is made again in its place (every insert of it follows); `at` places one more.
/// One undo step “Malzeme lejandı”.
///
/// Refusals, in this order: `not_finite` (`at`, then the height), `invalid_height` (not above 0 or above 50 mm),
/// `invalid_revision`, `revision_conflict` (status `conflict`), `no_materials` (no wall with a component), `no_legend`
/// (no `at` and no legend to make again), then, with `at`, the layer's as every create command's
/// (`layer_not_found`, `not_a_layer`, `layer_locked`). Warnings: `layer_hidden`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LegendsMaterials {
    /// Where a new insert's top left corner goes; absent: the legend is only made again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Option<Vec2>,
    /// The layer a new insert goes on, a layer's id (read with `at`).
    pub layer_id: String,
    /// Its text's height on paper, millimetres, above 0 and at most 50; absent: 2.5.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub height: Option<f64>,
    /// The document revision the input was prepared against, as decimal text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = REVISION_TEXT)))]
    pub expected_revision: Option<String>,
}

/// One row of a material legend.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LegendMaterial {
    /// Its name: the component's material, else its own name.
    pub name: String,
    /// Its hatch pattern's name (absent: none, an empty swatch).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub pattern: Option<String>,
    /// The project material it is (docs/adr/0785), by its id; absent: a text material.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub material: Option<String>,
    /// How many walls have it.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub walls: u32,
}

/// Plan of `cad.legends.materials` v1: the rows, the block as execute would write it and the new insert (`id` 0).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LegendPlan {
    pub materials: Vec<LegendMaterial>,
    pub block: BlockDefinition,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub remade: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub insert: Option<Entity>,
    pub revision: String,
}

/// Output of `cad.legends.materials` v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LegendMade {
    /// The rows, top down.
    pub materials: Vec<LegendMaterial>,
    /// The legend's block.
    pub block: BlockId,
    /// The block was made again (the drawing had it); false: added.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub remade: bool,
    /// The new insert's persistent id, with `at`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(regex(pattern = UID_TEXT)))]
    pub uid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub id: Option<u32>,
    pub revision: String,
}
