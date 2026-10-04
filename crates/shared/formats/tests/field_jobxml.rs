//! The Trimble JobXML field book (docs/adr/0169 §1) against
//! fixtures/field/v1/jobxml.json (scripts/fixtures/field_jobxml_cases.py,
//! from Trimble's schema alone, read with Python's own XML parser): the
//! FieldBook's stations, targets and raw readings, the records not read
//! with the same words. The web runs the same file through WASM
//! (`apps/web/src/io/formats.wasm.test.ts`).

use kentos_contracts::FieldStation;
use kentos_formats::field::jobxml::{
    DEEP, FIELDBOOK, HORIZONTAL, METHOD, NEGATIVE, ROOT, STATION, TARGET, TURN, VALUE, XML, read,
};
use serde_json::Value;

#[test]
fn a_jobxml_book_reads_as_the_reference_does() {
    let file: Value =
        serde_json::from_str(include_str!("../../../../fixtures/field/v1/jobxml.json"))
            .expect("the cases read");
    assert_eq!(file["format"], "kentos.field-jobxml");
    for (key, text) in [
        ("xml", XML),
        ("deep", DEEP),
        ("root", ROOT),
        ("fieldbook", FIELDBOOK),
        ("value", VALUE),
        ("turn", TURN),
        ("negative", NEGATIVE),
        ("method", METHOD),
        ("station", STATION),
        ("horizontal", HORIZONTAL),
        ("target", TARGET),
    ] {
        assert_eq!(file["texts"][key], text, "{key}");
    }
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() >= 7);
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
            ("jobxml", "UTF-8"),
            "{name}"
        );
    }
}

/// Documents that are not XML, cut short, deeply nested or with odd
/// values never panic: they are said, and what can be read is read.
#[test]
fn odd_documents_are_said_not_panicked_on() {
    let deep = format!(
        "<JOBFile><FieldBook>{}{}</FieldBook></JOBFile>",
        "<a>".repeat(5000),
        "</a>".repeat(5000)
    );
    let odd = [
        "",
        "<",
        "<JOBFile",
        "<JOBFile/>",
        "<JOBFile><FieldBook/></JOBFile>",
        "<JOBFile><FieldBook><PointRecord/></FieldBook></JOBFile>",
        "<JOBFile><FieldBook><PointRecord><Circle/></PointRecord></FieldBook></JOBFile>",
        "<JOBFile><FieldBook><StationRecord/><TargetRecord/></FieldBook></JOBFile>",
        "<JOBFile><FieldBook><PointRecord><Method>DirectReading</Method><Circle><HorizontalCircle>ğ</HorizontalCircle></Circle></PointRecord></FieldBook></JOBFile>",
        "<!DOCTYPE x [<!ENTITY a \"aaaaaaaaaa\">]><JOBFile><FieldBook/></JOBFile>",
        deep.as_str(),
    ];
    for o in odd {
        let _ = read(o.as_bytes());
    }
    let _ = read(b"\xff\xfe\x00<");
}

/// The JobXML sample (fixtures/field/v1/sample.jxl, written from the sample
/// table by scripts/fixtures/field_samples.py) reads into the GSI sample's
/// stations and observations, its angles in degrees (a gon is 0.9°).
#[test]
fn the_jobxml_sample_is_the_gsi_sample_in_degrees() {
    let sample = |name: &str| {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/field/v1")
            .join(name);
        kentos_formats::field::read(&std::fs::read(path).expect("the sample"), None)
    };
    let (gsi, jxl) = (sample("sample.gsi"), sample("sample.jxl"));
    assert_eq!(
        (gsi.format.as_str(), jxl.format.as_str()),
        ("gsi", "jobxml")
    );
    assert_eq!(
        (gsi.unit.as_deref(), jxl.unit.as_deref()),
        (Some("grad"), Some("deg"))
    );
    assert!(jxl.problems.is_empty(), "{:?}", jxl.problems);
    assert_eq!(jxl.stations.len(), gsi.stations.len());
    for (j, g) in jxl.stations.iter().zip(&gsi.stations) {
        assert_eq!(
            (&j.station, j.instrument_height, j.east, j.north, j.height),
            (&g.station, g.instrument_height, g.east, g.north, g.height)
        );
        assert_eq!(j.observations.len(), g.observations.len());
        for (a, b) in j.observations.iter().zip(&g.observations) {
            assert_eq!(
                (&a.target, a.slope, a.target_height, &a.code),
                (&b.target, b.slope, b.target_height, &b.code)
            );
            assert!((a.hz - b.hz * 0.9).abs() < 1e-12, "{} {}", a.hz, b.hz);
            let (za, zb) = (a.zenith.expect("a zenith"), b.zenith.expect("a zenith"));
            assert!((za - zb * 0.9).abs() < 1e-12, "{za} {zb}");
        }
    }
}
