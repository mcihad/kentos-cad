//! A surface's coverage of a project boundary (docs/adr/0920 §2; ROAD-SURVEY-08, ROAD-UX-07): how much of the boundary
//! the surface covers, the no-data rest, the surface outside it, and which part is estimated rather than measured: a
//! triangle with a plan edge longer than the user's limit spans too wide a gap between shots (Civil 3D's maximum
//! triangle length). Each triangle is clipped exactly against the boundary's rings (Sutherland–Hodgman, the triangle
//! the convex window); holes count out.

use super::Surface;

/// A boundary: its outer ring and holes, straight edges, in plan.
#[derive(Clone, Debug, PartialEq)]
pub struct Region {
    pub outer: Vec<[f64; 2]>,
    pub holes: Vec<Vec<[f64; 2]>>,
}

/// The coverage read: areas in plan (m²).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Coverage {
    /// The whole surface's area.
    pub surface_area: f64,
    /// The boundary's (its outer ring less its holes); none without a boundary.
    pub boundary_area: Option<f64>,
    /// The surface inside the boundary (all of it without one).
    pub covered: f64,
    /// The surface outside the boundary; none without one.
    pub outside: Option<f64>,
    /// Of `covered`, the triangles with an edge longer than the limit; 0 without a limit.
    pub estimated: f64,
    /// How many triangles are estimated and reach into the boundary.
    pub estimated_triangles: usize,
    /// The longest plan edge of a triangle reaching into the boundary; 0 when none does.
    pub longest_edge: f64,
}

fn shoelace(p: &[[f64; 2]]) -> f64 {
    let n = p.len();
    (0..n).map(|i| p[i][0] * p[(i + 1) % n][1] - p[(i + 1) % n][0] * p[i][1]).sum::<f64>() / 2.0
}

/// The plan area of `ring` inside the counter-clockwise triangle `t`: the ring clipped edge by edge against it.
pub fn clipped_area(ring: &[[f64; 2]], t: &[[f64; 2]; 3]) -> f64 {
    let mut poly: Vec<[f64; 2]> = ring.to_vec();
    for k in 0..3 {
        if poly.is_empty() {
            return 0.0;
        }
        let (a, b) = (t[k], t[(k + 1) % 3]);
        let side = |p: [f64; 2]| (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
        let mut out = Vec::with_capacity(poly.len() + 2);
        for i in 0..poly.len() {
            let (p, q) = (poly[i], poly[(i + 1) % poly.len()]);
            let (sp, sq) = (side(p), side(q));
            if sp >= 0.0 {
                out.push(p);
            }
            if (sp >= 0.0) != (sq >= 0.0) {
                let u = sp / (sp - sq);
                out.push([p[0] + u * (q[0] - p[0]), p[1] + u * (q[1] - p[1])]);
            }
        }
        poly = out;
    }
    if poly.len() < 3 { 0.0 } else { shoelace(&poly).abs() }
}

impl Region {
    /// The region's area: the outer ring's less its holes'.
    pub fn area(&self) -> f64 {
        shoelace(&self.outer).abs() - self.holes.iter().map(|h| shoelace(h).abs()).sum::<f64>()
    }

    /// The area of the region inside triangle `t` (any orientation).
    pub fn inside(&self, t: [[f64; 2]; 3]) -> f64 {
        let ccw = if (t[1][0] - t[0][0]) * (t[2][1] - t[0][1]) - (t[2][0] - t[0][0]) * (t[1][1] - t[0][1]) < 0.0 {
            [t[0], t[2], t[1]]
        } else {
            t
        };
        clipped_area(&self.outer, &ccw) - self.holes.iter().map(|h| clipped_area(h, &ccw)).sum::<f64>()
    }

    fn bounds(&self) -> [[f64; 2]; 2] {
        self.outer.iter().fold([[f64::INFINITY; 2], [f64::NEG_INFINITY; 2]], |b, p| {
            [[b[0][0].min(p[0]), b[0][1].min(p[1])], [b[1][0].max(p[0]), b[1][1].max(p[1])]]
        })
    }
}

impl Surface {
    /// A triangle's plan corners.
    fn plan(&self, t: &[u32; 3]) -> [[f64; 2]; 3] {
        t.map(|i| {
            let v = self.vertices[i as usize];
            [v[0], v[1]]
        })
    }

    /// A triangle's longest plan edge.
    pub fn longest_edge(&self, t: usize) -> f64 {
        let p = self.plan(&self.triangles[t]);
        (0..3).map(|k| (p[k][0] - p[(k + 1) % 3][0]).hypot(p[k][1] - p[(k + 1) % 3][1])).fold(0.0, f64::max)
    }

    /// The coverage of `boundary` (none: the surface itself) with triangles longer than `longest` estimated.
    pub fn coverage(&self, boundary: Option<&Region>, longest: Option<f64>) -> Coverage {
        let mut out = Coverage { boundary_area: boundary.map(Region::area), ..Coverage::default() };
        let bounds = boundary.map(Region::bounds);
        for (i, t) in self.triangles.iter().enumerate() {
            let p = self.plan(t);
            let whole = shoelace(&p).abs();
            out.surface_area += whole;
            let inside = match (boundary, bounds) {
                (Some(r), Some(b)) => {
                    let (lo, hi) = (p.iter().fold([f64::INFINITY; 2], |m, q| [m[0].min(q[0]), m[1].min(q[1])]), p
                        .iter()
                        .fold([f64::NEG_INFINITY; 2], |m, q| [m[0].max(q[0]), m[1].max(q[1])]));
                    if hi[0] < b[0][0] || lo[0] > b[1][0] || hi[1] < b[0][1] || lo[1] > b[1][1] {
                        0.0
                    } else {
                        r.inside(p)
                    }
                }
                _ => whole,
            };
            if inside <= 0.0 {
                continue;
            }
            out.covered += inside;
            let edge = self.longest_edge(i);
            out.longest_edge = out.longest_edge.max(edge);
            if longest.is_some_and(|l| edge > l) {
                out.estimated += inside;
                out.estimated_triangles += 1;
            }
        }
        if boundary.is_some() {
            out.outside = Some(out.surface_area - out.covered);
        }
        out
    }

    /// The triangles longer than `longest` reaching into `boundary` (none: anywhere), by index.
    pub fn estimated_triangles(&self, boundary: Option<&Region>, longest: f64) -> Vec<usize> {
        (0..self.triangles.len())
            .filter(|&i| self.longest_edge(i) > longest && boundary.is_none_or(|r| r.inside(self.plan(&self.triangles[i])) > 0.0))
            .collect()
    }
}
