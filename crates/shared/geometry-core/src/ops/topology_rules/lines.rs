//! The rules over line work and points (docs/adr/0202 §2): duplicates,
//! dangles, short edges, small angles, ADR 0201's problems, boundaries off
//! another layer's line work, and points off its line ends.

use super::{
    Finding, MeasureKind, Spot, Taken, dist, middle_of, near_all, pairs, sub_edge, within_all,
    written,
};
use crate::entity::Shape;
use crate::geom::arrangement::{edge_box, edge_len};
use crate::geom::intersect::{Edge, closest_on_edge, point_at};
use crate::geometry::Bounds;
use crate::jsmath::{atan2, cos, js_hypot, js_max, js_sign, sin, stable_sort};
use crate::ops::edges::entity_edges;
use crate::ops::geoprocess::validity::{Kind as Problem, problems, repair};
use crate::ops::geoprocess::{Class, boundary_edges};
use crate::vec2::Vec2;

fn paths_of(taken: &[Taken], i: usize) -> &[Vec<Edge>] {
    match &taken[i].class {
        Class::Paths(p) => p,
        _ => &[],
    }
}

fn points_of(taken: &[Taken], i: usize) -> &[Vec2] {
    match &taken[i].class {
        Class::Points(p) => p,
        _ => &[],
    }
}

/// Yinelenmemeli: lines whose edges lie whole within `t` of each other,
/// points within `t` of each other; pair by pair.
pub(super) fn duplicates(
    shapes: &[Shape],
    taken: &[Taken],
    rule: usize,
    idx: &[usize],
    t: f64,
) -> Vec<Finding> {
    let boxes: Vec<Bounds> = idx.iter().map(|&i| taken[i].bounds).collect();
    let mut out = Vec::new();
    for (a, b) in pairs(&boxes, None, t) {
        let (i, j) = (idx[a], idx[b]);
        match (&taken[i].class, &taken[j].class) {
            (Class::Paths(pa), Class::Paths(pb)) => {
                let ea: Vec<Edge> = pa.iter().flatten().copied().collect();
                let eb: Vec<Edge> = pb.iter().flatten().copied().collect();
                let long = |e: &Edge| edge_len(e) > 2.0 * t;
                let da: Vec<Edge> = ea
                    .iter()
                    .filter(|e| long(e) && within_all(e, &eb, t))
                    .copied()
                    .collect();
                let db: Vec<Edge> = eb
                    .iter()
                    .filter(|e| long(e) && within_all(e, &ea, t))
                    .copied()
                    .collect();
                if da.is_empty() && db.is_empty() {
                    continue;
                }
                let la: f64 = da.iter().map(edge_len).sum();
                let lb: f64 = db.iter().map(edge_len).sum();
                let mut longest = da.first().or(db.first()).copied().unwrap_or(Edge::Seg {
                    a: Vec2::default(),
                    b: Vec2::default(),
                });
                for e in da.iter().chain(&db) {
                    if edge_len(e) > edge_len(&longest) {
                        longest = *e;
                    }
                }
                // A line all of whose edges (longer than 2t) lie on the other is the one to delete.
                let whole = |all: &[Edge], dup: &[Edge]| {
                    !dup.is_empty() && dup.len() == all.iter().filter(|e| long(e)).count()
                };
                let subject = if whole(&eb, &db) {
                    Some(j)
                } else if whole(&ea, &da) {
                    Some(i)
                } else {
                    None
                };
                let mut f =
                    Finding::new(rule, "duplicateEdge", vec![i, j], point_at(&longest, 0.5))
                        .measured(MeasureKind::Length, js_max(la, lb))
                        .fixes(if subject.is_some() {
                            &["deleteDuplicate"]
                        } else {
                            &[]
                        })
                        .shown(Vec::new(), da.into_iter().chain(db).collect());
                f.subject = subject;
                out.push(f);
            }
            (Class::Points(qa), Class::Points(qb)) => {
                let mut best: Option<(f64, Vec2)> = None;
                for p in qa {
                    for q in qb {
                        let d = dist(*p, *q);
                        if best.is_none_or(|(bd, _)| d < bd) {
                            best = Some((d, *q));
                        }
                    }
                }
                let Some((d, q)) = best else {
                    continue;
                };
                if d > t {
                    continue;
                }
                let single = matches!(&shapes[j], Shape::Point { parts, .. } if parts.as_ref().is_none_or(Vec::is_empty));
                let mut f = Finding::new(rule, "duplicatePoint", vec![i, j], q)
                    .measured(MeasureKind::Distance, d)
                    .fixes(if single { &["deleteDuplicate"] } else { &[] });
                f.subject = single.then_some(j);
                out.push(f);
            }
            _ => {}
        }
    }
    out
}

