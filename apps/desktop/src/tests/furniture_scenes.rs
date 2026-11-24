//! The pictures of docs/adr/0702: a 9 × 6 m flat of one closed wall and two inner walls, furnished with KentOS's
//! library at 1:50 (bedroom, living room, kitchen, bathroom with the fixtures' clearance shown); the Bloklar panel's
//! cards of the kitchen; Blok ekle's Duvara hizala laying the bed's head on the bedroom's wall:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=mobilya-daire,mobilya-kartlar,mobilya-duvara-hizala cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! And docs/adr/0706's Mutfak dizisi in a 4.20 × 3.00 m kitchen with no cabinet block of its own: the run along the top
//! wall under the cursor, its side chosen below the line (Otomatik evye and ocak, the tool's settings beside); an L of
//! two runs written with wall cabinets; a run too short for one module, said in red; the Mimari tab with the tool:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=mutfak-dizisi,mutfak-l-yazildi,mutfak-kisa,mutfak-serit cargo test -p kentos-desktop tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.
// The core's test kit (`testing`, docs/adr/9199 §4.11): a module package's tests use its helpers; what only the
// core's tests use is left unused there.
#![cfg_attr(not(test), allow(dead_code, unused_imports))]

use std::f64::consts::{FRAC_PI_2, PI};

use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, layer, open_edited, run, typed};
use crate::tools_screens::Scene;

const LIBRARY: &str = include_str!("../../../../catalog/v1/blocks/kentos-temel.kblok");

pub fn library() -> Value {
    let file: Value = serde_json::from_str(LIBRARY).expect("the library");
    file["blocks"].clone()
}

pub fn id_of(name: &str) -> String {
    library()
        .as_array()
        .expect("blocks")
        .iter()
        .find(|b| b["name"] == name)
        .and_then(|b| b["id"].as_str())
        .expect("a block of that name")
        .to_owned()
}

pub fn xy(x: f64, y: f64) -> Value {
    json!({ "x": E + x, "y": N + y })
}

pub fn wall(o: &mut Objects, pts: &[(f64, f64)], width: f64, closed: bool) {
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
pub(crate) fn flat(app: &mut App) {
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

/// The kitchen: one closed wall whose inner faces are 0–4.20 × 0–3.00, no block in the drawing; the view on it.
fn kitchen(app: &mut App) {
    let mut o = Objects::new();
    wall(
        &mut o,
        &[(-0.125, -0.125), (4.325, -0.125), (4.325, 3.125), (-0.125, 3.125)],
        0.25,
        true,
    );
    o.text("cizim", [1.4, 1.1], "MUTFAK", 0.16, 0.0);
    open_edited(app, o, |d| {
        d["layers"] = json!([
            layer("cizim", "Çizim", "fg", 0.25),
            layer("duvar", "Duvar", "fg", 0.5),
            layer("mutfak", "Mutfak", "#C8A165", 0.25),
        ]);
        d["settings"]["plotScale"] = json!(50);
    });
    make_cad(app);
    if let Some(doc) = app.document.as_mut() {
        doc.model.set_active_layer("mutfak");
    }
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 0.6,
            min_y: N - 0.9,
            max_x: E + 4.8,
            max_y: N + 3.6,
        },
        24.0,
    );
}

/// docs/adr/0706: Mutfak dizisi along the top wall's inner face, start and end clicked, the cursor below the line: six
/// modules, the sink in the second, the cooktop in the fifth, a 0.30 m filler; the tool's settings beside.
fn kitchen_run(app: &mut App) {
    kitchen(app);
    app.command_expanded = true;
    run(app, "tool.casework");
    click(app, [0.0, 3.0]);
    click(app, [4.2, 3.0]);
    hover(app, [2.0, 1.9]);
}

