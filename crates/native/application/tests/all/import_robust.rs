//! AutoCAD drawings that were refused whole (the owner's files, 23 October): a block placing another defined after it
//! in the BLOCKS section, and a layout's viewport under a millimetre. The drawing comes in; what the sheet book cannot
//! hold is said.

use kentos_commands::CallOp;
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

fn drawing() -> Document {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/file.import.dwg.json"
    ))
    .expect("the cases read");
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(&fixture["setup"].to_string())
        .expect("the drawing reads");
    Document::from_snapshot(snapshot).expect("opens")
}

fn import(name: &str) -> Value {
    let path = format!(
        "{}/../../../fixtures/formats/v1/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut doc = drawing();
    registry::run(
        &mut ExecutionContext::new(&mut doc),
        "file.import.dxf",
        CallOp::Execute,
        json!({ "path": path }),
    )
    .expect("the command runs")
}

#[test]
fn a_block_defined_after_the_one_placing_it_comes_in() {
    let ordered = import("blocks.dxf");
    let swapped = import("blocks-out-of-order.dxf");
    assert_eq!(ordered["status"], "completed", "{ordered}");
    assert_eq!(swapped["status"], "completed", "{swapped}");
    assert_eq!(swapped["output"]["blocks"], ordered["output"]["blocks"]);
    assert_eq!(swapped["output"]["objects"], ordered["output"]["objects"]);
}

#[test]
fn a_viewport_under_a_millimetre_is_left_out_and_said() {
    let answer = import("cad-io/layouts-tiny-viewport.dxf");
    assert_eq!(answer["status"], "completed", "{answer}");
    let notes = answer["output"]["report"]["notes"]
        .as_array()
        .expect("notes");
    assert!(
        notes
            .iter()
            .any(|n| n["what"] == "Görünüm penceresi (VIEWPORT)" && n["count"] == 1),
        "{notes:?}"
    );
}
