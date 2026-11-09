//! The clearances of `cad.pipes.check` to another network's pipes (plan/map-cad.md MAP-16, docs/adr/2360):
//!
//! - where a checked pipe crosses one of the other's in plan, the vertical clearance there: the upper pipe's invert
//!   less the lower one's crown (invert + diameter), the upper one the one whose bore's middle is higher;
//! - where two pipes pass without crossing, the least horizontal clearance: the distance between their axes less the
//!   two radii, reported where it is below the least the user gave.
//!
//! The diameters are the inner ones. With a wall thickness asked (docs/adr/3960) both clearances are outside to outside:
//! the upper pipe's bottom outside (invert − wall) less the lower one's top outside (invert + diameter + wall), the
//! axes' distance less both radii and walls; without, they are larger than outside to outside by the walls (as the
//! cover is, ADR 1640). A pipe whose wall is asked and not given counts without one (the checked network's is said). The crossings are the shared core's edge intersections (lines and circular
//! arcs, exact predicates); two edges that do not cross are closest at an end of one or where their common normal
//! meets both (a segment's normal through an arc's centre, the line of two arcs' centres), so the least distance is
//! exact too. No limit is the program's.

use kentos_commands::{
    Entity, NetworkDef, PIPE_FINDING_LIMIT, PIPE_LIMIT, PipeAbove, PipeClearanceAsk, PipeCrossing, PipeDiameter,
    PipeInverts, PipeNear,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::geom::arrangement::tight_box;
use kentos_geometry_core::geom::bulge::bulge_path_edges;
use kentos_geometry_core::geom::intersect::{Edge, closest_on_edge, intersect_edges, on_edge_arc, point_at};
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::ops::network::{Graph, NetworkSession};
use kentos_geometry_core::store::rtree::PackedTree;
use kentos_geometry_core::vec2::Vec2;

use crate::checks::Stop;
use crate::codes;
use crate::pipes_check::{Path, asked_values, attr, diameter_of, network_def, path_of, refuse, wall_of};
use crate::processing_host::host;

/// A stop of the other network's question or loading, its place under `clearance.`.
fn under(stop: Stop) -> Stop {
    match stop {
        Stop::Failed(mut e) => {
            e.path = e.path.take().map(|p| format!("clearance.{p}"));
            Stop::Failed(e)
        }
        other => other,
    }
}

/// The clearance question's own rules, before the drawing is read.
pub(crate) fn asked(c: &PipeClearanceAsk) -> Result<(), Stop> {
    asked_values(&c.inverts, c.diameter.as_ref(), None).map_err(under)?;
    let limits = [
        (c.vertical, "clearance.vertical", "En az düşey açıklık", "yalnız çakışmalar aranacaksa boş bırakın"),
        (c.horizontal, "clearance.horizontal", "En az yatay açıklık", "yatay açıklık denetlenmeyecekse boş bırakın"),
    ];
    for (v, path, words, empty) in limits {
        if let Some(v) = v
            && !(v.is_finite() && v > 0.0)
        {
            return Err(refuse(codes::INVALID_PIPE_CHECK, format!("{words} 0'dan büyük bir sayı olmalı; {empty}."), path));
        }
    }
    Ok(())
}

/// The other network asked by its id or name, built; refused under `clearance.network`, and when it is the network
/// checked.
pub(crate) fn load(doc: &Document, c: &PipeClearanceAsk, checked: &NetworkDef) -> Result<(NetworkDef, NetworkSession), Stop> {
    load_other(
        doc,
        &c.network,
        checked,
        "clearance.network",
        |name| format!("Açıklık başka bir ağın borularına ölçülür; “{name}” denetlenen ağın kendisi."),
        |name, n| format!("“{name}” ağında {n} boru var; en çok {PIPE_LIMIT} boruya açıklık ölçülür. Ağı süzgeçle bölün."),
    )
}

/// Another network than `checked` by its id or name, built; refused at `at`, with `same`'s words when it is `checked`
/// and `large`'s (its name, its pipes) when it has more than `PIPE_LIMIT` pipes.
pub(crate) fn load_other(
    doc: &Document,
    network: &str,
    checked: &NetworkDef,
    at: &str,
    same: impl Fn(&str) -> String,
    large: impl Fn(&str, usize) -> String,
) -> Result<(NetworkDef, NetworkSession), Stop> {
    let def = network_def(doc, network).map_err(|stop| match stop {
        Stop::Failed(mut e) => {
            e.path = Some(at.to_owned());
            Stop::Failed(e)
        }
        other => other,
    })?;
    if def.id == checked.id {
        return Err(refuse(codes::INVALID_PIPE_CHECK, same(&def.name), at));
    }
    let (_, session) = host()
        .map_err(|e| Stop::failed(*e))
        .and_then(|h| h.network(doc, &def.id).map_err(|why| refuse(codes::INVALID_NETWORK, why, at)))?;
    if session.graph.edges.len() > PIPE_LIMIT {
        return Err(refuse(codes::INVALID_PIPE_CHECK, large(&def.name, session.graph.edges.len()), at));
    }
    Ok((def, session))
}

/// A pipe of either network as the clearances take it.
struct Pipe {
    uid: String,
    /// Its `Ad` attribute.
    name: Option<String>,
    path: Path,
    /// Its plan: the path's segments and arcs, in its stations' order.
    edges: Vec<Edge>,
    inverts: Inverts,
    /// Half its diameter (metres): 0 where no diameter is asked, none where one is asked and the pipe has none.
    radius: Option<f64>,
    /// Its wall (metres; docs/adr/3960): 0 where none is asked or given.
    wall: f64,
    /// Its wall as read (metres; docs/adr/4360): none where none is asked or given.
    wall_read: Option<f64>,
    bounds: Bounds,
}

/// Where a pipe's inverts come from: its ends' attributes (none where it lacks them) or its vertices' heights.
enum Inverts {
    Ends(Option<(f64, f64)>),
    Heights,
}

impl Pipe {
    /// The invert at station `s`: only from where the question says (no fallback from the attributes to the heights).
    fn invert(&self, s: f64) -> Option<f64> {
        match self.inverts {
            Inverts::Ends(ends) => self.path.invert(Some(ends?), s),
            Inverts::Heights => self.path.invert(None, s),
        }
    }

    /// The station of parameter `t` (0..1) along edge `i`.
    fn station(&self, i: usize, t: f64) -> f64 {
        let st = &self.path.stations;
        match (st.get(i), st.get(i + 1)) {
            (Some(a), Some(b)) => a + t.clamp(0.0, 1.0) * (b - a),
            _ => 0.0,
        }
    }
}

fn v(p: [f64; 2]) -> Vec2 {
    Vec2::new(p[0], p[1])
}

fn dist(a: Vec2, b: Vec2) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

/// A pipe's plan, the same path the slope and cover are measured along: a line's segment, a polyline's segments and
/// arcs, an arc's arc (drawn counter-clockwise, as the network takes it).
fn plan_of(e: Option<&Entity>, path: &Path) -> Vec<Edge> {
    match e {
        Some(Entity::Arc(a)) => {
            let sweep = (a.a1 - a.a0).rem_euclid(std::f64::consts::TAU);
            let sweep = if sweep == 0.0 { std::f64::consts::TAU } else { sweep };
            vec![Edge::Arc { c: Vec2::new(a.c.x, a.c.y), r: a.r, a0: a.a0, sweep }]
        }
        Some(Entity::Polyline(p)) if p.pts.len() >= 2 => {
            let pts: Vec<Vec2> = p.pts.iter().map(|q| Vec2::new(q.x, q.y)).collect();
            bulge_path_edges(&pts, p.bulges.as_deref(), false)
        }
        _ => path.pts.windows(2).map(|w| Edge::Seg { a: v(w[0]), b: v(w[1]) }).collect(),
    }
}

fn pipes_of(doc: &Document, g: &Graph, inverts: &PipeInverts, diameter: Option<&PipeDiameter>) -> Vec<Pipe> {
    g.edges
        .iter()
        .map(|edge| {
            let slot = Slot(edge.id as u32);
            let e = doc.get(slot);
            let path = path_of(e, [edge.a.x, edge.a.y], [edge.b.x, edge.b.y]);
            let edges = plan_of(e, &path);
            let mut bounds = Bounds { min_x: f64::INFINITY, min_y: f64::INFINITY, max_x: f64::NEG_INFINITY, max_y: f64::NEG_INFINITY };
            for b in edges.iter().map(tight_box) {
                bounds.min_x = bounds.min_x.min(b.min_x);
                bounds.min_y = bounds.min_y.min(b.min_y);
                bounds.max_x = bounds.max_x.max(b.max_x);
                bounds.max_y = bounds.max_y.max(b.max_y);
            }
            let inverts = match inverts {
                PipeInverts::Fields { start, end } => Inverts::Ends(e.and_then(|e| attr(e, start).zip(attr(e, end)))),
                PipeInverts::Heights => Inverts::Heights,
            };
            let radius = match diameter {
                None => Some(0.0),
                Some(d) => diameter_of(e, d).ok().map(|x| x * d.unit.metres() / 2.0),
            };
            let wall_read = diameter.and_then(|d| wall_of(e, d)).and_then(Result::ok);
            let wall = wall_read.unwrap_or(0.0);
            Pipe {
                uid: doc.uid(slot).map(|u| u.to_string()).unwrap_or_default(),
                name: e.and_then(|e| e.base().attrs.get("Ad").cloned()),
                path,
                edges,
                inverts,
                radius,
                wall,
                wall_read,
                bounds,
            }
        })
        .collect()
}

/// The other network made ready: its pipes and their boxes in a tree.
pub(crate) struct Other<'a> {
    ask: &'a PipeClearanceAsk,
    pipes: Vec<Pipe>,
    tree: PackedTree,
    /// The largest half diameter and wall of its pipes (metres), how far beyond its boxes a pipe can reach.
    reach: f64,
}

