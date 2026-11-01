//! Independent dense checks supplement, rather than define, interval bounds.
use super::*;
use crate::geom::intersect::{Edge, closest_on_edge};
fn v(x: f64, y: f64) -> Vec2 {
    Vec2::new(x, y)
}
fn points(s: &Shape) -> &[Vec2] {
    match s {
        Shape::Polyline { pts, .. } | Shape::Polygon { pts, .. } => pts,
        _ => panic!(),
    }
}
fn gap(s: &Shape, p: Vec2) -> f64 {
    let pts = points(s);
    let closed = crate::entity::is_closed_path(s);
    (0..pts.len() - usize::from(!closed))
        .map(|i| {
            closest_on_edge(
                &Edge::Seg {
                    a: pts[i],
                    b: pts[(i + 1) % pts.len()],
                },
                p,
            )
            .d
        })
        .fold(f64::INFINITY, f64::min)
}
fn rational_quarter(o: Vec2) -> Shape {
    Shape::Nurbs {
        degree: 2,
        ctrl: vec![v(o.x + 1., o.y), v(o.x + 1., o.y + 1.), v(o.x, o.y + 1.)],
        knots: vec![0., 0., 0., 1., 1., 1.],
        weights: Some(vec![1., core::f64::consts::FRAC_1_SQRT_2, 1.]),
        closed: false,
        periodic: false,
        fit: None,
        fit_tolerance: None,
        tangents: None,
    }
}
#[test]
fn a_weighted_quarter_circle_has_the_independent_parallel_radius() {
    for origin in [v(0., 0.), v(500_000., 4_500_000.)] {
        for (side, r) in [
            (v(origin.x + 2., origin.y + 2.), 1.3),
            (v(origin.x + 0.1, origin.y + 0.1), 0.7),
        ] {
            let copy = offset(&rational_quarter(origin), 0.3, side, 1e-4).unwrap();
            assert!(copy.error_bound <= 1e-4);
            for i in 0..=3000 {
                let t = core::f64::consts::FRAC_PI_2 * i as f64 / 3000.;
                let p = v(origin.x + r * cos(t), origin.y + r * sin(t));
                assert!(
                    gap(&copy.shape, p) <= copy.error_bound,
                    "{i}: {} > {}",
                    gap(&copy.shape, p),
                    copy.error_bound
                );
            }
        }
    }
}
#[test]
fn tighter_tolerance_refines_and_never_changes_the_source() {
    let s = rational_quarter(v(0., 0.));
    let before = s.clone();
    let a = offset(&s, 0.3, v(2., 2.), 1e-3).unwrap();
    let b = offset(&s, 0.3, v(2., 2.), 1e-5).unwrap();
    assert!(b.vertices > a.vertices);
    assert!(b.error_bound < a.error_bound);
    assert_eq!(s, before);
}
#[test]
fn cusps_collapses_and_impossible_precision_refuse_without_geometry() {
    let s = rational_quarter(v(0., 0.));
    assert!(offset(&s, 1., v(0., 0.), 1e-4).is_err());
    assert!(offset(&s, 1.1, v(0., 0.), 1e-4).is_err());
    assert!(offset(&rational_quarter(v(5e6, 5e6)), 0.3, v(6e6, 6e6), 1e-12).is_err());
    for tol in [0., -1., f64::NAN, f64::INFINITY] {
        assert!(offset(&s, 0.3, v(2., 2.), tol).is_err());
    }
}

#[test]
fn ellipse_parallel_checks_the_normal_not_an_inflated_ellipse() {
    let e = crate::geom::ellipse::EllipseGeom {
        c: v(500_000., 4_500_000.),
        major: v(3., 4.),
        ratio: 0.4,
        t0: 0.,
        t1: core::f64::consts::TAU,
    };
    let s = Shape::Ellipse {
        c: e.c,
        major: e.major,
        ratio: e.ratio,
        t0: e.t0,
        t1: e.t1,
    };
    for (at, direction) in [(v(e.c.x + 20., e.c.y + 20.), 1.), (e.c, -1.)] {
        let out = offset(&s, 0.2, at, 1e-4).unwrap();
        assert!(matches!(out.shape, Shape::Polygon { .. }));
        let mut max: f64 = 0.;
        for i in 0..3000 {
            let t = core::f64::consts::TAU * i as f64 / 3000.;
            let p = ellipse_point(&e, t);
            let t = ellipse_derivative(&e, t);
            let n = js_hypot(t.x, t.y);
            let q = v(
                p.x + direction * 0.2 * t.y / n,
                p.y - direction * 0.2 * t.x / n,
            );
            max = max.max(gap(&out.shape, q));
        }
        assert!(max <= out.error_bound, "{max} > {}", out.error_bound);
    }
}

