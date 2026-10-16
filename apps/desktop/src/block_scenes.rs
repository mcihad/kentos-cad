//! The pictures of docs/adr/0315 (a block's layers, its own y factor and its unit), and the drawing that shows the
//! layer rule at work: one door block placed on Kapılar, on Mobilya and on Kapılar in a blue of its own. Its leaf is
//! on the block's layer and follows the layer (BYLAYER: the insert's layer's colour), its swing is on the block's
//! layer and takes the insert (BYBLOCK: the insert's own blue), its frame is on Duvar (always red), and a detail on
//! the hidden Ayrıntı layer does not show. Beside them a table with four chairs placed as drawn and stretched
//! 1.6 × 1 (the chairs' circles ellipses). `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=blok-katman-kurali,blok-olcek,blok-olcek-oznitelik,blok-ekle-y cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! And docs/adr/0316's Blok düzenleyici: the door opened in it (its own layer “0 (blok)”, Duvar, the hidden
//! Ayrıntı), a handle drawn, and the drawing after Kaydet, every door with its handle:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=blok-duzenleyici,blok-duzenleyici-cizim,blok-duzenleyici-kaydedildi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, hover, layer, open_edited, run, typed};
use crate::tools_screens::Scene;
use kentos_ui::widget::docking;

const KAPI: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d5101";
const MASA: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d5102";
const ALL: [&str; 4] = ["color", "lineType", "lineWeight", "transparency"];

fn piece(kind: &str, i: u32, layer: &str, by_layer: bool, fields: Value) -> Value {
    let mut e = json!({ "kind": kind, "id": i, "layerId": layer, "attrs": {} });
    for (k, v) in fields.as_object().expect("fields") {
        e[k] = v.clone();
    }
    if by_layer {
        e["byLayer"] = json!(ALL);
    }
    e
}

fn p(x: f64, y: f64) -> Value {
    json!({ "x": x, "y": y })
}

fn blocks() -> Value {
    json!([
        {
            "id": KAPI, "name": "Kapı 90", "base": p(0.0, 0.0), "units": "m",
            "entities": [
                // The leaf on the block's layer, BYLAYER: the insert's layer.
                piece("line", 1, "", true, json!({ "a": p(0.0, 0.0), "b": p(0.0, 0.9), "lineWeight": 0.5 })),
                // The swing on the block's layer, BYBLOCK: the insert's own.
                piece("arc", 2, "", false, json!({ "c": p(0.0, 0.0), "r": 0.9, "a0": 0.0, "a1": std::f64::consts::FRAC_PI_2 })),
                // The frame on Duvar, BYLAYER: always Duvar's.
                piece("polygon", 3, "duvar", true, json!({ "pts": [p(-0.1, -0.1), p(1.0, -0.1), p(1.0, 0.0), p(-0.1, 0.0)] })),
                // A detail on the hidden Ayrıntı layer: not shown.
                piece("circle", 4, "ayrinti", true, json!({ "c": p(0.45, 0.45), "r": 0.12 })),
            ]
        },
        {
            "id": MASA, "name": "Masa", "base": p(0.0, 0.0), "units": "m",
            "entities": [
                piece("polygon", 1, "", true, json!({ "pts": [p(-0.6, -0.4), p(0.6, -0.4), p(0.6, 0.4), p(-0.6, 0.4)] })),
                piece("circle", 2, "", true, json!({ "c": p(-0.35, 0.7), "r": 0.22 })),
                piece("circle", 3, "", true, json!({ "c": p(0.35, 0.7), "r": 0.22 })),
                piece("circle", 4, "", true, json!({ "c": p(-0.35, -0.7), "r": 0.22 })),
                piece("circle", 5, "", true, json!({ "c": p(0.35, -0.7), "r": 0.22 })),
            ]
        }
    ])
}

