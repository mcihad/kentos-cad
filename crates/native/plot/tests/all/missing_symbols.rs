//! docs/adr/9317: a map's layer that draws with a symbol neither the project nor the system has (one of Kitaplığım the
//! project was not saved with) prints its plain look, and Ön denetim says which, once a symbol; a symbol the project
//! holds prints and is not said.
use kentos_domain::Document;
use kentos_plot::book;
use kentos_sheet::project::project_sheets;
use serde_json::{Value, json};

/// The fixture's map over a polygon of “Çizim” drawn with `u-deneme`, the project holding that symbol or not.
fn drawing(held: bool) -> Document {
    let fx: Value =
        serde_json::from_str(include_str!("../../../../../fixtures/sheet/v1/native-preflight.json")).unwrap();
    let (mut raw, page) = (fx["model"].clone(), fx["book"].clone());
    raw["layers"][0]["style"]["renderer"] = json!({ "type": "single", "symbols": { "fill": { "ref": "u-deneme" } } });
    raw["entities"] = json!([{ "kind": "polygon", "id": 1, "layerId": "cizim", "attrs": {},
        "pts": [{ "x": 0.0, "y": -1.0 }, { "x": 5.0, "y": -1.0 }, { "x": 5.0, "y": 1.0 }, { "x": 0.0, "y": 1.0 }] }]);
    if held {
        raw["styles"]["items"] = json!([{ "kind": "symbol", "id": "u-deneme", "name": "Deneme", "path": [],
            "symbol": { "type": "fill", "layers": [{ "id": "0", "type": "simpleFill", "color": "#FF00FF" }] } }]);
    }
    let mut doc = Document::from_snapshot(serde_json::from_value(raw).unwrap()).unwrap();
    let book: kentos_sheet::SheetBook = serde_json::from_value(page).unwrap();
    doc.set_sheets(project_sheets(&book, |_| None).unwrap()).unwrap();
    doc
}

#[test]
fn a_symbol_the_print_has_not_is_said_once_and_one_it_has_prints() {
    let missing = book::book_pdf(&drawing(false), &[], "Test", "2026-11-26").unwrap();
    let said: Vec<_> = missing.findings.iter().filter(|f| f.code == "symbol_missing").collect();
    assert_eq!(said.len(), 1, "{:?}", missing.findings);
    assert!(
        said[0].message.contains("“Çizim” katmanının çizdiği “u-deneme” sembolü"),
        "{}",
        said[0].message
    );
    let held = book::book_pdf(&drawing(true), &[], "Test", "2026-11-26").unwrap();
    assert!(held.findings.iter().all(|f| f.code != "symbol_missing"), "{:?}", held.findings);
    // The symbol the project holds is on the page: its fill.
    let page = kentos_plot::preview::page(&drawing(true), "ground", "Test", "2026-11-26", None, 72).unwrap();
    let magenta = page.rgba.chunks(4).filter(|p| p[0] > 200 && p[1] < 80 && p[2] > 200).count();
    assert!(magenta > 500, "{magenta} pixels of its fill");
    let plain = kentos_plot::preview::page(&drawing(false), "ground", "Test", "2026-11-26", None, 72).unwrap();
    assert_eq!(plain.rgba.chunks(4).filter(|p| p[0] > 200 && p[1] < 80 && p[2] > 200).count(), 0, "the plain look");
}
