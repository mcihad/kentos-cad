//! Yüzey yapıştır (docs/adr/3920): another surface of the project pasted into a surface's input, as Civil 3D's Paste
//! Surface and OpenRoads' Merge: where the pasted surface's kept triangles lie, the records made so far go and its
//! corners and edges come in, so its triangles are made again as they are and the rest is triangulated round them.
//!
//! The rule, exact (the shared core's orientation predicate):
//! - a record is covered when its place is inside a kept triangle of the pasted surface or on one's edge;
//! - a breakline piece stays when neither end is covered and it meets no edge of the pasted surface's outline (the
//!   edges of one kept triangle only), touching counts; the pieces left are cut into runs, a closed ring that lost a
//!   piece starting after the first piece lost; a breakline too short to have a piece goes when a corner is covered;
//! - the pasted surface's corners (those of its kept triangles) come in ordered by place, x then y, each with its
//!   height; its edges as breaklines of two corners, each from its smaller corner by place, ordered by them.

use kentos_contracts::{LayerNode, SurfaceEdit, TerrainSurfaceDef};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::predicates::orient2d;

use crate::sources::{SourceObject, gather_in};
use crate::surface::{Breakline, Surface, build};

/// What a pasted surface covers.
pub(crate) struct Cover {
    xy: Vec<Vec2>,
    z: Vec<f64>,
    triangles: Vec<[u32; 3]>,
    /// The outline: the kept triangles' edges that one triangle alone has.
    outline: Vec<[u32; 2]>,
    /// The triangles by cell of a uniform grid over their box: a place asks only its cell's.
    grid: Grid,
}

struct Grid {
    min: Vec2,
    size: f64,
    columns: usize,
    rows: usize,
    cells: Vec<Vec<u32>>,
}

impl Grid {
    fn cell(&self, x: f64, y: f64) -> Option<usize> {
        let (c, r) = ((x - self.min.x) / self.size, (y - self.min.y) / self.size);
        if !(c >= 0.0 && r >= 0.0) {
            return None;
        }
        let (c, r) = (c as usize, r as usize);
        (c < self.columns && r < self.rows).then_some(r * self.columns + c)
    }
}

/// The surface `terrain` (one of `others`) as `into` pastes it: made from the drawing now, as it is made on its own;
/// why not: `no_surface` (no such surface), `cycle` (it is `into`, or its pastes, followed on, come back to `into`:
/// decided on the definitions alone, so a surface is the same wherever it is pasted), `unbuildable`.
pub(crate) fn pasted<'a>(
    into: &str,
    terrain: &str,
    others: &[TerrainSurfaceDef],
    tree: &[LayerNode],
    objects: &[SourceObject<'a>],
) -> Result<Cover, &'static str> {
    let Some(def) = others.iter().find(|d| d.id == terrain) else {
        return Err("no_surface");
    };
    if terrain == into || reaches(others, terrain, into) {
        return Err("cycle");
    }
    // Each step pastes a surface whose pastes do not come back to the one pasting it, so no surface comes twice on
    // one chain and the depth is at most the number of surfaces.
    let g = gather_in(def, others, tree, objects);
    let surface = build(&g.input).map_err(|_| "unbuildable")?;
    Ok(Cover::of(&surface))
}

/// Whether `target` is pasted, at any depth, by the surface `from` (one of `others`; one not there pastes nothing).
fn reaches(others: &[TerrainSurfaceDef], from: &str, target: &str) -> bool {
    let mut todo = vec![from];
    let mut seen = vec![from];
    while let Some(id) = todo.pop() {
        let Some(def) = others.iter().find(|d| d.id == id) else { continue };
        for e in &def.edits {
            if let SurfaceEdit::Paste { terrain } = e {
                if terrain == target {
                    return true;
                }
                if !seen.contains(&terrain.as_str()) {
                    seen.push(terrain);
                    todo.push(terrain);
                }
            }
        }
    }
    false
}

