//! A local system's plane transform (docs/adr/0168 §1): how its coordinates
//! become its base system's, by a similarity (a shift east and north, a
//! counter-clockwise turn in degrees, a scale) or by an affine transform's
//! six coefficients; and back. PROJ's `affine` step is the reference
//! (`scripts/fixtures/crs_custom_cases.py`).

use crate::jsmath::PI;
use crate::vec2::Vec2;

/// This system → its base: base x = a·x + b·y + c, base y = d·x + e·y + f.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Plane {
    /// Turned by `rotation` degrees counter-clockwise, scaled, then shifted.
    Similarity {
        east: f64,
        north: f64,
        rotation: f64,
        scale: f64,
    },
    Affine {
        a: f64,
        b: f64,
        c: f64,
        d: f64,
        e: f64,
        f: f64,
    },
}

crate::json_tagged!(
    Plane,
    "kind",
    Similarity => "similarity" { east, north, rotation, scale },
    Affine => "affine" { a, b, c, d, e, f },
);

impl Plane {
    /// The six coefficients [a, b, c, d, e, f].
    pub fn coefficients(&self) -> [f64; 6] {
        match *self {
            Plane::Similarity {
                east,
                north,
                rotation,
                scale,
            } => {
                let th = rotation * (PI / 180.0);
                let (c, s) = (libm::cos(th), libm::sin(th));
                [scale * c, -scale * s, east, scale * s, scale * c, north]
            }
            Plane::Affine { a, b, c, d, e, f } => [a, b, c, d, e, f],
        }
    }

    /// A point of this system in its base (in PROJ's order of sums).
    pub fn forward(&self, p: Vec2) -> Vec2 {
        let [a, b, c, d, e, f] = self.coefficients();
        Vec2::new(a * p.x + b * p.y + c, d * p.x + e * p.y + f)
    }

    /// A point of the base in this system, as PROJ's reverse `affine` step
    /// finds it (the inverse matrix first); none when the transform folds
    /// the plane.
    pub fn inverse(&self, p: Vec2) -> Option<Vec2> {
        let [a, b, c, d, e, f] = self.coefficients();
        let det = a * e - b * d;
        if det == 0.0 || !det.is_finite() {
            return None;
        }
        let (ia, ib, id, ie) = (e / det, -b / det, -d / det, a / det);
        let (x, y) = (p.x - c, p.y - f);
        Some(Vec2::new(ia * x + ib * y, id * x + ie * y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_point_goes_there_and_back() {
        let planes = [
            Plane::Similarity {
                east: 492_345.678,
                north: 4_422_345.678,
                rotation: 12.5,
                scale: 1.000_012,
            },
            Plane::Affine {
                a: 1.000_02,
                b: -0.0003,
                c: 412_345.0,
                d: 0.000_25,
                e: 0.999_98,
                f: 4_512_345.0,
            },
        ];
        for plane in planes {
            let p = Vec2::new(1234.567, -876.543);
            let back = plane.inverse(plane.forward(p)).expect("a plane");
            assert!(
                (back.x - p.x).abs() < 1e-8 && (back.y - p.y).abs() < 1e-8,
                "{back:?}"
            );
        }
        let flat = Plane::Affine {
            a: 1.0,
            b: 2.0,
            c: 0.0,
            d: 2.0,
            e: 4.0,
            f: 0.0,
        };
        assert_eq!(flat.inverse(Vec2::new(1.0, 1.0)), None);
    }
}
