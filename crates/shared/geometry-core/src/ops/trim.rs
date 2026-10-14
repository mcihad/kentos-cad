//! Quick trim and extend, AutoCAD-style: every other visible edge is a
//! boundary (`apps/web/src/model/ops/trim.ts`). The target is a path parameterised by
//! arc length; cuts are the s values where boundaries cross it.

use crate::api::Op;
use crate::entity::{
    CLOSED_HAS_NO_END, CONSTRUCTION_REACH, Entity, MULTI_PART_REFUSED, Shape, is_multi_part,
    leader_refused,
};
use crate::geom::arc::norm_angle;
use crate::geom::bulge::{bulge_arc, bulge_at, bulge_of_sweep};
use crate::geom::intersect::{
    Edge, circle_circle, full_circle, line_circle_params, on_edge_arc, point_at, ray_edge,
};
use crate::geometry::Bounds;
use crate::jsmath::{
    TAU, acos, atan2, cos, js_hypot, js_max, js_max_all, js_min, js_min_all, sin,
};
use crate::op;
use crate::ops::curve_cuts::{
    Cut, Geometry, construction_of, ellipse_of, extend_ellipse, trim_construction, trim_ellipse,
};
use crate::ops::path::{cuts_on, nearest_s, path_of, sub_path};
use crate::vec2::Vec2;

/// Removes the part of `target` between the two cuts that bracket `pick`.
pub fn trim_entity(target: &Entity, pick: Vec2, boundaries: &[Edge]) -> Cut {
    let s = &target.shape;
    match s {
        Shape::Spline { .. } => {
            return Cut::Error(
                "Eğri budanamaz; önce Patlat (X) ile çoklu çizgiye dönüştürün.".into(),
            );
        }
        // Cut on the curve itself, each piece the same curve (docs/adr/0259 §2).
        Shape::Nurbs { .. } => return crate::ops::nurbs_cuts::trim(target, pick, boundaries),
        Shape::Dimension { .. } | Shape::Hatch { .. } => {
            return Cut::Error("Ölçü ve tarama budanamaz.".into());
        }
        Shape::Leader { .. } => return Cut::Error(leader_refused("budanamaz")),
        _ => {}
    }
    if let Some(g) = ellipse_of(s) {
        return trim_ellipse(&g, pick, boundaries);
    }
    if let Some(c) = construction_of(s) {
        return trim_construction(&c, pick, boundaries);
    }
    if is_multi_part(s) {
        return Cut::Error(MULTI_PART_REFUSED.into());
    }
    let Some(path) = path_of(s) else {
        return Cut::Error("Bu nesne budanamaz.".into());
    };
    let cuts = cuts_on(&path, boundaries);
    let sp = nearest_s(&path, pick);
    let eps = 1e-7 * js_max(1.0, path.length);
    if !path.closed {
        let lo = js_max_all(std::iter::once(0.0).chain(cuts.iter().copied().filter(|&c| c < sp)));
        let hi = js_min_all(
            std::iter::once(path.length).chain(cuts.iter().copied().filter(|&c| c > sp)),
        );
        if lo <= eps && hi >= path.length - eps {
            return Cut::Error("Tıklanan kısmı kesen bir kenar yok.".into());
        }
        let mut pieces = Vec::new();
        if lo > eps {
            pieces.push(sub_path(&path, 0.0, lo, s));
        }
        if hi < path.length - eps {
            pieces.push(sub_path(&path, hi, path.length, s));
        }
        return Cut::Pieces(pieces);
    }
    if cuts.len() < 2 {
        return Cut::Error("Kapalı nesneyi budamak için en az iki kesişim gerekir.".into());
    }
    let lo = cuts
        .iter()
        .copied()
        .rfind(|&c| c < sp)
        .unwrap_or(cuts[cuts.len() - 1] - path.length);
    let hi = cuts
        .iter()
        .copied()
        .find(|&c| c > sp)
        .unwrap_or(cuts[0] + path.length);
    // Keep the complement: from hi around to lo.
    Cut::Pieces(vec![sub_path(&path, hi, lo + path.length, s)])
}

/// Whether [`trim_entity`] cuts objects of this kind at all: a failure on
/// one of them means no boundary cuts it there (AutoCAD's quick mode then
/// erases it, docs/adr/0294), on any other its kind is refused.
pub fn trimmable(s: &Shape) -> bool {
    match s {
        Shape::Spline { .. }
        | Shape::Dimension { .. }
        | Shape::Hatch { .. }
        | Shape::Leader { .. } => false,
        Shape::Nurbs { .. } => true,
        _ if ellipse_of(s).is_some() || construction_of(s).is_some() => true,
        _ => !is_multi_part(s) && path_of(s).is_some(),
    }
}

