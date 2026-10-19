//! Olay kaymasını denetle (docs/adr/0526): how far an event object lies from the place its route gives it now. A
//! point's drift is the distance between the two points; a line's is the farthest of either's vertices and arc
//! midpoints from the other's path, so a vertex of the route moved under the event, an end walked along it or an
//! arc bent away are all found.

use super::{Route, RoutePart, Walk};
use crate::jsmath::{js_hypot, js_max};
use crate::vec2::Vec2;

/// A point event's drift (m).
pub fn point_drift(now: Vec2, placed: Vec2) -> f64 {
    js_hypot(now.x - placed.x, now.y - placed.y)
}

/// The places of a path the drift is measured at: its vertices, then each arc edge's midpoint.
fn probes(parts: &[RoutePart]) -> Vec<Vec2> {
    let mut out = Vec::new();
    for q in parts {
        out.extend(q.pts.iter().copied());
        let Some(bulges) = &q.bulges else {
            continue;
        };
        for (i, w) in q.pts.windows(2).enumerate() {
            let b = bulges.get(i).copied().unwrap_or(0.0);
            if b == 0.0 {
                continue;
            }
            let (a, c) = (w[0], w[1]);
            let (dx, dy) = (c.x - a.x, c.y - a.y);
            let len = js_hypot(dx, dy);
            if len == 0.0 {
                continue;
            }
            // The sagitta: a positive bulge (counter-clockwise) bends to the chord's right.
            let s = b * len / 2.0;
            out.push(Vec2 {
                x: (a.x + c.x) / 2.0 + dy / len * s,
                y: (a.y + c.y) / 2.0 - dx / len * s,
            });
        }
    }
    out
}

/// The farthest of `from`'s probes from `to`'s path; infinite when `to` has no edge.
fn farthest(from: &[RoutePart], to: &[RoutePart]) -> f64 {
    let walk = Walk::new(&Route { parts: to.to_vec() });
    let mut far: f64 = 0.0;
    for p in probes(from) {
        match walk.locate(p) {
            Some(place) => far = js_max(far, place.distance.abs()),
            None => return f64::INFINITY,
        }
    }
    far
}

/// A line event's drift (m): the farthest probe of either from the other's path.
pub fn line_drift(now: &[RoutePart], placed: &[RoutePart]) -> f64 {
    js_max(farthest(now, placed), farthest(placed, now))
}
