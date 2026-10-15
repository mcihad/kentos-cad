//! `cad.entities.cleanup` v1 (docs/adr/0305; AutoCAD's OVERKILL, Çizimi temizle): empty objects and duplicates within
//! a tolerance removed, lines on one line merged, a path's needless vertices dropped, as one undo step “Çizimi
//! temizle”, with a report of what went and the farthest a point that went lies from what stays. The geometry is the
//! shared core's (`ops::overkill`); the writes go through `cad.entities.edit`'s `cleanup` operation. The shared cases
//! are `fixtures/commands/v1/cad.entities.cleanup.json`.

use std::collections::HashMap;

use kentos_contracts::{
    CLEANUP_TOLERANCE, CleanupIgnore, CleanupReport, CommandResult, CommandWarning, EditOperation,
    EntitiesCleaned, EntitiesCleanup, EntitiesCleanupPlan, EntitiesEdit, Entity, EntityEdit,
    EntityGeometry, LineEntity, MAX_CLEANUP_TOLERANCE, PathEntity, Vec2,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::Vec2 as P;
use kentos_geometry_core::jsmath::js_max;
use kentos_geometry_core::ops::overkill::{Turn, far, kept_bulges, merge_lines, same_points, thin};
use serde_json::Value;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::edit;
use crate::trim::relay;

/// A bulge two paths may differ in and still be the same.
const BULGE: f64 = 1e-9;

/// The base's fields that are not the object's own properties or data.
const NOT_COMPARED: [&str; 2] = ["id", "order"];
/// Every base field, for what is left of an object besides them.
const BASE: [&str; 14] = [
    "id",
    "layerId",
    "color",
    "attrs",
    "label",
    "symbol",
    "lineWeight",
    "labelPins",
    "attachments",
    "lineType",
    "lineTypeScale",
    "transparency",
    "order",
    "byLayer",
];

fn p(v: Vec2) -> P {
    P::new(v.x, v.y)
}

fn ignored_key(i: CleanupIgnore) -> &'static str {
    match i {
        CleanupIgnore::Layer => "layerId",
        CleanupIgnore::Color => "color",
        CleanupIgnore::LineType => "lineType",
        CleanupIgnore::LineTypeScale => "lineTypeScale",
        CleanupIgnore::LineWeight => "lineWeight",
        CleanupIgnore::Transparency => "transparency",
        CleanupIgnore::Symbol => "symbol",
    }
}

/// An object looked at.
struct Item {
    uid: String,
    e: Entity,
    /// Its kind, own properties and data but the ignored ones: duplicates share it.
    key: String,
}

/// The objects looked at and the warnings of the ones left out.
fn scope(
    doc: &Document,
    input: &EntitiesCleanup,
) -> Result<(Vec<Item>, Vec<CommandWarning>), Stop> {
    let key_of = |e: &Entity| {
        let mut base = serde_json::to_value(e.base()).unwrap_or(Value::Null);
        if let Value::Object(map) = &mut base {
            for k in NOT_COMPARED {
                map.remove(k);
            }
            for i in &input.ignore {
                map.remove(ignored_key(*i));
            }
        }
        format!("{}|{}", e.kind(), base)
    };
    let mut warnings = Vec::new();
    let found: Vec<(Slot, String)> = match &input.uids {
        Some(uids) => {
            let named = checks::named(doc, uids)?;
            let (open, locked) = checks::skip_locked(doc, named)?;
            warnings.extend(locked);
            open.into_iter()
                .map(|(_, slot, _, uid)| (slot, uid.clone()))
                .collect()
        }
        None => {
            let layers = doc.layers();
            doc.entities()
                .filter(|e| {
                    let l = &e.base().layer_id;
                    layers.is_visible(l) && !layers.is_locked(l)
                })
                .filter_map(|e| {
                    let slot = Slot(e.base().id);
                    Some((slot, doc.uid(slot)?.to_string()))
                })
                .collect()
        }
    };
    let items = found
        .into_iter()
        .filter_map(|(slot, uid)| {
            let e = doc.get(slot)?.clone();
            let key = key_of(&e);
            Some(Item { uid, e, key })
        })
        .collect();
    Ok((items, warnings))
}

