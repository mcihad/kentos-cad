//! Topological editing (docs/adr/0160), one for both platforms (the web
//! through WASM): the neighbours of an edited vertex or edge put right with
//! it, so that shared corners and edges stay shared. Nothing is computed:
//! vertices take the changes' points, edges their bulges, so both platforms
//! give the same bits. The independent reference is
//! `scripts/fixtures/topology_edit_cases.py`.

use crate::api::Op;
use crate::api::json::{FromJson, Json, ToJson, field, read_field};
use crate::entity::{Part, Shape};
use crate::geom::arrangement::Ring;
use crate::geometry::dist;
use crate::op;
use crate::vec2::Vec2;

/// Two places this close are one (the elevations' “at a vertex”, docs/adr/0142).
pub const SAME: f64 = 1e-6;

/// Two bulges this close are the same arc's.
const BULGE: f64 = 1e-9;

/// A bulge this small is a straight edge (`geom::bulge`).
const STRAIGHT: f64 = 1e-12;

/// An object near the edited one: its shape and whether its layer is locked.
#[derive(Clone, Debug, PartialEq)]
pub struct Neighbour {
    pub shape: Shape,
    pub locked: bool,
}

impl FromJson for Neighbour {
    fn from_json(v: &Json) -> Result<Neighbour, String> {
        Ok(Neighbour {
            shape: read_field(v, "shape")?,
            locked: read_field::<Option<bool>>(v, "locked")?.unwrap_or(false),
        })
    }
}

/// What the edit did at shared places: the moves happen together, the other
/// changes one after the other.
#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    /// A vertex moved from `at` to `to`.
    Move { at: Vec2, to: Vec2 },
    /// `p` added on the straight edge from `a` to `b`.
    Insert { a: Vec2, b: Vec2, p: Vec2 },
    /// The edge from `a` to `b` reshaped: its bulge `from` became `to`.
    Bulge {
        a: Vec2,
        b: Vec2,
        from: f64,
        to: f64,
    },
    /// The vertex at `at` removed, its path neighbours `prev` and `next`.
    Remove { at: Vec2, prev: Vec2, next: Vec2 },
}

crate::json_tagged!(Change, "kind",
    Move => "move" { at, to },
    Insert => "insert" { a, b, p },
    Bulge => "bulge" { a, b, from, to },
    Remove => "remove" { at, prev, next },
);

/// The neighbours put right (their indices and shapes), how many would have
/// changed but lie on a locked layer, and how many would have been left
/// invalid (too few vertices).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Answer {
    pub edited: Vec<(usize, Shape)>,
    pub locked: usize,
    pub invalid: usize,
}

impl ToJson for Answer {
    fn write_json(&self, out: &mut String) {
        out.push_str("{\"edited\":[");
        for (k, (i, shape)) in self.edited.iter().enumerate() {
            if k > 0 {
                out.push(',');
            }
            out.push('{');
            let mut first = true;
            field(out, &mut first, "index", &(*i as f64));
            field(out, &mut first, "shape", shape);
            out.push('}');
        }
        out.push(']');
        let mut first = false;
        field(out, &mut first, "locked", &(self.locked as f64));
        field(out, &mut first, "invalid", &(self.invalid as f64));
        out.push('}');
    }
}

/// A path of a neighbour: a line's two ends, a polyline, or one of an
/// area's rings and holes.
pub(crate) struct Path {
    pub(crate) pts: Vec<Vec2>,
    pub(crate) bulges: Vec<f64>,
    pub(crate) closed: bool,
}

/// How the paths go back into a shape: a line, a polyline (with its holes
/// as they were), or an area's parts with how many holes each has.
pub(crate) enum Plan {
    Line,
    /// A polyline: its holes (kept as they are) and how many parts it has past its first (docs/adr/0174).
    Polyline(Option<Vec<Ring>>, usize),
    Polygon(Vec<usize>),
}

pub(crate) fn padded(bulges: &Option<Vec<f64>>, n: usize) -> Vec<f64> {
    let mut b = bulges.clone().unwrap_or_default();
    b.resize(n, 0.0);
    b
}

