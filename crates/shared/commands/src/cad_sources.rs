//! The commands of bound layers (docs/adr/0503, plan/gis.md CBS-01, CBS-02, CBS-11, CBS-36): what a file's tables are
//! and what the project's bound layers' state is (`cad.sources.describe`), binding, unbinding and relocating a layer
//! (`cad.sources.edit`), and writing a layer's changes to its table or reading the table again (`cad.sources.sync`).
//! A layer and its table are compared object by object, by lasting id, on their geometry and attributes.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::fields::LayerFieldKind;
use crate::file_import::ImportedLayer;
use crate::sources::SourceLink;

pub const CAD_SOURCES_DESCRIBE: &str = "cad.sources.describe";
pub const CAD_SOURCES_DESCRIBE_VERSION: u32 = 1;
pub const CAD_SOURCES_EDIT: &str = "cad.sources.edit";
pub const CAD_SOURCES_EDIT_VERSION: u32 = 1;
pub const CAD_SOURCES_SYNC: &str = "cad.sources.sync";
pub const CAD_SOURCES_SYNC_VERSION: u32 = 1;

/// What a source can do (docs/adr/0503 §1): each told apart, so a refusal says which.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourceCapabilities {
    pub read: bool,
    pub insert: bool,
    pub update: bool,
    pub delete: bool,
    /// Every write of a sync in one transaction.
    pub transaction: bool,
    pub z: bool,
    pub m: bool,
    /// Arcs kept as arcs.
    pub curves: bool,
}

/// A column of a table and the field kind it reads as; none for one that is not read (a blob).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourceColumn {
    pub name: String,
    /// As the file declares it (`MEDIUMINT`, `TEXT(40)` …).
    pub declared: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub kind: Option<LayerFieldKind>,
}

/// A table of a file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourceTable {
    pub name: String,
    /// `features` or `attributes`.
    pub data: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rows: u64,
    /// The geometry column's type, none for an attribute table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub geometry: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "epsg")))]
    pub srid: Option<u32>,
    /// The box the source declares for the table, in the table's system: min x, min y, max x, max y (docs/adr/0599
    /// §2). A GeoPackage's `gpkg_contents`, PostGIS's `ST_EstimatedExtent`; declared, not counted, and none when
    /// the source declares none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "system")))]
    pub extent: Option<[f64; 4]>,
    pub columns: Vec<SourceColumn>,
    pub capabilities: SourceCapabilities,
    /// The relations of the file this table is a side of (GeoPackage related tables, docs/adr/0532 §2), each with
    /// whether KentOS reads and writes its representation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<SourceRelation>>", optional))]
    pub relations: Vec<SourceRelation>,
}

/// How a relation of a file is held (docs/adr/0532 §2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SourceRelationKind {
    /// KentOS's representation (`kentos_references`): a mapping table of base and related rows.
    Kentos,
    /// A well-formed relation of another kind: read and listed, never rewritten.
    Foreign,
    /// A relation whose mapping table or tables are not as the profile says; `why` says how.
    Broken,
}

/// A relation of a file's table (a row of `gpkgext_relations`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourceRelation {
    /// The relation's name as the file writes it (`kentos_references`, `media`, …).
    pub name: String,
    /// The base table (for KentOS's: the child, which holds the references).
    pub base: String,
    /// The related table (for KentOS's: the parent).
    pub related: String,
    /// The mapping table.
    pub mapping: String,
    /// How many rows the mapping table holds.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rows: u64,
    pub kind: SourceRelationKind,
    /// Why a broken relation is (none otherwise).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub why: Option<String>,
}

/// What a bound layer's state asks for next (docs/adr/0543): the one way out a user and a script take from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SourceNext {
    /// The layer and the table hold the same.
    None,
    /// Only the layer changed: `cad.sources.sync` with `write`.
    Write,
    /// Only the file changed (or it differs without either side's mark): `cad.sources.sync` with `read`.
    Read,
    /// Both changed: `cad.sources.reconcile`.
    Reconcile,
    /// The file, its table or the database is not reached: `cad.sources.edit` `relink` (a file), the address again (a
    /// database).
    Relink,
}

