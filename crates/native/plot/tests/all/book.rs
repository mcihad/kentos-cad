//! A book PDF without a window (docs/adr/0252 §3): the acceptance drawing's
//! sheets (fixtures/acceptance/v1/room.json) over a drawing that holds the
//! scenario's 5000 mm line, read back without KentOS code: A3 pages, the line
//! 100 mm long at 1:50 and 50 mm at 1:100, where its map puts it.

use kentos_contracts::{DocumentSnapshotV1, DrawingFont, DrawingUnit, Entity, Workspace};
use kentos_domain::Document;
use kentos_plot::book::book_pdf;
use kentos_project::new_project::{NewProject, new_project};
use kentos_sheet::model::SheetBook;
use serde_json::{Value, json};

use crate::files::pdf_problems;

const SCENARIO: &str = include_str!("../../../../../fixtures/acceptance/v1/room.json");

fn step(scenario: &Value, what: impl Fn(&Value) -> bool) -> &Value {
    scenario["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .find(|s| what(s))
        .expect("the step")
}

/// A local mm project with the K11 line, and the scenario's two sheets in its book.
fn drawing(scenario: &Value) -> Document {
    let mut snapshot: DocumentSnapshotV1 = new_project(&NewProject {
        name: "Oda".into(),
        srid: 0,
        plot_scale: 50.0,
        workspace: Workspace::Cad,
        drawing_font: DrawingFont::Barlow,
        province: None,
        drawing_unit: Some(DrawingUnit::Mm),
    })
    .expect("a new project");
    let k11 = &step(scenario, |s| s["as"] == "k11")["input"];
    let line: Entity = serde_json::from_value(json!({
        "kind": "line", "id": 1, "layerId": snapshot.active_layer, "attrs": {},
        "a": k11["a"], "b": k11["b"]
    }))
    .expect("a line");
    snapshot.entities.push(line);
    let mut doc = Document::from_snapshot(snapshot).expect("opens");
    let ops: Vec<kentos_sheet::ops::Op> = serde_json::from_value(
        step(scenario, |s| s["command"] == "cad.sheets.edit")["input"]["ops"].clone(),
    )
    .expect("the sheet core's operations");
    let book = kentos_sheet::ops::apply_all(&SheetBook::default(), &ops)
        .expect("the sheets")
        .book;
    let kept = kentos_sheet::project::project_sheets(&book, |_| None).expect("kept");
    doc.set_sheets(kept).expect("the book");
    doc
}

#[test]
fn a_book_s_sheets_print_their_maps_at_scale() {
    let scenario: Value = serde_json::from_str(SCENARIO).expect("the scenario");
    let doc = drawing(&scenario);
    let made = book_pdf(&doc, &[], "kabul", "2026-10-13").expect("a PDF");
    assert_eq!(made.sheets, ["kat-plani", "vaziyet"]);
    assert!(made.findings.is_empty(), "{:?}", made.findings);
    let want = &step(&scenario, |s| s["export"] == "pdf")["expect"]["pdf"];
    let problems = pdf_problems(&made.bytes, want);
    assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn a_project_without_sheets_says_so() {
    let scenario: Value = serde_json::from_str(SCENARIO).expect("the scenario");
    let mut doc = drawing(&scenario);
    doc.set_sheets(None).expect("no book");
    let e = book_pdf(&doc, &[], "kabul", "2026-10-13").expect_err("no sheets");
    assert_eq!(e.code, "no_sheets");
}
