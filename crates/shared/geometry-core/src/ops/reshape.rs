//! Whole-object reshaping (docs/adr/0140): every corner of a path rounded or
//! cut at once (Tüm köşeleri yuvarla, Tüm köşelere pah), a path's direction
//! reversed (Yönü çevir) and a path's vertices thinned within a tolerance
//! (Sadeleştir). Each takes an object and gives it back with its other
//! fields (id, layer, colour, attributes …) untouched, and says what it did.

use crate::api::Op;
use crate::api::json::{ToJson, field};
use crate::entity::{Entity, Shape, area_parts, is_multi_part, join_parts};
use crate::geom::arrangement::Ring;
use crate::geom::bulge::{BulgePath, bulge_at, clean_bulge_path, is_arc_bulge, reverse_bulge_path};
use crate::geometry::dist_to_segment;
use crate::jsmath::{PI, acos, atan, cos, js_hypot, js_max, js_min, sin, tan};
use crate::op;
use crate::ops::fillet::{CornerOp, CornerResult, corner_of_path};
use crate::vec2::Vec2;

/// Two directions closer than this (radians) make no corner.
const STRAIGHT: f64 = 1e-6;

/// An object reshaped, with counts: corners done and skipped, or vertices
/// removed and how far the removed ones lay from the new outline.
pub struct Reshaped {
    pub entity: Entity,
    /// Corners rounded or cut; vertices removed (Sadeleştir).
    pub done: usize,
    /// Corners that could not be done: too tight for the value, or next to an arc edge.
    pub skipped: usize,
    /// Sadeleştir: the largest distance of a removed vertex from the new outline (m); 0 otherwise.
    pub deviation: f64,
}

impl ToJson for Reshaped {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        field(out, &mut first, "entity", &self.entity);
        field(out, &mut first, "done", &(self.done as f64));
        field(out, &mut first, "skipped", &(self.skipped as f64));
        field(out, &mut first, "deviation", &self.deviation);
        out.push('}');
    }
}

fn unit(a: Vec2, b: Vec2) -> Option<Vec2> {
    let l = js_hypot(b.x - a.x, b.y - a.y);
    (l > 1e-12).then(|| Vec2::new((b.x - a.x) / l, (b.y - a.y) / l))
}

fn rotate(v: Vec2, t: f64) -> Vec2 {
    Vec2::new(v.x * cos(t) - v.y * sin(t), v.x * sin(t) + v.y * cos(t))
}

/// The direction of travel where the edge from `a` to `b` (bulge `bulge`)
/// leaves `a` (`at_start`) or reaches `b`: an arc's tangent is its chord
/// turned by half the sweep.
fn edge_direction(a: Vec2, b: Vec2, bulge: f64, at_start: bool) -> Option<Vec2> {
    let chord = unit(a, b)?;
    let half = 2.0 * atan(bulge);
    Some(rotate(chord, if at_start { -half } else { half }))
}

/// What vertex `i` of a path is for the corner tools.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Vertex {
    /// No corner: an open path's end, a straight run, a zero-length edge, a smooth join.
    Plain,
    /// A corner between straight edges, with the distances the value takes
    /// along the edge before it and the edge after it.
    Corner { before: f64, after: f64 },
    /// A corner with an arc edge on one side: the tools leave it.
    NextToArc,
}

fn vertex(pts: &[Vec2], bulges: Option<&[f64]>, closed: bool, i: usize, op: &CornerOp) -> Vertex {
    let n = pts.len();
    if n < 3 || (!closed && (i == 0 || i + 1 >= n)) {
        return Vertex::Plain;
    }
    let (ip, inx) = ((i + n - 1) % n, (i + 1) % n);
    let (bp, bn) = (bulge_at(bulges, ip), bulge_at(bulges, i));
    let (Some(din), Some(dout)) = (
        edge_direction(pts[ip], pts[i], bp, false),
        edge_direction(pts[i], pts[inx], bn, true),
    ) else {
        return Vertex::Plain;
    };
    let turn = acos(js_max(-1.0, js_min(1.0, din.x * dout.x + din.y * dout.y)));
    if turn < STRAIGHT || PI - turn < STRAIGHT {
        return Vertex::Plain;
    }
    if is_arc_bulge(bp) || is_arc_bulge(bn) {
        return Vertex::NextToArc;
    }
    // The inner angle between the two edges, as corner_of_path measures it.
    let phi = PI - turn;
    match *op {
        CornerOp::Radius(r) => {
            let d = r / tan(phi / 2.0);
            Vertex::Corner {
                before: d,
                after: d,
            }
        }
        CornerOp::Chamfer(d1, d2) => Vertex::Corner {
            before: d1,
            after: d2,
        },
    }
}

