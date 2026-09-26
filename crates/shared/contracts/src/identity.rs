//! Persistent object identity (docs/adr/0014): the text form of a UUID,
//! UUIDv5 (RFC 9562 §5.5) and the ids a v1 drawing's objects get when it is
//! opened (TODOS.md `DOM-04`).
//!
//! A v1 `.kcad` numbers its objects 1…n inside the file and keeps no
//! persistent ids. They are derived from the file's content, so the same
//! drawing gets the same ids every time and wherever it is opened: in the
//! browser (through the formats WASM module), in the desktop app, on the
//! server.
//!
//! 1. The file is read as `DocumentSnapshotV1` and written again canonically
//!    (`write_canonical_v1`).
//! 2. `namespace = UUIDv5(KENTOS_V1_IMPORT, h)`, where `h` is the sha256 of
//!    that text written as 64 lowercase hexadecimal digits.
//! 3. An object's id is `UUIDv5(namespace, "entity/" + its local id)`, the
//!    local id in decimal; the project's is `UUIDv5(namespace, "project")`.
//!
//! Names are hashed as UTF-8 text. SHA-1 serves determinism here, not
//! security. The same steps are computed independently, with Python's
//! standard library, by `scripts/fixtures/v1_identity_reference.py`;
//! `fixtures/document/v1/identity` holds its results, which the native test
//! (`tests/identity.rs`) and the browser's (`apps/web/src/io/identity.wasm.test.ts`)
//! reproduce.

use std::collections::HashSet;
use std::fmt::Write as _;
use std::io;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha1::{Digest, Sha1};
use sha2::Sha256;
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::document::{
    DOCUMENT_FORMAT, DOCUMENT_VERSION_2, DocumentSnapshotV1, DocumentSnapshotV2, MigrationSource,
};
use crate::layer::LayerNode;

/// `4a5259a1-97f7-4742-88ce-b747287025ed`: the namespace every v1 drawing's
/// ids are derived from (a random v4 UUID drawn once, 25 September 2026).
/// It never changes: another value would give every old drawing new ids.
pub const KENTOS_V1_IMPORT: [u8; 16] = [
    0x4a, 0x52, 0x59, 0xa1, 0x97, 0xf7, 0x47, 0x42, 0x88, 0xce, 0xb7, 0x47, 0x28, 0x70, 0x25, 0xed,
];

/// The persistent ids of a v1 drawing's objects, and where they come from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct V1Identities {
    /// sha256 of the canonical text, 64 lowercase hexadecimal digits: the
    /// source a v2 file records next to the ids (TODOS.md `FILE-05`, `FILE-21`).
    pub source_sha256: String,
    /// `UUIDv5(KENTOS_V1_IMPORT, source_sha256)`.
    pub namespace: String,
    /// `UUIDv5(namespace, "project")`.
    pub project: String,
    /// Every object in file order.
    pub entities: Vec<V1EntityIdentity>,
}

/// One object of a v1 drawing: its local id in the file and its persistent id.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct V1EntityIdentity {
    pub id: u32,
    /// `UUIDv5(namespace, "entity/" + id)`, lowercase with hyphens.
    pub uid: String,
}

/// An object's persistent id (docs/adr/0014): given when the object is
/// created (UUIDv7) or derived from a v1 file (UUIDv5), kept by every edit,
/// never reused. Files, the server, Python and AI name objects by it; the
/// open document's slot (`EntityBase.id`) never leaves the document.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export, type = "string"))]
pub struct EntityId(pub [u8; 16]);

/// A project's persistent id, when it has one: derived from a v1 file
/// (`UUIDv5(namespace, "project")`, docs/adr/0014) or read from a v2 file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export, type = "string"))]
pub struct ProjectId(pub [u8; 16]);

/// What a UUID of the contracts is: its 16 bytes in memory and in `.kcad` v2
/// files (docs/specs/kcad-v2.md §6.3), lowercase text with hyphens in JSON and
/// on the wire (docs/adr/0014). Reading text refuses anything else: upper
/// case, braces, missing hyphens.
macro_rules! uuid_impls {
    ($name:ident) => {
        impl $name {
            /// The UUID as KentOS writes it: lowercase, with hyphens.
            pub fn to_text(&self) -> String {
                uuid_text(&self.0)
            }

            /// Reads a UUID written as KentOS writes it; anything else is `None`.
            pub fn parse(text: &str) -> Option<Self> {
                uuid_from_text(text).map(Self)
            }

            /// The nil UUID (16 zero bytes): never a valid id in a file.
            pub fn is_nil(&self) -> bool {
                self.0 == [0; 16]
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.to_text())
            }
        }

        impl Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(&self.to_text())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let text = std::borrow::Cow::<str>::deserialize(d)?;
                Self::parse(&text).ok_or_else(|| {
                    serde::de::Error::custom(format!(
                        "“{text}” küçük harfli, tireli bir UUID değil (8-4-4-4-12 onaltılık hane)"
                    ))
                })
            }
        }

        #[cfg(feature = "schema")]
        impl schemars::JsonSchema for $name {
            fn schema_name() -> std::borrow::Cow<'static, str> {
                stringify!($name).into()
            }

            fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
                schemars::json_schema!({
                    "type": "string",
                    "format": "uuid",
                    "pattern": "^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"
                })
            }
        }
    };
}

