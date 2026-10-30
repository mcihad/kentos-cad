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
        annotation: None,
        cad_work: None,
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
    // Both fixture viewports are intentionally unlocked; native preflight now names them per page.
    let mut actual=made.findings.iter().map(|f|(f.code.as_str(),f.item.as_deref())).collect::<Vec<_>>();actual.sort();
    assert_eq!(actual,[("no_kerning",None),("viewport_scale_unlocked",Some("kat-plani-harita")),("viewport_scale_unlocked",Some("vaziyet-harita"))]);
    assert!(made.findings.iter().all(|f|f.severity==if f.code=="no_kerning"{kentos_sheet::preflight::Severity::Info}else{kentos_sheet::preflight::Severity::Warning}));
    let mut owners=made.sheet_findings.iter().map(|(id,f)|(id.as_str(),f.code.as_str())).collect::<Vec<_>>();owners.sort();
    assert_eq!(owners,[("kat-plani","no_kerning"),("kat-plani","viewport_scale_unlocked"),("vaziyet","no_kerning"),("vaziyet","viewport_scale_unlocked")]);
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

/// A gradient and a dense hatch on a sheet go to the PDF as vectors (docs/adr/0333 §1): the map is not left blank as a
/// map with no vector form was.
#[test]
fn a_gradient_and_a_dense_hatch_go_to_the_pdf_as_vectors() {
    let scenario: Value = serde_json::from_str(SCENARIO).expect("the scenario");
    let mut doc = drawing(&scenario);
    let k11 = &step(&scenario, |s| s["as"] == "k11")["input"];
    let (ax, ay) = (k11["a"]["x"].as_f64().expect("x"), k11["a"]["y"].as_f64().expect("y"));
    let ring = |dx: f64| {
        json!([{ "x": ax + dx, "y": ay + 0.5 }, { "x": ax + dx + 1.5, "y": ay + 0.5 },
               { "x": ax + dx + 1.5, "y": ay + 2.0 }, { "x": ax + dx, "y": ay + 2.0 }])
    };
    let layer = doc.layers().active().to_owned();
    let plain = book_pdf(&doc, &[], "kabul", "2026-10-17").expect("a PDF").bytes.len();
    for (k, pattern) in [
        json!({ "type": "gradient", "angle": 30, "spacing": 1,
                "gradient": { "shape": "spherical", "inverted": false, "color2": "#FFFFFF" } }),
        json!({ "type": "lines", "angle": 45, "spacing": 0.01 }),
    ]
    .into_iter()
    .enumerate()
    {
        let hatch: Entity = serde_json::from_value(json!({
            "kind": "hatch", "id": 0, "layerId": layer, "attrs": {}, "ring": ring(2.0 * k as f64), "pattern": pattern
        }))
        .expect("a hatch");
        doc.add(hatch).expect("added");
    }
    let made = book_pdf(&doc, &[], "kabul", "2026-10-17").expect("a PDF");
    assert!(
        !made.notes.iter().any(|n| n.contains("vektörle yazılamadı")),
        "{:?}",
        made.notes
    );
    // KENTOS_PLOT_OUT=dir writes the PDF there to look at (docs/adr/0333 Doğrulama).
    if let Some(dir) = std::env::var_os("KENTOS_PLOT_OUT") {
        std::fs::write(std::path::Path::new(&dir).join("tarama-pdf.pdf"), &made.bytes).expect("written");
    }
    // The bands and the lines are in it: the map grew by them (it would not if they were outside its frame).
    assert!(made.bytes.len() > plain + 20_000, "{} against {plain}", made.bytes.len());
}
