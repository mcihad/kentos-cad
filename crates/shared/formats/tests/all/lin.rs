//! LIN files and the patterns' lengths (docs/adr/0313 §2, §3) against the shared cases,
//! `fixtures/linetypes/v1/cases.json` (`scripts/fixtures/line_type_cases.py`): the types and what does not read, the
//! types written back, an element's length in the drawing.

use kentos_contracts::{LineTypeDef, PatternUnit, pattern_length_m};
use kentos_formats::lin::{read_lin, write_lin};
use serde_json::Value;

fn cases() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../fixtures/linetypes/v1/cases.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("the cases")).expect("they read")
}

#[test]
fn lin_files_read_as_the_cases_say() {
    let all = cases();
    for case in all["lin"].as_array().expect("lin") {
        let name = case["name"].as_str().unwrap_or_default();
        let got = read_lin(
            case["text"].as_str().expect("a text"),
            case["source"].as_str().expect("a source"),
        );
        let want: Vec<LineTypeDef> =
            serde_json::from_value(case["expect"]["types"].clone()).expect("types");
        assert_eq!(got.types, want, "{name}");
        let problems: Vec<(u64, String)> = case["expect"]["problems"]
            .as_array()
            .expect("problems")
            .iter()
            .map(|p| {
                (
                    p["line"].as_u64().unwrap_or(0),
                    p["message"].as_str().unwrap_or_default().to_owned(),
                )
            })
            .collect();
        let got: Vec<(u64, String)> = got
            .problems
            .iter()
            .map(|p| (u64::from(p.line), p.message.clone()))
            .collect();
        assert_eq!(got, problems, "{name}");
    }
}

#[test]
fn the_types_are_written_back_as_the_cases_say() {
    for case in cases()["written"].as_array().expect("written") {
        let name = case["name"].as_str().unwrap_or_default();
        let types: Vec<LineTypeDef> = serde_json::from_value(case["types"].clone()).expect("types");
        let text = write_lin(&types);
        assert_eq!(text, case["expect"].as_str().expect("a text"), "{name}");
        assert_eq!(
            read_lin(&text, "deneme.lin").types,
            types,
            "{name}: read back"
        );
    }
}

#[test]
fn an_elements_length_in_the_drawing_is_as_the_cases_say() {
    for case in cases()["scale"].as_array().expect("scale") {
        let name = case["name"].as_str().unwrap_or_default();
        let unit = match case["unit"].as_str() {
            Some("paper") => PatternUnit::Paper,
            _ => PatternUnit::Drawing,
        };
        let project = match case["drawingUnit"].as_str().unwrap_or("m") {
            "mm" => (1.0, 1000.0),
            "cm" => (1.0, 100.0),
            "in" => (254.0, 10_000.0),
            "ft" => (3048.0, 10_000.0),
            _ => (1.0, 1.0),
        };
        let f = |k: &str| case[k].as_f64().expect(k);
        let got: Vec<f64> = case["lengths"]
            .as_array()
            .expect("lengths")
            .iter()
            .map(|x| {
                pattern_length_m(
                    x.as_f64().unwrap_or(0.0),
                    unit,
                    f("plotScale"),
                    project,
                    f("ltscale"),
                    f("objectScale"),
                )
            })
            .collect();
        let want: Vec<f64> = case["expect"]["lengths"]
            .as_array()
            .expect("lengths")
            .iter()
            .map(|x| x.as_f64().unwrap_or(f64::NAN))
            .collect();
        assert_eq!(
            got.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
            want.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
            "{name}: {got:?} {want:?}"
        );
    }
}
