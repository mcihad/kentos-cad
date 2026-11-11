//! Dönüşüm uygula's machine (plan/map-cad.md MAP-13, docs/adr/3100): an affine matrix of 4 × 4 numbers, row-major as
//! PDAL's filters.transformation and CloudCompare's Apply Transformation take it (the rotation, or any linear part, in
//! the upper left 3 × 3, the translation in the fourth column, the bottom row 0 0 0 1), applied to each record's X, Y
//! and Z. A point's coordinate is its whole number on the grid (X · scale + offset, as the file says); moved, it goes
//! back onto the same grid rounded half away from zero, so a coordinate is off by at most half the scale. The record's
//! other fields stay as they are: the matrix moves points, not normals in extra bytes nor scan angles. Before anything
//! is written the box's eight corners are moved: an affine map keeps every point inside the moved corners' hull, so a
//! box that stays within the grid's 32-bit reach keeps every point there.

use crate::record::Layout;
use crate::{PcError, Result};

/// How far the linear part may be from a rotation and still be called rigid: |RᵀR − I| per element.
const RIGID: f64 = 1e-9;

/// The axes' names in the messages.
const AXES: [&str; 3] = ["X", "Y", "Z"];

/// An affine map of 3D points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine {
    m: [f64; 16],
}

impl Affine {
    /// The map of 16 numbers, row-major; refused unless every number is finite, the bottom row is 0 0 0 1 and the
    /// linear part flattens no axis (its determinant is not zero).
    pub fn new(m: [f64; 16]) -> Result<Affine> {
        if m.iter().any(|v| !v.is_finite()) {
            return Err(PcError::new("Matrisin bütün sayıları sonlu olmalı."));
        }
        if m[12] != 0.0 || m[13] != 0.0 || m[14] != 0.0 || m[15] != 1.0 {
            return Err(PcError::new(
                "Matrisin son satırı 0 0 0 1 olmalı: yalnız afin dönüşüm uygulanır (perspektif yok).",
            ));
        }
        let a = Affine { m };
        if !(a.determinant().abs() > 1e-12) {
            return Err(PcError::new(
                "Matrisin dönüklük kısmı tekil (determinantı sıfır): noktaları bir düzleme ya da doğruya ezer.",
            ));
        }
        Ok(a)
    }

    /// The map from text: 16 numbers with a point for the decimal, apart by spaces, line ends, commas or semicolons
    /// (a pasted list's brackets too), row by row.
    pub fn parse(text: &str) -> Result<Affine> {
        let mut m = [0.0; 16];
        let mut n = 0;
        for word in text
            .split(|c: char| c.is_whitespace() || matches!(c, ',' | ';' | '[' | ']'))
            .filter(|w| !w.is_empty())
        {
            let v: f64 = word.parse().map_err(|_| {
                PcError::new(format!(
                    "Matriste “{word}” bir sayı değil; sayıları boşlukla ayırın, ondalık için nokta kullanın."
                ))
            })?;
            if let Some(slot) = m.get_mut(n) {
                *slot = v;
            }
            n += 1;
        }
        if n != 16 {
            return Err(PcError::new(format!(
                "Matris 16 sayıdır (4 satır, 4 sütun, satır satır); {n} sayı verildi."
            )));
        }
        Affine::new(m)
    }