/// A neighbour's paths in the elevations' order: an area's outer ring, its
/// holes, then each further part's ring and holes.
pub(crate) fn paths_of(shape: &Shape) -> Option<(Vec<Path>, Plan)> {
    match shape {
        Shape::Line { a, b } => Some((
            vec![Path {
                pts: vec![*a, *b],
                bulges: vec![0.0],
                closed: false,
            }],
            Plan::Line,
        )),
        // A multi-part polyline's parts after its own path (docs/adr/0174).
        // Its widths are the application's to carry (docs/adr/0258 §2).
        Shape::Polyline {
            pts,
            bulges,
            holes,
            parts,
            closed,
            ..
        } => {
            let path = |pts: &Vec<Vec2>, bulges: &Option<Vec<f64>>| Path {
                pts: pts.clone(),
                bulges: padded(bulges, pts.len().saturating_sub(1)),
                closed: false,
            };
            // A closed line's first path is a ring, its closing edge's bulge with it (docs/adr/0258 §1).
            let first = if *closed {
                Path {
                    pts: pts.clone(),
                    bulges: padded(bulges, pts.len()),
                    closed: true,
                }
            } else {
                path(pts, bulges)
            };
            let mut out = vec![first];
            out.extend(parts.iter().flatten().map(|q| path(&q.pts, &q.bulges)));
            let n = out.len() - 1;
            Some((out, Plan::Polyline(holes.clone(), n)))
        }
        Shape::Polygon {
            pts,
            bulges,
            holes,
            parts,
        } => {
            let mut out = Vec::new();
            let mut plan = Vec::new();
            let mut ring =
                |pts: &Vec<Vec2>, bulges: &Option<Vec<f64>>, holes: &Option<Vec<Ring>>| {
                    out.push(Path {
                        pts: pts.clone(),
                        bulges: padded(bulges, pts.len()),
                        closed: true,
                    });
                    let holes = holes.as_deref().unwrap_or_default();
                    for h in holes {
                        out.push(Path {
                            pts: h.pts.clone(),
                            bulges: padded(&h.bulges, h.pts.len()),
                            closed: true,
                        });
                    }
                    plan.push(holes.len());
                };
            ring(pts, bulges, holes);
            for part in parts.iter().flatten() {
                ring(&part.pts, &part.bulges, &part.holes);
            }
            Some((out, Plan::Polygon(plan)))
        }
        _ => None,
    }
}

pub(crate) fn shape_of(paths: Vec<Path>, plan: Plan) -> Shape {
    let mut paths = paths.into_iter();
    match plan {
        Plan::Line | Plan::Polyline(..) => {
            let Path {
                pts,
                bulges,
                closed,
            } = paths.next().unwrap_or(Path {
                pts: Vec::new(),
                bulges: Vec::new(),
                closed: false,
            });
            match plan {
                Plan::Line if pts.len() == 2 && bulges.iter().all(|&b| b == 0.0) => Shape::Line {
                    a: pts[0],
                    b: pts[1],
                },
                Plan::Polyline(holes, n) => {
                    let parts: Vec<Part> = paths
                        .by_ref()
                        .take(n)
                        .map(|q| Part {
                            pts: q.pts,
                            bulges: Some(q.bulges),
                            holes: None,
                        })
                        .collect();
                    Shape::Polyline {
                        pts,
                        bulges: Some(bulges),
                        holes,
                        parts: (n > 0).then_some(parts),
                        closed,
                        widths: None,
                    }
                }
                _ => Shape::Polyline {
                    pts,
                    bulges: Some(bulges),
                    holes: None,
                    parts: None,
                    closed: false,
                    widths: None,
                },
            }
        }
        Plan::Polygon(holes_per) => {
            let mut rings = Vec::with_capacity(holes_per.len());
            for count in holes_per {
                let ring = paths.next();
                let holes: Vec<Ring> = (0..count)
                    .filter_map(|_| paths.next())
                    .map(|h| Ring {
                        pts: h.pts,
                        bulges: Some(h.bulges),
                    })
                    .collect();
                if let Some(ring) = ring {
                    rings.push((ring, (!holes.is_empty()).then_some(holes)));
                }
            }
            let mut rings = rings.into_iter();
            let Some((first, holes)) = rings.next() else {
                return Shape::Polygon {
                    pts: Vec::new(),
                    bulges: None,
                    holes: None,
                    parts: None,
                };
            };
            let parts: Vec<Part> = rings
                .map(|(r, h)| Part {
                    pts: r.pts,
                    bulges: Some(r.bulges),
                    holes: h,
                })
                .collect();
            Shape::Polygon {
                pts: first.pts,
                bulges: Some(first.bulges),
                holes,
                parts: (!parts.is_empty()).then_some(parts),
            }
        }
    }
}

