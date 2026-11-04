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

/// A square of two triangles from (x0, y0) to (x1, y1) on the plane z = c + gx·x + gy·y.
fn square(x0: f64, y0: f64, x1: f64, y1: f64, z: impl Fn(f64, f64) -> f64) -> Vec<kentos_road::volume::Triangle> {
    let p = |x: f64, y: f64| [x, y, z(x, y)];
    vec![[p(x0, y0), p(x1, y0), p(x1, y1)], [p(x0, y0), p(x1, y1), p(x0, y1)]]
}

#[test]
fn the_difference_of_two_planes_in_closed_form() {
    // Base z = 10 over [0, 10]²; comparison z = 9 + 0.2 x over [0, 10] × [0, 10], from −1 to +1: cut where x < 5.
    let base = square(0.0, 0.0, 10.0, 10.0, |_, _| 10.0);
    let comparison = square(0.0, 0.0, 10.0, 10.0, |x, _| 9.0 + 0.2 * x);
    let d = kentos_road::volume::difference(&comparison, &base, &[-1.0, -0.5, 0.0, 0.5, 1.0]);
    // Each half: a wedge of length 5 rising to 1 over 10 wide: 25.
    assert!((d.volumes.cut - 25.0).abs() < 1e-9 && (d.volumes.fill - 25.0).abs() < 1e-9, "{d:?}");
    assert!((d.volumes.cut_area - 50.0).abs() < 1e-9 && d.volumes.uncovered == 0.0, "{d:?}");
    for b in &d.bands {
        assert!((b.2 - 25.0).abs() < 1e-9, "{b:?}");
    }
    assert!(d.outside.abs() < 1e-9);
    assert_eq!(d.least.map(|l| l.1), Some([0.0, 0.0]));
    assert!((d.largest.expect("a largest").0 - 1.0).abs() < 1e-12);
    // Half the comparison off the base: uncovered.
    let shifted = square(5.0, 0.0, 15.0, 10.0, |_, _| 11.0);
    let d = kentos_road::volume::difference(&shifted, &base, &[0.0, 2.0]);
    assert!((d.volumes.fill - 50.0).abs() < 1e-9 && (d.volumes.uncovered - 50.0).abs() < 1e-9, "{d:?}");
    assert!((d.bands[0].2 - 50.0).abs() < 1e-9);
}
