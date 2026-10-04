//! How a field book's format is told by its content (docs/adr/0169 §1, §6)
//! against fixtures/field/v1/sniff.json (scripts/fixtures/field_sniff_cases.py,
//! from the rules alone): Leica GSI by its first line's first word, Sokkia
//! SDR by its header, anything else a text book; the one entry reads each as
//! its format. The web runs
//! the same file through WASM (`apps/web/src/io/formats.wasm.test.ts`).

use kentos_formats::field::{read, sniff};
use serde_json::Value;

#[test]
fn a_field_books_format_is_told_as_the_reference_tells_it() {
    let file: Value =
        serde_json::from_str(include_str!("../../../../fixtures/field/v1/sniff.json"))
            .expect("the cases read");
    assert_eq!(file["format"], "kentos.field-sniff");
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() >= 12);
    for case in cases {
        let name = case["name"].as_str().expect("a name");
        let bytes = case["text"].as_str().expect("a text").as_bytes();
        let want = case["format"].as_str();
        assert_eq!(sniff(bytes), want, "{name}");
        assert_eq!(read(bytes, None).format, want.unwrap_or("csv"), "{name}");
    }
}
