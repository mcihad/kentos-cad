//! A road's earthwork volumes (docs/adr/0906, plan/road.md Y8; ROAD-VOL-01…08, 16): by sections, the average end
//! area between consecutive sections (or the prismoidal formula with a middle section), and by surfaces, the integral
//! of `d = z_design − z_ground` over the plan where both surfaces are, cut and fill apart.
//!
//! The surfaces' method lays the two triangulations over each other: each design triangle is clipped by each ground
//! triangle it meets (both convex, both planes there), the piece split where `d` changes sign, and each part's volume
//! `A·(d₀ + d₁ + d₂)/3` (exact for a linear `d`) goes to the fill when `d ≥ 0`, the cut otherwise. The design's area
//! the ground does not cover is reported, not counted as nothing (ROAD-VOL-08). Rounding is left to the report.

/// A section's place and areas: its station, its cut and fill areas (m²); none where it is not solved.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SectionAreas {
    pub station: f64,
    pub areas: Option<(f64, f64)>,
}

/// The volumes between two consecutive sections: their stations, the length, the cut and fill (m³); none where a
/// section is not solved (the stretch is left out, not counted as nothing).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Between {
    pub from: f64,
    pub to: f64,
    pub volumes: Option<(f64, f64)>,
}

/// The average end area volumes between consecutive sections: `L·(A₀ + A₁)/2` for the cut and the fill apart.
pub fn end_areas(sections: &[SectionAreas]) -> Vec<Between> {
    sections
        .windows(2)
        .map(|w| {
            let l = w[1].station - w[0].station;
            let volumes = match (w[0].areas, w[1].areas) {
                (Some((c0, f0)), Some((c1, f1))) => Some((l * (c0 + c1) / 2.0, l * (f0 + f1) / 2.0)),
                _ => None,
            };
            Between { from: w[0].station, to: w[1].station, volumes }
        })
        .collect()
}

/// A section on a curve (docs/adr/0934 §1): its station, cut and fill areas (m²), their first moments about the
/// alignment (`∫ o dA`, offset right positive, m³/m), the alignment's curvature there (left turns positive, 1/m) and
/// the farthest offsets the section reaches on each side (left negative, right positive).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CurvedSection {
    pub station: f64,
    pub areas: Option<(f64, f64)>,
    pub moments: (f64, f64),
    pub curvature: f64,
    pub reach: (f64, f64),
}

/// The section's areas as they sweep along a curve: an element at offset o travels `ds·(1 + κ·o)` while the alignment
/// travels ds (κ left positive, o right positive: on a left turn the left side is inside and travels less). So the
/// volume is `∫ (A + κ·Q) ds` (Pappus's centroid theorem when the section is constant on a circle); `A + κ·Q` is the
/// effective area. None where the section reaches past the curve's centre (`1 + κ·o ≤ 0`: the lines cross, Y29).
pub fn effective(s: &CurvedSection) -> Option<Option<(f64, f64)>> {
    let k = s.curvature;
    let inside = if k > 0.0 { s.reach.0 } else { s.reach.1 };
    if k != 0.0 && 1.0 + k * inside <= 0.0 {
        return None;
    }
    Some(s.areas.map(|(c, f)| (c + k * s.moments.0, f + k * s.moments.1)))
}

/// The average end area volumes of the effective areas: `L·(A′₀ + A′₁)/2`, cut and fill apart; exact for a constant
/// section on a circle, second order in L on a clothoid. A stretch with a section past its centre is left out (none).
pub fn end_areas_curved(sections: &[CurvedSection]) -> Vec<Between> {
    sections
        .windows(2)
        .map(|w| {
            let l = w[1].station - w[0].station;
            let volumes = match (effective(&w[0]), effective(&w[1])) {
                (Some(Some((c0, f0))), Some(Some((c1, f1)))) => Some((l * (c0 + c1) / 2.0, l * (f0 + f1) / 2.0)),
                _ => None,
            };
            Between { from: w[0].station, to: w[1].station, volumes }
        })
        .collect()
}

/// The prismoidal volume between end areas `a0`, `a1` with the middle section's area `am`, over the length `l`:
/// `L·(A₀ + 4Aₘ + A₁)/6`.
pub fn prismoidal(a0: f64, am: f64, a1: f64, l: f64) -> f64 {
    l * (a0 + 4.0 * am + a1) / 6.0
}

