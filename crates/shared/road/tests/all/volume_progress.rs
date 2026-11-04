//! An earthwork's progress between its original ground and a later survey against the shared cases
//! (fixtures/road/v1/volume_progress.json, docs/adr/0942): written by scripts/fixtures/road_volume_progress_cases.py
//! in exact fractions and checked there against closed forms and Y8's pairwise reference, no KentOS code.

use kentos_road::volume::progress;
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/road/v1/volume_progress.json");

fn n(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

fn tris(v: &Value) -> Vec<[[f64; 3]; 3]> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|t| {
            let p = |k: usize| [n(&t[k][0]), n(&t[k][1]), n(&t[k][2])];
            [p(0), p(1), p(2)]
        })
        .collect()
}

#[test]
fn the_progress_is_the_reference() {
    let doc: Value = serde_json::from_str(CASES).expect("cases");
    let cases = doc["cases"].as_array().expect("cases");
    assert!(cases.len() >= 20);
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let got = progress(&tris(&c["design"]), &tris(&c["original"]), &tris(&c["now"]));
        let e = &c["expected"];
        for (key, a) in [
            ("cutDone", got.cut_done),
            ("cutLeft", got.cut_left),
            ("cutOver", got.cut_over),
            ("fillDone", got.fill_done),
            ("fillLeft", got.fill_left),
            ("fillOver", got.fill_over),
            ("covered", got.covered),
            ("uncovered", got.uncovered),
        ] {
            let b = n(&e[key]);
            assert!((a - b).abs() <= 1e-9 * b.abs().max(1.0), "{name}: {key} {a} ≠ {b}");
        }
    }
}
