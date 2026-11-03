//! A road's earthwork volumes against the shared cases (fixtures/road/v1/volume.json, docs/adr/0906): written in exact
//! fractions by scripts/fixtures/road_volume_cases.py, no KentOS code.

use kentos_road::volume::{SectionAreas, between_surfaces, end_areas, prismoidal};
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/road/v1/volume.json");

fn n(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

/// A fraction written as "p/q" or "p".
fn frac(v: &Value) -> f64 {
    let s = v.as_str().expect("a fraction");
    match s.split_once('/') {
        Some((p, q)) => p.parse::<f64>().expect("p") / q.parse::<f64>().expect("q"),
        None => s.parse().expect("an integer"),
    }
}

fn triangles(v: &Value) -> Vec<[[f64; 3]; 3]> {
    v.as_array()
        .expect("triangles")
        .iter()
        .map(|t| {
            let p = |i: usize| [frac(&t[i][0]), frac(&t[i][1]), frac(&t[i][2])];
            [p(0), p(1), p(2)]
        })
        .collect()
}

fn near(a: f64, b: f64, what: &str) {
    assert!((a - b).abs() <= 1e-9 * (1.0 + b.abs()), "{what}: {a} vs {b}");
}

#[test]
fn volumes_between_surfaces_are_the_references() {
    let all: Value = serde_json::from_str(CASES).expect("json");
    for c in all["surfaces"].as_array().expect("cases") {
        let name = c["name"].as_str().expect("name");
        let got = between_surfaces(&triangles(&c["design"]), &triangles(&c["ground"]));
        near(got.cut, n(&c["cut"]), &format!("{name}: cut"));
        near(got.fill, n(&c["fill"]), &format!("{name}: fill"));
        near(got.cut_area, n(&c["cutArea"]), &format!("{name}: cut area"));
        near(got.fill_area, n(&c["fillArea"]), &format!("{name}: fill area"));
        near(got.covered, n(&c["covered"]), &format!("{name}: covered"));
        near(got.uncovered, n(&c["uncovered"]), &format!("{name}: uncovered"));
    }
}

#[test]
fn volumes_by_sections_are_the_references() {
    let all: Value = serde_json::from_str(CASES).expect("json");
    let s = &all["sections"];
    let rows: Vec<SectionAreas> = s["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .map(|r| SectionAreas { station: n(&r["station"]), areas: r.get("cut").map(|c| (n(c), n(&r["fill"]))) })
        .collect();
    let got = end_areas(&rows);
    let want = s["between"].as_array().expect("between");
    assert_eq!(got.len(), want.len());
    for (g, w) in got.iter().zip(want) {
        near(g.from, n(&w["from"]), "from");
        near(g.to, n(&w["to"]), "to");
        match (g.volumes, w.get("cut")) {
            (Some((c, f)), Some(wc)) => {
                near(c, n(wc), "cut");
                near(f, n(&w["fill"]), "fill");
            }
            (None, None) => {}
            other => panic!("{other:?}"),
        }
    }
    for p in s["prismoidal"].as_array().expect("prismoidal") {
        near(prismoidal(n(&p["a0"]), n(&p["am"]), n(&p["a1"]), n(&p["l"])), n(&p["v"]), "prismoidal");
    }
}
