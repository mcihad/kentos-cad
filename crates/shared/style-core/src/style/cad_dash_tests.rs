//! ADR 1122: distances on the source curve, cuts at those distances, the seam,
//! dots, the fallback and the opt-in (`View::cad_phase`). Expected values are the
//! curves' own closed forms (r·θ, quarter symmetry), not this code's output.
use super::{distances, ink};
use super::super::cad_marks::Route;
use kentos_geometry_core::{Vec2, entity::Shape};
use std::f64::consts::{FRAC_1_SQRT_2, FRAC_PI_4, PI, TAU};

fn p(x: f64, y: f64) -> Vec2 {
    Vec2::new(x, y)
}
fn route(shape: &Shape) -> Route {
    Route::of(shape).unwrap().unwrap()
}
fn ring(c: Vec2, r: f64, n: usize) -> Vec<Vec2> {
    (0..n)
        .map(|k| {
            let a = TAU * k as f64 / n as f64;
            p(c.x + r * a.cos(), c.y + r * a.sin())
        })
        .collect()
}

#[test]
fn a_circle_vertex_is_at_r_theta_not_at_the_chords_sum() {
    let c = p(487_000., 4_420_000.);
    let shape = Shape::Circle { c, r: 2. };
    let path = ring(c, 2., 8);
    let d = distances(&route(&shape), &path).unwrap();
    for (k, d) in d.iter().enumerate() {
        assert!((d - 2. * FRAC_PI_4 * k as f64).abs() < 1e-9, "{k}: {d}");
    }
    // Chords would give 8·2·2·sin(π/8) = 12.25 for 4π = 12.57.
    assert!((2. * 2. * (PI / 8.).sin() * 8. - 4. * PI).abs() > 0.3);
}

#[test]
fn every_chord_carries_the_same_half_dash_and_the_seam_joins_ink() {
    let shape = Shape::Circle { c: p(0., 0.), r: 1. };
    let path = ring(p(0., 0.), 1., 8);
    let r = route(&shape);
    let d = distances(&r, &path).unwrap();
    // A dash of π/8 and a gap of π/8: from every vertex to the arc's own point halfway.
    let got = ink(&path, &d, true, r.total, &[PI / 8., PI / 8.], 0., &|t| r.point_at(t)).unwrap();
    assert_eq!(got.pieces.len(), 8);
    for (k, piece) in got.pieces.iter().enumerate() {
        let (a, b) = (path[k], path[(k + 1) % 8]);
        assert_eq!(piece[0], a, "{k}");
        let _ = b;
        let half = PI / 8. * (2 * k + 1) as f64;
        assert!((piece[1].x - half.cos()).abs() < 1e-12 && (piece[1].y - half.sin()).abs() < 1e-12);
    }
    // Phase π/16 puts ink across the seam: the last run joins the first.
    let got = ink(&path, &d, true, r.total, &[PI / 8., PI / 8.], PI / 16., &|t| r.point_at(t)).unwrap();
    assert_eq!(got.pieces.len(), 8, "eight runs, one of them through the seam");
    let seam = got.pieces.iter().find(|q| q.contains(&path[0])).unwrap();
    assert_eq!(seam.len(), 3, "half a chord before the seam, the seam, half after");
}

#[test]
fn a_bulge_path_keeps_its_phase_from_the_line_through_the_arc() {
    // 3 m east, a semicircle of radius 1 (bulge 1), 2 m west.
    let pts = vec![p(0., 0.), p(3., 0.), p(3., 2.), p(1., 2.)];
    let shape = Shape::Polyline {
        pts: pts.clone(),
        bulges: Some(vec![0., 1., 0., 0.]),
        closed: false,
        widths: None,
        holes: None,
        parts: None,
    };
    let r = route(&shape);
    assert!((r.total - (5. + PI)).abs() < 1e-12);
    let mut path = vec![p(0., 0.), p(3., 0.)];
    for k in 1..=6 {
        let a = -PI / 2. + PI * k as f64 / 6.;
        path.push(p(3. + a.cos(), 1. + a.sin()));
    }
    path.push(p(1., 2.));
    let d = distances(&r, &path).unwrap();
    for k in 1..=6 {
        assert!((d[1 + k] - (3. + PI * k as f64 / 6.)).abs() < 1e-12, "{k}");
    }
    assert!((d[8] - (5. + PI)).abs() < 1e-12);
    // A dash ending exactly where the arc ends: 3 + π long, then a gap of 1.
    let got = ink(&path, &d, false, r.total, &[3. + PI, 1.], 0., &|t| r.point_at(t)).unwrap();
    let near = |a: Vec2, b: Vec2| (a.x - b.x).abs() < 1e-12 && (a.y - b.y).abs() < 1e-12;
    assert_eq!(got.pieces.len(), 2);
    assert!(near(*got.pieces[0].last().unwrap(), p(3., 2.)), "{:?}", got.pieces[0]);
    assert!(near(got.pieces[1][0], p(2., 2.)), "{:?}", got.pieces[1]);
    assert!(near(*got.pieces[1].last().unwrap(), p(1., 2.)));
}

