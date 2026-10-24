//! `cad.roads.edit` v1 (docs/adr/0198, 0622 §9): Plan yolu, Kavşak temizle and Refüj kapat computed with the shared
//! core (`ops::road`) and written as `cad.entities.create` and `cad.entities.edit` write them, one undo step named
//! after the tool. The shared cases are `fixtures/commands/v1/cad.roads.edit.json`
//! (`scripts/fixtures/roads_edit_cases.py`).

use std::collections::BTreeMap;

use kentos_commands::{
    CommandResult, CreateOperation, EditOperation, EntitiesCreate, EntitiesEdit, EntitiesEditPlan,
    Entity, EntityEdit, EntityGeometry, NewObject, RoadKind, RoadWork, RoadsEdit, RoadsEdited,
    Vec2 as Wire,
};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::geom::arrangement::Area;
use kentos_geometry_core::geom::parallel::clean_axis;
use kentos_geometry_core::ops::areas::areas_of_entity;
use kentos_geometry_core::ops::parts::one_area;
use kentos_geometry_core::ops::road::{median_ring, road_junctions, road_parts};
use kentos_geometry_core::vec2::Vec2;

use crate::checks::{self, Stop, error, is_uid_text};
use crate::geometry::{edit_geometry, shape};
use crate::{ExecutionContext, codes, create, edit};

const KIND: &str = "Tür";
const WIDTH: &str = "Genişlik";
const CARRIAGEWAY: &str = "Taşıt yolu";
/// The kinds Kavşak temizle leaves as they are.
const NOT_JOINED: [&str; 2] = ["Refüj", "Yol ekseni"];

fn refuse(code: &str, message: impl Into<String>, path: &str) -> Stop {
    Stop::failed(error(code, message.into(), Some(path.into())))
}

fn object_at<'d>(doc: &'d Document, uid: &str, path: &str) -> Result<&'d Entity, Stop> {
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
        .and_then(|s| doc.get(s))
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

