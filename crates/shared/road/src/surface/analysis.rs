//! A surface's analysis (plan/road.md Y18.1, docs/adr/0916 §1; ROAD-TIN-16): the plan and the 3D area in ranges of
//! elevation, slope and aspect (Civil 3D's Elevations, Slopes and Directions analyses, OpenRoads' slope and aspect of a
//! triangle), the surface's roughness and how far a raster DEM sampled from it would put its slope.
//!
//! - **Elevation:** a triangle is a plane, so the part of it between two levels is the triangle clipped by the two
//!   planes z = lo and z = hi: its plan area exactly; its 3D area the plan area times √(1 + |∇z|²).
//! - **Slope and aspect:** constant on a triangle: the whole triangle goes to its range. Slope in percent (rise over
//!   run × 100); aspect the azimuth it falls towards, degrees clockwise from north (OpenRoads' Analyze Point); a flat
//!   triangle has none and is counted apart.
//! - **Ranges:** the breaks, increasing; a value v is in [bᵢ, bᵢ₊₁), the last range closed; below the first or above
//!   the last is outside. KentOS gives no breaks: they are the user's.
//! - **Roughness:** the vector ruggedness measure (Sappington et al., 2007) on the triangles: at a corner,
//!   1 − |Σ αᵢ nᵢ| / Σ αᵢ over the triangles round it, nᵢ a triangle's unit normal and αᵢ its plan angle at the corner
//!   (so a plane split into triangles either way is the same); 0 on a plane, towards 1 the more the facets turn.
//! - **TIN against DEM:** heights sampled at a grid's cell centres, the slope by Horn's 3 × 3 differences, against
//!   the slope of the triangle the centre falls in; a raster smooths what a triangle keeps.

use super::Surface;

/// Why an analysis is not made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnalysisError {
    /// Fewer than two breaks, a break not finite, or not increasing.
    Breaks,
    /// A grid's cell not a number above zero.
    Cell,
    /// More grid cells than [`MAX_CELLS`].
    TooMany,
}

/// The most grid cells one comparison samples.
pub const MAX_CELLS: u64 = 4_000_000;

/// A range's area: from `from` to `to`, the plan and the 3D area in it, and how many triangles (slope and aspect) are
/// in it.
#[derive(Clone, Debug, PartialEq)]
pub struct Band {
    pub from: f64,
    pub to: f64,
    pub area: f64,
    pub surface_area: f64,
    pub triangles: usize,
}

/// The ranges and what is outside them (and, for aspect, flat).
#[derive(Clone, Debug, PartialEq)]
pub struct Bands {
    pub bands: Vec<Band>,
    pub outside_area: f64,
    pub outside_triangles: usize,
    pub flat_area: f64,
    pub flat_triangles: usize,
}

/// The roughness: its mean over the corners, its largest and where (the smallest x, then y, of the largest).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Roughness {
    pub mean: f64,
    pub max: f64,
    pub at: [f64; 2],
}

/// A raster's slope against the triangles': the cells compared, the mean (raster − triangle) and mean absolute
/// difference, the largest absolute one and where; percent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridSlope {
    pub cells: usize,
    pub mean: f64,
    pub mean_absolute: f64,
    pub max_absolute: f64,
    pub at: Option<[f64; 2]>,
}

/// Each range's parts as plan polygons.
pub type BandParts = Vec<Vec<Vec<[f64; 2]>>>;

/// The measure banded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Measure {
    Elevation,
    Slope,
    Aspect,
}

fn checked(breaks: &[f64]) -> Result<(), AnalysisError> {
    if breaks.len() < 2 || breaks.iter().any(|b| !b.is_finite()) || breaks.windows(2).any(|w| w[0] >= w[1]) {
        return Err(AnalysisError::Breaks);
    }
    Ok(())
}

/// The range of `v`, or none outside.
fn range_of(breaks: &[f64], v: f64) -> Option<usize> {
    let n = breaks.len();
    if v < breaks[0] || v > breaks[n - 1] {
        return None;
    }
    if v == breaks[n - 1] {
        return Some(n - 2);
    }
    Some(breaks.partition_point(|b| *b <= v) - 1)
}

/// A plan polygon's area (counter-clockwise positive).
fn area(poly: &[[f64; 3]]) -> f64 {
    let n = poly.len();
    let mut s = 0.0;
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        s += a[0] * b[1] - b[0] * a[1];
    }
    s / 2.0
}

