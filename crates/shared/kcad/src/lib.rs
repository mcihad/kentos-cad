//! The KentOS project file, `.kcad` v2 (docs/specs/kcad-v2.md, docs/adr/0025):
//! a versioned binary snapshot for saving and loading only, never a database,
//! index or tile store (docs/adr/0011).
//!
//! - **Container** (`header`): the `\x89KCAD\r\n\x1a\n` signature, versions,
//!   the payload's encoding and codec, lengths, required extensions and a
//!   SHA-256 trailer that detects corruption (it is not a signature).
//! - **KentOS CBOR profile 1** (`cbor`): definite lengths, shortest integers,
//!   binary64 floats only (NaN and infinities refused, −0 kept), text keys in
//!   RFC 8949 §4.2.1 order without duplicates, no tags; depth, length and size
//!   limits. Written here: no CBOR crate (owner's decision, 2026-09-26).
//! - **Document schemas 2 to 7** (`encode`, `decode`): the contract
//!   `DocumentSnapshotV2`, every object with its 16-byte persistent id
//!   (docs/adr/0014); the open document's slots never enter the file. Schema 3
//!   adds an object's own line weight, schema 4 vertex elevations, schema 5
//!   multi-part areas, schema 6 blocks, schema 7 a text's alignment, width
//!   factor and mask; a writer writes the oldest schema that holds what the
//!   drawing has.
//!
//! One implementation for every platform (CLAUDE.md §14): the desktop app and
//! the server call it natively, the browser through the formats WASM module
//! (`kentos-formats-wasm`). An independent reader in Python
//! (`tools/kcad/kcad.py`) and the byte-level fixtures (`fixtures/kcad/v2`)
//! hold it to the specification.
//!
//! Large drawings (docs/adr/0030): every read and write can be watched and
//! stopped (`watch`: the project first, then the objects, a few thousand at a
//! time); the browser hands its drawings to and from the codec as typed
//! columns (`columns`), not as JSON text.
//!
//! Rules: nothing a file holds may crash the reader or make it reserve more
//! memory than the file itself could fill; a drawing is written only if a
//! reader would take it back; the same drawing always gives the same bytes.
#![forbid(unsafe_code)]
// A panic traps the WASM module; user data must never cause one (docs/adr/0008).
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

mod blocks;
mod cbor;
pub mod columns;
mod decode;
mod encode;
mod error;
mod header;
mod opening;
mod space;
mod watch;

use kentos_contracts::DocumentSnapshotV2;
use serde::Serialize;

pub use cbor::{MAX_DEPTH, MAX_ITEMS, MAX_STRING};
pub use columns::Columns;
pub use error::{Code, KcadError};
pub use header::{
    CODEC_NONE, ENCODING_CBOR_PROFILE_1, FIXED_HEADER, HASH_SIZE, Header, MAGIC, MAJOR,
    MAX_PAYLOAD, MINOR,
};
pub use kentos_contracts as contracts;
pub use watch::{Quiet, Step, Watch};

/// The file extension (with the dot).
pub const EXTENSION: &str = ".kcad";
/// The media type: no registered KCAD type exists, so none is claimed (TODOS.md FILE-13).
pub const MIME: &str = "application/octet-stream";

/// Document schema 3 (docs/specs/kcad-v2.md §6.1): schema 2 and an object's
/// own line weight, `lineWeight` (docs/adr/0139). A writer writes it only
/// when an object has one: a drawing without one stays schema 2, byte for
/// byte, and a reader of schema 2 still opens it.
pub const SCHEMA_WITH_LINE_WEIGHTS: u32 = 3;

/// Document schema 4 (docs/specs/kcad-v2.md §6.1): schema 3 and vertex
/// elevations, `za` and `zb` of a line, `zs` of a polyline, a polygon and a
/// polygon's hole (docs/adr/0142). A writer writes it only when an object has
/// one: a drawing without one stays schema 3 or 2, byte for byte, and a reader
/// of those still opens it.
pub const SCHEMA_WITH_ELEVATIONS: u32 = 4;

/// Document schema 5 (docs/specs/kcad-v2.md §6.1): schema 4 and multi-part
/// areas, a polygon's `parts` (docs/adr/0143). A writer writes it only when an
/// area has them: a drawing without one stays schema 4, 3 or 2, byte for byte,
/// and a reader of those still opens it; one of those refuses a drawing that
/// has them rather than keep only each area's first part.
pub const SCHEMA_WITH_PARTS: u32 = 5;

/// Document schema 6 (docs/specs/kcad-v2.md §6.1): schema 5 and blocks, the
/// document's `blocks` and the `insert` kind (docs/adr/0144). A writer writes
/// it only when the drawing has a definition or an insert: any other stays 5
/// or older, byte for byte, and a reader of those still opens it; one of
/// those refuses a drawing that has them rather than drop them.
pub const SCHEMA_WITH_BLOCKS: u32 = 6;

/// Document schema 7 (docs/specs/kcad-v2.md §6.1): schema 6 and a text's
/// alignment, width factor and mask (`align`, `widthFactor`, `mask`), the
/// first two on an attribute definition too (docs/adr/0145). A writer writes
/// it only when a text or an attribute definition has one: any other drawing
/// stays 6 or older, byte for byte, and a reader of those still opens it; one
/// of those refuses a drawing that has them rather than move its texts.
pub const SCHEMA_WITH_TEXT_EXTRAS: u32 = 7;

/// Document schema 8 (docs/specs/kcad-v2.md §6.1): schema 7 and the `leader`
/// kind (docs/adr/0146). A writer writes it only when the drawing or a block
/// definition has a leader: any other drawing stays 7 or older, byte for
/// byte, and a reader of those still opens it; one of those refuses a drawing
/// that has one rather than drop it.
pub const SCHEMA_WITH_LEADERS: u32 = 8;

/// Document schema 9 (docs/specs/kcad-v2.md §6.1): schema 8 and the
/// dimension's new kinds (`ordinate`, `arcLength`, `jogged`, `azimuth`,
/// `slope`), its value's `mask` and a slope's `za`, `zb` (docs/adr/0147). A
/// writer writes it only when a dimension of the drawing or of a block
/// definition has one of them: any other drawing stays 8 or older, byte for
/// byte, and a reader of those still opens it; one of those refuses a drawing
/// that has them rather than draw them as another dimension.
pub const SCHEMA_WITH_DIMENSIONS: u32 = 9;

/// Document schema 10 (docs/specs/kcad-v2.md §6.1): schema 9 and a layer's
/// own snapping, a layer node's `snap` (docs/adr/0163 §4). A writer writes it
/// only when a layer has one: any other drawing stays 9 or older, byte for
/// byte, and a reader of those still opens it; one of those refuses a
/// drawing that has one rather than snap to a layer its author turned off.
pub const SCHEMA_WITH_LAYER_SNAP: u32 = 10;

