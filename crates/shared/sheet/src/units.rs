//! Paper units (design §2): lengths in whole micrometres (`Um`), angles in
//! thousandths of a degree (`Mdeg`), the page's origin at its top-left corner
//! with y growing downwards, rotation clockwise as on a screen. Every result
//! is rounded to a whole micrometre by one rule, a half away from zero; the
//! transcendental functions come from `libm`, so native and WASM agree bit
//! for bit.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// A paper length in micrometres (±2 147 m).
pub type Um = i32;
/// An angle in thousandths of a degree; clockwise is positive.
pub type Mdeg = i32;
/// A point on the paper: `[left, top]` in micrometres.
pub type PointUm = [Um; 2];

pub const UM_PER_MM: Um = 1000;
pub const FULL_TURN: Mdeg = 360_000;
/// 1 pt (1/72 in) in micrometres, rounded: what the interface shows as “pt”.
pub const UM_PER_PT: f64 = 352.777_777_777_777_8;

/// A rectangle on the paper: its top-left corner and size, before rotation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct RectUm {
    pub left: Um,
    pub top: Um,
    pub width: Um,
    pub height: Um,
}

impl RectUm {
    pub const fn new(left: Um, top: Um, width: Um, height: Um) -> RectUm {
        RectUm {
            left,
            top,
            width,
            height,
        }
    }

    /// A rectangle from two corners in any order (saturated to `Um`).
    pub fn from_edges(left: i64, top: i64, right: i64, bottom: i64) -> RectUm {
        let (l, r) = if left <= right {
            (left, right)
        } else {
            (right, left)
        };
        let (t, b) = if top <= bottom {
            (top, bottom)
        } else {
            (bottom, top)
        };
        RectUm::new(sat(l), sat(t), sat(r - l), sat(b - t))
    }

    pub fn right(&self) -> i64 {
        i64::from(self.left) + i64::from(self.width)
    }

    pub fn bottom(&self) -> i64 {
        i64::from(self.top) + i64::from(self.height)
    }

    /// The centre, which may fall on a half micrometre.
    pub fn center(&self) -> [f64; 2] {
        [
            f64::from(self.left) + f64::from(self.width) / 2.0,
            f64::from(self.top) + f64::from(self.height) / 2.0,
        ]
    }

    /// The rectangle grown by `d` on every side (shrunk when negative; never below zero size).
    pub fn inset(&self, d: Um) -> RectUm {
        self.inset_sides(d, d, d, d)
    }

    pub fn inset_sides(&self, top: Um, right: Um, bottom: Um, left: Um) -> RectUm {
        let l = i64::from(self.left) + i64::from(left);
        let t = i64::from(self.top) + i64::from(top);
        let w = (i64::from(self.width) - i64::from(left) - i64::from(right)).max(0);
        let h = (i64::from(self.height) - i64::from(top) - i64::from(bottom)).max(0);
        RectUm::new(sat(l), sat(t), sat(w), sat(h))
    }

    pub fn translate(&self, dx: i64, dy: i64) -> RectUm {
        RectUm::new(
            sat(i64::from(self.left) + dx),
            sat(i64::from(self.top) + dy),
            self.width,
            self.height,
        )
    }

    /// Whether `p` is inside or on the edge.
    pub fn contains(&self, p: [f64; 2]) -> bool {
        p[0] >= f64::from(self.left)
            && p[0] <= self.right() as f64
            && p[1] >= f64::from(self.top)
            && p[1] <= self.bottom() as f64
    }

    /// Whether `o` lies wholly inside this one.
    pub fn contains_rect(&self, o: &RectUm) -> bool {
        o.left >= self.left
            && o.top >= self.top
            && o.right() <= self.right()
            && o.bottom() <= self.bottom()
    }

    pub fn intersects(&self, o: &RectUm) -> bool {
        i64::from(self.left) < o.right()
            && i64::from(o.left) < self.right()
            && i64::from(self.top) < o.bottom()
            && i64::from(o.top) < self.bottom()
    }

    /// The smallest rectangle holding both.
    pub fn union(&self, o: &RectUm) -> RectUm {
        RectUm::from_edges(
            i64::from(self.left.min(o.left)),
            i64::from(self.top.min(o.top)),
            self.right().max(o.right()),
            self.bottom().max(o.bottom()),
        )
    }

    pub fn area(&self) -> i64 {
        i64::from(self.width.max(0)) * i64::from(self.height.max(0))
    }
}

/// A paper size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SizeUm {
    pub width: Um,
    pub height: Um,
}

/// Empty space on each side of the page; the printable area is inside it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct Margins {
    pub top: Um,
    pub right: Um,
    pub bottom: Um,
    pub left: Um,
}

/// `x` saturated into `Um`.
pub fn sat(x: i64) -> Um {
    x.clamp(i64::from(Um::MIN), i64::from(Um::MAX)) as Um
}

/// `x` micrometres rounded to a whole one, a half away from zero (the one
/// rounding rule of the sheet, design §3.2); NaN is 0, the rest saturates.
pub fn round_um(x: f64) -> Um {
    if x.is_nan() {
        return 0;
    }
    let r = libm::round(x);
    if r >= f64::from(Um::MAX) {
        Um::MAX
    } else if r <= f64::from(Um::MIN) {
        Um::MIN
    } else {
        r as Um
    }
}

