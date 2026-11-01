//! All non-neighbour segment pairs are considered through sorted bounds.
//! Exhausting the budget is an error, never the false answer “zero crossings”.
use crate::entity::{Shape, is_closed_path};
use crate::geom::intersect::{Edge, intersect_edges};
use crate::jsmath::{js_max, js_min};
use crate::vec2::Vec2;

pub(super) fn count(s: &Shape) -> Result<usize, String> {
    let pts = match s {
        Shape::Polyline { pts, .. } | Shape::Polygon { pts, .. } => pts,
        _ => return Ok(0),
    };
    let closed = is_closed_path(s);
    let n = pts.len() - usize::from(!closed);
    let mut bounds: Vec<_> = (0..n)
        .map(|i| {
            let a = pts[i];
            let b = pts[(i + 1) % pts.len()];
            (
                js_min(a.x, b.x),
                js_max(a.x, b.x),
                js_min(a.y, b.y),
                js_max(a.y, b.y),
                i,
            )
        })
        .collect();
    bounds.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.4.cmp(&b.4)));
    let mut visits = 0;
    let mut count = 0;
    for (at, a) in bounds.iter().enumerate() {
        for b in &bounds[at + 1..] {
            if b.0 > a.1 + 1e-10 {
                break;
            }
            visits += 1;
            if visits > 2_000_000 {
                return Err("Ötelemenin kendi kesişim denetimi hesap sınırını aşıyor; eğriyi bölün veya toleransı büyütün.".into());
            }
            if a.4.abs_diff(b.4) == 1
                || (closed && a.4.min(b.4) == 0 && a.4.max(b.4) == n - 1)
                || a.3 + 1e-10 < b.2
                || b.3 + 1e-10 < a.2
            {
                continue;
            }
            let edge = |i| Edge::Seg {
                a: pts[i],
                b: pts[(i + 1) % pts.len()],
            };
            if !intersect_edges(&edge(a.4), &edge(b.4)).is_empty()
                || overlaps(
                    pts[a.4],
                    pts[(a.4 + 1) % pts.len()],
                    pts[b.4],
                    pts[(b.4 + 1) % pts.len()],
                )
            {
                count += 1;
            }
        }
    }
    Ok(count)
}
fn overlaps(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> bool {
    let length = crate::jsmath::js_hypot(b.x - a.x, b.y - a.y);
    if length == 0. {
        return false;
    }
    let cross = |p: Vec2| ((b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x)).abs() / length;
    if cross(c) > 1e-10 || cross(d) > 1e-10 {
        return false;
    }
    let project = |p: Vec2| ((p.x - a.x) * (b.x - a.x) + (p.y - a.y) * (b.y - a.y)) / length;
    let (u, v) = (project(c), project(d));
    js_max(js_min(u, v), 0.) <= js_min(js_max(u, v), length) + 1e-10
}