/// How far an empty object reaches: `Some` when it is empty within `tol`.
fn empty(e: &Entity, tol: f64) -> Option<f64> {
    match e {
        Entity::Line(l) => Some(far(p(l.a), p(l.b))).filter(|d| *d <= tol),
        Entity::Circle(c) => Some(c.r).filter(|r| *r <= tol),
        Entity::Arc(a) => Some(a.r).filter(|r| *r <= tol),
        Entity::Polyline(path) | Entity::Polygon(path)
            if path.parts.is_none() && path.holes.is_none() =>
        {
            let first = p(*path.pts.first()?);
            let reach = path
                .pts
                .iter()
                .fold(0.0_f64, |m, q| js_max(m, far(p(*q), first)));
            Some(reach).filter(|d| *d <= tol)
        }
        Entity::Text(t) => t.text.trim().is_empty().then_some(0.0),
        _ => None,
    }
}

/// Two optional numbers within `tol` (both none, or both some).
fn same_opt(a: Option<f64>, b: Option<f64>, tol: f64) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => (x - y).abs() <= tol,
        _ => false,
    }
}

/// What is left of an object besides its base: the fields a duplicate of a kind compared exactly matches.
fn rest(e: &Entity) -> Value {
    let mut v = serde_json::to_value(e).unwrap_or(Value::Null);
    if let Value::Object(map) = &mut v {
        for k in BASE {
            map.remove(k);
        }
    }
    v
}

/// The bulge of edge `i` of `a` where `b` matched `a` by `turn`, read from `b`.
fn matched_bulge(b: &[f64], i: usize, n: usize, turn: Turn) -> f64 {
    let at = |k: usize| b.get(k % n).copied().unwrap_or(0.0);
    match turn {
        Turn::Same => at(i),
        Turn::From(s) => at(s + i),
        Turn::Reversed => -at(2 * n - 2 - i),
        Turn::ReversedFrom(s) => -at(2 * n - 2 - s % n - i),
    }
}

/// Whether two paths are one within `tol`, and how far.
fn same_path(a: &PathEntity, b: &PathEntity, polygon: bool, tol: f64) -> Option<f64> {
    if a.parts.is_some() || b.parts.is_some() || a.holes.is_some() || b.holes.is_some() {
        return None;
    }
    let closed = polygon || a.closed;
    if a.closed != b.closed {
        return None;
    }
    let pa: Vec<P> = a.pts.iter().map(|v| p(*v)).collect();
    let pb: Vec<P> = b.pts.iter().map(|v| p(*v)).collect();
    let (d, turn) = same_points(&pa, &pb, closed, tol)?;
    // Elevations, measures and widths are compared only in order.
    let extras = a.zs.is_some()
        || b.zs.is_some()
        || a.ms.is_some()
        || b.ms.is_some()
        || a.widths.is_some()
        || b.widths.is_some();
    if extras {
        if turn != Turn::Same {
            return None;
        }
        let zs = |x: &Option<Vec<Option<f64>>>, y: &Option<Vec<Option<f64>>>| match (x, y) {
            (None, None) => true,
            (Some(x), Some(y)) => {
                x.len() == y.len() && x.iter().zip(y).all(|(s, t)| same_opt(*s, *t, tol))
            }
            _ => false,
        };
        if !zs(&a.zs, &b.zs) || !zs(&a.ms, &b.ms) || a.widths != b.widths {
            return None;
        }
    }
    let n = pa.len();
    let ba = a.bulges.clone().unwrap_or_default();
    let bb = b.bulges.clone().unwrap_or_default();
    let edges = if closed { n } else { n - 1 };
    for i in 0..edges {
        let x = ba.get(i).copied().unwrap_or(0.0);
        let y = matched_bulge(&bb, i, n, turn);
        if (x - y).abs() > BULGE {
            return None;
        }
    }
    Some(d)
}

