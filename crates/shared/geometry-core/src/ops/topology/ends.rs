//! Uzat, Buda and Uçlar onto an edge (docs/adr/0148 §5), on the joined
//! drawing: an open path's end that touches no other object is trimmed
//! where it runs past a crossing by at most the tolerance, or extended
//! along its straight end edge to a line it stops short of (the smaller of
//! the two), or else moved onto the nearest line within the tolerance. The
//! boundaries are the other objects' edges; every end is worked out on the
//! drawing as joined, then all are applied. Candidates are taken in the
//! drawing's order (object, then edge), so a tie goes the same way on
//! every target.

use super::{Out, TOUCH, TopoPath, TopoWorks, Work};
use crate::geom::bulge::bulge_path_edges;
use crate::geom::intersect::{Edge, closest_on_edge, intersect_edges, ray_edge};
use crate::geometry::Bounds;
use crate::jsmath::{js_hypot, js_max, js_min, tan};
use crate::store::rtree::{PackedTree, overlaps};
use crate::vec2::Vec2;

/// A cut: its distance along the path, its edge, the fraction along that edge, the place.
#[derive(Clone, Copy, Debug)]
struct Cut {
    s: f64,
    j: usize,
    t: f64,
    q: Vec2,
}

#[derive(Clone, Copy, Debug)]
enum Fix {
    Trim(Cut),
    Extend { t: f64, to: Vec2 },
    Edge { to: Vec2 },
}

#[derive(Default)]
struct Plan {
    start: Option<Fix>,
    end: Option<Fix>,
}

fn dist(a: Vec2, b: Vec2) -> f64 {
    js_hypot(b.x - a.x, b.y - a.y)
}

fn edge_length(e: &Edge) -> f64 {
    match *e {
        Edge::Seg { a, b } => dist(a, b),
        Edge::Arc { r, sweep, .. } => r * sweep.abs(),
    }
}

fn edge_box(e: &Edge) -> Bounds {
    match *e {
        Edge::Seg { a, b } => Bounds {
            min_x: js_min(a.x, b.x),
            min_y: js_min(a.y, b.y),
            max_x: js_max(a.x, b.x),
            max_y: js_max(a.y, b.y),
        },
        Edge::Arc { c, r, .. } => Bounds {
            min_x: c.x - r,
            min_y: c.y - r,
            max_x: c.x + r,
            max_y: c.y + r,
        },
    }
}

fn around(p: Vec2, d: f64) -> Bounds {
    Bounds {
        min_x: p.x - d,
        min_y: p.y - d,
        max_x: p.x + d,
        max_y: p.y + d,
    }
}

fn path_edges(p: &TopoPath) -> Vec<Edge> {
    bulge_path_edges(&p.pts, p.bulges.as_deref(), p.closed)
}

/// Every object's edges and vertices, for what lies near an end.
struct Index {
    edges: Vec<(usize, Edge)>,
    tree: PackedTree,
    verts: Vec<(usize, Vec2)>,
    vtree: PackedTree,
}

impl Index {
    fn new(objs: &[Work]) -> Index {
        let mut edges = Vec::new();
        let mut verts = Vec::new();
        for (o, w) in objs.iter().enumerate() {
            for p in &w.paths {
                for e in path_edges(p) {
                    edges.push((o, e));
                }
                if w.kind != super::Kind::Edges {
                    for &q in &p.pts {
                        verts.push((o, q));
                    }
                }
            }
        }
        let boxes: Vec<(u32, Bounds)> = edges
            .iter()
            .enumerate()
            .map(|(i, (_, e))| (i as u32, edge_box(e)))
            .collect();
        let vboxes: Vec<(u32, Bounds)> = verts
            .iter()
            .enumerate()
            .map(|(i, (_, q))| (i as u32, around(*q, 0.0)))
            .collect();
        Index {
            tree: PackedTree::build(&boxes),
            vtree: PackedTree::build(&vboxes),
            edges,
            verts,
        }
    }

    /// The other objects' edges whose boxes reach `area`, in the drawing's order.
    fn edges_near(&self, own: usize, area: &Bounds) -> Vec<&Edge> {
        let mut hits = Vec::new();
        self.tree.search(area, &mut hits);
        hits.sort_unstable();
        hits.into_iter()
            .map(|i| &self.edges[i as usize])
            .filter(|(o, e)| *o != own && overlaps(&edge_box(e), area))
            .map(|(_, e)| e)
            .collect()
    }

