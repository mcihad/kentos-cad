//! Nokta editörü's batch operations against
//! `fixtures/point-editor/v1/batch.json` (scripts/fixtures/point_batch_cases.py,
//! worked out from the rules with no KentOS code): every case's drawing after
//! the operation, the messages said and the undo step, and the rows the
//! operations take; as the web's `ui/bottom/pointBatch.test.ts`.

use kentos_domain::Slot;
use serde_json::{Value, json};

use super::batch::{Op, run, targets};

fn file() -> Value {
    serde_json::from_str(include_str!(
        "../../../../fixtures/point-editor/v1/batch.json"
    ))
    .expect("batch.json reads")
}

/// The reference's drawing: its layers (one hidden, one locked) and objects.
fn drawing(file: &Value) -> kentos_domain::Document {
    let layers: Vec<Value> = file["layers"]
        .as_array()
        .expect("layers")
        .iter()
        .map(|l| {
            json!({ "id": l["id"], "name": l["name"], "type": "layer", "visible": l["visible"], "locked": l["locked"],
                "expanded": true, "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] })
        })
        .collect();
    let snapshot = json!({
        "format": "kentos.document", "version": 1, "name": "Deneme",
        "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
            "plotScale": 1000, "workspace": "hybrid", "drawingFont": "barlow" },
        "origin": { "x": 0, "y": 0 }, "layers": layers, "activeLayer": "cizim", "entities": file["objects"],
        "styles": { "items": [], "categories": [] }
    });
    let snapshot = kentos_contracts::DocumentSnapshotV1::from_json(&snapshot.to_string())
        .expect("the drawing");
    kentos_domain::Document::from_snapshot(snapshot).expect("opens")
}

/// An object as the cases compare it: its kind, layer, label and attributes,
/// and a point's place and elevation.
fn view(e: &kentos_contracts::Entity) -> Value {
    let b = e.base();
    let mut v =
        json!({ "kind": e.kind(), "layerId": b.layer_id, "label": b.label, "attrs": b.attrs });
    if let kentos_contracts::Entity::Point(p) = e {
        v["at"] = json!([p.p.x, p.p.y]);
        v["z"] = json!(p.z);
    }
    v
}

fn op(v: &Value) -> Op {
    let text = |k: &str| v[k].as_str().unwrap_or_default().to_owned();
    match v["kind"].as_str() {
        Some("rename") => Op::Rename {
            add: v["mode"] == "add",
            prefix: text("prefix"),
        },
        Some("number") => Op::Number {
            start: text("start"),
        },
        _ => Op::Layer {
            layer: text("layer"),
        },
    }
}

fn slots(v: &Value) -> Vec<Slot> {
    v.as_array()
        .expect("ids")
        .iter()
        .map(|id| Slot(id.as_u64().unwrap_or_default() as u32))
        .collect()
}

#[test]
fn every_operation_is_written_as_the_reference_writes_it() {
    let file = file();
    assert_eq!(file["format"], "kentos.point-editor-batch");
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() >= 20, "{} cases", cases.len());
    let mut off = Vec::new();
    for c in cases {
        let name = c["name"].as_str().unwrap_or_default();
        let mut doc = drawing(&file);
        let out = run(&mut doc, &slots(&c["targets"]), &op(&c["op"]));
        let objects: Vec<Value> = doc.entities().map(view).collect();
        let step = match out.step {
            Some(_) => doc.undo(),
            None => doc.can_undo().then(|| "yazıldı".to_owned()),
        };
        let seen = json!({ "said": out.said, "step": step, "objects": objects });
        if seen != c["expected"] {
            off.push(format!("{name}: {seen}"));
        }
    }
    assert!(
        off.is_empty(),
        "{} durum farklı:\n{}",
        off.len(),
        off.join("\n")
    );
}

#[test]
fn the_selected_rows_in_the_tables_order_or_every_row() {
    let file = file();
    for c in file["targets"].as_array().expect("targets") {
        let selected = slots(&c["selected"]);
        let (ids, header) = targets(&slots(&c["shown"]), |s| selected.contains(&s));
        assert_eq!(
            (ids, header.as_str()),
            (
                slots(&c["expected"]["targets"]),
                c["expected"]["header"].as_str().unwrap_or_default()
            ),
            "{}",
            c["name"]
        );
    }
}