/// An L kitchen written by two runs with wall cabinets: the top run's sink and cooktop, then down the right wall from
/// under its countertop, without them.
fn kitchen_l(app: &mut App) {
    kitchen(app);
    app.command_expanded = true;
    run(app, "tool.casework");
    typed(app, "U");
    click(app, [0.0, 3.0]);
    click(app, [4.2, 3.0]);
    click(app, [2.0, 1.9]);
    typed(app, "E");
    typed(app, "Yok");
    typed(app, "O");
    typed(app, "Yok");
    click(app, [4.2, 2.4]);
    click(app, [4.2, 0.0]);
    click(app, [3.0, 1.2]);
    hover(app, [1.2, 0.5]);
}

/// The L kitchen on CAD's Mimari tab: Mutfak dizisi with the stair and the ramp under Yerleştir.
fn kitchen_ribbon(app: &mut App) {
    kitchen_l(app);
    app.tab = "arch";
}

/// A run of 0.40 m: shorter than a 60 cm module; the tag says why in red and nothing is written.
fn kitchen_short(app: &mut App) {
    kitchen(app);
    app.command_expanded = true;
    run(app, "tool.casework");
    click(app, [0.0, 3.0]);
    click(app, [0.4, 3.0]);
    hover(app, [0.6, 2.2]);
}

/// docs/adr/9307: the everyday families placed in one plan at 1:100 — an office with a meeting table and workstations,
/// a classroom, a clinic room, an accessible wet room (clearances shown), a car park's bays and the landscape.
pub(crate) fn families_plan(app: &mut App) {
    let mut o = Objects::new();
    for (x0, y0, x1, y1) in [
        (0.0, 0.0, 7.0, 5.0),
        (8.0, 0.0, 15.0, 5.0),
        (0.0, -7.0, 7.0, -2.0),
        (8.0, -7.0, 12.5, -2.0),
    ] {
        wall(
            &mut o,
            &[(x0 - 0.1, y0 - 0.1), (x1 + 0.1, y0 - 0.1), (x1 + 0.1, y1 + 0.1), (x0 - 0.1, y1 + 0.1)],
            0.2,
            true,
        );
    }
    let mut put = |layer: &str, name: &str, at: [f64; 2], turn: f64, params: Value| {
        let i = o.insert(layer, &id_of(name), at, 1.0, turn, false);
        if !params.is_null() {
            o.fields(i, json!({ "parameters": params }));
        }
    };
    // Office: the eight-seat table, two workstations on the bottom wall, the archive cabinet on the top one.
    put("ofis", "Toplantı masası 8 kişilik", [3.5, 3.0], 0.0, Value::Null);
    put("ofis", "İş istasyonu", [0.1, 0.1], 0.0, Value::Null);
    put("ofis", "İş istasyonu", [5.2, 0.1], 0.0, Value::Null);
    put("ofis", "Arşiv dolabı", [6.9, 5.0], PI, json!({ "Genişlik": 1.4 }));
    // Classroom: the board on the bottom wall, the teacher's desk before it, the pupils' desks facing it.
    put("okul", "Yazı tahtası", [9.0, 0.0], 0.0, Value::Null);
    put("okul", "Öğretmen masası", [13.2, 1.4], PI, Value::Null);
    for (row, y) in [2.4, 3.4, 4.4].into_iter().enumerate() {
        put("okul", "Öğrenci sırası (çift)", [8.6, y], 0.0, Value::Null);
        put("okul", "Öğrenci sırası (çift)", [10.9, y], 0.0, Value::Null);
        if row < 2 {
            put("okul", "Öğrenci sırası (tek)", [13.3, y], 0.0, Value::Null);
        }
    }
    // Clinic room: the beds' heads on the bottom wall, the couch, the basin.
    put("saglik", "Hasta yatağı", [0.6, -7.0], 0.0, Value::Null);
    put("saglik", "Hasta yatağı", [2.0, -7.0], 0.0, Value::Null);
    put("saglik", "Muayene masası", [3.8, -7.0], 0.0, Value::Null);
    put("saglik", "Lavabo (sağlık)", [6.2, -7.0], 0.0, Value::Null);
    // Accessible wet room: the toilet 46 cm from the left wall, the basin, the 150 cm turning circle.
    let shown = json!({ "Görünüş": "Boşluk zarfı" });
    put("erisilebilir", "Erişilebilir klozet", [8.46, -7.0], 0.0, shown.clone());
    put("erisilebilir", "Erişilebilir lavabo", [11.2, -7.0], 0.0, shown);
    put("erisilebilir", "Dönüş alanı Ø150", [10.1, -4.6], 0.0, Value::Null);
    // Outside: two cars and two bicycle places, the trees and shrubs.
    put("park", "Otomobil", [13.4, -7.0], 0.0, Value::Null);
    put("park", "Otomobil", [15.6, -7.0], 0.0, Value::Null);
    put("park", "Bisiklet park yeri", [17.8, -7.0], 0.0, Value::Null);
    put("park", "Bisiklet park yeri", [18.6, -7.0], 0.0, Value::Null);
    put("bitki", "Ağaç Ø8 m", [19.8, 3.0], 0.0, Value::Null);
    put("bitki", "Ağaç Ø5 m", [23.0, -3.0], 0.0, Value::Null);
    put("bitki", "Ağaç Ø3 m", [21.2, -6.0], 0.0, Value::Null);
    put("bitki", "Çalı", [16.5, -1.4], 0.0, Value::Null);
    put("bitki", "Çalı", [16.2, -0.2], 0.0, Value::Null);
    for (x, y, words) in [
        (0.4, 4.4, "OFİS"),
        (8.4, 4.6, "DERSLİK"),
        (0.4, -2.5, "MUAYENE"),
        (8.4, -2.5, "ERİŞİLEBİLİR WC"),
    ] {
        o.text("cizim", [x, y], words, 0.2, 0.0);
    }
    open_edited(app, o, |d| {
        d["layers"] = json!([
            layer("cizim", "Çizim", "fg", 0.25),
            layer("duvar", "Duvar", "fg", 0.5),
            layer("ofis", "Ofis", "#3BA55D", 0.25),
            layer("okul", "Okul", "#C8A165", 0.25),
            layer("saglik", "Sağlık", "#D14343", 0.25),
            layer("erisilebilir", "Erişilebilir", "#12A594", 0.25),
            layer("park", "Taşıt", "#5B6B8C", 0.25),
            layer("bitki", "Bitki", "#3BA55D", 0.25),
        ]);
        d["blocks"] = library();
        d["settings"]["plotScale"] = json!(100);
    });
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds { min_x: E - 1.0, min_y: N - 8.0, max_x: E + 25.5, max_y: N + 7.5 },
        24.0,
    );
}

