//! `cad.topology.clean` v1 (plan/map-cad.md MAP-07, docs/adr/1262): Topolojik temizlik (docs/adr/0148) as a command
//! with its moves reported. The objects go to the shared core (`ops::topology::topology_clean`) as the tool gives them
//! ([`object`]), the corrected ones come back as `cad.entities.edit` writes them ([`geometry_of`]); the answer says
//! every move, each changed object's source vertices kept bit for bit and moved, its area or length before and after,
//! the farthest and total move, and the moves past the quality profile's acceptance threshold (docs/adr/1260 §2),
//! which the write refuses. Nothing is written but by `execute`, as one undo step “Topolojik temizlik”. The tool
//! `tool.topology` (crates/native/interaction/src/topology.rs) takes its objects and geometries from here too.

use std::collections::HashSet;

use kentos_commands::{
    AreaPart, CommandResult, EditOperation, EntitiesEdit, Entity, EntityEdit, EntityGeometry, RingGeometry,
    TopologyClean, TopologyCleanChange, TopologyCleanCounts, TopologyCleanMove, TopologyCleaned,
};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::entity::{Shape, entity_area, entity_length};
use kentos_geometry_core::geom::arc::ArcGeom;
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::ops::edges::entity_edges;
use kentos_geometry_core::ops::topology::{TopoObject, TopoPath, TopoWorks, arc_path, path_arc, topology_clean};
use kentos_geometry_core::vec2::Vec2;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::elevation::paths as elevated_paths;
use crate::geometry::{entity_of, shape};

/// The smallest tolerance, metres: the core's “the same place”.
pub const LEAST_TOLERANCE: f64 = 1e-6;
/// The works when none is given (docs/adr/0148 §3): Köşeler off, the others on.
pub const FIRST_WORKS: TopoWorks = TopoWorks {
    ends: true,
    vertices: false,
    extend: true,
    trim: true,
};

/// The kinds the cleanup corrects (docs/adr/0148 §2).
pub fn corrected(e: &Entity) -> bool {
    matches!(e, Entity::Line(_) | Entity::Polyline(_) | Entity::Arc(_) | Entity::Polygon(_))
}

/// The kinds that take part: the corrected ones and the supports' own.
pub fn takes_part(e: &Entity) -> bool {
    corrected(e)
        || matches!(
            e,
            Entity::Point(_) | Entity::Circle(_) | Entity::Ellipse(_) | Entity::Spline(_) | Entity::Nurbs(_)
        )
}

/// A path's bulges as the core takes them: one a vertex, when any bends; none when all are straight.
fn bulges_of(n: usize, bulges: Option<&[f64]>) -> Option<Vec<f64>> {
    let b = bulges?;
    b.iter().any(|v| *v != 0.0).then(|| (0..n).map(|i| b.get(i).copied().unwrap_or(0.0)).collect())
}

/// A circle as a boundary: two half turns (docs/adr/0148 §2).
fn circle_object(c: Vec2, r: f64) -> TopoObject {
    TopoObject {
        kind: "edges".into(),
        fixed: true,
        paths: vec![TopoPath {
            pts: vec![Vec2::new(c.x + r, c.y), Vec2::new(c.x - r, c.y)],
            bulges: Some(vec![1.0, 1.0]),
            closed: true,
            zs: vec![None, None],
        }],
    }
}

/// An edge of an ellipse's or a curve's outline (its 0.1 mm chords, docs/adr/0149 §5.3) as a path of its own.
fn edge_path(edge: &Edge) -> Option<TopoPath> {
    match *edge {
        Edge::Seg { a, b } => Some(TopoPath { pts: vec![a, b], bulges: None, closed: false, zs: vec![None, None] }),
        Edge::Arc { c, r, a0, sweep } => arc_path(&ArcGeom { c, r, a0, a1: a0 + sweep }),
    }
}

fn v(p: &kentos_commands::Vec2) -> Vec2 {
    Vec2::new(p.x, p.y)
}

fn w(p: Vec2) -> kentos_commands::Vec2 {
    kentos_commands::Vec2 { x: p.x, y: p.y }
}

