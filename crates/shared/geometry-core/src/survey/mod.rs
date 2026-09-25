//! Surveying computations, Netcad's "Hesap" menu: traverses (poligon),
//! polar surveys from a station (kutupsal alım), stake-out values
//! (aplikasyon), forward and backward intersections (önden ve geriden
//! kestirme). Conventions of Turkish field notes: Y is east (`x`), X north
//! (`y`); a bearing (semt) runs clockwise from north; horizontal angles
//! and direction readings run clockwise. Angles come and go in the
//! project's unit (grad or degrees), distances in metres.

pub mod intersection;
pub mod polar;
pub mod traverse;

use crate::api::Op;
use crate::jsmath::{PI, TAU, atan2, js_hypot};
use crate::vec2::Vec2;

/// The project's angle unit: `"grad"` (400 to a turn) or `"deg"` (360).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Unit {
    full: f64,
}

impl Unit {
    pub const GRAD: Unit = Unit { full: 400.0 };
    pub const DEG: Unit = Unit { full: 360.0 };

    pub fn parse(s: &str) -> Result<Unit, String> {
        match s {
            "grad" => Ok(Unit::GRAD),
            "deg" => Ok(Unit::DEG),
            _ => Err(format!("Açı birimi “{s}” tanınmıyor (grad ya da deg).")),
        }
    }

    /// An angle in this unit, in radians.
    pub fn rad(self, v: f64) -> f64 {
        v * TAU / self.full
    }

    /// Radians in this unit.
    pub fn of(self, r: f64) -> f64 {
        r * self.full / TAU
    }
}

/// An angle in [0, 2π).
pub fn positive(r: f64) -> f64 {
    let m = r % TAU;
    if m < 0.0 { m + TAU } else { m }
}

/// An angle in (−π, π].
pub fn signed(r: f64) -> f64 {
    let m = positive(r);
    if m > PI { m - TAU } else { m }
}

/// Bearing (semt) from `a` to `b` in radians, clockwise from north, in [0, 2π).
pub fn bearing(a: Vec2, b: Vec2) -> f64 {
    positive(atan2(b.x - a.x, b.y - a.y))
}

/// Horizontal distance from `a` to `b`.
pub fn distance(a: Vec2, b: Vec2) -> f64 {
    js_hypot(b.x - a.x, b.y - a.y)
}

/// The point at `d` from `a` along the bearing `t` (radians): the first fundamental task.
pub fn from_bearing(a: Vec2, t: f64, d: f64) -> Vec2 {
    Vec2::new(
        a.x + d * crate::jsmath::sin(t),
        a.y + d * crate::jsmath::cos(t),
    )
}

/// A known point that must differ from another, or a message naming both.
fn distinct(a: Vec2, b: Vec2, what: &str) -> Result<(), String> {
    if !(distance(a, b) > 0.0) {
        return Err(format!("{what} aynı yerde; doğrultu tanımsız."));
    }
    Ok(())
}

/// A finite number, or a message naming it.
fn finite(v: f64, what: &str) -> Result<f64, String> {
    if v.is_finite() {
        Ok(v)
    } else {
        Err(format!("{what} bir sayı değil."))
    }
}

pub(crate) static OPS: &[Op] = &[
    crate::survey::traverse::OP,
    crate::survey::polar::POLAR_OP,
    crate::survey::polar::STAKEOUT_OP,
    crate::survey::intersection::FORWARD_OP,
    crate::survey::intersection::RESECTION_OP,
];

#[cfg(test)]
mod tests;