/// A connector's edges, each with its path's part and its place in it (none: an area's ring).
type Tagged = Vec<(Edge, Option<(usize, usize)>)>;

/// The distance from `p` to a box (0 inside).
fn box_dist(b: &Bounds, p: Vec2) -> f64 {
    let dx = js_max(js_max(b.min_x - p.x, p.x - b.max_x), 0.0);
    let dy = js_max(js_max(b.min_y - p.y, p.y - b.max_y), 0.0);
    js_hypot(dx, dy)
}

/// Sarkan uç olmamalı: line ends that meet nothing within `t` (docs/adr/0202 §2.5).
pub(super) fn dangles(
    shapes: &[Shape],
    taken: &[Taken],
    rule: usize,
    idx: &[usize],
    areas: &[usize],
    t: f64,
) -> Vec<Finding> {
    // Every connector object of the layer, in the objects' order: its edges, each with its part and place.
    let mut all: Vec<usize> = idx.iter().chain(areas).copied().collect();
    all.sort_unstable();
    let edges_of = |j: usize| -> Tagged {
        match &taken[j].class {
            Class::Paths(p) => p
                .iter()
                .enumerate()
                .flat_map(|(k, path)| {
                    path.iter()
                        .enumerate()
                        .map(move |(e, edge)| (*edge, Some((k, e))))
                })
                .collect(),
            Class::Areas(a) => boundary_edges(a).into_iter().map(|e| (e, None)).collect(),
            _ => Vec::new(),
        }
    };
    let edges: Vec<Tagged> = all.iter().map(|&j| edges_of(j)).collect();
    let mut out = Vec::new();
    for &i in idx {
        let movable = matches!(shapes[i], Shape::Line { .. } | Shape::Polyline { .. });
        for (k, path) in paths_of(taken, i).iter().enumerate() {
            let (Some(first), Some(last)) = (path.first(), path.last()) else {
                continue;
            };
            let (start, end) = (point_at(first, 0.0), point_at(last, 1.0));
            if dist(start, end) <= t {
                continue;
            }
            let n = path.len();
            let ends = [
                (start, 0usize, [0usize, 1]),
                (end, 1, [n - 1, n.saturating_sub(2)]),
            ];
            for (p, which, own) in ends {
                // The nearest place on a connector: objects by how near their box is, the first of equal ones.
                let mut order: Vec<usize> = (0..all.len()).collect();
                stable_sort(&mut order, &mut |&a, &b| {
                    box_dist(&taken[all[a]].bounds, p)
                        .total_cmp(&box_dist(&taken[all[b]].bounds, p))
                        .then(a.cmp(&b))
                });
                let mut best: Option<(f64, usize, Vec2)> = None;
                for &o in &order {
                    if let Some((bd, _, _)) = best
                        && box_dist(&taken[all[o]].bounds, p) > bd
                    {
                        break;
                    }
                    let j = all[o];
                    for (e, tag) in &edges[o] {
                        if j == i && tag.is_some_and(|(pk, pe)| pk == k && own.contains(&pe)) {
                            continue;
                        }
                        let q = closest_on_edge(e, p);
                        let better = match best {
                            None => true,
                            Some((bd, bo, _)) => q.d < bd || (q.d == bd && o < bo),
                        };
                        if better {
                            best = Some((q.d, o, q.p));
                        }
                    }
                }
                if best.is_some_and(|(d, _, _)| d <= t) {
                    continue;
                }
                let mut f = Finding::new(rule, "dangle", vec![i], p);
                match best {
                    Some((d, _, q)) => {
                        f = f.measured(MeasureKind::Distance, d);
                        if movable {
                            f = f.fixes(&["snapEnd"]);
                            f.target = Some(q);
                        }
                    }
                    None => f.measure_kind = Some(MeasureKind::Distance),
                }
                f.spot = Some(Spot {
                    part: k,
                    ring: 0,
                    index: which,
                });
                out.push(f);
            }
        }
    }
    out
}