/// The volumes between two surfaces over the design's plan: cut and fill (m³), the plan areas where the design is
/// below and above the ground, the area both cover, and the design's area the ground does not cover (m²).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Volumes {
    pub cut: f64,
    pub fill: f64,
    pub cut_area: f64,
    pub fill_area: f64,
    pub covered: f64,
    pub uncovered: f64,
}

/// A triangle's corners (x, y, z).
pub type Triangle = [[f64; 3]; 3];

fn area2(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])
}

/// A triangle counterclockwise in plan; none when it has no area.
fn ccw(t: &Triangle) -> Option<Triangle> {
    let a = area2([t[0][0], t[0][1]], [t[1][0], t[1][1]], [t[2][0], t[2][1]]);
    if a > 0.0 {
        Some(*t)
    } else if a < 0.0 {
        Some([t[0], t[2], t[1]])
    } else {
        None
    }
}

/// The height of a triangle's plane at a place.
fn plane_z(t: &Triangle, p: [f64; 2]) -> f64 {
    let (a, b, c) = ([t[0][0], t[0][1]], [t[1][0], t[1][1]], [t[2][0], t[2][1]]);
    let whole = area2(a, b, c);
    let wa = area2(p, b, c) / whole;
    let wb = area2(a, p, c) / whole;
    let wc = 1.0 - wa - wb;
    wa * t[0][2] + wb * t[1][2] + wc * t[2][2]
}

/// A convex polygon (counterclockwise) clipped by the half plane left of `a → b`.
fn clip(poly: &[[f64; 2]], a: [f64; 2], b: [f64; 2]) -> Vec<[f64; 2]> {
    let side = |p: [f64; 2]| area2(a, b, p);
    let mut out = Vec::with_capacity(poly.len() + 1);
    for i in 0..poly.len() {
        let (p, q) = (poly[i], poly[(i + 1) % poly.len()]);
        let (sp, sq) = (side(p), side(q));
        if sp >= 0.0 {
            out.push(p);
        }
        if (sp > 0.0 && sq < 0.0) || (sp < 0.0 && sq > 0.0) {
            let t = sp / (sp - sq);
            out.push([p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t]);
        }
    }
    out
}

/// A convex polygon's part where the linear `d` (given at its corners) is ≥ 0 (`above`) or ≤ 0, with `d` there.
fn split(poly: &[([f64; 2], f64)], above: bool) -> Vec<([f64; 2], f64)> {
    let keep = |d: f64| if above { d >= 0.0 } else { d <= 0.0 };
    let mut out = Vec::with_capacity(poly.len() + 1);
    for i in 0..poly.len() {
        let (p, q) = (poly[i], poly[(i + 1) % poly.len()]);
        if keep(p.1) {
            out.push(p);
        }
        if (p.1 > 0.0 && q.1 < 0.0) || (p.1 < 0.0 && q.1 > 0.0) {
            let t = p.1 / (p.1 - q.1);
            out.push(([p.0[0] + (q.0[0] - p.0[0]) * t, p.0[1] + (q.0[1] - p.0[1]) * t], 0.0));
        }
    }
    out
}

/// A convex polygon's plan area and the integral of the linear `d` over it (a fan from its first corner).
fn integral(poly: &[([f64; 2], f64)]) -> (f64, f64) {
    let (mut area, mut volume) = (0.0, 0.0);
    for i in 1..poly.len().saturating_sub(1) {
        let (a, b, c) = (poly[0], poly[i], poly[i + 1]);
        let s = area2(a.0, b.0, c.0) / 2.0;
        area += s;
        volume += s * (a.1 + b.1 + c.1) / 3.0;
    }
    (area, volume)
}

/// A uniform grid over the ground triangles' boxes, so a design triangle meets only the ground triangles near it.
struct Grid {
    x0: f64,
    y0: f64,
    size: f64,
    nx: usize,
    ny: usize,
    cells: Vec<Vec<u32>>,
}

