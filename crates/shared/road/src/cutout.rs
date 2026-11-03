//! A terrain with corridors cut out of it in plan (docs/adr/0910 §6; OpenRoads and Civil 3D show the existing ground
//! with the corridor's footprint taken out, so a cut's pavement is not under the ground it removes): the terrain's
//! triangles outside the corridors' top surfaces kept as they are, those under them dropped, those across their edges
//! clipped by the geometry core's exact overlay (the triangle minus the corridor triangles near it) and
//! triangulated again; a new vertex's height is the terrain triangle's own plane at it, so the surface left is the
//! terrain itself, to the bit at the corners it keeps.
//!
//! Only the corridor triangles whose boxes meet a terrain triangle's box take part in its clipping (a grid over them);
//! a terrain triangle inside one of them is dropped without an overlay. A corridor's triangles may overlap one another
//! or another corridor's: the overlay takes their union.

use kentos_geometry_core::Vec2;
use kentos_geometry_core::geom::arrangement::{Rule, Source};
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::geom::overlay::overlay;
use kentos_geometry_core::predicates::orientation;
use kentos_geometry_core::triangulate::triangulate_into;

/// The terrain left: its vertices (the terrain's, then the new ones), its triangles counterclockwise in plan, and how
/// many of the terrain's triangles were kept whole, dropped and clipped.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cut {
    pub vertices: Vec<[f64; 3]>,
    pub triangles: Vec<[u32; 3]>,
    pub kept: usize,
    pub dropped: usize,
    pub clipped: usize,
}

fn plan(p: [f64; 3]) -> Vec2 {
    Vec2::new(p[0], p[1])
}

/// A triangle counterclockwise in plan, none when it has no area.
fn ccw(t: [[f64; 3]; 3]) -> Option<[Vec2; 3]> {
    let [a, b, c] = t.map(plan);
    match orientation(a, b, c) {
        1 => Some([a, b, c]),
        -1 => Some([a, c, b]),
        _ => None,
    }
}

fn source(t: &[Vec2; 3]) -> Source {
    Source {
        edges: (0..3).map(|k| Edge::Seg { a: t[k], b: t[(k + 1) % 3] }).collect(),
        points: Some(t.to_vec()),
        cut: None,
    }
}

/// Whether `p` lies in the closed triangle `t` (counterclockwise), exactly.
fn inside(t: &[Vec2; 3], p: Vec2) -> bool {
    (0..3).all(|k| orientation(t[k], t[(k + 1) % 3], p) >= 0)
}

/// A grid of square cells over the footprint's triangles: a cell lists the triangles whose boxes meet it.
struct Grid {
    x0: f64,
    y0: f64,
    size: f64,
    nx: usize,
    ny: usize,
    cells: Vec<Vec<u32>>,
}

impl Grid {
    fn of(tris: &[[Vec2; 3]]) -> Grid {
        let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for t in tris {
            for p in t {
                x0 = x0.min(p.x);
                y0 = y0.min(p.y);
                x1 = x1.max(p.x);
                y1 = y1.max(p.y);
            }
        }
        let n = tris.len().max(1) as f64;
        let size = (((x1 - x0) * (y1 - y0)).max(1e-12) / n).sqrt().max(1e-6) * 2.0;
        let nx = (((x1 - x0) / size).ceil() as usize).clamp(1, 4096);
        let ny = (((y1 - y0) / size).ceil() as usize).clamp(1, 4096);
        let size = ((x1 - x0) / nx as f64).max((y1 - y0) / ny as f64).max(1e-6);
        let mut g = Grid { x0, y0, size, nx, ny, cells: vec![Vec::new(); nx * ny] };
        for (i, t) in tris.iter().enumerate() {
            let (a, b) = g.span(bounds(t));
            for y in a.1..=b.1 {
                for x in a.0..=b.0 {
                    g.cells[y * nx + x].push(i as u32);
                }
            }
        }
        g
    }

