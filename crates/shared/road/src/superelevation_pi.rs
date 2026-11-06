//! A curve's superelevation as a plan's PI data writes it (plan/road.md Y77, docs/adr/0983): the rate S, the widening
//! Gn and, at each end, the transition's lengths Lr, Lt and La.
//!
//! KGM 2019 Kesin ve Ön Projeler Teknik Şartnamesi (s. 5) lists them among a curve's data: "uygulanan dever (S), Şerit
//! Genişleme miktarı (Gn), rakordman bilgileri (Lr, Lt, La)". Their meaning is Netcad's (Güzergâh editörü, some
//! bilgileri): Lt is "aliyman çatı eğiminden (%-2), %0 devere kadar olan mesafe", Lr "%0 deverden max. devere ulaşmak
//! için gerekli olan mesafe", La the stretch applied on the tangent. AASHTO calls the first two the tangent runout and
//! the superelevation runoff. KGM's sample sheet (EK-10, S1: S=6.00, Lr=76.800-76.800, Lt=25.600-25.600,
//! La=25.600-25.600) agrees: Lt = Lr·2/6, and the runoff lies on the clothoid, so the tangent holds the runout alone.
//!
//! The lengths are read from the schedule's break points, whatever wrote them (curves or rows): on the curve's
//! outside, walking from the circular part's middle toward the end, the full rate (FS) is where the slope first drops
//! below its value there, the level crown (LC) where it crosses 0, the normal crown (NC) where it reaches the normal
//! slope. Lr = |FS − LC|, Lt = |LC − NC|, La = how far NC lies outside the curve (TS, or ST on the way out), 0 when
//! it lies on it.

use crate::alignment::CurveReport;
use crate::superelevation::{Crossfall, Schedule};
use crate::superelevation_physics::toward_centre;

const EPS: f64 = 1e-9;

/// One end of a curve's transition (metres).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransitionEnd {
    /// Lr: the level crown to the full rate.
    pub runoff: f64,
    /// Lt: the normal crown to the level crown.
    pub runout: f64,
    /// La: the stretch of the transition beyond the curve (before TS entering, after ST leaving); 0 when it starts on
    /// the curve.
    pub on_tangent: f64,
}

/// A curve's superelevation as its PI data writes it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CurveSuper {
    /// S: the slope toward the centre at the circular part's middle (a grade; the lesser side's), none when the
    /// schedule does not bank the curve there (an adverse or level crown).
    pub rate: Option<f64>,
    /// Gn: the inside's extra width there (m).
    pub widening: f64,
    /// The way in and the way out; none where the outside's slopes do not run from the normal crown through level to
    /// the full rate (a schedule that starts or ends inside the transition, a normal slope already toward the centre).
    pub entry: Option<TransitionEnd>,
    pub exit: Option<TransitionEnd>,
}

/// The value of piecewise linear break points at `s`, the end value beyond them (as the schedule reads it).
fn along(points: &[[f64; 2]], s: f64, none: f64) -> f64 {
    let (Some(first), Some(last)) = (points.first(), points.last()) else { return none };
    if s <= first[0] {
        return first[1];
    }
    if s >= last[0] {
        return last[1];
    }
    let i = points.partition_point(|p| p[0] <= s);
    let (a, b) = (points[i - 1], points[i]);
    if b[0] == a[0] { b[1] } else { a[1] + (b[1] - a[1]) * (s - a[0]) / (b[0] - a[0]) }
}

/// Where the walk first meets `v` going from `from` on (stations and values in walking order): the station, by linear
/// interpolation on the segment that brackets it.
fn first_at(walk: &[[f64; 2]], from: usize, v: f64, falling: bool) -> Option<(usize, f64)> {
    for i in from..walk.len().saturating_sub(1) {
        let (a, b) = (walk[i], walk[i + 1]);
        let met = if falling { a[1] >= v - EPS && b[1] <= v + EPS } else { a[1] <= v + EPS && b[1] >= v - EPS };
        if met {
            let s = if (b[1] - a[1]).abs() < EPS { a[0] } else { a[0] + (v - a[1]) * (b[0] - a[0]) / (b[1] - a[1]) };
            return Some((i, s));
        }
    }
    None
}

/// One end: `dir` −1 entering (walking back from the middle), +1 leaving; `edge` the curve's TS or ST.
fn end(points: &[[f64; 2]], normal: f64, mid: f64, dir: f64, edge: f64) -> Option<TransitionEnd> {
    if points.is_empty() || normal >= 0.0 {
        return None;
    }
    let full = along(points, mid, normal);
    if full <= EPS {
        return None;
    }
    let mut walk = vec![[mid, full]];
    let mut beyond: Vec<[f64; 2]> = points.iter().copied().filter(|p| dir * (p[0] - mid) > EPS).collect();
    if dir < 0.0 {
        beyond.reverse();
    }
    walk.extend(beyond);
    // The full rate: the last point of the plateau from the middle.
    let mut i = 0;
    while i + 1 < walk.len() && walk[i + 1][1] >= full - EPS {
        i += 1;
    }
    let fs = walk[i][0];
    let (j, lc) = first_at(&walk, i, 0.0, true)?;
    // The level crown's station starts the rest of the walk.
    let mut rest = vec![[lc, 0.0]];
    rest.extend_from_slice(&walk[j + 1..]);
    let (_, nc) = first_at(&rest, 0, normal, true)?;
    Some(TransitionEnd { runoff: (fs - lc).abs(), runout: (lc - nc).abs(), on_tangent: (dir * (nc - edge)).max(0.0) })
}

/// The curve's superelevation from a schedule built on its alignment with `normal` slopes.
pub fn curve_super(schedule: &Schedule, normal: Crossfall, curve: &CurveReport) -> CurveSuper {
    let left_turn = curve.delta > 0.0;
    let mid = 0.5 * (curve.stations[1] + curve.stations[2]);
    let here = schedule.at(mid, normal);
    let rate = toward_centre(here.left, here.right, left_turn);
    let (wl, wr) = schedule.widening(mid);
    // Turning left the centre is on the left: the right side is outside, the left inside.
    let (outside, n_out, widening) = if left_turn {
        (&schedule.right, normal.right, wl)
    } else {
        (&schedule.left, normal.left, wr)
    };
    CurveSuper {
        rate: (rate > EPS).then_some(rate),
        widening,
        entry: end(outside, n_out, mid, -1.0, curve.stations[0]),
        exit: end(outside, n_out, mid, 1.0, curve.stations[3]),
    }
}