/// Every corner of one path rounded or cut with the same value. A corner is
/// left when the value does not fit its edges, or when it and the next
/// corner together need more than the edge between them (both are left
/// then, so the result does not depend on where the path starts), or when
/// an arc edge meets it. Straight runs and smooth joins are not corners.
pub fn all_corners_of_path(
    pts: &[Vec2],
    bulges: Option<&[f64]>,
    closed: bool,
    op: &CornerOp,
) -> (BulgePath, usize, usize) {
    corners_of_path_where(pts, bulges, closed, op, |_| true)
}

/// Whether vertex `i` of a closed ring turns into the area (Kavşak temizle's
/// inner corners, docs/adr/0198 §3): right with the area's inside on the
/// ring's left (`area_left`), left with it on the right. An arc edge's
/// tangent there gives its direction.
pub fn turns_inward(pts: &[Vec2], bulges: Option<&[f64]>, i: usize, area_left: bool) -> bool {
    let n = pts.len();
    if n < 3 {
        return false;
    }
    let (ip, inx) = ((i + n - 1) % n, (i + 1) % n);
    let (Some(din), Some(dout)) = (
        edge_direction(pts[ip], pts[i], bulge_at(bulges, ip), false),
        edge_direction(pts[i], pts[inx], bulge_at(bulges, i), true),
    ) else {
        return false;
    };
    let cross = din.x * dout.y - din.y * dout.x;
    if area_left { cross < 0.0 } else { cross > 0.0 }
}

/// [`all_corners_of_path`] for the corners `keep` takes (by vertex index):
/// the others are no corners, and the counts are of the kept ones only
/// (Kavşak temizle's inner corners, docs/adr/0198 §3).
pub fn corners_of_path_where(
    pts: &[Vec2],
    bulges: Option<&[f64]>,
    closed: bool,
    op: &CornerOp,
    keep: impl Fn(usize) -> bool,
) -> (BulgePath, usize, usize) {
    let n = pts.len();
    let vertices: Vec<Vertex> = (0..n)
        .map(|i| {
            if keep(i) {
                vertex(pts, bulges, closed, i, op)
            } else {
                Vertex::Plain
            }
        })
        .collect();
    let edge_len = |i: usize| {
        let j = (i + 1) % n;
        js_hypot(pts[j].x - pts[i].x, pts[j].y - pts[i].y)
    };
    let eps = |l: f64| 1e-9 * js_max(1.0, l);
    // First each corner on its own edges.
    let mut take: Vec<bool> = (0..n)
        .map(|i| match vertices[i] {
            Vertex::Corner { before, after } => {
                let (lp, ln) = (edge_len((i + n - 1) % n), edge_len(i));
                before > 0.0 && after > 0.0 && before <= lp + eps(lp) && after <= ln + eps(ln)
            }
            _ => false,
        })
        .collect();
    // Then the edges two corners share.
    let edges = if closed { n } else { n.saturating_sub(1) };
    let mut clash = vec![false; n];
    for i in 0..edges {
        let j = (i + 1) % n;
        if let (true, true, Vertex::Corner { after, .. }, Vertex::Corner { before, .. }) =
            (take[i], take[j], vertices[i], vertices[j])
        {
            let l = edge_len(i);
            if after + before > l + eps(l) {
                clash[i] = true;
                clash[j] = true;
            }
        }
    }
    for (t, c) in take.iter_mut().zip(&clash) {
        *t &= !c;
    }
    let corners = vertices
        .iter()
        .filter(|v| !matches!(v, Vertex::Plain))
        .count();
    // The last first: a corner done leaves the indices before it where they were.
    let mut path = BulgePath {
        pts: pts.to_vec(),
        bulges: bulges.map(<[f64]>::to_vec),
    };
    let mut done = 0;
    for i in (0..n).rev() {
        if !take[i] {
            continue;
        }
        if let Ok(CornerResult::Path(next)) =
            corner_of_path(&path.pts, path.bulges.as_deref(), closed, i, op)
        {
            path = next;
            done += 1;
        }
    }
    (path, done, corners - done)
}

