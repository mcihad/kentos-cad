//! The display rule (docs/adr/0149) against the independent reference in
//! `fixtures/numeric/v1/display.json` (Python decimal, not KentOS code;
//! `scripts/fixtures/numeric_display.py`). The web's
//! `apps/web/src/core/displayNumber.test.ts` reads the same cases.

use kentos_geometry_core::display::fixed;
use serde_json::Value;

#[test]
fn every_case_is_written_as_the_reference_writes_it() {
    let path = format!(
        "{}/../../../fixtures/numeric/v1/display.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.display-fixtures");
    assert_eq!(file["version"], 1);
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() > 1000, "the reference's cases");
    let mut failures = Vec::new();
    for c in cases {
        // The text parses to the very float the reference wrote (Rust's parsing is exact).
        let v: f64 = c["v"].as_str().expect("v").parse().expect("a float");
        let d = c["d"].as_u64().expect("d") as usize;
        let want = c["expected"].as_str().expect("expected");
        let got = fixed(v, d);
        if got != want {
            failures.push(format!("{} with {d}: {got}, expected {want}", c["v"]));
        }
    }
    assert!(
        failures.is_empty(),
        "{} cases differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
