//! Topolojik temizlik (docs/adr/0148) against the independent reference in
//! `fixtures/topology/v1/clean.json` (`scripts/fixtures/topology_cases.py`,
//! no KentOS code): the operation, called by name as the web calls it
//! through WASM, gives every case's changed paths, changes, counts and
//! largest move, numbers within 1e-9 m.

// Test harness code, not the core: the std float methods are fine here.
#![allow(clippy::disallowed_methods)]

use kentos_geometry_core::api::run_named;
use serde_json::{Value, json};

/// Numbers within 1e-9 (null equals null), everything else exactly.
fn same(a: &Value, e: &Value, path: &str) -> Result<(), String> {
    match (a, e) {
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (x.as_f64().unwrap(), y.as_f64().unwrap());
            if (x - y).abs() <= 1e-9 {
                Ok(())
            } else {
                Err(format!("{path}: {x} ≠ {y}"))
            }
        }
        (Value::Array(x), Value::Array(y)) => {
            if x.len() != y.len() {
                return Err(format!("{path}: {} öğe ≠ {} öğe", x.len(), y.len()));
            }
            for (i, (p, q)) in x.iter().zip(y).enumerate() {
                same(p, q, &format!("{path}[{i}]"))?;
            }
            Ok(())
        }
        (Value::Object(x), Value::Object(y)) => {
            let mut keys: Vec<&String> = x.keys().chain(y.keys()).collect();
            keys.sort();
            keys.dedup();
            for k in keys {
                same(
                    x.get(k).unwrap_or(&Value::Null),
                    y.get(k).unwrap_or(&Value::Null),
                    &format!("{path}.{k}"),
                )?;
            }
            Ok(())
        }
        _ if a == e => Ok(()),
        _ => Err(format!("{path}: {a} ≠ {e}")),
    }
}

#[test]
fn every_case_is_cleaned_as_the_reference_cleans_it() {
    let path = format!(
        "{}/../../../fixtures/topology/v1/clean.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.topology-fixtures");
    assert_eq!(file["version"], 1);
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() >= 50, "{} cases", cases.len());
    let mut failures = Vec::new();
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let args = json!([c["objects"], c["tolerance"], c["works"]]).to_string();
        let got: Value = match run_named("topologyClean", &args) {
            Ok(text) => serde_json::from_str(&text).expect("answer JSON"),
            Err(e) => {
                failures.push(format!("{name}: {e}"));
                continue;
            }
        };
        if let Err(e) = same(&got, &c["expected"], name) {
            failures.push(e);
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} cases differ:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}

#[test]
fn a_tolerance_below_a_micrometre_is_refused() {
    let args = json!([[], 1e-7, { "ends": true, "vertices": false, "extend": true, "trim": true }]);
    let e = run_named("topologyClean", &args.to_string()).expect_err("refused");
    assert!(e.contains("0.000001"), "{e}");
}