    /// docs/adr/4300: a rotation of `degrees` about `axis` (right-handed: counter-clockwise seen from the axis's tip;
    /// Rodrigues' formula) about the point `center`, then the move `translation`: p ↦ R(p − c) + c + t, Tarama kaydı's
    /// recipe, as one map. Each number in a fixed order so that an independent reference gets the same bits: the axis
    /// over its length √(x·x + y·y + z·z); the angle in radians (degrees × π/180); with s and c its sine and cosine and
    /// k = 1 − c the rows [k·x·x + c, k·x·y − s·z, k·x·z + s·y], [k·x·y + s·z, k·y·y + c, k·y·z − s·x],
    /// [k·x·z − s·y, k·y·z + s·x, k·z·z + c]; the translation's row c_i + t_i − (R_i0·c0 + R_i1·c1 + R_i2·c2). Refused
    /// for a number not finite or an axis of no length.
    pub fn axis_angle(axis: [f64; 3], degrees: f64, center: [f64; 3], translation: [f64; 3]) -> Result<Affine> {
        if axis.iter().chain(&center).chain(&translation).chain(std::iter::once(&degrees)).any(|v| !v.is_finite()) {
            return Err(PcError::new("Eksenin, açının, merkezin ve ötelemenin bütün sayıları sonlu olmalı."));
        }
        let n = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
        if !(n > 0.0) {
            return Err(PcError::new("Dönme ekseninin uzunluğu sıfır olamaz; ekseni üç sayıyla yazın (Z için 0 0 1)."));
        }
        let (x, y, z) = (axis[0] / n, axis[1] / n, axis[2] / n);
        let r = degrees.to_radians();
        let (s, c) = (r.sin(), r.cos());
        let k = 1.0 - c;
        let rot = [
            [k * x * x + c, k * x * y - s * z, k * x * z + s * y],
            [k * x * y + s * z, k * y * y + c, k * y * z - s * x],
            [k * x * z - s * y, k * y * z + s * x, k * z * z + c],
        ];
        let mut m = [0.0; 16];
        for (i, row) in rot.iter().enumerate() {
            let rc = row[0] * center[0] + row[1] * center[1] + row[2] * center[2];
            m[4 * i..4 * i + 4].copy_from_slice(&[row[0], row[1], row[2], center[i] + translation[i] - rc]);
        }
        m[15] = 1.0;
        Affine::new(m)
    }

    /// docs/adr/5100: a rotation by Euler angles in degrees — `yaw` about Z, `pitch` about Y, `roll` about X, each
    /// right-handed, turned about the rotated axes Z, Y′, X″ in that order (the same as about the fixed axes X, Y, Z
    /// in that order): R = Rz(yaw)·Ry(pitch)·Rx(roll) — about the point `center`, then the move `translation`, as
    /// [`Affine::axis_angle`] places it. Each number in a fixed order so that an independent reference gets the same
    /// bits: with c1, s1 the cosine and sine of the yaw, c2, s2 the pitch's, c3, s3 the roll's (radians, degrees ×
    /// π/180) the rows [c2·c1, s3·s2·c1 − c3·s1, c3·s2·c1 + s3·s1], [c2·s1, s3·s2·s1 + c3·c1, c3·s2·s1 − s3·c1],
    /// [−s2, s3·c2, c3·c2], each product left to right; the translation's row as the axis's. Refused for a number not
    /// finite.
    pub fn euler(yaw: f64, pitch: f64, roll: f64, center: [f64; 3], translation: [f64; 3]) -> Result<Affine> {
        if [yaw, pitch, roll].iter().chain(&center).chain(&translation).any(|v| !v.is_finite()) {
            return Err(PcError::new("Açıların, merkezin ve ötelemenin bütün sayıları sonlu olmalı."));
        }
        let (a, b, c) = (yaw.to_radians(), pitch.to_radians(), roll.to_radians());
        let (s1, c1) = (a.sin(), a.cos());
        let (s2, c2) = (b.sin(), b.cos());
        let (s3, c3) = (c.sin(), c.cos());
        let rot = [
            [c2 * c1, s3 * s2 * c1 - c3 * s1, c3 * s2 * c1 + s3 * s1],
            [c2 * s1, s3 * s2 * s1 + c3 * c1, c3 * s2 * s1 - s3 * c1],
            [-s2, s3 * c2, c3 * c2],
        ];
        Affine::placed(rot, center, translation)
    }

    /// A rotation `rot` about the point `center`, then the move `translation`: p ↦ R(p − c) + c + t, the translation's
    /// row c_i + t_i − (R_i0·c0 + R_i1·c1 + R_i2·c2).
    fn placed(rot: [[f64; 3]; 3], center: [f64; 3], translation: [f64; 3]) -> Result<Affine> {
        let mut m = [0.0; 16];
        for (i, row) in rot.iter().enumerate() {
            let rc = row[0] * center[0] + row[1] * center[1] + row[2] * center[2];
            m[4 * i..4 * i + 4].copy_from_slice(&[row[0], row[1], row[2], center[i] + translation[i] - rc]);
        }
        m[15] = 1.0;
        Affine::new(m)
    }

