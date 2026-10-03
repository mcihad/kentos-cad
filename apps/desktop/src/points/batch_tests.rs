//! Nokta editörü's batch operations against
//! `fixtures/point-editor/v1/batch.json` and `dedupe.json`
//! (scripts/fixtures/point_batch_cases.py, point_dedupe_cases.py, worked out
//! from the rules with no KentOS code): every case's drawing after the
//! operation, the messages said and the undo step, the rows the operations
//! take, Çift noktaları ayıkla's groups and summary; as the web's
//! `ui/bottom/pointBatch.test.ts`.

use kentos_domain::Slot;
use serde_json::{Value, json};

use kentos_geometry_core::ops::point_editor::Keep;

use super::batch::{Op, import_targets, plan_dedupe, run, targets};

fn file() -> Value {
    serde_json::from_str(include_str!(
        "../../../../fixtures/point-editor/v1/batch.json"
    ))
    .expect("batch.json reads")
}

/// A reference's drawing: its layers (one hidden or locked) and objects.
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
            "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow" },
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
        let out = run(&mut doc, &slots(&c["targets"]), &op(&c["op"]), true);
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

/// An object with line work's paths too (dedupe.json's view).
fn seen(e: &kentos_contracts::Entity) -> Value {
    let mut v = view(e);
    if !matches!(e, kentos_contracts::Entity::Point(_)) {
        v["paths"] = kentos_native_application::elevation::paths(e)
            .iter()
            .map(|p| json!({ "pts": p.pts.iter().map(|q| [q.x, q.y]).collect::<Vec<_>>(), "zs": p.zs }))
            .collect();
    }
    v
}

#[test]
fn every_dedupe_finds_the_groups_and_writes_as_the_reference_does() {
    let file: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/point-editor/v1/dedupe.json"
    ))
    .expect("dedupe.json reads");
    assert_eq!(file["format"], "kentos.point-editor-dedupe");
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() >= 15, "{} cases", cases.len());
    let mut off = Vec::new();
    for c in cases {
        let name = c["name"].as_str().unwrap_or_default();
        let mut doc = drawing(&file);
        let op = Op::Dedupe {
            by_name: c["by"] == "name",
            tolerance: c["tolerance"].as_str().unwrap_or_default().to_owned(),
            keep: match c["keep"].as_str() {
                Some("last") => Keep::Last,
                Some("average") => Keep::Average,
                _ => Keep::First,
            },
        };
        let targets = slots(&c["targets"]);
        let plan = plan_dedupe(&doc, &targets, &op);
        let groups: Vec<Vec<u32>> = plan
            .groups
            .iter()
            .map(|g| g.iter().map(|s| s.0).collect())
            .collect();
        let out = run(
            &mut doc,
            &targets,
            &op,
            c["follow"].as_bool().unwrap_or(false),
        );
        let objects: Vec<Value> = doc.entities().map(seen).collect();
        let step = match out.step {
            Some(_) => doc.undo(),
            None => doc.can_undo().then(|| "yazıldı".to_owned()),
        };
        let got = json!({ "groups": groups, "summary": plan.summary, "said": out.said, "step": step, "objects": objects });
        if let Err(e) = super::tests::same(&got, &c["expected"], name) {
            off.push(e);
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
fn after_an_import_the_points_named_like_the_imported_ones_by_name() {
    let file: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/point-editor/v1/dedupe.json"
    ))
    .expect("dedupe.json reads");
    let cases = file["imports"].as_array().expect("imports");
    assert!(cases.len() >= 5, "{} cases", cases.len());
    // The import cases' drawing in place of the others'.
    let mut imports = file.clone();
    imports["objects"] = file["importObjects"].clone();
    let mut off = Vec::new();
    for c in cases {
        let name = c["name"].as_str().unwrap_or_default();
        let mut doc = drawing(&imports);
        let found = import_targets(&doc, &slots(&c["imported"]));
        let head = json!({
            "targets": found.as_ref().map_or_else(Vec::new, |(s, _)| s.iter().map(|s| s.0).collect()),
            "header": found.as_ref().map(|(_, h)| h.clone()),
        });
        let got = match found {
            None => head,
            Some((targets, _)) => {
                let op = Op::Dedupe {
                    by_name: true,
                    tolerance: String::new(),
                    keep: match c["keep"].as_str() {
                        Some("last") => Keep::Last,
                        Some("average") => Keep::Average,
                        _ => Keep::First,
                    },
                };
                let plan = plan_dedupe(&doc, &targets, &op);
                let groups: Vec<Vec<u32>> = plan
                    .groups
                    .iter()
                    .map(|g| g.iter().map(|s| s.0).collect())
                    .collect();
                let out = run(
                    &mut doc,
                    &targets,
                    &op,
                    c["follow"].as_bool().unwrap_or(false),
                );
                let objects: Vec<Value> = doc.entities().map(seen).collect();
                let step = match out.step {
                    Some(_) => doc.undo(),
                    None => doc.can_undo().then(|| "yazıldı".to_owned()),
                };
                let mut got = head;
                got["groups"] = json!(groups);
                got["summary"] = json!(plan.summary);
                got["said"] = json!(out.said);
                got["step"] = json!(step);
                got["objects"] = json!(objects);
                got
            }
        };
        if let Err(e) = super::tests::same(&got, &c["expected"], name) {
            off.push(e);
        }
    }
    assert!(
        off.is_empty(),
        "{} durum farklı:\n{}",
        off.len(),
        off.join("\n")
    );
}