/// Every vertex of a shape: a path's and an area's with their holes' and parts', a line's two ends, a point's (and its
/// parts') only with `points`.
fn vertices(shape: &Shape, points: bool) -> Vec<Vec2> {
    let mut out = Vec::new();
    match shape {
        Shape::Line { a, b } => {
            out.push(*a);
            out.push(*b);
        }
        Shape::Polyline {
            pts, holes, parts, ..
        }
        | Shape::Polygon {
            pts, holes, parts, ..
        } => {
            out.extend_from_slice(pts);
            for h in holes.iter().flatten() {
                out.extend_from_slice(&h.pts);
            }
            for p in parts.iter().flatten() {
                out.extend_from_slice(&p.pts);
                for h in p.holes.iter().flatten() {
                    out.extend_from_slice(&h.pts);
                }
            }
        }
        Shape::Point { p, parts, .. } if points => {
            out.push(*p);
            out.extend(parts.iter().flatten().map(|q| q.p));
        }
        _ => {}
    }
    out
}

/// The places of `neighbours`' vertices strictly inside the straight edge from `a` to `b`: within [`SAME`] of it and
/// farther than [`SAME`] from both its ends, each place once, in the neighbours' order. Paralel kaydır carries them with
/// a shared edge (docs/adr/2600): a third object's corner on the edge (a T junction) and a neighbour's own vertex
/// along it. A point counts only with `points`.
pub fn inside_segment(neighbours: &[Neighbour], a: Vec2, b: Vec2, points: bool) -> Vec<Vec2> {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l2 = dx * dx + dy * dy;
    let mut out: Vec<Vec2> = Vec::new();
    if !(l2 > 0.0) {
        return out;
    }
    let l = l2.sqrt();
    for n in neighbours {
        for v in vertices(&n.shape, points) {
            let along = ((v.x - a.x) * dx + (v.y - a.y) * dy) / l;
            let off = ((v.x - a.x) * dy - (v.y - a.y) * dx).abs() / l;
            if off <= SAME && along > SAME && l - along > SAME && !out.iter().any(|q| same(*q, v)) {
                out.push(v);
            }
        }
    }
    out
}

/// The same inside an arc edge (centre `c`, radius `r`, from angle `a0` sweeping `sweep`, signed, between its ends `a`
/// and `b`): the places within [`SAME`] of the arc, farther than [`SAME`] from both ends along it.
pub fn inside_arc(neighbours: &[Neighbour], c: Vec2, r: f64, a0: f64, sweep: f64, points: bool) -> Vec<Vec2> {
    use crate::geom::arc::norm_angle;
    let mut out: Vec<Vec2> = Vec::new();
    let span = sweep.abs();
    for n in neighbours {
        for v in vertices(&n.shape, points) {
            if (dist(v, c) - r).abs() > SAME {
                continue;
            }
            let theta = crate::jsmath::atan2(v.y - c.y, v.x - c.x);
            let d = if sweep > 0.0 { norm_angle(theta - a0) } else { norm_angle(a0 - theta) };
            if d < span && d * r > SAME && (span - d) * r > SAME && !out.iter().any(|q| same(*q, v)) {
                out.push(v);
            }
        }
    }
    out
}

fn same(p: Vec2, q: Vec2) -> bool {
    dist(p, q) <= SAME
}

