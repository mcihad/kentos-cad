//! Hatch patterns (docs/adr/0186) against the independent reference in
//! `fixtures/hatch/v1/cases.json` (`scripts/fixtures/hatch_pattern_cases.py`,
//! written from the ADR, no KentOS code): the operations, called by name as
//! the web calls them through WASM, give the library as the ADR lists it, a
//! family's dashes as drawn, a pattern's families as hatch paints, its lines
//! and dots cut to a region, a pattern carried through a similarity and a
//! hatch's region (its area, holes and the islands and cutouts that reach
//! in). Numbers within 1e-9 (relative for the larger).

// Test harness code, not the core: the std float methods are fine here.
#![allow(clippy::disallowed_methods)]

use kentos_geometry_core::api::run_named;
use kentos_geometry_core::geom::hatch_pattern::{Dashes, dash_runs};
use serde_json::{Value, json};

fn cases() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../fixtures/hatch/v1/cases.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("the cases")).expect("JSON")
}

fn run(name: &str, args: Value) -> Value {
    let out = run_named(name, &args.to_string()).unwrap_or_else(|e| panic!("{name}: {e}"));
    serde_json::from_str(&out).expect("the answer is JSON")
}

/// Whether two JSON values agree, numbers within 1e-9 (relative above 1).
fn close(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (
                x.as_f64().unwrap_or(f64::NAN),
                y.as_f64().unwrap_or(f64::NAN),
            );
            (x - y).abs() <= 1e-9 * x.abs().max(y.abs()).max(1.0)
        }
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| close(p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| close(v, w)))
        }
        _ => a == b,
    }
}

/// Whether two JSON values are the same, numbers bit for bit (45 is 45.0).
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            x.as_f64().map(f64::to_bits) == y.as_f64().map(f64::to_bits)
        }
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same(p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w)))
        }
        _ => a == b,
    }
}

#[test]
fn the_library_is_the_adr_s() {
    let c = cases();
    let got = run("hatchPatterns", json!([]));
    let want = &c["library"];
    assert_eq!(got.as_array().map(Vec::len), want.as_array().map(Vec::len));
    for (g, w) in got.as_array().unwrap().iter().zip(want.as_array().unwrap()) {
        // The library's numbers are the ADR's, bit for bit.
        assert!(same(g, w), "{}:\n{g}\n≠ {w}", w["name"]);
    }
}

#[test]
fn a_family_s_dashes_are_drawn_as_the_adr_says() {
    for c in cases()["dashes"].as_array().unwrap() {
        let signed: Vec<f64> = c["signed"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        let got = match dash_runs(&signed) {
            Dashes::Whole => json!({ "kind": "whole" }),
            Dashes::Never => json!({ "kind": "never" }),
            Dashes::Runs { runs, shift } => json!({ "kind": "runs", "runs": runs, "shift": shift }),
        };
        assert!(
            close(&got, &c["expect"]),
            "{}: {got} ≠ {}",
            c["note"],
            c["expect"]
        );
    }
}

/// A paint without dashes as the reference writes it: no `dash` field.
fn paint_json(v: &Value) -> Value {
    let mut v = v.clone();
    if let Some(o) = v.as_object_mut()
        && o.get("dash").is_some_and(Value::is_null)
    {
        o.remove("dash");
    }
    v
}

#[test]
fn a_pattern_s_families_are_its_paints() {
    for c in cases()["paints"].as_array().unwrap() {
        let got = run("hatchPaints", json!([c["pattern"]]));
        let got: Vec<Value> = got.as_array().unwrap().iter().map(paint_json).collect();
        let got = Value::Array(got);
        assert!(
            close(&got, &c["expect"]),
            "{}:\n{got}\n≠ {}",
            c["note"],
            c["expect"]
        );
    }
}

#[test]
fn a_pattern_s_lines_and_dots_are_cut_to_the_region() {
    for c in cases()["pieces"].as_array().unwrap() {
        let ring: Vec<Value> = c["ring"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| json!({ "x": p[0], "y": p[1] }))
            .collect();
        let holes: Vec<Vec<Value>> = c["holes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|h| {
                h.as_array()
                    .unwrap()
                    .iter()
                    .map(|p| json!({ "x": p[0], "y": p[1] }))
                    .collect()
            })
            .collect();
        let got = run(
            "hatchPatternPieces",
            json!([ring, holes, c["pattern"], c["budget"]]),
        );
        let pt = |p: &Value| json!([p["x"], p["y"]]);
        let segments: Vec<Value> = got["segments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| json!([pt(&s[0]), pt(&s[1])]))
            .collect();
        let dots: Vec<Value> = got["dots"].as_array().unwrap().iter().map(pt).collect();
        let got = json!({ "segments": segments, "dots": dots, "capped": got["capped"] });
        assert!(
            close(&got, &c["expect"]),
            "{}:\n{got}\n≠ {}",
            c["note"],
            c["expect"]
        );
    }
}

#[test]
fn a_pattern_is_carried_through_a_similarity() {
    use kentos_geometry_core::api::json::{FromJson, Json, to_string};
    use kentos_geometry_core::entity::HatchPattern;
    use kentos_geometry_core::geom::hatch_pattern::carried;
    for c in cases()["carried"].as_array().unwrap() {
        let p = HatchPattern::from_json(&Json::parse(&c["pattern"].to_string()).unwrap()).unwrap();
        let out = carried(
            &p,
            c["turn"].as_f64().unwrap(),
            c["scale"].as_f64().unwrap(),
            c["reflect"].as_bool().unwrap(),
        );
        let got: Value = serde_json::from_str(&to_string(&out)).unwrap();
        assert!(
            close(&got, &c["expect"]),
            "{}:\n{got}\n≠ {}",
            c["note"],
            c["expect"]
        );
    }
}

/// A ring's area (shoelace), positive or negative by its way.
fn ring_area(ring: &[Value]) -> f64 {
    let p: Vec<(f64, f64)> = ring
        .iter()
        .map(|q| (q["x"].as_f64().unwrap(), q["y"].as_f64().unwrap()))
        .collect();
    (0..p.len())
        .map(|i| {
            let (a, b) = (p[i], p[(i + 1) % p.len()]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum::<f64>()
        / 2.0
}

#[test]
fn a_hatch_s_region_is_its_outer_less_what_reaches_in() {
    for c in cases()["regions"].as_array().unwrap() {
        let boxes: Vec<Vec<Value>> = c["cutouts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| {
                r.as_array()
                    .unwrap()
                    .iter()
                    .map(|p| json!({ "x": p[0], "y": p[1] }))
                    .collect()
            })
            .collect();
        let got = run(
            "hatchRegion",
            json!([c["outer"], c["islands"], boxes, c["seed"]]),
        );
        let want = &c["expect"];
        if want.is_null() {
            assert!(got.is_null(), "{}: {got}", c["note"]);
            continue;
        }
        let holes = got["holes"].as_array().unwrap();
        let area = ring_area(got["ring"].as_array().unwrap()).abs()
            - holes
                .iter()
                .map(|h| ring_area(h.as_array().unwrap()).abs())
                .sum::<f64>();
        assert!(
            (area - want["area"].as_f64().unwrap()).abs() < 1e-6,
            "{}: {area}",
            c["note"]
        );
        assert_eq!(
            holes.len() as u64,
            want["holes"].as_u64().unwrap(),
            "{}",
            c["note"]
        );
        assert_eq!(got["islands"], want["islands"], "{}", c["note"]);
        assert_eq!(got["cutouts"], want["cutouts"], "{}", c["note"]);
    }
}