/// A bound layer's state: whether its file is there, whether the file changed since the layer last matched it, and
/// how the layer differs from it now (objects the layer has and the table lacks, both have but differ, the table has
/// and the layer lacks); how much of the table the layer holds and what to do next.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourceState {
    pub link: SourceLink,
    pub found: bool,
    /// The file changed since the layer last matched it.
    pub changed: bool,
    /// The layer changed since it last matched the file.
    pub edited: bool,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub added: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub modified: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub removed: u32,
    /// The layer's objects, of any kind.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    /// The table's rows the working copy covers (a part's: those whose box meets its area; docs/adr/0510) and the
    /// layer can hold; none when the table is not reached.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub rows: Option<u32>,
    /// The whole table's rows; none when the table is not reached.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub total: Option<u32>,
    pub next: SourceNext,
}

impl SourceNext {
    /// The rule (docs/adr/0543): not reached, relink; no difference, none; both sides changed, reconcile; the layer
    /// changed, write; else read.
    pub fn of(found: bool, reached: bool, changed: bool, edited: bool, differences: u32) -> Self {
        if !found || !reached {
            Self::Relink
        } else if differences == 0 {
            Self::None
        } else if changed && edited {
            Self::Reconcile
        } else if edited {
            Self::Write
        } else {
            Self::Read
        }
    }
}

/// Input of `cad.sources.describe` v1: a file's or a PostGIS database's tables (`path`), or the project's bound layers'
/// state (none). Refusals: `invalid_file_path`, `file_not_found`, `import_unreadable`, `invalid_address`,
/// `source_unreachable` (docs/adr/0505).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourcesDescribe {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub path: Option<String>,
}

/// Output of `cad.sources.describe` v1, and its plan.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourcesDescribed {
    /// The file's tables (with `path`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<SourceTable>>", optional))]
    pub tables: Vec<SourceTable>,
    /// The project's bound layers (without `path`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<SourceState>>", optional))]
    pub links: Vec<SourceState>,
    pub revision: String,
}

/// An edit of the project's bound layers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SourcesEditOp {
    /// Binds a table: onto a new layer with its rows (one undo step, as `file.import.gpkg` takes it), or onto the
    /// `layer` given, which must hold the table's objects as they are. A table in another coordinate system is bound
    /// only with `reproject` (docs/adr/0504): the layer holds its objects in the project's system, the table keeps
    /// its own, and every sync moves them across.
    Link {
        path: String,
        table: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        layer: Option<String>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
        reproject: bool,
        /// Onto a new layer from a file, only the rows whose box meets this area (minimum east, north, maximum east,
        /// north, in the project's system; docs/adr/0510): the working copy of a part of a large table.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        extent: Option<[f64; 4]>,
        /// The attribute that names a row whatever its place in the table (docs/adr/0533 §1): unique and not empty
        /// among the rows taken in.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        key: Option<String>,
    },
    /// Sets or clears (`key` absent) a bound layer's key (docs/adr/0533 §1): the layer's objects must hold the
    /// attribute with a value each, no two alike. Nothing else changes; the next reload follows the pins by it.
    Key {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        key: Option<String>,
    },
    /// Changes the area a part's working copy holds (docs/adr/0533 §2), or takes the whole table (`extent`
    /// absent): the table is read again in the new area as one undo step, the rows that left it removed from
    /// the layer and the ones that came in added. The layer must match its table (no pending changes); the pins
    /// of an object that left the area are said. Undoing brings the objects back; the binding's area, like any
    /// setting, is not an undo step.
    Extent {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
        extent: Option<[f64; 4]>,
    },
    /// Unbinds a layer; its objects stay.
    Unlink { id: String },
    /// The file moved: binds the layer to the file at `path`, which must be the same file (the revision's).
    Relink { id: String, path: String },
}