/// The polygon's part with z on the `keep_above` side of `level` (z ≥ level, or z ≤ level).
fn clip(poly: &[[f64; 3]], level: f64, keep_above: bool) -> Vec<[f64; 3]> {
    let inside = |p: &[f64; 3]| if keep_above { p[2] >= level } else { p[2] <= level };
    let mut out = Vec::with_capacity(poly.len() + 2);
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let (ia, ib) = (inside(&a), inside(&b));
        if ia {
            out.push(a);
        }
        if ia != ib {
            let t = (level - a[2]) / (b[2] - a[2]);
            out.push([a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1]), level]);
        }
    }
    out
}

impl Surface {
    fn corners(&self, t: usize) -> [[f64; 3]; 3] {
        self.triangles[t].map(|i| self.vertices[i as usize])
    }

    /// A triangle's plan area and its 3D area over plan.
    fn stretch(&self, t: usize) -> (f64, f64) {
        let s = self.slope_of(t as u32).map_or(0.0, |s| s.slope);
        (area(&self.corners(t)), (1.0 + s * s).sqrt())
    }

    /// The plan and 3D areas of the surface.
    pub fn areas(&self) -> (f64, f64) {
        (0..self.triangles.len()).map(|t| self.stretch(t)).fold((0.0, 0.0), |(a, s), (pa, k)| (a + pa, s + pa * k))
    }

    /// The areas in the ranges of `measure` between `breaks` (elevation in metres, slope in percent, aspect in degrees
    /// from north). With `pieces`, each range's parts as plan polygons too (elevation: the clipped parts; slope and
    /// aspect: the triangles).
    pub fn bands(&self, measure: Measure, breaks: &[f64], pieces: bool) -> Result<(Bands, BandParts), AnalysisError> {
        checked(breaks)?;
        let n = breaks.len() - 1;
        let mut out = Bands {
            bands: (0..n)
                .map(|i| Band { from: breaks[i], to: breaks[i + 1], area: 0.0, surface_area: 0.0, triangles: 0 })
                .collect(),
            outside_area: 0.0,
            outside_triangles: 0,
            flat_area: 0.0,
            flat_triangles: 0,
        };
        let mut parts: BandParts = vec![Vec::new(); if pieces { n } else { 0 }];
        let flat = |p: &[[f64; 3]]| p.iter().map(|v| [v[0], v[1]]).collect::<Vec<_>>();
        for t in 0..self.triangles.len() {
            let (plan, k) = self.stretch(t);
            let tri = self.corners(t);
            match measure {
                Measure::Elevation => {
                    let mut inside = 0.0;
                    for (i, b) in out.bands.iter_mut().enumerate() {
                        let low = clip(&tri, b.from, true);
                        if low.len() < 3 {
                            continue;
                        }
                        let part = clip(&low, b.to, false);
                        if part.len() < 3 {
                            continue;
                        }
                        let a = area(&part);
                        if a <= 0.0 {
                            continue;
                        }
                        b.area += a;
                        b.surface_area += a * k;
                        inside += a;
                        if pieces {
                            parts[i].push(flat(&part));
                        }
                    }
                    let left = (plan - inside).max(0.0);
                    out.outside_area += left;
                }
                Measure::Slope | Measure::Aspect => {
                    let s = self.slope_of(t as u32);
                    let v = match (measure, s) {
                        (Measure::Slope, Some(s)) => Some(s.slope * 100.0),
                        (Measure::Aspect, Some(s)) => s.aspect.map(f64::to_degrees),
                        _ => None,
                    };
                    let Some(v) = v else {
                        out.flat_area += plan;
                        out.flat_triangles += 1;
                        continue;
                    };
                    match range_of(breaks, v) {
                        Some(i) => {
                            let b = &mut out.bands[i];
                            b.area += plan;
                            b.surface_area += plan * k;
                            b.triangles += 1;
                            if pieces {
                                parts[i].push(flat(&tri));
                            }
                        }
                        None => {
                            out.outside_area += plan;
                            out.outside_triangles += 1;
                        }
                    }
                }
            }
        }
        Ok((out, parts))
    }

