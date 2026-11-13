//! ADR 8053 §14: a block's text prints in its own colour, else the layer it follows, else the insert's; before, every
//! block text printed in the insert's colour (a definition's green words came out in the insert's red).
use kentos_domain::Document;
use kentos_scene_query::Spatial;
use kentos_sheet::{
    SheetBook,
    display::{self, Prim},
    project::project_sheets,
};
use serde_json::{Value, json};

const BLOCK: &str = "0192f5a1-2900-7000-8000-0000000000d1";

fn drawn() -> (Document, SheetBook) {
    let fx: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/native-preflight.json"
    ))
    .unwrap();
    let (mut raw, mut page) = (fx["model"].clone(), fx["book"].clone());
    let layer = raw["entities"][0]["layerId"].clone();
    let text = |id: u32, y: f64, more: Value| {
        let mut t = json!({"kind":"text","id":id,"layerId":"","attrs":{},"p":{"x":0.0,"y":y},
            "text":format!("P{id}"),"height":0.25,"rotation":0.0});
        t.as_object_mut()
            .unwrap()
            .extend(more.as_object().unwrap().clone());
        t
    };
    raw["blocks"] = json!([{"id":BLOCK,"name":"B","base":{"x":0.0,"y":0.0},"units":"m","entities":[
        text(1, 0.0, json!({"color":"#00FF00"})),
        text(2, 0.5, json!({"byLayer":["color"]})),
        text(3, 1.0, json!({})),
    ]}]);
    raw["entities"] = json!([{"kind":"insert","id":1,"layerId":layer,"attrs":{},"color":"#FF0000","block":BLOCK,
        "p":{"x":-1.0,"y":0.0},"scale":1.0,"rotation":0.0}]);
    page["sheets"][0]["items"][0]["kind"]["view"]["center"] = json!({"x":0.,"y":0.5});
    page["sheets"][0]["items"][0]["kind"]["view"]["scale"] = json!(50);
    let mut doc = Document::from_snapshot(serde_json::from_value(raw).unwrap()).unwrap();
    let book: SheetBook = serde_json::from_value(page).unwrap();
    doc.set_sheets(project_sheets(&book, |_| None).unwrap())
        .unwrap();
    (doc, book)
}

#[test]
fn a_blocks_text_prints_in_its_own_colour() {
    let (doc, book) = drawn();
    let render = kentos_plot::book::base_inputs(&doc, "Test", "2026-11-12");
    let list = display::display_list(&book, "ground", &render).unwrap();
    let map = list
        .prims
        .iter()
        .find_map(|p| if let Prim::Map(m) = p { Some(m) } else { None })
        .unwrap();
    let mut spatial = Spatial::new();
    spatial.reload(&doc);
    let mut colours: Vec<(String, String)> = kentos_plot::labels::map_labels(&doc, &spatial, map)
        .into_iter()
        .filter(|l| !l.text.is_empty())
        .map(|l| (l.text, l.color.to_lowercase()))
        .collect();
    colours.sort();
    assert_eq!(
        colours,
        [
            // Its own green.
            ("P1".to_owned(), "#00ff00".to_owned()),
            // Its layer's (the block's layer 0: the insert's, whose theme ink is the paper's ink).
            ("P2".to_owned(), "#111111".to_owned()),
            // BYBLOCK: the insert's red.
            ("P3".to_owned(), "#ff0000".to_owned()),
        ]
    );
}
