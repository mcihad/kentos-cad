//! The Sokkia SDR field book (docs/adr/0169 §1) against
//! fixtures/field/v1/sdr.json (scripts/fixtures/field_sdr_cases.py, from
//! Sokkia's description alone): SDR2x and SDR33 records, the jobs' units,
//! the stations with their coordinates and heights, the observations in the
//! book's angle unit, the records not read with the same words. The web
//! runs the same file through WASM (`apps/web/src/io/formats.wasm.test.ts`).

use kentos_contracts::{FieldObservation, FieldStation};
use kentos_formats::field::sdr::{
    ANGLE_UNIT, BEFORE, COORDINATES, CORRECTED, DIRECTION, DISTANCE_UNIT, HORIZONTAL, MIXED,
    NEGATIVE, TARGET, TURN, VALUE, ZENITH, read,
};
use serde_json::Value;

#[test]
fn an_sdr_book_reads_as_the_reference_does() {
    let file: Value = serde_json::from_str(include_str!("../../../../fixtures/field/v1/sdr.json"))
        .expect("the cases read");
    assert_eq!(file["format"], "kentos.field-sdr");
    for (key, text) in [
        ("before", BEFORE),
        ("angleUnit", ANGLE_UNIT),
        ("distanceUnit", DISTANCE_UNIT),
        ("direction", DIRECTION),
        ("mixed", MIXED),
        ("value", VALUE),
        ("turn", TURN),
        ("negative", NEGATIVE),
        ("target", TARGET),
        ("horizontal", HORIZONTAL),
        ("zenith", ZENITH),
        ("corrected", CORRECTED),
        ("coordinates", COORDINATES),
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
        assert_eq!(
            (got.format.as_str(), got.encoding.as_str()),
            ("sdr", "UTF-8"),
            "{name}"
        );
        assert!(got.first_line.is_empty(), "{name}");
    }
}

/// Records cut short, overlong, not ASCII or with huge numbers never panic:
/// they are said, or passed over, and the rest of the book is read.
#[test]
fn odd_records_are_said_not_panicked_on() {
    let header = "00NMSDR33 V04-04.02 0000                211111";
    let odd = [
        "0",
        "00",
        "00NM",
        "09",
        "09F1",
        "09F1ğğğğğğğğğğğğğğğğğğğğğğğğğğğğğğğğğğğğğğğğğğğğğğğğğğğğğğğğğğğğ",
        "02TP",
        "02TPS1              99999999999999999999999999999999",
        "09F1S1              P1              999999999999999.9999999999999999999999",
        "09F1S1              P1              1.0             100.000000000000000000000000000001",
        "03NM-",
        "03NM.",
        "01",
        "\u{feff}00NMSDR33",
        "\r\r\n\n",
    ];
    for o in odd {
        let text = format!(
            "{header}\n{o}\n09F1S1              P2              10.0            100.0           50.0\n"
        );
        let got = read(text.as_bytes());
        // A line that begins a header begins a job without units: it is said, the rest passed over.
        if !o.starts_with("00") {
            let last = got.stations.last().and_then(|s| s.observations.last());
            assert_eq!(
                last.map(|l| l.target.as_str()),
                Some("P2"),
                "{o:?}: {got:?}"
            );
        }
    }
    for bytes in [&b"\xff\xfe\x00"[..], b"", b"\x02", b"\x03"] {
        let _ = read(bytes);
    }
}

/// Every sample book is the same book (fixtures/field/v1, written from one
/// table by scripts/fixtures/field_samples.py): the SDR33 sample reads into
/// the GSI sample's stations and observations, value for value.
#[test]
fn the_sdr_sample_is_the_gsi_sample() {
    let sample = |name: &str| {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/field/v1")
            .join(name);
        kentos_formats::field::read(&std::fs::read(path).expect("the sample"), None)
    };
    let (gsi, sdr) = (sample("sample.gsi"), sample("sample.sdr"));
    assert_eq!((gsi.format.as_str(), sdr.format.as_str()), ("gsi", "sdr"));
    assert_eq!(gsi.unit, sdr.unit);
    assert!(gsi.problems.is_empty() && sdr.problems.is_empty());
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
    assert_eq!(without_lines(&sdr.stations), without_lines(&gsi.stations));
    assert_eq!(gsi.stations.len(), 2);
}