/// A width as JavaScript writes the number: `10`, `2.5`.
fn js_number(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

fn area_geometry(area: &Area) -> Option<EntityGeometry> {
    one_area(std::slice::from_ref(area)).and_then(|e| edit_geometry(e.shape))
}

fn attrs(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

/// What a work writes: new objects, or an edit of the drawing's.
enum Prepared {
    Create {
        objects: Vec<(EntityGeometry, BTreeMap<String, String>)>,
        layer: String,
    },
    Edit {
        operation: EditOperation,
        changes: Vec<EntityEdit>,
        count: usize,
        skipped: usize,
    },
}

#[allow(clippy::too_many_arguments)]
fn plan_of(
    axis: &[Wire],
    road: RoadKind,
    width: f64,
    kerb: Option<f64>,
    median: Option<f64>,
    axis_line: bool,
    input: &RoadsEdit,
) -> Result<Prepared, Stop> {
    for (i, p) in axis.iter().enumerate() {
        checks::point(*p, "Eksenin noktasının", &format!("work.axis[{i}]"))?;
    }
    let pts: Vec<Vec2> = axis.iter().map(|p| Vec2::new(p.x, p.y)).collect();
    let axis = clean_axis(&pts, false);
    if axis.len() < 2 {
        return Err(refuse(
            codes::TOO_FEW_POINTS,
            "Eksen en az iki noktalı olmalı.",
            "work.axis",
        ));
    }
    checks::finite(width, "Genişlik", "Sonlu bir sayı verin.", "work.width")?;
    if width <= 0.0 {
        return Err(refuse(
            codes::INVALID_DISTANCE,
            "Yolun genişliği sıfırdan büyük olmalı.",
            "work.width",
        ));
    }
    let (kerb, median) = (kerb.unwrap_or(0.0), median.unwrap_or(0.0));
    checks::finite(kerb, "Kaldırım", "Sonlu bir sayı verin.", "work.kerb")?;
    checks::finite(median, "Refüj", "Sonlu bir sayı verin.", "work.median")?;
    if road != RoadKind::Road && (kerb != 0.0 || median != 0.0) {
        let path = if kerb != 0.0 {
            "work.kerb"
        } else {
            "work.median"
        };
        return Err(refuse(
            codes::INVALID_INPUT,
            "Kaldırım ve refüj yalnız Yol'da olur.",
            path,
        ));
    }
    if kerb < 0.0 || 2.0 * kerb >= width {
        return Err(refuse(
            codes::INVALID_DISTANCE,
            "Kaldırımlar yolun genişliğine sığmıyor; Kaldırım'ı ya da Genişlik'i değiştirin.",
            "work.kerb",
        ));
    }
    if median < 0.0 || median >= width - 2.0 * kerb {
        return Err(refuse(
            codes::INVALID_DISTANCE,
            "Refüj taşıt yoluna sığmıyor; Refüj'ü ya da Genişlik'i değiştirin.",
            "work.median",
        ));
    }
    let Some(layer) = input.layer_id.clone() else {
        return Err(refuse(
            codes::INVALID_INPUT,
            "Plan yolu'nun katmanını verin (layerId).",
            "layerId",
        ));
    };
    let parts = road_parts(&axis, width, kerb, median).ok_or_else(|| {
        refuse(
            codes::DEGENERATE_SHAPE,
            "Bu eksenden yol alanı çıkmıyor.",
            "work.axis",
        )
    })?;
    let word = match road {
        RoadKind::Road => "Yol",
        RoadKind::Footway => "Yaya yolu",
        RoadKind::Cycleway => "Bisiklet yolu",
    };
    let mut objects = Vec::new();
    if let Some(g) = area_geometry(&parts.road) {
        objects.push((g, attrs(&[(KIND, word), (WIDTH, &js_number(width))])));
    }
    if let Some(c) = parts.carriageway.as_ref().and_then(area_geometry) {
        objects.push((
            c,
            attrs(&[(KIND, CARRIAGEWAY), (WIDTH, &js_number(width - 2.0 * kerb))]),
        ));
    }
    if let Some(m) = parts.median.as_ref().and_then(area_geometry) {
        objects.push((m, attrs(&[(KIND, "Refüj"), (WIDTH, &js_number(median))])));
    }
    if axis_line {
        objects.push((
            EntityGeometry::Polyline {
                ms: None,
                pts: axis.iter().map(|p| Wire { x: p.x, y: p.y }).collect(),
                bulges: None,
                zs: None,
                parts: None,
                closed: false,
                widths: None,
            },
            attrs(&[(KIND, "Yol ekseni")]),
        ));
    }
    Ok(Prepared::Create { objects, layer })
}

fn junctions_of(doc: &Document, ids: &[String], block: f64, kerb: f64) -> Result<Prepared, Stop> {
    let mut chosen = Vec::new();
    for (i, id) in ids.iter().enumerate() {
        let e = object_at(doc, id, &format!("work.ids[{i}]"))?;
        chosen.push(e.base().id);
    }
    for (value, what, path) in [
        (block, "Ada köşesi", "work.blockRadius"),
        (kerb, "Kaldırım köşesi", "work.kerbRadius"),
    ] {
        checks::finite(value, what, "Sonlu bir sayı verin.", path)?;
        if value < 0.0 {
            return Err(refuse(
                codes::INVALID_RADIUS,
                "Köşe yarıçapı sıfır ya da büyük olmalı; 0 yuvarlamaz.",
                path,
            ));
        }
    }
    // The areas kind by kind, in the drawing's order.
    let mut kinds: Vec<(String, Vec<u32>, Vec<Area>)> = Vec::new();
    for e in doc.entities() {
        let id = e.base().id;
        let kind = e.base().attrs.get(KIND).cloned().unwrap_or_default();
        if !chosen.contains(&id) || e.kind() != "polygon" || NOT_JOINED.contains(&kind.as_str()) {
            continue;
        }
        let areas = areas_of_entity(&shape(e));
        match kinds.iter_mut().find(|(k, ..)| *k == kind) {
            Some((_, slots, all)) => {
                slots.push(id);
                all.extend(areas);
            }
            None => kinds.push((kind, vec![id], areas)),
        }
    }
    if kinds.is_empty() {
        return Err(refuse(
            codes::NOT_AN_AREA,
            "Seçimde yol alanı yok.",
            "work.ids",
        ));
    }
    let uid = |slot: u32| {
        doc.uid(kentos_domain::Slot(slot))
            .map(|u| u.to_string())
            .unwrap_or_default()
    };
    let (mut changes, mut count, mut skipped) = (Vec::new(), 0, 0);
    for (kind, slots, areas) in kinds {
        let radius = if kind == CARRIAGEWAY { kerb } else { block };
        let joined = road_junctions(&areas, radius);
        count += joined.done;
        skipped += joined.skipped;
        let Some((first, others)) = slots.split_first() else {
            continue;
        };
        let Some(geometry) = one_area(&joined.areas).and_then(|e| edit_geometry(e.shape)) else {
            continue;
        };
        if others.is_empty() && joined.done == 0 {
            continue;
        }
        changes.push(EntityEdit::Update {
            uid: uid(*first),
            geometry,
        });
        changes.extend(others.iter().map(|s| EntityEdit::Remove { uid: uid(*s) }));
    }
    Ok(Prepared::Edit {
        operation: EditOperation::RoadJunctions,
        changes,
        count,
        skipped,
    })
}

/// A kerb's path: a line's or a one-part polyline's points and bends.
fn kerb_of(e: &Entity, path: &str) -> Result<(Vec<Vec2>, Option<Vec<f64>>), Stop> {
    match e {
        Entity::Line(l) => Ok((vec![Vec2::new(l.a.x, l.a.y), Vec2::new(l.b.x, l.b.y)], None)),
        Entity::Polyline(p) if p.parts.as_ref().is_some_and(|parts| !parts.is_empty()) => {
            Err(refuse(
                codes::MULTI_PART,
                "Çok parçalı çizgi refüjün kenarı olamaz; parçalarına ayırın.",
                path,
            ))
        }
        Entity::Polyline(p) => Ok((
            p.pts.iter().map(|q| Vec2::new(q.x, q.y)).collect(),
            p.bulges.clone(),
        )),
        _ => Err(refuse(
            codes::NOT_A_PATH,
            "Refüjün kenarı çizgi ya da çoklu çizgi olmalı.",
            path,
        )),
    }
}

fn median_of(doc: &Document, first: &str, second: &str, round: bool) -> Result<Prepared, Stop> {
    let a = object_at(doc, first, "work.first")?;
    let b = object_at(doc, second, "work.second")?;
    let (pa, ba) = kerb_of(a, "work.first")?;
    let (pb, bb) = kerb_of(b, "work.second")?;
    if first == second {
        return Err(refuse(
            codes::INVALID_INPUT,
            "Refüjün iki ayrı kenarını verin.",
            "work.second",
        ));
    }
    let ring = median_ring(&pa, ba.as_deref(), &pb, bb.as_deref(), round).ok_or_else(|| {
        refuse(
            codes::DEGENERATE_SHAPE,
            "Bu iki çizgi refüj olarak kapatılamaz.",
            "work",
        )
    })?;
    Ok(Prepared::Edit {
        operation: EditOperation::MedianClose,
        changes: vec![
            EntityEdit::Replace {
                uid: first.to_owned(),
                geometry: EntityGeometry::Polygon {
                    pts: ring.pts.iter().map(|p| Wire { x: p.x, y: p.y }).collect(),
                    bulges: ring.bulges,
                    holes: None,
                    zs: None,
                    parts: None,
                },
                keep_data: Some(true),
            },
            EntityEdit::Remove {
                uid: second.to_owned(),
            },
        ],
        count: 1,
        skipped: 0,
    })
}

fn prepare(doc: &Document, input: &RoadsEdit) -> Result<Prepared, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    match &input.work {
        RoadWork::Plan {
            axis,
            road,
            width,
            kerb,
            median,
            axis_line,
        } => plan_of(axis, *road, *width, *kerb, *median, *axis_line, input),
        RoadWork::Junctions {
            ids,
            block_radius,
            kerb_radius,
        } => junctions_of(doc, ids, *block_radius, *kerb_radius),
        RoadWork::Median {
            first,
            second,
            round,
        } => median_of(doc, first, second, *round),
    }
}

fn create_input(
    objects: &[(EntityGeometry, BTreeMap<String, String>)],
    layer: &str,
    input: &RoadsEdit,
) -> EntitiesCreate {
    EntitiesCreate {
        layer_id: layer.to_owned(),
        objects: objects
            .iter()
            .map(|(geometry, attrs)| NewObject {
                line_type: None,
                line_weight: None,
                geometry: geometry.clone(),
                color: input.color.clone(),
                attrs: Some(attrs.clone()),
                label: None,
                label_of: None,
                label_scale: None,
                symbol: None,
                order: None,
                transparency: None,
            })
            .collect(),
        operation: Some(CreateOperation::PlanRoad),
        expected_revision: input.expected_revision.clone(),
    }
}

fn edit_input(operation: EditOperation, changes: &[EntityEdit], input: &RoadsEdit) -> EntitiesEdit {
    EntitiesEdit {
        operation,
        changes: changes.to_vec(),
        expected_revision: input.expected_revision.clone(),
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

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// The work's checks and the writing command's.
pub fn validate(cx: &ExecutionContext<'_>, input: &RoadsEdit) -> CommandResult<()> {
    match prepare(cx.doc, input) {
        Ok(Prepared::Create { objects, layer }) => {
            create::validate(cx, &create_input(&objects, &layer, input))
        }
        Ok(Prepared::Edit { changes, .. }) if changes.is_empty() => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Ok(Prepared::Edit {
            operation, changes, ..
        }) => ours(edit::validate(cx, &edit_input(operation, &changes, input))),
        Err(stop) => stop.into(),
    }
}

/// The objects as they would be written: new ones in `created`.
pub fn plan(cx: &ExecutionContext<'_>, input: &RoadsEdit) -> CommandResult<EntitiesEditPlan> {
    plan_in(cx.doc, input)
}

/// [`plan`] over the document (a tool's preview).
pub fn plan_in(doc: &Document, input: &RoadsEdit) -> CommandResult<EntitiesEditPlan> {
    match prepare(doc, input) {
        Ok(Prepared::Create { objects, layer }) => {
            match create::plan_in(doc, &create_input(&objects, &layer, input)) {
                CommandResult::Completed { output, warnings } => CommandResult::Completed {
                    output: EntitiesEditPlan {
                        changed: Vec::new(),
                        created: output.entities,
                        removed: Vec::new(),
                        revision: output.revision,
                    },
                    warnings,
                },
                other => crate::trim::relay(other),
            }
        }
        Ok(Prepared::Edit { changes, .. }) if changes.is_empty() => CommandResult::Completed {
            output: EntitiesEditPlan {
                changed: Vec::new(),
                created: Vec::new(),
                removed: Vec::new(),
                revision: doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Ok(Prepared::Edit {
            operation, changes, ..
        }) => ours(edit::plan_in(doc, &edit_input(operation, &changes, input))),
        Err(stop) => stop.into(),
    }
}

/// Draws or tidies the roads, one undo step named after the tool.
pub fn execute(cx: &mut ExecutionContext<'_>, input: RoadsEdit) -> CommandResult<RoadsEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_ROADS_EDIT,
        kentos_commands::CAD_ROADS_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: RoadsEdit) -> CommandResult<RoadsEdited> {
    let prepared = match prepare(cx.doc, &input) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    match prepared {
        Prepared::Create { objects, layer } => {
            let n = objects.len();
            match create::write_in(cx, create_input(&objects, &layer, &input)) {
                CommandResult::Completed { output, warnings } => CommandResult::Completed {
                    output: RoadsEdited {
                        changed: Vec::new(),
                        created: output.created,
                        removed: Vec::new(),
                        count: count(n),
                        skipped: 0,
                        revision: output.revision,
                    },
                    warnings,
                },
                other => crate::trim::relay(other),
            }
        }
        Prepared::Edit {
            operation,
            changes,
            count: done,
            skipped,
        } => {
            if changes.is_empty() {
                return CommandResult::Completed {
                    output: RoadsEdited {
                        changed: Vec::new(),
                        created: Vec::new(),
                        removed: Vec::new(),
                        count: count(done),
                        skipped: count(skipped),
                        revision: cx.doc.revision().to_string(),
                    },
                    warnings: Vec::new(),
                };
            }
            match ours(edit::write_in(cx, edit_input(operation, &changes, &input))) {
                CommandResult::Completed { output, warnings } => CommandResult::Completed {
                    output: RoadsEdited {
                        changed: output.changed,
                        created: output.created,
                        removed: output.removed,
                        count: count(done),
                        skipped: count(skipped),
                        revision: output.revision,
                    },
                    warnings,
                },
                other => crate::trim::relay(other),
            }
        }
    }
}