/// Document schema 11 (docs/specs/kcad-v2.md §6.1): schema 10 and a local
/// project's drawing unit, the settings' `drawingUnit` (docs/adr/0165 §2). A
/// writer writes it only when the drawing names a unit: any other stays 10 or
/// older, byte for byte, and a reader of those still opens it; one of those
/// refuses a drawing that names one rather than read its millimetres as metres.
pub const SCHEMA_WITH_DRAWING_UNIT: u32 = 11;

/// Document schema 12 (docs/specs/kcad-v2.md §6.1): schema 11 and the
/// project's second coordinate system, the settings' `secondSrid`
/// (docs/adr/0167 §1). A writer writes it only when the project has one: any
/// other drawing stays 11 or older, byte for byte, and a reader of those still
/// opens it; one of those refuses a drawing that has one rather than drop it
/// unseen at the next save.
pub const SCHEMA_WITH_SECOND_SRID: u32 = 12;

/// Document schema 13 (docs/specs/kcad-v2.md §6.1): schema 12 and the
/// project's own coordinate systems and datum choices, the settings'
/// `customCrs`, `secondCustomCrs` and `datumTransforms` (docs/adr/0168). A
/// writer writes it only when the project has one of them: any other drawing
/// stays 12 or older, byte for byte; a reader of those refuses a drawing that
/// has one rather than open a project of its own system as one without any.
pub const SCHEMA_WITH_CUSTOM_CRS: u32 = 13;

/// Document schema 14 (docs/specs/kcad-v2.md §6.1): schema 13 and the
/// project's survey constants and tolerances, the settings' `survey`
/// (docs/adr/0169 §3). A writer writes it only when the project has them:
/// any other drawing stays 13 or older, byte for byte; a reader of those
/// refuses a drawing that has them rather than reduce its field book with
/// another refraction coefficient and no tolerances.
pub const SCHEMA_WITH_SURVEY: u32 = 14;

/// Document schema 15 (docs/specs/kcad-v2.md §6.1): schema 14 and the
/// survey settings' traverse tolerances, `twoWay`, `traverseAngle` and
/// `traverseCoord` (docs/adr/0169 §3). A writer writes it only when the
/// project has one of them: any other drawing stays 14 or older, byte for
/// byte; a reader of those refuses a drawing that has them rather than
/// check a traverse against none.
pub const SCHEMA_WITH_TRAVERSE_TOLERANCES: u32 = 15;

/// Document schema 16 (docs/specs/kcad-v2.md §6.1): schema 15 and the
/// survey settings' ground, `groundHeight` and `reduceToGrid`
/// (docs/adr/0171 §2, §4). A writer writes it only when the project has one
/// of them: any other drawing stays 15 or older, byte for byte; a reader of
/// those refuses a drawing that has them rather than give ground values at
/// another height or leave its lengths on the ground.
pub const SCHEMA_WITH_GROUND: u32 = 16;

/// Document schema 17 (docs/specs/kcad-v2.md §6.1): schema 16 and the
/// multi-part polyline and multi-point object, a polyline's and a point's
/// `parts` (docs/adr/0174). A writer writes it only when one of them has
/// them: any other drawing stays 16 or older, byte for byte; a reader of
/// those refuses a drawing that has them rather than keep its first part.
pub const SCHEMA_WITH_LINE_PARTS: u32 = 17;

/// Document schema 18 (docs/specs/kcad-v2.md §6.1): schema 17 and the text
/// that writes an object's label and follows it, a text's `labelOf` and
/// `labelScale` (docs/adr/0175 §4). A writer writes it only when a text has
/// them: any other drawing stays 17 or older, byte for byte; a reader of
/// those refuses a drawing that has them rather than keep its texts unlinked.
pub const SCHEMA_WITH_LINKED_TEXTS: u32 = 18;

/// Document schema 19 (docs/specs/kcad-v2.md §6.1): schema 18 and the
/// project's named layer states, the settings' `layerStates`
/// (docs/adr/0177 §4). A writer writes it only when the project has one: any
/// other drawing stays 18 or older, byte for byte; a reader of those refuses
/// a drawing that has them rather than drop them on its next save.
pub const SCHEMA_WITH_LAYER_STATES: u32 = 19;

/// Document schema 20 (docs/specs/kcad-v2.md §6.1): schema 19 and the
/// multi-line text's box width, line spacing and letter formats, a text's
/// `boxWidth`, `lineSpacing` and `runs` (docs/adr/0182 §1), in the drawing
/// and in block definitions. A writer writes it only when a text has one:
/// any other drawing stays 19 or older, byte for byte; a reader of those
/// refuses a drawing that has them rather than lose its texts' formats.
pub const SCHEMA_WITH_PARAGRAPHS: u32 = 20;

/// Document schema 21 (docs/specs/kcad-v2.md §6.1): schema 20 and the named
/// text and dimension styles, the settings' `textStyles` and
/// `dimensionStyles`, a text's face (`textStyle`, `font`, `bold`, `italic`,
/// `oblique`) and a dimension's look (`dimStyle`, `arrow`, `arrowSize`,
/// `extOffset`, `extBeyond`, `textGap`, `textPlace`, `decimals`, `unit`,
/// `prefix`, `suffix`, `font`; docs/adr/0183), in the drawing and in block
/// definitions. A writer writes it only when the project has a style or an
/// object one of these fields: any other drawing stays 20 or older, byte for
/// byte; a reader of those refuses a drawing that has them rather than lose
/// its styles.
pub const SCHEMA_WITH_STYLES: u32 = 21;

/// Document schema 22 (docs/specs/kcad-v2.md §6.1): schema 21 and the
/// `table` kind (docs/adr/0184 §1), in the drawing only. A writer writes it
/// only when the drawing has a table: any other drawing stays 21 or older,
/// byte for byte; a reader of those refuses a table rather than lose it.
pub const SCHEMA_WITH_TABLES: u32 = 22;

/// Document schema 23 (docs/specs/kcad-v2.md §6.1): schema 22 and the
/// hatch's patterns, gradients and ties (docs/adr/0186): `pattern`'s `type`
/// `pattern` and `gradient`, its `name`, `scale`, `lines` and `gradient`, in
/// the drawing and in block definitions; a hatch's `assoc`, in the drawing
/// only. A writer writes it only when a hatch has one of these: any other
/// drawing stays 22 or older, byte for byte; a reader of those refuses them
/// rather than draw another pattern.
pub const SCHEMA_WITH_HATCH_PATTERNS: u32 = 23;