/// Kısa kenar olmamalı: edges as written shorter than `least`.
pub(super) fn short_edges(
    shapes: &[Shape],
    rule: usize,
    idx: &[usize],
    least: f64,
) -> Vec<Finding> {
    let mut out = Vec::new();
    for &i in idx {
        if let Shape::Arc { .. } = shapes[i] {
            for e in entity_edges(&shapes[i]) {
                let l = edge_len(&e);
                if l < least {
                    out.push(
                        Finding::new(rule, "shortEdge", vec![i], point_at(&e, 0.5))
                            .measured(MeasureKind::Length, l)
                            .fixes(&["deleteObject"])
                            .shown(Vec::new(), vec![e]),
                    );
                }
            }
            continue;
        }
        let rings = written(&shapes[i]);
        let parts = rings.iter().filter(|w| w.ring == 0).count();
        for w in &rings {
            let n = w.pts.len();
            for k in 0..w.edge_count() {
                let e = w.edge(k);
                let l = edge_len(&e);
                if l >= least {
                    continue;
                }
                let mut f = Finding::new(rule, "shortEdge", vec![i], point_at(&e, 0.5))
                    .measured(MeasureKind::Length, l)
                    .shown(Vec::new(), vec![e]);
                if w.closed {
                    if n >= 4 {
                        f = f.fixes(&["removeVertex"]);
                        f.spot = Some(Spot {
                            part: w.part,
                            ring: w.ring,
                            index: (k + 1) % n,
                        });
                    }
                } else if n >= 3 {
                    // An open path keeps its ends: its last edge loses the vertex before the end.
                    let v = if k + 1 == n - 1 { k } else { k + 1 };
                    f = f.fixes(&["removeVertex"]);
                    f.spot = Some(Spot {
                        part: w.part,
                        ring: w.ring,
                        index: v,
                    });
                } else if parts == 1 {
                    f = f.fixes(&["deleteObject"]);
                }
                out.push(f);
            }
        }
    }
    out
}

/// An edge's direction at one of its ends (the way it runs).
fn tangent(e: &Edge, at_end: bool) -> Vec2 {
    match *e {
        Edge::Seg { a, b } => Vec2::new(b.x - a.x, b.y - a.y),
        Edge::Arc { a0, sweep, .. } => {
            let th = if at_end { a0 + sweep } else { a0 };
            let s = js_sign(sweep);
            Vec2::new(-sin(th) * s, cos(th) * s)
        }
    }
}

/// The angle at a vertex between the way back along the edge coming in and
/// the way along the edge going out (0–π).
fn corner(incoming: &Edge, outgoing: &Edge) -> f64 {
    let u = tangent(incoming, true);
    let u = Vec2::new(-u.x, -u.y);
    let w = tangent(outgoing, false);
    atan2((u.x * w.y - u.y * w.x).abs(), u.x * w.x + u.y * w.y)
}

