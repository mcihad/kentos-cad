//! `road.stakeout.send`'s string road (plan/road.md Y10.4, docs/adr/0908 §9): the shared cases' main corridor sent as
//! LandXML. `KENTOS_WRITE_LANDXML=1` writes it as fixtures/road/v1/landxml/out/aplikasyon.xml, which
//! `scripts/fixtures/road_stakeout_send_cases.py --written` checks against the LandXML 1.2 schema (xmllint) and the
//! references; without it the file goes to a folder of the test's and is read back by the LandXML reader.

use std::path::PathBuf;

use kentos_commands::{CommandResult, StakeoutSend, StakeoutSendFormat};
use kentos_native_application::{ExecutionContext, road_stakeout_send};

#[test]
fn a_stakeout_goes_out_as_a_string_road() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../fixtures/commands/v1/road.stakeout.send.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("setup");
    let mut doc = kentos_domain::Document::from_snapshot_v2(snapshot).expect("opens");
    let folder = if std::env::var_os("KENTOS_WRITE_LANDXML").is_some() {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/road/v1/landxml/out")
    } else {
        let d = std::env::temp_dir().join(format!("kentos-stakeout-send-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    };
    let path = folder.join("aplikasyon.xml");
    let input = StakeoutSend {
        corridor: "Ana koridor".into(),
        from: Some(100.0),
        to: Some(200.0),
        step: Some(20.0),
        codes: None,
        format: StakeoutSendFormat::Landxml,
        path: path.display().to_string(),
        overwrite: true,
        first: None,
        job: None,
        stamp: Some("2026-11-03T09:30:00".into()),
    };
    let out = match road_stakeout_send::execute(&mut ExecutionContext::new(&mut doc), input) {
        CommandResult::Completed { output, .. } => output,
        other => panic!("writes: {other:?}"),
    };
    assert_eq!(out.strings, 13);
    let read = kentos_formats::landxml::read(&std::fs::read(&path).unwrap()).expect("reads back");
    assert_eq!(read.alignments.len(), 1);
}
