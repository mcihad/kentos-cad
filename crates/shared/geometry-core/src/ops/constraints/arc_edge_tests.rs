//! CAD-ADV-05: a path arc varies its own bulge while retaining vertices, sign and minor/major branch.
use super::tests::{c, dim, r, solve_of};
use super::*;
use crate::geom::bulge::bulge_arc;
use crate::vec2::Vec2;
const EDGE: Element = Element::ArcEdge {
    part: 0,
    ring: 0,
    index: 0,
};
fn vertex(index: u32) -> Element {
    Element::Point(PointAt::Vertex {
        part: 0,
        ring: 0,
        index,
    })
}
fn path(bulge: f64, origin: Vec2) -> Shape {
    Shape::Polyline {
        pts: vec![origin, Vec2::new(origin.x + 4., origin.y)],
        bulges: Some(vec![bulge, 0.]),
        holes: None,
        parts: None,
        closed: false,
        widths: Some(vec![[0.1, 0.2], [0.0, 0.0]]),
    }
}
fn fixed_radius(radius: f64) -> Vec<Constraint> {
    vec![
        c(Rule::Fix { a: r(0, vertex(0)) }),
        c(Rule::Fix { a: r(0, vertex(1)) }),
        dim(Rule::Radius { a: r(0, EDGE) }, radius),
    ]
}
fn arc(s: &Shape) -> crate::geom::bulge::BulgeArc {
    let Shape::Polyline { pts, bulges, .. } = s else {
        panic!("path")
    };
    bulge_arc(pts[0], pts[1], bulges.as_ref().unwrap()[0]).unwrap()
}
#[test]
fn radius_changes_the_bulge_without_moving_fixed_vertices_on_both_branches_and_signs() {
    for origin in [Vec2::new(0., 0.), Vec2::new(487000., 4420000.)] {
        for bulge in [0.41421356237309503, -0.41421356237309503, 2., -2., 1., -1.] {
            let before = path(bulge, origin);
            let out = solve_of(std::slice::from_ref(&before), &fixed_radius(3.));
            assert!(out.status.holds(), "{bulge} at {origin:?}: {out:?}");
            assert_eq!(out.dof, 0);
            let Shape::Polyline {
                pts,
                bulges,
                widths,
                ..
            } = &out.shapes[0]
            else {
                panic!("path")
            };
            assert_eq!(pts, &vec![origin, Vec2::new(origin.x + 4., origin.y)]);
            assert_eq!(widths, &Some(vec![[0.1, 0.2], [0.0, 0.0]]));
            let b = bulges.as_ref().unwrap()[0];
            assert!(b * bulge > 0.0);
            assert_eq!(b.abs() <= 1., bulge.abs() <= 1.);
            assert!((arc(&out.shapes[0]).r - 3.).abs() < 1e-8);
            let held = [Hold {
                object: 0,
                element: None,
            }];
            let verified = solve(
                &Problem {
                    shapes: &out.shapes,
                    constraints: &fixed_radius(3.),
                    held: &held,
                    fix_from: Some(&[before]),
                },
                &|| false,
            );
            assert!(verified.status.holds(), "stored shape: {verified:?}");
        }
    }
}
#[test]
fn a_radius_below_half_a_fixed_chord_is_refused_without_a_branch_jump_or_partial_shape() {
    for b in [0.5, -0.5, 1., -1., 2., -2.] {
        let shape = path(b, Vec2::new(0., 0.));
        let shapes = [shape];
        let out = solve_of(&shapes, &fixed_radius(1.5));
        assert!(!out.status.holds(), "{out:?}");
        assert_eq!(out.shapes, shapes);
        assert!(out.changed.is_empty());
    }
}
#[test]
fn old_vertex_constraints_keep_the_unmentioned_bulge_and_original_freedoms() {
    let shape = path(0.5, Vec2::new(0., 0.));
    let out = solve_of(
        std::slice::from_ref(&shape),
        &[c(Rule::Fix { a: r(0, vertex(0)) })],
    );
    assert_eq!(out.dof, 2);
    assert_eq!(out.shapes[0], shape);
    assert!(!has_part(
        &shape,
        &Element::Segment {
            part: 0,
            ring: 0,
            index: 0
        }
    ));
    assert!(has_part(&shape, &EDGE));
    assert!(!has_part(&path(0., Vec2::new(0., 0.)), &EDGE));
}
#[test]
fn a_tangent_at_a_shared_path_vertex_keeps_its_single_root_when_radius_changes() {
    let shapes = [
        path(0.41421356237309503, Vec2::new(0., 0.)),
        Shape::Line {
            a: Vec2::new(-2., 2.),
            b: Vec2::new(0., 0.),
        },
    ];
    let line = Element::Segment {
        part: 0,
        ring: 0,
        index: 0,
    };
    let mut rules = fixed_radius(3.);
    rules.extend([
        c(Rule::Coincident {
            a: r(0, vertex(0)),
            b: r(1, Element::Point(PointAt::End)),
        }),
        dim(
            Rule::Distance {
                a: r(1, Element::Point(PointAt::Start)),
                b: r(1, Element::Point(PointAt::End)),
                axis: Axis::Aligned,
                left: false,
            },
            8_f64.sqrt(),
        ),
        c(Rule::Tangent {
            a: r(1, line),
            b: r(0, EDGE),
            inside: false,
            left: true,
        }),
    ]);
    let out = solve_of(&shapes, &rules);
    assert!(out.status.holds(), "{out:?}");
    assert_eq!(out.dof, 0);
    let Shape::Line { a, b } = out.shapes[1] else {
        panic!("line")
    };
    let center = arc(&out.shapes[0]).c;
    let dot = (b.x - a.x) * (b.x - center.x) + (b.y - a.y) * (b.y - center.y);
    assert!(dot.abs() < 1e-8, "{dot}");
    assert!(a.x < 0. && a.y > 0.);
    assert_eq!(b, Vec2::new(0., 0.));
}
#[test]
fn moving_a_vertex_does_not_hold_the_curvature_but_an_explicit_bulge_edit_does() {
    let before = path(0.5, Vec2::new(0., 0.));
    let mut after = before.clone();
    if let Shape::Polyline { pts, .. } = &mut after {
        pts[1].x = 5.;
    }
    assert_eq!(moved_parts(&before, &after), Some(vec![vertex(1)]));
    if let Shape::Polyline { bulges, .. } = &mut after {
        bulges.as_mut().unwrap()[0] = 0.6;
    }
    assert_eq!(moved_parts(&before, &after), Some(vec![vertex(1), EDGE]));
}

