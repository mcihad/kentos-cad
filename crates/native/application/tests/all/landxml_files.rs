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
        // Docs/adr/1031: the drawing says its heights' system, so it goes out for machine control too.
        machine_control: true,
    };
    let out = match road_landxml::execute(&mut ExecutionContext::new(&mut doc), input) {
        CommandResult::Completed { output, .. } => output,
        other => panic!("writes: {other:?}"),
    };
    assert_eq!(out.alignments.len(), 3);
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("<CoordinateSystem ") && text.contains("verticalCoordinateSystemName="), "the systems said");
    let read = kentos_formats::landxml::read(&std::fs::read(&path).unwrap()).expect("reads back");
    assert_eq!(read.surfaces[0].points.len(), 120);
    assert_eq!(read.metres, 1.0);
}

/// Docs/adr/0988: an alignment at 80 km/h with its curve at 50 goes out as a roadway's design speeds, start–TS 80,
/// TS–ST 50, ST–end 80; the file holds to the LandXML 1.2 schema (xmllint, when there is one) and comes back in
/// with the same speeds.
#[test]
fn design_speeds_go_out_as_a_roadways_and_come_back() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../fixtures/commands/v1/file.export.landxml.json")).expect("cases");
    let mut setup = cases["setup"].clone();
    let g1 = setup["settings"]["road"]["alignments"].as_array_mut().unwrap().iter_mut().find(|a| a["id"] == "g-1").unwrap();
    g1["designSpeed"] = serde_json::json!(80.0);
    g1["pis"].as_array_mut().unwrap().iter_mut().find(|p| p["id"] == "p1").unwrap()["speed"] = serde_json::json!(50.0);
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("setup");
    let mut doc = kentos_domain::Document::from_snapshot_v2(snapshot).expect("opens");
    let curve = {
        let built = kentos_native_application::road_alignments::made_alignments(&doc)
            .into_iter()
            .find(|(d, _)| d.id == "g-1")
            .and_then(|(_, b)| b.ok())
            .expect("g-1 builds");
        let c = built.core.curves[0].clone();
        (c.stations[0], c.stations[3], built.core.length)
    };
    let d = std::env::temp_dir().join(format!("kentos-landxml-speed-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let path = d.join("hiz.xml");
    let input = ExportLandxml {
        path: path.display().to_string(),
        surfaces: Some(vec![]),
        alignments: Some(vec!["g-1".into()]),
        without_profiles: true,
        stamp: Some("2026-11-06T10:00:00".into()),
        overwrite: true,
        machine_control: false,
    };
    match road_landxml::execute(&mut ExecutionContext::new(&mut doc), input) {
        CommandResult::Completed { .. } => {}
        other => panic!("writes: {other:?}"),
    }
    let read = kentos_formats::landxml::read(&std::fs::read(&path).unwrap()).expect("reads back");
    let a = &read.alignments[0];
    let (ts, st, length) = curve;
    let want = [(0.0, ts, 80.0), (ts, st, 50.0), (st, length, 80.0)];
    assert_eq!(a.speeds.len(), 3, "{:?}", a.speeds);
    for (s, (from, to, v)) in a.speeds.iter().zip(want) {
        assert!((s.sta_start.unwrap() - (a.sta_start + from)).abs() < 1e-6 && (s.sta_end.unwrap() - (a.sta_start + to)).abs() < 1e-6, "{s:?}");
        assert_eq!(s.speed, v);
    }
    let xsd = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/road/v1/landxml/LandXML-1.2.xsd");
    if let Ok(o) = std::process::Command::new("xmllint").arg("--noout").arg("--schema").arg(&xsd).arg(&path).output() {
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    }
    // Back in, into a road project without alignments.
    let mut empty: serde_json::Value = cases["setup"].clone();
    empty["settings"]["road"].as_object_mut().unwrap().retain(|k, _| k == "surfaces");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(empty).expect("setup");
    let mut fresh = kentos_domain::Document::from_snapshot_v2(snapshot).expect("opens");
    let input = kentos_commands::AlignmentsImport {
        path: path.display().to_string(),
        alignments: None,
        without_profiles: true,
        expected_revision: None,
    };
    match kentos_native_application::road_alignments_import::execute(&mut ExecutionContext::new(&mut fresh), input) {
        CommandResult::Completed { .. } => {}
        other => panic!("reads in: {other:?}"),
    }
    let back = &fresh.road_alignments()[0];
    assert_eq!(back.design_speed, Some(80.0));
    assert_eq!(back.pis.iter().find(|p| p.id == "p1").and_then(|p| p.speed), Some(50.0));
}