/// `x` rounded to a whole number, a half away from zero, as an `i64` (saturated).
pub fn round_i64(x: f64) -> i64 {
    if x.is_nan() {
        return 0;
    }
    let r = libm::round(x);
    if r >= 9.0e18 {
        9_000_000_000_000_000_000
    } else if r <= -9.0e18 {
        -9_000_000_000_000_000_000
    } else {
        r as i64
    }
}

/// An angle brought into `[0, 360 000)`.
pub fn norm_mdeg(a: i64) -> Mdeg {
    a.rem_euclid(i64::from(FULL_TURN)) as Mdeg
}

/// Sine and cosine of an angle in thousandths of a degree; exact at the quarter turns.
pub fn sin_cos(a: Mdeg) -> (f64, f64) {
    match norm_mdeg(i64::from(a)) {
        0 => (0.0, 1.0),
        90_000 => (1.0, 0.0),
        180_000 => (0.0, -1.0),
        270_000 => (-1.0, 0.0),
        n => {
            let r = f64::from(n) / 1000.0 * core::f64::consts::PI / 180.0;
            (libm::sin(r), libm::cos(r))
        }
    }
}

/// `p` turned by `a` (clockwise on the paper) around `c`.
pub fn rotate(p: [f64; 2], c: [f64; 2], a: Mdeg) -> [f64; 2] {
    if norm_mdeg(i64::from(a)) == 0 {
        return p;
    }
    let (s, k) = sin_cos(a);
    let (dx, dy) = (p[0] - c[0], p[1] - c[1]);
    [c[0] + dx * k - dy * s, c[1] + dx * s + dy * k]
}

/// The four corners of `r` turned by `a` around its centre: top-left, top-right, bottom-right, bottom-left.
pub fn corners(r: &RectUm, a: Mdeg) -> [[f64; 2]; 4] {
    let c = r.center();
    let (l, t) = (f64::from(r.left), f64::from(r.top));
    let (rr, b) = (r.right() as f64, r.bottom() as f64);
    [[l, t], [rr, t], [rr, b], [l, b]].map(|p| rotate(p, c, a))
}

/// The upright box around `r` turned by `a` around its centre, rounded outwards to whole micrometres.
pub fn rotated_bounds(r: &RectUm, a: Mdeg) -> RectUm {
    if norm_mdeg(i64::from(a)) == 0 {
        return *r;
    }
    let cs = corners(r, a);
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for p in cs {
        x0 = x0.min(p[0]);
        y0 = y0.min(p[1]);
        x1 = x1.max(p[0]);
        y1 = y1.max(p[1]);
    }
    // A corner a rounding error away from a whole micrometre stays on it.
    let snap = |v: f64| {
        let r = libm::round(v);
        if (v - r).abs() < 1e-6 { r } else { v }
    };
    RectUm::from_edges(
        libm::floor(snap(x0)) as i64,
        libm::floor(snap(y0)) as i64,
        libm::ceil(snap(x1)) as i64,
        libm::ceil(snap(y1)) as i64,
    )
}

/// A length written in millimetres with up to three decimals, exactly (12 345 → "12.345", 2 000 → "2").
pub fn mm_text(um: i64) -> String {
    thousandths_text(um)
}

/// A whole number of thousandths written exactly with up to three decimals (an angle in `Mdeg` as degrees).
pub fn thousandths_text(v: i64) -> String {
    let neg = v < 0;
    let a = v.unsigned_abs();
    let (whole, frac) = (a / 1000, a % 1000);
    let mut s = String::new();
    if neg && a != 0 {
        s.push('-');
    }
    s.push_str(&whole.to_string());
    if frac != 0 {
        let f = format!("{frac:03}");
        s.push('.');
        s.push_str(f.trim_end_matches('0'));
    }
    s
}

/// A length in millimetres with one decimal and the unit, as the interface's distance badges show it (“12.5 mm”).
pub fn mm1_text(um: i64) -> String {
    // Tenths of a millimetre, a half away from zero.
    let tenths = if um >= 0 {
        (um + 50) / 100
    } else {
        (um - 50) / 100
    };
    let neg = tenths < 0;
    let a = tenths.unsigned_abs();
    format!("{}{}.{} mm", if neg { "-" } else { "" }, a / 10, a % 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounding_is_a_half_away_from_zero() {
        assert_eq!(round_um(2.5), 3);
        assert_eq!(round_um(-2.5), -3);
        assert_eq!(round_um(2.4999), 2);
        assert_eq!(round_um(f64::NAN), 0);
        assert_eq!(round_um(1e30), Um::MAX);
    }

    #[test]
    fn quarter_turns_are_exact() {
        assert_eq!(rotate([10.0, 0.0], [0.0, 0.0], 90_000), [0.0, 10.0]);
        assert_eq!(rotate([10.0, 0.0], [0.0, 0.0], -90_000), [0.0, -10.0]);
        let r = RectUm::new(0, 0, 100, 40);
        assert_eq!(rotated_bounds(&r, 90_000), RectUm::new(30, -30, 40, 100));
    }

    #[test]
    fn texts_are_exact() {
        assert_eq!(mm_text(12_345), "12.345");
        assert_eq!(mm_text(2_000), "2");
        assert_eq!(mm_text(-500), "-0.5");
        assert_eq!(mm1_text(12_450), "12.5 mm");
        assert_eq!(mm1_text(12_449), "12.4 mm");
        assert_eq!(thousandths_text(90_000), "90");
    }
}