    fn cell(&self, x: f64, y: f64) -> (usize, usize) {
        let cx = ((x - self.x0) / self.size).floor().clamp(0.0, (self.nx - 1) as f64) as usize;
        let cy = ((y - self.y0) / self.size).floor().clamp(0.0, (self.ny - 1) as f64) as usize;
        (cx, cy)
    }

    fn span(&self, b: [f64; 4]) -> ((usize, usize), (usize, usize)) {
        (self.cell(b[0], b[1]), self.cell(b[2], b[3]))
    }
}

fn bounds(t: &[Vec2; 3]) -> [f64; 4] {
    [
        t[0].x.min(t[1].x).min(t[2].x),
        t[0].y.min(t[1].y).min(t[2].y),
        t[0].x.max(t[1].x).max(t[2].x),
        t[0].y.max(t[1].y).max(t[2].y),
    ]
}

fn meet(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] <= b[2] && b[0] <= a[2] && a[1] <= b[3] && b[1] <= a[3]
}

/// The height of the plane through `t` at `p`.
fn height(t: [[f64; 3]; 3], p: Vec2) -> f64 {
    let [a, b, c] = t;
    let (ux, uy, uz) = (b[0] - a[0], b[1] - a[1], b[2] - a[2]);
    let (vx, vy, vz) = (c[0] - a[0], c[1] - a[1], c[2] - a[2]);
    let d = ux * vy - uy * vx;
    let (dx, dy) = (p.x - a[0], p.y - a[1]);
    let s = (dx * vy - dy * vx) / d;
    let r = (ux * dy - uy * dx) / d;
    a[2] + s * uz + r * vz
}

/// A run of the terrain's triangles cut: its triangles (a corner the terrain's vertex or a new one of the run's) and
/// its new vertices.
#[derive(Default)]
struct Part {
    triangles: Vec<[Corner; 3]>,
    vertices: Vec<[f64; 3]>,
    kept: usize,
    dropped: usize,
    clipped: usize,
}

#[derive(Clone, Copy)]
enum Corner {
    Terrain(u32),
    New(u32),
}

struct Job<'a> {
    vertices: &'a [[f64; 3]],
    triangles: &'a [[u32; 3]],
    foot: &'a [[Vec2; 3]],
    boxes: &'a [[f64; 4]],
    grid: &'a Grid,
}

