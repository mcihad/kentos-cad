//! ADR 1126 §3: a table that follows a table style prints the style's fill, colour and alignment where it sets none
//! itself, its own cell styles over the style's; a table without a style is as before.
use kentos_domain::Document;
use kentos_interaction::Spatial;
use kentos_sheet::{
    SheetBook,
    display::{self, Prim},
    project::project_sheets,
};
use serde_json::{Value, json};

fn drawn(table_style: Option<&str>, own: Value) -> (Document, SheetBook) {
    let fx: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/native-preflight.json"
    ))
    .unwrap();
    let (mut raw, mut page) = (fx["model"].clone(), fx["book"].clone());
    raw["settings"]["tableStyles"] = json!([{"id":"ofis","name":"OFIS",
        "title":{"fill":"#eeeeee","align":"center"},"header":{"color":"#c0392b","fill":"#dddddd"},
        "data":{"align":"right","color":"#2244dd"}}]);
    let mut t = json!({"kind":"table","id":1,"attrs":{},"p":{"x":-2.0,"y":1.5},"rotation":0.0,"height":0.25,
        "rows":[0.6,0.5,0.5],"columns":[2.0,2.0],"cells":[["Liste",""],["Ada","Alan"],["101","512"]],
        "merges":[{"row":0,"col":0,"rows":1,"cols":2}],"header":true,"styles":own});
    if let Some(s) = table_style {
        t["tableStyle"] = json!(s);
    }
    t["layerId"] = raw["entities"][0]["layerId"].clone();
    raw["entities"] = json!([t]);
    page["sheets"][0]["items"][0]["kind"]["view"]["center"] = json!({"x":0.,"y":0.});
    page["sheets"][0]["items"][0]["kind"]["view"]["scale"] = json!(50);
    let mut doc = Document::from_snapshot(serde_json::from_value(raw).unwrap()).unwrap();
    let book: SheetBook = serde_json::from_value(page).unwrap();
    doc.set_sheets(project_sheets(&book, |_| None).unwrap()).unwrap();
    (doc, book)
}

fn labels(doc: &Document, book: &SheetBook) -> Vec<kentos_plot::maps::MapLabel> {
    let render = kentos_plot::book::base_inputs(doc, "Test", "2026-11-06");
    let list = display::display_list(book, "ground", &render).unwrap();
    let map = list.prims.iter().find_map(|p| if let Prim::Map(m) = p { Some(m) } else { None }).unwrap();
    let mut spatial = Spatial::new();
    spatial.reload(doc);
    kentos_plot::labels::map_labels(doc, &spatial, map)
}

#[test]
fn a_table_prints_its_styles_fills_and_colours_under_its_own() {
    let (doc, book) = drawn(Some("ofis"), json!([]));
    let found = labels(&doc, &book);
    let fills: Vec<&str> = found.iter().filter(|l| l.fill.is_some()).map(|l| l.color.as_str()).collect();
    assert!(fills.contains(&"#eeeeee") && fills.contains(&"#dddddd"), "{fills:?}");
    let colour = |words: &str| found.iter().find(|l| l.text == words).map(|l| l.color.clone());
    assert_eq!(colour("Ada").as_deref(), Some("#c0392b"), "the heading row's colour");
    assert_eq!(colour("512").as_deref(), Some("#2244dd"), "a data cell's colour");
    // Its own cell style stays over the style's.
    let (doc, book) = drawn(Some("ofis"), json!([{"row":2,"col":1,"rows":1,"cols":1,"color":"#00aa00"}]));
    let found = labels(&doc, &book);
    assert_eq!(found.iter().find(|l| l.text == "512").map(|l| l.color.as_str()), Some("#00aa00"));
    assert_eq!(found.iter().find(|l| l.text == "101").map(|l| l.color.as_str()), Some("#2244dd"));
    // No style: no fill, the layer's colour.
    let (doc, book) = drawn(None, json!([]));
    let found = labels(&doc, &book);
    assert!(found.iter().all(|l| l.fill.is_none()));
    assert_ne!(found.iter().find(|l| l.text == "512").map(|l| l.color.as_str()), Some("#2244dd"));
    kentos_plot::book::book_pdf(&doc, &[], "Test", "2026-11-06").unwrap();
}
