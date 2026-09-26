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
//! - **Document schema 2** (`encode`, `decode`): the contract
//!   `DocumentSnapshotV2`, every object with its 16-byte persistent id
//!   (docs/adr/0014); the open document's slots never enter the file.
//!
//! One implementation for every platform (CLAUDE.md §14): the desktop app and
//! the server call it natively, the browser through the formats WASM module
//! (`kentos-formats-wasm`). An independent reader in Python
//! (`tools/kcad/kcad.py`) and the byte-level fixtures (`fixtures/kcad/v2`)
//! hold it to the specification.
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

mod cbor;
mod decode;
mod encode;
mod error;
mod header;

use kentos_contracts::{DocumentSnapshotV2, Entity};
use serde::Serialize;

pub use cbor::{MAX_DEPTH, MAX_ITEMS, MAX_STRING};
pub use error::{Code, KcadError};
pub use header::{
    CODEC_NONE, ENCODING_CBOR_PROFILE_1, FIXED_HEADER, HASH_SIZE, Header, MAGIC, MAJOR,
    MAX_PAYLOAD, MINOR,
};
pub use kentos_contracts as contracts;

/// The file extension (with the dot).
pub const EXTENSION: &str = ".kcad";
/// The media type: no registered KCAD type exists, so none is claimed (TODOS.md FILE-13).
pub const MIME: &str = "application/octet-stream";

/// The file a drawing is saved as.
pub fn encode(doc: &DocumentSnapshotV2) -> Result<Vec<u8>, KcadError> {
    header::write(&encode::payload(doc)?)
}

/// The file a drawing is saved as, read back before it is handed out: the
/// bytes must decode to the same drawing, every float64 bit for bit (only
/// the objects' slots, which the file does not keep, may differ). A save
/// writes only verified bytes, so a file is never replaced by one that does
/// not open again (TODOS.md FILE-16, FILE-21).
pub fn encode_verified(doc: &DocumentSnapshotV2) -> Result<Vec<u8>, KcadError> {
    let bytes = encode(doc)?;
    let back = decode(&bytes).map_err(|e| unverified(&e.message))?;
    if let Some(what) = difference(doc, &back) {
        return Err(unverified(&what));
    }
    Ok(bytes)
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
    read(data).map(|(_, doc)| doc)
}

/// The header and the drawing of a file (`kcad inspect`).
pub fn read(data: &[u8]) -> Result<(Header, DocumentSnapshotV2), KcadError> {
    let (head, payload) = header::read(data)?;
    Ok((head, decode::payload(payload)?))
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

/// Where two drawings differ, compared as their JSON text (every float64 bit
/// for bit, `-0.0` apart from `0.0`); the objects' slots are not compared.
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
    a.entities
        .iter()
        .zip(&b.entities)
        .position(|(x, y)| !same(&with_slot(x, 0), &with_slot(y, 0)))
        .map(|i| format!("nesne {}", i + 1))
}

/// An object with another slot.
fn with_slot(entity: &Entity, slot: u32) -> Entity {
    let mut e = entity.clone();
    match &mut e {
        Entity::Point(x) => x.base.id = slot,
        Entity::Line(x) => x.base.id = slot,
        Entity::Polyline(x) | Entity::Polygon(x) => x.base.id = slot,
        Entity::Circle(x) => x.base.id = slot,
        Entity::Arc(x) => x.base.id = slot,
        Entity::Ellipse(x) => x.base.id = slot,
        Entity::Spline(x) => x.base.id = slot,
        Entity::Xline(x) | Entity::Ray(x) => x.base.id = slot,
        Entity::Text(x) => x.base.id = slot,
        Entity::Dimension(x) => x.base.id = slot,
        Entity::Hatch(x) => x.base.id = slot,
    }
    e
}
