//! TS 5319's insulation symbol fitted to a cut band (docs/adr/8014, 8071 §8): one zigzag between the band's two long
//! sides, each vee as wide as the band is thick, its points on both sides, clipped to the band's outline. A section draws
//! a cut insulation layer so, as the plan draws a wall's (`geom::wall::FITTED_INSULATION`): the library's rows, tiled at
//! a size of their own, break into pieces in a band thinner than them or sloping across them.

use crate::geometry::point_in_polygon;
use crate::jsmath::{js_floor, js_hypot};
use crate::ops::generate::bounding::convex_hull;
use crate::vec2::Vec2;

/// At most this many points in one band's zigzag: a band longer than that many vees draws its first ones.
const MOST: usize = 4000;

/// The zigzag of a cut band `ring` (closed implicitly): its run along the edge of its hull it is narrowest across, its
/// thickness that narrowest width; points every half its thickness along that side, every other one across on the far
/// side; the line clipped to the ring, in pieces. None for a ring thinner than `least` (TS 5319: the symbol goes by the
/// drawing's scale). A piece shorter than it is thick runs the other way: its narrowest width is then its length.
pub fn fitted(ring: &[Vec2], least: f64) -> Vec<Vec<Vec2>> {
    let hull = convex_hull(ring);
    let n = hull.len();
    if n < 3 {
        return Vec::new();
    }
    // The hull is counter-clockwise: every point is left of each edge, as far as `across` says.
    let mut best: Option<(f64, Vec2, Vec2)> = None;
    for i in 0..n {
        let (a, b) = (hull[i], hull[(i + 1) % n]);
        let l = js_hypot(b.x - a.x, b.y - a.y);
        if !(l > 0.0) {
            continue;
        }
        let u = Vec2::new((b.x - a.x) / l, (b.y - a.y) / l);
        let width = hull
            .iter()
            .map(|p| u.x * (p.y - a.y) - u.y * (p.x - a.x))
            .fold(0.0_f64, |m, w| if w > m { w } else { m });
        if best.is_none_or(|(w, ..)| width < w) {
            best = Some((width, u, a));
        }
    }
    let Some((thick, u, base)) = best else {
        return Vec::new();
    };
    // At the limit it draws (a 5 cm layer at 1:50), the measured thickness a hair under it.
    if !(thick > 1e-9) || thick < least * (1.0 - 1e-9) {
        return Vec::new();
    }
    let along = |p: &Vec2| (p.x - base.x) * u.x + (p.y - base.y) * u.y;
    let (lo, hi) = hull.iter().map(along).fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), s| {
        (if s < lo { s } else { lo }, if s > hi { s } else { hi })
    });
    let across = Vec2::new(-u.y, u.x);
    let step = thick / 2.0;
    // One step past its far end: the clip ends it where the band does.
    let count = (js_floor((hi - lo) / step) as usize + 1).min(MOST);
    let pts: Vec<Vec2> = (0..=count)
        .map(|k| {
            let s = lo + k as f64 * step;
            let p = Vec2::new(base.x + s * u.x, base.y + s * u.y);
            if k % 2 == 1 {
                Vec2::new(p.x + thick * across.x, p.y + thick * across.y)
            } else {
                p
            }
        })
        .collect();
    clip(&pts, ring)
}

