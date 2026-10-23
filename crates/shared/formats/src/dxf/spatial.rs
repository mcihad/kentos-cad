//! DXF placements stay in XYZ until the final projection. Dropping Z before
//! composing a tilted parent loses both plan coordinates and elevations.
use super::entity::P3;
use crate::geom::{ocs_axes, Tf};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Map3 {
    x: P3,
    y: P3,
    z: P3,
    origin: P3,
}

impl Map3 {
    pub const IDENTITY: Self = Self {
        x: [1., 0., 0.],
        y: [0., 1., 0.],
        z: [0., 0., 1.],
        origin: [0.; 3],
    };

    pub fn is_identity(self) -> bool {
        self == Self::IDENTITY
    }

    pub fn translate(p: P3) -> Self {
        Self {
            origin: p,
            ..Self::IDENTITY
        }
    }

    pub fn scale([x, y, z]: P3) -> Self {
        Self {
            x: [x, 0., 0.],
            y: [0., y, 0.],
            z: [0., 0., z],
            origin: [0.; 3],
        }
    }

    pub fn rotate(degrees: f64) -> Self {
        let m = Tf::rotate_deg(degrees);
        Self {
            x: [m.a, m.b, 0.],
            y: [m.c, m.d, 0.],
            ..Self::IDENTITY
        }
    }

    pub fn ocs(normal: P3) -> Option<Self> {
        let (x, y, z) = ocs_axes(normal)?;
        Some(Self {
            x,
            y,
            z,
            origin: [0.; 3],
        })
    }

    pub fn vector(self, p: P3) -> P3 {
        if self.is_identity() {
            return p;
        }
        std::array::from_fn(|i| {
            // Keep the old planar arithmetic, including its rounding, when
            // an axis does not contribute. All three axes count otherwise.
            let xy = if self.x[i] == 0. && self.y[i] == 0. {
                0.
            } else {
                self.x[i] * p[0] + self.y[i] * p[1]
            };
            if self.z[i] == 0. {
                xy
            } else if self.x[i] == 0. && self.y[i] == 0. {
                self.z[i] * p[2]
            } else {
                xy + self.z[i] * p[2]
            }
        })
    }

    pub fn point(self, p: P3) -> P3 {
        if self.is_identity() {
            return p;
        }
        let v = self.vector(p);
        std::array::from_fn(|i| v[i] + self.origin[i])
    }

    /// Apply this placement first, then the parent's.
    pub fn then(self, parent: Self) -> Self {
        if self.is_identity() {
            return parent;
        }
        if parent.is_identity() {
            return self;
        }
        Self {
            x: parent.vector(self.x),
            y: parent.vector(self.y),
            z: parent.vector(self.z),
            origin: parent.point(self.origin),
        }
    }

    /// The XY image of the local plane at z; projection happens after composition.
    pub fn plane(self, z: f64) -> Tf {
        let o = self.point([0., 0., z]);
        Tf {
            a: self.x[0],
            b: self.x[1],
            c: self.y[0],
            d: self.y[1],
            e: o[0],
            f: o[1],
        }
    }

    pub fn height_origin(self) -> f64 {
        self.origin[2]
    }

    pub fn mixes_z(self) -> bool {
        self.x[2] != 0. || self.y[2] != 0. || self.z[0] != 0. || self.z[1] != 0.
    }
}