/// An object as the cleanup takes it (docs/adr/0148 §2), or none when it takes no part: a line, a polyline and an
/// area by their paths with their elevations (`elevation::paths`' order), an arc as its two ends and a bulge, a point
/// (always fixed), a circle, an ellipse or a curve as edges (always fixed). The web's is `topoObject`
/// (`topologyTool.ts`).
pub fn object(e: &Entity, fixed: bool) -> Option<TopoObject> {
    let paths = || {
        elevated_paths(e)
            .into_iter()
            .map(|p| TopoPath { bulges: bulges_of(p.pts.len(), p.bulges.as_deref()), pts: p.pts, closed: p.closed, zs: p.zs })
            .collect()
    };
    let kind = |k: &str, paths: Vec<TopoPath>| Some(TopoObject { kind: k.into(), fixed, paths });
    match e {
        Entity::Line(_) => kind("line", paths()),
        Entity::Polyline(_) => kind("polyline", paths()),
        Entity::Polygon(_) => kind("area", paths()),
        Entity::Arc(a) => match arc_path(&ArcGeom { c: v(&a.c), r: a.r, a0: a.a0, a1: a.a1 }) {
            Some(path) => kind("arc", vec![path]),
            None => Some(circle_object(v(&a.c), a.r)),
        },
        Entity::Point(p) => Some(TopoObject {
            kind: "point".into(),
            fixed: true,
            paths: vec![TopoPath { pts: vec![v(&p.p)], bulges: None, closed: false, zs: vec![p.z] }],
        }),
        Entity::Circle(c) => Some(circle_object(v(&c.c), c.r)),
        Entity::Ellipse(_) | Entity::Spline(_) | Entity::Nurbs(_) => Some(TopoObject {
            kind: "edges".into(),
            fixed: true,
            paths: entity_edges(&shape(e)).iter().filter_map(edge_path).collect(),
        }),
        _ => None,
    }
}

/// The geometry a corrected object takes, as `cad.entities.edit` writes it: its paths back in `elevation::paths`'
/// order (an area's outer ring, its holes, then each other part's ring and holes), each with its elevations explicit;
/// an arc rebuilt from its ends and bulge. None for a kind the cleanup does not correct. The web's is `geometryOf`.
pub fn geometry_of(e: &Entity, paths: &[TopoPath]) -> Option<EntityGeometry> {
    let pts = |p: &TopoPath| p.pts.iter().copied().map(w).collect::<Vec<_>>();
    let ring = |p: &TopoPath| RingGeometry {
        pts: pts(p),
        bulges: bulges_of(p.pts.len(), p.bulges.as_deref()),
        zs: Some(p.zs.clone()),
    };
    match e {
        Entity::Line(_) => {
            let p = paths.first()?;
            Some(EntityGeometry::Line { ms: None, a: w(*p.pts.first()?), b: w(*p.pts.get(1)?), zs: Some(p.zs.clone()) })
        }
        Entity::Polyline(_) => {
            let mut next = paths.iter();
            let p = next.next()?;
            // Every other part, in `elevation::paths`' order (docs/adr/0174).
            let parts: Vec<AreaPart> = next
                .map(|q| {
                    let own = ring(q);
                    AreaPart { ms: None, pts: own.pts, bulges: own.bulges, holes: None, zs: own.zs }
                })
                .collect();
            Some(EntityGeometry::Polyline {
                ms: None,
                pts: pts(p),
                bulges: bulges_of(p.pts.len(), p.bulges.as_deref()),
                zs: Some(p.zs.clone()),
                parts: (!parts.is_empty()).then_some(parts),
                // A closed line stays closed; its widths are carried (docs/adr/0258).
                closed: p.closed,
                widths: None,
            })
        }
        Entity::Polygon(area) => {
            let mut next = paths.iter();
            let outer = next.next()?;
            let mut holes = Vec::new();
            for _ in area.holes.iter().flatten() {
                holes.push(ring(next.next()?));
            }
            let mut parts = Vec::new();
            for part in area.parts.iter().flatten() {
                let own = ring(next.next()?);
                let mut part_holes = Vec::new();
                for _ in part.holes.iter().flatten() {
                    part_holes.push(ring(next.next()?));
                }
                parts.push(AreaPart {
                    ms: None,
                    pts: own.pts,
                    bulges: own.bulges,
                    holes: (!part_holes.is_empty()).then_some(part_holes),
                    zs: own.zs,
                });
            }
            Some(EntityGeometry::Polygon {
                pts: pts(outer),
                bulges: bulges_of(outer.pts.len(), outer.bulges.as_deref()),
                holes: (!holes.is_empty()).then_some(holes),
                zs: Some(outer.zs.clone()),
                parts: (!parts.is_empty()).then_some(parts),
            })
        }
        Entity::Arc(_) => {
            let arc = path_arc(paths.first()?)?;
            Some(EntityGeometry::Arc { c: w(arc.c), r: arc.r, a0: arc.a0, a1: arc.a1 })
        }
        _ => None,
    }
}

fn count(n: usize) -> u32 {
    n.min(u32::MAX as usize) as u32
}

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// How far a vertex moved, metres.
fn far(a: Vec2, b: Vec2) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