#[test]
fn an_ellipse_quarter_is_a_quarter_of_its_length() {
    let shape = Shape::Ellipse {
        c: p(500_000., 4_400_000.),
        major: p(6., 3.),
        ratio: 0.3,
        t0: 0.,
        t1: 0.,
    };
    let r = route(&shape);
    let e = kentos_geometry_core::geom::ellipse::EllipseGeom {
        c: p(500_000., 4_400_000.),
        major: p(6., 3.),
        ratio: 0.3,
        t0: 0.,
        t1: 0.,
    };
    let path: Vec<Vec2> = (0..64)
        .map(|k| kentos_geometry_core::geom::ellipse::ellipse_point(&e, TAU * k as f64 / 64.))
        .collect();
    let d = distances(&r, &path).unwrap();
    for (k, want) in [(16, 0.25), (32, 0.5), (48, 0.75)] {
        assert!((d[k] - want * r.total).abs() < 1e-9 * r.total, "{k}");
    }
}

#[test]
fn a_rational_nurbs_circle_is_measured_by_its_own_arc() {
    let shape = Shape::Nurbs {
        degree: 2,
        ctrl: vec![p(1., 0.), p(1., 1.), p(0., 1.)],
        knots: vec![0., 0., 0., 1., 1., 1.],
        weights: Some(vec![1., FRAC_1_SQRT_2, 1.]),
        closed: false,
        periodic: false,
        fit: None,
        fit_tolerance: None,
        tangents: None,
    };
    let r = route(&shape);
    assert!((r.total - PI / 2.).abs() < 1e-12);
    let path: Vec<Vec2> = [0., 0.3, 0.7, 1.2, PI / 2.].iter().map(|a| p(a.cos(), a.sin())).collect();
    let d = distances(&r, &path).unwrap();
    for (k, a) in [0., 0.3, 0.7, 1.2, PI / 2.].iter().enumerate() {
        assert!((d[k] - a).abs() < 1e-9, "{k}: {} {a}", d[k]);
    }
}

#[test]
fn dots_sit_at_their_distances_and_other_carriers_fall_back() {
    let shape = Shape::Circle { c: p(0., 0.), r: 1. };
    let path = ring(p(0., 0.), 1., 8);
    let r = route(&shape);
    let d = distances(&r, &path).unwrap();
    let got = ink(&path, &d, true, r.total, &[0., FRAC_PI_4], 0., &|t| r.point_at(t)).unwrap();
    assert!(got.pieces.is_empty());
    assert_eq!(got.dots.len(), 8, "one at each vertex, one at the seam");
    for (k, (at, _)) in got.dots.iter().enumerate() {
        assert!((at.x - path[k].x).abs() < 1e-12 && (at.y - path[k].y).abs() < 1e-12);
    }
    // An offset carrier is not on the route: the renderer's dash stays.
    let off = ring(p(0., 0.), 1.5, 8);
    assert!(distances(&r, &off).is_none());
    // Past the budget: no analytic output (the caller keeps the earlier stroke).
    assert!(ink(&path, &d, true, r.total, &[1e-6, 1e-6], 0., &|t| r.point_at(t)).is_none());
}

/// Only a build that asks for it (the drawing's CAD looks, `View::cad_phase`;
/// a preview's `cadPhase`) changes: others keep the renderer's dash (the frozen
/// style answers, the web's).
#[test]
fn only_a_cad_phase_build_is_cut_on_the_curve() {
    use kentos_geometry_core::api::json::Json;
    let compile = |cad: bool| {
        let layer = serde_json::json!({"id":"l","type":"simpleLine","color":"#000000","width":0.25,
            "unit":"mm","dash":[2.,1.],"cap":"butt","join":"miter"});
        let input = serde_json::json!({"symbol":{"type":"line","layers":[layer]},
            "entity":{"id":1,"layerId":"a","attrs":{},"kind":"arc","c":{"x":0.,"y":0.},"r":10.,"a0":0.,"a1":3.},
            "plotScale":1000.,"cadPhase":cad});
        let out = super::super::build::compile_one(&Json::parse(&input.to_string()).unwrap()).unwrap();
        serde_json::from_str::<serde_json::Value>(&out).unwrap()
    };
    let plain = compile(false);
    assert_eq!(plain["strokes"].as_array().unwrap().len(), 1);
    assert_eq!(plain["strokes"][0]["style"]["dash"].as_array().map(Vec::len), Some(2));
    let cad = compile(true);
    let strokes = cad["strokes"].as_array().unwrap();
    // 30 m of arc, 3 m period at 1:1000 (2 mm + 1 mm): ten dashes.
    assert_eq!(strokes.len(), 10);
    assert!(strokes.iter().all(|s| s["style"]["dash"].is_null()));
    let last = strokes.last().unwrap()["path"].as_array().unwrap();
    let end = last.last().unwrap();
    // The cut is the arc's own point at 29 m: 2.9 rad, on the 10 m circle.
    let (x, y) = (end["x"].as_f64().unwrap(), end["y"].as_f64().unwrap());
    assert!((y.atan2(x) - 2.9).abs() < 1e-12, "{}", y.atan2(x));
    assert!((x.hypot(y) - 10.).abs() < 1e-12);
}
