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

/// The document schemas this codec reads, oldest first.
pub const SCHEMAS: [u32; 28] = [
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