#[test]
fn closing_arc_in_a_multipart_hole_preserves_every_other_vertex_and_bulge() {
    use crate::entity::Part;
    use crate::geom::arrangement::Ring;
    let shape = Shape::Polygon {
        pts: vec![Vec2::new(0., 0.), Vec2::new(10., 0.), Vec2::new(10., 10.)],
        bulges: None,
        holes: None,
        parts: Some(vec![Part {
            pts: vec![
                Vec2::new(20., 0.),
                Vec2::new(30., 0.),
                Vec2::new(30., 10.),
                Vec2::new(20., 10.),
            ],
            bulges: None,
            holes: Some(vec![Ring {
                pts: vec![Vec2::new(22., 2.), Vec2::new(26., 2.), Vec2::new(24., 5.)],
                bulges: Some(vec![0., 0., 0.5]),
            }]),
        }]),
    };
    let edge = Element::ArcEdge {
        part: 1,
        ring: 1,
        index: 2,
    };
    let v = |index| {
        Element::Point(PointAt::Vertex {
            part: 1,
            ring: 1,
            index,
        })
    };
    let out = solve_of(
        std::slice::from_ref(&shape),
        &[
            c(Rule::Fix { a: r(0, v(2)) }),
            c(Rule::Fix { a: r(0, v(0)) }),
            dim(Rule::Radius { a: r(0, edge) }, 5.),
        ],
    );
    assert!(out.status.holds(), "{out:?}");
    assert!((circle_of(&out.shapes[0], &edge).unwrap().1 - 5.).abs() < 1e-8);
    let mut expected = shape;
    let Shape::Polygon {
        parts: Some(parts), ..
    } = &mut expected
    else {
        panic!("parts")
    };
    let Shape::Polygon {
        parts: Some(actual),
        ..
    } = &out.shapes[0]
    else {
        panic!("parts")
    };
    parts[0].holes.as_mut().unwrap()[0].bulges.as_mut().unwrap()[2] =
        actual[0].holes.as_ref().unwrap()[0]
            .bulges
            .as_ref()
            .unwrap()[2];
    assert_eq!(out.shapes[0], expected);
}

