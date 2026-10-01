//! The measures against the independent reference in
//! `fixtures/measure/v1/cases.json` (docs/adr/0149 §3; 50-digit mpmath,
//! `scripts/fixtures/measure_cases.py`, not KentOS code): every operation,
//! called by name as the web calls it through WASM
//! (`apps/web/src/wasm/measure.wasm.test.ts`), must give the exact value
//! within the case's bound and be written, with 0 to 6 decimals, as the
//! exact value is written by the display rule.

// Test harness code, not the core: the std float methods are fine here.
#![allow(clippy::disallowed_methods)]

use kentos_geometry_core::api::run_named;
use kentos_geometry_core::display::fixed;
use serde_json::Value;

#[test]
fn every_measure_is_exact_within_its_bound_and_shown_as_the_exact_value() {
    let path = format!(
        "{}/../../../fixtures/measure/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.measure-fixtures");
    assert_eq!(file["version"], 1);
    let cases = file["cases"].as_array().expect("cases");
    let mut failures = Vec::new();
    for c in cases {
        let what = c["what"].as_str().expect("what");
        let op = c["op"].as_str().expect("op");
        let args = serde_json::to_string(&c["args"]).expect("args");
        let out = match run_named(op, &args) {
            Ok(text) => serde_json::from_str::<Value>(&text).expect("result JSON"),
            Err(e) => {
                failures.push(format!("{what}: {e}"));
                continue;
            }
        };
        let picked = match c["pick"].as_str() {
            Some(key) => &out[key],
            None => &out,
        };
        let Some(got) = picked.as_f64() else {
            failures.push(format!("{what}: no number ({out})"));
            continue;
        };
        let exact: f64 = c["exact"].as_str().expect("exact").parse().expect("exact");
        let bound = c["abs"].as_f64().expect("abs") + c["rel"].as_f64().expect("rel") * exact.abs();
        let off = (got - exact).abs();
        if off > bound {
            failures.push(format!(
                "{what}: {got} ≠ {} (fark {off:.3e}, sınır {bound:.1e})",
                c["exact"].as_str().unwrap_or("")
            ));
        }
        // What the app writes: an angle in grads, as `(rad * 200) / π`.
        let shown_value = if c["unit"] == "angle" {
            (got * 200.0) / std::f64::consts::PI
        } else {
            got
        };
        for (d, want) in c["shown"].as_array().expect("shown").iter().enumerate() {
            let want = want.as_str().expect("shown text");
            let text = fixed(shown_value, d);
            if text != want {
                failures.push(format!(
                    "{what}: {d} basamakta {text}, kesin değerinki {want}"
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} sapma:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
