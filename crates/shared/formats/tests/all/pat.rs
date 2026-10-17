//! PAT files and a project pattern's metres (docs/adr/0331 §1) against the shared cases, `fixtures/hatch/v1/pat.json`
//! (`scripts/fixtures/hatch_pattern_file_cases.py`): the patterns and what does not read, the patterns written back,
//! a definition unit's metres in the drawing.

use kentos_contracts::{HatchPatternDef, HatchPatternUnit};
use kentos_formats::pat::{read_pat, write_pat};
use serde_json::Value;

fn cases() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../fixtures/hatch/v1/pat.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("the cases")).expect("they read")
}

fn unit(text: &str) -> HatchPatternUnit {
    serde_json::from_value(Value::String(text.to_owned())).expect("a unit")
}

#[test]
fn pat_files_read_as_the_cases_say() {
    for case in cases()["pat"].as_array().expect("pat") {
        let name = case["name"].as_str().unwrap_or_default();
        let got = read_pat(
            case["text"].as_str().expect("a text"),
            unit(case["unit"].as_str().expect("a unit")),
            case["source"].as_str().expect("a source"),
        );
        let want: Vec<HatchPatternDef> =
            serde_json::from_value(case["expect"]["patterns"].clone()).expect("patterns");
        assert_eq!(got.patterns, want, "{name}");
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
fn the_patterns_are_written_back_as_the_cases_say() {
    for case in cases()["written"].as_array().expect("written") {
        let name = case["name"].as_str().unwrap_or_default();
        let patterns: Vec<HatchPatternDef> =
            serde_json::from_value(case["patterns"].clone()).expect("patterns");
        let text = write_pat(&patterns);
        assert_eq!(text, case["pat"].as_str().expect("a text"), "{name}");
        let back = read_pat(&text, HatchPatternUnit::Mm, "deneme.pat");
        assert!(back.problems.is_empty(), "{name}: {:?}", back.problems);
        for (b, p) in back.patterns.iter().zip(&patterns) {
            assert_eq!(
                (&b.name, &b.description, &b.lines),
                (&p.name, &p.description, &p.lines),
                "{name}"
            );
        }
    }
}

#[test]
fn a_definition_units_metres_are_as_the_cases_say() {
    for case in cases()["scale"].as_array().expect("scale") {
        let name = case["name"].as_str().unwrap_or_default();
        let (mul, div) = match case["drawingUnit"].as_str().unwrap_or("m") {
            "mm" => (1.0, 1000.0),
            "cm" => (1.0, 100.0),
            "in" => (254.0, 10_000.0),
            "ft" => (3048.0, 10_000.0),
            _ => (1.0, 1.0),
        };
        let f = |k: &str| case[k].as_f64().expect(k);
        let got = f("factor")
            * unit(case["unit"].as_str().expect("unit")).metres(f("plotScale"), mul / div);
        let want = f("metres");
        assert_eq!(got.to_bits(), want.to_bits(), "{name}: {got} ≠ {want}");
    }
}
