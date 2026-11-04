//! A surface's vertical accuracy from check points against the shared cases (fixtures/road/v1/accuracy.json,
//! docs/adr/0945): written by scripts/fixtures/road_accuracy_cases.py with numpy and scipy, no KentOS code.

use kentos_road::accuracy::{accuracy, chi2_quantile};
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/road/v1/accuracy.json");

fn n(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

#[test]
fn the_chi_square_quantiles_are_scipys() {
    let doc: Value = serde_json::from_str(CASES).expect("cases");
    for c in doc["chi2"].as_array().expect("quantiles") {
        let (p, k, x) = (n(&c["p"]), n(&c["k"]), n(&c["x"]));
        let got = chi2_quantile(p, k);
        assert!((got - x).abs() <= 1e-10 * x.max(1.0), "χ²({p}, {k}) = {got} vs {x}");
    }
}

#[test]
fn the_accuracies_are_the_references() {
    let doc: Value = serde_json::from_str(CASES).expect("cases");
    let cases = doc["cases"].as_array().expect("cases");
    assert!(cases.len() >= 15);
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let d: Vec<f64> = c["differences"].as_array().unwrap().iter().map(n).collect();
        let got = accuracy(&d, c.get("survey").and_then(Value::as_f64)).expect("two or more");
        let e = &c["expected"];
        assert_eq!(got.count, e["count"].as_u64().unwrap() as usize, "{name}");
        for (key, a) in [
            ("mean", got.mean),
            ("rmse", got.rmse),
            ("rmseLow", got.rmse_low),
            ("rmseHigh", got.rmse_high),
            ("median", got.median),
            ("nmad", got.nmad),
            ("q68", got.q68),
            ("q95", got.q95),
            ("skewness", got.skewness),
            ("kurtosis", got.kurtosis),
        ] {
            let b = n(&e[key]);
            assert!((a - b).abs() <= 1e-9 * b.abs().max(1.0), "{name}: {key} {a} vs {b}");
        }
        let far: Vec<usize> = e["far"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as usize).collect();
        assert_eq!(got.far, far, "{name}: far points");
        match (got.combined, e.get("combined").and_then(Value::as_f64)) {
            (Some(a), Some(b)) => assert!((a - b).abs() <= 1e-12, "{name}: combined"),
            (None, None) => {}
            other => panic!("{name}: combined {other:?}"),
        }
    }
}