/// Tüm köşeleri yuvarla and Tüm köşelere pah on an object: a polyline, or
/// an area with its holes. `None` for any other kind, or a radius or a
/// distance that is not above zero.
pub fn all_corners(e: &Entity, op: &CornerOp) -> Option<Reshaped> {
    let positive = match *op {
        CornerOp::Radius(r) => r > 0.0,
        CornerOp::Chamfer(d1, d2) => d1 > 0.0 && d2 > 0.0,
    };
    if !positive {
        return None;
    }
    // A multi-part area or polyline: every part's corners (docs/adr/0143, 0174).
    if is_multi_part(&e.shape) {
        let (mut shapes, mut done, mut skipped) = (Vec::new(), 0, 0);
        for part in area_parts(&e.shape).iter() {
            let r = all_corners(&e.with(part.clone()), op)?;
            done += r.done;
            skipped += r.skipped;
            shapes.push(r.entity.shape);
        }
        return Some(Reshaped {
            entity: e.with(join_parts(&shapes)?),
            done,
            skipped,
            deviation: 0.0,
        });
    }
    let (shape, done, skipped) = match &e.shape {
        Shape::Polyline {
            pts, bulges, holes, ..
        } => {
            let (p, done, skipped) = all_corners_of_path(pts, bulges.as_deref(), false, op);
            (
                Shape::Polyline {
                    pts: p.pts,
                    bulges: p.bulges,
                    holes: holes.clone(),
                    parts: None,
                },
                done,
                skipped,
            )
        }
        Shape::Polygon {
            pts, bulges, holes, ..
        } => {
            let (p, mut done, mut skipped) = all_corners_of_path(pts, bulges.as_deref(), true, op);
            let holes = holes.as_ref().map(|hs| {
                hs.iter()
                    .map(|h| {
                        let (hp, d, s) = all_corners_of_path(&h.pts, h.bulges.as_deref(), true, op);
                        done += d;
                        skipped += s;
                        Ring {
                            pts: hp.pts,
                            bulges: hp.bulges,
                        }
                    })
                    .collect()
            });
            (
                Shape::Polygon {
                    pts: p.pts,
                    bulges: p.bulges,
                    holes,
                    parts: None,
                },
                done,
                skipped,
            )
        }
        _ => return None,
    };
    Some(Reshaped {
        entity: e.with(shape),
        done,
        skipped,
        deviation: 0.0,
    })
}

/// Yönü çevir: the object drawn the other way round, the same outline. Lines
/// swap their ends, polylines and splines run backwards with their arcs
/// turning the other way, an area's ring and each hole run backwards. `None`
/// for kinds without a direction of their own here: arcs and ellipses always
/// run counter-clockwise, circles, points, text, dimensions and hatches.
pub fn reverse(e: &Entity) -> Option<Entity> {
    // A multi-part area or polyline: every part runs the other way (docs/adr/0143, 0174).
    if is_multi_part(&e.shape) {
        let shapes: Option<Vec<Shape>> = area_parts(&e.shape)
            .iter()
            .map(|part| reverse(&e.with(part.clone())).map(|r| r.shape))
            .collect();
        return Some(e.with(join_parts(&shapes?)?));
    }
    let shape = match &e.shape {
        Shape::Line { a, b } => Shape::Line { a: *b, b: *a },
        Shape::Polyline {
            pts, bulges, holes, ..
        } => {
            let r = reverse_bulge_path(pts, bulges.as_deref(), false);
            Shape::Polyline {
                pts: r.pts,
                bulges: bulges.as_ref().and(r.bulges),
                holes: holes.clone(),
                parts: None,
            }
        }
        Shape::Polygon {
            pts, bulges, holes, ..
        } => {
            let r = reverse_bulge_path(pts, bulges.as_deref(), true);
            Shape::Polygon {
                parts: None,
                pts: r.pts,
                bulges: bulges.as_ref().and(r.bulges),
                holes: holes.as_ref().map(|hs| {
                    hs.iter()
                        .map(|h| {
                            let r = reverse_bulge_path(&h.pts, h.bulges.as_deref(), true);
                            Ring {
                                pts: r.pts,
                                bulges: h.bulges.as_ref().and(r.bulges),
                            }
                        })
                        .collect()
                }),
            }
        }
        Shape::Spline { pts, closed } => Shape::Spline {
            pts: pts.iter().rev().copied().collect(),
            closed: *closed,
        },
        _ => return None,
    };
    Some(e.with(shape))
}