/// Document schema 24 (docs/specs/kcad-v2.md §6.1): schema 23 and the
/// `image` kind (docs/adr/0192 §1), in the drawing only. A writer writes it
/// only when the drawing has a picture: any other drawing stays 23 or older,
/// byte for byte; a reader of those refuses a picture rather than lose it.
pub const SCHEMA_WITH_IMAGES: u32 = 24;

/// Document schema 25 (docs/specs/kcad-v2.md §6.1): schema 24 and the text
/// along a curve, a text's `path` (docs/adr/0196 §1), in the drawing and in
/// block definitions. A writer writes it only when a text has one: any other
/// drawing stays 24 or older, byte for byte; a reader of those refuses a
/// curve rather than set the text straight.
pub const SCHEMA_WITH_TEXT_PATHS: u32 = 25;

/// Document schema 26 (docs/specs/kcad-v2.md §6.1): schema 25 and a layer's
/// fields, a layer node's `fields` (docs/adr/0199 §1). A writer writes it
/// only when a layer has them: any other drawing stays 25 or older, byte for
/// byte; a reader of those refuses a drawing that has them rather than drop
/// its layers' schemas on its next save.
pub const SCHEMA_WITH_LAYER_FIELDS: u32 = 26;

/// Document schema 27 (docs/specs/kcad-v2.md §6.1): schema 26 and the
/// project's topology rules, tolerance and exceptions, the settings'
/// `topology` (docs/adr/0202 §7). A writer writes it only when a project has
/// them: any other drawing stays 26 or older, byte for byte; a reader of
/// those refuses a drawing that has them rather than drop the rules and the
/// exceptions on its next save.
pub const SCHEMA_WITH_TOPOLOGY: u32 = 27;

/// Document schema 28 (docs/specs/kcad-v2.md §6.1): schema 27 and the
/// survey settings' a priori standard deviations of a network adjustment,
/// `sigmaDirection`, `sigmaDistance`, `sigmaPpm`, `sigmaCentering`,
/// `sigmaZenith` and `sigmaLevelling` (docs/adr/0203 §1). A writer writes
/// it only when a project names one: any other drawing stays 27 or older,
/// byte for byte; a reader of those refuses a drawing that has them rather
/// than drop them on its next save.
pub const SCHEMA_WITH_SURVEY_SIGMAS: u32 = 28;

/// Document schema 29 (docs/specs/kcad-v2.md §6.1): schema 28 and the
/// `raster` kind (docs/adr/0204 §2), in the drawing only. A writer writes it
/// only when the drawing has a raster: any other drawing stays 28 or older,
/// byte for byte; a reader of those refuses a raster rather than lose it.
pub const SCHEMA_WITH_RASTERS: u32 = 29;

/// Document schema 30 (docs/specs/kcad-v2.md §6.1): schema 29 and the
/// project's annotation heights, the settings' `annotation`; a dimension's
/// and a dimension style's line fields (`dimLineColor`, `lineWeight`,
/// `lineType`, `extColor`, `extWeight`, `extLineType`, `textColor`); a
/// leader's `arrowSize` and the arrowheads AutoCAD has beside the first three
/// (docs/adr/0205). A writer writes it only when a drawing has one of these:
/// any other drawing stays 29 or older, byte for byte; a reader of those
/// refuses them rather than draw another look.
pub const SCHEMA_WITH_ANNOTATION: u32 = 30;

/// Document schema 31 (docs/specs/kcad-v2.md §6.1): schema 30 and the
/// `pointcloud` kind (docs/adr/0207 §3), in the drawing only, and a raster's
/// address, `url` (§1). A writer writes it only when the drawing has a point
/// cloud or a raster read from an address: any other drawing stays 30 or
/// older, byte for byte; a reader of those refuses them rather than lose them.
pub const SCHEMA_WITH_POINT_CLOUDS: u32 = 31;

/// Document schema 32 (docs/specs/kcad-v2.md §6.4, §6.5): schema 31 and
/// map services (docs/adr/0208 §2): a layer node's `service` and `feed`, and
/// the settings' `connections` (without their secrets). A writer writes it
/// only when the drawing has one of these: any other drawing stays 31 or
/// older, byte for byte; a reader of those refuses them rather than lose them.
pub const SCHEMA_WITH_SERVICES: u32 = 32;

/// Document schema 33 (docs/specs/kcad-v2.md §6.1, §6.4.8): schema 32 and the
/// settings' `networks` (docs/adr/0209 §2). A writer writes it only when the
/// project has a network: any other drawing stays 32 or older, byte for
/// byte; a reader of those refuses them rather than lose them.
pub const SCHEMA_WITH_NETWORKS: u32 = 33;

/// Document schema 34 (docs/specs/kcad-v2.md §6.5): schema 33 and a layer
/// node's `time`, a group's `scenario` and a scenario layer's `replaces`
/// (docs/adr/0210 §2). A writer writes it only when a node has one of these:
/// any other drawing stays 33 or older, byte for byte; a reader of those
/// refuses them rather than lose them.
pub const SCHEMA_WITH_TEMPORAL: u32 = 34;

/// Document schema 35 (docs/specs/kcad-v2.md §6.5): schema 34 and a layer
/// node's `filter` (docs/adr/0211 §2). A writer writes it only when a layer
/// has one: any other drawing stays 34 or older, byte for byte; a reader of
/// those refuses it rather than lose it.
pub const SCHEMA_WITH_FILTERS: u32 = 35;

/// Document schema 36 (docs/specs/kcad-v2.md §6.5, §6.6): schema 35 and the
/// label engine (docs/adr/0212 §2): a label style's engine fields, a layer
/// style's `labels` and an object's `labelPins`. A writer writes it only when
/// the drawing has one of these: any other drawing stays 35 or older, byte for
/// byte; a reader of those refuses them rather than lose them.
pub const SCHEMA_WITH_LABELS: u32 = 36;

/// Document schema 37 (docs/specs/kcad-v2.md §6.4.9): schema 36 and the
/// settings' `variables`, the project's `@` values (docs/adr/0214 §2.3). A
/// writer writes it only when the project has one: any other drawing stays
/// 36 or older, byte for byte; a reader of those refuses it rather than lose it.
pub const SCHEMA_WITH_VARIABLES: u32 = 37;

/// Document schema 38 (docs/specs/kcad-v2.md §6.6): schema 37 and
/// multidimensional rasters (docs/adr/0243 §6): a raster's `dataset` (a NetCDF
/// variable and its slice) and its look's `edges` (a mesh's lines). A writer
/// writes it only when a raster has one of these: any other drawing stays 37
/// or older, byte for byte; a reader of those refuses them rather than lose them.
pub const SCHEMA_WITH_MULTIDIM: u32 = 38;

