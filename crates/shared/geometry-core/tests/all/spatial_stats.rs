//! Mekânsal istatistik (docs/adr/0238) against the independent reference in
//! `fixtures/spatial-stats/v1/cases.json` (`scripts/fixtures/spatial_stats_cases.py`,
//! no KentOS code; exact places and comparisons, 40-digit mpmath): every
//! case run through the core's calls by name, as the web makes them. Texts
//! (summary, warnings, tables, attributes, colours) exactly; places and
//! lengths within 10⁻⁶ m, statistics within 10⁻⁹ (relative), p within 10⁻¹².

use kentos_geometry_core::api::run_named;
use serde_json::{Value, json};

fn cases() -> Value {
    let path = format!(
        "{}/../../../fixtures/spatial-stats/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.spatial-stats-cases");
    file
}

/// The case's call: the operation's name and its arguments.
fn call(c: &Value) -> (&'static str, Value) {
    let p = &c["params"];
    let shapes = &c["shapes"];
    match c["tool"].as_str().expect("tool") {
        "centers" => (
            "statsCenters",
            json!([
                shapes,
                p["kind"],
                p["weights"],
                p["groups"],
                p["weightField"],
                p["k"]
            ]),
        ),
        "nearest" => ("statsNearest", json!([shapes, p["area"]])),
        "moran" => (
            "statsMoran",
            json!([
                shapes,
                p["values"],
                p["field"],
                p["concept"],
                p["band"],
                p["k"],
                p["standardize"]
            ]),
        ),
        "hotSpots" => (
            "statsHotSpots",
            json!([
                shapes,
                p["values"],
                p["field"],
                p["concept"],
                p["band"],
                p["k"]
            ]),
        ),
        "dbscan" => (
            "statsDbscan",
            json!([shapes, p["radius"], p["minPoints"], p["borderNoise"]]),
        ),
        "kMeans" => ("statsKMeans", json!([shapes, p["k"]])),
        other => panic!("bilinmeyen araç {other}"),
    }
}

fn near(got: f64, want: f64, tol: f64) -> bool {
    (got - want).abs() <= tol
}

fn xy(v: &Value) -> (f64, f64) {
    (
        v["x"].as_f64().unwrap_or(f64::NAN),
        v["y"].as_f64().unwrap_or(f64::NAN),
    )
}

fn pair(v: &Value) -> (f64, f64) {
    (
        v[0].as_f64().unwrap_or(f64::NAN),
        v[1].as_f64().unwrap_or(f64::NAN),
    )
}

/// The differences between the run and the case's expectation.
fn compare(name: &str, got: &Value, want: &Value, tol: &Value) -> Vec<String> {
    let mut bad = Vec::new();
    let place = tol["place"].as_f64().expect("place");
    let rel = tol["relative"].as_f64().expect("relative");
    let pt = tol["p"].as_f64().expect("p");
    for k in ["summary", "warnings", "infos"] {
        if got[k] != want[k] {
            bad.push(format!("{name}: {k} {} ≠ {}", got[k], want[k]));
        }
    }
    let table = got.get("table").cloned().unwrap_or(Value::Null);
    if table != want.get("table").cloned().unwrap_or(Value::Null) {
        bad.push(format!("{name}: tablo {table} ≠ {}", want["table"]));
    }
    let numbers: Vec<(String, f64)> = got["numbers"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|n| {
                    (
                        n["name"].as_str().unwrap_or("").to_owned(),
                        n["value"].as_f64().unwrap_or(f64::NAN),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    let wanted = want
        .get("numbers")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if numbers.len() != wanted.len() {
        bad.push(format!("{name}: sayılar {numbers:?} ≠ {wanted:?}"));
    }
    for (k, v) in &numbers {
        let w = wanted.get(k).and_then(Value::as_f64).unwrap_or(f64::NAN);
        let ok = if k == "p" {
            near(*v, w, pt)
        } else {
            near(*v, w, rel * if w.abs() > 1.0 { w.abs() } else { 1.0 })
        };
        if !ok {
            bad.push(format!("{name}: {k} {v} ≠ {w}"));
        }
    }
    let objects = got["objects"].as_array().cloned().unwrap_or_default();
    let want_objects = want
        .get("objects")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if objects.len() != want_objects.len() {
        bad.push(format!(
            "{name}: {} nesne ≠ {}",
            objects.len(),
            want_objects.len()
        ));
    }
    for (i, (o, w)) in objects.iter().zip(&want_objects).enumerate() {
        let s = &o["shape"];
        if s["kind"] != w["kind"] || o["attrs"] != w["attrs"] {
            bad.push(format!("{name}: nesne {i} {o} ≠ {w}"));
            continue;
        }
        let close = |a: (f64, f64), b: (f64, f64), t: f64| near(a.0, b.0, t) && near(a.1, b.1, t);
        let ok = match w["kind"].as_str() {
            Some("point") => close(xy(&s["p"]), pair(&w["p"]), place),
            Some("circle") => {
                close(xy(&s["c"]), pair(&w["c"]), place)
                    && near(
                        s["r"].as_f64().unwrap_or(f64::NAN),
                        w["r"].as_f64().unwrap_or(f64::NAN),
                        place,
                    )
            }
            Some("ellipse") => {
                close(xy(&s["c"]), pair(&w["c"]), place)
                    && close(xy(&s["major"]), pair(&w["major"]), place)
                    && near(
                        s["ratio"].as_f64().unwrap_or(f64::NAN),
                        w["ratio"].as_f64().unwrap_or(f64::NAN),
                        rel,
                    )
                    && s["t0"].as_f64() == Some(0.0)
                    && s["t1"].as_f64() == Some(std::f64::consts::TAU)
            }
            _ => false,
        };
        if !ok {
            bad.push(format!("{name}: nesne {i} {s} ≠ {w}"));
        }
    }
    let copies = got["copies"].clone();
    let want_copies = want.get("copies").cloned().unwrap_or(json!([]));
    if copies != want_copies {
        bad.push(format!("{name}: kopyalar {copies} ≠ {want_copies}"));
    }
    bad
}

#[test]
fn every_case_runs_as_the_reference_says() {
    let file = cases();
    let tol = &file["tolerance"];
    let mut bad = Vec::new();
    let mut checked = 0;
    for c in file["cases"].as_array().expect("cases") {
        let name = c["name"].as_str().expect("name");
        let (op, args) = call(c);
        let want = &c["expect"];
        match (run_named(op, &args.to_string()), want.get("refused")) {
            (Err(e), Some(why)) => {
                if why.as_str() != Some(e.as_str()) {
                    bad.push(format!("{name}: ret “{e}” ≠ {why}"));
                }
            }
            (Err(e), None) => bad.push(format!("{name}: ret “{e}”")),
            (Ok(_), Some(why)) => bad.push(format!("{name}: çalıştı, ret bekleniyordu ({why})")),
            (Ok(text), None) => {
                let got: Value = serde_json::from_str(&text).expect("the run's JSON");
                bad.extend(compare(name, &got, want, tol));
            }
        }
        checked += 1;
    }
    assert!(bad.is_empty(), "{} fark:\n{}", bad.len(), bad.join("\n"));
    assert_eq!(checked, file["cases"].as_array().map_or(0, Vec::len));
}
