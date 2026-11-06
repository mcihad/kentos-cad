//! The pictures of docs/adr/1124 and 1125 on the drawing area: texts whose style names the embedded test unifont and
//! Big Font (fixtures/shx/v1) drawn in the font's strokes (left, centred, turned, leaning, three lines, a wrapped box,
//! a Big Font code, a mask over a line) beside a TrueType text, one with a letter the font lacks (drawn in the
//! style's face, the inspection says so), and thick dash-dot lines whose dots are round and the line's weight.
//! `tools_screens` takes them:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=shx-metin-cizim,shx-metin-yakin,shx-blok-metin,shx-olcu-cizim,shx-olcu-penceresi,shx-kilavuz-penceresi,nokta-yuvarlak cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use crate::app::App;
use crate::tools_scenes::{E, N, Objects, layer, open_edited};
use crate::tools_screens::Scene;
use serde_json::json;

fn resources() -> serde_json::Value {
    let item = |file: &str, bytes: &[u8]| {
        kentos_commands::shape_resource::item(file, bytes).expect("a test SHX resource")
    };
    json!([
        item("kentos-uni.shx", include_bytes!("../../../fixtures/shx/v1/kentos-uni.shx")),
        item("kentos-big.shx", include_bytes!("../../../fixtures/shx/v1/kentos-big.shx")),
    ])
}

const SHX: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d1125";
const SHX_BLOCK: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d1020";
const SHX_DIM: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d1023";
const SHX_LEADER: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d1025";

fn drawing(app: &mut App) {
    let mut o = Objects::new();
    let shx = |o: &mut Objects, at: [f64; 2], words: &str, h: f64, turn: f64, more: serde_json::Value| {
        let id = o.text("shx", at, words, h, turn);
        o.fields(id, json!({"textStyle": SHX}));
        o.fields(id, more);
        id
    };
    shx(&mut o, [0.0, 30.0], "ŞU-S US", 3.0, 0.0, json!({}));
    shx(&mut o, [40.0, 30.0], "SUS", 3.0, 0.0, json!({"align": "baselineCenter", "widthFactor": 0.7, "oblique": 15.0}));
    shx(&mut o, [55.0, 18.0], "S-U-S", 2.5, 30.0, json!({}));
    shx(&mut o, [0.0, 20.0], "SU\nUS\nS-U", 2.0, 0.0, json!({"lineSpacing": 1.2}));
    shx(&mut o, [20.0, 20.0], "SU US S SUS U", 2.0, 0.0, json!({"boxWidth": 9.0}));
    shx(&mut o, [0.0, 4.0], "Ş\\M+18140\\M+18141\\M+18142U", 3.0, 0.0, json!({}));
    // A mask over a line: the text's own box in the area's colour.
    o.line("ref", [34.0, 5.0], [52.0, 5.0]);
    shx(&mut o, [35.0, 4.0], "SUS", 3.0, 0.0, json!({"mask": true}));
    // A letter the font lacks (A): the style's face, never a gap.
    shx(&mut o, [60.0, 4.0], "SAU", 3.0, 0.0, json!({}));
    o.text("ttf", [60.0, 30.0], "SU TTF", 3.0, 0.0);
    // ADR 1124: thick dash-dot lines, their dots round and the line's weight.
    o.line("dot", [0.0, -8.0], [70.0, -8.0]);
    o.line("dot", [0.0, -14.0], [40.0, -24.0]);
    o.circle("dot", [58.0, -18.0], 6.0);
    // docs/adr/1020: a block's one-line text in the SHX style, through an insert straight and one turned 30°
    // (an insert turns in radians); its two-line text too (docs/adr/1022).
    o.insert("shx", SHX_BLOCK, [0.0, -40.0], 1.0, 0.0, false);
    o.insert("shx", SHX_BLOCK, [30.0, -44.0], 1.5, 30f64.to_radians(), false);
    // docs/adr/1023: dimensions whose style writes its value in the SHX style (the test font has no digits: their
    // words are its letters), one turned.
    o.dimension("shx", None, [[0.0, -60.0], [20.0, -60.0]], None, -4.0, json!({"dimStyle": SHX_DIM, "text": "SUS-U"}));
    o.dimension("shx", None, [[30.0, -60.0], [44.0, -52.0]], None, -3.0, json!({"dimStyle": SHX_DIM, "text": "Ş\\M+18140U"}));
    // docs/adr/1026: a table in the SHX face.
    o.push("shx", json!({"kind": "table", "p": crate::tools_scenes::xy([60.0, -40.0]), "rotation": 0.0, "height": 1.5,
        "rows": [2.5, 2.5], "columns": [6.0, 6.0], "cells": [["SU", "US"], ["S-U", "SUS"]], "textStyle": SHX}));
    // docs/adr/1025: a leader whose style writes its note in the SHX style.
    o.push("shx", json!({"kind": "leader", "pts": [crate::tools_scenes::xy([52.0, -62.0]), crate::tools_scenes::xy([56.0, -56.0])],
        "text": "SUS-U", "height": 2.5, "rotation": 0.0, "leaderStyle": SHX_LEADER}));
    open_edited(app, o, |d| {
        d["settings"]["leaderStyles"] = json!([
            {"id": SHX_LEADER, "name": "Ofis not", "height": 2.5, "textStyle": "Uni"}
        ]);
        d["settings"]["dimensionStyles"] = json!([
            {"id": SHX_DIM, "name": "Ofis ölçü", "height": 2.5, "arrow": "closed", "textStyle": "Uni"}
        ]);
        d["blocks"] = json!([{"id": SHX_BLOCK, "name": "SHX-ETIKET", "base": {"x": 0.0, "y": 0.0}, "entities": [
            {"kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": {"x": -1.0, "y": -1.0}, "b": {"x": 14.0, "y": -1.0}},
            {"kind": "text", "id": 2, "layerId": "", "attrs": {}, "p": {"x": 0.0, "y": 0.0}, "text": "SUS-U",
             "height": 2.5, "rotation": 0.0, "textStyle": SHX},
            {"kind": "text", "id": 3, "layerId": "", "attrs": {}, "p": {"x": 16.0, "y": 3.0}, "text": "SU\nUS-S",
             "height": 1.5, "rotation": 0.0, "textStyle": SHX, "lineSpacing": 1.2}]}]);
        let mut dots = layer("dot", "NOKTA", "#E5484D", 2.0);
        dots["style"]["lineType"] = json!("dashed");
        dots["style"]["linePattern"] = json!("DD");
        d["layers"] = json!([
            layer("shx", "SHX", "fg", 0.35),
            layer("ttf", "TTF", "#3E63DD", 0.25),
            layer("ref", "REFERANS", "#30A46C", 0.5),
            dots,
        ]);
        d["activeLayer"] = json!("shx");
        d["settings"]["drawingUnit"] = json!("m");
        d["settings"]["textStyles"] = json!([
            {"id": SHX, "name": "Uni", "font": "barlow", "fontFile": "kentos-uni.shx", "bigFontFile": "kentos-big.shx"}
        ]);
        d["settings"]["lineTypes"] = json!([
            {"name": "DD", "unit": "drawing", "elements": [{"length": 4.0}, {"length": -1.5}, {"length": 0.0}, {"length": -1.5}]}
        ]);
        d["styles"]["items"] = resources();
    });
    crate::files_testing::make_cad(app);
}

