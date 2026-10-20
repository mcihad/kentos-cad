//! What a base keeps of an object besides its two hashes, and the field-by-field merge it allows (docs/adr/0553 §1):
//! the hashes of the object's kind and geometry on each side, of each attribute (name and value) and of its vertices.
//! With them an object both sides changed is put together against the base: an attribute only one side changed takes
//! that side's value, the geometry likewise; only what both changed differently is a conflict. The vertices tell
//! neighbours whose shared corner was moved from neighbours that merely sit next to each other (`reconcile`'s groups).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use kentos_commands::{
    BaseDetail, ConflictField, Entity, MAX_BASE_VERTICES, ReconcileSide,
};
use serde_json::Value as J;
use sha2::{Digest, Sha256};

use crate::geopackage as gp;

/// How the table holds an object when a base is taken.
pub(super) enum Table<'a> {
    /// As the layer does (right after a link or a reload).
    Same,
    /// The table has no row for it.
    Absent,
    /// The object its row reads as.
    Is(&'a Entity),
}

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// An attribute name's hash: the first 4 bytes of its SHA-256.
pub(super) fn name_hash(name: &str) -> u32 {
    let d = digest(name.as_bytes());
    u32::from_be_bytes([d[0], d[1], d[2], d[3]])
}

/// An attribute value's hash: the first 8 bytes of its SHA-256.
pub(super) fn value_hash(value: &str) -> u64 {
    let d = digest(value.as_bytes());
    u64::from_be_bytes(d[..8].try_into().expect("8 bytes"))
}

/// The essence of an object without its attributes: its kind and geometry.
fn shape_value(e: &Entity) -> J {
    let mut v = gp::essence(e);
    if let Some(m) = v.as_object_mut() {
        m.remove("attrs");
    }
    v
}

/// The hash of an object's kind and geometry: the first 8 bytes of the SHA-256 of their canonical JSON.
pub(super) fn shape_hash(e: &Entity) -> u64 {
    let text = kentos_commands::canonical::canonical_json(&shape_value(e));
    u64::from_be_bytes(digest(text.as_bytes())[..8].try_into().expect("8 bytes"))
}

/// Whether two objects' kinds or geometries differ.
pub(super) fn shapes_differ(a: &Entity, b: &Entity) -> bool {
    shape_value(a) != shape_value(b)
}

fn collect(v: &J, out: &mut Vec<(u64, u64)>) {
    match v {
        J::Object(m) => {
            if let (Some(x), Some(y)) = (m.get("x").and_then(J::as_f64), m.get("y").and_then(J::as_f64))
                && m.len() <= 4
            {
                out.push((x.to_bits(), y.to_bits()));
                return;
            }
            for v in m.values() {
                collect(v, out);
            }
        }
        J::Array(a) => a.iter().for_each(|v| collect(v, out)),
        _ => {}
    }
}