/// The neighbours put right by `changes` (docs/adr/0160 §2–§3); points
/// take part only with `points`.
pub fn apply(neighbours: &[Neighbour], changes: &[Change], points: bool) -> Answer {
    let moves: Vec<(Vec2, Vec2)> = changes
        .iter()
        .filter_map(|c| match c {
            Change::Move { at, to } => Some((*at, *to)),
            _ => None,
        })
        .collect();
    let mut answer = Answer::default();
    for (i, n) in neighbours.iter().enumerate() {
        if let Shape::Point { p, z, parts } = &n.shape {
            if !points {
                continue;
            }
            // A multi-point object's points move each on its own (docs/adr/0174).
            let to_of = |q: Vec2| moves.iter().find(|(at, _)| same(q, *at)).map(|&(_, to)| to);
            let first = to_of(*p);
            let others: Vec<Option<Vec2>> = parts.iter().flatten().map(|q| to_of(q.p)).collect();
            if first.is_none() && others.iter().all(Option::is_none) {
                continue;
            }
            if n.locked {
                answer.locked += 1;
            } else {
                let parts = parts.as_ref().map(|ps| {
                    ps.iter()
                        .zip(&others)
                        .map(|(q, to)| crate::entity::PointPart {
                            p: to.unwrap_or(q.p),
                            z: q.z,
                        })
                        .collect()
                });
                answer.edited.push((
                    i,
                    Shape::Point {
                        p: first.unwrap_or(*p),
                        z: *z,
                        parts,
                    },
                ));
            }
            continue;
        }
        let Some((mut paths, plan)) = paths_of(&n.shape) else {
            continue;
        };
        let (mut changed, mut bad) = (false, false);
        // The moves together, against the places as they were.
        for path in &mut paths {
            for v in &mut path.pts {
                if let Some(&(_, to)) = moves.iter().find(|(at, _)| same(*v, *at)) {
                    *v = to;
                    changed = true;
                }
            }
        }
        for c in changes {
            for path in &mut paths {
                let len = path.pts.len();
                let count = if path.closed {
                    len
                } else {
                    len.saturating_sub(1)
                };
                match *c {
                    Change::Move { .. } => {}
                    Change::Insert { a, b, p } => {
                        for e in (0..count).rev() {
                            let (u, w) = (path.pts[e], path.pts[(e + 1) % len]);
                            let on = (same(u, a) && same(w, b)) || (same(u, b) && same(w, a));
                            if on && path.bulges[e].abs() <= STRAIGHT {
                                path.pts.insert(e + 1, p);
                                path.bulges.splice(e..=e, [0.0, 0.0]);
                                changed = true;
                            }
                        }
                    }
                    Change::Bulge { a, b, from, to } => {
                        for e in 0..count {
                            let (u, w) = (path.pts[e], path.pts[(e + 1) % len]);
                            if same(u, a) && same(w, b) && (path.bulges[e] - from).abs() <= BULGE {
                                path.bulges[e] = to;
                                changed = true;
                            } else if same(u, b)
                                && same(w, a)
                                && (path.bulges[e] + from).abs() <= BULGE
                            {
                                path.bulges[e] = if to == 0.0 { 0.0 } else { -to };
                                changed = true;
                            }
                        }
                    }
                    Change::Remove { at, prev, next } => {
                        for k in 0..len {
                            if !same(path.pts[k], at) {
                                continue;
                            }
                            let (before, after) = if path.closed {
                                (path.pts[(k + len - 1) % len], path.pts[(k + 1) % len])
                            } else if k == 0 || k == len - 1 {
                                continue;
                            } else {
                                (path.pts[k - 1], path.pts[k + 1])
                            };
                            let chain = (same(before, prev) && same(after, next))
                                || (same(before, next) && same(after, prev));
                            if !chain {
                                continue;
                            }
                            if len - 1 < if path.closed { 3 } else { 2 } {
                                bad = true;
                                break;
                            }
                            let prev_edge = if path.closed {
                                (k + len - 1) % len
                            } else {
                                k - 1
                            };
                            path.bulges[prev_edge] = 0.0;
                            path.pts.remove(k);
                            let gone = if k < path.bulges.len() {
                                k
                            } else {
                                path.bulges.len() - 1
                            };
                            path.bulges.remove(gone);
                            changed = true;
                            break;
                        }
                    }
                }
            }
        }
        if !changed && !bad {
            continue;
        }
        if n.locked {
            answer.locked += 1;
        } else if bad {
            answer.invalid += 1;
        } else {
            answer.edited.push((i, shape_of(paths, plan)));
        }
    }
    answer
}

/// The changes an edit made, from the edited object before and after it
/// (docs/adr/0160 §7), path by path in the elevations' order: with as many
/// vertices, every vertex that changed place is a move and every edge whose
/// bulge changed a bulge, named by its ends after the edit (the moves come
/// first: an arc shifted parallel moves its ends and turns its bulge, and a
/// neighbour sharing it takes both, docs/adr/1960); with one vertex more, the
/// first that differs is an insert on the edge it splits; with one fewer, a
/// remove with its two path neighbours (neither at an open path's end). The
/// moves come first. A point is a corner of its own (Noktalar da): moved, it
/// is a move of its place.
pub fn changes(before: &Shape, after: &Shape) -> Vec<Change> {
    if let (Shape::Point { p: at, .. }, Shape::Point { p: to, .. }) = (before, after) {
        return if at == to {
            Vec::new()
        } else {
            vec![Change::Move { at: *at, to: *to }]
        };
    }
    let (Some((pb, _)), Some((pa, _))) = (paths_of(before), paths_of(after)) else {
        return Vec::new();
    };
    if pb.len() != pa.len() {
        return Vec::new();
    }
    let (mut moves, mut others) = (Vec::new(), Vec::new());
    for (b, a) in pb.iter().zip(&pa) {
        let (n, m) = (b.pts.len(), a.pts.len());
        if m == n {
            for k in 0..n {
                if b.pts[k] != a.pts[k] {
                    moves.push(Change::Move {
                        at: b.pts[k],
                        to: a.pts[k],
                    });
                }
            }
            let count = if b.closed { n } else { n.saturating_sub(1) };
            for e in 0..count {
                if b.bulges[e] != a.bulges[e] {
                    others.push(Change::Bulge {
                        a: a.pts[e],
                        b: a.pts[(e + 1) % n],
                        from: b.bulges[e],
                        to: a.bulges[e],
                    });
                }
            }
        } else if m == n + 1 {
            let j = (0..n).find(|&k| b.pts[k] != a.pts[k]).unwrap_or(n);
            if b.closed {
                others.push(Change::Insert {
                    a: b.pts[(j + n - 1) % n],
                    b: b.pts[j % n],
                    p: a.pts[j],
                });
            } else if j > 0 && j < n {
                others.push(Change::Insert {
                    a: b.pts[j - 1],
                    b: b.pts[j],
                    p: a.pts[j],
                });
            }
        } else if m + 1 == n {
            let j = (0..m).find(|&k| b.pts[k] != a.pts[k]).unwrap_or(m);
            if b.closed {
                others.push(Change::Remove {
                    at: b.pts[j],
                    prev: b.pts[(j + n - 1) % n],
                    next: b.pts[(j + 1) % n],
                });
            } else if j > 0 && j + 1 < n {
                others.push(Change::Remove {
                    at: b.pts[j],
                    prev: b.pts[j - 1],
                    next: b.pts[j + 1],
                });
            }
        }
    }
    moves.extend(others);
    moves
}

