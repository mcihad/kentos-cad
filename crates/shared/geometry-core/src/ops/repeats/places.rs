//! Topological correspondence for a verified replacement at its final world placement.
//! Ordinary edit following keeps indices; repeat conversion must instead account for
//! reversed lines and cyclic/reversed path numbering before replacing root identities.
use super::{Shape, equal, path, supported};
use crate::geometry::dist;
use crate::ops::subref::Place;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlaceMatch {
    At(Place),
    NoMatch,
    /// Several complete geometric correspondences disagree on the named place.
    Ambiguous,
    /// Cancellation or work budget: never treat an incomplete scan as unambiguous.
    Stopped,
}

/// Maps a place on `target` to `replacement`, both in the SAME world coordinates.
/// The replacement must already include its actual block placement (including reflection).
/// This does not inverse-transform a circle's world-east fraction or re-reverse a reflected
/// arc: block subplaces are evaluated on the canonical, placed geometry too.
/// Only the repeat matcher's supported analytic shapes are accepted. `check` is the
/// caller's shared bounded work/cancellation gate, true to continue.
pub fn map_place(
    replacement: &Shape,
    target: &Shape,
    place: Place,
    tolerance: f64,
    check: &mut impl FnMut() -> bool,
) -> PlaceMatch {
    if !tolerance.is_finite() || tolerance <= 0.0 || !supported(replacement) || !supported(target) {
        return PlaceMatch::NoMatch;
    }
    if !check() {
        return PlaceMatch::Stopped;
    }
    if matches!(
        (replacement, target),
        (Shape::Polyline { .. }, Shape::Polyline { .. })
    ) {
        return path::map_place(replacement, target, place, tolerance, check);
    }
    match equal(replacement, target, tolerance, check) {
        None => return PlaceMatch::Stopped,
        Some(false) => return PlaceMatch::NoMatch,
        Some(true) => {}
    }
    use PlaceMatch::{Ambiguous, At, NoMatch};
    let whole = |p| {
        matches!(p,
        Place::Along { part:0,ring:0,index:0,t } if (0.0..=1.0).contains(&t))
    };
    match (replacement, target) {
        (Shape::Line { a, b }, Shape::Line { a: c, b: d }) => {
            if !matches!(
                place,
                Place::Start
                    | Place::End
                    | Place::Mid {
                        part: 0,
                        ring: 0,
                        index: 0
                    }
            ) && !whole(place)
            {
                return NoMatch;
            }
            let forward = dist(*a, *c) <= tolerance && dist(*b, *d) <= tolerance;
            let reverse = dist(*a, *d) <= tolerance && dist(*b, *c) <= tolerance;
            let reversed = match place {
                Place::Start => Place::End,
                Place::End => Place::Start,
                Place::Along { t, .. } => Place::Along {
                    part: 0,
                    ring: 0,
                    index: 0,
                    t: 1.0 - t,
                },
                p => p,
            };
            if forward && reverse && place != reversed {
                Ambiguous
            } else {
                At(if forward { place } else { reversed })
            }
        }
        (Shape::Arc { .. }, Shape::Arc { .. })
            if matches!(place, Place::Start | Place::End | Place::Center) || whole(place) =>
        {
            At(place)
        }
        (Shape::Circle { .. }, Shape::Circle { .. }) if place == Place::Center || whole(place) => {
            At(place)
        }
        _ => NoMatch,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::Entity;
    use crate::ops::subref;
    use crate::ops::transform::transform_shape;
    use crate::vec2::Vec2;
    fn v(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }
    fn along(index: usize, t: f64) -> Place {
        Place::Along {
            part: 0,
            ring: 0,
            index,
            t,
        }
    }
    fn vertex(index: usize) -> Place {
        Place::Vertex {
            part: 0,
            ring: 0,
            index,
        }
    }
    fn mid(index: usize) -> Place {
        Place::Mid {
            part: 0,
            ring: 0,
            index,
        }
    }
    fn mapped(a: &Shape, b: &Shape, p: Place) -> PlaceMatch {
        map_place(a, b, p, 1e-6, &mut || true)
    }
    fn path() -> Shape {
        Shape::Polyline {
            pts: vec![v(0., 0.), v(4., 0.), v(0., 4.)],
            bulges: Some(vec![0.25, 0., 0.25]),
            widths: Some(vec![[0.1, 0.2], [0.3, 0.4], [0.5, 0.6]]),
            closed: true,
            holes: None,
            parts: None,
        }
    }
    fn ordered(start: usize, reverse: bool, closed: bool) -> Shape {
        let Shape::Polyline {
            pts,
            bulges,
            widths,
            ..
        } = path()
        else {
            unreachable!()
        };
        let indices: Vec<_> = (0..3)
            .map(|i| {
                if reverse {
                    (start + 3 - i) % 3
                } else {
                    (start + i) % 3
                }
            })
            .collect();
        let mut bs = Vec::new();
        let mut ws = Vec::new();
        for &j in &indices {
            let edge = if reverse { (j + 2) % 3 } else { j };
            bs.push(bulges.as_ref().unwrap()[edge] * if reverse { -1. } else { 1. });
            let w = widths.as_ref().unwrap()[edge];
            ws.push(if reverse { [w[1], w[0]] } else { w });
        }
        if !closed {
            bs[2] = 0.;
            ws[2] = [0.; 2];
        }
        Shape::Polyline {
            pts: indices.iter().map(|i| pts[*i]).collect(),
            bulges: Some(bs),
            widths: Some(ws),
            closed,
            holes: None,
            parts: None,
        }
    }
    #[test]
    fn every_cyclic_reverse_vertex_edge_and_fraction_keeps_its_named_topology() {
        let a = path();
        for start in 0..3 {
            for reverse in [false, true] {
                let b = ordered(start, reverse, true);
                for target in 0..3 {
                    let expected = if reverse {
                        (start + 3 - target) % 3
                    } else {
                        (start + target) % 3
                    };
                    assert_eq!(
                        mapped(&a, &b, vertex(target)),
                        PlaceMatch::At(vertex(expected))
                    );
                    let edge = if reverse {
                        (expected + 2) % 3
                    } else {
                        expected
                    };
                    assert_eq!(mapped(&a, &b, mid(target)), PlaceMatch::At(mid(edge)));
                    for t in [0., 0.125, 0.5, 0.875, 1.] {
                        assert_eq!(
                            mapped(&a, &b, along(target, t)),
                            PlaceMatch::At(along(edge, if reverse { 1. - t } else { t }))
                        );
                    }
                }
            }
        }
        let PlaceMatch::At(p) = mapped(&a, &ordered(1, true, true), mid(1)) else {
            panic!("closing edge")
        };
        // Independent closing bulge midpoint for (0,4)->(0,0), bulge .25.
        assert_eq!(p, mid(2));
        let at = subref::point(&Entity::new(a), p).unwrap();
        assert!((at.x + 0.5).abs() < 1e-12 && (at.y - 2.).abs() < 1e-12);
    }
    #[test]
    fn open_paths_reverse_only_existing_edges_and_reject_invalid_places() {
        let a = ordered(0, false, false);
        let b = ordered(2, true, false);
        assert_eq!(mapped(&a, &b, vertex(0)), PlaceMatch::At(vertex(2)));
        assert_eq!(mapped(&a, &b, mid(0)), PlaceMatch::At(mid(1)));
        assert_eq!(
            mapped(&a, &b, along(1, 0.25)),
            PlaceMatch::At(along(0, 0.75))
        );
        for p in [
            mid(2),
            along(2, 0.5),
            vertex(3),
            Place::Center,
            Place::Start,
            Place::Vertex {
                part: 1,
                ring: 0,
                index: 0,
            },
            Place::Mid {
                part: 0,
                ring: 1,
                index: 0,
            },
            along(0, -0.1),
            along(0, f64::NAN),
        ] {
            assert_eq!(mapped(&a, &b, p), PlaceMatch::NoMatch, "{p:?}");
        }
        let mut changed = b.clone();
        if let Shape::Polyline { widths, .. } = &mut changed {
            widths.as_mut().unwrap()[0][0] += 0.01;
        }
        assert_eq!(mapped(&a, &changed, mid(0)), PlaceMatch::NoMatch);
    }
    #[test]
    fn complete_correspondence_disambiguates_crossing_and_refuses_repeated_traversal() {
        let a = Shape::Polyline {
            pts: vec![
                v(0., 0.),
                v(2., 0.),
                v(2., 1.),
                v(0., 0.),
                v(0., 3.),
                v(-1., 3.),
            ],
            bulges: None,
            widths: None,
            closed: true,
            holes: None,
            parts: None,
        };
        // A repeated vertex alone is not ambiguous: its surrounding full traversal differs.
        assert_eq!(mapped(&a, &a, vertex(3)), PlaceMatch::At(vertex(3)));
        let b = Shape::Polyline {
            pts: vec![
                v(0., 0.),
                v(2., 0.),
                v(0., 3.),
                v(0., 0.),
                v(2., 0.),
                v(0., 3.),
            ],
            bulges: None,
            widths: None,
            closed: true,
            holes: None,
            parts: None,
        };
        assert_eq!(mapped(&b, &b, vertex(0)), PlaceMatch::Ambiguous);
        assert_eq!(mapped(&b, &b, mid(2)), PlaceMatch::Ambiguous);
        let mut calls = 0;
        let result = map_place(&b, &b, mid(2), 1e-6, &mut || {
            calls += 1;
            calls <= 9
        });
        assert_eq!(
            result,
            PlaceMatch::Stopped,
            "partial first alignment cannot be accepted"
        );
    }
    #[test]
    fn reversed_lines_keep_mid_but_flip_endpoints_and_fraction_and_report_tolerance_ambiguity() {
        let a = Shape::Line {
            a: v(0., 0.),
            b: v(4., 0.),
        };
        let b = Shape::Line {
            a: v(4., 0.),
            b: v(0., 0.),
        };
        assert_eq!(mapped(&a, &b, Place::Start), PlaceMatch::At(Place::End));
        assert_eq!(mapped(&a, &b, Place::End), PlaceMatch::At(Place::Start));
        assert_eq!(mapped(&a, &b, mid(0)), PlaceMatch::At(mid(0)));
        assert_eq!(
            mapped(&a, &b, along(0, 0.125)),
            PlaceMatch::At(along(0, 0.875))
        );
        let tiny = Shape::Line {
            a: v(0., 0.),
            b: v(0.0000001, 0.),
        };
        assert_eq!(mapped(&tiny, &tiny, Place::Start), PlaceMatch::Ambiguous);
        assert_eq!(mapped(&tiny, &tiny, mid(0)), PlaceMatch::At(mid(0)));
        assert_eq!(mapped(&a, &b, vertex(0)), PlaceMatch::NoMatch);
    }
    #[test]
    fn placed_arc_and_circle_use_the_world_shape_not_an_inverse_local_angle() {
        for mirror in [false, true] {
            for scale in [0.25, 1., 4.] {
                for angle in [0., 0.7, 2.3] {
                    let m =
                        crate::block::placement(v(452345.125, 4412345.25), scale, angle, mirror);
                    for original in [
                        Shape::Circle {
                            c: v(2., 3.),
                            r: 4.,
                        },
                        Shape::Arc {
                            c: v(2., 3.),
                            r: 4.,
                            a0: 0.2,
                            a1: 2.3,
                        },
                    ] {
                        let a = transform_shape(&original, &m);
                        for p in [
                            Place::Center,
                            along(0, 0.),
                            along(0, 0.125),
                            along(0, 0.5),
                            along(0, 1.),
                        ] {
                            assert_eq!(mapped(&a, &a, p), PlaceMatch::At(p));
                        }
                        if matches!(a, Shape::Arc { .. }) {
                            assert_eq!(mapped(&a, &a, Place::Start), PlaceMatch::At(Place::Start));
                            assert_eq!(mapped(&a, &a, Place::End), PlaceMatch::At(Place::End));
                        } else {
                            assert_eq!(mapped(&a, &a, Place::Start), PlaceMatch::NoMatch);
                        }
                    }
                }
            }
        }
        let a = path();
        assert_eq!(
            map_place(&a, &a, mid(0), 1e-6, &mut || false),
            PlaceMatch::Stopped
        );
        for tolerance in [0., -1., f64::NAN, f64::INFINITY] {
            assert_eq!(
                map_place(&a, &a, mid(0), tolerance, &mut || true),
                PlaceMatch::NoMatch
            );
        }
    }
}
