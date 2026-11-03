//! A surface of points made tile by tile (plan/road.md Y18.8, docs/adr/0916 §7; ROAD-TIN-12, 13), for surveys too
//! large to triangulate at once: the points' box cut into square tiles, each tile triangulated with the points of a
//! halo round it, and the triangles merged.
//!
//! A fixed halo (LAStools' buffer) does not make the tiles' triangles the whole surface's: a point just outside the
//! halo can still lie in a triangle's circumcircle. So a triangle is kept only when it is certified, as Isenburg et
//! al.'s streaming Delaunay finalises one when its circumcircle reaches no unseen space: every point of the halo was
//! seen, and the points outside it in the cells the circle reaches are asked by the exact in-circle predicate; with
//! none inside, it is one of the whole surface's Delaunay triangles. Every triangle over a tile must be certified, so the tile is covered by the whole surface's triangles; the
//! tile owns those whose centroid is in it (half-open, so each is owned once). Near the outer hull the halo's points may
//! not reach it: the triangles over the tile must cover as much of it as the whole hull does. While one is not
//! certified or the cover falls short, the tile's halo doubles (adaptive growth), up to the whole box.
//!
//! The merge is then checked against the whole surface without making it: a triangulation of n corners with h on its
//! hull has 2n − 2 − h triangles, and their area is the hull's. Points four on a circle make the Delaunay triangulation
//! not one (two tiles may cut such a square by different diagonals): where the count or the area does not hold, every
//! halo doubles again and the merge is made anew; when every tile already saw every point, the surface is made whole.
//! Breaklines, boundaries and voids are not tiled here.

use std::collections::HashMap;

use kentos_geometry_core::Vec2;
use kentos_geometry_core::predicates::incircle;

use super::{SpotPoint, SurfaceError, SurfaceInput, build};

/// Why a surface is not made by tiles.
#[derive(Clone, Debug, PartialEq)]
pub enum TiledError {
    /// The tile or the halo not a number above zero.
    Tile,
    /// A tile's surface does not build (the whole one would not either).
    Build(SurfaceError),
}

/// A surface made by tiles: its triangles (x, y, z each corner, counter-clockwise), the tiles, the surfaces made (the
/// halos grown counted), the triangles that cross a tile's edge (the seams), the largest halo used and how many times
/// the merge was made again.
#[derive(Clone, Debug, PartialEq)]
pub struct Tiled {
    pub triangles: Vec<[[f64; 3]; 3]>,
    pub tiles: usize,
    pub builds: usize,
    pub seams: usize,
    pub largest_halo: f64,
    pub merges: usize,
}

fn cross(o: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
}

/// The hull's ring (counter-clockwise), its area and how many of the corners are on it (its vertices and those on its
/// edges).
fn hull(points: &[[f64; 2]]) -> (Vec<[f64; 2]>, f64, usize) {
    let mut p = points.to_vec();
    p.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    p.dedup();
    if p.len() < 3 {
        return (Vec::new(), 0.0, p.len());
    }
    let mut lower: Vec<[f64; 2]> = Vec::new();
    for q in &p {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], *q) <= 0.0 {
            lower.pop();
        }
        lower.push(*q);
    }
    let mut upper: Vec<[f64; 2]> = Vec::new();
    for q in p.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], *q) <= 0.0 {
            upper.pop();
        }
        upper.push(*q);
    }
    lower.pop();
    upper.pop();
    let ring: Vec<[f64; 2]> = lower.into_iter().chain(upper).collect();
    let n = ring.len();
    let area = (0..n).map(|i| ring[i][0] * ring[(i + 1) % n][1] - ring[(i + 1) % n][0] * ring[i][1]).sum::<f64>() / 2.0;
    let on = p
        .iter()
        .filter(|q| {
            (0..n).any(|i| {
                let (a, b) = (ring[i], ring[(i + 1) % n]);
                cross(a, b, **q) == 0.0
                    && q[0] >= a[0].min(b[0])
                    && q[0] <= a[0].max(b[0])
                    && q[1] >= a[1].min(b[1])
                    && q[1] <= a[1].max(b[1])
            })
        })
        .count();
    (ring, area, on)
}

