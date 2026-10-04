//! The plain-text field book (docs/adr/0169 §1–§2) against
//! fixtures/field/v1/csv.json (scripts/fixtures/field_csv_cases.py, from the
//! rules alone): the stations, their instrument heights and observations,
//! the lines not read with the same words. The web runs the same file
//! through WASM (`apps/web/src/io/formats.wasm.test.ts`).

use kentos_contracts::{FieldCsvOptions, FieldStation};
use kentos_formats::field::{NO_HZ, POINT, ROW, read_csv};
use serde_json::Value;

#[test]
fn a_field_book_reads_as_the_reference_does() {
    let file: Value = serde_json::from_str(include_str!("../../../../fixtures/field/v1/csv.json"))
        .expect("the cases read");
    assert_eq!(file["format"], "kentos.field-csv");
    for (key, text) in [("row", ROW), ("point", POINT), ("hz", NO_HZ)] {
        assert_eq!(file["texts"][key], text, "{key}");
    }
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() >= 5);
    for case in cases {
        let name = case["name"].as_str().expect("a name");
        let mut options = case["mapping"].clone();
        options["header"] = case["header"].clone();
        let options: FieldCsvOptions = serde_json::from_value(options).expect("options");
        let got = read_csv(case["text"].as_str().expect("a text").as_bytes(), &options);
        let want: Vec<FieldStation> =
            serde_json::from_value(case["expect"]["stations"].clone()).expect("stations");
        assert_eq!(got.stations, want, "{name}");
        let problems: Vec<(u64, String)> = case["expect"]["problems"]
            .as_array()
            .expect("problems")
            .iter()
            .map(|p| {
                (
                    p["line"].as_u64().expect("a line"),
                    p["problem"].as_str().expect("a text").to_owned(),
                )
            })
            .collect();
        let got_problems: Vec<(u64, String)> = got
            .problems
            .iter()
            .map(|p| (u64::from(p.line), p.message.clone()))
            .collect();
        assert_eq!(got_problems, problems, "{name}");
        assert_eq!(got.encoding, "UTF-8", "{name}");
    }
}