uuid_impls!(EntityId);
uuid_impls!(ProjectId);

/// Reads a UUID written lowercase with hyphens (8-4-4-4-12), as `uuid_text` writes it.
pub fn uuid_from_text(text: &str) -> Option<[u8; 16]> {
    let raw = text.as_bytes();
    if raw.len() != 36 {
        return None;
    }
    let digit = |c: u8| match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    };
    let mut out = [0u8; 16];
    let mut at = 0;
    for (i, byte) in out.iter_mut().enumerate() {
        if matches!(i, 4 | 6 | 8 | 10) {
            if raw[at] != b'-' {
                return None;
            }
            at += 1;
        }
        *byte = (digit(raw[at])? << 4) | digit(raw[at + 1])?;
        at += 2;
    }
    Some(out)
}

/// UUIDv5 (RFC 9562 §5.5): SHA-1 of the namespace's 16 bytes and the name,
/// its first 16 bytes with the version (5) and the variant (10) set.
pub fn uuid_v5(namespace: &[u8; 16], name: &[u8]) -> [u8; 16] {
    let mut sha = Sha1::new();
    sha.update(namespace);
    sha.update(name);
    let digest = sha.finalize();
    let mut out = [0u8; 16];
    out.copy_from_slice(&digest[..16]);
    out[6] = (out[6] & 0x0f) | 0x50;
    out[8] = (out[8] & 0x3f) | 0x80;
    out
}

/// A UUID as KentOS writes it: lowercase, with hyphens (8-4-4-4-12).
pub fn uuid_text(uuid: &[u8; 16]) -> String {
    let h = hex(uuid);
    format!(
        "{}-{}-{}-{}-{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    )
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(char::from(DIGITS[usize::from(b >> 4)]));
        out.push(char::from(DIGITS[usize::from(b & 0x0f)]));
    }
    out
}

/// Writes a v1 snapshot as its canonical JSON text: what `serde_json` writes
/// for the contract types, without whitespace. Fields come in the order the
/// contract declares them (`document.rs`, `layer.rs`, `entity.rs`), absent
/// optional fields are left out, every float64 is written in its shortest
/// round-trip form (`486512.0`, `0.18`, `1e-7`), attributes are sorted by
/// name, and the keys of the opaque parts (the project's style items and
/// categories, layer renderers) are sorted at every depth. Fields the
/// contract does not know are not written.
///
/// So whitespace, line ends, the order of keys and the spelling of a float64
/// in the file (`486512`, `486512.0`, `4.86512e5`) do not change the text;
/// any change of content does. In the opaque parts an integer and a float
/// stay apart (`2` and `2.0`), as JSON values do.
///
/// The declaration order of the contract's fields is part of this text:
/// reordering them would give every v1 drawing new ids, and the fixture test
/// fails.
pub fn write_canonical_v1<W: io::Write>(
    snapshot: &DocumentSnapshotV1,
    out: W,
) -> Result<(), String> {
    // serde_json keeps object keys sorted unless a build enables its
    // `preserve_order` feature (none does); then they are sorted on a copy, so
    // the text never follows the file's key order.
    if opaque_sorted(snapshot) {
        return write(snapshot, out);
    }
    let mut sorted = snapshot.clone();
    for v in sorted
        .styles
        .items
        .iter_mut()
        .chain(sorted.styles.categories.iter_mut())
    {
        v.sort_all_objects();
    }
    sort_renderers(&mut sorted.layers);
    write(&sorted, out)
}

fn write<W: io::Write>(snapshot: &DocumentSnapshotV1, out: W) -> Result<(), String> {
    serde_json::to_writer(out, snapshot).map_err(|e| format!("Çizim yazılamadı: {e}"))
}

/// Whether every object in the opaque parts lists its keys in order.
fn opaque_sorted(snapshot: &DocumentSnapshotV1) -> bool {
    snapshot
        .styles
        .items
        .iter()
        .chain(&snapshot.styles.categories)
        .all(sorted_value)
        && renderers_sorted(&snapshot.layers)
}

fn sorted_value(v: &Value) -> bool {
    match v {
        Value::Object(map) => {
            map.keys().zip(map.keys().skip(1)).all(|(a, b)| a < b) && map.values().all(sorted_value)
        }
        Value::Array(list) => list.iter().all(sorted_value),
        _ => true,
    }
}

fn renderers_sorted(nodes: &[LayerNode]) -> bool {
    nodes.iter().all(|n| {
        n.style.renderer.as_ref().is_none_or(sorted_value) && renderers_sorted(&n.children)
    })
}

fn sort_renderers(nodes: &mut [LayerNode]) {
    for n in nodes {
        if let Some(r) = n.style.renderer.as_mut() {
            r.sort_all_objects();
        }
        sort_renderers(&mut n.children);
    }
}

