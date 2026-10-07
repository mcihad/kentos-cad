//! The attribute table's rows (docs/adr/0199 §4) against the independent
//! reference in `fixtures/feature-table/v1/cases.json`
//! (`scripts/fixtures/feature_table_cases.py`, no KentOS code): the
//! operation `featureTable`, called by name as the web calls it through
//! WASM, gives every query's rows in their order.

use kentos_geometry_core::api::run_named;
use serde_json::{Value, json};

#[test]
fn every_query_shows_the_rows_the_reference_does() {
    let path = format!(
        "{}/../../../fixtures/feature-table/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.feature-table-cases");
    let mut queries = 0;
    for table in file["tables"].as_array().expect("tables") {
        for q in table["queries"].as_array().expect("queries") {
            let args = json!([table["columns"], table["rows"], q["query"]]).to_string();
            let got: Value =
                serde_json::from_str(&run_named("featureTable", &args).expect("featureTable"))
                    .expect("JSON");
            assert_eq!(got, q["want"], "{} › {}", table["name"], q["name"]);
            queries += 1;
        }
    }
    assert!(queries >= 60, "{queries}");
}
