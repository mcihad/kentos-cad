//! `cad.objects.derive` v1 (docs/adr/0622 §9): Köşelere nokta, Orta hat and Toplu alan computed from the source
//! objects with the shared core (`ops::vertex_points`, `ops::centerline`, `ops::polygonize`), written as
//! `cad.entities.create` writes them, one undo step named after the tool. The shared cases are
//! `fixtures/commands/v1/cad.objects.derive.json` (`scripts/fixtures/objects_derive_cases.py`).

use std::collections::{BTreeMap, HashSet};

use kentos_commands::{
    CommandResult, CreateOperation, DeriveWork, EntitiesCreate, EntitiesCreatePlan, Entity,
    EntityGeometry, NewObject, ObjectsDerive, ObjectsDerived, Vec2 as Wire,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::api::json::Json;
use kentos_geometry_core::entity::{Entity as CoreEntity, Shape, TextPlace};
use kentos_geometry_core::ops::areas::polygon_of_area;
use kentos_geometry_core::ops::centerline::{self as axis, Method};
use kentos_geometry_core::ops::elevation::{Carry, Elevated, carry_elevations};
use kentos_geometry_core::ops::join::join_entities;
use kentos_geometry_core::ops::polygonize::{PolyLabel, polygonize};
use kentos_geometry_core::ops::vertex_points::vertex_points;
use kentos_geometry_core::vec2::Vec2;

use crate::checks::{self, Stop, error, is_uid_text};
use crate::elevation::{elevated, paths};
use crate::geometry::{drawing_font, edit_geometry, shape};
use crate::{ExecutionContext, codes, create};

fn refuse(code: &str, message: impl Into<String>, path: &str) -> Stop {
    Stop::failed(error(code, message.into(), Some(path.into())))
}

fn w(p: Vec2) -> Wire {
    Wire { x: p.x, y: p.y }
}

/// The objects `ids` names, each once, in order: their slots.
fn slots(doc: &Document, ids: &[String], path: &str) -> Result<Vec<Slot>, Stop> {
    let mut out: Vec<Slot> = Vec::new();
    for (i, uid) in ids.iter().enumerate() {
        let at = format!("{path}[{i}]");
        if !is_uid_text(uid) {
            return Err(refuse(
                codes::INVALID_UID,
                format!(
                    "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
                ),
                &at,
            ));
        }
        let slot = Uuid::parse_str(uid)
            .ok()
            .and_then(|u| doc.slot_of(u))
            .filter(|s| doc.get(*s).is_some())
            .ok_or_else(|| {
                refuse(
                    codes::ENTITY_NOT_FOUND,
                    format!(
                        "“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
                    ),
                    &at,
                )
            })?;
        if !out.contains(&slot) {
            out.push(slot);
        }
    }
    Ok(out)
}

/// What the work writes and how it is said.
struct Found {
    objects: Vec<NewObject>,
    operation: CreateOperation,
    skipped: usize,
    many: usize,
    existing: usize,
    next: Option<String>,
    method: Option<String>,
}

impl Found {
    fn new(operation: CreateOperation) -> Self {
        Found {
            objects: Vec::new(),
            operation,
            skipped: 0,
            many: 0,
            existing: 0,
            next: None,
            method: None,
        }
    }
}

fn object(geometry: EntityGeometry, input: &ObjectsDerive) -> NewObject {
    NewObject {
        line_type: None,
        line_weight: geometry
            .draws_lines()
            .then_some(input.line_weight)
            .flatten(),
        geometry,
        color: input.color.clone(),
        attrs: None,
        label: None,
        label_of: None,
        label_scale: None,
        symbol: None,
        order: None,
        transparency: None,
    }
}

// ── Köşelere nokta ───────────────────────────────────────────────────────────────────────────────────────────────

fn vertex_points_of(
    doc: &Document,
    ids: &[String],
    name: Option<&str>,
    code: Option<&str>,
    input: &ObjectsDerive,
) -> Result<Found, Stop> {
    let chosen: HashSet<Slot> = slots(doc, ids, "work.ids")?.into_iter().collect();
    let takes = |e: &Entity| {
        matches!(
            e,
            Entity::Line(_) | Entity::Polyline(_) | Entity::Polygon(_)
        )
    };
    if !chosen.iter().any(|s| doc.get(*s).is_some_and(&takes)) {
        return Err(refuse(
            codes::NOT_A_PATH,
            "Köşelere nokta: seçimde çizgi, çoklu çizgi ya da alan yok.",
            "work.ids",
        ));
    }
    // In the drawing's order: the objects' vertices, and every point's place (hidden layers too: a place is taken).
    let mut objects = Vec::new();
    let mut existing = Vec::new();
    for e in doc.entities() {
        match e {
            Entity::Point(q) => existing.push(Vec2::new(q.p.x, q.p.y)),
            _ if takes(e) && chosen.contains(&Slot(e.base().id)) => objects.push(paths(e)),
            _ => {}
        }
    }
    let name = name.map(str::trim).filter(|n| !n.is_empty());
    let found = vertex_points(&objects, &existing, name);
    if found.points.is_empty() {
        let text = if found.skipped > 0 {
            format!(
                "Köşelere nokta: yazılacak nokta yok; {} köşede zaten nokta var.",
                found.skipped
            )
        } else {
            "Köşelere nokta: yazılacak nokta yok.".to_owned()
        };
        return Err(refuse(codes::NOTHING_DERIVED, text, "work.ids"));
    }
    let code = code.map(str::trim).filter(|c| !c.is_empty());
    let mut out = Found::new(CreateOperation::VertexPoints);
    out.objects = found
        .points
        .iter()
        .map(|pt| NewObject {
            attrs: code.map(|c| BTreeMap::from([("Kod".to_owned(), c.to_owned())])),
            label: pt.name.clone(),
            ..object(
                EntityGeometry::Point {
                    p: w(pt.p),
                    z: pt.z,
                    parts: None,
                },
                input,
            )
        })
        .collect();
    out.skipped = found.skipped;
    out.next = found.next;
    Ok(out)
}

// ── Orta hat ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// A side's path: the one object's shape, or the chain's joined into one.
fn side(doc: &Document, ids: &[String], tolerance: f64, path: &str) -> Result<Shape, Stop> {
    let list = slots(doc, ids, path)?;
    let Some(&first) = list.first() else {
        return Err(refuse(
            codes::TOO_FEW_OBJECTS,
            "Kenarın en az bir nesnesini verin.",
            path,
        ));
    };
    if list.len() == 1 {
        return doc
            .get(first)
            .map(shape)
            .ok_or_else(|| refuse(codes::ENTITY_NOT_FOUND, "Nesne çizimde yok.", path));
    }
    // The core names each object by its `id`: the slot (Birleştir's `run_join`).
    let members: Vec<CoreEntity> = list
        .iter()
        .filter_map(|slot| {
            let e = doc.get(*slot)?;
            Some(CoreEntity {
                shape: shape(e),
                rest: vec![("id".to_owned(), Json::Num(f64::from(slot.0)))],
            })
        })
        .collect();
    match join_entities(&members, tolerance) {
        Ok(joined) if joined.groups.len() == 1 => Ok(joined.groups[0].geometry.shape.clone()),
        _ => Err(refuse(
            codes::SIDE_NOT_JOINED,
            "Kenarın nesneleri uç uca tek bir yol olmuyor; uçları birleşen çizgileri verin.",
            path,
        )),
    }
}

fn centerline_of(
    doc: &Document,
    first: &[String],
    second: &[String],
    step: f64,
    tolerance: Option<f64>,
    input: &ObjectsDerive,
) -> Result<Found, Stop> {
    checks::finite(step, "Adım", "Sonlu bir uzunluk verin.", "work.step")?;
    let tolerance = match tolerance {
        Some(t) => {
            checks::finite(t, "Tolerans", "Sonlu bir uzunluk verin.", "work.tolerance")?;
            t.max(1e-9)
        }
        None => 0.001,
    };
    let a = side(doc, first, tolerance, "work.first")?;
    let b = side(doc, second, tolerance, "work.second")?;
    if first.iter().any(|u| second.contains(u)) {
        return Err(refuse(
            codes::INVALID_INPUT,
            "İkinci kenar birincisiyle aynı olamaz; öbür kenarı verin.",
            "work.second",
        ));
    }
    let found = axis::centerline(&a, &b, step).map_err(|why| {
        let (code, path) = match why.as_str() {
            axis::BAD_STEP => (codes::INVALID_DISTANCE, "work.step"),
            axis::TOO_MANY => (codes::TOO_MANY_POINTS, "work.step"),
            axis::CLOSED => (codes::NOT_OPEN, "work.first"),
            _ => (codes::NO_ROUTE, "work.first"),
        };
        refuse(code, why, path)
    })?;
    let geometry = EntityGeometry::Polyline {
        ms: None,
        pts: found.pts.iter().copied().map(w).collect(),
        bulges: found.bulges.clone().filter(|b| b.iter().any(|&x| x != 0.0)),
        zs: None,
        parts: None,
        closed: false,
        widths: None,
    };
    let mut out = Found::new(CreateOperation::Centerline);
    out.objects = vec![object(geometry, input)];
    out.method = Some(
        match found.method {
            Method::Matched => "matched",
            Method::Sampled => "sampled",
        }
        .to_owned(),
    );
    Ok(out)
}

// ── Toplu alan ───────────────────────────────────────────────────────────────────────────────────────────────────

/// The kinds that close regions (docs/adr/0151 §2): construction lines, leaders and blocks do not.
pub fn line_work(e: &Entity) -> bool {
    matches!(
        e,
        Entity::Line(_)
            | Entity::Polyline(_)
            | Entity::Arc(_)
            | Entity::Circle(_)
            | Entity::Ellipse(_)
            | Entity::Spline(_)
            | Entity::Nurbs(_)
            | Entity::Polygon(_)
    )
}

/// A label's value: a text's (trimmed), a point's label; none for anything else or an empty one.
pub fn label_value(e: &Entity) -> Option<String> {
    let v = match e {
        Entity::Text(t) => t.text.trim(),
        Entity::Point(p) => p.base.label.as_deref().map_or("", str::trim),
        _ => "",
    };
    (!v.is_empty()).then(|| v.to_owned())
}

/// Where a label is: a text's box middle (its alignment and the drawing's typeface counted), a point itself.
pub fn label_at(e: &Entity, doc: &Document) -> Option<Vec2> {
    match e {
        Entity::Point(p) => Some(Vec2::new(p.p.x, p.p.y)),
        Entity::Text(_) => {
            let s = shape(e);
            let b = TextPlace::of(&s)?.outline(drawing_font(doc.settings().drawing_font));
            if b.len() < 4 {
                return None;
            }
            Some(Vec2::new(
                (b[0].x + b[1].x + b[2].x + b[3].x) / 4.0,
                (b[0].y + b[1].y + b[2].y + b[3].y) / 4.0,
            ))
        }
        _ => None,
    }
}

/// An area's elevations carried from the line work along it (docs/adr/0142).
fn with_elevations(g: &mut EntityGeometry, sources: &[Elevated]) {
    let zs = |pts: &[Wire]| {
        let pts: Vec<Vec2> = pts.iter().map(|p| Vec2::new(p.x, p.y)).collect();
        let out = carry_elevations(&pts, true, None, sources, Carry::Along);
        out.iter().any(Option::is_some).then_some(out)
    };
    if let EntityGeometry::Polygon {
        pts,
        holes,
        zs: own,
        ..
    } = g
    {
        *own = zs(pts);
        for h in holes.iter_mut().flatten() {
            h.zs = zs(&h.pts);
        }
    }
}

fn polygonize_of(
    doc: &Document,
    lines: &[String],
    labels: &[String],
    islands: bool,
    attribute: &str,
    input: &ObjectsDerive,
) -> Result<Found, Stop> {
    let attribute = attribute.trim();
    if attribute.is_empty() {
        return Err(refuse(
            codes::INVALID_ATTRIBUTE,
            "Etiketlerin yazılacağı özniteliğin adını verin (ör. Ad).",
            "work.attribute",
        ));
    }
    let line_slots: HashSet<Slot> = slots(doc, lines, "work.lines")?.into_iter().collect();
    let label_slots: HashSet<Slot> = slots(doc, labels, "work.labels")?.into_iter().collect();
    // In the drawing's order: an input area is named by its place there.
    let mut work: Vec<&Entity> = Vec::new();
    let mut found_labels: Vec<PolyLabel> = Vec::new();
    for e in doc.entities() {
        let slot = Slot(e.base().id);
        if line_slots.contains(&slot) && line_work(e) {
            work.push(e);
        }
        if label_slots.contains(&slot)
            && let (Some(value), Some(at)) = (label_value(e), label_at(e, doc))
        {
            found_labels.push(PolyLabel { at, value });
        }
    }
    if work.is_empty() {
        return Err(refuse(
            codes::NOT_A_PATH,
            "Toplu alan: bölge kapatacak çizgi yok; çizgi, çoklu çizgi, yay ya da alan seçin.",
            "work.lines",
        ));
    }
    let core: Vec<CoreEntity> = work.iter().map(|e| CoreEntity::new(shape(e))).collect();
    let result = polygonize(&core, &found_labels, islands);
    let sources: Vec<Elevated> = work.iter().flat_map(|e| paths(e)).collect();
    let carry = elevated(&sources);
    let mut out = Found::new(CreateOperation::Polygonize);
    for r in &result.regions {
        if r.existing.is_some() {
            out.existing += 1;
            continue;
        }
        let value = match r.labels.len() {
            1 => Some(found_labels[r.labels[0]].value.clone()),
            0 => {
                out.skipped += 1;
                None
            }
            _ => {
                out.many += 1;
                None
            }
        };
        let Some(mut geometry) = edit_geometry(polygon_of_area(&r.area).shape) else {
            continue;
        };
        if carry {
            with_elevations(&mut geometry, &sources);
        }
        out.objects.push(NewObject {
            attrs: value.map(|v| BTreeMap::from([(attribute.to_owned(), v)])),
            ..object(geometry, input)
        });
    }
    if out.objects.is_empty() {
        return Err(refuse(
            codes::NOTHING_DERIVED,
            "Toplu alan: yazılacak alan yok; hiçbir şey değişmedi.",
            "work.lines",
        ));
    }
    Ok(out)
}

fn find(doc: &Document, input: &ObjectsDerive) -> Result<Found, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    match &input.work {
        DeriveWork::VertexPoints { ids, name, code } => {
            vertex_points_of(doc, ids, name.as_deref(), code.as_deref(), input)
        }
        DeriveWork::Centerline {
            first,
            second,
            step,
            tolerance,
        } => centerline_of(doc, first, second, *step, *tolerance, input),
        DeriveWork::Polygonize {
            lines,
            labels,
            islands,
            attribute,
        } => polygonize_of(doc, lines, labels, *islands, attribute, input),
    }
}

