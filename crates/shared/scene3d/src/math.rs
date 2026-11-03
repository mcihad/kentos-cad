//! f64 vectors (`[f64; 3]`) and column-major 4×4 matrices: the few operations the camera and the tiles need, written
//! out so the core carries no dependency and every platform computes them alike.

pub type V3 = [f64; 3];

pub fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn scale(a: V3, k: f64) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}

pub fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

pub fn length(a: V3) -> f64 {
    dot(a, a).sqrt()
}

/// `a` of length one; the zero vector stays zero.
pub fn normalize(a: V3) -> V3 {
    let l = length(a);
    if l > 0.0 { scale(a, 1.0 / l) } else { a }
}

/// A column-major 4×4 matrix: element (row r, column c) at `c * 4 + r`, as WGSL's `mat4x4<f32>` reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct M4(pub [f64; 16]);

impl M4 {
    pub const IDENTITY: M4 = M4([1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]);

    /// The matrix of rows.
    pub fn from_rows(r: [[f64; 4]; 4]) -> M4 {
        let mut m = [0.0; 16];
        for (i, row) in r.iter().enumerate() {
            for (j, v) in row.iter().enumerate() {
                m[j * 4 + i] = *v;
            }
        }
        M4(m)
    }

    pub fn at(&self, r: usize, c: usize) -> f64 {
        self.0[c * 4 + r]
    }

    /// `self · o`.
    pub fn mul(&self, o: &M4) -> M4 {
        let mut m = [0.0; 16];
        for c in 0..4 {
            for r in 0..4 {
                m[c * 4 + r] = (0..4).map(|k| self.at(r, k) * o.at(k, c)).sum();
            }
        }
        M4(m)
    }

    /// `self · (v, 1)`.
    pub fn apply(&self, v: V3) -> [f64; 4] {
        std::array::from_fn(|r| self.at(r, 0) * v[0] + self.at(r, 1) * v[1] + self.at(r, 2) * v[2] + self.at(r, 3))
    }

    /// The matrix as the GPU takes it.
    pub fn to_f32(&self) -> [f32; 16] {
        self.0.map(|v| v as f32)
    }
}
