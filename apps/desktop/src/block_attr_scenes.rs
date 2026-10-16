//! The pictures of docs/adr/0317: a door block with attributes of every option (Kapı no asked twice, its kind
//! preset, a note of lines, the maker constant, a code that is a field), its card, three doors and a table placed.
//! `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=blok-oznitelik-secenekleri,blok-oznitelik-degerleri,blok-karti,blok-cizelgesi,blok-kartlar cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, layer, open_edited, run};
use crate::tools_screens::Scene;

const KAPI: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d5201";
const MASA: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d5202";
const PENCERE: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d5203";

fn p(x: f64, y: f64) -> Value {
    json!({ "x": x, "y": y })
}

fn piece(kind: &str, i: u32, fields: Value) -> Value {
    let mut e = json!({ "kind": kind, "id": i, "layerId": "", "attrs": {},
        "byLayer": ["color", "lineType", "lineWeight", "transparency"] });
    for (k, v) in fields.as_object().expect("fields") {
        e[k] = v.clone();
    }
    e
}

fn attribute(tag: &str, prompt: &str, value: &str, y: f64, options: Value) -> Value {
    let mut a = json!({ "tag": tag, "prompt": prompt, "value": value, "p": p(0.1, y), "height": 0.12,
        "rotation": 0.0 });
    for (k, v) in options.as_object().expect("options") {
        a[k] = v.clone();
    }
    a
}

fn blocks() -> Value {
    json!([
        {
            "id": KAPI, "name": "Kapı 90", "base": p(0.0, 0.0), "units": "m",
            "description": "Tek kanatlı iç kapı",
            "card": { "category": "Mimari/Kapılar", "tags": ["kapı", "tek kanat"], "version": 3 },
            "entities": [
                piece("line", 1, json!({ "a": p(0.0, 0.0), "b": p(0.0, 0.9) })),
                piece("arc", 2, json!({ "c": p(0.0, 0.0), "r": 0.9, "a0": 0.0,
                    "a1": std::f64::consts::FRAC_PI_2 })),
            ],
            "attributes": [
                attribute("NO", "Kapı no", "", -0.25, json!({ "verify": true, "movable": true })),
                attribute("TUR", "Türü", "İç kapı", -0.42, json!({ "preset": true })),
                attribute("NOT", "Not", "Ahşap\nsöve", -0.59, json!({ "multiline": true, "boxWidth": 1.2 })),
                attribute("YAPIMCI", "Yapımcı", "KentOS Yapı", -0.76, json!({ "constant": true,
                    "invisible": true })),
                attribute("KOD", "Kod", "", -0.93, json!({ "field": "{NO}/{TUR}" })),
            ]
        },
        {
            "id": MASA, "name": "Masa 4 kişilik", "base": p(0.0, 0.0), "units": "m",
            "card": { "category": "Mobilya/Masalar", "tags": ["masa"], "version": 1 },
            "entities": [
                piece("polygon", 1, json!({ "pts": [p(-0.6, -0.4), p(0.6, -0.4), p(0.6, 0.4), p(-0.6, 0.4)] })),
                piece("circle", 2, json!({ "c": p(-0.35, 0.7), "r": 0.22 })),
                piece("circle", 3, json!({ "c": p(0.35, 0.7), "r": 0.22 })),
                piece("circle", 4, json!({ "c": p(-0.35, -0.7), "r": 0.22 })),
                piece("circle", 5, json!({ "c": p(0.35, -0.7), "r": 0.22 })),
            ]
        },
        {
            "id": PENCERE, "name": "Pencere 120 (çift kanatlı, sürme, ısı yalıtımlı)", "base": p(0.0, 0.0),
            "units": "m",
            "card": { "category": "Mimari/Pencereler", "tags": [], "version": 1 },
            "entities": [
                piece("polygon", 1, json!({ "pts": [p(0.0, 0.0), p(1.2, 0.0), p(1.2, 0.15), p(0.0, 0.15)] })),
                piece("line", 2, json!({ "a": p(0.6, 0.0), "b": p(0.6, 0.15) })),
            ]
        }
    ])
}

fn ground() -> Objects {
    let mut o = Objects::new();
    for (x, no, layer) in [
        (0.0, "K-01", "kapilar"),
        (2.5, "K-02", "kapilar"),
        (5.0, "K-03", "zemin"),
    ] {
        let id = o.insert(layer, KAPI, [x, 0.0], 1.0, 0.0, false);
        o.fields(
            id,
            json!({ "attrs": { "NO": no, "TUR": "İç kapı", "KOD": format!("{no}/İç kapı") } }),
        );
    }
    o.insert("mobilya", MASA, [9.0, 0.6], 1.0, 0.0, false);
    o.insert("kapilar", PENCERE, [11.0, 0.0], 1.0, 0.0, false);
    o
}

fn drawn(app: &mut App) {
    open_edited(app, ground(), |d| {
        d["layers"] = json!([
            layer("kapilar", "Kapılar", "#C8A165", 0.35),
            layer("zemin", "Zemin kat", "#3E63DD", 0.35),
            layer("mobilya", "Mobilya", "#3BA55D", 0.25),
        ]);
        d["blocks"] = blocks();
        d["settings"]["plotScale"] = json!(50);
    });
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 1.0,
            min_y: N - 1.6,
            max_x: E + 13.0,
            max_y: N + 2.0,
        },
        24.0,
    );
}

fn door() -> kentos_contracts::BlockId {
    serde_json::from_value(json!(KAPI)).expect("an id")
}

/// Blok öznitelikleri of the door: its rows and their options.
fn options(app: &mut App) {
    drawn(app);
    app.open_block_attributes(door());
}

/// Blok ekle's question: Kapı no twice, the note's lines; the kind, the maker and the code not asked.
fn values(app: &mut App) {
    drawn(app);
    app.memory.block_insert = Some(door());
    run(app, "tool.blockInsert");
    click(app, [7.0, 0.0]);
}

/// The door's card.
fn card(app: &mut App) {
    drawn(app);
    run(app, "block.panel");
    app.open_block_card(door());
}

/// Blok çizelgesi of every block, the inserts whose kind is İç kapı scheduled.
fn schedule(app: &mut App) {
    use crate::block_schedule::Event;
    drawn(app);
    app.open_block_schedule(None);
    for e in [
        Event::Tag("TUR".to_owned()),
        Event::Value("iç kapı".to_owned()),
        Event::Find,
    ] {
        let _ = app.update(Message::BlockSchedule(e));
    }
}

/// Bloklar as cards.
fn cards(app: &mut App) {
    drawn(app);
    run(app, "block.panel");
    let _ = app.update(Message::BlocksPanel(crate::blocks_panel::Event::Cards(
        true,
    )));
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("blok-oznitelik-secenekleri", options),
        ("blok-oznitelik-degerleri", values),
        ("blok-karti", card),
        ("blok-cizelgesi", schedule),
        ("blok-kartlar", cards),
    ]
}