fn create_input(found: &Found, input: &ObjectsDerive) -> EntitiesCreate {
    EntitiesCreate {
        layer_id: input.layer_id.clone(),
        objects: found.objects.clone(),
        operation: Some(found.operation),
        expected_revision: input.expected_revision.clone(),
    }
}

/// `cad.entities.create`'s refusal of one object's look (`objects[2].lineWeight`) is the input's own field.
fn ours<T>(result: CommandResult<T>) -> CommandResult<T> {
    fn path(error: &mut kentos_commands::CommandError) {
        if let Some(p) = &error.path
            && let Some(rest) = p.strip_prefix("objects[")
            && let Some((_, field)) = rest.split_once("].")
        {
            error.path = Some(field.to_owned());
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

/// The work's checks and `cad.entities.create`'s.
pub fn validate(cx: &ExecutionContext<'_>, input: &ObjectsDerive) -> CommandResult<()> {
    match find(cx.doc, input) {
        Ok(found) => ours(create::validate(cx, &create_input(&found, input))),
        Err(stop) => stop.into(),
    }
}

/// The objects as they would be written.
pub fn plan(cx: &ExecutionContext<'_>, input: &ObjectsDerive) -> CommandResult<EntitiesCreatePlan> {
    match find(cx.doc, input) {
        Ok(found) => ours(create::plan(cx, &create_input(&found, input))),
        Err(stop) => stop.into(),
    }
}

/// The objects a tool's preview draws: what the command would write, or its refusal.
pub fn geometries(
    doc: &Document,
    input: &ObjectsDerive,
) -> Result<Vec<NewObject>, Box<kentos_commands::CommandError>> {
    find(doc, input)
        .map(|f| f.objects)
        .map_err(|stop| Box::new(stop.into_error()))
}

/// Writes the objects, one undo step named after the tool.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: ObjectsDerive,
) -> CommandResult<ObjectsDerived> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_OBJECTS_DERIVE,
        kentos_commands::CAD_OBJECTS_DERIVE_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: ObjectsDerive,
) -> CommandResult<ObjectsDerived> {
    let found = match find(cx.doc, &input) {
        Ok(found) => found,
        Err(stop) => return stop.into(),
    };
    match ours(create::write_in(cx, create_input(&found, &input))) {
        CommandResult::Completed { output, warnings } => {
            let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
            CommandResult::Completed {
                output: ObjectsDerived {
                    ids: output.ids,
                    uids: output.created,
                    skipped: count(found.skipped),
                    many: count(found.many),
                    existing: count(found.existing),
                    next: found.next,
                    method: found.method,
                    revision: output.revision,
                },
                warnings,
            }
        }
        other => crate::trim::relay(other),
    }
}
