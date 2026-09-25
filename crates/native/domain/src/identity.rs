//! Object identity in the native document (docs/adr/0014): a runtime slot for
//! hot paths and a persistent UUID for everything that leaves the document.

use kentos_contracts::DocumentSnapshotV1;
pub use uuid::Uuid;

/// An object's place in the open document: the web's `Entity.id` and the
/// local id a `.kcad` v1 file writes. It lives as long as the open document
/// and is never given out twice, not even after an undone or failed edit
/// (docs/adr/0003).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Slot(pub u32);

/// A new persistent object id: UUIDv7, ordered by creation time (docs/adr/0014).
///
/// The one place the document reads the clock and the random source. When the
/// application layer gets its `IdGenerator` port (TODOS.md ARCH-02) this is
/// where it plugs in.
pub fn new_uid() -> Uuid {
    Uuid::now_v7()
}

/// Persistent ids for the objects of a `.kcad` v1 file, which has none, in
/// the file's order: the deterministic migration of ADR 0014 (TODOS.md
/// DOM-04, `kentos_contracts::v1_uids`). `UUIDv5(namespace, "entity/" + local
/// id)`, where the namespace is `UUIDv5(KENTOS_V1_IMPORT, sha256 of the
/// canonical v1 text)`, so the same file gets the same ids every time, here
/// and in the browser. Refused, with the reason, only for local ids that are
/// not unique positive numbers, which `Document::from_snapshot` checks first.
pub fn v1_entity_uids(snapshot: &DocumentSnapshotV1) -> Result<Vec<Uuid>, String> {
    let ids = kentos_contracts::v1_uids(snapshot)?;
    Ok(ids
        .entities
        .into_iter()
        .map(|(_, uid)| Uuid::from_bytes(uid))
        .collect())
}
