//! An entity as a path parameterised by arc length s ∈ [0, L]
//! (`apps/web/src/model/ops/path.ts`). Trim, break, divide and measure all work on
//! s values, so every kind that has edges behaves the same.

use crate::api::Op;
use crate::api::json::{FromJson, Json, read_field};
use crate::entity::{Entity, Shape, ellipse_geom};
use crate::geom::arc::norm_angle;
use crate::geom::bulge::{bulge_of_sweep, clean_bulge_path};
use crate::geom::ellipse::{closest_param, ellipse_point, is_full_ellipse};
use crate::geom::intersect::{Edge, closest_on_edge, intersect_edges, point_at};
use crate::jsmath::{js_cmp, js_floor, js_hypot, js_max, js_min, js_sign, or, stable_sort};
use crate::op;
use crate::ops::edges::{edge_length, entity_edges};
use crate::vec2::Vec2;

#[derive(Clone, Debug, PartialEq)]
pub struct Path {
    pub edges: Vec<Edge>,
    /// Arc length at the start of each edge.
    pub cum: Vec<f64>,
    pub length: f64,
    pub closed: bool,
}

crate::json_struct!(Path {
    edges,
    cum,
    length,
    closed
});

pub fn path_of(e: &Shape) -> Option<Path> {
    let edges = entity_edges(e);
    if edges.is_empty() {
        return None;
    }
    let mut cum = Vec::with_capacity(edges.len());
    let mut s = 0.0;
    for ed in &edges {
        cum.push(s);
        s += edge_length(ed);
    }
    // A full ellipse runs round like a circle (the TypeScript counted it open: Böl put n − 1 points).
    let closed = match *e {
        Shape::Polygon { .. } | Shape::Circle { .. } => true,
        Shape::Spline { closed, .. } => closed,
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => is_full_ellipse(&ellipse_geom(c, major, ratio, t0, t1)),
        _ => false,
    };
    Some(Path {
        edges,
        cum,
        length: s,
        closed,
    })
}

/// Wraps s into [0, L) on closed paths, clamps it on open ones.
pub fn norm_s(path: &Path, s: f64) -> f64 {
    let l = path.length;
    if path.closed {
        ((s % l) + l) % l
    } else {
        js_max(0.0, js_min(l, s))
    }
}

fn edge_index_at(path: &Path, q: f64) -> usize {
    for i in (0..path.edges.len()).rev() {
        if q >= path.cum[i] - 1e-12 {
            return i;
        }
    }
    0
}

pub fn point_at_s(path: &Path, s: f64) -> Vec2 {
    let q = norm_s(path, s);
    let i = edge_index_at(path, q);
    let len = or(edge_length(&path.edges[i]), 1.0);
    point_at(&path.edges[i], js_min(1.0, (q - path.cum[i]) / len))
}

/// Unit travel direction at s.
pub fn tangent_at_s(path: &Path, s: f64) -> Vec2 {
    let q = norm_s(path, s);
    match path.edges[edge_index_at(path, q)] {
        Edge::Seg { a, b } => {
            let l = or(js_hypot(b.x - a.x, b.y - a.y), 1.0);
            Vec2::new((b.x - a.x) / l, (b.y - a.y) / l)
        }
        Edge::Arc { c, r, sweep, .. } => {
            let p = point_at_s(path, s);
            let dir = or(js_sign(sweep), 1.0);
            Vec2::new((-(p.y - c.y) / r) * dir, ((p.x - c.x) / r) * dir)
        }
    }
}

/// Arc length of the point on the path nearest to p.
pub fn nearest_s(path: &Path, p: Vec2) -> f64 {
    let (mut best_d, mut best_s) = (f64::INFINITY, 0.0);
    for (i, ed) in path.edges.iter().enumerate() {
        let c = closest_on_edge(ed, p);
        if c.d < best_d {
            best_d = c.d;
            best_s = path.cum[i] + c.t * edge_length(ed);
        }
    }
    best_s
}

/// Sorted, de-duplicated s values where `boundaries` cross the path (ends excluded when open).
pub fn cuts_on(path: &Path, boundaries: &[Edge]) -> Vec<f64> {
    let mut out = Vec::new();
    for (i, ed) in path.edges.iter().enumerate() {
        let len = edge_length(ed);
        for b in boundaries {
            for h in intersect_edges(ed, b) {
                out.push(path.cum[i] + h.t * len);
            }
        }
    }
    let eps = 1e-7 * js_max(1.0, path.length);
    let mut sorted: Vec<f64> = out
        .into_iter()
        .map(|s| if path.closed { norm_s(path, s) } else { s })
        .filter(|&s| path.closed || (s > eps && s < path.length - eps))
        .collect();
    stable_sort(&mut sorted, &mut |a, b| js_cmp(*a - *b, 0.0));
    let mut kept = Vec::with_capacity(sorted.len());
    for (i, &s) in sorted.iter().enumerate() {
        if i == 0 || s - sorted[i - 1] > eps {
            kept.push(s);
        }
    }
    kept
}

