//! The Leica GSI field book (docs/adr/0169 §1) against
//! fixtures/field/v1/gsi.json (scripts/fixtures/field_gsi_cases.py, from
//! Leica's description alone): GSI-8 and GSI-16 blocks, the stations with
//! their coordinates and heights, the observations in the book's angle
//! unit, the blocks not read with the same words. The web runs the same
//! file through WASM (`apps/web/src/io/formats.wasm.test.ts`).

use kentos_contracts::FieldStation;
use kentos_formats::field::gsi::{HORIZONTAL, TARGET, TURN, UNIT, VALUE, WORD, read};
use serde_json::Value;

#[test]
fn a_gsi_book_reads_as_the_reference_does() {
    let file: Value =
        serde_json::from_str(include_str!("../../../../../fixtures/field/v1/gsi.json"))
            .expect("the cases read");
    assert_eq!(file["format"], "kentos.field-gsi");
    for (key, text) in [
        ("word", WORD),
        ("unit", UNIT),
        ("value", VALUE),
        ("turn", TURN),
        ("target", TARGET),
        ("horizontal", HORIZONTAL),
    ] {
        assert_eq!(file["texts"][key], text, "{key}");
    }
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() >= 4);
    for case in cases {
        let name = case["name"].as_str().expect("a name");
        let got = read(case["text"].as_str().expect("a text").as_bytes());
        assert_eq!(
            got.unit.as_deref(),
            case["expect"]["unit"].as_str(),
            "{name}"
        );
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
        assert!(got.first_line.is_empty(), "{name}");
        assert_eq!(got.format, "gsi", "{name}");
    }
}

/// Words that are not ASCII, cut short or overlong never panic: they are
/// said, or passed over, and the rest of the book is read.
#[test]
fn odd_words_are_said_not_panicked_on() {
    let odd = [
        "*",
        "**",
        "ğğğğğğ+ğğğ",
        "11ş001+P1 21.322+00000000",
        "11.... 21.322+",
        "110001+P1 21.32ş+00000000",
        "110001+P1 21.322+99999999999999999999999999",
        "110001+P1 21.324+99999999999999999999999999",
        "110001+P1 21.324+5959",
        "110001+P1 21.322+00000000 31..00+99999999999999999999999999",
        "\u{feff}110001+P1 21.322+00000000",
        "\r\r\n\n",
    ];
    for text in odd {
        let got = read(text.as_bytes());
        assert!(got.stations.len() <= 1, "{text:?}");
    }
    let mut book = odd.join("\n");
    book.push_str("\n110001+00000P99 21.322+00010000\n");
    let got = read(book.as_bytes());
    let last = got.stations.last().and_then(|s| s.observations.last());
    assert_eq!(last.map(|o| (o.target.as_str(), o.hz)), Some(("P99", 0.1)));
}
