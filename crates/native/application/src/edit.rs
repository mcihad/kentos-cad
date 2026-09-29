//! `cad.entities.edit` v1 (docs/adr/0047): objects named by their
//! persistent ids given a new geometry, replaced in their place, followed by
//! new objects made from them, or deleted, as one undo step named after the
//! modify tool. The desktop's handler over the native document; the web's
//! is `apps/web/src/product/entitiesEdit.ts`. Both pass the shared cases in
//! `fixtures/commands/v1/cad.entities.edit.json`.
//!
//! The edge, corner and object tools (Ötele, Buda, Uzat, Köşe yuvarla, Pah,
//! Kır, Birleştir, Patlat, Uzat-kısalt, Köşe ekle/sil) and Esnet compute the
//! geometry with the shared core and write it here (TODOS.md CMD-07);
//! nothing is computed in this module. Öznitelikler's geometry rows and the
//! text field over the drawing write the value typed (operation
//! `properties`, docs/adr/0066).
//!
//! The checks, in order (the first that fails answers):
//! 1. at least one change; every change's id lowercase UUID text with
//!    hyphens (in order);
//! 2. every geometry, in order: enough points for its kind, a text that is
//!    not blank, every number finite, a circle's or an arc's radius above zero;
//! 3. the expected revision (every command's, `checks.rs`);
//! 4. every id names an object of the document (in order);
//! 5. no object changed twice;
//! 6. no object on a locked layer: an edit is written whole or not at all.

use std::collections::HashSet;

use kentos_contracts::{
    CommandError, CommandResult, CommandWarning, EditOperation, EntitiesEdit, EntitiesEditPlan,
    EntitiesEdited, Entity, EntityBase, EntityEdit, EntityGeometry, RingGeometry, Vec2,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::ops::elevation::{Carry, Elevated};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error, is_blank};
/// The stable codes of the answers (`CommandError.code`, `CommandWarning.code`).
pub use crate::codes;
use crate::elevation;
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
            warnings: checked.warnings,
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
    let warnings = checked.warnings;
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
        warnings,
    }
}