/// Geometry of the sub-path from s0 to s1 (s1 may exceed L on closed paths):
/// pieces of lines stay lines, of arcs and circles arcs, of polylines
/// polylines with their arc segments cut exactly.
pub fn sub_path(path: &Path, s0: f64, s1: f64, source: &Shape) -> Entity {
    if let (Shape::Arc { .. } | Shape::Circle { .. }, Some(Edge::Arc { c, r, a0, .. })) =
        (source, path.edges.first())
    {
        // The edge is counter-clockwise; s is arc length along it.
        return Entity::new(Shape::Arc {
            c: *c,
            r: *r,
            a0: norm_angle(a0 + s0 / r),
            a1: norm_angle(a0 + s1 / r),
        });
    }
    let l = path.length;
    let mut pts = vec![point_at_s(path, s0)];
    let mut bulges = Vec::new();
    let rounds = if path.closed { 2 } else { 1 };
    for round in 0..rounds {
        for (i, ed) in path.edges.iter().enumerate() {
            let len = edge_length(ed);
            let g0 = path.cum[i] + round as f64 * l;
            let lo = js_max(s0, g0);
            let hi = js_min(s1, g0 + len);
            if hi - lo <= 1e-9 * js_max(1.0, l) {
                continue;
            }
            let t1 = (hi - g0) / or(len, 1.0);
            let t0 = (lo - g0) / or(len, 1.0);
            bulges.push(match *ed {
                Edge::Arc { sweep, .. } => bulge_of_sweep(sweep * (t1 - t0)),
                Edge::Seg { .. } => 0.0,
            });
            pts.push(point_at(ed, js_min(1.0, t1)));
        }
    }
    bulges.push(0.0);
    if matches!(source, Shape::Line { .. }) || (pts.len() == 2 && bulges[0] == 0.0) {
        return Entity::new(Shape::Line {
            a: pts[0],
            b: pts[pts.len() - 1],
        });
    }
    let clean = clean_bulge_path(&pts, Some(&bulges), false, 1e-9);
    Entity::new(Shape::Polyline {
        pts: clean.pts,
        bulges: clean.bulges,
        holes: None,
    })
}

/// Evenly spaced s values: n parts (points between them) or every `step` from the start.
pub enum Division {
    Parts(f64),
    Step(f64),
}

impl FromJson for Division {
    fn from_json(v: &Json) -> Result<Division, String> {
        let Json::Obj(fields) = v else {
            return Err("{ parts } ya da { step } bekleniyordu".into());
        };
        // `'parts' in opts`, as the TypeScript asks.
        if fields.iter().any(|(k, _)| k == "parts") {
            Ok(Division::Parts(read_field(v, "parts")?))
        } else {
            Ok(Division::Step(read_field(v, "step")?))
        }
    }
}

pub fn division_params(path: &Path, opts: &Division) -> Vec<f64> {
    let l = path.length;
    let mut out = Vec::new();
    match *opts {
        Division::Parts(parts) => {
            let n = js_floor(parts);
            if n < 2.0 {
                return out;
            }
            // A closed path gets n points (its start included); an open one n − 1.
            let mut k = if path.closed { 0.0 } else { 1.0 };
            while k < n {
                out.push((l * k) / n);
                k += 1.0;
            }
        }
        Division::Step(step) => {
            if !(step > 0.0) {
                return out;
            }
            let eps = 1e-9 * js_max(1.0, l);
            let mut s = step;
            while s < l - eps {
                out.push(s);
                s += step;
            }
        }
    }
    out
}

/// The Böl tool's points in one call (S3): `division_params` along the
/// path, counted from the end when `from_end`; an ellipse is measured along
/// fine chords, so its points are then placed on the true curve.
pub fn division_points(e: &Shape, opts: &Division, from_end: bool) -> Vec<Vec2> {
    let Some(path) = path_of(e) else {
        return Vec::new();
    };
    let at = |s: f64| point_at_s(&path, if from_end { path.length - s } else { s });
    let params = division_params(&path, opts);
    match *e {
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => {
            let g = ellipse_geom(c, major, ratio, t0, t1);
            params
                .into_iter()
                .map(|s| ellipse_point(&g, closest_param(&g, at(s))))
                .collect()
        }
        _ => params.into_iter().map(at).collect(),
    }
}

pub(crate) static OPS: &[Op] = &[
    op!("pathOf", |e: Entity| path_of(&e.shape)),
    op!("normS", |path: Path, s: f64| norm_s(&path, s)),
    op!("pointAtS", |path: Path, s: f64| point_at_s(&path, s)),
    op!("tangentAtS", |path: Path, s: f64| tangent_at_s(&path, s)),
    op!("nearestS", |path: Path, p: Vec2| nearest_s(&path, p)),
    op!("cutsOn", |path: Path, boundaries: Vec<Edge>| cuts_on(
        &path,
        &boundaries
    )),
    op!("subPath", |path: Path, s0: f64, s1: f64, source: Entity| {
        sub_path(&path, s0, s1, &source.shape)
    }),
    op!("divisionParams", |path: Path, opts: Division| {
        division_params(&path, &opts)
    }),
    op!(
        "divisionPoints",
        |e: Entity, opts: Division, from_end: bool| { division_points(&e.shape, &opts, from_end) }
    ),
];