/// The middle of the first part of `s` (its edges in their order) of
/// positive length inside the closed rectangle `w`: where a crossing window
/// (Kesişen, docs/adr/0294) meets the object, or `None` when it only
/// touches it or misses it.
pub fn window_point(s: &Shape, w: &Bounds) -> Option<Vec2> {
    window_points(s, w).first().copied()
}

/// [`window_point`], then the same part's points a third and two thirds
/// along it: a pick that lands on a crossing takes the parts on both sides
/// together, so a window tries the others before it calls the object uncut.
pub fn window_points(s: &Shape, w: &Bounds) -> Vec<Vec2> {
    const LEAST: f64 = 1e-9;
    let inside = |p: Vec2| p.x >= w.min_x && p.x <= w.max_x && p.y >= w.min_y && p.y <= w.max_y;
    for e in crate::ops::edges::entity_edges(s) {
        match e {
            Edge::Seg { a, b } => {
                // Liang–Barsky: the part of a→b inside the rectangle, t0..t1.
                let d = Vec2::new(b.x - a.x, b.y - a.y);
                let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
                let mut out = false;
                for (p, q) in [
                    (-d.x, a.x - w.min_x),
                    (d.x, w.max_x - a.x),
                    (-d.y, a.y - w.min_y),
                    (d.y, w.max_y - a.y),
                ] {
                    if p == 0.0 {
                        if q < 0.0 {
                            out = true;
                        }
                        continue;
                    }
                    let r = q / p;
                    if p < 0.0 {
                        t0 = js_max(t0, r);
                    } else {
                        t1 = js_min(t1, r);
                    }
                }
                if !out && (t1 - t0) * js_hypot(d.x, d.y) > LEAST {
                    return [0.5, 1.0 / 3.0, 2.0 / 3.0]
                        .map(|f| {
                            let t = t0 + (t1 - t0) * f;
                            Vec2::new(a.x + d.x * t, a.y + d.y * t)
                        })
                        .to_vec();
                }
            }
            Edge::Arc { c, r, a0, sweep } => {
                // The arc split where its circle crosses the rectangle's sides; a piece whose
                // middle is inside is inside.
                let mut us = vec![0.0, 1.0];
                let mut at = |theta: f64| {
                    let u = if sweep >= 0.0 {
                        norm_angle(theta - a0) / sweep
                    } else {
                        norm_angle(a0 - theta) / -sweep
                    };
                    if u > 0.0 && u < 1.0 {
                        us.push(u);
                    }
                };
                for x in [w.min_x, w.max_x] {
                    let k = (x - c.x) / r;
                    if k.abs() <= 1.0 {
                        let th = acos(k);
                        at(th);
                        at(-th);
                    }
                }
                for y in [w.min_y, w.max_y] {
                    let k = (y - c.y) / r;
                    if k.abs() <= 1.0 {
                        let th = acos(k);
                        at(PI_2 - th);
                        at(PI_2 + th);
                    }
                }
                us.sort_by(f64::total_cmp);
                for pair in us.windows(2) {
                    if (pair[1] - pair[0]) * (sweep * r).abs() <= LEAST {
                        continue;
                    }
                    let at = |f: f64| {
                        let th = a0 + sweep * (pair[0] + (pair[1] - pair[0]) * f);
                        Vec2::new(c.x + cos(th) * r, c.y + sin(th) * r)
                    };
                    if inside(at(0.5)) {
                        return [0.5, 1.0 / 3.0, 2.0 / 3.0].map(at).to_vec();
                    }
                }
            }
        }
    }
    Vec::new()
}

/// π/2: the angle whose sine is 1 (an arc's crossings with a horizontal side).
const PI_2: f64 = std::f64::consts::FRAC_PI_2;

