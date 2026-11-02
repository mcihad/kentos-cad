//! Full-ellipse axis edits retain the intrinsic seam, including axis-order changes.
use kentos_geometry_core::Vec2;
use kentos_geometry_core::entity::{Entity, Shape, ellipse_geom};
use kentos_geometry_core::geom::ellipse::{ellipse_point, is_full_ellipse};
use kentos_geometry_core::jsmath::{PI, TAU, cos, sin, js_hypot};
use kentos_geometry_core::ops::grips::{entity_grips, move_grip};

fn close(a: Vec2, b: Vec2) {
    assert!(js_hypot(a.x - b.x, a.y - b.y) < 3e-9, "{a:?} != {b:?}");
}

#[test]
fn axis_grips_keep_seam_points_across_rotation_swaps_and_world_origins() {
    for c in [Vec2::new(0., 0.), Vec2::new(487000., 4420000.)] {
        for angle in [0., 0.4, -1.2] {
            let u = Vec2::new(cos(angle), sin(angle));
            let v = Vec2::new(-u.y, u.x);
            for span in [0., TAU] {
                let old = Entity::new(Shape::Ellipse {
                    c,
                    major: Vec2::new(4. * u.x, 4. * u.y),
                    ratio: 0.5,
                    t0: 0.6,
                    t1: 0.6 + span,
                });
                for index in 1..=4 {
                    // Below, at, and above the other radius; both signs of each axis.
                    for length in [1.5, 2., 6.] {
                        let major = index == 1 || index == 3;
                        let direction = if major { u } else { v };
                        let sign = if index <= 2 { 1. } else { -1. };
                        let target = Vec2::new(
                            c.x + sign * length * direction.x,
                            c.y + sign * length * direction.y,
                        );
                        let moved = move_grip(&old, index, target).expect("valid axis");
                        let Shape::Ellipse {
                            c,
                            major: m,
                            ratio,
                            t0,
                            t1,
                        } = moved.shape
                        else {
                            panic!()
                        };
                        let g = ellipse_geom(c, m, ratio, t0, t1);
                        assert!(is_full_ellipse(&g));
                        assert!((t1 - t0 - span).abs() < 1e-14);
                        let (a, b) = if major { (length, 2.) } else { (4., length) };
                        for offset in [0., 0.8, PI] {
                            let t = 0.6 + offset;
                            let expected = Vec2::new(
                                c.x + a * u.x * cos(t) + b * v.x * sin(t),
                                c.y + a * u.y * cos(t) + b * v.y * sin(t),
                            );
                            close(ellipse_point(&g, t0 + offset), expected);
                        }
                    }
                    let grip = entity_grips(&old.shape)[index];
                    let noop = move_grip(&old, index, grip).unwrap();
                    let Shape::Ellipse { t0, t1, .. } = noop.shape else {
                        panic!()
                    };
                    assert_eq!(t0, 0.6);
                    assert_eq!(t1, 0.6 + span);
                }
            }
        }
    }
}

#[test]
fn reaiming_major_axis_keeps_its_seam_and_zero_axis_is_refused() {
    let c = Vec2::new(5., -2.);
    let old = Entity::new(Shape::Ellipse {
        c,
        major: Vec2::new(4., 0.),
        ratio: 0.5,
        t0: 0.6,
        t1: 0.6,
    });
    for (index, target) in [(1, Vec2::new(5., 4.)), (3, Vec2::new(5., -8.))] {
        let result = move_grip(&old, index, target).unwrap();
        let Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } = result.shape
        else {
            panic!()
        };
        close(
            ellipse_point(&ellipse_geom(c, major, ratio, t0, t1), t0),
            Vec2::new(5. - 2. * sin(0.6), -2. + 6. * cos(0.6)),
        );
    }
    for index in 1..=4 {
        assert!(move_grip(&old, index, c).is_none());
    }
}
