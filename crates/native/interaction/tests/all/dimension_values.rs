//! A dimension's value as written in its look (docs/adr/0183 §3), the
//! formatter's `dimension_in`, holds to the `dimensionValue` cases of
//! fixtures/text/v1/styles.json (scripts/fixtures/annotation_style_cases.py);
//! the web's `dimensionValue` holds to the same.

use kentos_contracts::{DimensionLook, DrawingUnit};
use kentos_interaction::Format;
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/text/v1/styles.json");

#[test]
fn a_dimensions_value_is_written_as_the_shared_cases_say() {
    let doc: Value = serde_json::from_str(CASES).expect("the cases read");
    let cases: Vec<&Value> = doc["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .filter(|c| c["op"] == "dimensionValue")
        .collect();
    assert!(cases.len() >= 10);
    let mut wrong = Vec::new();
    for c in cases {
        let project = &c["project"];
        let f = Format {
            unit: serde_json::from_value::<DrawingUnit>(project["unit"].clone()).expect("a unit"),
            length_decimals: project["lengthDecimals"].as_u64().expect("decimals") as usize,
            ..Format::default()
        };
        let look: DimensionLook = serde_json::from_value(c["look"].clone()).expect("a look");
        let m = &c["measured"];
        let got = f.dimension_in(
            m["prefix"].as_str().unwrap_or_default(),
            m["unit"].as_str().unwrap_or_default(),
            m["value"].as_f64().expect("a value"),
            &look,
        );
        if got != c["expect"].as_str().unwrap_or_default() {
            wrong.push(format!("{}: {got}, beklenen {}", c["name"], c["expect"]));
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}