/// The vertices of a shape's paths, as the core takes them.
fn vertices(paths: &[TopoPath]) -> impl Iterator<Item = Vec2> + '_ {
    paths.iter().flat_map(|p| p.pts.iter().copied())
}

/// What the clean-up would write, and the answer.
struct Planned {
    edits: Vec<EntityEdit>,
    output: TopologyCleaned,
}

fn plan_in(doc: &Document, input: &TopologyClean) -> Result<Planned, Stop> {
    if !input.tolerance.is_finite() || input.tolerance < LEAST_TOLERANCE {
        return Err(fail(
            codes::INVALID_INPUT,
            format!("Tolerans sonlu ve en az {LEAST_TOLERANCE} m olmalı (yazılan {}).", input.tolerance),
            "tolerance",
        ));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let layers = doc.layers();
    let open = |e: &Entity| {
        let layer = &e.base().layer_id;
        corrected(e) && layers.is_visible(layer) && !layers.is_locked(layer)
    };
    // What it corrects (docs/adr/0148 §2): the given objects that can be, else every one that can be.
    let mut targets: HashSet<u32> = HashSet::new();
    let mut skipped = 0usize;
    match &input.uids {
        Some(uids) => {
            for (i, uid) in uids.iter().enumerate() {
                let path = format!("uids[{i}]");
                if !checks::is_uid_text(uid) {
                    return Err(fail(codes::INVALID_UID, format!("“{uid}” geçerli bir nesne kimliği değil."), &path));
                }
                let Some(e) = Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u)).and_then(|s| doc.get(s)) else {
                    return Err(fail(codes::ENTITY_NOT_FOUND, format!("“{uid}” kimlikli nesne çizimde yok."), &path));
                };
                if open(e) {
                    targets.insert(e.base().id);
                } else {
                    skipped += 1;
                }
            }
        }
        None => targets.extend(doc.entities().filter(|e| open(e)).map(|e| e.base().id)),
    }
    let works = TopoWorks {
        ends: input.ends.unwrap_or(FIRST_WORKS.ends),
        vertices: input.vertices.unwrap_or(FIRST_WORKS.vertices),
        extend: input.extend.unwrap_or(FIRST_WORKS.extend),
        trim: input.trim.unwrap_or(FIRST_WORKS.trim),
    };
    // Every object on a visible layer that takes part, in the drawing's order (ties go by it).
    let mut objects: Vec<TopoObject> = Vec::new();
    let mut taken: Vec<&Entity> = Vec::new();
    for e in doc.entities() {
        if !layers.is_visible(&e.base().layer_id) {
            continue;
        }
        if let Some(o) = object(e, !targets.contains(&e.base().id)) {
            objects.push(o);
            taken.push(e);
        }
    }
    let supports = objects.iter().filter(|o| o.fixed).count();
    let result = topology_clean(&objects, input.tolerance, works)
        .map_err(|why| fail(codes::INVALID_INPUT, format!("Temizlik hesaplanamadı: {why}."), "tolerance"))?;
    let uid_of = |k: usize| doc.uid(kentos_domain::Slot(taken[k].base().id)).map(|u| u.to_string()).unwrap_or_default();
    let moves: Vec<TopologyCleanMove> = result
        .changes
        .iter()
        .map(|c| TopologyCleanMove {
            uid: uid_of(c.object),
            kind: c.kind.clone(),
            from: w(c.from),
            to: w(c.to),
            distance: far(c.from, c.to),
        })
        .collect();
    let acceptance = doc.settings().topology.as_ref().and_then(|t| t.profile.as_ref()).map(|p| p.acceptance);
    let mut changes = Vec::new();
    let mut edits = Vec::new();
    let mut area_change = 0.0;
    for c in &result.changed {
        if objects[c.object].paths == c.paths {
            continue;
        }
        let e = taken[c.object];
        let Some(geometry) = geometry_of(e, &c.paths) else { continue };
        let uid = uid_of(c.object);
        let before: Shape = shape(e);
        let after: Shape = shape(&entity_of(&geometry, e.base().clone()));
        let area_before = entity_area(&before);
        let area_after = area_before.and(entity_area(&after));
        if let (Some(a), Some(b)) = (area_before, area_after) {
            area_change += b - a;
        }
        let now: Vec<Vec2> = vertices(&c.paths).collect();
        let kept = vertices(&objects[c.object].paths)
            .filter(|p| now.iter().any(|q| q.x.to_bits() == p.x.to_bits() && q.y.to_bits() == p.y.to_bits()))
            .count();
        let own: Vec<f64> = result.changes.iter().filter(|m| m.object == c.object).map(|m| far(m.from, m.to)).collect();
        changes.push(TopologyCleanChange {
            uid: uid.clone(),
            layer: e.base().layer_id.clone(),
            area_before,
            area_after,
            length_before: if area_before.is_none() { entity_length(&before) } else { None },
            length_after: if area_before.is_none() { entity_length(&after) } else { None },
            kept: count(kept),
            moves: count(own.len()),
            max_moved: own.iter().copied().fold(0.0, f64::max),
            total_moved: own.iter().sum(),
        });
        edits.push(EntityEdit::Update { uid, geometry });
    }
    let beyond = acceptance.map_or(0, |a| moves.iter().filter(|m| m.distance > a).count());
    let c = result.counts;
    Ok(Planned {
        edits,
        output: TopologyCleaned {
            objects: count(targets.len()),
            supports: count(supports),
            skipped: count(skipped),
            counts: TopologyCleanCounts {
                ends: count(c.ends),
                vertices: count(c.vertices),
                extended: count(c.extended),
                trimmed: count(c.trimmed),
                edges: count(c.edges),
            },
            moved: count(moves.len()),
            max_moved: moves.iter().map(|m| m.distance).fold(0.0, f64::max),
            total_moved: moves.iter().map(|m| m.distance).sum(),
            moves,
            changes,
            area_change,
            acceptance,
            beyond: count(beyond),
            revision: doc.revision().to_string(),
        },
    })
}