/// The undo step's name: the tool's (docs/adr/0047); Öznitelikler's is the document's own “Değiştir”.
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
        // Öznitelikler and the text field over the drawing: the document's own “Değiştir” (docs/adr/0066).
        EditOperation::Properties => "Değiştir",
        // The area tools (docs/adr/0065, 0069).
        EditOperation::AreaUnion => "Alan birleştir",
        EditOperation::AreaIntersect => "Alan kesiştir",
        EditOperation::AreaSubtract => "Alan çıkar",
        EditOperation::AreaSplit => "Alan böl",
        EditOperation::ToArea => "Alana çevir",
        EditOperation::ToPolyline => "Çizgiye çevir",
        // The grips and their menu (docs/adr/0068; the web's dd39864).
        EditOperation::Grip => "Tutamaçla düzenle",
        EditOperation::StraightEdge => "Düz kenar yap",
        EditOperation::ArcEdge => "Yaya dönüştür",
        // The drawing and editing tools of docs/adr/0140.
        EditOperation::Split => "Parçala",
        EditOperation::Reverse => "Yönü çevir",
        EditOperation::Simplify => "Sadeleştir",
        EditOperation::Cleanup => "Çizimi temizle",
        // Kot ver (docs/adr/0142).
        EditOperation::Elevation => "Kot ver",
        EditOperation::PartsJoin => "Parçaları birleştir",
        EditOperation::PartsSplit => "Parçalara ayır",
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
    /// That elevations were lost (docs/adr/0142).
    warnings: Vec<CommandWarning>,
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
            check_geometry(g, "changes", i, "değişikliğin")?;
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
        warnings: Vec::new(),
    };
    // The elevations of the objects the edit names, for what it writes
    // (docs/adr/0142); nothing to carry when none has one.
    let sources: Vec<Elevated> = found
        .iter()
        .flat_map(|(_, e)| elevation::paths(e))
        .filter(|p| elevation::elevated(std::slice::from_ref(p)))
        .collect();
    let how = if input.operation == EditOperation::Offset {
        Carry::Offset
    } else {
        Carry::Along
    };
    // The object keeps its own vertices, moved (a grip, Esnet, a typed
    // coordinate), or Ötele's copy has one for each of its source's: a
    // vertex takes the elevation of the one in its place. Any other edit
    // cuts or reshapes, and its vertices take theirs by where they lie.
    let by_place = matches!(
        input.operation,
        EditOperation::Grip
            | EditOperation::Stretch
            | EditOperation::Properties
            | EditOperation::Offset
    );
    let mut lost = 0;
    // Elevations written with the geometry are written as they are (Kot ver,
    // Öznitelikler, a script): `entity_of` already holds them.
    let mut elevate = |mut entity: Entity, from: &Entity, geometry: &EntityGeometry| -> Entity {
        if written(geometry) || sources.is_empty() {
            return entity;
        }
        let before = elevation::paths(from);
        let same = if by_place { before.as_slice() } else { &[] };
        if !elevation::carry(&mut entity, &sources, same, how) && elevation::elevated(&before) {
            lost += 1;
        }
        entity
    };
    for (change, (slot, entity)) in input.changes.iter().zip(found) {
        let base = entity.base();
        match change {
            EntityEdit::Update { uid, geometry } => checked.changed.push(Changed {
                slot,
                uid: uid.clone(),
                entity: elevate(entity_of(geometry, base.clone()), entity, geometry),
            }),
            EntityEdit::Replace {
                uid,
                geometry,
                keep_data,
            } => checked.changed.push(Changed {
                slot,
                uid: uid.clone(),
                entity: elevate(
                    entity_of(
                        geometry,
                        inherited(base, slot.0, keep_data.unwrap_or(false)),
                    ),
                    entity,
                    geometry,
                ),
            }),
            EntityEdit::Add {
                geometry,
                keep_data,
                ..
            } => checked.created.push(elevate(
                entity_of(geometry, inherited(base, 0, keep_data.unwrap_or(false))),
                entity,
                geometry,
            )),
            EntityEdit::Remove { uid } => checked.removed.push((slot, uid.clone())),
        }
    }
    if lost > 0 {
        checked.warnings.push(CommandWarning {
            code: codes::ELEVATION_LOST.into(),
            message: format!("{lost} nesnenin kotu bu işlemde korunmadı."),
            path: Some("changes".into()),
        });
    }
    Ok(checked)
}

/// What a replacement or a new piece takes from its object: the layer, the
/// colour and its own line weight (a trimmed 0.70 mm line stays 0.70 mm,
/// docs/adr/0139), the attributes and the label with `keep_data`; not the
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
        line_weight: base.line_weight,
    }
}

/// Whether a closed area's ring (or hole) encloses anything: 3 corners, or 2
/// when one of its two edges is an arc (a circle made an area, a lens, a
/// half-disc; docs/adr/0069). −0 is straight; NaN is an arc here and is
/// refused as not finite next.
fn ring_closes(corners: usize, bulges: Option<&[f64]>) -> bool {
    let bulge = |i: usize| bulges.and_then(|b| b.get(i)).copied().unwrap_or(0.0);
    corners >= 3 || (corners == 2 && (bulge(0) != 0.0 || bulge(1) != 0.0))
}

