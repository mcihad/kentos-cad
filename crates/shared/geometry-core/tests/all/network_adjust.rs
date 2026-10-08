//! Ağ dengelemesi ve kot ağı (docs/adr/0203) against the independent
//! reference in `fixtures/network-adjust/v1/cases.json`
//! (`scripts/fixtures/network_adjust_cases.py`, mpmath at 50 digits, no
//! KentOS code): the adjusted coordinates and heights, their standard
//! deviations and error ellipses, the orientations, every observation's
//! residual, redundancy number and test value, the counts, m0 and the model
//! test, the refusals word for word; by name, as the web calls them.

use kentos_geometry_core::api::run_named;
use kentos_geometry_core::jsmath::{atan2, js_hypot, js_max, js_min};
use serde_json::{Value, json};

fn file() -> Value {
    let path = format!(
        "{}/../../../fixtures/network-adjust/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.network-adjust-cases");
    file
}

fn call(name: &str, args: Value) -> Result<Value, String> {
    run_named(name, &args.to_string()).map(|s| serde_json::from_str(&s).expect("JSON"))
}

fn f(v: &Value) -> f64 {
    v.as_f64().expect("number")
}

/// Within `abs` of the reference, or `rel` of it.
fn near(got: &Value, want: &Value, abs: f64, rel: f64) -> bool {
    match (got.as_f64(), want.as_f64()) {
        (Some(g), Some(w)) => (g - w).abs() <= js_max(abs, rel * w.abs()),
        _ => got.is_null() && want.is_null(),
    }
}

/// A case's input as the window gives it.
fn input(c: &Value) -> Value {
    if c["kind"] == "horizontal" {
        json!({
            "unit": c["unit"],
            "sigma": c["sigma"],
            "grid": c.get("grid").cloned().unwrap_or(Value::Null),
            "known": c["known"],
            "approx": c.get("approx").cloned().unwrap_or(json!([])),
            "rows": c["rows"],
        })
    } else {
        json!({
            "kind": c["levelKind"],
            "sigma": c["sigma"],
            "known": c["known"],
            "rows": c["rows"],
        })
    }
}

