//! Ellipse constraints: independent geometric residuals, local origins and endpoint Jacobians.
use super::tests::{c, dim, r, solve_of};
use super::*;
use crate::geom::ellipse::{EllipseGeom, ellipse_point};
use crate::jsmath::{PI, TAU, cos, js_hypot, sin};
use crate::vec2::Vec2;
const CENTER: Element = Element::Point(PointAt::Center);
const START: Element = Element::Point(PointAt::Start);
const END: Element = Element::Point(PointAt::End);
fn ellipse(c: Vec2, a: f64, b: f64, angle: f64, t0: f64, t1: f64) -> Shape {
    Shape::Ellipse {
        c,
        major: Vec2::new(a * cos(angle), a * sin(angle)),
        ratio: b / a,
        t0,
        t1,
    }
}
fn point(p: Vec2) -> Shape {
    Shape::Point {
        p,
        z: None,
        parts: None,
    }
}
fn at(s: &Shape, t: f64) -> Vec2 {
    let Shape::Ellipse {
        c,
        major,
        ratio,
        t0,
        t1,
    } = s
    else {
        panic!("ellipse")
    };
    ellipse_point(
        &EllipseGeom {
            c: *c,
            major: *major,
            ratio: *ratio,
            t0: *t0,
            t1: *t1,
        },
        t,
    )
}
fn close(a: Vec2, b: Vec2) {
    assert!(js_hypot(a.x - b.x, a.y - b.y) < 1e-8, "{a:?} != {b:?}");
}
#[test]
fn ellipse_center_at_large_world_coordinates_preserves_axes_and_closedness() {
    let s = ellipse(Vec2::new(3e6, 1e7), 4., 2., 0.4, 0.7, 0.7);
    let target = Vec2::new(3e6 + 7., 1e7 - 3.);
    let shapes = [s.clone(), point(target)];
    let out = solve_of(
        &shapes,
        &[
            c(Rule::Fix {
                a: r(1, Element::Point(PointAt::Node(0))),
            }),
            c(Rule::Coincident {
                a: r(0, CENTER),
                b: r(1, Element::Point(PointAt::Node(0))),
            }),
        ],
    );
    assert_eq!(out.status, Status::Under, "{out:?}");
    assert_eq!(out.dof, 3);
    let mut expected = s;
    if let Shape::Ellipse { c, .. } = &mut expected {
        *c = target;
    }
    assert_eq!(out.shapes[0], expected);
}
#[test]
fn full_ellipse_axis_is_driven_by_real_seam_point_without_opening_curve() {
    let shapes = [ellipse(Vec2::new(1., 2.), 4., 2., 0.2, 0., TAU)];
    let out = solve_of(
        &shapes,
        &[
            c(Rule::Fix { a: r(0, CENTER) }),
            c(Rule::Horizontal {
                a: r(0, CENTER),
                b: Some(r(0, START)),
            }),
            dim(
                Rule::Distance {
                    a: r(0, CENTER),
                    b: r(0, START),
                    axis: Axis::Aligned,
                    left: false,
                },
                6.,
            ),
        ],
    );
    assert!(out.status.holds(), "{out:?}");
    assert_eq!(out.dof, 1);
    close(at(&out.shapes[0], 0.), Vec2::new(7., 2.));
    let Shape::Ellipse {
        major,
        ratio,
        t0,
        t1,
        ..
    } = &out.shapes[0]
    else {
        panic!()
    };
    assert!((js_hypot(major.x, major.y) * ratio - 2.).abs() < 1e-9);
    assert_eq!((*t0, *t1), (0., TAU));
}
#[test]
fn elliptical_arc_end_changes_minor_axis_with_center_and_other_end_fixed() {
    let target = Vec2::new(0., 3.);
    let shapes = [
        ellipse(Vec2::new(0., 0.), 4., 2., 0., 0., PI / 2.),
        point(target),
    ];
    let out = solve_of(
        &shapes,
        &[
            c(Rule::Fix { a: r(0, CENTER) }),
            c(Rule::Fix { a: r(0, START) }),
            c(Rule::Fix {
                a: r(1, Element::Point(PointAt::Node(0))),
            }),
            c(Rule::Coincident {
                a: r(0, END),
                b: r(1, Element::Point(PointAt::Node(0))),
            }),
        ],
    );
    assert!(out.status.holds(), "{out:?}");
    assert_eq!(out.dof, 1);
    let Shape::Ellipse { t0, t1, .. } = out.shapes[0] else {
        panic!()
    };
    close(at(&out.shapes[0], t0), Vec2::new(4., 0.));
    close(at(&out.shapes[0], t1), target);
}
#[test]
fn endpoint_duals_match_independent_ellipse_formula_for_every_unknown() {
    use super::dual::D;
    use super::model::{Built, Model, build};
    let shapes = [ellipse(Vec2::new(3., -2.), 5., 2., 0.37, 0.4, 2.1)];
    let model = Model::new(&shapes, &[true]);
    for (el, index) in [(START, 5), (END, 6)] {
        let vars = model.element_vars(&shapes, 0, &el).unwrap();
        let v: Vec<_> = vars
            .iter()
            .enumerate()
            .map(|(k, &i)| D::var(model.x[i], k))
            .collect();
        let Built::Point(p) = build(&shapes[0], &el, &v) else {
            panic!()
        };
        close(Vec2::new(p.x.v, p.y.v), at(&shapes[0], model.x[index]));
        for (k, &i) in vars.iter().enumerate() {
            let h = 1e-6;
            let eval = |delta: f64| {
                let mut x = model.x.clone();
                x[i] += delta;
                at(
                    &ellipse(Vec2::new(x[0], x[1]), x[2], x[3], x[4], x[5], x[6]),
                    x[index],
                )
            };
            let (a, b) = (eval(h), eval(-h));
            assert!(
                (p.x.d[k] - (a.x - b.x) / (2. * h)).abs() < 2e-8,
                "x derivative {k}"
            );
            assert!(
                (p.y.d[k] - (a.y - b.y) / (2. * h)).abs() < 2e-8,
                "y derivative {k}"
            );
        }
    }
}
#[test]
fn ellipse_branch_boundaries_are_not_accepted_as_valid_endpoints() {
    use super::dual::D;
    use super::model::{Built, Model, build};
    let shapes = [ellipse(Vec2::new(0., 0.), 4., 2., 0., 5., 0.5)];
    let model = Model::new(&shapes, &[true]);
    assert!(model.x[6] > model.x[5]);
    for (i, value) in [
        (2, 0.),
        (3, -1.),
        (3, 5.),
        (6, model.x[5]),
        (6, model.x[5] + TAU),
    ] {
        let mut x = model.x.clone();
        x[i] = value;
        let v: Vec<_> = x.into_iter().map(D::c).collect();
        let Built::Point(p) = build(&shapes[0], &END, &v) else {
            panic!()
        };
        assert!(!p.x.v.is_finite());
    }
    let full = ellipse(Vec2::new(0., 0.), 4., 2., 0., 0., 0.);
    assert!(!same_layout(&shapes[0], &full));
    assert!(
        !has_part(&full, &Element::Circle),
        "ellipse must not pretend to be circular"
    );
}
#[test]
fn symmetry_of_two_elliptical_arc_ends_fits_dual_capacity() {
    let shapes = [
        ellipse(Vec2::new(-5., 0.), 3., 1., 0., 0., PI / 2.),
        ellipse(Vec2::new(5., 0.), 3., 1., PI, 0., PI / 2.),
        Shape::Line {
            a: Vec2::new(0., -5.),
            b: Vec2::new(0., 5.),
        },
    ];
    let constraints = [c(Rule::Symmetric {
        a: r(0, END),
        b: r(1, END),
        axis: r(
            2,
            Element::Segment {
                part: 0,
                ring: 0,
                index: 0,
            },
        ),
    })];
    let out = solve_of(&shapes, &constraints);
    assert!(out.broken.is_empty());
    assert!(out.status.holds(), "{out:?}");
    let Shape::Ellipse { t1: a, .. } = out.shapes[0] else {
        panic!()
    };
    let Shape::Ellipse { t1: b, .. } = out.shapes[1] else {
        panic!()
    };
    let (p, q) = (at(&out.shapes[0], a), at(&out.shapes[1], b));
    let Shape::Line { a, b } = out.shapes[2] else {
        panic!()
    };
    let midpoint = Vec2::new((p.x + q.x) / 2., (p.y + q.y) / 2.);
    assert!(((b.x - a.x) * (midpoint.y - a.y) - (b.y - a.y) * (midpoint.x - a.x)).abs() < 1e-8);
    assert!(((b.x - a.x) * (q.x - p.x) + (b.y - a.y) * (q.y - p.y)).abs() < 1e-8);
}