/// An edge of a path: its two ends and its bulge.
pub type Span = (Vec2, Vec2, f64);

/// The edges an edit changed (docs/adr/3400), as they were and as they are, path by path in the elevations' order:
/// on a path with as many vertices after as before, every edge whose ends moved or whose bulge changed. A path that
/// gained or lost a vertex gives none.
pub fn changed_edges(before: &Shape, after: &Shape) -> Vec<(Span, Span)> {
    let (Some((pb, _)), Some((pa, _))) = (paths_of(before), paths_of(after)) else {
        return Vec::new();
    };
    if pb.len() != pa.len() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (b, a) in pb.iter().zip(&pa) {
        let n = b.pts.len();
        if a.pts.len() != n {
            continue;
        }
        let count = if b.closed { n } else { n.saturating_sub(1) };
        for e in 0..count {
            let old = (b.pts[e], b.pts[(e + 1) % n], b.bulges[e]);
            let new = (a.pts[e], a.pts[(e + 1) % n], a.bulges[e]);
            if old != new {
                out.push((old, new));
            }
        }
    }
    out
}

/// A place on an edge as the fraction of the edge's length from its start: by the length along a straight edge, by
/// the angle round an arc. The caller knows the place lies on the edge.
fn fraction(edge: Span, v: Vec2) -> f64 {
    let (a, b, bulge) = edge;
    if bulge.abs() > STRAIGHT
        && let Some(arc) = crate::geom::bulge::bulge_arc(a, b, bulge)
    {
        use crate::geom::arc::norm_angle;
        let theta = crate::jsmath::atan2(v.y - arc.c.y, v.x - arc.c.x);
        let d = if arc.sweep > 0.0 { norm_angle(theta - arc.a0) } else { norm_angle(arc.a0 - theta) };
        return d / arc.sweep.abs();
    }
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    ((v.x - a.x) * dx + (v.y - a.y) * dy) / (dx * dx + dy * dy)
}

