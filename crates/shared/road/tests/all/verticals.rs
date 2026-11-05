//! The vertical alignment against the shared cases (fixtures/road/v1/verticals.json, docs/adr/0902 §2–§4): written by
//! the textbook forms in exact fractions and a circle in mpmath (scripts/fixtures/road_vertical_cases.py), no KentOS
//! code.

use kentos_road::profile::vertical::{Pvi, VCurve, VerticalError, build};
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/road/v1/verticals.json");

fn n(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

fn near(a: f64, b: f64, tol: f64, what: &str) {
    assert!((a - b).abs() <= tol, "{what}: {a} vs {b}");
}

pub fn pvis(v: &Value) -> Vec<Pvi> {
    v.as_array()
        .expect("pvis")
        .iter()
        .map(|p| Pvi {
            id: p["id"].as_str().unwrap().to_owned(),
            station: n(&p["station"]),
            elevation: n(&p["elevation"]),
            curve: match p.get("curve") {
                None => VCurve::None,
                Some(c) => match c["kind"].as_str().unwrap() {
                    "parabola" => VCurve::Parabola { length: n(&c["length"]) },
                    "asymmetric" => VCurve::Asymmetric { length_in: n(&c["lengthIn"]), length_out: n(&c["lengthOut"]) },
                    _ => VCurve::Circle { radius: n(&c["radius"]) },
                },
            },
        })
        .collect()
}

#[test]
fn every_vertical_is_what_the_reference_says() {
    let all: Value = serde_json::from_str(CASES).expect("json");
    let cases = all["cases"].as_array().expect("cases");
    assert!(cases.len() >= 20);
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let built = build(&pvis(&c["pvis"]));
        if let Some(e) = c.get("error") {
            let got = built.expect_err(name);
            let kind = match &got {
                VerticalError::TooFewPoints => "TooFewPoints",
                VerticalError::DuplicateId { .. } => "DuplicateId",
                VerticalError::NotFinite { .. } => "NotFinite",
                VerticalError::NotIncreasing { .. } => "NotIncreasing",
                VerticalError::BadCurve { .. } => "BadCurve",
                VerticalError::Overlap { .. } => "Overlap",
            };
            assert_eq!(kind, e["kind"].as_str().unwrap(), "{name}");
            if let VerticalError::Overlap { a, b, excess } = &got {
                assert_eq!((a.as_str(), b.as_str()), (e["a"].as_str().unwrap(), e["b"].as_str().unwrap()), "{name}");
                near(*excess, n(&e["excess"]), 1e-9, name);
            }
            continue;
        }
        let v = built.unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let curves = c["curves"].as_array().unwrap();
        assert_eq!(v.curves.len(), curves.len(), "{name}");
        for (r, w) in v.curves.iter().zip(curves) {
            let what = format!("{name} {}", r.pvi);
            near(r.g_in, n(&w["gIn"]), 1e-12, &what);
            near(r.g_out, n(&w["gOut"]), 1e-12, &what);
            for k in 0..2 {
                near(r.start[k], n(&w["start"][k]), 1e-8, &what);
                near(r.end[k], n(&w["end"][k]), 1e-8, &what);
            }
            near(r.a, n(&w["a"]), 1e-9, &what);
            near(r.k.expect("k"), n(&w["k"]), 1e-8, &what);
            assert_eq!(r.crest, w["crest"].as_bool().unwrap(), "{what}");
            match (r.extreme, w["extreme"].as_array()) {
                (None, None) => {}
                (Some(x), Some(y)) => {
                    near(x[0], n(&y[0]), 1e-7, &what);
                    near(x[1], n(&y[1]), 1e-9, &what);
                }
                other => panic!("{what}: extreme {other:?}"),
            }
        }
        for w in c["at"].as_array().unwrap() {
            let s = n(&w["station"]);
            let (z, g, k) = v.at(s).unwrap_or_else(|| panic!("{name}: {s}"));
            let want = w["value"].as_array().unwrap();
            let what = format!("{name} at {s}");
            near(z, n(&want[0]), 1e-9, &what);
            near(g, n(&want[1]), 1e-11, &what);
            near(k, n(&want[2]), 1e-12, &what);
        }
    }
}

/// The red line's length in space (docs/adr/0968): a grade's Δs·√(1 + g²), a circle's R·Δθ, a parabola's closed
/// form and its Gauss–Legendre branch (where the grade hardly changes) against composite Simpson on √(1 + z′²).
#[test]
fn the_length_in_space_is_the_arc_length() {
    let p = |id: &str, s: f64, z: f64, curve: VCurve| Pvi { id: id.into(), station: s, elevation: z, curve };
    let v = build(&[
        p("a", 0.0, 100.0, VCurve::None),
        p("v1", 200.0, 130.0, VCurve::Parabola { length: 100.0 }),
        p("v2", 400.0, 110.0, VCurve::Circle { radius: 500.0 }),
        p("b", 600.0, 140.0, VCurve::None),
    ])
    .expect("builds");
    near(v.length_to(150.0).unwrap(), 150.0 * 1.0225f64.sqrt(), 1e-12, "15 % grade");
    let c = &v.curves[1];
    let arc = 500.0 * (c.g_out.atan() - c.g_in.atan()).abs();
    near(v.length_to(c.end[0]).unwrap() - v.length_to(c.start[0]).unwrap(), arc, 1e-9, "circle R·Δθ");
    near(v.length(), v.length_to(600.0).unwrap(), 0.0, "whole");
    assert!(v.length_to(-1.0).is_none() && v.length_to(600.1).is_none());
    let simpson = |a: f64, b: f64| {
        let m = 20_000;
        let h = (b - a) / m as f64;
        let f = |s: f64| v.at(s).unwrap().1.hypot(1.0);
        (0..=m).map(|i| f(a + i as f64 * h) * if i == 0 || i == m { 1.0 } else if i % 2 == 1 { 4.0 } else { 2.0 }).sum::<f64>() * h / 3.0
    };
    // Piece by piece, so Simpson never straddles a curve's end.
    let mut cuts = vec![0.0, 200.0, 400.0, 600.0];
    cuts.extend(v.curves.iter().flat_map(|c| [c.start[0], c.end[0]]));
    cuts.sort_by(f64::total_cmp);
    let mut sum = 0.0;
    for w in cuts.windows(2) {
        sum += simpson(w[0], w[1]);
        near(v.length_to(w[1]).unwrap(), sum, 1e-9, "Simpson");
    }
    // A grade change of 0.0004 over the curve: the Gauss–Legendre branch; 0.002: the closed form. Both against Simpson.
    for dz in [0.04, 0.2] {
        let flat = build(&[
            p("a", 0.0, 100.0, VCurve::None),
            p("v", 100.0, 101.0, VCurve::Parabola { length: 100.0 }),
            p("b", 200.0, 102.0 + dz, VCurve::None),
        ])
        .expect("builds");
        let c = &flat.curves[0];
        let m = 20_000;
        let h = (c.end[0] - c.start[0]) / m as f64;
        let f = |s: f64| flat.at(s).unwrap().1.hypot(1.0);
        let simpson = (0..=m)
            .map(|i| f(c.start[0] + i as f64 * h) * if i == 0 || i == m { 1.0 } else if i % 2 == 1 { 4.0 } else { 2.0 })
            .sum::<f64>()
            * h
            / 3.0;
        near(flat.length_to(c.end[0]).unwrap() - flat.length_to(c.start[0]).unwrap(), simpson, 1e-10, "small Δg");
    }
}
