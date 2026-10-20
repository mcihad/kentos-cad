//! Birimi değiştir over what the shared cases cannot set up (docs/adr/0256
//! §1): a drawing with a sheet book (the book stays, a warning says so), a
//! dimension bound to its wall (it follows the scaled wall in the same step)
//! and the journal's one row for the step.

use kentos_commands::{
    CommandResult, DocumentSnapshotV1, DrawingUnit, Entity, ProjectSheets, UnitGeometry,
    UnitsChange, UnitsChanged,
};
use kentos_domain::{Document, Slot};
use kentos_native_application::{ExecutionContext, units_change};
use serde_json::json;

fn drawing() -> Document {
    let snapshot = json!({"format":"kentos.document","version":1,"name":"Oda","settings":{"srid":0,
        "lengthDecimals":0,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":50,"workspace":"cad",
        "drawingUnit":"mm"},
        "origin":{"x":0,"y":0},"layers":[{"id":"a","name":"A","type":"layer","visible":true,"locked":false,
        "expanded":true,"style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],
        "activeLayer":"a","entities":[
        {"kind":"polygon","id":1,"layerId":"a","attrs":{},"pts":[{"x":0,"y":0},{"x":4,"y":0},{"x":4,"y":3},{"x":0,"y":3}]}],
        "styles":{"items":[],"categories":[]}});
    Document::from_snapshot(DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads"))
        .expect("opens")
}

fn change(doc: &mut Document, input: UnitsChange) -> CommandResult<UnitsChanged> {
    units_change::execute(&mut ExecutionContext::new(doc), input)
}

fn scale(from: DrawingUnit, to: DrawingUnit) -> UnitsChange {
    UnitsChange {
        from,
        to,
        geometry: UnitGeometry::Scale,
        expected_revision: None,
    }
}

#[test]
fn a_sheet_book_stays_and_the_warning_says_so() {
    let mut doc = drawing();
    doc.set_sheets(Some(ProjectSheets {
        book: json!({ "schema": "kentos.sheet/1", "sheets": [{ "id": "pafta-1", "name": "Pafta 1" }] }),
        pictures: Vec::new(),
    }))
    .expect("kept");
    let book = doc.sheets().cloned();
    match change(&mut doc, scale(DrawingUnit::Mm, DrawingUnit::M)) {
        CommandResult::Completed { output, warnings } => {
            assert_eq!(
                (output.factor, output.objects, output.blocks),
                (1000.0, 1, 0)
            );
            let codes: Vec<&str> = warnings.iter().map(|w| w.code.as_str()).collect();
            assert_eq!(codes, ["sheets_kept"]);
        }
        other => panic!("not completed: {other:?}"),
    }
    assert_eq!(doc.sheets().cloned(), book, "the book is the paper's");
}

#[test]
fn the_unit_and_the_scaled_drawing_are_one_step_and_one_journal_row() {
    let mut doc = drawing();
    let rows = doc.journal().count();
    let before = doc.get(Slot(1)).cloned();
    match change(&mut doc, scale(DrawingUnit::Mm, DrawingUnit::Cm)) {
        CommandResult::Completed { output, .. } => assert_eq!(output.factor, 10.0),
        other => panic!("not completed: {other:?}"),
    }
    let Some(Entity::Polygon(room)) = doc.get(Slot(1)) else {
        panic!("a polygon");
    };
    assert_eq!(room.pts[2].x, 40.0);
    assert_eq!(doc.settings().drawing_unit, Some(DrawingUnit::Cm));
    assert_eq!(doc.journal().count(), rows + 1, "one row for the step");
    let row = doc.journal().last().expect("a row");
    assert_eq!(row.command.as_deref(), Some("cad.units.change"));
    assert_eq!(doc.undo().as_deref(), Some(units_change::STEP));
    assert_eq!(doc.get(Slot(1)).cloned(), before);
    assert_eq!(doc.settings().drawing_unit, Some(DrawingUnit::Mm));
    assert!(!doc.can_undo());
    assert_eq!(doc.redo().as_deref(), Some(units_change::STEP));
    assert_eq!(doc.settings().drawing_unit, Some(DrawingUnit::Cm));
}

#[test]
fn keeping_the_sizes_is_an_undo_step_too() {
    let mut doc = drawing();
    let input = UnitsChange {
        from: DrawingUnit::Mm,
        to: DrawingUnit::M,
        geometry: UnitGeometry::Keep,
        expected_revision: None,
    };
    assert!(matches!(
        change(&mut doc, input),
        CommandResult::Completed { .. }
    ));
    assert_eq!(doc.settings().drawing_unit, None, "metres are no unit");
    assert_eq!(doc.undo().as_deref(), Some(units_change::STEP));
    assert_eq!(doc.settings().drawing_unit, Some(DrawingUnit::Mm));
}
