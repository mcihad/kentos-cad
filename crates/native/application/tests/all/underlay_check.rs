//! Altlık modeli denetle (docs/adr/8102; plan/bim-plan.md BIM-17): the declared section building on TUREF / TM33
//! (fixtures/arch/v1/ifc/kesit-binasi-tam.ifc is its model) against its structural model written by IfcOpenShell
//! (scripts/fixtures/structural_underlay.py): aligned, every column, beam and slab has its match and nothing clashes;
//! with the ground floor's column on A-1 moved 40 cm east and "1. Kat" said at 3.15 m, both are findings; on another
//! height datum, nothing is looked at.

use std::path::PathBuf;

use kentos_commands::{CommandResult, UnderlayChange, UnderlayCheck, UnderlayEdit, UnderlayPlacement};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, underlay_check, underlay_edit};

fn fixture(rel: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures")
        .join(rel)
        .canonicalize()
        .expect("the fixture")
        .display()
        .to_string()
}

/// The declared section building, whose model is fixtures/arch/v1/ifc/kesit-binasi-tam.ifc (crate::ifc_required).
fn building() -> Document {
    crate::ifc_required::declared()
}

/// The file added as an underlay placed by its map conversion; its id, or the refusal's code and words.
fn added(doc: &mut Document, file: &str) -> Result<String, (String, String)> {
    let change = UnderlayChange::Add {
        id: None,
        file: fixture(file),
        name: None,
        discipline: Some("Statik".into()),
        revision: None,
        placement: Some(UnderlayPlacement::Map),
    };
    match underlay_edit::execute(&mut ExecutionContext::new(doc), UnderlayEdit { change, expected_revision: None }) {
        CommandResult::Completed { output, .. } => Ok(output.id),
        CommandResult::Failed { error } => Err((error.code, error.message)),
        other => panic!("{other:?}"),
    }
}

fn checked(doc: &mut Document, id: &str) -> kentos_commands::UnderlayChecked {
    match underlay_check::execute(&mut ExecutionContext::new(doc), UnderlayCheck { id: id.into(), tolerance: None }) {
        CommandResult::Completed { output, .. } => output,
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_aligned_structural_model_matches_every_element_and_clashes_with_nothing() {
    let mut doc = building();
    let id = added(&mut doc, "ifc/v1/statik/kesit-binasi-statik.ifc").expect("placed");
    let c = checked(&mut doc, &id);
    let levels: Vec<(Option<&str>, Option<&str>, Option<f64>)> =
        c.levels.iter().map(|l| (l.level.as_deref(), l.storey.as_deref(), l.difference)).collect();
    assert_eq!(levels.len(), 2, "{levels:?}");
    assert!(c.levels.iter().all(|l| l.difference.is_some_and(|d| d.abs() < 1e-6)), "{levels:?}");
    let labels: Vec<&str> = c.axes.iter().map(|a| a.label.as_str()).collect();
    assert_eq!(labels, ["1", "2", "3", "A", "B", "C", "D"]);
    assert!(c.axes.iter().all(|a| a.in_project && a.in_file && a.offset.is_some_and(|o| o < 1e-6)), "{:?}", c.axes);
    assert_eq!(c.matched, 27, "16 columns, 8 beams, 3 slabs");
    assert!(c.clashes.is_empty(), "{:?}", c.clashes);
    assert!(c.unmatched.is_empty(), "{:?}", c.unmatched);
}

#[test]
fn a_moved_column_and_a_storey_said_higher_are_findings() {
    let mut doc = building();
    let id = added(&mut doc, "ifc/v1/statik/kesit-binasi-statik-kaymis.ifc").expect("placed");
    let c = checked(&mut doc, &id);
    let off: Vec<(Option<&str>, Option<&str>, f64)> = c
        .levels
        .iter()
        .filter_map(|l| l.difference.filter(|d| d.abs() > 1e-6).map(|d| (l.level.as_deref(), l.storey.as_deref(), d)))
        .collect();
    assert_eq!(off.len(), 1, "{:?}", c.levels);
    assert_eq!((off[0].0, off[0].1), (Some("1. kat"), Some("1. Kat")));
    assert!((off[0].2 - 0.15).abs() < 1e-9, "{off:?}");
    assert_eq!(c.matched, 26);
    let note: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(fixture("ifc/v1/statik/kesit-binasi-statik.json")).unwrap()).unwrap();
    let moved = note["moved"]["globalId"].as_str().expect("the moved column");
    assert!(!c.clashes.is_empty(), "the moved column clashes");
    assert!(c.clashes.iter().all(|x| x.object == moved), "{:?}", c.clashes);
    assert!(c.clashes.iter().any(|x| x.class == "IfcWall"), "{:?}", c.clashes);
    // The drawing's column on A-1 has no match in the file now.
    assert_eq!(c.unmatched.len(), 1, "{:?}", c.unmatched);
    assert_eq!(c.unmatched[0].class, "IfcColumn");
    assert!(c.unmatched[0].uid.is_some());
}

#[test]
fn another_height_datum_stops_before_any_geometry() {
    let mut doc = building();
    let (code, why) = added(&mut doc, "ifc/v1/statik/kesit-binasi-statik-datum.ifc").expect_err("not placed");
    assert_eq!(code, "model_underlay_not_placed");
    assert!(why.contains("ITRF96") && why.contains("Antalya"), "{why}");
    assert!(doc.underlays().is_empty());
}
