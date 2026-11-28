//! The elementary functions against the independent reference in `fixtures/numeric/v1/elementary.json` (mpmath at
//! two precisions, exact fractions and the standard's rules for special values, in
//! scripts/fixtures/numeric_elementary.py; not KentOS code): bit for bit, without a tolerance.

use serde_json::Value;

fn bits(v: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(v.as_str().expect("hex text"), 16).expect("hex bits"))
}

fn one(name: &str, x: f64) -> Option<f64> {
    Some(match name {
        "sin" => kentos_num::sin(x),
        "cos" => kentos_num::cos(x),
        "tan" => kentos_num::tan(x),
        "asin" => kentos_num::asin(x),
        "acos" => kentos_num::acos(x),
        "atan" => kentos_num::atan(x),
        "exp" => kentos_num::exp(x),
        "exp2" => kentos_num::exp2(x),
        "ln" => kentos_num::ln(x),
        "log2" => kentos_num::log2(x),
        "log10" => kentos_num::log10(x),
        "cbrt" => kentos_num::cbrt(x),
        "sinh" => kentos_num::sinh(x),
        "cosh" => kentos_num::cosh(x),
        "tanh" => kentos_num::tanh(x),
        "asinh" => kentos_num::asinh(x),
        "atanh" => kentos_num::atanh(x),
        "erfc" => kentos_num::erfc(x),
        "lgamma" => kentos_num::lgamma(x),
        _ => return None,
    })
}

fn two(name: &str, a: f64, b: f64) -> Option<f64> {
    Some(match name {
        "atan2" => kentos_num::atan2(a, b),
        "pow" => kentos_num::pow(a, b),
        _ => return None,
    })
}

/// The same double: the same bits, or a NaN where a NaN is expected.
fn same(got: f64, want: Option<f64>) -> bool {
    match want {
        Some(w) => got.to_bits() == w.to_bits(),
        None => got.is_nan(),
    }
}

#[test]
fn every_case_is_the_nearest_double() {
    let path = format!(
        "{}/../../../fixtures/numeric/v1/elementary.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.numeric-elementary");
    assert_eq!(file["version"], 1);
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() > 8000, "{} cases", cases.len());
    let mut failures = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for c in cases {
        let row = c.as_array().expect("a case");
        let (kind, name) = (
            row[0].as_str().expect("kind"),
            row[1].as_str().expect("name"),
        );
        seen.insert(name.to_owned());
        let last = &row[row.len() - 1];
        let want = (last != "nan").then(|| bits(last));
        let shown = |v: Option<f64>| v.map_or("NaN".to_owned(), |v| format!("{v:e}"));
        if row.len() == 4 {
            let x = bits(&row[2]);
            let got = one(name, x).unwrap_or_else(|| panic!("no function {name}"));
            if !same(got, want) {
                failures.push(format!("{kind}: {name}({x:e}) = {got:e} ≠ {}", shown(want)));
            }
            // The pair is the two functions' own values.
            let pair = kentos_num::sin_cos(x);
            if (name == "sin" && !same(pair.0, want)) || (name == "cos" && !same(pair.1, want)) {
                failures.push(format!(
                    "{kind}: sin_cos({x:e}) = {pair:?}, its {name} ≠ {}",
                    shown(want)
                ));
            }
        } else {
            let (a, b) = (bits(&row[2]), bits(&row[3]));
            let got = two(name, a, b).unwrap_or_else(|| panic!("no function {name}"));
            if !same(got, want) {
                failures.push(format!(
                    "{kind}: {name}({a:e}, {b:e}) = {got:e} ≠ {}",
                    shown(want)
                ));
            }
        }
    }
    assert_eq!(seen.len(), 21, "{seen:?}");
    assert!(
        failures.is_empty(),
        "{} of {} case(s) differ:\n{}",
        failures.len(),
        cases.len(),
        failures[..failures.len().min(40)].join("\n")
    );
}
