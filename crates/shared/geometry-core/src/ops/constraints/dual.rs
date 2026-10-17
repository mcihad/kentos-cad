//! Forward-mode dual numbers over a constraint's own unknowns (docs/adr/0334 §4): each residual is written once and
//! gives its exact first derivatives, so the solver's Jacobian has no finite-difference error. A constraint touches at
//! most [`SLOTS`] unknowns (two points and a segment, or an arc's centre, radius and angles with a point).

use std::ops::{Add, Div, Mul, Neg, Sub};

/// The most unknowns one constraint touches.
pub const SLOTS: usize = 16;

/// A value and its derivatives by the constraint's unknowns, slot by slot.
#[derive(Clone, Copy, Debug)]
pub struct D {
    pub v: f64,
    pub d: [f64; SLOTS],
}

impl D {
    /// A constant: no derivative.
    pub fn c(v: f64) -> D {
        D { v, d: [0.0; SLOTS] }
    }

    /// The unknown in `slot`, at `v`.
    pub fn var(v: f64, slot: usize) -> D {
        let mut d = [0.0; SLOTS];
        d[slot] = 1.0;
        D { v, d }
    }

    fn map(self, v: f64, k: f64) -> D {
        let mut d = self.d;
        for x in &mut d {
            *x *= k;
        }
        D { v, d }
    }

    /// The square root; at zero its derivative is taken as zero (the constraint is singular there, which the solver
    /// sees as a row that lost its rank).
    pub fn sqrt(self) -> D {
        let v = libm::sqrt(self.v);
        if v > 0.0 {
            self.map(v, 0.5 / v)
        } else {
            D::c(v)
        }
    }

    pub fn sin(self) -> D {
        self.map(libm::sin(self.v), libm::cos(self.v))
    }

    pub fn cos(self) -> D {
        self.map(libm::cos(self.v), -libm::sin(self.v))
    }

    pub fn abs(self) -> D {
        if self.v < 0.0 { -self } else { self }
    }

    /// The angle of (x, y) from east, in (−π, π]; its derivative is zero at the origin.
    pub fn atan2(y: D, x: D) -> D {
        let v = libm::atan2(y.v, x.v);
        let r2 = x.v * x.v + y.v * y.v;
        if r2 == 0.0 {
            return D::c(v);
        }
        let mut d = [0.0; SLOTS];
        for (k, o) in d.iter_mut().enumerate() {
            *o = (x.v * y.d[k] - y.v * x.d[k]) / r2;
        }
        D { v, d }
    }
}

impl Add for D {
    type Output = D;
    fn add(self, o: D) -> D {
        let mut d = self.d;
        for (x, y) in d.iter_mut().zip(o.d) {
            *x += y;
        }
        D { v: self.v + o.v, d }
    }
}

impl Sub for D {
    type Output = D;
    fn sub(self, o: D) -> D {
        let mut d = self.d;
        for (x, y) in d.iter_mut().zip(o.d) {
            *x -= y;
        }
        D { v: self.v - o.v, d }
    }
}

impl Mul for D {
    type Output = D;
    fn mul(self, o: D) -> D {
        let mut d = [0.0; SLOTS];
        for (k, x) in d.iter_mut().enumerate() {
            *x = self.d[k] * o.v + self.v * o.d[k];
        }
        D { v: self.v * o.v, d }
    }
}

impl Div for D {
    type Output = D;
    fn div(self, o: D) -> D {
        let v = self.v / o.v;
        let mut d = [0.0; SLOTS];
        for (k, x) in d.iter_mut().enumerate() {
            *x = (self.d[k] - v * o.d[k]) / o.v;
        }
        D { v, d }
    }
}

impl Neg for D {
    type Output = D;
    fn neg(self) -> D {
        self.map(-self.v, -1.0)
    }
}

impl Mul<f64> for D {
    type Output = D;
    fn mul(self, k: f64) -> D {
        self.map(self.v * k, k)
    }
}

impl Add<f64> for D {
    type Output = D;
    fn add(self, k: f64) -> D {
        D {
            v: self.v + k,
            d: self.d,
        }
    }
}

impl Sub<f64> for D {
    type Output = D;
    fn sub(self, k: f64) -> D {
        D {
            v: self.v - k,
            d: self.d,
        }
    }
}

/// A point of duals.
#[derive(Clone, Copy, Debug)]
pub struct P {
    pub x: D,
    pub y: D,
}

impl Sub for P {
    type Output = P;
    fn sub(self, o: P) -> P {
        P {
            x: self.x - o.x,
            y: self.y - o.y,
        }
    }
}

impl Neg for P {
    type Output = P;
    fn neg(self) -> P {
        P {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl P {
    pub fn len(self) -> D {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn cross(self, o: P) -> D {
        self.x * o.y - self.y * o.x
    }

    pub fn dot(self, o: P) -> D {
        self.x * o.x + self.y * o.y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derivatives_are_exact() {
        let x = D::var(3.0, 0);
        let y = D::var(4.0, 1);
        let r = (x * x + y * y).sqrt();
        assert_eq!(r.v, 5.0);
        assert!((r.d[0] - 0.6).abs() < 1e-15 && (r.d[1] - 0.8).abs() < 1e-15);
        let a = D::atan2(y, x);
        assert!((a.d[0] + 4.0 / 25.0).abs() < 1e-15 && (a.d[1] - 3.0 / 25.0).abs() < 1e-15);
        let q = x / y;
        assert!((q.d[0] - 0.25).abs() < 1e-15 && (q.d[1] + 3.0 / 16.0).abs() < 1e-15);
    }
}
