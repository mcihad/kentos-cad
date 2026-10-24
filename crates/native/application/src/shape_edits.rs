//! `cad.shapes.edit` v1 (docs/adr/0622 §9): Biçim değiştir, Sürdür, Paralel kaydır, Sadeleştir, Kot ver and Okunur yap
//! computed from the objects and what the user gives with the shared core (`ops::reshape_by`, `ops::continuation`,
//! `ops::edge_shift`, `ops::reshape::simplify`, the elevation rule, the text's place), written as `cad.entities.edit`
//! writes them, one undo step named after the tool. The shared cases are `fixtures/commands/v1/cad.shapes.edit.json`
//! (`scripts/fixtures/shape_edits_cases.py`).

use kentos_commands::{
    CommandResult, EditOperation, EntitiesEdit, EntitiesEditPlan, Entity, EntityEdit,
    EntityGeometry, ShapeElevation, ShapeWork, ShapesEdit, ShapesEdited, TextEntity, Vec2 as Wire,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::entity::{
    Entity as CoreEntity, Shape, TextPlace, entity_area, entity_length,
};
use kentos_geometry_core::ops::continuation::continue_path;
use kentos_geometry_core::ops::edge_shift as rules;
use kentos_geometry_core::ops::reshape::simplify;
use kentos_geometry_core::ops::reshape_by::{ReshapeRefusal, reshape};
use kentos_geometry_core::vec2::Vec2;

use crate::checks::{self, Stop, error, is_uid_text};
use crate::elevation::{Change, geometry_with, has_any, takes};
use crate::geometry::{contract_curve, drawing_font, edit_geometry, shape};
use crate::{ExecutionContext, codes, edit};

fn refuse(code: &str, message: impl Into<String>, path: &str) -> Stop {
    Stop::failed(error(code, message.into(), Some(path.into())))
}

fn v(p: Wire) -> Vec2 {
    Vec2::new(p.x, p.y)
}

fn object<'d>(doc: &'d Document, uid: &str, path: &str) -> Result<(Slot, &'d Entity), Stop> {
    if !is_uid_text(uid) {
        return Err(refuse(
            codes::INVALID_UID,
            format!(
                "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
            ),
            path,
        ));
    }
    Uuid::parse_str(uid)
        .ok()
        .and_then(|u| doc.slot_of(u))
        .and_then(|s| Some((s, doc.get(s)?)))
        .ok_or_else(|| {
            refuse(
                codes::ENTITY_NOT_FOUND,
                format!(
                    "“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
                ),
                path,
            )
        })
}

fn objects<'d>(doc: &'d Document, ids: &[String]) -> Result<Vec<(String, &'d Entity)>, Stop> {
    let mut out: Vec<(String, Slot, &Entity)> = Vec::new();
    for (i, uid) in ids.iter().enumerate() {
        let (slot, e) = object(doc, uid, &format!("work.ids[{i}]"))?;
        if !out.iter().any(|(_, s, _)| *s == slot) {
            out.push((uid.clone(), slot, e));
        }
    }
    Ok(out.into_iter().map(|(u, _, e)| (u, e)).collect())
}

fn points(pts: &[Wire], whose: &str, field: &str) -> Result<Vec<Vec2>, Stop> {
    for (i, p) in pts.iter().enumerate() {
        checks::point(*p, whose, &format!("work.{field}[{i}]"))?;
    }
    Ok(pts.iter().copied().map(v).collect())
}

/// The object's size: an area's net area, a path's length.
fn size(s: &Shape, area: bool) -> f64 {
    if area {
        entity_area(s)
    } else {
        entity_length(s)
    }
    .unwrap_or(0.0)
}

/// What an edit writes and how it is said.
struct Prepared {
    operation: EditOperation,
    changes: Vec<EntityEdit>,
    before: f64,
    after: f64,
    objects: usize,
    skipped: usize,
    count: usize,
    distance: f64,
    /// Parçala: the edit makes new objects, said in `created`.
    creates: bool,
}

