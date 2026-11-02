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

#[test]
fn ellipse_center_ray_turns_a_quarter_without_collapsing_axes() {
    for (angle, vertical) in [(0., true), (PI / 2., false), (PI, true), (-PI / 2., false)] {
        for reverse in [false, true] {
            let center = Vec2::new(3e6, 1e7);
            let shapes = [ellipse(center, 4., 2., angle, 0., 0.)];
            let (a, b) = if reverse {
                (r(0, START), r(0, CENTER))
            } else {
                (r(0, CENTER), r(0, START))
            };
            let direction = if vertical {
                Rule::Vertical { a, b: Some(b) }
            } else {
                Rule::Horizontal { a, b: Some(b) }
            };
            let out = solve_of(
                &shapes,
                &[
                    c(Rule::Fix { a: r(0, CENTER) }),
                    dim(
                        Rule::Distance {
                            a: r(0, CENTER),
                            b: r(0, START),
                            axis: Axis::Aligned,
                            left: false,
                        },
                        4.,
                    ),
                    c(direction),
                ],
            );
            assert!(
                out.status.holds(),
                "angle={angle}, vertical={vertical}, reverse={reverse}: {out:?}"
            );
            assert_eq!(out.dof, 1);
            let Shape::Ellipse {
                c: actual,
                major,
                ratio,
                t0,
                t1,
            } = out.shapes[0]
            else {
                panic!()
            };
            close(actual, center);
            assert!((js_hypot(major.x, major.y) - 4.).abs() < 1e-8);
            assert!((ratio - 0.5).abs() < 1e-8);
            assert_eq!((t0, t1), (0., 0.));
            assert!((if vertical { major.x } else { major.y }).abs() < 1e-8);
        }
    }
}

#[test]
fn ellipse_direction_conflict_and_held_orientation_leave_shapes_unchanged() {
    let shapes = [ellipse(Vec2::new(0., 0.), 4., 2., 0., 0., 0.)];
    let v = c(Rule::Vertical {
        a: r(0, CENTER),
        b: Some(r(0, START)),
    });
    let h = c(Rule::Horizontal {
        a: r(0, CENTER),
        b: Some(r(0, START)),
    });
    let constraints = [c(Rule::Fix { a: r(0, CENTER) }), h, v];
    let out = solve_of(&shapes, &constraints);
    assert!(!out.status.holds());
    assert!(!out.conflicting.is_empty());
    assert_eq!(out.shapes, shapes);
    let out = solve(
        &Problem {
            shapes: &shapes,
            constraints: &[v],
            held: &[Hold {
                object: 0,
                element: None,
            }],
            fix_from: None,
        },
        &|| false,
    );
    assert!(!out.status.holds());
    assert_eq!(out.shapes, shapes);
}

#[test]
fn ellipse_ray_alignment_uses_the_true_partial_end_and_nearest_direction() {
    for angle in [-2.7, -1.1, -0.1, 0.1, 1.1, 2.7] {
        let shapes = [ellipse(Vec2::new(0., 0.), 4., 2., angle, 0., PI / 2.)];
        let initial = at(&shapes[0], PI / 2.);
        let out = solve_of(
            &shapes,
            &[
                c(Rule::Fix { a: r(0, CENTER) }),
                c(Rule::Horizontal {
                    a: r(0, CENTER),
                    b: Some(r(0, END)),
                }),
            ],
        );
        assert!(out.status.holds(), "angle={angle}: {out:?}");
        let Shape::Ellipse {
            t0,
            t1,
            major,
            ratio,
            ..
        } = out.shapes[0]
        else {
            panic!()
        };
        let end = at(&out.shapes[0], t1);
        assert!(end.y.abs() < 1e-8);
        assert!(
            end.x * initial.x > 0.,
            "must not flip the selected ray's half plane"
        );
        assert!(js_hypot(major.x, major.y) > 0. && ratio > 0. && ratio <= 1.);
        assert!(t1 > t0 && t1 - t0 < TAU);
    }
}

#[test]
fn changing_only_an_elliptical_arc_start_does_not_hold_its_unchanged_end() {
    let before = ellipse(Vec2::new(0., 0.), 4., 2., 0., 0., PI / 2.);
    let after = ellipse(Vec2::new(0., 0.), 4., 2., 0., 0.3, PI / 2.);
    assert_eq!(moved_parts(&before, &after), Some(vec![START]));
}

