//! A block's inserts kept with its definition (docs/adr/0317 §1, §2): `cad.blocks.edit`'s `sync` (ATTSYNC), the
//! sync `attributes` and `update` do unless asked not to, `fields` (UPDATEFIELD) and `place`.
//!
//! An insert holds only the values it was given (Blok ekle writes none for an attribute left at its default), so a
//! sync writes only what no longer fits:
//! - a value under a renamed tag moves to the new tag (`renames`);
//! - a value under a tag no attribute has any more (or named by `removeTags`) is taken off;
//! - a constant attribute's own value is taken off (the definition's shows);
//! - a place of an attribute no longer movable is dropped;
//! - a field's value is filled again (its template with the insert's values, `info_templates`).
//!
//! The definition's tags an insert has no value of are reported (`added`): the insert shows their default. Only the
//! drawing's inserts are synced; an insert inside another definition keeps its values (a definition is changed by
//! its own edits).

use std::collections::{BTreeMap, HashSet};

use kentos_contracts::{
    AttributeDefinition, AttributeSync, BlockDefinition, BlockId, Entity, InsertEntity, Vec2,
};
use kentos_domain::{Document, Slot};

use crate::checks::{Stop, error};
use crate::codes;
use crate::info_templates::{CompiledTemplate, compile_template};

/// The inserts' changes and what they did.
pub(crate) struct Synced {
    pub changes: Vec<(Slot, Entity)>,
    pub report: AttributeSync,
    /// Inserts on a locked layer, left as they are (docs/adr/0278).
    pub locked: usize,
}

/// The undo steps' names.
pub const SYNC: &str = "Öznitelikleri eşitle";
pub const FIELDS: &str = "Alanları güncelle";
pub const PLACE: &str = "Öznitelik konumu";

/// The fields of `attributes` compiled, each with its tag; the first that does not compile refuses (`invalid_field`),
/// said at `path(i)` (the input's attribute, or the block the input names).
pub(crate) fn fields_of(
    doc: &Document,
    attributes: &[AttributeDefinition],
    path: &dyn Fn(usize) -> String,
) -> Result<Vec<(String, CompiledTemplate)>, Stop> {
    let unit = crate::expression_unit::reading_unit(doc.settings());
    let mut out = Vec::new();
    for (i, a) in attributes.iter().enumerate() {
        let Some(text) = &a.options.field else {
            continue;
        };
        let compiled = compile_template(text, unit).map_err(|why| {
            Stop::failed(error(
                codes::INVALID_FIELD,
                format!(
                    "“{}” özniteliğinin alanı okunamadı: {why} Alanı {{ifade}} parçalı bir şablon olarak yazın.",
                    a.tag
                ),
                Some(path(i)),
            ))
        })?;
        out.push((a.tag.clone(), compiled));
    }
    Ok(out)
}

