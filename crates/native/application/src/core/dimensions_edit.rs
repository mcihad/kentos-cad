//! `cad.dimensions.edit` v1 (docs/adr/0321 §4): dimensions spaced from a base (Ölçü aralığı, AutoCAD's DIMSPACE),
//! broken where other objects cross their lines (Ölçüyü kır, DIMBREAK) or made whole again, their value moved from its
//! place, a value written over theirs (`<>` the measured one) and their options, each as one undo step named after the
//! edit. The web is paused (docs/adr/0228): the desktop's handler alone, passing
//! `fixtures/commands/v1/cad.dimensions.edit.json`.
//!
//! The geometry is the core's: a straight dimension's line is its first point moved `offset` along its direction's
//! left normal (`geom::dimension`), a break's place the fraction of one of its three lines (the dimension line, and the
//! extension lines from the measured points to it) where an object's edge crosses it (`intersect_edges`).

use kentos_commands::{
    BreakLine, CommandResult, CommandWarning, DimensionBreak, DimensionEdit, DimensionEntity, DimensionStyle,
    DimensionsEdit, DimensionsEditPlan, DimensionsEdited, Entity, TextShift,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::entity::dimension_geom;
use kentos_geometry_core::geom::dimension::layout_dimension;
use kentos_geometry_core::geom::intersect::{Edge, intersect_edges};
use kentos_geometry_core::ops::edges::entity_edges;
use kentos_geometry_core::vec2::Vec2;

use crate::ExecutionContext;
use crate::blocks::core_entity;
use crate::checks::{self, Stop, error, is_uid_text};
pub use crate::codes;

/// The most breaks a dimension keeps (the contract's).
const MOST: usize = kentos_commands::MAX_BREAKS;

fn fail(code: &str, message: String, path: String) -> Stop {
    Stop::failed(error(code, message, Some(path)))
}

/// The undo step's name: the edit's.
fn label(e: &DimensionEdit) -> &'static str {
    match e {
        DimensionEdit::Space { .. } => "Ölçü aralığı",
        DimensionEdit::Break { .. } => "Ölçüyü kır",
        DimensionEdit::Unbreak { .. } => "Kırmayı kaldır",
        DimensionEdit::MoveText { .. } => "Ölçü yazısını taşı",
        DimensionEdit::Text { .. } => "Ölçü değerini yaz",
        DimensionEdit::Options { .. } => "Ölçü seçenekleri",
        DimensionEdit::Level { .. } => "Kotun katı",
    }
}

/// The dimensions the edit names, with their paths.
fn named(e: &DimensionEdit) -> Vec<(&String, String)> {
    fn list(uids: &[String]) -> Vec<(&String, String)> {
        uids.iter()
            .enumerate()
            .map(|(i, u)| (u, format!("edit.uids[{i}]")))
            .collect()
    }
    match e {
        DimensionEdit::Space { uids, .. }
        | DimensionEdit::Break { uids, .. }
        | DimensionEdit::Unbreak { uids }
        | DimensionEdit::Text { uids, .. }
        | DimensionEdit::Options { uids, .. }
        | DimensionEdit::Level { uids, .. } => list(uids),
        DimensionEdit::MoveText { uid, .. } => vec![(uid, "edit.uid".into())],
    }
}

/// A straight dimension's direction (unit), its line's left normal and a point of its line; none for any other.
fn frame(d: &DimensionEntity) -> Option<(Vec2, Vec2, Vec2)> {
    let a = Vec2::new(d.a.x, d.a.y);
    let u = match d.style {
        None | Some(DimensionStyle::Aligned) => {
            let (dx, dy) = (d.b.x - d.a.x, d.b.y - d.a.y);
            let l = kentos_geometry_core::jsmath::js_hypot(dx, dy);
            if l < 1e-9 {
                return None;
            }
            Vec2::new(dx / l, dy / l)
        }
        Some(DimensionStyle::Linear) => {
            let t = d.angle.unwrap_or(0.0) * kentos_geometry_core::jsmath::PI / 180.0;
            Vec2::new(kentos_geometry_core::jsmath::cos(t), kentos_geometry_core::jsmath::sin(t))
        }
        _ => return None,
    };
    let n = Vec2::new(-u.y, u.x);
    Some((u, n, Vec2::new(a.x + n.x * d.offset, a.y + n.y * d.offset)))
}

