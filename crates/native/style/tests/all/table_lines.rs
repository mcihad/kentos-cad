//! A table (docs/adr/0184 §2) through a layer's look: its outline and its
//! lines between rows and columns are strokes; a frame width draws the
//! outline as a band, solid fills; its words are not the layer's (the host
//! draws them as text).

use kentos_contracts::{Entity, LayerStyle};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::store::Store;
use kentos_native_application::geometry::shape;
use kentos_native_style::library::StyleLibrary;
use kentos_native_style::program::{BuildOptions, build_layer};
use serde_json::json;

fn table(grid: Option<&str>, frame: Option<f64>) -> Entity {
    let mut t = json!({
        "kind": "table", "id": 1, "layerId": "a", "attrs": {},
        "p": { "x": 0.0, "y": 0.0 }, "rotation": 0.0, "height": 1.0,
        "rows": [2.0, 2.0], "columns": [5.0, 3.0],
        "cells": [["Ad", "Değer"], ["a", "1"]], "header": true
    });
    if let Some(g) = grid {
        t["grid"] = json!(g);
    }
    if let Some(f) = frame {
        t["frame"] = json!(f);
    }
    serde_json::from_value(t).expect("a table")
}

/// How many stroke and fill batches the layer makes of `e`.
fn batches(e: &Entity) -> (usize, usize) {
    let style: LayerStyle = serde_json::from_value(json!({
        "color": "#336699", "lineType": "continuous", "lineWeight": 0.25
    }))
    .expect("a style");
    let mut store = Store::new();
    store.put_many([(1.0, "a", false, shape(e))]);
    let library = StyleLibrary::default();
    let names = |_: &str| "A".to_owned();
    let opts = BuildOptions {
        origin: Vec2::new(0.0, 0.0),
        plot_scale: 1000.0,
        screen: false,
        hairlines: false,
        clip: None,
        library: &library,
        layer_name: &names,
        view: Default::default(),
        frame: None,
    };
    let (_, batches) = build_layer(&store, &style, &[e], &opts).expect("a build");
    // The batches' JSON names each batch's kind.
    let count = |kind: &str| {
        batches
            .json
            .matches(&format!("\"kind\":\"{kind}\""))
            .count()
    };
    (count("stroke"), count("fill"))
}

#[test]
fn a_table_strokes_its_lines_and_fills_its_frame() {
    let (lines, fills) = batches(&table(None, None));
    assert!(lines > 0 && fills == 0, "the whole grid: {lines}, {fills}");
    let (lines, fills) = batches(&table(Some("outer"), None));
    assert!(
        lines > 0 && fills == 0,
        "the outline alone: {lines}, {fills}"
    );
    let (lines, fills) = batches(&table(None, Some(0.2)));
    assert!(
        lines > 0 && fills > 0,
        "inner lines and the band: {lines}, {fills}"
    );
    let (lines, fills) = batches(&table(Some("outer"), Some(0.2)));
    assert!(fills > 0, "the band alone: {lines}, {fills}");
    assert_eq!(batches(&table(Some("none"), Some(0.2))), (0, 0), "nothing");
}