impl Prepared {
    fn new(operation: EditOperation, changes: Vec<EntityEdit>) -> Self {
        Prepared {
            operation,
            objects: changes.len(),
            changes,
            before: 0.0,
            after: 0.0,
            skipped: 0,
            count: 0,
            distance: 0.0,
            creates: false,
        }
    }
}

/// A line becomes a polyline in its place, with its attributes and label; anything else is updated.
fn in_place(uid: &str, e: &Entity, geometry: EntityGeometry) -> EntityEdit {
    if matches!(e, Entity::Line(_)) {
        EntityEdit::Replace {
            uid: uid.to_owned(),
            geometry,
            keep_data: Some(true),
        }
    } else {
        EntityEdit::Update {
            uid: uid.to_owned(),
            geometry,
        }
    }
}

fn written(s: &Shape) -> Result<EntityGeometry, Stop> {
    edit_geometry(s.clone()).ok_or_else(|| {
        refuse(
            codes::DEGENERATE_SHAPE,
            "Sonuç yazılabilir bir nesne değil.",
            "work",
        )
    })
}

/// Biçim değiştir's refusals, in the tool's words.
fn reshape_refusal(why: &ReshapeRefusal, area: bool) -> Stop {
    let (code, text, path) = match why {
        ReshapeRefusal::NotShape => (
            codes::NOT_A_PATH,
            "Biçim değiştir alan, çizgi ve çoklu çizgi içindir.",
            "work.id",
        ),
        ReshapeRefusal::TooShort => (
            codes::TOO_FEW_POINTS,
            "Hat en az iki noktalı olmalı.",
            "work.line",
        ),
        ReshapeRefusal::NoCrossing if area => (
            codes::RESHAPE_REFUSED,
            "Hat alanın sınırını iki kez kesmeli ya da ona değmeli.",
            "work.line",
        ),
        ReshapeRefusal::NoCrossing => (
            codes::RESHAPE_REFUSED,
            "Hat çizgiyi kesmeli ya da ona değmeli.",
            "work.line",
        ),
        ReshapeRefusal::ManyParts => (
            codes::RESHAPE_REFUSED,
            "Hat alanın birden çok parçasına değiyor; bir parçayı düzenleyin.",
            "work.line",
        ),
        ReshapeRefusal::TouchesHole => (
            codes::RESHAPE_REFUSED,
            "Hat bir deliğe değiyor; deliği Delik araçlarıyla düzenleyin.",
            "work.line",
        ),
        ReshapeRefusal::BothWays => (
            codes::RESHAPE_REFUSED,
            "Hat alanı hem kesiyor hem büyütüyor; ikisini ayrı hatlarla yapın.",
            "work.line",
        ),
        ReshapeRefusal::Apart => (
            codes::RESHAPE_REFUSED,
            "Hattın kapattığı cepler alanı parçalara ayırırdı.",
            "work.line",
        ),
        ReshapeRefusal::MultiPart => (
            codes::MULTI_PART,
            kentos_geometry_core::entity::MULTI_PART_REFUSED,
            "work.id",
        ),
    };
    refuse(code, text, path)
}

fn reshape_of(doc: &Document, id: &str, line: &[Wire]) -> Result<Prepared, Stop> {
    let (_, e) = object(doc, id, "work.id")?;
    let sketch = points(line, "Hattın noktasının", "line")?;
    let area = matches!(e, Entity::Polygon(_));
    let before_shape = shape(e);
    let after = reshape(&before_shape, &sketch).map_err(|why| reshape_refusal(&why, area))?;
    let mut p = Prepared::new(
        EditOperation::Reshape,
        vec![in_place(id, e, written(&after)?)],
    );
    p.before = size(&before_shape, area);
    p.after = size(&after, area);
    Ok(p)
}