/// Document schema 39 (docs/specs/kcad-v2.md §6.6): schema 38 and vertex
/// measures (M), a line's `ma` and `mb`, a polyline's and its parts' `ms`
/// (docs/adr/0224 §1). A writer writes it only when an object of the
/// drawing or of a block definition has one: any other drawing stays 38 or
/// older, byte for byte; a reader of those refuses them rather than lose
/// them. An area has none.
pub const SCHEMA_WITH_MEASURES: u32 = 39;

/// Document schema 40 (docs/specs/kcad-v2.md §6.5, §6.6): schema 39 and
/// the objects' `attachments`, a layer's `actions` and `card` (docs/adr/0225).
/// A writer writes it only when an object has an attachment or a layer an
/// action or a card: any other drawing stays 39 or older, byte for byte; a
/// reader of those refuses them rather than lose them. A block definition's
/// objects have no attachments.
pub const SCHEMA_WITH_ATTACHMENTS: u32 = 40;

/// Schema 41 (docs/adr/0226 §3, §4): a layer's `compose` (its opacity, its
/// blending mode and the mask its labels and markers cut into other layers),
/// written only when a layer has one; older readers refuse such a file
/// instead of drawing it as it was not meant.
pub const SCHEMA_WITH_COMPOSE: u32 = 41;

/// Schema 42 (docs/specs/kcad-v2.md §6.4.10, docs/adr/0229 §2): the
/// settings' `addressLocators` (a project's address data: its doors', streets'
/// and areas' layers and fields), written only when the project has one; older
/// readers refuse such a file instead of losing them.
pub const SCHEMA_WITH_ADDRESSES: u32 = 42;

/// Schema 43 (docs/specs/kcad-v2.md §6.6, docs/adr/0249 §4): a dimension's
/// `refs` (the places on objects its points follow), written only when a
/// dimension of the drawing has one; older readers refuse such a file instead
/// of losing them. A block definition's dimensions have none.
pub const SCHEMA_WITH_DIMENSION_REFS: u32 = 43;

/// Schema 44 (docs/specs/kcad-v2.md §6.12, docs/adr/0251): the project's
/// sheet book and its pictures' bytes (`sheets`), written only when the
/// project has a book; older readers refuse such a file instead of losing it.
pub const SCHEMA_WITH_SHEETS: u32 = 44;

/// Schema 45 (docs/specs/kcad-v2.md §6.13, docs/adr/0253 §6): the requests
/// a drawing executed (`requests`), written only when it executed any; older
/// readers refuse such a file instead of losing them.
pub const SCHEMA_WITH_REQUESTS: u32 = 45;