/// The hash of a vertex: FNV-1a over the bits of its two coordinates.
fn vertex_hash((x, y): (u64, u64)) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for b in x.to_be_bytes().into_iter().chain(y.to_be_bytes()) {
        h ^= u32::from(b);
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

/// The hashes of an object's vertices, rising and once each; none when it has more than `MAX_BASE_VERTICES` (or
/// the point is in a bare `x`, `y` it does not have).
pub(super) fn vertex_set(e: &Entity) -> Option<Vec<u32>> {
    let essence = gp::essence(e);
    let mut raw = Vec::new();
    if let Some(m) = essence.as_object() {
        for key in ["p", "pts", "parts", "holes", "a", "b"] {
            if let Some(v) = m.get(key) {
                collect(v, &mut raw);
            }
        }
    }
    if raw.len() > MAX_BASE_VERTICES {
        return None;
    }
    let set: BTreeSet<u32> = raw.into_iter().map(vertex_hash).collect();
    Some(set.into_iter().collect())
}

/// An object's attribute hashes by rising name hash; none when two names share a hash.
fn attr_hashes(e: &Entity) -> Option<Vec<(u32, u64)>> {
    let mut by: BTreeMap<u32, u64> = BTreeMap::new();
    for (n, v) in &e.base().attrs {
        if by.insert(name_hash(n), value_hash(v)).is_some() {
            return None;
        }
    }
    Some(by.into_iter().collect())
}

/// What a base keeps of `layer`'s object with `table` as the table holds it.
pub(super) fn detail_of(layer: &Entity, table: Table<'_>) -> BaseDetail {
    let own = shape_hash(layer);
    let (table_shape, same_attrs, same_shape) = match table {
        Table::Same => (own, true, true),
        Table::Absent => (0, false, false),
        Table::Is(t) => {
            let ts = shape_hash(t);
            (ts, t.base().attrs == layer.base().attrs, ts == own)
        }
    };
    BaseDetail {
        shape: (own, table_shape),
        attrs: if same_attrs { attr_hashes(layer) } else { None },
        vertices: if same_shape { vertex_set(layer) } else { None },
    }
}

/// An object both sides changed, put together against the base.
pub(super) struct Merge {
    /// The object: the kind and geometry that won, the attributes merged (a conflicting one as `prefer` says, the
    /// layer's when no side is preferred).
    pub entity: Entity,
    pub from_layer: Vec<String>,
    pub from_table: Vec<String>,
    /// The side whose geometry the merge has, when only one changed it.
    pub geometry: Option<ReconcileSide>,
    /// The attributes both changed differently.
    pub conflicts: Vec<ConflictField>,
    /// Both changed the kind or geometry differently.
    pub geometry_conflict: bool,
}

fn changed(base: Option<u64>, value: Option<&String>) -> bool {
    match (base, value) {
        (None, None) => false,
        (Some(b), Some(v)) => b != value_hash(v),
        _ => true,
    }
}

/// The three-way merge of `layer` and `table` against `detail`; none where the base keeps no attribute hashes of the
/// object (it conflicts as a whole).
pub(super) fn merge(
    layer: &Entity,
    table: &Entity,
    detail: &BaseDetail,
    prefer: Option<ReconcileSide>,
) -> Option<Merge> {
    let base: HashMap<u32, u64> = detail.attrs.as_ref()?.iter().copied().collect();
    let (la, ta) = (&layer.base().attrs, &table.base().attrs);
    let names: BTreeSet<&String> = la.keys().chain(ta.keys()).collect();
    let mut attrs: BTreeMap<String, String> = BTreeMap::new();
    let (mut from_layer, mut from_table, mut conflicts) = (Vec::new(), Vec::new(), Vec::new());
    for n in names {
        let b = base.get(&name_hash(n)).copied();
        let (lv, tv) = (la.get(n), ta.get(n));
        let pick = match (changed(b, lv), changed(b, tv)) {
            (false, false) => lv,
            (true, false) => {
                from_layer.push(n.clone());
                lv
            }
            (false, true) => {
                from_table.push(n.clone());
                tv
            }
            (true, true) if lv == tv => lv,
            (true, true) => {
                conflicts.push(ConflictField {
                    name: n.clone(),
                    layer: lv.cloned(),
                    table: tv.cloned(),
                });
                if prefer == Some(ReconcileSide::Table) { tv } else { lv }
            }
        };
        if let Some(v) = pick {
            attrs.insert(n.clone(), v.clone());
        }
    }
    let (ls, ts) = (shape_hash(layer), shape_hash(table));
    let (l_moved, t_moved) = (ls != detail.shape.0, ts != detail.shape.1);
    let (from, geometry, geometry_conflict) = match (l_moved, t_moved) {
        (false, false) => (layer, None, false),
        (true, false) => (layer, Some(ReconcileSide::Layer), false),
        (false, true) => (table, Some(ReconcileSide::Table), false),
        (true, true) if ls == ts => (layer, None, false),
        (true, true) => (
            if prefer == Some(ReconcileSide::Table) { table } else { layer },
            None,
            true,
        ),
    };
    let mut entity = from.clone();
    entity.base_mut().attrs = attrs;
    Some(Merge {
        entity,
        from_layer,
        from_table,
        geometry,
        conflicts,
        geometry_conflict,
    })
}