    /// The vector ruggedness measure at every corner of a kept triangle, with the plan angles weighting the facets.
    pub fn roughness(&self) -> Option<Roughness> {
        let mut sum = vec![[0.0_f64; 3]; self.vertices.len()];
        let mut weight = vec![0.0_f64; self.vertices.len()];
        for t in &self.triangles {
            let p = t.map(|i| self.vertices[i as usize]);
            let u = [p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2]];
            let v = [p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]];
            let n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            if len == 0.0 {
                continue;
            }
            let n = [n[0] / len, n[1] / len, n[2] / len];
            for k in 0..3 {
                let (a, b, c) = (p[k], p[(k + 1) % 3], p[(k + 2) % 3]);
                let (x, y) = ([b[0] - a[0], b[1] - a[1]], [c[0] - a[0], c[1] - a[1]]);
                let angle = (x[0] * y[1] - x[1] * y[0]).abs().atan2(x[0] * y[0] + x[1] * y[1]);
                let i = t[k] as usize;
                for d in 0..3 {
                    sum[i][d] += angle * n[d];
                }
                weight[i] += angle;
            }
        }
        let mut count = 0usize;
        let mut total = 0.0;
        let mut best: Option<(f64, [f64; 2])> = None;
        for (i, w) in weight.iter().enumerate() {
            if *w <= 0.0 {
                continue;
            }
            let s = sum[i];
            let r = (1.0 - (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt() / w).max(0.0);
            count += 1;
            total += r;
            let at = [self.vertices[i][0], self.vertices[i][1]];
            let better = match best {
                None => true,
                Some((m, p)) => r > m || (r == m && (at[0], at[1]) < (p[0], p[1])),
            };
            if better {
                best = Some((r, at));
            }
        }
        best.map(|(max, at)| Roughness { mean: total / count as f64, max, at })
    }

    /// The slope a raster of `cell` sampled from the surface gives by Horn's method, against the triangles': on the
    /// grid of the kept triangles' bounding box, the first centre half a cell in; a cell is compared where it and its
    /// eight neighbours are on the surface.
    pub fn grid_slope(&self, cell: f64) -> Result<GridSlope, AnalysisError> {
        if !(cell.is_finite() && cell > 0.0) {
            return Err(AnalysisError::Cell);
        }
        let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for t in &self.triangles {
            for i in t {
                let v = self.vertices[*i as usize];
                x0 = x0.min(v[0]);
                y0 = y0.min(v[1]);
                x1 = x1.max(v[0]);
                y1 = y1.max(v[1]);
            }
        }
        if !x0.is_finite() {
            return Ok(GridSlope { cells: 0, mean: 0.0, mean_absolute: 0.0, max_absolute: 0.0, at: None });
        }
        let nx = ((x1 - x0) / cell).floor() as u64;
        let ny = ((y1 - y0) / cell).floor() as u64;
        if nx.saturating_mul(ny) > MAX_CELLS {
            return Err(AnalysisError::TooMany);
        }
        let (nx, ny) = (nx as usize, ny as usize);
        let centre = |i: usize, j: usize| [x0 + cell / 2.0 + i as f64 * cell, y0 + cell / 2.0 + j as f64 * cell];
        let mut z = vec![None; nx * ny];
        let mut start = 0u32;
        for j in 0..ny {
            for i in 0..nx {
                let p = centre(i, j);
                let h = self.height_walking(p[0], p[1], &mut start);
                z[j * nx + i] = h.z.zip(h.triangle);
            }
        }
        let (mut cells, mut sum, mut abs, mut best) = (0usize, 0.0, 0.0, None::<(f64, [f64; 2])>);
        for j in 1..ny.saturating_sub(1) {
            for i in 1..nx.saturating_sub(1) {
                let Some((_, t)) = z[j * nx + i] else { continue };
                let mut w = [[0.0; 3]; 3];
                let mut whole = true;
                for (dj, row) in w.iter_mut().enumerate() {
                    for (di, v) in row.iter_mut().enumerate() {
                        match z[(j + dj - 1) * nx + (i + di - 1)] {
                            Some((h, _)) => *v = h,
                            None => whole = false,
                        }
                    }
                }
                if !whole {
                    continue;
                }
                // Horn: rows run north (j + 1) to south; w[dj][di] is the cell (i + di − 1, j + dj − 1).
                let gx = ((w[0][2] + 2.0 * w[1][2] + w[2][2]) - (w[0][0] + 2.0 * w[1][0] + w[2][0])) / (8.0 * cell);
                let gy = ((w[2][0] + 2.0 * w[2][1] + w[2][2]) - (w[0][0] + 2.0 * w[0][1] + w[0][2])) / (8.0 * cell);
                let raster = gx.hypot(gy) * 100.0;
                let Some(facet) = self.slope_of(t) else { continue };
                let d = raster - facet.slope * 100.0;
                cells += 1;
                sum += d;
                abs += d.abs();
                let at = centre(i, j);
                let better = match best {
                    None => true,
                    Some((m, p)) => d.abs() > m || (d.abs() == m && (at[0], at[1]) < (p[0], p[1])),
                };
                if better {
                    best = Some((d.abs(), at));
                }
            }
        }
        Ok(GridSlope {
            cells,
            mean: if cells > 0 { sum / cells as f64 } else { 0.0 },
            mean_absolute: if cells > 0 { abs / cells as f64 } else { 0.0 },
            max_absolute: best.map_or(0.0, |b| b.0),
            at: best.map(|b| b.1),
        })
    }
}