/// Extends the end of `target` nearest to `pick` to the first boundary it meets.
pub fn extend_entity(target: &Entity, pick: Vec2, boundaries: &[Edge]) -> Geometry {
    let s = &target.shape;
    if let Shape::Leader { .. } = s {
        return Geometry::Error(leader_refused("uzatılamaz"));
    }
    // Which part's end would go on is not known (docs/adr/0174).
    if is_multi_part(s) {
        return Geometry::Error(MULTI_PART_REFUSED.into());
    }
    if let Shape::Polyline { closed: true, .. } = s {
        return Geometry::Error(CLOSED_HAS_NO_END.into());
    }
    if let Some(g) = ellipse_of(s) {
        return extend_ellipse(&g, pick, boundaries);
    }
    match s {
        Shape::Line { .. } | Shape::Polyline { .. } => {
            let (mut pts, mut bulges) = match s {
                Shape::Line { a, b } => (vec![*a, *b], None),
                Shape::Polyline { pts, bulges, .. } => (pts.clone(), bulges.clone()),
                _ => (Vec::new(), None),
            };
            let n = pts.len();
            // A path needs an end segment to grow.
            if n < 2 {
                return Geometry::Error("Uzatmak için en az iki köşe gerekir.".into());
            }
            let at_end = js_hypot(pick.x - pts[n - 1].x, pick.y - pts[n - 1].y)
                <= js_hypot(pick.x - pts[0].x, pick.y - pts[0].y);
            let seg = if at_end { n - 2 } else { 0 };
            if let Some(arc) = bulge_arc(pts[seg], pts[seg + 1], bulge_at(bulges.as_deref(), seg)) {
                // An arc end segment grows along its own circle.
                let ccw = arc.sweep > 0.0;
                let end_angle = if at_end { arc.a0 + arc.sweep } else { arc.a0 };
                let forward = at_end == ccw; // the tip moves counter-clockwise
                let Some(delta) = arc_reach(
                    arc.c,
                    arc.r,
                    end_angle,
                    forward,
                    TAU - arc.sweep.abs(),
                    boundaries,
                ) else {
                    return Geometry::Error("Bu yönde ulaşılacak bir sınır yok.".into());
                };
                let tip_angle = end_angle + if forward { delta } else { -delta };
                let tip = Vec2::new(
                    arc.c.x + cos(tip_angle) * arc.r,
                    arc.c.y + sin(tip_angle) * arc.r,
                );
                let sw = arc.sweep + if ccw { delta } else { -delta };
                if at_end {
                    pts[n - 1] = tip;
                } else {
                    pts[0] = tip;
                }
                let mut b = bulges.take().unwrap_or_default();
                if seg < b.len() {
                    b[seg] = bulge_of_sweep(sw);
                }
                return Geometry::Ok(Entity::new(Shape::Polyline {
                    pts,
                    bulges: Some(b),
                    holes: None,
                    parts: None,
                    closed: false,
                    widths: None,
                }));
            }
            let tip = if at_end { pts[n - 1] } else { pts[0] };
            let prev = if at_end { pts[n - 2] } else { pts[1] };
            let len = js_hypot(tip.x - prev.x, tip.y - prev.y);
            if len < 1e-12 {
                return Geometry::Error("Sıfır uzunluklu kenar uzatılamaz.".into());
            }
            let dir = Vec2::new((tip.x - prev.x) / len, (tip.y - prev.y) / len);
            let mut best = f64::INFINITY;
            for b in boundaries {
                for t in ray_edge(tip, dir, b, 1e-7) {
                    best = js_min(best, t);
                }
            }
            if !best.is_finite() {
                return Geometry::Error("Bu doğrultuda ulaşılacak bir sınır yok.".into());
            }
            let np = Vec2::new(tip.x + dir.x * best, tip.y + dir.y * best);
            if at_end {
                pts[n - 1] = np;
            } else {
                pts[0] = np;
            }
            if matches!(s, Shape::Line { .. }) {
                return Geometry::Ok(Entity::new(Shape::Line {
                    a: pts[0],
                    b: pts[1],
                }));
            }
            Geometry::Ok(Entity::new(Shape::Polyline {
                pts,
                bulges,
                holes: None,
                parts: None,
                closed: false,
                widths: None,
            }))
        }
        Shape::Arc { c, r, a0, a1 } => {
            let to_start = js_hypot(pick.x - (c.x + cos(*a0) * r), pick.y - (c.y + sin(*a0) * r));
            let to_end = js_hypot(pick.x - (c.x + cos(*a1) * r), pick.y - (c.y + sin(*a1) * r));
            let at_end = to_end <= to_start;
            let gap = TAU - norm_angle(a1 - a0);
            let Some(best) = arc_reach(
                *c,
                *r,
                if at_end { *a1 } else { *a0 },
                at_end,
                gap,
                boundaries,
            ) else {
                return Geometry::Error("Bu yönde ulaşılacak bir sınır yok.".into());
            };
            Geometry::Ok(Entity::new(Shape::Arc {
                c: *c,
                r: *r,
                a0: if at_end { *a0 } else { norm_angle(a0 - best) },
                a1: if at_end { norm_angle(a1 + best) } else { *a1 },
            }))
        }
        _ => Geometry::Error(
            "Yalnızca çizgi, açık çoklu çizgi, yay ve eliptik yay uzatılabilir.".into(),
        ),
    }
}

