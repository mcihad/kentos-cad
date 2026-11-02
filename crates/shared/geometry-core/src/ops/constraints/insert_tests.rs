//! Insertion-anchor constraints translate an existing placement without changing its definition or affine.
use super::tests::{c, dim, r, solve_of};
use super::*;
use crate::vec2::Vec2;

const ANCHOR: Element = Element::Point(PointAt::Node(0));
fn insert(p: Vec2, mirror: bool, rotation: f64) -> Shape {
    Shape::Insert {
        block: "component".into(),
        p,
        scale: 2.,
        y_scale: Some(0.7),
        rotation,
        mirror: mirror.then_some(true),
        attrs: None,
        places: None,
        params: None,
        clip: None,
    }
}
fn point(p: Vec2) -> Shape {
    Shape::Point {
        p,
        z: None,
        parts: None,
    }
}
#[test]
fn rotated_mirrored_nonuniform_inserts_translate_only_their_anchor_at_large_coordinates() {
    for mirror in [false, true] {
        for angle in [0., 0.37, crate::jsmath::PI / 2., -2.3] {
            let origin = Vec2::new(3e6, 1e7);
            let target = Vec2::new(origin.x + 8., origin.y - 3.);
            let old = insert(origin, mirror, angle);
            let out = solve_of(
                &[old.clone(), point(target)],
                &[
                    c(Rule::Fix { a: r(1, ANCHOR) }),
                    c(Rule::Coincident {
                        a: r(0, ANCHOR),
                        b: r(1, ANCHOR),
                    }),
                ],
            );
            assert!(out.status.holds(), "{out:?}");
            assert_eq!(out.dof, 0, "only placement translation is a solver unknown");
            let mut expected = old;
            if let Shape::Insert { p, .. } = &mut expected {
                *p = target;
            }
            assert_eq!(out.shapes[0], expected);
            assert_eq!(
                moved_parts(&insert(origin, mirror, angle), &expected),
                Some(vec![ANCHOR])
            );
            for unsupported in [
                Element::Point(PointAt::Node(1)),
                Element::Point(PointAt::Start),
                Element::Circle,
                Element::Segment {
                    part: 0,
                    ring: 0,
                    index: 0,
                },
            ] {
                assert!(!has_part(&expected, &unsupported));
            }
        }
    }
}
#[test]
fn anchor_distance_retains_the_selected_solution_side_and_all_placement_constants() {
    for sign in [-1., 1.] {
        let shapes = [
            point(Vec2::new(0., 0.)),
            insert(Vec2::new(sign * 6., 0.), true, 0.4),
        ];
        let out = solve_of(
            &shapes,
            &[
                c(Rule::Fix { a: r(0, ANCHOR) }),
                c(Rule::Horizontal {
                    a: r(0, ANCHOR),
                    b: Some(r(1, ANCHOR)),
                }),
                dim(
                    Rule::Distance {
                        a: r(0, ANCHOR),
                        b: r(1, ANCHOR),
                        axis: Axis::Aligned,
                        left: false,
                    },
                    8.,
                ),
            ],
        );
        assert!(out.status.holds(), "{out:?}");
        let mut expected = shapes[1].clone();
        if let Shape::Insert { p, .. } = &mut expected {
            *p = Vec2::new(sign * 8., 0.);
        }
        assert_eq!(out.shapes[1], expected);
    }
}
#[test]
fn a_dragged_anchor_holds_only_translation_and_a_locked_anchor_conflict_is_atomic() {
    let before = [
        insert(Vec2::new(6., 0.), true, 0.3),
        point(Vec2::new(6., 0.)),
    ];
    let rules = [c(Rule::Coincident {
        a: r(0, ANCHOR),
        b: r(1, ANCHOR),
    })];
    let mut edited = before.clone();
    if let Shape::Insert { p, .. } = &mut edited[0] {
        *p = Vec2::new(8., 2.);
    }
    let out = solve(
        &Problem {
            shapes: &edited,
            constraints: &rules,
            held: &[Hold {
                object: 0,
                element: Some(ANCHOR),
            }],
            fix_from: Some(&before),
        },
        &|| false,
    );
    assert!(out.status.holds(), "{out:?}");
    assert_eq!(out.shapes[0], edited[0]);
    assert_eq!(out.shapes[1], point(Vec2::new(8., 2.)));
    let mut locked_rules = rules.to_vec();
    locked_rules.push(c(Rule::Fix { a: r(1, ANCHOR) }));
    let out = solve(
        &Problem {
            shapes: &edited,
            constraints: &locked_rules,
            held: &[Hold {
                object: 0,
                element: None,
            }],
            fix_from: Some(&before),
        },
        &|| false,
    );
    assert!(!out.status.holds(), "{out:?}");
    assert_eq!(out.shapes, edited);
}