    /// Whether another object's vertex lies within TOUCH of p.
    fn vertex_at(&self, own: usize, p: Vec2) -> bool {
        let mut hits = Vec::new();
        self.vtree.search(&around(p, TOUCH), &mut hits);
        hits.iter().any(|&i| {
            let (o, q) = self.verts[i as usize];
            o != own && dist(p, q) <= TOUCH
        })
    }
}

/// What to do with one end, or nothing.
fn plan_end(
    p: &TopoPath,
    own: usize,
    start: bool,
    tol: f64,
    works: TopoWorks,
    ix: &Index,
) -> Option<Fix> {
    let n = p.pts.len();
    let i = if start { 0 } else { n - 1 };
    let e = p.pts[i];
    // Free: no other object's vertex or edge, no other vertex of its path, within TOUCH.
    if ix.vertex_at(own, e) {
        return None;
    }
    if ix
        .edges_near(own, &around(e, TOUCH))
        .iter()
        .any(|b| closest_on_edge(b, e).d <= TOUCH)
    {
        return None;
    }
    if p.pts
        .iter()
        .enumerate()
        .any(|(j, &q)| j != i && dist(e, q) <= TOUCH)
    {
        return None;
    }
    let reach = around(e, tol);
    let near = ix.edges_near(own, &reach);
    let own_edges = path_edges(p);
    let lengths: Vec<f64> = own_edges.iter().map(edge_length).collect();
    let total: f64 = lengths.iter().sum();
    let mut best: Option<(Fix, f64)> = None;
    if works.trim {
        let mut before = 0.0;
        let mut pick: Option<Cut> = None;
        for (j, ed) in own_edges.iter().enumerate() {
            for b in &near {
                for h in intersect_edges(ed, b) {
                    let s = before + h.t * lengths[j];
                    let cut = Cut {
                        s,
                        j,
                        t: h.t,
                        q: h.p,
                    };
                    let (amount, better) = if start {
                        (s, pick.is_none_or(|c| s < c.s))
                    } else {
                        (total - s, pick.is_none_or(|c| s > c.s))
                    };
                    if amount > TOUCH && amount <= tol && better {
                        pick = Some(cut);
                    }
                }
            }
            before += lengths[j];
        }
        if let Some(c) = pick {
            best = Some((Fix::Trim(c), if start { c.s } else { total - c.s }));
        }
    }
    if works.extend {
        let edge = if start {
            own_edges[0]
        } else {
            own_edges[own_edges.len() - 1]
        };
        if let Edge::Seg { a, b } = edge {
            let (from, to) = if start { (b, a) } else { (a, b) };
            let l = dist(from, to);
            if l > 0.0 {
                let d = Vec2::new((to.x - from.x) / l, (to.y - from.y) / l);
                let mut hit: Option<f64> = None;
                for bnd in &near {
                    for t in ray_edge(to, d, bnd, TOUCH) {
                        if t <= tol && hit.is_none_or(|h| t < h) {
                            hit = Some(t);
                        }
                    }
                }
                if let Some(t) = hit
                    && best.is_none_or(|(_, amount)| t < amount)
                {
                    best = Some((
                        Fix::Extend {
                            t,
                            to: Vec2::new(to.x + d.x * t, to.y + d.y * t),
                        },
                        t,
                    ));
                }
            }
        }
    }
    if best.is_none() && works.ends {
        let mut nearest: Option<(f64, Vec2)> = None;
        for b in &near {
            let c = closest_on_edge(b, e);
            if c.d > TOUCH && c.d <= tol && nearest.is_none_or(|(d, _)| c.d < d) {
                nearest = Some((c.d, c.p));
            }
        }
        if let Some((d, to)) = nearest {
            best = Some((Fix::Edge { to }, d));
        }
    }
    best.map(|(f, _)| f)
}

/// A cut's place and elevation: the crossing, and its edge's elevation by
/// length (an arc by angle) when both its ends have one, else the cut end's own.
fn cut_point(p: &TopoPath, c: &Cut, start: bool) -> (Vec2, Option<f64>) {
    let n = p.pts.len();
    let (za, zb) = (p.zs[c.j], p.zs[(c.j + 1) % n]);
    let z = match (za, zb) {
        (Some(a), Some(b)) => Some(a + (b - a) * c.t),
        _ => {
            if start {
                p.zs[0]
            } else {
                p.zs[n - 1]
            }
        }
    };
    (c.q, z)
}