/// Küçük açı olmamalı: corners sharper than `least` (docs/adr/0202 §2.7).
pub(super) fn small_angles(
    shapes: &[Shape],
    rule: usize,
    idx: &[usize],
    least: f64,
    t: f64,
) -> Vec<Finding> {
    let tiny = crate::geom::arrangement::TOL;
    let mut out = Vec::new();
    for &i in idx {
        for w in written(&shapes[i]) {
            let n = w.pts.len();
            if n < 3 {
                continue;
            }
            let closed_path = !w.closed && n >= 4 && dist(w.pts[0], w.pts[n - 1]) <= t;
            for k in 0..n {
                let (incoming, outgoing, fixable) = if w.closed {
                    (w.edge((k + n - 1) % n), w.edge(k), n >= 4)
                } else if k == 0 {
                    if !closed_path {
                        continue;
                    }
                    (w.edge(n - 2), w.edge(0), false)
                } else if k == n - 1 {
                    continue;
                } else {
                    (w.edge(k - 1), w.edge(k), true)
                };
                if edge_len(&incoming) <= tiny || edge_len(&outgoing) <= tiny {
                    continue;
                }
                let a = corner(&incoming, &outgoing);
                if a >= least {
                    continue;
                }
                let mut f = Finding::new(rule, "smallAngle", vec![i], w.pts[k])
                    .measured(MeasureKind::Angle, a);
                if fixable {
                    f = f.fixes(&["removeVertex"]);
                    f.spot = Some(Spot {
                        part: w.part,
                        ring: w.ring,
                        index: k,
                    });
                }
                out.push(f.shown(Vec::new(), vec![incoming, outgoing]));
            }
        }
    }
    out
}

/// Geçerli olmalı: ADR 0201 §6's problems, object by object.
pub(super) fn validity(shapes: &[Shape], rule: usize, idx: &[usize]) -> Vec<Finding> {
    let mut out = Vec::new();
    for &i in idx {
        let found = problems(&shapes[i]);
        if found.is_empty() {
            continue;
        }
        // Onar makes something else of it, and something is left.
        let mended = repair(&shapes[i])
            .shape
            .filter(|s| s != &shapes[i])
            .is_some();
        for p in found {
            let fixes: &[&'static str] = if mended && p.kind != Problem::PathCrossing {
                &["repair"]
            } else {
                &[]
            };
            out.push(Finding::new(rule, p.kind.key(), vec![i], p.at).fixes(fixes));
        }
    }
    out
}

