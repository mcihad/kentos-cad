//! A sheet that shows a drawing not up to date is not delivered (docs/adr/1720; plan/map-cad.md MAP-20): the map
//! project of `road.terrains.zeroline`'s cases, the zero line of Tümsek over Düz drawn and a sheet with a fixed map
//! over it; one of Tümsek's heights set: the print check's `outdated_drawing` is an error on that map; drawn again
//! (`road.project.rebuild`), the sheet is clean. A map elsewhere on the ground is not concerned.

use kentos_commands::{CallOp, DocumentSnapshotV2};
use kentos_domain::Document;
use kentos_native_application::ExecutionContext;
use kentos_plot::book::base_inputs;
use kentos_sheet::model::SheetBook;
use serde_json::{Value, json};

fn run(doc: &mut Document, command: &str, input: Value) {
    let mut cx = ExecutionContext::new(doc);
    let out = kentos_native_application::registry::run(&mut cx, command, CallOp::Execute, input).expect("runs");
    assert_eq!(out["status"], "completed", "{command}: {out}");
}

fn map(id: &str, x: f64, y: f64) -> Value {
    json!({"id": id, "name": id, "frame": {"left": 20000, "top": 20000, "width": 200000, "height": 150000},
           "kind": {"type": "map", "view": {"type": "fixed", "center": {"x": x, "y": y}, "scale": 100, "rotation": 0}}})
}

fn drawing() -> Document {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../../fixtures/commands/v1/road.terrains.zeroline.json")).expect("cases");
    let case = cases["cases"].as_array().expect("cases").iter().find(|c| c["setup"]["settings"]["cadWork"] == "map").expect("a map case");
    let snapshot: DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).expect("reads");
    let mut doc = Document::from_snapshot_v2(snapshot).expect("opens");
    run(&mut doc, "road.terrains.zeroline", json!({"base": "Düz", "comparison": "Tümsek", "layerId": "sifir"}));
    let ops: Vec<kentos_sheet::ops::Op> = serde_json::from_value(json!([{"op": "addSheet", "sheet": {
        "id": "pafta", "name": "Pafta",
        "page": {"paper": "a3", "orientation": "landscape", "size": {"width": 420000, "height": 297000},
                 "margins": {"top": 10000, "right": 10000, "bottom": 10000, "left": 20000}},
        "items": [map("tumsek", 6005.0, 5.0), map("uzak", 2500.0, 500.0)]}}]))
    .expect("ops");
    let book = kentos_sheet::ops::apply_all(&SheetBook::default(), &ops).expect("the sheet").book;
    doc.set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).expect("kept")).expect("the book");
    doc
}

fn outdated(doc: &Document) -> Vec<(String, bool)> {
    let (book, _) = kentos_sheet::project::read_project_sheets(doc.sheets().expect("sheets")).expect("the book");
    let inputs = base_inputs(doc, "Ayşe", "2026-11-07");
    kentos_plot::preflight::inspect(doc, &book, "pafta", &inputs, None)
        .expect("checked")
        .into_iter()
        .filter(|f| f.code == "outdated_drawing")
        .map(|f| (f.item.unwrap_or_default(), f.severity == kentos_sheet::preflight::Severity::Error))
        .collect()
}

#[test]
fn a_stale_zero_line_on_a_sheet_stops_its_delivery_until_drawn_again() {
    let mut doc = drawing();
    assert!(outdated(&doc).is_empty(), "drawn now: up to date");
    let surface = json!({"id": "t-10", "name": "Tümsek", "points": {"layers": ["tumsek"]},
                         "edits": [{"op": "setHeight", "x": 6000.0, "y": 0.0, "z": 119.0}]});
    run(&mut doc, "road.terrains.edit", json!({"operation": "set", "surface": surface}));
    assert_eq!(outdated(&doc), [("tumsek".to_owned(), true)], "the map over it, an error; the map elsewhere, nothing");
    // The book's PDF is refused as a stale table field's is.
    let refused = kentos_plot::book::book_pdf(&doc, &[], "Ayşe", "2026-11-07").expect_err("not delivered");
    assert_eq!(refused.code, "outdated_drawing", "{}", refused.message);
    run(&mut doc, "road.project.rebuild", json!({}));
    assert!(outdated(&doc).is_empty(), "drawn again: delivered");
    assert!(kentos_plot::book::book_pdf(&doc, &[], "Ayşe", "2026-11-07").is_ok());
}
