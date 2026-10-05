//! Biçim değiştir, Sürdür and the hole operations (docs/adr/0173) against the
//! independent reference in `fixtures/reshape/v1/cases.json`
//! (`scripts/fixtures/reshape_cases.py`: rings and paths spliced with exact
//! fractions, the arc case in mpmath; no KentOS code). The operations are
//! called by name, as the web calls them through WASM. Rings are compared as
//! shapes: their start, their direction and vertices on straight lines
//! between straight edges do not matter; coordinates within 1e-9 m, bulges
//! within 1e-12.

// Test harness code, not the core: the std float methods are fine here.
#![allow(clippy::disallowed_methods)]

use kentos_geometry_core::api::run_named;
use serde_json::{Value, json};

type Vertex = (f64, f64, f64);

fn ring_of(v: &Value) -> Vec<Vertex> {
    let pts = v["pts"].as_array().expect("pts");
    let bulges = v["bulges"].as_array();
    pts.iter()
        .enumerate()
        .map(|(i, p)| {
            let b = bulges
                .and_then(|b| b.get(i))
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            (p["x"].as_f64().unwrap(), p["y"].as_f64().unwrap(), b)
        })
        .collect()
}

/// A ring as a shape: repeated vertices and vertices on straight lines
/// between straight edges dropped, counter-clockwise, from its least vertex.
fn canonical(mut r: Vec<Vertex>) -> Vec<Vertex> {
    let same = |a: &Vertex, b: &Vertex| (a.0 - b.0).abs() <= 1e-9 && (a.1 - b.1).abs() <= 1e-9;
    loop {
        let n = r.len();
        if n < 3 {
            break;
        }
        let mut drop = None;
        for i in 0..n {
            let (p, q, s) = (r[(i + n - 1) % n], r[i], r[(i + 1) % n]);
            if same(&p, &q) {
                drop = Some(i);
                break;
            }
            let cross = (q.0 - p.0) * (s.1 - p.1) - (q.1 - p.1) * (s.0 - p.0);
            let scale = (q.0 - p.0).hypot(q.1 - p.1) * (s.0 - p.0).hypot(s.1 - p.1);
            if p.2.abs() <= 1e-12 && q.2.abs() <= 1e-12 && cross.abs() <= 1e-12 * scale.max(1.0) {
                drop = Some(i);
                break;
            }
        }
        match drop {
            // The previous vertex's edge now runs to the next one.
            Some(i) => {
                r.remove(i);
            }
            None => break,
        }
    }
    let n = r.len();
    let area: f64 = (0..n)
        .map(|i| r[i].0 * r[(i + 1) % n].1 - r[(i + 1) % n].0 * r[i].1)
        .sum();
    if area < 0.0 {
        // Reversed: edge i of the reversed ring is edge (n − 2 − i) of the old, turned.
        let pts: Vec<(f64, f64)> = r.iter().rev().map(|v| (v.0, v.1)).collect();
        r = (0..n)
            .map(|i| (pts[i].0, pts[i].1, -r[(2 * n - 2 - i) % n].2))
            .collect();
    }
    let start = (0..n)
        .min_by(|&a, &b| r[a].0.total_cmp(&r[b].0).then(r[a].1.total_cmp(&r[b].1)))
        .unwrap_or(0);
    r.rotate_left(start);
    r
}

fn same_ring(a: Vec<Vertex>, b: Vec<Vertex>, path: &str) -> Result<(), String> {
    let (a, b) = (canonical(a), canonical(b));
    if a.len() != b.len() {
        return Err(format!(
            "{path}: {} köşe ≠ {} köşe ({a:?} ≠ {b:?})",
            a.len(),
            b.len()
        ));
    }
    for (i, (x, y)) in a.iter().zip(&b).enumerate() {
        if (x.0 - y.0).abs() > 1e-9 || (x.1 - y.1).abs() > 1e-9 || (x.2 - y.2).abs() > 1e-12 {
            return Err(format!("{path}[{i}]: {x:?} ≠ {y:?}"));
        }
    }
    Ok(())
}

/// An area's parts as the cases list them: each its ring and its holes.
fn parts_of(e: &Value) -> Vec<(Vec<Vertex>, Vec<Vec<Vertex>>)> {
    let holes = |v: &Value| -> Vec<Vec<Vertex>> {
        v["holes"]
            .as_array()
            .map(|hs| hs.iter().map(ring_of).collect())
            .unwrap_or_default()
    };
    let mut out = vec![(ring_of(e), holes(e))];
    for p in e["parts"].as_array().into_iter().flatten() {
        out.push((ring_of(p), holes(p)));
    }
    out
}