/// Douglas–Peucker over the straight run `from`..=`to` (indices into
/// `pts`, wrapping), keeping in `keep` the vertices further than `tol` from
/// the chord that would replace them; the largest distance among the removed
/// ones goes to `worst`.
fn thin(pts: &[Vec2], from: usize, to: usize, tol: f64, keep: &mut [bool], worst: &mut f64) {
    let n = pts.len();
    let span = (to + n - from) % n;
    if span < 2 {
        return;
    }
    let (a, b) = (pts[from], pts[to]);
    let (mut far, mut far_d) = (from, -1.0);
    for k in 1..span {
        let i = (from + k) % n;
        let d = dist_to_segment(pts[i], a, b);
        if d > far_d {
            far = i;
            far_d = d;
        }
    }
    if far_d > tol {
        keep[far] = true;
        thin(pts, from, far, tol, keep, worst);
        thin(pts, far, to, tol, keep, worst);
    } else {
        *worst = js_max(*worst, far_d);
    }
}

/// One path thinned: the vertices kept, their bulges, how many went and
/// the largest distance of one that went.
fn simplify_path(
    pts: &[Vec2],
    bulges: Option<&[f64]>,
    closed: bool,
    tol: f64,
) -> (BulgePath, usize, f64) {
    let n = pts.len();
    let unchanged = || {
        (
            BulgePath {
                pts: pts.to_vec(),
                bulges: bulges.map(<[f64]>::to_vec),
            },
            0,
            0.0,
        )
    };
    let least = if closed { 3 } else { 2 };
    if n <= least || !(tol > 0.0) {
        return unchanged();
    }
    // Anchors: the ends of an open path and both ends of every arc edge.
    let mut keep = vec![false; n];
    if !closed {
        keep[0] = true;
        keep[n - 1] = true;
    }
    let edges = if closed { n } else { n - 1 };
    for i in 0..edges {
        if is_arc_bulge(bulge_at(bulges, i)) {
            keep[i] = true;
            keep[(i + 1) % n] = true;
        }
    }
    // A closed ring of straight edges: its first vertex and the one furthest from it.
    if keep.iter().filter(|k| **k).count() < 2 {
        keep[0] = true;
        let far = (1..n)
            .max_by(|&i, &j| {
                let d = |k: usize| js_hypot(pts[k].x - pts[0].x, pts[k].y - pts[0].y);
                d(i).total_cmp(&d(j))
            })
            .unwrap_or(1);
        keep[far] = true;
    }
    let anchors: Vec<usize> = (0..n).filter(|&i| keep[i]).collect();
    let mut worst: f64 = 0.0;
    let runs = if closed {
        anchors.len()
    } else {
        anchors.len() - 1
    };
    for r in 0..runs {
        let (from, to) = (anchors[r], anchors[(r + 1) % anchors.len()]);
        // An arc edge's two ends are neighbours: nothing between them.
        if is_arc_bulge(bulge_at(bulges, from)) && (from + 1) % n == to {
            continue;
        }
        thin(pts, from, to, tol, &mut keep, &mut worst);
    }
    // A ring keeps three vertices at least: the removed one furthest from the rest comes back.
    while keep.iter().filter(|k| **k).count() < least {
        let kept: Vec<Vec2> = (0..n).filter(|&i| keep[i]).map(|i| pts[i]).collect();
        let back = (0..n).filter(|&i| !keep[i]).max_by(|&i, &j| {
            let d = |k: usize| {
                kept.iter()
                    .map(|q| js_hypot(pts[k].x - q.x, pts[k].y - q.y))
                    .fold(f64::INFINITY, js_min)
            };
            d(i).total_cmp(&d(j))
        });
        match back {
            Some(i) => keep[i] = true,
            None => break,
        }
    }
    let removed = keep.iter().filter(|k| !**k).count();
    if removed == 0 {
        return unchanged();
    }
    let out_p: Vec<Vec2> = (0..n).filter(|&i| keep[i]).map(|i| pts[i]).collect();
    // A kept vertex keeps its bulge only when its edge still reaches the same next vertex.
    let out_b: Option<Vec<f64>> = bulges.map(|_| {
        (0..n)
            .filter(|&i| keep[i])
            .map(|i| {
                let b = bulge_at(bulges, i);
                if is_arc_bulge(b) && keep[(i + 1) % n] {
                    b
                } else {
                    0.0
                }
            })
            .collect()
    });
    let clean = clean_bulge_path(&out_p, out_b.as_deref(), closed, 0.0);
    (clean, removed, worst)
}