/// The `i`-th geometry of the input's `list` (`changes`; `objects` of
/// `cad.entities.create`): enough points for its kind, a text that is not
/// empty or only white space, every number finite, a positive radius.
/// `whose` names it in a message: “değişikliğin”.
pub(crate) fn check_geometry(
    g: &EntityGeometry,
    list: &str,
    i: usize,
    whose: &str,
) -> Result<(), Stop> {
    let at = |field: &str| Some(format!("{list}[{i}].geometry{field}"));
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
        EntityGeometry::Polygon {
            pts,
            bulges,
            holes,
            parts,
            ..
        } => {
            if !ring_closes(pts.len(), bulges.as_deref()) {
                return Err(Stop::Failed(error(
                    codes::TOO_FEW_CORNERS,
                    format!(
                        "Kapalı alanın en az 3 köşesi olmalı (kenarlarından biri yaysa 2); {} köşe verildi. Eksik köşeleri ekleyin.",
                        pts.len()
                    ),
                    at(".pts"),
                )));
            }
            for (h, ring) in holes.iter().flatten().enumerate() {
                if !ring_closes(ring.pts.len(), ring.bulges.as_deref()) {
                    return Err(Stop::Failed(error(
                        codes::TOO_FEW_CORNERS,
                        format!(
                            "{}. deliğin en az 3 köşesi olmalı (kenarlarından biri yaysa 2); {} köşe verildi. Eksik köşeleri ekleyin ya da deliği çıkarın.",
                            h + 1,
                            ring.pts.len()
                        ),
                        at(&format!(".holes[{h}].pts")),
                    )));
                }
            }
            // A multi-part area's other parts close as its own ring does (docs/adr/0143).
            for (k, part) in parts.iter().flatten().enumerate() {
                if !ring_closes(part.pts.len(), part.bulges.as_deref()) {
                    return Err(Stop::Failed(error(
                        codes::TOO_FEW_CORNERS,
                        format!(
                            "{}. parçanın en az 3 köşesi olmalı (kenarlarından biri yaysa 2); {} köşe verildi. Eksik köşeleri ekleyin ya da parçayı çıkarın.",
                            k + 2,
                            part.pts.len()
                        ),
                        at(&format!(".parts[{k}].pts")),
                    )));
                }
                for (h, ring) in part.holes.iter().flatten().enumerate() {
                    if !ring_closes(ring.pts.len(), ring.bulges.as_deref()) {
                        return Err(Stop::Failed(error(
                            codes::TOO_FEW_CORNERS,
                            format!(
                                "{}. parçanın {}. deliğinin en az 3 köşesi olmalı (kenarlarından biri yaysa 2); {} köşe verildi. Eksik köşeleri ekleyin ya da deliği çıkarın.",
                                k + 2,
                                h + 1,
                                ring.pts.len()
                            ),
                            at(&format!(".parts[{k}].holes[{h}].pts")),
                        )));
                    }
                }
            }
        }
        EntityGeometry::Hatch { ring, holes, .. } => {
            if ring.len() < 3 {
                return Err(Stop::Failed(error(
                    codes::TOO_FEW_CORNERS,
                    format!(
                        "Taramanın en az 3 köşesi olmalı; {} köşe verildi. Eksik köşeleri ekleyin.",
                        ring.len()
                    ),
                    at(".ring"),
                )));
            }
            for (h, hole) in holes.iter().flatten().enumerate() {
                if hole.len() < 3 {
                    return Err(Stop::Failed(error(
                        codes::TOO_FEW_CORNERS,
                        format!(
                            "{}. deliğin en az 3 köşesi olmalı; {} köşe verildi. Eksik köşeleri ekleyin ya da deliği çıkarın.",
                            h + 1,
                            hole.len()
                        ),
                        at(&format!(".holes[{h}]")),
                    )));
                }
            }
        }
        // A dimension's text is not checked: an empty one shows the measured value.
        EntityGeometry::Text { text, .. } if is_blank(text) => {
            return Err(Stop::Failed(error(
                codes::EMPTY_TEXT,
                "Yazının metni boş olamaz; yalnız boşluktan oluşan metin de boştur. Yazıya bir metin verin.".into(),
                at(".text"),
            )));
        }
        _ => {}
    }
    // Elevations as written (docs/adr/0142): one for each vertex.
    for (zs, n, path) in written_elevations(g) {
        if zs.len() != n {
            return Err(Stop::Failed(error(
                codes::INVALID_ELEVATIONS,
                format!(
                    "Kotların sayısı köşelerin sayısıyla aynı olmalı; {n} köşeye {} kot verildi. Her köşeye bir kot verin; kotsuz köşeye null.",
                    zs.len()
                ),
                at(&path),
            )));
        }
    }
    if !finite(g) {
        return Err(Stop::Failed(error(
            codes::NOT_FINITE,
            format!(
                "{}. {whose} geometrisinde sonlu olmayan bir değer var (NaN ya da sonsuz). Geometriyi sonlu sayılarla verin.",
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

/// Whether the geometry carries its elevations (docs/adr/0142): a
/// multi-part area's when its own or a part's list is given (docs/adr/0143).
fn written(g: &EntityGeometry) -> bool {
    match g {
        EntityGeometry::Line { zs, .. } | EntityGeometry::Polyline { zs, .. } => zs.is_some(),
        EntityGeometry::Polygon { zs, parts, .. } => {
            zs.is_some() || parts.iter().flatten().any(|p| p.zs.is_some())
        }
        _ => false,
    }
}

/// A geometry's written elevations (docs/adr/0142): each list with its
/// vertex count and its path.
fn written_elevations(g: &EntityGeometry) -> Vec<(&[Option<f64>], usize, String)> {
    let mut out: Vec<(&[Option<f64>], usize, String)> = Vec::new();
    match g {
        EntityGeometry::Line { zs: Some(zs), .. } => out.push((zs, 2, ".zs".into())),
        EntityGeometry::Polyline {
            pts, zs: Some(zs), ..
        } => out.push((zs, pts.len(), ".zs".into())),
        EntityGeometry::Polygon {
            pts,
            zs,
            holes,
            parts,
            ..
        } => {
            if let Some(zs) = zs {
                out.push((zs, pts.len(), ".zs".into()));
            }
            for (h, ring) in holes.iter().flatten().enumerate() {
                if let Some(zs) = &ring.zs {
                    out.push((zs, ring.pts.len(), format!(".holes[{h}].zs")));
                }
            }
            // A multi-part area's other parts (docs/adr/0143).
            for (k, part) in parts.iter().flatten().enumerate() {
                if let Some(zs) = &part.zs {
                    out.push((zs, part.pts.len(), format!(".parts[{k}].zs")));
                }
                for (h, ring) in part.holes.iter().flatten().enumerate() {
                    if let Some(zs) = &ring.zs {
                        out.push((zs, ring.pts.len(), format!(".parts[{k}].holes[{h}].zs")));
                    }
                }
            }
        }
        _ => {}
    }
    out
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
    fn heights(zs: &Option<Vec<Option<f64>>>) -> bool {
        zs.iter().flatten().flatten().all(|z| z.is_finite())
    }
    match g {
        EntityGeometry::Point { p, z } => pt(p) && z.is_none_or(f64::is_finite),
        EntityGeometry::Line { a, b, zs } => pt(a) && pt(b) && heights(zs),
        EntityGeometry::Polyline { pts: p, bulges, zs } => pts(p) && values(bulges) && heights(zs),
        EntityGeometry::Polygon {
            pts: p,
            bulges,
            holes,
            zs,
            parts,
        } => {
            let holes_ok = |holes: &Option<Vec<RingGeometry>>| {
                holes
                    .iter()
                    .flatten()
                    .all(|h| pts(&h.pts) && values(&h.bulges) && heights(&h.zs))
            };
            pts(p)
                && values(bulges)
                && heights(zs)
                && holes_ok(holes)
                && parts.iter().flatten().all(|q| {
                    pts(&q.pts) && values(&q.bulges) && heights(&q.zs) && holes_ok(&q.holes)
                })
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
        EntityGeometry::Dimension {
            a,
            b,
            offset,
            height,
            angle,
            c,
            ..
        } => {
            pt(a)
                && pt(b)
                && offset.is_finite()
                && height.is_finite()
                && angle.is_none_or(f64::is_finite)
                && c.as_ref().is_none_or(pt)
        }
        EntityGeometry::Hatch {
            ring,
            holes,
            pattern,
        } => {
            pts(ring)
                && holes.iter().flatten().all(|h| pts(h))
                && pattern.angle.is_finite()
                && pattern.spacing.is_finite()
        }
    }
}