/// The object's vertex elevations in the order of its vertices, none when it has none.
fn elevations(e: &Entity) -> Option<Vec<Option<f64>>> {
    let zs = match e {
        Entity::Line(l) => vec![l.za, l.zb],
        Entity::Polyline(p) => p.zs.clone()?,
        _ => return None,
    };
    zs.iter().any(Option::is_some).then_some(zs)
}

fn continue_of(
    doc: &Document,
    id: &str,
    from_start: bool,
    pts: &[Wire],
    bulges: Option<&[f64]>,
) -> Result<Prepared, Stop> {
    let (_, e) = object(doc, id, "work.id")?;
    if !matches!(e, Entity::Line(_) | Entity::Polyline(_)) {
        return Err(refuse(
            codes::NOT_A_PATH,
            "Sürdür çizgi ve çoklu çizgi içindir.",
            "work.id",
        ));
    }
    let drawn_tail = points(pts, "Noktanın", "pts")?;
    if let Some(b) = bulges {
        for (i, x) in b.iter().enumerate() {
            checks::finite(
                *x,
                "Büküm",
                "Sonlu bir sayı verin.",
                &format!("work.bulges[{i}]"),
            )?;
        }
        if b.len() != pts.len() {
            return Err(refuse(
                codes::BULGE_COUNT,
                format!(
                    "Çizilen her kenarın bir bükümü olmalı: {} nokta için {} büküm verildi.",
                    pts.len(),
                    b.len()
                ),
                "work.bulges",
            ));
        }
    }
    let before_shape = shape(e);
    let ends =
        kentos_geometry_core::ops::continuation::path_ends(&before_shape).ok_or_else(|| {
            refuse(
                codes::NOT_A_PATH,
                "Sürdür çizgi ve çoklu çizgi içindir.",
                "work.id",
            )
        })?;
    let end = if from_start { ends.first } else { ends.last };
    let drawn: Vec<Vec2> = std::iter::once(end).chain(drawn_tail).collect();
    let zero = vec![0.0; pts.len()];
    let bulges = bulges.unwrap_or(&zero);
    let after = continue_path(&before_shape, from_start, &drawn, bulges).ok_or_else(|| {
        refuse(
            codes::TOO_FEW_POINTS,
            "Sürdürmek için en az bir nokta verin.",
            "work.pts",
        )
    })?;
    let mut geometry = written(&after)?;
    // Its own vertices keep their elevations; the new ones have none.
    if let (Some(zs), EntityGeometry::Polyline { pts, zs: wrote, .. }) =
        (elevations(e), &mut geometry)
        && pts.len() >= zs.len()
    {
        let added = vec![None; pts.len() - zs.len()];
        *wrote = Some(if from_start {
            added.into_iter().chain(zs).collect()
        } else {
            zs.into_iter().chain(added).collect()
        });
    }
    let count_of = |s: &Shape| match s {
        Shape::Polyline { pts, .. } => pts.len(),
        _ => 2,
    };
    let mut p = Prepared::new(EditOperation::Continue, vec![in_place(id, e, geometry)]);
    p.before = size(&before_shape, false);
    p.after = size(&after, false);
    p.count = count_of(&after).saturating_sub(count_of(&before_shape));
    Ok(p)
}