/// Schema 46 (docs/specs/kcad-v2.md §6.6, docs/adr/0258): a polyline's
/// `closed` (a closed line, not an area) and `widths` (each edge's start and
/// end width in the drawing's length), written only when a polyline of the
/// drawing or of a block definition has one; older readers refuse such a file
/// instead of opening the line open or narrow.
pub const SCHEMA_WITH_POLYLINE_EXTRAS: u32 = 46;
/// Schema 47 (docs/adr/0259): the general NURBS curve (`nurbs`: degree,
/// control points, knots, weights, and the fit points, tangents and
/// tolerance kept with them), written only when the drawing or a block
/// definition has one; older readers refuse such a file instead of losing
/// the curve.
pub const SCHEMA_WITH_NURBS: u32 = 47;
/// Schema 48 (docs/adr/0260): an object's own line type, line type scale,
/// transparency and drawing order, a block's object's parts that follow its
/// layer (`byLayer`), and a layer's `plot`, written only when the drawing has
/// one; older readers refuse such a file instead of drawing it otherwise.
pub const SCHEMA_WITH_LOOK: u32 = 48;
/// Schema 49 (docs/adr/0261): the settings' `ucs` (the drawing's user
/// coordinate system) and `namedUcs`, written only when the current UCS is
/// not the world's or a named one is kept; older readers refuse such a file
/// instead of opening it in the world's axes.
pub const SCHEMA_WITH_UCS: u32 = 49;
/// Schema 50 (docs/adr/0262): the drawing units `in` and `ft` and the
/// settings' `lengthStyle`, written only when the drawing has one; older
/// readers refuse an inch drawing instead of reading it in metres.
pub const SCHEMA_WITH_IMPERIAL: u32 = 50;
/// Schema 51 (docs/adr/0263): a dimension reference's `along` place (a
/// point along an edge or a curve) and its block `path` (a piece inside an
/// insert), written only when a reference has one; older readers refuse the
/// file instead of losing where the point follows.
pub const SCHEMA_WITH_SUBREF_EXTRAS: u32 = 51;
/// Schema 52 (docs/adr/0285): the settings' `namedViews`, the project's
/// named views (a middle, a width and a height, a twist), written only when
/// the project keeps one; older readers refuse the file instead of losing them.
pub const SCHEMA_WITH_VIEWS: u32 = 52;
/// Schema 53 (docs/specs/kcad-v2.md §6.14, docs/adr/0300): the document's
/// associative arrays (`arrays`), written only when the drawing has one;
/// older readers refuse the file instead of losing the arrays.
pub const SCHEMA_WITH_ARRAYS: u32 = 53;
/// Schema 54 (docs/adr/0310): an area's mask (`wipeout`: `{ frame }`), written only when an object of the drawing or
/// of a block definition has one; older readers refuse the file instead of drawing the mask as a plain area.
pub const SCHEMA_WITH_WIPEOUTS: u32 = 54;
/// Schema 55 (docs/adr/0311 §1, §3): a layer's `frozen` (only `true`), and a layer state node's `frozen` and `plot`,
/// written only when a node is frozen or a state keeps them; older readers refuse the file instead of drawing a frozen
/// layer or applying a state without its freeze.
pub const SCHEMA_WITH_FROZEN: u32 = 55;
/// Schema 56 (docs/adr/0313 §1): the settings' line types with patterns (`lineTypes`) and line type scale
/// (`lineTypeScale`), a layer style's and an object's `pattern`, written only when the drawing has one; older readers
/// refuse the file instead of drawing a pattern as its nearest built-in type.
pub const SCHEMA_WITH_PATTERNS: u32 = 56;
/// Schema 57 (docs/adr/0315 §3, §4): an insert's own y factor (`yScale`, only when it is not `scale`) and a block
/// definition's `units`, written only when the drawing has one; older readers refuse the file instead of drawing a
/// stretched block at one scale or losing its unit.
pub const SCHEMA_WITH_BLOCK_EXTRAS: u32 = 57;
/// Schema 58 (docs/adr/0317 §1, §4): an attribute definition's options (`invisible`, `constant`, `verify`, `preset`,
/// `movable`, `multiline`, `boxWidth`, `field`), an insert's `attributePlaces` and a block definition's `card`,
/// written only when the drawing has one; older readers refuse the file instead of showing a hidden attribute or
/// losing a moved one.
pub const SCHEMA_WITH_BLOCK_CARDS: u32 = 58;
/// Schema 59 (docs/adr/0318): a block definition's `dynamic` part (its parameters and actions) and an insert's
/// `parameters`, written only when the drawing has one; older readers refuse the file instead of drawing every
/// insert of a dynamic block at its defaults.
pub const SCHEMA_WITH_DYNAMIC_BLOCKS: u32 = 59;
/// Schema 60 (docs/adr/0320 §1): a dimension's references' `rule` (`curve`, `edges`): how its points no place names
/// follow, written only when a dimension has one; older readers refuse the file instead of leaving a radius's centre
/// or an angle's vertex behind.
pub const SCHEMA_WITH_DIMENSION_RULES: u32 = 60;
/// Schema 61 (docs/adr/0321): a dimension's and a dimension style's `options` (fit, increment, zeros, feet and inches,
/// alternate unit, tolerance, break size) and a dimension's `textShift` and `breaks`, written only when one has them;
/// older readers refuse the file instead of writing the value without its tolerance or drawing a broken line whole.
pub const SCHEMA_WITH_DIMENSION_OPTIONS: u32 = 61;
/// Schema 62 (docs/adr/0322): a dimension's kinds `centerMark`, `centerLine` and `elevation` and its references' rule
/// `pair`, written only when a dimension has one; older readers refuse the file instead of losing the marks.
pub const SCHEMA_WITH_DIMENSION_MARKS: u32 = 62;
/// Schema 63 (docs/adr/0322 §5): a text's and a dimension's `overrides`, the fields set apart from their style by
/// hand, written only when one has them; older readers refuse the file instead of letting a style change take them.
pub const SCHEMA_WITH_OVERRIDES: u32 = 63;
/// Schema 64 (docs/adr/0323): a text's, a dimension's and a leader's `annotative` (its scales and what each sets
/// apart), written only when one has them; older readers refuse the file instead of drawing every scale at one height.
pub const SCHEMA_WITH_ANNOTATIVE: u32 = 64;
/// Schema 65 (docs/adr/0324): a multi-line text's `layout` (indents, tab stops, columns) and a run's `stack` script
/// (a stacked fraction), written only when a text has one; older readers refuse the file instead of losing them.
pub const SCHEMA_WITH_PARAGRAPH_LAYOUT: u32 = 65;
/// Schema 66 (docs/adr/0325): a text's `fields` (ranges of its letters bound to a source), written only when a text
/// has one; older readers refuse the file instead of losing the bindings.
pub const SCHEMA_WITH_TEXT_FIELDS: u32 = 66;
/// Schema 67 (docs/adr/0326): a leader's `arms`, `tip`, `blocks` and `leaderStyle`, and the settings'
/// `leaderStyles`, written only when one has them; older readers refuse the file instead of losing them.
pub const SCHEMA_WITH_MULTILEADER: u32 = 67;
/// Schema 68 (docs/adr/0327): a table's `styles`, `formulas`, `fields`, a linked file's `link` and a block
/// schedule's `schedule`.
pub const SCHEMA_WITH_TABLE_CELLS: u32 = 68;
/// Schema 69 (docs/adr/0328): a hatch's exact `boundary` (loops of lines, arcs, ellipses' arcs and NURBS curves)
/// and a tie's `bounds` (the line work a face follows, `outer` then absent), written only when a hatch has one;
/// older readers refuse the file instead of losing them.
pub const SCHEMA_WITH_HATCH_BOUNDARIES: u32 = 69;
/// Schema 70 (docs/adr/0329 §3): a face's tie's `gap` (the gaps its line work was closed across), written only when a
/// tie has one; older readers refuse the file instead of losing the hatch's way of following.
pub const SCHEMA_WITH_HATCH_GAPS: u32 = 70;
/// Schema 71 (docs/adr/0331): the settings' `hatchPatterns` (a project's own patterns), a pattern's `origin` and a
/// hatch's `annotative`, written only when the drawing has one; older readers refuse the file instead of losing
/// them or drawing an annotative hatch's pattern at one scale.
pub const SCHEMA_WITH_HATCH_EXTRAS: u32 = 71;
/// Schema 72 (docs/adr/0401 §2, §3): the settings' `forms` (the project's forms, a contract value read
/// whole and checked by `forms_problem`) and a layer field's stable `id`, written only when the drawing
/// has one; older readers refuse the file instead of losing a form or a binding.
pub const SCHEMA_WITH_FORMS: u32 = 72;
/// Schema 73 (docs/adr/0334): the document's `constraints` (the geometric and dimensional constraints and the
/// parameter table, a contract value read whole and checked by `constraints_problem`), written only when the drawing
/// has one; older readers refuse the file instead of losing them.
pub const SCHEMA_WITH_CONSTRAINTS: u32 = 73;
/// Schema 74 (docs/adr/0503): the settings' `sources` (layers bound to a table of a file, with the file's revision),
/// written only when the drawing has one; older readers refuse the file instead of losing the bindings.
pub const SCHEMA_WITH_SOURCES: u32 = 74;
/// Schema 75 (docs/adr/0339 §4): the wall (`wall`: its path, width, justification, offset, style, components, ends,
/// fill, height and architectural information) and the settings' `wallStyles` and `arch` (levels, the walls' detail),
/// written only when the drawing has one; older readers refuse the file instead of losing them.
pub const SCHEMA_WITH_WALLS: u32 = 75;
/// Schema 76 (docs/adr/0340 §1): a wall's `joins` (its start's and end's join when not by the rule) and `cleanup`
/// (written only when false), and a wall component's `priority` (on a wall and in a wall style), written only when
/// the drawing has one; older readers refuse the file instead of losing them.
pub const SCHEMA_WITH_WALL_JOINS: u32 = 76;
/// Schema 77 (docs/adr/0341 §1): the opening (`opening`: its wall, its place along it, its width, its jambs and the
/// rest of its fields), written only when the drawing has one; older readers refuse the file instead of losing them.
pub const SCHEMA_WITH_OPENINGS: u32 = 77;
/// Schema 78 (docs/adr/0342 §1): the space (`space`: its boundary's parts and the rest of its fields) and the
/// settings' `arch.spaceMethod`, written only when the drawing has one; older readers refuse the file instead of
/// losing them.
pub const SCHEMA_WITH_SPACES: u32 = 78;
/// Schema 79 (docs/adr/0343 §1): the grid, the column, the stair and the ramp (each its fields as one map under its
/// kind's name), written only when the drawing has one.
pub const SCHEMA_WITH_ARCH_ELEMENTS: u32 = 79;
/// Schema 80 (docs/adr/0344 §1, §3): the tag (its fields as one map under `tag`) and a table's schedule of openings,
/// spaces, columns or walls (as a block schedule, its source's JSON under `schedule`), written only when the drawing
/// has one.
pub const SCHEMA_WITH_TAGS: u32 = 80;
/// Schema 81 (docs/adr/0345 §1, §2): the view mark (its fields as one map under `viewmark`) and a named view's `id`,
/// written only when the drawing has one.
pub const SCHEMA_WITH_VIEWMARKS: u32 = 81;
/// Schema 82 (docs/adr/0346 §2): the architectural settings' `phaseView`, written only when the project sets one.
pub const SCHEMA_WITH_PHASES: u32 = 82;
/// Schema 83 (docs/adr/0348 §1): a block definition's external reference source (`xref`, its JSON), written only when
/// the drawing has one.
pub const SCHEMA_WITH_XREFS: u32 = 83;
/// Schema 84 (docs/adr/0349 §1): the settings' `xrefPaths`, the folders external references are looked for in, written
/// only when the project has one.
pub const SCHEMA_WITH_XREF_PATHS: u32 = 84;
/// Schema 85 (docs/adr/0351 §1, §2): an insert's `clipping` (its JSON) and an external reference's `page` and
/// `underlay` (a PDF underlay), written only when the drawing has one.
pub const SCHEMA_WITH_CLIPPING: u32 = 85;
/// Schema 86 (docs/adr/0519 §1): the settings' `heights` (what the elevations are: their kind, vertical system and
/// geoid model) and `coordinateEpoch`, written only when the project names one.
pub const SCHEMA_WITH_HEIGHTS: u32 = 86;
/// Schema 87 (docs/adr/0521): a network's `turns` (U-turns, turn delays, turn layers) and `levels` (the level fields)
/// and a speed cost's `periods` (its time profile), written only when a network has one.
pub const SCHEMA_WITH_NETWORK_TURNS: u32 = 87;
/// Schema 88 (docs/adr/0518 §1): a raster's `georef`, the record of how it was georeferenced (its JSON), written only
/// when a raster has one.
pub const SCHEMA_WITH_GEOREF: u32 = 88;
/// Schema 89 (docs/adr/0527): the settings' `joinViews`, the project's join views (their JSON), written only when
/// the project has one.
pub const SCHEMA_WITH_JOIN_VIEWS: u32 = 89;
/// Schema 90 (docs/adr/0537): a layer time's `recordedStart` and `recordedEnd`, the attributes of its objects'
/// system time (when each version was recorded and superseded), written only when a layer has them.
pub const SCHEMA_WITH_RECORDED_TIME: u32 = 90;
/// Schema 91 (docs/adr/0538 §1): the settings' `sourceBases`, each bound layer's objects' hashes when the layer and
/// its table last matched (their JSON, the rows base64), written only when a project has one.
pub const SCHEMA_WITH_SOURCE_BASES: u32 = 91;
/// Schema 92 (docs/adr/0553 §1): a source base's `fields`, what each object held besides its hashes (the hashes of its
/// kind and geometry on each side, of each attribute and of its vertices), written only when a base has them.
pub const SCHEMA_WITH_SOURCE_FIELD_BASES: u32 = 92;
/// Schema 93 (docs/adr/0557): a network's `terminals` (its devices' terminals and the paths through them, the edge fields
/// naming the terminals) and `subnetworks` (the attribute the subnetworks' names are written to), written only when a
/// network has one.
pub const SCHEMA_WITH_NETWORK_TERMINALS: u32 = 93;
/// Schema 94 (docs/adr/0562): a network device's `by` and `configs` (terminals and paths chosen by an attribute of its
/// points) and the subnetworks' `keepCurrent`, written only when a network has one.
pub const SCHEMA_WITH_NETWORK_CONFIGS: u32 = 94;
/// Schema 95 (docs/adr/0567): a network's vehicle `profiles` (limits, forbidding values, top speed, both ways) and
/// dated `closures` (the edge fields of a closure's first and last day), written only when a network has one.
pub const SCHEMA_WITH_NETWORK_PROFILES: u32 = 95;
/// Schema 96 (docs/adr/0568): a network closure's `days` and `hours` (the edge fields of the weekdays and the hours
/// an edge is closed), written only when a closure names one.
pub const SCHEMA_WITH_TIMED_CLOSURES: u32 = 96;
/// Schema 97 (docs/adr/0569): the settings' `linearEvents`, the project's links of event layers to their routes (their
/// JSON), written only when the project has one.
pub const SCHEMA_WITH_LINEAR_EVENTS: u32 = 97;
/// Schema 98 (docs/adr/0583): a layer's `sourceCrs` (the system its objects came from) and the settings' `mapSrid`
/// (the map's system), written only when a layer has one or the project has one.
pub const SCHEMA_WITH_SOURCE_CRS: u32 = 98;
/// Schema 99 (docs/adr/0589): a topology exception's `digest`, its objects' geometry when it was marked, written only
/// when an exception has one.
pub const SCHEMA_WITH_EXCEPTION_DIGESTS: u32 = 99;
/// Schema 100 (docs/adr/0594 §1): a georeference record's `crs`, the targets' system as a definition when it is no
/// EPSG code, written only when a record has one.
pub const SCHEMA_WITH_GEOREF_CRS: u32 = 100;
/// Schema 101 (docs/adr/0598 §1): the settings' `lineage`, what the split and merged objects came from (their JSON),
/// written only when the project has a record.
pub const SCHEMA_WITH_LINEAGE: u32 = 101;
/// The largest drawing order a file holds: 2³¹ − 1; the smallest is −2³¹.
pub const MAX_ORDER: u64 = i32::MAX as u64;