impl Grid {
    fn new(tris: &[Triangle]) -> Self {
        let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for t in tris {
            for p in t {
                x0 = x0.min(p[0]);
                y0 = y0.min(p[1]);
                x1 = x1.max(p[0]);
                y1 = y1.max(p[1]);
            }
        }
        let n = (tris.len() as f64).sqrt().ceil().clamp(1.0, 2048.0) as usize;
        let size = ((x1 - x0).max(y1 - y0) / n as f64).max(1e-9);
        let nx = (((x1 - x0) / size).ceil() as usize).max(1);
        let ny = (((y1 - y0) / size).ceil() as usize).max(1);
        let mut grid = Self { x0, y0, size, nx, ny, cells: vec![Vec::new(); nx * ny] };
        for (i, t) in tris.iter().enumerate() {
            let (a, b) = grid.span(t);
            for cy in a[1]..=b[1] {
                for cx in a[0]..=b[0] {
                    grid.cells[cy * nx + cx].push(i as u32);
                }
            }
        }
        grid
    }

    fn cell(&self, x: f64, y: f64) -> [usize; 2] {
        let cx = ((x - self.x0) / self.size).floor().clamp(0.0, (self.nx - 1) as f64) as usize;
        let cy = ((y - self.y0) / self.size).floor().clamp(0.0, (self.ny - 1) as f64) as usize;
        [cx, cy]
    }

    fn span(&self, t: &Triangle) -> ([usize; 2], [usize; 2]) {
        let xs = [t[0][0], t[1][0], t[2][0]];
        let ys = [t[0][1], t[1][1], t[2][1]];
        let lo = self.cell(xs.iter().copied().fold(f64::INFINITY, f64::min), ys.iter().copied().fold(f64::INFINITY, f64::min));
        let hi = self.cell(xs.iter().copied().fold(f64::NEG_INFINITY, f64::max), ys.iter().copied().fold(f64::NEG_INFINITY, f64::max));
        (lo, hi)
    }
}

/// The ground surface laid out once: its triangles counterclockwise and a grid over them, so many designs (a corridor's
/// strips, one by one) are measured against it without building the grid again.
pub struct Ground {
    tris: Vec<Triangle>,
    grid: Option<Grid>,
    seen: Vec<u32>,
    stamp: u32,
}

impl Ground {
    /// The ground of triangles that do not overlap in plan (x, y, z each corner).
    pub fn new(ground: &[Triangle]) -> Self {
        let tris: Vec<Triangle> = ground.iter().filter_map(ccw).collect();
        let grid = (!tris.is_empty()).then(|| Grid::new(&tris));
        let seen = vec![0; tris.len()];
        Self { tris, grid, seen, stamp: 0 }
    }

    /// The volumes between a design (triangles that do not overlap in plan) and this ground, over the design's plan.
    pub fn volumes(&mut self, design: &[Triangle]) -> Volumes {
        let mut out = Volumes::default();
        for d in design {
            let Some(d) = ccw(d) else { continue };
            let plan = [[d[0][0], d[0][1]], [d[1][0], d[1][1]], [d[2][0], d[2][1]]];
            let whole = area2(plan[0], plan[1], plan[2]) / 2.0;
            let mut covered = 0.0;
            if let Some(grid) = &self.grid {
                if self.stamp == u32::MAX {
                    self.seen.iter_mut().for_each(|v| *v = 0);
                    self.stamp = 0;
                }
                self.stamp += 1;
                let (lo, hi) = grid.span(&d);
                for cy in lo[1]..=hi[1] {
                    for cx in lo[0]..=hi[0] {
                        for &gi in &grid.cells[cy * grid.nx + cx] {
                            if self.seen[gi as usize] == self.stamp {
                                continue;
                            }
                            self.seen[gi as usize] = self.stamp;
                            let g = &self.tris[gi as usize];
                            let mut poly = plan.to_vec();
                            for k in 0..3 {
                                poly = clip(&poly, [g[k][0], g[k][1]], [g[(k + 1) % 3][0], g[(k + 1) % 3][1]]);
                                if poly.len() < 3 {
                                    break;
                                }
                            }
                            if poly.len() < 3 {
                                continue;
                            }
                            let with: Vec<([f64; 2], f64)> = poly.iter().map(|p| (*p, plane_z(&d, *p) - plane_z(g, *p))).collect();
                            let (a, _) = integral(&with);
                            if a <= 0.0 {
                                continue;
                            }
                            covered += a;
                            let (fa, fv) = integral(&split(&with, true));
                            let (ca, cv) = integral(&split(&with, false));
                            out.fill += fv;
                            out.fill_area += fa;
                            out.cut -= cv;
                            out.cut_area += ca;
                        }
                    }
                }
            }
            out.covered += covered;
            out.uncovered += (whole - covered).max(0.0);
        }
        out
    }
}