/// Input of `cad.sources.edit` v1. Refusals: `invalid_file_path`, `file_not_found`, `import_unreadable`,
/// `table_not_found`, `layer_not_found`, `not_a_layer`, `layer_locked`, `crs_mismatch`, `source_bound` (the layer or
/// the table is bound already), `source_differs` (the layer given does not hold the table's objects),
/// `source_not_found` (an id), `source_changed` (relink to another file), `too_many_sources`, `invalid_extent` (an area
/// that is not one, on a table without geometry, onto a layer given, or of a database), `invalid_key` (a key that is
/// not a column of the objects, is empty on one or repeats); conflicts: `pending_changes` (an area changed over the
/// layer's own edits). Warnings: `label_pins_lost`, `label_pins_moved`, `label_pins_parked`,
/// `label_pins_restored` (an area change that took objects with label pins out of the layer, or brought them in).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourcesEdit {
    pub edit: SourcesEditOp,
}

/// Output of `cad.sources.edit` v1, and its plan.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourcesEdited {
    /// The binding as it is now; none after unlinking.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub link: Option<SourceLink>,
    /// The objects taken in, and the layers made (linking onto a new layer).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub objects: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ImportedLayer>>", optional))]
    pub layers: Vec<ImportedLayer>,
    /// What an area change did to the label pins (docs/adr/0533 §2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub pins: Option<SyncedPins>,
    pub revision: String,
}

/// Why a label pin went with no object to hold it (docs/adr/0533 §1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum PinLoss {
    /// The table has no such row any more.
    Deleted,
    /// The row is there but outside the area the layer holds.
    Outside,
    /// The row id is another record now (the key's value differs) and no row of the old value is left.
    Rebound,
    /// The row it would follow already has pins of its own.
    Taken,
    /// Without a key: more than one row has the object's geometry, or two are equally near it, or two objects chose
    /// one row (docs/adr/0578 §2): a pin is never given to a row on a guess.
    Ambiguous,
}

/// What tied a pin's old object to its new row (docs/adr/0578 §2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum PinMatch {
    /// The key's value.
    #[default]
    Key,
    /// Without a key: the one row of the same kind with the very same coordinates.
    Geometry,
    /// Without a key: the nearest row of the same kind within the tolerance (`distance`).
    Nearest,
}

impl PinMatch {
    pub fn is_key(&self) -> bool {
        *self == PinMatch::Key
    }
}

/// A label pin followed from one object to the row of the same key (docs/adr/0533 §1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PinMove {
    pub from: String,
    pub to: String,
    /// The key's value that tied them; absent when no key did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub key: Option<String>,
    /// How they were tied: the key (absent, as before docs/adr/0578), the very same geometry, or the nearest row.
    #[serde(default, skip_serializing_if = "PinMatch::is_key")]
    #[cfg_attr(feature = "ts", ts(as = "Option<PinMatch>", optional))]
    pub by: PinMatch,
    /// The distance between the two features' middles, metres, when `by` is `nearest`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub distance: Option<f64>,
}

/// The pins of an object a reload took out of the layer, or could not keep.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LostPin {
    /// The object's lasting id.
    pub uid: String,
    pub reason: PinLoss,
}

/// What a reload (or an area change) did to the label pins of the layer's objects (docs/adr/0533 §1): how many
/// objects kept theirs, which followed their key to another row, which could not be kept. A layer with no pins
/// has none of this.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SyncedPins {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub kept: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<PinMove>>", optional))]
    pub moved: Vec<PinMove>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<LostPin>>", optional))]
    pub lost: Vec<LostPin>,
    /// The objects whose pins were parked in the link now: their row is outside the part (docs/adr/0578 §3).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub parked: Vec<String>,
    /// The objects that got their parked pins back: their row is in the part again.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub restored: Vec<String>,
}

/// Which way a sync goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SyncDirection {
    /// The layer's changes into its table.
    Write,
    /// The table's rows into the layer.
    Read,
}

/// How a write takes the table's rows outside a part's area (docs/adr/0531, 0585): those sharing a corner or an edge
/// the written objects changed follow them, in the same transaction. A row is read by the changes' box from the file
/// or the database; a database's row is checked again under the table's lock (`source_changed` when someone changed it
/// since, `rows_denied` when the account may see but not change it: both profiles write nothing then).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SyncTopology {
    /// All or nothing: a row that would be left invalid (too few vertices, or newly invalid: a ring crossing itself, of
    /// no area, a hole out of its ring) refuses the whole write (`outside_neighbour_invalid`).
    Strict,
    /// The rows that can follow do; one that would be left invalid keeps its shape and is said (`neighbours_left`).
    Loose,
}