#[test]
fn the_adjustments_give_what_the_reference_gives() {
    let file = file();
    for c in file["cases"].as_array().expect("cases") {
        let name = c["name"].as_str().expect("name");
        // A case's own tolerances over the file's (the grid's factors are PROJ's numerical derivatives).
        let tol = |k: &str| {
            f(c.get("tolerances")
                .and_then(|t| t.get(k))
                .unwrap_or(&file["tolerances"][k]))
        };
        let (coord, height, rel, angle, wtol) = (
            tol("coordinate"),
            tol("height"),
            tol("relative"),
            tol("angle"),
            tol("w"),
        );
        let op = if c["kind"] == "horizontal" {
            "networkAdjust"
        } else {
            "levelAdjust"
        };
        let got = call(op, json!([input(c)]));
        if let Some(error) = c.get("error") {
            assert_eq!(got.err().as_deref(), error.as_str(), "{name}");
            continue;
        }
        let got = got.unwrap_or_else(|e| panic!("{name}: {e}"));
        let want = &c["expect"];
        for key in ["n", "u", "f", "worst", "passed"] {
            assert_eq!(got[key], want[key], "{name}: {key}");
        }
        assert!(
            near(&got["omega"], &want["omega"], 1e-9, rel),
            "{name}: Ω {} ≠ {}",
            got["omega"],
            want["omega"]
        );
        assert!(
            near(&got["m0"], &want["m0"], 1e-12, rel),
            "{name}: m0 {} ≠ {}",
            got["m0"],
            want["m0"]
        );
        assert!(near(&got["chi2"], &want["chi2"], 0.0, 1e-9), "{name}: χ²");
        let (gp, wp) = (
            got["points"].as_array().expect("points"),
            want["points"].as_array().expect("points"),
        );
        assert_eq!(gp.len(), wp.len(), "{name}: noktalar");
        for (g, w) in gp.iter().zip(wp) {
            assert_eq!(g["name"], w["name"], "{name}");
            let p = &w["name"];
            if c["kind"] == "horizontal" {
                for k in ["y", "x"] {
                    assert!(
                        near(&g[k], &w[k], coord, 0.0),
                        "{name} {p}: {k} {} ≠ {}",
                        g[k],
                        w[k]
                    );
                }
                for k in ["sy", "sx", "sp", "a", "b"] {
                    assert!(
                        near(&g[k], &w[k], 1e-9, rel),
                        "{name} {p}: {k} {} ≠ {}",
                        g[k],
                        w[k]
                    );
                }
                // The axis's bearing only where the ellipse is no circle.
                if (f(&w["a"]) - f(&w["b"])) > 1e-3 * f(&w["a"]) {
                    let d = (f(&g["theta"]) - f(&w["theta"])).abs();
                    let d = js_min(d, std::f64::consts::PI - d);
                    assert!(d <= 1e-6, "{name} {p}: θ {} ≠ {}", g["theta"], w["theta"]);
                }
            } else {
                assert!(
                    near(&g["h"], &w["h"], height, 0.0),
                    "{name} {p}: h {} ≠ {}",
                    g["h"],
                    w["h"]
                );
                assert!(near(&g["sh"], &w["sh"], 1e-9, rel), "{name} {p}: σH");
            }
        }
        if let Some(wo) = want.get("orientations") {
            for (g, w) in got["orientations"]
                .as_array()
                .expect("orientations")
                .iter()
                .zip(wo.as_array().expect("orientations"))
            {
                assert_eq!(g["station"], w["station"], "{name}");
                let d = (f(&g["z"]) - f(&w["z"])).abs();
                let d = js_min(d, std::f64::consts::TAU - d);
                assert!(
                    d <= angle * 10.0,
                    "{name}: {} yöneltmesi {} ≠ {}",
                    w["station"],
                    g["z"],
                    w["z"]
                );
            }
        }
        let (go, wo) = (
            got["observations"].as_array().expect("obs"),
            want["observations"].as_array().expect("obs"),
        );
        assert_eq!(go.len(), wo.len(), "{name}: gözlemler");
        for (i, (g, w)) in go.iter().zip(wo).enumerate() {
            for k in ["kind", "row", "flag"] {
                assert_eq!(g[k], w[k], "{name} gözlem {i}: {k}");
            }
            let vtol = if w["kind"] == "direction" {
                angle
            } else {
                1e-7
            };
            assert!(
                near(&g["v"], &w["v"], vtol, 0.0),
                "{name} gözlem {i}: v {} ≠ {}",
                g["v"],
                w["v"]
            );
            assert!(
                near(&g["sigma"], &w["sigma"], 0.0, 1e-12),
                "{name} gözlem {i}: σ"
            );
            assert!(
                near(&g["r"], &w["r"], 1e-7, 0.0),
                "{name} gözlem {i}: r {} ≠ {}",
                g["r"],
                w["r"]
            );
            assert!(
                near(&g["w"], &w["w"], wtol, 0.0),
                "{name} gözlem {i}: w {} ≠ {}",
                g["w"],
                w["w"]
            );
        }
    }
}

#[test]
fn the_chi2_quantiles_are_the_references() {
    let file = file();
    for q in file["chi2"].as_array().expect("chi2") {
        let got = call("chi2Quantile", json!([q["f"], 0.95])).expect("chi2Quantile");
        assert!(
            near(&got, &q["chi2"], 0.0, 1e-9),
            "f {}: {got} ≠ {}",
            q["f"],
            q["chi2"]
        );
    }
}

