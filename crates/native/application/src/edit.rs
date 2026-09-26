//! `cad.entities.edit` v1 (docs/adr/0047): objects named by their
//! persistent ids given a new geometry, replaced in their place, followed by
//! new objects made from them, or deleted, as one undo step named after the
//! modify tool. The desktop's handler over the native document; the web's
//! is `apps/web/src/product/entitiesEdit.ts`. Both pass the shared cases in
//! `fixtures/commands/v1/cad.entities.edit.json`.
//!
//! The edge, corner and object tools (Ötele, Buda, Uzat, Köşe yuvarla, Pah,
//! Kır, Birleştir, Patlat, Uzat-kısalt, Köşe ekle/sil) compute the geometry
//! with the shared core and write it here (TODOS.md CMD-07); nothing is
//! computed in this module.
//!
//! The checks, in order (the first that fails answers):
//! 1. at least one change; every change's id lowercase UUID text with
//!    hyphens (in order);
//! 2. every geometry, in order: enough points for its kind, every number
//!    finite, a circle's or an arc's radius above zero;
//! 3. the expected revision (every command's, `checks.rs`);
//! 4. every id names an object of the document (in order);
//! 5. no object changed twice;
//! 6. no object on a locked layer: an edit is written whole or not at all.

use std::collections::HashSet;