/// Input of `cad.sources.sync` v1. Refusals: `source_not_found` (the layer is not bound), `layer_not_found`,
/// `source_gone` (the file is gone), `source_readonly`, `geometry_mismatch`, `file_unwritable`, `rows_denied` (a
/// database's rows the account may not change: privilege or row security, docs/adr/0585 §2); conflicts:
/// `source_changed` (writing, and the file changed since the layer last matched it), `pending_changes` (reading, and
/// the layer differs from the table, without `discard`). Warnings: `export_losses` (objects a table has no geometry
/// for stay in the drawing only), `label_pins_lost` (a read that took objects with label pins out, or could not keep
/// their pins), `label_pins_moved` (pins that followed their key, or without a key their geometry or nearest feature
/// of the same kind within 0,05 m, to another row; docs/adr/0578 §2), `label_pins_parked` (pins of objects whose row
/// is outside the part's area now, kept in the link; docs/adr/0578 §3), `label_pins_restored` (parked pins given
/// back to their row that came into the part), `key_not_unique` (a read whose table repeats the layer's key: the
/// pins of those rows are not followed).
///
/// `with` (docs/adr/0532 §1) writes more bound layers in the same transaction: a parent and its children go in
/// together or not at all. Every layer is checked first; one refused (`source_readonly`, `geometry_mismatch`,
/// `source_changed`, `relation_dangling` …) and nothing is written. The layers must lie in one GeoPackage file
/// (`transaction_split`: another file, or a database, which writes one table a transaction), must not repeat, and
/// `with` is for writing (`invalid_input` with `read`). A relation KentOS maps (`cad.sources.relate`) is written with
/// its layers: its mapping rows follow the child's field (`relation_dangling`: a reference to an object the table
/// does not hold, not even after this write; `relation_unmapped`, a warning: a multi-valued reference that stays text).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourcesSync {
    /// The bound layer, by id.
    pub layer: String,
    pub direction: SyncDirection,
    /// Reading: take the table's rows though the layer differs (one undo step brings the layer back).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub discard: bool,
    /// Writing: more bound layers (by id) written in the same transaction: of the same GeoPackage file or the same
    /// PostGIS database (docs/adr/0532 §1, 0584 §1); layers of two sources are `transaction_split`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub with: Vec<String>,
    /// Writing a part's working copy (docs/adr/0531, 0585): the table's rows outside the area that share the changed
    /// corners and edges follow in the same transaction, a GeoPackage's or a PostGIS table's. Reading refuses it
    /// (`topology_on_read`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub topology: Option<SyncTopology>,
}

/// What a write did to one layer's table.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SyncedLayer {
    pub layer: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub added: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub modified: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub removed: u32,
}

/// What a write did to a mapped relation's mapping table: rows added and removed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SyncedRelation {
    /// The project relation's id.
    pub relation: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub added: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub removed: u32,
}

/// Output of `cad.sources.sync` v1, and its plan: what was (or would be) added, changed and removed on the other side,
/// the columns a write added to the table, and the file's revision after.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourcesSynced {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub added: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub modified: u32,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub removed: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub columns: Vec<String>,
    /// The file's SHA-256 after (plan: before).
    pub source: String,
    /// What reading did to the label pins of the layer's objects (docs/adr/0533 §1); only a read of a layer with pins.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub pins: Option<SyncedPins>,
    pub revision: String,
    /// A write with `with`: each layer's share, the first being `layer`; the counts above are their sums.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<SyncedLayer>>", optional))]
    pub layers: Vec<SyncedLayer>,
    /// The mapped relations' mapping rows written.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<SyncedRelation>>", optional))]
    pub relations: Vec<SyncedRelation>,
    /// The rows outside the part's area that followed the written objects' shared corners and edges (docs/adr/0531).
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub followed: u32,
    /// Those that would have been left invalid and kept their shape (`loose`).
    #[serde(default, skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "ts", ts(as = "Option<u32>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "count")))]
    pub left: u32,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}