impl<'a> Other<'a> {
    pub(crate) fn new(doc: &Document, ask: &'a PipeClearanceAsk, session: &NetworkSession) -> Other<'a> {
        let pipes = pipes_of(doc, &session.graph, &ask.inverts, ask.diameter.as_ref());
        let boxes: Vec<(u32, Bounds)> = pipes
            .iter()
            .enumerate()
            .filter(|(_, p)| [p.bounds.min_x, p.bounds.min_y, p.bounds.max_x, p.bounds.max_y].iter().all(|x| x.is_finite()))
            .map(|(k, p)| (k as u32, p.bounds))
            .collect();
        let reach = pipes.iter().filter_map(|p| p.radius.map(|r| r + p.wall)).fold(0.0, f64::max);
        Other { ask, pipes, tree: PackedTree::build(&boxes), reach }
    }
}

/// A finding of the clearances: its kind, the checked pipe's uid, where, its measure.
pub(crate) struct Mark {
    pub(crate) kind: &'static str,
    pub(crate) uid: String,
    pub(crate) at: [f64; 2],
    pub(crate) value: Option<f64>,
}

pub(crate) struct Measured {
    pub(crate) crossings: Vec<PipeCrossing>,
    pub(crate) near: Vec<PipeNear>,
    pub(crate) marks: Vec<Mark>,
}

/// The least distance between two edges that do not cross, and the closest places on each: at an end of one, or
/// where their common normal meets both.
fn closest(a: &Edge, b: &Edge) -> (f64, Vec2, Vec2) {
    let mut best = (f64::INFINITY, Vec2::new(0.0, 0.0), Vec2::new(0.0, 0.0));
    let mut take = |d: f64, p: Vec2, q: Vec2| {
        if d < best.0 {
            best = (d, p, q);
        }
    };
    for t in [0.0, 1.0] {
        let p = point_at(a, t);
        let c = closest_on_edge(b, p);
        take(c.d, p, c.p);
    }
    for t in [0.0, 1.0] {
        let q = point_at(b, t);
        let c = closest_on_edge(a, q);
        take(c.d, c.p, q);
    }
    match (*a, *b) {
        (Edge::Seg { a: s0, b: s1 }, Edge::Arc { c, r, a0, sweep }) => {
            for (q, p) in normal_feet(s0, s1, c, r, a0, sweep) {
                take(dist(p, q), q, p);
            }
        }
        (Edge::Arc { c, r, a0, sweep }, Edge::Seg { a: s0, b: s1 }) => {
            for (q, p) in normal_feet(s0, s1, c, r, a0, sweep) {
                take(dist(p, q), p, q);
            }
        }
        (Edge::Arc { c: c1, r: r1, a0: a01, sweep: s1 }, Edge::Arc { c: c2, r: r2, a0: a02, sweep: s2 }) => {
            let d = dist(c1, c2);
            if d > 0.0 {
                let u = Vec2::new((c2.x - c1.x) / d, (c2.y - c1.y) / d);
                for k1 in [1.0, -1.0] {
                    let p = Vec2::new(c1.x + k1 * r1 * u.x, c1.y + k1 * r1 * u.y);
                    if !on_edge_arc(a01, s1, (k1 * u.y).atan2(k1 * u.x)) {
                        continue;
                    }
                    for k2 in [1.0, -1.0] {
                        let q = Vec2::new(c2.x + k2 * r2 * u.x, c2.y + k2 * r2 * u.y);
                        if on_edge_arc(a02, s2, (k2 * u.y).atan2(k2 * u.x)) {
                            take(dist(p, q), p, q);
                        }
                    }
                }
            }
        }
        _ => {}
    }
    best
}

/// The places where a segment's normal through an arc's centre meets the arc (both sides) with its foot inside the
/// segment: (foot on the segment, place on the arc).
fn normal_feet(s0: Vec2, s1: Vec2, c: Vec2, r: f64, a0: f64, sweep: f64) -> Vec<(Vec2, Vec2)> {
    let (dx, dy) = (s1.x - s0.x, s1.y - s0.y);
    let l = dx.hypot(dy);
    if l == 0.0 {
        return Vec::new();
    }
    let n = Vec2::new(-dy / l, dx / l);
    let mut out = Vec::new();
    for k in [1.0, -1.0] {
        let p = Vec2::new(c.x + k * r * n.x, c.y + k * r * n.y);
        if !on_edge_arc(a0, sweep, (k * n.y).atan2(k * n.x)) {
            continue;
        }
        let t = ((p.x - s0.x) * dx + (p.y - s0.y) * dy) / (l * l);
        if (0.0..=1.0).contains(&t) {
            out.push((Vec2::new(s0.x + t * dx, s0.y + t * dy), p));
        }
    }
    out
}

/// The crossings of two pipes in plan: (station on the first, station on the second, place), along the first; a place
/// two edges share is one crossing.
fn crossings_of(p: &Pipe, o: &Pipe) -> Vec<(f64, f64, Vec2)> {
    crossings_along(&p.edges, &p.path, o)
}

/// [`crossings_of`] of a plan's `edges` along `path` (its stations) and the pipe `o`.
fn crossings_along(edges: &[Edge], path: &Path, o: &Pipe) -> Vec<(f64, f64, Vec2)> {
    let station = |i: usize, t: f64| match (path.stations.get(i), path.stations.get(i + 1)) {
        (Some(a), Some(b)) => a + t.clamp(0.0, 1.0) * (b - a),
        _ => 0.0,
    };
    let mut out: Vec<(f64, f64, Vec2)> = Vec::new();
    let (tol_p, tol_o) = (1e-9 * path.length().max(1.0), 1e-9 * o.path.length().max(1.0));
    for (i, ep) in edges.iter().enumerate() {
        for (j, eo) in o.edges.iter().enumerate() {
            for h in intersect_edges(ep, eo) {
                let (sp, so) = (station(i, h.t), o.station(j, h.u));
                if out.iter().any(|x| (x.0 - sp).abs() <= tol_p && (x.1 - so).abs() <= tol_o) {
                    continue;
                }
                out.push((sp, so, h.p));
            }
        }
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out
}

/// A pipe of the other network crossing a pipe of a run in plan (docs/adr/2760): the station along the run's pipe, the
/// other's uid and name, its invert there and half its diameter (as the clearances read them), and the place.
pub(crate) struct Crossed {
    pub(crate) station: f64,
    pub(crate) uid: String,
    pub(crate) name: Option<String>,
    pub(crate) invert: Option<f64>,
    pub(crate) radius: Option<f64>,
    /// Its wall (metres) when asked and read (docs/adr/4360).
    pub(crate) wall: Option<f64>,
    pub(crate) at: [f64; 2],
}

/// The other network's pipes the pipe `e` (its plan `path`) crosses, along it.
pub(crate) fn crossing_pipes(e: Option<&Entity>, path: &Path, other: &Other<'_>) -> Vec<Crossed> {
    let edges = plan_of(e, path);
    let mut bounds = Bounds { min_x: f64::INFINITY, min_y: f64::INFINITY, max_x: f64::NEG_INFINITY, max_y: f64::NEG_INFINITY };
    for b in edges.iter().map(tight_box) {
        bounds.min_x = bounds.min_x.min(b.min_x);
        bounds.min_y = bounds.min_y.min(b.min_y);
        bounds.max_x = bounds.max_x.max(b.max_x);
        bounds.max_y = bounds.max_y.max(b.max_y);
    }
    if ![bounds.min_x, bounds.min_y, bounds.max_x, bounds.max_y].iter().all(|v| v.is_finite()) {
        return Vec::new();
    }
    let mut found = Vec::new();
    other.tree.search(&bounds, &mut found);
    found.sort_unstable();
    let mut out: Vec<Crossed> = Vec::new();
    for &k in &found {
        let o = &other.pipes[k as usize];
        for (sp, so, at) in crossings_along(&edges, path, o) {
            out.push(Crossed {
                station: sp,
                uid: o.uid.clone(),
                name: o.name.clone(),
                invert: o.invert(so),
                radius: o.radius,
                wall: o.wall_read,
                at: [at.x, at.y],
            });
        }
    }
    out.sort_by(|a, b| a.station.total_cmp(&b.station).then_with(|| a.uid.cmp(&b.uid)));
    out
}

/// The clearances of the checked network's graph `g` to the other's.
pub(crate) fn measure(
    doc: &Document,
    g: &Graph,
    inverts: &PipeInverts,
    diameter: Option<&PipeDiameter>,
    other: &Other<'_>,
) -> Measured {
    let pipes = pipes_of(doc, g, inverts, diameter);
    let ask = other.ask;
    let mut out = Measured { crossings: Vec::new(), near: Vec::new(), marks: Vec::new() };
    let mut found: Vec<u32> = Vec::new();
    for p in &pipes {
        if !p.bounds.min_x.is_finite() {
            continue;
        }
        let grow = ask.horizontal.map_or(0.0, |h| h + p.radius.unwrap_or(0.0) + p.wall + other.reach);
        let q = Bounds {
            min_x: p.bounds.min_x - grow,
            min_y: p.bounds.min_y - grow,
            max_x: p.bounds.max_x + grow,
            max_y: p.bounds.max_y + grow,
        };
        found.clear();
        other.tree.search(&q, &mut found);
        found.sort_unstable();
        for &k in &found {
            let o = &other.pipes[k as usize];
            let hits = crossings_of(p, o);
            for (sp, so, at) in &hits {
                let (ip, io) = (p.invert(*sp), o.invert(*so));
                let level = match (ip, io, p.radius, o.radius) {
                    (Some(ip), Some(io), Some(rp), Some(ro)) => {
                        // Above by the bores' middles; the clearance outside to outside (the walls 0 when not asked).
                        let pipe_up = ip + rp >= io + ro;
                        let c = if pipe_up {
                            (ip - p.wall) - (io + 2.0 * ro + o.wall)
                        } else {
                            (io - o.wall) - (ip + 2.0 * rp + p.wall)
                        };
                        Some((c, if pipe_up { PipeAbove::Pipe } else { PipeAbove::Other }))
                    }
                    _ => None,
                };
                let place = [at.x, at.y];
                if let Some((c, above)) = level {
                    let mut mark = |kind: &'static str| {
                        out.marks.push(Mark { kind, uid: p.uid.clone(), at: place, value: Some(c) })
                    };
                    if c < 0.0 {
                        mark("clash");
                    } else if ask.vertical.is_some_and(|v| c < v) {
                        mark("closeCrossing");
                    }
                    if ask.above.is_some_and(|want| want != above) {
                        mark("wrongSide");
                    }
                }
                out.crossings.push(PipeCrossing {
                    pipe: p.uid.clone(),
                    other: o.uid.clone(),
                    at: place,
                    invert: ip,
                    other_invert: io,
                    clearance: level.map(|l| l.0),
                    above: level.map(|l| l.1),
                });
            }
            let Some(h) = ask.horizontal else { continue };
            if !hits.is_empty() {
                continue;
            }
            let mut best: Option<(f64, Vec2, Vec2)> = None;
            for ep in &p.edges {
                for eo in &o.edges {
                    let c = closest(ep, eo);
                    if best.is_none_or(|b| c.0 < b.0) {
                        best = Some(c);
                    }
                }
            }
            let Some((d, at, other_at)) = best else { continue };
            let clearance = p.radius.zip(o.radius).map(|(rp, ro)| d - (rp + p.wall) - (ro + o.wall));
            if clearance.map_or(d < h, |c| c < h) {
                out.marks.push(Mark { kind: "closeAlongside", uid: p.uid.clone(), at: [at.x, at.y], value: clearance });
                out.near.push(PipeNear {
                    pipe: p.uid.clone(),
                    other: o.uid.clone(),
                    at: [at.x, at.y],
                    other_at: [other_at.x, other_at.y],
                    distance: d,
                    clearance,
                });
            }
        }
    }
    out.crossings.truncate(PIPE_FINDING_LIMIT);
    out.near.truncate(PIPE_FINDING_LIMIT);
    out
}
