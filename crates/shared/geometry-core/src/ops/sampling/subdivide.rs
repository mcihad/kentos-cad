//! Alt bölümlere ayır (docs/adr/0220 §6): PostGIS's `ST_Subdivide` on
//! KentOS's curved geometry. A piece with more vertices than the most is
//! cut across the long side of its box, at the coordinate of the ring's
//! vertex nearest the middle (a path's, and a pivot on the box's side, at
//! the middle); an area's halves are cut along the line (`cut`: what the
//! overlay gives, arcs exact, in a fraction of its time), a path is cut on
//! the line and its pieces go by their middles, so a piece on the line goes
//! one way. Each result is taken again, at most 50 deep. The
//! pieces keep their elevations by docs/adr/0142's rules.

use super::cut::{Line, cut_area};
use crate::entity::Shape;
use crate::geom::arrangement::{Area, Ring, TOL, tight_box};
use crate::geom::intersect::{Edge, closest_on_edge, intersect_edges, point_at};
use crate::geom::region::{ring_area, ring_edges};
use crate::geometry::{Bounds, empty_bounds};
use crate::jsmath::{js_hypot, js_max, js_min};
use crate::ops::areas::polygon_of_area;
use crate::ops::convert::{Elevations, Zs, paths};
use crate::ops::elevation::ON;
use crate::ops::geoprocess::{Class, chain, class_of, path_shape};
use crate::store::rtree::PackedTree;
use crate::vec2::Vec2;

/// How deep the cutting goes.
pub const DEPTH: u32 = 50;

/// The least “most vertices”.
pub const LEAST: usize = 5;

enum Piece {
    Area(Area),
    Path(Vec<Edge>),
}

fn grow(b: &mut Bounds, o: &Bounds) {
    b.min_x = js_min(b.min_x, o.min_x);
    b.min_y = js_min(b.min_y, o.min_y);
    b.max_x = js_max(b.max_x, o.max_x);
    b.max_y = js_max(b.max_y, o.max_y);
}

fn edges_box(edges: &[Edge]) -> Bounds {
    let mut b = empty_bounds();
    for e in edges {
        grow(&mut b, &tight_box(e));
    }
    b
}

impl Piece {
    fn vertices(&self) -> usize {
        match self {
            Piece::Area(a) => {
                a.outer.pts.len() + a.holes.iter().map(|h| h.pts.len()).sum::<usize>()
            }
            Piece::Path(edges) => edges.len() + 1,
        }
    }

    fn bounds(&self) -> Bounds {
        match self {
            Piece::Area(a) => edges_box(&ring_edges(&a.outer)),
            Piece::Path(edges) => edges_box(edges),
        }
    }
}

/// A ring's lowest vertex: least y, then least x.
fn lowest(r: &Ring) -> Vec2 {
    let mut best = r.pts[0];
    for &p in &r.pts[1..] {
        if p.y < best.y || (p.y == best.y && p.x < best.x) {
            best = p;
        }
    }
    best
}

/// The ring whose vertex sets the cut: the outer ring, or the largest hole when the holes hold most vertices.
fn trimmed(a: &Area, n: usize) -> &Ring {
    if a.holes.is_empty() || n < 2 * a.outer.pts.len() {
        return &a.outer;
    }
    let mut best = &a.holes[0];
    let mut size = ring_area(best).abs();
    for h in &a.holes[1..] {
        let s = ring_area(h).abs();
        let (p, q) = (lowest(h), lowest(best));
        if s > size || (s == size && (p.y < q.y || (p.y == q.y && p.x < q.x))) {
            best = h;
            size = s;
        }
    }
    best
}

