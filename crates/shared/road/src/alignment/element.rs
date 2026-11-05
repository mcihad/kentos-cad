//! An alignment's element (docs/adr/0901 §1): a tangent, a circular curve or a clothoid, all in one form. The
//! direction θ(s) = θ0 + k0·s + (k1−k0)·s²/(2L); a place is the start plus ∫(cos θ, sin θ). A tangent's and a
//! curve's place is closed form; a clothoid's is the integral by 10-point Gauss–Legendre on equal pieces in which θ
//! turns at most 0.2 rad, their count fixed by the element's largest curvature and its length.

/// What an element is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Curvature 0: an aliyman.
    Tangent,
    /// Constant curvature: a kurp.
    Arc,
    /// Curvature changing linearly: a klotoid (an ove one when neither end is 0).
    Spiral,
}

impl Kind {
    pub fn word(self) -> &'static str {
        match self {
            Kind::Tangent => "aliyman",
            Kind::Arc => "kurp",
            Kind::Spiral => "klotoid",
        }
    }
}

/// An element: its id (from its PI, docs/adr/0901 §2), kind, start place and direction (radians from east, counter-
/// clockwise), length, curvatures at its start and end (left turn positive), and its station at the start.
#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    pub id: String,
    pub kind: Kind,
    pub start: [f64; 2],
    pub theta0: f64,
    pub length: f64,
    pub k0: f64,
    pub k1: f64,
    pub station: f64,
}

/// The 10-point Gauss–Legendre rule on [−1, 1]: the positive nodes and their weights (the nearest doubles).
const GL_NODES: [f64; 5] = [
    0.14887433898163122,
    0.4333953941292472,
    0.6794095682990244,
    0.8650633666889845,
    0.9739065285171717,
];
const GL_WEIGHTS: [f64; 5] = [
    0.29552422471475287,
    0.26926671930999635,
    0.21908636251598204,
    0.1494513491505806,
    0.06667134430868814,
];

/// The most θ may turn in one piece of the integral.
const PIECE_TURN: f64 = 0.2;

/// sin(x)/x, exact near 0.
fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-4 {
        1.0 - x * x / 6.0
    } else {
        x.sin() / x
    }
}

/// ∫₀ˢ (cos θ(t), sin θ(t)) dt for θ(t) = θ0 + k0·t + c·t²/2.
///
/// The error is bounded (docs/adr/0969): on a piece the code's turn rule keeps θ′·h/2 ≤ 0.2 and |c|·h²/8 ≤ 0.05, so
/// cos θ and sin θ stay under e^{3.49} on the Bernstein ellipse ρ = 10, and Gauss's error with 10 nodes,
/// (64/15)·M·ρ⁻²⁰/(ρ² − 1) (Trefethen 2008, Thm 4.5), comes to at most 10⁻²⁰·|s| for the whole integral. What is left is
/// rounding: against the Fresnel integrals at 60 digits at most 2.5·10⁻¹³ m (fixtures/road/v1/clothoid-integrals.json).
pub fn integral(theta0: f64, k0: f64, c: f64, s: f64) -> [f64; 2] {
    if s == 0.0 {
        return [0.0, 0.0];
    }
    if c == 0.0 {
        // A line or a curve: the chord, s·sinc(k·s/2) long, at the mean direction.
        let half = k0 * s / 2.0;
        let chord = s * sinc(half);
        let mid = theta0 + half;
        return [chord * mid.cos(), chord * mid.sin()];
    }
    let turn = (k0.abs() + 0.5 * c.abs() * s.abs()) * s.abs();
    let pieces = ((turn / PIECE_TURN).ceil() as usize).clamp(1, 1 << 20);
    let h = s / pieces as f64;
    let (mut x, mut y) = (0.0, 0.0);
    for i in 0..pieces {
        let mid = h * (i as f64 + 0.5);
        let half = h / 2.0;
        let (mut px, mut py) = (0.0, 0.0);
        for (node, weight) in GL_NODES.iter().zip(GL_WEIGHTS) {
            for t in [mid - half * node, mid + half * node] {
                let th = theta0 + k0 * t + 0.5 * c * t * t;
                px += weight * th.cos();
                py += weight * th.sin();
            }
        }
        x += px * half;
        y += py * half;
    }
    [x, y]
}

impl Element {
    /// The change of curvature per metre.
    pub fn rate(&self) -> f64 {
        if self.length > 0.0 { (self.k1 - self.k0) / self.length } else { 0.0 }
    }

    /// The direction `s` metres from the start.
    pub fn theta(&self, s: f64) -> f64 {
        self.theta0 + self.k0 * s + 0.5 * self.rate() * s * s
    }

    /// The curvature `s` metres from the start.
    pub fn curvature(&self, s: f64) -> f64 {
        self.k0 + self.rate() * s
    }

    /// The place `s` metres from the start.
    pub fn point(&self, s: f64) -> [f64; 2] {
        let c = if self.kind == Kind::Spiral { self.rate() } else { 0.0 };
        let k = if self.kind == Kind::Tangent { 0.0 } else { self.k0 };
        let [dx, dy] = integral(self.theta0, k, c, s);
        [self.start[0] + dx, self.start[1] + dy]
    }

    /// The end place.
    pub fn end(&self) -> [f64; 2] {
        self.point(self.length)
    }

    /// The station at the end.
    pub fn end_station(&self) -> f64 {
        self.station + self.length
    }

    /// The direction the element turns through, end minus start.
    pub fn turn(&self) -> f64 {
        self.theta(self.length) - self.theta0
    }

    /// For a clothoid, its parameter A = √(L / |k1 − k0|) (A² = R·L when one end is straight); none otherwise.
    pub fn a(&self) -> Option<f64> {
        (self.kind == Kind::Spiral && self.k1 != self.k0).then(|| (self.length / (self.k1 - self.k0).abs()).sqrt())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_curve_closes_its_circle_and_a_spiral_matches_its_series() {
        let r = 100.0;
        let circle = integral(0.3, 1.0 / r, 0.0, 2.0 * std::f64::consts::PI * r);
        assert!(circle[0].abs() < 1e-9 && circle[1].abs() < 1e-9, "{circle:?}");
        // A clothoid from straight: x = L(1 − τ²/10 + τ⁴/216 …), y = L(τ/3 − τ³/42 …), τ = L/(2R).
        let (l, rr) = (80.0, 250.0);
        let t: f64 = l / (2.0 * rr);
        let [x, y] = integral(0.0, 0.0, 1.0 / (rr * l), l);
        let sx = l * (1.0 - t.powi(2) / 10.0 + t.powi(4) / 216.0 - t.powi(6) / 9360.0);
        let sy = l * (t / 3.0 - t.powi(3) / 42.0 + t.powi(5) / 1320.0 - t.powi(7) / 75600.0);
        assert!((x - sx).abs() < 1e-10 && (y - sy).abs() < 1e-10, "{x} {sx} {y} {sy}");
    }
}
