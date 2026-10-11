//! Öznitelik tablosu's Dışa aktar (docs/adr/0223 §3): the table as a
//! workbook's sheet, as fixtures/feature-table/v1/export.json says (written
//! by scripts/fixtures/feature_export_cases.py from the rule and the layer
//! fields' reference, not from this code). The web gives the same
//! (model/featureExport.test.ts).

use std::path::PathBuf;

use kentos_contracts::{
    Entity, EntityBase, LayerField, PathEntity, PointEntity, Vec2, WorkbookSheet,
};
use kentos_interaction::feature_table::{feature_table_of, workbook_sheet};
use serde_json::Value;

fn object(o: &Value, id: u32) -> Entity {
    let base = EntityBase {
        id,
        layer_id: "katman".into(),
        color: None,
        attrs: serde_json::from_value(o["attrs"].clone()).expect("attributes"),
        label: None,
        symbol: None,
        line_weight: None,
        label_pins: Vec::new(),
    };
    let v = |x: f64, y: f64| Vec2 { x, y };
    match o["kind"].as_str() {
        Some("polygon") => Entity::Polygon(PathEntity {
            base,
            pts: vec![v(0.0, 0.0), v(1.0, 0.0), v(1.0, 1.0)],
            bulges: None,
            holes: None,
            zs: None,
            parts: None,
        }),
        _ => Entity::Point(PointEntity {
            base,
            p: v(0.0, 0.0),
            z: None,
            parts: None,
        }),
    }
}

#[test]
fn the_table_is_the_sheet_the_cases_say() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/feature-table/v1/export.json");
    let doc: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("export.json")).expect("JSON");
    let cases = doc["cases"].as_array().expect("cases");
    assert!(cases.len() >= 2);
    for c in cases {
        let fields: Vec<LayerField> = serde_json::from_value(c["fields"].clone()).expect("fields");
        let objects: Vec<Entity> = c["objects"]
            .as_array()
            .expect("objects")
            .iter()
            .enumerate()
            .map(|(i, o)| object(o, i as u32 + 1))
            .collect();
        let refs: Vec<&Entity> = objects.iter().collect();
        let table = feature_table_of(&fields, &refs, &|_| false, &|_| true, None);
        let shown: Vec<usize> = (0..objects.len()).collect();
        let want: WorkbookSheet = serde_json::from_value(c["expect"].clone()).expect("a sheet");
        assert_eq!(
            workbook_sheet(&table, &shown, c["layer"].as_str().expect("a layer")),
            want,
            "{}",
            c["title"]
        );
    }
}
