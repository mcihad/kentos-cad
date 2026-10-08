//! The snap additions (docs/adr/0163) against the independent reference in
//! `fixtures/snap/v1/cases.json` (`scripts/fixtures/snap_cases.py`: exact
//! rationals, the arcs with 50-digit mpmath, no KentOS code): Ağırlık
//! merkezi, Karelaj, Uzantı, Paralel, a layer's own kinds and the object
//! being drawn, through `Store::snap_ex` as the platforms call it; the kind
//! and the object exactly, the point within 1e-9 m; and the acquisitions
//! (`extensions_at`, `direction_at`) within 1e-12.

// Test harness code, not the core: the std float methods are fine here.
#![allow(clippy::disallowed_methods)]

use kentos_geometry_core::Vec2;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::store::Store;
use kentos_geometry_core::store::snap::{Extension, SnapExtras, SnapKind};
use serde_json::Value;

fn fixture() -> Value {
    let path = format!(
        "{}/../../../fixtures/snap/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.snap");
    assert_eq!(file["version"], 1);
    file
}

fn num(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

fn pt(v: &Value) -> Vec2 {
    Vec2::new(num(&v[0]), num(&v[1]))
}

fn store(case: &Value) -> Store {
    let mut s = Store::new();
    s.put_json(&case["entities"].to_string()).expect("entities");
    if let Some(layers) = case.get("layers") {
        s.set_layers_json(&layers.to_string()).expect("layers");
    }
    s
}

/// Extensions as the WASM records give them.
fn extensions(records: &[Value]) -> Vec<Extension> {
    let f: Vec<f64> = records.iter().map(num).collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < f.len() {
        if f[i] == 0.0 {
            out.push(Extension::Line {
                end: Vec2::new(f[i + 1], f[i + 2]),
                dir: Vec2::new(f[i + 3], f[i + 4]),
            });
            i += 5;
        } else {
            out.push(Extension::Arc {
                c: Vec2::new(f[i + 1], f[i + 2]),
                r: f[i + 3],
                a0: f[i + 4],
                sweep: f[i + 5],
            });
            i += 6;
        }
    }
    out
}

fn extras(v: &Value) -> SnapExtras {
    let draft = v
        .get("draft")
        .map(|d| {
            vec![Shape::Polyline {
                pts: d["pts"].as_array().unwrap().iter().map(pt).collect(),
                bulges: d
                    .get("bulges")
                    .and_then(Value::as_array)
                    .map(|b| b.iter().map(num).collect()),
                holes: None,
                parts: None,
            }]
        })
        .unwrap_or_default();
    SnapExtras {
        extensions: v
            .get("extensions")
            .and_then(Value::as_array)
            .map(|e| {
                e.iter()
                    .flat_map(|r| extensions(r.as_array().unwrap()))
                    .collect()
            })
            .unwrap_or_default(),
        parallels: v
            .get("parallels")
            .and_then(Value::as_array)
            .map(|p| p.iter().map(pt).collect())
            .unwrap_or_default(),
        draft,
        grid: v.get("grid").map(|g| [num(&g[0]), num(&g[1])]),
    }
}

#[test]
fn every_snap_is_the_references() {
    let file = fixture();
    let names: Vec<&str> = file["kinds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|k| k.as_str().unwrap())
        .collect();
    // The fixture's kinds are the core's, bit for bit.
    assert_eq!(names.len(), SnapKind::ALL.len());
    let cases = file["snap"].as_array().expect("snap");
    assert!(cases.len() >= 35, "{} cases", cases.len());
    let mut failures = Vec::new();
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let kinds = c["kinds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|k| {
                1u32 << names
                    .iter()
                    .position(|n| *n == k.as_str().unwrap())
                    .unwrap()
            })
            .fold(0, |m, b| m | b);
        let from = (!c["from"].is_null()).then(|| pt(&c["from"]));
        let got = store(c).snap_ex(
            pt(&c["p"]),
            num(&c["tol"]),
            kinds,
            from,
            &extras(&c["extras"]),
        );
        let want = &c["expect"];
        match (got, want.is_null()) {
            (None, true) => {}
            (Some(h), true) => failures.push(format!("{name}: {h:?}, beklenen yok")),
            (None, false) => failures.push(format!("{name}: yok, beklenen {want}")),
            (Some(h), false) => {
                let kind = names[h.kind as usize];
                let w = pt(&want["point"]);
                // A grid node is the decimal node's nearest double, bit for bit.
                let tol = if kind == "grid" { 0.0 } else { 1e-9 };
                if kind != want["kind"].as_str().unwrap()
                    || h.id != num(&want["id"])
                    || (h.point.x - w.x).abs() > tol
                    || (h.point.y - w.y).abs() > tol
                {
                    failures.push(format!(
                        "{name}: {kind} {:?} #{}, beklenen {want}",
                        h.point, h.id
                    ));
                }
            }
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
fn extensions_and_directions_are_acquired_as_the_reference_says() {
    let file = fixture();
    for c in file["extensionsAt"].as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let got = store(c).extensions_at(num(&c["id"]), pt(&c["at"]));
        let want = extensions(c["expect"].as_array().unwrap());
        assert_eq!(got.len(), want.len(), "{name}: {got:?}");
        for (g, w) in got.iter().zip(&want) {
            let close = |a: f64, b: f64| (a - b).abs() <= 1e-12;
            let same = match (g, w) {
                (Extension::Line { end, dir }, Extension::Line { end: e, dir: d }) => {
                    close(end.x, e.x) && close(end.y, e.y) && close(dir.x, d.x) && close(dir.y, d.y)
                }
                (
                    Extension::Arc { c, r, a0, sweep },
                    Extension::Arc {
                        c: c2,
                        r: r2,
                        a0: a2,
                        sweep: s2,
                    },
                ) => {
                    close(c.x, c2.x)
                        && close(c.y, c2.y)
                        && close(*r, *r2)
                        && close(*a0, *a2)
                        && close(*sweep, *s2)
                }
                _ => false,
            };
            assert!(same, "{name}: {g:?} ≠ {w:?}");
        }
    }
    for c in file["directionAt"].as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let got = store(c).direction_at(pt(&c["p"]), num(&c["tol"]));
        match (got, c["expect"].is_null()) {
            (None, true) => {}
            (Some(u), false) => {
                let w = pt(&c["expect"]);
                assert!(
                    (u.x - w.x).abs() <= 1e-12 && (u.y - w.y).abs() <= 1e-12,
                    "{name}: {u:?} ≠ {w:?}"
                );
            }
            (got, _) => panic!("{name}: {got:?}, beklenen {}", c["expect"]),
        }
    }
}

#[test]
fn along_an_extension_as_the_reference_says() {
    let file = fixture();
    let one = |v: &Value| extensions(v.as_array().unwrap())[0];
    for c in file["extensionAlong"].as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let got = one(&c["ext"]).along(pt(&c["p"]));
        match (got, c["expect"].as_f64()) {
            (None, None) => {}
            (Some(d), Some(w)) => assert!((d - w).abs() <= 1e-9, "{name}: {d} ≠ {w}"),
            (got, _) => panic!("{name}: {got:?}, beklenen {}", c["expect"]),
        }
    }
    for c in file["extensionAt"].as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let got = one(&c["ext"]).at(num(&c["d"]));
        match (got, c["expect"].is_null()) {
            (None, true) => {}
            (Some(q), false) => {
                let w = pt(&c["expect"]);
                assert!(
                    (q.x - w.x).abs() <= 1e-9 && (q.y - w.y).abs() <= 1e-9,
                    "{name}: {q:?} ≠ {w:?}"
                );
            }
            (got, _) => panic!("{name}: {got:?}, beklenen {}", c["expect"]),
        }
    }
}
