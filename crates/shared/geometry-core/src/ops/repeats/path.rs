//! Analytic, edge-aligned comparison of single-part 2D polylines.
//! A changed starting vertex or reversed direction does not change a closed path's geometry.
use super::{PATH_VERTEX_LIMIT, Shape, Vec2, finite};
use crate::geom::bulge::{bulge_arc, bulge_at, is_arc_bulge, segment_mid};
use crate::geometry::dist;
use crate::jsmath::js_max;

#[derive(Clone, Copy)]
struct Path<'a> {
    pts: &'a [Vec2],
    bulges: Option<&'a [f64]>,
    widths: Option<&'a [[f64; 2]]>,
    closed: bool,
}

fn view(shape: &Shape) -> Option<Path<'_>> {
    let Shape::Polyline {
        pts,
        bulges,
        widths,
        closed,
        holes,
        parts,
    } = shape
    else {
        return None;
    };
    if holes.as_ref().is_some_and(|v| !v.is_empty())
        || parts.as_ref().is_some_and(|v| !v.is_empty())
    {
        return None;
    }
    Some(Path {
        pts,
        bulges: bulges.as_deref(),
        widths: widths.as_deref(),
        closed: *closed,
    })
}

impl Path<'_> {
    fn edges(self) -> usize {
        self.pts.len() - usize::from(!self.closed)
    }
    fn width(self, edge: usize) -> [f64; 2] {
        self.widths
            .and_then(|v| v.get(edge))
            .copied()
            .unwrap_or([0.0; 2])
    }
}

pub(super) fn vertices(shape: &Shape) -> usize {
    match shape {
        Shape::Polyline { pts, .. } => pts.len(),
        _ => 0,
    }
}

pub(super) fn supported(shape: &Shape) -> bool {
    let Some(p) = view(shape) else {
        return false;
    };
    if !(2..=PATH_VERTEX_LIMIT).contains(&p.pts.len())
        || !p.pts.iter().all(|v| finite(*v))
        || p.bulges
            .is_some_and(|v| v.len() > p.pts.len() || v.iter().any(|b| !b.is_finite()))
        || p.widths.is_some_and(|v| {
            v.len() != p.pts.len() || v.iter().flatten().any(|w| !w.is_finite() || *w < 0.0)
        })
    {
        return false;
    }
    let mut directed = false;
    for i in 0..p.edges() {
        let (a, b) = (p.pts[i], p.pts[(i + 1) % p.pts.len()]);
        directed |= dist(a, b) > 0.0;
        let bulge = bulge_at(p.bulges, i);
        if is_arc_bulge(bulge)
            && !bulge_arc(a, b, bulge).is_some_and(|g| finite(g.c) && g.r.is_finite())
        {
            return false;
        }
    }
    directed
}

/// Vertex mean, independent of path direction/start, computed relative to a local vertex.
pub(super) fn centre(shape: &Shape) -> Option<Vec2> {
    let p = view(shape)?;
    let first = *p.pts.first()?;
    let n = p.pts.len() as f64;
    let mut x = 0.0;
    let mut y = 0.0;
    for v in p.pts {
        x += (v.x - first.x) / n;
        y += (v.y - first.y) / n;
    }
    Some(Vec2::new(first.x + x, first.y + y))
}

pub(super) fn anchor(shape: &Shape) -> Option<(Vec2, Vec2)> {
    let p = view(shape)?;
    (0..p.edges()).find_map(|i| {
        let (a, b) = (p.pts[i], p.pts[(i + 1) % p.pts.len()]);
        (dist(a, b) > 0.0).then_some((a, b))
    })
}

pub(super) fn pairs(shape: &Shape) -> Vec<(Vec2, Vec2)> {
    let Some(p) = view(shape) else {
        return Vec::new();
    };
    (0..p.edges())
        .flat_map(|i| {
            let (a, b) = (p.pts[i], p.pts[(i + 1) % p.pts.len()]);
            [(a, b), (b, a)]
        })
        .filter(|(a, b)| dist(*a, *b) > 0.0)
        .collect()
}