fn dot(a: Vec2, b: Vec2) -> f64 {
    a.x * b.x + a.y * b.y
}

/// What the checks give: each dimension changed, as it would be, and the warnings.
struct Checked {
    changed: Vec<(Slot, Uuid, DimensionEntity)>,
    warnings: Vec<CommandWarning>,
}

fn check(doc: &Document, input: &DimensionsEdit) -> Result<Checked, Stop> {
    let e = &input.edit;
    let bad_uid = |uid: &str, path: String| {
        fail(
            codes::INVALID_UID,
            format!(
                "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
            ),
            path,
        )
    };
    if let DimensionEdit::Space { base, .. } = e
        && !is_uid_text(base)
    {
        return Err(bad_uid(base, "edit.base".into()));
    }
    let names = named(e);
    for (u, path) in &names {
        if !is_uid_text(u) {
            return Err(bad_uid(u, path.clone()));
        }
    }
    if let DimensionEdit::Break {
        objects: Some(objects),
        ..
    } = e
    {
        for (i, u) in objects.iter().enumerate() {
            if !is_uid_text(u) {
                return Err(bad_uid(u, format!("edit.objects[{i}]")));
            }
        }
    }
    if names.is_empty() {
        return Err(fail(
            codes::NO_ENTITIES,
            "Düzenlenecek ölçü yok. En az bir ölçünün kimliğini verin.".into(),
            "edit.uids".into(),
        ));
    }
    match e {
        DimensionEdit::Space {
            spacing: Some(s), ..
        } => {
            checks::finite(*s, "Aralık", "Aralığı metre olarak sonlu bir sayıyla verin.", "edit.spacing")?;
            if *s < 0.0 {
                return Err(fail(
                    codes::INVALID_OFFSET,
                    format!("Aralık {s}; 0 ya da büyük olmalı (0 ölçüleri temelin çizgisine dizer)."),
                    "edit.spacing".into(),
                ));
            }
        }
        DimensionEdit::MoveText { along, across, .. } => {
            checks::finite(*along, "Kayma", "Kaymayı metre olarak sonlu bir sayıyla verin.", "edit.along")?;
            checks::finite(*across, "Kayma", "Kaymayı metre olarak sonlu bir sayıyla verin.", "edit.across")?;
        }
        DimensionEdit::Text { text: Some(t), .. } if t.trim().is_empty() => {
            return Err(fail(
                codes::EMPTY_TEXT,
                "Yazılan değer boş; ölçülen değere dönmek için null verin.".into(),
                "edit.text".into(),
            ));
        }
        DimensionEdit::Options {
            options: Some(o), ..
        } => {
            if let Some(words) = o.problem() {
                return Err(fail(codes::INVALID_STYLE, words, "edit.options".into()));
            }
        }
        DimensionEdit::Level { level: Some(l), .. } => {
            if let Some(words) = l.problem() {
                return Err(fail(codes::INVALID_DIMENSION, words, "edit.level".into()));
            }
        }
        _ => {}
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let find = |uid: &str, path: &str| -> Result<(Slot, Uuid), Stop> {
        Uuid::parse_str(uid)
            .ok()
            .and_then(|u| doc.slot_of(u).map(|s| (s, u)))
            .ok_or_else(|| {
                fail(
                    codes::ENTITY_NOT_FOUND,
                    format!(
                        "“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
                    ),
                    path.into(),
                )
            })
    };
    let base = match e {
        DimensionEdit::Space { base, .. } => Some(find(base, "edit.base")?),
        _ => None,
    };
    let mut dims = Vec::with_capacity(names.len());
    for (u, path) in &names {
        dims.push((find(u, path)?, path.clone()));
    }
    let objects = match e {
        DimensionEdit::Break {
            objects: Some(objects),
            ..
        } => {
            let mut out = Vec::with_capacity(objects.len());
            for (i, u) in objects.iter().enumerate() {
                out.push(find(u, &format!("edit.objects[{i}]"))?.0);
            }
            Some(out)
        }
        _ => None,
    };
    let dimension = |slot: Slot, path: &str| -> Result<DimensionEntity, Stop> {
        match doc.get(slot) {
            Some(Entity::Dimension(d)) => Ok(d.clone()),
            _ => Err(fail(
                codes::NOT_A_DIMENSION,
                "Verilen nesne bir ölçü değil; bu düzenleme yalnız ölçüleri değiştirir.".into(),
                path.into(),
            )),
        }
    };
    let base_slot = base.map(|(s, _)| s);
    let base = match base {
        Some((slot, _)) => Some(dimension(slot, "edit.base")?),
        None => None,
    };
    let mut list = Vec::with_capacity(dims.len());
    for ((slot, uid), path) in &dims {
        let d = dimension(*slot, path)?;
        let layers = doc.layers();
        if layers.is_locked(&d.base.layer_id) {
            let name = layers
                .get(&d.base.layer_id)
                .map_or(d.base.layer_id.as_str(), |n| n.name.as_str());
            return Err(fail(
                codes::LAYER_LOCKED,
                format!("“{name}” katmanı kilitli; üzerindeki ölçü değişmez. Kilidi Katmanlar panelinden açın."),
                path.clone(),
            ));
        }
        list.push((*slot, *uid, path.clone(), d));
    }
    let straight = |d: &DimensionEntity, path: &str| -> Result<(Vec2, Vec2, Vec2), Stop> {
        frame(d).ok_or_else(|| {
            fail(
                codes::NOT_STRAIGHT,
                "Ölçü hizalı ya da doğrusal değil: aralık ve kırma yalnız hizalı ve doğrusal ölçülerde yapılır.".into(),
                path.into(),
            )
        })
    };
    let mut warnings = Vec::new();
    let mut changed = Vec::new();
    match e {
        DimensionEdit::Space { spacing, .. } => {
            let base = base.as_ref().map(|b| (b, straight(b, "edit.base"))).ok_or_else(|| {
                fail(codes::NOT_A_DIMENSION, "Temel ölçü yok.".into(), "edit.base".into())
            })?;
            let (b, frame0) = (base.0, base.1?);
            let (u0, n0, line0) = frame0;
            let pos0 = dot(line0, n0);
            let spacing = spacing.unwrap_or(crate::dimension_create::LEVEL_TEXT_HEIGHTS * b.height);
            let mut placed = Vec::new();
            for (i, (slot, uid, path, d)) in list.iter().enumerate() {
                if Some(*slot) == base_slot {
                    continue;
                }
                let (u, n, line) = straight(d, path)?;
                if (u.x * u0.y - u.y * u0.x).abs() > 1e-9 {
                    return Err(fail(
                        codes::NOT_PARALLEL,
                        "Ölçü temel ölçüye paralel değil; aralık yalnız paralel ölçülerde verilir.".into(),
                        path.clone(),
                    ));
                }
                let pos = dot(line, n0);
                placed.push((pos - pos0, *slot, *uid, d.clone(), n, i));
            }
            // Each side of the base in its order of distance (equal distances in the input's order).
            placed.sort_by(|x, y| x.0.abs().total_cmp(&y.0.abs()));
            let (mut above, mut below) = (0.0, 0.0);
            let mut spaced = Vec::new();
            for (from, slot, uid, d, n, i) in placed {
                let side = if from < 0.0 { -1.0 } else { 1.0 };
                let k = if side > 0.0 {
                    above += 1.0;
                    above
                } else {
                    below += 1.0;
                    below
                };
                let q = pos0 + side * k * spacing;
                let a = Vec2::new(d.a.x, d.a.y);
                let offset = (q - dot(a, n0)) / dot(n, n0);
                if offset != d.offset {
                    spaced.push((i, (slot, uid, DimensionEntity { offset, ..d })));
                }
            }
            // In the input's order.
            spaced.sort_by_key(|(i, _)| *i);
            changed.extend(spaced.into_iter().map(|(_, c)| c));
        }
        DimensionEdit::Break { .. } => {
            let candidates: Vec<Slot> = match &objects {
                Some(list) => list.clone(),
                None => doc
                    .entities()
                    .filter(|o| {
                        let l = &o.base().layer_id;
                        !matches!(o, Entity::Dimension(_) | Entity::Text(_))
                            && doc.layers().is_visible(l)
                            && !doc.layers().is_frozen(l)
                    })
                    .map(|o| Slot(o.base().id))
                    .collect(),
            };
            let edges: Vec<(Slot, Vec<Edge>)> = candidates
                .iter()
                .filter_map(|s| {
                    let o = doc.get(*s)?;
                    Some((*s, entity_edges(&core_entity(o).shape)))
                })
                .collect();
            for (slot, uid, path, d) in &list {
                straight(d, path)?;
                let breaks = crossings(d, *slot, &edges);
                if breaks.is_empty() {
                    warnings.push(CommandWarning {
                        code: codes::NO_CROSSING.into(),
                        message: "Ölçünün çizgilerini kesen nesne yok; kırma yapılmadı.".into(),
                        path: Some(path.clone()),
                    });
                }
                if breaks != d.breaks {
                    changed.push((*slot, *uid, DimensionEntity { breaks, ..d.clone() }));
                }
            }
        }
        DimensionEdit::Unbreak { .. } => {
            for (slot, uid, _, d) in &list {
                if !d.breaks.is_empty() {
                    changed.push((*slot, *uid, DimensionEntity { breaks: Vec::new(), ..d.clone() }));
                }
            }
        }
        DimensionEdit::MoveText { along, across, .. } => {
            let shift = (*along != 0.0 || *across != 0.0).then_some(TextShift {
                along: *along,
                across: *across,
            });
            for (slot, uid, _, d) in &list {
                if d.text_shift != shift {
                    changed.push((*slot, *uid, DimensionEntity { text_shift: shift, ..d.clone() }));
                }
            }
        }
        DimensionEdit::Text { text, .. } => {
            for (slot, uid, _, d) in &list {
                if d.text != *text {
                    changed.push((*slot, *uid, DimensionEntity { text: text.clone(), ..d.clone() }));
                }
            }
        }
        DimensionEdit::Options { options, .. } => {
            let options = options.clone().filter(|o| !o.is_empty()).map(Box::new);
            for (slot, uid, _, d) in &list {
                if d.look.options != options {
                    let mut next = d.clone();
                    next.look.options = options.clone();
                    changed.push((*slot, *uid, next));
                }
            }
        }
        // Kotun katı (docs/adr/0721 §2): bound, the value the level's now; set free, the value kept.
        DimensionEdit::Level { level, .. } => {
            for (_, _, path, d) in &list {
                if d.style != Some(kentos_commands::DimensionStyle::Elevation) {
                    return Err(fail(
                        codes::INVALID_DIMENSION,
                        "Ölçü kot değil: katı yalnız kot ölçüsü anar.".into(),
                        path.clone(),
                    ));
                }
            }
            let levels = doc.settings().arch.as_ref().map_or(&[][..], |a| a.levels.as_slice());
            let value = match level {
                Some(l) => Some(l.value(levels).ok_or_else(|| {
                    fail(
                        codes::LEVEL_NOT_FOUND,
                        format!("Projede “{}” kimlikli kat yok. Var olan bir katın kimliğini verin.", l.id),
                        "edit.level.id".into(),
                    )
                })?),
                None => None,
            };
            for (slot, uid, _, d) in &list {
                let mut next = d.clone();
                next.level = level.clone().map(Box::new);
                if let Some(z) = value {
                    next.za = Some(z);
                }
                if next != *d {
                    changed.push((*slot, *uid, next));
                }
            }
        }
    }
    Ok(Checked { changed, warnings })
}

/// Where the objects' edges cross a straight dimension's three lines: its breaks, by line and fraction (none on an
/// extension line within its gap from the measured point), at most [`MOST`].
fn crossings(d: &DimensionEntity, own: Slot, edges: &[(Slot, Vec<Edge>)]) -> Vec<DimensionBreak> {
    let e = core_entity(&Entity::Dimension(DimensionEntity {
        breaks: Vec::new(),
        text_shift: None,
        ..d.clone()
    }));
    let Some(l) = dimension_geom(&e.shape).and_then(|g| layout_dimension(&g)) else {
        return Vec::new();
    };
    let (a, b) = (Vec2::new(d.a.x, d.a.y), Vec2::new(d.b.x, d.b.y));
    let mut out = Vec::new();
    for (line, from, to) in [
        (BreakLine::Dimension, l.d1, l.d2),
        (BreakLine::First, a, l.d1),
        (BreakLine::Second, b, l.d2),
    ] {
        let ll = dot(Vec2::new(to.x - from.x, to.y - from.y), Vec2::new(to.x - from.x, to.y - from.y));
        if ll < 1e-18 {
            continue;
        }
        let seg = Edge::Seg { a: from, b: to };
        // An extension line is drawn from its gap past the measured point: an object meeting it nearer is no crossing.
        let skip = if line == BreakLine::Dimension {
            0.0
        } else {
            d.look.ext_offset_or_default() * d.height / ll.sqrt()
        };
        let mut ts: Vec<f64> = Vec::new();
        for (slot, list) in edges {
            if *slot == own {
                continue;
            }
            for edge in list {
                for hit in intersect_edges(&seg, edge) {
                    let t = dot(Vec2::new(hit.p.x - from.x, hit.p.y - from.y), Vec2::new(to.x - from.x, to.y - from.y)) / ll;
                    if (-1e-9..=1.0 + 1e-9).contains(&t) && t > skip + 1e-9 {
                        ts.push(t.clamp(0.0, 1.0));
                    }
                }
            }
        }
        ts.sort_by(f64::total_cmp);
        ts.dedup_by(|x, y| (*x - *y).abs() < 1e-9);
        out.extend(ts.into_iter().map(|t| DimensionBreak { line, t }));
    }
    out.truncate(MOST);
    out
}

pub fn validate(cx: &ExecutionContext<'_>, input: &DimensionsEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: (),
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &DimensionsEdit) -> CommandResult<DimensionsEditPlan> {
    plan_in(cx.doc, input)
}

/// [`plan`] over the document (a tool's preview).
pub fn plan_in(doc: &Document, input: &DimensionsEdit) -> CommandResult<DimensionsEditPlan> {
    match check(doc, input) {
        Ok(c) => CommandResult::Completed {
            output: DimensionsEditPlan {
                dimensions: c.changed.into_iter().map(|(_, _, d)| Entity::Dimension(d)).collect(),
                revision: doc.revision().to_string(),
            },
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: DimensionsEdit) -> CommandResult<DimensionsEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_DIMENSIONS_EDIT,
        kentos_commands::CAD_DIMENSIONS_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: DimensionsEdit) -> CommandResult<DimensionsEdited> {
    let c = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    let changed = c.changed.iter().map(|(_, u, _)| u.to_string()).collect();
    if !c.changed.is_empty() {
        doc.update_many(
            c.changed
                .into_iter()
                .map(|(s, _, d)| (s, Entity::Dimension(d)))
                .collect(),
            label(&input.edit),
        );
    }
    CommandResult::Completed {
        output: DimensionsEdited {
            changed,
            revision: doc.revision().to_string(),
        },
        warnings: c.warnings,
    }
}