impl Ground {
    /// Each piece where a design triangle and a ground triangle overlap in plan, with `d = z_design − z_ground` at its
    /// corners (counterclockwise, convex); and the area both cover and the design's area the ground does not.
    pub fn overlay(&mut self, design: &[Triangle], mut visit: impl FnMut(&[([f64; 2], f64)])) -> (f64, f64) {
        self.overlay_on(design, |_, with| visit(with))
    }

    /// [`Ground::overlay`] with each piece's ground triangle (its index among the triangles given, those with area).
    fn overlay_on(&mut self, design: &[Triangle], mut visit: impl FnMut(usize, &[([f64; 2], f64)])) -> (f64, f64) {
        let (mut covered_all, mut uncovered) = (0.0, 0.0);
        for d in design {
            let Some(d) = ccw(d) else { continue };
            let plan = [[d[0][0], d[0][1]], [d[1][0], d[1][1]], [d[2][0], d[2][1]]];
            let whole = area2(plan[0], plan[1], plan[2]) / 2.0;
            let mut covered = 0.0;
            if let Some(grid) = &self.grid {
                if self.stamp == u32::MAX {
                    self.seen.iter_mut().for_each(|v| *v = 0);
                    self.stamp = 0;
                }
                self.stamp += 1;
                let (lo, hi) = grid.span(&d);
                for cy in lo[1]..=hi[1] {
                    for cx in lo[0]..=hi[0] {
                        for &gi in &grid.cells[cy * grid.nx + cx] {
                            if self.seen[gi as usize] == self.stamp {
                                continue;
                            }
                            self.seen[gi as usize] = self.stamp;
                            let g = &self.tris[gi as usize];
                            let mut poly = plan.to_vec();
                            for k in 0..3 {
                                poly = clip(&poly, [g[k][0], g[k][1]], [g[(k + 1) % 3][0], g[(k + 1) % 3][1]]);
                                if poly.len() < 3 {
                                    break;
                                }
                            }
                            if poly.len() < 3 {
                                continue;
                            }
                            let with: Vec<([f64; 2], f64)> = poly.iter().map(|p| (*p, plane_z(&d, *p) - plane_z(g, *p))).collect();
                            let (a, _) = integral(&with);
                            if a <= 0.0 {
                                continue;
                            }
                            covered += a;
                            visit(gi as usize, &with);
                        }
                    }
                }
            }
            covered_all += covered;
            uncovered += (whole - covered).max(0.0);
        }
        (covered_all, uncovered)
    }
}

/// The difference of two surfaces (docs/adr/0916 §3; Civil 3D's TIN volume surface): the volumes, the least and the
/// largest `d = z_comparison − z_base` and where, and the plan area of `d` in ranges `[bᵢ, bᵢ₊₁)` (the last closed).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Difference {
    pub volumes: Volumes,
    pub least: Option<(f64, [f64; 2])>,
    pub largest: Option<(f64, [f64; 2])>,
    pub bands: Vec<(f64, f64, f64)>,
    pub outside: f64,
}

/// The part of a convex polygon with the linear `d` on the `above` side of `level`.
fn cut_at(poly: &[([f64; 2], f64)], level: f64, above: bool) -> Vec<([f64; 2], f64)> {
    let shifted: Vec<([f64; 2], f64)> = poly.iter().map(|(p, d)| (*p, d - level)).collect();
    split(&shifted, above).into_iter().map(|(p, d)| (p, d + level)).collect()
}

/// The difference of `comparison` over `base`, over the comparison's plan; `breaks` (increasing, at least two) bands
/// the difference.
pub fn difference(comparison: &[Triangle], base: &[Triangle], breaks: &[f64]) -> Difference {
    let mut out = Difference {
        bands: breaks.windows(2).map(|w| (w[0], w[1], 0.0)).collect(),
        ..Difference::default()
    };
    let mut ground = Ground::new(base);
    let (covered, uncovered) = ground.overlay(comparison, |with| {
        let (fa, fv) = integral(&split(with, true));
        let (ca, cv) = integral(&split(with, false));
        out.volumes.fill += fv;
        out.volumes.fill_area += fa;
        out.volumes.cut -= cv;
        out.volumes.cut_area += ca;
        for (p, d) in with {
            if out.least.is_none_or(|(v, at)| *d < v || (*d == v && (p[0], p[1]) < (at[0], at[1]))) {
                out.least = Some((*d, *p));
            }
            if out.largest.is_none_or(|(v, at)| *d > v || (*d == v && (p[0], p[1]) < (at[0], at[1]))) {
                out.largest = Some((*d, *p));
            }
        }
        let mut inside = 0.0;
        for band in out.bands.iter_mut() {
            let low = cut_at(with, band.0, true);
            if low.len() < 3 {
                continue;
            }
            let part = cut_at(&low, band.1, false);
            if part.len() < 3 {
                continue;
            }
            let (a, _) = integral(&part);
            if a > 0.0 {
                band.2 += a;
                inside += a;
            }
        }
        let (whole, _) = integral(with);
        out.outside += (whole - inside).max(0.0);
    });
    out.volumes.covered = covered;
    out.volumes.uncovered = uncovered;
    out
}

