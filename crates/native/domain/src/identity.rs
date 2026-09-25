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
/// the file's order.
///
/// PLACEHOLDER until the deterministic migration of ADR 0014 lands (TODOS.md
/// DOM-04, written in parallel): today every call gives fresh v7 ids, so the
/// same file opened twice gets different ids. The migration replaces only this
/// body, with `UUIDv5(namespace, "entity/" + local id)` where the namespace is
/// `UUIDv5(KENTOS_V1_IMPORT, sha256(canonical v1 text))`; nothing else depends
/// on how these ids are made.
pub fn v1_entity_uids(snapshot: &DocumentSnapshotV1) -> Vec<Uuid> {
    snapshot.entities.iter().map(|_| new_uid()).collect()
}
