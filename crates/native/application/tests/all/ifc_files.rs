//! `file.export.ifc`'s file (plan/road.md Y10.3, docs/adr/0908 §8): `file.export.landxml`'s shared cases' drawing
//! written as IFC 4.3. `KENTOS_WRITE_IFC=1` writes it as fixtures/road/v1/ifc/out/yol.ifc, which
//! `scripts/fixtures/road_ifc_cases.py --written` checks with IfcOpenShell (the schema, the business logic against the
//! references, the curves IfcOpenShell evaluates against the references' places and heights); without it the file goes
//! to a folder of the test's.

use std::path::PathBuf;

use kentos_commands::{CommandResult, ExportIfc};
use kentos_native_application::{ExecutionContext, road_ifc};

#[test]
fn a_road_project_goes_out_as_ifc() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../fixtures/commands/v1/file.export.landxml.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("setup");
    let mut doc = kentos_domain::Document::from_snapshot_v2(snapshot).expect("opens");
    let folder = if std::env::var_os("KENTOS_WRITE_IFC").is_some() {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/road/v1/ifc/out")
    } else {
        let d = std::env::temp_dir().join(format!("kentos-ifc-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    };
    let path = folder.join("yol.ifc");
    let input = ExportIfc {
        path: path.display().to_string(),
        alignments: None,
        without_profiles: false,
        stamp: Some("2026-11-03T10:00:00".into()),
        overwrite: true,
    };
    let out = match road_ifc::execute(&mut ExecutionContext::new(&mut doc), input) {
        CommandResult::Completed { output, .. } => output,
        other => panic!("writes: {other:?}"),
    };
    assert_eq!(out.alignments.len(), 3);
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.starts_with("ISO-10303-21;") && text.contains("FILE_SCHEMA(('IFC4X3_ADD2'));"));
}