/// The volumes between the design surface and the ground surface, each a set of triangles that do not overlap in plan
/// (x, y, z each corner), over the design's plan.
pub fn between_surfaces(design: &[Triangle], ground: &[Triangle]) -> Volumes {
    Ground::new(ground).volumes(design)
}

/// How each ground corner moves the volumes (docs/adr/0941 §1): with `d = z_design − z_ground` and the ground linear
/// on each triangle (z = Σ φⱼ·zⱼ, φ its corners' barycentric weights), `∂cut/∂zⱼ = ∫_{d<0} φⱼ dA` and
/// `∂fill/∂zⱼ = −∫_{d>0} φⱼ dA` (the zero line moves too, but d is 0 there: first order exact). Each piece is convex and
/// φⱼ linear on it, so `∫ φⱼ = area·φⱼ(centroid)`. Indexed by the ground's corners; also the cut and fill plan areas
/// (the sums of the weights: Σφ = 1).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sensitivity {
    pub cut: Vec<f64>,
    pub fill: Vec<f64>,
    pub cut_area: f64,
    pub fill_area: f64,
    /// The plan area where `|d| < near`: there an error of the heights moves the zero line and the cut and fill apart
    /// are not linear in them (their net is).
    pub near_area: f64,
    /// The plan area both surfaces cover.
    pub covered: f64,
}

/// A convex polygon's plan area and centroid.
fn area_centroid(poly: &[([f64; 2], f64)]) -> (f64, [f64; 2]) {
    let (mut area, mut cx, mut cy) = (0.0, 0.0, 0.0);
    for i in 1..poly.len().saturating_sub(1) {
        let (a, b, c) = (poly[0].0, poly[i].0, poly[i + 1].0);
        let s = area2(a, b, c) / 2.0;
        area += s;
        cx += s * (a[0] + b[0] + c[0]) / 3.0;
        cy += s * (a[1] + b[1] + c[1]) / 3.0;
    }
    if area > 0.0 { (area, [cx / area, cy / area]) } else { (0.0, [0.0, 0.0]) }
}

/// The sensitivity of the volumes between `design` and the ground TIN (`vertices`, `triangles` by corner index), and the
/// area where `|d| < near`.
pub fn sensitivity(design: &[Triangle], vertices: &[[f64; 3]], triangles: &[[u32; 3]], near: f64) -> Sensitivity {
    let mut kept: Vec<(Triangle, [u32; 3])> = Vec::with_capacity(triangles.len());
    for t in triangles {
        let tri = [vertices[t[0] as usize], vertices[t[1] as usize], vertices[t[2] as usize]];
        let a = area2([tri[0][0], tri[0][1]], [tri[1][0], tri[1][1]], [tri[2][0], tri[2][1]]);
        if a > 0.0 {
            kept.push((tri, *t));
        } else if a < 0.0 {
            kept.push(([tri[0], tri[2], tri[1]], [t[0], t[2], t[1]]));
        }
    }
    let plain: Vec<Triangle> = kept.iter().map(|k| k.0).collect();
    let mut ground = Ground::new(&plain);
    let mut out = Sensitivity { cut: vec![0.0; vertices.len()], fill: vec![0.0; vertices.len()], ..Sensitivity::default() };
    let covered = ground.overlay_on(design, |gi, with| {
        let band = cut_at(with, -near, true);
        if band.len() >= 3 {
            let band = cut_at(&band, near, false);
            if band.len() >= 3 {
                out.near_area += integral(&band).0;
            }
        }
        let (g, ids) = &kept[gi];
        let (a, b, c) = ([g[0][0], g[0][1]], [g[1][0], g[1][1]], [g[2][0], g[2][1]]);
        let whole = area2(a, b, c);
        let weights = |p: [f64; 2]| {
            let wa = area2(p, b, c) / whole;
            let wb = area2(a, p, c) / whole;
            [wa, wb, 1.0 - wa - wb]
        };
        for (above, sign) in [(true, -1.0), (false, 1.0)] {
            let part = split(with, above);
            if part.len() < 3 {
                continue;
            }
            let (area, at) = area_centroid(&part);
            if area <= 0.0 {
                continue;
            }
            let phi = weights(at);
            let into = if above { &mut out.fill } else { &mut out.cut };
            for k in 0..3 {
                into[ids[k] as usize] += sign * area * phi[k];
            }
            if above {
                out.fill_area += area;
            } else {
                out.cut_area += area;
            }
        }
    });
    out.covered = covered.0;
    out
}