/// Where the cut goes on its axis (`across_x`: a vertical line).
fn pivot(piece: &Piece, b: &Bounds, across_x: bool, n: usize) -> f64 {
    let (lo, hi) = if across_x {
        (b.min_x, b.max_x)
    } else {
        (b.min_y, b.max_y)
    };
    let middle = (lo + hi) / 2.0;
    let Piece::Area(a) = piece else {
        return middle;
    };
    let ring = trimmed(a, n);
    let mut best = (f64::INFINITY, middle);
    for p in &ring.pts {
        let c = if across_x { p.x } else { p.y };
        let gap = (c - middle).abs();
        if gap < best.0 || (gap == best.0 && c < best.1) {
            best = (gap, c);
        }
    }
    let c = best.1;
    if (c - lo).abs() <= 1e-12 || (c - hi).abs() <= 1e-12 {
        middle
    } else {
        c
    }
}

/// The piece of an edge between parameters `t0` and `t1`, from `p0` to `p1`.
fn sub_edge(e: &Edge, t0: f64, t1: f64, p0: Vec2, p1: Vec2) -> Edge {
    match *e {
        Edge::Seg { .. } => Edge::Seg { a: p0, b: p1 },
        Edge::Arc { c, r, a0, sweep } => Edge::Arc {
            c,
            r,
            a0: a0 + sweep * t0,
            sweep: sweep * (t1 - t0),
        },
    }
}

/// An edge cut where it crosses the line (a straight edge's crossing exactly on the line).
fn cut_edge(e: &Edge, across_x: bool, cut: f64, b: &Bounds) -> Vec<Edge> {
    let mut ts: Vec<(f64, Vec2)> = Vec::new();
    match *e {
        Edge::Seg { a, b: q } => {
            let (ca, cb) = if across_x { (a.x, q.x) } else { (a.y, q.y) };
            if (ca < cut && cb > cut) || (ca > cut && cb < cut) {
                let t = (cut - ca) / (cb - ca);
                let p = if across_x {
                    Vec2::new(cut, a.y + (q.y - a.y) * t)
                } else {
                    Vec2::new(a.x + (q.x - a.x) * t, cut)
                };
                ts.push((t, p));
            }
        }
        Edge::Arc { .. } => {
            let line = if across_x {
                Edge::Seg {
                    a: Vec2::new(cut, b.min_y - 1.0),
                    b: Vec2::new(cut, b.max_y + 1.0),
                }
            } else {
                Edge::Seg {
                    a: Vec2::new(b.min_x - 1.0, cut),
                    b: Vec2::new(b.max_x + 1.0, cut),
                }
            };
            for h in intersect_edges(e, &line) {
                if h.t > 1e-12 && h.t < 1.0 - 1e-12 {
                    ts.push((h.t, h.p));
                }
            }
            ts.sort_by(|x, y| x.0.total_cmp(&y.0));
        }
    }
    let (start, end) = (point_at(e, 0.0), point_at(e, 1.0));
    let mut out = Vec::with_capacity(ts.len() + 1);
    let (mut t0, mut p0) = (0.0, start);
    for (t, p) in ts {
        if js_hypot(p.x - p0.x, p.y - p0.y) > TOL {
            out.push(sub_edge(e, t0, t, p0, p));
            t0 = t;
            p0 = p;
        }
    }
    out.push(sub_edge(e, t0, 1.0, p0, end));
    out
}

/// A path cut on the line: its runs below (on the line counts below) and above.
fn split_path(
    edges: &[Edge],
    across_x: bool,
    cut: f64,
    b: &Bounds,
) -> (Vec<Vec<Edge>>, Vec<Vec<Edge>>) {
    let (mut low, mut high) = (Vec::new(), Vec::new());
    for e in edges {
        for piece in cut_edge(e, across_x, cut, b) {
            let m = point_at(&piece, 0.5);
            let c = if across_x { m.x } else { m.y };
            if c <= cut {
                low.push(piece);
            } else {
                high.push(piece);
            }
        }
    }
    (chain(low), chain(high))
}

