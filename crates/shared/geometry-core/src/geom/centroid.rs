//! The centroid of an area (docs/adr/0100 §3, 0163 §1): the rings' first
//! moments relative to the first vertex (projected coordinates of millions
//! of metres would cancel away the decimals otherwise), a bulged edge's
//! circular segment exactly, holes subtracted, a multi-part area's parts
//! weighted by their areas. The expression's `$merkez_y` and `$merkez_x`
//! and the Ağırlık merkezi snap read it; the independent reference is
//! `scripts/fixtures/expression_geometry.py`.

use crate::entity::{Shape, area_parts, entity_anchor, entity_area, is_multi_part};
use crate::geom::arrangement::Area;
use crate::geom::bulge::{bulge_arc, bulge_at, segment_mid};
use crate::jsmath::sin;
use crate::vec2::Vec2;

/// A ring's signed area and first moments about `o`, a bulged edge's
/// circular segment included (signed as the geometry core's area signs it).
fn moments(pts: &[Vec2], bulges: Option<&[f64]>, o: Vec2) -> (f64, f64, f64) {
    let n = pts.len();
    let (mut a, mut mx, mut my) = (0.0, 0.0, 0.0);
    if n < 2 {
        return (a, mx, my);
    }
    for i in 0..n {
        let (p, q) = (pts[i], pts[(i + 1) % n]);
        let (px, py, qx, qy) = (p.x - o.x, p.y - o.y, q.x - o.x, q.y - o.y);
        let cross = px * qy - qx * py;
        a += cross / 2.0;
        mx += (px + qx) * cross / 6.0;
        my += (py + qy) * cross / 6.0;
        if let Some(arc) = bulge_arc(p, q, bulge_at(bulges, i)) {
            let t = arc.sweep.abs();
            let lens = t - sin(t);
            if lens <= 0.0 || arc.r <= 0.0 || lens.is_nan() || arc.r.is_nan() {
                continue;
            }
            // The segment between chord and arc: its area, signed as the
            // sweep, and its centroid on the bisector at 4r·sin³(θ/2)/(3(θ − sin θ)).
            let s = arc.r * arc.r / 2.0 * (arc.sweep - sin(arc.sweep));
            let half = sin(t / 2.0);
            let d = 4.0 * arc.r * half * half * half / (3.0 * lens);
            let m = segment_mid(p, q, bulge_at(bulges, i));
            let (ux, uy) = ((m.x - arc.c.x) / arc.r, (m.y - arc.c.y) / arc.r);
            let (gx, gy) = (arc.c.x - o.x + ux * d, arc.c.y - o.y + uy * d);
            a += s;
            mx += s * gx;
            my += s * gy;
        }
    }
    (a, mx, my)
}

/// The centroid of an outer ring with holes (areas taken positive, holes
/// removed), or None when the area is empty.
pub fn area_centroid<'r>(
    outer: (&'r [Vec2], Option<&'r [f64]>),
    holes: impl Iterator<Item = (&'r [Vec2], Option<&'r [f64]>)>,
) -> Option<Vec2> {
    let o = *outer.0.first()?;
    let (a, mx, my) = moments(outer.0, outer.1, o);
    let sign = if a < 0.0 { -1.0 } else { 1.0 };
    let (mut area, mut sx, mut sy) = (a * sign, mx * sign, my * sign);
    for (pts, bulges) in holes {
        let (a, mx, my) = moments(pts, bulges, o);
        let sign = if a < 0.0 { -1.0 } else { 1.0 };
        area -= a * sign;
        sx -= mx * sign;
        sy -= my * sign;
    }
    if area == 0.0 || !area.is_finite() {
        return None;
    }
    Some(Vec2::new(o.x + sx / area, o.y + sy / area))
}

/// The centroid: of the area for an area (holes removed), a circle and a
/// whole ellipse; the anchor for everything else and for an empty area.
pub fn shape_centroid(s: &Shape) -> Option<Vec2> {
    // A multi-part area's is its parts' centroids weighted by their areas (docs/adr/0143).
    if is_multi_part(s) {
        let (mut total, mut sx, mut sy) = (0.0, 0.0, 0.0);
        for part in area_parts(s).iter() {
            let (Some(c), Some(a)) = (shape_centroid(part), entity_area(part)) else {
                continue;
            };
            total += a;
            sx += a * c.x;
            sy += a * c.y;
        }
        if total > 0.0 && total.is_finite() {
            return Some(Vec2::new(sx / total, sy / total));
        }
        return entity_anchor(s);
    }
    let area = match s {
        Shape::Polygon {
            pts, bulges, holes, ..
        } => area_centroid(
            (pts, bulges.as_deref()),
            holes
                .iter()
                .flatten()
                .map(|h| (h.pts.as_slice(), h.bulges.as_deref())),
        ),
        Shape::Hatch { ring, holes, .. } => area_centroid(
            (ring, None),
            holes.iter().flatten().map(|h| (h.as_slice(), None)),
        ),
        Shape::Circle { c, .. } => Some(*c),
        // A whole ellipse has an area (`entity_area`), an arc of one has none.
        Shape::Ellipse { c, .. } if entity_area(s).is_some() => Some(*c),
        _ => None,
    };
    area.or_else(|| entity_anchor(s))
}

/// The centroid of areas taken together (a multi-part area's parts, a
/// closed path's area), each weighted by its area less its holes; `None`
/// when they have none.
pub fn areas_centroid(areas: &[Area]) -> Option<Vec2> {
    let (mut total, mut sx, mut sy) = (0.0, 0.0, 0.0);
    for a in areas {
        let holes = a.holes.iter().map(|h| (h.pts.as_slice(), h.bulges.as_deref()));
        let Some(c) = area_centroid((&a.outer.pts, a.outer.bulges.as_deref()), holes) else {
            continue;
        };
        let weight = net(a);
        total += weight;
        sx += weight * c.x;
        sy += weight * c.y;
    }
    (total > 0.0 && total.is_finite()).then(|| Vec2::new(sx / total, sy / total))
}

/// An area's size less its holes, from the same moments.
fn net(a: &Area) -> f64 {
    let o = a.outer.pts.first().copied().unwrap_or_default();
    let (outer, _, _) = moments(&a.outer.pts, a.outer.bulges.as_deref(), o);
    let holes: f64 = a
        .holes
        .iter()
        .map(|h| moments(&h.pts, h.bulges.as_deref(), o).0.abs())
        .sum();
    outer.abs() - holes
}