impl Cover {
    pub(crate) fn of(s: &Surface) -> Self {
        let xy: Vec<Vec2> = s.vertices.iter().map(|v| Vec2::new(v[0], v[1])).collect();
        let z: Vec<f64> = s.vertices.iter().map(|v| v[2]).collect();
        let triangles = s.triangles.clone();
        let mut edges: Vec<[u32; 2]> = triangles
            .iter()
            .flat_map(|t| [[t[0], t[1]], [t[1], t[2]], [t[2], t[0]]])
            .map(|[a, b]| [a.min(b), a.max(b)])
            .collect();
        edges.sort_unstable();
        let mut outline = Vec::new();
        let mut k = 0;
        while k < edges.len() {
            let mut n = 1;
            while k + n < edges.len() && edges[k + n] == edges[k] {
                n += 1;
            }
            if n == 1 {
                outline.push(edges[k]);
            }
            k += n;
        }
        let grid = grid_of(&xy, &triangles);
        Self { xy, z, triangles, outline, grid }
    }

    /// Whether `(x, y)` is inside a kept triangle or on one's edge.
    pub(crate) fn covers(&self, x: f64, y: f64) -> bool {
        let p = Vec2::new(x, y);
        let Some(cell) = self.grid.cell(x, y) else { return false };
        self.grid.cells[cell].iter().any(|&t| {
            let [a, b, c] = self.triangles[t as usize].map(|i| self.xy[i as usize]);
            orient2d(a, b, p) >= 0.0 && orient2d(b, c, p) >= 0.0 && orient2d(c, a, p) >= 0.0
        })
    }

    /// Whether the piece from `p` to `q` meets an edge of the outline (touching counts).
    fn crosses(&self, p: Vec2, q: Vec2) -> bool {
        let (lo, hi) = (Vec2::new(p.x.min(q.x), p.y.min(q.y)), Vec2::new(p.x.max(q.x), p.y.max(q.y)));
        self.outline.iter().any(|&[i, j]| {
            let (c, d) = (self.xy[i as usize], self.xy[j as usize]);
            if c.x.max(d.x) < lo.x || c.x.min(d.x) > hi.x || c.y.max(d.y) < lo.y || c.y.min(d.y) > hi.y {
                return false;
            }
            meets(p, q, c, d)
        })
    }

    /// A breakline's pieces that stay, as runs (the module's rule).
    pub(crate) fn runs(&self, b: Breakline) -> Vec<Breakline> {
        let n = b.points.len();
        if n < if b.closed { 3 } else { 2 } {
            // No pieces (the core refuses it): it goes as a record does, when a corner is covered.
            let covered = b.points.iter().any(|p| self.covers(p[0], p[1]));
            return if covered { Vec::new() } else { vec![b] };
        }
        let pieces = if b.closed { n } else { n - 1 };
        let at = |k: usize| Vec2::new(b.points[k][0], b.points[k][1]);
        let keep: Vec<bool> = (0..pieces)
            .map(|k| {
                let (p, q) = (at(k), at((k + 1) % n));
                !self.covers(p.x, p.y) && !self.covers(q.x, q.y) && !self.crosses(p, q)
            })
            .collect();
        if keep.iter().all(|k| *k) {
            return vec![b];
        }
        let start = if b.closed { keep.iter().position(|k| !k).map_or(0, |i| i + 1) } else { 0 };
        let mut out = Vec::new();
        let mut run: Vec<[f64; 3]> = Vec::new();
        for j in 0..pieces {
            let k = (start + j) % pieces;
            if keep[k] {
                if run.is_empty() {
                    run.push(b.points[k]);
                }
                run.push(b.points[(k + 1) % n]);
            } else if !run.is_empty() {
                out.push(Breakline { points: std::mem::take(&mut run), closed: false, source: b.source.clone() });
            }
        }
        if !run.is_empty() {
            out.push(Breakline { points: run, closed: false, source: b.source.clone() });
        }
        out
    }