fn recurse(piece: Piece, max: usize, depth: u32, out: &mut Vec<Piece>) {
    let n = piece.vertices();
    if n <= max || depth > DEPTH {
        out.push(piece);
        return;
    }
    let b = piece.bounds();
    let (w, h) = (b.max_x - b.min_x, b.max_y - b.min_y);
    if w == 0.0 && h == 0.0 {
        out.push(piece);
        return;
    }
    let across_x = w > h;
    let cut = pivot(&piece, &b, across_x, n);
    match piece {
        Piece::Area(a) => {
            for side in cut_area(&a, Line { across_x, at: cut }) {
                for got in side {
                    recurse(Piece::Area(got), max, depth + 1, out);
                }
            }
        }
        Piece::Path(edges) => {
            let (low, high) = split_path(&edges, across_x, cut, &b);
            for run in low.into_iter().chain(high) {
                recurse(Piece::Path(run), max, depth + 1, out);
            }
        }
    }
}

/// A ring turned to run counter-clockwise (`ccw`) or clockwise and to start
/// at its lowest vertex (least y, then least x), its bulges with it: the
/// pieces as the reference writes them.
fn settled(r: &Ring, ccw: bool) -> Ring {
    let n = r.pts.len();
    let mut pts = r.pts.clone();
    let mut bulges = r.bulges.clone().unwrap_or_else(|| vec![0.0; n]);
    bulges.resize(n, 0.0);
    if (ring_area(r) > 0.0) != ccw {
        // Reversed, edge k runs back along the old edge n − 2 − k (mod n), its bulge negated.
        bulges = (0..n).map(|k| -bulges[(2 * n - 2 - k) % n]).collect();
        pts.reverse();
    }
    let low = lowest(&Ring {
        pts: pts.clone(),
        bulges: None,
    });
    let start = pts.iter().position(|&p| p == low).unwrap_or(0);
    pts.rotate_left(start);
    bulges.rotate_left(start);
    let curved = bulges.iter().any(|&b| b != 0.0);
    Ring {
        pts,
        bulges: curved.then_some(bulges),
    }
}

/// An area as the tool writes it: its outer ring counter-clockwise, its
/// holes clockwise, each from its lowest vertex, the holes by their lowest vertices.
fn settled_area(a: &Area) -> Area {
    let mut holes: Vec<Ring> = a.holes.iter().map(|h| settled(h, false)).collect();
    holes.sort_by(|p, q| {
        let (a, b) = (p.pts[0], q.pts[0]);
        a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x))
    });
    Area {
        outer: settled(&a.outer, true),
        holes,
    }
}

/// The source's elevations for the pieces (docs/adr/0142): a vertex on a
/// source vertex takes its own, on a source edge whose ends have them
/// linearly; the vertices and edges found through packed trees.
struct Heights {
    verts: Vec<(Vec2, f64)>,
    vert_tree: PackedTree,
    edges: Vec<(Edge, Option<f64>, Option<f64>)>,
    edge_tree: PackedTree,
}

impl Heights {
    fn new(s: &Shape, zs: &Zs) -> Heights {
        let mut verts = Vec::new();
        let mut edges = Vec::new();
        for (k, p) in paths(s).iter().enumerate() {
            let z = |i: usize| zs.get(k).and_then(|z| z.get(i)).copied().flatten();
            for (i, &v) in p.pts.iter().enumerate() {
                if let Some(h) = z(i) {
                    verts.push((v, h));
                }
            }
            for i in 0..p.edge_count() {
                edges.push((p.edge(i), z(i), z((i + 1) % p.pts.len())));
            }
        }
        let vert_boxes: Vec<(u32, Bounds)> = verts
            .iter()
            .enumerate()
            .map(|(i, (v, _))| {
                (
                    i as u32,
                    Bounds {
                        min_x: v.x,
                        min_y: v.y,
                        max_x: v.x,
                        max_y: v.y,
                    },
                )
            })
            .collect();
        let edge_boxes: Vec<(u32, Bounds)> = edges
            .iter()
            .enumerate()
            .map(|(i, (e, _, _))| (i as u32, tight_box(e)))
            .collect();
        Heights {
            vert_tree: PackedTree::build(&vert_boxes),
            verts,
            edge_tree: PackedTree::build(&edge_boxes),
            edges,
        }
    }