/// Smallest angle (< `gap`) by which an arc end at `from` can grow (CCW when `ccw`) before it meets a boundary.
fn arc_reach(c: Vec2, r: f64, from: f64, ccw: bool, gap: f64, boundaries: &[Edge]) -> Option<f64> {
    let mut best = f64::INFINITY;
    for b in boundaries {
        for p in circle_hits(c, r, b) {
            let th = atan2(p.y - c.y, p.x - c.x);
            let delta = if ccw {
                norm_angle(th - from)
            } else {
                norm_angle(from - th)
            };
            if delta > 1e-9 && delta < gap - 1e-9 {
                best = js_min(best, delta);
            }
        }
    }
    best.is_finite().then_some(best)
}

/// Intersections of a full circle with an edge (boundary side bounded).
fn circle_hits(c: Vec2, r: f64, b: &Edge) -> Vec<Vec2> {
    match *b {
        Edge::Seg { a, b: bb } => line_circle_params(a, bb, c, r)
            .into_iter()
            .filter(|&t| t >= -1e-9 && t <= 1.0 + 1e-9)
            .map(|t| point_at(&Edge::Seg { a, b: bb }, t))
            .collect(),
        Edge::Arc {
            c: bc,
            r: br,
            a0,
            sweep,
        } => circle_circle(c, r, bc, br)
            .into_iter()
            .filter(|p| on_edge_arc(a0, sweep, atan2(p.y - bc.y, p.x - bc.x)))
            .collect(),
    }
}

/// A boundary's edges as AutoCAD's EDGEMODE 1 has them (docs/adr/0294):
/// along its natural path past its ends. A line is its whole line, an arc
/// its whole circle; an open polyline's first segment goes on back past its
/// start and its last on past its end the same way (a straight one as its
/// line, an arc as its circle), the segments between as they are. `None`
/// for every other kind: a closed object has no end, and an elliptic arc's
/// or a curve's way on is not one AutoCAD and KentOS agree on; they bound
/// where they are. The infinite line is a segment reaching
/// `CONSTRUCTION_REACH` past each end, as a construction line is.
pub fn extended_edges(s: &Shape) -> Option<Vec<Edge>> {
    let on = |a: Vec2, b: Vec2, back: bool, forth: bool| -> Edge {
        let len = js_hypot(b.x - a.x, b.y - a.y);
        if len < 1e-12 {
            return Edge::Seg { a, b };
        }
        let d = Vec2::new((b.x - a.x) / len, (b.y - a.y) / len);
        let r = CONSTRUCTION_REACH;
        Edge::Seg {
            a: if back {
                Vec2::new(a.x - d.x * r, a.y - d.y * r)
            } else {
                a
            },
            b: if forth {
                Vec2::new(b.x + d.x * r, b.y + d.y * r)
            } else {
                b
            },
        }
    };
    let grown = |e: Edge, back: bool, forth: bool| match e {
        Edge::Seg { a, b } => on(a, b, back, forth),
        Edge::Arc { c, r, .. } => full_circle(c, r),
    };
    match s {
        Shape::Line { a, b } => Some(vec![on(*a, *b, true, true)]),
        Shape::Arc { c, r, .. } => Some(vec![full_circle(*c, *r)]),
        Shape::Polyline { closed: false, .. } if !is_multi_part(s) => {
            let mut edges = crate::ops::edges::entity_edges(s);
            let n = edges.len();
            if n == 0 {
                return None;
            }
            edges[0] = grown(edges[0], true, n == 1);
            if n > 1 {
                edges[n - 1] = grown(edges[n - 1], false, true);
            }
            Some(edges)
        }
        _ => None,
    }
}

/// Buda and Uzat, Çit yöntemi (docs/adr/0140): where the fence (a drawn
/// open path) crosses `target`, in the fence's order: each crossing is
/// where the tool picks the object, as a click there would, so the part
/// the fence crosses is trimmed and the end it passes nearest is extended.
/// Crossings closer than 1e-9 m to one before are one.
pub fn fence_crossings(target: &Entity, fence: &[Vec2]) -> Vec<Vec2> {
    let edges = crate::ops::edges::entity_edges(&target.shape);
    let mut out: Vec<Vec2> = Vec::new();
    for w in fence.windows(2) {
        let seg = Edge::Seg { a: w[0], b: w[1] };
        let mut here: Vec<(f64, Vec2)> = edges
            .iter()
            .flat_map(|e| crate::geom::intersect::intersect_edges(&seg, e))
            .map(|h| (h.t, h.p))
            .collect();
        here.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (_, p) in here {
            if !out.iter().any(|q| js_hypot(q.x - p.x, q.y - p.y) <= 1e-9) {
                out.push(p);
            }
        }
    }
    out
}

