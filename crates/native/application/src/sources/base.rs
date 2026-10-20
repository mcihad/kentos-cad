//! A bound layer's base (docs/adr/0538 §1): each object's hashes when the layer and its table last matched, kept in
//! the project's `sourceBases` beside the link. Taken wherever a link's revision and digest are (a link, a write, a
//! reload); a reconcile reads it only when it was taken at the link's revision and digest.

use std::collections::HashMap;

use kentos_commands::{BaseRow, Entity, MAX_BASE_ROWS, SourceBase, SourceLink};
use kentos_domain::{Document, Uuid};
use sha2::{Digest, Sha256};

use crate::geopackage as gp;

/// The table's hash of an object its table did not hold when the two matched (a text the table has no geometry for).
pub(super) const ABSENT: u64 = 0;

/// An object's hash: the first 8 bytes of the SHA-256 of its essence's canonical JSON (kind, geometry, attributes),
/// never `ABSENT`.
pub(super) fn hash_of(e: &Entity) -> u64 {
    let text = kentos_commands::canonical::canonical_json(&gp::essence(e));
    let d = Sha256::digest(text.as_bytes());
    let mut b = [0u8; 8];
    b.copy_from_slice(&d[..8]);
    match u64::from_be_bytes(b) {
        ABSENT => 1,
        h => h,
    }
}

/// The objects' hashes by lasting id.
pub(super) fn hashes<'a>(items: impl IntoIterator<Item = (Uuid, &'a Entity)>) -> HashMap<Uuid, u64> {
    items.into_iter().map(|(u, e)| (u, hash_of(e))).collect()
}

/// Takes the base of `link` (as it now is: its revision and digest set) from the layer's objects and, where they read
/// back otherwise, the table's: `table` the table's objects as a reload reads them, none when they are the layer's
/// (right after a link or a reload). A layer of more than `MAX_BASE_ROWS` objects keeps none.
pub(super) fn take(doc: &mut Document, link: &SourceLink, table: Option<&[(Uuid, Entity)]>) {
    let layer: Vec<(Uuid, u64)> = doc
        .by_layer_with_uids(&link.layer)
        .map(|(u, e)| (u, hash_of(e)))
        .collect();
    let base = (layer.len() <= MAX_BASE_ROWS).then(|| {
        let table = table.map(|t| hashes(t.iter().map(|(u, e)| (*u, e))));
        let rows: Vec<BaseRow> = layer
            .iter()
            .map(|&(u, lh)| {
                let th = match &table {
                    None => lh,
                    Some(t) => t.get(&u).copied().unwrap_or(ABSENT),
                };
                (*u.as_bytes(), lh, th)
            })
            .collect();
        SourceBase {
            source: link.id.clone(),
            revision: link.revision.clone(),
            digest: link.digest.clone(),
            rows: kentos_commands::base_text(rows),
        }
    });
    let mut settings = doc.settings().clone();
    settings.source_bases.retain(|b| b.source != link.id);
    settings.source_bases.extend(base);
    doc.set_settings(settings);
}

/// The base of `link` by lasting id (the layer's hash, the table's), when it was taken at the link's revision and
/// digest.
pub(super) fn of(doc: &Document, link: &SourceLink) -> Option<HashMap<Uuid, (u64, u64)>> {
    let b = doc
        .settings()
        .source_bases
        .iter()
        .find(|b| b.source == link.id && b.revision == link.revision && b.digest == link.digest)?;
    let rows = kentos_commands::base_rows(&b.rows)?;
    Some(
        rows.into_iter()
            .map(|(u, lh, th)| (Uuid::from_bytes(u), (lh, th)))
            .collect(),
    )
}

/// Takes the base of `link` after a write: the table read again as a reload reads it, so a row that does not read back
/// bit for bit as its object is kept as the table holds it. A table that cannot be read again (gone the moment after)
/// leaves the old base, which the new revision makes stale.
pub(super) fn after_write(doc: &mut Document, link: &SourceLink) {
    if doc.by_layer_with_uids(&link.layer).nth(MAX_BASE_ROWS).is_some() {
        take(doc, link, None);
        return;
    }
    let Ok(mut loaded) = super::together::load_link(doc, link, "layer") else {
        return;
    };
    if super::together::patch_read(doc, link, &mut loaded.stored).is_err() {
        return;
    }
    let objects = loaded.stored.in_project(loaded.mover.as_ref());
    take(doc, link, Some(&objects));
}

/// The base of a link whose file another table's write changed (docs/adr/0532 §1): its table was not touched, so its
/// base holds at the file's new revision.
pub(super) fn carried(doc: &mut Document, link: &SourceLink, before: &str) {
    let mut settings = doc.settings().clone();
    let Some(b) = settings
        .source_bases
        .iter_mut()
        .find(|b| b.source == link.id && b.revision == before && b.digest == link.digest)
    else {
        return;
    };
    b.revision = link.revision.clone();
    doc.set_settings(settings);
}
