//! Where two alternatives part against the shared cases (fixtures/road/v1/alternatives.json, docs/adr/0943): written by
//! scripts/fixtures/road_alternatives_cases.py by dense sampling and exact refinement (the alignment and vertical
//! references at 40 digits), no KentOS code and not KentOS's marching.

use kentos_road::alignment::build;
use kentos_road::alternatives::{Departure, height, plan};
use kentos_road::profile::vertical;
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/road/v1/alternatives.json");

fn n(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

fn same(name: &str, what: &str, got: &[Departure], want: &Value) {
    let want = want.as_array().expect("departures");
    assert_eq!(got.len(), want.len(), "{name}: {what}: {got:?}");
    for (g, w) in got.iter().zip(want) {
        let close = |a: f64, b: &Value, tol: f64, field: &str| {
            assert!((a - n(b)).abs() <= tol, "{name}: {what}.{field}: {a} vs {b}");
        };
        close(g.from, &w["from"], 1e-6, "from");
        close(g.to, &w["to"], 1e-6, "to");
        close(g.largest, &w["largest"], 1e-6, "largest");
        for (a, b, field) in [(g.base_from, &w["baseFrom"], "baseFrom"), (g.base_to, &w["baseTo"], "baseTo")] {
            match (a, b.as_f64()) {
                (Some(a), Some(b)) => assert!((a - b).abs() <= 1e-6, "{name}: {what}.{field}: {a} vs {b}"),
                (None, None) => {}
                _ => panic!("{name}: {what}.{field}: {a:?} vs {b}"),
            }
        }
    }
}

#[test]
fn the_departures_are_the_references() {
    let all: Value = serde_json::from_str(CASES).expect("json");
    let cases = all["cases"].as_array().expect("cases");
    assert!(cases.len() >= 15);
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let base = build(&crate::alignments::pis(&c["base"]["pis"])).expect("the base builds");
        let other = build(&crate::alignments::pis(&c["other"]["pis"])).expect("the other builds");
        let tol = n(&c["tolerance"]);
        same(name, "plan", &plan(&base, &other, tol), &c["expected"]["plan"]);
        if let Some(h) = c.get("height") {
            let bz = vertical::build(&crate::verticals::pvis(&c["base"]["pvis"])).expect("the base's profile");
            let oz = vertical::build(&crate::verticals::pvis(&c["other"]["pvis"])).expect("the other's profile");
            same(name, "height", &height(&base, &bz, &other, &oz, tol, n(h)), &c["expected"]["height"]);
        }
    }
}
