//! docs/adr/1128 §4: a DXF's table styles taken into a project. KentOS's own come back exactly, ids and all, the
//! table following its style; a project's style of the same name is the one a table then follows; another
//! program's table style an ACAD_TABLE follows becomes a project table style (the table its block's lines and words).

use kentos_commands::{DrawingUnit, DxfReadOptions, DxfWriteInput, Entity, TableStyleDef};
use kentos_domain::Document;
use kentos_native_application::import_apply::{self, ImportPlan, LayerTarget, Styles};
use serde_json::{Value, json};

const EXPORT_INPUT: &str = include_str!("../../../../../fixtures/exchange/v1/table-styles-export.json");
const FOREIGN: &[u8] = include_bytes!("../../../../../fixtures/exchange/v1/table-styles.dxf");

fn project(table_styles: Value) -> Document {
    let style = json!({"color": "fg", "lineType": "continuous", "lineWeight": 0.25});
    Document::from_snapshot_v2(serde_json::from_value(json!({
        "format": "kentos.document", "version": 2, "name": "Tablo stili DXF",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
            "plotScale": 100.0, "workspace": "cad", "tableStyles": table_styles},
        "origin": {"x": 0.0, "y": 0.0},
        "layers": [{"id": "0", "name": "0", "type": "layer", "visible": true, "locked": false, "expanded": true,
            "style": style, "children": []}],
        "activeLayer": "0", "entities": [], "uids": [], "styles": {"items": [], "categories": []}}))
    .unwrap())
    .unwrap()
}

fn import(doc: &mut Document, bytes: &[u8]) {
    let opts = DxfReadOptions { unit: Some(DrawingUnit::M), scale: Some(100.0), ..Default::default() };
    let read = kentos_formats::dxf::read(bytes, &opts).expect("reads");
    let plan = ImportPlan {
        label: "Tablo stilleri".into(),
        layers: read.layers.iter().map(|l| (l.name.clone(), LayerTarget::Existing("0".into()))).collect(),
        group: None,
        source_srid: None,
    };
    let styles = Styles {
        text: read.text_styles,
        dimension: read.dimension_styles,
        table: read.table_styles,
        ..Styles::default()
    };
    import_apply::apply_styled_import(doc, read.entities, read.blocks, styles, &plan).expect("applies");
}

fn written() -> (Vec<u8>, Vec<TableStyleDef>) {
    let input: DxfWriteInput = kentos_formats::dxf::input_from_json(EXPORT_INPUT).expect("writer input");
    (kentos_formats::dxf::write(&input).0, input.table_styles)
}

fn tables(doc: &Document) -> Vec<Option<String>> {
    doc.entities().filter_map(|e| if let Entity::Table(t) = e { Some(t.table_style.clone()) } else { None }).collect()
}

#[test]
fn kentos_table_styles_come_back_exactly_into_a_new_project() {
    let (bytes, styles) = written();
    let mut doc = project(json!([]));
    import(&mut doc, &bytes);
    assert_eq!(doc.settings().table_styles, styles, "ids and values as they went out");
    assert_eq!(tables(&doc), [Some("ofis-cetvel".to_owned())]);
}

#[test]
fn a_table_follows_the_projects_style_of_the_same_name() {
    let (bytes, _) = written();
    let own = json!([{"id": "proje-cetvel", "name": "cetvel", "data": {"align": "left"}}]);
    let mut doc = project(own.clone());
    import(&mut doc, &bytes);
    let names: Vec<(String, String)> =
        doc.settings().table_styles.iter().map(|s| (s.id.clone(), s.name.clone())).collect();
    assert_eq!(
        names,
        [("proje-cetvel".to_owned(), "cetvel".to_owned()), ("yerel-sade".to_owned(), "SADE".to_owned())],
        "the project's CETVEL stays as it is; SADE comes in with its id"
    );
    assert_eq!(doc.settings().table_styles[0], serde_json::from_value::<TableStyleDef>(own[0].clone()).unwrap());
    assert_eq!(tables(&doc), [Some("proje-cetvel".to_owned())]);
}

#[test]
fn another_programs_table_style_becomes_a_project_style_with_a_new_id() {
    let mut doc = project(json!([]));
    import(&mut doc, FOREIGN);
    let styles = &doc.settings().table_styles;
    assert_eq!(styles.len(), 1, "BOS, followed by nothing, stays out");
    assert_eq!(styles[0].name, "CETVEL");
    assert!(!styles[0].id.starts_with("dxf-table-"), "a project id, not the reader's number: {}", styles[0].id);
    assert!(doc.settings().text_styles.iter().any(|t| t.name == "A-TABLO"), "the cell style's text style with it");
    assert!(tables(&doc).is_empty(), "the ACAD_TABLE is its block's lines and words, not a KentOS table");
}