/// Whether `b` is `a` within `tol`, and the farthest their points lie apart.
fn same(a: &Entity, b: &Entity, tol: f64) -> Option<f64> {
    match (a, b) {
        (Entity::Point(x), Entity::Point(y)) => {
            if x.parts.is_some() || y.parts.is_some() {
                return (rest(a) == rest(b)).then_some(0.0);
            }
            let d = far(p(x.p), p(y.p));
            (d <= tol && same_opt(x.z, y.z, tol)).then_some(d)
        }
        (Entity::Line(x), Entity::Line(y)) => {
            let (xa, xb, ya, yb) = (p(x.a), p(x.b), p(y.a), p(y.b));
            let forward = js_max(far(xa, ya), far(xb, yb));
            if forward <= tol
                && same_opt(x.za, y.za, tol)
                && same_opt(x.zb, y.zb, tol)
                && same_opt(x.ma, y.ma, tol)
                && same_opt(x.mb, y.mb, tol)
            {
                return Some(forward);
            }
            let back = js_max(far(xa, yb), far(xb, ya));
            (back <= tol
                && same_opt(x.za, y.zb, tol)
                && same_opt(x.zb, y.za, tol)
                && same_opt(x.ma, y.mb, tol)
                && same_opt(x.mb, y.ma, tol))
            .then_some(back)
        }
        (Entity::Circle(x), Entity::Circle(y)) => {
            let d = js_max(far(p(x.c), p(y.c)), (x.r - y.r).abs());
            (d <= tol).then_some(d)
        }
        (Entity::Arc(x), Entity::Arc(y)) => {
            let end = |c: Vec2, r: f64, a: f64| P::new(c.x + r * a.cos(), c.y + r * a.sin());
            let d = js_max(
                js_max(far(p(x.c), p(y.c)), (x.r - y.r).abs()),
                js_max(
                    far(end(x.c, x.r, x.a0), end(y.c, y.r, y.a0)),
                    far(end(x.c, x.r, x.a1), end(y.c, y.r, y.a1)),
                ),
            );
            (d <= tol).then_some(d)
        }
        (Entity::Polyline(x), Entity::Polyline(y)) => same_path(x, y, false, tol),
        (Entity::Polygon(x), Entity::Polygon(y)) => same_path(x, y, true, tol),
        _ => (rest(a) == rest(b)).then_some(0.0),
    }
}

/// The points an object is found by: a duplicate has one of its points in the cell of one of these or a neighbour.
fn anchors(e: &Entity) -> Vec<P> {
    match e {
        Entity::Point(x) => vec![p(x.p)],
        Entity::Line(x) => vec![p(x.a), p(x.b)],
        Entity::Circle(x) => vec![p(x.c)],
        Entity::Arc(x) => vec![p(x.c)],
        Entity::Polyline(x) | Entity::Polygon(x) => {
            let closed = matches!(e, Entity::Polygon(_)) || x.closed;
            if closed {
                x.pts.iter().map(|v| p(*v)).collect()
            } else {
                x.pts
                    .first()
                    .into_iter()
                    .chain(x.pts.last())
                    .map(|v| p(*v))
                    .collect()
            }
        }
        _ => Vec::new(),
    }
}

/// The point a candidate looks up with.
fn probe(e: &Entity) -> Option<P> {
    match e {
        Entity::Point(x) => Some(p(x.p)),
        Entity::Line(x) => Some(p(x.a)),
        Entity::Circle(x) => Some(p(x.c)),
        Entity::Arc(x) => Some(p(x.c)),
        Entity::Polyline(x) | Entity::Polygon(x) => x.pts.first().map(|v| p(*v)),
        _ => None,
    }
}