fn same_area(got: &Value, want: &Value, name: &str) -> Result<(), String> {
    let got = parts_of(got);
    let want: Vec<(Vec<Vertex>, Vec<Vec<Vertex>>)> = want["parts"]
        .as_array()
        .expect("parts")
        .iter()
        .map(|p| {
            (
                ring_of(p),
                p["holes"].as_array().unwrap().iter().map(ring_of).collect(),
            )
        })
        .collect();
    if got.len() != want.len() {
        return Err(format!(
            "{name}: {} parça ≠ {} parça",
            got.len(),
            want.len()
        ));
    }
    for (k, ((go, gh), (wo, wh))) in got.into_iter().zip(want).enumerate() {
        same_ring(go, wo, &format!("{name}.parts[{k}]"))?;
        if gh.len() != wh.len() {
            return Err(format!(
                "{name}.parts[{k}]: {} delik ≠ {} delik",
                gh.len(),
                wh.len()
            ));
        }
        // Holes in any order: each wanted one matches some hole got.
        let mut left: Vec<Vec<Vertex>> = gh.into_iter().map(canonical).collect();
        for (h, w) in wh.into_iter().enumerate() {
            let w = canonical(w);
            let at = left
                .iter()
                .position(|g| same_ring(g.clone(), w.clone(), "").is_ok())
                .ok_or_else(|| format!("{name}.parts[{k}].holes[{h}]: {w:?} yok"))?;
            left.remove(at);
        }
    }
    Ok(())
}

fn same_path(got: &Value, want: &Value, name: &str) -> Result<(), String> {
    let (g, w) = (ring_of(got), ring_of(want));
    if g.len() != w.len() {
        return Err(format!("{name}: {g:?} ≠ {w:?}"));
    }
    for (i, (x, y)) in g.iter().zip(&w).enumerate() {
        if (x.0 - y.0).abs() > 1e-9 || (x.1 - y.1).abs() > 1e-9 || (x.2 - y.2).abs() > 1e-12 {
            return Err(format!("{name}[{i}]: {x:?} ≠ {y:?}"));
        }
    }
    Ok(())
}

#[test]
fn every_case_is_the_references() {
    let path = format!(
        "{}/../../../fixtures/reshape/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.reshape");
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() >= 30, "{} cases", cases.len());
    let mut failures = Vec::new();
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let shape = &c["shape"];
        // Sürdür's answers are the object or nothing, and a path's ends.
        let plain = match c["op"].as_str().unwrap() {
            "continue" => Some((
                "continuePath",
                json!([shape, c["fromFirst"], c["drawn"], c["bulges"]]),
            )),
            "pathEnds" => Some(("pathEnds", json!([shape]))),
            _ => None,
        };
        if let Some((op, args)) = plain {
            let got: Value = match run_named(op, &args.to_string()) {
                Ok(text) => serde_json::from_str(&text).expect("answer"),
                Err(e) => {
                    failures.push(format!("{name}: {e}"));
                    continue;
                }
            };
            let want = &c["expect"];
            let checked = if want.is_null() {
                if got.is_null() {
                    Ok(())
                } else {
                    Err(format!("{name}: hiçbiri beklenirken {got}"))
                }
            } else if let Some(ends) = want.get("ends") {
                let near = ["first", "last", "outFirst", "outLast"].iter().all(|k| {
                    ["x", "y"].iter().all(|a| {
                        (got[k][a].as_f64().unwrap_or(f64::NAN) - ends[k][a].as_f64().unwrap())
                            .abs()
                            <= 1e-12
                    })
                });
                if near {
                    Ok(())
                } else {
                    Err(format!("{name}: {got} ≠ {ends}"))
                }
            } else if got.is_null() {
                Err(format!("{name}: nesne beklenirken hiçbiri"))
            } else {
                same_path(&got, want, name)
            };
            if let Err(e) = checked {
                failures.push(e);
            }
            continue;
        }
        let (op, args) = match c["op"].as_str().unwrap() {
            "reshape" => ("reshapeBy", json!([shape, c["sketch"]])),
            "holeAdd" => ("holeAdd", json!([shape, c["ring"]])),
            "holeRemove" => ("holeRemove", json!([shape, c["at"]])),
            "holeRing" => ("holeRing", json!([shape, c["at"]])),
            other => panic!("{name}: {other}"),
        };
        let got: Value = match run_named(op, &args.to_string()) {
            Ok(text) => serde_json::from_str(&text).expect("answer"),
            Err(e) => {
                failures.push(format!("{name}: {e}"));
                continue;
            }
        };
        let checked = match (&c["refusal"], &got["refusal"]) {
            (Value::String(want), Value::Object(r)) => {
                if r["why"] == *want {
                    Ok(())
                } else {
                    Err(format!("{name}: ret {} ≠ {want}", r["why"]))
                }
            }
            (Value::String(want), _) => Err(format!("{name}: {want} reddi beklenirken {got}")),
            (_, Value::Object(r)) => Err(format!("{name}: beklenmeyen ret {}", r["why"])),
            _ => {
                let want = &c["expect"];
                let done = &got["done"];
                if want.get("ring").is_some() {
                    same_ring(ring_of(done), ring_of(&want["ring"]), name)
                } else if want["kind"] == "polygon" {
                    same_area(done, want, name)
                } else {
                    same_path(done, want, name)
                }
            }
        };
        if let Err(e) = checked {
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
