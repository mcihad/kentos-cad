//! The pictures of docs/adr/0702: a 9 × 6 m flat of one closed wall and two inner walls, furnished with KentOS's
//! library at 1:50 (bedroom, living room, kitchen, bathroom with the fixtures' clearance shown); the Bloklar panel's
//! cards of the kitchen; Blok ekle's Duvara hizala laying the bed's head on the bedroom's wall:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=mobilya-daire,mobilya-kartlar,mobilya-duvara-hizala cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use std::f64::consts::{FRAC_PI_2, PI};

use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, hover, layer, open_edited, run, typed};
use crate::tools_screens::Scene;

const LIBRARY: &str = include_str!("../../../catalog/v1/blocks/kentos-temel.kblok");

fn library() -> Value {
    let file: Value = serde_json::from_str(LIBRARY).expect("the library");
    file["blocks"].clone()
}

fn id_of(name: &str) -> String {
    library()
        .as_array()
        .expect("blocks")
        .iter()
        .find(|b| b["name"] == name)
        .and_then(|b| b["id"].as_str())
        .expect("a block of that name")
        .to_owned()
}

fn xy(x: f64, y: f64) -> Value {
    json!({ "x": E + x, "y": N + y })
}

fn wall(o: &mut Objects, pts: &[(f64, f64)], width: f64, closed: bool) {
    let mut fields = json!({
        "kind": "wall", "pts": pts.iter().map(|&(x, y)| xy(x, y)).collect::<Vec<_>>(),
        "width": width, "justify": "center",
    });
    if closed {
        fields["closed"] = json!(true);
    }
    o.push("duvar", fields);
}

/// The flat: the outer wall's inner faces at 0–9 × 0–6, the bedroom's wall at y = 3 to x = 4, the bathroom's from
/// (6, 6) down to y = 3.5 and on to the outer wall; its furniture, turned onto their walls.
fn flat(app: &mut App) {
    let mut o = Objects::new();
    wall(
        &mut o,
        &[(-0.125, -0.125), (9.125, -0.125), (9.125, 6.125), (-0.125, 6.125)],
        0.25,
        true,
    );
    wall(&mut o, &[(0.0, 3.0), (4.0, 3.0), (4.0, 6.0)], 0.1, false);
    wall(&mut o, &[(6.0, 6.0), (6.0, 3.5), (9.0, 3.5)], 0.1, false);
    let mut put = |layer: &str, name: &str, at: [f64; 2], turn: f64, params: Value| {
        let i = o.insert(layer, &id_of(name), at, 1.0, turn, false);
        if !params.is_null() {
            o.fields(i, json!({ "parameters": params }));
        }
    };
    // The bedroom: the bed's head and the night stands on the top wall, the wardrobe on the left one.
    put("mobilya", "Çift kişilik yatak", [2.8, 6.0], PI, Value::Null);
    put("mobilya", "Komodin", [1.1, 6.0], PI, Value::Null);
    put("mobilya", "Komodin", [3.4, 6.0], PI, Value::Null);
    put("mobilya", "Dolap", [0.0, 4.6], -FRAC_PI_2, json!({ "Genişlik": 1.2 }));
    // The living room: the sofa's back on the bedroom's wall facing the TV unit, an armchair, the bookcase, the table.
    put("mobilya", "İkili kanepe", [2.65, 2.95], PI, Value::Null);
    put("mobilya", "TV ünitesi", [1.0, 0.0], 0.0, Value::Null);
    put("mobilya", "Koltuk", [3.6, 1.2], FRAC_PI_2, Value::Null);
    put("mobilya", "Kitaplık", [0.0, 2.3], -FRAC_PI_2, json!({ "Genişlik": 1.5 }));
    put("mobilya", "Yuvarlak masa", [4.9, 1.7], 0.0, Value::Null);
    // The kitchen along the bottom wall, the fridge on the right one, the wall units dashed over the counter.
    put("mutfak", "Alt dolap", [6.0, 0.0], 0.0, Value::Null);
    put("mutfak", "Evye", [6.6, 0.0], 0.0, Value::Null);
    put("mutfak", "Bulaşık makinesi", [7.7, 0.0], 0.0, Value::Null);
    put("mutfak", "Ocak", [8.3, 0.0], 0.0, Value::Null);
    put("mutfak", "Üst dolap", [6.0, 0.0], 0.0, json!({ "Genişlik": 1.2 }));
    put("mutfak", "Buzdolabı", [9.0, 1.3], FRAC_PI_2, Value::Null);
    // The bathroom, its fixtures' clearance shown.
    let shown = json!({ "Görünüş": "Boşluk zarfı" });
    put("islak", "Köşe duş", [9.0, 6.0], PI, shown.clone());
    put("islak", "Asma klozet", [7.4, 6.0], PI, shown.clone());
    put("islak", "Dolaplı lavabo", [6.05, 4.6], -FRAC_PI_2, shown);
    put("islak", "Çamaşır makinesi", [8.4, 3.55], 0.0, Value::Null);
    for (x, y, words) in [
        (1.4, 3.4, "YATAK ODASI"),
        (0.6, 0.9, "SALON"),
        (6.3, 1.4, "MUTFAK"),
        (6.4, 3.8, "BANYO"),
    ] {
        o.text("cizim", [x, y], words, 0.16, 0.0);
    }
    open_edited(app, o, |d| {
        d["layers"] = json!([
            layer("cizim", "Çizim", "fg", 0.25),
            layer("duvar", "Duvar", "fg", 0.5),
            layer("mobilya", "Mobilya", "#3BA55D", 0.25),
            layer("mutfak", "Mutfak", "#C8A165", 0.25),
            layer("islak", "Islak hacim", "#12A594", 0.25),
        ]);
        d["blocks"] = library();
        d["settings"]["plotScale"] = json!(50);
    });
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 0.8,
            min_y: N - 0.8,
            max_x: E + 9.8,
            max_y: N + 6.8,
        },
        24.0,
    );
}

/// The Bloklar panel's cards of the kitchen.
fn cards(app: &mut App) {
    flat(app);
    run(app, "block.panel");
    let _ = app.update(Message::BlocksPanel(crate::blocks_panel::Event::Cards(
        true,
    )));
    let _ = app.update(Message::BlocksPanel(crate::blocks_panel::Event::Category(
        Some("Mutfak".into()),
    )));
}

/// Blok ekle with Duvara hizala: the single bed's head lands on the bedroom's left wall, its foot into the room.
fn to_wall(app: &mut App) {
    flat(app);
    app.memory.block_insert =
        Some(serde_json::from_value(json!(id_of("Tek kişilik yatak"))).expect("an id"));
    app.command_expanded = true;
    run(app, "tool.blockInsert");
    typed(app, "H");
    hover(app, [0.2, 5.6]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("mobilya-daire", flat),
        ("mobilya-kartlar", cards),
        ("mobilya-duvara-hizala", to_wall),
    ]
}