    /// The map `after` applied after this one (docs/adr/5100: a cloud's place given again on top of the one it has),
    /// each number in a fixed order so that an independent reference gets the same bits: element (i, j) of
    /// after · self is Σ_k after(i, k) · self(k, j) for k = 0…3 added in that order; the bottom row 0 0 0 1.
    pub fn then(&self, after: &Affine) -> Affine {
        let (a, b) = (&after.m, &self.m);
        let mut m = [0.0; 16];
        for i in 0..3 {
            for j in 0..4 {
                m[4 * i + j] =
                    a[4 * i] * b[j] + a[4 * i + 1] * b[4 + j] + a[4 * i + 2] * b[8 + j] + a[4 * i + 3] * b[12 + j];
            }
        }
        m[15] = 1.0;
        Affine { m }
    }

    /// Three numbers from text (an axis, a centre, a move), apart by spaces or commas, a point for the decimal; `what`
    /// names them in the refusal.
    pub fn triple(text: &str, what: &str) -> Result<[f64; 3]> {
        let words: Vec<&str> =
            text.split(|c: char| c.is_whitespace() || matches!(c, ',' | ';')).filter(|w| !w.is_empty()).collect();
        if words.len() != 3 {
            return Err(PcError::new(format!("{what} 3 sayıdır (X Y Z, boşlukla ayrılmış); {} sayı verildi.", words.len())));
        }
        let mut out = [0.0; 3];
        for (slot, word) in out.iter_mut().zip(&words) {
            *slot = word.parse().map_err(|_| {
                PcError::new(format!("{what}: “{word}” bir sayı değil; ondalık için nokta kullanın."))
            })?;
        }
        Ok(out)
    }

    /// The 16 numbers, row-major.
    pub fn matrix(&self) -> [f64; 16] {
        self.m
    }

