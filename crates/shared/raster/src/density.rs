//! Densities (docs/adr/0232 §10, §11): Çekirdek yoğunluğu, each point's
//! kernel over a disc summed at a cell's centre; Çizgi yoğunluğu, the
//! length of the lines within a disc round the centre over the disc's area.

use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::vec2::Vec2;

use crate::index::{EdgeIndex, Index};
use crate::points::{Lines, Weighted};

/// A kernel, its integral over the plane 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kernel {
    Quartic,
    Triangular,
    Uniform,
    Epanechnikov,
    Triweight,
}

impl Kernel {
    /// K at squared relative distance u² = d² / r² < 1, times r².
    fn at(self, u2: f64) -> f64 {
        let pi = std::f64::consts::PI;
        let t = 1.0 - u2;
        match self {
            Kernel::Quartic => 3.0 / pi * t * t,
            Kernel::Triangular => 3.0 / pi * (1.0 - u2.sqrt()),
            Kernel::Uniform => 1.0 / pi,
            Kernel::Epanechnikov => 2.0 / pi * t,
            Kernel::Triweight => 4.0 / pi * t * t * t,
        }
    }
}

/// The weighted Silverman radius (ArcGIS's default, §10): none when it is 0 or the weights sum to 0.
pub fn silverman(p: &Weighted) -> Option<f64> {
    let total: f64 = p.w.iter().sum();
    if !(total > 0.0) {
        return None;
    }
    let (mut mx, mut my) = (0.0, 0.0);
    for (q, w) in p.xy.iter().zip(&p.w) {
        mx += w * q.x;
        my += w * q.y;
    }
    let (mx, my) = (mx / total, my / total);
    let mut sd = 0.0;
    let mut d: Vec<(f64, u32, f64)> = Vec::with_capacity(p.xy.len());
    for (i, (q, w)) in p.xy.iter().zip(&p.w).enumerate() {
        let (dx, dy) = (q.x - mx, q.y - my);
        let d2 = dx * dx + dy * dy;
        sd += w * d2;
        d.push((d2.sqrt(), i as u32, *w));
    }
    let sd = (sd / total).sqrt();
    d.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let half = total / 2.0;
    let mut acc = 0.0;
    let mut median = 0.0;
    for (dist, _, w) in &d {
        acc += w;
        if acc >= half {
            median = *dist;
            break;
        }
    }
    let r = 0.9 * sd.min((1.0 / std::f64::consts::LN_2).sqrt() * median) * libm::pow(total, -0.2);
    (r > 0.0 && r.is_finite()).then_some(r)
}

/// Çekirdek yoğunluğu over some points.
pub struct KernelDensity {
    pub points: Weighted,
    index: Index,
    pub radius: f64,
    kernel: Kernel,
    /// The unit's scale (10⁶ for km²).
    scale: f64,
}

impl KernelDensity {
    pub fn new(points: Weighted, radius: f64, kernel: Kernel, scale: f64) -> KernelDensity {
        let index = Index::new(&points.xy);
        KernelDensity {
            points,
            index,
            radius,
            kernel,
            scale,
        }
    }

    /// The density at `q`.
    pub fn at(&self, q: Vec2) -> f64 {
        let r2 = self.radius * self.radius;
        let mut sum = 0.0;
        self.index.within(&self.points.xy, q, self.radius, |i, d2| {
            if d2 < r2 {
                sum += self.points.w[i as usize] * self.kernel.at(d2 / r2);
            }
        });
        self.scale * sum / r2
    }
}

/// Çizgi yoğunluğu over some edges.
pub struct LineDensity {
    pub lines: Lines,
    index: EdgeIndex,
    pub radius: f64,
    scale: f64,
}

/// One thread's stamps for the edges a query has met.
pub struct Stamps {
    seen: Vec<u32>,
    stamp: u32,
}

impl LineDensity {
    pub fn new(lines: Lines, radius: f64, scale: f64) -> LineDensity {
        let index = EdgeIndex::new(&lines.edges);
        LineDensity {
            lines,
            index,
            radius,
            scale,
        }
    }

    pub fn stamps(&self) -> Stamps {
        Stamps {
            seen: vec![0; self.lines.edges.len()],
            stamp: 0,
        }
    }

    /// The density at `q`.
    pub fn at(&self, q: Vec2, s: &mut Stamps) -> f64 {
        s.stamp = s.stamp.wrapping_add(1);
        if s.stamp == 0 {
            s.seen.iter_mut().for_each(|v| *v = 0);
            s.stamp = 1;
        }
        let mut sum = 0.0;
        let r = self.radius;
        self.index.near(q, r, &mut s.seen, s.stamp, |i| {
            let len = inside(&self.lines.edges[i as usize], q, r);
            if len > 0.0 {
                sum += self.lines.w[i as usize] * len;
            }
        });
        self.scale * sum / (std::f64::consts::PI * r * r)
    }
}