#[test]
fn adjacent_straight_and_circular_edges_are_tangent_at_their_inherent_shared_vertex() {
    let shape = Shape::Polyline {
        pts: vec![Vec2::new(-3., 4.), Vec2::new(0., 0.), Vec2::new(4., 0.)],
        bulges: Some(vec![0., 0.35, 0.]),
        holes: None,
        parts: None,
        closed: false,
        widths: None,
    };
    let mut cs: Vec<_> = (0..3)
        .map(|i| c(Rule::Fix { a: r(0, vertex(i)) }))
        .collect();
    cs.push(c(Rule::Tangent {
        a: r(
            0,
            Element::Segment {
                part: 0,
                ring: 0,
                index: 0,
            },
        ),
        b: r(
            0,
            Element::ArcEdge {
                part: 0,
                ring: 0,
                index: 1,
            },
        ),
        inside: false,
        left: true,
    }));
    let out = solve_of(std::slice::from_ref(&shape), &cs);
    assert!(out.status.holds(), "{out:?}");
    assert_eq!(out.dof, 0);
    let Shape::Polyline { pts, bulges, .. } = &out.shapes[0] else {
        panic!("path")
    };
    let Shape::Polyline { pts: before, .. } = &shape else {
        panic!("path")
    };
    assert_eq!(pts, before);
    assert!((bulges.as_ref().unwrap()[1] - 0.5).abs() < 1e-8);
}

#[test]
fn a_large_coordinate_strict_solution_rounds_back_to_the_identical_stored_model() {
    let origin = Vec2::new(487000., 4420000.);
    let shapes = [
        path(0.5, origin),
        Shape::Line {
            a: Vec2::new(origin.x - 1.5, origin.y + 2.),
            b: origin,
        },
    ];
    let mut cs = fixed_radius(3.);
    cs.extend([
        c(Rule::Coincident {
            a: r(0, vertex(0)),
            b: r(1, Element::Point(PointAt::End)),
        }),
        dim(
            Rule::Distance {
                a: r(1, Element::Point(PointAt::Start)),
                b: r(1, Element::Point(PointAt::End)),
                axis: Axis::Aligned,
                left: false,
            },
            2.5,
        ),
        c(Rule::Tangent {
            a: r(
                1,
                Element::Segment {
                    part: 0,
                    ring: 0,
                    index: 0,
                },
            ),
            b: r(0, EDGE),
            inside: false,
            left: true,
        }),
    ]);
    let out = solve_of(&shapes, &cs);
    assert!(out.status.holds(), "{out:?}");
    let held = [
        Hold {
            object: 0,
            element: None,
        },
        Hold {
            object: 1,
            element: None,
        },
    ];
    let q = solve(
        &Problem {
            shapes: &out.shapes,
            constraints: &cs,
            held: &held,
            fix_from: Some(&shapes),
        },
        &|| false,
    );
    assert!(q.status.holds(), "stored solution: {q:?}");
    assert_eq!(q.shapes, out.shapes);
    assert!(q.changed.is_empty());
    let mut wrong = out.shapes.clone();
    if let Shape::Line { a, .. } = &mut wrong[1] {
        a.y += 1e-7;
    }
    let bad = solve(
        &Problem {
            shapes: &wrong,
            constraints: &cs,
            held: &held,
            fix_from: Some(&shapes),
        },
        &|| false,
    );
    assert!(
        !bad.status.holds(),
        "a real gap must not be certified: {bad:?}"
    );
    assert_eq!(bad.shapes, wrong);
}