    /// The inverse map (docs/adr/3500), each number in a fixed order so that an independent reference gets the same
    /// bits: the linear part's cofactors over its determinant (the determinant as `determinant` takes it), row by row;
    /// the translation the negated rows of that inverse times the translation, the three products added in order; the
    /// bottom row 0 0 0 1. A map's determinant is never zero (`new`).
    pub fn inverse(&self) -> Affine {
        let m = &self.m;
        let d = self.determinant();
        let i = [
            (m[5] * m[10] - m[6] * m[9]) / d,
            (m[2] * m[9] - m[1] * m[10]) / d,
            (m[1] * m[6] - m[2] * m[5]) / d,
            (m[6] * m[8] - m[4] * m[10]) / d,
            (m[0] * m[10] - m[2] * m[8]) / d,
            (m[2] * m[4] - m[0] * m[6]) / d,
            (m[4] * m[9] - m[5] * m[8]) / d,
            (m[1] * m[8] - m[0] * m[9]) / d,
            (m[0] * m[5] - m[1] * m[4]) / d,
        ];
        let t = |r: usize| -(i[3 * r] * m[3] + i[3 * r + 1] * m[7] + i[3 * r + 2] * m[11]);
        Affine {
            m: [
                i[0], i[1], i[2], t(0), //
                i[3], i[4], i[5], t(1), //
                i[6], i[7], i[8], t(2), //
                0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    /// The 16 numbers as text, row-major, each as Rust writes an f64 (the shortest that reads back the same, no
    /// exponent; a negative zero as 0), apart by spaces: the object's transform history keeps it (docs/adr/3500).
    pub fn text(&self) -> String {
        self.m.iter().map(|v| format!("{}", v + 0.0)).collect::<Vec<_>>().join(" ")
    }

    /// A point moved: each coordinate its row's three products and the translation, added in that order.
    #[inline]
    pub fn apply(&self, p: [f64; 3]) -> [f64; 3] {
        let m = &self.m;
        [
            m[0] * p[0] + m[1] * p[1] + m[2] * p[2] + m[3],
            m[4] * p[0] + m[5] * p[1] + m[6] * p[2] + m[7],
            m[8] * p[0] + m[9] * p[1] + m[10] * p[2] + m[11],
        ]
    }

    fn determinant(&self) -> f64 {
        let m = &self.m;
        m[0] * (m[5] * m[10] - m[6] * m[9]) - m[1] * (m[4] * m[10] - m[6] * m[8])
            + m[2] * (m[4] * m[9] - m[5] * m[8])
    }

    /// Whether the linear part is a rotation (RᵀR = I within 10⁻⁹, determinant positive): lengths and angles kept.
    pub fn rigid(&self) -> bool {
        let m = &self.m;
        let col = |j: usize| [m[j], m[4 + j], m[8 + j]];
        for i in 0..3 {
            for j in 0..3 {
                let (a, b) = (col(i), col(j));
                let dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
                let want = if i == j { 1.0 } else { 0.0 };
                if (dot - want).abs() > RIGID {
                    return false;
                }
            }
        }
        self.determinant() > 0.0
    }

    /// The rotation's angle about its axis in degrees, for a rigid map: atan2 of the axis vector's length (2 sin θ)
    /// and the trace less one (2 cos θ), exact for small angles too.
    pub fn angle(&self) -> f64 {
        let m = &self.m;
        let (ax, ay, az) = (m[9] - m[6], m[2] - m[8], m[4] - m[1]);
        let s = (ax * ax + ay * ay + az * az).sqrt();
        s.atan2(m[0] + m[5] + m[10] - 1.0) * (180.0 / std::f64::consts::PI)
    }

    /// How far the point `c` goes.
    pub fn shift(&self, c: [f64; 3]) -> f64 {
        let q = self.apply(c);
        let (dx, dy, dz) = (q[0] - c[0], q[1] - c[1], q[2] - c[2]);
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Whether every point of the box `min`–`max`, moved, stays on the grid `scale`, `offset` (32-bit whole
    /// numbers); refused with the axis otherwise.
    pub fn fits(&self, min: [f64; 3], max: [f64; 3], scale: [f64; 3], offset: [f64; 3]) -> Result<()> {
        for k in 0..8 {
            let pick = |bit: usize, a: usize| if k & bit == 0 { min[a] } else { max[a] };
            let q = self.apply([pick(1, 0), pick(2, 1), pick(4, 2)]);
            for a in 0..3 {
                if whole(q[a], scale[a], offset[a]).is_none() {
                    return Err(PcError::new(format!(
                        "Dönüşen bulut dosyanın ızgarasına sığmıyor: {} ekseninde kayıtların 32 bitlik tam sayılarına \
                         sığmayacak kadar uzağa gidiyor. Matrisin ötelemesini denetleyin.",
                        AXES[a]
                    )));
                }
            }
        }
        Ok(())
    }

    /// Each record of `records` (the layout's) moved: its X, Y and Z from the grid `scale`, `offset`, moved, and back
    /// on the same grid rounded half away from zero. A point leaving the grid's reach is refused.
    pub fn move_records(
        &self,
        layout: &Layout,
        records: &mut [u8],
        scale: [f64; 3],
        offset: [f64; 3],
    ) -> Result<()> {
        if layout.len < 12 {
            return Err(PcError::new("Nokta kaydı koordinatlarını taşıyamayacak kadar kısa."));
        }
        for r in records.chunks_exact_mut(layout.len) {
            let p = [
                f64::from(layout.x(r)) * scale[0] + offset[0],
                f64::from(layout.y(r)) * scale[1] + offset[1],
                f64::from(layout.z(r)) * scale[2] + offset[2],
            ];
            let q = self.apply(p);
            let mut ints = [0i32; 3];
            for k in 0..3 {
                ints[k] = whole(q[k], scale[k], offset[k]).ok_or_else(|| {
                    PcError::new(format!(
                        "Dönüşen bir nokta {} ekseninde dosyanın ızgarasına sığmıyor; matrisi denetleyin.",
                        AXES[k]
                    ))
                })?;
            }
            layout.set_xyz(r, ints[0], ints[1], ints[2]);
        }
        Ok(())
    }
}

/// A coordinate's whole number on the grid, rounded half away from zero; none outside 32 bits.
fn whole(v: f64, scale: f64, offset: f64) -> Option<i32> {
    let n = ((v - offset) / scale).round();
    (n >= f64::from(i32::MIN) && n <= f64::from(i32::MAX)).then_some(n as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// docs/adr/4300: a quarter turn about Z about the centre (10, 0, 0) takes (11, 0, 0) to (10, 1, 0); the axis is
    /// read for its direction only; a zero axis and two numbers are refused.
    #[test]
    fn an_axis_and_an_angle_make_the_map() {
        let a = Affine::axis_angle([0.0, 0.0, 2.0], 90.0, [10.0, 0.0, 0.0], [0.0, 0.0, 5.0]).unwrap();
        let p = a.apply([11.0, 0.0, 0.0]);
        assert!((p[0] - 10.0).abs() < 1e-12 && (p[1] - 1.0).abs() < 1e-12 && (p[2] - 5.0).abs() < 1e-12, "{p:?}");
        assert!(Affine::axis_angle([0.0; 3], 10.0, [0.0; 3], [0.0; 3]).is_err());
        assert!(Affine::triple("1 2", "Eksen").is_err());
        assert_eq!(Affine::triple("0, 0, 1", "Eksen").unwrap(), [0.0, 0.0, 1.0]);
    }

    /// docs/adr/5100: each angle alone is a right-handed quarter turn about its axis (yaw +X → +Y, pitch +Z → +X,
    /// roll +Y → +Z); the three together turn about Z first, then the turned Y, then the twice turned X; a map then
    /// another is their product.
    #[test]
    fn euler_angles_make_the_map() {
        let near = |p: [f64; 3], q: [f64; 3]| p.iter().zip(&q).all(|(a, b)| (a - b).abs() < 1e-12);
        let o = [0.0; 3];
        assert!(near(Affine::euler(90.0, 0.0, 0.0, o, o).unwrap().apply([1.0, 0.0, 0.0]), [0.0, 1.0, 0.0]));
        assert!(near(Affine::euler(0.0, 90.0, 0.0, o, o).unwrap().apply([0.0, 0.0, 1.0]), [1.0, 0.0, 0.0]));
        assert!(near(Affine::euler(0.0, 0.0, 90.0, o, o).unwrap().apply([0.0, 1.0, 0.0]), [0.0, 0.0, 1.0]));
        // Z then Y′ then X″: the same as the roll first, then the pitch, then the yaw about the fixed axes.
        let all = Affine::euler(30.0, -20.0, 10.0, [5.0, 6.0, 7.0], [1.0, 2.0, 3.0]).unwrap();
        let fixed = Affine::euler(0.0, 0.0, 10.0, [5.0, 6.0, 7.0], o)
            .unwrap()
            .then(&Affine::euler(0.0, -20.0, 0.0, [5.0, 6.0, 7.0], o).unwrap())
            .then(&Affine::euler(30.0, 0.0, 0.0, [5.0, 6.0, 7.0], [1.0, 2.0, 3.0]).unwrap());
        let p = [8.0, -3.0, 2.5];
        assert!(near(all.apply(p), fixed.apply(p)), "{:?} {:?}", all.apply(p), fixed.apply(p));
        assert!(all.rigid());
        assert!(Affine::euler(f64::NAN, 0.0, 0.0, o, o).is_err());
    }

    /// docs/adr/3500: a map and its inverse give the point back (within rounding); a translation's inverse is exactly
    /// its negation; the inverse of the inverse is the map within rounding.
    #[test]
    fn the_inverse_takes_the_points_back() {
        let a = Affine::new([
            0.999998, -0.002, 0.0, 8812.25, //
            0.002, 0.999998, 0.0, -973.5, //
            0.0, 0.0, 1.0, 0.05, //
            0.0, 0.0, 0.0, 1.0,
        ])
        .expect("a map");
        let p = [487123.25, 4420456.5, 852.125];
        let back = a.inverse().apply(a.apply(p));
        assert!((0..3).all(|k| (back[k] - p[k]).abs() < 1e-6), "{back:?}");
        let moved = Affine::new([1.0, 0.0, 0.0, 12.5, 0.0, 1.0, 0.0, -3.25, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]).expect("a map");
        assert_eq!(moved.inverse().matrix(), [1.0, 0.0, 0.0, -12.5, 0.0, 1.0, 0.0, 3.25, 0.0, 0.0, 1.0, -0.0, 0.0, 0.0, 0.0, 1.0]);
        assert_eq!(moved.inverse().text(), "1 0 0 -12.5 0 1 0 3.25 0 0 1 0 0 0 0 1");
        let twice = a.inverse().inverse().matrix();
        assert!(twice.iter().zip(a.matrix()).all(|(x, y)| (x - y).abs() < 1e-9));
    }

    const ID: [f64; 16] = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];

    fn turn_z(t: f64, by: [f64; 3]) -> Affine {
        let (s, c) = t.sin_cos();
        Affine::new([c, -s, 0.0, by[0], s, c, 0.0, by[1], 0.0, 0.0, 1.0, by[2], 0.0, 0.0, 0.0, 1.0]).expect("affine")
    }

    #[test]
    fn the_text_reads_row_by_row_with_any_separator() {
        let a = Affine::parse("[1, 0, 0, 2.5;\n 0 1 0 -3\n0\t0 1 0.25\n0 0 0 1]").expect("reads");
        assert_eq!(a.matrix()[3], 2.5);
        assert_eq!(a.matrix()[7], -3.0);
        assert_eq!(a.matrix()[11], 0.25);
        assert_eq!(a.apply([1.0, 2.0, 3.0]), [3.5, -1.0, 3.25]);
    }

    #[test]
    fn a_wrong_matrix_says_why() {
        let words = |t: &str| Affine::parse(t).expect_err("refused").0;
        assert!(words("1 0 0 0 0 1 0 0 0 0 1 0").contains("12 sayı verildi"));
        // A decimal comma splits a number in two.
        assert!(words("1 0 0 0,5 0 1 0 0 0 0 1 0 0 0 0 1").contains("17 sayı verildi"));
        assert!(words("1 0 0 a 0 1 0 0 0 0 1 0 0 0 0 1").contains("“a” bir sayı değil"));
        assert!(words("1 0 0 0 0 1 0 0 0 0 1 0 0 0 0.001 1").contains("son satırı 0 0 0 1"));
        assert!(words("1 0 0 0 0 1 0 0 0 0 0 0 0 0 0 1").contains("tekil"));
        assert!(words("1 0 0 inf 0 1 0 0 0 0 1 0 0 0 0 1").contains("sonlu"));
    }

    #[test]
    fn a_rotation_is_rigid_and_says_its_angle() {
        let a = turn_z(0.002, [0.35, -0.2, 0.05]);
        assert!(a.rigid());
        assert!((a.angle() - 0.002_f64.to_degrees()).abs() < 1e-12, "{}", a.angle());
        assert!(Affine::new(ID).expect("id").rigid());
        assert_eq!(Affine::new(ID).expect("id").angle(), 0.0);
        let mut scaled = ID;
        scaled[0] = 1.0001;
        assert!(!Affine::new(scaled).expect("affine").rigid());
        let mut mirror = ID;
        mirror[0] = -1.0;
        assert!(!Affine::new(mirror).expect("affine").rigid(), "a mirror keeps lengths but is not a rotation");
    }

    #[test]
    fn the_shift_is_the_point_s_way() {
        let a = turn_z(0.0, [3.0, 4.0, 0.0]);
        assert_eq!(a.shift([100.0, 200.0, 5.0]), 5.0);
    }

    #[test]
    fn a_box_leaving_the_grid_is_refused_with_its_axis() {
        let far = turn_z(0.0, [0.0, 3.0e6, 0.0]);
        let why = far.fits([0.0; 3], [10.0; 3], [0.001; 3], [0.0; 3]).expect_err("too far").0;
        assert!(why.contains("Y ekseninde"), "{why}");
        assert!(turn_z(0.001, [5.0, 5.0, 0.0]).fits([0.0; 3], [10.0; 3], [0.001; 3], [0.0; 3]).is_ok());
    }

    #[test]
    fn records_move_on_their_grid_half_away_from_zero() {
        let layout = Layout::new(0, 20);
        let mut rec = vec![0u8; 40];
        layout.set_xyz(&mut rec[..20], 1000, 2000, 300);
        layout.set_xyz(&mut rec[20..], -1000, 5, 0);
        rec[12] = 77; // intensity stays
        let (scale, offset) = ([0.01; 3], [100.0, 200.0, 0.0]);
        let a = turn_z(0.0, [0.006, -0.004, 0.0]);
        a.move_records(&layout, &mut rec, scale, offset).expect("moves");
        // (1000.6 → 1001, 2000 − 0.4 → 2000), (−1000 + 0.6 → −999, 5 − 0.4 → 5).
        assert_eq!((layout.x(&rec[..20]), layout.y(&rec[..20]), layout.z(&rec[..20])), (1001, 2000, 300));
        assert_eq!((layout.x(&rec[20..]), layout.y(&rec[20..])), (-999, 5));
        assert_eq!(rec[12], 77);
        // A quarter turn about the origin: (x, y) → (−y, x).
        let mut one = vec![0u8; 20];
        layout.set_xyz(&mut one, 300, 400, 7);
        turn_z(std::f64::consts::FRAC_PI_2, [0.0; 3]).move_records(&layout, &mut one, [1.0; 3], [0.0; 3]).expect("moves");
        assert_eq!((layout.x(&one), layout.y(&one), layout.z(&one)), (-400, 300, 7));
    }
}