/// The length of an edge inside the disc of radius `r` round `q`.
pub fn inside(e: &Edge, q: Vec2, r: f64) -> f64 {
    match *e {
        Edge::Seg { a, b } => {
            let (ax, ay) = (a.x - q.x, a.y - q.y);
            let (dx, dy) = (b.x - a.x, b.y - a.y);
            let len2 = dx * dx + dy * dy;
            if !(len2 > 0.0) {
                return 0.0;
            }
            // |a + t·d|² = r²: t² len2 + 2 t (a·d) + |a|² − r² = 0.
            let half_b = ax * dx + ay * dy;
            let c = ax * ax + ay * ay - r * r;
            let disc = half_b * half_b - len2 * c;
            if !(disc > 0.0) {
                return 0.0;
            }
            let root = disc.sqrt();
            let t0 = ((-half_b - root) / len2).max(0.0);
            let t1 = ((-half_b + root) / len2).min(1.0);
            if t1 > t0 {
                (t1 - t0) * len2.sqrt()
            } else {
                0.0
            }
        }
        Edge::Arc {
            c,
            r: ra,
            a0,
            sweep,
        } => {
            let (dx, dy) = (q.x - c.x, q.y - c.y);
            let dc = (dx * dx + dy * dy).sqrt();
            if !(ra > 0.0) || dc >= ra + r {
                return 0.0;
            }
            let span = sweep.abs().min(std::f64::consts::TAU);
            if dc + ra <= r {
                return ra * span;
            }
            if dc + r <= ra || dc == 0.0 {
                return 0.0;
            }
            // The arc's circle inside the disc: cos(θ − φ) ≥ (ra² + dc² − r²) / (2 ra dc).
            let kappa = ((ra * ra + dc * dc - r * r) / (2.0 * ra * dc)).clamp(-1.0, 1.0);
            let alpha = libm::acos(kappa);
            let phi = libm::atan2(dy, dx);
            let start = if sweep >= 0.0 { a0 } else { a0 + sweep };
            let tau = std::f64::consts::TAU;
            // The arc [start, start + span] against [φ − α, φ + α] turned by whole turns.
            let lo = phi - alpha;
            let base = ((start - lo) / tau).floor();
            let mut overlap = 0.0;
            for k in -1..=2 {
                let (s0, s1) = (
                    lo + (base + f64::from(k)) * tau,
                    lo + (base + f64::from(k)) * tau + 2.0 * alpha,
                );
                let (o0, o1) = (s0.max(start), s1.min(start + span));
                if o1 > o0 {
                    overlap += o1 - o0;
                }
            }
            ra * overlap.min(span)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_segment_and_an_arc_in_a_disc() {
        let q = Vec2::new(0.0, 0.0);
        let seg = |ax, ay, bx, by| Edge::Seg {
            a: Vec2::new(ax, ay),
            b: Vec2::new(bx, by),
        };
        assert!((inside(&seg(-10.0, 0.0, 10.0, 0.0), q, 5.0) - 10.0).abs() < 1e-12);
        assert!((inside(&seg(0.0, 0.0, 10.0, 0.0), q, 5.0) - 5.0).abs() < 1e-12);
        assert!((inside(&seg(-10.0, 3.0, 10.0, 3.0), q, 5.0) - 8.0).abs() < 1e-12);
        assert_eq!(inside(&seg(-10.0, 6.0, 10.0, 6.0), q, 5.0), 0.0);
        assert!((inside(&seg(1.0, 1.0, 2.0, 1.0), q, 5.0) - 1.0).abs() < 1e-12);
        // A whole circle of radius 2 round (3, 0) against a disc of 5: x ≤ 4 of its points…
        let pi = std::f64::consts::PI;
        let circle = Edge::Arc {
            c: Vec2::new(3.0, 0.0),
            r: 2.0,
            a0: 0.0,
            sweep: 2.0 * pi,
        };
        // |(3 + 2cosθ, 2sinθ)| ≤ 5 ⟺ 13 + 12cosθ ≤ 25: every θ. Inside whole.
        assert!((inside(&circle, q, 5.0) - 4.0 * pi).abs() < 1e-12);
        // Against a disc of 3: 13 + 12 cos θ ≤ 9 ⟺ cos θ ≤ −1/3.
        let want = 2.0 * (pi - libm::acos(-1.0 / 3.0)) * 2.0;
        assert!(
            (inside(&circle, q, 3.0) - want).abs() < 1e-12,
            "{}",
            inside(&circle, q, 3.0)
        );
        // Half of that circle (θ from π/2 to 3π/2), which holds all of the inside part.
        let half = Edge::Arc {
            c: Vec2::new(3.0, 0.0),
            r: 2.0,
            a0: pi / 2.0,
            sweep: pi,
        };
        assert!((inside(&half, q, 3.0) - want).abs() < 1e-12);
        // Clockwise, the same half.
        let back = Edge::Arc {
            c: Vec2::new(3.0, 0.0),
            r: 2.0,
            a0: 3.0 * pi / 2.0,
            sweep: -pi,
        };
        assert!((inside(&back, q, 3.0) - want).abs() < 1e-12);
    }
}