#[test]
fn a_dragged_ellipse_start_is_a_point_target_not_a_lock_on_all_seven_parameters() {
    let before = [ellipse(Vec2::new(0., 0.), 4., 2., 0., 0., PI / 2.)];
    let after = [ellipse(Vec2::new(0., 0.), 4., 2., 0., 0.3, PI / 2.)];
    let target = at(&after[0], 0.3);
    let constraints = [
        c(Rule::Fix { a: r(0, CENTER) }),
        dim(
            Rule::Distance {
                a: r(0, START),
                b: r(0, END),
                axis: Axis::Aligned,
                left: false,
            },
            20f64.sqrt(),
        ),
    ];
    let out = solve(
        &Problem {
            shapes: &after,
            constraints: &constraints,
            held: &[Hold {
                object: 0,
                element: Some(START),
            }],
            fix_from: Some(&before),
        },
        &|| false,
    );
    assert!(out.status.holds(), "{out:?}");
    let Shape::Ellipse { c, t0, t1, .. } = out.shapes[0] else {
        panic!()
    };
    close(c, Vec2::new(0., 0.));
    close(at(&out.shapes[0], t0), target);
    let end = at(&out.shapes[0], t1);
    assert!((js_hypot(target.x - end.x, target.y - end.y) - 20f64.sqrt()).abs() < 1e-8);
    assert!(
        js_hypot(end.x, end.y - 2.) > 1e-3,
        "the unchanged endpoint must be free to satisfy the dimension"
    );
    assert_eq!(
        out.dof, 2,
        "only the dragged point's two coordinates are held"
    );
}

#[test]
fn an_ellipse_drag_target_that_conflicts_with_a_permanent_fix_is_not_silently_released() {
    let before = [ellipse(Vec2::new(0., 0.), 4., 2., 0., 0., PI / 2.)];
    let after = [ellipse(Vec2::new(0., 0.), 4., 2., 0., 0.3, PI / 2.)];
    let constraints = [
        c(Rule::Fix { a: r(0, CENTER) }),
        c(Rule::Fix { a: r(0, START) }),
    ];
    let out = solve(
        &Problem {
            shapes: &after,
            constraints: &constraints,
            held: &[Hold {
                object: 0,
                element: Some(START),
            }],
            fix_from: Some(&before),
        },
        &|| false,
    );
    assert!(!out.status.holds(), "{out:?}");
    assert_eq!(out.shapes, after);
    assert!(
        out.conflicting.contains(&1),
        "the permanent Fix preventing this drag must be named: {out:?}"
    );
}

#[test]
fn holding_the_full_ellipse_seam_leaves_the_minor_axis_free() {
    let shapes = [ellipse(Vec2::new(0., 0.), 5., 2., 0., 0., 0.)];
    let constraints = [c(Rule::Fix { a: r(0, CENTER) })];
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
    assert_eq!(out.status, Status::Under, "{out:?}");
    assert_eq!(out.dof, 1);
    assert_eq!(out.shapes, shapes);
}