/// What an earthwork has come to between its original ground and a later survey, against its design (docs/adr/0942,
/// plan/road.md Y36; ROAD-VOL-15), m³; at each place, with `z_o` the original ground, `z_n` the survey now and `z_d`
/// the design:
///
/// - in cut (`z_o > z_d`): done `max(0, z_o − max(z_n, z_d))`, left `max(0, min(z_n, z_o) − z_d)`, over (dug below the
///   design) `max(0, z_d − z_n)`;
/// - in fill (`z_d > z_o`): done `max(0, min(z_n, z_d) − z_o)`, left `max(0, z_d − max(z_n, z_o))`, over (placed above
///   the design) `max(0, z_n − z_d)`.
///
/// Done + left is the design's own cut (or fill) at every place the survey covers, whatever the survey says: the survey
/// moves volume between the two, never makes or loses any. Material
/// heaped on a cut above its original ground or dug out of a fill below its original ground is neither (a stockpile, a
/// borrow pit of the site): it is not the design's work. Also the plan areas: covered by all three surfaces, the
/// design's not covered by the original or the survey.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Progress {
    pub cut_done: f64,
    pub cut_left: f64,
    pub cut_over: f64,
    pub fill_done: f64,
    pub fill_left: f64,
    pub fill_over: f64,
    pub covered: f64,
    pub uncovered: f64,
}

/// A convex polygon's corners with the design's, the original's and the survey's heights there.
type Tri3 = Vec<([f64; 2], [f64; 3])>;

/// The part of a convex polygon (three linear heights at its corners) where `f` (a linear combination of them, given
/// by its coefficients) is ≥ 0 (`above`) or ≤ 0.
fn split3(poly: &Tri3, f: [f64; 3], above: bool) -> Tri3 {
    let val = |z: &[f64; 3]| f[0] * z[0] + f[1] * z[1] + f[2] * z[2];
    let keep = |v: f64| if above { v >= 0.0 } else { v <= 0.0 };
    let mut out = Vec::with_capacity(poly.len() + 1);
    for i in 0..poly.len() {
        let (p, q) = (&poly[i], &poly[(i + 1) % poly.len()]);
        let (vp, vq) = (val(&p.1), val(&q.1));
        if keep(vp) {
            out.push(*p);
        }
        if (vp > 0.0 && vq < 0.0) || (vp < 0.0 && vq > 0.0) {
            let t = vp / (vp - vq);
            let at = [p.0[0] + (q.0[0] - p.0[0]) * t, p.0[1] + (q.0[1] - p.0[1]) * t];
            let z = [0, 1, 2].map(|k| p.1[k] + (q.1[k] - p.1[k]) * t);
            out.push((at, z));
        }
    }
    out
}

/// A convex polygon's area and its three heights at its centroid (each linear on it: the mean over it).
fn mean3(poly: &Tri3) -> (f64, [f64; 3]) {
    let (mut area, mut z) = (0.0, [0.0; 3]);
    for i in 1..poly.len().saturating_sub(1) {
        let (a, b, c) = (&poly[0], &poly[i], &poly[i + 1]);
        let s = area2(a.0, b.0, c.0) / 2.0;
        area += s;
        for (k, zk) in z.iter_mut().enumerate() {
            *zk += s * (a.1[k] + b.1[k] + c.1[k]) / 3.0;
        }
    }
    if area > 0.0 { (area, z.map(|v| v / area)) } else { (0.0, [0.0; 3]) }
}