    fn z(&self, p: Vec2, near: &mut Vec<u32>) -> Option<f64> {
        let q = Bounds {
            min_x: p.x - ON,
            min_y: p.y - ON,
            max_x: p.x + ON,
            max_y: p.y + ON,
        };
        near.clear();
        self.vert_tree.search(&q, near);
        near.sort_unstable();
        for &i in near.iter() {
            let (v, h) = self.verts[i as usize];
            if js_hypot(p.x - v.x, p.y - v.y) <= ON {
                return Some(h);
            }
        }
        near.clear();
        self.edge_tree.search(&q, near);
        near.sort_unstable();
        for &i in near.iter() {
            let (e, za, zb) = &self.edges[i as usize];
            let hit = closest_on_edge(e, p);
            if hit.d <= ON
                && let (Some(za), Some(zb)) = (za, zb)
            {
                return Some(za + (zb - za) * hit.t);
            }
        }
        None
    }
}

/// A piece's elevations in `paths`' order; none when no vertex has one.
fn piece_zs(shape: &Shape, heights: &Heights, near: &mut Vec<u32>) -> Option<Zs> {
    let zs: Zs = paths(shape)
        .iter()
        .map(|p| p.pts.iter().map(|&v| heights.z(v, near)).collect())
        .collect();
    zs.iter()
        .any(|z| z.iter().any(Option::is_some))
        .then_some(zs)
}

/// The object's pieces, ordered by their boxes' lower left corners (west, then south; then their upper right
/// corners and first vertices), with their elevations.
pub fn subdivide(s: &Shape, zs: Option<&Zs>, max: usize) -> Vec<Elevations> {
    let max = max.max(LEAST);
    let mut pieces = Vec::new();
    match class_of(s) {
        Class::Areas(areas) => {
            for a in areas {
                recurse(Piece::Area(a), max, 0, &mut pieces);
            }
        }
        Class::Paths(list) => {
            for p in list {
                recurse(Piece::Path(p), max, 0, &mut pieces);
            }
        }
        _ => {}
    }
    let mut placed: Vec<(Bounds, Shape)> = pieces
        .into_iter()
        .filter_map(|p| {
            let b = p.bounds();
            let shape = match p {
                Piece::Area(a) => Some(polygon_of_area(&settled_area(&a)).shape),
                Piece::Path(edges) => path_shape(&edges),
            }?;
            Some((b, shape))
        })
        .collect();
    // By the boxes' lower left corners, then their upper right ones, then the first (lowest) vertices: an order
    // that does not hang on how the overlay hands its parts back.
    let first = |s: &Shape| {
        match s {
            Shape::Polygon { pts, .. } | Shape::Polyline { pts, .. } => pts.first().copied(),
            _ => None,
        }
        .unwrap_or(Vec2::new(0.0, 0.0))
    };
    placed.sort_by(|a, b| {
        let (p, q) = (first(&a.1), first(&b.1));
        a.0.min_x
            .total_cmp(&b.0.min_x)
            .then(a.0.min_y.total_cmp(&b.0.min_y))
            .then(a.0.max_x.total_cmp(&b.0.max_x))
            .then(a.0.max_y.total_cmp(&b.0.max_y))
            .then(p.y.total_cmp(&q.y))
            .then(p.x.total_cmp(&q.x))
    });
    let heights = zs
        .filter(|z| z.iter().any(|p| p.iter().any(Option::is_some)))
        .map(|z| Heights::new(s, z));
    let mut near = Vec::new();
    placed
        .into_iter()
        .map(|(_, shape)| {
            let zs = heights
                .as_ref()
                .and_then(|h| piece_zs(&shape, h, &mut near));
            Elevations { shape, zs }
        })
        .collect()
}
