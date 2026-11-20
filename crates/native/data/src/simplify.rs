//! A geometry simplified for drawing (docs/adr/9100 §4), for a driver whose source cannot do it itself: each straight
//! line by Douglas–Peucker (Douglas & Peucker 1973; the iterative form, so a long line cannot overflow the stack)
//! with the request's tolerance in the geometry's units. A line keeps its two ends; a ring stays closed and keeps at
//! least four positions (the first again at the end), so an area never vanishes and is never turned inside out by
//! the simplification alone; arcs and points are kept as they are. Z and M ride with the positions kept.

use crate::geom::{Coord, Curve, Geometry};

/// `g` simplified with `tolerance` (no change below or at zero).
pub fn simplify(g: &Geometry, tolerance: f64) -> Geometry {
    if tolerance.is_nan() || tolerance <= 0.0 {
        return g.clone();
    }
    match g {
        Geometry::Point(_) | Geometry::MultiPoint(_) => g.clone(),
        Geometry::Curve(c) => Geometry::Curve(curve(c, tolerance, false)),
        Geometry::Polygon(rings) => {
            Geometry::Polygon(rings.iter().map(|r| curve(r, tolerance, true)).collect())
        }
        Geometry::MultiCurve(cs) => {
            Geometry::MultiCurve(cs.iter().map(|c| curve(c, tolerance, false)).collect())
        }
        Geometry::MultiPolygon(ps) => Geometry::MultiPolygon(
            ps.iter()
                .map(|rings| rings.iter().map(|r| curve(r, tolerance, true)).collect())
                .collect(),
        ),
        Geometry::Collection(gs) => {
            Geometry::Collection(gs.iter().map(|g| simplify(g, tolerance)).collect())
        }
    }
}

fn curve(c: &Curve, tolerance: f64, ring: bool) -> Curve {
    match c {
        Curve::Line(points) => Curve::Line(line(points, tolerance, ring)),
        Curve::Arc(_) => c.clone(),
        Curve::Compound(parts) => Curve::Compound(
            parts
                .iter()
                .map(|p| match p {
                    Curve::Line(points) => Curve::Line(line(points, tolerance, false)),
                    other => other.clone(),
                })
                .collect(),
        ),
    }
}

/// The distance of `p` from the segment `a`–`b`.
fn distance(p: &Coord, a: &Coord, b: &Coord) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len2 = dx * dx + dy * dy;
    if len2 == 0.0 {
        return ((p.x - a.x).powi(2) + (p.y - a.y).powi(2)).sqrt();
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / len2).clamp(0.0, 1.0);
    let (qx, qy) = (a.x + t * dx, a.y + t * dy);
    ((p.x - qx).powi(2) + (p.y - qy).powi(2)).sqrt()
}

fn line(points: &[Coord], tolerance: f64, ring: bool) -> Vec<Coord> {
    let n = points.len();
    let least = if ring { 4 } else { 2 };
    if n <= least {
        return points.to_vec();
    }
    let mut keep = vec![false; n];
    keep[0] = true;
    keep[n - 1] = true;
    let mut stack = vec![(0usize, n - 1)];
    while let Some((first, last)) = stack.pop() {
        if last <= first + 1 {
            continue;
        }
        let mut far = first;
        let mut most = -1.0;
        for i in first + 1..last {
            let d = distance(&points[i], &points[first], &points[last]);
            if d > most {
                most = d;
                far = i;
            }
        }
        if most > tolerance {
            keep[far] = true;
            stack.push((first, far));
            stack.push((far, last));
        }
    }
    let mut out: Vec<Coord> = points
        .iter()
        .zip(&keep)
        .filter(|(_, k)| **k)
        .map(|(p, _)| *p)
        .collect();
    if ring && out.len() < least {
        // A closed ring's ends are the same position: Douglas–Peucker from it alone keeps nothing between. Keep the
        // two positions farthest along it from its start, so the ring stays an area.
        out = ring_least(points);
    }
    out
}

/// A ring cut down to four positions: its start, the position farthest from it, the one farthest from both, the start.
fn ring_least(points: &[Coord]) -> Vec<Coord> {
    let n = points.len();
    let start = points[0];
    let d2 = |a: &Coord, b: &Coord| (a.x - b.x).powi(2) + (a.y - b.y).powi(2);
    let (mut far, mut most) = (1usize, -1.0);
    for (i, p) in points.iter().enumerate().take(n - 1).skip(1) {
        let d = d2(p, &start);
        if d > most {
            most = d;
            far = i;
        }
    }
    let (mut third, mut best) = (if far == 1 { 2 } else { 1 }, -1.0);
    for (i, p) in points.iter().enumerate().take(n - 1).skip(1) {
        if i == far {
            continue;
        }
        let d = distance(p, &start, &points[far]);
        if d > best {
            best = d;
            third = i;
        }
    }
    let (a, b) = if third < far {
        (third, far)
    } else {
        (far, third)
    };
    vec![start, points[a], points[b], points[n - 1]]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pts(p: &[(f64, f64)]) -> Vec<Coord> {
        p.iter().map(|&(x, y)| Coord::xy(x, y)).collect()
    }

    #[test]
    fn a_line_loses_its_nearly_straight_vertices() {
        let g = Geometry::Curve(Curve::Line(pts(&[
            (0.0, 0.0),
            (1.0, 0.01),
            (2.0, -0.01),
            (3.0, 5.0),
            (4.0, 6.0),
        ])));
        let Geometry::Curve(Curve::Line(out)) = simplify(&g, 0.5) else {
            panic!("line expected");
        };
        assert_eq!(out.len(), 4);
        assert_eq!(out.first().map(|p| (p.x, p.y)), Some((0.0, 0.0)));
        assert_eq!(out.last().map(|p| (p.x, p.y)), Some((4.0, 6.0)));
    }

    #[test]
    fn a_ring_never_falls_below_four_positions() {
        let ring = pts(&[(0.0, 0.0), (10.0, 0.0), (10.0, 0.1), (0.0, 0.1), (0.0, 0.0)]);
        let g = Geometry::Polygon(vec![Curve::Line(ring)]);
        let Geometry::Polygon(rings) = simplify(&g, 1.0) else {
            panic!("polygon expected");
        };
        let Curve::Line(r) = &rings[0] else {
            panic!("line ring expected");
        };
        assert_eq!(r.len(), 4);
        assert_eq!((r[0].x, r[0].y), (r[3].x, r[3].y));
    }

    #[test]
    fn nothing_changes_without_a_tolerance() {
        let g = Geometry::Curve(Curve::Line(pts(&[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)])));
        assert_eq!(simplify(&g, 0.0), g);
    }
}
