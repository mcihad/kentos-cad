//! A corridor's footprint against protected areas (docs/adr/0952, plan/road.md Y46; ROAD-GRADE-06): how much of a
//! protected area — a building's plot, a heritage site, a stream's bank — the corridor's top surface covers in plan,
//! and how far each section's footprint reaches into it.
//!
//! - **An area** is an outer ring and holes (an area object's), straight between its corners.
//! - **The covered area** is the sum over the top surface's triangles of each one's overlap with the area: the outer
//!   ring's overlap less the holes'. A triangle's overlap with a ring is the ring clipped to the triangle (Sutherland–
//!   Hodgman: the clip region convex, the ring any simple polygon; where a concave ring folds over the triangle's edge
//!   the clip leaves zero-width bridges that add no area). Where the corridor folds, the folded triangles count twice
//!   (the corridor's own warning says where).
//! - **A section's reach** is the length of its footprint (from the left slope's end to the right's, in plan) inside the
//!   area: the segment cut at every edge it crosses, each piece inside by its middle (even–odd), the holes' pieces less.

/// An area: its outer ring and its holes, each corners in order (either way round).
#[derive(Clone, Debug, PartialEq)]
pub struct Area {
    pub outer: Vec<[f64; 2]>,
    pub holes: Vec<Vec<[f64; 2]>>,
}

/// A ring's signed area (counter-clockwise positive).
pub fn signed_area(ring: &[[f64; 2]]) -> f64 {
    let n = ring.len();
    (0..n).map(|i| {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        a[0] * b[1] - b[0] * a[1]
    })
    .sum::<f64>()
        / 2.0
}

/// The ring clipped to a triangle (Sutherland–Hodgman), its area.
pub fn overlap(triangle: [[f64; 2]; 3], ring: &[[f64; 2]]) -> f64 {
    let mut t = triangle;
    if signed_area(&t) < 0.0 {
        t.swap(1, 2);
    }
    if signed_area(&t).abs() == 0.0 || ring.len() < 3 {
        return 0.0;
    }
    let mut poly: Vec<[f64; 2]> = ring.to_vec();
    for k in 0..3 {
        let (a, b) = (t[k], t[(k + 1) % 3]);
        // Inside: on the left of a→b.
        let side = |p: [f64; 2]| (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
        let mut out = Vec::with_capacity(poly.len() + 3);
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
        if poly.len() < 3 {
            return 0.0;
        }
    }
    signed_area(&poly).abs()
}

/// Whether `p` lies inside the ring (even–odd).
pub fn inside(ring: &[[f64; 2]], p: [f64; 2]) -> bool {
    let n = ring.len();
    let mut c = false;
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        if (a[1] > p[1]) != (b[1] > p[1]) {
            let x = a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1]);
            if p[0] < x {
                c = !c;
            }
        }
    }
    c
}

/// The length of the segment a→b inside the ring.
pub fn length_inside(ring: &[[f64; 2]], a: [f64; 2], b: [f64; 2]) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let len = d[0].hypot(d[1]);
    if len == 0.0 || ring.len() < 3 {
        return 0.0;
    }
    let mut ts = vec![0.0, 1.0];
    let n = ring.len();
    for i in 0..n {
        let (p, q) = (ring[i], ring[(i + 1) % n]);
        let e = [q[0] - p[0], q[1] - p[1]];
        let den = d[0] * e[1] - d[1] * e[0];
        if den == 0.0 {
            continue;
        }
        let w = [p[0] - a[0], p[1] - a[1]];
        let t = (w[0] * e[1] - w[1] * e[0]) / den;
        let u = (w[0] * d[1] - w[1] * d[0]) / den;
        if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
            ts.push(t);
        }
    }
    ts.sort_by(f64::total_cmp);
    ts.windows(2)
        .filter(|w| w[1] > w[0])
        .filter(|w| {
            let m = 0.5 * (w[0] + w[1]);
            inside(ring, [a[0] + m * d[0], a[1] + m * d[1]])
        })
        .map(|w| (w[1] - w[0]) * len)
        .sum()
}

impl Area {
    /// The area's own size: the outer ring's less the holes'.
    pub fn size(&self) -> f64 {
        signed_area(&self.outer).abs() - self.holes.iter().map(|h| signed_area(h).abs()).sum::<f64>()
    }

    /// A triangle's overlap with the area.
    pub fn covered(&self, triangle: [[f64; 2]; 3]) -> f64 {
        overlap(triangle, &self.outer) - self.holes.iter().map(|h| overlap(triangle, h)).sum::<f64>()
    }

    /// The length of the segment a→b inside the area.
    pub fn reach(&self, a: [f64; 2], b: [f64; 2]) -> f64 {
        length_inside(&self.outer, a, b) - self.holes.iter().map(|h| length_inside(h, a, b)).sum::<f64>()
    }
}