/// What the cleaning changes: the objects removed and updated, and the report.
struct Plan {
    removed: Vec<usize>,
    updated: Vec<(usize, EntityGeometry)>,
    report: CleanupReport,
}

/// The findings, in the scope's order.
fn find(items: &[Item], input: &EntitiesCleanup, tol: f64) -> Plan {
    let mut report = CleanupReport::default();
    let mut gone = vec![false; items.len()];
    // Empty objects.
    for (i, it) in items.iter().enumerate() {
        if let Some(d) = empty(&it.e, tol) {
            gone[i] = true;
            report.empty += 1;
            report.loss = js_max(report.loss, d);
        }
    }
    // Duplicates: the kept objects by key and cell (geometry within `tol`), or by key and fields (the rest).
    let cell = js_max(2.0 * tol, 1e-6);
    let at = |q: P| ((q.x / cell).floor() as i64, (q.y / cell).floor() as i64);
    let mut grid: HashMap<(&str, i64, i64), Vec<usize>> = HashMap::new();
    let mut exact: HashMap<(&str, String), usize> = HashMap::new();
    for (i, it) in items.iter().enumerate() {
        if gone[i] {
            continue;
        }
        let key = it.key.as_str();
        let found = match probe(&it.e) {
            Some(q) => {
                let (x, y) = at(q);
                let mut near: Vec<usize> = (x - 1..=x + 1)
                    .flat_map(|cx| (y - 1..=y + 1).map(move |cy| (cx, cy)))
                    .filter_map(|(cx, cy)| grid.get(&(key, cx, cy)))
                    .flatten()
                    .copied()
                    .collect();
                near.sort_unstable();
                near.dedup();
                near.into_iter().find_map(|k| same(&items[k].e, &it.e, tol))
            }
            None => exact.get(&(key, rest(&it.e).to_string())).map(|_| 0.0),
        };
        match found {
            Some(d) => {
                gone[i] = true;
                report.duplicates += 1;
                report.loss = js_max(report.loss, d);
            }
            None => match probe(&it.e) {
                Some(_) => {
                    for q in anchors(&it.e) {
                        let (x, y) = at(q);
                        let list = grid.entry((key, x, y)).or_default();
                        if !list.contains(&i) {
                            list.push(i);
                        }
                    }
                }
                None => {
                    exact.insert((key, rest(&it.e).to_string()), i);
                }
            },
        }
    }
    let mut updated: Vec<(usize, EntityGeometry)> = Vec::new();
    // Lines on one line.
    if input.overlaps.unwrap_or(true) {
        let lines: Vec<(usize, &LineEntity)> = items
            .iter()
            .enumerate()
            .filter(|(i, _)| !gone[*i])
            .filter_map(|(i, it)| match &it.e {
                Entity::Line(l)
                    if l.za.is_none() && l.zb.is_none() && l.ma.is_none() && l.mb.is_none() =>
                {
                    Some((i, l))
                }
                _ => None,
            })
            .collect();
        let mut keys: HashMap<&str, u64> = HashMap::new();
        let groups: Vec<u64> = lines
            .iter()
            .map(|(i, _)| {
                let next = keys.len() as u64;
                *keys.entry(items[*i].key.as_str()).or_insert(next)
            })
            .collect();
        let ends: Vec<[P; 2]> = lines.iter().map(|(_, l)| [p(l.a), p(l.b)]).collect();
        for m in merge_lines(&ends, &groups, tol, input.end_to_end.unwrap_or(false)) {
            let kept = lines[m.kept].0;
            for a in &m.absorbed {
                gone[lines[*a].0] = true;
                report.merged += 1;
            }
            report.merging += 1;
            report.loss = js_max(report.loss, m.loss);
            updated.push((
                kept,
                EntityGeometry::Line {
                    a: Vec2 { x: m.a.x, y: m.a.y },
                    b: Vec2 { x: m.b.x, y: m.b.y },
                    zs: None,
                    ms: None,
                },
            ));
        }
    }
    // A path's vertices.
    let straight = input.straight_vertices.unwrap_or(true);
    for (i, it) in items.iter().enumerate() {
        if gone[i] {
            continue;
        }
        let (path, polygon) = match &it.e {
            Entity::Polyline(x) => (x, false),
            Entity::Polygon(x) => (x, true),
            _ => continue,
        };
        if path.parts.is_some() || path.holes.is_some() {
            continue;
        }
        let closed = polygon || path.closed;
        let n = path.pts.len();
        let pts: Vec<P> = path.pts.iter().map(|v| p(*v)).collect();
        let own = |xs: &Option<Vec<Option<f64>>>, k: usize| {
            xs.as_ref()
                .is_some_and(|v| v.get(k).copied().flatten().is_some())
        };
        let widths = path.widths.as_deref();
        let keeps = |k: usize| {
            own(&path.zs, k)
                || own(&path.ms, k)
                || widths.is_some_and(|w| {
                    let before = w.get((k + n - 1) % n).copied().unwrap_or([0.0, 0.0]);
                    let after = w.get(k).copied().unwrap_or([0.0, 0.0]);
                    before[1] != after[0]
                })
        };
        let Some((kept, loss)) = thin(&pts, path.bulges.as_deref(), closed, tol, straight, &keeps)
        else {
            continue;
        };
        report.vertices += (n - kept.len()) as u32;
        report.paths += 1;
        report.loss = js_max(report.loss, loss);
        let bulges = path
            .bulges
            .as_deref()
            .map(|b| kept_bulges(b, &kept, n, closed))
            .filter(|b| b.iter().any(|x| *x != 0.0));
        let pick = |xs: &Option<Vec<Option<f64>>>| {
            xs.as_ref().map(|v| {
                kept.iter()
                    .map(|k| v.get(*k).copied().flatten())
                    .collect::<Vec<_>>()
            })
        };
        let new_pts: Vec<Vec2> = kept.iter().map(|k| path.pts[*k]).collect();
        let geometry = if polygon {
            EntityGeometry::Polygon {
                pts: new_pts,
                bulges,
                holes: None,
                zs: pick(&path.zs),
                parts: None,
            }
        } else {
            let new_widths = widths.map(|w| {
                kept.iter()
                    .enumerate()
                    .map(|(j, &k)| {
                        let last = match kept.get(j + 1) {
                            Some(&next) => next - 1,
                            None if closed => n - 1,
                            None => k,
                        };
                        let start = w.get(k).copied().unwrap_or([0.0, 0.0]);
                        let end = w.get(last).copied().unwrap_or([0.0, 0.0]);
                        [start[0], end[1]]
                    })
                    .collect()
            });
            EntityGeometry::Polyline {
                pts: new_pts,
                bulges,
                zs: pick(&path.zs),
                ms: pick(&path.ms),
                parts: None,
                closed: path.closed,
                widths: new_widths,
            }
        };
        updated.push((i, geometry));
    }
    updated.sort_by_key(|(i, _)| *i);
    Plan {
        removed: (0..items.len()).filter(|i| gone[*i]).collect(),
        updated,
        report,
    }
}