fn edge_shift_of(
    doc: &Document,
    id: &str,
    at: Wire,
    distance: Option<f64>,
    area: Option<f64>,
) -> Result<Prepared, Stop> {
    let (_, e) = object(doc, id, "work.id")?;
    checks::point(at, "Kenarın noktasının", "work.at")?;
    let refused = |text: String, path: &str| refuse(codes::EDGE_SHIFT_REFUSED, text, path);
    if !matches!(e, Entity::Polygon(_) | Entity::Polyline(_)) {
        return Err(refused(rules::NOT_PATH.to_owned(), "work.id"));
    }
    let s = shape(e);
    let picked = rules::pick(&s, v(at)).map_err(|why| refused(why, "work.at"))?;
    let d = match (distance, area) {
        (Some(d), None) => {
            checks::finite(d, "Uzaklık", "Sonlu bir sayı verin.", "work.distance")?;
            d
        }
        (None, Some(target)) => {
            checks::finite(target, "Hedef alan", "Sonlu bir sayı verin.", "work.area")?;
            rules::for_area(&s, picked.ring, picked.edge, target)
                .map_err(|why| refused(why, "work.area"))?
        }
        _ => {
            return Err(refuse(
                codes::INVALID_INPUT,
                "Kenarı bir uzaklıkla (distance) ya da hedef alanla (area) kaydırın; ikisi birden değil.",
                "work.distance",
            ));
        }
    };
    let moved = rules::shifted(&s, picked.ring, picked.edge, d)
        .map_err(|why| refused(why, "work.distance"))?;
    let is_area = rules::area_of(&s).is_some();
    let mut p = Prepared::new(
        EditOperation::EdgeShift,
        vec![EntityEdit::Update {
            uid: id.to_owned(),
            geometry: written(&moved)?,
        }],
    );
    p.before = rules::area_of(&s).unwrap_or_else(|| size(&s, false));
    p.after = rules::area_of(&moved).unwrap_or_else(|| size(&moved, is_area));
    p.distance = d;
    Ok(p)
}

fn simplify_of(doc: &Document, ids: &[String], tolerance: f64) -> Result<Prepared, Stop> {
    checks::finite(
        tolerance,
        "Tolerans",
        "Sonlu bir sayı verin.",
        "work.tolerance",
    )?;
    if tolerance <= 0.0 {
        return Err(refuse(
            codes::INVALID_TOLERANCE,
            "Tolerans sıfırdan büyük olmalı.",
            "work.tolerance",
        ));
    }
    let list = objects(doc, ids)?;
    let (mut changes, mut done, mut deviation, mut ignored, mut idle) =
        (Vec::new(), 0, 0.0f64, 0, 0);
    for (uid, e) in &list {
        let Some(r) = simplify(&CoreEntity::new(shape(e)), tolerance) else {
            ignored += 1;
            continue;
        };
        if r.done == 0 {
            idle += 1;
            continue;
        }
        done += r.done;
        deviation = deviation.max(r.deviation);
        changes.push(EntityEdit::Update {
            uid: uid.clone(),
            geometry: written(&r.entity.shape)?,
        });
    }
    if changes.is_empty() {
        let (code, text) = if ignored == list.len() {
            (
                codes::NOT_A_PATH,
                "Seçimde çoklu çizgi ya da kapalı alan yok; Sadeleştir yalnız onların köşelerini atar.",
            )
        } else {
            (
                codes::NOTHING_TO_SIMPLIFY,
                "Bu toleransla atılacak köşe yok; daha büyük bir tolerans yazın.",
            )
        };
        return Err(refuse(code, text, "work.ids"));
    }
    let mut p = Prepared::new(EditOperation::Simplify, changes);
    p.skipped = ignored + idle;
    p.count = done;
    p.distance = deviation;
    Ok(p)
}