/// The place at fraction `t` of an edge's length (its own ends at 0 and 1).
fn at_fraction(edge: Span, t: f64) -> Vec2 {
    let (a, b, bulge) = edge;
    if t == 0.0 {
        return a;
    }
    if t == 1.0 {
        return b;
    }
    if bulge.abs() > STRAIGHT
        && let Some(arc) = crate::geom::bulge::bulge_arc(a, b, bulge)
    {
        let angle = arc.a0 + arc.sweep * t;
        return Vec2::new(arc.c.x + arc.r * crate::jsmath::cos(angle), arc.c.y + arc.r * crate::jsmath::sin(angle));
    }
    Vec2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

/// The bulge of the part of an edge from fraction `t1` to fraction `t2` (the other way round: the opposite sign).
fn part_bulge(edge: Span, t1: f64, t2: f64) -> f64 {
    let (a, b, bulge) = edge;
    match crate::geom::bulge::bulge_arc(a, b, bulge) {
        Some(arc) if bulge.abs() > STRAIGHT => crate::geom::bulge::bulge_of_sweep(arc.sweep * (t2 - t1)),
        _ => 0.0,
    }
}

/// What the edges an edit changed carry (docs/adr/3400; ArcGIS Pro's “stretch topology proportionately”: the relative
/// lengths of the connected edges kept). The neighbours' vertices strictly inside an edge as it was (a third object's
/// corner on it, a T junction; a neighbour's own vertex along it; a point only with `points`) go to the same fraction
/// of the edge as it is: moves. Where the edge's bulge changed, a neighbour's edge along it between two such places (or
/// such a place and an end of the edge) takes the bulge of the same part of the edge as it is: bulge changes named by
/// their ends after the edit. Each place once; the edges in `edges`' order.
pub fn carried(neighbours: &[Neighbour], edges: &[(Span, Span)], points: bool) -> Vec<Change> {
    let mut moves: Vec<Change> = Vec::new();
    let mut bends: Vec<Change> = Vec::new();
    let mut seen: Vec<Vec2> = Vec::new();
    for &(old, new) in edges {
        let (a, b, bulge) = old;
        let inside = if bulge.abs() <= STRAIGHT {
            inside_segment(neighbours, a, b, points)
        } else {
            match crate::geom::bulge::bulge_arc(a, b, bulge) {
                Some(arc) => inside_arc(neighbours, arc.c, arc.r, arc.a0, arc.sweep, points),
                None => Vec::new(),
            }
        };
        if inside.is_empty() {
            continue;
        }
        let places: Vec<(Vec2, f64)> = inside.iter().map(|&v| (v, fraction(old, v))).collect();
        for &(v, t) in &places {
            if seen.iter().any(|q| same(*q, v)) {
                continue;
            }
            seen.push(v);
            let to = at_fraction(new, t);
            if to != v {
                moves.push(Change::Move { at: v, to });
            }
        }
        let bent = (new.2 - bulge).abs() > BULGE || ((new.2.abs() > STRAIGHT) != (bulge.abs() > STRAIGHT));
        if !bent {
            continue;
        }
        // Where a neighbour's vertex lies on the edge: its fraction, the ends at 0 and 1.
        let on = |v: Vec2| -> Option<(f64, bool)> {
            if same(v, a) {
                return Some((0.0, false));
            }
            if same(v, b) {
                return Some((1.0, false));
            }
            places.iter().find(|(q, _)| same(*q, v)).map(|&(_, t)| (t, true))
        };
        for n in neighbours {
            let Some((paths, _)) = paths_of(&n.shape) else {
                continue;
            };
            for path in &paths {
                let len = path.pts.len();
                let count = if path.closed { len } else { len.saturating_sub(1) };
                for e in 0..count {
                    let (u, w, be) = (path.pts[e], path.pts[(e + 1) % len], path.bulges[e]);
                    let (Some((tu, iu)), Some((tw, iw))) = (on(u), on(w)) else {
                        continue;
                    };
                    // The whole edge shared is the edit's own bulge change; a part needs an inside end.
                    if !(iu || iw) || tu == tw || (be - part_bulge(old, tu, tw)).abs() > BULGE {
                        continue;
                    }
                    let (ua, wa) = (at_fraction(new, tu), at_fraction(new, tw));
                    let to = part_bulge(new, tu, tw);
                    let known = bends.iter().any(|c| match *c {
                        Change::Bulge { a: x, b: y, .. } => (same(x, ua) && same(y, wa)) || (same(x, wa) && same(y, ua)),
                        _ => false,
                    });
                    if !known {
                        bends.push(Change::Bulge { a: ua, b: wa, from: be, to });
                    }
                }
            }
        }
    }
    moves.extend(bends);
    moves
}

pub(crate) static OPS: &[Op] = &[
    op!("topologyEdit", |neighbours: Vec<Neighbour>,
                         changes: Vec<Change>,
                         points: bool| {
        apply(&neighbours, &changes, points)
    }),
    op!("topologyChanges", |before: Shape, after: Shape| changes(
        &before, &after
    )),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn line(a: (f64, f64), b: (f64, f64)) -> Neighbour {
        Neighbour { shape: Shape::Line { a: Vec2::new(a.0, a.1), b: Vec2::new(b.0, b.1) }, locked: false }
    }

    /// docs/adr/3400: an edge turned about its start carries a T junction's corner at the same fraction of it; an edge
    /// of the edit that did not change carries nothing; a polygon's moved corner changes its two edges.
    #[test]
    fn a_turned_edge_carries_the_corner_inside_it_at_its_fraction() {
        let before = Shape::Polygon {
            pts: vec![Vec2::new(0.0, 0.0), Vec2::new(40.0, 0.0), Vec2::new(40.0, 30.0), Vec2::new(0.0, 30.0)],
            bulges: None,
            holes: None,
            parts: None,
        };
        let mut after = before.clone();
        if let Shape::Polygon { pts, .. } = &mut after {
            pts[1] = Vec2::new(40.0, 4.0);
        }
        let edges = changed_edges(&before, &after);
        assert_eq!(edges.len(), 2, "{edges:?}");
        let arm = line((10.0, -20.0), (10.0, 0.0));
        let far = line((-10.0, 15.0), (0.0, 15.0));
        let got = carried(&[arm, far], &edges, false);
        assert_eq!(got, vec![Change::Move { at: Vec2::new(10.0, 0.0), to: Vec2::new(10.0, 1.0) }]);
    }

    /// The same round an arc whose end moved, its bulge kept: the vertex at the arc's middle goes to the new arc's
    /// middle, on its circle; no bulge changes (the parts of an arc of the same sweep keep theirs).
    #[test]
    fn an_arc_keeps_the_vertex_at_its_fraction() {
        let old = (Vec2::new(0.0, 0.0), Vec2::new(20.0, 0.0), 1.0);
        let new = (Vec2::new(0.0, 0.0), Vec2::new(30.0, 0.0), 1.0);
        let n = Neighbour {
            shape: Shape::Polyline {
                pts: vec![Vec2::new(10.0, -10.0), Vec2::new(20.0, 0.0)],
                bulges: Some(vec![crate::jsmath::tan(crate::jsmath::PI / 8.0)]),
                holes: None,
                parts: None,
                closed: false,
                widths: None,
            },
            locked: false,
        };
        let got = carried(&[n], &[(old, new)], false);
        let [Change::Move { at, to }] = got.as_slice() else { panic!("{got:?}") };
        assert!(same(*at, Vec2::new(10.0, -10.0)));
        assert!((to.x - 15.0).abs() < 1e-9 && (to.y + 15.0).abs() < 1e-9, "{to:?}");
    }

    /// A straight edge made an arc (bulge 0.5: its middle 10 m off the chord of 40 m): the T junction at its middle goes to
    /// the arc's middle, and the neighbour's edge from the edge's start to it takes the bulge of that half of the arc (its
    /// other edges stay).
    #[test]
    fn an_edge_made_an_arc_bends_the_neighbour_s_part_of_it() {
        let old = (Vec2::new(0.0, 0.0), Vec2::new(40.0, 0.0), 0.0);
        let new = (Vec2::new(0.0, 0.0), Vec2::new(40.0, 0.0), 0.5);
        let b1 = area(&[(0.0, 0.0), (20.0, 0.0), (20.0, 10.0), (0.0, 10.0)]);
        let got = carried(&[b1], &[(old, new)], false);
        let sweep = 4.0 * crate::jsmath::atan(0.5);
        assert_eq!(got.len(), 2, "{got:?}");
        let Change::Move { at, to } = got[0] else { panic!() };
        assert!(same(at, Vec2::new(20.0, 0.0)) && (to.x - 20.0).abs() < 1e-9 && (to.y + 10.0).abs() < 1e-9, "{to:?}");
        let Change::Bulge { a, b, from, to: bulge } = got[1] else { panic!() };
        assert!(same(a, Vec2::new(0.0, 0.0)) && same(b, to) && from == 0.0);
        assert!((bulge - crate::jsmath::tan(sweep / 8.0)).abs() < 1e-12, "{bulge}");
    }

    fn area(pts: &[(f64, f64)]) -> Neighbour {
        Neighbour {
            shape: Shape::Polygon {
                pts: pts.iter().map(|&(x, y)| Vec2::new(x, y)).collect(),
                bulges: None,
                holes: None,
                parts: None,
            },
            locked: false,
        }
    }

    /// docs/adr/2600: a T junction's corner and a neighbour's own vertex inside the edge, each once; the edge's ends,
    /// a vertex on its line beyond them and one a millimetre off it are not inside; a point only with `points`.
    #[test]
    fn the_vertices_strictly_inside_an_edge_each_once() {
        let b1 = area(&[(10.0, 0.0), (20.0, 0.0), (20.0, 10.0), (10.0, 10.0)]);
        let b2 = area(&[(10.0, 10.0), (20.0, 10.0), (20.0, 20.0), (10.0, 20.0)]);
        let beyond = Neighbour {
            shape: Shape::Line { a: Vec2::new(10.0, 25.0), b: Vec2::new(15.0, 30.0) },
            locked: false,
        };
        let off = Neighbour {
            shape: Shape::Line { a: Vec2::new(10.001, 16.0), b: Vec2::new(20.0, 16.0) },
            locked: false,
        };
        let stone = Neighbour {
            shape: Shape::Point { p: Vec2::new(10.0, 5.0), z: None, parts: None },
            locked: false,
        };
        let all = [b1, b2, beyond, off, stone];
        let (a, b) = (Vec2::new(10.0, 0.0), Vec2::new(10.0, 20.0));
        assert_eq!(inside_segment(&all, a, b, false), vec![Vec2::new(10.0, 10.0)]);
        assert_eq!(inside_segment(&all, a, b, true), vec![Vec2::new(10.0, 10.0), Vec2::new(10.0, 5.0)]);
    }

    /// An arc's middle vertex is inside it; its ends and the chord's middle are not.
    #[test]
    fn a_vertex_on_an_arc_between_its_ends_is_inside_it() {
        let (a, b) = (Vec2::new(0.0, 0.0), Vec2::new(0.0, 20.0));
        let arc = crate::geom::bulge::bulge_arc(a, b, 0.25).expect("an arc");
        let mid = crate::geom::bulge::segment_mid(a, b, 0.25);
        let on = Neighbour { shape: Shape::Line { a: mid, b: Vec2::new(9.0, 10.0) }, locked: false };
        let chord = Neighbour { shape: Shape::Line { a: Vec2::new(0.0, 10.0), b: Vec2::new(-5.0, 10.0) }, locked: false };
        let ends = Neighbour { shape: Shape::Line { a, b }, locked: false };
        let got = inside_arc(&[on, chord, ends], arc.c, arc.r, arc.a0, arc.sweep, false);
        assert_eq!(got.len(), 1);
        assert!(dist(got[0], mid) < 1e-12);
    }

    /// The reference's cases (scripts/fixtures/topology_edit_cases.py): the
    /// neighbours put right, bit for bit, and the counts.
    #[test]
    fn every_case_is_put_right_as_the_reference_puts_it() {
        let file = Json::parse(include_str!(
            "../../../../../fixtures/topology/v1/edit.json"
        ))
        .expect("edit.json reads");
        let Json::Arr(cases) = file.get("cases") else {
            panic!("cases")
        };
        assert!(cases.len() >= 11, "{} cases", cases.len());
        let mut off = Vec::new();
        for case in cases {
            let name = String::from_json(case.get("name")).unwrap_or_default();
            let neighbours =
                Vec::<Neighbour>::from_json(case.get("neighbours")).expect("neighbours");
            let changes = Vec::<Change>::from_json(case.get("changes")).expect("changes");
            let points = bool::from_json(case.get("points")).expect("points");
            let got = apply(&neighbours, &changes, points);
            let expected = case.get("expected");
            let Json::Arr(edited) = expected.get("edited") else {
                panic!("edited")
            };
            let want: Vec<(usize, Shape)> = edited
                .iter()
                .map(|e| {
                    (
                        usize::from_json(e.get("index")).expect("index"),
                        Shape::from_json(e.get("shape")).expect("shape"),
                    )
                })
                .collect();
            if got.edited != want {
                off.push(format!("{name}: {:?} ≠ {want:?}", got.edited));
            }
            for (what, n) in [("locked", got.locked), ("invalid", got.invalid)] {
                let w = usize::from_json(expected.get(what)).expect(what);
                if n != w {
                    off.push(format!("{name}: {what} {n} ≠ {w}"));
                }
            }
        }
        assert!(off.is_empty(), "{}", off.join("\n"));
    }

    /// The reference's edits (its `diffs`): the changes found from before and after.
    #[test]
    fn every_edit_gives_the_references_changes() {
        let file = Json::parse(include_str!(
            "../../../../../fixtures/topology/v1/edit.json"
        ))
        .expect("edit.json reads");
        let Json::Arr(diffs) = file.get("diffs") else {
            panic!("diffs")
        };
        assert!(diffs.len() >= 9, "{} diffs", diffs.len());
        let mut off = Vec::new();
        for d in diffs {
            let name = String::from_json(d.get("name")).unwrap_or_default();
            let before = Shape::from_json(d.get("before")).expect("before");
            let after = Shape::from_json(d.get("after")).expect("after");
            let want = Vec::<Change>::from_json(d.get("expected")).expect("expected");
            let got = changes(&before, &after);
            if got != want {
                off.push(format!("{name}: {got:?} ≠ {want:?}"));
            }
        }
        assert!(off.is_empty(), "{}", off.join("\n"));
    }
}