/// Sadeleştir on an object: a polyline, or an area with its holes; each
/// vertex within `tol` metres of the line that would replace it goes, arc
/// edges stay whole. `None` for other kinds or a tolerance not above zero.
pub fn simplify(e: &Entity, tol: f64) -> Option<Reshaped> {
    if !(tol > 0.0) {
        return None;
    }
    // A multi-part area or polyline: every part (docs/adr/0143, 0174).
    if is_multi_part(&e.shape) {
        let (mut shapes, mut removed, mut deviation) = (Vec::new(), 0, 0.0);
        for part in area_parts(&e.shape).iter() {
            let r = simplify(&e.with(part.clone()), tol)?;
            removed += r.done;
            deviation = js_max(deviation, r.deviation);
            shapes.push(r.entity.shape);
        }
        return Some(Reshaped {
            entity: e.with(join_parts(&shapes)?),
            done: removed,
            skipped: 0,
            deviation,
        });
    }
    let (shape, removed, deviation) = match &e.shape {
        Shape::Polyline {
            pts, bulges, holes, ..
        } => {
            let (p, removed, dev) = simplify_path(pts, bulges.as_deref(), false, tol);
            (
                Shape::Polyline {
                    pts: p.pts,
                    bulges: p.bulges,
                    holes: holes.clone(),
                    parts: None,
                },
                removed,
                dev,
            )
        }
        Shape::Polygon {
            pts, bulges, holes, ..
        } => {
            let (p, mut removed, mut dev) = simplify_path(pts, bulges.as_deref(), true, tol);
            let holes = holes.as_ref().map(|hs| {
                hs.iter()
                    .map(|h| {
                        let (hp, r, d) = simplify_path(&h.pts, h.bulges.as_deref(), true, tol);
                        removed += r;
                        dev = js_max(dev, d);
                        Ring {
                            pts: hp.pts,
                            bulges: hp.bulges,
                        }
                    })
                    .collect()
            });
            (
                Shape::Polygon {
                    pts: p.pts,
                    bulges: p.bulges,
                    holes,
                    parts: None,
                },
                removed,
                dev,
            )
        }
        _ => return None,
    };
    Some(Reshaped {
        entity: e.with(shape),
        done: removed,
        skipped: 0,
        deviation,
    })
}

