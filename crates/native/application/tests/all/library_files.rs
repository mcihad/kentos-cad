//! The texts `cad.library.edit` and `cad.blocks.edit` give out (docs/adr/0314 §3), which the shared cases take as
//! `$any`: canonical JSON without a time, read back by the style library's file reader and by the blocks' import
//! into a drawing without them, the same definitions.

use kentos_domain::Document;
use kentos_commands::{CallOp, DocumentSnapshotV1};
use kentos_native_application::{ExecutionContext, registry};
use kentos_style_library::file::parse_style_file;
use serde_json::{Value, json};

fn setup(command: &str) -> Value {
    let path = format!(
        "{}/../../../fixtures/commands/v1/{command}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("the cases")).expect("JSON");
    file["setup"].clone()
}

fn open(snapshot: &Value) -> Document {
    Document::from_snapshot(DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads"))
        .expect("opens")
}

fn run(doc: &mut Document, command: &str, input: Value) -> Value {
    let answer = registry::run(
        &mut ExecutionContext::new(doc),
        command,
        CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    answer["output"].clone()
}

#[test]
fn a_library_export_is_canonical_and_reads_back_with_what_it_draws() {
    let mut doc = open(&setup("cad.library.edit"));
    let out = run(
        &mut doc,
        "cad.library.edit",
        json!({ "operation": "export", "ids": ["p-agac"] }),
    );
    let text = out["kstil"].as_str().expect("the text");
    let value: Value = serde_json::from_str(text).expect("JSON");
    assert_eq!(
        text,
        kentos_commands::canonical::canonical_json(&value)
    );
    assert!(value.get("exported").is_none(), "no time in it");
    let file = parse_style_file(text).expect("the reader takes it");
    let ids: Vec<&str> = file.items.iter().filter_map(|i| i["id"].as_str()).collect();
    assert_eq!(ids, ["p-agac", "p-cizim"]);
    // The same library gives the same text.
    let again = run(
        &mut doc,
        "cad.library.edit",
        json!({ "operation": "export", "ids": ["p-agac"] }),
    );
    assert_eq!(again["kstil"], out["kstil"]);
}

#[test]
fn a_blocks_export_reads_back_into_a_drawing_without_them() {
    let snapshot = setup("cad.blocks.edit");
    let mut doc = open(&snapshot);
    let direk = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d2003";
    let out = run(
        &mut doc,
        "cad.blocks.edit",
        json!({ "operation": "export", "blocks": [direk] }),
    );
    let text = out["library"].as_str().expect("the text");
    let value: Value = serde_json::from_str(text).expect("JSON");
    assert_eq!(
        text,
        kentos_commands::canonical::canonical_json(&value)
    );
    let names: Vec<&str> = value["blocks"]
        .as_array()
        .expect("blocks")
        .iter()
        .filter_map(|b| b["name"].as_str())
        .collect();
    // What it inserts first.
    assert_eq!(names, ["Lamba", "Aydınlatma direği"]);
    let mut empty = snapshot.clone();
    empty["blocks"] = json!([]);
    empty["entities"] = json!([]);
    let mut other = open(&empty);
    let taken = run(
        &mut other,
        "cad.blocks.edit",
        json!({ "operation": "import", "library": text }),
    );
    assert_eq!(taken["added"].as_array().map(Vec::len), Some(2));
    for name in ["Lamba", "Aydınlatma direği"] {
        let mine = doc.block_named(name).expect("in the drawing");
        let theirs = other.block_named(name).expect("taken in");
        assert_eq!(mine, theirs, "{name}: the same definition, its id too");
    }
}

/// `cargo test --release -p kentos-native-application --test all library_files::timing -- --ignored --nocapture`:
/// the reach of a layer's look and of a line type over 100 000 objects on 50 layers, 200 blocks of five pieces
/// each, 1 000 inserts (docs/adr/0314 Doğrulama).
#[test]
#[ignore = "a timing, run on purpose"]
fn timing() {
    use std::time::Instant;
    let layers: Vec<Value> = (0..50)
        .map(|k| {
            let mut style = json!({ "color": "fg", "lineType": "continuous", "lineWeight": 0.25 });
            if k % 5 == 0 {
                style["lineType"] = json!("dashed");
                style["linePattern"] = json!("EKSEN");
            }
            json!({ "id": format!("l{k}"), "name": format!("Katman {k}"), "type": "layer", "visible": true,
                    "locked": false, "expanded": true, "style": style, "children": [] })
        })
        .collect();
    let block_id = |b: u32| format!("0192f5a0-7c3e-7d4a-9b1e-{b:012x}");
    let blocks: Vec<Value> = (0..200u32)
        .map(|b| {
            let pieces: Vec<Value> = (0..5u32)
                .map(|p| json!({ "kind": "line", "id": p + 1, "layerId": format!("l{}", (b + p) % 50), "attrs": {},
                                 "byLayer": ["color", "lineType"], "a": { "x": 0, "y": 0 }, "b": { "x": 1, "y": f64::from(p) } }))
                .collect();
            json!({ "id": block_id(b), "name": format!("Blok {b}"), "base": { "x": 0, "y": 0 }, "entities": pieces })
        })
        .collect();
    let mut entities: Vec<Value> = (0..99_000u32)
        .map(|i| {
            let (x, y) = (f64::from(i % 300) * 3.0, f64::from(i / 300) * 3.0);
            let mut e = json!({ "kind": "line", "id": i + 1, "layerId": format!("l{}", i % 50), "attrs": {},
                                "a": { "x": x, "y": y }, "b": { "x": x + 2.0, "y": y } });
            if i % 7 == 0 {
                e["color"] = json!("#FF0000");
            }
            e
        })
        .collect();
    entities.extend((0..1000u32).map(|i| {
        json!({ "kind": "insert", "id": 99_001 + i, "layerId": "l1", "attrs": {}, "block": block_id(i % 200),
                "p": { "x": f64::from(i), "y": -10.0 }, "scale": 1, "rotation": 0 })
    }));
    let snapshot = json!({ "format": "kentos.document", "version": 1, "name": "Etki",
        "settings": { "srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 100,
                      "lineTypes": [{ "name": "EKSEN", "elements": [{ "length": 12.0 }, { "length": -3.0 }], "unit": "drawing" }] },
        "origin": { "x": 0, "y": 0 }, "layers": layers, "activeLayer": "l1", "entities": entities,
        "styles": { "items": [], "categories": [] }, "blocks": blocks });
    let mut doc = open(&snapshot);
    for (what, input) in [
        (
            "Katmanın görünüşü (2 000 nesne)",
            json!({ "target": "layer", "id": "l3" }),
        ),
        (
            "Çizgi tipi (10 katman, 20 000 nesne)",
            json!({ "target": "lineType", "id": "EKSEN" }),
        ),
    ] {
        run(&mut doc, "cad.styles.impact", input.clone());
        let t = Instant::now();
        let out = run(&mut doc, "cad.styles.impact", input);
        println!(
            "{what}: {} izleyen, {} blok parçası, {} yerleştirme, {:.1} ms",
            out["follow"],
            out["blockPieces"],
            out["inserts"],
            t.elapsed().as_secs_f64() * 1e3
        );
    }
}