/// The drawing's inserts of `next`'s id fitted to `next`, which was `old` (the same for a sync of its own).
pub(crate) fn sync(
    doc: &Document,
    old: &BlockDefinition,
    next: &BlockDefinition,
    renames: &BTreeMap<String, String>,
    remove: &[String],
    fill: bool,
    only_fields: bool,
) -> Result<Synced, Stop> {
    let tag_of = |tag: &str| next.attributes.iter().find(|a| a.tag == tag);
    // Tags whose values go: an old attribute's that is not renamed and no longer is, and those named.
    let mut gone: Vec<String> = old
        .attributes
        .iter()
        .map(|a| a.tag.clone())
        .filter(|t| tag_of(t).is_none() && !renames.contains_key(t))
        .collect();
    for t in remove {
        if !gone.contains(t) {
            gone.push(t.clone());
        }
    }
    let fields = if fill {
        fields_of(doc, &next.attributes, &|i| format!("attributes[{i}].field"))?
    } else {
        Vec::new()
    };
    let mut report = AttributeSync::default();
    // What was done, each tag once; ordered at the end (`gone`'s, the renames', the definition's order).
    let mut removed: HashSet<String> = HashSet::new();
    let mut renamed: HashSet<String> = HashSet::new();
    let mut constant: HashSet<String> = HashSet::new();
    let mut added: HashSet<&str> = HashSet::new();
    let mut fitted: Vec<(Slot, InsertEntity, bool)> = Vec::new();
    let mut locked = 0;
    let layers = doc.layers();
    for e in doc.entities() {
        let Entity::Insert(i) = e else { continue };
        if i.block != next.id {
            continue;
        }
        if layers.is_locked(&i.base.layer_id) {
            locked += 1;
            continue;
        }
        let mut i = i.clone();
        let mut changed = false;
        if only_fields {
            fitted.push((Slot(i.base.id), i, false));
            continue;
        }
        for (from, to) in renames {
            if let Some(v) = i.base.attrs.remove(from) {
                i.base.attrs.insert(to.clone(), v);
                changed = true;
                renamed.insert(from.clone());
            }
            if let Some(p) = i.attribute_places.remove(from) {
                i.attribute_places.insert(to.clone(), p);
                changed = true;
            }
        }
        for t in &gone {
            if i.base.attrs.remove(t).is_some() {
                changed = true;
                removed.insert(t.clone());
            }
        }
        for a in &next.attributes {
            if a.options.constant {
                if i.base.attrs.remove(&a.tag).is_some() {
                    changed = true;
                    constant.insert(a.tag.clone());
                }
            } else if !i.base.attrs.contains_key(&a.tag) {
                added.insert(&a.tag);
            }
        }
        let before = i.attribute_places.len();
        i.attribute_places
            .retain(|tag, _| tag_of(tag).is_some_and(|a| a.options.movable));
        let dropped = before - i.attribute_places.len();
        if dropped > 0 {
            report.places += u32::try_from(dropped).unwrap_or(u32::MAX);
            changed = true;
        }
        fitted.push((Slot(i.base.id), i, changed));
    }
    // Fields, filled with the insert's values as they are now.
    if !fields.is_empty() && !fitted.is_empty() {
        let entities: Vec<Entity> = fitted
            .iter()
            .map(|(_, i, _)| Entity::Insert(i.clone()))
            .collect();
        let shapes: Vec<kentos_geometry_core::entity::Shape> =
            entities.iter().map(crate::geometry::shape).collect();
        let list: Vec<&Entity> = entities.iter().collect();
        let layer_name = |id: &str| {
            doc.layers()
                .get(id)
                .map_or_else(|| id.to_owned(), |n| n.name.clone())
        };
        for (tag, template) in &fields {
            let texts = template.texts(&list, &layer_name, &|k| shapes.get(k), &|_| None);
            for ((_, i, changed), text) in fitted.iter_mut().zip(texts) {
                if i.base.attrs.get(tag) != Some(&text) {
                    i.base.attrs.insert(tag.clone(), text);
                    *changed = true;
                    report.fields += 1;
                }
            }
        }
    }
    let changes: Vec<(Slot, Entity)> = fitted
        .into_iter()
        .filter(|(_, _, changed)| *changed)
        .map(|(slot, i, _)| (slot, Entity::Insert(i)))
        .collect();
    report.inserts = u32::try_from(changes.len()).unwrap_or(u32::MAX);
    report.added = next
        .attributes
        .iter()
        .filter(|a| added.contains(a.tag.as_str()))
        .map(|a| a.tag.clone())
        .collect();
    report.removed = gone.into_iter().filter(|t| removed.contains(t)).collect();
    report.renamed = renames
        .iter()
        .filter(|(from, _)| renamed.contains(*from))
        .map(|(from, to)| [from.clone(), to.clone()])
        .collect();
    report.constant = next
        .attributes
        .iter()
        .filter(|a| constant.contains(&a.tag))
        .map(|a| a.tag.clone())
        .collect();
    Ok(Synced {
        changes,
        report,
        locked,
    })
}

/// The renames given checked against `old` and `next` (docs/adr/0317 §2): each from an old attribute's tag (or a
/// value's, for `sync`) to one of `next`'s attributes that is not constant, each target once.
pub(crate) fn renames_ok(
    next: &BlockDefinition,
    renames: &BTreeMap<String, String>,
) -> Result<(), Stop> {
    let mut targets = HashSet::new();
    for (from, to) in renames {
        let fits = next
            .attributes
            .iter()
            .any(|a| &a.tag == to && !a.options.constant);
        if !fits || from == to || !targets.insert(to) {
            return Err(Stop::failed(error(
                codes::UNKNOWN_TAG,
                format!(
                    "“{from}” → “{to}”: yeni etiket bloğun sabit olmayan bir özniteliği olmalı ve bir kez verilmeli."
                ),
                Some(format!("renames.{from}")),
            )));
        }
    }
    Ok(())
}

/// Where an insert's movable attribute `tag` stands for a point `at` of the drawing: the point in the definition's
/// coordinates (the inverse of its placement); none when the placement cannot be undone (a zero scale).
pub(crate) fn definition_point(
    block: &BlockDefinition,
    insert: &InsertEntity,
    at: Vec2,
) -> Option<Vec2> {
    use kentos_geometry_core::block::placement_xy;
    use kentos_geometry_core::geom::affine::determinant;
    let m = placement_xy(
        kentos_geometry_core::Vec2::new(insert.p.x, insert.p.y),
        insert.scale,
        insert.y_scale.unwrap_or(insert.scale),
        insert.rotation,
        insert.mirror,
    );
    let det = determinant(&m);
    if !(det.is_finite() && det != 0.0) {
        return None;
    }
    // The placement's inverse at `at`: the base-relative point, then the base point added back.
    let (dx, dy) = (at.x - m[4], at.y - m[5]);
    let x = (m[3] * dx - m[2] * dy) / det;
    let y = (m[0] * dy - m[1] * dx) / det;
    Some(Vec2 {
        x: x + block.base.x,
        y: y + block.base.y,
    })
}

/// The ids of the blocks a `fields` run fills: the one given, else every block with a field.
pub(crate) fn field_blocks(doc: &Document, block: Option<BlockId>) -> Vec<BlockId> {
    match block {
        Some(id) => vec![id],
        None => doc
            .blocks()
            .iter()
            .filter(|b| b.attributes.iter().any(|a| a.options.field.is_some()))
            .map(|b| b.id)
            .collect(),
    }
}