pub(crate) static OPS: &[Op] = &[
    op!("allCorners", |e: Entity, op: CornerOp| all_corners(&e, &op)),
    op!("reverseEntity", |e: Entity| reverse(&e)),
    op!("simplifyEntity", |e: Entity, tol: f64| simplify(&e, tol)),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::json::{FromJson, Json};

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }

    fn polygon(pts: &[(f64, f64)]) -> Entity {
        Entity::new(Shape::Polygon {
            pts: pts.iter().map(|&(x, y)| v(x, y)).collect(),
            bulges: None,
            holes: None,
            parts: None,
        })
    }

    fn polyline(pts: &[(f64, f64)]) -> Entity {
        Entity::new(Shape::Polyline {
            pts: pts.iter().map(|&(x, y)| v(x, y)).collect(),
            bulges: None,
            holes: None,
            parts: None,
        })
    }

    fn arcs(e: &Entity) -> usize {
        match &e.shape {
            Shape::Polygon { bulges, .. } | Shape::Polyline { bulges, .. } => bulges
                .as_ref()
                .map_or(0, |b| b.iter().filter(|x| is_arc_bulge(**x)).count()),
            _ => 0,
        }
    }

    fn count(e: &Entity) -> usize {
        match &e.shape {
            Shape::Polygon { pts, .. } | Shape::Polyline { pts, .. } => pts.len(),
            _ => 0,
        }
    }

    #[test]
    fn every_corner_of_a_rectangle_is_rounded_or_cut() {
        let rect = polygon(&[(0.0, 0.0), (20.0, 0.0), (20.0, 10.0), (0.0, 10.0)]);
        let r = all_corners(&rect, &CornerOp::Radius(2.0)).expect("rounded");
        assert_eq!((r.done, r.skipped), (4, 0));
        assert_eq!(count(&r.entity), 8);
        assert_eq!(arcs(&r.entity), 4);
        let c = all_corners(&rect, &CornerOp::Chamfer(1.0, 2.0)).expect("cut");
        assert_eq!((c.done, c.skipped), (4, 0));
        assert_eq!((count(&c.entity), arcs(&c.entity)), (8, 0));
    }

    #[test]
    fn corners_that_share_a_short_edge_are_both_left() {
        // Radius 6 needs 6 m of each edge: the 10 m sides cannot hold two, the 20 m ones can.
        let rect = polygon(&[(0.0, 0.0), (20.0, 0.0), (20.0, 10.0), (0.0, 10.0)]);
        let r = all_corners(&rect, &CornerOp::Radius(6.0)).expect("answered");
        assert_eq!((r.done, r.skipped), (0, 4));
        assert_eq!(r.entity, rect, "nothing changed");
        // An L: the short end edges hold their corners, whatever the start.
        let l = polygon(&[
            (0.0, 0.0),
            (30.0, 0.0),
            (30.0, 4.0),
            (4.0, 4.0),
            (4.0, 30.0),
            (0.0, 30.0),
        ]);
        let a = all_corners(&l, &CornerOp::Radius(1.5)).expect("rounded");
        let turned = polygon(&[
            (30.0, 4.0),
            (4.0, 4.0),
            (4.0, 30.0),
            (0.0, 30.0),
            (0.0, 0.0),
            (30.0, 0.0),
        ]);
        let b = all_corners(&turned, &CornerOp::Radius(1.5)).expect("rounded");
        assert_eq!((a.done, a.skipped), (b.done, b.skipped));
    }

    #[test]
    fn an_open_paths_ends_and_straight_runs_are_not_corners() {
        let p = polyline(&[
            (0.0, 0.0),
            (10.0, 0.0),
            (20.0, 0.0),
            (20.0, 10.0),
            (30.0, 10.0),
        ]);
        let r = all_corners(&p, &CornerOp::Radius(1.0)).expect("rounded");
        // (10, 0) is on a straight run; the ends are no corners: two corners.
        assert_eq!((r.done, r.skipped), (2, 0));
        assert_eq!(arcs(&r.entity), 2);
    }

    #[test]
    fn a_corner_next_to_an_arc_is_left_and_counted() {
        let p = Entity::new(Shape::Polyline {
            pts: vec![v(0.0, 0.0), v(10.0, 0.0), v(10.0, 10.0), v(0.0, 10.0)],
            bulges: Some(vec![0.0, 0.5, 0.0, 0.0]),
            holes: None,
            parts: None,
        });
        let r = all_corners(&p, &CornerOp::Radius(1.0)).expect("answered");
        // Both vertices of the arc edge meet it at an angle: left.
        assert_eq!((r.done, r.skipped), (0, 2));
    }

    #[test]
    fn a_hole_is_rounded_too_and_bad_values_are_refused() {
        let e = Entity::new(Shape::Polygon {
            pts: vec![v(0.0, 0.0), v(40.0, 0.0), v(40.0, 40.0), v(0.0, 40.0)],
            bulges: None,
            holes: Some(vec![Ring {
                pts: vec![v(10.0, 10.0), v(10.0, 20.0), v(20.0, 20.0), v(20.0, 10.0)],
                bulges: None,
            }]),
            parts: None,
        });
        let r = all_corners(&e, &CornerOp::Radius(1.0)).expect("rounded");
        assert_eq!((r.done, r.skipped), (8, 0));
        assert!(all_corners(&e, &CornerOp::Radius(0.0)).is_none());
        assert!(all_corners(&e, &CornerOp::Chamfer(1.0, 0.0)).is_none());
        let circle = Entity::new(Shape::Circle {
            c: v(0.0, 0.0),
            r: 1.0,
        });
        assert!(all_corners(&circle, &CornerOp::Radius(1.0)).is_none());
    }

    #[test]
    fn reversing_twice_gives_the_object_back_and_arcs_turn_round() {
        let p = Entity::new(Shape::Polyline {
            pts: vec![v(0.0, 0.0), v(10.0, 0.0), v(10.0, 10.0)],
            bulges: Some(vec![0.4, 0.0, 0.0]),
            holes: None,
            parts: None,
        });
        let r = reverse(&p).expect("reversed");
        let Shape::Polyline { pts, bulges, .. } = &r.shape else {
            panic!("polyline");
        };
        assert_eq!(pts, &vec![v(10.0, 10.0), v(10.0, 0.0), v(0.0, 0.0)]);
        assert_eq!(bulges.as_deref(), Some(&[0.0, -0.4, 0.0][..]));
        assert_eq!(reverse(&r).expect("back"), p);
        let line = Entity::new(Shape::Line {
            a: v(0.0, 0.0),
            b: v(1.0, 2.0),
        });
        assert_eq!(
            reverse(&line).expect("line").shape,
            Shape::Line {
                a: v(1.0, 2.0),
                b: v(0.0, 0.0)
            }
        );
        let arc = Entity::new(Shape::Arc {
            c: v(0.0, 0.0),
            r: 1.0,
            a0: 0.0,
            a1: 1.0,
        });
        assert!(reverse(&arc).is_none(), "arcs always run counter-clockwise");
        let square = polygon(&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]);
        assert_eq!(
            reverse(&reverse(&square).expect("once")).expect("twice"),
            square
        );
    }

    #[test]
    fn simplifying_drops_vertices_within_the_tolerance_and_keeps_arcs() {
        let p = polyline(&[
            (0.0, 0.0),
            (5.0, 0.004),
            (10.0, 0.0),
            (15.0, 3.0),
            (20.0, 0.0),
        ]);
        let s = simplify(&p, 0.01).expect("simplified");
        assert_eq!(s.done, 1);
        assert!((s.deviation - 0.004).abs() < 1e-12);
        assert_eq!(count(&s.entity), 4);
        // Nothing within 1 mm: unchanged.
        let none = simplify(&p, 0.001).expect("answered");
        assert_eq!((none.done, &none.entity), (0, &p));
        // The ends of an arc edge stay.
        let a = Entity::new(Shape::Polyline {
            pts: vec![v(0.0, 0.0), v(5.0, 0.001), v(10.0, 0.0), v(20.0, 0.0)],
            bulges: Some(vec![0.0, 0.0, 0.3, 0.0]),
            holes: None,
            parts: None,
        });
        let s = simplify(&a, 0.01).expect("simplified");
        let Shape::Polyline { pts, bulges, .. } = &s.entity.shape else {
            panic!("polyline");
        };
        assert_eq!(pts, &vec![v(0.0, 0.0), v(10.0, 0.0), v(20.0, 0.0)]);
        assert_eq!(bulges.as_deref(), Some(&[0.0, 0.3, 0.0][..]));
    }

    #[test]
    fn a_ring_keeps_three_vertices_at_least() {
        // A sliver: everything within the tolerance, but an area needs three vertices.
        let sliver = polygon(&[(0.0, 0.0), (10.0, 0.0), (10.0, 0.001), (0.0, 0.001)]);
        let s = simplify(&sliver, 1.0).expect("answered");
        assert_eq!(count(&s.entity), 3);
        assert!(simplify(&sliver, 0.0).is_none());
    }

    #[test]
    fn the_ops_answer_through_json() {
        let e = Entity::from_json(
            &Json::parse(r#"{"id":7,"layerId":"a","kind":"polygon","pts":[{"x":0,"y":0},{"x":20,"y":0},{"x":20,"y":10},{"x":0,"y":10}]}"#)
                .expect("json"),
        )
        .expect("entity");
        let r = all_corners(&e, &CornerOp::Radius(2.0)).expect("rounded");
        let mut out = String::new();
        r.write_json(&mut out);
        assert!(
            out.starts_with(r#"{"entity":{"id":7,"layerId":"a","kind":"polygon""#),
            "{out}"
        );
        assert!(
            out.ends_with(r#""done":4,"skipped":0,"deviation":0}"#),
            "{out}"
        );
    }
}