/// The pieces of the line `pts` inside `ring` (closed implicitly): each segment split where it crosses the ring's edges,
/// the parts whose middles are inside kept, joined where they meet.
fn clip(pts: &[Vec2], ring: &[Vec2]) -> Vec<Vec<Vec2>> {
    let mut out: Vec<Vec<Vec2>> = Vec::new();
    let mut run: Vec<Vec2> = Vec::new();
    let n = ring.len();
    let at = |a: Vec2, d: Vec2, t: f64| Vec2::new(a.x + d.x * t, a.y + d.y * t);
    for seg in pts.windows(2) {
        let (a, b) = (seg[0], seg[1]);
        let d = Vec2::new(b.x - a.x, b.y - a.y);
        let mut ts = vec![0.0, 1.0];
        for i in 0..n {
            let (p, q) = (ring[i], ring[(i + 1) % n]);
            let e = Vec2::new(q.x - p.x, q.y - p.y);
            let den = d.x * e.y - d.y * e.x;
            if den == 0.0 {
                continue;
            }
            let (px, py) = (p.x - a.x, p.y - a.y);
            let t = (px * e.y - py * e.x) / den;
            let s = (px * d.y - py * d.x) / den;
            if t > 0.0 && t < 1.0 && (0.0..=1.0).contains(&s) {
                ts.push(t);
            }
        }
        ts.sort_by(f64::total_cmp);
        ts.dedup_by(|x, y| (*x - *y).abs() < 1e-12);
        for w in ts.windows(2) {
            let (p0, p1) = (at(a, d, w[0]), at(a, d, w[1]));
            if point_in_polygon(at(a, d, (w[0] + w[1]) / 2.0), ring) {
                if run.last().is_some_and(|l| js_hypot(l.x - p0.x, l.y - p0.y) < 1e-12) {
                    run.push(p1);
                } else {
                    if run.len() >= 2 {
                        out.push(std::mem::take(&mut run));
                    }
                    run = vec![p0, p1];
                }
            } else if run.len() >= 2 {
                out.push(std::mem::take(&mut run));
            } else {
                run.clear();
            }
        }
    }
    if run.len() >= 2 {
        out.push(run);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }

    /// A level band 2 m long, 10 cm thick: one zigzag from its one end to the other, a point every 5 cm along it,
    /// every other one on its top; at 1:50 a 1 mm limit is 5 cm.
    #[test]
    fn a_level_band_is_one_zigzag_between_its_sides() {
        let ring = [p(0.0, 0.0), p(2.0, 0.0), p(2.0, 0.1), p(0.0, 0.1)];
        let z = fitted(&ring, 0.05);
        assert_eq!(z.len(), 1, "{z:?}");
        let line = &z[0];
        assert_eq!(line.len(), 41);
        for (k, q) in line.iter().enumerate() {
            assert!((q.x - 0.05 * k as f64).abs() < 1e-12, "{k}: {q:?}");
            assert!((q.y - if k % 2 == 1 { 0.1 } else { 0.0 }).abs() < 1e-12, "{k}: {q:?}");
        }
    }

    /// A roof's insulation down its fall line (docs/adr/8071 §8): between z = u/3 and 20 cm over it, its ends upright.
    /// Its zigzag's points lie on the two sides by turns, a thickness (0.2 · 3 / √10 across) apart along them, and inside
    /// its ends.
    #[test]
    fn a_sloping_band_s_zigzag_keeps_to_its_sides() {
        let ring = [p(0.0, 0.0), p(3.0, 1.0), p(3.0, 1.2), p(0.0, 0.2)];
        let z = fitted(&ring, 0.05);
        let thick = 0.2 * 3.0 / 10.0_f64.sqrt();
        assert!(!z.is_empty());
        let side = |q: &Vec2| {
            if (q.y - q.x / 3.0).abs() < 1e-9 {
                0
            } else if (q.y - q.x / 3.0 - 0.2).abs() < 1e-9 {
                1
            } else {
                2
            }
        };
        for line in &z {
            for q in line {
                assert!(q.x >= -1e-12 && q.x <= 3.0 + 1e-12, "{q:?} inside its ends");
            }
            // Inside: every point but a clipped end on a side, by turns, the same side's a thickness apart.
            let inner = &line[1..line.len() - 1];
            for w in inner.windows(2) {
                assert_ne!(side(&w[0]), 2, "{:?}", w[0]);
                assert_eq!(side(&w[0]) + side(&w[1]), 1, "{w:?} by turns");
            }
            for w in inner.windows(3) {
                assert!((js_hypot(w[2].x - w[0].x, w[2].y - w[0].y) - thick).abs() < 1e-9, "{w:?}");
            }
        }
        let points: usize = z.iter().map(Vec::len).sum();
        assert!(points as f64 >= (10.0_f64.sqrt() / (thick / 2.0)).floor(), "{points}");
    }

    /// An upright band (a wall's layer cut across): its zigzag runs up it.
    #[test]
    fn an_upright_band_s_zigzag_runs_up_it() {
        let ring = [p(0.0, 0.0), p(0.1, 0.0), p(0.1, 3.0), p(0.0, 3.0)];
        let z = fitted(&ring, 0.05);
        assert_eq!(z.len(), 1);
        assert!(z[0].iter().all(|q| q.x.abs() < 1e-12 || (q.x - 0.1).abs() < 1e-12));
        assert!((z[0].last().unwrap().y - z[0][0].y).abs() > 2.9);
    }

    /// Thinner than the limit, or no band at all: no symbol.
    #[test]
    fn a_thin_band_draws_none() {
        assert!(fitted(&[p(0.0, 0.0), p(2.0, 0.0), p(2.0, 0.04), p(0.0, 0.04)], 0.05).is_empty());
        assert!(fitted(&[p(0.0, 0.0), p(2.0, 0.0)], 0.05).is_empty());
    }
}