impl Job<'_> {
    fn run(&self, range: std::ops::Range<usize>) -> Part {
        let mut out = Part::default();
        let mut seen = vec![usize::MAX; self.foot.len()];
        for ti in range {
            let tri = self.triangles[ti];
            if tri.iter().any(|&k| k as usize >= self.vertices.len()) {
                continue;
            }
            let whole = tri.map(Corner::Terrain);
            let corners = tri.map(|k| self.vertices[k as usize]);
            let Some(t) = ccw(corners) else {
                out.triangles.push(whole);
                out.kept += 1;
                continue;
            };
            let b = bounds(&t);
            let (lo, hi) = self.grid.span(b);
            let mut near: Vec<u32> = Vec::new();
            for y in lo.1..=hi.1 {
                for x in lo.0..=hi.0 {
                    for &i in &self.grid.cells[y * self.grid.nx + x] {
                        if seen[i as usize] != ti && meet(b, self.boxes[i as usize]) {
                            seen[i as usize] = ti;
                            near.push(i);
                        }
                    }
                }
            }
            if near.is_empty() {
                out.triangles.push(whole);
                out.kept += 1;
                continue;
            }
            if near.iter().any(|&i| t.iter().all(|p| inside(&self.foot[i as usize], *p))) {
                out.dropped += 1;
                continue;
            }
            near.sort_unstable();
            let mut sources = vec![source(&t)];
            sources.extend(near.iter().map(|&i| source(&self.foot[i as usize])));
            let areas = overlay(&sources, Rule::FirstNotOthers);
            let unchanged = areas.len() == 1
                && areas[0].holes.is_empty()
                && areas[0].outer.pts.len() == 3
                && t.iter().all(|p| areas[0].outer.pts.iter().any(|q| q.x == p.x && q.y == p.y));
            if unchanged {
                out.triangles.push(whole);
                out.kept += 1;
                continue;
            }
            if areas.is_empty() {
                out.dropped += 1;
                continue;
            }
            out.clipped += 1;
            for a in &areas {
                let mut pts: Vec<Vec2> = Vec::new();
                let mut rings = Vec::new();
                for r in std::iter::once(&a.outer).chain(&a.holes) {
                    rings.push(pts.len()..pts.len() + r.pts.len());
                    pts.extend_from_slice(&r.pts);
                }
                let mut idx = Vec::new();
                triangulate_into(&pts, &rings, &mut idx);
                let ids: Vec<Corner> = pts
                    .iter()
                    .map(|p| match (0..3).find(|&k| corners[k][0] == p.x && corners[k][1] == p.y) {
                        Some(k) => Corner::Terrain(tri[k]),
                        None => {
                            out.vertices.push([p.x, p.y, height(corners, *p)]);
                            Corner::New((out.vertices.len() - 1) as u32)
                        }
                    })
                    .collect();
                for c in idx.chunks_exact(3) {
                    out.triangles.push([ids[c[0] as usize], ids[c[1] as usize], ids[c[2] as usize]]);
                }
            }
        }
        out
    }

    /// The runs, on threads of their own for a large terrain (the browser has none: one run there); in order.
    #[cfg(not(target_arch = "wasm32"))]
    fn parts(&self) -> Vec<Part> {
        let n = self.triangles.len();
        let threads = std::thread::available_parallelism().map_or(1, |k| k.get()).min(16);
        if n < 50_000 || threads < 2 {
            return vec![self.run(0..n)];
        }
        let step = n.div_ceil(threads);
        std::thread::scope(|s| {
            let handles: Vec<_> = (0..n).step_by(step).map(|a| s.spawn(move || self.run(a..(a + step).min(n)))).collect();
            handles.into_iter().map(|h| h.join().unwrap_or_else(|e| std::panic::resume_unwind(e))).collect()
        })
    }

    #[cfg(target_arch = "wasm32")]
    fn parts(&self) -> Vec<Part> {
        vec![self.run(0..self.triangles.len())]
    }
}

/// The terrain (`vertices`, `triangles`) less the corridors' triangles `footprint` in plan. The triangles keep the
/// terrain's order; a clipped one's pieces stand in its place.
pub fn cut_out(vertices: &[[f64; 3]], triangles: &[[u32; 3]], footprint: &[[[f64; 3]; 3]]) -> Cut {
    let mut out = Cut { vertices: vertices.to_vec(), ..Cut::default() };
    let foot: Vec<[Vec2; 3]> = footprint.iter().filter_map(|t| ccw(*t)).collect();
    if foot.is_empty() {
        out.triangles = triangles.iter().filter(|t| t.iter().all(|&k| (k as usize) < vertices.len())).copied().collect();
        out.kept = out.triangles.len();
        return out;
    }
    let grid = Grid::of(&foot);
    let boxes: Vec<[f64; 4]> = foot.iter().map(bounds).collect();
    let job = Job { vertices, triangles, foot: &foot, boxes: &boxes, grid: &grid };
    for part in job.parts() {
        let base = out.vertices.len() as u32;
        out.vertices.extend(part.vertices);
        out.triangles.extend(part.triangles.iter().map(|t| {
            t.map(|c| match c {
                Corner::Terrain(k) => k,
                Corner::New(k) => base + k,
            })
        }));
        out.kept += part.kept;
        out.dropped += part.dropped;
        out.clipped += part.clipped;
    }
    out
}