/// The progress of `design` over `original` as `now` surveys it (each a set of triangles that do not overlap in plan),
/// over the design's plan. Each place where a design, an original and a survey triangle overlap is cut along the
/// lines where two of the three heights meet; on each piece their order is fixed, every quantity is linear and its
/// integral is the area times its value at the centroid: exact.
pub fn progress(design: &[Triangle], original: &[Triangle], now: &[Triangle]) -> Progress {
    let (o_tris, n_tris): (Vec<Triangle>, Vec<Triangle>) =
        (original.iter().filter_map(ccw).collect(), now.iter().filter_map(ccw).collect());
    let mut out = Progress::default();
    let (Some(go), Some(gn)) = ((!o_tris.is_empty()).then(|| Grid::new(&o_tris)), (!n_tris.is_empty()).then(|| Grid::new(&n_tris)))
    else {
        out.uncovered = design.iter().filter_map(ccw).map(|d| area2([d[0][0], d[0][1]], [d[1][0], d[1][1]], [d[2][0], d[2][1]]) / 2.0).sum();
        return out;
    };
    let near = |grid: &Grid, t: &Triangle| {
        let (lo, hi) = grid.span(t);
        let mut ids: Vec<u32> = Vec::new();
        for cy in lo[1]..=hi[1] {
            for cx in lo[0]..=hi[0] {
                ids.extend_from_slice(&grid.cells[cy * grid.nx + cx]);
            }
        }
        ids.sort_unstable();
        ids.dedup();
        ids
    };
    for d in design {
        let Some(d) = ccw(d) else { continue };
        let plan = [[d[0][0], d[0][1]], [d[1][0], d[1][1]], [d[2][0], d[2][1]]];
        let whole = area2(plan[0], plan[1], plan[2]) / 2.0;
        let mut covered = 0.0;
        let ns = near(&gn, &d);
        for oi in near(&go, &d) {
            let o = &o_tris[oi as usize];
            let mut base = plan.to_vec();
            for k in 0..3 {
                base = clip(&base, [o[k][0], o[k][1]], [o[(k + 1) % 3][0], o[(k + 1) % 3][1]]);
                if base.len() < 3 {
                    break;
                }
            }
            if base.len() < 3 {
                continue;
            }
            for &ni in &ns {
                let n = &n_tris[ni as usize];
                let mut poly = base.clone();
                for k in 0..3 {
                    poly = clip(&poly, [n[k][0], n[k][1]], [n[(k + 1) % 3][0], n[(k + 1) % 3][1]]);
                    if poly.len() < 3 {
                        break;
                    }
                }
                if poly.len() < 3 {
                    continue;
                }
                let piece: Tri3 = poly.iter().map(|p| (*p, [plane_z(&d, *p), plane_z(o, *p), plane_z(n, *p)])).collect();
                covered += mean3(&piece).0;
                // The lines z_o = z_d, z_n = z_d, z_o = z_n (heights in the order design, original, survey). A piece
                // on such a line throughout (a survey that has reached the design) goes to one side only, or it would
                // be counted twice.
                let mut parts = vec![piece];
                for f in [[-1.0, 1.0, 0.0], [-1.0, 0.0, 1.0], [0.0, 1.0, -1.0]] {
                    parts = parts
                        .into_iter()
                        .flat_map(|p| {
                            let flat = p.iter().all(|(_, z)| (f[0] * z[0] + f[1] * z[1] + f[2] * z[2]).abs() <= 1e-12);
                            if flat { vec![p] } else { vec![split3(&p, f, true), split3(&p, f, false)] }
                        })
                        .filter(|p| p.len() >= 3)
                        .collect();
                }
                for p in &parts {
                    let (area, [zd, zo, zn]) = mean3(p);
                    if area <= 0.0 {
                        continue;
                    }
                    if zo > zd {
                        out.cut_done += area * (zo - zn.max(zd)).max(0.0);
                        out.cut_left += area * (zn.min(zo) - zd).max(0.0);
                        out.cut_over += area * (zd - zn).max(0.0);
                    } else if zd > zo {
                        out.fill_done += area * (zn.min(zd) - zo).max(0.0);
                        out.fill_left += area * (zd - zn.max(zo)).max(0.0);
                        out.fill_over += area * (zn - zd).max(0.0);
                    }
                }
            }
        }
        out.covered += covered;
        out.uncovered += (whole - covered).max(0.0);
    }
    out
}