/// Sınırı … sınırlarında olmalı: the stretches of each area's boundary not
/// within `t` of the other layer's line work, joined across vertices.
pub(super) fn boundary_covered(
    taken: &[Taken],
    rule: usize,
    idx: &[usize],
    by: &[usize],
    t: f64,
) -> Vec<Finding> {
    let mut cover: Vec<Edge> = Vec::new();
    for &j in by {
        match &taken[j].class {
            Class::Areas(a) => cover.extend(boundary_edges(a)),
            Class::Paths(p) => cover.extend(p.iter().flatten().copied()),
            _ => {}
        }
    }
    let boxes: Vec<Bounds> = cover.iter().map(edge_box).collect();
    let mut out = Vec::new();
    for &i in idx {
        let Class::Areas(areas) = &taken[i].class else {
            continue;
        };
        let mut runs: Vec<Vec<Edge>> = Vec::new();
        for a in areas {
            for ring in std::iter::once(&a.outer).chain(&a.holes) {
                let edges = crate::geom::region::ring_edges(ring);
                // Each edge's uncovered stretches, in ring order: (edge, s0, s1).
                let mut pieces: Vec<(usize, f64, f64)> = Vec::new();
                for (k, e) in edges.iter().enumerate() {
                    let b = edge_box(e);
                    let near = near_all(
                        e,
                        cover
                            .iter()
                            .zip(&boxes)
                            .filter(|(_, fb)| {
                                fb.min_x <= b.max_x + t
                                    && b.min_x <= fb.max_x + t
                                    && fb.min_y <= b.max_y + t
                                    && b.min_y <= fb.max_y + t
                            })
                            .map(|(f, _)| f),
                        t,
                    );
                    let mut s = 0.0;
                    for (c0, c1) in near {
                        if c0 > s {
                            pieces.push((k, s, c0));
                        }
                        s = js_max(s, c1);
                    }
                    if s < 1.0 {
                        pieces.push((k, s, 1.0));
                    }
                }
                let eps = 1e-9;
                let mut ring_runs: Vec<Vec<(usize, f64, f64)>> = Vec::new();
                for piece in pieces {
                    let joins =
                        ring_runs
                            .last()
                            .and_then(|r| r.last())
                            .is_some_and(|&(k, _, s1)| {
                                k + 1 == piece.0 && s1 >= 1.0 - eps && piece.1 <= eps
                            });
                    if joins {
                        if let Some(r) = ring_runs.last_mut() {
                            r.push(piece);
                        }
                    } else {
                        ring_runs.push(vec![piece]);
                    }
                }
                let n = edges.len();
                if ring_runs.len() > 1 {
                    let wraps = ring_runs
                        .last()
                        .and_then(|r| r.last())
                        .is_some_and(|&(k, _, s1)| k + 1 == n && s1 >= 1.0 - eps)
                        && ring_runs
                            .first()
                            .and_then(|r| r.first())
                            .is_some_and(|&(k, s0, _)| k == 0 && s0 <= eps);
                    if wraps && let Some(mut tail) = ring_runs.pop() {
                        tail.append(&mut ring_runs[0]);
                        ring_runs[0] = tail;
                    }
                }
                for r in ring_runs {
                    let run: Vec<Edge> = r
                        .iter()
                        .map(|&(k, s0, s1)| sub_edge(&edges[k], s0, s1))
                        .collect();
                    if run.iter().map(edge_len).sum::<f64>() > t {
                        runs.push(run);
                    }
                }
            }
        }
        if runs.is_empty() {
            continue;
        }
        let lengths: Vec<f64> = runs.iter().map(|r| r.iter().map(edge_len).sum()).collect();
        let mut long = 0;
        for (k, l) in lengths.iter().enumerate() {
            if *l > lengths[long] {
                long = k;
            }
        }
        let at = middle_of(&runs[long]);
        out.push(
            Finding::new(rule, "uncoveredBoundary", vec![i], at)
                .measured(MeasureKind::Length, lengths.iter().sum())
                .shown(Vec::new(), runs.into_iter().flatten().collect()),
        );
    }
    out
}

/// … çizgilerinin ucunda olmalı: points farther than `t` from every line end of the other layer.
pub(super) fn on_end_of(
    taken: &[Taken],
    rule: usize,
    idx: &[usize],
    by: &[usize],
    t: f64,
) -> Vec<Finding> {
    let mut ends: Vec<Vec2> = Vec::new();
    for &j in by {
        for path in paths_of(taken, j) {
            let (Some(first), Some(last)) = (path.first(), path.last()) else {
                continue;
            };
            let (a, b) = (point_at(first, 0.0), point_at(last, 1.0));
            ends.push(a);
            if dist(a, b) > t {
                ends.push(b);
            }
        }
    }
    let mut out = Vec::new();
    for &i in idx {
        for (k, &p) in points_of(taken, i).iter().enumerate() {
            let mut best: Option<(f64, Vec2)> = None;
            for &q in &ends {
                let d = dist(p, q);
                if best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, q));
                }
            }
            if best.is_some_and(|(d, _)| d <= t) {
                continue;
            }
            let mut f = Finding::new(rule, "notOnEnd", vec![i], p);
            match best {
                Some((d, q)) => {
                    f = f.measured(MeasureKind::Distance, d).fixes(&["snapToEnd"]);
                    f.target = Some(q);
                }
                None => f.measure_kind = Some(MeasureKind::Distance),
            }
            f.spot = Some(Spot {
                part: k,
                ring: 0,
                index: 0,
            });
            out.push(f);
        }
    }
    out
}
