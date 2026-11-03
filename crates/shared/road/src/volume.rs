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

type Triangle = [[f64; 3]; 3];

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

/// The volumes between the design surface and the ground surface, each a set of triangles that do not overlap in plan
/// (x, y, z each corner), over the design's plan.
pub fn between_surfaces(design: &[Triangle], ground: &[Triangle]) -> Volumes {
    let ground: Vec<Triangle> = ground.iter().filter_map(ccw).collect();
    let mut out = Volumes::default();
    let grid = (!ground.is_empty()).then(|| Grid::new(&ground));
    let mut seen: Vec<u32> = vec![u32::MAX; ground.len()];
    for (di, d) in design.iter().enumerate() {
        let Some(d) = ccw(d) else { continue };
        let plan = [[d[0][0], d[0][1]], [d[1][0], d[1][1]], [d[2][0], d[2][1]]];
        let whole = area2(plan[0], plan[1], plan[2]) / 2.0;
        let mut covered = 0.0;
        if let Some(grid) = &grid {
            let (lo, hi) = grid.span(&d);
            for cy in lo[1]..=hi[1] {
                for cx in lo[0]..=hi[0] {
                    for &gi in &grid.cells[cy * grid.nx + cx] {
                        if seen[gi as usize] == di as u32 {
                            continue;
                        }
                        seen[gi as usize] = di as u32;
                        let g = &ground[gi as usize];
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