#[test]
fn stored_world_endpoint_coincidence_survives_local_origin_rounding_but_a_real_gap_is_not_relaxed()
{
    // Saved result of the actual grip test at map coordinates; the canonical endpoint and follower match bits.
    let e = Shape::Ellipse {
        c: Vec2::new(487000., 4420000.),
        major: Vec2::new(3.975923693455719, 0.008856974324449825),
        ratio: 0.5328000117676391,
        t0: 0.2785762202467755,
        t1: 1.6667796983056156,
    };
    let p = at(&e, 1.6667796983056156);
    let constraints = [c(Rule::Coincident {
        a: r(0, END),
        b: r(1, Element::Point(PointAt::Node(0))),
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
    for (gap, holds) in [(0., true), (p.y.next_up() - p.y, false), (1e-6, false)] {
        let shapes = [e.clone(), point(Vec2::new(p.x, p.y + gap))];
        let out = solve(
            &Problem {
                shapes: &shapes,
                constraints: &constraints,
                held: &held,
                fix_from: None,
            },
            &|| false,
        );
        assert_eq!(out.status.holds(), holds, "gap {gap}: {out:?}");
        assert_eq!(out.shapes, shapes);
    }
    let e = ellipse(Vec2::new(0., 0.), 4., 2., 0., 0., PI / 2.);
    let p = at(&e, PI / 2.);
    let shapes = [e, point(Vec2::new(p.x, p.y + 5e-10))];
    let out = solve(
        &Problem {
            shapes: &shapes,
            constraints: &constraints,
            held: &held,
            fix_from: None,
        },
        &|| false,
    );
    assert!(
        !out.status.holds(),
        "a near-origin physical gap keeps the original tolerance: {out:?}"
    );
}

#[test]
fn a_real_mouse_drag_endpoint_uses_the_independently_rounded_map_coordinate() {
    let e = ellipse(Vec2::new(487000., 4420000.), 4., 2., 0., 0., PI / 2.);
    assert_eq!(at(&e, 0.), Vec2::new(487004., 4420000.));
    // Parameters saved by the real GUI drag. mpmath at 100 digits, from the exact f64 inputs,
    // independently rounds this analytic point to the same two numbers as the stored follower.
    let e = Shape::Ellipse {
        c: Vec2::new(487000., 4420000.),
        major: Vec2::new(3.9758045793195307, 0.008850533012778335),
        ratio: 0.5328630330147123,
        t0: 0.27910843737677354,
        t1: 1.6670507904458485,
    };
    let expected = Vec2::new(486999.61320742796, 4420002.10790213);
    assert_eq!(at(&e, 1.6670507904458485), expected);
    let shapes = [e, point(expected)];
    let constraints = [c(Rule::Coincident {
        a: r(0, END),
        b: r(1, Element::Point(PointAt::Node(0))),
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
            constraints: &constraints,
            held: &held,
            fix_from: None,
        },
        &|| false,
    );
    assert!(out.status.holds(), "{out:?}");
    assert_eq!(out.shapes, shapes);
}

#[test]
fn cancellation_of_a_held_endpoint_verification_returns_no_changes() {
    let e = ellipse(Vec2::new(487000., 4420000.), 4., 2., 0., 0., PI / 2.);
    let end = at(&e, PI / 2.);
    let shapes = [e, point(end)];
    let constraints = [c(Rule::Coincident {
        a: r(0, END),
        b: r(1, Element::Point(PointAt::Node(0))),
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
            constraints: &constraints,
            held: &held,
            fix_from: None,
        },
        &|| true,
    );
    assert_eq!(out.status, Status::Cancelled);
    assert!(out.changed.is_empty());
    assert_eq!(out.shapes, shapes);
}

#[test]
fn explicit_axis_targets_leave_other_radius_free_instead_of_holding_the_seam() {
    use crate::entity::Entity;
    use crate::ops::grips::{ellipse_axis_after_grip, ellipse_axis_point, move_grip};
    let before = ellipse(Vec2::new(0., 0.), 4., 2., 0., 0.6, 0.6);
    let original = at(&before, 0.6);
    let distance = js_hypot(original.x, original.y);
    let cs = [
        c(Rule::Fix { a: r(0, CENTER) }),
        dim(
            Rule::Distance {
                a: r(0, CENTER),
                b: r(0, START),
                axis: Axis::Aligned,
                left: false,
            },
            distance,
        ),
    ];
    for (index, target) in [(1, Vec2::new(0., 3.)), (0, Vec2::new(1.5, 0.))] {
        let shape = move_grip(&Entity::new(before.clone()), index as usize + 1, target)
            .unwrap()
            .shape;
        let axis = ellipse_axis_after_grip(&before, index, target).unwrap();
        let requested = ellipse_axis_point(&shape, axis).unwrap();
        let out = solve_with_ellipse_axes(
            &Problem {
                shapes: &[shape],
                constraints: &cs,
                held: &[],
                fix_from: Some(std::slice::from_ref(&before)),
            },
            &[EllipseAxisTarget {
                object: 0,
                index: axis,
                point: requested,
            }],
            &|| false,
        );
        assert!(out.status.holds(), "{out:?}");
        close(ellipse_axis_point(&out.shapes[0], axis).unwrap(), requested);
        let Shape::Ellipse { t0, .. } = out.shapes[0] else {
            panic!()
        };
        let p = at(&out.shapes[0], t0);
        assert!((js_hypot(p.x, p.y) - distance).abs() < 1e-9);
        assert!(
            (p.x - original.x).abs() > 0.1,
            "the undragged seam must be free to follow its own bond"
        );
    }
}

#[test]
fn invalid_transient_axis_targets_return_original_shapes_without_panicking() {
    let shapes = [ellipse(Vec2::new(0., 0.), 4., 2., 0., 0.6, 0.6)];
    for target in [
        EllipseAxisTarget {
            object: 0,
            index: 4,
            point: Vec2::new(1., 0.),
        },
        EllipseAxisTarget {
            object: 1,
            index: 0,
            point: Vec2::new(1., 0.),
        },
        EllipseAxisTarget {
            object: 0,
            index: 0,
            point: Vec2::new(f64::NAN, 0.),
        },
    ] {
        let result = solve_with_ellipse_axes(
            &Problem {
                shapes: &shapes,
                constraints: &[],
                held: &[],
                fix_from: None,
            },
            &[target],
            &|| false,
        );
        assert_eq!(result.status, Status::Singular);
        assert_eq!(result.shapes, shapes);
        assert!(result.changed.is_empty());
    }
}