fn ground() -> Objects {
    let mut o = Objects::new();
    let kapi =
        |o: &mut Objects, layer: &str, x: f64| o.insert(layer, KAPI, [x, 0.0], 1.0, 0.0, false);
    kapi(&mut o, "kapilar", 0.0);
    kapi(&mut o, "mobilya", 2.5);
    let own = kapi(&mut o, "kapilar", 5.0);
    o.fields(own, json!({ "color": "#0090FF" }));
    o.insert("mobilya", MASA, [9.5, 0.6], 1.0, 0.0, false);
    let stretched = o.insert("mobilya", MASA, [13.0, 0.6], 1.6, 0.0, false);
    o.fields(stretched, json!({ "yScale": 1.0 }));
    for (x, words) in [
        (0.0, "Kapılar"),
        (2.5, "Mobilya"),
        (5.0, "Kendi rengi mavi"),
        (9.0, "Masa"),
        (12.2, "x 1,6  y 1"),
    ] {
        o.text("cizim", [x, -0.8], words, 0.22, 0.0);
    }
    o
}

pub(crate) fn drawn(app: &mut App) {
    open_edited(app, ground(), |d| {
        d["layers"] = json!([
            layer("cizim", "Çizim", "fg", 0.25),
            layer("duvar", "Duvar", "#E5484D", 0.5),
            layer("kapilar", "Kapılar", "#C8A165", 0.35),
            layer("mobilya", "Mobilya", "#3BA55D", 0.25),
            { "id": "ayrinti", "name": "Ayrıntı", "type": "layer", "visible": false, "locked": false,
              "expanded": true, "style": { "color": "#E06C75", "lineType": "continuous", "lineWeight": 0.25 },
              "children": [] },
        ]);
        d["blocks"] = blocks();
        // A plan's scale: its line weights on paper, not metres wide.
        d["settings"]["plotScale"] = json!(50);
    });
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 1.0,
            min_y: N - 1.4,
            max_x: E + 15.5,
            max_y: N + 2.0,
        },
        24.0,
    );
}

/// The layer rule at work: the door three times, the table as drawn and stretched.
fn layers(app: &mut App) {
    drawn(app);
}

/// The stretched table selected: its chairs ellipses, its outline wider.
fn stretched(app: &mut App) {
    drawn(app);
    app.selection.set([Slot(5)]);
}

/// Öznitelikler of the stretched table: X ölçeği 1.6, Y ölçeği 1 (the dock's upper panel folded, as the mask's).
fn properties(app: &mut App) {
    stretched(app);
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
}

/// Blok ekle with Y ölçeği 0.5: the table's ghost flattened at the cursor.
fn insert_y(app: &mut App) {
    drawn(app);
    run(app, "tool.blockInsert");
    typed(app, "B");
    typed(app, "Y");
    typed(app, "0.5");
    hover(app, [11.5, 1.5]);
}

/// The door block's id.
pub(crate) fn door() -> kentos_contracts::BlockId {
    serde_json::from_value(json!(KAPI)).expect("an id")
}

/// The door opened in Blok düzenleyici.
pub(crate) fn editing(app: &mut App) {
    drawn(app);
    let _ = app.update(Message::BlockEditor(crate::block_editor::Event::Open(
        door(),
    )));
}

/// A handle drawn on the block's layer, in the block's own coordinates.
pub(crate) fn handle(app: &mut App) {
    let doc = app.document.as_mut().expect("the editor's drawing");
    let line: kentos_contracts::LineCreate = serde_json::from_value(json!({
        "layerId": crate::block_editor::BLOCK_LAYER, "a": p(0.06, 0.45), "b": p(0.3, 0.45)
    }))
    .expect("a line");
    let done = kentos_native_application::line::execute(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        line,
    );
    assert!(
        matches!(done, kentos_contracts::CommandResult::Completed { .. }),
        "{done:?}"
    );
    app.spatial
        .reload(&app.document.as_ref().expect("a drawing").model);
}

fn editor_drawn(app: &mut App) {
    editing(app);
    handle(app);
}

fn editor_saved(app: &mut App) {
    editor_drawn(app);
    let _ = app.update(Message::BlockEditor(crate::block_editor::Event::Save));
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("blok-katman-kurali", layers),
        ("blok-olcek", stretched),
        ("blok-olcek-oznitelik", properties),
        ("blok-ekle-y", insert_y),
        ("blok-duzenleyici", editing),
        ("blok-duzenleyici-cizim", editor_drawn),
        ("blok-duzenleyici-kaydedildi", editor_saved),
    ]
}
