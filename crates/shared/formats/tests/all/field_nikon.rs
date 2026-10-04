//! The Nikon RAW field book (docs/adr/0169 §1) against
//! fixtures/field/v1/nikon.json (scripts/fixtures/field_nikon_cases.py, from
//! Nikon's description alone): comma-separated records, the units from the
//! download's comments, DDD.MMSS degrees and gon, zeniths or angles from the
//! horizon, the stations with their coordinates, the observations, the
//! records not read with the same words. The web runs the same file
//! through WASM (`apps/web/src/io/formats.wasm.test.ts`).

use kentos_contracts::{FieldObservation, FieldStation};
use kentos_formats::field::nikon::{
    ANGLES, BEFORE, BELOW, DISTANCE, DMS, HORIZONTAL, MIXED, NEGATIVE, TARGET, TURN, VALUE,
    VERTICAL, ZENITH, read,
};
use serde_json::Value;

#[test]
fn a_nikon_book_reads_as_the_reference_does() {
    let file: Value =
        serde_json::from_str(include_str!("../../../../../fixtures/field/v1/nikon.json"))
            .expect("the cases read");
    assert_eq!(file["format"], "kentos.field-nikon");
    for (key, text) in [
        ("distance", DISTANCE),
        ("angles", ANGLES),
        ("vertical", VERTICAL),
        ("mixed", MIXED),
        ("before", BEFORE),
        ("zenith", ZENITH),
        ("value", VALUE),
        ("dms", DMS),
        ("turn", TURN),
        ("below", BELOW),
        ("negative", NEGATIVE),
        ("target", TARGET),
        ("horizontal", HORIZONTAL),
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
            ("nikon", "UTF-8"),
            "{name}"
        );
    }
}

/// Records cut short, overlong, not ASCII or with huge numbers never panic:
/// they are said, or passed over, and the rest of the book is read.
#[test]
fn odd_records_are_said_not_panicked_on() {
    let odd = [
        "SS",
        "SS,,,,,,,,,,,,",
        "SS,ğ,ğ,ğ,ğ,ğ,ğ",
        "SS,P1,1.6,10.0,99999999999999999999999999999999999,1,x,",
        "SS,P1,1.6,10.0,1.000000000000000000000000000001,1,x,",
        "CO",
        "CO,",
        "CO,:",
        "CO,Angle Units",
        "ST",
        "MP,,,,,",
        "\u{feff}SS,P1",
        "F1,,,,,,",
        "\r\r\n\n",
    ];
    for o in odd {
        let text = format!(
            "CO,Dist Units: Metres\nCO,Angle Units: Gons\nCO,Zero VA: Zenith\nST,S1,,B,,1.5,0,0\n{o}\nSS,P2,1.6,10.0,50.0,100.0,10:00:00,\n"
        );
        let got = read(text.as_bytes());
        let last = got.stations.last().and_then(|s| s.observations.last());
        assert_eq!(
            last.map(|l| l.target.as_str()),
            Some("P2"),
            "{o:?}: {got:?}"
        );
    }
    for bytes in [&b"\xff\xfe\x00"[..], b"", b"CO,", b"SS,1,2,3,4,5,6"] {
        let _ = read(bytes);
    }
}

/// The Nikon RAW sample (fixtures/field/v1/sample-nikon.raw, written from
/// the sample table by scripts/fixtures/field_samples.py) reads into the
/// GSI sample's stations and observations, value for value.
#[test]
fn the_nikon_sample_is_the_gsi_sample() {
    let sample = |name: &str| {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/field/v1")
            .join(name);
        kentos_formats::field::read(&std::fs::read(path).expect("the sample"), None)
    };
    let (gsi, nikon) = (sample("sample.gsi"), sample("sample-nikon.raw"));
    assert_eq!(
        (gsi.format.as_str(), nikon.format.as_str()),
        ("gsi", "nikon")
    );
    assert_eq!(gsi.unit, nikon.unit);
    assert!(nikon.problems.is_empty(), "{:?}", nikon.problems);
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
    assert_eq!(without_lines(&nikon.stations), without_lines(&gsi.stations));
}