#[test]
fn a_field_book_gives_a_networks_rows_and_a_levelling_networks() {
    // docs/adr/0203 §6: a station's reduced rows; a direction only has no distance and no height difference.
    let station = json!({
        "station": "S1",
        "instrumentHeight": 1.5,
        "observations": [
            {"target": "A", "hz": 0.0, "zenith": 100.0, "slope": 100.0, "targetHeight": 1.5},
            {"target": "B", "hz": 100.0, "zenith": 100.0, "slope": 50.0},
            {"target": "C", "hz": 200.0}
        ]
    });
    let rows = call(
        "fieldNetwork",
        json!([[station], "grad", 0.13, null, "deg"]),
    )
    .expect("fieldNetwork");
    let rows = rows.as_array().expect("rows");
    assert_eq!(rows.len(), 3);
    assert_eq!(
        (rows[0]["station"].as_str(), rows[0]["target"].as_str()),
        (Some("S1"), Some("A"))
    );
    assert!((f(&rows[1]["direction"]) - 90.0).abs() < 1e-12);
    assert!((f(&rows[1]["distance"]) - 50.0).abs() < 1e-12);
    assert!(rows[2]["distance"].is_null());
    let levels = call("fieldLevels", json!([[station], "grad", 0.13, null])).expect("fieldLevels");
    let levels = levels.as_array().expect("levels");
    assert_eq!(levels.len(), 2, "a direction only has no height difference");
    let curvature = (1.0 - 0.13) * 100.0 * 100.0 / (2.0 * 6_371_000.0);
    assert!(
        (f(&levels[0]["dh"]) - curvature).abs() < 1e-9,
        "{}",
        levels[0]["dh"]
    );
    assert!((f(&levels[0]["length"]) - 100.0).abs() < 1e-12);
    assert!(
        (f(&levels[1]["dh"]) - (1.5 + (1.0 - 0.13) * 2500.0 / (2.0 * 6_371_000.0))).abs() < 1e-9
    );
}

/// How long a grid network takes (release build): N × N points about 200 m
/// apart, the first row known, every point a station with directions to its
/// neighbours and the distances to its right and upper ones, read with a few
/// cc and millimetres of error:
/// `cargo test --release -p kentos-geometry-core --test all network_adjust::timing -- --ignored --nocapture`.
#[test]
#[ignore]
fn timing() {
    use std::f64::consts::{PI, TAU};
    let sigma = file()["defaults"].clone();
    for n in [12_usize, 20] {
        let name = |i: usize, j: usize| format!("P{i}-{j}");
        let place = |i: usize, j: usize| {
            (
                487_000.0 + i as f64 * 200.0 + ((i * 7 + j * 3) % 5) as f64,
                4_420_000.0 + j as f64 * 200.0 + ((i * 3 + j * 11) % 7) as f64,
            )
        };
        let known: Vec<Value> = (0..n)
            .map(|i| {
                let (y, x) = place(i, 0);
                json!({ "name": name(i, 0), "y": y, "x": x })
            })
            .collect();
        let mut rows = Vec::new();
        for i in 0..n {
            for j in 0..n {
                let (sy, sx) = place(i, j);
                let z = 0.37 * (i * n + j) as f64;
                let around = [
                    (i + 1, j),
                    (i, j + 1),
                    (i.wrapping_sub(1), j),
                    (i, j.wrapping_sub(1)),
                ];
                for (k, &(a, b)) in around.iter().enumerate() {
                    if a >= n || b >= n {
                        continue;
                    }
                    let (ty, tx) = place(a, b);
                    let t = atan2(ty - sy, tx - sx);
                    let noise = ((i * 31 + j * 17 + k * 7) % 5) as f64 - 2.0;
                    let reading = (t - z).rem_euclid(TAU) * 200.0 / PI + noise * 1e-4;
                    let distance = (k < 2).then(|| js_hypot(ty - sy, tx - sx) + noise * 0.001);
                    rows.push(json!({ "station": name(i, j), "target": name(a, b), "direction": reading, "distance": distance }));
                }
            }
        }
        let input = json!({ "unit": "grad", "sigma": sigma, "grid": null, "known": known, "approx": [], "rows": rows });
        let start = std::time::Instant::now();
        let r = call("networkAdjust", json!([input])).expect("adjusts");
        println!(
            "{n} × {n}: {} yeni nokta, {} bilinmeyen, {} gözlem, {} yineleme: {:.3} s",
            r["points"].as_array().map_or(0, Vec::len),
            r["u"],
            r["n"],
            r["iterations"],
            start.elapsed().as_secs_f64()
        );
        assert!(
            r["iterations"].as_u64().is_some_and(|k| k >= 2),
            "{}",
            r["iterations"]
        );
    }
}