    /// The corners of the kept triangles with their heights, by place (x, then y).
    pub(crate) fn corners(&self) -> Vec<[f64; 3]> {
        let mut used: Vec<u32> = self.triangles.iter().flatten().copied().collect();
        used.sort_unstable();
        used.dedup();
        let mut out: Vec<[f64; 3]> = used.iter().map(|&i| [self.xy[i as usize].x, self.xy[i as usize].y, self.z[i as usize]]).collect();
        out.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
        out
    }

    /// The kept triangles' edges, each from its smaller corner by place, ordered by their corners.
    pub(crate) fn edges(&self) -> Vec<[[f64; 3]; 2]> {
        let mut pairs: Vec<[u32; 2]> = self
            .triangles
            .iter()
            .flat_map(|t| [[t[0], t[1]], [t[1], t[2]], [t[2], t[0]]])
            .map(|[a, b]| [a.min(b), a.max(b)])
            .collect();
        pairs.sort_unstable();
        pairs.dedup();
        let corner = |i: u32| [self.xy[i as usize].x, self.xy[i as usize].y, self.z[i as usize]];
        let by_place = |a: &[f64; 3], b: &[f64; 3]| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1]));
        let mut out: Vec<[[f64; 3]; 2]> = pairs
            .into_iter()
            .map(|[a, b]| {
                let (p, q) = (corner(a), corner(b));
                if by_place(&p, &q).is_le() { [p, q] } else { [q, p] }
            })
            .collect();
        out.sort_by(|e, f| by_place(&e[0], &f[0]).then(by_place(&e[1], &f[1])));
        out
    }
}

/// Whether the closed pieces `pq` and `cd` meet (crossing or touching).
fn meets(p: Vec2, q: Vec2, c: Vec2, d: Vec2) -> bool {
    let (d1, d2) = (orient2d(c, d, p), orient2d(c, d, q));
    let (d3, d4) = (orient2d(p, q, c), orient2d(p, q, d));
    if ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0)) && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0)) {
        return true;
    }
    let within = |u: Vec2, v: Vec2, w: Vec2| w.x >= u.x.min(v.x) && w.x <= u.x.max(v.x) && w.y >= u.y.min(v.y) && w.y <= u.y.max(v.y);
    (d1 == 0.0 && within(c, d, p)) || (d2 == 0.0 && within(c, d, q)) || (d3 == 0.0 && within(p, q, c)) || (d4 == 0.0 && within(p, q, d))
}