/// Works the free ends out on the joined drawing, then applies them all.
pub(super) fn ends(objs: &mut [Work], tol: f64, works: TopoWorks, out: &mut Out) {
    let ix = Index::new(objs);
    let mut plans: Vec<(usize, Plan)> = Vec::new();
    for (o, w) in objs.iter().enumerate() {
        if w.fixed || !w.kind.open() {
            continue;
        }
        let p = &w.paths[0];
        if p.closed || p.pts.len() < 2 {
            continue;
        }
        let mut plan = Plan {
            start: plan_end(p, o, true, tol, works, &ix),
            end: plan_end(p, o, false, tol, works, &ix),
        };
        // Both ends cut past each other: neither is.
        if let (Some(Fix::Trim(a)), Some(Fix::Trim(b))) = (plan.start, plan.end)
            && a.s >= b.s - TOUCH
        {
            plan = Plan::default();
        }
        if plan.start.is_some() || plan.end.is_some() {
            plans.push((o, plan));
        }
    }
    for (o, plan) in plans {
        let w = &mut objs[o];
        let p = &mut w.paths[0];
        let (first, last) = (p.pts[0], p.pts[p.pts.len() - 1]);
        let own = path_edges(p);
        let cut_start = match plan.start {
            Some(Fix::Trim(c)) => Some(c),
            _ => None,
        };
        let cut_end = match plan.end {
            Some(Fix::Trim(c)) => Some(c),
            _ => None,
        };
        // The cuts, on the path as it was: a sub-path from the start's cut to the end's.
        if cut_start.is_some() || cut_end.is_some() {
            let n = p.pts.len();
            let (j0, t0) = cut_start.map_or((0, 0.0), |c| (c.j, c.t));
            let (j1, t1) = cut_end.map_or((n - 2, 1.0), |c| (c.j, c.t));
            let mut pts = Vec::with_capacity(j1 - j0 + 2);
            let mut zs = Vec::with_capacity(j1 - j0 + 2);
            let (q, z) = match &cut_start {
                Some(c) => cut_point(p, c, true),
                None => (p.pts[0], p.zs[0]),
            };
            pts.push(q);
            zs.push(z);
            for i in j0 + 1..=j1 {
                pts.push(p.pts[i]);
                zs.push(p.zs[i]);
            }
            let (q, z) = match &cut_end {
                Some(c) => cut_point(p, c, false),
                None => (p.pts[n - 1], p.zs[n - 1]),
            };
            pts.push(q);
            zs.push(z);
            if p.bulges.is_some() {
                let mut bulges = Vec::with_capacity(pts.len());
                for (i, ed) in own.iter().enumerate().take(j1 + 1).skip(j0) {
                    let lo = if i == j0 { t0 } else { 0.0 };
                    let hi = if i == j1 { t1 } else { 1.0 };
                    bulges.push(match *ed {
                        Edge::Arc { sweep, .. } => tan(sweep * (hi - lo) / 4.0),
                        Edge::Seg { .. } => 0.0,
                    });
                }
                bulges.push(0.0);
                p.bulges = Some(bulges);
            }
            p.pts = pts;
            p.zs = zs;
        }
        for (start, fix) in [(true, plan.start), (false, plan.end)] {
            let Some(fix) = fix else { continue };
            let n = p.pts.len();
            let i = if start { 0 } else { n - 1 };
            let old = if start { first } else { last };
            match fix {
                Fix::Extend { t, to } => {
                    let a = if start { p.pts[1] } else { p.pts[n - 2] };
                    let (za, zb) = if start {
                        (p.zs[1], p.zs[0])
                    } else {
                        (p.zs[n - 2], p.zs[n - 1])
                    };
                    let end_at = p.pts[i];
                    p.pts[i] = to;
                    if let (Some(za), Some(zb)) = (za, zb) {
                        p.zs[i] = Some(zb + (zb - za) * (t / dist(a, end_at)));
                    }
                    out.counts.extended += 1;
                }
                Fix::Edge { to } => {
                    p.pts[i] = to;
                    out.counts.edges += 1;
                }
                Fix::Trim(_) => out.counts.trimmed += 1,
            }
            let kind = match fix {
                Fix::Extend { .. } => "extended",
                Fix::Edge { .. } => "edge",
                Fix::Trim(_) => "trimmed",
            };
            let new = p.pts[i];
            out.note(kind, old, new, o);
        }
        w.changed = true;
    }
}