/// The document schemas this codec reads, oldest first.
pub const SCHEMAS: [u32; 100] = [
    kentos_contracts::DOCUMENT_VERSION_2,
    SCHEMA_WITH_LINE_WEIGHTS,
    SCHEMA_WITH_ELEVATIONS,
    SCHEMA_WITH_PARTS,
    SCHEMA_WITH_BLOCKS,
    SCHEMA_WITH_TEXT_EXTRAS,
    SCHEMA_WITH_LEADERS,
    SCHEMA_WITH_DIMENSIONS,
    SCHEMA_WITH_LAYER_SNAP,
    SCHEMA_WITH_DRAWING_UNIT,
    SCHEMA_WITH_SECOND_SRID,
    SCHEMA_WITH_CUSTOM_CRS,
    SCHEMA_WITH_SURVEY,
    SCHEMA_WITH_TRAVERSE_TOLERANCES,
    SCHEMA_WITH_GROUND,
    SCHEMA_WITH_LINE_PARTS,
    SCHEMA_WITH_LINKED_TEXTS,
    SCHEMA_WITH_LAYER_STATES,
    SCHEMA_WITH_PARAGRAPHS,
    SCHEMA_WITH_STYLES,
    SCHEMA_WITH_TABLES,
    SCHEMA_WITH_HATCH_PATTERNS,
    SCHEMA_WITH_IMAGES,
    SCHEMA_WITH_TEXT_PATHS,
    SCHEMA_WITH_LAYER_FIELDS,
    SCHEMA_WITH_TOPOLOGY,
    SCHEMA_WITH_SURVEY_SIGMAS,
    SCHEMA_WITH_RASTERS,
    SCHEMA_WITH_ANNOTATION,
    SCHEMA_WITH_POINT_CLOUDS,
    SCHEMA_WITH_SERVICES,
    SCHEMA_WITH_NETWORKS,
    SCHEMA_WITH_TEMPORAL,
    SCHEMA_WITH_FILTERS,
    SCHEMA_WITH_LABELS,
    SCHEMA_WITH_VARIABLES,
    SCHEMA_WITH_MULTIDIM,
    SCHEMA_WITH_MEASURES,
    SCHEMA_WITH_ATTACHMENTS,
    SCHEMA_WITH_COMPOSE,
    SCHEMA_WITH_ADDRESSES,
    SCHEMA_WITH_DIMENSION_REFS,
    SCHEMA_WITH_SHEETS,
    SCHEMA_WITH_REQUESTS,
    SCHEMA_WITH_POLYLINE_EXTRAS,
    SCHEMA_WITH_NURBS,
    SCHEMA_WITH_LOOK,
    SCHEMA_WITH_UCS,
    SCHEMA_WITH_IMPERIAL,
    SCHEMA_WITH_SUBREF_EXTRAS,
    SCHEMA_WITH_VIEWS,
    SCHEMA_WITH_ARRAYS,
    SCHEMA_WITH_WIPEOUTS,
    SCHEMA_WITH_FROZEN,
    SCHEMA_WITH_PATTERNS,
    SCHEMA_WITH_BLOCK_EXTRAS,
    SCHEMA_WITH_BLOCK_CARDS,
    SCHEMA_WITH_DYNAMIC_BLOCKS,
    SCHEMA_WITH_DIMENSION_RULES,
    SCHEMA_WITH_DIMENSION_OPTIONS,
    SCHEMA_WITH_DIMENSION_MARKS,
    SCHEMA_WITH_OVERRIDES,
    SCHEMA_WITH_ANNOTATIVE,
    SCHEMA_WITH_PARAGRAPH_LAYOUT,
    SCHEMA_WITH_TEXT_FIELDS,
    SCHEMA_WITH_MULTILEADER,
    SCHEMA_WITH_TABLE_CELLS,
    SCHEMA_WITH_HATCH_BOUNDARIES,
    SCHEMA_WITH_HATCH_GAPS,
    SCHEMA_WITH_HATCH_EXTRAS,
    SCHEMA_WITH_FORMS,
    SCHEMA_WITH_CONSTRAINTS,
    SCHEMA_WITH_SOURCES,
    SCHEMA_WITH_WALLS,
    SCHEMA_WITH_WALL_JOINS,
    SCHEMA_WITH_OPENINGS,
    SCHEMA_WITH_SPACES,
    SCHEMA_WITH_ARCH_ELEMENTS,
    SCHEMA_WITH_TAGS,
    SCHEMA_WITH_VIEWMARKS,
    SCHEMA_WITH_PHASES,
    SCHEMA_WITH_XREFS,
    SCHEMA_WITH_XREF_PATHS,
    SCHEMA_WITH_CLIPPING,
    SCHEMA_WITH_HEIGHTS,
    SCHEMA_WITH_NETWORK_TURNS,
    SCHEMA_WITH_GEOREF,
    SCHEMA_WITH_JOIN_VIEWS,
    SCHEMA_WITH_RECORDED_TIME,
    SCHEMA_WITH_SOURCE_BASES,
    SCHEMA_WITH_SOURCE_FIELD_BASES,
    SCHEMA_WITH_NETWORK_TERMINALS,
    SCHEMA_WITH_NETWORK_CONFIGS,
    SCHEMA_WITH_NETWORK_PROFILES,
    SCHEMA_WITH_TIMED_CLOSURES,
    SCHEMA_WITH_LINEAR_EVENTS,
    SCHEMA_WITH_SOURCE_CRS,
    SCHEMA_WITH_EXCEPTION_DIGESTS,
    SCHEMA_WITH_GEOREF_CRS,
    SCHEMA_WITH_LINEAGE,
];