#[test]
fn weighted_multispan_and_unclamped_knots_follow_the_same_analytic_curve() {
    for (degree, ctrl, knots, weights) in [
        (
            3,
            vec![v(0., 0.), v(1., 2.), v(3., 2.), v(4., 0.)],
            vec![0., 0., 0., 0., 1., 1., 1., 1.],
            vec![1., 0.5, 2., 1.],
        ),
        (
            2,
            vec![v(0., 0.), v(1., 2.), v(3., 2.), v(4., 0.)],
            vec![0., 1., 2., 3., 4., 5., 6.],
            vec![1., 1., 1., 1.],
        ),
        (
            2,
            vec![v(0., 0.), v(1., 2.), v(3., 2.), v(4., 0.)],
            vec![0., 0., 0., 0.5, 1., 1., 1.],
            vec![1., 0.7, 1.3, 1.],
        ),
    ] {
        let s = Shape::Nurbs {
            degree,
            ctrl,
            knots,
            weights: Some(weights),
            closed: false,
            periodic: false,
            fit: None,
            fit_tolerance: None,
            tangents: None,
        };
        let n = nurbs::of(&s).unwrap();
        let at = v(2., 5.);
        let signed = {
            let ds = n.derivs(n.closest(at), 1);
            side(ds[1], ds[0], at, 0.1)
        };
        let out = offset(&s, 0.1, at, 1e-4)
            .unwrap_or_else(|e| panic!("degree {degree}, knots {:?}: {e}", n.knots));
        let (a, b) = n.domain();
        for i in 0..=3000 {
            let ds = n.derivs(a + (b - a) * i as f64 / 3000., 1);
            let speed = js_hypot(ds[1].x, ds[1].y);
            let p = v(
                ds[0].x - signed * ds[1].y / speed,
                ds[0].y + signed * ds[1].x / speed,
            );
            assert!(
                gap(&out.shape, p) <= out.error_bound,
                "degree {degree}, step {i}: {} > {}",
                gap(&out.shape, p),
                out.error_bound
            );
        }
    }
}

#[test]
fn c0_joins_and_zero_velocity_do_not_get_an_invented_bridge() {
    let s = Shape::Nurbs {
        degree: 1,
        ctrl: vec![v(0., 0.), v(1., 0.), v(1., 1.)],
        knots: vec![0., 0., 0.5, 1., 1.],
        weights: None,
        closed: false,
        periodic: false,
        fit: None,
        fit_tolerance: None,
        tangents: None,
    };
    assert!(
        offset(&s, 0.1, v(2., 2.), 1e-4)
            .unwrap_err()
            .contains("birleşim")
    );
    let s = Shape::Spline {
        pts: vec![v(0., 0.), v(0., 0.), v(1., 1.)],
        closed: false,
    };
    assert!(offset(&s, 0.1, v(2., 2.), 1e-4).is_err());
}

#[test]
fn closed_fit_curve_keeps_its_seam_and_a_long_output_is_actually_checked() {
    let source = Shape::Spline {
        pts: vec![v(0., -2.), v(3., 0.), v(0., 2.), v(-3., 0.)],
        closed: true,
    };
    let out = offset(&source, 0.2, v(4., 0.), 1e-4).unwrap();
    assert_eq!(out.crossings, 0);
    assert!(crate::entity::is_closed_path(&out.shape));
    let n = 5000;
    let pts: Vec<_> = (0..n)
        .map(|i| {
            let t = core::f64::consts::TAU * i as f64 / n as f64;
            v(cos(t), sin(t))
        })
        .collect();
    let ring = Shape::Polyline {
        pts,
        closed: true,
        bulges: None,
        holes: None,
        parts: None,
        widths: None,
    };
    assert_eq!(crossings::count(&ring).unwrap(), 0);
    let crossing = Shape::Polyline {
        pts: vec![v(0., 0.), v(2., 2.), v(0., 2.), v(2., 0.)],
        closed: false,
        bulges: None,
        holes: None,
        parts: None,
        widths: None,
    };
    assert_eq!(crossings::count(&crossing).unwrap(), 1);
}
