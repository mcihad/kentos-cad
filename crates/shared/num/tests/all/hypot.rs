//! `hypot` against the independent reference in `fixtures/numeric/v1/hypot.json` (exact fractions in Python,
//! scripts/fixtures/numeric_hypot.py; not KentOS code): bit for bit, without a tolerance.

use kentos_num::hypot;
use serde_json::Value;

fn bits(v: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(v.as_str().expect("hex text"), 16).expect("hex bits"))
}

#[test]
fn every_case_is_the_nearest_double() {
    let path = format!(
        "{}/../../../fixtures/numeric/v1/hypot.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.numeric-hypot");
    assert_eq!(file["version"], 1);
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() > 5000, "{} cases", cases.len());
    let mut failures = Vec::new();
    for c in cases {
        let (x, y, want) = (bits(&c[1]), bits(&c[2]), bits(&c[3]));
        // Either order and any signs: the length does not know them.
        for got in [hypot(x, y), hypot(y, x), hypot(-x, y), hypot(-y, -x)] {
            if got.to_bits() != want.to_bits() {
                failures.push(format!("{} hypot({x:e}, {y:e}) = {got:e} ≠ {want:e}", c[0]));
                break;
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} case(s) differ:\n{}",
        failures.len(),
        cases.len(),
        failures[..failures.len().min(20)].join("\n")
    );
}