/// A convex polygon's area within a box (Sutherland–Hodgman against its four sides).
fn area_in(poly: &[[f64; 2]], b: &Box2) -> f64 {
    let mut p = poly.to_vec();
    let sides: [(usize, f64, bool); 4] = [(0, b.x0, true), (0, b.x1, false), (1, b.y0, true), (1, b.y1, false)];
    for (axis, at, above) in sides {
        let inside = |q: &[f64; 2]| if above { q[axis] >= at } else { q[axis] <= at };
        let mut out = Vec::with_capacity(p.len() + 2);
        for i in 0..p.len() {
            let (a, c) = (p[i], p[(i + 1) % p.len()]);
            if inside(&a) {
                out.push(a);
            }
            if inside(&a) != inside(&c) {
                let t = (at - a[axis]) / (c[axis] - a[axis]);
                out.push([a[0] + t * (c[0] - a[0]), a[1] + t * (c[1] - a[1])]);
            }
        }
        p = out;
        if p.len() < 3 {
            return 0.0;
        }
    }
    let n = p.len();
    (0..n).map(|i| p[i][0] * p[(i + 1) % n][1] - p[(i + 1) % n][0] * p[i][1]).sum::<f64>().abs() / 2.0
}

/// A triangle's circumcircle: centre and radius; none when it has no area.
fn circle(t: &[[f64; 3]; 3]) -> Option<([f64; 2], f64)> {
    let (a, b, c) = ([t[0][0], t[0][1]], [t[1][0], t[1][1]], [t[2][0], t[2][1]]);
    let d = 2.0 * cross(a, b, c);
    if d == 0.0 {
        return None;
    }
    let (b0, c0) = ([b[0] - a[0], b[1] - a[1]], [c[0] - a[0], c[1] - a[1]]);
    let (bb, cc) = (b0[0] * b0[0] + b0[1] * b0[1], c0[0] * c0[0] + c0[1] * c0[1]);
    let ux = (c0[1] * bb - b0[1] * cc) / d;
    let uy = (b0[0] * cc - c0[0] * bb) / d;
    Some(([a[0] + ux, a[1] + uy], ux.hypot(uy)))
}

#[derive(Clone, Copy)]
struct Box2 {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

impl Box2 {
    fn grown(&self, h: f64) -> Self {
        Self { x0: self.x0 - h, y0: self.y0 - h, x1: self.x1 + h, y1: self.y1 + h }
    }

    fn holds(&self, p: [f64; 2]) -> bool {
        p[0] >= self.x0 && p[0] <= self.x1 && p[1] >= self.y0 && p[1] <= self.y1
    }

    fn covers(&self, o: &Box2) -> bool {
        self.x0 <= o.x0 && self.y0 <= o.y0 && self.x1 >= o.x1 && self.y1 >= o.y1
    }
}

/// What one tile gives: its owned triangles, its seams, the surfaces made and the halo it ended with.
struct TileOut {
    triangles: Vec<[[f64; 3]; 3]>,
    seams: usize,
    builds: usize,
    halo: f64,
}

/// What every tile reads.
struct Grid<'a> {
    points: &'a [SpotPoint],
    tolerance: Option<f64>,
    all: Box2,
    tile: f64,
    nx: usize,
    ny: usize,
    by: HashMap<(usize, usize), Vec<usize>>,
    ring: Vec<[f64; 2]>,
}