/// A uniform grid over the triangles' box, about one triangle a cell; each triangle in every cell its box meets.
fn grid_of(xy: &[Vec2], triangles: &[[u32; 3]]) -> Grid {
    let corners = || triangles.iter().flatten().map(|&i| xy[i as usize]);
    let (mut lo, mut hi) = (Vec2::new(f64::INFINITY, f64::INFINITY), Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY));
    for p in corners() {
        lo = Vec2::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Vec2::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    if triangles.is_empty() || !lo.x.is_finite() {
        return Grid { min: Vec2::new(0.0, 0.0), size: 1.0, columns: 0, rows: 0, cells: Vec::new() };
    }
    let (w, h) = ((hi.x - lo.x).max(1e-9), (hi.y - lo.y).max(1e-9));
    let size = ((w * h) / triangles.len() as f64).sqrt().max(w.max(h) / 1024.0).max(1e-9);
    // At most 1025 a side: the size is at least the box's longer side over 1024.
    let columns = (w / size).floor() as usize + 1;
    let rows = (h / size).floor() as usize + 1;
    let mut cells = vec![Vec::new(); columns * rows];
    for (k, t) in triangles.iter().enumerate() {
        let ps = t.map(|i| xy[i as usize]);
        let (x0, x1) = (ps.iter().map(|p| p.x).fold(f64::INFINITY, f64::min), ps.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max));
        let (y0, y1) = (ps.iter().map(|p| p.y).fold(f64::INFINITY, f64::min), ps.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max));
        let c0 = (((x0 - lo.x) / size).floor() as usize).min(columns - 1);
        let c1 = (((x1 - lo.x) / size).floor() as usize).min(columns - 1);
        let r0 = (((y0 - lo.y) / size).floor() as usize).min(rows - 1);
        let r1 = (((y1 - lo.y) / size).floor() as usize).min(rows - 1);
        for r in r0..=r1 {
            for c in c0..=c1 {
                cells[r * columns + c].push(k as u32);
            }
        }
    }
    Grid { min: lo, size, columns, rows, cells }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::{SpotPoint, SurfaceInput};

    /// A pad over the square (0, 0)–(10, 10), a corner raised in its middle.
    fn cover() -> Cover {
        let p = |x: f64, y: f64, z: f64| SpotPoint { x, y, z, source: None };
        let s = build(&SurfaceInput {
            points: vec![p(0.0, 0.0, 1.0), p(10.0, 0.0, 1.0), p(10.0, 10.0, 1.0), p(0.0, 10.0, 1.0), p(4.0, 6.0, 2.0)],
            ..SurfaceInput::default()
        })
        .expect("a pad");
        Cover::of(&s)
    }

    fn line(points: &[[f64; 2]], closed: bool) -> Breakline {
        Breakline { points: points.iter().map(|p| [p[0], p[1], 0.0]).collect(), closed, source: Some("b".into()) }
    }

    fn places(runs: &[Breakline]) -> Vec<(Vec<[f64; 2]>, bool)> {
        runs.iter().map(|b| (b.points.iter().map(|p| [p[0], p[1]]).collect(), b.closed)).collect()
    }

    #[test]
    fn covered_is_inside_or_on_an_edge_and_the_outline_is_the_square() {
        let c = cover();
        assert!(c.covers(5.0, 5.0) && c.covers(0.0, 5.0) && c.covers(10.0, 10.0) && c.covers(4.0, 6.0));
        assert!(!c.covers(-0.000001, 5.0) && !c.covers(11.0, 5.0) && !c.covers(5.0, -1.0));
        assert_eq!(c.outline.len(), 4);
        assert_eq!(c.corners().len(), 5);
        // Five corners, the square their hull: 3·5 − 4 − 3 edges.
        assert_eq!(c.edges().len(), 8);
        assert!(c.edges().iter().all(|[p, q]| (p[0], p[1]) < (q[0], q[1])));
    }

    #[test]
    fn a_breakline_loses_the_pieces_that_reach_the_pad_and_is_cut_into_runs() {
        let c = cover();
        // Open: two runs round the piece into the pad and the one out of it.
        let open = line(&[[-10.0, -5.0], [-5.0, -5.0], [5.0, -5.0], [5.0, 5.0], [15.0, 5.0], [15.0, -5.0]], false);
        assert_eq!(
            places(&c.runs(open)),
            vec![(vec![[-10.0, -5.0], [-5.0, -5.0], [5.0, -5.0]], false), (vec![[15.0, 5.0], [15.0, -5.0]], false)]
        );
        // A ring round the pad stays whole and closed.
        let round = line(&[[-5.0, -5.0], [15.0, -5.0], [15.0, 15.0], [-5.0, 15.0]], true);
        assert_eq!(places(&c.runs(round.clone())), places(&[round]));
        // A ring with a corner on the pad: one open run, starting after the first piece lost.
        let into = line(&[[-5.0, -5.0], [5.0, -5.0], [5.0, 5.0], [-5.0, 5.0]], true);
        assert_eq!(places(&c.runs(into)), vec![(vec![[-5.0, 5.0], [-5.0, -5.0], [5.0, -5.0]], false)]);
        // A piece crossing the pad with both ends outside goes; one touching its corner goes too.
        assert!(c.runs(line(&[[-5.0, 5.0], [15.0, 5.0]], false)).is_empty());
        assert!(c.runs(line(&[[-5.0, -5.0], [0.0, 0.0]], false)).is_empty());
        assert!(c.runs(line(&[[-5.0, -5.0], [-1.0, 0.0]], false)).len() == 1);
        // Too short to have a piece: it goes when its corner is covered.
        assert!(c.runs(line(&[[5.0, 5.0]], false)).is_empty());
        assert_eq!(c.runs(line(&[[50.0, 5.0]], false)).len(), 1);
    }
}