#[test]
fn roundtrip_verification_cannot_hide_a_real_small_gap_or_a_coarse_coordinate_system() {
    let relation = dim(
        Rule::Distance {
            a: r(0, Element::Point(PointAt::Start)),
            b: r(0, Element::Point(PointAt::End)),
            axis: Axis::Aligned,
            left: false,
        },
        2.5,
    );
    let held = [Hold {
        object: 0,
        element: None,
    }];
    for shapes in [
        vec![Shape::Line {
            a: Vec2::new(0., 0.),
            b: Vec2::new(2.5 + 1e-9, 0.),
        }],
        vec![Shape::Line {
            a: Vec2::new(1e15, 1e15),
            b: Vec2::new(1e15 + 2.625, 1e15),
        }],
    ] {
        let out = solve(
            &Problem {
                shapes: &shapes,
                constraints: &[relation],
                held: &held,
                fix_from: None,
            },
            &|| false,
        );
        assert!(!out.status.holds(), "{out:?}");
        assert_eq!(out.shapes, shapes);
        assert!(out.changed.is_empty());
    }
    let a = Vec2::new(487000., 4420000.);
    let b = Vec2::new(a.x, f64::from_bits(a.y.to_bits() + 1));
    let shapes = [
        Shape::Point {
            p: a,
            z: None,
            parts: None,
        },
        Shape::Point {
            p: b,
            z: None,
            parts: None,
        },
    ];
    let refs = Element::Point(PointAt::Node(0));
    let cs = [c(Rule::Coincident {
        a: r(0, refs),
        b: r(1, refs),
    })];
    let held = [
        Hold {
            object: 0,
            element: None,
        },
        Hold {
            object: 1,
            element: None,
        },
    ];
    let out = solve(
        &Problem {
            shapes: &shapes,
            constraints: &cs,
            held: &held,
            fix_from: None,
        },
        &|| false,
    );
    assert!(
        !out.status.holds(),
        "a direct one-ULP point gap stays a gap: {out:?}"
    );
}
#[test]
fn cancellation_during_a_roundtrip_certificate_returns_no_shapes_to_write() {
    let origin = Vec2::new(487000., 4420000.);
    let shapes = [
        path(0.5, origin),
        Shape::Line {
            a: Vec2::new(origin.x - 1.5, origin.y + 2.),
            b: origin,
        },
    ];
    let mut cs = fixed_radius(3.);
    cs.extend([
        c(Rule::Coincident {
            a: r(0, vertex(0)),
            b: r(1, Element::Point(PointAt::End)),
        }),
        dim(
            Rule::Distance {
                a: r(1, Element::Point(PointAt::Start)),
                b: r(1, Element::Point(PointAt::End)),
                axis: Axis::Aligned,
                left: false,
            },
            2.5,
        ),
        c(Rule::Tangent {
            a: r(
                1,
                Element::Segment {
                    part: 0,
                    ring: 0,
                    index: 0,
                },
            ),
            b: r(0, EDGE),
            inside: false,
            left: true,
        }),
    ]);
    let stored = solve_of(&shapes, &cs).shapes;
    let held = [
        Hold {
            object: 0,
            element: None,
        },
        Hold {
            object: 1,
            element: None,
        },
    ];
    let calls = std::cell::Cell::new(0);
    let stop = || {
        calls.set(calls.get() + 1);
        true
    };
    let out = solve(
        &Problem {
            shapes: &stored,
            constraints: &cs,
            held: &held,
            fix_from: Some(&shapes),
        },
        &stop,
    );
    assert_eq!(out.status, Status::Cancelled);
    assert!(calls.get() > 0);
    assert!(out.changed.is_empty());
    assert_eq!(out.shapes, stored);
}