use kentos_contracts::{
    CommandError, CommandResult, EditOperation, EntitiesEdit, EntitiesEditPlan, EntitiesEdited,
    Entity, EntityBase, EntityEdit, EntityGeometry, Vec2,
};
use kentos_domain::{Document, Slot, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
/// The stable codes of the answers (`CommandError.code`, `CommandWarning.code`).
pub use crate::codes;
use crate::geometry::entity_of;

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now: the objects as they would be, and the
/// revision to expect for exactly that; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &EntitiesEdit) -> CommandResult<EntitiesEditPlan> {
    match check(cx.doc, input) {
        Ok(checked) => CommandResult::Completed {
            output: EntitiesEditPlan {
                changed: checked.changed.into_iter().map(|c| c.entity).collect(),
                created: checked.created,
                removed: checked.removed.into_iter().map(|(_, uid)| uid).collect(),
                revision: cx.doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes it as one undo step named after the
/// tool: into the open transaction or group, if one is. Nothing is written
/// when the document has no slot left for the new objects.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: EntitiesEdit,
) -> CommandResult<EntitiesEdited> {
    let checked = match check(cx.doc, &input) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    let label = label(input.operation);
    let changed_uids: Vec<String> = checked.changed.iter().map(|c| c.uid.clone()).collect();
    let removed_uids: Vec<String> = checked.removed.iter().map(|(_, uid)| uid.clone()).collect();
    let written = cx.doc.transact(label, |doc| {
        let gone: Vec<Slot> = checked.removed.iter().map(|(slot, _)| *slot).collect();
        doc.remove(&gone);
        let changes = checked
            .changed
            .into_iter()
            .map(|c| (c.slot, c.entity))
            .collect();
        doc.update_many(changes, label);
        doc.add_many(checked.created, label)
    });
    let slots = match written {
        Ok(slots) => slots,
        Err(full) => {
            return CommandResult::Failed {
                error: CommandError {
                    code: codes::SLOTS_EXHAUSTED.into(),
                    message: full.to_string(),
                    path: None,
                    revision: None,
                },
            };
        }
    };
    let doc = &*cx.doc;
    let created = slots
        .iter()
        .filter_map(|slot| doc.uid(*slot))
        .map(|uid| uid.to_string())
        .collect();
    CommandResult::Completed {
        output: EntitiesEdited {
            changed: changed_uids,
            created,
            removed: removed_uids,
            revision: doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}

/// The undo step's name: the tool's (docs/adr/0047).
pub fn label(operation: EditOperation) -> &'static str {
    match operation {
        EditOperation::Offset => "Ötele",
        EditOperation::Trim => "Buda",
        EditOperation::Extend => "Uzat",
        EditOperation::Fillet => "Köşe yuvarla",
        EditOperation::Chamfer => "Pah",
        EditOperation::Break => "Kır",
        EditOperation::Join => "Birleştir",
        EditOperation::Explode => "Patlat",
        EditOperation::Lengthen => "Uzat-kısalt",
        EditOperation::VertexAdd => "Köşe ekle",
        EditOperation::VertexRemove => "Köşe sil",
        EditOperation::Stretch => "Esnet",
    }
}

/// An object changed in place: its slot, its id and itself as it will be.
struct Changed {
    slot: Slot,
    uid: String,
    entity: Entity,
}

/// What may be written: the objects changed in place, the new ones (slot 0)
/// and the ones to delete, each in the input's order.
struct Checked {
    changed: Vec<Changed>,
    created: Vec<Entity>,
    removed: Vec<(Slot, String)>,
}

/// The id a change names and the field that holds it: `uid`, or `from` for an `add`.
fn named(change: &EntityEdit) -> (&str, &'static str) {
    match change {
        EntityEdit::Update { uid, .. }
        | EntityEdit::Replace { uid, .. }
        | EntityEdit::Remove { uid } => (uid, "uid"),
        EntityEdit::Add { from, .. } => (from, "from"),
    }
}

fn geometry_of(change: &EntityEdit) -> Option<&EntityGeometry> {
    match change {
        EntityEdit::Update { geometry, .. }
        | EntityEdit::Replace { geometry, .. }
        | EntityEdit::Add { geometry, .. } => Some(geometry),
        EntityEdit::Remove { .. } => None,
    }
}

/// The checks in the contract's order.
fn check(doc: &Document, input: &EntitiesEdit) -> Result<Checked, Stop> {
    if input.changes.is_empty() {
        return Err(Stop::Failed(error(
            codes::NO_CHANGES,
            "Yapılacak değişiklik verilmedi. En az bir değişiklik verin.".into(),
            Some("changes".into()),
        )));
    }
    for (i, change) in input.changes.iter().enumerate() {
        let (uid, field) = named(change);
        if !checks::is_uid_text(uid) {
            return Err(Stop::Failed(error(
                codes::INVALID_UID,
                format!(
                    "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın."
                ),
                Some(format!("changes[{i}].{field}")),
            )));
        }
    }
    for (i, change) in input.changes.iter().enumerate() {
        if let Some(g) = geometry_of(change) {
            check_geometry(g, i)?;
        }
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    // Every id names an object, in order.
    let mut found: Vec<(Slot, &Entity)> = Vec::with_capacity(input.changes.len());
    for (i, change) in input.changes.iter().enumerate() {
        let (uid, field) = named(change);
        let at = Uuid::parse_str(uid)
            .ok()
            .and_then(|u| doc.slot_of(u))
            .and_then(|slot| Some((slot, doc.get(slot)?)));
        let Some(at) = at else {
            return Err(Stop::Failed(error(
                codes::ENTITY_NOT_FOUND,
                format!(
                    "“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
                ),
                Some(format!("changes[{i}].{field}")),
            )));
        };
        found.push(at);
    }
    // An object changes once: an `add` may come from one that changes.
    let mut targets = HashSet::new();
    for (i, change) in input.changes.iter().enumerate() {
        if matches!(change, EntityEdit::Add { .. }) {
            continue;
        }
        if !targets.insert(found[i].0) {
            let (uid, _) = named(change);
            return Err(Stop::Failed(error(
                codes::REPEATED_ENTITY,
                format!(
                    "“{uid}” kimlikli nesne birden çok değişiklikte değişiyor. Bir nesneye tek değişiklik verin."
                ),
                Some(format!("changes[{i}].uid")),
            )));
        }
    }
    // Nothing on a locked layer is changed or copied from.
    let layers = doc.layers();
    for (i, change) in input.changes.iter().enumerate() {
        let layer = &found[i].1.base().layer_id;
        if layers.is_locked(layer) {
            let name = layers
                .get(layer)
                .map_or(layer.as_str(), |n| n.name.as_str());
            let (_, field) = named(change);
            return Err(Stop::Failed(error(
                codes::LAYER_LOCKED,
                format!(
                    "“{name}” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın."
                ),
                Some(format!("changes[{i}].{field}")),
            )));
        }
    }
    let mut checked = Checked {
        changed: Vec::new(),
        created: Vec::new(),
        removed: Vec::new(),
    };
    for (change, (slot, entity)) in input.changes.iter().zip(found) {
        let base = entity.base();
        match change {
            EntityEdit::Update { uid, geometry } => checked.changed.push(Changed {
                slot,
                uid: uid.clone(),
                entity: entity_of(geometry, base.clone()),
            }),
            EntityEdit::Replace {
                uid,
                geometry,
                keep_data,
            } => checked.changed.push(Changed {
                slot,
                uid: uid.clone(),
                entity: entity_of(
                    geometry,
                    inherited(base, slot.0, keep_data.unwrap_or(false)),
                ),
            }),
            EntityEdit::Add {
                geometry,
                keep_data,
                ..
            } => checked.created.push(entity_of(
                geometry,
                inherited(base, 0, keep_data.unwrap_or(false)),
            )),
            EntityEdit::Remove { uid } => checked.removed.push((slot, uid.clone())),
        }
    }
    Ok(checked)
}

/// What a replacement or a new piece takes from its object: the layer and
/// the colour, the attributes and the label with `keep_data`; not the
/// symbol (the web's `EdgePickTool.inherit`). `id` is its slot (0 for a new one).
fn inherited(base: &EntityBase, id: u32, keep_data: bool) -> EntityBase {
    EntityBase {
        id,
        layer_id: base.layer_id.clone(),
        color: base.color.clone(),
        attrs: if keep_data {
            base.attrs.clone()
        } else {
            Default::default()
        },
        label: if keep_data { base.label.clone() } else { None },
        symbol: None,
    }
}

/// One change's geometry: enough points for its kind, every number finite,
/// a positive radius.
fn check_geometry(g: &EntityGeometry, i: usize) -> Result<(), Stop> {
    let at = |field: &str| Some(format!("changes[{i}].geometry{field}"));
    match g {
        EntityGeometry::Polyline { pts, .. } if pts.len() < 2 => {
            return Err(Stop::Failed(error(
                codes::TOO_FEW_POINTS,
                format!(
                    "Çoklu çizginin en az 2 noktası olmalı; {} nokta verildi. Eksik noktaları ekleyin.",
                    pts.len()
                ),
                at(".pts"),
            )));
        }
        EntityGeometry::Polygon { pts, holes, .. } => {
            if pts.len() < 3 {
                return Err(Stop::Failed(error(
                    codes::TOO_FEW_CORNERS,
                    format!(
                        "Kapalı alanın en az 3 köşesi olmalı; {} köşe verildi. Eksik köşeleri ekleyin.",
                        pts.len()
                    ),
                    at(".pts"),
                )));
            }
            for (h, ring) in holes.iter().flatten().enumerate() {
                if ring.pts.len() < 3 {
                    return Err(Stop::Failed(error(
                        codes::TOO_FEW_CORNERS,
                        format!(
                            "{}. deliğin en az 3 köşesi olmalı; {} köşe verildi. Eksik köşeleri ekleyin ya da deliği çıkarın.",
                            h + 1,
                            ring.pts.len()
                        ),
                        at(&format!(".holes[{h}].pts")),
                    )));
                }
            }
        }
        _ => {}
    }
    if !finite(g) {
        return Err(Stop::Failed(error(
            codes::NOT_FINITE,
            format!(
                "{}. değişikliğin geometrisinde sonlu olmayan bir değer var (NaN ya da sonsuz). Geometriyi sonlu sayılarla verin.",
                i + 1
            ),
            at(""),
        )));
    }
    if let EntityGeometry::Circle { r, .. } | EntityGeometry::Arc { r, .. } = g
        && *r <= 0.0
    {
        return Err(Stop::Failed(error(
            codes::INVALID_RADIUS,
            "Yarıçap sıfırdan büyük olmalı. Pozitif bir yarıçap verin.".into(),
            at(".r"),
        )));
    }
    Ok(())
}

/// Every number of a geometry is finite.
fn finite(g: &EntityGeometry) -> bool {
    fn pt(p: &Vec2) -> bool {
        p.x.is_finite() && p.y.is_finite()
    }
    fn pts(ps: &[Vec2]) -> bool {
        ps.iter().all(pt)
    }
    fn values(vs: &Option<Vec<f64>>) -> bool {
        vs.iter().flatten().all(|v| v.is_finite())
    }
    match g {
        EntityGeometry::Point { p, z } => pt(p) && z.is_none_or(f64::is_finite),
        EntityGeometry::Line { a, b } => pt(a) && pt(b),
        EntityGeometry::Polyline { pts: p, bulges } => pts(p) && values(bulges),
        EntityGeometry::Polygon {
            pts: p,
            bulges,
            holes,
        } => {
            pts(p)
                && values(bulges)
                && holes
                    .iter()
                    .flatten()
                    .all(|h| pts(&h.pts) && values(&h.bulges))
        }
        EntityGeometry::Circle { c, r } => pt(c) && r.is_finite(),
        EntityGeometry::Arc { c, r, a0, a1 } => {
            pt(c) && r.is_finite() && a0.is_finite() && a1.is_finite()
        }
        EntityGeometry::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => pt(c) && pt(major) && ratio.is_finite() && t0.is_finite() && t1.is_finite(),
        EntityGeometry::Spline { pts: p, .. } => pts(p),
        EntityGeometry::Xline { p, dir } | EntityGeometry::Ray { p, dir } => pt(p) && pt(dir),
        EntityGeometry::Text {
            p,
            height,
            rotation,
            ..
        } => pt(p) && height.is_finite() && rotation.is_finite(),
    }
}
