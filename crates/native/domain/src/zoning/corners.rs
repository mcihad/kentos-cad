//! Cephe köşesi (docs/adr/8097): two rooms' contours (each to its walls' outer faces, less the other's axis outline,
//! docs/adr/8069) both hold the end of the wall between them where it runs into the façade wall: the corner. The ratios
//! halve its area between the two rooms (docs/adr/8079 §6); the shapes the model's emsal zones are made of split it at
//! the wall between them, each room keeping its own side, so that a zone's area is its rooms' gross.
//!
//! The wall between them is the line halfway between the two rooms' net outlines at the corner: the perpendicular
//! bisector of the two points of the net outlines nearest the corner's middle (for a straight wall, its axis).

use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::geom::region;
use kentos_geometry_core::vec2::Vec2;

/// The point of a polyline ring's edges nearest `p`.
fn nearest_on(ring: &[Vec2], p: Vec2) -> Option<(Vec2, f64)> {
    let n = ring.len();
    let mut best: Option<(Vec2, f64)> = None;
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let len2 = dx * dx + dy * dy;
        let t = if len2 > 0.0 { (((p.x - a.x) * dx + (p.y - a.y) * dy) / len2).clamp(0.0, 1.0) } else { 0.0 };
        let q = Vec2::new(a.x + dx * t, a.y + dy * t);
        let d = (q.x - p.x).powi(2) + (q.y - p.y).powi(2);
        if best.is_none_or(|(_, e)| d < e) {
            best = Some((q, d));
        }
    }
    best
}

/// The point of an outline's rings nearest `p`.
fn nearest(areas: &[Area], p: Vec2) -> Option<Vec2> {
    areas
        .iter()
        .flat_map(|a| std::iter::once(&a.outer).chain(&a.holes))
        .filter_map(|r| nearest_on(&kentos_geometry_core::entity::polygon_ring(&r.pts, r.bulges.as_deref()), p))
        .min_by(|x, y| x.1.total_cmp(&y.1))
        .map(|(q, _)| q)
}

/// The box's middle and its diagonal.
fn middle(areas: &[Area]) -> Option<(Vec2, f64)> {
    let mut b = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    for a in areas {
        for p in &a.outer.pts {
            b = [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)];
        }
    }
    b[0].is_finite().then(|| {
        let diag = ((b[2] - b[0]).powi(2) + (b[3] - b[1]).powi(2)).sqrt();
        (Vec2::new((b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0), diag)
    })
}

/// The corner `both` two rooms' contours share, split at the wall between them: the part on the first room's side and
/// the part on the second's, by their net outlines `first` and `second`. None when the sides are not found (no net
/// outline, the two nearest points one).
pub(super) fn split(both: &[Area], first: &[Area], second: &[Area]) -> Option<(Vec<Area>, Vec<Area>)> {
    let (c, diag) = middle(both)?;
    let (p, q) = (nearest(first, c)?, nearest(second, c)?);
    let (dx, dy) = (q.x - p.x, q.y - p.y);
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-9 {
        return None;
    }
    // The half-plane on the first room's side of the bisector, as a square far larger than the corner.
    let (d, u) = (Vec2::new(dx / len, dy / len), Vec2::new(-dy / len, dx / len));
    let m = Vec2::new((p.x + q.x) / 2.0, (p.y + q.y) / 2.0);
    let r = 10.0 * (diag + len) + 1.0;
    let at = |s: f64, t: f64| Vec2::new(m.x + u.x * s - d.x * t, m.y + u.y * s - d.y * t);
    let half = Area {
        outer: Ring { pts: vec![at(-r, 0.0), at(r, 0.0), at(r, r), at(-r, r)], bulges: None },
        holes: Vec::new(),
    };
    let own = region::intersect_area_sets(&[both.to_vec(), vec![half.clone()]]);
    let other = region::subtract_areas(both, &[half]);
    Some((own, other))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Area {
        Area {
            outer: Ring {
                pts: vec![Vec2::new(x0, y0), Vec2::new(x1, y0), Vec2::new(x1, y1), Vec2::new(x0, y1)],
                bulges: None,
            },
            holes: Vec::new(),
        }
    }

    /// A 40 cm wall on x = 5 runs into a 50 cm façade wall (y 0–0.5): the corner the two rooms' contours share is the
    /// wall's end within the façade wall's outer half, 0.40 × 0.25 m; split at x = 5, 0.05 m² each.
    #[test]
    fn a_shared_facade_corner_is_split_at_the_wall_between_the_rooms() {
        let both = [rect(4.8, 0.0, 5.2, 0.25)];
        let (left, right) = (rect(0.5, 0.5, 4.8, 5.5), rect(5.2, 0.5, 9.5, 5.5));
        let (own, other) = split(&both, &[left], &[right]).expect("split");
        let area = |a: &[Area]| a.iter().map(region::net_area).sum::<f64>();
        assert!((area(&own) - 0.05).abs() < 1e-12 && (area(&other) - 0.05).abs() < 1e-12);
        let xs: Vec<f64> = own.iter().flat_map(|a| a.outer.pts.iter().map(|p| p.x)).collect();
        assert!(xs.iter().all(|x| *x <= 5.0 + 1e-12), "the left room's side: {xs:?}");
        assert!(split(&both, &[], &[rect(5.2, 0.5, 9.5, 5.5)]).is_none());
    }
}