impl Grid<'_> {
    fn cell(&self, x: f64, y: f64) -> (usize, usize) {
        let i = (((x - self.all.x0) / self.tile).floor().max(0.0) as usize).min(self.nx - 1);
        let j = (((y - self.all.y0) / self.tile).floor().max(0.0) as usize).min(self.ny - 1);
        (i, j)
    }

    fn own(&self, i: usize, j: usize) -> Box2 {
        Box2 {
            x0: self.all.x0 + i as f64 * self.tile,
            y0: self.all.y0 + j as f64 * self.tile,
            x1: self.all.x0 + (i + 1) as f64 * self.tile,
            y1: self.all.y0 + (j + 1) as f64 * self.tile,
        }
    }

    /// Whether no point outside `region` is strictly inside the triangle's circumcircle: the cells the circle reaches
    /// and the region does not cover are asked, exactly.
    fn certified(&self, tri: &[[f64; 3]; 3], region: &Box2) -> bool {
        let Some((o, r)) = circle(tri) else { return false };
        let (lo, hi) = (self.cell(o[0] - r, o[1] - r), self.cell(o[0] + r, o[1] + r));
        let v = |p: [f64; 3]| Vec2::new(p[0], p[1]);
        for cj in lo.1..=hi.1 {
            for ci in lo.0..=hi.0 {
                let b = self.own(ci, cj);
                // A cell the circle does not reach, or one the region holds whole, has no unseen point in it.
                let (dx, dy) = ((b.x0 - o[0]).max(0.0).max(o[0] - b.x1), (b.y0 - o[1]).max(0.0).max(o[1] - b.y1));
                if dx * dx + dy * dy > r * r || region.covers(&b) {
                    continue;
                }
                for k in self.by.get(&(ci, cj)).into_iter().flatten() {
                    let p = &self.points[*k];
                    if !region.holds([p.x, p.y]) && incircle(v(tri[0]), v(tri[1]), v(tri[2]), Vec2::new(p.x, p.y)) > 0.0 {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// Tile (i, j) from a halo of `h`, grown until its triangles are certified and cover it.
    fn tile(&self, i: usize, j: usize, mut h: f64) -> Result<TileOut, TiledError> {
        let own = self.own(i, j);
        let (nx, ny) = (self.nx, self.ny);
        // Half-open: the last tile of a row or column holds its upper edge.
        let owns = |c: [f64; 2]| (c[0] >= own.x0 && (c[0] < own.x1 || i + 1 == nx)) && (c[1] >= own.y0 && (c[1] < own.y1 || j + 1 == ny));
        let mut builds = 0;
        loop {
            let region = own.grown(h);
            let whole = region.covers(&self.all);
            let reach = (h / self.tile).ceil() as usize + 1;
            let mut input = SurfaceInput { tolerance: self.tolerance, ..SurfaceInput::default() };
            for tj in j.saturating_sub(reach)..(j + reach + 1).min(ny) {
                for ti in i.saturating_sub(reach)..(i + reach + 1).min(nx) {
                    for k in self.by.get(&(ti, tj)).into_iter().flatten() {
                        let p = &self.points[*k];
                        if region.holds([p.x, p.y]) {
                            input.points.push(p.clone());
                        }
                    }
                }
            }
            builds += 1;
            let s = match build(&input) {
                Ok(s) => s,
                Err(e) if e.code == "too_few_points" || input.points.len() < 3 => {
                    if whole {
                        return Ok(TileOut { triangles: Vec::new(), seams: 0, builds, halo: h });
                    }
                    h *= 2.0;
                    continue;
                }
                Err(e) => return Err(TiledError::Build(e)),
            };
            let mut mine = Vec::new();
            let mut sure = true;
            for t in &s.triangles {
                let tri = t.map(|v| s.vertices[v as usize]);
                // Every triangle over the tile is certified, so the tile is covered by the whole surface's triangles
                // and those owned by their centroid are all there; only the owned are kept.
                let (bx0, bx1) = (tri[0][0].min(tri[1][0]).min(tri[2][0]), tri[0][0].max(tri[1][0]).max(tri[2][0]));
                let (by0, by1) = (tri[0][1].min(tri[1][1]).min(tri[2][1]), tri[0][1].max(tri[1][1]).max(tri[2][1]));
                if bx1 < own.x0 || bx0 > own.x1 || by1 < own.y0 || by0 > own.y1 {
                    continue;
                }
                if !whole && !self.certified(&tri, &region) {
                    sure = false;
                    break;
                }
                let c = [(tri[0][0] + tri[1][0] + tri[2][0]) / 3.0, (tri[0][1] + tri[1][1] + tri[2][1]) / 3.0];
                if owns(c) {
                    mine.push(tri);
                }
            }
            // The local hull may stop short of the whole one near it: the triangles over the tile must cover as much
            // of it as the whole hull does.
            if sure && !whole {
                let want = area_in(&self.ring, &own);
                let got: f64 = s
                    .triangles
                    .iter()
                    .map(|t| area_in(&t.map(|v| [s.vertices[v as usize][0], s.vertices[v as usize][1]]), &own))
                    .sum();
                sure = got >= want - 1e-9 * want.max(1.0);
            }
            if !sure {
                h *= 2.0;
                continue;
            }
            let seams = mine.iter().filter(|t| t.iter().any(|v| !own.holds([v[0], v[1]]))).count();
            return Ok(TileOut { triangles: mine, seams, builds, halo: h });
        }
    }
}

/// The surface of `points` made in square tiles of `tile` metres with a first halo of `halo` metres, the tiles made on
/// as many threads as the machine gives.
pub fn build_tiled(points: &[SpotPoint], tolerance: Option<f64>, tile: f64, halo: f64) -> Result<Tiled, TiledError> {
    if !(tile.is_finite() && tile > 0.0 && halo.is_finite() && halo > 0.0) {
        return Err(TiledError::Tile);
    }
    let mut all = Box2 { x0: f64::INFINITY, y0: f64::INFINITY, x1: f64::NEG_INFINITY, y1: f64::NEG_INFINITY };
    for p in points {
        all = Box2 { x0: all.x0.min(p.x), y0: all.y0.min(p.y), x1: all.x1.max(p.x), y1: all.y1.max(p.y) };
    }
    let plan: Vec<[f64; 2]> = points.iter().map(|p| [p.x, p.y]).collect();
    let (ring, hull_area, on_hull) = hull(&plan);
    let mut distinct = plan.clone();
    distinct.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    distinct.dedup();
    let want = (2 * distinct.len()).saturating_sub(2 + on_hull);
    let nx = (((all.x1 - all.x0) / tile).ceil() as usize).max(1);
    let ny = (((all.y1 - all.y0) / tile).ceil() as usize).max(1);
    let mut grid = Grid { points, tolerance, all, tile, nx, ny, by: HashMap::new(), ring };
    for (k, p) in points.iter().enumerate() {
        let c = grid.cell(p.x, p.y);
        grid.by.entry(c).or_default().push(k);
    }
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get()).min(nx * ny).max(1);
    let mut halos = vec![halo; nx * ny];
    let mut out = Tiled { triangles: Vec::new(), tiles: nx * ny, builds: 0, seams: 0, largest_halo: halo, merges: 0 };
    loop {
        let grid = &grid;
        let halos_now = &halos;
        let made: Vec<Result<TileOut, TiledError>> = std::thread::scope(|scope| {
            let workers: Vec<_> = (0..threads)
                .map(|w| {
                    scope.spawn(move || {
                        (w..nx * ny).step_by(threads).map(|k| (k, grid.tile(k % nx, k / nx, halos_now[k]))).collect::<Vec<_>>()
                    })
                })
                .collect();
            let mut all: Vec<(usize, Result<TileOut, TiledError>)> =
                workers.into_iter().flat_map(|h| h.join().unwrap_or_default()).collect();
            all.sort_by_key(|(k, _)| *k);
            all.into_iter().map(|(_, r)| r).collect()
        });
        let mut triangles = Vec::new();
        let mut seams = 0;
        for (k, r) in made.into_iter().enumerate() {
            let t = r?;
            out.builds += t.builds;
            out.largest_halo = out.largest_halo.max(t.halo);
            halos[k] = t.halo;
            seams += t.seams;
            triangles.extend(t.triangles);
        }
        let area: f64 = triangles.iter().map(|t| cross([t[0][0], t[0][1]], [t[1][0], t[1][1]], [t[2][0], t[2][1]]) / 2.0).sum();
        let held = triangles.len() == want && (area - hull_area).abs() <= 1e-9 * hull_area.abs().max(1.0);
        let all_whole = (0..nx * ny).all(|k| grid.own(k % nx, k / nx).grown(halos[k]).covers(&grid.all));
        out.merges += 1;
        if held {
            out.triangles = triangles;
            out.seams = seams;
            return Ok(out);
        }
        if all_whole {
            // Every tile saw every point and the pieces still do not agree (points on one circle, cut differently by
            // order): the surface is made whole, once.
            let input = SurfaceInput { points: points.to_vec(), tolerance, ..SurfaceInput::default() };
            let s = build(&input).map_err(TiledError::Build)?;
            out.builds += 1;
            out.triangles = s.triangles.iter().map(|t| t.map(|v| s.vertices[v as usize])).collect();
            out.seams = 0;
            return Ok(out);
        }
        for h in &mut halos {
            *h *= 2.0;
        }
    }
}