/// The canonical text of a v1 snapshot (`write_canonical_v1`).
pub fn canonical_v1(snapshot: &DocumentSnapshotV1) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    write_canonical_v1(snapshot, &mut out)?;
    Ok(out)
}

/// The persistent ids of a v1 snapshot's objects as UUID bytes (`v1_uids`);
/// `V1Identities` is their text form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V1Uids {
    /// sha256 of the canonical text, 64 lowercase hexadecimal digits.
    pub source_sha256: String,
    pub namespace: [u8; 16],
    pub project: [u8; 16],
    /// Every object in file order: its local id and its persistent id.
    pub entities: Vec<(u32, [u8; 16])>,
}

/// The persistent ids of a v1 snapshot's objects (see the module). Local ids
/// must be positive and unique, as the readers require; the canonical text
/// goes straight into the hash, it is never held whole.
pub fn v1_uids(snapshot: &DocumentSnapshotV1) -> Result<V1Uids, String> {
    let mut sha = HashWriter(Sha256::new());
    write_canonical_v1(snapshot, &mut sha)?;
    let source_sha256 = hex(&sha.0.finalize());
    let namespace = uuid_v5(&KENTOS_V1_IMPORT, source_sha256.as_bytes());
    let mut seen = HashSet::with_capacity(snapshot.entities.len());
    let mut name = String::new();
    let mut entities = Vec::with_capacity(snapshot.entities.len());
    for (i, e) in snapshot.entities.iter().enumerate() {
        let id = e.base().id;
        if id == 0 || !seen.insert(id) {
            return Err(format!(
                "Nesne {} ({}): yerel kimlik {id} benzersiz, pozitif bir tam sayı olmalı.",
                i + 1,
                e.kind()
            ));
        }
        name.clear();
        let _ = write!(name, "entity/{id}");
        entities.push((id, uuid_v5(&namespace, name.as_bytes())));
    }
    Ok(V1Uids {
        source_sha256,
        namespace,
        project: uuid_v5(&namespace, b"project"),
        entities,
    })
}

/// `v1_uids` in text form, as the browser and the fixture take them.
pub fn v1_identities_of(snapshot: &DocumentSnapshotV1) -> Result<V1Identities, String> {
    let ids = v1_uids(snapshot)?;
    Ok(V1Identities {
        source_sha256: ids.source_sha256,
        namespace: uuid_text(&ids.namespace),
        project: uuid_text(&ids.project),
        entities: ids
            .entities
            .iter()
            .map(|(id, uid)| V1EntityIdentity {
                id: *id,
                uid: uuid_text(uid),
            })
            .collect(),
    })
}

/// The persistent ids of a v1 drawing's objects, from the file's text, read as
/// `DocumentSnapshotV1::from_json` reads it.
pub fn v1_identities(text: &str) -> Result<V1Identities, String> {
    v1_identities_of(&DocumentSnapshotV1::from_json(text)?)
}

/// A v1 drawing as a v2 snapshot (docs/specs/kcad-v2.md §7): every object gets
/// its derived persistent id, the project its derived id, and the source is
/// recorded, so a v2 file keeps where the ids came from (TODOS.md FILE-05,
/// FILE-21). The objects keep their v1 local ids as slots; a v2 file does not
/// write them.
pub fn migrate_v1(snapshot: DocumentSnapshotV1) -> Result<DocumentSnapshotV2, String> {
    let ids = v1_uids(&snapshot)?;
    let DocumentSnapshotV1 {
        name,
        settings,
        origin,
        home_view,
        layers,
        active_layer,
        entities,
        styles,
        ..
    } = snapshot;
    Ok(DocumentSnapshotV2 {
        format: DOCUMENT_FORMAT.to_owned(),
        version: DOCUMENT_VERSION_2,
        name,
        settings,
        origin,
        home_view,
        layers,
        active_layer,
        entities,
        uids: ids.entities.iter().map(|&(_, uid)| EntityId(uid)).collect(),
        styles,
        project_id: Some(ProjectId(ids.project)),
        migrated_from: Some(MigrationSource::v1(ids.source_sha256)),
    })
}

/// A hash that takes what `serde_json` writes, piece by piece.
struct HashWriter(Sha256);

impl io::Write for HashWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_v5_matches_the_rfc_example() {
        // RFC 9562 Appendix A.4: the DNS namespace and "www.example.com".
        let dns = [
            0x6b, 0xa7, 0xb8, 0x10, 0x9d, 0xad, 0x11, 0xd1, 0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4,
            0x30, 0xc8,
        ];
        assert_eq!(
            uuid_text(&uuid_v5(&dns, b"www.example.com")),
            "2ed6657d-e927-568b-95e1-2665a8aea6a2"
        );
    }

    #[test]
    fn the_import_namespace_is_the_documented_one() {
        assert_eq!(
            uuid_text(&KENTOS_V1_IMPORT),
            "4a5259a1-97f7-4742-88ce-b747287025ed"
        );
    }
}
