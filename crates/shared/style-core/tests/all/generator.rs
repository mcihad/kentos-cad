//! Geometri üretici against its independent reference (docs/adr/0226 §1):
//! `scripts/fixtures/geometry_generator_cases.py` computes each case from the
//! ADR's definitions (`fixtures/style/v1/generator.json`); the core runs the
//! steps on the same geometry. Exact results within the fixture's tolerance;
//! buffers by what they must satisfy: their vertices at the distance from the
//! input (or on it, a one-sided buffer's flat side) and their area within its
//! chords' loss of the exact one.

use std::collections::HashMap;

use kentos_geometry_core::Vec2;
use kentos_geometry_core::api::json::Json as CoreJson;
use kentos_geometry_core::jsmath::{js_hypot, js_max, js_min};
use kentos_style_core::style::compile::{Env, Values};
use kentos_style_core::style::generate::{Gen, run};
use kentos_style_core::style::model::{Exprs, Unit};
use serde_json::Value as Json;

const CASES: &str = include_str!("../../../../../fixtures/style/v1/generator.json");

/// No object's values: an expression gives nothing, so a step takes its fallback.
struct NoValues;

impl Values for NoValues {
    fn number(&self, _: usize) -> Option<f64> {
        None
    }
    fn text(&self, _: usize) -> Option<String> {
        None
    }
    fn truth(&self, _: usize) -> Option<bool> {
        None
    }
}

fn point(v: &Json) -> Vec2 {
    Vec2::new(v[0].as_f64().expect("x"), v[1].as_f64().expect("y"))
}

fn ring(v: &Json) -> Vec<Vec2> {
    v.as_array().expect("a ring").iter().map(point).collect()
}

/// A case's geometry as the generator takes it.
fn gen_of(v: &Json) -> Gen {
    if let Some(p) = v.get("points") {
        return Gen::Points(ring(p));
    }
    if let Some(lines) = v.get("lines") {
        return Gen::Lines(
            lines
                .as_array()
                .expect("lines")
                .iter()
                .map(|l| (ring(&l["points"]), l["closed"].as_bool().expect("closed")))
                .collect(),
        );
    }
    let areas = v.get("areas").expect("points, lines or areas");
    Gen::Areas(
        areas
            .as_array()
            .expect("areas")
            .iter()
            .map(|rings| rings.as_array().expect("rings").iter().map(ring).collect())
            .collect(),
    )
}

fn close(a: Vec2, b: Vec2, tol: f64) -> bool {
    let k = tol * js_max(js_max(a.x.abs(), a.y.abs()), 1.0);
    (a.x - b.x).abs() <= k && (a.y - b.y).abs() <= k
}

/// Two rings alike, point by point; `turned`: the same ring started at another corner.
fn same_ring(got: &[Vec2], want: &[Vec2], tol: f64, turned: bool) -> bool {
    if got.len() != want.len() {
        return false;
    }
    let starts: Vec<usize> = if turned {
        (0..got.len()).collect()
    } else {
        vec![0]
    };
    starts.into_iter().any(|s| {
        want.iter()
            .enumerate()
            .all(|(i, w)| close(got[(s + i) % got.len()], *w, tol))
    })
}

fn describe(g: &Gen) -> String {
    format!("{g:?}")
}

fn check_exact(name: &str, got: &Gen, want: &Gen, tol: f64, turned: bool) {
    let ok = match (got, want) {
        (Gen::Points(a), Gen::Points(b)) => same_ring(a, b, tol, false),
        (Gen::Lines(a), Gen::Lines(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b)
                    .all(|((p, c), (q, d))| c == d && same_ring(p, q, tol, false))
        }
        (Gen::Areas(a), Gen::Areas(b)) => {
            a.len() == b.len()
                && a.iter().zip(b).all(|(p, q)| {
                    p.len() == q.len() && p.iter().zip(q).all(|(r, s)| same_ring(r, s, tol, turned))
                })
        }
        _ => false,
    };
    assert!(
        ok,
        "{name}:\n  the core: {}\n  expected: {}",
        describe(got),
        describe(want)
    );
}

fn seg_distance(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l2 = dx * dx + dy * dy;
    let t = if l2 == 0.0 {
        0.0
    } else {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / l2).clamp(0.0, 1.0)
    };
    js_hypot(p.x - a.x - t * dx, p.y - a.y - t * dy)
}

fn ring_area(r: &[Vec2]) -> f64 {
    let n = r.len();
    (0..n)
        .map(|i| r[i].x * r[(i + 1) % n].y - r[(i + 1) % n].x * r[i].y)
        .sum::<f64>()
        / 2.0
}

/// A buffer's result against what it must satisfy.
fn check_buffer(name: &str, got: &Gen, input: &Gen, check: &Json, tol: f64) {
    let Gen::Areas(parts) = got else {
        panic!("{name}: a buffer gives areas, not {}", describe(got));
    };
    assert!(!parts.is_empty(), "{name}: the buffer is empty");
    let d = check["distance"].as_f64().expect("distance");
    let on_input = check["onInput"].as_bool().expect("onInput");
    let distance = |p: Vec2| -> f64 {
        match input {
            Gen::Points(ps) => ps
                .iter()
                .map(|q| js_hypot(p.x - q.x, p.y - q.y))
                .fold(f64::INFINITY, js_min),
            Gen::Lines(paths) => paths
                .iter()
                .flat_map(|(ps, _)| ps.windows(2).map(move |w| seg_distance(p, w[0], w[1])))
                .fold(f64::INFINITY, js_min),
            Gen::Areas(_) => unreachable!("areas are checked exactly"),
        }
    };
    let k = tol * js_max(d, 1.0);
    for rings in parts {
        for r in rings {
            for &p in r {
                let s = distance(p);
                assert!(
                    (s - d).abs() <= k || (on_input && s <= k),
                    "{name}: a vertex {p:?} lies {s} m from the input, not {d}"
                );
            }
        }
    }
    let area: f64 = parts
        .iter()
        .map(|rings| rings.iter().map(|r| ring_area(r)).sum::<f64>())
        .sum();
    let lo = check["area"][0].as_f64().expect("area");
    let hi = check["area"][1].as_f64().expect("area");
    assert!(
        (lo..=hi).contains(&area),
        "{name}: the area {area} is not within {lo} … {hi}"
    );
}

#[test]
fn every_step_is_the_references() {
    let cases: Json = serde_json::from_str(CASES).expect("generator.json reads");
    let tol = cases["tolerance"].as_f64().expect("tolerance");
    let aspects = HashMap::new();
    let list = cases["cases"].as_array().expect("cases");
    assert!(list.len() >= 30, "{} cases", list.len());
    for c in list {
        let name = c["name"].as_str().expect("name");
        let env = Env {
            plot_scale: c["plotScale"].as_f64().expect("plotScale"),
            aspects: &aspects,
            screen: false,
            moment: None,
        };
        let unit = match c["unit"].as_str() {
            Some("px") => Unit::Px,
            Some("m") => Unit::M,
            _ => Unit::Mm,
        };
        let mut exprs = Exprs::default();
        let steps = exprs.steps(&CoreJson::parse(&c["steps"].to_string()).expect("steps"));
        let input = gen_of(&c["geometry"]);
        let got = run(&steps, input.clone(), &NoValues, &env, unit);
        if let Some(want) = c.get("expect") {
            let turned = c["unordered"].as_bool().unwrap_or(false);
            check_exact(name, &got, &gen_of(want), tol, turned);
        } else {
            check_buffer(name, &got, &input, &c["check"], tol);
        }
    }
}