fn fit(app: &mut App, b: [f64; 4]) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds { min_x: E + b[0], min_y: N + b[1], max_x: E + b[2], max_y: N + b[3] },
        24.0,
    );
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("shx-metin-cizim", |app| {
            drawing(app);
            fit(app, [-2.0, -26.0, 80.0, 36.0]);
        }),
        ("shx-metin-yakin", |app| {
            drawing(app);
            fit(app, [-1.0, 1.0, 32.0, 34.0]);
        }),
        ("shx-blok-metin", |app| {
            drawing(app);
            fit(app, [-3.0, -50.0, 64.0, -20.0]);
        }),
        ("shx-olcu-cizim", |app| {
            drawing(app);
            fit(app, [-3.0, -68.0, 76.0, -36.0]);
        }),
        ("shx-kilavuz-penceresi", |app| {
            drawing(app);
            fit(app, [-3.0, -68.0, 72.0, -48.0]);
            crate::tools_scenes::run(app, "leader.styles");
            let _ = app.update(crate::app::Message::LeaderTools(crate::leader_tools::Event::Choose(0)));
        }),
        ("shx-olcu-penceresi", |app| {
            drawing(app);
            fit(app, [-3.0, -68.0, 50.0, -48.0]);
            crate::tools_scenes::run(app, "style.dimensionStyles");
            let _ = app.update(crate::app::Message::AnnotationStyles(crate::annotation_styles::Event::Choose(Some(
                SHX_DIM.into(),
            ))));
        }),
        ("nokta-yuvarlak", |app| {
            drawing(app);
            fit(app, [0.0, -12.0, 20.0, -4.0]);
        }),
    ]
}