/// An arc goes to the cleanup as its two ends and a bulge, and comes back
/// as the same arc (docs/adr/0148 §2): counter-clockwise, angles in
/// [0, 2π), whatever turn they were given in; a whole turn has no ends.
#[test]
fn an_arc_goes_as_two_ends_and_a_bulge_and_comes_back() {
    let (e, n) = (487_000.0, 4_420_000.0);
    let pi = std::f64::consts::PI;
    for (a0, a1) in [
        (0.0, pi / 2.0),
        (5.5, 0.4),
        (-pi / 3.0, pi / 4.0),
        (1.0, 1.0 + 2.0 * pi - 0.5),
    ] {
        let arc = json!({ "c": { "x": e + 3.0, "y": n - 2.0 }, "r": 12.5, "a0": a0, "a1": a1 });
        let path: Value =
            serde_json::from_str(&run_named("topologyArcPath", &json!([arc]).to_string()).unwrap())
                .unwrap();
        let sweep = (a1 - a0).rem_euclid(2.0 * pi);
        let end = |a: f64| [e + 3.0 + 12.5 * a.cos(), n - 2.0 + 12.5 * a.sin()];
        let pts: Vec<[f64; 2]> = path["pts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| [p["x"].as_f64().unwrap(), p["y"].as_f64().unwrap()])
            .collect();
        for (have, want) in pts.iter().zip([end(a0), end(a1)]) {
            assert!(
                (have[0] - want[0]).abs() < 1e-9 && (have[1] - want[1]).abs() < 1e-9,
                "{have:?} {want:?}"
            );
        }
        let bulge = path["bulges"][0].as_f64().unwrap();
        assert!((bulge - (sweep / 4.0).tan()).abs() < 1e-12, "{bulge}");
        assert_eq!(path["bulges"][1], 0.0);
        assert_eq!(path["zs"], json!([null, null]));
        let back: Value = serde_json::from_str(
            &run_named("topologyPathArc", &json!([path]).to_string()).unwrap(),
        )
        .unwrap();
        // Ends at TM coordinates carry their last bit (1e-9 m): the centre comes back within a few.
        assert!(
            (back["c"]["x"].as_f64().unwrap() - (e + 3.0)).abs() < 1e-7,
            "{back}"
        );
        assert!(
            (back["c"]["y"].as_f64().unwrap() - (n - 2.0)).abs() < 1e-7,
            "{back}"
        );
        assert!((back["r"].as_f64().unwrap() - 12.5).abs() < 1e-7, "{back}");
        for (k, want) in [("a0", a0), ("a1", a1)] {
            let have = back[k].as_f64().unwrap();
            assert!((0.0..2.0 * pi).contains(&have), "{k} {have}");
            assert!(
                (have - want.rem_euclid(2.0 * pi)).abs() < 1e-9,
                "{k} {have} {want}"
            );
        }
    }
    let whole = json!({ "c": { "x": 0.0, "y": 0.0 }, "r": 1.0, "a0": 1.0, "a1": 1.0 });
    assert_eq!(
        run_named("topologyArcPath", &json!([whole]).to_string()).unwrap(),
        "null"
    );
    let straight = json!({ "pts": [{ "x": 0.0, "y": 0.0 }, { "x": 1.0, "y": 0.0 }], "bulges": [0.0, 0.0], "closed": false, "zs": [null, null] });
    assert_eq!(
        run_named("topologyPathArc", &json!([straight]).to_string()).unwrap(),
        "null"
    );
}

/// 50 000 lines of a street network at TM coordinates, every second end a
/// few centimetres off its neighbour's, cleaned whole (docs/adr/0148 §7:
/// under a second natively). Run by hand in release:
///
/// ```text
/// cargo test --release -p kentos-geometry-core --test topology -- --ignored --nocapture
/// ```
#[test]
#[ignore = "timings, run by hand in release"]
fn fifty_thousand_lines_are_cleaned_within_a_second() {
    use kentos_geometry_core::Vec2;
    use kentos_geometry_core::ops::topology::{TopoObject, TopoPath, TopoWorks, topology_clean};
    let (e, n) = (487_000.0, 4_420_000.0);
    let side = 158; // 158 × 158 cells, two lines a cell: ~50 000 lines
    let mut objects = Vec::new();
    let line = |a: Vec2, b: Vec2| TopoObject {
        kind: "line".into(),
        fixed: false,
        paths: vec![TopoPath {
            pts: vec![a, b],
            bulges: None,
            closed: false,
            zs: vec![None, None],
        }],
    };
    for i in 0..side {
        for j in 0..side {
            let (x, y) = (e + i as f64 * 20.0, n + j as f64 * 20.0);
            let off = if (i + j) % 2 == 0 { 0.03 } else { 0.0 };
            objects.push(line(Vec2::new(x + off, y), Vec2::new(x + 20.0, y)));
            objects.push(line(Vec2::new(x, y + off), Vec2::new(x, y + 20.0)));
        }
    }
    let works = TopoWorks {
        ends: true,
        vertices: false,
        extend: true,
        trim: true,
    };
    let start = std::time::Instant::now();
    let r = topology_clean(&objects, 0.05, works).expect("cleaned");
    let took = start.elapsed().as_secs_f64();
    println!(
        "{} çizgi: {:.3} sn, {} uç birleşti, {} uzadı, {} kısaldı",
        objects.len(),
        took,
        r.counts.ends,
        r.counts.extended,
        r.counts.trimmed
    );
    assert!(took < 1.0, "{took} s");
}