/// What the checks give: the edit, its report and warnings, and the ids changed and removed in the scope's order.
struct Checked {
    change: EntitiesEdit,
    report: CleanupReport,
    warnings: Vec<CommandWarning>,
    changed: Vec<String>,
    removed: Vec<String>,
}

/// The checks in the contract's order, and the edit with its report.
fn check(doc: &Document, input: &EntitiesCleanup) -> Result<Checked, Stop> {
    if let Some(uids) = &input.uids {
        checks::uids(uids, "Temizlenecek nesne verilmedi.")?;
    }
    let tol = input.tolerance.unwrap_or(CLEANUP_TOLERANCE);
    checks::finite(
        tol,
        "Tolerans",
        "Toleransı sonlu bir sayıyla verin.",
        "tolerance",
    )?;
    if !(0.0..=MAX_CLEANUP_TOLERANCE).contains(&tol) {
        return Err(Stop::failed(error(
            codes::INVALID_TOLERANCE,
            format!(
                "Tolerans 0 ile {MAX_CLEANUP_TOLERANCE} m arasında olmalı. Başka bir tolerans verin."
            ),
            Some("tolerance".into()),
        )));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let (items, warnings) = scope(doc, input)?;
    let plan = find(&items, input, tol);
    let removed: Vec<String> = plan.removed.iter().map(|i| items[*i].uid.clone()).collect();
    let changed: Vec<String> = plan
        .updated
        .iter()
        .map(|(i, _)| items[*i].uid.clone())
        .collect();
    let mut changes: Vec<EntityEdit> = removed
        .iter()
        .map(|uid| EntityEdit::Remove { uid: uid.clone() })
        .collect();
    changes.extend(
        plan.updated
            .into_iter()
            .map(|(i, geometry)| EntityEdit::Update {
                uid: items[i].uid.clone(),
                geometry,
            }),
    );
    Ok(Checked {
        change: EntitiesEdit {
            operation: EditOperation::Cleanup,
            changes,
            expected_revision: None,
        },
        report: plan.report,
        warnings,
        changed,
        removed,
    })
}

/// `cad.entities.cleanup` v1: Çizimi temizle.
pub mod entities_cleanup {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesCleanup) -> CommandResult<()> {
        match check(cx.doc, input) {
            Ok(checked) => CommandResult::Completed {
                output: (),
                warnings: checked.warnings,
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &EntitiesCleanup,
    ) -> CommandResult<EntitiesCleanupPlan> {
        plan_in(cx.doc, input)
    }

    /// [`plan`] over the document alone (the tool's preview).
    pub fn plan_in(doc: &Document, input: &EntitiesCleanup) -> CommandResult<EntitiesCleanupPlan> {
        let Checked {
            change,
            report,
            warnings,
            removed,
            ..
        } = match check(doc, input) {
            Ok(checked) => checked,
            Err(stop) => return stop.into(),
        };
        if change.changes.is_empty() {
            return CommandResult::Completed {
                output: EntitiesCleanupPlan {
                    changed: Vec::new(),
                    removed: Vec::new(),
                    report,
                    revision: doc.revision().to_string(),
                },
                warnings,
            };
        }
        match edit::plan_in(doc, &change) {
            CommandResult::Completed {
                output,
                warnings: w,
            } => CommandResult::Completed {
                output: EntitiesCleanupPlan {
                    changed: output.changed,
                    removed,
                    report,
                    revision: doc.revision().to_string(),
                },
                warnings: [warnings, w].concat(),
            },
            other => relay(other),
        }
    }

    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: EntitiesCleanup,
    ) -> CommandResult<EntitiesCleaned> {
        crate::call::recorded(
            cx,
            kentos_contracts::CAD_ENTITIES_CLEANUP,
            kentos_contracts::CAD_ENTITIES_CLEANUP_VERSION,
            input,
            |cx, input| {
                let Checked {
                    change,
                    report,
                    warnings,
                    changed,
                    removed,
                } = match check(cx.doc, &input) {
                    Ok(checked) => checked,
                    Err(stop) => return stop.into(),
                };
                if change.changes.is_empty() {
                    return CommandResult::Completed {
                        output: EntitiesCleaned {
                            changed,
                            removed,
                            report,
                            revision: cx.doc.revision().to_string(),
                        },
                        warnings,
                    };
                }
                match edit::execute(cx, change) {
                    CommandResult::Completed { warnings: w, .. } => CommandResult::Completed {
                        output: EntitiesCleaned {
                            changed,
                            removed,
                            report,
                            revision: cx.doc.revision().to_string(),
                        },
                        warnings: [warnings, w].concat(),
                    },
                    other => relay(other),
                }
            },
        )
    }
}
