//! `file.export.landxml`'s file (plan/road.md Y10.2, docs/adr/0908 §6): the shared cases' drawing written whole.
//! `KENTOS_WRITE_LANDXML=1` writes it as fixtures/road/v1/landxml/out/yol.xml, which
//! `scripts/fixtures/road_landxml_cases.py --written` checks against the LandXML 1.2 schema (xmllint) and the
//! references; without it the file goes to a folder of the test's and is read back by the LandXML reader.

use std::path::PathBuf;

use kentos_commands::{CommandResult, ExportLandxml};
use kentos_native_application::{ExecutionContext, road_landxml};

#[test]
fn a_road_project_goes_out_as_landxml() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../fixtures/commands/v1/file.export.landxml.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("setup");
    let mut doc = kentos_domain::Document::from_snapshot_v2(snapshot).expect("opens");
    let folder = if std::env::var_os("KENTOS_WRITE_LANDXML").is_some() {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/road/v1/landxml/out")
    } else {
        let d = std::env::temp_dir().join(format!("kentos-landxml-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    };
    let path = folder.join("yol.xml");
    let input = ExportLandxml {
        path: path.display().to_string(),
        surfaces: None,
        alignments: None,
        without_profiles: false,
        stamp: Some("2026-11-03T08:30:00".into()),
        overwrite: true,
    };
    let out = match road_landxml::execute(&mut ExecutionContext::new(&mut doc), input) {
        CommandResult::Completed { output, .. } => output,
        other => panic!("writes: {other:?}"),
    };
    assert_eq!(out.alignments.len(), 3);
    let read = kentos_formats::landxml::read(&std::fs::read(&path).unwrap()).expect("reads back");
    assert_eq!(read.surfaces[0].points.len(), 120);
    assert_eq!(read.metres, 1.0);
}
