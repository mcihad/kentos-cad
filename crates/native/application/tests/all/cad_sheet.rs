//! A selectable frame must not change the CAD paper geometry inside it (ADR 0589).
use kentos_commands::{DocumentSnapshotV1, DrawingUnit, DxfReadOptions, Entity};
use kentos_domain::Document;
use kentos_native_application::cad_layouts;
use serde_json::Value;

#[test]
fn thin_paper_frames_preserve_all_twenty_independent_paths() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures");
    let fixture: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("commands/v1/file.import.dwg.json")).unwrap(),
    )
    .unwrap();
    let mut doc = Document::from_snapshot(
        DocumentSnapshotV1::from_json(&fixture["setup"].to_string()).unwrap(),
    )
    .unwrap();
    let cases: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("formats/v1/cad-sheet/cases.json")).unwrap(),
    )
    .unwrap();
    let cases = cases["paper"].as_array().unwrap();
    assert!(cases.len() >= 20);
    let source = std::fs::read(root.join("formats/v1/cad-sheet/thin-paper.dxf")).unwrap();
    let imported = kentos_formats::dxf::read_checked(
        &source,
        &DxfReadOptions {
            unit: Some(DrawingUnit::M),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        imported.entities.len(),
        1,
        "paper geometry stays outside model space"
    );
    assert_eq!(imported.layouts.len(), 1);
    let sheets = cad_layouts::merge(&doc, &imported.layouts)
        .expect("valid even for horizontal, vertical and sub-mm paths");
    let (book, _) = kentos_sheet::project::read_project_sheets(sheets.as_ref().unwrap()).unwrap();
    assert_eq!(book.sheets[0].items.len(), cases.len());
    for item in &book.sheets[0].items {
        assert!(item.frame.width >= kentos_sheet::validate::ITEM_MIN);
        assert!(item.frame.height >= kentos_sheet::validate::ITEM_MIN);
    }
    doc.set_sheets(sheets).unwrap();
    let (exported, losses) = cad_layouts::export(&doc);
    assert!(losses.is_empty(), "{losses:?}");
    assert_eq!(exported[0].entities.len(), cases.len());
    for (entity, case) in exported[0].entities.iter().zip(cases) {
        let (Entity::Polyline(p) | Entity::Polygon(p)) = entity else {
            panic!("paper path");
        };
        let want = case["pointsUm"].as_array().unwrap();
        assert_eq!(p.pts.len(), want.len(), "{case}");
        for (got, want) in p.pts.iter().zip(want) {
            assert!(
                (got.x * 1e6 - want[0].as_f64().unwrap()).abs() < 1.0,
                "{case}: x {got:?}"
            );
            assert!(
                (got.y * 1e6 - want[1].as_f64().unwrap()).abs() < 1.0,
                "{case}: y {got:?}"
            );
        }
    }
}