/// The write's last check: no move past the quality profile's acceptance threshold (docs/adr/1260 §2).
fn within_acceptance(doc: &Document, out: &TopologyCleaned) -> Result<(), Stop> {
    let (Some(a), Some(far)) = (out.acceptance, out.moves.iter().max_by(|x, y| x.distance.total_cmp(&y.distance))) else {
        return Ok(());
    };
    if far.distance <= a {
        return Ok(());
    }
    let name = doc.settings().topology.as_ref().and_then(|t| t.profile.as_ref()).map(|p| p.name.clone()).unwrap_or_default();
    Err(fail(
        codes::CLEAN_BEYOND_ACCEPTANCE,
        format!(
            "Temizlik {} köşeyi kabul eşiğinden uzağa taşıyor; en uzağı “{}” nesnesinde {:.4} m, kalite profilinin (“{name}”) kabul eşiği {a} m. Hiçbir şey yazılmadı: toleransı kabul eşiğine indirin ya da o köşeleri ölçüyle düzeltin.",
            out.beyond, far.uid, far.distance
        ),
        "tolerance",
    ))
}

pub fn validate(cx: &ExecutionContext<'_>, input: &TopologyClean) -> CommandResult<()> {
    match plan_in(cx.doc, input).and_then(|p| within_acceptance(cx.doc, &p.output)) {
        Ok(()) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The report; nothing written. A move past the acceptance threshold is counted (`beyond`), not refused.
pub fn plan(cx: &ExecutionContext<'_>, input: &TopologyClean) -> CommandResult<TopologyCleaned> {
    match plan_in(cx.doc, input) {
        Ok(p) => CommandResult::Completed { output: p.output, warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: TopologyClean) -> CommandResult<TopologyCleaned> {
    crate::call::recorded(cx, kentos_commands::CAD_TOPOLOGY_CLEAN, kentos_commands::CAD_TOPOLOGY_CLEAN_VERSION, input, write)
}

/// [`execute`]'s write: the objects through `cad.entities.edit`'s write (`topology`), its checks once more.
fn write(cx: &mut ExecutionContext<'_>, input: TopologyClean) -> CommandResult<TopologyCleaned> {
    let Planned { edits, mut output } = match plan_in(cx.doc, &input).and_then(|p| within_acceptance(cx.doc, &p.output).map(|()| p)) {
        Ok(p) => p,
        Err(stop) => return stop.into(),
    };
    if edits.is_empty() {
        return CommandResult::Completed { output, warnings: Vec::new() };
    }
    let edit = EntitiesEdit { operation: EditOperation::Topology, changes: edits, expected_revision: None };
    match crate::edit::write_in(cx, edit) {
        CommandResult::Completed { warnings, .. } => {
            output.revision = cx.doc.revision().to_string();
            CommandResult::Completed { output, warnings }
        }
        CommandResult::Failed { error } => CommandResult::Failed { error },
        CommandResult::Conflict { error } => CommandResult::Conflict { error },
        CommandResult::NeedsInput { error } => CommandResult::NeedsInput { error },
        CommandResult::Cancelled => CommandResult::Cancelled,
        CommandResult::Queued { job_id } => CommandResult::Queued { job_id },
    }
}
