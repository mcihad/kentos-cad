//! Instrument coordinate files (docs/adr/0169 §4) against
//! fixtures/field/v1/write.json (scripts/fixtures/field_write_cases.py, from
//! the formats' descriptions alone): every format's text byte for byte, how
//! many points went in, those that did not with the same words, and a job's
//! name a format cannot carry. The web runs the same file through WASM
//! (`apps/web/src/io/formats.wasm.test.ts`).

use kentos_contracts::{FieldPoint, FieldWriteOptions};
use kentos_formats::field::write::{
    BLANK, CHARS, CODE_BLANK, CODE_CHARS, CODE_CONTROL, CODE_LONG, CODE_ZEROS, CONTROL, EMPTY, JOB,
    LONG, RANGE, VALUE, ZEROS, write,
};
use serde_json::Value;

#[test]
fn instrument_files_are_written_as_the_reference_writes_them() {
    let file: Value =
        serde_json::from_str(include_str!("../../../../fixtures/field/v1/write.json"))
            .expect("the cases read");
    assert_eq!(file["format"], "kentos.field-write");
    for (key, text) in [
        ("empty", EMPTY),
        ("control", CONTROL),
        ("blank", BLANK),
        ("chars", CHARS),
        ("long", LONG),
        ("zeros", ZEROS),
        ("codeControl", CODE_CONTROL),
        ("codeBlank", CODE_BLANK),
        ("codeChars", CODE_CHARS),
        ("codeLong", CODE_LONG),
        ("codeZeros", CODE_ZEROS),
        ("value", VALUE),
        ("range", RANGE),
        ("job", JOB),
    ] {
        assert_eq!(file["texts"][key], text, "{key}");
    }
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() >= 14);
    for case in cases {
        let name = case["name"].as_str().expect("a name");
        let points: Vec<FieldPoint> =
            serde_json::from_value(case["points"].clone()).expect("points");
        let options: FieldWriteOptions =
            serde_json::from_value(case["options"].clone()).expect("options");
        let got = write(&points, &options);
        let want = &case["expect"];
        if let Some(error) = want["error"].as_str() {
            assert_eq!(got.error.as_deref(), Some(error), "{name}");
            assert_eq!((got.text.as_str(), got.written), ("", 0), "{name}");
            continue;
        }
        assert_eq!(got.error, None, "{name}");
        assert_eq!(got.text, want["text"].as_str().expect("a text"), "{name}");
        assert_eq!(
            u64::from(got.written),
            want["written"].as_u64().expect("a count"),
            "{name}"
        );
        let skipped: Vec<(u64, &str, &str)> = want["skipped"]
            .as_array()
            .expect("skipped")
            .iter()
            .map(|s| {
                (
                    s["index"].as_u64().expect("an index"),
                    s["name"].as_str().expect("a name"),
                    s["problem"].as_str().expect("a problem"),
                )
            })
            .collect();
        let got_skipped: Vec<(u64, &str, &str)> = got
            .skipped
            .iter()
            .map(|s| (u64::from(s.index), s.name.as_str(), s.problem.as_str()))
            .collect();
        assert_eq!(got_skipped, skipped, "{name}");
    }
}