fn elevation_of(doc: &Document, ids: &[String], z: ShapeElevation) -> Result<Prepared, Stop> {
    let change = match z {
        ShapeElevation::Set { z } => {
            checks::finite(z, "Kot", "Sonlu bir sayı verin.", "work.elevation.z")?;
            Change::Set(Some(z))
        }
        ShapeElevation::Raise { by } => {
            checks::finite(by, "Fark", "Sonlu bir sayı verin.", "work.elevation.by")?;
            Change::Raise(by)
        }
        ShapeElevation::Clear => Change::Set(None),
    };
    let list = objects(doc, ids)?;
    let takers: Vec<&(String, &Entity)> = list.iter().filter(|(_, e)| takes(e)).collect();
    if takers.is_empty() {
        return Err(refuse(
            codes::NOT_A_PATH,
            "Seçimde kot alan nesne yok; çizgi, çoklu çizgi, alan ya da nokta seçin.",
            "work.ids",
        ));
    }
    let chosen: Vec<&&(String, &Entity)> = match change {
        Change::Raise(_) => takers.iter().filter(|(_, e)| has_any(e)).collect(),
        _ => takers.iter().collect(),
    };
    if chosen.is_empty() {
        return Err(refuse(
            codes::NO_ELEVATION,
            "Seçili nesnelerin hiçbir köşesinde kot yok; fark eklenecek bir şey bulunamadı.",
            "work.ids",
        ));
    }
    let changes: Vec<EntityEdit> = chosen
        .iter()
        .filter_map(|(uid, e)| {
            Some(EntityEdit::Update {
                uid: uid.clone(),
                geometry: geometry_with(e, change)?,
            })
        })
        .collect();
    let mut p = Prepared::new(EditOperation::Elevation, changes);
    p.skipped = list.len() - chosen.len();
    Ok(p)
}

fn readable_of(doc: &Document, ids: &[String]) -> Result<Prepared, Stop> {
    let list = objects(doc, ids)?;
    let texts: Vec<(&String, &TextEntity)> = list
        .iter()
        .filter_map(|(uid, e)| match e {
            Entity::Text(t) => Some((uid, t)),
            _ => None,
        })
        .collect();
    if texts.is_empty() {
        return Err(refuse(
            codes::NOT_A_TEXT,
            "Seçimde yazı yok. Okunur yap yazı nesnelerini çevirir; yazıları seçip yeniden deneyin.",
            "work.ids",
        ));
    }
    let font = drawing_font(doc.settings().drawing_font);
    let changes: Vec<EntityEdit> = texts
        .iter()
        .filter_map(|(uid, t)| {
            // A multi-line text turns about its box's middle too (docs/adr/0182); one along a curve turns its curve
            // the other way, its alignment's shares swapped (docs/adr/0196 §3).
            let s = shape(&Entity::Text((*t).clone()));
            let place = TextPlace::of(&s)?;
            let turned = if t.path.is_some() {
                let placed = place.along(font)?.readable()?;
                TextEntity {
                    p: Wire {
                        x: placed.p.x,
                        y: placed.p.y,
                    },
                    rotation: placed.rotation,
                    align: placed
                        .align
                        .and_then(|a| kentos_commands::TextAlign::from_name(a.name())),
                    path: Some(contract_curve(placed.curve)),
                    ..(*t).clone()
                }
            } else {
                let (p, rotation) = place.readable(font)?;
                TextEntity {
                    p: Wire { x: p.x, y: p.y },
                    rotation,
                    ..(*t).clone()
                }
            };
            Some(EntityEdit::Update {
                uid: (*uid).clone(),
                geometry: edit_geometry(shape(&Entity::Text(turned)))?,
            })
        })
        .collect();
    let mut p = Prepared::new(EditOperation::Readable, changes);
    p.count = p.changes.len();
    p.skipped = list.len() - p.changes.len();
    Ok(p)
}

fn prepare(doc: &Document, input: &ShapesEdit) -> Result<Prepared, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    match &input.work {
        ShapeWork::Reshape { id, line } => reshape_of(doc, id, line),
        ShapeWork::Continue {
            id,
            from_start,
            pts,
            bulges,
        } => continue_of(doc, id, *from_start, pts, bulges.as_deref()),
        ShapeWork::EdgeShift {
            id,
            at,
            distance,
            area,
        } => edge_shift_of(doc, id, *at, *distance, *area),
        ShapeWork::Simplify { ids, tolerance } => simplify_of(doc, ids, *tolerance),
        ShapeWork::Elevation { ids, elevation } => elevation_of(doc, ids, *elevation),
        ShapeWork::Readable { ids } => readable_of(doc, ids),
        ShapeWork::Split {
            ids,
            method,
            parts,
            length,
            from_end,
        } => {
            let list = objects(doc, ids)?;
            let cut = crate::shape_split::split(&list, *method, *parts, *length, *from_end)?;
            let mut p = Prepared::new(EditOperation::Split, cut.changes);
            p.objects = cut.objects;
            p.skipped = cut.skipped;
            p.count = cut.pieces;
            p.creates = true;
            Ok(p)
        }
    }
}

