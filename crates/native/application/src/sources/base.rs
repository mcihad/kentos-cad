//! A bound layer's base (docs/adr/0538 §1): each object's hashes when the layer and its table last matched, kept in
//! the project's `sourceBases` beside the link. Taken wherever a link's revision and digest are (a link, a write, a
//! reload); a reconcile reads it only when it was taken at the link's revision and digest.

use std::collections::HashMap;

use kentos_commands::{
    BaseDetail, BaseRow, Entity, MAX_BASE_ROWS, SourceBase, SourceLink, base_details,
    base_details_text,
};
use kentos_domain::{Document, Uuid};
use sha2::{Digest, Sha256};

use super::merge::{self, Table};
use crate::geopackage as gp;

/// The table's hash of an object its table did not hold when the two matched (a text the table has no geometry for).
pub(crate) const ABSENT: u64 = 0;

/// An object's hash: the first 8 bytes of the SHA-256 of its essence's canonical JSON (kind, geometry, attributes),
/// never `ABSENT`.
pub(crate) fn hash_of(e: &Entity) -> u64 {
    let text = kentos_commands::canonical::canonical_json(&gp::essence(e));
    let d = Sha256::digest(text.as_bytes());
    let mut b = [0u8; 8];
    b.copy_from_slice(&d[..8]);
    match u64::from_be_bytes(b) {
        ABSENT => 1,
        h => h,
    }
}

/// Takes the base of `link` (as it now is: its revision and digest set) from the layer's objects and, where they read
/// back otherwise, the table's: `table` the table's objects as a reload reads them, none when they are the layer's
/// (right after a link or a reload). A layer of more than `MAX_BASE_ROWS` objects keeps none.
pub(super) fn take(doc: &mut Document, link: &SourceLink, table: Option<&[(Uuid, Entity)]>) {
    let layer: Vec<(Uuid, &Entity)> = doc.by_layer_with_uids(&link.layer).collect();
    // The objects of the other layers bound to the same source (one GeoPackage file, one database): a reconcile takes
    // them together (docs/adr/0648 §3), so a corner shared with one of them ties too (docs/adr/0737 §2).
    let others: Vec<&Entity> = doc
        .settings()
        .sources
        .iter()
        .filter(|l| l.layer != link.layer && l.kind == link.kind && l.path == link.path)
        .flat_map(|l| doc.by_layer_with_uids(&l.layer).map(|(_, e)| e))
        .collect();
    let base = (layer.len() <= MAX_BASE_ROWS).then(|| {
        let table: Option<HashMap<Uuid, &Entity>> =
            table.map(|t| t.iter().map(|(u, e)| (*u, e)).collect());
        let mut kept: Vec<(BaseRow, BaseDetail)> = layer
            .iter()
            .map(|&(u, e)| {
                let lh = hash_of(e);
                let (th, detail) = match &table {
                    None => (lh, merge::detail_of(e, Table::Same)),
                    Some(t) => match t.get(&u) {
                        None => (ABSENT, merge::detail_of(e, Table::Absent)),
                        Some(te) => (hash_of(te), merge::detail_of(e, Table::Is(te))),
                    },
                };
                ((*u.as_bytes(), lh, th), detail)
            })
            .collect();
        shared_corners(&layer, &others, &mut kept);
        kept.sort_by_key(|(row, _)| row.0);
        let details: Vec<BaseDetail> = kept.iter().map(|(_, d)| d.clone()).collect();
        let rows: Vec<BaseRow> = kept.into_iter().map(|(r, _)| r).collect();
        SourceBase {
            source: link.id.clone(),
            revision: link.revision.clone(),
            digest: link.digest.clone(),
            rows: kentos_commands::base_text(rows),
            fields: base_details_text(&details),
        }
    });
    let mut settings = doc.settings().clone();
    settings.source_bases.retain(|b| b.source != link.id);
    settings.source_bases.extend(base);
    doc.set_settings(settings);
}

/// docs/adr/0648 §4: an object of more vertices than a record keeps (a many-cornered or many-part one) keeps the
/// vertices it shares with another object of the layer or of the source's other bound layers (`others`, docs/adr/0737
/// §2), when they are few enough: its neighbours whose shared corner both sides moved are still one group. `kept` is in
/// `layer`'s order.
fn shared_corners(
    layer: &[(Uuid, &Entity)],
    others: &[&Entity],
    kept: &mut [(BaseRow, BaseDetail)],
) {
    let big: Vec<usize> = kept
        .iter()
        .enumerate()
        .filter(|(_, (_, d))| d.vertices.is_none() && d.shape.0 == d.shape.1)
        .map(|(i, _)| i)
        .collect();
    if big.is_empty() {
        return;
    }
    let all: Vec<Vec<u32>> = layer
        .iter()
        .map(|(_, e)| merge::vertex_set_all(e))
        .collect();
    let mut count: HashMap<u32, u32> = HashMap::new();
    for set in &all {
        for q in set {
            *count.entry(*q).or_default() += 1;
        }
    }
    for e in others {
        for q in merge::vertex_set_all(e) {
            *count.entry(q).or_default() += 1;
        }
    }
    for i in big {
        let shared: Vec<u32> = all[i]
            .iter()
            .copied()
            .filter(|q| count.get(q).is_some_and(|n| *n > 1))
            .collect();
        if shared.len() <= kentos_commands::MAX_BASE_VERTICES {
            kept[i].1.vertices = Some(shared);
        }
    }
}

/// One object of a base: the layer's hash, the table's and what the base keeps besides (none when the base has no
/// field records).
pub(super) struct BaseObj {
    pub layer: u64,
    pub table: u64,
    pub detail: Option<BaseDetail>,
}

/// The base of `link` by lasting id, when it was taken at the link's revision and digest.
pub(super) fn of(doc: &Document, link: &SourceLink) -> Option<HashMap<Uuid, BaseObj>> {
    let b =
        doc.settings().source_bases.iter().find(|b| {
            b.source == link.id && b.revision == link.revision && b.digest == link.digest
        })?;
    let rows = kentos_commands::base_rows(&b.rows)?;
    let mut details = b
        .fields
        .as_deref()
        .and_then(|t| base_details(rows.len(), t))
        .map(Vec::into_iter);
    Some(
        rows.into_iter()
            .map(|(u, lh, th)| {
                let detail = details.as_mut().and_then(Iterator::next);
                (
                    Uuid::from_bytes(u),
                    BaseObj {
                        layer: lh,
                        table: th,
                        detail,
                    },
                )
            })
            .collect(),
    )
}

/// Takes the base of `link` after a write: the table read again as a reload reads it, so a row that does not read back
/// bit for bit as its object is kept as the table holds it. A table that cannot be read again (gone the moment after)
/// leaves the old base, which the new revision makes stale.
pub(super) fn after_write(doc: &mut Document, link: &SourceLink) {
    if doc
        .by_layer_with_uids(&link.layer)
        .nth(MAX_BASE_ROWS)
        .is_some()
    {
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
