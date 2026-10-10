//! The legend as `fixtures/style/v1/legend.json` holds it (recorded from the
//! web's `style/legend.ts`, which checks the same file in
//! `legendFixture.test.ts`): the layers it reads, each layer's rows, the
//! picture's layout and the window's texts.

use std::path::PathBuf;

use kentos_contracts::{Entity, LayerStyle};
use kentos_native_style::legend::{
    LegendGroup, LegendLayer, LegendSources, PAPER, legend_layers, legend_layout, legend_of, texts,
};
use serde_json::{Value, json};

fn fixture() -> Value {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/style/v1/legend.json");
    let text = std::fs::read_to_string(&path).expect("legend.json");
    serde_json::from_str(&text).expect("legend.json is JSON")
}

/// Numbers as floats on both sides: the web writes 7, a Rust f64 7.0.
fn norm(v: &Value) -> Value {
    match v {
        Value::Number(n) => n.as_f64().map_or(Value::Null, Value::from),
        Value::Array(a) => Value::Array(a.iter().map(norm).collect()),
        Value::Object(o) => Value::Object(o.iter().map(|(k, v)| (k.clone(), norm(v))).collect()),
        other => other.clone(),
    }
}

struct Sources {
    entities: Vec<Entity>,
    library: Value,
}

impl LegendSources for Sources {
    fn entities(&self, layer: &str) -> Vec<&Entity> {
        self.entities
            .iter()
            .filter(|e| e.base().layer_id == layer)
            .collect()
    }

    fn symbol(&self, id: &str) -> Option<Value> {
        self.library.get(id).and_then(|i| i.get("symbol")).cloned()
    }

    fn item_name(&self, id: &str) -> Option<String> {
        self.library
            .get(id)
            .and_then(|i| i.get("name"))
            .and_then(Value::as_str)
            .map(str::to_owned)
    }
}

struct Leaf {
    id: String,
    name: String,
    style: LayerStyle,
    visible: bool,
}

#[test]
fn draws_the_web_s_legend() {
    let f = fixture();
    assert_eq!(f["format"], "kentos.style-legend");
    assert_eq!(f["version"], 1);
    let leaves: Vec<Leaf> = f["layers"]
        .as_array()
        .expect("layers")
        .iter()
        .map(|l| Leaf {
            id: l["id"].as_str().expect("id").to_owned(),
            name: l["name"].as_str().expect("name").to_owned(),
            style: serde_json::from_value(l["style"].clone()).expect("style"),
            visible: l.get("visible").and_then(Value::as_bool) != Some(false),
        })
        .collect();
    let src = Sources {
        entities: f["entities"]
            .as_array()
            .expect("entities")
            .iter()
            .map(|e| serde_json::from_value(e.clone()).expect("entity"))
            .collect(),
        library: f["library"].clone(),
    };
    let listed: Vec<(LegendLayer<'_>, bool)> = leaves
        .iter()
        .map(|l| {
            (
                LegendLayer {
                    id: &l.id,
                    name: &l.name,
                    style: &l.style,
                },
                l.visible,
            )
        })
        .collect();
    let mut problems = Vec::new();

    for case in f["legends"].as_array().expect("legends") {
        let visible_only = case["visibleOnly"].as_bool().expect("visibleOnly");
        let layers = legend_layers(&listed, visible_only);
        let ids: Vec<&str> = layers.iter().map(|l| l.id).collect();
        if json!(ids) != case["layers"] {
            problems.push(format!("layers {visible_only}: {ids:?}"));
        }
        let groups = serde_json::to_value(legend_of(&layers, &src)).expect("groups");
        if norm(&groups) != norm(&case["groups"]) {
            problems.push(format!(
                "groups {visible_only}:\n  got  {groups}\n  want {}",
                case["groups"]
            ));
        }
    }

    let visible = legend_layers(&listed, true);
    let drawing = f["drawingName"].as_str().expect("drawingName");
    for case in f["layouts"].as_array().expect("layouts") {
        let left: Vec<&str> = case["left"]
            .as_array()
            .expect("left")
            .iter()
            .filter_map(Value::as_str)
            .collect();
        let groups: Vec<LegendGroup> = legend_of(&visible, &src)
            .into_iter()
            .filter(|g| !left.contains(&g.layer_id.as_str()))
            .collect();
        if let Some(rows) = case.get("rows").and_then(Value::as_u64) {
            let n: usize = groups.iter().map(|g| g.entries.len()).sum();
            assert_eq!(n as u64, rows, "rows with {left:?} left out");
        }
        let headings = case["headings"].as_bool().expect("headings");
        let layout =
            serde_json::to_value(legend_layout(&groups, headings, drawing)).expect("layout");
        if norm(&layout) != norm(&case["layout"]) {
            problems.push(format!(
                "layout headings {headings}, left {left:?}:\n  got  {layout}\n  want {}",
                case["layout"]
            ));
        }
    }

    let paper = json!({
        "background": PAPER.background,
        "ink": PAPER.ink,
        "paper": PAPER.paper,
        "fg": PAPER.fg,
        "fgDim": PAPER.fg_dim,
    });
    if norm(&paper) != norm(&f["paper"]) {
        problems.push(format!("paper: {paper}"));
    }
    let words = json!({
        "title": texts::TITLE,
        "save": texts::SAVE,
        "close": texts::CLOSE,
        "visibleOnly": texts::VISIBLE_ONLY,
        "headings": texts::HEADINGS,
        "rows": { "n": 12, "text": texts::rows(12) },
        "nothing": texts::NOTHING,
        "noRows": texts::NO_ROWS,
        "saved": texts::SAVED,
        "file": texts::FILE,
        "heading": texts::HEADING,
    });
    if norm(&words) != norm(&f["texts"]) {
        problems.push(format!("texts:\n  got  {words}\n  want {}", f["texts"]));
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn a_layer_with_texts_only_or_an_unreadable_renderer_has_no_rows_of_its_own() {
    let style: LayerStyle = serde_json::from_value(json!({
        "color": "#4E79A7", "lineType": "continuous", "lineWeight": 0.25,
        "renderer": { "type": "voronoi", "cells": 5 },
    }))
    .expect("style");
    let entities: Vec<Entity> = [
        json!({ "kind": "polyline", "id": 1, "layerId": "a", "attrs": {}, "pts": [{ "x": 0, "y": 0 }, { "x": 1, "y": 1 }], "symbol": "kendi" }),
        json!({ "kind": "text", "id": 2, "layerId": "b", "attrs": {}, "p": { "x": 0, "y": 0 }, "text": "Yazı", "height": 2, "rotation": 0 }),
    ]
    .into_iter()
    .map(|e| serde_json::from_value(e).expect("entity"))
    .collect();
    let src = Sources {
        entities,
        library: json!({ "kendi": { "name": "Kendi çizgim", "symbol": { "type": "line", "layers": [] } } }),
    };
    let layers = [
        LegendLayer {
            id: "a",
            name: "A",
            style: &style,
        },
        LegendLayer {
            id: "b",
            name: "B",
            style: &style,
        },
    ];
    let groups = legend_of(&layers, &src);
    // A kind this version cannot read draws nothing here; the object's own symbol still has its row; B has only a text.
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].entries.len(), 1);
    assert_eq!(groups[0].entries[0].label, "Kendi çizgim");
}