/// `cad.entities.edit`'s refusal of one of its changes names the work (`work`).
fn ours<T>(result: CommandResult<T>) -> CommandResult<T> {
    fn path(error: &mut kentos_commands::CommandError) {
        if error
            .path
            .as_deref()
            .is_some_and(|p| p.starts_with("changes["))
        {
            error.path = Some("work".to_owned());
        }
    }
    match result {
        CommandResult::Failed { mut error } => {
            path(&mut error);
            CommandResult::Failed { error }
        }
        CommandResult::Conflict { mut error } => {
            path(&mut error);
            CommandResult::Conflict { error }
        }
        CommandResult::NeedsInput { mut error } => {
            path(&mut error);
            CommandResult::NeedsInput { error }
        }
        other => other,
    }
}

fn edit_input(p: &Prepared, input: &ShapesEdit) -> EntitiesEdit {
    EntitiesEdit {
        operation: p.operation,
        changes: p.changes.clone(),
        expected_revision: input.expected_revision.clone(),
    }
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// The edit's checks and `cad.entities.edit`'s; Okunur yap with nothing upside down checks only its own.
pub fn validate(cx: &ExecutionContext<'_>, input: &ShapesEdit) -> CommandResult<()> {
    match prepare(cx.doc, input) {
        Ok(p) if p.changes.is_empty() => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Ok(p) => ours(edit::validate(cx, &edit_input(&p, input))),
        Err(stop) => stop.into(),
    }
}

/// The objects as they would be written.
pub fn plan(cx: &ExecutionContext<'_>, input: &ShapesEdit) -> CommandResult<EntitiesEditPlan> {
    plan_in(cx.doc, input)
}

/// The result as a tool's preview draws it: what the command would write, or its refusal.
pub fn plan_in(doc: &Document, input: &ShapesEdit) -> CommandResult<EntitiesEditPlan> {
    match prepare(doc, input) {
        Ok(p) if p.changes.is_empty() => CommandResult::Completed {
            output: EntitiesEditPlan {
                changed: Vec::new(),
                created: Vec::new(),
                removed: Vec::new(),
                revision: doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Ok(p) => ours(edit::plan_in(doc, &edit_input(&p, input))),
        Err(stop) => stop.into(),
    }
}

/// Edits the shapes, one undo step named after the tool.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ShapesEdit) -> CommandResult<ShapesEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_SHAPES_EDIT,
        kentos_commands::CAD_SHAPES_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: ShapesEdit) -> CommandResult<ShapesEdited> {
    let p = match prepare(cx.doc, &input) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    let output = |changed: Vec<String>, created: Vec<String>, revision: String| ShapesEdited {
        changed,
        before: p.before,
        after: p.after,
        objects: count(p.objects),
        skipped: count(p.skipped),
        count: count(p.count),
        distance: p.distance,
        created: if p.creates { created } else { Vec::new() },
        revision,
    };
    // Okunur yap with every text readable already: nothing to write, nothing refused.
    if p.changes.is_empty() {
        return CommandResult::Completed {
            output: output(Vec::new(), Vec::new(), cx.doc.revision().to_string()),
            warnings: Vec::new(),
        };
    }
    match ours(edit::write_in(cx, edit_input(&p, &input))) {
        CommandResult::Completed {
            output: o,
            warnings,
        } => CommandResult::Completed {
            output: output(o.changed, o.created, o.revision),
            warnings,
        },
        other => crate::trim::relay(other),
    }
}