/// The file a drawing is saved as.
pub fn encode(doc: &DocumentSnapshotV2) -> Result<Vec<u8>, KcadError> {
    encode_watched(doc, &mut Quiet)
}

/// `encode`, the objects reported to `watch` as they are written; it may stop the writing.
pub fn encode_watched(
    doc: &DocumentSnapshotV2,
    watch: &mut dyn Watch,
) -> Result<Vec<u8>, KcadError> {
    header::write(&encode::payload(doc, watch)?)
}

/// The file a drawing is saved as, read back before it is handed out: the
/// bytes must decode to the same drawing, every float64 bit for bit (only
/// the objects' slots, which the file does not keep, may differ). A save
/// writes only verified bytes, so a file is never replaced by one that does
/// not open again (TODOS.md FILE-16, FILE-21).
pub fn encode_verified(doc: &DocumentSnapshotV2) -> Result<Vec<u8>, KcadError> {
    encode_verified_watched(doc, &mut Quiet)
}

/// `encode_verified` with its stages reported to `watch` (writing, then
/// `Verifying` and the reading back), which may stop it.
pub fn encode_verified_watched(
    doc: &DocumentSnapshotV2,
    watch: &mut dyn Watch,
) -> Result<Vec<u8>, KcadError> {
    let bytes = encode_watched(doc, watch)?;
    watch::report(watch, Step::Verifying)?;
    let back = decode_watched(&bytes, watch).map_err(not_read_back)?;
    if let Some(what) = difference(doc, &back) {
        return Err(unverified(&what));
    }
    Ok(bytes)
}

/// Bytes that did not read back: a verification failure, unless the watcher stopped the reading.
fn not_read_back(e: KcadError) -> KcadError {
    if e.code == Code::Cancelled {
        e
    } else {
        unverified(&e.message)
    }
}

fn unverified(what: &str) -> KcadError {
    KcadError::new(
        Code::VerifyFailed,
        format!(
            "Çizim kaydedilmedi: yazılacak KCAD baytları geri okununca çizimle aynı çıkmadı ({what}). Bu bir yazılım hatasıdır; eski dosyaya dokunulmadı. Çizimi başka bir yere kaydetmeyi deneyin ve durumu bildirin."
        ),
    )
}

