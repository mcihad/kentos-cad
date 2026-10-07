//! Verilerden al (docs/adr/0199 §3) holds to the shared cases the independent
//! reference writes (scripts/fixtures/layer_field_cases.py,
//! fixtures/layer-fields/v1/cases.json): the contract's kinds composed in
//! the geometry core's natural order.

use kentos_domain::fields::fields_from_rows;
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/layer-fields/v1/cases.json");

#[test]
fn fields_from_data_are_as_the_cases_say() {
    let cases: Value = serde_json::from_str(CASES).expect("the cases");
    let infer = cases["infer"].as_array().expect("infer");
    assert!(!infer.is_empty());
    for c in infer {
        let rows: Vec<Vec<(String, String)>> = c["rows"]
            .as_array()
            .expect("rows")
            .iter()
            .map(|r| {
                r.as_object()
                    .expect("a row")
                    .iter()
                    .map(|(k, v)| (k.clone(), v.as_str().expect("a text").to_owned()))
                    .collect()
            })
            .collect();
        let got = serde_json::to_value(fields_from_rows(&rows)).expect("json");
        assert_eq!(got, c["want"], "{}", c["name"]);
    }
}