fn same_arc((a, b, ab): (Vec2, Vec2, f64), (c, d, cd): (Vec2, Vec2, f64), t: f64) -> bool {
    match (is_arc_bulge(ab), is_arc_bulge(cd)) {
        (false, false) => true,
        (true, true) => {
            let (Some(x), Some(y)) = (bulge_arc(a, b, ab), bulge_arc(c, d, cd)) else {
                return false;
            };
            dist(x.c, y.c) <= t
                && (x.r - y.r).abs() <= t
                && (x.sweep - y.sweep).abs() * js_max(x.r, y.r) <= t
                && dist(segment_mid(a, b, ab), segment_mid(c, d, cd)) <= t
        }
        _ => false,
    }
}

/// Visits every complete analytic vertex correspondence. Returning false from `found`
/// ends the scan successfully; None means the work/cancellation gate stopped it.
fn alignments(
    a: &Shape,
    b: &Shape,
    tolerance: f64,
    check: &mut impl FnMut() -> bool,
    mut found: impl FnMut(usize, bool) -> bool,
) -> Option<()> {
    let (Some(a), Some(b)) = (view(a), view(b)) else {
        return Some(());
    };
    let n = a.pts.len();
    if n != b.pts.len() || a.closed != b.closed {
        return Some(());
    }
    for reverse in [false, true] {
        let starts = if a.closed {
            0..n
        } else if reverse {
            n - 1..n
        } else {
            0..1
        };
        for start in starts {
            if !check() {
                return None;
            }
            if dist(a.pts[0], b.pts[start]) > tolerance {
                continue;
            }
            let mut same = true;
            for i in 0..a.edges() {
                if !check() {
                    return None;
                }
                let j = if reverse {
                    (start + n - i) % n
                } else {
                    (start + i) % n
                };
                let k = if reverse {
                    (j + n - 1) % n
                } else {
                    (j + 1) % n
                };
                let an = (i + 1) % n;
                let edge = if reverse { k } else { j };
                let [w0, w1] = a.width(i);
                let [v0, v1] = b.width(edge);
                let (v0, v1) = if reverse { (v1, v0) } else { (v0, v1) };
                let bulge = bulge_at(b.bulges, edge) * if reverse { -1.0 } else { 1.0 };
                if dist(a.pts[i], b.pts[j]) > tolerance
                    || dist(a.pts[an], b.pts[k]) > tolerance
                    || (w0 - v0).abs() > tolerance
                    || (w1 - v1).abs() > tolerance
                    || !same_arc(
                        (a.pts[i], a.pts[an], bulge_at(a.bulges, i)),
                        (b.pts[j], b.pts[k], bulge),
                        tolerance,
                    )
                {
                    same = false;
                    break;
                }
            }
            if same && !found(start, reverse) {
                return Some(());
            }
        }
    }
    Some(())
}

/// None means that the caller's work/cancellation gate stopped the comparison.
pub(super) fn equal(
    a: &Shape,
    b: &Shape,
    tolerance: f64,
    check: &mut impl FnMut() -> bool,
) -> Option<bool> {
    let mut same = false;
    alignments(a, b, tolerance, check, |_, _| {
        same = true;
        false
    })?;
    Some(same)
}

