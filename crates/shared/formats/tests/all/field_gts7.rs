//! The Topcon GTS-7 field book (docs/adr/0169 §1) against
//! fixtures/field/v1/gts7.json (scripts/fixtures/field_gts7_cases.py, from
//! Topcon's description alone): control words and their fields, the units,
//! DDD.MMSS degrees and gon, the stations with their coordinates, the
//! observations of the point named last, the records not read with the
//! same words. The web runs the same file through WASM
//! (`apps/web/src/io/formats.wasm.test.ts`).

use kentos_contracts::{FieldObservation, FieldStation};
use kentos_formats::field::gts7::{
    BEFORE, DMS, HORIZONTAL, MIXED, NEGATIVE, OFFSET, POINT, REDUCED, TURN, UNITS, VALUE, ZENITH,
    read,
};
use serde_json::Value;

#[test]
fn a_gts7_book_reads_as_the_reference_does() {
    let file: Value =
        serde_json::from_str(include_str!("../../../../../fixtures/field/v1/gts7.json"))
            .expect("the cases read");
    assert_eq!(file["format"], "kentos.field-gts7");
    for (key, text) in [
        ("units", UNITS),
        ("mixed", MIXED),
        ("before", BEFORE),
        ("value", VALUE),
        ("dms", DMS),
        ("turn", TURN),
        ("zenith", ZENITH),
        ("negative", NEGATIVE),
        ("point", POINT),
        ("horizontal", HORIZONTAL),
        ("reduced", REDUCED),
        ("offset", OFFSET),
    ] {
        assert_eq!(file["texts"][key], text, "{key}");
    }
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() >= 3);
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
        assert_eq!(
            (got.format.as_str(), got.encoding.as_str()),
            ("gts7", "UTF-8"),
            "{name}"
        );
    }
}

/// Records cut short, overlong, not ASCII or with huge numbers never panic:
/// they are said, or passed over, and the rest of the book is read.
#[test]
fn odd_records_are_said_not_panicked_on() {
    let odd = [
        "SD",
        "SD ,,,,,,",
        "SD ğ,ğ,ğ",
        "SD 99999999999999999999999999999999999,1,1",
        "SD 1.999999999999999999999999999999999,1,1",
        "SD 1.00000000000000000000000000000,100,1",
        "OFFSET 1",
        "OFFSET x,y,z",
        "XYZ 1,2",
        "STN",
        "UNITS",
        "UNITS ,",
        "\u{feff}SS",
        "SS ,,,,,",
        "\r\r\n\n",
    ];
    for o in odd {
        let text =
            format!("UNITS M,G\nSTN S1,1.5\nSS P1,1.6,\n{o}\nSS P2,1.6,\nSD 50.0,100.0,10.0\n");
        let got = read(text.as_bytes());
        let last = got.stations.last().and_then(|s| s.observations.last());
        if !o.starts_with("STN") && !o.starts_with("UNITS") {
            assert_eq!(
                last.map(|l| l.target.as_str()),
                Some("P2"),
                "{o:?}: {got:?}"
            );
        }
    }
    for bytes in [
        &b"\xff\xfe\x00"[..],
        b"",
        b"GTS-700",
        b"UNITS M,D\nSD 1,2,3",
    ] {
        let _ = read(bytes);
    }
}

/// The GTS-7 sample (fixtures/field/v1/sample.gt7, written from the sample
/// table by scripts/fixtures/field_samples.py) reads into the GSI sample's
/// stations and observations, value for value.
#[test]
fn the_gts7_sample_is_the_gsi_sample() {
    let sample = |name: &str| {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/field/v1")
            .join(name);
        kentos_formats::field::read(&std::fs::read(path).expect("the sample"), None)
    };
    let (gsi, gts7) = (sample("sample.gsi"), sample("sample.gt7"));
    assert_eq!((gsi.format.as_str(), gts7.format.as_str()), ("gsi", "gts7"));
    assert_eq!(gsi.unit, gts7.unit);
    assert!(gts7.problems.is_empty(), "{:?}", gts7.problems);
    let without_lines = |stations: &[FieldStation]| -> Vec<FieldStation> {
        stations
            .iter()
            .map(|s| FieldStation {
                observations: s
                    .observations
                    .iter()
                    .map(|o| FieldObservation {
                        line: 0,
                        ..o.clone()
                    })
                    .collect(),
                ..s.clone()
            })
            .collect()
    };
    assert_eq!(without_lines(&gts7.stations), without_lines(&gsi.stations));
}
