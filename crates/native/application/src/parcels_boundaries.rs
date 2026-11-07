//! `cad.parcels.boundaries` v1 (plan/map-cad.md MAP-08, docs/adr/2000): Sınır grafiği, read only. The rules, as the
//! independent reference (`scripts/fixtures/parcel_boundaries_cases.py`) writes them:
//!
//! - a parcel's rings: its outer ring, its holes, then each further part's ring and holes; a ring's edges in order,
//!   each with its bulge (none: straight); an edge whose ends are the same place is left out;
//! - a corner is a place where a ring has a vertex, the same place exactly (its coordinates as stored); the corners in
//!   order of east, then north, are `K1`, `K2` …;
//! - a boundary is an edge between two corners with one bulge (taken from the lower corner to the higher, the same
//!   within 1e-9): every ring edge on it is the same boundary; the boundaries in order of their corners are `S1` …;
//! - a ring's parcel lies on the left of its edges when the ring runs counter-clockwise (its signed area, arcs
//!   included, positive), on the right otherwise; a hole's the other way. A boundary's `left` and `right` are its
//!   first parcel on each side; a further one on a side, or a third parcel, is an `overlap`;
//! - `missingVertex`: a corner that is no end of a straight boundary yet within 1 µm of its inside (the foot of its
//!   perpendicular strictly between the ends); `splitCorner`: two corners within 1 µm; problems in order of kind, then
//!   of their first corner or boundary;
//! - a parcel's neighbours: the parcels across its boundaries, in the drawing's order.

use std::collections::BTreeMap;

use kentos_commands::{
    BoundaryProblem, CommandResult, Entity, ParcelBoundary, ParcelCorner, ParcelInGraph, ParcelsBoundaries, ParcelsBounded,
};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::entity::{Shape, entity_area};
use kentos_geometry_core::geom::bulge::bulge_ring_area;
use kentos_geometry_core::vec2::Vec2;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::geometry::shape;

/// Two places this close are one (`splitCorner`, `missingVertex`), m.
const NEAR: f64 = 1e-6;
/// Two bulges this close are one boundary's.
const BULGE: f64 = 1e-9;
/// The parcel layer, as Parsel alan denetimi reads it.
const PARCEL_LAYER: &str = "parsel";

