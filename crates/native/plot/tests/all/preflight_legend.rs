//! The full print check reads a legend as the output prints it (UR2-023): a legend over a map whose layers are shown is
//! not said empty, though the caller gave no legend rows.

use crate::atlas_drawing;
use serde_json::json;

#[test]
fn a_legend_over_shown_layers_is_not_said_empty() {
    let doc = atlas_drawing::document();
    let mut book = atlas_drawing::sheet_book();
    let sheet = &mut book.sheets[0];
    let map = sheet
        .items
        .iter()
        .find(|i| matches!(i.kind, kentos_sheet::kinds::ItemKind::Map(_)))
        .map(|i| i.id.clone())
        .expect("the template's map");
    let mut value = serde_json::to_value(&*sheet).expect("json");
    value["items"]
        .as_array_mut()
        .expect("items")
        .retain(|i| i["kind"]["type"] != "legend");
    value["items"].as_array_mut().expect("items").push(json!({
        "id": "lejant", "name": "Lejant", "frame": {"left": 20000, "top": 200000, "width": 80000, "height": 40000},
        "kind": {"type": "legend", "map": map, "title": "Lejant"}
    }));
    *sheet = serde_json::from_value(value).expect("a sheet");
    let render = kentos_plot::book::base_inputs(&doc, "Test", "20.11.2026");
    let found = kentos_plot::preflight::inspect(&doc, &book, "atlas-paftasi", &render, None)
        .expect("checked");
    let codes: Vec<&str> = found.iter().map(|f| f.code.as_str()).collect();
    assert!(!codes.contains(&"legend_empty"), "{codes:?}");
}