/// The Bloklar panel's cards of the accessible wet room's family.
fn families_cards(app: &mut App) {
    families_plan(app);
    run(app, "block.panel");
    let _ = app.update(Message::BlocksPanel(crate::blocks_panel::Event::Cards(true)));
    let _ = app.update(Message::BlocksPanel(crate::blocks_panel::Event::Category(
        Some("Erişilebilir".into()),
    )));
}

/// The Bloklar panel's list of the landscape family.
fn families_list(app: &mut App) {
    families_plan(app);
    run(app, "block.panel");
    let _ = app.update(Message::BlocksPanel(crate::blocks_panel::Event::Cards(false)));
    let _ = app.update(Message::BlocksPanel(crate::blocks_panel::Event::Category(
        Some("Peyzaj".into()),
    )));
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("mobilya-daire", flat),
        ("mobilya-kartlar", cards),
        ("mobilya-duvara-hizala", to_wall),
        ("mutfak-dizisi", kitchen_run),
        ("mutfak-l-yazildi", kitchen_l),
        ("mutfak-kisa", kitchen_short),
        ("mutfak-serit", kitchen_ribbon),
        ("blok-aileleri", families_plan),
        ("blok-aileleri-kartlar", families_cards),
        ("blok-aileleri-liste", families_list),
    ]
}