fn fail(code: &str, message: String, path: String) -> Stop {
    Stop::failed(error(code, message, Some(path)))
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// A ring: its vertices and each edge's bulge, and whether its parcel lies outside it (a hole).
struct RingOf {
    pts: Vec<Vec2>,
    bulges: Vec<f64>,
    hole: bool,
}

fn rings_of(s: &Shape) -> Vec<RingOf> {
    let ring = |pts: &[Vec2], bulges: &Option<Vec<f64>>, hole: bool| RingOf {
        pts: pts.to_vec(),
        bulges: (0..pts.len()).map(|i| bulges.as_ref().and_then(|b| b.get(i).copied()).unwrap_or(0.0)).collect(),
        hole,
    };
    let mut out = Vec::new();
    if let Shape::Polygon { pts, bulges, holes, parts } = s {
        out.push(ring(pts, bulges, false));
        for h in holes.iter().flatten() {
            out.push(ring(&h.pts, &h.bulges, true));
        }
        for p in parts.iter().flatten() {
            out.push(ring(&p.pts, &p.bulges, false));
            for h in p.holes.iter().flatten() {
                out.push(ring(&h.pts, &h.bulges, true));
            }
        }
    }
    out
}

/// A place's key: its coordinates' bits (−0 as 0).
fn key(p: Vec2) -> (u64, u64) {
    let bits = |v: f64| if v == 0.0 { 0.0f64.to_bits() } else { v.to_bits() };
    (bits(p.x), bits(p.y))
}

/// An edge's length: its chord, or its arc's.
fn length(a: Vec2, b: Vec2, bulge: f64) -> f64 {
    let c = (b.x - a.x).hypot(b.y - a.y);
    if bulge == 0.0 || c == 0.0 {
        return c;
    }
    let theta = 4.0 * bulge.atan();
    c * theta / (2.0 * (theta / 2.0).sin())
}

/// An edge's middle on its curve (a bulge bows right of the way from `a` to `b` when positive).
fn middle(a: Vec2, b: Vec2, bulge: f64) -> [f64; 2] {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    [(a.x + b.x) / 2.0 + dy * bulge / 2.0, (a.y + b.y) / 2.0 - dx * bulge / 2.0]
}

/// One ring edge on a boundary: its parcel (index) and whether the parcel lies left of the way from the lower corner
/// to the higher.
struct Side {
    parcel: usize,
    left: bool,
}

struct Edge {
    lo: usize,
    hi: usize,
    bulge: f64,
    sides: Vec<Side>,
}

fn parcels_of<'d>(doc: &'d Document, input: &ParcelsBoundaries) -> Result<Vec<&'d Entity>, Stop> {
    match &input.uids {
        Some(uids) => {
            let mut out = Vec::new();
            for (i, uid) in uids.iter().enumerate() {
                let path = format!("uids[{i}]");
                if !checks::is_uid_text(uid) {
                    return Err(fail(codes::INVALID_UID, format!("“{uid}” geçerli bir nesne kimliği değil."), path));
                }
                let Some(e) = Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u)).and_then(|s| doc.get(s)) else {
                    return Err(fail(codes::ENTITY_NOT_FOUND, format!("“{uid}” kimlikli nesne çizimde yok."), path));
                };
                if !matches!(e, Entity::Polygon(_)) {
                    return Err(fail(
                        codes::NOT_A_PARCEL,
                        format!("“{uid}” bir alan değil; sınır grafiği yalnız alanlardan kurulur."),
                        path,
                    ));
                }
                out.push(e);
            }
            Ok(out)
        }
        None => Ok(doc.by_layer(PARCEL_LAYER).filter(|e| matches!(e, Entity::Polygon(_))).collect()),
    }
}

