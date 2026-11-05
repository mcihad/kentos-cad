//! A footprint against protected areas, against the shared cases (fixtures/road/v1/protected.json, docs/adr/0952):
//! written by scripts/fixtures/road_protected_cases.py with GEOS's polygon overlay (shapely), no KentOS code.

use kentos_road::protected::Area;
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/road/v1/protected.json");

fn pt(v: &Value) -> [f64; 2] {
    [v[0].as_f64().unwrap(), v[1].as_f64().unwrap()]
}

fn ring(v: &Value) -> Vec<[f64; 2]> {
    v.as_array().unwrap().iter().map(pt).collect()
}

#[test]
fn the_overlaps_and_reaches_are_geos() {
    let doc: Value = serde_json::from_str(CASES).expect("cases");
    for c in doc["cases"].as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let area = Area { outer: ring(&c["area"]["outer"]), holes: c["area"]["holes"].as_array().unwrap().iter().map(ring).collect() };
        let close = |a: f64, b: f64, what: &str| assert!((a - b).abs() <= 1e-9 * b.abs().max(1.0), "{name} {what}: {a} vs {b}");
        close(area.size(), c["size"].as_f64().unwrap(), "size");
        for t in c["triangles"].as_array().unwrap() {
            let v = &t["triangle"];
            let tri = [pt(&v[0]), pt(&v[1]), pt(&v[2])];
            close(area.covered(tri), t["covered"].as_f64().unwrap(), &format!("covered {tri:?}"));
        }
        for s in c["segments"].as_array().unwrap() {
            let v = &s["segment"];
            close(area.reach(pt(&v[0]), pt(&v[1])), s["reach"].as_f64().unwrap(), &format!("reach {v}"));
        }
    }
}