/// The drawing in a file; the whole file is checked before anything is returned.
pub fn decode(data: &[u8]) -> Result<DocumentSnapshotV2, KcadError> {
    decode_watched(data, &mut Quiet)
}

/// `decode` with its stages reported to `watch` (the integrity check, the
/// project, the objects), which may stop it: then nothing is returned, so
/// half a file never becomes a drawing (TODOS.md FILE-20).
pub fn decode_watched(data: &[u8], watch: &mut dyn Watch) -> Result<DocumentSnapshotV2, KcadError> {
    let (_, payload) = header::read_watched(data, watch)?;
    decode::payload(payload, watch)
}

/// The header and the drawing of a file (`kcad inspect`).
pub fn read(data: &[u8]) -> Result<(Header, DocumentSnapshotV2), KcadError> {
    let (head, payload) = header::read(data)?;
    Ok((head, decode::payload(payload, &mut Quiet)?))
}

// ── The browser's typed boundary (docs/adr/0030) ────────────────────────

/// A drawing taken apart for the browser's page: the contract's JSON of
/// everything but the objects (`entities` and `uids` empty), and the objects
/// as columns. Each object is dropped once packed, so a large drawing is in
/// memory about once.
pub fn split(mut doc: DocumentSnapshotV2) -> Result<(String, Columns), KcadError> {
    let entities = std::mem::take(&mut doc.entities);
    let uids = std::mem::take(&mut doc.uids);
    let head = head_json(&doc)?;
    Ok((head, columns::pack(entities, uids)))
}

fn head_json(doc: &DocumentSnapshotV2) -> Result<String, KcadError> {
    serde_json::to_string(doc).map_err(|e| {
        KcadError::new(
            Code::BadColumns,
            format!(
                "Çizimin proje bilgisi yazılamadı ({e}). Bu bir yazılım hatasıdır; durumu bildirin."
            ),
        )
    })
}

/// The drawing the browser's page packed, as a `.kcad` v2 file's bytes,
/// verified before they are handed out: `head` is the contract's JSON of
/// everything but the objects, `cols` the objects. The bytes are read back
/// and the objects compared with the columns as they were sent, every field
/// and every float bit for bit (`columns::differs`). Returns the bytes and
/// the head as the file holds it (JSON), for the page's side to compare with
/// the head it sent: a field the contract does not read would show there.
pub fn encode_columns(
    head: &str,
    cols: &Columns,
    watch: &mut dyn Watch,
) -> Result<(Vec<u8>, String), KcadError> {
    let mut doc: DocumentSnapshotV2 = serde_json::from_str(head).map_err(|e| {
        KcadError::new(
            Code::BadColumns,
            format!(
                "Kaydedilecek çizimin proje bilgisi okunamadı ({e}); uygulama ile dosya biçimi paketi uyuşmuyor olabilir (pnpm wasm). Dosya yazılmadı."
            ),
        )
    })?;
    let (entities, uids) = columns::unpack(cols)?;
    doc.entities = entities;
    doc.uids = uids;
    let bytes = encode_watched(&doc, watch)?;
    // The drawing goes before it is read back: a large one is not in memory twice.
    drop(doc);
    watch::report(watch, Step::Verifying)?;
    let mut back = decode_watched(&bytes, watch).map_err(not_read_back)?;
    if let Some(what) = columns::differs(cols, &back.entities, &back.uids) {
        return Err(unverified(&what));
    }
    back.entities = Vec::new();
    back.uids = Vec::new();
    Ok((bytes, head_json(&back)?))
}

/// What a file is by its content, not its name (docs/specs/kcad-v2.md §8, TODOS.md FILE-03).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sniff {
    /// The full KCAD signature: the v2 reader decides the rest.
    Kcad,
    /// “\x89KCAD” with the rest of the signature changed: a KCAD file a text transfer damaged.
    KcadDamaged,
    /// A JSON object: the v1 reader (`kentos.document` 1) decides.
    Json,
    /// Nothing but a byte order mark and white space.
    Empty,
    /// Anything else (DXF, a picture, another program's file).
    Foreign,
}

impl Sniff {
    /// As the specification and the fixtures write it.
    pub fn as_str(self) -> &'static str {
        match self {
            Sniff::Kcad => "kcad",
            Sniff::KcadDamaged => "kcad-damaged",
            Sniff::Json => "json",
            Sniff::Empty => "empty",
            Sniff::Foreign => "foreign",
        }
    }
}

pub fn sniff(data: &[u8]) -> Sniff {
    if data.starts_with(&MAGIC) {
        return Sniff::Kcad;
    }
    if data.starts_with(&header::MAGIC_PREFIX) {
        return Sniff::KcadDamaged;
    }
    let rest = data.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(data);
    match rest
        .iter()
        .find(|b| !matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
    {
        None => Sniff::Empty,
        Some(b'{') => Sniff::Json,
        Some(_) => Sniff::Foreign,
    }
}

/// Where two drawings differ: the small parts as their JSON text (every
/// float64 bit for bit, `-0.0` apart from `0.0`), the objects through their
/// typed columns; the objects' slots are not compared.
fn difference(a: &DocumentSnapshotV2, b: &DocumentSnapshotV2) -> Option<String> {
    fn same<T: Serialize + ?Sized>(x: &T, y: &T) -> bool {
        match (serde_json::to_vec(x), serde_json::to_vec(y)) {
            (Ok(x), Ok(y)) => x == y,
            _ => false,
        }
    }
    let parts: [(&str, bool); 11] = [
        ("format", a.format == b.format && a.version == b.version),
        ("name", a.name == b.name),
        ("settings", same(&a.settings, &b.settings)),
        ("origin", same(&a.origin, &b.origin)),
        ("homeView", same(&a.home_view, &b.home_view)),
        ("layers", same(&a.layers, &b.layers)),
        ("activeLayer", a.active_layer == b.active_layer),
        ("uids", a.uids == b.uids),
        ("styles", same(&a.styles, &b.styles)),
        ("projectId", a.project_id == b.project_id),
        ("migratedFrom", a.migrated_from == b.migrated_from),
    ];
    if let Some((name, _)) = parts.iter().find(|(_, same)| !same) {
        return Some((*name).to_owned());
    }
    if a.entities.len() != b.entities.len() {
        return Some("nesne sayısı".to_owned());
    }
    // Object by object through the typed columns: every field, every float bit for bit, no JSON
    // text of a large drawing (docs/adr/0030).
    columns::first_difference(&a.entities, &a.uids, &b.entities, &b.uids)
        .map(|i| format!("nesne {}", i + 1))
}