fn run(doc: &Document, input: &ParcelsBoundaries) -> Result<ParcelsBounded, Stop> {
    let parcels = parcels_of(doc, input)?;
    let uid_of = |e: &Entity| doc.uid(kentos_domain::Slot(e.base().id)).map(|u| u.to_string()).unwrap_or_default();
    let uids: Vec<String> = parcels.iter().map(|e| uid_of(e)).collect();
    let shapes: Vec<Shape> = parcels.iter().map(|e| shape(e)).collect();
    // The corners, in order of east, then north.
    let mut places: BTreeMap<(u64, u64), Vec2> = BTreeMap::new();
    for s in &shapes {
        for r in rings_of(s) {
            for p in &r.pts {
                places.entry(key(*p)).or_insert(*p);
            }
        }
    }
    let mut corners: Vec<Vec2> = places.values().copied().collect();
    corners.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    let index: BTreeMap<(u64, u64), usize> = corners.iter().enumerate().map(|(i, p)| (key(*p), i)).collect();
    let mut at_corner: Vec<Vec<usize>> = vec![Vec::new(); corners.len()];
    // The boundaries: every ring edge on its corners and bulge.
    let mut edges: Vec<Edge> = Vec::new();
    let mut by_ends: BTreeMap<(usize, usize), Vec<usize>> = BTreeMap::new();
    let mut rings_edges: Vec<Vec<usize>> = vec![Vec::new(); shapes.len()];
    for (k, s) in shapes.iter().enumerate() {
        for r in rings_of(s) {
            let n = r.pts.len();
            let ccw = bulge_ring_area(&r.pts, Some(&r.bulges)) > 0.0;
            for i in 0..n {
                let (a, b) = (r.pts[i], r.pts[(i + 1) % n]);
                let (ia, ib) = (index[&key(a)], index[&key(b)]);
                if !at_corner[ia].contains(&k) {
                    at_corner[ia].push(k);
                }
                if ia == ib {
                    continue;
                }
                let forward = ia < ib;
                let (lo, hi) = if forward { (ia, ib) } else { (ib, ia) };
                let bulge = if forward { r.bulges[i] } else { -r.bulges[i] };
                // Left of its own way: a counter-clockwise outer ring's parcel, a clockwise hole's.
                let left_of_way = ccw != r.hole;
                let left = if forward { left_of_way } else { !left_of_way };
                let same = by_ends.entry((lo, hi)).or_default();
                let at = match same.iter().copied().find(|&j| (edges[j].bulge - bulge).abs() <= BULGE) {
                    Some(j) => j,
                    None => {
                        edges.push(Edge { lo, hi, bulge, sides: Vec::new() });
                        same.push(edges.len() - 1);
                        edges.len() - 1
                    }
                };
                edges[at].sides.push(Side { parcel: k, left });
                rings_edges[k].push(at);
            }
        }
    }
    for list in &mut at_corner {
        list.sort_unstable();
    }
    let mut order: Vec<usize> = (0..edges.len()).collect();
    order.sort_by(|&x, &y| {
        let (a, b) = (&edges[x], &edges[y]);
        a.lo.cmp(&b.lo).then(a.hi.cmp(&b.hi)).then(a.bulge.total_cmp(&b.bulge))
    });
    let mut boundary_id = vec![0usize; edges.len()];
    for (n, &e) in order.iter().enumerate() {
        boundary_id[e] = n;
    }
    let kid = |i: usize| format!("K{}", i + 1);
    let sid = |e: usize| format!("S{}", boundary_id[e] + 1);
    let mut problems: Vec<(u8, usize, BoundaryProblem)> = Vec::new();
    let mut boundaries = Vec::new();
    let (mut shared, mut outer) = (0usize, 0usize);
    for &e in &order {
        let edge = &edges[e];
        let left = edge.sides.iter().find(|s| s.left).map(|s| s.parcel);
        let right = edge.sides.iter().find(|s| !s.left).map(|s| s.parcel);
        let used = usize::from(left.is_some()) + usize::from(right.is_some());
        if used == 2 {
            shared += 1;
        } else {
            outer += 1;
        }
        if edge.sides.len() > used {
            let mut on: Vec<usize> = edge.sides.iter().map(|s| s.parcel).collect();
            on.sort_unstable();
            on.dedup();
            problems.push((
                2,
                boundary_id[e],
                BoundaryProblem {
                    kind: "overlap".into(),
                    at: middle(corners[edge.lo], corners[edge.hi], edge.bulge),
                    parcels: on.iter().map(|&k| uids[k].clone()).collect(),
                },
            ));
        }
        boundaries.push(ParcelBoundary {
            id: sid(e),
            from: kid(edge.lo),
            to: kid(edge.hi),
            bulge: edge.bulge,
            length: length(corners[edge.lo], corners[edge.hi], edge.bulge),
            left: left.map(|k| uids[k].clone()),
            right: right.map(|k| uids[k].clone()),
        });
    }
    // A corner on the inside of a straight boundary that does not end there: the straight boundaries by the cells of
    // a grid their boxes (grown by 1 µm) meet, each corner tried against its cell's (the answer is the grid's own
    // whatever the cell's size).
    const CELL: f64 = 20.0;
    let cell = |v: f64| (v / CELL).floor() as i64;
    let mut grid: BTreeMap<(i64, i64), Vec<usize>> = BTreeMap::new();
    for (n, &e) in order.iter().enumerate() {
        let edge = &edges[e];
        if edge.bulge != 0.0 {
            continue;
        }
        let (a, b) = (corners[edge.lo], corners[edge.hi]);
        for gx in cell(a.x.min(b.x) - NEAR)..=cell(a.x.max(b.x) + NEAR) {
            for gy in cell(a.y.min(b.y) - NEAR)..=cell(a.y.max(b.y) + NEAR) {
                grid.entry((gx, gy)).or_default().push(n);
            }
        }
    }
    for (c, p) in corners.iter().enumerate() {
        let Some(near) = grid.get(&(cell(p.x), cell(p.y))) else { continue };
        for &e in near.iter().map(|n| &order[*n]) {
            let edge = &edges[e];
            if edge.lo == c || edge.hi == c {
                continue;
            }
            let (a, b) = (corners[edge.lo], corners[edge.hi]);
            let (dx, dy) = (b.x - a.x, b.y - a.y);
            let l2 = dx * dx + dy * dy;
            let t = ((p.x - a.x) * dx + (p.y - a.y) * dy) / l2;
            if t <= 0.0 || t >= 1.0 {
                continue;
            }
            let d = ((p.x - a.x) * dy - (p.y - a.y) * dx).abs() / l2.sqrt();
            if d <= NEAR {
                let mut on: Vec<usize> = at_corner[c].iter().copied().chain(edge.sides.iter().map(|s| s.parcel)).collect();
                on.sort_unstable();
                on.dedup();
                problems.push((
                    0,
                    c,
                    BoundaryProblem {
                        kind: "missingVertex".into(),
                        at: [p.x, p.y],
                        parcels: on.iter().map(|&k| uids[k].clone()).collect(),
                    },
                ));
            }
        }
    }
    // Two corners within 1 µm that are not one.
    for i in 0..corners.len() {
        for j in i + 1..corners.len() {
            if corners[j].x - corners[i].x > NEAR {
                break;
            }
            if (corners[j].x - corners[i].x).hypot(corners[j].y - corners[i].y) <= NEAR {
                let mut on: Vec<usize> = at_corner[i].iter().chain(&at_corner[j]).copied().collect();
                on.sort_unstable();
                on.dedup();
                problems.push((
                    1,
                    i,
                    BoundaryProblem {
                        kind: "splitCorner".into(),
                        at: [corners[i].x, corners[i].y],
                        parcels: on.iter().map(|&k| uids[k].clone()).collect(),
                    },
                ));
            }
        }
    }
    problems.sort_by_key(|(kind, first, _)| (*kind, *first));
    let parcels_out = parcels
        .iter()
        .enumerate()
        .map(|(k, e)| {
            let attr = |name: &str| e.base().attrs.get(name).map(|v| v.trim().to_owned()).unwrap_or_default();
            let name: Vec<String> =
                ["İl", "İlçe", "Mahalle", "Ada", "Parsel"].iter().map(|n| attr(n)).filter(|v| !v.is_empty()).collect();
            let mut neighbours: Vec<usize> = rings_edges[k]
                .iter()
                .flat_map(|&e| edges[e].sides.iter().map(|s| s.parcel))
                .filter(|&other| other != k)
                .collect();
            neighbours.sort_unstable();
            neighbours.dedup();
            ParcelInGraph {
                uid: uids[k].clone(),
                name: name.join("/"),
                area: entity_area(&shapes[k]).unwrap_or(0.0),
                boundaries: rings_edges[k].iter().map(|&e| sid(e)).collect(),
                neighbours: neighbours.iter().map(|&o| uids[o].clone()).collect(),
            }
        })
        .collect();
    Ok(ParcelsBounded {
        corners: corners
            .iter()
            .enumerate()
            .map(|(i, p)| ParcelCorner { id: kid(i), at: [p.x, p.y], parcels: at_corner[i].iter().map(|&k| uids[k].clone()).collect() })
            .collect(),
        boundaries,
        parcels: parcels_out,
        problems: problems.into_iter().map(|(_, _, p)| p).collect(),
        shared: count(shared),
        outer: count(outer),
    })
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &ParcelsBoundaries) -> CommandResult<()> {
    match run(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &ParcelsBoundaries) -> CommandResult<ParcelsBounded> {
    match run(cx.doc, input) {
        Ok(output) => CommandResult::Completed { output, warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ParcelsBoundaries) -> CommandResult<ParcelsBounded> {
    plan(cx, &input)
}

/// The graph for a window, or why it does not read.
pub fn preview(doc: &Document, input: &ParcelsBoundaries) -> Result<ParcelsBounded, String> {
    run(doc, input).map_err(crate::road_corridors::words)
}