pub(crate) static OPS: &[Op] = &[
    op!("fenceCrossings", |target: Entity, fence: Vec<Vec2>| {
        fence_crossings(&target, &fence)
    }),
    op!("trimEntity", |target: Entity,
                       pick: Vec2,
                       boundaries: Vec<Edge>| {
        trim_entity(&target, pick, &boundaries)
    }),
    op!(
        "extendEntity",
        |target: Entity, pick: Vec2, boundaries: Vec<Edge>| extend_entity(
            &target,
            pick,
            &boundaries
        )
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    /// EDGEMODE 1 (docs/adr/0294): a line bounds as its whole line, an open polyline past its ends only, a
    /// closed object where it is.
    #[test]
    fn extended_edges_go_on_past_the_ends_of_open_boundaries_only() {
        let target = Entity::new(Shape::Line {
            a: Vec2::new(0.0, 0.0),
            b: Vec2::new(30.0, 0.0),
        });
        let short = Shape::Line {
            a: Vec2::new(5.0, 2.0),
            b: Vec2::new(5.0, 6.0),
        };
        let pick = Vec2::new(3.0, 0.0);
        assert!(matches!(
            trim_entity(&target, pick, &crate::ops::edges::entity_edges(&short)),
            Cut::Error(_)
        ));
        let Some(grown) = extended_edges(&short) else {
            panic!("a line grows")
        };
        let Cut::Pieces(pieces) = trim_entity(&target, pick, &grown) else {
            panic!("its line cuts")
        };
        assert_eq!(pieces.len(), 1);
        let Shape::Line { a, .. } = pieces[0].shape else {
            panic!("a line")
        };
        assert!((a.x - 5.0).abs() < 1e-9 && a.y.abs() < 1e-9, "{a:?}");
        let square = Shape::Polygon {
            pts: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(1.0, 1.0),
            ],
            bulges: None,
            holes: None,
            parts: None,
        };
        assert!(extended_edges(&square).is_none());
    }

    /// Kesişen's point: on a part of positive length inside the window; touching is not meeting.
    #[test]
    fn a_window_point_lies_inside_and_a_touch_is_none() {
        let line = Shape::Line {
            a: Vec2::new(0.0, 0.0),
            b: Vec2::new(10.0, 0.0),
        };
        let w = |x0: f64, y0: f64, x1: f64, y1: f64| Bounds {
            min_x: x0,
            min_y: y0,
            max_x: x1,
            max_y: y1,
        };
        let mid = window_point(&line, &w(2.0, -1.0, 4.0, 1.0)).expect("inside");
        assert!((mid.x - 3.0).abs() < 1e-12 && mid.y == 0.0, "{mid:?}");
        assert_eq!(window_point(&line, &w(10.0, -1.0, 12.0, 1.0)), None);
        assert_eq!(window_point(&line, &w(2.0, 1.0, 4.0, 2.0)), None);
        let circle = Shape::Circle {
            c: Vec2::new(0.0, 0.0),
            r: 5.0,
        };
        let top = window_point(&circle, &w(-1.0, 4.0, 1.0, 6.0)).expect("the top");
        assert!(top.y > 4.0 && top.x.abs() <= 1.0, "{top:?}");
        assert_eq!(window_points(&circle, &w(-1.0, 4.0, 1.0, 6.0)).len(), 3);
    }

    #[test]
    fn a_fence_picks_each_object_where_it_crosses_it_in_its_order() {
        let square = Entity::new(Shape::Polygon {
            pts: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(10.0, 0.0),
                Vec2::new(10.0, 10.0),
                Vec2::new(0.0, 10.0),
            ],
            bulges: None,
            holes: None,
            parts: None,
        });
        // A fence across the square from west to east, then on north of it.
        let fence = [
            Vec2::new(-5.0, 5.0),
            Vec2::new(15.0, 5.0),
            Vec2::new(15.0, 20.0),
        ];
        assert_eq!(
            fence_crossings(&square, &fence),
            vec![Vec2::new(0.0, 5.0), Vec2::new(10.0, 5.0)]
        );
        assert!(fence_crossings(&square, &[Vec2::new(20.0, 0.0), Vec2::new(30.0, 0.0)]).is_empty());
    }
}