/// A target place mapped to the replacement's vertex/edge numbering. All complete
/// alignments must agree: coincident vertices are not permission to guess a topology.
pub(super) fn map_place(
    replacement: &Shape,
    target: &Shape,
    place: crate::ops::subref::Place,
    tolerance: f64,
    check: &mut impl FnMut() -> bool,
) -> super::PlaceMatch {
    use super::PlaceMatch;
    use crate::ops::subref::Place;
    let Some(p) = view(target) else {
        return PlaceMatch::NoMatch;
    };
    let (index, edge) = match place {
        Place::Vertex {
            part: 0,
            ring: 0,
            index,
        } if index < p.pts.len() => (index, false),
        Place::Mid {
            part: 0,
            ring: 0,
            index,
        } if index < p.edges() => (index, true),
        Place::Along {
            part: 0,
            ring: 0,
            index,
            t,
        } if index < p.edges() && (0.0..=1.0).contains(&t) => (index, true),
        _ => return PlaceMatch::NoMatch,
    };
    let n = p.pts.len();
    let mut result = PlaceMatch::NoMatch;
    let done = alignments(replacement, target, tolerance, check, |start, reverse| {
        // Alignment maps replacement i to target start +/- i. A reversed target
        // edge begins at the corresponding replacement edge's END, not its start.
        let j = if edge && reverse {
            (index + 1) % n
        } else {
            index
        };
        let i = if reverse {
            (start + n - j) % n
        } else {
            (j + n - start) % n
        };
        let mapped = match place {
            Place::Vertex { .. } => Place::Vertex {
                part: 0,
                ring: 0,
                index: i,
            },
            Place::Mid { .. } => Place::Mid {
                part: 0,
                ring: 0,
                index: i,
            },
            Place::Along { t, .. } => Place::Along {
                part: 0,
                ring: 0,
                index: i,
                t: if reverse { 1.0 - t } else { t },
            },
            _ => unreachable!(),
        };
        match result {
            PlaceMatch::NoMatch => result = PlaceMatch::At(mapped),
            PlaceMatch::At(previous) if previous != mapped => result = PlaceMatch::Ambiguous,
            _ => {}
        }
        result != PlaceMatch::Ambiguous
    });
    if done.is_none() {
        PlaceMatch::Stopped
    } else {
        result
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Error, Options, VERTEX_LIMIT, find};
    use super::*;

    fn path() -> Shape {
        Shape::Polyline {
            pts: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(2.0, 0.0),
                Vec2::new(1.5, 1.0),
            ],
            bulges: Some(vec![0.25, 0.0, -0.1]),
            widths: Some(vec![[0.1, 0.2], [0.2, 0.3], [0.3, 0.1]]),
            closed: true,
            holes: None,
            parts: None,
        }
    }

    #[test]
    fn cyclic_reverse_keeps_the_edge_bulge_and_each_width_endpoint() {
        let a = path();
        let b = Shape::Polyline {
            pts: vec![
                Vec2::new(2.0, 0.0),
                Vec2::new(0.0, 0.0),
                Vec2::new(1.5, 1.0),
            ],
            bulges: Some(vec![-0.25, 0.1, 0.0]),
            widths: Some(vec![[0.2, 0.1], [0.1, 0.3], [0.3, 0.2]]),
            closed: true,
            holes: None,
            parts: None,
        };
        assert!(supported(&a) && supported(&b));
        assert_eq!(equal(&a, &b, 1e-6, &mut || true), Some(true));
        // A distinct taper, curvature and closure each change the actual path.
        let mut changed = b.clone();
        if let Shape::Polyline { widths, .. } = &mut changed {
            widths.as_mut().unwrap()[0][0] += 0.01;
        }
        assert_eq!(equal(&a, &changed, 1e-6, &mut || true), Some(false));
        let mut changed = b.clone();
        if let Shape::Polyline { bulges, .. } = &mut changed {
            bulges.as_mut().unwrap()[0] = -0.2;
        }
        assert_eq!(equal(&a, &changed, 1e-6, &mut || true), Some(false));
        let mut changed = b.clone();
        if let Shape::Polyline { closed, .. } = &mut changed {
            *closed = false;
        }
        assert_eq!(equal(&a, &changed, 1e-6, &mut || true), Some(false));
        assert_eq!(equal(&a, &b, 1e-6, &mut || false), None);
    }

    // Deliberately independent of transform_shape/reverse_bulge_path: reconstruct every
    // vertex, arc sign and taper endpoint from the expected source correspondence.
    fn placed(i: usize, closed: bool, scale: f64) -> Shape {
        let Shape::Polyline {
            pts,
            bulges,
            widths,
            ..
        } = path()
        else {
            unreachable!()
        };
        let angle = i as f64 * 0.17;
        let (s, c) = (crate::jsmath::sin(angle), crate::jsmath::cos(angle));
        let mirror = i % 2 == 1;
        let reverse = i % 3 == 1;
        let start = if closed {
            i % 3
        } else if reverse {
            2
        } else {
            0
        };
        let mut p = Vec::new();
        let mut b = Vec::new();
        let mut w = Vec::new();
        for k in 0..3 {
            let j = if reverse {
                (start + 3 - k) % 3
            } else {
                (start + k) % 3
            };
            let v = pts[j];
            let y = if mirror { -v.y } else { v.y };
            p.push(Vec2::new(
                452345.125 + (i % 10) as f64 * 10.0 + scale * (v.x * c - y * s),
                4412345.25 + (i / 10) as f64 * 10.0 + scale * (v.x * s + y * c),
            ));
            let edge = if reverse { (j + 2) % 3 } else { j };
            let value = bulges.as_ref().unwrap()[edge];
            b.push(value * if mirror { -1.0 } else { 1.0 } * if reverse { -1.0 } else { 1.0 });
            let [a, z] = widths.as_ref().unwrap()[edge];
            w.push(if reverse {
                [z * scale, a * scale]
            } else {
                [a * scale, z * scale]
            });
        }
        if !closed {
            b[2] = 0.0;
            w[2] = [0.0; 2];
        }
        Shape::Polyline {
            pts: p,
            bulges: Some(b),
            widths: Some(w),
            closed,
            holes: None,
            parts: None,
        }
    }

    #[test]
    fn forty_survey_coordinate_curved_tapered_rings_match_across_start_and_direction() {
        let mut scene: Vec<_> = (0..40).map(|i| placed(i, true, 1.0)).collect();
        scene.push(placed(40, true, 2.0));
        let mut wrong_width = placed(41, true, 1.0);
        if let Shape::Polyline { widths, .. } = &mut wrong_width {
            widths.as_mut().unwrap()[0][0] += 0.01;
        }
        scene.push(wrong_width);
        let mut wrong_arc = placed(42, true, 1.0);
        if let Shape::Polyline { bulges, .. } = &mut wrong_arc {
            bulges.as_mut().unwrap()[0] += 0.01;
        }
        scene.push(wrong_arc);
        scene.push(placed(43, false, 1.0));
        let options = Options {
            mirrors: true,
            ..Default::default()
        };
        let a = find(&scene, &[0], options, |_, _| true, || false).unwrap();
        assert_eq!(a.matches.len(), 40);
        assert!(!a.truncated);
        assert_eq!(
            a.matches.iter().map(|m| m.members[0]).collect::<Vec<_>>(),
            (0..40).collect::<Vec<_>>()
        );
        assert_eq!(
            a,
            find(&scene, &[0], options, |_, _| true, || false).unwrap()
        );
        let b = find(
            &scene,
            &[0],
            Options {
                scales: true,
                ..options
            },
            |_, _| true,
            || false,
        )
        .unwrap();
        assert_eq!(b.matches.len(), 41);
        assert!((b.matches[40].scale - 2.0).abs() < 1e-8);
        assert_eq!(
            find(&scene, &[0], Options::default(), |_, _| true, || false)
                .unwrap()
                .matches
                .len(),
            20
        );
    }

    #[test]
    fn open_direction_widths_work_budget_and_preparation_limits_are_explicit() {
        let a = Shape::Polyline {
            pts: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(2.0, 0.0),
                Vec2::new(3.0, 1.0),
            ],
            bulges: Some(vec![0.25, 0.0]),
            widths: Some(vec![[0.1, 0.2], [0.3, 0.4], [0.0; 2]]),
            closed: false,
            holes: None,
            parts: None,
        };
        let b = Shape::Polyline {
            pts: vec![
                Vec2::new(3.0, 1.0),
                Vec2::new(2.0, 0.0),
                Vec2::new(0.0, 0.0),
            ],
            bulges: Some(vec![0.0, -0.25]),
            widths: Some(vec![[0.4, 0.3], [0.2, 0.1], [0.0; 2]]),
            closed: false,
            holes: None,
            parts: None,
        };
        assert_eq!(equal(&a, &b, 1e-6, &mut || true), Some(true));
        assert_eq!(
            find(
                &[a.clone(), b],
                &[0],
                Options::default(),
                |_, _| true,
                || false
            )
            .unwrap()
            .matches
            .len(),
            2
        );
        let limited = find(
            std::slice::from_ref(&a),
            &[0],
            Options {
                budget: 2,
                ..Default::default()
            },
            |_, _| true,
            || false,
        )
        .unwrap();
        assert!(limited.truncated && limited.matches.is_empty());
        let mut calls = 0;
        assert_eq!(
            find(
                std::slice::from_ref(&a),
                &[0],
                Options::default(),
                |_, _| true,
                || {
                    calls += 1;
                    calls > 8
                }
            ),
            Err(Error::Cancelled)
        );
        let mut big = a.clone();
        if let Shape::Polyline {
            pts,
            bulges,
            widths,
            ..
        } = &mut big
        {
            *pts = vec![Vec2::new(1.0, 1.0); 4096];
            *bulges = None;
            *widths = None;
        }
        let mut scene = vec![a];
        scene.extend(vec![big; VERTEX_LIMIT / 4096 + 1]);
        assert_eq!(
            find(&scene, &[0], Options::default(), |_, _| true, || false),
            Err(Error::GeometryLimit)
        );
    }
}
