//! Bounded endpoint constraints must use genuine clamped NURBS endpoints and preserve source data.
use super::tests::{c, dim, r, solve_of};
use super::*;
use crate::vec2::Vec2;
const START: Element = Element::Point(PointAt::Start);
const END: Element = Element::Point(PointAt::End);
fn curve(degree: usize, origin: Vec2) -> Shape {
    let count = degree + 2;
    let ctrl: Vec<_> = (0..count)
        .map(|i| {
            Vec2::new(
                origin.x + 6. * i as f64 / (count - 1) as f64,
                origin.y + if i == 0 || i + 1 == count { 0. } else { 3. },
            )
        })
        .collect();
    let mut knots = vec![2.; degree + 1];
    knots.push(3.);
    knots.extend(vec![8.; degree + 1]);
    Shape::Nurbs {
        degree,
        ctrl: ctrl.clone(),
        knots,
        weights: Some((0..count).map(|i| 1. + i as f64 / 3.).collect()),
        closed: false,
        periodic: false,
        fit: Some(ctrl),
        fit_tolerance: Some(1e-5),
        tangents: Some([Some(Vec2::new(1., 2.)), Some(Vec2::new(1., -2.))]),
    }
}
#[test]
fn clamped_rational_nurbs_endpoint_changes_preserve_every_interior_pole_and_source_knot() {
    for degree in [1, 2, 3, 7] {
        let origin = Vec2::new(3e6, 1e7);
        let shapes = [curve(degree, origin)];
        let out = solve_of(
            &shapes,
            &[
                c(Rule::Fix { a: r(0, START) }),
                c(Rule::Horizontal {
                    a: r(0, START),
                    b: Some(r(0, END)),
                }),
                dim(
                    Rule::Distance {
                        a: r(0, START),
                        b: r(0, END),
                        axis: Axis::Aligned,
                        left: false,
                    },
                    8.,
                ),
            ],
        );
        assert!(out.status.holds(), "degree={degree}: {out:?}");
        assert!(out.broken.is_empty(), "{out:?}");
        assert_eq!(
            out.dof, 0,
            "only endpoint coordinates participate in this adapter"
        );
        let Shape::Nurbs {
            ctrl,
            knots,
            weights,
            degree: actual,
            closed,
            periodic,
            fit,
            fit_tolerance,
            tangents,
        } = &out.shapes[0]
        else {
            panic!()
        };
        let Shape::Nurbs {
            ctrl: old_ctrl,
            knots: old_knots,
            weights: old_weights,
            ..
        } = &shapes[0]
        else {
            panic!()
        };
        assert_eq!(*actual, degree);
        assert_eq!(knots, old_knots);
        assert_eq!(weights, old_weights);
        assert!(!closed && !periodic);
        assert_eq!(&ctrl[..ctrl.len() - 1], &old_ctrl[..old_ctrl.len() - 1]);
        assert_eq!(ctrl.last(), Some(&Vec2::new(origin.x + 8., origin.y)));
        assert!(fit.is_none() && fit_tolerance.is_none() && tangents.is_none());
        let evaluated = crate::geom::nurbs::of(&out.shapes[0]).unwrap();
        let (a, b) = evaluated.domain();
        assert_eq!(evaluated.point(a), origin);
        let end = evaluated.point(b);
        assert!((end.x - origin.x - 8.).abs() < 1e-8 && (end.y - origin.y).abs() < 1e-8);
    }
}
#[test]
fn unclamped_overclamped_closed_and_periodic_nurbs_are_never_misread_as_endpoint_poles() {
    let original = curve(3, Vec2::new(0., 0.));
    for kind in 0..4 {
        let mut shape = original.clone();
        let Shape::Nurbs {
            knots,
            closed,
            periodic,
            ..
        } = &mut shape
        else {
            panic!()
        };
        match kind {
            0 => knots[0] = 1.,
            1 => knots[4] = 2.,
            2 => *closed = true,
            _ => *periodic = true,
        }
        assert!(!has_part(&shape, &START), "kind={kind}");
        assert!(!has_part(&shape, &END), "kind={kind}");
    }
    let Shape::Nurbs {
        mut knots,
        degree,
        ctrl,
        weights,
        ..
    } = original
    else {
        panic!()
    };
    knots[0] = 0.;
    knots[1] = 0.5;
    knots[2] = 1.;
    let n = crate::geom::nurbs::Nurbs {
        degree,
        ctrl: &ctrl,
        knots: &knots,
        weights: weights.as_deref(),
    };
    assert!(n.valid());
    assert_ne!(
        n.point(n.domain().0),
        ctrl[0],
        "the first pole is not generally the curve endpoint"
    );
}
#[test]
fn unchanged_nurbs_constraint_keeps_fit_metadata_and_one_endpoint_hold_leaves_other_end_free() {
    let shapes = [curve(3, Vec2::new(0., 0.))];
    let out = solve_of(&shapes, &[c(Rule::Fix { a: r(0, START) })]);
    assert!(out.status.holds());
    assert_eq!(out.shapes, shapes);
    assert_eq!(out.dof, 2);
    let constraints = [dim(
        Rule::Distance {
            a: r(0, START),
            b: r(0, END),
            axis: Axis::Horizontal,
            left: false,
        },
        8.,
    )];
    let out = solve(
        &Problem {
            shapes: &shapes,
            constraints: &constraints,
            held: &[Hold {
                object: 0,
                element: Some(START),
            }],
            fix_from: None,
        },
        &|| false,
    );
    assert!(out.status.holds(), "{out:?}");
    let Shape::Nurbs { ctrl, .. } = &out.shapes[0] else {
        panic!()
    };
    assert_eq!(ctrl[0], Vec2::new(0., 0.));
    assert_eq!(*ctrl.last().unwrap(), Vec2::new(8., 0.));
    assert!(!has_part(&shapes[0], &Element::Circle));
    assert!(!has_part(
        &shapes[0],
        &Element::Segment {
            part: 0,
            ring: 0,
            index: 0
        }
    ));
}
